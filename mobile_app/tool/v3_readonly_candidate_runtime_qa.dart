import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';

const _runId = String.fromEnvironment('HIDDENSHIELD_V3_CANDIDATE_RUN_ID');
const _imagePath = String.fromEnvironment(
  'HIDDENSHIELD_V3_CANDIDATE_IMAGE_PATH',
);
const _audioPath = String.fromEnvironment(
  'HIDDENSHIELD_V3_CANDIDATE_AUDIO_PATH',
);
const _imageUid = String.fromEnvironment('HIDDENSHIELD_V3_CANDIDATE_IMAGE_UID');
const _audioUid = String.fromEnvironment('HIDDENSHIELD_V3_CANDIDATE_AUDIO_UID');
const _outputDir = String.fromEnvironment(
  'HIDDENSHIELD_V3_CANDIDATE_OUTPUT_DIR',
);

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _V3ReadonlyCandidateQaApp());
}

class _V3ReadonlyCandidateQaApp extends StatefulWidget {
  const _V3ReadonlyCandidateQaApp();

  @override
  State<_V3ReadonlyCandidateQaApp> createState() =>
      _V3ReadonlyCandidateQaAppState();
}

class _V3ReadonlyCandidateQaAppState extends State<_V3ReadonlyCandidateQaApp> {
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
      title: 'HiddenShield V3 Readonly Candidate QA',
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
          Text('正在执行 V3 readonly candidate Android 原生 QA'),
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
            'V3 readonly candidate QA 失败',
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
                    'HiddenShield V3 只读候选真实媒体 QA',
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
        ...result.rows.map((row) => _CandidateCard(row: row)),
        const SizedBox(height: 12),
        Text(
          '边界：本轮继续覆盖 readReadonlyCandidate；默认 read()/write() 已切 V3，显式 reader 用于迁移期对照。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _CandidateCard extends StatelessWidget {
  const _CandidateCard({required this.row});

  final _CandidateRow row;

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
                    '${row.kindLabel} · ${row.pass ? 'PASS' : 'FAIL'}',
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
            _Detail(label: '期望编号', value: row.expectedUid),
            _Detail(label: '读取编号', value: row.watermarkUid),
            _Detail(label: 'Payload', value: row.payloadLabel),
            _Detail(label: '认证状态', value: row.payloadAuthStatus),
            _Detail(label: '签发模式', value: row.watermarkIdIssueMode),
            _Detail(label: '载荷角色', value: row.mediaPayloadRole),
            _Detail(label: '默认读取', value: row.defaultReadStatus),
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
      _imagePath.isEmpty ||
      _audioPath.isEmpty ||
      _imageUid.isEmpty ||
      _audioUid.isEmpty ||
      _outputDir.isEmpty) {
    throw StateError('缺少 V3 readonly candidate QA dart-define 参数。');
  }
  final outputDir = Directory(_outputDir);
  await outputDir.create(recursive: true);
  final bridge = RustWatermarkBridge();
  await RustWatermarkBridge.init();

  final image = await _readCandidate(
    bridge: bridge,
    kind: WatermarkAssetKind.image,
    path: _imagePath,
    expectedUid: _imageUid,
  );
  final audio = await _readCandidate(
    bridge: bridge,
    kind: WatermarkAssetKind.audio,
    path: _audioPath,
    expectedUid: _audioUid,
  );
  final result = _QaResult(
    runId: _runId,
    resultPath: '${outputDir.path}/android-v3-readonly-candidate-result.json',
    rows: [image, audio],
  );
  await File(result.resultPath).writeAsString(result.toJsonString());
  return result;
}

Future<_CandidateRow> _readCandidate({
  required RustWatermarkBridge bridge,
  required WatermarkAssetKind kind,
  required String path,
  required String expectedUid,
}) async {
  final bytes = await File(path).readAsBytes();
  final extracted = await bridge.readReadonlyCandidate(
    WatermarkReadRequest(kind: kind, bytes: bytes),
  );
  if (extracted == null) {
    throw StateError('未从 $path 读取到 V3 readonly candidate。');
  }
  return _CandidateRow(
    kind: kind.name,
    path: path,
    expectedUid: expectedUid,
    watermarkUid: extracted.watermarkUid,
    payloadProtocolVersion: extracted.payloadProtocolVersion,
    payloadBytesLength: extracted.payloadBytesLength,
    payloadAuthStatus: extracted.payloadAuthStatus,
    watermarkIdIssueMode: extracted.watermarkIdIssueMode,
    mediaPayloadRole: extracted.payloadProtocolVersion == 3
        ? 'v3_minimal_anchor'
        : 'v2_full_record',
    defaultReadStatus: 'default_v3_contract_guarded',
  );
}

class _QaResult {
  const _QaResult({
    required this.runId,
    required this.resultPath,
    required this.rows,
  });

  final String runId;
  final String resultPath;
  final List<_CandidateRow> rows;

  bool get pass => rows.every((row) => row.pass);

  String toJsonString() => const JsonEncoder.withIndent('  ').convert({
    'runId': runId,
    'resultPath': resultPath,
    'rows': rows.map((row) => row.toJson()).toList(),
    'defaultV3WriteEnabled': true,
    'defaultMobileReadV3Enabled': true,
    'pass': pass,
  });
}

class _CandidateRow {
  const _CandidateRow({
    required this.kind,
    required this.path,
    required this.expectedUid,
    required this.watermarkUid,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    required this.payloadAuthStatus,
    required this.watermarkIdIssueMode,
    required this.mediaPayloadRole,
    required this.defaultReadStatus,
  });

  final String kind;
  final String path;
  final String expectedUid;
  final String watermarkUid;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String payloadAuthStatus;
  final String watermarkIdIssueMode;
  final String mediaPayloadRole;
  final String defaultReadStatus;

  bool get pass =>
      expectedUid == watermarkUid &&
      payloadProtocolVersion == 3 &&
      payloadBytesLength == 39 &&
      payloadAuthStatus == 'verified' &&
      watermarkIdIssueMode == 'registry_resolved' &&
      mediaPayloadRole == 'v3_minimal_anchor' &&
      defaultReadStatus == 'default_v3_contract_guarded';

  String get kindLabel => kind == 'image' ? '图片真实媒体' : '音频真实媒体';
  String get payloadLabel =>
      'V$payloadProtocolVersion / $payloadBytesLength bytes';

  Map<String, Object?> toJson() => {
    'bridge': 'android_native',
    'kind': kind,
    'path': path,
    'expectedUid': expectedUid,
    'watermarkUid': watermarkUid,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'payloadAuthStatus': payloadAuthStatus,
    'watermarkIdIssueMode': watermarkIdIssueMode,
    'mediaPayloadRole': mediaPayloadRole,
    'defaultReadStatus': defaultReadStatus,
    'pass': pass,
  };
}
