import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/public_rights/public_metadata_embedder.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:http/http.dart' as http;

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_QA_BACKEND_URL',
  defaultValue: 'http://10.0.2.2:43188',
);
const _runId = String.fromEnvironment(
  'HIDDENSHIELD_QA_RUN_ID',
  defaultValue: '',
);
const _outputDir = String.fromEnvironment('HIDDENSHIELD_QA_OUTPUT_DIR');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _PublicMetadataEmbedRuntimeQaApp());
}

class _PublicMetadataEmbedRuntimeQaApp extends StatefulWidget {
  const _PublicMetadataEmbedRuntimeQaApp();

  @override
  State<_PublicMetadataEmbedRuntimeQaApp> createState() =>
      _PublicMetadataEmbedRuntimeQaAppState();
}

class _PublicMetadataEmbedRuntimeQaAppState
    extends State<_PublicMetadataEmbedRuntimeQaApp> {
  _PublicMetadataEmbedQaResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      await _waitForBackendHealth();
      final result = await _runQa();
      debugPrint(
        'HIDDENSHIELD_ANDROID_PUBLIC_METADATA_EMBED_QA_RESULT ${jsonEncode(result.toJson())}',
      );
      if (!result.pass) {
        throw StateError('Android public metadata embedded image QA failed');
      }
      if (mounted) setState(() => _result = result);
    } catch (error) {
      debugPrint('HIDDENSHIELD_ANDROID_PUBLIC_METADATA_EMBED_QA_ERROR $error');
      if (mounted) setState(() => _error = error);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Android Public Metadata Embed QA',
      theme: buildHiddenShieldTheme(),
      home: Scaffold(
        body: SafeArea(
          child: _error != null
              ? _ErrorView(error: _error!)
              : _result == null
              ? const _LoadingView()
              : _ResultView(result: _result!),
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
          Text('正在执行 Android 图片嵌入元数据副本 QA'),
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

class _ResultView extends StatelessWidget {
  const _ResultView({required this.result});

  final _PublicMetadataEmbedQaResult result;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.image_search_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Android 图片嵌入元数据副本 QA',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 4),
                  Text('后端 $_backendUrl · ${result.runId}'),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        ...result.rows.map((row) => _QaCard(row: row)),
        const SizedBox(height: 12),
        Text(
          'PNG 保护副本来自 Android 原生 Rust bridge 写入；JPEG 用同一份 registry metadata 做移动端容器字节检查，不改变正式移动端默认 PNG 写入路径。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _QaCard extends StatelessWidget {
  const _QaCard({required this.row});

  final _EmbedQaRow row;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.image_outlined, size: 20),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${row.format.toUpperCase()} · ${row.watermarkUid}',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                Chip(
                  label: Text(row.pass ? 'PASS' : 'FAIL'),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            const SizedBox(height: 12),
            _Detail(label: 'manifestHash', value: row.manifestHash),
            _Detail(label: 'legalConclusion', value: '${row.legalConclusion}'),
            _Detail(label: '嵌入副本', value: row.embeddedPath),
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
          SizedBox(
            width: 112,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(child: Text(value, overflow: TextOverflow.visible)),
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

Future<_PublicMetadataEmbedQaResult> _runQa() async {
  final runId = _runId.isEmpty
      ? DateTime.now().millisecondsSinceEpoch.toString()
      : _runId;
  final outputDir = Directory(
    _outputDir.isEmpty
        ? '${Directory.systemTemp.path}/hiddenshield-android-metadata-embed-$runId'
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
  final password = 'android-metadata-embed-$runId';
  await appState.completeOnboarding(
    accountLabel: 'android-metadata-embed-$runId@hiddenshield.local',
    password: password,
    creatorLabel: 'Android 嵌入元数据 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);

  final protectedPng = await _writeProtectedPng(
    runId: runId,
    bridge: bridge,
    appState: appState,
  );
  await appState.syncPendingQueue();
  final metadata = await appState.fetchPublicRightsMetadata(
    protectedPng.watermarkUid,
  );
  if (metadata['legalConclusion'] == true) {
    throw StateError('metadata legalConclusion must remain false');
  }

  final metadataPath = '${outputDir.path}/android-public-metadata-$runId.json';
  await File(metadataPath).writeAsString(
    '${const JsonEncoder.withIndent('  ').convert(metadata)}\n',
    flush: true,
  );

  final pngRow = await _embedAndCheck(
    format: PublicMetadataImageFormat.png,
    sourceBytes: protectedPng.bytes,
    metadata: metadata,
    outputPath:
        '${outputDir.path}/android-protected-public-metadata-$runId.png',
  );
  final jpegRow = await _embedAndCheck(
    format: PublicMetadataImageFormat.jpeg,
    sourceBytes: _minimalJpeg(),
    metadata: metadata,
    outputPath:
        '${outputDir.path}/android-protected-public-metadata-$runId.jpg',
  );
  final result = _PublicMetadataEmbedQaResult(
    runId: runId,
    backendUrl: _backendUrl,
    metadataPath: metadataPath,
    resultPath: '${outputDir.path}/android-public-metadata-embed-result.json',
    rows: [pngRow, jpegRow],
  );
  await File(result.resultPath).writeAsString(
    '${const JsonEncoder.withIndent('  ').convert(result.toJson())}\n',
    flush: true,
  );
  return result;
}

Future<_ProtectedPng> _writeProtectedPng({
  required String runId,
  required RustWatermarkBridge bridge,
  required MobileAppState appState,
}) async {
  final bytes = _makePpmImage();
  final originalHash = appState.sha256HexForBytes(bytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: WatermarkAssetKind.image,
    originalHash: originalHash,
    revision: 1,
  );
  final result = await bridge.write(
    WatermarkWriteRequest(
      kind: WatermarkAssetKind.image,
      bytes: bytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: appState.creatorLabel,
        deviceIdentity: 'android-public-metadata-embed-qa',
        mediaBytes: bytes,
        timestamp: DateTime.now().microsecondsSinceEpoch,
      ),
      allowRewrite: true,
      registryDraft: reserved?.toDraft(),
    ),
  );
  final confirmed = await appState.confirmWatermarkIdForWrite(
    result: result,
    originalHash: originalHash,
    reserved: reserved,
  );
  appState.addWriteResult(
    result: result.copyWithOutputArtifact(
      outputFileName: 'android-protected-public-metadata-$runId.png',
      outputLocationLabel: 'Android QA sandbox',
      outputActionLabel: 'QA artifact',
    ),
    fileName: 'android-public-metadata-embed-$runId.png',
    allowRewrite: false,
    registryResult: confirmed,
    declaration: const WorkDeclaration(
      workSourceDeclaration: 'ai_assisted',
      trainingPermissionDeclaration: 'commercial_allowed',
      creationMethodDeclaration: 'text_to_image',
      humanEditLevelDeclaration: 'light',
      authenticityClaimDeclaration: 'synthetic',
      customRightsStatement: 'Android public metadata embed runtime QA',
    ),
  );
  return _ProtectedPng(
    bytes: Uint8List.fromList(result.bytes),
    watermarkUid: result.watermarkUid,
  );
}

Future<_EmbedQaRow> _embedAndCheck({
  required PublicMetadataImageFormat format,
  required Uint8List sourceBytes,
  required Map<String, Object?> metadata,
  required String outputPath,
}) async {
  final embedded = embedPublicRightsMetadataInImage(
    sourceBytes: sourceBytes,
    metadata: metadata,
    format: format,
  );
  await File(outputPath).writeAsBytes(embedded.bytes, flush: true);
  final watermarkUid = metadata['watermarkUid']?.toString() ?? '';
  final manifestHash = metadata['manifestHash']?.toString() ?? '';
  final checks = checkEmbeddedPublicMetadataBytes(
    bytes: embedded.bytes,
    format: format,
    watermarkUid: watermarkUid,
    manifestHash: manifestHash,
  );
  return _EmbedQaRow(
    format: format.name,
    watermarkUid: watermarkUid,
    manifestHash: manifestHash,
    legalConclusion: embedded.legalConclusion,
    outputSha256: sha256.convert(embedded.bytes).toString(),
    embeddedPath: outputPath,
    byteChecks: checks,
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

Uint8List _minimalJpeg() => Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9]);

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

class _ProtectedPng {
  const _ProtectedPng({required this.bytes, required this.watermarkUid});

  final Uint8List bytes;
  final String watermarkUid;
}

class _PublicMetadataEmbedQaResult {
  const _PublicMetadataEmbedQaResult({
    required this.runId,
    required this.backendUrl,
    required this.metadataPath,
    required this.resultPath,
    required this.rows,
  });

  final String runId;
  final String backendUrl;
  final String metadataPath;
  final String resultPath;
  final List<_EmbedQaRow> rows;

  bool get pass => rows.every((row) => row.pass);

  Map<String, Object?> toJson() => {
    'runId': runId,
    'backendUrl': backendUrl,
    'metadataPath': metadataPath,
    'resultPath': resultPath,
    'rows': rows.map((row) => row.toJson()).toList(),
    'pass': pass,
  };
}

class _EmbedQaRow {
  const _EmbedQaRow({
    required this.format,
    required this.watermarkUid,
    required this.manifestHash,
    required this.legalConclusion,
    required this.outputSha256,
    required this.embeddedPath,
    required this.byteChecks,
  });

  final String format;
  final String watermarkUid;
  final String manifestHash;
  final bool legalConclusion;
  final String outputSha256;
  final String embeddedPath;
  final PublicMetadataByteCheck byteChecks;

  bool get pass => legalConclusion == false && byteChecks.pass;

  Map<String, Object?> toJson() => {
    'format': format,
    'watermarkUid': watermarkUid,
    'manifestHash': manifestHash,
    'legalConclusion': legalConclusion,
    'outputSha256': outputSha256,
    'embeddedPath': embeddedPath,
    'byteChecks': byteChecks.toJson(),
    'pass': pass,
  };
}
