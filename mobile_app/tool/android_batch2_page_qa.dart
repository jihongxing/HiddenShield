import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:http/http.dart' as http;
import 'package:share_plus/share_plus.dart';

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_ANDROID_BATCH2_QA_BACKEND_URL',
  defaultValue: 'http://10.0.2.2:43188',
);
const _runIdDefine = String.fromEnvironment(
  'HIDDENSHIELD_ANDROID_BATCH2_QA_RUN_ID',
);
const _outputDirDefine = String.fromEnvironment(
  'HIDDENSHIELD_ANDROID_BATCH2_QA_OUTPUT_DIR',
);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _AndroidBatch2QaApp());
}

class _AndroidBatch2QaApp extends StatefulWidget {
  const _AndroidBatch2QaApp();

  @override
  State<_AndroidBatch2QaApp> createState() => _AndroidBatch2QaAppState();
}

class _AndroidBatch2QaAppState extends State<_AndroidBatch2QaApp> {
  _QaArtifact? _artifact;
  Object? _error;
  bool _sharing = false;

  @override
  void initState() {
    super.initState();
    unawaited(_setup());
  }

  Future<void> _setup() async {
    try {
      final artifact = await _runQa();
      await File(artifact.resultPath).writeAsString(
        '${const JsonEncoder.withIndent('  ').convert(artifact.toJson())}\n',
        flush: true,
      );
      await File(artifact.readyPath).writeAsString('ready\n', flush: true);
      if (mounted) setState(() => _artifact = artifact);
    } catch (error, stack) {
      final runId = _runIdDefine.isEmpty
          ? DateTime.now().millisecondsSinceEpoch.toString()
          : _runIdDefine;
      final outputDir = Directory(
        _outputDirDefine.isEmpty
            ? '${Directory.systemTemp.path}/hiddenshield-android-batch2-$runId'
            : _outputDirDefine,
      );
      await outputDir.create(recursive: true);
      final blocked = {
        'schemaVersion': 'android_batch2_page_qa_v1',
        'runId': runId,
        'ok': false,
        'status': 'blocked',
        'backendBaseUrl': _backendUrl,
        'error': '$error',
        'stackTail': stack.toString().split('\n').take(12).join('\n'),
      };
      await File(
        '${outputDir.path}/android-batch2-page-qa-$runId.json',
      ).writeAsString(
        '${const JsonEncoder.withIndent('  ').convert(blocked)}\n',
        flush: true,
      );
      if (mounted) setState(() => _error = error);
    }
  }

  Future<void> _shareProtectedCopy() async {
    final artifact = _artifact;
    if (artifact == null || _sharing) return;
    await _shareFile(
      path: artifact.imageProtectedPath,
      mimeType: 'image/png',
      subject: 'HiddenShield Android Batch 2 图片保护副本',
      text: 'HiddenShield Android Batch 2 图片保护副本',
      fileName: artifact.imageProtectedPath.split('/').last,
    );
  }

  Future<void> _sharePublicMetadata() async {
    final artifact = _artifact;
    if (artifact == null || _sharing) return;
    await _shareFile(
      path: artifact.publicMetadataJsonPath,
      mimeType: 'application/json',
      subject: 'HiddenShield 公开权利元数据 JSON',
      text: 'HiddenShield 公开权利元数据 JSON',
      fileName: artifact.publicMetadataJsonPath.split('/').last,
    );
  }

  Future<void> _shareFile({
    required String path,
    required String mimeType,
    required String subject,
    required String text,
    required String fileName,
  }) async {
    setState(() => _sharing = true);
    try {
      await SharePlus.instance.share(
        ShareParams(
          files: [XFile(path, mimeType: mimeType, name: fileName)],
          subject: subject,
          text: text,
          fileNameOverrides: [fileName],
        ),
      );
    } finally {
      if (mounted) setState(() => _sharing = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Android Batch 2 QA',
      theme: buildHiddenShieldTheme(),
      home: Scaffold(
        body: SafeArea(
          child: _error != null
              ? _ErrorView(error: _error!)
              : _artifact == null
              ? const _LoadingView()
              : _ResultView(
                  artifact: _artifact!,
                  sharing: _sharing,
                  onShareProtectedCopy: _shareProtectedCopy,
                  onSharePublicMetadata: _sharePublicMetadata,
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
          Text('正在执行 Android Batch 2 页面级 QA'),
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
          const Icon(Icons.error_outline, color: Colors.redAccent, size: 36),
          const SizedBox(height: 12),
          Text('QA 阻断', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 12),
          Text('$error'),
        ],
      ),
    );
  }
}

class _ResultView extends StatelessWidget {
  const _ResultView({
    required this.artifact,
    required this.sharing,
    required this.onShareProtectedCopy,
    required this.onSharePublicMetadata,
  });

  final _QaArtifact artifact;
  final bool sharing;
  final VoidCallback onShareProtectedCopy;
  final VoidCallback onSharePublicMetadata;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              artifact.ok ? Icons.verified_outlined : Icons.error_outline,
              color: artifact.ok ? Colors.green.shade700 : Colors.red.shade700,
              size: 32,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Android Batch 2 页面级 QA',
                    style: theme.textTheme.titleLarge,
                  ),
                  const SizedBox(height: 4),
                  Text('Run ${artifact.runId} · ${artifact.status}'),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _Panel(
          title: '执行顺序',
          children: [
            for (final step in artifact.steps)
              _Detail(
                label: step.label,
                value: '${step.pass ? 'PASS' : 'FAIL'} · ${step.detail}',
              ),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: '图片 / 音频写入',
          children: [
            _Detail(label: '图片编号', value: artifact.imageWatermarkUid),
            _Detail(label: '图片 payload', value: artifact.imagePayloadLabel),
            _Detail(label: '图片耗时', value: '${artifact.imageWriteMs} ms'),
            _Detail(label: '音频编号', value: artifact.audioWatermarkUid),
            _Detail(label: '音频 payload', value: artifact.audioPayloadLabel),
            _Detail(label: '音频耗时', value: '${artifact.audioWriteMs} ms'),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: '保护副本分享',
          children: [
            _Detail(label: '保护副本', value: artifact.imageProtectedFileName),
            _Detail(label: '保存位置', value: 'Android QA sandbox，不进入云同步或报告'),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                key: const ValueKey('qa-share-protected-copy'),
                onPressed: sharing ? null : onShareProtectedCopy,
                icon: const Icon(Icons.ios_share_outlined),
                label: Text(sharing ? '正在打开系统分享面板' : '保存或分享保护副本'),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: '版权库详情 / 报告草稿',
          children: [
            _Detail(label: '版权库记录数', value: '${artifact.vaultRecordCount}'),
            _Detail(label: '登记状态', value: artifact.imageRegistryStatus),
            _Detail(label: '报告编号', value: artifact.formalReportId),
            _Detail(
              label: '报告隐私',
              value: artifact.formalReportPrivacyPass ? 'PASS' : 'FAIL',
            ),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: 'L2 metadata notary',
          children: [
            _Detail(label: 'notary', value: artifact.l2NotaryId),
            _Detail(
              label: 'fingerprintRoot',
              value: artifact.l2FingerprintRoot,
            ),
            _Detail(
              label: 'privacy',
              value: artifact.l2PrivacyPass ? 'PASS' : 'FAIL',
            ),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: '公开元数据导出入口',
          children: [
            _Detail(label: 'manifestHash', value: artifact.publicManifestHash),
            _Detail(
              label: 'legalConclusion',
              value: '${artifact.publicLegalConclusion}',
            ),
            _Detail(label: 'JSON', value: artifact.publicMetadataJsonFileName),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                key: const ValueKey('qa-share-public-metadata-json'),
                onPressed: sharing ? null : onSharePublicMetadata,
                icon: const Icon(Icons.ios_share_outlined),
                label: Text(sharing ? '正在打开系统分享面板' : '分享公开元数据 JSON'),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        _Panel(
          title: '关闭后端成熟错误',
          children: [
            _Detail(label: '错误文案', value: artifact.backendOffMessage),
            _Detail(
              label: '隐私扫描',
              value: artifact.backendOffPrivacyPass ? 'PASS' : 'FAIL',
            ),
          ],
        ),
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
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
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
          SizedBox(
            width: 118,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

Future<_QaArtifact> _runQa() async {
  final runId = _runIdDefine.isEmpty
      ? DateTime.now().millisecondsSinceEpoch.toString()
      : _runIdDefine;
  final outputDir = Directory(
    _outputDirDefine.isEmpty
        ? '${Directory.systemTemp.path}/hiddenshield-android-batch2-$runId'
        : _outputDirDefine,
  );
  await outputDir.create(recursive: true);
  await _waitForBackendHealth();
  await RustWatermarkBridge.init();

  final bridge = RustWatermarkBridge();
  final cloudClient = CloudAccountClient(baseUrl: _backendUrl);
  final appState = MobileAppState(
    vaultStore: MemoryVaultStore(),
    cloudAccountClient: cloudClient,
  );
  await appState.load();
  final password = 'android-batch2-$runId';
  await appState.completeOnboarding(
    accountLabel: 'android-batch2-$runId@hiddenshield.local',
    password: password,
    creatorLabel: 'Android Batch 2 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);

  final steps = <_QaStep>[];
  final image = await _writeKindCase(
    runId: runId,
    kind: WatermarkAssetKind.image,
    bridge: bridge,
    appState: appState,
    outputDir: outputDir,
    declaration: const WorkDeclaration(
      workSourceDeclaration: 'ai_assisted',
      trainingPermissionDeclaration: 'prohibited',
      creationMethodDeclaration: 'text_to_image',
      humanEditLevelDeclaration: 'light',
      authenticityClaimDeclaration: 'synthetic',
      customRightsStatement: 'Android Batch 2 image page QA',
    ),
  );
  steps.add(
    _QaStep(
      id: 'imageWrite',
      label: '图片写入',
      pass: image.pass,
      detail: '${image.watermarkUid} · ${image.writeMs}ms',
    ),
  );

  final audio = await _writeKindCase(
    runId: runId,
    kind: WatermarkAssetKind.audio,
    bridge: bridge,
    appState: appState,
    outputDir: outputDir,
    declaration: const WorkDeclaration(
      workSourceDeclaration: 'human_created',
      trainingPermissionDeclaration: 'prohibited',
      creationMethodDeclaration: 'field_recording',
      humanEditLevelDeclaration: 'none',
      authenticityClaimDeclaration: 'captured',
      customRightsStatement: 'Android Batch 2 audio page QA',
    ),
  );
  steps.add(
    _QaStep(
      id: 'audioWrite',
      label: '音频写入',
      pass: audio.pass,
      detail: '${audio.watermarkUid} · ${audio.writeMs}ms',
    ),
  );

  steps.add(
    _QaStep(
      id: 'protectedCopyShare',
      label: '保护副本分享',
      pass: await File(image.protectedPath).exists(),
      detail: image.protectedFileName,
    ),
  );

  final vaultDetailPass =
      appState.records.any((record) => record.id == image.record.id) &&
      image.record.payloadProtocolVersion == 3 &&
      image.record.payloadBytesLength == 39 &&
      image.record.protectedCopyName?.isNotEmpty == true &&
      image.record.protectedCopyHash?.isNotEmpty == true;
  steps.add(
    _QaStep(
      id: 'vaultDetail',
      label: '版权库详情',
      pass: vaultDetailPass,
      detail:
          'V${image.record.payloadProtocolVersion}/${image.record.payloadBytesLength} · ${image.record.watermarkIdRegistryStatus}',
    ),
  );

  final report = await appState.buildFormalReportDraft(image.record);
  final reportPath = '${outputDir.path}/android-batch2-formal-report-$runId.md';
  await File(reportPath).writeAsString(report.markdown, flush: true);
  final reportPrivacyPass = _formalReportPrivacyPass(
    markdown: report.markdown,
    outputDir: outputDir.path,
    protectedPath: image.protectedPath,
  );
  steps.add(
    _QaStep(
      id: 'formalReportDraft',
      label: '报告草稿',
      pass: reportPrivacyPass && report.markdown.contains(image.watermarkUid),
      detail: report.reportId,
    ),
  );

  final l2Record = await appState.createL2VideoFingerprintNotaryFromBytes(
    bytes: _makeTinyMp4LikeBytes(runId),
    fileName: 'android-batch2-l2-metadata-$runId.mp4',
    durationMs: 36000,
    width: 1280,
    height: 720,
    frameCount: 1080,
  );
  final l2PrivacyPass =
      l2Record.videoBundleSha256?.startsWith('sha256:') == true &&
      l2Record.customRightsStatement?.contains('no raw video') == true &&
      l2Record.videoNotaryId?.isNotEmpty == true;
  steps.add(
    _QaStep(
      id: 'l2MetadataNotary',
      label: 'L2 metadata notary',
      pass: l2PrivacyPass,
      detail: l2Record.videoNotaryId ?? 'missing',
    ),
  );

  await appState.syncPendingQueue();
  final metadata = await appState.fetchPublicRightsMetadata(image.watermarkUid);
  final metadataPath =
      '${outputDir.path}/android-batch2-public-metadata-$runId.json';
  await File(metadataPath).writeAsString(
    '${const JsonEncoder.withIndent('  ').convert(metadata)}\n',
    flush: true,
  );
  final publicMetadataPass =
      metadata['watermarkUid'] == image.watermarkUid &&
      metadata['legalConclusion'] == false &&
      metadata['manifestHash']?.toString().trim().isNotEmpty == true;
  steps.add(
    _QaStep(
      id: 'publicMetadataExportEntry',
      label: '公开元数据入口',
      pass: publicMetadataPass,
      detail: metadata['manifestHash']?.toString() ?? 'missing',
    ),
  );

  final backendOff = await _runBackendOffMatureErrorCheck(appState.syncProfile);
  steps.add(
    _QaStep(
      id: 'backendOffMatureError',
      label: '关闭后端错误',
      pass: backendOff.privacyPass && backendOff.message.contains('暂时无法连接服务'),
      detail: backendOff.message,
    ),
  );

  return _QaArtifact(
    runId: runId,
    generatedAt: DateTime.now().toUtc().toIso8601String(),
    backendBaseUrl: _backendUrl,
    outputDir: outputDir.path,
    readyPath: '${outputDir.path}/android-batch2-ready-$runId.txt',
    resultPath: '${outputDir.path}/android-batch2-page-qa-$runId.json',
    image: image,
    audio: audio,
    l2Record: l2Record,
    report: report,
    reportPath: reportPath,
    formalReportPrivacyPass: reportPrivacyPass,
    publicMetadata: metadata,
    publicMetadataJsonPath: metadataPath,
    backendOff: backendOff,
    vaultRecordCount: appState.records.length,
    steps: steps,
  );
}

Future<_WriteCaseResult> _writeKindCase({
  required String runId,
  required WatermarkAssetKind kind,
  required RustWatermarkBridge bridge,
  required MobileAppState appState,
  required Directory outputDir,
  required WorkDeclaration declaration,
}) async {
  final sourceBytes = kind == WatermarkAssetKind.image
      ? _makePpmImage()
      : _makeWavAudio(seconds: 31);
  final originalHash = appState.sha256HexForBytes(sourceBytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: kind,
    originalHash: originalHash,
    revision: 1,
  );
  final startedAt = DateTime.now();
  final write = await bridge.write(
    WatermarkWriteRequest(
      kind: kind,
      bytes: sourceBytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: appState.creatorLabel,
        deviceIdentity: 'android-batch2-page-qa',
        mediaBytes: sourceBytes,
        timestamp: DateTime.now().microsecondsSinceEpoch,
      ),
      allowRewrite: true,
      registryDraft: reserved?.toDraft(),
    ),
  );
  final confirmed = await appState.confirmWatermarkIdForWrite(
    result: write,
    originalHash: originalHash,
    reserved: reserved,
  );
  final extension = kind == WatermarkAssetKind.image ? 'png' : 'wav';
  final protectedFileName =
      'android-batch2-${kind.name}-protected-$runId.$extension';
  final protectedPath = '${outputDir.path}/$protectedFileName';
  await File(protectedPath).writeAsBytes(write.bytes, flush: true);
  final withArtifact = write.copyWithOutputArtifact(
    outputFileName: protectedFileName,
    outputLocationLabel: 'Android QA sandbox',
    outputActionLabel: '保存或分享保护副本',
  );
  final record = appState.addWriteResult(
    result: withArtifact,
    fileName: 'android-batch2-${kind.name}-source-$runId.$extension',
    allowRewrite: false,
    registryResult: confirmed,
    declaration: declaration,
  );
  return _WriteCaseResult(
    kind: kind,
    record: record,
    write: withArtifact,
    protectedPath: protectedPath,
    protectedFileName: protectedFileName,
    writeMs: DateTime.now().difference(startedAt).inMilliseconds,
  );
}

Future<_BackendOffResult> _runBackendOffMatureErrorCheck(
  SyncProfile sourceProfile,
) async {
  final store = MemoryVaultStore();
  await store.saveSyncProfile(
    sourceProfile.copyWith(
      cloudBaseUrl: 'http://127.0.0.1:9',
      clearLastError: true,
    ),
  );
  final downState = MobileAppState(
    vaultStore: store,
    cloudAccountClient: CloudAccountClient(
      baseUrl: 'http://127.0.0.1:9',
      timeout: const Duration(milliseconds: 900),
    ),
  );
  await downState.load();
  await downState.refreshCloudDevices();
  final message =
      downState.syncProfile.lastError ??
      mobileUserFacingErrorMessage(
        const CloudAccountException('connection refused'),
        action: '读取设备列表',
      );
  final forbidden = RegExp(
    r'(127\.0\.0\.1|10\.0\.2\.2|:\d{2,5}|SocketException|ClientException|stack|panic)',
    caseSensitive: false,
  ).hasMatch(message);
  return _BackendOffResult(message: message, privacyPass: !forbidden);
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

Future<void> _waitForBackendHealth() async {
  final client = http.Client();
  try {
    final deadline = DateTime.now().add(const Duration(seconds: 60));
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

bool _formalReportPrivacyPass({
  required String markdown,
  required String outputDir,
  required String protectedPath,
}) {
  final forbidden = [
    outputDir,
    protectedPath,
    '/data/',
    r'\data\',
    'mediaBytes',
    'originalPath',
    'protectedCopyPath',
    'signedUrl',
    'objectRef',
  ];
  return forbidden.every((item) => !markdown.contains(item));
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

Uint8List _makeWavAudio({required int seconds}) {
  const sampleRate = 44100;
  const channels = 1;
  const bitsPerSample = 16;
  final sampleCount = sampleRate * seconds;
  final dataBytes = sampleCount * channels * (bitsPerSample ~/ 8);
  final bytes = ByteData(44 + dataBytes);
  void asciiAt(int offset, String value) {
    for (var i = 0; i < value.length; i++) {
      bytes.setUint8(offset + i, value.codeUnitAt(i));
    }
  }

  asciiAt(0, 'RIFF');
  bytes.setUint32(4, 36 + dataBytes, Endian.little);
  asciiAt(8, 'WAVE');
  asciiAt(12, 'fmt ');
  bytes.setUint32(16, 16, Endian.little);
  bytes.setUint16(20, 1, Endian.little);
  bytes.setUint16(22, channels, Endian.little);
  bytes.setUint32(24, sampleRate, Endian.little);
  bytes.setUint32(
    28,
    sampleRate * channels * (bitsPerSample ~/ 8),
    Endian.little,
  );
  bytes.setUint16(32, channels * (bitsPerSample ~/ 8), Endian.little);
  bytes.setUint16(34, bitsPerSample, Endian.little);
  asciiAt(36, 'data');
  bytes.setUint32(40, dataBytes, Endian.little);
  for (var i = 0; i < sampleCount; i++) {
    final sample = (math.sin(2 * math.pi * 440 * i / sampleRate) * 12000)
        .round();
    bytes.setInt16(44 + i * 2, sample, Endian.little);
  }
  return bytes.buffer.asUint8List();
}

Uint8List _makeTinyMp4LikeBytes(String runId) {
  final body = utf8.encode('HiddenShield Android Batch 2 L2 metadata $runId');
  final bytes = BytesBuilder();
  bytes.add([0, 0, 0, 24]);
  bytes.add(ascii.encode('ftypisom'));
  bytes.add(List<int>.filled(12, 0));
  bytes.add([0, 0, 0, body.length + 8]);
  bytes.add(ascii.encode('mdat'));
  bytes.add(body);
  return bytes.toBytes();
}

class _QaArtifact {
  const _QaArtifact({
    required this.runId,
    required this.generatedAt,
    required this.backendBaseUrl,
    required this.outputDir,
    required this.readyPath,
    required this.resultPath,
    required this.image,
    required this.audio,
    required this.l2Record,
    required this.report,
    required this.reportPath,
    required this.formalReportPrivacyPass,
    required this.publicMetadata,
    required this.publicMetadataJsonPath,
    required this.backendOff,
    required this.vaultRecordCount,
    required this.steps,
  });

  final String runId;
  final String generatedAt;
  final String backendBaseUrl;
  final String outputDir;
  final String readyPath;
  final String resultPath;
  final _WriteCaseResult image;
  final _WriteCaseResult audio;
  final VaultRecord l2Record;
  final FormalReportDraft report;
  final String reportPath;
  final bool formalReportPrivacyPass;
  final Map<String, Object?> publicMetadata;
  final String publicMetadataJsonPath;
  final _BackendOffResult backendOff;
  final int vaultRecordCount;
  final List<_QaStep> steps;

  bool get ok => steps.every((step) => step.pass);
  String get status => ok ? 'ready' : 'blocked';
  String get imageWatermarkUid => image.watermarkUid;
  String get audioWatermarkUid => audio.watermarkUid;
  int get imageWriteMs => image.writeMs;
  int get audioWriteMs => audio.writeMs;
  String get imagePayloadLabel => image.payloadLabel;
  String get audioPayloadLabel => audio.payloadLabel;
  String get imageProtectedPath => image.protectedPath;
  String get imageProtectedFileName => image.protectedFileName;
  String get imageRegistryStatus => image.record.watermarkIdRegistryStatus;
  String get formalReportId => report.reportId;
  String get l2NotaryId => l2Record.videoNotaryId ?? '';
  String get l2FingerprintRoot => l2Record.videoFingerprintRoot ?? '';
  bool get l2PrivacyPass =>
      l2Record.customRightsStatement?.contains('no raw video') == true;
  String get publicManifestHash =>
      publicMetadata['manifestHash']?.toString() ?? '';
  Object? get publicLegalConclusion => publicMetadata['legalConclusion'];
  String get publicMetadataJsonFileName =>
      publicMetadataJsonPath.split('/').where((part) => part.isNotEmpty).last;
  String get backendOffMessage => backendOff.message;
  bool get backendOffPrivacyPass => backendOff.privacyPass;

  Map<String, Object?> toJson() => {
    'schemaVersion': 'android_batch2_page_qa_v1',
    'runId': runId,
    'generatedAt': generatedAt,
    'ok': ok,
    'status': status,
    'backendBaseUrl': backendBaseUrl,
    'outputDir': outputDir,
    'readyPath': readyPath,
    'resultPath': resultPath,
    'orderedSteps': steps.map((step) => step.toJson()).toList(growable: false),
    'imageWrite': image.toJson(),
    'audioWrite': audio.toJson(),
    'protectedCopyShare': {
      'entryRendered': true,
      'protectedPath': image.protectedPath,
      'protectedFileName': image.protectedFileName,
    },
    'vaultDetail': {
      'recordCount': vaultRecordCount,
      'imageRecord': _recordSummary(image.record),
      'audioRecord': _recordSummary(audio.record),
    },
    'formalReportDraft': {
      'reportId': report.reportId,
      'reportPath': reportPath,
      'recordCount': report.recordCount,
      'markdownBytes': utf8.encode(report.markdown).length,
      'privacyPass': formalReportPrivacyPass,
    },
    'l2MetadataNotary': {
      'record': _recordSummary(l2Record),
      'notaryId': l2Record.videoNotaryId,
      'fingerprintRoot': l2Record.videoFingerprintRoot,
      'bundleSha256': l2Record.videoBundleSha256,
      'bundleBytes': l2Record.videoBundleBytes,
      'sceneCount': l2Record.videoBundleSceneCount,
      'elapsedMs': l2Record.videoBundleElapsedMs,
      'privacyPass': l2PrivacyPass,
    },
    'publicMetadataExportEntry': {
      'jsonPath': publicMetadataJsonPath,
      'watermarkUid': publicMetadata['watermarkUid'],
      'manifestHash': publicMetadata['manifestHash'],
      'legalConclusion': publicMetadata['legalConclusion'],
      'entryRendered': true,
    },
    'backendOffMatureError': backendOff.toJson(),
  };
}

Map<String, Object?> _recordSummary(VaultRecord record) => {
  'id': record.id,
  'kind': record.kind.name,
  'title': record.title,
  'watermarkUid': record.watermarkUid,
  'revision': record.revision,
  'syncStatus': record.syncStatus.name,
  'writeVerificationStatus': record.writeVerificationStatus?.name,
  'payloadProtocolVersion': record.payloadProtocolVersion,
  'payloadBytesLength': record.payloadBytesLength,
  'watermarkIdIssueMode': record.watermarkIdIssueMode,
  'watermarkIdRegistryStatus': record.watermarkIdRegistryStatus,
  'payloadAuthStatus': record.payloadAuthStatus,
  'protectedCopyName': record.protectedCopyName,
  'protectedCopyHash': record.protectedCopyHash,
  'outputStrategy': record.outputStrategy,
};

class _WriteCaseResult {
  const _WriteCaseResult({
    required this.kind,
    required this.record,
    required this.write,
    required this.protectedPath,
    required this.protectedFileName,
    required this.writeMs,
  });

  final WatermarkAssetKind kind;
  final VaultRecord record;
  final WatermarkWriteResult write;
  final String protectedPath;
  final String protectedFileName;
  final int writeMs;

  String get watermarkUid => record.watermarkUid;
  String get payloadLabel =>
      'V${record.payloadProtocolVersion} / ${record.payloadBytesLength} bytes';
  bool get pass =>
      write.verification.verified &&
      record.writeVerificationStatus == WriteVerificationStatus.verified &&
      record.payloadProtocolVersion == 3 &&
      record.payloadBytesLength == 39 &&
      record.watermarkIdRegistryStatus == 'server_confirmed';

  Map<String, Object?> toJson() => {
    'kind': kind.name,
    'watermarkUid': watermarkUid,
    'revision': record.revision,
    'writeMs': writeMs,
    'bridgeProcessTimeMs': write.processTimeMs,
    'payloadProtocolVersion': record.payloadProtocolVersion,
    'payloadBytesLength': record.payloadBytesLength,
    'writeVerificationStatus': record.writeVerificationStatus?.name,
    'watermarkIdIssueMode': record.watermarkIdIssueMode,
    'watermarkIdRegistryStatus': record.watermarkIdRegistryStatus,
    'payloadAuthStatus': record.payloadAuthStatus,
    'protectedPath': protectedPath,
    'protectedFileName': protectedFileName,
    'protectedBytes': write.bytes.length,
    'protectedCopyHash': record.protectedCopyHash,
    'pass': pass,
  };
}

class _BackendOffResult {
  const _BackendOffResult({required this.message, required this.privacyPass});

  final String message;
  final bool privacyPass;

  Map<String, Object?> toJson() => {
    'mode': 'closed_local_endpoint_no_shared_backend_shutdown',
    'message': message,
    'privacyPass': privacyPass,
  };
}

class _QaStep {
  const _QaStep({
    required this.id,
    required this.label,
    required this.pass,
    required this.detail,
  });

  final String id;
  final String label;
  final bool pass;
  final String detail;

  Map<String, Object?> toJson() => {
    'id': id,
    'label': label,
    'pass': pass,
    'detail': detail,
  };
}
