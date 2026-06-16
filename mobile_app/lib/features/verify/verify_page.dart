import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/widgets/feature_page_scaffold.dart';

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
  String? _errorText;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '取证',
      subtitle: '检测疑似侵权图片或 WAV 音频，展示命中和链路',
      bridge: widget.bridge,
      children: [
        _SectionCard(
          title: '文件提取',
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
          label: Text(_isProcessing ? '正在提取' : '提取水印'),
        ),
        if (_errorText != null) ...[
          const SizedBox(height: 12),
          _MessageCard(
            icon: Icons.error_outline,
            title: '未能提取',
            detail: _errorText!,
          ),
        ],
        if (_result != null) ...[
          const SizedBox(height: 12),
          _ResultCard(result: _result!),
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
        _errorText = result == null ? '没有检测到有效隐盾水印。' : null;
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
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: const Color(0xFF141B22),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
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
        ? '选择疑似侵权图片，检测是否含有隐盾水印。'
        : '选择疑似侵权 WAV，检测是否含有隐盾水印。';
    final detail = bytes == null
        ? emptyText
        : '${(bytes!.length / 1024).toStringAsFixed(1)} KB';
    return Container(
      height: 150,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white12),
        color: const Color(0xFF0F151B),
      ),
      child: Row(
        children: [
          Icon(
            kind == WatermarkAssetKind.image
                ? Icons.image_search_outlined
                : Icons.graphic_eq_outlined,
            size: 42,
            color: Colors.white54,
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
                Text(detail, style: const TextStyle(color: Colors.white70)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result});

  final WatermarkReadResult result;

  @override
  Widget build(BuildContext context) {
    final parent = result.parentWatermarkUid ?? '无';
    final reason = result.rewriteReason ?? '无';
    return _MessageCard(
      icon: Icons.fact_check_outlined,
      title: '提取成功',
      detail:
          'UID: ${result.watermarkUid}\nrevision: ${result.revision}\nparent UID: $parent\nrewrite_reason: $reason\n设备: ${result.deviceIdHex}\n文件哈希片段: ${result.fileHashHex}',
    );
  }
}

class _MessageCard extends StatelessWidget {
  const _MessageCard({
    required this.icon,
    required this.title,
    required this.detail,
  });

  final IconData icon;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: const Color(0xFF162028),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: ListTile(
        leading: Icon(icon, color: const Color(0xFF59D2C2)),
        title: Text(title),
        subtitle: Text(detail),
      ),
    );
  }
}
