import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
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

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _RuntimeQaApp());
}

class _RuntimeQaApp extends StatefulWidget {
  const _RuntimeQaApp();

  @override
  State<_RuntimeQaApp> createState() => _RuntimeQaAppState();
}

class _RuntimeQaAppState extends State<_RuntimeQaApp> {
  _RuntimeQaResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      final result = await _runRealRuntimeQa();
      if (mounted) {
        setState(() => _result = result);
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Real Runtime QA',
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
          Text('正在执行原生移动端真实写入 QA'),
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

  final _RuntimeQaResult result;

  @override
  Widget build(BuildContext context) {
    final rows = result.cases.expand((item) => item.rows).toList();
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.verified_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'HiddenShield 移动端真实运行态 QA',
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
        ...rows.map((row) => _StatusCard(row: row)),
        const SizedBox(height: 12),
        Text(
          '证据来自 Android 原生 Flutter 运行态、真实 Rust watermark bridge、真实 feedback-backend reserve / confirm / reconcile API。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({required this.row});

  final _RuntimeQaRow row;

  @override
  Widget build(BuildContext context) {
    final ok = row.pass;
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
                    '${_kindLabel(row.mediaKind)} · ${row.workflow}',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                Chip(
                  label: Text(ok ? 'PASS' : 'FAIL'),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            const SizedBox(height: 12),
            _Detail(label: '版权编号', value: row.watermarkUid),
            _Detail(label: '编号签发模式', value: _issueModeLabel(row.issueMode)),
            _Detail(
              label: '登记状态',
              value: _registryStatusLabel(row.registryStatus),
            ),
            _Detail(
              label: 'Payload',
              value:
                  'V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes',
            ),
            _Detail(
              label: '父编号 / 版本',
              value: '${row.parentWatermarkUid ?? '无'} / 第 ${row.revision} 次',
            ),
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

Future<_RuntimeQaResult> _runRealRuntimeQa() async {
  final runId = DateTime.now().millisecondsSinceEpoch.toString();
  final bridge = RustWatermarkBridge();
  await RustWatermarkBridge.init();
  final cloudClient = CloudAccountClient(baseUrl: _backendUrl);
  final appState = MobileAppState(
    vaultStore: MemoryVaultStore(),
    cloudAccountClient: cloudClient,
  );
  await appState.load();
  final password = 'qa-$runId';
  await appState.completeOnboarding(
    accountLabel: 'mobile-real-runtime-$runId@hiddenshield.local',
    password: password,
    creatorLabel: '移动端 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);

  final cases = <_RuntimeQaCase>[];
  for (final kind in [WatermarkAssetKind.image, WatermarkAssetKind.audio]) {
    cases.add(await _runKindCase(runId, kind, bridge, appState));
  }
  return _RuntimeQaResult(runId: runId, cases: cases);
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

Future<_RuntimeQaCase> _runKindCase(
  String runId,
  WatermarkAssetKind kind,
  RustWatermarkBridge bridge,
  MobileAppState appState,
) async {
  final bytes = kind == WatermarkAssetKind.image
      ? _makePpmImage()
      : _makeWavAudio(seconds: 31);
  final originalHash = appState.sha256HexForBytes(bytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: kind,
    originalHash: originalHash,
    revision: 1,
  );
  final onlineResult = await _write(kind, bytes, bridge, appState, reserved);
  final confirmed = await appState.confirmWatermarkIdForWrite(
    result: onlineResult,
    originalHash: originalHash,
    reserved: reserved,
  );
  final onlineRecord = appState.addWriteResult(
    result: onlineResult,
    fileName: 'mobile-${kind.name}-server-$runId',
    allowRewrite: false,
    registryResult: confirmed,
  );

  final offlineResult = await _write(kind, bytes, bridge, appState, null);
  final offlineRecord = appState.addWriteResult(
    result: offlineResult,
    fileName: 'mobile-${kind.name}-offline-$runId',
    allowRewrite: false,
    registryResult: null,
  );
  final pendingRow = _RuntimeQaRow.fromRecord(
    kind: kind,
    workflow: '后端不可用时离线生成，仅本地待登记',
    expectedStatus: 'pending_registration',
    record: offlineRecord,
  );

  await Future<void>.delayed(const Duration(milliseconds: 300));
  await appState.syncPendingQueue();
  final reconciledRecord = appState.records.firstWhere(
    (record) => record.id == offlineRecord.id,
  );

  return _RuntimeQaCase(
    kind: kind,
    rows: [
      _RuntimeQaRow.fromRecord(
        kind: kind,
        workflow: '在线 reserve -> confirm',
        expectedStatus: 'server_confirmed',
        record: onlineRecord,
      ),
      pendingRow,
      _RuntimeQaRow.fromRecord(
        kind: kind,
        workflow: '云同步前 reconcile 后补登记',
        expectedStatus: 'offline_confirmed',
        record: reconciledRecord,
      ),
    ],
  );
}

Future<WatermarkWriteResult> _write(
  WatermarkAssetKind kind,
  List<int> bytes,
  RustWatermarkBridge bridge,
  MobileAppState appState,
  WatermarkIdRegistryResult? registry,
) {
  final now = DateTime.now().microsecondsSinceEpoch;
  return bridge.write(
    WatermarkWriteRequest(
      kind: kind,
      bytes: bytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: appState.creatorLabel,
        deviceIdentity: 'mobile-runtime-qa',
        mediaBytes: bytes,
        timestamp: now,
      ),
      allowRewrite: true,
      registryDraft: registry?.toDraft(),
    ),
  );
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

class _RuntimeQaResult {
  const _RuntimeQaResult({required this.runId, required this.cases});

  final String runId;
  final List<_RuntimeQaCase> cases;
}

class _RuntimeQaCase {
  const _RuntimeQaCase({required this.kind, required this.rows});

  final WatermarkAssetKind kind;
  final List<_RuntimeQaRow> rows;
}

class _RuntimeQaRow {
  const _RuntimeQaRow({
    required this.mediaKind,
    required this.workflow,
    required this.expectedStatus,
    required this.watermarkUid,
    required this.issueMode,
    required this.registryStatus,
    required this.parentWatermarkUid,
    required this.revision,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
  });

  factory _RuntimeQaRow.fromRecord({
    required WatermarkAssetKind kind,
    required String workflow,
    required String expectedStatus,
    required VaultRecord record,
  }) {
    return _RuntimeQaRow(
      mediaKind: kind,
      workflow: workflow,
      expectedStatus: expectedStatus,
      watermarkUid: record.watermarkUid,
      issueMode: record.watermarkIdIssueMode,
      registryStatus: record.watermarkIdRegistryStatus,
      parentWatermarkUid: record.parentWatermarkUid,
      revision: record.revision,
      payloadProtocolVersion: record.payloadProtocolVersion,
      payloadBytesLength: record.payloadBytesLength,
    );
  }

  final WatermarkAssetKind mediaKind;
  final String workflow;
  final String expectedStatus;
  final String watermarkUid;
  final String issueMode;
  final String registryStatus;
  final String? parentWatermarkUid;
  final int revision;
  final int payloadProtocolVersion;
  final int payloadBytesLength;

  bool get pass => registryStatus == expectedStatus;
}

String _kindLabel(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => '图片写入',
    WatermarkAssetKind.audio => '音频写入',
    WatermarkAssetKind.video => '视频写入',
  };
}

String _issueModeLabel(String value) {
  return switch (value) {
    'server_reserved' => '后端预签发',
    'server_confirmed' => '后端已确认',
    'server_reissued' => '后端重新签发',
    'offline_generated' => '离线高熵生成',
    _ => value,
  };
}

String _registryStatusLabel(String value) {
  return switch (value) {
    'reserved' => '已预留，等待写入确认',
    'server_confirmed' => '后端已确认',
    'offline_confirmed' => '离线编号已补登记',
    'pending_registration' => '待联网登记',
    _ => value,
  };
}
