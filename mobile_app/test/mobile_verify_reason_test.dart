import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/verify/mobile_verify_reason.dart';

void main() {
  test('maps successful verification with lineage to reason code', () {
    final reason = MobileVerifyReason.forSuccess(
      const WatermarkReadResult(
        kind: WatermarkAssetKind.image,
        watermarkUid: 'uid',
        revision: 2,
        timestamp: 123,
        deviceIdHex: 'device',
        fileHashHex: 'hash',
        parentWatermarkUid: 'parent',
        rewriteReason: 'rewrite',
      ),
    );

    expect(reason.code, 'matched_with_lineage');
    expect(reason.detail, contains('链路'));
  });

  test('maps errors to actionable reason codes', () {
    expect(
      MobileVerifyReason.forError('audio_extract_failed').code,
      'audio_extract_failed',
    );
    expect(
      MobileVerifyReason.forError(
        'image_watermark_extract_failed: decode',
      ).code,
      'no_valid_watermark',
    );
    expect(
      MobileVerifyReason.forError('image_read_failed: denied').code,
      'file_read_failed',
    );
  });
}
