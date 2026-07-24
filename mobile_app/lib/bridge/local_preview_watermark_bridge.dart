import 'watermark_bridge.dart';
import 'watermark_models.dart';

class PreviewWatermarkBridge extends WatermarkBridge {
  const PreviewWatermarkBridge();

  @override
  bool get supportsProductionWatermark => false;

  @override
  Future<BridgeStatus> status() {
    return Future.value(
      const BridgeStatus(
        label: 'Web 预览模式',
        detail: '当前浏览器预览只用于界面体验，不生成可被桌面端验证的正式盲水印。请使用移动端原生运行进行真实写入和跨端验证。',
        capabilities: BridgeCapabilities(
          supportedKinds: [],
          supportsDesktopSync: false,
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

    final hash = _previewHash(_stripPreviewMarker(request.bytes));
    final markerUid = _readPreviewWatermarkUid(request.bytes);
    return WatermarkReadResult(
      kind: request.kind,
      watermarkUid: markerUid ?? _previewWatermarkUidFromHash(hash),
      revision: 1,
      timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
      deviceIdHex: '090a0b0c',
      fileHashHex: hash.substring(0, 4),
      isProductionWatermark: false,
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) async {
    final startedAt = DateTime.now();
    await Future<void>.delayed(const Duration(milliseconds: 450));
    if (request.kind == WatermarkAssetKind.video) {
      throw UnsupportedError('Mobile local video watermarking is disabled.');
    }
    if (!request.allowRewrite && _hasPreviewMarker(request.bytes)) {
      final existingUid =
          _readPreviewWatermarkUid(request.bytes) ??
          _previewWatermarkUidFromHash(
            _previewHash(_stripPreviewMarker(request.bytes)),
          );
      throw StateError(
        'watermark already exists in source media: $existingUid',
      );
    }
    final revision = request.allowRewrite ? 2 : 1;
    final hash = _previewHash(request.bytes);
    final watermarkUid = _previewWatermarkUidFromSeed(request.seed);
    return WatermarkWriteResult(
      kind: request.kind,
      bytes: [...request.bytes, ..._previewMarkerForUid(watermarkUid)],
      watermarkUid: watermarkUid,
      revision: revision,
      sha256: hash,
      seed: request.seed,
      processTimeMs: DateTime.now().difference(startedAt).inMilliseconds,
      isProductionWatermark: false,
      verification: WatermarkWriteVerification(
        verified: true,
        watermarkUid: watermarkUid,
        revision: revision,
        message: '已回读验证版权编号，保护副本可取证。',
        fileHashHex: hash.substring(0, 4),
        deviceIdHex: '090a0b0c',
      ),
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

const List<int> _previewMarkerSeparator = [0x3a];
const int _previewWatermarkUidLength = 20;

bool _hasPreviewMarker(List<int> bytes) {
  if (_readPreviewWatermarkUid(bytes) != null) {
    return true;
  }
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
  final newMarkerStart = _newPreviewMarkerStart(bytes);
  if (newMarkerStart != null) {
    return bytes.sublist(0, newMarkerStart);
  }
  if (!_hasLegacyPreviewMarker(bytes)) {
    return bytes;
  }
  return bytes.sublist(0, bytes.length - _previewMarker.length);
}

bool _hasLegacyPreviewMarker(List<int> bytes) {
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

List<int> _previewMarkerForUid(String watermarkUid) {
  return [
    ..._previewMarker,
    ..._previewMarkerSeparator,
    ...watermarkUid.codeUnits,
  ];
}

String? _readPreviewWatermarkUid(List<int> bytes) {
  final markerStart = _newPreviewMarkerStart(bytes);
  if (markerStart == null) {
    return null;
  }
  final uidStart = markerStart + _previewMarker.length + 1;
  return String.fromCharCodes(bytes.sublist(uidStart));
}

int? _newPreviewMarkerStart(List<int> bytes) {
  final markerLength =
      _previewMarker.length +
      _previewMarkerSeparator.length +
      _previewWatermarkUidLength;
  if (bytes.length < markerLength) {
    return null;
  }
  final offset = bytes.length - markerLength;
  for (var i = 0; i < _previewMarker.length; i += 1) {
    if (bytes[offset + i] != _previewMarker[i]) {
      return null;
    }
  }
  if (bytes[offset + _previewMarker.length] != _previewMarkerSeparator.single) {
    return null;
  }
  final uid = String.fromCharCodes(
    bytes.sublist(offset + _previewMarker.length + 1),
  );
  return RegExp(r'^PREVIEW-[0-9A-F]{12}$').hasMatch(uid) ? offset : null;
}

String _previewWatermarkUidFromSeed(WatermarkPayloadSeed seed) {
  final creatorBytes = seed.creatorIdentity.codeUnits;
  final deviceBytes = seed.deviceIdentity.codeUnits;

  String byteAt(List<int> bytes, int index) {
    final value = index < bytes.length ? bytes[index] : 0;
    return value.toRadixString(16).padLeft(2, '0').toUpperCase();
  }

  return 'PREVIEW-${byteAt(creatorBytes, 0)}${byteAt(creatorBytes, 1)}'
      '${byteAt(creatorBytes, 2)}${byteAt(creatorBytes, 3)}'
      '${byteAt(deviceBytes, 0)}${byteAt(deviceBytes, 1)}';
}

String _previewWatermarkUidFromHash(String hash) {
  final normalized = hash.padRight(12, '0').substring(0, 12).toUpperCase();
  return 'PREVIEW-$normalized';
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
