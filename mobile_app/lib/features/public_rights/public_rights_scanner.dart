import '../../app/mobile_app_state.dart';
import '../../sync/cloud_account_client.dart';

enum PublicRightsSdkStatus { ok, error }

enum PublicRightsSdkErrorCode {
  notFound,
  registryUnavailable,
  payloadInvalid,
  manifestConflict,
  backfillPending,
  backfillDisputed,
  internalError,
}

const publicRightsMetadataJsonExportLabel = '导出公开元数据 JSON';
const publicRightsEmbeddedImageExportUnavailableMessage =
    '移动端当前版权库详情先提供公开元数据 JSON 分享；图片嵌入副本需要重新选择 PNG / JPEG 保护副本文件后再导出。';

class PublicRightsPolicyResolution {
  const PublicRightsPolicyResolution({
    required this.trainingPolicy,
    required this.trainingPolicyLabel,
    required this.rightsManifestStatus,
    required this.registryStatus,
    required this.scanStatus,
    required this.legalConclusion,
    required this.requiresHumanReview,
    required this.canTreatAsTrainingAllowed,
  });

  final String trainingPolicy;
  final String trainingPolicyLabel;
  final String rightsManifestStatus;
  final String registryStatus;
  final String scanStatus;
  final bool legalConclusion;
  final bool requiresHumanReview;
  final bool canTreatAsTrainingAllowed;
}

class PublicRightsSdkResult {
  const PublicRightsSdkResult({
    required this.status,
    required this.scan,
    required this.error,
    required this.warnings,
    required this.message,
    required this.policy,
  });

  final PublicRightsSdkStatus status;
  final PublicRightsQueryResponse? scan;
  final PublicRightsSdkErrorCode? error;
  final List<String> warnings;
  final String message;
  final PublicRightsPolicyResolution? policy;
}

class PublicRightsScanner {
  const PublicRightsScanner({required this.appState});

  final MobileAppState appState;

  Future<PublicRightsSdkResult> scanOne(String watermarkUid) async {
    final uid = watermarkUid.trim();
    if (uid.isEmpty) {
      return _buildErrorResult(null, PublicRightsSdkErrorCode.notFound);
    }
    try {
      return _buildOkResult(await appState.fetchPublicRights(uid));
    } catch (error) {
      return _buildErrorResult(null, classifyPublicRightsError(error));
    }
  }

  PublicRightsPolicyResolution resolvePolicy(
    PublicRightsQueryResponse scanResult,
  ) => resolvePublicRightsPolicy(scanResult);

  String formatUserMessage(PublicRightsSdkResult result) =>
      formatPublicRightsUserMessage(result);
}

PublicRightsPolicyResolution resolvePublicRightsPolicy(
  PublicRightsQueryResponse scanResult,
) {
  final warnings = scanResult.warnings.toSet();
  final rightsManifestStatus = scanResult.rightsManifest?.status ?? 'missing';
  return PublicRightsPolicyResolution(
    trainingPolicy: scanResult.trainingPermission.policy,
    trainingPolicyLabel: scanResult.trainingPermission.label,
    rightsManifestStatus: rightsManifestStatus,
    registryStatus: scanResult.registry.registryStatus,
    scanStatus: scanResult.scanStatus,
    legalConclusion: false,
    requiresHumanReview:
        scanResult.scanStatus == 'backfill_disputed' ||
        rightsManifestStatus == 'disputed' ||
        warnings.contains('registry_requires_human_review'),
    canTreatAsTrainingAllowed: false,
  );
}

String formatPublicRightsUserMessage(PublicRightsSdkResult result) {
  if (result.status == PublicRightsSdkStatus.error) {
    return _messageForError(
      result.error ?? PublicRightsSdkErrorCode.internalError,
    );
  }
  final scan = result.scan;
  if (scan == null) {
    return _messageForError(PublicRightsSdkErrorCode.internalError);
  }
  if (result.warnings.contains('backfill_pending')) {
    return '登记记录已找到，但公开权利 manifest 尚未完成回填。';
  }
  return switch (scan.scanStatus) {
    'backfill_disputed' => '公开权利声明需要人工处理，请以 registry 和人工核验为准。',
    'registry_revoked' => '该公开权利声明已撤销，请不要按旧声明直接使用。',
    'registry_superseded' => '该公开权利声明已有新版，请查看最新 registry 记录。',
    'watermark_only' => '仅发现水印锚点，尚未查询到 active 公开权利 manifest。',
    _ => '已读取创作者声明与 registry 快照；该结果不是法律授权结论。',
  };
}

PublicRightsSdkErrorCode classifyPublicRightsError(Object error) {
  final message = error.toString().toLowerCase();
  if (message.contains('http 404') ||
      message.contains('missing') ||
      message.contains('not_found')) {
    return PublicRightsSdkErrorCode.notFound;
  }
  if (message.contains('http 401') ||
      message.contains('http 403') ||
      message.contains('unavailable') ||
      message.contains('未连接公开 registry')) {
    return PublicRightsSdkErrorCode.registryUnavailable;
  }
  if (message.contains('payload_invalid') || message.contains('auth')) {
    return PublicRightsSdkErrorCode.payloadInvalid;
  }
  if (message.contains('conflict') || message.contains('disputed')) {
    return PublicRightsSdkErrorCode.manifestConflict;
  }
  return PublicRightsSdkErrorCode.internalError;
}

PublicRightsSdkResult _buildOkResult(PublicRightsQueryResponse scan) {
  final error = _errorCodeFromScan(scan);
  final status = error == null
      ? PublicRightsSdkStatus.ok
      : PublicRightsSdkStatus.error;
  late final PublicRightsSdkResult result;
  result = PublicRightsSdkResult(
    status: status,
    scan: scan,
    error: error,
    warnings: scan.warnings,
    message: '',
    policy: resolvePublicRightsPolicy(scan),
  );
  return PublicRightsSdkResult(
    status: result.status,
    scan: result.scan,
    error: result.error,
    warnings: result.warnings,
    message: formatPublicRightsUserMessage(result),
    policy: result.policy,
  );
}

PublicRightsSdkResult _buildErrorResult(
  PublicRightsQueryResponse? scan,
  PublicRightsSdkErrorCode error,
) {
  late final PublicRightsSdkResult result;
  result = PublicRightsSdkResult(
    status: PublicRightsSdkStatus.error,
    scan: scan,
    error: error,
    warnings: scan?.warnings ?? const [],
    message: '',
    policy: scan == null ? null : resolvePublicRightsPolicy(scan),
  );
  return PublicRightsSdkResult(
    status: result.status,
    scan: result.scan,
    error: result.error,
    warnings: result.warnings,
    message: formatPublicRightsUserMessage(result),
    policy: result.policy,
  );
}

PublicRightsSdkErrorCode? _errorCodeFromScan(PublicRightsQueryResponse scan) {
  if (scan.warnings.contains('backfill_pending')) {
    return PublicRightsSdkErrorCode.backfillPending;
  }
  if (scan.scanStatus == 'backfill_disputed') {
    return PublicRightsSdkErrorCode.backfillDisputed;
  }
  if (scan.scanStatus == 'metadata_registry_conflict') {
    return PublicRightsSdkErrorCode.manifestConflict;
  }
  if (scan.registry.payloadAuthStatus == 'failed' ||
      scan.registry.payloadAuthStatus == 'invalid') {
    return PublicRightsSdkErrorCode.payloadInvalid;
  }
  return null;
}

String publicRightsScanStatusLabel(String value) => switch (value) {
  'registry_active' => 'registry 已生效',
  'watermark_only' => '仅识别到水印锚点',
  'registry_revoked' => 'registry 已撤销',
  'registry_superseded' => 'registry 已被替代',
  'backfill_disputed' => '需要人工复核',
  _ => value.trim().isEmpty ? '未记录' : value,
};

String publicRightsAnchorProtocolLabel(String value) => switch (value) {
  'v3_minimal_anchor' => 'V3 最小媒体锚点',
  'v2_migration_anchor' => 'V2 迁移桥接锚点',
  _ => value.trim().isEmpty ? '未记录' : value,
};

String _messageForError(PublicRightsSdkErrorCode error) {
  return switch (error) {
    PublicRightsSdkErrorCode.notFound => '未找到公开 registry 记录。',
    PublicRightsSdkErrorCode.registryUnavailable => '公开 registry 暂不可用，请稍后重试。',
    PublicRightsSdkErrorCode.payloadInvalid => '水印锚点认证失败，不能据此判断权利状态。',
    PublicRightsSdkErrorCode.manifestConflict =>
      '公开元数据与 registry 声明存在冲突，请人工核验。',
    PublicRightsSdkErrorCode.backfillPending => '公开权利 manifest 尚未完成回填。',
    PublicRightsSdkErrorCode.backfillDisputed => '公开权利声明需要人工处理。',
    PublicRightsSdkErrorCode.internalError => '公开权利查询暂时失败。',
  };
}
