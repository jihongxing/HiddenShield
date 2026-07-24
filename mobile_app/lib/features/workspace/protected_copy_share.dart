import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:share_plus/share_plus.dart';

import '../../bridge/watermark_models.dart';

Future<void> shareProtectedCopy({
  required BuildContext context,
  required WatermarkWriteResult result,
  required String fallbackFileName,
  required String mimeType,
}) async {
  final fileName = _safeProtectedCopyName(
    result.outputFileName ?? fallbackFileName,
    fallbackFileName,
  );
  final messenger = ScaffoldMessenger.of(context);
  final box = context.findRenderObject() as RenderBox?;
  final sharePositionOrigin = box == null
      ? null
      : box.localToGlobal(Offset.zero) & box.size;

  try {
    final shareResult = await SharePlus.instance.share(
      ShareParams(
        title: '保存或分享保护副本',
        subject: fileName,
        files: [
          XFile.fromData(
            Uint8List.fromList(result.bytes),
            mimeType: mimeType,
            name: fileName,
            length: result.bytes.length,
          ),
        ],
        fileNameOverrides: [fileName],
        sharePositionOrigin: sharePositionOrigin,
      ),
    );
    if (!context.mounted) {
      return;
    }
    messenger.showSnackBar(
      SnackBar(content: Text(_shareResultMessage(shareResult.status))),
    );
  } catch (_) {
    if (!context.mounted) {
      return;
    }
    messenger.showSnackBar(
      const SnackBar(content: Text('当前设备暂时无法打开系统分享面板，请稍后重试。')),
    );
  }
}

String _safeProtectedCopyName(String candidate, String fallback) {
  final trimmed = candidate.trim();
  if (trimmed.isEmpty) {
    return fallback;
  }
  return trimmed.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_');
}

String _shareResultMessage(ShareResultStatus status) {
  return switch (status) {
    ShareResultStatus.success => '已交给系统分享面板处理。',
    ShareResultStatus.dismissed => '已取消保存或分享。',
    ShareResultStatus.unavailable => '当前设备暂时无法打开系统分享面板，请稍后重试。',
  };
}
