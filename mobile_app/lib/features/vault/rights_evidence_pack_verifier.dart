import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import 'rights_evidence_pack_access_failure.dart';
import 'rights_evidence_pack_file_reader.dart';

typedef RightsEvidencePackBytesReader =
    Future<Uint8List> Function(String caseDir, String relativePath);
typedef RightsEvidencePackDirectoryReader =
    Future<RightsEvidencePackDirectoryListing> Function(String caseDir);

class RightsEvidencePackVerificationResult {
  const RightsEvidencePackVerificationResult({
    required this.packId,
    required this.caseId,
    required this.caseDir,
    required this.verifiedAt,
    required this.manifestSchemaVersion,
    required this.directoryContractStatus,
    required this.attachmentIntegrityStatus,
    required this.eventChainStatus,
    required this.attachmentChainStatus,
    required this.signatureStatus,
    required this.trustedTimeStatus,
    required this.declaredRootDigest,
    required this.computedRootDigest,
    required this.attachments,
    required this.message,
  });

  final String? packId;
  final String? caseId;
  final String caseDir;
  final String verifiedAt;
  final int? manifestSchemaVersion;
  final String directoryContractStatus;
  final String attachmentIntegrityStatus;
  final String eventChainStatus;
  final String attachmentChainStatus;
  final String signatureStatus;
  final String trustedTimeStatus;
  final String? declaredRootDigest;
  final String? computedRootDigest;
  final List<RightsEvidencePackVerifiedAttachment> attachments;
  final String message;

  Map<String, dynamic> toJson() {
    return {
      'packId': packId,
      'caseId': caseId,
      'caseDir': caseDir,
      'verifiedAt': verifiedAt,
      'manifestSchemaVersion': manifestSchemaVersion,
      'directoryContractStatus': directoryContractStatus,
      'attachmentIntegrityStatus': attachmentIntegrityStatus,
      'eventChainStatus': eventChainStatus,
      'attachmentChainStatus': attachmentChainStatus,
      'signatureStatus': signatureStatus,
      'trustedTimeStatus': trustedTimeStatus,
      'declaredRootDigest': declaredRootDigest,
      'computedRootDigest': computedRootDigest,
      'attachments': attachments
          .map((attachment) => attachment.toJson())
          .toList(),
      'message': message,
    };
  }
}

class RightsEvidencePackVerifiedAttachment {
  const RightsEvidencePackVerifiedAttachment({
    required this.attachmentId,
    required this.path,
    required this.role,
    required this.expectedBytes,
    required this.actualBytes,
    required this.expectedSha256,
    required this.actualSha256,
    required this.status,
  });

  final String attachmentId;
  final String path;
  final String role;
  final int expectedBytes;
  final int? actualBytes;
  final String expectedSha256;
  final String? actualSha256;
  final String status;

  Map<String, dynamic> toJson() {
    return {
      'attachmentId': attachmentId,
      'path': path,
      'role': role,
      'expectedBytes': expectedBytes,
      'actualBytes': actualBytes,
      'expectedSha256': expectedSha256,
      'actualSha256': actualSha256,
      'status': status,
    };
  }
}

class RightsEvidencePackVerifier {
  const RightsEvidencePackVerifier({
    this.readBytes = readRightsEvidencePackFileBytes,
    this.readDirectory = listRightsEvidencePackDirectory,
  });

  final RightsEvidencePackBytesReader readBytes;
  final RightsEvidencePackDirectoryReader readDirectory;

  Future<RightsEvidencePackVerificationResult> verify(String caseDir) async {
    final caseBytes = await readBytes(caseDir, 'case.json');
    final manifestBytes = await readBytes(caseDir, 'case-manifest.json');
    final caseDocument = _decodeObject(caseBytes, 'case.json');
    final manifest = _decodeObject(manifestBytes, 'case-manifest.json');
    final directory = await readDirectory(caseDir);

    final packId = _requiredString(caseDocument, 'packId');
    final caseIdentity = _requiredObject(caseDocument, 'case');
    final caseId = _requiredString(caseIdentity, 'caseId');
    final caseAttachments = _requiredObjectList(caseDocument, 'attachments');
    final manifestFiles = _requiredObjectList(manifest, 'files');
    final collectionEvents = _requiredObjectList(
      caseDocument,
      'collectionEvents',
    );
    final automatedFindings = _requiredObjectList(
      caseDocument,
      'automatedFindings',
    );

    final expectedAttachmentPaths =
        caseAttachments
            .map((attachment) => _requiredString(attachment, 'relativePath'))
            .toList()
          ..sort();
    final manifestAttachmentPaths =
        manifestFiles
            .map((attachment) => _requiredString(attachment, 'path'))
            .toList()
          ..sort();
    final physicalAttachmentPaths = [...directory.attachmentPaths]..sort();

    final attachmentMetadataValid = _verifyAttachmentMetadata(
      caseAttachments,
      automatedFindings,
      manifestFiles,
    );
    final attachments = <RightsEvidencePackVerifiedAttachment>[];
    for (final manifestFile in manifestFiles) {
      attachments.add(await _verifyAttachment(caseDir, manifestFile));
    }
    final attachmentFilesMatch = attachments.every(
      (attachment) => attachment.status == 'matched',
    );
    final attachmentIntegrityValid =
        attachmentMetadataValid &&
        attachmentFilesMatch &&
        directory.attachmentTreeSafe &&
        _sameStrings(physicalAttachmentPaths, expectedAttachmentPaths) &&
        _sameStrings(physicalAttachmentPaths, manifestAttachmentPaths);

    final expectedEventChain = _buildEventChain(collectionEvents);
    final expectedAttachmentChain = _buildAttachmentChain(caseAttachments);
    final eventChainValid =
        stableRightsEvidenceJsonString(expectedEventChain) ==
        stableRightsEvidenceJsonString(_requiredObject(manifest, 'eventChain'));
    final attachmentChainValid =
        stableRightsEvidenceJsonString(expectedAttachmentChain) ==
        stableRightsEvidenceJsonString(
          _requiredObject(manifest, 'attachmentChain'),
        );

    final caseDigest = _sha256Bytes(caseBytes);
    final eventRootDigest = _requiredString(expectedEventChain, 'rootDigest');
    final attachmentRootDigest = _requiredString(
      expectedAttachmentChain,
      'rootDigest',
    );
    final computedRootDigest = _sha256Text(
      'HiddenShield-Rights-Evidence-Pack-Root-v1\n'
      '$caseDigest\n$eventRootDigest\n$attachmentRootDigest',
    );

    final directoryContract = _requiredObject(manifest, 'directoryContract');
    final caseFile = _requiredObject(manifest, 'caseFile');
    final integrity = _requiredObject(manifest, 'integrity');
    final allowedTopLevelEntries = _requiredStringList(
      directoryContract,
      'allowedTopLevelEntries',
    )..sort();
    final actualTopLevelEntries = [...directory.topLevelEntries]..sort();
    const expectedTopLevelEntries = [
      'attachments',
      'case-manifest.json',
      'case.json',
    ];
    final directoryContractValid =
        _requiredInt(manifest, 'schemaVersion') == 1 &&
        _requiredString(manifest, 'manifestType') ==
            'rights_evidence_pack_manifest' &&
        _requiredInt(caseDocument, 'schemaVersion') == 1 &&
        _requiredString(caseDocument, 'documentType') ==
            'rights_evidence_pack' &&
        _requiredString(manifest, 'packId') == packId &&
        _requiredString(manifest, 'caseId') == caseId &&
        _requiredString(directoryContract, 'caseDocument') == 'case.json' &&
        _requiredString(directoryContract, 'manifest') ==
            'case-manifest.json' &&
        _requiredString(directoryContract, 'attachmentRoot') ==
            'attachments/' &&
        _sameStrings(allowedTopLevelEntries, expectedTopLevelEntries) &&
        _sameStrings(actualTopLevelEntries, expectedTopLevelEntries) &&
        directory.caseFileSafe &&
        directory.manifestFileSafe &&
        _requiredString(caseFile, 'path') == 'case.json' &&
        _requiredInt(caseFile, 'bytes') == caseBytes.length &&
        _requiredString(caseFile, 'sha256') == caseDigest &&
        _requiredString(integrity, 'algorithm') ==
            'sha256_case_event_attachment_roots_v1' &&
        _requiredString(integrity, 'rootDigest') == computedRootDigest &&
        directory.attachmentTreeSafe &&
        _sameStrings(physicalAttachmentPaths, manifestAttachmentPaths);

    final signature = _requiredObject(manifest, 'signature');
    final trustedTime = _requiredObject(manifest, 'trustedTime');
    final signatureStatus = _requiredString(signature, 'status') == 'not_signed'
        ? 'not_signed'
        : 'present_unverified';
    final trustedTimeStatus =
        _requiredString(trustedTime, 'status') == 'not_timestamped'
        ? 'not_timestamped'
        : 'present_unverified';
    final allIntegrityMatched =
        directoryContractValid &&
        attachmentIntegrityValid &&
        eventChainValid &&
        attachmentChainValid;

    return RightsEvidencePackVerificationResult(
      packId: packId,
      caseId: caseId,
      caseDir: caseDir,
      verifiedAt: DateTime.now().toUtc().toIso8601String(),
      manifestSchemaVersion: _requiredInt(manifest, 'schemaVersion'),
      directoryContractStatus: _status(directoryContractValid),
      attachmentIntegrityStatus: _status(attachmentIntegrityValid),
      eventChainStatus: _status(eventChainValid),
      attachmentChainStatus: _status(attachmentChainValid),
      signatureStatus: signatureStatus,
      trustedTimeStatus: trustedTimeStatus,
      declaredRootDigest: _requiredString(integrity, 'rootDigest'),
      computedRootDigest: computedRootDigest,
      attachments: attachments,
      message: allIntegrityMatched
          ? '案件包目录、附件、采集事件链和附件链匹配；当前案件包未签名，也未获得包级可信时间。'
          : '案件包至少一项目录或完整性校验不匹配；请保留原目录并核对逐项状态。',
    );
  }

  Future<RightsEvidencePackVerifiedAttachment> _verifyAttachment(
    String caseDir,
    Map<String, dynamic> expected,
  ) async {
    final attachmentId = _requiredString(expected, 'attachmentId');
    final relativePath = _requiredString(expected, 'path');
    final role = _requiredString(expected, 'role');
    final expectedBytes = _requiredInt(expected, 'bytes');
    final expectedSha256 = _requiredString(expected, 'sha256');
    if (!_isSafeAttachmentPath(relativePath)) {
      return RightsEvidencePackVerifiedAttachment(
        attachmentId: attachmentId,
        path: relativePath,
        role: role,
        expectedBytes: expectedBytes,
        actualBytes: null,
        expectedSha256: expectedSha256,
        actualSha256: null,
        status: 'unsafe_path',
      );
    }
    try {
      final bytes = await readBytes(caseDir, relativePath);
      final actualSha256 = _sha256Bytes(bytes);
      return RightsEvidencePackVerifiedAttachment(
        attachmentId: attachmentId,
        path: relativePath,
        role: role,
        expectedBytes: expectedBytes,
        actualBytes: bytes.length,
        expectedSha256: expectedSha256,
        actualSha256: actualSha256,
        status: bytes.length == expectedBytes && actualSha256 == expectedSha256
            ? 'matched'
            : 'mismatch',
      );
    } on RightsEvidencePackAccessException {
      rethrow;
    } on Object {
      return RightsEvidencePackVerifiedAttachment(
        attachmentId: attachmentId,
        path: relativePath,
        role: role,
        expectedBytes: expectedBytes,
        actualBytes: null,
        expectedSha256: expectedSha256,
        actualSha256: null,
        status: 'missing',
      );
    }
  }
}

String stableRightsEvidenceJsonString(Object? value) {
  if (value == null || value is bool || value is String || value is int) {
    return jsonEncode(value);
  }
  if (value is double) {
    if (!value.isFinite) {
      throw const FormatException('稳定 JSON 不接受非有限数字');
    }
    if (value == value.truncateToDouble()) {
      return value.toInt().toString();
    }
    return jsonEncode(value);
  }
  if (value is List) {
    return '[${value.map(stableRightsEvidenceJsonString).join(',')}]';
  }
  if (value is Map) {
    if (value.keys.any((key) => key is! String)) {
      throw const FormatException('稳定 JSON object 键必须是字符串');
    }
    final object = value.cast<String, dynamic>();
    final keys = object.keys.toList()..sort();
    return '{${keys.map((key) {
      return '${jsonEncode(key)}:${stableRightsEvidenceJsonString(object[key])}';
    }).join(',')}}';
  }
  throw FormatException('稳定 JSON 不支持类型 ${value.runtimeType}');
}

Map<String, dynamic> _buildEventChain(List<Map<String, dynamic>> events) {
  const genesis = 'HiddenShield-Rights-Evidence-Pack-Event-Chain-v1';
  var previousChainDigest = _sha256Text(genesis);
  final entries = <Map<String, dynamic>>[];
  for (var index = 0; index < events.length; index += 1) {
    final sequence = index + 1;
    final event = events[index];
    final eventId = _requiredString(event, 'eventId');
    if (_requiredInt(event, 'sequence') != sequence) {
      throw FormatException('采集事件 sequence 不连续: eventId=$eventId');
    }
    final eventDigest = _sha256Text(stableRightsEvidenceJsonString(event));
    final chainDigest = _sha256Text(
      '$sequence\n$eventId\n$eventDigest\n$previousChainDigest',
    );
    entries.add({
      'sequence': sequence,
      'eventId': eventId,
      'eventDigest': eventDigest,
      'previousChainDigest': previousChainDigest,
      'chainDigest': chainDigest,
    });
    previousChainDigest = chainDigest;
  }
  return {
    'algorithm': 'sha256_append_chain_v1',
    'genesis': genesis,
    'entries': entries,
    'rootDigest': previousChainDigest,
  };
}

Map<String, dynamic> _buildAttachmentChain(
  List<Map<String, dynamic>> attachments,
) {
  const genesis = 'HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1';
  var previousChainDigest = _sha256Text(genesis);
  final entries = <Map<String, dynamic>>[];
  for (var index = 0; index < attachments.length; index += 1) {
    final sequence = index + 1;
    final attachment = attachments[index];
    final attachmentId = _requiredString(attachment, 'attachmentId');
    if (_requiredInt(attachment, 'sequence') != sequence) {
      throw FormatException('附件 sequence 不连续: attachmentId=$attachmentId');
    }
    final relativePath = _requiredString(attachment, 'relativePath');
    final role = _requiredString(attachment, 'role');
    final fileBytes = _requiredInt(attachment, 'bytes');
    final fileSha256 = _requiredString(attachment, 'sha256');
    final chainDigest = _sha256Text(
      '$sequence\n$attachmentId\n$relativePath\n$role\n'
      '$fileBytes\n$fileSha256\n$previousChainDigest',
    );
    entries.add({
      'sequence': sequence,
      'attachmentId': attachmentId,
      'path': relativePath,
      'role': role,
      'fileBytes': fileBytes,
      'fileSha256': fileSha256,
      'previousChainDigest': previousChainDigest,
      'chainDigest': chainDigest,
    });
    previousChainDigest = chainDigest;
  }
  return {
    'algorithm': 'sha256_append_chain_v1',
    'genesis': genesis,
    'entries': entries,
    'rootDigest': previousChainDigest,
  };
}

bool _verifyAttachmentMetadata(
  List<Map<String, dynamic>> attachments,
  List<Map<String, dynamic>> automatedFindings,
  List<Map<String, dynamic>> manifestFiles,
) {
  if (attachments.length != manifestFiles.length) return false;
  const allowedRoles = {
    'original',
    'working_copy',
    'capture',
    'external_receipt',
  };
  final attachmentIds = attachments
      .map((attachment) => _requiredString(attachment, 'attachmentId'))
      .toSet();
  final attachmentPaths = attachments
      .map((attachment) => _requiredString(attachment, 'relativePath'))
      .toSet();
  if (attachmentIds.length != attachments.length ||
      attachmentPaths.length != attachments.length) {
    return false;
  }
  for (final finding in automatedFindings) {
    for (final attachmentId in _requiredStringList(
      finding,
      'inputAttachmentIds',
    )) {
      if (!attachmentIds.contains(attachmentId)) return false;
    }
  }
  for (var index = 0; index < attachments.length; index += 1) {
    final attachment = attachments[index];
    final manifestFile = manifestFiles[index];
    final attachmentId = _requiredString(attachment, 'attachmentId');
    final relativePath = _requiredString(attachment, 'relativePath');
    final role = _requiredString(attachment, 'role');
    final derivedFrom = attachment['derivedFromAttachmentId'];
    if (_requiredInt(attachment, 'sequence') != index + 1 ||
        !allowedRoles.contains(role) ||
        !_isSafeAttachmentPath(relativePath) ||
        attachmentId != _requiredString(manifestFile, 'attachmentId') ||
        relativePath != _requiredString(manifestFile, 'path') ||
        role != _requiredString(manifestFile, 'role') ||
        _requiredInt(attachment, 'bytes') !=
            _requiredInt(manifestFile, 'bytes') ||
        _requiredString(attachment, 'sha256') !=
            _requiredString(manifestFile, 'sha256')) {
      return false;
    }
    if (role == 'working_copy') {
      if (derivedFrom is! String) return false;
      final sourceIndex = attachments.indexWhere(
        (candidate) =>
            _requiredString(candidate, 'attachmentId') == derivedFrom,
      );
      if (sourceIndex < 0 ||
          sourceIndex >= index ||
          _requiredString(attachments[sourceIndex], 'role') != 'original') {
        return false;
      }
    } else if (derivedFrom != null) {
      return false;
    }
  }
  return true;
}

bool _isSafeAttachmentPath(String value) {
  if (value.contains(r'\')) return false;
  final components = value.split('/');
  return components.length >= 3 &&
      components.first == 'attachments' &&
      components.every(
        (component) =>
            component.isNotEmpty && component != '.' && component != '..',
      );
}

Map<String, dynamic> _decodeObject(Uint8List bytes, String name) {
  final value = jsonDecode(utf8.decode(bytes));
  if (value is! Map<String, dynamic>) {
    throw FormatException('$name 顶层必须是 JSON object');
  }
  return value;
}

Map<String, dynamic> _requiredObject(Map<String, dynamic> value, String key) {
  final item = value[key];
  if (item is! Map<String, dynamic>) {
    throw FormatException('$key 必须是 object');
  }
  return item;
}

List<Map<String, dynamic>> _requiredObjectList(
  Map<String, dynamic> value,
  String key,
) {
  final items = value[key];
  if (items is! List) throw FormatException('$key 必须是 array');
  return items.map((item) {
    if (item is! Map<String, dynamic>) {
      throw FormatException('$key 包含无效条目');
    }
    return item;
  }).toList();
}

List<String> _requiredStringList(Map<String, dynamic> value, String key) {
  final items = value[key];
  if (items is! List || items.any((item) => item is! String)) {
    throw FormatException('$key 必须是 string array');
  }
  return items.cast<String>().toList();
}

String _requiredString(Map<String, dynamic> value, String key) {
  final item = value[key];
  if (item is! String || item.isEmpty) {
    throw FormatException('$key 必须是非空字符串');
  }
  return item;
}

int _requiredInt(Map<String, dynamic> value, String key) {
  final item = value[key];
  if (item is! int) throw FormatException('$key 必须是整数');
  return item;
}

String _sha256Text(String value) {
  return sha256.convert(utf8.encode(value)).toString();
}

String _sha256Bytes(List<int> value) {
  return sha256.convert(value).toString();
}

String _status(bool matched) => matched ? 'matched' : 'mismatch';

bool _sameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index += 1) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
