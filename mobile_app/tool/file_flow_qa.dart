import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';

const _runId = String.fromEnvironment('HIDDENSHIELD_FILE_FLOW_RUN_ID');
const _desktopImagePath = String.fromEnvironment(
  'HIDDENSHIELD_QA_DESKTOP_IMAGE_PATH',
);
const _desktopAudioPath = String.fromEnvironment(
  'HIDDENSHIELD_QA_DESKTOP_AUDIO_PATH',
);
const _desktopImageUid = String.fromEnvironment(
  'HIDDENSHIELD_QA_DESKTOP_IMAGE_UID',
);
const _desktopAudioUid = String.fromEnvironment(
  'HIDDENSHIELD_QA_DESKTOP_AUDIO_UID',
);
const _outputDir = String.fromEnvironment('HIDDENSHIELD_QA_OUTPUT_DIR');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _FileFlowQaApp());
}

class _FileFlowQaApp extends StatefulWidget {
  const _FileFlowQaApp();

  @override
  State<_FileFlowQaApp> createState() => _FileFlowQaAppState();
}

class _FileFlowQaAppState extends State<_FileFlowQaApp> {
  _FileFlowResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      final result = await _runFileFlowQa();
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
      title: 'HiddenShield File Flow QA',
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
          Text('正在执行真实保护副本双端文件流转 QA'),
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
          Text('文件流转 QA 失败', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 12),
          Text('$error'),
        ],
      ),
    );
  }
}

class _ResultView extends StatelessWidget {
  const _ResultView({required this.result});

  final _FileFlowResult result;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.compare_arrows_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'HiddenShield 真实保护副本文件流转 QA',
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
        ...result.rows.map((row) => _FlowCard(row: row)),
        const SizedBox(height: 12),
        Text(
          '解密项：当前图片 / 音频保护副本没有额外加密 envelope，本轮按读取与验证当前默认媒体 payload 闭环验收。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _FlowCard extends StatelessWidget {
  const _FlowCard({required this.row});

  final _FlowRow row;

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
                    '${row.direction} · ${row.kindLabel}',
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
            _Detail(label: '读取编号', value: row.extractedUid),
            _Detail(label: 'Payload', value: row.payloadLabel),
            _Detail(label: '版本次数', value: '第 ${row.revision} 次'),
            if (row.parentWatermarkUid != null)
              _Detail(label: '上一版', value: row.parentWatermarkUid!),
            _Detail(label: '签发模式', value: row.issueModeLabel),
            _Detail(label: '认证状态', value: row.payloadAuthStatus),
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

Future<_FileFlowResult> _runFileFlowQa() async {
  if (_runId.isEmpty ||
      _desktopImagePath.isEmpty ||
      _desktopAudioPath.isEmpty ||
      _desktopImageUid.isEmpty ||
      _desktopAudioUid.isEmpty ||
      _outputDir.isEmpty) {
    throw StateError('缺少 file-flow QA dart-define 参数。');
  }
  final outputDir = Directory(_outputDir);
  await outputDir.create(recursive: true);
  final bridge = RustWatermarkBridge();
  await RustWatermarkBridge.init();

  final desktopImageRow = await _readExpected(
    bridge: bridge,
    direction: 'desktop -> mobile',
    kind: WatermarkAssetKind.image,
    path: _desktopImagePath,
    expectedUid: _desktopImageUid,
  );
  final desktopAudioRow = await _readExpected(
    bridge: bridge,
    direction: 'desktop -> mobile',
    kind: WatermarkAssetKind.audio,
    path: _desktopAudioPath,
    expectedUid: _desktopAudioUid,
  );
  final mobileImageRow = await _writeMobileArtifact(
    bridge: bridge,
    kind: WatermarkAssetKind.image,
    outputPath: '${outputDir.path}/mobile-protected-image-$_runId.png',
  );
  final mobileAudioRow = await _writeMobileArtifact(
    bridge: bridge,
    kind: WatermarkAssetKind.audio,
    outputPath: '${outputDir.path}/mobile-protected-audio-$_runId.wav',
  );
  final result = _FileFlowResult(
    runId: _runId,
    resultPath: '${outputDir.path}/mobile-file-flow-result.json',
    rows: [desktopImageRow, desktopAudioRow, mobileImageRow, mobileAudioRow],
  );
  await File(result.resultPath).writeAsString(result.toJsonString());
  return result;
}

Future<_FlowRow> _readExpected({
  required RustWatermarkBridge bridge,
  required String direction,
  required WatermarkAssetKind kind,
  required String path,
  required String expectedUid,
}) async {
  final bytes = await File(path).readAsBytes();
  final extracted = await bridge.read(
    WatermarkReadRequest(kind: kind, bytes: bytes),
  );
  if (extracted == null) {
    throw StateError('未从 $path 读取到水印。');
  }
  return _FlowRow(
    direction: direction,
    kind: kind.name,
    path: path,
    expectedUid: expectedUid,
    extractedUid: extracted.watermarkUid,
    parentWatermarkUid: extracted.parentWatermarkUid,
    revision: extracted.revision,
    payloadProtocolVersion: extracted.payloadProtocolVersion,
    payloadBytesLength: extracted.payloadBytesLength,
    watermarkIdIssueMode: extracted.watermarkIdIssueMode,
    payloadAuthStatus: extracted.payloadAuthStatus,
    mediaType: extracted.mediaType ?? kind.name,
  );
}

Future<_FlowRow> _writeMobileArtifact({
  required RustWatermarkBridge bridge,
  required WatermarkAssetKind kind,
  required String outputPath,
}) async {
  final bytes = kind == WatermarkAssetKind.image
      ? _makePpmImage()
      : _makeWavAudio(seconds: 31);
  final result = await bridge.write(
    WatermarkWriteRequest(
      kind: kind,
      bytes: bytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: '移动端文件流转 QA',
        deviceIdentity: 'android-file-flow-qa',
        mediaBytes: bytes,
        timestamp: DateTime.now().microsecondsSinceEpoch,
      ),
      allowRewrite: true,
    ),
  );
  await File(outputPath).writeAsBytes(result.bytes, flush: true);
  final extracted = await bridge.read(
    WatermarkReadRequest(kind: kind, bytes: result.bytes),
  );
  return _FlowRow(
    direction: 'mobile -> desktop',
    kind: kind.name,
    path: outputPath,
    expectedUid: result.watermarkUid,
    extractedUid: extracted?.watermarkUid ?? '未读取',
    parentWatermarkUid: extracted?.parentWatermarkUid,
    revision: extracted?.revision ?? result.revision,
    payloadProtocolVersion: extracted?.payloadProtocolVersion ?? 2,
    payloadBytesLength: extracted?.payloadBytesLength ?? 119,
    watermarkIdIssueMode:
        extracted?.watermarkIdIssueMode ?? 'offline_generated',
    payloadAuthStatus: extracted?.payloadAuthStatus ?? 'unknown',
    mediaType: extracted?.mediaType ?? kind.name,
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

class _FileFlowResult {
  const _FileFlowResult({
    required this.runId,
    required this.resultPath,
    required this.rows,
  });

  final String runId;
  final String resultPath;
  final List<_FlowRow> rows;

  String toJsonString() => const JsonEncoder.withIndent('  ').convert({
    'runId': runId,
    'resultPath': resultPath,
    'rows': rows.map((row) => row.toJson()).toList(),
  });
}

class _FlowRow {
  const _FlowRow({
    required this.direction,
    required this.kind,
    required this.path,
    required this.expectedUid,
    required this.extractedUid,
    required this.parentWatermarkUid,
    required this.revision,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    required this.watermarkIdIssueMode,
    required this.payloadAuthStatus,
    required this.mediaType,
  });

  final String direction;
  final String kind;
  final String path;
  final String expectedUid;
  final String extractedUid;
  final String? parentWatermarkUid;
  final int revision;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String watermarkIdIssueMode;
  final String payloadAuthStatus;
  final String mediaType;

  bool get pass => expectedUid == extractedUid;

  String get kindLabel => kind == 'image' ? '图片保护副本' : '音频保护副本';
  String get payloadLabel =>
      'V$payloadProtocolVersion / $payloadBytesLength bytes';
  String get issueModeLabel => switch (watermarkIdIssueMode) {
    'server_reserved' => '后端预签发',
    'server_confirmed' => '后端已确认',
    'server_reissued' => '后端重签发',
    'offline_generated' => '离线生成',
    _ => watermarkIdIssueMode,
  };

  Map<String, Object?> toJson() => {
    'direction': direction,
    'kind': kind,
    'path': path,
    'expectedUid': expectedUid,
    'extractedUid': extractedUid,
    'parentWatermarkUid': parentWatermarkUid,
    'revision': revision,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'watermarkIdIssueMode': watermarkIdIssueMode,
    'payloadAuthStatus': payloadAuthStatus,
    'mediaType': mediaType,
    'pass': pass,
  };
}
