import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import 'mobile_verify_reason.dart';

class VerifyPage extends StatefulWidget {
  const VerifyPage({super.key, required this.bridge, required this.appState});

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  State<VerifyPage> createState() => _VerifyPageState();
}

class _VerifyPageState extends State<VerifyPage> {
  WatermarkAssetKind _kind = WatermarkAssetKind.image;
  Uint8List? _selectedBytes;
  String? _fileName;
  bool _isProcessing = false;
  WatermarkReadResult? _result;
  MobileVerifyReason? _reason;
  String? _errorText;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '取证',
      subtitle: '检查文件是否保留隐盾版权记录',
      children: [
        HsPanel(
          title: '检测文件',
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SegmentedButton<WatermarkAssetKind>(
                segments: const [
                  ButtonSegment(
                    value: WatermarkAssetKind.image,
                    icon: Icon(Icons.image_outlined),
                    label: Text('图片'),
                  ),
                  ButtonSegment(
                    value: WatermarkAssetKind.audio,
                    icon: Icon(Icons.graphic_eq_outlined),
                    label: Text('WAV'),
                  ),
                ],
                selected: {_kind},
                onSelectionChanged: _isProcessing
                    ? null
                    : (value) => setState(() {
                        _kind = value.single;
                        _selectedBytes = null;
                        _fileName = null;
                        _result = null;
                        _errorText = null;
                      }),
              ),
              const SizedBox(height: 12),
              _SelectedFileSummary(
                kind: _kind,
                bytes: _selectedBytes,
                fileName: _fileName,
              ),
              const SizedBox(height: 12),
              FilledButton.icon(
                onPressed: _isProcessing ? null : _pickFile,
                icon: const Icon(Icons.upload_file_outlined),
                label: Text(_selectedBytes == null ? '选择文件' : '重新选择'),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        FilledButton.icon(
          onPressed: _selectedBytes == null || _isProcessing ? null : _verify,
          icon: _isProcessing
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.document_scanner_outlined),
          label: Text(_isProcessing ? '正在检测' : '开始检测'),
        ),
        if (_errorText != null) ...[
          const SizedBox(height: 12),
          HsMessageCard(
            icon: Icons.error_outline,
            title: '未检测到记录',
            detail: _errorText!,
            detailWidget: _MessageDetail(detail: _errorText!, reason: _reason),
          ),
        ],
        if (_result != null) ...[
          const SizedBox(height: 12),
          _ResultCard(result: _result!, reason: _reason),
        ],
      ],
    );
  }

  Future<void> _pickFile() async {
    final result = await FilePicker.pickFiles(
      type: _kind == WatermarkAssetKind.image
          ? FileType.image
          : FileType.custom,
      allowedExtensions: _kind == WatermarkAssetKind.audio ? ['wav'] : null,
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
      _result = null;
      _reason = null;
      _errorText = null;
    });
  }

  Future<void> _verify() async {
    final bytes = _selectedBytes;
    if (bytes == null) {
      return;
    }

    setState(() {
      _isProcessing = true;
      _result = null;
      _reason = null;
      _errorText = null;
    });

    try {
      final result = await widget.bridge.read(
        WatermarkReadRequest(kind: _kind, bytes: bytes),
      );
      if (!mounted) return;
      if (result != null) {
        widget.appState.addReadResult(result: result, fileName: _fileName);
      }
      setState(() {
        _result = result;
        _reason = result == null
            ? MobileVerifyReason.noWatermark()
            : MobileVerifyReason.forSuccess(result);
        _errorText = result == null ? '没有检测到有效隐盾水印。' : null;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _reason = MobileVerifyReason.forError(error.toString());
        _errorText = '提取过程未完成。';
      });
    } finally {
      if (mounted) {
        setState(() => _isProcessing = false);
      }
    }
  }
}

class _SelectedFileSummary extends StatelessWidget {
  const _SelectedFileSummary({
    required this.kind,
    required this.bytes,
    required this.fileName,
  });

  final WatermarkAssetKind kind;
  final Uint8List? bytes;
  final String? fileName;

  @override
  Widget build(BuildContext context) {
    final emptyText = kind == WatermarkAssetKind.image
        ? '选择疑似侵权图片，检查是否保留版权记录。'
        : '选择疑似侵权 WAV，检查是否保留版权记录。';
    final detail = bytes == null
        ? emptyText
        : '${(bytes!.length / 1024).toStringAsFixed(1)} KB';
    return HsPreviewBox(
      height: 150,
      child: Row(
        children: [
          Icon(
            kind == WatermarkAssetKind.image
                ? Icons.image_search_outlined
                : Icons.graphic_eq_outlined,
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
                  fileName ?? '未选择文件',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(detail, style: const TextStyle(color: HsColors.textMuted)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result, required this.reason});

  final WatermarkReadResult result;
  final MobileVerifyReason? reason;

  @override
  Widget build(BuildContext context) {
    return HsMessageCard(
      icon: Icons.fact_check_outlined,
      title: '检测到版权记录',
      detail: [
        '版权编号: ${result.watermarkUid}',
        '写入次数: 第 ${result.revision} 次',
        if (result.parentWatermarkUid != null)
          '上一版本: ${result.parentWatermarkUid}',
        if (result.rewriteReason != null) '重写原因: ${result.rewriteReason}',
        '作品指纹: ${result.fileHashHex}',
      ].join('\n'),
      detailWidget: _MessageDetail(
        detail: [
          '版权编号: ${result.watermarkUid}',
          '写入次数: 第 ${result.revision} 次',
          if (result.parentWatermarkUid != null)
            '上一版本: ${result.parentWatermarkUid}',
          if (result.rewriteReason != null) '重写原因: ${result.rewriteReason}',
          '作品指纹: ${result.fileHashHex}',
        ].join('\n'),
        reason: reason,
      ),
    );
  }
}

class _MessageDetail extends StatelessWidget {
  const _MessageDetail({required this.detail, this.reason});

  final String detail;
  final MobileVerifyReason? reason;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(detail),
        if (reason != null) ...[
          const SizedBox(height: HsSpacing.sm),
          Text(
            reason!.detail,
            style: const TextStyle(color: HsColors.textMuted, fontSize: 12),
          ),
        ],
      ],
    );
  }
}
