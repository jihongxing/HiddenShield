import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_models.dart';

class MobileVerifyReason {
  const MobileVerifyReason({
    required this.code,
    required this.detail,
    this.checklist = const [],
  });

  final String code;
  final String detail;
  final List<String> checklist;

  static const defaultDetectionScope =
      '默认检测只读取当前样本，会尝试图片二次保存、JPEG 压缩、轻度缩放/轻裁剪、局部遮挡、90/180/270 度旋转与水平/垂直镜像。任意角度旋转、严重裁剪或内容不足不作为默认范围。';
  static const vaultDeepDetectionScope =
      '版权库深度检测需要本机已有作品记录，用于对裁剪、尺寸变化等疑难样本做复核；它不是纯盲检测，也不会上传原始文件或保护副本。';

  static const imageTroubleshootingChecklist = [
    '确认样本是否由隐盾生成过保护副本。',
    '图片默认检测会尝试轻度缩放、轻裁剪、局部遮挡、90/180/270 度旋转与水平/垂直镜像。',
    '任意角度旋转、严重裁剪或内容不足时，请优先换更接近原发布内容的样本。',
    '音频样本请确认音轨仍完整，且没有被大幅重采样或二次转码。',
    '仍无法识别时，保留样本和本机版权库记录，再发送反馈排查。',
  ];

  factory MobileVerifyReason.noWatermark() {
    return const MobileVerifyReason(
      code: 'no_valid_watermark',
      detail: '没有找到可验证的隐盾版权记录；可能不是本软件处理的文件，或样本已经超出默认检测范围。',
      checklist: imageTroubleshootingChecklist,
    );
  }

  factory MobileVerifyReason.forSuccess(WatermarkReadResult result) {
    if (result.parentWatermarkUid != null || result.rewriteReason != null) {
      return const MobileVerifyReason(
        code: 'matched_with_lineage',
        detail: '已读取到版权记录，并能识别版本信息，说明这是带版本记录的保护副本。',
      );
    }
    return const MobileVerifyReason(
      code: 'matched_original',
      detail: '已读取到版权记录，当前样本可与本机版权库中的记录对应。',
    );
  }

  factory MobileVerifyReason.matchedOriginal(VaultRecord record) {
    return MobileVerifyReason(
      code: 'matched_original',
      detail: '水印有效，并已命中本机版权库记录：第 ${record.revision} 次版本。',
    );
  }

  factory MobileVerifyReason.matchedHashMismatch(VaultRecord record) {
    return MobileVerifyReason(
      code: 'matched_hash_mismatch',
      detail: '检测到有效水印，版权编号命中本机记录，但样本指纹与原记录不一致。文件可能经过压缩、裁剪、转码或二次传播。',
      checklist: imageTroubleshootingChecklist,
    );
  }

  factory MobileVerifyReason.unregisteredWatermark() {
    return const MobileVerifyReason(
      code: 'watermark_detected_unregistered',
      detail: '检测到有效水印，但本机版权库没有对应记录。可能来自其他设备，或记录尚未同步到当前设备。',
    );
  }

  factory MobileVerifyReason.forError(String error) {
    final lower = error.toLowerCase();
    if (lower.contains('ffmpeg')) {
      return const MobileVerifyReason(
        code: 'audio_extract_failed',
        detail: '音频检查未完成；请确认文件没有损坏，并尽量使用完整音频作品重新检查。',
        checklist: imageTroubleshootingChecklist,
      );
    }
    if (lower.contains('audio_extract_failed')) {
      return const MobileVerifyReason(
        code: 'audio_extract_failed',
        detail: '无法从音频文件读取可验证片段；可能没有音轨、音轨损坏或格式暂不受支持。',
        checklist: imageTroubleshootingChecklist,
      );
    }
    if (lower.contains('image_read_failed') ||
        lower.contains('wav_read_failed')) {
      return const MobileVerifyReason(
        code: 'file_read_failed',
        detail: '样本读取失败，请确认文件存在且当前用户有读取权限。',
        checklist: imageTroubleshootingChecklist,
      );
    }
    if (lower.contains('watermark_extract_failed')) {
      return const MobileVerifyReason(
        code: 'no_valid_watermark',
        detail: '未读取到可验证记录；图片或音频可能已经超出默认检测范围。',
        checklist: imageTroubleshootingChecklist,
      );
    }
    return MobileVerifyReason(
      code: 'extract_failed',
      detail: error,
      checklist: imageTroubleshootingChecklist,
    );
  }
}
