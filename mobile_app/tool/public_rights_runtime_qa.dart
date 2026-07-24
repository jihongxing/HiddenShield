import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_QA_BACKEND_URL',
  defaultValue: 'http://10.0.2.2:43188',
);

const _runId = String.fromEnvironment(
  'HIDDENSHIELD_QA_RUN_ID',
  defaultValue: '',
);

bool _rustBridgeInitialized = false;

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _PublicRightsRuntimeQaApp());
}

class _PublicRightsRuntimeQaApp extends StatefulWidget {
  const _PublicRightsRuntimeQaApp();

  @override
  State<_PublicRightsRuntimeQaApp> createState() =>
      _PublicRightsRuntimeQaAppState();
}

class _PublicRightsRuntimeQaAppState extends State<_PublicRightsRuntimeQaApp> {
  _PublicRightsRuntimeQaResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      _PublicRightsRuntimeQaResult? result;
      Object? lastError;
      for (var attempt = 1; attempt <= 3; attempt++) {
        try {
          await _waitForBackendHealth();
          result = await _runQa();
          break;
        } catch (error) {
          lastError = error;
          debugPrint(
            'HIDDENSHIELD_PUBLIC_RIGHTS_QA_ATTEMPT_FAILED $attempt $error',
          );
          await Future<void>.delayed(Duration(seconds: attempt * 2));
        }
      }
      if (result == null) {
        throw StateError(
          'public rights runtime QA failed after retries: $lastError',
        );
      }
      final passed = result.rows.every((row) => row.pass);
      debugPrint(
        'HIDDENSHIELD_PUBLIC_RIGHTS_QA_RESULT ${jsonEncode(result.toJson())}',
      );
      if (!passed) {
        throw StateError('public rights runtime QA assertions failed');
      }
      if (mounted) setState(() => _result = result);
    } catch (error) {
      debugPrint('HIDDENSHIELD_PUBLIC_RIGHTS_QA_ERROR $error');
      if (mounted) setState(() => _error = error);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Public Rights Runtime QA',
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
          Text('正在执行公开权利信号移动端运行态 QA'),
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

  final _PublicRightsRuntimeQaResult result;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.verified_user_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'HiddenShield 移动端公开权利信号 QA',
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
        ...result.rows.map((row) => _RightsCard(row: row)),
        const SizedBox(height: 12),
        Text(
          '证据来自 Android 原生 Flutter 运行态、真实 Rust watermark bridge、真实 feedback-backend reserve / confirm / sync / public rights API。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _RightsCard extends StatelessWidget {
  const _RightsCard({required this.row});

  final _RightsQaRow row;

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
                Icon(
                  row.mediaKind == WatermarkAssetKind.image
                      ? Icons.image_outlined
                      : Icons.graphic_eq_outlined,
                  size: 20,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${_kindLabel(row.mediaKind)} · ${row.fileName}',
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
            _Detail(label: '版权编号', value: row.watermarkUid),
            _Detail(label: '本地训练许可', value: _trainingLabel(row.localTraining)),
            _Detail(label: '公开训练许可', value: row.publicTrainingLabel),
            _Detail(label: '扫描状态', value: _scanStatusLabel(row.scanStatus)),
            _Detail(
              label: '锚点协议',
              value: _anchorProtocolLabel(row.anchorProtocol),
            ),
            _Detail(label: 'Manifest', value: 'v${row.manifestVersion}'),
            _Detail(label: '法律结论', value: row.legalConclusion ? '是' : '否'),
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
            width: 96,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(child: Text(value, overflow: TextOverflow.visible)),
        ],
      ),
    );
  }
}

Future<_PublicRightsRuntimeQaResult> _runQa() async {
  final runId = _runId.isEmpty
      ? DateTime.now().millisecondsSinceEpoch.toString()
      : _runId;
  if (!_rustBridgeInitialized) {
    await RustWatermarkBridge.init();
    _rustBridgeInitialized = true;
  }
  final bridge = RustWatermarkBridge();
  final cloudClient = CloudAccountClient(baseUrl: _backendUrl);
  final appState = MobileAppState(
    vaultStore: MemoryVaultStore(),
    cloudAccountClient: cloudClient,
  );
  await appState.load();
  final password = 'rights-mobile-$runId';
  await appState.completeOnboarding(
    accountLabel: 'mobile-public-rights-$runId@hiddenshield.local',
    password: password,
    creatorLabel: '移动端公开权利 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);

  final rows = <_RightsQaRow>[];
  rows.add(
    await _runKindCase(
      runId,
      WatermarkAssetKind.image,
      bridge,
      appState,
      trainingPermission: 'commercial_allowed',
      workSource: 'ai_assisted',
    ),
  );
  rows.add(
    await _runKindCase(
      runId,
      WatermarkAssetKind.audio,
      bridge,
      appState,
      trainingPermission: 'prohibited',
      workSource: 'human_created',
    ),
  );
  return _PublicRightsRuntimeQaResult(runId: runId, rows: rows);
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

Future<_RightsQaRow> _runKindCase(
  String runId,
  WatermarkAssetKind kind,
  RustWatermarkBridge bridge,
  MobileAppState appState, {
  required String trainingPermission,
  required String workSource,
}) async {
  final bytes = kind == WatermarkAssetKind.image
      ? _makePpmImage()
      : _makeWavAudio(seconds: 31);
  final originalHash = appState.sha256HexForBytes(bytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: kind,
    originalHash: originalHash,
    revision: 1,
  );
  final result = await bridge.write(
    WatermarkWriteRequest(
      kind: kind,
      bytes: bytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: appState.creatorLabel,
        deviceIdentity: 'mobile-public-rights-runtime-qa',
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
  final record = appState.addWriteResult(
    result: result,
    fileName: 'mobile-${kind.name}-public-rights-$runId',
    allowRewrite: false,
    registryResult: confirmed,
    declaration: WorkDeclaration(
      workSourceDeclaration: workSource,
      trainingPermissionDeclaration: trainingPermission,
      creationMethodDeclaration: kind == WatermarkAssetKind.image
          ? 'text_to_image'
          : 'audio_generation',
      humanEditLevelDeclaration: 'light',
      authenticityClaimDeclaration: 'synthetic',
      customRightsStatement: 'mobile public rights runtime QA',
    ),
  );

  await appState.syncPendingQueue();
  final rights = await appState.fetchPublicRights(record.watermarkUid);
  final expectedPolicy = _expectedPublicTrainingPolicy(trainingPermission);
  final manifestVersion = rights.rightsManifest?.manifestVersion ?? 0;
  final pass =
      rights.scanStatus == 'registry_active' &&
      rights.trainingPermission.policy == expectedPolicy &&
      rights.trainingPermission.legalConclusion == false &&
      manifestVersion >= 1 &&
      rights.registry.anchorProtocol == 'v2_migration_anchor';
  return _RightsQaRow(
    mediaKind: kind,
    fileName: record.title,
    watermarkUid: record.watermarkUid,
    localTraining: trainingPermission,
    publicTrainingPolicy: rights.trainingPermission.policy,
    publicTrainingLabel: rights.trainingPermission.label,
    scanStatus: rights.scanStatus,
    anchorProtocol: rights.registry.anchorProtocol,
    manifestVersion: manifestVersion,
    legalConclusion: rights.trainingPermission.legalConclusion,
    pass: pass,
  );
}

String _expectedPublicTrainingPolicy(String local) {
  return switch (local) {
    'commercial_allowed' => 'commercial_training_allowed',
    'non_commercial_allowed' => 'non_commercial_research_allowed',
    'separate_authorization_required' => 'separate_license_required',
    _ => 'no_ai_training',
  };
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
    final sample = (sin(2 * pi * 440 * i / sampleRate) * 12000).round();
    bytes.setInt16(44 + i * 2, sample, Endian.little);
  }
  return bytes.buffer.asUint8List();
}

class _PublicRightsRuntimeQaResult {
  const _PublicRightsRuntimeQaResult({required this.runId, required this.rows});

  final String runId;
  final List<_RightsQaRow> rows;

  Map<String, Object?> toJson() => {
    'runId': runId,
    'rows': rows.map((row) => row.toJson()).toList(),
    'passed': rows.every((row) => row.pass),
  };
}

class _RightsQaRow {
  const _RightsQaRow({
    required this.mediaKind,
    required this.fileName,
    required this.watermarkUid,
    required this.localTraining,
    required this.publicTrainingPolicy,
    required this.publicTrainingLabel,
    required this.scanStatus,
    required this.anchorProtocol,
    required this.manifestVersion,
    required this.legalConclusion,
    required this.pass,
  });

  final WatermarkAssetKind mediaKind;
  final String fileName;
  final String watermarkUid;
  final String localTraining;
  final String publicTrainingPolicy;
  final String publicTrainingLabel;
  final String scanStatus;
  final String anchorProtocol;
  final int manifestVersion;
  final bool legalConclusion;
  final bool pass;

  Map<String, Object?> toJson() => {
    'mediaKind': mediaKind.name,
    'fileName': fileName,
    'watermarkUid': watermarkUid,
    'localTraining': localTraining,
    'publicTrainingPolicy': publicTrainingPolicy,
    'publicTrainingLabel': publicTrainingLabel,
    'scanStatus': scanStatus,
    'anchorProtocol': anchorProtocol,
    'manifestVersion': manifestVersion,
    'legalConclusion': legalConclusion,
    'pass': pass,
  };
}

String _kindLabel(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => '图片写入',
    WatermarkAssetKind.audio => '音频写入',
    WatermarkAssetKind.video => '视频写入',
  };
}

String _trainingLabel(String value) {
  return switch (value) {
    'commercial_allowed' => '允许商业训练',
    'non_commercial_allowed' => '允许非商业训练',
    'separate_authorization_required' => '需单独授权',
    'prohibited' => '禁止模型训练',
    _ => value,
  };
}

String _scanStatusLabel(String value) {
  return switch (value) {
    'registry_active' => 'registry 已生效',
    'watermark_only' => '仅识别到水印锚点',
    'backfill_disputed' => '需要人工复核',
    _ => value,
  };
}

String _anchorProtocolLabel(String value) {
  return switch (value) {
    'v2_migration_anchor' => 'V2 迁移桥接锚点',
    'v3_minimal_anchor' => 'V3 最小媒体锚点',
    _ => value,
  };
}
