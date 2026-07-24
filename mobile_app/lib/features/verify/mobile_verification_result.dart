import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_models.dart';
import 'mobile_verify_reason.dart';

class MobileVerificationResult {
  const MobileVerificationResult({
    required this.readResult,
    required this.matched,
    required this.confidence,
    required this.reason,
    this.matchedRecord,
  });

  final WatermarkReadResult readResult;
  final bool matched;
  final double confidence;
  final MobileVerifyReason reason;
  final VaultRecord? matchedRecord;
}

MobileVerificationResult buildMobileVerificationResult({
  required WatermarkReadResult readResult,
  required List<VaultRecord> records,
}) {
  final sameUidRecords = records
      .where((record) => record.watermarkUid == readResult.watermarkUid)
      .toList(growable: false);
  final matchedRecord = sameUidRecords.cast<VaultRecord?>().firstWhere(
    (record) => _recordMatchesReadHash(record, readResult),
    orElse: () => null,
  );

  if (matchedRecord != null) {
    return MobileVerificationResult(
      readResult: readResult,
      matched: true,
      confidence: 1,
      matchedRecord: matchedRecord,
      reason: MobileVerifyReason.matchedOriginal(matchedRecord),
    );
  }

  if (sameUidRecords.isNotEmpty) {
    return MobileVerificationResult(
      readResult: readResult,
      matched: false,
      confidence: 1,
      matchedRecord: sameUidRecords.first,
      reason: MobileVerifyReason.matchedHashMismatch(sameUidRecords.first),
    );
  }

  return MobileVerificationResult(
    readResult: readResult,
    matched: false,
    confidence: 1,
    reason: MobileVerifyReason.unregisteredWatermark(),
  );
}

bool _recordMatchesReadHash(VaultRecord? record, WatermarkReadResult result) {
  if (record == null) {
    return false;
  }
  final prefix = result.fileHashHex.toLowerCase();
  if (prefix.isEmpty) {
    return false;
  }
  final sha256 = record.sha256?.toLowerCase();
  if (sha256?.startsWith(prefix) == true) {
    return true;
  }
  final extractedHash = record.extractedFileHashHex?.toLowerCase();
  return extractedHash == prefix || extractedHash?.startsWith(prefix) == true;
}
