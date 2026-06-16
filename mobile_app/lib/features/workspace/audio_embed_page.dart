import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';

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
  Uint8List? _selectedBytes;
  String? _fileName;
  bool _allowRewrite = false;
  bool _isProcessing = false;
  WatermarkWriteResult? _result;
  String? _errorText;

  @override
  Widget build(BuildContext context) {
    final selectedBytes = _selectedBytes;
    return Scaffold(
      appBar: AppBar(title: const Text('音频嵌入')),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            _SectionCard(
              title: '导入 WAV',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _AudioPreview(bytes: selectedBytes, fileName: _fileName),
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
            _SectionCard(
              title: '写入设置',
              child: SwitchListTile(
                value: _allowRewrite,
                onChanged: _isProcessing
                    ? null
                    : (value) => setState(() => _allowRewrite = value),
                title: const Text('允许重写已有隐盾水印'),
                subtitle: const Text('默认关闭，避免覆盖第一次写入记录。'),
                contentPadding: EdgeInsets.zero,
              ),
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: selectedBytes == null || _isProcessing
                  ? null
                  : _embedAudio,
              icon: _isProcessing
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.shield_outlined),
              label: Text(_isProcessing ? '正在写入' : '写入盲水印'),
            ),
            if (_errorText != null) ...[
              const SizedBox(height: 12),
              _MessageCard(
                icon: Icons.error_outline,
                title: '处理失败',
                detail: _errorText!,
              ),
            ],
            if (_result != null) ...[
              const SizedBox(height: 12),
              _ResultCard(result: _result!),
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
      _result = null;
      _errorText = null;
    });
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
    });

    try {
      final result = await widget.bridge.write(
        WatermarkWriteRequest(
          kind: WatermarkAssetKind.audio,
          bytes: bytes,
          seed: WatermarkPayloadSeed(
            userSeed: const [1, 2, 3, 4, 5, 6, 7, 8],
            timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
            deviceId: const [9, 10, 11, 12],
            fileHash: const [13, 14],
          ),
          allowRewrite: _allowRewrite,
          rewriteReason: _allowRewrite ? 'mobile explicit rewrite' : null,
        ),
      );
      if (!mounted) return;
      widget.appState.addWriteResult(
        result: result,
        fileName: _fileName,
        allowRewrite: _allowRewrite,
        rewriteReason: _allowRewrite ? 'mobile explicit rewrite' : null,
      );
      setState(() => _result = result);
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

class _AudioPreview extends StatelessWidget {
  const _AudioPreview({required this.bytes, required this.fileName});

  final Uint8List? bytes;
  final String? fileName;

  @override
  Widget build(BuildContext context) {
    final sizeText = bytes == null
        ? '选择一段 WAV 音频，生成本地版权记录。'
        : '${(bytes!.length / 1024).toStringAsFixed(1)} KB';
    return Container(
      height: 160,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.white12),
        color: const Color(0xFF0F151B),
      ),
      child: Row(
        children: [
          const Icon(
            Icons.graphic_eq_outlined,
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
                  fileName ?? '未选择音频',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(sizeText, style: const TextStyle(color: Colors.white70)),
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

  final WatermarkWriteResult result;

  @override
  Widget build(BuildContext context) {
    final shaPreview = result.sha256.length > 16
        ? '${result.sha256.substring(0, 16)}...'
        : result.sha256;
    return _MessageCard(
      icon: Icons.verified_outlined,
      title: '写入完成',
      detail:
          'UID: ${result.watermarkUid}\nrevision: ${result.revision}\nsha256: $shaPreview',
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
