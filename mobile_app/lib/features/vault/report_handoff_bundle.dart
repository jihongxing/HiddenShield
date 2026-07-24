import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import '../../app/mobile_app_state.dart';

class MobileReportHandoffBundle {
  const MobileReportHandoffBundle({
    required this.reportId,
    required this.reportJsonBytes,
    required this.manifestJsonBytes,
  });

  final String reportId;
  final Uint8List reportJsonBytes;
  final Uint8List manifestJsonBytes;
}

MobileReportHandoffBundle buildMobileReportHandoffBundle({
  required VaultRecord record,
  required FormalReportDraft draft,
}) {
  final reportDocument = <String, dynamic>{
    'schemaVersion': 2,
    'reportId': draft.reportId,
    'reportType': 'formal_report_handoff',
    'exportedAt': draft.exportedAt.toUtc().toIso8601String(),
    'appVersion': 'mobile',
    'sourcePlatform': 'flutter_mobile',
    'records': [
      {
        'sourceRecordId': record.id,
        'mediaKind': record.kind.name,
        'fileName': record.title,
        'watermarkUid': record.watermarkUid,
        'creatorDisplayName': record.creatorDisplayName,
        'originalHash': record.sha256 ?? record.extractedFileHashHex ?? '',
        'createdAt': record.createdAt.toUtc().toIso8601String(),
        'revision': record.revision,
        'parentWatermarkUid': record.parentWatermarkUid,
        'rewriteReason': record.rewriteReason,
        'writeVerificationStatus': record.writeVerificationStatus?.name,
        'writeVerificationMessage': record.writeVerificationMessage,
        'writeVerificationAt': record.writeVerificationAt
            ?.toUtc()
            .toIso8601String(),
        'payloadRegistry': {
          'payloadProtocolVersion': record.payloadProtocolVersion,
          'payloadBytesLength': record.payloadBytesLength,
          'mediaPayloadRole': record.payloadProtocolVersion >= 3
              ? 'v3_minimal_anchor'
              : 'v2_full_record',
          'watermarkIdIssueMode': record.watermarkIdIssueMode,
          'watermarkIdRegistryStatus': record.watermarkIdRegistryStatus,
          'watermarkIdRegistryReceipt': record.watermarkIdRegistryReceipt,
          'payloadAuthStatus': record.payloadAuthStatus,
        },
        'protectedCopy': {
          'name': record.protectedCopyName,
          'hash': record.protectedCopyHash,
          'outputStrategy': record.outputStrategy,
        },
        'trustedTime': {
          'networkTime': record.trustedTimeAt?.toUtc().toIso8601String(),
          'tsaSource': record.trustedTimeSource,
          'tsaTokenPresent': false,
        },
        'rightsDeclaration': {
          'workSourceDeclaration': record.workSourceDeclaration,
          'trainingPermissionDeclaration': record.trainingPermissionDeclaration,
          'creationMethodDeclaration': record.creationMethodDeclaration,
          'humanEditLevelDeclaration': record.humanEditLevelDeclaration,
          'authenticityClaimDeclaration': record.authenticityClaimDeclaration,
          'customRightsStatement': record.customRightsStatement,
        },
        'videoNotary': {
          'notaryId': record.videoNotaryId,
          'notaryAt': record.videoNotaryAt?.toUtc().toIso8601String(),
          'receiptSignature': record.videoNotaryReceiptSignature,
          'usageLedgerId': record.videoNotaryUsageLedgerId,
          'fingerprintRoot': record.videoFingerprintRoot,
          'bundleSha256': record.videoBundleSha256,
          'bundleBytes': record.videoBundleBytes,
          'bundleSceneCount': record.videoBundleSceneCount,
          'bundleElapsedMs': record.videoBundleElapsedMs,
          'frameSamplePolicy': record.videoFrameSamplePolicy,
        },
        'videoVisualWatermark': {
          'taskId': record.videoVisualTaskId,
          'completedAt': record.videoVisualCompletedAt
              ?.toUtc()
              .toIso8601String(),
          'strategyDigest': record.videoVisualStrategyDigest,
          'selfCheckConfidence': record.videoVisualSelfCheckConfidence,
          'selfCheckThreshold': record.videoVisualSelfCheckThreshold,
          'checkedFrames': record.videoVisualCheckedFrames,
          'mediaHash': record.videoVisualMediaHash,
          'receiptHash': record.videoVisualReceiptHash,
          'outputBytes': record.videoVisualOutputBytes,
          'outputContentType': record.videoVisualOutputContentType,
        },
      },
    ],
    'privacy': {
      'excludesOriginalMedia': true,
      'excludesWatermarkedMedia': true,
      'excludesLocalMediaPaths': true,
    },
    'handoff': {
      'status': 'awaiting_desktop_render',
      'requestedOutput': ['report.pdf', 'report.json', 'manifest.json'],
      'note': '该交接包尚未生成 PDF，也未完成数字签名或报告包可信时间。',
    },
    'disclaimer':
        '本交接包由 HiddenShield 移动端根据本地版权记录生成，仅用于桌面渲染或云端签发交接，不构成法律意见、司法鉴定意见或诉讼结果承诺。',
  };
  final reportJsonBytes = _prettyJsonBytes(reportDocument);
  final reportFile = {
    'path': 'report.json',
    'mediaType': 'application/json',
    'bytes': reportJsonBytes.length,
    'sha256': sha256.convert(reportJsonBytes).toString(),
  };
  final integrity = _buildIntegrityChain([reportFile]);
  final generatedAt = draft.exportedAt.toUtc().toIso8601String();
  final manifest = <String, dynamic>{
    'schemaVersion': 2,
    'reportId': draft.reportId,
    'reportType': 'formal_report_handoff',
    'generatedAt': generatedAt,
    'sourceSchemaVersion': 2,
    'bundle': {
      'sourceKey': sha256
          .convert(utf8.encode('formal_report_handoff|${record.id}'))
          .toString(),
      'bundleVersion': 1,
      'supersedesReportId': null,
    },
    'renderer': {
      'engine': 'mobile_handoff',
      'workerMode': 'not_rendered',
      'templateVersion': 'R3-handoff-v1',
      'controlledFonts': <String>[],
      'generationMs': 0,
      'generationBudgetMs': 0,
      'pageCount': 0,
      'paginationStable': true,
    },
    'files': [reportFile],
    'integrity': integrity,
    'signature': {
      'status': 'not_signed',
      'profile': null,
      'signerKeyId': null,
      'certificateChainStatus': 'not_evaluated',
      'revocationStatus': 'not_applicable',
      'signedAt': null,
      'note': '移动交接包尚未生成 PDF，也未执行数字签名。',
    },
    'trustedTime': {
      'status': 'not_verified',
      'packageTimestampPresent': false,
      'recordMaterialTokenPresent': record.trustedTimeAt != null,
      'note': '记录级时间材料不等于交接包已获得可信时间戳。',
    },
    'verification': {
      'offlineMode': 'sha256_chain_v1',
      'onlineStatus': 'not_deployed',
      'qrStatus': 'not_issued',
      'onlineVerificationUrl': null,
    },
  };
  return MobileReportHandoffBundle(
    reportId: draft.reportId,
    reportJsonBytes: reportJsonBytes,
    manifestJsonBytes: _prettyJsonBytes(manifest),
  );
}

Map<String, dynamic> _buildIntegrityChain(List<Map<String, dynamic>> files) {
  const genesis = 'HiddenShield-Report-Manifest-v2';
  var previousChainDigest = sha256.convert(utf8.encode(genesis)).toString();
  final entries = <Map<String, dynamic>>[];
  for (var index = 0; index < files.length; index += 1) {
    final sequence = index + 1;
    final file = files[index];
    final chainDigest = sha256
        .convert(
          utf8.encode(
            '$sequence\n${file['path']}\n${file['bytes']}\n'
            '${file['sha256']}\n$previousChainDigest',
          ),
        )
        .toString();
    entries.add({
      'sequence': sequence,
      'path': file['path'],
      'fileSha256': file['sha256'],
      'fileBytes': file['bytes'],
      'previousChainDigest': previousChainDigest,
      'chainDigest': chainDigest,
    });
    previousChainDigest = chainDigest;
  }
  return {
    'algorithm': 'sha256_chain_v1',
    'genesis': genesis,
    'entries': entries,
    'rootDigest': previousChainDigest,
  };
}

Uint8List _prettyJsonBytes(Map<String, dynamic> value) {
  final text = '${const JsonEncoder.withIndent('  ').convert(value)}\n';
  return Uint8List.fromList(utf8.encode(text));
}
