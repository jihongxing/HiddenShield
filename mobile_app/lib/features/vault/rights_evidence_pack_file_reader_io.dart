import 'dart:io';
import 'dart:typed_data';

import 'package:path/path.dart' as path;

import 'rights_evidence_pack_file_reader.dart';

Future<Uint8List> readRightsEvidencePackFileBytes(
  String caseDir,
  String relativePath,
) async {
  final canonicalRoot = await Directory(caseDir).resolveSymbolicLinks();
  final file = File(path.joinAll([caseDir, ...relativePath.split('/')]));
  final canonicalFile = await file.resolveSymbolicLinks();
  final rootPrefix = canonicalRoot.endsWith(Platform.pathSeparator)
      ? canonicalRoot
      : '$canonicalRoot${Platform.pathSeparator}';
  if (!canonicalFile.startsWith(rootPrefix)) {
    throw FileSystemException('案件包文件路径越界', canonicalFile);
  }
  return file.readAsBytes();
}

Future<RightsEvidencePackDirectoryListing> listRightsEvidencePackDirectory(
  String caseDir,
) async {
  final root = Directory(caseDir);
  final topLevelEntries = <String>[];
  await for (final entity in root.list(followLinks: false)) {
    topLevelEntries.add(path.basename(entity.path));
  }
  topLevelEntries.sort();

  final caseFileSafe =
      await FileSystemEntity.type(
        path.join(caseDir, 'case.json'),
        followLinks: false,
      ) ==
      FileSystemEntityType.file;
  final manifestFileSafe =
      await FileSystemEntity.type(
        path.join(caseDir, 'case-manifest.json'),
        followLinks: false,
      ) ==
      FileSystemEntityType.file;
  final attachmentsRoot = path.join(caseDir, 'attachments');
  final attachmentsRootType = await FileSystemEntity.type(
    attachmentsRoot,
    followLinks: false,
  );
  if (attachmentsRootType != FileSystemEntityType.directory) {
    return RightsEvidencePackDirectoryListing(
      topLevelEntries: topLevelEntries,
      attachmentPaths: const [],
      caseFileSafe: caseFileSafe,
      manifestFileSafe: manifestFileSafe,
      attachmentTreeSafe: false,
    );
  }

  final attachmentPaths = <String>[];
  var attachmentTreeSafe = true;
  await for (final entity in Directory(
    attachmentsRoot,
  ).list(recursive: true, followLinks: false)) {
    final entityType = await FileSystemEntity.type(
      entity.path,
      followLinks: false,
    );
    if (entityType == FileSystemEntityType.file) {
      attachmentPaths.add(
        path.relative(entity.path, from: caseDir).replaceAll(r'\', '/'),
      );
    } else if (entityType != FileSystemEntityType.directory) {
      attachmentTreeSafe = false;
    }
  }
  attachmentPaths.sort();

  return RightsEvidencePackDirectoryListing(
    topLevelEntries: topLevelEntries,
    attachmentPaths: attachmentPaths,
    caseFileSafe: caseFileSafe,
    manifestFileSafe: manifestFileSafe,
    attachmentTreeSafe: attachmentTreeSafe,
  );
}
