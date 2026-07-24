import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/licensing/offline_license.dart';
import 'package:hidden_shield_mobile/licensing/offline_license_manager.dart';
import 'package:hidden_shield_mobile/licensing/offline_license_state.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

void main() {
  final fixture =
      jsonDecode(
            File(
              '../docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json',
            ).readAsStringSync(),
          )
          as Map<String, dynamic>;
  final publicKey = decodeOfflineLicenseBase64Url(
    fixture['publicKeyBase64Url'] as String,
  );
  final privateSeed = _decodeHex(
    fixture['testOnlyPrivateKeySeedHex'] as String,
  );
  final publicKeyRing = <String, Uint8List>{'offline-test-k0': publicKey};

  test('desktop-bound license returns device mismatch on mobile', () async {
    final desktopStore = MemoryOfflineLicenseSecureStore();
    final desktopManager = _manager(
      secureStore: desktopStore,
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 1),
    );
    final desktopRequest = parseActivationRequestV1(
      await desktopManager.createActivationRequest(),
    );
    final token = await _signLicense(
      installationId: desktopRequest.payload.installationId,
      privateSeed: privateSeed,
    );

    final mobileManager = _manager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 1),
      randomSeed: 8,
    );

    await expectLater(
      mobileManager.importLicense(token),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_device_mismatch',
        ),
      ),
    );
  });

  test('expired license fails closed at import', () async {
    final secureStore = MemoryOfflineLicenseSecureStore();
    final manager = _manager(
      secureStore: secureStore,
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2028, 7, 15),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    final token = await _signLicense(
      installationId: request.payload.installationId,
      privateSeed: privateSeed,
      expiresAt: DateTime.utc(2027, 7, 15),
    );

    await expectLater(
      manager.importLicense(token),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_expired',
        ),
      ),
    );
    expect((await manager.readStatus()).status, OfflineLicenseStatus.inactive);
  });

  test('secure-store failure disables offline execution', () async {
    final manager = _manager(
      secureStore: ThrowingOfflineLicenseSecureStore(),
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15),
    );
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      offlineLicenseManager: manager,
    );

    await state.load();

    expect(
      state.offlineLicenseSnapshot.status,
      OfflineLicenseStatus.secureStoreFailure,
    );
    expect(state.canUseLocalBatchProcessing, isFalse);
    expect(state.canExportFormalReports, isFalse);
    expect(
      (await state.authorizeLocalExecution('batch_processing')).allowed,
      isFalse,
    );
  });

  test('offline Creator merges only local batch and report features', () async {
    final secureStore = MemoryOfflineLicenseSecureStore();
    final manager = _manager(
      secureStore: secureStore,
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 1),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    await manager.importLicense(
      await _signLicense(
        installationId: request.payload.installationId,
        privateSeed: privateSeed,
      ),
    );
    final store = MemoryVaultStore();
    await store.saveSyncProfile(
      SyncProfile.localOnly().copyWith(
        entitlementFeatures: const {
          'cloud_sync': false,
          'batch_processing': false,
          'report_export': false,
          'cloud_batch_processing': false,
          'cloud_video_processing': false,
          'priority_queue': false,
          'team_workspace': false,
          'api_access': false,
        },
      ),
    );
    final state = MobileAppState(
      vaultStore: store,
      offlineLicenseManager: manager,
    );

    await state.load();

    expect(state.canUseLocalBatchProcessing, isTrue);
    expect(state.canExportFormalReports, isTrue);
    expect(state.effectiveEntitlementLabel, 'Creator（离线授权）');
    expect(
      (await state.authorizeLocalExecution('batch_processing')).source,
      'offline_cdkey',
    );
    expect(
      (await state.authorizeLocalExecution('report_export')).source,
      'offline_cdkey',
    );
    final metadata = await store.loadOfflineLicenseMetadata();
    expect(metadata?.licenseId, 'lic_k3_test');
    expect(state.syncProfile.entitlementPlanCode, 'free');
  });

  test('all cloud features remain false for offline-only activation', () async {
    final secureStore = MemoryOfflineLicenseSecureStore();
    final manager = _manager(
      secureStore: secureStore,
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 1),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    await manager.importLicense(
      await _signLicense(
        installationId: request.payload.installationId,
        privateSeed: privateSeed,
      ),
    );
    final state = MobileAppState(
      vaultStore: MemoryVaultStore(),
      offlineLicenseManager: manager,
    );

    await state.load();

    for (final feature in const [
      'cloud_sync',
      'cloud_batch_processing',
      'cloud_video_processing',
      'priority_queue',
      'team_workspace',
      'api_access',
    ]) {
      expect(
        state.effectiveEntitlementFeatures[feature] == true,
        isFalse,
        reason: feature,
      );
    }
    expect(state.canUseCloudSync, isFalse);
    expect(state.canUseTeamWorkspace, isFalse);
    expect(
      (await state.authorizeLocalExecution('cloud_sync')).source,
      'server_only',
    );
  });

  test('K4 trust policy fixture preserves key statuses and purposes', () {
    final policy = parseOfflineLicenseTrustPolicy(
      File(
        '../docs/fixtures/offline-license-k4/trust-policy-v1.json',
      ).readAsStringSync(),
    );

    expect(policy['offline-test-k0']?.status, 'active');
    expect(
      policy['offline-test-k0']?.purposes,
      containsAll(<String>['license', 'revocation']),
    );
    expect(policy['offline-test-disabled']?.status, 'disabled');
    expect(
      policy['offline-test-verify-only']?.purposes,
      equals(<String>{'license'}),
    );
  });

  test('K4 trusted clock rejects rollback beyond five minutes', () async {
    var observedNow = DateTime.utc(2026, 7, 15, 12);
    final manager = OfflineLicenseManager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      platform: 'android',
      appVersion: '1.0.0',
      publicKeyRing: publicKeyRing,
      now: () => observedNow,
      random: Random(11),
    );
    await manager.createActivationRequest();

    observedNow = DateTime.utc(2026, 7, 15, 11, 54, 59);
    final status = await manager.readStatus();

    expect(status.status, OfflineLicenseStatus.invalid);
    expect(status.lastError, 'offline_license_clock_rollback');
  });

  test('K4 revocation high water rejects replay and equivocation', () async {
    final manager = _manager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 12),
    );
    final first = await _signRevocation(
      privateSeed: privateSeed,
      sequence: 7,
      listId: 'rvl_k4_0007',
    );
    await manager.importRevocationList(first);
    await manager.importRevocationList(first);

    final replay = await _signRevocation(
      privateSeed: privateSeed,
      sequence: 6,
      listId: 'rvl_k4_0006',
    );
    await expectLater(
      manager.importRevocationList(replay),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_revocation_replay',
        ),
      ),
    );

    final equivocation = await _signRevocation(
      privateSeed: privateSeed,
      sequence: 7,
      listId: 'rvl_k4_conflict',
    );
    await expectLater(
      manager.importRevocationList(equivocation),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_revocation_equivocation',
        ),
      ),
    );
  });

  test('K4 key rotation preserves revocations from every keyId', () async {
    final trustedKeyRing = <String, OfflineLicenseTrustedKey>{
      for (final keyId in ['offline-test-k0', 'offline-test-k1'])
        keyId: OfflineLicenseTrustedKey(
          keyId: keyId,
          publicKey: publicKey,
          status: 'active',
          purposes: const {'license', 'revocation'},
          notBefore: DateTime.utc(2026),
          notAfter: DateTime.utc(2028),
        ),
    };
    final manager = OfflineLicenseManager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      platform: 'android',
      appVersion: '1.0.0',
      trustedKeyRing: trustedKeyRing,
      now: () => DateTime.utc(2026, 7, 15, 12),
      random: Random(13),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    await manager.importLicense(
      await _signLicense(
        installationId: request.payload.installationId,
        privateSeed: privateSeed,
      ),
    );
    await manager.importRevocationList(
      await _signRevocation(
        privateSeed: privateSeed,
        sequence: 7,
        listId: 'rvl_k4_key0',
        revokedLicenseIds: const ['lic_k3_test'],
      ),
    );
    expect((await manager.readStatus()).status, OfflineLicenseStatus.revoked);

    await manager.importRevocationList(
      await _signRevocation(
        privateSeed: privateSeed,
        sequence: 1,
        listId: 'rvl_k4_key1',
        keyId: 'offline-test-k1',
      ),
    );
    expect((await manager.readStatus()).status, OfflineLicenseStatus.revoked);
  });

  test('K4 mobile rejects a license issued in the future', () async {
    final manager = _manager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 12),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    final token = await _signLicense(
      installationId: request.payload.installationId,
      privateSeed: privateSeed,
      issuedAt: DateTime.utc(2026, 7, 15, 13),
    );

    await expectLater(
      manager.importLicense(token),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_not_yet_valid',
        ),
      ),
    );
  });

  test('K4 disabled signing key fails closed', () async {
    final policy = parseOfflineLicenseTrustPolicy(
      File(
        '../docs/fixtures/offline-license-k4/trust-policy-v1.json',
      ).readAsStringSync(),
    );
    final manager = OfflineLicenseManager(
      secureStore: MemoryOfflineLicenseSecureStore(),
      platform: 'android',
      appVersion: '1.0.0',
      trustedKeyRing: policy,
      now: () => DateTime.utc(2026, 7, 15, 12),
      random: Random(12),
    );
    final request = parseActivationRequestV1(
      await manager.createActivationRequest(),
    );
    final token = await _signLicense(
      installationId: request.payload.installationId,
      privateSeed: privateSeed,
      keyId: 'offline-test-disabled',
    );

    await expectLater(
      manager.importLicense(token),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'offline_license_key_disabled',
        ),
      ),
    );
  });

  test('K4 license replacement appends transfer audit', () async {
    final secureStore = MemoryOfflineLicenseSecureStore();
    final manager = _manager(
      secureStore: secureStore,
      publicKeyRing: publicKeyRing,
      now: DateTime.utc(2026, 7, 15, 12),
    );
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      offlineLicenseManager: manager,
    );
    await state.load();
    final request = parseActivationRequestV1(
      await state.createOfflineActivationRequest(),
    );
    await state.importOfflineLicenseToken(
      await _signLicense(
        installationId: request.payload.installationId,
        privateSeed: privateSeed,
        licenseId: 'lic_k4_old',
      ),
    );
    await state.importOfflineLicenseToken(
      await _signLicense(
        installationId: request.payload.installationId,
        privateSeed: privateSeed,
        licenseId: 'lic_k4_new',
      ),
    );

    final audit = await store.loadOfflineLicenseAudit();
    final replacement = audit.singleWhere(
      (event) => event.action == 'replace_license',
    );
    expect(replacement.licenseId, 'lic_k4_old');
    expect(replacement.detailCode, 'lic_k4_new');
  });
}

OfflineLicenseManager _manager({
  required OfflineLicenseSecureStore secureStore,
  required Map<String, Uint8List> publicKeyRing,
  required DateTime now,
  int randomSeed = 7,
}) {
  return OfflineLicenseManager(
    secureStore: secureStore,
    platform: 'android',
    appVersion: '1.0.0',
    publicKeyRing: publicKeyRing,
    now: () => now,
    random: Random(randomSeed),
  );
}

Future<String> _signLicense({
  required String installationId,
  required Uint8List privateSeed,
  DateTime? expiresAt,
  DateTime? issuedAt,
  String keyId = 'offline-test-k0',
  String licenseId = 'lic_k3_test',
}) async {
  final payload = <String, Object>{
    'expiresAt': _formatUtc(expiresAt ?? DateTime.utc(2027, 7, 15)),
    'installationId': installationId,
    'issuedAt': _formatUtc(issuedAt ?? DateTime.utc(2026, 7, 15)),
    'keyId': keyId,
    'licenseId': licenseId,
    'notBefore': '2026-07-15T00:00:00Z',
    'productCode': 'creator_offline',
    'schemaVersion': 1,
  };
  final payloadBytes = utf8.encode(jsonEncode(payload));
  final signingMessage = Uint8List.fromList([
    ...utf8.encode('HiddenShield-Offline-License-v1\u0000'),
    ...payloadBytes,
  ]);
  final keyPair = await Ed25519().newKeyPairFromSeed(privateSeed);
  final signature = await Ed25519().sign(signingMessage, keyPair: keyPair);
  return 'HSLIC1.${_base64Url(payloadBytes)}.${_base64Url(signature.bytes)}';
}

Future<String> _signRevocation({
  required Uint8List privateSeed,
  required int sequence,
  required String listId,
  String keyId = 'offline-test-k0',
  List<String> revokedLicenseIds = const [],
}) async {
  final payload = <String, Object>{
    'generatedAt': '2026-07-15T01:00:00Z',
    'keyId': keyId,
    'listId': listId,
    'listType': 'offline_license_revocations',
    'revokedLicenseIds': revokedLicenseIds,
    'schemaVersion': 1,
    'sequence': sequence,
  };
  final payloadBytes = utf8.encode(jsonEncode(payload));
  final signingMessage = Uint8List.fromList([
    ...utf8.encode('HiddenShield-Offline-Revocation-List-v1\u0000'),
    ...payloadBytes,
  ]);
  final keyPair = await Ed25519().newKeyPairFromSeed(privateSeed);
  final signature = await Ed25519().sign(signingMessage, keyPair: keyPair);
  return 'HSRVL1.${_base64Url(payloadBytes)}.${_base64Url(signature.bytes)}';
}

String _formatUtc(DateTime value) {
  final utc = value.toUtc();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${utc.year.toString().padLeft(4, '0')}-'
      '${two(utc.month)}-${two(utc.day)}T'
      '${two(utc.hour)}:${two(utc.minute)}:${two(utc.second)}Z';
}

String _base64Url(List<int> bytes) =>
    base64UrlEncode(bytes).replaceAll('=', '');

Uint8List _decodeHex(String value) {
  return Uint8List.fromList([
    for (var index = 0; index < value.length; index += 2)
      int.parse(value.substring(index, index + 2), radix: 16),
  ]);
}

class MemoryOfflineLicenseSecureStore implements OfflineLicenseSecureStore {
  final Map<String, String> values = {};

  @override
  Future<void> delete(String key) async {
    values.remove(key);
  }

  @override
  Future<String?> read(String key) async => values[key];

  @override
  Future<void> write(String key, String value) async {
    values[key] = value;
  }
}

class ThrowingOfflineLicenseSecureStore implements OfflineLicenseSecureStore {
  @override
  Future<void> delete(String key) {
    throw const OfflineLicenseSecureStoreException(
      'offline_license_secure_storage_unavailable',
    );
  }

  @override
  Future<String?> read(String key) {
    throw const OfflineLicenseSecureStoreException(
      'offline_license_secure_storage_unavailable',
    );
  }

  @override
  Future<void> write(String key, String value) {
    throw const OfflineLicenseSecureStoreException(
      'offline_license_secure_storage_unavailable',
    );
  }
}
