import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import 'report_bundle_file_reader.dart';

typedef ReportBundleBytesReader =
    Future<Uint8List> Function(String reportDir, String relativePath);

class MobileReportBundleVerificationResult {
  const MobileReportBundleVerificationResult({
    required this.reportId,
    required this.reportType,
    required this.reportDir,
    required this.bundleVersion,
    required this.supersedesReportId,
    required this.integrityStatus,
    required this.manifestChainStatus,
    required this.documentContractStatus,
    required this.signatureStatus,
    required this.trustedTimeStatus,
    required this.files,
    required this.message,
  });

  final String reportId;
  final String reportType;
  final String reportDir;
  final int bundleVersion;
  final String? supersedesReportId;
  final String integrityStatus;
  final String manifestChainStatus;
  final String documentContractStatus;
  final String signatureStatus;
  final String trustedTimeStatus;
  final List<MobileReportVerifiedFile> files;
  final String message;

  bool get isIntegrityMatched => integrityStatus == 'matched';
}

class MobileReportVerifiedFile {
  const MobileReportVerifiedFile({
    required this.path,
    required this.expectedBytes,
    required this.actualBytes,
    required this.expectedSha256,
    required this.actualSha256,
    required this.status,
  });

  final String path;
  final int expectedBytes;
  final int? actualBytes;
  final String expectedSha256;
  final String? actualSha256;
  final String status;
}

Future<MobileReportBundleVerificationResult> verifyMobileReportBundle(
  String reportDir, {
  ReportBundleBytesReader readBytes = readReportBundleFileBytes,
}) async {
  final manifestBytes = await readBytes(reportDir, 'manifest.json');
  final manifest = _decodeObject(manifestBytes, 'manifest.json');
  final schemaVersion = _requiredInt(manifest, 'schemaVersion');
  if (schemaVersion != 2) {
    throw FormatException('不支持的 Manifest schema: $schemaVersion，当前要求 v2');
  }

  final reportId = _requiredString(manifest, 'reportId');
  final reportType = _requiredString(manifest, 'reportType');
  final bundle = _requiredObject(manifest, 'bundle');
  final filesJson = _requiredList(manifest, 'files');
  final integrity = _requiredObject(manifest, 'integrity');
  final signature = _requiredObject(manifest, 'signature');
  final trustedTime = _requiredObject(manifest, 'trustedTime');
  const allowedFilePaths = {'report.pdf', 'report.json'};
  final manifestFilePaths = filesJson.map((fileValue) {
    if (fileValue is! Map<String, dynamic>) {
      throw const FormatException('Manifest files 包含无效条目');
    }
    return _requiredString(fileValue, 'path');
  }).toSet();
  if (filesJson.length != allowedFilePaths.length ||
      manifestFilePaths.length != allowedFilePaths.length ||
      !manifestFilePaths.containsAll(allowedFilePaths)) {
    throw const FormatException(
      'Manifest v2 只允许 report.pdf 和 report.json 两个受校验文件',
    );
  }
  final files = <MobileReportVerifiedFile>[];
  Uint8List? reportJsonBytes;

  for (final fileValue in filesJson) {
    if (fileValue is! Map<String, dynamic>) {
      throw const FormatException('Manifest files 包含无效条目');
    }
    final path = _requiredString(fileValue, 'path');
    final expectedBytes = _requiredInt(fileValue, 'bytes');
    final expectedSha256 = _requiredString(fileValue, 'sha256');
    if (!_isSafeRelativePath(path)) {
      files.add(
        MobileReportVerifiedFile(
          path: path,
          expectedBytes: expectedBytes,
          actualBytes: null,
          expectedSha256: expectedSha256,
          actualSha256: null,
          status: 'unsafe_path',
        ),
      );
      continue;
    }
    try {
      final bytes = await readBytes(reportDir, path);
      if (path == 'report.json') reportJsonBytes = bytes;
      final actualSha256 = sha256.convert(bytes).toString();
      final status =
          bytes.length == expectedBytes && actualSha256 == expectedSha256
          ? 'matched'
          : 'mismatch';
      files.add(
        MobileReportVerifiedFile(
          path: path,
          expectedBytes: expectedBytes,
          actualBytes: bytes.length,
          expectedSha256: expectedSha256,
          actualSha256: actualSha256,
          status: status,
        ),
      );
    } on Object {
      files.add(
        MobileReportVerifiedFile(
          path: path,
          expectedBytes: expectedBytes,
          actualBytes: null,
          expectedSha256: expectedSha256,
          actualSha256: null,
          status: 'missing',
        ),
      );
    }
  }

  final manifestChainStatus = _verifyIntegrityChain(filesJson, integrity)
      ? 'matched'
      : 'mismatch';
  final documentContractStatus =
      _verifyDocumentContract(reportJsonBytes, reportId, reportType)
      ? 'matched'
      : 'mismatch';
  final filesMatched = files.every((file) => file.status == 'matched');
  final integrityStatus =
      filesMatched &&
          manifestChainStatus == 'matched' &&
          documentContractStatus == 'matched'
      ? 'matched'
      : 'mismatch';
  final signatureStatus = _requiredString(signature, 'status') == 'not_signed'
      ? 'not_signed'
      : 'present_unverified';
  final trustedTimeStatus = trustedTime['packageTimestampPresent'] == true
      ? 'present_unverified'
      : 'not_timestamped';

  return MobileReportBundleVerificationResult(
    reportId: reportId,
    reportType: reportType,
    reportDir: reportDir,
    bundleVersion: _requiredInt(bundle, 'bundleVersion'),
    supersedesReportId: bundle['supersedesReportId'] as String?,
    integrityStatus: integrityStatus,
    manifestChainStatus: manifestChainStatus,
    documentContractStatus: documentContractStatus,
    signatureStatus: signatureStatus,
    trustedTimeStatus: trustedTimeStatus,
    files: files,
    message: integrityStatus == 'matched'
        ? '报告包文件、Manifest 摘要链与 report.json 合同匹配；当前报告包未签名，也未获得报告包级可信时间戳。'
        : '报告包校验失败；至少一个文件、Manifest 摘要链或 report.json 合同不匹配。',
  );
}

bool _verifyIntegrityChain(
  List<dynamic> files,
  Map<String, dynamic> integrity,
) {
  if (integrity['algorithm'] != 'sha256_chain_v1') return false;
  final genesis = integrity['genesis'];
  final entries = integrity['entries'];
  final rootDigest = integrity['rootDigest'];
  if (genesis is! String ||
      entries is! List<dynamic> ||
      rootDigest is! String ||
      entries.length != files.length) {
    return false;
  }

  var previousChainDigest = sha256.convert(utf8.encode(genesis)).toString();
  for (var index = 0; index < files.length; index += 1) {
    final file = files[index];
    final entry = entries[index];
    if (file is! Map<String, dynamic> || entry is! Map<String, dynamic>) {
      return false;
    }
    final sequence = index + 1;
    final path = file['path'];
    final bytes = file['bytes'];
    final fileSha256 = file['sha256'];
    if (path is! String ||
        bytes is! int ||
        fileSha256 is! String ||
        entry['sequence'] != sequence ||
        entry['path'] != path ||
        entry['fileBytes'] != bytes ||
        entry['fileSha256'] != fileSha256 ||
        entry['previousChainDigest'] != previousChainDigest) {
      return false;
    }
    final chainDigest = sha256
        .convert(
          utf8.encode(
            '$sequence\n$path\n$bytes\n$fileSha256\n$previousChainDigest',
          ),
        )
        .toString();
    if (entry['chainDigest'] != chainDigest) return false;
    previousChainDigest = chainDigest;
  }
  return previousChainDigest == rootDigest;
}

bool _verifyDocumentContract(
  Uint8List? reportJsonBytes,
  String reportId,
  String reportType,
) {
  if (reportJsonBytes == null) return false;
  try {
    final document = _decodeObject(reportJsonBytes, 'report.json');
    return document['schemaVersion'] == 2 &&
        document['reportId'] == reportId &&
        document['reportType'] == reportType;
  } on Object {
    return false;
  }
}

Map<String, dynamic> _decodeObject(Uint8List bytes, String label) {
  final decoded = jsonDecode(utf8.decode(bytes));
  if (decoded is! Map<String, dynamic>) {
    throw FormatException('$label 必须是 JSON object');
  }
  return decoded;
}

Map<String, dynamic> _requiredObject(Map<String, dynamic> source, String key) {
  final value = source[key];
  if (value is! Map<String, dynamic>) {
    throw FormatException('$key 必须是 object');
  }
  return value;
}

List<dynamic> _requiredList(Map<String, dynamic> source, String key) {
  final value = source[key];
  if (value is! List<dynamic>) {
    throw FormatException('$key 必须是 array');
  }
  return value;
}

String _requiredString(Map<String, dynamic> source, String key) {
  final value = source[key];
  if (value is! String || value.isEmpty) {
    throw FormatException('$key 必须是非空字符串');
  }
  return value;
}

int _requiredInt(Map<String, dynamic> source, String key) {
  final value = source[key];
  if (value is! int) {
    throw FormatException('$key 必须是整数');
  }
  return value;
}

bool _isSafeRelativePath(String value) {
  return value.isNotEmpty &&
      value != '.' &&
      value != '..' &&
      !value.contains('/') &&
      !value.contains('\\') &&
      !value.contains(':');
}
