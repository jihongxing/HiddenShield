import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/src/rust/api.dart' as rust_api;

const _runId = String.fromEnvironment('HIDDENSHIELD_V3_INTERNAL_QA_RUN_ID');
const _imageSourcePath = String.fromEnvironment(
  'HIDDENSHIELD_V3_INTERNAL_QA_IMAGE_SOURCE_PATH',
);
const _audioSourcePath = String.fromEnvironment(
  'HIDDENSHIELD_V3_INTERNAL_QA_AUDIO_SOURCE_PATH',
);
const _imageV3Uid = String.fromEnvironment(
  'HIDDENSHIELD_V3_INTERNAL_QA_IMAGE_UID',
);
const _audioV3Uid = String.fromEnvironment(
  'HIDDENSHIELD_V3_INTERNAL_QA_AUDIO_UID',
);
const _outputDir = String.fromEnvironment(
  'HIDDENSHIELD_V3_INTERNAL_QA_OUTPUT_DIR',
);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _V3InternalQaWriteApp());
}

class _V3InternalQaWriteApp extends StatefulWidget {
  const _V3InternalQaWriteApp();

  @override
  State<_V3InternalQaWriteApp> createState() => _V3InternalQaWriteAppState();
}

class _V3InternalQaWriteAppState extends State<_V3InternalQaWriteApp> {
  _QaResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      final result = await _runQa();
      if (mounted) setState(() => _result = result);
    } catch (error) {
      if (mounted) setState(() => _error = error);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield V3 Internal QA Write',
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
          Text('正在执行 V3 internal_qa 写入 Android 原生 QA'),
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
          Text(
            'V3 internal_qa 写入 QA 失败',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 12),
          Text('$error'),
        ],
      ),
    );
  }
}

class _ResultView extends StatelessWidget {
  const _ResultView({required this.result});

  final _QaResult result;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.fact_check_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'HiddenShield V3 internal_qa 写入运行态 QA',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 4),
                  Text('Run ID ${result.runId} · Android 原生运行态'),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        ...result.rows.map((row) => _ResultCard(row: row)),
        const SizedBox(height: 12),
        Text(
          '边界：internal_qa 与默认 write() 均应生成 V3/39；V2 只允许 force_v2_rollback 门禁验证。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.row});

  final _QaRow row;

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
                  row.kind == 'image'
                      ? Icons.image_outlined
                      : Icons.graphic_eq_outlined,
                  size: 20,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${row.writePath} · ${row.kindLabel}',
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
            _Detail(label: 'Payload', value: row.payloadLabel),
            _Detail(label: '认证状态', value: row.payloadAuthStatus),
            _Detail(label: '签发模式', value: row.watermarkIdIssueMode),
            _Detail(label: '载荷角色', value: row.mediaPayloadRole),
            _Detail(label: '默认路径', value: row.defaultWritePathStatus),
            _Detail(label: '文件', value: row.path),
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
            width: 86,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(child: Text(value, overflow: TextOverflow.visible)),
        ],
      ),
    );
  }
}

Future<_QaResult> _runQa() async {
  if (_runId.isEmpty ||
      _imageSourcePath.isEmpty ||
      _audioSourcePath.isEmpty ||
      _imageV3Uid.isEmpty ||
      _audioV3Uid.isEmpty ||
      _outputDir.isEmpty) {
    throw StateError('缺少 V3 internal_qa QA dart-define 参数。');
  }
  final outputDir = Directory(_outputDir);
  await outputDir.create(recursive: true);
  final bridge = RustWatermarkBridge();
  await RustWatermarkBridge.init();

  final imageSource = await File(_imageSourcePath).readAsBytes();
  final audioSource = await File(_audioSourcePath).readAsBytes();
  final rows = <_QaRow>[
    await _writeInternalQaV3(
      kind: WatermarkAssetKind.image,
      source: imageSource,
      watermarkUid: _imageV3Uid,
      outputPath: '${outputDir.path}/android-internal-qa-v3-image.png',
    ),
    await _writeInternalQaV3(
      kind: WatermarkAssetKind.audio,
      source: audioSource,
      watermarkUid: _audioV3Uid,
      outputPath: '${outputDir.path}/android-internal-qa-v3-audio.wav',
    ),
    await _writeDefaultV3(
      bridge: bridge,
      kind: WatermarkAssetKind.image,
      source: imageSource,
      outputPath: '${outputDir.path}/android-default-v3-image.png',
    ),
    await _writeDefaultV3(
      bridge: bridge,
      kind: WatermarkAssetKind.audio,
      source: audioSource,
      outputPath: '${outputDir.path}/android-default-v3-audio.wav',
    ),
  ];
  final result = _QaResult(
    runId: _runId,
    resultPath: '${outputDir.path}/android-v3-internal-qa-write-result.json',
    rows: rows,
  );
  await File(result.resultPath).writeAsString(result.toJsonString());
  return result;
}

Future<_QaRow> _writeInternalQaV3({
  required WatermarkAssetKind kind,
  required List<int> source,
  required String watermarkUid,
  required String outputPath,
}) async {
  final result = await rust_api.embedV3InternalQaForMobile(
    mediaBytes: source,
    mediaType: kind.name,
    watermarkUid: watermarkUid,
  );
  await File(outputPath).writeAsBytes(result.bytes);
  final extracted = await _readReadonly(kind: kind, bytes: result.bytes);
  return _QaRow(
    bridge: 'android_native',
    writePath: 'internal_qa',
    kind: kind.name,
    path: outputPath,
    watermarkUid: extracted.watermarkUid,
    payloadProtocolVersion: extracted.payloadProtocolVersion,
    payloadBytesLength: extracted.payloadBytesLength,
    payloadAuthStatus: extracted.payloadAuthStatus,
    watermarkIdIssueMode: extracted.watermarkIdIssueMode,
    mediaPayloadRole: 'v3_minimal_anchor',
    defaultWritePathStatus: 'not_used_internal_qa_only',
  );
}

Future<_QaRow> _writeDefaultV3({
  required RustWatermarkBridge bridge,
  required WatermarkAssetKind kind,
  required List<int> source,
  required String outputPath,
}) async {
  final result = await bridge.write(
    WatermarkWriteRequest(
      kind: kind,
      bytes: source,
      seed: WatermarkPayloadSeed(
        creatorIdentity: 'android-v3-internal-qa-runtime',
        deviceIdentity: 'android-native-qa',
        mediaBytes: source,
        timestamp: 1786147200,
      ),
      allowRewrite: true,
    ),
  );
  await File(outputPath).writeAsBytes(result.bytes);
  final extracted = await bridge.read(
    WatermarkReadRequest(kind: kind, bytes: result.bytes),
  );
  if (extracted == null) {
    throw StateError('默认 write() 后无法读回 ${kind.name}。');
  }
  return _QaRow(
    bridge: 'android_native',
    writePath: 'default_write',
    kind: kind.name,
    path: outputPath,
    watermarkUid: extracted.watermarkUid,
    payloadProtocolVersion: extracted.payloadProtocolVersion,
    payloadBytesLength: extracted.payloadBytesLength,
    payloadAuthStatus: extracted.payloadAuthStatus,
    watermarkIdIssueMode: extracted.watermarkIdIssueMode,
    mediaPayloadRole: 'v3_minimal_anchor',
    defaultWritePathStatus: 'v3_minimal_anchor_verified',
  );
}

Future<WatermarkReadResult> _readReadonly({
  required WatermarkAssetKind kind,
  required List<int> bytes,
}) async {
  final bridge = RustWatermarkBridge();
  final extracted = await bridge.readReadonlyCandidate(
    WatermarkReadRequest(kind: kind, bytes: bytes),
  );
  if (extracted == null) {
    throw StateError('internal_qa V3 ${kind.name} 未被 readonly candidate 读回。');
  }
  return extracted;
}

class _QaResult {
  const _QaResult({
    required this.runId,
    required this.resultPath,
    required this.rows,
  });

  final String runId;
  final String resultPath;
  final List<_QaRow> rows;

  bool get pass => rows.every((row) => row.pass);

  String toJsonString() => const JsonEncoder.withIndent('  ').convert({
    'runId': runId,
    'resultPath': resultPath,
    'rows': rows.map((row) => row.toJson()).toList(),
    'defaultV3WriteEnabled': true,
    'defaultMobileWriteV3Enabled': true,
    'v3InternalQaWriteGate': 'internal_qa',
    'pass': pass,
  });
}

class _QaRow {
  const _QaRow({
    required this.bridge,
    required this.writePath,
    required this.kind,
    required this.path,
    required this.watermarkUid,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    required this.payloadAuthStatus,
    required this.watermarkIdIssueMode,
    required this.mediaPayloadRole,
    required this.defaultWritePathStatus,
  });

  final String bridge;
  final String writePath;
  final String kind;
  final String path;
  final String watermarkUid;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String payloadAuthStatus;
  final String watermarkIdIssueMode;
  final String mediaPayloadRole;
  final String defaultWritePathStatus;

  bool get pass {
    if (writePath == 'internal_qa') {
      return payloadProtocolVersion == 3 &&
          payloadBytesLength == 39 &&
          payloadAuthStatus == 'verified' &&
          watermarkIdIssueMode == 'registry_resolved' &&
          mediaPayloadRole == 'v3_minimal_anchor' &&
          defaultWritePathStatus == 'not_used_internal_qa_only';
    }
    return writePath == 'default_write' &&
        payloadProtocolVersion == 3 &&
        payloadBytesLength == 39 &&
        payloadAuthStatus == 'verified' &&
        mediaPayloadRole == 'v3_minimal_anchor' &&
        defaultWritePathStatus == 'v3_minimal_anchor_verified';
  }

  String get kindLabel => kind == 'image' ? '图片真实媒体' : '音频真实媒体';
  String get payloadLabel =>
      'V$payloadProtocolVersion / $payloadBytesLength bytes';

  Map<String, Object?> toJson() => {
    'bridge': bridge,
    'writePath': writePath,
    'kind': kind,
    'path': path,
    'watermarkUid': watermarkUid,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'payloadAuthStatus': payloadAuthStatus,
    'watermarkIdIssueMode': watermarkIdIssueMode,
    'mediaPayloadRole': mediaPayloadRole,
    'defaultWritePathStatus': defaultWritePathStatus,
    'pass': pass,
  };
}
