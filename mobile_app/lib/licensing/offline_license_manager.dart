import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'offline_license.dart';
import 'offline_license_state.dart';

const offlineLicenseTrustPolicyJson = String.fromEnvironment(
  'HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON',
  defaultValue: '',
);

class OfflineLicenseTrustedKey {
  const OfflineLicenseTrustedKey({
    required this.keyId,
    required this.publicKey,
    required this.status,
    required this.purposes,
    required this.notBefore,
    required this.notAfter,
  });

  final String keyId;
  final Uint8List publicKey;
  final String status;
  final Set<String> purposes;
  final DateTime notBefore;
  final DateTime notAfter;
}

abstract class OfflineLicenseSecureStore {
  Future<String?> read(String key);

  Future<void> write(String key, String value);

  Future<void> delete(String key);
}

class PlatformOfflineLicenseSecureStore implements OfflineLicenseSecureStore {
  PlatformOfflineLicenseSecureStore({
    FlutterSecureStorage storage = const FlutterSecureStorage(
      aOptions: AndroidOptions(),
      iOptions: IOSOptions(
        accessibility: KeychainAccessibility.first_unlock_this_device,
      ),
    ),
  }) : _storage = storage;

  final FlutterSecureStorage _storage;

  void _ensureSupported() {
    if (kIsWeb ||
        (defaultTargetPlatform != TargetPlatform.android &&
            defaultTargetPlatform != TargetPlatform.iOS)) {
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
  }

  @override
  Future<String?> read(String key) async {
    _ensureSupported();
    try {
      return await _storage.read(key: key);
    } catch (_) {
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
  }

  @override
  Future<void> write(String key, String value) async {
    _ensureSupported();
    try {
      await _storage.write(key: key, value: value);
    } catch (_) {
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
  }

  @override
  Future<void> delete(String key) async {
    _ensureSupported();
    try {
      await _storage.delete(key: key);
    } catch (_) {
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
  }
}

class OfflineLicenseSecureStoreException implements Exception {
  const OfflineLicenseSecureStoreException(this.code);

  final String code;

  @override
  String toString() => code;
}

class OfflineLicenseManager {
  OfflineLicenseManager({
    required OfflineLicenseSecureStore secureStore,
    required String platform,
    required String appVersion,
    Map<String, Uint8List>? publicKeyRing,
    Map<String, OfflineLicenseTrustedKey>? trustedKeyRing,
    DateTime Function()? now,
    Random? random,
  }) : _secureStore = secureStore,
       _platform = platform,
       _appVersion = appVersion,
       _trustedKeyRing =
           trustedKeyRing ??
           (publicKeyRing != null
               ? _legacyTrustedKeyRing(publicKeyRing)
               : parseOfflineLicenseTrustPolicy(offlineLicenseTrustPolicyJson)),
       _now = now ?? DateTime.now,
       _random = random ?? Random.secure();

  static const _installationSecretKey =
      'hidden_shield.offline_license.installation_secret.v1';
  static const _installationSaltKey =
      'hidden_shield.offline_license.installation_salt.v1';
  static const _licenseTokenKey =
      'hidden_shield.offline_license.license_token.v1';
  static const _legacyRevocationTokenKey =
      'hidden_shield.offline_license.revocation_token.v1';
  static const _revocationTokensKey =
      'hidden_shield.offline_license.revocation_tokens.v2';
  static const _highestObservedUtcKey =
      'hidden_shield.offline_license.highest_observed_utc.v1';
  static const _clockRollbackTolerance = Duration(minutes: 5);
  static const _futureArtifactTolerance = Duration(minutes: 5);

  final OfflineLicenseSecureStore _secureStore;
  final String _platform;
  final String _appVersion;
  final Map<String, OfflineLicenseTrustedKey> _trustedKeyRing;
  final DateTime Function() _now;
  final Random _random;

  Future<OfflineLicenseSnapshot> readStatus() async {
    try {
      await _checkAndRecordTrustedTime();
      final identity = await _ensureInstallationIdentity();
      final token = await _secureStore.read(_licenseTokenKey);
      final revocationTokens = await _readRevocationTokens();
      if (token == null || token.isEmpty) {
        return OfflineLicenseSnapshot(
          status: OfflineLicenseStatus.inactive,
          installationId: identity.installationId,
        );
      }
      return _validateLicense(
        token,
        identity.installationId,
        revocationTokens: revocationTokens,
      );
    } on OfflineLicenseSecureStoreException catch (error) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.secureStoreFailure,
        installationId: '',
        lastError: error.code,
      );
    } on FormatException catch (error) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.invalid,
        installationId: '',
        lastError: error.message.toString(),
      );
    }
  }

  Future<String> createActivationRequest() async {
    await _checkAndRecordTrustedTime();
    final identity = await _ensureInstallationIdentity();
    final now = _now().toUtc();
    final nonce = _base64UrlNoPad(_randomBytes(16));
    final requestId =
        'req_${now.microsecondsSinceEpoch.toRadixString(36)}_'
        '${_base64UrlNoPad(_randomBytes(6)).toLowerCase()}';
    return encodeActivationRequestV1(
      ActivationRequestPayloadV1(
        appVersion: _normalizedAppVersion(),
        createdAt: _formatUtcSeconds(now),
        installationId: identity.installationId,
        nonce: nonce,
        platform: _platform,
        requestId: requestId,
        requestedProductCode: 'creator_offline',
        schemaVersion: 1,
      ),
    );
  }

  Future<OfflineLicenseSnapshot> importLicense(String rawToken) async {
    await _checkAndRecordTrustedTime();
    final identity = await _ensureInstallationIdentity();
    final token = rawToken.trim();
    final revocationTokens = await _readRevocationTokens();
    final snapshot = await _validateLicense(
      token,
      identity.installationId,
      revocationTokens: revocationTokens,
    );
    if (!snapshot.isActive) {
      throw FormatException(
        snapshot.lastError ?? _statusErrorCode(snapshot.status),
      );
    }
    await _secureStore.write(_licenseTokenKey, token);
    return snapshot;
  }

  Future<OfflineLicenseSnapshot> importRevocationList(String rawToken) async {
    final now = await _checkAndRecordTrustedTime();
    final token = rawToken.trim();
    final parsed = parseRevocationListV1(token);
    final publicKey = _publicKeyFor(
      parsed.payload.keyId,
      purpose: 'revocation',
      now: now,
    );
    if (!await verifyRevocationListV1Signature(parsed, publicKey)) {
      throw const FormatException(
        'offline_license_revocation_signature_invalid',
      );
    }
    final generatedAt = DateTime.parse(parsed.payload.generatedAt).toUtc();
    if (generatedAt.isAfter(now.add(_futureArtifactTolerance))) {
      throw const FormatException('offline_license_artifact_from_future');
    }
    final revocationTokens = await _readRevocationTokens();
    final currentToken = revocationTokens[parsed.payload.keyId];
    if (currentToken != null) {
      final current = parseRevocationListV1(currentToken);
      if (parsed.payload.sequence < current.payload.sequence) {
        throw const FormatException('offline_license_revocation_replay');
      }
      if (parsed.payload.sequence == current.payload.sequence) {
        if (token == currentToken) {
          return readStatus();
        }
        throw const FormatException('offline_license_revocation_equivocation');
      }
    }
    revocationTokens[parsed.payload.keyId] = token;
    await _writeRevocationTokens(revocationTokens);
    return readStatus();
  }

  Future<OfflineLicenseSnapshot> clearLicense() async {
    await _checkAndRecordTrustedTime();
    await _secureStore.delete(_licenseTokenKey);
    return readStatus();
  }

  Future<OfflineLicenseSnapshot> _validateLicense(
    String token,
    String installationId, {
    Map<String, String> revocationTokens = const {},
  }) async {
    final parsed = parseOfflineLicenseV1(token);
    final payload = parsed.payload;
    final now = await _checkAndRecordTrustedTime();
    final publicKey = _publicKeyFor(
      payload.keyId,
      purpose: 'license',
      now: now,
    );
    if (!await verifyOfflineLicenseV1Signature(parsed, publicKey)) {
      throw const FormatException('offline_license_signature_invalid');
    }
    if (payload.productCode != 'creator_offline') {
      throw const FormatException('offline_license_unknown_product');
    }
    if (payload.installationId != installationId) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.deviceMismatch,
        installationId: installationId,
        licenseId: payload.licenseId,
        productCode: payload.productCode,
        keyId: payload.keyId,
        notBefore: DateTime.parse(payload.notBefore),
        expiresAt: DateTime.parse(payload.expiresAt),
        lastError: 'offline_license_device_mismatch',
      );
    }
    final issuedAt = DateTime.parse(payload.issuedAt).toUtc();
    final notBefore = DateTime.parse(payload.notBefore).toUtc();
    final expiresAt = DateTime.parse(payload.expiresAt).toUtc();
    if (issuedAt.isAfter(expiresAt) || notBefore.isAfter(expiresAt)) {
      throw const FormatException('offline_license_time_invalid');
    }
    if (issuedAt.isAfter(now) || notBefore.isAfter(now)) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.notYetValid,
        installationId: installationId,
        licenseId: payload.licenseId,
        productCode: payload.productCode,
        keyId: payload.keyId,
        notBefore: notBefore,
        expiresAt: expiresAt,
        lastError: 'offline_license_not_yet_valid',
      );
    }
    if (!now.isBefore(expiresAt)) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.expired,
        installationId: installationId,
        licenseId: payload.licenseId,
        productCode: payload.productCode,
        keyId: payload.keyId,
        notBefore: notBefore,
        expiresAt: expiresAt,
        lastError: 'offline_license_expired',
      );
    }
    final revocations = await _validatedRevocations(revocationTokens.values);
    ParsedRevocationListV1? revocation;
    for (final candidate in revocations) {
      if (candidate.payload.revokedLicenseIds.contains(payload.licenseId)) {
        revocation = candidate;
        break;
      }
    }
    if (revocation != null) {
      return OfflineLicenseSnapshot(
        status: OfflineLicenseStatus.revoked,
        installationId: installationId,
        licenseId: payload.licenseId,
        productCode: payload.productCode,
        keyId: payload.keyId,
        notBefore: notBefore,
        expiresAt: expiresAt,
        revocationListId: revocation.payload.listId,
        revocationSequence: revocation.payload.sequence,
        lastError: 'offline_license_revoked',
      );
    }
    final highestRevocation = _highestRevocation(revocations);
    final common = OfflineLicenseSnapshot(
      status: OfflineLicenseStatus.active,
      installationId: installationId,
      licenseId: payload.licenseId,
      productCode: payload.productCode,
      keyId: payload.keyId,
      notBefore: notBefore,
      expiresAt: expiresAt,
      revocationListId: highestRevocation?.payload.listId,
      revocationSequence: highestRevocation?.payload.sequence,
    );
    return common;
  }

  Future<List<ParsedRevocationListV1>> _validatedRevocations(
    Iterable<String> tokens,
  ) async {
    final now = await _checkAndRecordTrustedTime();
    final result = <ParsedRevocationListV1>[];
    for (final token in tokens) {
      final parsed = parseRevocationListV1(token);
      if (!await verifyRevocationListV1Signature(
        parsed,
        _publicKeyFor(parsed.payload.keyId, purpose: 'revocation', now: now),
      )) {
        throw const FormatException(
          'offline_license_revocation_signature_invalid',
        );
      }
      if (DateTime.parse(
        parsed.payload.generatedAt,
      ).toUtc().isAfter(now.add(_futureArtifactTolerance))) {
        throw const FormatException('offline_license_artifact_from_future');
      }
      result.add(parsed);
    }
    return result;
  }

  Future<Map<String, String>> _readRevocationTokens() async {
    final encoded = await _secureStore.read(_revocationTokensKey);
    if (encoded != null && encoded.isNotEmpty) {
      try {
        final decoded = jsonDecode(encoded);
        if (decoded is! Map<String, dynamic>) {
          throw const FormatException(
            'offline_license_revocation_state_tampered',
          );
        }
        final result = <String, String>{};
        for (final entry in decoded.entries) {
          if (entry.value is! String) {
            throw const FormatException(
              'offline_license_revocation_state_tampered',
            );
          }
          final token = entry.value as String;
          final parsed = parseRevocationListV1(token);
          if (parsed.payload.keyId != entry.key) {
            throw const FormatException(
              'offline_license_revocation_state_tampered',
            );
          }
          result[entry.key] = token;
        }
        return result;
      } on FormatException {
        rethrow;
      } catch (_) {
        throw const FormatException(
          'offline_license_revocation_state_tampered',
        );
      }
    }

    final legacy = await _secureStore.read(_legacyRevocationTokenKey);
    if (legacy == null || legacy.isEmpty) return <String, String>{};
    final parsed = parseRevocationListV1(legacy);
    final migrated = <String, String>{parsed.payload.keyId: legacy};
    await _writeRevocationTokens(migrated);
    await _secureStore.delete(_legacyRevocationTokenKey);
    return migrated;
  }

  Future<void> _writeRevocationTokens(Map<String, String> tokens) {
    final sorted = Map<String, String>.fromEntries(
      tokens.entries.toList()
        ..sort((left, right) => left.key.compareTo(right.key)),
    );
    return _secureStore.write(_revocationTokensKey, jsonEncode(sorted));
  }

  ParsedRevocationListV1? _highestRevocation(
    List<ParsedRevocationListV1> revocations,
  ) {
    ParsedRevocationListV1? highest;
    for (final candidate in revocations) {
      if (highest == null ||
          candidate.payload.sequence > highest.payload.sequence) {
        highest = candidate;
      }
    }
    return highest;
  }

  Future<_InstallationIdentity> _ensureInstallationIdentity() async {
    final secretText = await _secureStore.read(_installationSecretKey);
    final saltText = await _secureStore.read(_installationSaltKey);
    if ((secretText == null) != (saltText == null)) {
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
    if (secretText != null && saltText != null) {
      final secret = decodeOfflineLicenseBase64Url(secretText);
      final salt = decodeOfflineLicenseBase64Url(saltText);
      return _InstallationIdentity(deriveInstallationIdV1(secret, salt));
    }
    final secret = _randomBytes(32);
    final salt = _randomBytes(16);
    try {
      await _secureStore.write(_installationSecretKey, _base64UrlNoPad(secret));
      await _secureStore.write(_installationSaltKey, _base64UrlNoPad(salt));
    } catch (_) {
      try {
        await _secureStore.delete(_installationSecretKey);
        await _secureStore.delete(_installationSaltKey);
      } catch (_) {}
      throw const OfflineLicenseSecureStoreException(
        'offline_license_secure_storage_unavailable',
      );
    }
    return _InstallationIdentity(deriveInstallationIdV1(secret, salt));
  }

  Uint8List _publicKeyFor(
    String keyId, {
    required String purpose,
    required DateTime now,
  }) {
    final trustedKey = _trustedKeyRing[keyId];
    if (trustedKey == null) {
      throw const FormatException('offline_license_unknown_key');
    }
    if (trustedKey.status == 'disabled') {
      throw const FormatException('offline_license_key_disabled');
    }
    if (trustedKey.status != 'active' && trustedKey.status != 'verify_only') {
      throw const FormatException('offline_license_trust_policy_invalid');
    }
    if (!trustedKey.purposes.contains(purpose)) {
      throw const FormatException('offline_license_key_purpose_invalid');
    }
    if (now.isBefore(trustedKey.notBefore) ||
        !now.isBefore(trustedKey.notAfter)) {
      throw const FormatException('offline_license_key_inactive');
    }
    return trustedKey.publicKey;
  }

  Future<DateTime> _checkAndRecordTrustedTime() async {
    final now = _now().toUtc();
    final highestText = await _secureStore.read(_highestObservedUtcKey);
    if (highestText != null && highestText.isNotEmpty) {
      final highest = DateTime.parse(highestText).toUtc();
      if (now.add(_clockRollbackTolerance).isBefore(highest)) {
        throw const FormatException('offline_license_clock_rollback');
      }
      if (!now.isAfter(highest)) {
        return now;
      }
    }
    await _secureStore.write(_highestObservedUtcKey, _formatUtcSeconds(now));
    return now;
  }

  Uint8List _randomBytes(int length) {
    return Uint8List.fromList(
      List<int>.generate(length, (_) => _random.nextInt(256)),
    );
  }

  String _normalizedAppVersion() {
    final match = RegExp(
      r'^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?',
    ).firstMatch(_appVersion);
    return match?.group(0) ?? '1.0.0';
  }
}

Map<String, OfflineLicenseTrustedKey> _legacyTrustedKeyRing(
  Map<String, Uint8List> publicKeyRing,
) {
  final result = <String, OfflineLicenseTrustedKey>{};
  for (final entry in publicKeyRing.entries) {
    result[entry.key] = OfflineLicenseTrustedKey(
      keyId: entry.key,
      publicKey: entry.value,
      status: 'active',
      purposes: const {'license', 'revocation'},
      notBefore: DateTime.utc(1970),
      notAfter: DateTime.utc(9999, 12, 31, 23, 59, 59),
    );
  }
  return Map.unmodifiable(result);
}

Map<String, OfflineLicenseTrustedKey> parseOfflineLicenseTrustPolicy(
  String source,
) {
  if (source.trim().isEmpty) return const {};
  try {
    final decoded = jsonDecode(source);
    if (decoded is Map<String, dynamic> &&
        decoded['policyType'] == 'offline_license_trust_policy') {
      if (decoded['schemaVersion'] != 1 || decoded['keys'] is! List) {
        throw const FormatException('offline_license_trust_policy_invalid');
      }
      final result = <String, OfflineLicenseTrustedKey>{};
      for (final item in decoded['keys'] as List) {
        if (item is! Map<String, dynamic> ||
            item['keyId'] is! String ||
            item['algorithm'] != 'Ed25519' ||
            item['publicKeyBase64Url'] is! String ||
            item['status'] is! String ||
            item['purposes'] is! List ||
            item['notBefore'] is! String ||
            item['notAfter'] is! String) {
          throw const FormatException('offline_license_trust_policy_invalid');
        }
        final publicKey = decodeOfflineLicenseBase64Url(
          item['publicKeyBase64Url'] as String,
        );
        if (publicKey.length != 32) {
          throw const FormatException('offline_license_trust_policy_invalid');
        }
        final purposes = (item['purposes'] as List)
            .map((purpose) => purpose.toString())
            .toSet();
        if (purposes.isEmpty ||
            purposes.any(
              (purpose) => purpose != 'license' && purpose != 'revocation',
            )) {
          throw const FormatException('offline_license_trust_policy_invalid');
        }
        final keyId = item['keyId'] as String;
        if (result.containsKey(keyId)) {
          throw const FormatException('offline_license_trust_policy_invalid');
        }
        result[keyId] = OfflineLicenseTrustedKey(
          keyId: keyId,
          publicKey: publicKey,
          status: item['status'] as String,
          purposes: Set.unmodifiable(purposes),
          notBefore: DateTime.parse(item['notBefore'] as String).toUtc(),
          notAfter: DateTime.parse(item['notAfter'] as String).toUtc(),
        );
      }
      return Map.unmodifiable(result);
    }
    throw const FormatException('offline_license_trust_policy_invalid');
  } on FormatException {
    rethrow;
  } catch (_) {
    throw const FormatException('offline_license_trust_policy_invalid');
  }
}

Map<String, Uint8List> parseOfflineLicensePublicKeyRing(String source) {
  if (source.trim().isEmpty) return const {};
  try {
    final decoded = jsonDecode(source);
    final result = <String, Uint8List>{};
    if (decoded is Map<String, dynamic>) {
      for (final entry in decoded.entries) {
        if (entry.value is! String) {
          throw const FormatException('offline_license_unknown_key');
        }
        result[entry.key] = decodeOfflineLicenseBase64Url(
          entry.value as String,
        );
      }
    } else if (decoded is List) {
      for (final item in decoded) {
        if (item is! Map<String, dynamic> ||
            item['keyId'] is! String ||
            item['publicKeyBase64Url'] is! String) {
          throw const FormatException('offline_license_unknown_key');
        }
        result[item['keyId'] as String] = decodeOfflineLicenseBase64Url(
          item['publicKeyBase64Url'] as String,
        );
      }
    } else {
      throw const FormatException('offline_license_unknown_key');
    }
    for (final publicKey in result.values) {
      if (publicKey.length != 32) {
        throw const FormatException('offline_license_unknown_key');
      }
    }
    return Map.unmodifiable(result);
  } on FormatException {
    rethrow;
  } catch (_) {
    throw const FormatException('offline_license_unknown_key');
  }
}

String _formatUtcSeconds(DateTime value) {
  final utc = value.toUtc();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${utc.year.toString().padLeft(4, '0')}-'
      '${two(utc.month)}-${two(utc.day)}T'
      '${two(utc.hour)}:${two(utc.minute)}:${two(utc.second)}Z';
}

String _base64UrlNoPad(List<int> bytes) =>
    base64UrlEncode(bytes).replaceAll('=', '');

String _statusErrorCode(OfflineLicenseStatus status) => switch (status) {
  OfflineLicenseStatus.notYetValid => 'offline_license_not_yet_valid',
  OfflineLicenseStatus.expired => 'offline_license_expired',
  OfflineLicenseStatus.revoked => 'offline_license_revoked',
  OfflineLicenseStatus.deviceMismatch => 'offline_license_device_mismatch',
  OfflineLicenseStatus.secureStoreFailure || OfflineLicenseStatus.unsupported =>
    'offline_license_secure_storage_unavailable',
  _ => 'offline_license_invalid_format',
};

class _InstallationIdentity {
  const _InstallationIdentity(this.installationId);

  final String installationId;
}
