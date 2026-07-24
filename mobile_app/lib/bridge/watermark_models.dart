enum WatermarkAssetKind { image, audio, video }

class WatermarkPayloadSeed {
  const WatermarkPayloadSeed({
    required this.creatorIdentity,
    required this.deviceIdentity,
    required this.mediaBytes,
    required this.timestamp,
  });

  final String creatorIdentity;
  final String deviceIdentity;
  final List<int> mediaBytes;
  final int timestamp;
}

class WatermarkWriteRequest {
  const WatermarkWriteRequest({
    required this.kind,
    required this.bytes,
    required this.seed,
    this.allowRewrite = false,
    this.rewriteReason,
    this.parentWatermarkUid,
    this.revision = 1,
    this.registryDraft,
  });

  final WatermarkAssetKind kind;
  final List<int> bytes;
  final WatermarkPayloadSeed seed;
  final bool allowRewrite;
  final String? rewriteReason;
  final String? parentWatermarkUid;
  final int revision;
  final WatermarkRegistryDraft? registryDraft;
}

class WatermarkRegistryDraft {
  const WatermarkRegistryDraft({
    required this.watermarkUid,
    required this.watermarkIdIssueMode,
    required this.registryStatus,
    required this.registryReceipt,
    required this.registryProofHash,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.parentWatermarkUid,
    required this.revision,
  });

  final String watermarkUid;
  final String watermarkIdIssueMode;
  final String registryStatus;
  final String registryReceipt;
  final String registryProofHash;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? parentWatermarkUid;
  final int revision;
}

class WatermarkWriteResult {
  const WatermarkWriteResult({
    required this.kind,
    required this.bytes,
    required this.watermarkUid,
    required this.revision,
    required this.sha256,
    required this.verification,
    required this.seed,
    required this.processTimeMs,
    this.isProductionWatermark = true,
    this.outputFileName,
    this.outputLocationLabel,
    this.outputActionLabel,
    this.registryDraft,
  });

  final WatermarkAssetKind kind;
  final List<int> bytes;
  final String watermarkUid;
  final int revision;
  final String sha256;
  final WatermarkWriteVerification verification;
  final WatermarkPayloadSeed seed;
  final int processTimeMs;
  final bool isProductionWatermark;
  final String? outputFileName;
  final String? outputLocationLabel;
  final String? outputActionLabel;
  final WatermarkRegistryDraft? registryDraft;

  WatermarkWriteResult copyWithOutputArtifact({
    required String outputFileName,
    required String outputLocationLabel,
    required String outputActionLabel,
  }) {
    return WatermarkWriteResult(
      kind: kind,
      bytes: bytes,
      watermarkUid: watermarkUid,
      revision: revision,
      sha256: sha256,
      verification: verification,
      seed: seed,
      processTimeMs: processTimeMs,
      isProductionWatermark: isProductionWatermark,
      outputFileName: outputFileName,
      outputLocationLabel: outputLocationLabel,
      outputActionLabel: outputActionLabel,
      registryDraft: registryDraft,
    );
  }
}

class WatermarkWriteVerification {
  const WatermarkWriteVerification({
    required this.verified,
    required this.watermarkUid,
    required this.revision,
    required this.message,
    this.fileHashHex,
    this.deviceIdHex,
    this.payloadProtocolVersion = 2,
    this.payloadBytesLength = 119,
  });

  final bool verified;
  final String watermarkUid;
  final int revision;
  final String message;
  final String? fileHashHex;
  final String? deviceIdHex;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
}

class WatermarkReadRequest {
  const WatermarkReadRequest({required this.kind, required this.bytes});

  final WatermarkAssetKind kind;
  final List<int> bytes;
}

class WatermarkReadResult {
  const WatermarkReadResult({
    required this.kind,
    required this.watermarkUid,
    required this.revision,
    required this.timestamp,
    required this.deviceIdHex,
    required this.fileHashHex,
    this.payloadProtocolVersion = 2,
    this.payloadBytesLength = 119,
    this.watermarkIdIssueMode = 'offline_generated',
    this.payloadAuthStatus = 'verified',
    this.mediaType,
    this.isProductionWatermark = true,
    this.parentWatermarkUid,
    this.rewriteReason,
  });

  final WatermarkAssetKind kind;
  final String watermarkUid;
  final int revision;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final int timestamp;
  final String deviceIdHex;
  final String fileHashHex;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String watermarkIdIssueMode;
  final String payloadAuthStatus;
  final String? mediaType;
  final bool isProductionWatermark;
}

class BridgeCapabilities {
  const BridgeCapabilities({
    required this.supportedKinds,
    required this.supportsDesktopSync,
    required this.supportsLocalVideo,
  });

  final List<WatermarkAssetKind> supportedKinds;
  final bool supportsDesktopSync;
  final bool supportsLocalVideo;
}

class BridgeStatus {
  const BridgeStatus({
    required this.label,
    required this.detail,
    required this.capabilities,
  });

  final String label;
  final String detail;
  final BridgeCapabilities capabilities;
}
