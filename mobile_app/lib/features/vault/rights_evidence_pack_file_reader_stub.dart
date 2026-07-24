import 'dart:typed_data';

import 'rights_evidence_pack_file_reader.dart';

Future<Uint8List> readRightsEvidencePackFileBytes(
  String caseDir,
  String relativePath,
) {
  throw UnsupportedError('当前预览环境不能读取案件包：$caseDir/$relativePath');
}

Future<RightsEvidencePackDirectoryListing> listRightsEvidencePackDirectory(
  String caseDir,
) {
  throw UnsupportedError('当前预览环境不能枚举案件包：$caseDir');
}
