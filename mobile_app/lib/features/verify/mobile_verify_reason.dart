import '../../bridge/watermark_models.dart';

class MobileVerifyReason {
  const MobileVerifyReason({required this.code, required this.detail});

  final String code;
  final String detail;

  factory MobileVerifyReason.noWatermark() {
    return const MobileVerifyReason(
      code: 'no_valid_watermark',
      detail: '没有检测到有效隐盾水印；可能不是本软件处理的文件，或水印已被严重压缩、裁剪、替换。',
    );
  }

  factory MobileVerifyReason.forSuccess(WatermarkReadResult result) {
    if (result.parentWatermarkUid != null || result.rewriteReason != null) {
      return const MobileVerifyReason(
        code: 'matched_with_lineage',
        detail: '已提取到有效水印，并能读取重写链路，说明这是带链路信息的存证版本。',
      );
    }
    return const MobileVerifyReason(
      code: 'matched_original',
      detail: '已提取到有效水印，当前样本可与本机版权库中的记录对应。',
    );
  }

  factory MobileVerifyReason.forError(String error) {
    final lower = error.toLowerCase();
    if (lower.contains('ffmpeg')) {
      return const MobileVerifyReason(
        code: 'ffmpeg_unavailable',
        detail: '音频取证需要 FFmpeg，当前环境未找到可用组件。',
      );
    }
    if (lower.contains('audio_extract_failed')) {
      return const MobileVerifyReason(
        code: 'audio_extract_failed',
        detail: '无法从音频文件抽取可检测音轨；可能没有音轨、音轨损坏或格式暂不受支持。',
      );
    }
    if (lower.contains('image_read_failed') ||
        lower.contains('wav_read_failed')) {
      return const MobileVerifyReason(
        code: 'file_read_failed',
        detail: '文件读取失败，请确认文件存在且当前用户有读取权限。',
      );
    }
    if (lower.contains('watermark_extract_failed')) {
      return const MobileVerifyReason(
        code: 'no_valid_watermark',
        detail: '未提取到可验证水印；可能经过强压缩、裁剪、重采样或转码。',
      );
    }
    return MobileVerifyReason(code: 'extract_failed', detail: error);
  }
}
