import 'watermark_bridge.dart';
import 'watermark_models.dart';

class PreviewWatermarkBridge extends WatermarkBridge {
  const PreviewWatermarkBridge();

  @override
  Future<BridgeStatus> status() {
    return Future.value(
      const BridgeStatus(
        label: '桥接层已接入',
        detail: 'Flutter 侧已预留 Rust/FRB 接口，当前移动端仅开放图片和音频的本地链路，视频仍由桌面端处理。',
        capabilities: BridgeCapabilities(
          supportedKinds: [WatermarkAssetKind.image, WatermarkAssetKind.audio],
          supportsDesktopSync: true,
          supportsLocalVideo: false,
        ),
      ),
    );
  }

  @override
  Future<WatermarkReadResult?> read(WatermarkReadRequest request) {
    return Future.error(
      UnsupportedError('Preview bridge does not read watermarks yet.'),
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) {
    return Future.error(
      UnsupportedError('Preview bridge does not write watermarks yet.'),
    );
  }
}
