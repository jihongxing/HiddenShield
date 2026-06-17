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
        label: '本地处理已就绪',
        detail: '图片和 WAV 音频可直接在本机处理，视频继续由桌面端负责。',
        capabilities: BridgeCapabilities(
          supportedKinds: [WatermarkAssetKind.image, WatermarkAssetKind.audio],
          supportsDesktopSync: false,
          supportsLocalVideo: false,
        ),
      ),
    );
  }

  @override
  Future<WatermarkReadResult?> read(WatermarkReadRequest request) async {
    final result = switch (request.kind) {
      WatermarkAssetKind.image => await rust_api.extractImageForMobile(
        imageBytes: request.bytes,
      ),
      WatermarkAssetKind.audio => await rust_api.extractAudioWavForMobile(
        audioBytes: request.bytes,
      ),
      WatermarkAssetKind.video => throw UnsupportedError(
        'Mobile local video watermarking is disabled.',
      ),
    };
    return WatermarkReadResult(
      kind: request.kind,
      watermarkUid: result.watermarkUid,
      revision: 1,
      timestamp: result.timestamp.toInt(),
      deviceIdHex: result.deviceIdHex,
      fileHashHex: result.fileHashHex,
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) async {
    final payload = rust_api.MobileMediaPayload(
      userSeed: Uint8List.fromList(request.seed.userSeed),
      timestamp: BigInt.from(request.seed.timestamp),
      deviceId: Uint8List.fromList(request.seed.deviceId),
      fileHash: Uint8List.fromList(request.seed.fileHash),
    );
    return switch (request.kind) {
      WatermarkAssetKind.image => _writeImage(request, payload),
      WatermarkAssetKind.audio => _writeAudio(request, payload),
      WatermarkAssetKind.video => throw UnsupportedError(
        'Mobile local video watermarking is disabled.',
      ),
    };
  }

  Future<WatermarkWriteResult> _writeImage(
    WatermarkWriteRequest request,
    rust_api.MobileMediaPayload payload,
  ) async {
    final result = await rust_api.embedImageForMobile(
      imageBytes: request.bytes,
      payload: payload,
      outputFormat: rust_api.MobileImageOutputFormat.png,
      allowRewrite: request.allowRewrite,
    );
    final revision = request.allowRewrite ? 2 : 1;
    final verification = await _verifyWriteResult(
      kind: WatermarkAssetKind.image,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: revision,
    );
    return WatermarkWriteResult(
      kind: WatermarkAssetKind.image,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: revision,
      sha256: result.sha256,
      verification: verification,
    );
  }

  Future<WatermarkWriteResult> _writeAudio(
    WatermarkWriteRequest request,
    rust_api.MobileMediaPayload payload,
  ) async {
    final result = await rust_api.embedAudioWavForMobile(
      audioBytes: request.bytes,
      payload: payload,
      allowRewrite: request.allowRewrite,
    );
    final revision = request.allowRewrite ? 2 : 1;
    final verification = await _verifyWriteResult(
      kind: WatermarkAssetKind.audio,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: revision,
    );
    return WatermarkWriteResult(
      kind: WatermarkAssetKind.audio,
      bytes: result.bytes,
      watermarkUid: result.watermarkUid,
      revision: revision,
      sha256: result.sha256,
      verification: verification,
    );
  }

  Future<WatermarkWriteVerification> _verifyWriteResult({
    required WatermarkAssetKind kind,
    required List<int> bytes,
    required String watermarkUid,
    required int revision,
  }) async {
    final extracted = await read(WatermarkReadRequest(kind: kind, bytes: bytes));
    if (extracted == null) {
      throw StateError('写入后回读失败，保护副本暂不可取证。');
    }
    if (extracted.watermarkUid != watermarkUid) {
      throw StateError(
        '写入后回读的版权编号不一致，期望 $watermarkUid，实际 ${extracted.watermarkUid}。',
      );
    }
    return WatermarkWriteVerification(
      verified: true,
      watermarkUid: extracted.watermarkUid,
      revision: revision,
      message: '已回读验证版权编号，保护副本可取证。',
      fileHashHex: extracted.fileHashHex,
      deviceIdHex: extracted.deviceIdHex,
    );
  }
}
