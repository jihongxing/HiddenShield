enum OfflineLicenseStatus {
  unsupported,
  inactive,
  active,
  notYetValid,
  expired,
  revoked,
  deviceMismatch,
  invalid,
  secureStoreFailure,
}

class OfflineLicenseSnapshot {
  const OfflineLicenseSnapshot({
    required this.status,
    required this.installationId,
    this.licenseId,
    this.productCode,
    this.keyId,
    this.notBefore,
    this.expiresAt,
    this.revocationListId,
    this.revocationSequence,
    this.lastError,
  });

  const OfflineLicenseSnapshot.unsupported()
    : this(
        status: OfflineLicenseStatus.unsupported,
        installationId: '',
        lastError: 'offline_license_secure_storage_unavailable',
      );

  final OfflineLicenseStatus status;
  final String installationId;
  final String? licenseId;
  final String? productCode;
  final String? keyId;
  final DateTime? notBefore;
  final DateTime? expiresAt;
  final String? revocationListId;
  final int? revocationSequence;
  final String? lastError;

  bool get isActive => status == OfflineLicenseStatus.active;

  Map<String, bool> get localFeatures => {
    'batch_processing': isActive,
    'report_export': false,
  };
}

class OfflineLicenseMetadata {
  const OfflineLicenseMetadata({
    required this.installationId,
    required this.status,
    required this.updatedAt,
    this.licenseId,
    this.productCode,
    this.keyId,
    this.notBefore,
    this.expiresAt,
    this.revocationListId,
    this.revocationSequence,
    this.lastError,
  });

  factory OfflineLicenseMetadata.fromSnapshot(
    OfflineLicenseSnapshot snapshot,
    DateTime updatedAt,
  ) {
    return OfflineLicenseMetadata(
      installationId: snapshot.installationId,
      status: snapshot.status,
      updatedAt: updatedAt,
      licenseId: snapshot.licenseId,
      productCode: snapshot.productCode,
      keyId: snapshot.keyId,
      notBefore: snapshot.notBefore,
      expiresAt: snapshot.expiresAt,
      revocationListId: snapshot.revocationListId,
      revocationSequence: snapshot.revocationSequence,
      lastError: snapshot.lastError,
    );
  }

  final String installationId;
  final OfflineLicenseStatus status;
  final DateTime updatedAt;
  final String? licenseId;
  final String? productCode;
  final String? keyId;
  final DateTime? notBefore;
  final DateTime? expiresAt;
  final String? revocationListId;
  final int? revocationSequence;
  final String? lastError;
}

class OfflineLicenseAuditEvent {
  const OfflineLicenseAuditEvent({
    required this.id,
    required this.occurredAt,
    required this.action,
    required this.result,
    this.licenseId,
    this.keyId,
    this.detailCode,
  });

  final String id;
  final DateTime occurredAt;
  final String action;
  final String result;
  final String? licenseId;
  final String? keyId;
  final String? detailCode;
}

class OfflineExecutionAuthorization {
  const OfflineExecutionAuthorization({
    required this.feature,
    required this.allowed,
    required this.source,
    this.errorCode,
  });

  final String feature;
  final bool allowed;
  final String source;
  final String? errorCode;
}
