import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../src/rust/api.dart' as rust_api;

class RewritePreflightResult {
  const RewritePreflightResult({
    required this.kind,
    required this.hasWatermark,
    required this.detectedRevision,
    required this.nextRevision,
    required this.watermarkUid,
    required this.parentWatermarkUid,
    required this.rewriteReason,
    required this.summary,
    required this.reasonCode,
    required this.reasonDetail,
    this.readResult,
  });

  final WatermarkAssetKind kind;
  final bool hasWatermark;
  final int? detectedRevision;
  final int nextRevision;
  final String? watermarkUid;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final String summary;
  final String reasonCode;
  final String reasonDetail;
  final WatermarkReadResult? readResult;

  bool get shouldBlockRewrite => reasonCode == 'preflight_extract_failed';
  bool shouldBlockInitialWrite({required bool allowRewrite}) {
    return hasWatermark && !allowRewrite;
  }
}

String existingWatermarkRewriteBlockedMessage(String? watermarkUid) {
  final uid = watermarkUid?.trim();
  if (uid != null && uid.isNotEmpty) {
    return '检测到已有版权记录 $uid。如需生成新版，请开启“作为新版写入”。';
  }
  return '检测到已有版权记录。如需生成新版，请开启“作为新版写入”。';
}

String mobileWatermarkWriteErrorMessage(Object error) {
  if (error is rust_api.MobileWatermarkError_OperationFailed) {
    if (error.code == 'already_watermarked') {
      return existingWatermarkRewriteBlockedMessage(error.existingUid);
    }
    if (error.code == 'audio_decode_failed' ||
        error.code == 'audio_track_missing' ||
        error.code == 'audio_sample_rate_missing' ||
        error.code == 'audio_normalize_failed' ||
        error.code == 'audio_channel_layout_changed') {
      return '音频暂时无法生成保护副本。请选择完整、可播放的音频文件后重试。';
    }
    return '保护副本未生成。请确认文件可读取后重试；如果持续失败，请导出日志反馈。';
  }
  if (error is rust_api.MobileWatermarkError_InvalidPayload) {
    if (error.code == 'missing_creator_identity') {
      return '请先完成创作者身份设置，再生成保护副本。';
    }
    return '写入信息不完整，请重新选择作品后再试。';
  }
  final raw = '$error';
  final lower = raw.toLowerCase();
  if (lower.contains('already_watermarked')) {
    final uid = _watermarkUidFromError(raw);
    return existingWatermarkRewriteBlockedMessage(uid);
  }
  if (lower.contains('watermark already exists in source media')) {
    final uid = _watermarkUidFromError(raw);
    return existingWatermarkRewriteBlockedMessage(uid);
  }
  if (raw.contains('Watermark embedding failed') ||
      lower.contains('watermark operation failed')) {
    return '保护副本未生成。请确认文件可读取后重试；如果持续失败，请导出日志反馈。';
  }
  return '处理过程未完成，请重试；如果持续失败，请导出日志反馈。';
}

String? _watermarkUidFromError(String raw) {
  return RegExp(
    r'(?:HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}|PREVIEW-[A-Z0-9]{12})',
  ).firstMatch(raw)?.group(0);
}

Future<RewritePreflightResult> inspectMobileRewriteTarget({
  required WatermarkBridge bridge,
  required MobileAppState appState,
  required WatermarkAssetKind kind,
  required List<int> bytes,
}) async {
  try {
    final readResult = await bridge.detectExisting(
      WatermarkReadRequest(kind: kind, bytes: bytes),
    );
    if (readResult == null) {
      return _firstWritePlan(kind);
    }

    final localRecord = latestLocalRecordByUid(
      appState: appState,
      watermarkUid: readResult.watermarkUid,
    );
    final detectedRevision = localRecord?.revision ?? readResult.revision;
    return RewritePreflightResult(
      kind: kind,
      hasWatermark: true,
      detectedRevision: detectedRevision,
      nextRevision: detectedRevision + 1,
      watermarkUid: readResult.watermarkUid,
      parentWatermarkUid: readResult.watermarkUid,
      rewriteReason: localRecord?.rewriteReason ?? readResult.rewriteReason,
      summary: '检测到已有版权编号，继续生成将记录为第 ${detectedRevision + 1} 次版本。',
      reasonCode: 'rewrite_detected',
      reasonDetail: localRecord == null
          ? '本机版权库未找到对应记录，生成新版时仍会保留已识别的上一版编号。'
          : '本机版权库已找到对应记录，生成新版时会保留上一版编号并递增版本次数。',
      readResult: readResult,
    );
  } on UnsupportedError catch (error) {
    return RewritePreflightResult(
      kind: kind,
      hasWatermark: false,
      detectedRevision: null,
      nextRevision: 1,
      watermarkUid: null,
      parentWatermarkUid: null,
      rewriteReason: null,
      summary: '该类型暂不支持版权记录检查。',
      reasonCode: 'unsupported_preflight',
      reasonDetail: error.message?.toString() ?? '当前移动端只支持图片和音频检查。',
    );
  } catch (_) {
    return _firstWritePlan(kind);
  }
}

VaultRecord? latestLocalRecordByUid({
  required MobileAppState appState,
  required String watermarkUid,
}) {
  final matches = appState.records
      .where((record) => record.watermarkUid == watermarkUid)
      .toList(growable: false);
  if (matches.isEmpty) {
    return null;
  }
  matches.sort((a, b) {
    final revisionCompare = b.revision.compareTo(a.revision);
    if (revisionCompare != 0) {
      return revisionCompare;
    }
    return b.createdAt.compareTo(a.createdAt);
  });
  return matches.first;
}

RewritePreflightResult _firstWritePlan(WatermarkAssetKind kind) {
  return RewritePreflightResult(
    kind: kind,
    hasWatermark: false,
    detectedRevision: null,
    nextRevision: 1,
    watermarkUid: null,
    parentWatermarkUid: null,
    rewriteReason: null,
    summary: '未检测到已有版权编号，将按首次写入处理。',
    reasonCode: 'no_valid_watermark',
    reasonDetail: '如果继续生成保护副本，会创建新的版权记录。',
  );
}

String preflightSummaryLabel(RewritePreflightResult? result) {
  if (result == null) return '';
  if (result.hasWatermark) return '已检测到已有版权记录';
  if (result.reasonCode == 'no_valid_watermark') return '未检测到已有隐盾水印';
  return '版权记录检查完成';
}

String preflightActionLabel(RewritePreflightResult? result) {
  if (result == null) return '';
  if (result.hasWatermark) {
    return '继续写入将记录为第 ${result.nextRevision} 次写入';
  }
  if (result.reasonCode == 'no_valid_watermark') {
    return '将按首次写入处理';
  }
  return '写前预检已完成';
}

List<String> preflightEvidenceLines(RewritePreflightResult? result) {
  if (result == null) return const [];
  final lines = <String>[];
  if (result.reasonDetail.trim().isNotEmpty) {
    lines.add(result.reasonDetail.trim());
  }
  if (result.watermarkUid != null && result.watermarkUid!.trim().isNotEmpty) {
    lines.add('上一版编号：${result.watermarkUid!.trim()}');
  }
  if (result.detectedRevision != null) {
    lines.add('当前识别为第 ${result.detectedRevision} 次版本');
  }
  return lines;
}
