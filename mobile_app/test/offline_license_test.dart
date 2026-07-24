import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/licensing/offline_license.dart';

void main() {
  final fixtureRoot = '../docs/fixtures/offline-license-k0';
  final licenseFixture = _readJson('$fixtureRoot/hslic1-ed25519-v1.json');
  final requestFixture = _readJson('$fixtureRoot/hsreq1-v1-valid.json');
  final revocationFixture = _readJson(
    '$fixtureRoot/hsrvl1-ed25519-v1-valid.json',
  );
  final errorVectors = _readJson('$fixtureRoot/offline-license-errors-v1.json');
  final installationIdentity = _readJson(
    '$fixtureRoot/installation-identity-v1.json',
  );

  test('parses and verifies all shared offline-license vectors', () async {
    final licenseExpected = licenseFixture['expected'] as Map<String, dynamic>;
    final license = parseOfflineLicenseV1(licenseFixture['token'] as String);
    final publicKey = decodeOfflineLicenseBase64Url(
      licenseFixture['publicKeyBase64Url'] as String,
    );
    expect(
      utf8.decode(license.payloadBytes),
      licenseFixture['canonicalPayload'],
    );
    expect(license.payload.schemaVersion, licenseExpected['schemaVersion']);
    expect(license.payload.licenseId, licenseExpected['licenseId']);
    expect(license.payload.productCode, licenseExpected['productCode']);
    expect(license.payload.installationId, licenseExpected['installationId']);
    expect(
      await verifyOfflineLicenseV1Signature(license, publicKey),
      licenseExpected['signatureValid'],
    );

    final requestExpected = requestFixture['expected'] as Map<String, dynamic>;
    final request = parseActivationRequestV1(requestFixture['token'] as String);
    expect(
      utf8.decode(request.payloadBytes),
      requestFixture['canonicalPayload'],
    );
    expect(request.payload.schemaVersion, requestExpected['schemaVersion']);
    expect(request.payload.requestId, requestExpected['requestId']);
    expect(
      request.payload.requestedProductCode,
      requestExpected['requestedProductCode'],
    );
    expect(request.payload.installationId, requestExpected['installationId']);
    expect(
      verifyActivationRequestV1Checksum(request),
      requestExpected['checksumValid'],
    );

    final revocationExpected =
        revocationFixture['expected'] as Map<String, dynamic>;
    final revocation = parseRevocationListV1(
      revocationFixture['token'] as String,
    );
    expect(
      utf8.decode(revocation.payloadBytes),
      revocationFixture['canonicalPayload'],
    );
    expect(
      revocation.payload.schemaVersion,
      revocationExpected['schemaVersion'],
    );
    expect(revocation.payload.listId, revocationExpected['listId']);
    expect(
      revocation.payload.revokedLicenseIds,
      revocationExpected['revokedLicenseIds'],
    );
    expect(
      await verifyRevocationListV1Signature(revocation, publicKey),
      revocationExpected['signatureValid'],
    );
  });

  test('matches all shared offline-license error vectors', () async {
    final sources = <String, Map<String, dynamic>>{
      'license': licenseFixture,
      'activation_request': requestFixture,
      'revocation_list': revocationFixture,
    };
    final cases = errorVectors['cases'] as List<dynamic>;
    for (final rawCase in cases) {
      final vector = rawCase as Map<String, dynamic>;
      final source = sources[vector['source']]!;
      final mutated = _mutateVector(
        source,
        vector['mutation'] as Map<String, dynamic>,
      );
      String? actualError;
      try {
        await validateOfflineArtifactV1(
          vector['source'] as String,
          mutated.token,
          publicKeyBytes: mutated.publicKeyBase64Url == null
              ? null
              : decodeOfflineLicenseBase64Url(mutated.publicKeyBase64Url!),
        );
      } on FormatException catch (error) {
        actualError = error.message;
      }
      expect(
        actualError,
        vector['expectedError'],
        reason: vector['caseId'] as String,
      );
    }
  });

  test('derives the shared installation identity vector', () {
    expect(
      deriveInstallationIdV1(
        decodeOfflineLicenseBase64Url(
          installationIdentity['testOnlySecretBase64Url'] as String,
        ),
        decodeOfflineLicenseBase64Url(
          installationIdentity['saltBase64Url'] as String,
        ),
      ),
      installationIdentity['expectedInstallationId'],
    );
  });
}

Map<String, dynamic> _readJson(String path) {
  return jsonDecode(File(path).readAsStringSync()) as Map<String, dynamic>;
}

_MutatedVector _mutateVector(
  Map<String, dynamic> source,
  Map<String, dynamic> mutation,
) {
  final segments = (source['token'] as String).split('.');
  var publicKeyBase64Url = source['publicKeyBase64Url'] as String?;
  switch (mutation['kind']) {
    case 'replace_prefix':
      segments[0] = mutation['value'] as String;
    case 'replace_payload':
      final payload = utf8.decode(
        base64Url.decode(base64Url.normalize(segments[1])),
      );
      final from = mutation['from'] as String;
      expect(from.allMatches(payload), hasLength(1));
      final mutated = payload.replaceFirst(from, mutation['to'] as String);
      segments[1] = base64UrlEncode(utf8.encode(mutated)).replaceAll('=', '');
    case 'replace_trailer':
      segments[2] = mutation['value'] as String;
    case 'replace_public_key':
      publicKeyBase64Url = mutation['value'] as String;
    default:
      throw StateError('unknown mutation ${mutation['kind']}');
  }
  return _MutatedVector(segments.join('.'), publicKeyBase64Url);
}

class _MutatedVector {
  const _MutatedVector(this.token, this.publicKeyBase64Url);

  final String token;
  final String? publicKeyBase64Url;
}
