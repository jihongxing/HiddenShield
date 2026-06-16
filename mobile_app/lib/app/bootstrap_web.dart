import '../bridge/local_preview_watermark_bridge.dart';
import '../bridge/watermark_bridge.dart';

Future<WatermarkBridge> createPlatformWatermarkBridge() async {
  return const PreviewWatermarkBridge();
}
