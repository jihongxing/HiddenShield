import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/tool_cards.dart';
import 'rewrite_preflight.dart';

class AudioEmbedPage extends StatefulWidget {
  const AudioEmbedPage({
    super.key,
    required this.bridge,
    required this.appState,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  State<AudioEmbedPage> createState() => _AudioEmbedPageState();
}

class _AudioEmbedPageState extends State<AudioEmbedPage> {
  static const int _minimumProtectionSeconds = 30;

  Uint8List? _selectedBytes;
  String? _fileName;
  double? _selectedDurationSeconds;
  bool _allowRewrite = false;
  bool _isProcessing = false;
  bool _isInspecting = false;
  WatermarkWriteResult? _result;
  VaultRecord? _savedRecord;
  RewritePreflightResult? _preflight;
  String? _errorText;
  int _preflightRequestId = 0;

  @override
  Widget build(BuildContext context) {
    final selectedBytes = _selectedBytes;
    final durationSeconds = _selectedDurationSeconds;
    final isTooShort =
        durationSeconds != null && durationSeconds < _minimumProtectionSeconds;
    return Scaffold(
      appBar: AppBar(title: const Text('保护音频')),
      body: SafeArea(
        child: ListView(
          cacheExtent: 1000,
          padding: const EdgeInsets.all(16),
          children: [
            HsPanel(
              title: '导入 WAV',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _AudioPreview(
                    bytes: selectedBytes,
                    fileName: _fileName,
                    durationSeconds: durationSeconds,
                  ),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    onPressed: _isProcessing ? null : _pickAudio,
                    icon: const Icon(Icons.upload_file_outlined),
                    label: Text(selectedBytes == null ? '选择 WAV' : '重新选择'),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            HsPanel(
              title: '保护设置',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SwitchListTile(
                    value: _allowRewrite,
                    onChanged: _isProcessing
                        ? null
                        : (value) => setState(() => _allowRewrite = value),
                    title: const Text('作为新版写入'),
                    subtitle: const Text('默认关闭。开启后会保留上一版记录，并生成新的写入次数。'),
                    contentPadding: EdgeInsets.zero,
                  ),
                  const SizedBox(height: 8),
                  const HsMessageCard(
                    icon: Icons.verified_outlined,
                    title: '音频取证优先',
                    detail: '支持 30 秒以上 WAV 作品。完成前会回读验证版权编号。',
                  ),
                  if (isTooShort) ...[
                    const SizedBox(height: 8),
                    const HsMessageCard(
                      icon: Icons.info_outline,
                      title: '音频时长不足',
                      detail: '当前音频短于 30 秒，暂不生成版权保护副本。请选择完整作品或更长片段。',
                    ),
                  ],
                  const SizedBox(height: 8),
                  _PreflightStatusCard(
                    isInspecting: _isInspecting,
                    result: _preflight,
                  ),
                ],
              ),
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: selectedBytes == null || _isProcessing || isTooShort
                  ? null
                  : _embedAudio,
              icon: _isProcessing
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.shield_outlined),
              label: Text(_isProcessing ? '正在处理' : '生成保护副本'),
            ),
            if (_errorText != null) ...[
              const SizedBox(height: 12),
              HsMessageCard(
                icon: Icons.error_outline,
                title: '处理失败',
                detail: _errorText!,
              ),
            ],
            if (_result != null) ...[
              const SizedBox(height: 12),
              _ResultCard(result: _result!, record: _savedRecord),
            ],
          ],
        ),
      ),
    );
  }

  Future<void> _pickAudio() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: const ['wav'],
      withData: true,
    );
    final file = result?.files.single;
    final bytes = file?.bytes;
    if (file == null || bytes == null) {
      return;
    }

    setState(() {
      _selectedBytes = bytes;
      _fileName = file.name;
      _selectedDurationSeconds = _wavDurationSeconds(bytes);
      _result = null;
      _savedRecord = null;
      _preflight = null;
      _errorText = null;
    });
    await _inspectSelected(bytes);
  }

  Future<void> _embedAudio() async {
    final bytes = _selectedBytes;
    if (bytes == null) {
      return;
    }

    setState(() {
      _isProcessing = true;
      _errorText = null;
      _result = null;
      _savedRecord = null;
    });

    try {
      final parent = _allowRewrite
          ? (_preflight?.readResult ?? await _readParentWatermark(bytes))
          : null;
      final result = await widget.bridge.write(
        WatermarkWriteRequest(
          kind: WatermarkAssetKind.audio,
          bytes: bytes,
          seed: _buildPayloadSeed(bytes, widget.appState.creatorLabel),
          allowRewrite: _allowRewrite,
          rewriteReason: _allowRewrite ? '移动端确认重写已有水印' : null,
        ),
      );
      if (!mounted) return;
      final revision = _allowRewrite
          ? (_preflight?.hasWatermark == true
                ? _preflight!.nextRevision
                : parent == null
                ? result.revision
                : parent.revision + 1)
          : result.revision;
      final record = widget.appState.addWriteResult(
        result: result,
        fileName: _fileName,
        allowRewrite: _allowRewrite,
        rewriteReason: _allowRewrite ? '移动端确认重写已有水印' : null,
        parentWatermarkUid: parent?.watermarkUid,
        revision: revision,
      );
      setState(() {
        _result = result;
        _savedRecord = record;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() => _errorText = error.toString());
    } finally {
      if (mounted) {
        setState(() => _isProcessing = false);
      }
    }
  }

  Future<WatermarkReadResult?> _readParentWatermark(List<int> bytes) async {
    if (!_allowRewrite) {
      return null;
    }
    try {
      return await widget.bridge.read(
        WatermarkReadRequest(kind: WatermarkAssetKind.audio, bytes: bytes),
      );
    } catch (_) {
      return null;
    }
  }

  Future<void> _inspectSelected(List<int> bytes) async {
    final requestId = ++_preflightRequestId;
    setState(() => _isInspecting = true);
    final result = await inspectMobileRewriteTarget(
      bridge: widget.bridge,
      appState: widget.appState,
      kind: WatermarkAssetKind.audio,
      bytes: bytes,
    );
    if (!mounted || requestId != _preflightRequestId) {
      return;
    }
    setState(() {
      _preflight = result;
      _isInspecting = false;
    });
  }
}

class _AudioPreview extends StatelessWidget {
  const _AudioPreview({
    required this.bytes,
    required this.fileName,
    required this.durationSeconds,
  });

  final Uint8List? bytes;
  final String? fileName;
  final double? durationSeconds;

  @override
  Widget build(BuildContext context) {
    final sizeText = bytes == null
        ? '选择 30 秒以上 WAV 音频，生成保护副本和版权记录。'
        : [
            '${(bytes!.length / 1024).toStringAsFixed(1)} KB',
            if (durationSeconds != null)
              '${durationSeconds!.toStringAsFixed(1)} 秒',
          ].join(' / ');
    return HsPreviewBox(
      height: 160,
      child: Row(
        children: [
          const Icon(
            Icons.graphic_eq_outlined,
            size: 42,
            color: HsColors.iconMuted,
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  fileName ?? '未选择音频',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(
                  sizeText,
                  style: const TextStyle(color: HsColors.textMuted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result, required this.record});

  final WatermarkWriteResult result;
  final VaultRecord? record;

  @override
  Widget build(BuildContext context) {
    final shaPreview = result.sha256.length > 16
        ? '${result.sha256.substring(0, 16)}...'
        : result.sha256;
    final savedRecord = record;
    final revision = savedRecord?.revision ?? result.revision;
    final parent = savedRecord?.parentWatermarkUid;
    return HsMessageCard(
      icon: Icons.verified_outlined,
      title: '写入完成',
      detail: [
        '版权编号: ${result.watermarkUid}',
        '写入次数: 第 $revision 次',
        if (parent != null) '上一版本: $parent',
        result.verification.message,
        '作品指纹: $shaPreview',
      ].join('\n'),
    );
  }
}

WatermarkPayloadSeed _buildPayloadSeed(List<int> bytes, String creatorLabel) {
  final creatorDigest = sha256.convert(creatorLabel.trim().codeUnits).bytes;
  final fileDigest = sha256.convert(bytes).bytes;
  return WatermarkPayloadSeed(
    userSeed: creatorDigest.take(8).toList(growable: false),
    timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
    deviceId: creatorDigest.skip(8).take(4).toList(growable: false),
    fileHash: fileDigest.take(2).toList(growable: false),
  );
}

double? _wavDurationSeconds(Uint8List bytes) {
  if (bytes.length < 44) {
    return null;
  }
  final data = ByteData.sublistView(bytes);
  final riff = String.fromCharCodes(bytes.sublist(0, 4));
  final wave = String.fromCharCodes(bytes.sublist(8, 12));
  if (riff != 'RIFF' || wave != 'WAVE') {
    return null;
  }

  int? channels;
  int? sampleRate;
  int? bitsPerSample;
  int? dataSize;
  var offset = 12;

  while (offset + 8 <= bytes.length) {
    final chunkId = String.fromCharCodes(bytes.sublist(offset, offset + 4));
    final chunkSize = data.getUint32(offset + 4, Endian.little);
    final chunkDataOffset = offset + 8;
    if (chunkDataOffset + chunkSize > bytes.length) {
      break;
    }

    if (chunkId == 'fmt ' && chunkSize >= 16) {
      channels = data.getUint16(chunkDataOffset + 2, Endian.little);
      sampleRate = data.getUint32(chunkDataOffset + 4, Endian.little);
      bitsPerSample = data.getUint16(chunkDataOffset + 14, Endian.little);
    } else if (chunkId == 'data') {
      dataSize = chunkSize;
      break;
    }

    offset = chunkDataOffset + chunkSize + (chunkSize.isOdd ? 1 : 0);
  }

  final resolvedChannels = channels;
  final resolvedSampleRate = sampleRate;
  final resolvedBitsPerSample = bitsPerSample;
  final resolvedDataSize = dataSize;
  if (resolvedChannels == null ||
      resolvedSampleRate == null ||
      resolvedBitsPerSample == null ||
      resolvedDataSize == null ||
      resolvedChannels <= 0 ||
      resolvedSampleRate <= 0 ||
      resolvedBitsPerSample <= 0) {
    return null;
  }

  final bytesPerSample = resolvedBitsPerSample / 8;
  final bytesPerSecond =
      resolvedSampleRate * resolvedChannels * bytesPerSample;
  if (bytesPerSecond <= 0) {
    return null;
  }
  return resolvedDataSize / bytesPerSecond;
}

class _PreflightStatusCard extends StatelessWidget {
  const _PreflightStatusCard({
    required this.isInspecting,
    required this.result,
  });

  final bool isInspecting;
  final RewritePreflightResult? result;

  @override
  Widget build(BuildContext context) {
    if (isInspecting) {
      return const HsMessageCard(
        icon: Icons.search_outlined,
        title: '写入检查',
        detail: '正在检查是否已有版权记录...',
      );
    }
    final result = this.result;
    if (result == null) {
      return const HsMessageCard(
        icon: Icons.info_outline,
        title: '写入检查',
        detail: '选择 WAV 后会自动检查是否已有版权记录。',
      );
    }
    final detail = [
      result.reasonDetail,
      if (result.watermarkUid != null) '上一版本: ${result.watermarkUid}',
      if (result.detectedRevision != null)
        '当前识别为第 ${result.detectedRevision} 次写入',
    ].join('\n');
    return HsMessageCard(
      icon: result.hasWatermark
          ? Icons.warning_amber_outlined
          : Icons.check_circle_outline,
      title: result.summary,
      detail: detail,
    );
  }
}
