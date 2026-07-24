import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
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
    expect(reason.detail, contains('版本信息'));
  });

  test('maps local vault verification to desktop-compatible reason codes', () {
    final record = VaultRecord(
      id: 'record-1',
      kind: WatermarkAssetKind.audio,
      title: 'song.wav',
      watermarkUid: 'uid',
      revision: 2,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.synced,
      createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
    );

    expect(MobileVerifyReason.matchedOriginal(record).code, 'matched_original');
    expect(
      MobileVerifyReason.matchedHashMismatch(record).code,
      'matched_hash_mismatch',
    );
    expect(
      MobileVerifyReason.unregisteredWatermark().code,
      'watermark_detected_unregistered',
    );
  });

  test('maps errors to actionable reason codes', () {
    expect(
      MobileVerifyReason.forError('ffmpeg unavailable').code,
      'audio_extract_failed',
    );
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
      MobileVerifyReason.forError(
        'image_watermark_extract_failed: decode',
      ).checklist,
      contains('图片默认检测会尝试轻度缩放、轻裁剪、局部遮挡、90/180/270 度旋转与水平/垂直镜像。'),
    );
    expect(
      MobileVerifyReason.forError(
        'image_watermark_extract_failed: decode',
      ).checklist,
      contains('任意角度旋转、严重裁剪或内容不足时，请优先换更接近原发布内容的样本。'),
    );
    expect(
      MobileVerifyReason.forError('image_read_failed: denied').code,
      'file_read_failed',
    );
  });
}
