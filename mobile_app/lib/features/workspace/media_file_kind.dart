import '../../bridge/watermark_models.dart';

const supportedImageExtensions = ['jpg', 'jpeg', 'png', 'bmp', 'tiff', 'webp'];
const supportedAudioExtensions = ['wav', 'mp3', 'aac', 'flac', 'ogg', 'm4a'];
const supportedVideoExtensions = ['mp4', 'mov', 'mkv', 'webm'];

const supportedEmbeddableMediaExtensions = [
  ...supportedImageExtensions,
  ...supportedAudioExtensions,
];

const supportedMediaExtensions = [
  ...supportedEmbeddableMediaExtensions,
  ...supportedVideoExtensions,
];

WatermarkAssetKind? mediaKindForFileName(String fileName) {
  final extension = fileNameExtension(fileName);
  if (supportedImageExtensions.contains(extension)) {
    return WatermarkAssetKind.image;
  }
  if (supportedAudioExtensions.contains(extension)) {
    return WatermarkAssetKind.audio;
  }
  if (supportedVideoExtensions.contains(extension)) {
    return WatermarkAssetKind.video;
  }
  return null;
}

String fileNameExtension(String fileName) {
  final match = RegExp(r'\.([^.]+)$').firstMatch(fileName.trim());
  return match?.group(1)?.toLowerCase() ?? '';
}
