import 'dart:typed_data';

import 'rights_evidence_pack_file_reader_stub.dart'
    if (dart.library.io) 'rights_evidence_pack_file_reader_io.dart'
    as implementation;

class RightsEvidencePackDirectoryListing {
  const RightsEvidencePackDirectoryListing({
    required this.topLevelEntries,
    required this.attachmentPaths,
    required this.caseFileSafe,
    required this.manifestFileSafe,
    required this.attachmentTreeSafe,
  });

  final List<String> topLevelEntries;
  final List<String> attachmentPaths;
  final bool caseFileSafe;
  final bool manifestFileSafe;
  final bool attachmentTreeSafe;
}

Future<Uint8List> readRightsEvidencePackFileBytes(
  String caseDir,
  String relativePath,
) {
  return implementation.readRightsEvidencePackFileBytes(caseDir, relativePath);
}

Future<RightsEvidencePackDirectoryListing> listRightsEvidencePackDirectory(
  String caseDir,
) {
  return implementation.listRightsEvidencePackDirectory(caseDir);
}
