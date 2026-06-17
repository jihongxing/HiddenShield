import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';

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
}

Future<RewritePreflightResult> inspectMobileRewriteTarget({
  required WatermarkBridge bridge,
  required MobileAppState appState,
  required WatermarkAssetKind kind,
  required List<int> bytes,
}) async {
  try {
    final readResult = await bridge.read(
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
      summary: '检测到已有隐盾水印，继续写入将记录为第 ${detectedRevision + 1} 次写入。',
      reasonCode: 'rewrite_detected',
      reasonDetail: localRecord == null
          ? '检测到有效水印但本机版权库未找到对应记录，重写仍会保留提取到的父级 UID。'
          : '已在本机版权库找到同 UID 记录，重写会保留父级 UID 和递增版本。',
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
      summary: '该类型暂不支持写前水印预检。',
      reasonCode: 'unsupported_preflight',
      reasonDetail: error.message?.toString() ?? '当前移动端只支持图片和 WAV 音频预检。',
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
    summary: '未检测到已有隐盾水印，将按首次写入处理。',
    reasonCode: 'no_valid_watermark',
    reasonDetail: '写前预检没有提取到有效水印；如果继续写入，会创建新的版权存证。',
  );
}
