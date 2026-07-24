enum RightsEvidencePackAccessFailureCode {
  authorizationRevoked(
    'evidence_pack_authorization_revoked',
    '目录授权已失效，请重新选择案件包目录。',
  ),
  directoryMissing('evidence_pack_directory_missing', '案件包目录已移动或删除，请重新选择。'),
  attachmentMissing('evidence_pack_attachment_missing', '案件包附件缺失，请恢复原目录内容后重试。'),
  providerUnavailable(
    'evidence_pack_provider_unavailable',
    '文件提供方当前不可用，请恢复对应应用或改选本地目录。',
  ),
  unknown('evidence_pack_access_failed', '案件包目录读取失败，请重新选择后重试。');

  const RightsEvidencePackAccessFailureCode(this.wireCode, this.userMessage);

  final String wireCode;
  final String userMessage;

  static RightsEvidencePackAccessFailureCode fromWireCode(String code) {
    return values.firstWhere(
      (value) => value.wireCode == code,
      orElse: () => unknown,
    );
  }
}

class RightsEvidencePackAccessException implements Exception {
  const RightsEvidencePackAccessException({
    required this.code,
    required this.userMessage,
    this.technicalMessage,
  });

  final RightsEvidencePackAccessFailureCode code;
  final String userMessage;
  final String? technicalMessage;

  @override
  String toString() => code.wireCode;
}
