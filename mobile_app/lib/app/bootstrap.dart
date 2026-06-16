import '../bridge/watermark_bridge.dart';
import 'bootstrap_io.dart' if (dart.library.js_interop) 'bootstrap_web.dart';

Future<WatermarkBridge> createDefaultWatermarkBridge() {
  return createPlatformWatermarkBridge();
}
