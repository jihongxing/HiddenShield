enum WatermarkAssetKind { image, audio, video }

class WatermarkPayloadSeed {
  const WatermarkPayloadSeed({
    required this.userSeed,
    required this.timestamp,
    required this.deviceId,
    required this.fileHash,
  });

  final List<int> userSeed;
  final int timestamp;
  final List<int> deviceId;
  final List<int> fileHash;
}

class WatermarkWriteRequest {
  const WatermarkWriteRequest({
    required this.kind,
    required this.bytes,
    required this.seed,
    this.allowRewrite = false,
    this.rewriteReason,
  });

  final WatermarkAssetKind kind;
  final List<int> bytes;
  final WatermarkPayloadSeed seed;
  final bool allowRewrite;
  final String? rewriteReason;
}

class WatermarkWriteResult {
  const WatermarkWriteResult({
    required this.kind,
    required this.bytes,
    required this.watermarkUid,
    required this.revision,
    required this.sha256,
  });

  final WatermarkAssetKind kind;
  final List<int> bytes;
  final String watermarkUid;
  final int revision;
  final String sha256;
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
