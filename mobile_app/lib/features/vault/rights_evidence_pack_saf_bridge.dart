import 'package:flutter/services.dart';

import 'rights_evidence_pack_access_failure.dart';
import 'rights_evidence_pack_file_reader.dart';

export 'rights_evidence_pack_access_failure.dart';

class SafRightsEvidencePackDirectory {
  const SafRightsEvidencePackDirectory({
    required this.treeUri,
    required this.displayName,
    required this.persisted,
  });

  final String treeUri;
  final String displayName;
  final bool persisted;

  factory SafRightsEvidencePackDirectory.fromJson(Map<Object?, Object?> json) {
    return SafRightsEvidencePackDirectory(
      treeUri: json['treeUri']! as String,
      displayName: json['displayName']! as String,
      persisted: json['persisted']! as bool,
    );
  }
}

class RightsEvidencePackSafBridge {
  const RightsEvidencePackSafBridge({
    this.channel = const MethodChannel(
      'com.hiddenshield.hidden_shield_mobile/rights_evidence_saf',
    ),
  });

  final MethodChannel channel;

  Future<SafRightsEvidencePackDirectory?> pickDirectory() async {
    final result = await channel.invokeMapMethod<Object?, Object?>('pickTree');
    return result == null
        ? null
        : SafRightsEvidencePackDirectory.fromJson(result);
  }

  Future<SafRightsEvidencePackDirectory?> getPersistedDirectory() async {
    final result = await channel.invokeMapMethod<Object?, Object?>(
      'getPersistedTree',
    );
    return result == null
        ? null
        : SafRightsEvidencePackDirectory.fromJson(result);
  }

  Future<void> clearPersistedDirectory() {
    return channel.invokeMethod<void>('clearPersistedTree');
  }

  Future<Uint8List> readBytes(String treeUri, String relativePath) async {
    try {
      final bytes = await channel.invokeMethod<Uint8List>('readFile', {
        'treeUri': treeUri,
        'relativePath': relativePath,
      });
      if (bytes == null) {
        throw const RightsEvidencePackAccessException(
          code: RightsEvidencePackAccessFailureCode.unknown,
          userMessage: '案件包目录读取失败，请重新选择后重试。',
          technicalMessage: 'Android SAF 未返回案件包文件字节。',
        );
      }
      return bytes;
    } on PlatformException catch (error) {
      throw _accessExceptionFromPlatform(error);
    }
  }

  Future<RightsEvidencePackDirectoryListing> listDirectory(
    String treeUri,
  ) async {
    try {
      final result = await channel.invokeMapMethod<Object?, Object?>(
        'listDirectory',
        {'treeUri': treeUri},
      );
      if (result == null) {
        throw const RightsEvidencePackAccessException(
          code: RightsEvidencePackAccessFailureCode.unknown,
          userMessage: '案件包目录读取失败，请重新选择后重试。',
          technicalMessage: 'Android SAF 未返回案件包目录结构。',
        );
      }
      return RightsEvidencePackDirectoryListing(
        topLevelEntries: _stringList(result['topLevelEntries']),
        attachmentPaths: _stringList(result['attachmentPaths']),
        caseFileSafe: result['caseFileSafe'] == true,
        manifestFileSafe: result['manifestFileSafe'] == true,
        attachmentTreeSafe: result['attachmentTreeSafe'] == true,
      );
    } on PlatformException catch (error) {
      throw _accessExceptionFromPlatform(error);
    }
  }

  List<String> _stringList(Object? value) {
    return (value! as List<Object?>).cast<String>();
  }

  RightsEvidencePackAccessException _accessExceptionFromPlatform(
    PlatformException error,
  ) {
    final code = RightsEvidencePackAccessFailureCode.fromWireCode(error.code);
    return RightsEvidencePackAccessException(
      code: code,
      userMessage: code.userMessage,
      technicalMessage: error.message,
    );
  }
}
