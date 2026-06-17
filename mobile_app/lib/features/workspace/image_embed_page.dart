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

class ImageEmbedPage extends StatefulWidget {
  const ImageEmbedPage({
    super.key,
    required this.bridge,
    required this.appState,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  State<ImageEmbedPage> createState() => _ImageEmbedPageState();
}

class _ImageEmbedPageState extends State<ImageEmbedPage> {
  Uint8List? _selectedBytes;
  String? _fileName;
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
    return Scaffold(
      appBar: AppBar(title: const Text('保护图片')),
      body: SafeArea(
        child: ListView(
          cacheExtent: 1000,
          padding: const EdgeInsets.all(16),
          children: [
            HsPanel(
              title: '导入图片',
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (selectedBytes == null)
                    const _EmptyPreview()
                  else
                    _ImagePreview(bytes: selectedBytes, fileName: _fileName),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    onPressed: _isProcessing ? null : _pickImage,
                    icon: const Icon(Icons.upload_file_outlined),
                    label: Text(selectedBytes == null ? '选择图片' : '重新选择'),
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
                    title: '图片取证优先',
                    detail: '将生成 PNG 保护副本，并在完成前回读验证版权编号。',
                  ),
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
              onPressed: selectedBytes == null || _isProcessing
                  ? null
                  : _embedImage,
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

  Future<void> _pickImage() async {
    final result = await FilePicker.pickFiles(
      type: FileType.image,
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
      _savedRecord = null;
      _preflight = null;
      _errorText = null;
    });
    await _inspectSelected(bytes);
  }

  Future<void> _embedImage() async {
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
          kind: WatermarkAssetKind.image,
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
        WatermarkReadRequest(kind: WatermarkAssetKind.image, bytes: bytes),
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
      kind: WatermarkAssetKind.image,
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

class _EmptyPreview extends StatelessWidget {
  const _EmptyPreview();

  @override
  Widget build(BuildContext context) {
    return HsPreviewBox(
      height: 180,
      child: const Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.image_outlined, size: 42, color: HsColors.iconMuted),
          SizedBox(height: 8),
          Text('选择一张作品，生成保护副本和版权记录。'),
        ],
      ),
    );
  }
}

class _ImagePreview extends StatelessWidget {
  const _ImagePreview({required this.bytes, required this.fileName});

  final Uint8List bytes;
  final String? fileName;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: Image.memory(
            bytes,
            height: 220,
            width: double.infinity,
            fit: BoxFit.cover,
          ),
        ),
        const SizedBox(height: 8),
        Text(
          fileName ?? '未命名图片',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(color: HsColors.textMuted),
        ),
      ],
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
        detail: '选择图片后会自动检查是否已有版权记录。',
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
