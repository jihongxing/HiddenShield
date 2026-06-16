import 'watermark_models.dart';

abstract class WatermarkBridge {
  const WatermarkBridge();

  Future<BridgeStatus> status();

  Future<WatermarkWriteResult> write(WatermarkWriteRequest request);

  Future<WatermarkReadResult?> read(WatermarkReadRequest request);
}
