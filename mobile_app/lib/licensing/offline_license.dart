import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as hashes;
import 'package:cryptography/cryptography.dart';

const _hslic1Prefix = 'HSLIC1';
const _hsreq1Prefix = 'HSREQ1';
const _hsrvl1Prefix = 'HSRVL1';
const _hslic1SignatureDomain = 'HiddenShield-Offline-License-v1';
const _hsreq1ChecksumDomain = 'HiddenShield-Offline-Activation-Request-v1';
const _hsrvl1SignatureDomain = 'HiddenShield-Offline-Revocation-List-v1';
const _installationIdDomain = 'HiddenShield-Installation-v1';
const _hslic1PayloadKeys = <String>[
  'expiresAt',
  'installationId',
  'issuedAt',
  'keyId',
  'licenseId',
  'notBefore',
  'productCode',
  'schemaVersion',
];
const _hsreq1PayloadKeys = <String>[
  'appVersion',
  'createdAt',
  'installationId',
  'nonce',
  'platform',
  'requestId',
  'requestedProductCode',
  'schemaVersion',
];
const _hsrvl1PayloadKeys = <String>[
  'generatedAt',
  'keyId',
  'listId',
  'listType',
  'revokedLicenseIds',
  'schemaVersion',
  'sequence',
];
final _timestampPattern = RegExp(r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$');
final _installationIdPattern = RegExp(r'^[A-Za-z0-9_-]{43}$');
final _noncePattern = RegExp(r'^[A-Za-z0-9_-]{22}$');
final _identifierPattern = RegExp(r'^[a-z0-9][a-z0-9._-]{2,63}$');
final _appVersionPattern = RegExp(
  r'^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$',
);
const _platforms = {'windows', 'macos', 'linux', 'android', 'ios'};

class OfflineLicensePayloadV1 {
  const OfflineLicensePayloadV1({
    required this.expiresAt,
    required this.installationId,
    required this.issuedAt,
    required this.keyId,
    required this.licenseId,
    required this.notBefore,
    required this.productCode,
    required this.schemaVersion,
  });

  final String expiresAt;
  final String installationId;
  final String issuedAt;
  final String keyId;
  final String licenseId;
  final String notBefore;
  final String productCode;
  final int schemaVersion;

  Map<String, Object> toJson() => {
    'expiresAt': expiresAt,
    'installationId': installationId,
    'issuedAt': issuedAt,
    'keyId': keyId,
    'licenseId': licenseId,
    'notBefore': notBefore,
    'productCode': productCode,
    'schemaVersion': schemaVersion,
  };
}

class ActivationRequestPayloadV1 {
  const ActivationRequestPayloadV1({
    required this.appVersion,
    required this.createdAt,
    required this.installationId,
    required this.nonce,
    required this.platform,
    required this.requestId,
    required this.requestedProductCode,
    required this.schemaVersion,
  });

  final String appVersion;
  final String createdAt;
  final String installationId;
  final String nonce;
  final String platform;
  final String requestId;
  final String requestedProductCode;
  final int schemaVersion;

  Map<String, Object> toJson() => {
    'appVersion': appVersion,
    'createdAt': createdAt,
    'installationId': installationId,
    'nonce': nonce,
    'platform': platform,
    'requestId': requestId,
    'requestedProductCode': requestedProductCode,
    'schemaVersion': schemaVersion,
  };
}

class RevocationListPayloadV1 {
  const RevocationListPayloadV1({
    required this.generatedAt,
    required this.keyId,
    required this.listId,
    required this.listType,
    required this.revokedLicenseIds,
    required this.schemaVersion,
    required this.sequence,
  });

  final String generatedAt;
  final String keyId;
  final String listId;
  final String listType;
  final List<String> revokedLicenseIds;
  final int schemaVersion;
  final int sequence;

  Map<String, Object> toJson() => {
    'generatedAt': generatedAt,
    'keyId': keyId,
    'listId': listId,
    'listType': listType,
    'revokedLicenseIds': revokedLicenseIds,
    'schemaVersion': schemaVersion,
    'sequence': sequence,
  };
}

class ParsedOfflineLicenseV1 {
  const ParsedOfflineLicenseV1({
    required this.payload,
    required this.payloadBytes,
    required this.signatureBytes,
    required this.signingMessage,
  });

  final OfflineLicensePayloadV1 payload;
  final Uint8List payloadBytes;
  final Uint8List signatureBytes;
  final Uint8List signingMessage;
}

class ParsedActivationRequestV1 {
  const ParsedActivationRequestV1({
    required this.payload,
    required this.payloadBytes,
    required this.checksumBytes,
  });

  final ActivationRequestPayloadV1 payload;
  final Uint8List payloadBytes;
  final Uint8List checksumBytes;
}

class ParsedRevocationListV1 {
  const ParsedRevocationListV1({
    required this.payload,
    required this.payloadBytes,
    required this.signatureBytes,
    required this.signingMessage,
  });

  final RevocationListPayloadV1 payload;
  final Uint8List payloadBytes;
  final Uint8List signatureBytes;
  final Uint8List signingMessage;
}

String encodeActivationRequestV1(ActivationRequestPayloadV1 payload) {
  final payloadText = jsonEncode(payload.toJson());
  final payloadBytes = Uint8List.fromList(utf8.encode(payloadText));
  final checksum = hashes.sha256.convert([
    ...utf8.encode('$_hsreq1ChecksumDomain\u0000'),
    ...payloadBytes,
  ]);
  return '$_hsreq1Prefix.'
      '${base64UrlEncode(payloadBytes).replaceAll('=', '')}.'
      '${base64UrlEncode(checksum.bytes.sublist(0, 12)).replaceAll('=', '')}';
}

ParsedOfflineLicenseV1 parseOfflineLicenseV1(String token) {
  final decoded = _decodeThreeSegmentToken(
    token,
    _hslic1Prefix,
    'offline_license_invalid_format',
    64,
  );
  final payloadText = _decodeUtf8(
    decoded.payloadBytes,
    'offline_license_invalid_format',
  );
  final document = _parseObject(payloadText, 'offline_license_invalid_format');
  _assertExactKeys(
    document,
    _hslic1PayloadKeys,
    'offline_license_unknown_schema',
  );
  _assertCanonical(document, payloadText);
  if (document['schemaVersion'] != 1) {
    throw const FormatException('offline_license_unknown_schema');
  }
  if (document['productCode'] != 'creator_offline') {
    throw const FormatException('offline_license_feature_profile_invalid');
  }
  final payload = OfflineLicensePayloadV1(
    expiresAt: _requiredString(document, 'expiresAt'),
    installationId: _requiredString(document, 'installationId'),
    issuedAt: _requiredString(document, 'issuedAt'),
    keyId: _requiredString(document, 'keyId'),
    licenseId: _requiredString(document, 'licenseId'),
    notBefore: _requiredString(document, 'notBefore'),
    productCode: _requiredString(document, 'productCode'),
    schemaVersion: _requiredInteger(document, 'schemaVersion'),
  );
  if (!_timestampPattern.hasMatch(payload.expiresAt) ||
      !_installationIdPattern.hasMatch(payload.installationId) ||
      !_timestampPattern.hasMatch(payload.issuedAt) ||
      !_identifierPattern.hasMatch(payload.keyId) ||
      !_identifierPattern.hasMatch(payload.licenseId) ||
      !_timestampPattern.hasMatch(payload.notBefore)) {
    throw const FormatException('offline_license_invalid_format');
  }
  return ParsedOfflineLicenseV1(
    payload: payload,
    payloadBytes: decoded.payloadBytes,
    signatureBytes: decoded.trailerBytes,
    signingMessage: _signingMessage(
      _hslic1SignatureDomain,
      decoded.payloadBytes,
    ),
  );
}

ParsedActivationRequestV1 parseActivationRequestV1(String token) {
  final decoded = _decodeThreeSegmentToken(
    token,
    _hsreq1Prefix,
    'offline_license_request_invalid_format',
    12,
  );
  final payloadText = _decodeUtf8(
    decoded.payloadBytes,
    'offline_license_request_invalid_format',
  );
  final document = _parseObject(
    payloadText,
    'offline_license_request_invalid_format',
  );
  _assertExactKeys(
    document,
    _hsreq1PayloadKeys,
    'offline_license_request_unknown_schema',
  );
  _assertCanonical(
    document,
    payloadText,
    'offline_license_request_non_canonical_payload',
  );
  if (document['schemaVersion'] != 1) {
    throw const FormatException('offline_license_request_unknown_schema');
  }
  if (document['requestedProductCode'] != 'creator_offline') {
    throw const FormatException('offline_license_request_product_invalid');
  }
  final payload = ActivationRequestPayloadV1(
    appVersion: _requiredString(document, 'appVersion'),
    createdAt: _requiredString(document, 'createdAt'),
    installationId: _requiredString(document, 'installationId'),
    nonce: _requiredString(document, 'nonce'),
    platform: _requiredString(document, 'platform'),
    requestId: _requiredString(document, 'requestId'),
    requestedProductCode: _requiredString(document, 'requestedProductCode'),
    schemaVersion: _requiredInteger(document, 'schemaVersion'),
  );
  if (!_appVersionPattern.hasMatch(payload.appVersion) ||
      !_timestampPattern.hasMatch(payload.createdAt) ||
      !_installationIdPattern.hasMatch(payload.installationId) ||
      !_noncePattern.hasMatch(payload.nonce) ||
      !_platforms.contains(payload.platform) ||
      !_identifierPattern.hasMatch(payload.requestId)) {
    throw const FormatException('offline_license_request_invalid_format');
  }
  return ParsedActivationRequestV1(
    payload: payload,
    payloadBytes: decoded.payloadBytes,
    checksumBytes: decoded.trailerBytes,
  );
}

ParsedRevocationListV1 parseRevocationListV1(String token) {
  final decoded = _decodeThreeSegmentToken(
    token,
    _hsrvl1Prefix,
    'offline_license_revocation_invalid_format',
    64,
  );
  final payloadText = _decodeUtf8(
    decoded.payloadBytes,
    'offline_license_revocation_invalid_format',
  );
  final document = _parseObject(
    payloadText,
    'offline_license_revocation_invalid_format',
  );
  _assertExactKeys(
    document,
    _hsrvl1PayloadKeys,
    'offline_license_revocation_unknown_schema',
  );
  _assertCanonical(
    document,
    payloadText,
    'offline_license_revocation_non_canonical_payload',
  );
  if (document['schemaVersion'] != 1) {
    throw const FormatException('offline_license_revocation_unknown_schema');
  }
  if (document['listType'] != 'offline_license_revocations') {
    throw const FormatException('offline_license_revocation_list_invalid');
  }
  final sequence = _requiredInteger(document, 'sequence');
  if (sequence < 1) {
    throw const FormatException('offline_license_revocation_sequence_invalid');
  }
  final revokedLicenseIds = _requiredStringList(document, 'revokedLicenseIds');
  final payload = RevocationListPayloadV1(
    generatedAt: _requiredString(document, 'generatedAt'),
    keyId: _requiredString(document, 'keyId'),
    listId: _requiredString(document, 'listId'),
    listType: _requiredString(document, 'listType'),
    revokedLicenseIds: revokedLicenseIds,
    schemaVersion: _requiredInteger(document, 'schemaVersion'),
    sequence: sequence,
  );
  if (!_timestampPattern.hasMatch(payload.generatedAt) ||
      !_identifierPattern.hasMatch(payload.keyId) ||
      !_identifierPattern.hasMatch(payload.listId) ||
      !_isSortedUniqueIdentifiers(payload.revokedLicenseIds)) {
    throw const FormatException('offline_license_revocation_list_invalid');
  }
  return ParsedRevocationListV1(
    payload: payload,
    payloadBytes: decoded.payloadBytes,
    signatureBytes: decoded.trailerBytes,
    signingMessage: _signingMessage(
      _hsrvl1SignatureDomain,
      decoded.payloadBytes,
    ),
  );
}

Future<bool> verifyOfflineLicenseV1Signature(
  ParsedOfflineLicenseV1 parsed,
  Uint8List publicKeyBytes,
) {
  return _verifyEd25519(
    parsed.signingMessage,
    parsed.signatureBytes,
    publicKeyBytes,
  );
}

bool verifyActivationRequestV1Checksum(ParsedActivationRequestV1 parsed) {
  final digest = hashes.sha256.convert(
    _signingMessage(_hsreq1ChecksumDomain, parsed.payloadBytes),
  );
  return _constantTimeEqual(
    parsed.checksumBytes,
    Uint8List.fromList(digest.bytes.take(12).toList(growable: false)),
  );
}

Future<bool> verifyRevocationListV1Signature(
  ParsedRevocationListV1 parsed,
  Uint8List publicKeyBytes,
) {
  return _verifyEd25519(
    parsed.signingMessage,
    parsed.signatureBytes,
    publicKeyBytes,
  );
}

Future<void> validateOfflineArtifactV1(
  String artifactType,
  String token, {
  Uint8List? publicKeyBytes,
}) async {
  if (artifactType == 'activation_request') {
    final parsed = parseActivationRequestV1(token);
    if (!verifyActivationRequestV1Checksum(parsed)) {
      throw const FormatException('offline_license_request_checksum_mismatch');
    }
    return;
  }
  if (publicKeyBytes == null) {
    throw const FormatException('offline_license_unknown_key');
  }
  if (artifactType == 'license') {
    final parsed = parseOfflineLicenseV1(token);
    if (!await verifyOfflineLicenseV1Signature(parsed, publicKeyBytes)) {
      throw const FormatException('offline_license_signature_invalid');
    }
    return;
  }
  if (artifactType == 'revocation_list') {
    final parsed = parseRevocationListV1(token);
    if (!await verifyRevocationListV1Signature(parsed, publicKeyBytes)) {
      throw const FormatException(
        'offline_license_revocation_signature_invalid',
      );
    }
    return;
  }
  throw const FormatException('offline_license_invalid_format');
}

String deriveInstallationIdV1(Uint8List installationSecret, Uint8List salt) {
  if (installationSecret.length != 32 || salt.length != 16) {
    throw const FormatException('offline_license_secure_storage_unavailable');
  }
  final digest = hashes.sha256.convert([
    ...utf8.encode('$_installationIdDomain\u0000'),
    ...installationSecret,
    ...salt,
  ]);
  return base64UrlEncode(digest.bytes).replaceAll('=', '');
}

Uint8List decodeOfflineLicenseBase64Url(String value) {
  return _decodeBase64UrlFor(value, 'offline_license_invalid_format');
}

Future<bool> _verifyEd25519(
  Uint8List message,
  Uint8List signatureBytes,
  Uint8List publicKeyBytes,
) async {
  if (publicKeyBytes.length != 32) {
    throw const FormatException('offline_license_unknown_key');
  }
  return Ed25519().verify(
    message,
    signature: Signature(
      signatureBytes,
      publicKey: SimplePublicKey(publicKeyBytes, type: KeyPairType.ed25519),
    ),
  );
}

_DecodedToken _decodeThreeSegmentToken(
  String token,
  String prefix,
  String errorCode,
  int trailerLength,
) {
  if (token.trim() != token || RegExp(r'\s').hasMatch(token)) {
    throw FormatException(errorCode);
  }
  final segments = token.split('.');
  if (segments.length != 3 || segments.first != prefix) {
    throw FormatException(errorCode);
  }
  final payloadBytes = _decodeBase64UrlFor(segments[1], errorCode);
  final trailerBytes = _decodeBase64UrlFor(segments[2], errorCode);
  if (trailerBytes.length != trailerLength) {
    throw FormatException(errorCode);
  }
  return _DecodedToken(payloadBytes, trailerBytes);
}

Uint8List _decodeBase64UrlFor(String value, String errorCode) {
  if (!RegExp(r'^[A-Za-z0-9_-]+$').hasMatch(value)) {
    throw FormatException(errorCode);
  }
  try {
    return Uint8List.fromList(base64Url.decode(base64Url.normalize(value)));
  } on FormatException {
    throw FormatException(errorCode);
  }
}

String _decodeUtf8(Uint8List bytes, String errorCode) {
  try {
    return utf8.decode(bytes, allowMalformed: false);
  } on FormatException {
    throw FormatException(errorCode);
  }
}

Map<String, dynamic> _parseObject(String payloadText, String errorCode) {
  try {
    final decoded = jsonDecode(payloadText);
    if (decoded is! Map<String, dynamic>) {
      throw FormatException(errorCode);
    }
    return decoded;
  } on FormatException {
    throw FormatException(errorCode);
  }
}

void _assertExactKeys(
  Map<String, dynamic> value,
  List<String> expected,
  String errorCode,
) {
  if (!_sameKeys(value.keys.toList(growable: false), expected)) {
    throw FormatException(errorCode);
  }
}

void _assertCanonical(
  Map<String, dynamic> value,
  String payloadText, [
  String errorCode = 'offline_license_non_canonical_payload',
]) {
  if (jsonEncode(value) != payloadText) {
    throw FormatException(errorCode);
  }
}

String _requiredString(Map<String, dynamic> value, String key) {
  final field = value[key];
  if (field is! String || field.isEmpty) {
    throw const FormatException('offline_license_invalid_format');
  }
  return field;
}

int _requiredInteger(Map<String, dynamic> value, String key) {
  final field = value[key];
  if (field is! int) {
    throw const FormatException('offline_license_invalid_format');
  }
  return field;
}

List<String> _requiredStringList(Map<String, dynamic> value, String key) {
  final field = value[key];
  if (field is! List || field.any((item) => item is! String)) {
    throw const FormatException('offline_license_revocation_list_invalid');
  }
  return field.cast<String>();
}

bool _isSortedUniqueIdentifiers(List<String> values) {
  String? previous;
  for (final value in values) {
    if (!_identifierPattern.hasMatch(value) ||
        (previous != null && value.compareTo(previous) <= 0)) {
      return false;
    }
    previous = value;
  }
  return true;
}

Uint8List _signingMessage(String domain, Uint8List payloadBytes) {
  return Uint8List.fromList([...utf8.encode('$domain\u0000'), ...payloadBytes]);
}

bool _constantTimeEqual(Uint8List left, Uint8List right) {
  if (left.length != right.length) return false;
  var difference = 0;
  for (var index = 0; index < left.length; index += 1) {
    difference |= left[index] ^ right[index];
  }
  return difference == 0;
}

bool _sameKeys(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index += 1) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

class _DecodedToken {
  const _DecodedToken(this.payloadBytes, this.trailerBytes);

  final Uint8List payloadBytes;
  final Uint8List trailerBytes;
}
