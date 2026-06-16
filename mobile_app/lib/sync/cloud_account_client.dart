import 'dart:convert';

import 'package:http/http.dart' as http;

import '../app/mobile_app_state.dart';

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

  Future<CloudAccountSession> continueWithAccount(
    ContinueAccountRequest request,
  ) async {
    final endpoint = _baseUri.resolve('/v1/auth/continue');
    final response = await _client
        .post(
          endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode(request.toJson()),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CloudAccountException(
        'continue account failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    return CloudAccountSession.fromJson(body);
  }
}

class ContinueAccountRequest {
  const ContinueAccountRequest({
    required this.identifier,
    required this.verificationCode,
    required this.device,
    required this.localCreatorProfile,
  });

  final String identifier;
  final String verificationCode;
  final ContinueAccountDevice device;
  final ContinueAccountCreatorProfile localCreatorProfile;

  Map<String, Object?> toJson() {
    return {
      'identifier': identifier,
      'verificationCode': verificationCode,
      'device': device.toJson(),
      'localCreatorProfile': localCreatorProfile.toJson(),
    };
  }
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
    );
  }

  final String accessToken;
  final String refreshToken;
  final CloudAccount account;
  final CloudWorkspace workspace;
  final CloudDevice device;
  final CloudCreatorProfile creatorProfile;
  final CloudEntitlement entitlement;

  SyncProfile applyTo(SyncProfile current, {required DateTime now}) {
    return current.copyWith(
      mode: SyncTransportMode.cloud,
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
      entitlementLabel: entitlement.planName ?? entitlement.planCode,
      entitlementStatus: entitlement.status,
      entitlementPlanCode: entitlement.planCode,
      entitlementFeatures: entitlement.features,
      entitlementLastCheckedAt: now,
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
    required this.status,
    required this.features,
  });

  factory CloudEntitlement.fromJson(Map<String, Object?> json) {
    return CloudEntitlement(
      id: json['id'] as String? ?? '',
      planName: json['planName'] as String?,
      planCode: json['planCode'] as String? ?? 'free',
      status: _entitlementStatusFromName(json['status'] as String? ?? 'free'),
      features: _decodeFeatureMap(json['features']),
    );
  }

  final String id;
  final String? planName;
  final String planCode;
  final EntitlementStatus status;
  final Map<String, bool> features;
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

String _shortBody(String body) {
  final trimmed = body.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  return trimmed.length > 160 ? '${trimmed.substring(0, 160)}...' : trimmed;
}
