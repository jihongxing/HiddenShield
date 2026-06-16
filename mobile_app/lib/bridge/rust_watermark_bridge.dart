import 'dart:typed_data';

import '../src/rust/api.dart' as rust_api;
import '../src/rust/frb_generated.dart';
import 'watermark_bridge.dart';
import 'watermark_models.dart';

class RustWatermarkBridge extends WatermarkBridge {
  RustWatermarkBridge();

  static Future<void> init() => RustLib.init();

  @override
  Future<BridgeStatus> status() {
    return Future.value(
      const BridgeStatus(
        label: 'Rust 桥接层已就绪',
        detail: '图片写入将调用 watermark-core 的移动端 Rust API；音频和平台打包仍在接入中。',
        capabilities: BridgeCapabilities(
          supportedKinds: [WatermarkAssetKind.image],
          supportsDesktopSync: false,
          supportsLocalVideo: false,
        ),
      ),
    );
  }

  @override
  Future<WatermarkReadResult?> read(WatermarkReadRequest request) async {
    if (request.kind != WatermarkAssetKind.image) {
      throw UnsupportedError(
        'Rust bridge currently supports image reads only.',
      );
    }
    final result = await rust_api.extractImageForMobile(
      imageBytes: request.bytes,
    );
    return WatermarkReadResult(
      kind: WatermarkAssetKind.image,
      watermarkUid: result.watermarkUid,
      revision: 1,
      timestamp: result.timestamp.toInt(),
      deviceIdHex: result.deviceIdHex,
      fileHashHex: result.fileHashHex,
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) async {
    if (request.kind != WatermarkAssetKind.image) {
      throw UnsupportedError(
        'Rust bridge currently supports image writes only.',
      );
    }
    final result = await rust_api.embedImageForMobile(
      imageBytes: request.bytes,
      payload: rust_api.MobileMediaPayload(
        userSeed: Uint8List.fromList(request.seed.userSeed),
        timestamp: BigInt.from(request.seed.timestamp),
        deviceId: Uint8List.fromList(request.seed.deviceId),
        fileHash: Uint8List.fromList(request.seed.fileHash),
      ),
      outputFormat: rust_api.MobileImageOutputFormat.png,
      allowRewrite: request.allowRewrite,
    );
    return WatermarkWriteResult(
      kind: WatermarkAssetKind.image,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: request.allowRewrite ? 2 : 1,
      sha256: result.sha256,
    );
  }
}
