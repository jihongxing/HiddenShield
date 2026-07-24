import 'watermark_models.dart';

abstract class WatermarkBridge {
  const WatermarkBridge();

  bool get supportsProductionWatermark => true;

  Future<BridgeStatus> status();

  Future<WatermarkWriteResult> write(WatermarkWriteRequest request);

  Future<WatermarkReadResult?> read(WatermarkReadRequest request);

  Future<WatermarkReadResult?> readReadonlyCandidate(
    WatermarkReadRequest request,
  ) {
    return read(request);
  }

  Future<WatermarkReadResult?> detectExisting(WatermarkReadRequest request) {
    return read(request);
  }
}
