import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/verify/mobile_verification_result.dart';

void main() {
  test('matches local vault record by uid and hash prefix', () {
    final result = buildMobileVerificationResult(
      readResult: const WatermarkReadResult(
        kind: WatermarkAssetKind.audio,
        watermarkUid: 'uid-audio',
        revision: 1,
        timestamp: 1,
        deviceIdHex: 'device',
        fileHashHex: 'abcd',
      ),
      records: [
        _record(watermarkUid: 'uid-audio', sha256: 'abcdef123456', revision: 3),
      ],
    );

    expect(result.matched, isTrue);
    expect(result.confidence, 1);
    expect(result.reason.code, 'matched_original');
    expect(result.matchedRecord?.revision, 3);
  });

  test('reports hash mismatch when uid exists but fingerprint changed', () {
    final result = buildMobileVerificationResult(
      readResult: const WatermarkReadResult(
        kind: WatermarkAssetKind.image,
        watermarkUid: 'uid-image',
        revision: 1,
        timestamp: 1,
        deviceIdHex: 'device',
        fileHashHex: 'abcd',
      ),
      records: [_record(watermarkUid: 'uid-image', sha256: '99887766')],
    );

    expect(result.matched, isFalse);
    expect(result.reason.code, 'matched_hash_mismatch');
    expect(result.matchedRecord?.watermarkUid, 'uid-image');
  });

  test('reports unregistered watermark when uid is not local', () {
    final result = buildMobileVerificationResult(
      readResult: const WatermarkReadResult(
        kind: WatermarkAssetKind.image,
        watermarkUid: 'uid-remote',
        revision: 1,
        timestamp: 1,
        deviceIdHex: 'device',
        fileHashHex: 'abcd',
      ),
      records: [_record(watermarkUid: 'uid-local', sha256: 'abcdef')],
    );

    expect(result.matched, isFalse);
    expect(result.matchedRecord, isNull);
    expect(result.reason.code, 'watermark_detected_unregistered');
  });
}

VaultRecord _record({
  required String watermarkUid,
  required String sha256,
  int revision = 1,
}) {
  return VaultRecord(
    id: 'record-$watermarkUid',
    kind: WatermarkAssetKind.audio,
    title: '$watermarkUid.wav',
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
    source: VaultRecordSource.write,
    syncStatus: SyncStatus.synced,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}
