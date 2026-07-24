import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/public_rights/public_metadata_embedder.dart';
import 'package:hidden_shield_mobile/src/rust/api.dart' as rust_api;
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:http/http.dart' as http;
import 'package:share_plus/share_plus.dart';

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_QA_BACKEND_URL',
  defaultValue: 'http://10.0.2.2:43188',
);
const _runId = String.fromEnvironment(
  'HIDDENSHIELD_QA_RUN_ID',
  defaultValue: '',
);
const _outputDir = String.fromEnvironment('HIDDENSHIELD_QA_OUTPUT_DIR');
const _imageFormat = String.fromEnvironment(
  'HIDDENSHIELD_QA_IMAGE_FORMAT',
  defaultValue: 'png',
);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _PublicMetadataEmbedClickQaApp());
}

class _PublicMetadataEmbedClickQaApp extends StatefulWidget {
  const _PublicMetadataEmbedClickQaApp();

  @override
  State<_PublicMetadataEmbedClickQaApp> createState() =>
      _PublicMetadataEmbedClickQaAppState();
}

class _PublicMetadataEmbedClickQaAppState
    extends State<_PublicMetadataEmbedClickQaApp> {
  _QaContext? _qa;
  Object? _error;
  bool _exporting = false;
  _ClickQaResult? _result;

  @override
  void initState() {
    super.initState();
    unawaited(_setup());
  }

  Future<void> _setup() async {
    try {
      await _waitForBackendHealth();
      final qa = await _prepareQaContext();
      await File(qa.readyPath).writeAsString('ready\n', flush: true);
      if (mounted) setState(() => _qa = qa);
    } catch (error) {
      debugPrint('HIDDENSHIELD_ANDROID_PUBLIC_METADATA_CLICK_QA_ERROR $error');
      if (mounted) setState(() => _error = error);
    }
  }

  Future<void> _exportByClick() async {
    final qa = _qa;
    if (qa == null || _exporting) return;
    setState(() => _exporting = true);
    try {
      final metadata = await qa.appState.fetchPublicRightsMetadata(
        qa.record.watermarkUid,
      );
      final metadataUid = metadata['watermarkUid']?.toString().trim() ?? '';
      if (metadataUid != qa.record.watermarkUid) {
        throw StateError('metadata watermarkUid mismatch');
      }
      final embedded = embedPublicRightsMetadataInImage(
        sourceBytes: qa.protectedBytes,
        metadata: metadata,
      );
      final manifestHash = metadata['manifestHash']?.toString() ?? '';
      final checks = checkEmbeddedPublicMetadataBytes(
        bytes: embedded.bytes,
        format: embedded.format,
        watermarkUid: qa.record.watermarkUid,
        manifestHash: manifestHash,
      );
      final extension = embedded.format == PublicMetadataImageFormat.jpeg
          ? 'jpg'
          : 'png';
      final embeddedPath =
          '${qa.outputDir.path}/android-click-export-${qa.runId}.$extension';
      await File(embeddedPath).writeAsBytes(embedded.bytes, flush: true);
      final result = _ClickQaResult(
        runId: qa.runId,
        format: embedded.format.name,
        watermarkUid: qa.record.watermarkUid,
        manifestHash: manifestHash,
        protectedPath: qa.protectedPath,
        embeddedPath: embeddedPath,
        resultPath: qa.resultPath,
        byteChecks: checks,
        legalConclusion: embedded.legalConclusion,
      );
      await File(result.resultPath).writeAsString(
        '${const JsonEncoder.withIndent('  ').convert(result.toJson())}\n',
        flush: true,
      );
      debugPrint(
        'HIDDENSHIELD_ANDROID_PUBLIC_METADATA_CLICK_QA_RESULT ${jsonEncode(result.toJson())}',
      );
      if (mounted) setState(() => _result = result);
      unawaited(_openSharePanel(result: result, embedded: embedded));
    } catch (error) {
      debugPrint('HIDDENSHIELD_ANDROID_PUBLIC_METADATA_CLICK_QA_ERROR $error');
      if (mounted) setState(() => _error = error);
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  Future<void> _openSharePanel({
    required _ClickQaResult result,
    required PublicMetadataEmbeddedImage embedded,
  }) async {
    await SharePlus.instance.share(
      ShareParams(
        files: [
          XFile(
            result.embeddedPath,
            mimeType: embedded.format == PublicMetadataImageFormat.jpeg
                ? 'image/jpeg'
                : 'image/png',
          ),
        ],
        subject: 'HiddenShield 嵌入公开元数据图片副本',
        text: 'HiddenShield 公开权利元数据图片副本',
        fileNameOverrides: [result.embeddedFileName],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Android Embedded Metadata Click QA',
      theme: buildHiddenShieldTheme(),
      home: Scaffold(
        body: SafeArea(
          child: _error != null
              ? _ErrorView(error: _error!)
              : _qa == null
              ? const _LoadingView()
              : _QaDetailView(
                  qa: _qa!,
                  result: _result,
                  exporting: _exporting,
                  onExport: _exportByClick,
                ),
        ),
      ),
    );
  }
}

class _LoadingView extends StatelessWidget {
  const _LoadingView();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircularProgressIndicator(),
          SizedBox(height: 16),
          Text('正在准备 Android 嵌入元数据点击 QA'),
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error});

  final Object error;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.error_outline, color: Colors.redAccent, size: 32),
          const SizedBox(height: 12),
          Text('QA 失败', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 12),
          Text('$error'),
        ],
      ),
    );
  }
}

class _QaDetailView extends StatelessWidget {
  const _QaDetailView({
    required this.qa,
    required this.result,
    required this.exporting,
    required this.onExport,
  });

  final _QaContext qa;
  final _ClickQaResult? result;
  final bool exporting;
  final VoidCallback onExport;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.inventory_2_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('版权库详情', style: Theme.of(context).textTheme.titleLarge),
                  const SizedBox(height: 4),
                  Text('${qa.record.title} · ${qa.format.toUpperCase()}'),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _Panel(
          title: '公开权利信号',
          children: [
            _Detail(label: '版权编号', value: qa.record.watermarkUid),
            _Detail(label: '训练许可', value: '允许商业训练'),
            const _Detail(label: '法律结论', value: '否'),
            const _Detail(
              label: '嵌入导出',
              value: publicRightsEmbeddedImageExportRequiresFileMessage,
            ),
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                key: const ValueKey('qa-export-embedded-image'),
                onPressed: exporting ? null : onExport,
                icon: const Icon(Icons.add_photo_alternate_outlined),
                label: Text(
                  exporting ? '导出中' : publicRightsEmbeddedImageExportLabel,
                ),
              ),
            ),
          ],
        ),
        if (result != null) ...[
          const SizedBox(height: 12),
          _Panel(
            title: '字节检查',
            children: [
              _Detail(
                label: '容器',
                value: result!.byteChecks.hasContainer ? 'PASS' : 'FAIL',
              ),
              _Detail(
                label: 'namespace',
                value: result!.byteChecks.hasNamespace ? 'PASS' : 'FAIL',
              ),
              _Detail(
                label: 'watermarkUid',
                value: result!.byteChecks.hasWatermarkUid ? 'PASS' : 'FAIL',
              ),
              _Detail(
                label: 'manifestHash',
                value: result!.byteChecks.hasManifestHash ? 'PASS' : 'FAIL',
              ),
              _Detail(
                label: 'legalConclusion=false',
                value: result!.byteChecks.hasLegalConclusionFalse
                    ? 'PASS'
                    : 'FAIL',
              ),
            ],
          ),
        ],
      ],
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            ...children,
          ],
        ),
      ),
    );
  }
}

class _Detail extends StatelessWidget {
  const _Detail({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 128, child: Text(label)),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

Future<void> _waitForBackendHealth() async {
  final client = http.Client();
  try {
    final deadline = DateTime.now().add(const Duration(seconds: 45));
    Object? lastError;
    while (DateTime.now().isBefore(deadline)) {
      try {
        final response = await client
            .get(Uri.parse('$_backendUrl/v1/health'))
            .timeout(const Duration(seconds: 3));
        if (response.statusCode == 200) return;
        lastError = 'HTTP ${response.statusCode}';
      } catch (error) {
        lastError = error;
      }
      await Future<void>.delayed(const Duration(seconds: 1));
    }
    throw StateError('backend health unavailable from Android: $lastError');
  } finally {
    client.close();
  }
}

Future<_QaContext> _prepareQaContext() async {
  final runId = _runId.isEmpty
      ? DateTime.now().millisecondsSinceEpoch.toString()
      : _runId;
  final outputDir = Directory(
    _outputDir.isEmpty
        ? '${Directory.systemTemp.path}/hiddenshield-android-click-$runId'
        : _outputDir,
  );
  await outputDir.create(recursive: true);
  await RustWatermarkBridge.init();
  final bridge = RustWatermarkBridge();
  final cloudClient = CloudAccountClient(baseUrl: _backendUrl);
  final appState = MobileAppState(
    vaultStore: MemoryVaultStore(),
    cloudAccountClient: cloudClient,
  );
  await appState.load();
  final password = 'android-click-$runId-$_imageFormat';
  await appState.completeOnboarding(
    accountLabel: 'android-click-$runId-$_imageFormat@hiddenshield.local',
    password: password,
    creatorLabel: 'Android 点击 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);
  final sourceBytes = _makePpmImage();
  final originalHash = appState.sha256HexForBytes(sourceBytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: WatermarkAssetKind.image,
    originalHash: originalHash,
    revision: 1,
  );
  final seed = WatermarkPayloadSeed(
    creatorIdentity: appState.creatorLabel,
    deviceIdentity: 'android-public-metadata-click-qa',
    mediaBytes: sourceBytes,
    timestamp: DateTime.now().microsecondsSinceEpoch,
  );
  final protected = await _writeProtectedImage(
    bridge: bridge,
    sourceBytes: sourceBytes,
    seed: seed,
    reserved: reserved?.toDraft(),
    format: _imageFormat,
  );
  final confirmed = await appState.confirmWatermarkIdForWrite(
    result: protected,
    originalHash: originalHash,
    reserved: reserved,
  );
  final extension = _imageFormat == 'jpeg' ? 'jpg' : 'png';
  final protectedPath =
      '${outputDir.path}/android-click-protected-$runId.$extension';
  await File(protectedPath).writeAsBytes(protected.bytes, flush: true);
  final record = appState.addWriteResult(
    result: protected.copyWithOutputArtifact(
      outputFileName: 'android-click-protected-$runId.$extension',
      outputLocationLabel: 'Android QA sandbox',
      outputActionLabel: 'QA artifact',
    ),
    fileName: 'android-click-public-metadata-$runId.$extension',
    allowRewrite: false,
    registryResult: confirmed,
    declaration: const WorkDeclaration(
      workSourceDeclaration: 'ai_assisted',
      trainingPermissionDeclaration: 'commercial_allowed',
      creationMethodDeclaration: 'text_to_image',
      humanEditLevelDeclaration: 'light',
      authenticityClaimDeclaration: 'synthetic',
      customRightsStatement: 'Android public metadata click QA',
    ),
  );
  await appState.syncPendingQueue();
  return _QaContext(
    runId: runId,
    format: _imageFormat,
    outputDir: outputDir,
    readyPath: '${outputDir.path}/android-click-ready-$runId.txt',
    resultPath: '${outputDir.path}/android-click-result-$runId.json',
    protectedPath: protectedPath,
    protectedBytes: Uint8List.fromList(protected.bytes),
    appState: appState,
    record: record,
  );
}

Future<WatermarkWriteResult> _writeProtectedImage({
  required RustWatermarkBridge bridge,
  required Uint8List sourceBytes,
  required WatermarkPayloadSeed seed,
  required WatermarkRegistryDraft? reserved,
  required String format,
}) async {
  if (format == 'jpeg') {
    final startedAt = DateTime.now();
    final payload = rust_api.MobileMediaPayload(
      creatorIdentity: seed.creatorIdentity,
      deviceIdentity: seed.deviceIdentity,
      mediaBytes: Uint8List.fromList(seed.mediaBytes),
      timestamp: BigInt.from(seed.timestamp),
      reservedWatermarkUid: reserved?.watermarkUid,
      registryProofHash: reserved?.registryProofHash,
      parentWatermarkUid: null,
      revision: 1,
      mediaType: 'image',
    );
    final result = await rust_api.embedImageForMobile(
      imageBytes: sourceBytes,
      payload: payload,
      outputFormat: rust_api.MobileImageOutputFormat.jpeg,
      allowRewrite: true,
    );
    final extracted = await bridge.read(
      WatermarkReadRequest(kind: WatermarkAssetKind.image, bytes: result.bytes),
    );
    return WatermarkWriteResult(
      kind: WatermarkAssetKind.image,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: 1,
      sha256: result.sha256,
      seed: seed,
      processTimeMs: DateTime.now().difference(startedAt).inMilliseconds,
      verification: WatermarkWriteVerification(
        verified: extracted?.watermarkUid == result.watermarkUid,
        watermarkUid: extracted?.watermarkUid ?? result.watermarkUid,
        revision: 1,
        message: '已回读验证版权编号，保护副本可取证。',
        fileHashHex: extracted?.fileHashHex,
        deviceIdHex: extracted?.deviceIdHex,
        payloadProtocolVersion: extracted?.payloadProtocolVersion ?? 3,
        payloadBytesLength: extracted?.payloadBytesLength ?? 39,
      ),
    );
  }
  return bridge.write(
    WatermarkWriteRequest(
      kind: WatermarkAssetKind.image,
      bytes: sourceBytes,
      seed: seed,
      allowRewrite: true,
      registryDraft: reserved,
    ),
  );
}

Future<void> _upgradeToCreator(
  MobileAppState appState,
  CloudAccountClient client, {
  required String password,
}) async {
  final profile = appState.syncProfile;
  if (profile.entitlementFeatures['cloud_sync'] == true) {
    appState.setSyncTransportMode(SyncTransportMode.cloud);
    return;
  }
  final payment = await client.createBillingPaymentSession(
    accessToken: profile.authToken!,
    accountId: profile.accountId!,
    workspaceId: profile.workspaceId!,
    planCode: 'creator',
    preferredProvider: 'fixture',
  );
  await client.reconcileBillingPaymentSession(
    accessToken: profile.authToken!,
    paymentSessionId: payment.paymentSessionId,
  );
  await appState.continueWithCloudAccount(
    identifier: profile.accountLabel!,
    password: password,
    localCreatorDisplayName: appState.creatorLabel,
  );
  appState.setSyncTransportMode(SyncTransportMode.cloud);
}

Uint8List _makePpmImage() {
  const width = 512;
  const height = 512;
  final header = ascii.encode('P6\n$width $height\n255\n');
  final pixels = BytesBuilder();
  for (var y = 0; y < height; y++) {
    for (var x = 0; x < width; x++) {
      pixels.addByte((x * 255 ~/ width).clamp(0, 255));
      pixels.addByte((y * 255 ~/ height).clamp(0, 255));
      pixels.addByte(((x + y) * 127 ~/ width).clamp(0, 255));
    }
  }
  return Uint8List.fromList([...header, ...pixels.toBytes()]);
}

class _QaContext {
  const _QaContext({
    required this.runId,
    required this.format,
    required this.outputDir,
    required this.readyPath,
    required this.resultPath,
    required this.protectedPath,
    required this.protectedBytes,
    required this.appState,
    required this.record,
  });

  final String runId;
  final String format;
  final Directory outputDir;
  final String readyPath;
  final String resultPath;
  final String protectedPath;
  final Uint8List protectedBytes;
  final MobileAppState appState;
  final VaultRecord record;
}

class _ClickQaResult {
  const _ClickQaResult({
    required this.runId,
    required this.format,
    required this.watermarkUid,
    required this.manifestHash,
    required this.protectedPath,
    required this.embeddedPath,
    required this.resultPath,
    required this.byteChecks,
    required this.legalConclusion,
  });

  final String runId;
  final String format;
  final String watermarkUid;
  final String manifestHash;
  final String protectedPath;
  final String embeddedPath;
  final String resultPath;
  final PublicMetadataByteCheck byteChecks;
  final bool legalConclusion;

  bool get pass => legalConclusion == false && byteChecks.pass;
  String get embeddedFileName => embeddedPath.split('/').last;

  Map<String, Object?> toJson() => {
    'runId': runId,
    'format': format,
    'watermarkUid': watermarkUid,
    'manifestHash': manifestHash,
    'protectedPath': protectedPath,
    'embeddedPath': embeddedPath,
    'resultPath': resultPath,
    'byteChecks': byteChecks.toJson(),
    'legalConclusion': legalConclusion,
    'pass': pass,
  };
}
