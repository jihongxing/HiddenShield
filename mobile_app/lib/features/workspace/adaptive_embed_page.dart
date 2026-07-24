import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import 'audio_embed_page.dart';
import 'image_embed_page.dart';
import 'media_file_kind.dart';

class AdaptiveEmbedPage extends StatefulWidget {
  const AdaptiveEmbedPage({
    super.key,
    required this.bridge,
    required this.appState,
    required this.onOpenVault,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;
  final VoidCallback onOpenVault;

  @override
  State<AdaptiveEmbedPage> createState() => _AdaptiveEmbedPageState();
}

class _AdaptiveEmbedPageState extends State<AdaptiveEmbedPage> {
  String? _errorText;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: FeaturePageScaffold(
        title: '作品写入',
        subtitle: '选择图片或音频作品，系统会自动识别类型并进入对应写入流程。',
        icon: Icons.upload_file_outlined,
        showBackButton: true,
        children: [
          HsPanel(
            title: '选择作品',
            icon: Icons.add_to_photos_outlined,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const HsPreviewBox(
                  height: 170,
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        Icons.auto_awesome_motion_outlined,
                        size: 42,
                        color: HsColors.iconMuted,
                      ),
                      SizedBox(height: 10),
                      Text('导入图片或音频作品'),
                      SizedBox(height: 6),
                      Text(
                        '支持 JPG / PNG / WebP 和 WAV / MP3 / AAC / FLAC / OGG / M4A',
                        textAlign: TextAlign.center,
                        style: TextStyle(color: HsColors.textMuted),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: HsSpacing.md),
                FilledButton.icon(
                  onPressed: _pickWork,
                  icon: const Icon(Icons.upload_file_outlined),
                  label: const Text('选择作品'),
                ),
              ],
            ),
          ),
          if (_errorText != null) ...[
            const SizedBox(height: HsSpacing.md),
            HsMessageCard(
              icon: Icons.info_outline,
              title: '暂不支持该文件',
              detail: _errorText!,
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _pickWork() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: supportedEmbeddableMediaExtensions,
      withData: true,
    );
    final file = result?.files.single;
    final bytes = file?.bytes;
    if (file == null || bytes == null) {
      return;
    }
    final kind = mediaKindForFileName(file.name);
    if (kind == null) {
      setState(() {
        _errorText = '请选择受支持的图片或音频文件。';
      });
      return;
    }
    if (!mounted) return;
    await Navigator.of(context).pushReplacement(
      MaterialPageRoute<void>(
        builder: (_) => _pageForKind(kind, file.name, bytes),
      ),
    );
  }

  Widget _pageForKind(
    WatermarkAssetKind kind,
    String fileName,
    Uint8List bytes,
  ) {
    return switch (kind) {
      WatermarkAssetKind.image => ImageEmbedPage(
        bridge: widget.bridge,
        appState: widget.appState,
        onOpenVault: widget.onOpenVault,
        initialBytes: bytes,
        initialFileName: fileName,
      ),
      WatermarkAssetKind.audio => AudioEmbedPage(
        bridge: widget.bridge,
        appState: widget.appState,
        onOpenVault: widget.onOpenVault,
        initialBytes: bytes,
        initialFileName: fileName,
      ),
      WatermarkAssetKind.video => throw StateError('unsupported media kind'),
    };
  }
}
