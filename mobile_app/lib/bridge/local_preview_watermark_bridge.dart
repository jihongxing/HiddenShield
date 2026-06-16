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
  Future<WatermarkReadResult?> read(WatermarkReadRequest request) async {
    await Future<void>.delayed(const Duration(milliseconds: 350));
    if (request.kind == WatermarkAssetKind.video) {
      throw UnsupportedError('Mobile local video watermarking is disabled.');
    }

    final uidPrefix = request.kind == WatermarkAssetKind.image ? 'img' : 'aud';
    final hash = _previewHash(request.bytes);
    return WatermarkReadResult(
      kind: request.kind,
      watermarkUid: 'preview-$uidPrefix-${hash.substring(0, 12)}',
      revision: 1,
      timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
      deviceIdHex: '090a0b0c',
      fileHashHex: hash.substring(0, 4),
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) async {
    await Future<void>.delayed(const Duration(milliseconds: 450));
    if (request.kind == WatermarkAssetKind.video) {
      throw UnsupportedError('Mobile local video watermarking is disabled.');
    }

    final uidPrefix = request.kind == WatermarkAssetKind.image ? 'img' : 'aud';
    final revision = request.allowRewrite ? 2 : 1;
    final hash = _previewHash(request.bytes);
    return WatermarkWriteResult(
      kind: request.kind,
      bytes: request.bytes,
      watermarkUid: 'preview-$uidPrefix-${hash.substring(0, 12)}',
      revision: revision,
      sha256: hash,
    );
  }
}

String _previewHash(List<int> bytes) {
  var hash = 0x811c9dc5;
  for (final byte in bytes) {
    hash ^= byte;
    hash = (hash * 0x01000193) & 0xffffffff;
  }
  final hex = hash.toRadixString(16).padLeft(8, '0');
  return List<String>.filled(8, hex).join();
}
