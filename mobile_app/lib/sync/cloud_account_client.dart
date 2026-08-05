import 'dart:convert';

import 'package:http/http.dart' as http;

import '../app/mobile_app_state.dart';
import '../bridge/watermark_models.dart';

class CloudAccountClient {
  CloudAccountClient({
    required String baseUrl,
    http.Client? client,
    Duration timeout = const Duration(seconds: 10),
  }) : _baseUri = Uri.parse(baseUrl),
       _client = client ?? http.Client(),
       _timeout = timeout;

  final Uri _baseUri;
  final http.Client _client;
  final Duration _timeout;

  String get baseUrl => _baseUri.toString();

  Future<AuthChallengeResponse> createAuthChallenge({
    required String identifier,
    required String clientDeviceId,
  }) async {
    final endpoint = _baseUri.resolve('/v1/auth/challenges');
    final response = await _client
        .post(
          endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode({
            'identifier': identifier,
            'purpose': 'register_or_login',
            'clientDeviceId': clientDeviceId,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create auth challenge failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return AuthChallengeResponse.fromJson(body);
  }

  Future<CloudAccountSession> continueWithAccount(
    ContinueAccountRequest request,
  ) async {
    final endpoint = _baseUri.resolve('/v1/auth/sessions');
    final response = await _client
        .post(
          endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'login account failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudAccountSession.fromJson(body);
  }

  Future<List<AccountDevice>> listDevices({required String accessToken}) async {
    final endpoint = _baseUri.resolve('/v1/devices');
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'list devices failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    final rows = body['devices'] as List<Object?>? ?? const [];
    return rows
        .whereType<Map<String, Object?>>()
        .map(AccountDevice.fromJson)
        .toList(growable: false);
  }

  Future<AccountDevice> updateDeviceName({
    required String accessToken,
    required String deviceId,
    required String name,
  }) async {
    final endpoint = _baseUri.resolve('/v1/devices/$deviceId');
    final response = await _client
        .patch(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({'name': name}),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'update device failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return AccountDevice.fromJson(body);
  }

  Future<RevokeDeviceResponse> revokeDevice({
    required String accessToken,
    required String deviceId,
  }) async {
    final endpoint = _baseUri.resolve('/v1/devices/$deviceId');
    final response = await _client
        .delete(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'revoke device failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return RevokeDeviceResponse.fromJson(body);
  }

  Future<CloudAccountSession> refreshSession({
    required String refreshToken,
    required String deviceId,
  }) async {
    final endpoint = _baseUri.resolve('/v1/auth/refresh');
    final response = await _client
        .post(
          endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode({
            'refreshToken': refreshToken,
            'deviceId': deviceId,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'refresh auth session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudAccountSession.fromJson(body);
  }

  Future<CloudAccountSnapshot> me({required String accessToken}) async {
    final endpoint = _baseUri.resolve('/v1/me');
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'get current account failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudAccountSnapshot.fromJson(body);
  }

  Future<void> logout({
    required String refreshToken,
    required String deviceId,
  }) async {
    if (refreshToken.trim().isEmpty || deviceId.trim().isEmpty) {
      return;
    }
    final endpoint = _baseUri.resolve('/v1/auth/logout');
    final response = await _client
        .post(
          endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode({
            'refreshToken': refreshToken,
            'deviceId': deviceId,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'logout auth session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
  }

  Future<SyncPreferencesResponse> updateSyncPreferences({
    required String accessToken,
    required bool autoSyncEnabled,
  }) async {
    final endpoint = _baseUri.resolve('/v1/me/sync-preferences');
    final response = await _client
        .patch(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({
            'autoSyncEnabled': autoSyncEnabled,
            'reason': autoSyncEnabled ? 'user_resumed' : 'user_paused',
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'update sync preferences failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return SyncPreferencesResponse.fromJson(body);
  }

  Future<BillingPaymentSession> createBillingPaymentSession({
    required String accessToken,
    required String accountId,
    required String workspaceId,
    required String planCode,
    String billingCycle = 'monthly',
    String preferredProvider = 'wechat_pay',
  }) async {
    final endpoint = _baseUri.resolve('/v1/billing/payment-sessions');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({
            'accountId': accountId,
            'workspaceId': workspaceId,
            'planCode': planCode,
            'billingCycle': billingCycle,
            'preferredProvider': preferredProvider,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create billing payment session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return BillingPaymentSession.fromJson(body);
  }

  Future<CloudEntitlement> getCurrentEntitlement({
    required String accessToken,
  }) async {
    final endpoint = _baseUri.resolve('/v1/entitlements/current');
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'refresh entitlement failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudEntitlement.fromJson(body);
  }

  Future<VideoFingerprintNotaryReceipt> createVideoFingerprintNotary({
    required String accessToken,
    required Map<String, Object?> request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/video-fingerprints/notaries');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create L2 video fingerprint notary failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return VideoFingerprintNotaryReceipt.fromJson(body);
  }

  Future<CloudVideoTaskRecord> getCloudVideoTask({
    required String accessToken,
    required String taskId,
  }) async {
    final endpoint = _baseUri.resolve('/v1/video-tasks/${taskId.trim()}');
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'get L3 video task failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudVideoTaskRecord.fromJson(body);
  }

  Future<CloudVideoTaskObjectUploadAuthorization>
  createCloudVideoTaskObjectUploadAuthorization({
    required String accessToken,
    required String workspaceId,
    required String creatorProfileId,
    required String sha256,
    required int bytes,
    String contentType = 'video/mp4',
    String objectKind = 'l3_user_object_upload_proxy',
    int ttlSeconds = 900,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/video-tasks/object-upload-authorizations',
    );
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({
            'workspaceId': workspaceId,
            'creatorProfileId': creatorProfileId,
            'sha256': sha256,
            'bytes': bytes,
            'contentType': contentType,
            'objectKind': objectKind,
            'ttlSeconds': ttlSeconds,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create L3 upload authorization failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudVideoTaskObjectUploadAuthorization.fromJson(body);
  }

  Future<CloudVideoTaskObjectUploadResponse> uploadCloudVideoTaskObjectBytes({
    required String uploadToken,
    required List<int> bytes,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/video-object-store/upload?token=${Uri.encodeQueryComponent(uploadToken.trim())}',
    );
    final response = await _client.put(endpoint, body: bytes).timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'upload L3 object failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudVideoTaskObjectUploadResponse.fromJson(body);
  }

  Future<CloudVideoTaskRecord> createCloudVideoTask({
    required String accessToken,
    required Map<String, Object?> request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/video-tasks');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create L3 video task failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudVideoTaskRecord.fromJson(body);
  }

  Future<CloudVideoTaskDownloadAuthorization>
  createCloudVideoTaskDownloadAuthorization({
    required String accessToken,
    required String taskId,
    int ttlSeconds = 900,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/video-tasks/${taskId.trim()}/output-download-authorizations',
    );
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({'ttlSeconds': ttlSeconds}),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create L3 download authorization failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudVideoTaskDownloadAuthorization.fromJson(body);
  }

  Future<List<int>> downloadCloudVideoTaskOutput({
    required String accessToken,
    required String taskId,
    required String downloadToken,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/video-tasks/${taskId.trim()}/output-download?token=${Uri.encodeQueryComponent(downloadToken)}',
    );
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'download L3 output failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    return response.bodyBytes;
  }

  Future<PublicRightsQueryResponse> getPublicRights({
    required String watermarkUid,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/public/rights/${Uri.encodeComponent(watermarkUid.trim())}',
    );
    final response = await _client.get(endpoint).timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'public rights query failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return PublicRightsQueryResponse.fromJson(body);
  }

  Future<Map<String, Object?>> getPublicRightsMetadata({
    required String watermarkUid,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/public/rights/${Uri.encodeComponent(watermarkUid.trim())}/metadata',
    );
    final response = await _client.get(endpoint).timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'public rights metadata export failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    return jsonDecode(response.body) as Map<String, Object?>;
  }

  Future<BillingPaymentSessionStatus> getBillingPaymentSessionStatus({
    required String accessToken,
    required String paymentSessionId,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/billing/payment-sessions/$paymentSessionId',
    );
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'get billing payment session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return BillingPaymentSessionStatus.fromJson(body);
  }

  Future<BillingPaymentSessionReconcileResult> reconcileBillingPaymentSession({
    required String accessToken,
    required String paymentSessionId,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/billing/payment-sessions/$paymentSessionId/reconcile',
    );
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: '{}',
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'reconcile billing payment session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return BillingPaymentSessionReconcileResult.fromJson(body);
  }

  Future<ReportPurchaseSession> createReportPurchaseSession({
    required String accessToken,
    required String accountId,
    required String workspaceId,
    required String creatorProfileId,
    required String vaultRecordId,
    required String productCode,
    String preferredProvider = 'fixture',
  }) async {
    final endpoint = _baseUri.resolve('/v1/billing/report-purchase-sessions');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode({
            'accountId': accountId,
            'workspaceId': workspaceId,
            'creatorProfileId': creatorProfileId,
            'vaultRecordId': vaultRecordId,
            'productCode': productCode,
            'preferredProvider': preferredProvider,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'create report purchase session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return ReportPurchaseSession.fromJson(body);
  }

  Future<ReportPurchaseSessionStatus> getReportPurchaseSessionStatus({
    required String accessToken,
    required String paymentSessionId,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/billing/report-purchase-sessions/$paymentSessionId',
    );
    final response = await _client
        .get(endpoint, headers: {'authorization': 'Bearer $accessToken'})
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'get report purchase session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return ReportPurchaseSessionStatus.fromJson(body);
  }

  Future<ReportPurchaseSessionReconcileResult> reconcileReportPurchaseSession({
    required String accessToken,
    required String paymentSessionId,
  }) async {
    final endpoint = _baseUri.resolve(
      '/v1/billing/report-purchase-sessions/$paymentSessionId/reconcile',
    );
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: '{}',
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'reconcile report purchase session failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return ReportPurchaseSessionReconcileResult.fromJson(body);
  }

  Future<WatermarkIdRegistryResult> reserveWatermarkId({
    required String accessToken,
    required WatermarkIdReserveRequest request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/watermark-ids/reserve');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'reserve watermark id failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return WatermarkIdRegistryResult.fromJson(body);
  }

  Future<WatermarkIdRegistryResult> confirmWatermarkId({
    required String accessToken,
    required WatermarkIdConfirmRequest request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/watermark-ids/confirm');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'confirm watermark id failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return WatermarkIdRegistryResult.fromJson(body);
  }

  Future<WatermarkIdRegistryResult> reconcileWatermarkId({
    required String accessToken,
    required WatermarkIdReconcileRequest request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/watermark-ids/reconcile');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'reconcile watermark id failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return WatermarkIdRegistryResult.fromJson(body);
  }

  Future<WatermarkIdReissueResult> reissueWatermarkId({
    required String accessToken,
    required WatermarkIdReissueRequest request,
  }) async {
    final endpoint = _baseUri.resolve('/v1/watermark-ids/reissue');
    final response = await _client
        .post(
          endpoint,
          headers: {
            'authorization': 'Bearer $accessToken',
            'content-type': 'application/json',
          },
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'reissue watermark id failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return WatermarkIdReissueResult.fromJson(body);
  }
}

class WatermarkIdReserveRequest {
  const WatermarkIdReserveRequest({
    required this.requestId,
    required this.workspaceId,
    required this.creatorProfileId,
    required this.mediaType,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.parentWatermarkUid,
    required this.revision,
    this.originalHash,
  });

  final String requestId;
  final String workspaceId;
  final String creatorProfileId;
  final String mediaType;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? parentWatermarkUid;
  final int revision;
  final String? originalHash;

  Map<String, Object?> toJson() => {
    'requestId': requestId,
    'workspaceId': workspaceId,
    'creatorProfileId': creatorProfileId,
    'mediaType': mediaType,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'parentWatermarkUid': parentWatermarkUid,
    'revision': revision,
    'originalHash': originalHash,
  };
}

class WatermarkIdConfirmRequest {
  const WatermarkIdConfirmRequest({
    required this.workspaceId,
    required this.creatorProfileId,
    required this.watermarkUid,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.originalHash,
    this.protectedCopyHash,
    required this.writeVerificationStatus,
  });

  final String workspaceId;
  final String creatorProfileId;
  final String watermarkUid;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? originalHash;
  final String? protectedCopyHash;
  final String writeVerificationStatus;

  Map<String, Object?> toJson() => {
    'workspaceId': workspaceId,
    'creatorProfileId': creatorProfileId,
    'watermarkUid': watermarkUid,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'originalHash': originalHash,
    'protectedCopyHash': protectedCopyHash,
    'writeVerificationStatus': writeVerificationStatus,
  };
}

class WatermarkIdReconcileRequest {
  const WatermarkIdReconcileRequest({
    required this.workspaceId,
    required this.creatorProfileId,
    required this.watermarkUid,
    required this.mediaType,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.parentWatermarkUid,
    required this.revision,
    this.originalHash,
    this.protectedCopyHash,
    this.writeVerificationStatus,
  });

  final String workspaceId;
  final String creatorProfileId;
  final String watermarkUid;
  final String mediaType;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? parentWatermarkUid;
  final int revision;
  final String? originalHash;
  final String? protectedCopyHash;
  final String? writeVerificationStatus;

  Map<String, Object?> toJson() => {
    'workspaceId': workspaceId,
    'creatorProfileId': creatorProfileId,
    'watermarkUid': watermarkUid,
    'mediaType': mediaType,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'parentWatermarkUid': parentWatermarkUid,
    'revision': revision,
    'originalHash': originalHash,
    'protectedCopyHash': protectedCopyHash,
    'writeVerificationStatus': writeVerificationStatus,
  };
}

class WatermarkIdReissueRequest {
  const WatermarkIdReissueRequest({
    required this.workspaceId,
    required this.creatorProfileId,
    required this.previousWatermarkUid,
    required this.mediaType,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.parentWatermarkUid,
    required this.revision,
    required this.reason,
    this.originalHash,
  });

  final String workspaceId;
  final String creatorProfileId;
  final String previousWatermarkUid;
  final String mediaType;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? parentWatermarkUid;
  final int revision;
  final String reason;
  final String? originalHash;

  Map<String, Object?> toJson() => {
    'workspaceId': workspaceId,
    'creatorProfileId': creatorProfileId,
    'previousWatermarkUid': previousWatermarkUid,
    'mediaType': mediaType,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'parentWatermarkUid': parentWatermarkUid,
    'revision': revision,
    'reason': reason,
    'originalHash': originalHash,
  };
}

class WatermarkIdRegistryResult {
  const WatermarkIdRegistryResult({
    required this.registryId,
    required this.watermarkUid,
    required this.watermarkIdIssueMode,
    required this.registryStatus,
    required this.registryReceipt,
    required this.registryProofHash,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    this.parentWatermarkUid,
    required this.revision,
    required this.issuedAt,
    required this.updatedAt,
  });

  factory WatermarkIdRegistryResult.fromJson(Map<String, Object?> json) {
    return WatermarkIdRegistryResult(
      registryId: json['registryId'] as String? ?? '',
      watermarkUid: json['watermarkUid'] as String? ?? '',
      watermarkIdIssueMode:
          json['watermarkIdIssueMode'] as String? ?? 'offline_generated',
      registryStatus:
          json['registryStatus'] as String? ?? 'pending_registration',
      registryReceipt: json['registryReceipt'] as String? ?? '',
      registryProofHash: json['registryProofHash'] as String? ?? '',
      payloadProtocolVersion:
          (json['payloadProtocolVersion'] as num?)?.toInt() ?? 2,
      payloadBytesLength: (json['payloadBytesLength'] as num?)?.toInt() ?? 119,
      parentWatermarkUid: json['parentWatermarkUid'] as String?,
      revision: (json['revision'] as num?)?.toInt() ?? 1,
      issuedAt: json['issuedAt'] as String? ?? '',
      updatedAt: json['updatedAt'] as String? ?? '',
    );
  }

  final String registryId;
  final String watermarkUid;
  final String watermarkIdIssueMode;
  final String registryStatus;
  final String registryReceipt;
  final String registryProofHash;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String? parentWatermarkUid;
  final int revision;
  final String issuedAt;
  final String updatedAt;

  WatermarkRegistryDraft toDraft() {
    return WatermarkRegistryDraft(
      watermarkUid: watermarkUid,
      watermarkIdIssueMode: watermarkIdIssueMode,
      registryStatus: registryStatus,
      registryReceipt: registryReceipt,
      registryProofHash: registryProofHash,
      payloadProtocolVersion: payloadProtocolVersion,
      payloadBytesLength: payloadBytesLength,
      parentWatermarkUid: parentWatermarkUid,
      revision: revision,
    );
  }
}

class WatermarkIdReissueResult {
  const WatermarkIdReissueResult({
    required this.jobId,
    required this.previousWatermarkUid,
    required this.replacement,
  });

  factory WatermarkIdReissueResult.fromJson(Map<String, Object?> json) {
    return WatermarkIdReissueResult(
      jobId: json['jobId'] as String? ?? '',
      previousWatermarkUid: json['previousWatermarkUid'] as String? ?? '',
      replacement: WatermarkIdRegistryResult.fromJson(
        json['replacement'] as Map<String, Object?>? ?? const {},
      ),
    );
  }

  final String jobId;
  final String previousWatermarkUid;
  final WatermarkIdRegistryResult replacement;
}

class PublicRightsQueryResponse {
  const PublicRightsQueryResponse({
    required this.watermarkUid,
    required this.scanStatus,
    required this.registry,
    required this.rightsManifest,
    required this.publicMetadata,
    required this.trainingPermission,
    required this.warnings,
    required this.resolvedAt,
  });

  final String watermarkUid;
  final String scanStatus;
  final PublicRightsRegistrySnapshot registry;
  final RightsManifestResponse? rightsManifest;
  final PublicRightsMetadata publicMetadata;
  final PublicTrainingPermissionSnapshot trainingPermission;
  final List<String> warnings;
  final DateTime? resolvedAt;

  factory PublicRightsQueryResponse.fromJson(Map<String, Object?> json) {
    final manifestJson = json['rightsManifest'];
    return PublicRightsQueryResponse(
      watermarkUid: json['watermarkUid'] as String? ?? '',
      scanStatus: json['scanStatus'] as String? ?? 'unknown',
      registry: PublicRightsRegistrySnapshot.fromJson(
        json['registry'] as Map<String, Object?>? ?? const {},
      ),
      rightsManifest: manifestJson is Map<String, Object?>
          ? RightsManifestResponse.fromJson(manifestJson)
          : null,
      publicMetadata: PublicRightsMetadata.fromJson(
        json['publicMetadata'] as Map<String, Object?>? ?? const {},
      ),
      trainingPermission: PublicTrainingPermissionSnapshot.fromJson(
        json['trainingPermission'] as Map<String, Object?>? ?? const {},
      ),
      warnings: (json['warnings'] as List<Object?>? ?? const [])
          .whereType<String>()
          .toList(growable: false),
      resolvedAt: _dateTimeOrNull(json['resolvedAt']),
    );
  }
}

class PublicRightsRegistrySnapshot {
  const PublicRightsRegistrySnapshot({
    required this.registryStatus,
    required this.payloadAuthStatus,
    required this.watermarkIdIssueMode,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    required this.anchorProtocol,
    required this.mediaPayloadRole,
    required this.rightsSource,
  });

  final String registryStatus;
  final String payloadAuthStatus;
  final String watermarkIdIssueMode;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String anchorProtocol;
  final String mediaPayloadRole;
  final String rightsSource;

  factory PublicRightsRegistrySnapshot.fromJson(Map<String, Object?> json) =>
      PublicRightsRegistrySnapshot(
        registryStatus: json['registryStatus'] as String? ?? '',
        payloadAuthStatus: json['payloadAuthStatus'] as String? ?? '',
        watermarkIdIssueMode: json['watermarkIdIssueMode'] as String? ?? '',
        payloadProtocolVersion: _intOrZero(json['payloadProtocolVersion']),
        payloadBytesLength: _intOrZero(json['payloadBytesLength']),
        anchorProtocol: json['anchorProtocol'] as String? ?? '',
        mediaPayloadRole: json['mediaPayloadRole'] as String? ?? '',
        rightsSource: json['rightsSource'] as String? ?? '',
      );
}

class RightsManifestResponse {
  const RightsManifestResponse({
    required this.rightsManifestId,
    required this.manifestVersion,
    required this.status,
    required this.trainingPolicy,
    required this.manifestSha256,
    required this.effectiveAt,
  });

  final String rightsManifestId;
  final int manifestVersion;
  final String status;
  final String trainingPolicy;
  final String manifestSha256;
  final DateTime? effectiveAt;

  factory RightsManifestResponse.fromJson(Map<String, Object?> json) =>
      RightsManifestResponse(
        rightsManifestId: json['rightsManifestId'] as String? ?? '',
        manifestVersion: _intOrZero(json['manifestVersion']),
        status: json['status'] as String? ?? '',
        trainingPolicy: json['trainingPolicy'] as String? ?? '',
        manifestSha256: json['manifestSha256'] as String? ?? '',
        effectiveAt: _dateTimeOrNull(json['effectiveAt']),
      );
}

class PublicRightsMetadata {
  const PublicRightsMetadata({required this.consistency});

  final String consistency;

  factory PublicRightsMetadata.fromJson(Map<String, Object?> json) =>
      PublicRightsMetadata(
        consistency: json['consistency'] as String? ?? 'unknown',
      );
}

class PublicTrainingPermissionSnapshot {
  const PublicTrainingPermissionSnapshot({
    required this.policy,
    required this.label,
    required this.effectiveSource,
    required this.legalConclusion,
  });

  final String policy;
  final String label;
  final String effectiveSource;
  final bool legalConclusion;

  factory PublicTrainingPermissionSnapshot.fromJson(
    Map<String, Object?> json,
  ) => PublicTrainingPermissionSnapshot(
    policy: json['policy'] as String? ?? 'not_declared',
    label: json['label'] as String? ?? '未声明',
    effectiveSource: json['effectiveSource'] as String? ?? '',
    legalConclusion: json['legalConclusion'] as bool? ?? false,
  );
}

class ContinueAccountRequest {
  const ContinueAccountRequest({
    required this.identifier,
    required this.password,
    required this.verificationCode,
    required this.device,
    required this.localCreatorProfile,
    this.challengeId,
  });

  final String identifier;
  final String password;
  final String verificationCode;
  final ContinueAccountDevice device;
  final ContinueAccountCreatorProfile localCreatorProfile;
  final String? challengeId;

  Map<String, Object?> toJson() {
    return {
      'identifier': identifier,
      'password': password,
      if (challengeId != null) 'challengeId': challengeId,
      'verificationCode': verificationCode,
      'device': device.toJson(),
      'localCreatorProfile': localCreatorProfile.toJson(),
    };
  }
}

class AuthChallengeResponse {
  const AuthChallengeResponse({
    required this.challengeId,
    required this.deliveryChannel,
    required this.expiresAt,
    required this.message,
    this.fixtureCode,
  });

  factory AuthChallengeResponse.fromJson(Map<String, Object?> json) {
    return AuthChallengeResponse(
      challengeId: json['challengeId'] as String? ?? '',
      deliveryChannel: json['deliveryChannel'] as String? ?? '',
      expiresAt:
          DateTime.tryParse(json['expiresAt'] as String? ?? '') ??
          DateTime.now().add(const Duration(minutes: 10)),
      message: json['message'] as String? ?? '',
      fixtureCode: json['fixtureCode'] as String?,
    );
  }

  final String challengeId;
  final String deliveryChannel;
  final DateTime expiresAt;
  final String message;
  final String? fixtureCode;
}

class AccountDevice {
  const AccountDevice({
    required this.id,
    required this.clientDeviceId,
    required this.name,
    required this.platform,
    required this.appVersion,
    required this.registered,
    required this.autoSyncEnabled,
    required this.isCurrent,
    required this.activeSessionCount,
    required this.createdAt,
    required this.updatedAt,
    this.lastSeenAt,
  });

  factory AccountDevice.fromJson(Map<String, Object?> json) {
    return AccountDevice(
      id: json['id'] as String? ?? '',
      clientDeviceId: json['clientDeviceId'] as String? ?? '',
      name: json['name'] as String? ?? '当前设备',
      platform: json['platform'] as String? ?? 'unknown',
      appVersion: json['appVersion'] as String? ?? '',
      registered: json['registered'] as bool? ?? false,
      autoSyncEnabled: json['autoSyncEnabled'] as bool? ?? false,
      isCurrent: json['isCurrent'] as bool? ?? false,
      activeSessionCount: (json['activeSessionCount'] as num?)?.toInt() ?? 0,
      lastSeenAt: DateTime.tryParse(json['lastSeenAt'] as String? ?? ''),
      createdAt:
          DateTime.tryParse(json['createdAt'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
      updatedAt:
          DateTime.tryParse(json['updatedAt'] as String? ?? '') ??
          DateTime.fromMillisecondsSinceEpoch(0),
    );
  }

  final String id;
  final String clientDeviceId;
  final String name;
  final String platform;
  final String appVersion;
  final bool registered;
  final bool autoSyncEnabled;
  final bool isCurrent;
  final int activeSessionCount;
  final DateTime? lastSeenAt;
  final DateTime createdAt;
  final DateTime updatedAt;
}

class RevokeDeviceResponse {
  const RevokeDeviceResponse({
    required this.ok,
    required this.deviceId,
    required this.revokedSessionCount,
  });

  factory RevokeDeviceResponse.fromJson(Map<String, Object?> json) {
    return RevokeDeviceResponse(
      ok: json['ok'] as bool? ?? false,
      deviceId: json['deviceId'] as String? ?? '',
      revokedSessionCount: (json['revokedSessionCount'] as num?)?.toInt() ?? 0,
    );
  }

  final bool ok;
  final String deviceId;
  final int revokedSessionCount;
}

class ContinueAccountDevice {
  const ContinueAccountDevice({
    required this.clientDeviceId,
    required this.name,
    required this.platform,
    required this.appVersion,
    this.publicKey,
  });

  final String clientDeviceId;
  final String name;
  final String platform;
  final String appVersion;
  final String? publicKey;

  Map<String, Object?> toJson() {
    return {
      'clientDeviceId': clientDeviceId,
      'name': name,
      'platform': platform,
      'appVersion': appVersion,
      'publicKey': publicKey,
    };
  }
}

class ContinueAccountCreatorProfile {
  const ContinueAccountCreatorProfile({
    required this.displayName,
    required this.creatorSeedRef,
    required this.seedEnvelopeVersion,
  });

  final String displayName;
  final String creatorSeedRef;
  final int seedEnvelopeVersion;

  Map<String, Object?> toJson() {
    return {
      'displayName': displayName,
      'creatorSeedRef': creatorSeedRef,
      'seedEnvelopeVersion': seedEnvelopeVersion,
    };
  }
}

class CloudAccountSession {
  const CloudAccountSession({
    required this.accessToken,
    required this.refreshToken,
    required this.account,
    required this.workspace,
    required this.device,
    required this.creatorProfile,
    required this.entitlement,
    required this.syncPolicy,
    required this.cloudVaultCursor,
  });

  factory CloudAccountSession.fromJson(Map<String, Object?> json) {
    return CloudAccountSession(
      accessToken: json['accessToken'] as String? ?? '',
      refreshToken: json['refreshToken'] as String? ?? '',
      account: CloudAccount.fromJson(
        json['account'] as Map<String, Object?>? ?? const {},
      ),
      workspace: CloudWorkspace.fromJson(
        json['workspace'] as Map<String, Object?>? ?? const {},
      ),
      device: CloudDevice.fromJson(
        json['device'] as Map<String, Object?>? ?? const {},
      ),
      creatorProfile: CloudCreatorProfile.fromJson(
        json['creatorProfile'] as Map<String, Object?>? ?? const {},
      ),
      entitlement: CloudEntitlement.fromJson(
        json['entitlement'] as Map<String, Object?>? ?? const {},
      ),
      syncPolicy: json['syncPolicy'] as String? ?? '',
      cloudVaultCursor: json['cloudVaultCursor'] as String?,
    );
  }

  final String accessToken;
  final String refreshToken;
  final CloudAccount account;
  final CloudWorkspace workspace;
  final CloudDevice device;
  final CloudCreatorProfile creatorProfile;
  final CloudEntitlement entitlement;
  final String syncPolicy;
  final String? cloudVaultCursor;

  SyncProfile applyTo(SyncProfile current, {required DateTime now}) {
    final normalizedSyncPolicy = syncPolicy.trim().isNotEmpty
        ? syncPolicy.trim()
        : _syncPolicyForFeatures(entitlement.features);
    return current.copyWith(
      mode: normalizedSyncPolicy == 'auto_cloud_vault'
          ? SyncTransportMode.cloud
          : SyncTransportMode.localOnly,
      status: SyncConnectionStatus.connected,
      accountId: account.id,
      accountLabel: account.displayName,
      authToken: accessToken,
      refreshToken: refreshToken,
      workspaceId: workspace.id,
      workspaceName: workspace.name,
      deviceId: device.id,
      deviceRegistered: device.registered,
      creatorProfileId: creatorProfile.id,
      creatorDisplayName: creatorProfile.displayName,
      creatorProfileSynced: true,
      entitlementId: entitlement.id,
      entitlementLabel: entitlement.planLabel,
      entitlementStatus: entitlement.status,
      entitlementPlanCode: entitlement.planCode,
      entitlementPlanKey: entitlement.planKey,
      entitlementFeatures: entitlement.features,
      entitlementLastCheckedAt: now,
      syncPolicy: normalizedSyncPolicy,
      lastRemotePullCursor: cloudVaultCursor,
      updatedAt: now,
      clearLastError: true,
    );
  }
}

class CloudAccountSnapshot {
  const CloudAccountSnapshot({
    required this.account,
    required this.workspace,
    required this.device,
    required this.creatorProfile,
    required this.entitlement,
    required this.syncPolicy,
    required this.cloudVaultCursor,
  });

  factory CloudAccountSnapshot.fromJson(Map<String, Object?> json) {
    return CloudAccountSnapshot(
      account: CloudAccount.fromJson(
        json['account'] as Map<String, Object?>? ?? const {},
      ),
      workspace: CloudWorkspace.fromJson(
        json['workspace'] as Map<String, Object?>? ?? const {},
      ),
      device: CloudDevice.fromJson(
        json['device'] as Map<String, Object?>? ?? const {},
      ),
      creatorProfile: CloudCreatorProfile.fromJson(
        json['creatorProfile'] as Map<String, Object?>? ?? const {},
      ),
      entitlement: CloudEntitlement.fromJson(
        json['entitlement'] as Map<String, Object?>? ?? const {},
      ),
      syncPolicy: json['syncPolicy'] as String? ?? '',
      cloudVaultCursor: json['cloudVaultCursor'] as String?,
    );
  }

  final CloudAccount account;
  final CloudWorkspace workspace;
  final CloudDevice device;
  final CloudCreatorProfile creatorProfile;
  final CloudEntitlement entitlement;
  final String syncPolicy;
  final String? cloudVaultCursor;
}

class SyncPreferencesResponse {
  const SyncPreferencesResponse({
    required this.syncPolicy,
    required this.autoSyncEnabled,
    required this.cloudVaultCursor,
    required this.entitlement,
  });

  factory SyncPreferencesResponse.fromJson(Map<String, Object?> json) {
    return SyncPreferencesResponse(
      syncPolicy: json['syncPolicy'] as String? ?? '',
      autoSyncEnabled: json['autoSyncEnabled'] as bool? ?? false,
      cloudVaultCursor: json['cloudVaultCursor'] as String?,
      entitlement: CloudEntitlement.fromJson(
        json['entitlement'] as Map<String, Object?>? ?? const {},
      ),
    );
  }

  final String syncPolicy;
  final bool autoSyncEnabled;
  final String? cloudVaultCursor;
  final CloudEntitlement entitlement;

  SyncProfile applyTo(SyncProfile current, {required DateTime now}) {
    final normalizedSyncPolicy = syncPolicy.trim().isNotEmpty
        ? syncPolicy.trim()
        : _syncPolicyForFeaturesAndPreference(
            entitlement.features,
            autoSyncEnabled,
          );
    return current.copyWith(
      mode: normalizedSyncPolicy == 'auto_cloud_vault'
          ? SyncTransportMode.cloud
          : SyncTransportMode.localOnly,
      entitlementId: entitlement.id,
      entitlementLabel: entitlement.planLabel,
      entitlementStatus: entitlement.status,
      entitlementPlanCode: entitlement.planCode,
      entitlementPlanKey: entitlement.planKey,
      entitlementFeatures: entitlement.features,
      entitlementLastCheckedAt: now,
      syncPolicy: normalizedSyncPolicy,
      lastRemotePullCursor: cloudVaultCursor,
      updatedAt: now,
      clearLastError: true,
    );
  }
}

class CloudAccount {
  const CloudAccount({required this.id, required this.displayName});

  factory CloudAccount.fromJson(Map<String, Object?> json) {
    return CloudAccount(
      id: json['id'] as String? ?? '',
      displayName: json['displayName'] as String? ?? '',
    );
  }

  final String id;
  final String displayName;
}

class CloudWorkspace {
  const CloudWorkspace({required this.id, required this.name});

  factory CloudWorkspace.fromJson(Map<String, Object?> json) {
    return CloudWorkspace(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '个人空间',
    );
  }

  final String id;
  final String name;
}

class CloudDevice {
  const CloudDevice({required this.id, required this.registered});

  factory CloudDevice.fromJson(Map<String, Object?> json) {
    return CloudDevice(
      id: json['id'] as String? ?? '',
      registered: json['registered'] as bool? ?? false,
    );
  }

  final String id;
  final bool registered;
}

class CloudCreatorProfile {
  const CloudCreatorProfile({
    required this.id,
    required this.displayName,
    required this.isDefault,
  });

  factory CloudCreatorProfile.fromJson(Map<String, Object?> json) {
    return CloudCreatorProfile(
      id: json['id'] as String? ?? '',
      displayName: json['displayName'] as String? ?? '',
      isDefault: json['isDefault'] as bool? ?? true,
    );
  }

  final String id;
  final String displayName;
  final bool isDefault;
}

class CloudEntitlement {
  const CloudEntitlement({
    required this.id,
    required this.planName,
    required this.planCode,
    required this.planKey,
    required this.planLabel,
    required this.status,
    required this.features,
  });

  factory CloudEntitlement.fromJson(Map<String, Object?> json) {
    final planCode = json['planCode'] as String? ?? 'free';
    final features = _decodeFeatureMap(json['features']);
    final planKey = normalizeEntitlementPlanKey(
      planKey: json['planKey'] as String?,
      planCode: planCode,
      features: features,
    );
    final planLabel = (json['planLabel'] as String?)?.trim();
    return CloudEntitlement(
      id: json['id'] as String? ?? '',
      planName: json['planName'] as String?,
      planCode: planCode,
      planKey: planKey,
      planLabel: planLabel?.isNotEmpty == true
          ? planLabel!
          : entitlementPlanLabel(planKey),
      status: _entitlementStatusFromName(json['status'] as String? ?? 'free'),
      features: features,
    );
  }

  final String id;
  final String? planName;
  final String planCode;
  final String planKey;
  final String planLabel;
  final EntitlementStatus status;
  final Map<String, bool> features;

  SyncProfile applyTo(SyncProfile current, {required DateTime now}) {
    final syncPolicy = _syncPolicyForFeaturesAndPreference(
      features,
      current.syncPolicy != 'manual_local_only',
    );
    return current.copyWith(
      mode: syncPolicy == 'auto_cloud_vault'
          ? SyncTransportMode.cloud
          : SyncTransportMode.localOnly,
      entitlementId: id,
      entitlementLabel: planLabel,
      entitlementStatus: status,
      entitlementPlanCode: planCode,
      entitlementPlanKey: planKey,
      entitlementFeatures: features,
      entitlementLastCheckedAt: now,
      syncPolicy: syncPolicy,
      updatedAt: now,
      clearLastError: true,
    );
  }
}

class CloudVideoTaskRecord {
  const CloudVideoTaskRecord({
    required this.taskId,
    required this.status,
    required this.capabilityLevel,
    required this.watermarkUid,
    required this.sourceHash,
    required this.durationMs,
    required this.strategyDigest,
    required this.selfCheckThreshold,
    required this.selfCheckConfidence,
    required this.checkedFrames,
    required this.watermarkedMediaHash,
    required this.outputMediaStorageRef,
    required this.outputMediaBytes,
    required this.outputMediaContentType,
    required this.workerReceiptHash,
    required this.serverReceiptSignature,
    required this.usageLedgerId,
    required this.completedAt,
    required this.updatedAt,
  });

  factory CloudVideoTaskRecord.fromJson(Map<String, Object?> json) {
    return CloudVideoTaskRecord(
      taskId: json['taskId'] as String? ?? '',
      status: json['status'] as String? ?? '',
      capabilityLevel: json['capabilityLevel'] as String? ?? '',
      watermarkUid: json['watermarkUid'] as String? ?? '',
      sourceHash: json['sourceHash'] as String? ?? '',
      durationMs: (json['durationMs'] as num?)?.toInt() ?? 0,
      strategyDigest: json['strategyDigest'] as String?,
      selfCheckThreshold: (json['selfCheckThreshold'] as num?)?.toDouble(),
      selfCheckConfidence: (json['selfCheckConfidence'] as num?)?.toDouble(),
      checkedFrames: (json['checkedFrames'] as num?)?.toInt(),
      watermarkedMediaHash: json['watermarkedMediaHash'] as String?,
      outputMediaStorageRef: json['outputMediaStorageRef'] as String?,
      outputMediaBytes: (json['outputMediaBytes'] as num?)?.toInt(),
      outputMediaContentType: json['outputMediaContentType'] as String?,
      workerReceiptHash: json['workerReceiptHash'] as String?,
      serverReceiptSignature: json['serverReceiptSignature'] as String?,
      usageLedgerId: json['usageLedgerId'] as String?,
      completedAt: DateTime.tryParse(json['completedAt'] as String? ?? ''),
      updatedAt: DateTime.tryParse(json['updatedAt'] as String? ?? ''),
    );
  }

  final String taskId;
  final String status;
  final String capabilityLevel;
  final String watermarkUid;
  final String sourceHash;
  final int durationMs;
  final String? strategyDigest;
  final double? selfCheckThreshold;
  final double? selfCheckConfidence;
  final int? checkedFrames;
  final String? watermarkedMediaHash;
  final String? outputMediaStorageRef;
  final int? outputMediaBytes;
  final String? outputMediaContentType;
  final String? workerReceiptHash;
  final String? serverReceiptSignature;
  final String? usageLedgerId;
  final DateTime? completedAt;
  final DateTime? updatedAt;
}

class VideoFingerprintNotaryReceipt {
  const VideoFingerprintNotaryReceipt({
    required this.schemaVersion,
    required this.notaryId,
    required this.watermarkUid,
    required this.sourceHash,
    required this.fingerprintRoot,
    required this.notarizedAt,
    required this.serverReceiptSignature,
    required this.usageLedgerId,
  });

  factory VideoFingerprintNotaryReceipt.fromJson(Map<String, Object?> json) {
    return VideoFingerprintNotaryReceipt(
      schemaVersion: json['schemaVersion'] as String? ?? '',
      notaryId: json['notaryId'] as String? ?? '',
      watermarkUid: json['watermarkUid'] as String? ?? '',
      sourceHash: json['sourceHash'] as String? ?? '',
      fingerprintRoot: json['fingerprintRoot'] as String? ?? '',
      notarizedAt:
          DateTime.tryParse(json['notarizedAt'] as String? ?? '') ??
          DateTime.now(),
      serverReceiptSignature: json['serverReceiptSignature'] as String? ?? '',
      usageLedgerId: json['usageLedgerId'] as String? ?? '',
    );
  }

  final String schemaVersion;
  final String notaryId;
  final String watermarkUid;
  final String sourceHash;
  final String fingerprintRoot;
  final DateTime notarizedAt;
  final String serverReceiptSignature;
  final String usageLedgerId;
}

class CloudVideoTaskObjectUploadAuthorization {
  const CloudVideoTaskObjectUploadAuthorization({
    required this.authorizationId,
    required this.storageRef,
    required this.expectedSha256,
    required this.expectedBytes,
    required this.contentType,
    required this.uploadToken,
    required this.privacyBoundary,
  });

  factory CloudVideoTaskObjectUploadAuthorization.fromJson(
    Map<String, Object?> json,
  ) {
    return CloudVideoTaskObjectUploadAuthorization(
      authorizationId: json['authorizationId'] as String? ?? '',
      storageRef: json['storageRef'] as String? ?? '',
      expectedSha256: json['expectedSha256'] as String? ?? '',
      expectedBytes: (json['expectedBytes'] as num?)?.toInt() ?? 0,
      contentType: json['contentType'] as String? ?? '',
      uploadToken: json['uploadToken'] as String? ?? '',
      privacyBoundary: json['privacyBoundary'] as String? ?? '',
    );
  }

  final String authorizationId;
  final String storageRef;
  final String expectedSha256;
  final int expectedBytes;
  final String contentType;
  final String uploadToken;
  final String privacyBoundary;
}

class CloudVideoTaskObjectUploadResponse {
  const CloudVideoTaskObjectUploadResponse({
    required this.status,
    required this.storageRef,
    required this.sha256,
    required this.bytes,
    required this.contentType,
    required this.privacyBoundary,
  });

  factory CloudVideoTaskObjectUploadResponse.fromJson(
    Map<String, Object?> json,
  ) {
    return CloudVideoTaskObjectUploadResponse(
      status: json['status'] as String? ?? '',
      storageRef: json['storageRef'] as String? ?? '',
      sha256: json['sha256'] as String? ?? '',
      bytes: (json['bytes'] as num?)?.toInt() ?? 0,
      contentType: json['contentType'] as String? ?? '',
      privacyBoundary: json['privacyBoundary'] as String? ?? '',
    );
  }

  final String status;
  final String storageRef;
  final String sha256;
  final int bytes;
  final String contentType;
  final String privacyBoundary;
}

class CloudVideoTaskDownloadAuthorization {
  const CloudVideoTaskDownloadAuthorization({
    required this.taskId,
    required this.status,
    required this.outputMediaBytes,
    required this.outputMediaContentType,
    required this.watermarkedMediaHash,
    required this.workerReceiptHash,
    required this.downloadToken,
  });

  factory CloudVideoTaskDownloadAuthorization.fromJson(
    Map<String, Object?> json,
  ) {
    return CloudVideoTaskDownloadAuthorization(
      taskId: json['taskId'] as String? ?? '',
      status: json['status'] as String? ?? '',
      outputMediaBytes: (json['outputMediaBytes'] as num?)?.toInt() ?? 0,
      outputMediaContentType: json['outputMediaContentType'] as String? ?? '',
      watermarkedMediaHash: json['watermarkedMediaHash'] as String? ?? '',
      workerReceiptHash: json['workerReceiptHash'] as String? ?? '',
      downloadToken: json['downloadToken'] as String? ?? '',
    );
  }

  final String taskId;
  final String status;
  final int outputMediaBytes;
  final String outputMediaContentType;
  final String watermarkedMediaHash;
  final String workerReceiptHash;
  final String downloadToken;
}

class BillingPaymentSession {
  const BillingPaymentSession({
    required this.paymentSessionId,
    required this.provider,
    required this.providerOrderId,
    required this.paymentAction,
    required this.expiresAt,
  });

  factory BillingPaymentSession.fromJson(Map<String, Object?> json) {
    return BillingPaymentSession(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      provider: json['provider'] as String? ?? '',
      providerOrderId: json['providerOrderId'] as String? ?? '',
      paymentAction: BillingPaymentAction.fromJson(
        json['paymentAction'] as Map<String, Object?>? ?? const {},
      ),
      expiresAt: json['expiresAt'] as String? ?? '',
    );
  }

  final String paymentSessionId;
  final String provider;
  final String providerOrderId;
  final BillingPaymentAction paymentAction;
  final String expiresAt;
}

class BillingPaymentAction {
  const BillingPaymentAction({
    required this.type,
    required this.qrCodeUrl,
    required this.h5Url,
  });

  factory BillingPaymentAction.fromJson(Map<String, Object?> json) {
    return BillingPaymentAction(
      type: json['type'] as String? ?? '',
      qrCodeUrl: json['qrCodeUrl'] as String?,
      h5Url: json['h5Url'] as String?,
    );
  }

  final String type;
  final String? qrCodeUrl;
  final String? h5Url;
}

class BillingPaymentSessionStatus {
  const BillingPaymentSessionStatus({
    required this.paymentSessionId,
    required this.provider,
    required this.providerOrderId,
    required this.status,
    required this.planCode,
    required this.billingCycle,
    required this.expiresAt,
    required this.lastCheckedAt,
    required this.nextCheckAfter,
    required this.checkAttempts,
    required this.entitlement,
  });

  factory BillingPaymentSessionStatus.fromJson(Map<String, Object?> json) {
    return BillingPaymentSessionStatus(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      provider: json['provider'] as String? ?? '',
      providerOrderId: json['providerOrderId'] as String? ?? '',
      status: json['status'] as String? ?? 'created',
      planCode: json['planCode'] as String? ?? 'free',
      billingCycle: json['billingCycle'] as String? ?? 'monthly',
      expiresAt: json['expiresAt'] as String? ?? '',
      lastCheckedAt: json['lastCheckedAt'] as String?,
      nextCheckAfter: json['nextCheckAfter'] as String?,
      checkAttempts: json['checkAttempts'] as int? ?? 0,
      entitlement: CloudEntitlement.fromJson(
        json['entitlement'] as Map<String, Object?>? ?? const {},
      ),
    );
  }

  final String paymentSessionId;
  final String provider;
  final String providerOrderId;
  final String status;
  final String planCode;
  final String billingCycle;
  final String expiresAt;
  final String? lastCheckedAt;
  final String? nextCheckAfter;
  final int checkAttempts;
  final CloudEntitlement entitlement;
}

class BillingPaymentSessionReconcileResult {
  const BillingPaymentSessionReconcileResult({
    required this.paymentSessionId,
    required this.status,
    required this.message,
    required this.entitlement,
  });

  factory BillingPaymentSessionReconcileResult.fromJson(
    Map<String, Object?> json,
  ) {
    return BillingPaymentSessionReconcileResult(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      status: json['status'] as String? ?? 'created',
      message: json['message'] as String? ?? '',
      entitlement: CloudEntitlement.fromJson(
        json['entitlement'] as Map<String, Object?>? ?? const {},
      ),
    );
  }

  final String paymentSessionId;
  final String status;
  final String message;
  final CloudEntitlement entitlement;
}

class ReportPurchaseSession {
  const ReportPurchaseSession({
    required this.paymentSessionId,
    required this.provider,
    required this.providerOrderId,
    required this.productCode,
    required this.priceCents,
    required this.currency,
    required this.paymentAction,
    required this.expiresAt,
  });

  factory ReportPurchaseSession.fromJson(Map<String, Object?> json) {
    return ReportPurchaseSession(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      provider: json['provider'] as String? ?? '',
      providerOrderId: json['providerOrderId'] as String? ?? '',
      productCode: json['productCode'] as String? ?? '',
      priceCents: json['priceCents'] as int? ?? 0,
      currency: json['currency'] as String? ?? 'CNY',
      paymentAction: BillingPaymentAction.fromJson(
        json['paymentAction'] as Map<String, Object?>? ?? const {},
      ),
      expiresAt: json['expiresAt'] as String? ?? '',
    );
  }

  final String paymentSessionId;
  final String provider;
  final String providerOrderId;
  final String productCode;
  final int priceCents;
  final String currency;
  final BillingPaymentAction paymentAction;
  final String expiresAt;
}

class ReportPurchaseGrant {
  const ReportPurchaseGrant({
    required this.grantId,
    required this.accountId,
    required this.workspaceId,
    required this.creatorProfileId,
    required this.vaultRecordId,
    required this.productCode,
    required this.priceCents,
    required this.currency,
    required this.status,
    required this.grantedAt,
    required this.revokedAt,
  });

  factory ReportPurchaseGrant.fromJson(Map<String, Object?> json) {
    return ReportPurchaseGrant(
      grantId: json['grantId'] as String? ?? '',
      accountId: json['accountId'] as String? ?? '',
      workspaceId: json['workspaceId'] as String? ?? '',
      creatorProfileId: json['creatorProfileId'] as String? ?? '',
      vaultRecordId: json['vaultRecordId'] as String? ?? '',
      productCode: json['productCode'] as String? ?? '',
      priceCents: json['priceCents'] as int? ?? 0,
      currency: json['currency'] as String? ?? 'CNY',
      status: json['status'] as String? ?? 'created',
      grantedAt: json['grantedAt'] as String? ?? '',
      revokedAt: json['revokedAt'] as String?,
    );
  }

  Map<String, Object?> toJson() {
    return {
      'grantId': grantId,
      'accountId': accountId,
      'workspaceId': workspaceId,
      'creatorProfileId': creatorProfileId,
      'vaultRecordId': vaultRecordId,
      'productCode': productCode,
      'priceCents': priceCents,
      'currency': currency,
      'status': status,
      'grantedAt': grantedAt,
      'revokedAt': revokedAt,
    };
  }

  final String grantId;
  final String accountId;
  final String workspaceId;
  final String creatorProfileId;
  final String vaultRecordId;
  final String productCode;
  final int priceCents;
  final String currency;
  final String status;
  final String grantedAt;
  final String? revokedAt;
}

class ReportPurchaseSessionStatus {
  const ReportPurchaseSessionStatus({
    required this.paymentSessionId,
    required this.provider,
    required this.providerOrderId,
    required this.status,
    required this.productCode,
    required this.priceCents,
    required this.currency,
    required this.vaultRecordId,
    required this.expiresAt,
    required this.lastCheckedAt,
    required this.nextCheckAfter,
    required this.checkAttempts,
    required this.grant,
  });

  factory ReportPurchaseSessionStatus.fromJson(Map<String, Object?> json) {
    return ReportPurchaseSessionStatus(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      provider: json['provider'] as String? ?? '',
      providerOrderId: json['providerOrderId'] as String? ?? '',
      status: json['status'] as String? ?? 'created',
      productCode: json['productCode'] as String? ?? '',
      priceCents: json['priceCents'] as int? ?? 0,
      currency: json['currency'] as String? ?? 'CNY',
      vaultRecordId: json['vaultRecordId'] as String? ?? '',
      expiresAt: json['expiresAt'] as String? ?? '',
      lastCheckedAt: json['lastCheckedAt'] as String?,
      nextCheckAfter: json['nextCheckAfter'] as String?,
      checkAttempts: json['checkAttempts'] as int? ?? 0,
      grant: json['grant'] is Map<String, Object?>
          ? ReportPurchaseGrant.fromJson(json['grant'] as Map<String, Object?>)
          : null,
    );
  }

  final String paymentSessionId;
  final String provider;
  final String providerOrderId;
  final String status;
  final String productCode;
  final int priceCents;
  final String currency;
  final String vaultRecordId;
  final String expiresAt;
  final String? lastCheckedAt;
  final String? nextCheckAfter;
  final int checkAttempts;
  final ReportPurchaseGrant? grant;
}

class ReportPurchaseSessionReconcileResult {
  const ReportPurchaseSessionReconcileResult({
    required this.paymentSessionId,
    required this.status,
    required this.message,
    required this.grant,
  });

  factory ReportPurchaseSessionReconcileResult.fromJson(
    Map<String, Object?> json,
  ) {
    return ReportPurchaseSessionReconcileResult(
      paymentSessionId: json['paymentSessionId'] as String? ?? '',
      status: json['status'] as String? ?? 'created',
      message: json['message'] as String? ?? '',
      grant: json['grant'] is Map<String, Object?>
          ? ReportPurchaseGrant.fromJson(json['grant'] as Map<String, Object?>)
          : null,
    );
  }

  final String paymentSessionId;
  final String status;
  final String message;
  final ReportPurchaseGrant? grant;
}

class CloudAccountException implements Exception {
  const CloudAccountException(this.message);

  final String message;

  @override
  String toString() => message;
}

EntitlementStatus _entitlementStatusFromName(String name) {
  return EntitlementStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => EntitlementStatus.free,
  );
}

Map<String, bool> _decodeFeatureMap(Object? raw) {
  if (raw is! Map<String, Object?>) {
    return const {};
  }
  return {for (final entry in raw.entries) entry.key: entry.value == true};
}

String _syncPolicyForFeatures(Map<String, bool> features) {
  return _syncPolicyForFeaturesAndPreference(features, true);
}

String _syncPolicyForFeaturesAndPreference(
  Map<String, bool> features,
  bool autoSyncEnabled,
) {
  if (features['cloud_sync'] != true) {
    return 'blocked_by_entitlement';
  }
  return autoSyncEnabled ? 'auto_cloud_vault' : 'manual_local_only';
}

String _shortBody(String body) {
  final trimmed = body.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  return trimmed.length > 160 ? '${trimmed.substring(0, 160)}...' : trimmed;
}

int _intOrZero(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? 0;
  return 0;
}

DateTime? _dateTimeOrNull(Object? value) {
  if (value is! String || value.trim().isEmpty) return null;
  return DateTime.tryParse(value);
}
