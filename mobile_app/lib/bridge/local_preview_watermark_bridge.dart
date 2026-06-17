import 'watermark_bridge.dart';
import 'watermark_models.dart';

class PreviewWatermarkBridge extends WatermarkBridge {
  const PreviewWatermarkBridge();

  @override
  Future<BridgeStatus> status() {
    return Future.value(
      const BridgeStatus(
        label: '本地处理已就绪',
        detail: '移动端支持图片和 WAV 音频的本地处理，视频继续由桌面端负责。',
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
    if (!_hasPreviewMarker(request.bytes)) {
      return null;
    }

    final uidPrefix = request.kind == WatermarkAssetKind.image ? 'img' : 'aud';
    final hash = _previewHash(_stripPreviewMarker(request.bytes));
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
      bytes: [...request.bytes, ..._previewMarker],
      watermarkUid: 'preview-$uidPrefix-${hash.substring(0, 12)}',
      revision: revision,
      sha256: hash,
    );
  }
}

const List<int> _previewMarker = [
  0x48,
  0x53,
  0x5f,
  0x50,
  0x52,
  0x45,
  0x56,
  0x49,
  0x45,
  0x57,
  0x5f,
  0x57,
  0x4d,
];

bool _hasPreviewMarker(List<int> bytes) {
  if (bytes.length < _previewMarker.length) {
    return false;
  }
  final offset = bytes.length - _previewMarker.length;
  for (var i = 0; i < _previewMarker.length; i += 1) {
    if (bytes[offset + i] != _previewMarker[i]) {
      return false;
    }
  }
  return true;
}

List<int> _stripPreviewMarker(List<int> bytes) {
  if (!_hasPreviewMarker(bytes)) {
    return bytes;
  }
  return bytes.sublist(0, bytes.length - _previewMarker.length);
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
