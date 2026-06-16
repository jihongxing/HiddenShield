import '../bridge/local_preview_watermark_bridge.dart';
import '../bridge/rust_watermark_bridge.dart';
import '../bridge/watermark_bridge.dart';

Future<WatermarkBridge> createPlatformWatermarkBridge() async {
  try {
    await RustWatermarkBridge.init();
    return RustWatermarkBridge();
  } catch (_) {
    return const PreviewWatermarkBridge();
  }
}
