import 'dart:convert';

import 'package:web/web.dart' as web;

import '../app/mobile_app_state.dart';
import 'vault_store.dart';

class WebProfileVaultStore extends MemoryVaultStore {
  WebProfileVaultStore._();

  static const _syncProfileKey = 'hiddenshield.mobile.sync_profile.v1';

  static Future<WebProfileVaultStore> open() async {
    return WebProfileVaultStore._();
  }

  @override
  Future<SyncProfile> loadSyncProfile() async {
    final raw = web.window.localStorage.getItem(_syncProfileKey);
    if (raw == null || raw.isEmpty) {
      return super.loadSyncProfile();
    }
    try {
      final decoded = jsonDecode(raw) as Map<String, Object?>;
      return _syncProfileFromJson(decoded);
    } catch (_) {
      return super.loadSyncProfile();
    }
  }

  @override
  Future<void> saveSyncProfile(SyncProfile profile) async {
    await super.saveSyncProfile(profile);
    web.window.localStorage.setItem(
      _syncProfileKey,
      jsonEncode(_syncProfileToJson(profile)),
    );
  }
}

Map<String, Object?> _syncProfileToJson(SyncProfile profile) {
  return {
    'mode': profile.mode.name,
    'status': profile.status.name,
    'updatedAt': profile.updatedAt.toIso8601String(),
    'accountId': profile.accountId,
    'accountLabel': profile.accountLabel,
    'authToken': profile.authToken,
    'refreshToken': profile.refreshToken,
    'workspaceId': profile.workspaceId,
    'workspaceName': profile.workspaceName,
    'deviceId': profile.deviceId,
    'deviceName': profile.deviceName,
    'devicePlatform': profile.devicePlatform,
    'deviceRegistered': profile.deviceRegistered,
    'creatorProfileId': profile.creatorProfileId,
    'creatorDisplayName': profile.creatorDisplayName,
    'creatorSeedRef': profile.creatorSeedRef,
    'creatorSeedEnvelopeVersion': profile.creatorSeedEnvelopeVersion,
    'creatorProfileSynced': profile.creatorProfileSynced,
    'onboardingCompleted': profile.onboardingCompleted,
    'entitlementId': profile.entitlementId,
    'entitlementLabel': profile.entitlementLabel,
    'entitlementStatus': profile.entitlementStatus.name,
    'entitlementPlanCode': profile.entitlementPlanCode,
    'entitlementFeatures': profile.entitlementFeatures,
    'entitlementLastCheckedAt': profile.entitlementLastCheckedAt
        ?.toIso8601String(),
    'syncPolicy': profile.syncPolicy,
    'cloudBaseUrl': profile.cloudBaseUrl,
    'lanDebugAddress': profile.lanDebugAddress,
    'lanDebugPairingCode': profile.lanDebugPairingCode,
    'lastError': profile.lastError,
    'lastRemotePullCursor': profile.lastRemotePullCursor,
    'lastSyncAttemptAt': profile.lastSyncAttemptAt?.toIso8601String(),
    'lastSyncSuccessAt': profile.lastSyncSuccessAt?.toIso8601String(),
    'lastSyncFailureAt': profile.lastSyncFailureAt?.toIso8601String(),
    'anonymousFeedbackEnabled': profile.anonymousFeedbackEnabled,
    'experienceImprovementEnabled': profile.experienceImprovementEnabled,
    'anonymousInstallId': profile.anonymousInstallId,
    'anonymousFeedbackLastEventAt': profile.anonymousFeedbackLastEventAt
        ?.toIso8601String(),
    'anonymousFeedbackLastAttemptAt': profile.anonymousFeedbackLastAttemptAt
        ?.toIso8601String(),
    'anonymousFeedbackLastSuccessAt': profile.anonymousFeedbackLastSuccessAt
        ?.toIso8601String(),
    'anonymousFeedbackNextRetryAt': profile.anonymousFeedbackNextRetryAt
        ?.toIso8601String(),
    'anonymousFeedbackLastFlushError': profile.anonymousFeedbackLastFlushError,
    'anonymousFeedbackConsecutiveFailures':
        profile.anonymousFeedbackConsecutiveFailures,
    'anonymousFeedbackQueueJson': profile.anonymousFeedbackQueueJson,
  };
}

SyncProfile _syncProfileFromJson(Map<String, Object?> values) {
  return SyncProfile(
    mode: _enumByName(
      SyncTransportMode.values,
      values['mode'],
      SyncTransportMode.localOnly,
    ),
    status: _enumByName(
      SyncConnectionStatus.values,
      values['status'],
      SyncConnectionStatus.unconfigured,
    ),
    updatedAt:
        _dateTime(values['updatedAt']) ??
        DateTime.fromMillisecondsSinceEpoch(0),
    accountId: _string(values['accountId']),
    accountLabel: _string(values['accountLabel']),
    authToken: _string(values['authToken']),
    refreshToken: _string(values['refreshToken']),
    workspaceId: _string(values['workspaceId']),
    workspaceName: _string(values['workspaceName']),
    deviceId: _string(values['deviceId']),
    deviceName: _string(values['deviceName']),
    devicePlatform: _string(values['devicePlatform']),
    deviceRegistered: values['deviceRegistered'] == true,
    creatorProfileId: _string(values['creatorProfileId']),
    creatorDisplayName: _string(values['creatorDisplayName']),
    creatorSeedRef: _string(values['creatorSeedRef']),
    creatorSeedEnvelopeVersion: _int(values['creatorSeedEnvelopeVersion']),
    creatorProfileSynced: values['creatorProfileSynced'] == true,
    onboardingCompleted: values['onboardingCompleted'] == true,
    entitlementId: _string(values['entitlementId']),
    entitlementLabel: _string(values['entitlementLabel']) ?? '免费版',
    entitlementStatus: _enumByName(
      EntitlementStatus.values,
      values['entitlementStatus'],
      EntitlementStatus.free,
    ),
    entitlementPlanCode: _string(values['entitlementPlanCode']) ?? 'free',
    entitlementFeatures: _boolMap(values['entitlementFeatures']),
    entitlementLastCheckedAt: _dateTime(values['entitlementLastCheckedAt']),
    syncPolicy:
        _string(values['syncPolicy']) ??
        (_boolMap(values['entitlementFeatures'])['cloud_sync'] == true
            ? 'auto_cloud_vault'
            : 'blocked_by_entitlement'),
    cloudBaseUrl: _string(values['cloudBaseUrl']) ?? '',
    lanDebugAddress: _string(values['lanDebugAddress']) ?? '',
    lanDebugPairingCode: _string(values['lanDebugPairingCode']) ?? '',
    lastError: _string(values['lastError']),
    lastRemotePullCursor: _string(values['lastRemotePullCursor']),
    lastSyncAttemptAt: _dateTime(values['lastSyncAttemptAt']),
    lastSyncSuccessAt: _dateTime(values['lastSyncSuccessAt']),
    lastSyncFailureAt: _dateTime(values['lastSyncFailureAt']),
    anonymousFeedbackEnabled: values['anonymousFeedbackEnabled'] == true,
    experienceImprovementEnabled:
        values['experienceImprovementEnabled'] != false,
    anonymousInstallId: _string(values['anonymousInstallId']),
    anonymousFeedbackLastEventAt: _dateTime(
      values['anonymousFeedbackLastEventAt'],
    ),
    anonymousFeedbackLastAttemptAt: _dateTime(
      values['anonymousFeedbackLastAttemptAt'],
    ),
    anonymousFeedbackLastSuccessAt: _dateTime(
      values['anonymousFeedbackLastSuccessAt'],
    ),
    anonymousFeedbackNextRetryAt: _dateTime(
      values['anonymousFeedbackNextRetryAt'],
    ),
    anonymousFeedbackLastFlushError: _string(
      values['anonymousFeedbackLastFlushError'],
    ),
    anonymousFeedbackConsecutiveFailures: _int(
      values['anonymousFeedbackConsecutiveFailures'],
    ),
    anonymousFeedbackQueueJson: _string(values['anonymousFeedbackQueueJson']),
  );
}

T _enumByName<T extends Enum>(List<T> values, Object? name, T fallback) {
  if (name is! String) {
    return fallback;
  }
  return values.firstWhere(
    (value) => value.name == name,
    orElse: () => fallback,
  );
}

String? _string(Object? value) =>
    value is String && value.isNotEmpty ? value : null;

DateTime? _dateTime(Object? value) {
  if (value is! String || value.isEmpty) {
    return null;
  }
  return DateTime.tryParse(value);
}

int _int(Object? value) {
  if (value is int) {
    return value;
  }
  if (value is num) {
    return value.toInt();
  }
  if (value is String) {
    return int.tryParse(value) ?? 0;
  }
  return 0;
}

Map<String, bool> _boolMap(Object? value) {
  if (value is! Map) {
    return const {};
  }
  return {
    for (final entry in value.entries)
      if (entry.key is String) entry.key as String: entry.value == true,
  };
}
