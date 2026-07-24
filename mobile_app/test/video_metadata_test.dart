import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/workspace/video_metadata.dart';

void main() {
  test('inspectVideoMetadata reads MP4 duration, dimensions, and fps', () {
    final bytes = _makeMp4(
      durationSeconds: 4,
      width: 1920,
      height: 1080,
      frameCount: 120,
    );

    final metadata = inspectVideoMetadata(bytes, fileName: 'clip.mp4');

    expect(metadata, isNotNull);
    expect(metadata!.container, 'mp4');
    expect(metadata.durationSeconds, closeTo(4, 0.001));
    expect(metadata.width, 1920);
    expect(metadata.height, 1080);
    expect(metadata.frameCount, 120);
    expect(metadata.frameRate, closeTo(30, 0.001));
    expect(metadata.probeSchema, trustedVideoMetadataProbe);
    expect(metadata.hasTrustedProbe, isTrue);
  });

  test('inspectVideoMetadata reads frame count from stts fallback', () {
    final bytes = _makeMp4(
      durationSeconds: 6,
      width: 1280,
      height: 720,
      frameCount: 144,
      includeStsz: false,
    );

    final metadata = inspectVideoMetadata(bytes, fileName: 'clip.mov');

    expect(metadata, isNotNull);
    expect(metadata!.container, 'mov');
    expect(metadata.frameCount, 144);
    expect(metadata.frameRate, closeTo(24, 0.001));
  });

  test('inspectVideoMetadata rejects malformed video bytes', () {
    final metadata = inspectVideoMetadata(Uint8List.fromList([1, 2, 3]));

    expect(metadata, isNull);
  });
}

Uint8List _makeMp4({
  required int durationSeconds,
  required int width,
  required int height,
  required int frameCount,
  bool includeStsz = true,
}) {
  const timescale = 1000;
  final duration = durationSeconds * timescale;
  return _concat([
    _box('ftyp', _concat([_ascii('isom'), _u32(0), _ascii('isommp42')])),
    _box(
      'moov',
      _concat([
        _box(
          'mvhd',
          _fullBox(
            _concat([_u32(0), _u32(0), _u32(timescale), _u32(duration)]),
          ),
        ),
        _box(
          'trak',
          _concat([
            _box('tkhd', _tkhdPayload(width: width, height: height)),
            _box(
              'mdia',
              _concat([
                _box(
                  'mdhd',
                  _fullBox(
                    _concat([
                      _u32(0),
                      _u32(0),
                      _u32(timescale),
                      _u32(duration),
                    ]),
                  ),
                ),
                _box(
                  'minf',
                  _box(
                    'stbl',
                    _concat([
                      _box(
                        'stts',
                        _fullBox(
                          _concat([
                            _u32(1),
                            _u32(frameCount),
                            _u32(duration ~/ frameCount),
                          ]),
                        ),
                      ),
                      if (includeStsz)
                        _box(
                          'stsz',
                          _fullBox(_concat([_u32(0), _u32(frameCount)])),
                        ),
                    ]),
                  ),
                ),
              ]),
            ),
          ]),
        ),
      ]),
    ),
  ]);
}

Uint8List _tkhdPayload({required int width, required int height}) {
  final payload = Uint8List(84);
  final data = ByteData.sublistView(payload);
  data.setUint32(0, 0x00000007, Endian.big);
  data.setUint32(12, 1, Endian.big);
  data.setUint32(20, 4000, Endian.big);
  data.setUint32(76, width << 16, Endian.big);
  data.setUint32(80, height << 16, Endian.big);
  return payload;
}

Uint8List _fullBox(Uint8List payload) => _concat([_u32(0), payload]);

Uint8List _box(String type, Uint8List payload) {
  final box = Uint8List(8 + payload.length);
  final data = ByteData.sublistView(box);
  data.setUint32(0, box.length, Endian.big);
  box.setRange(4, 8, _ascii(type));
  box.setRange(8, box.length, payload);
  return box;
}

Uint8List _u32(int value) {
  final bytes = Uint8List(4);
  ByteData.sublistView(bytes).setUint32(0, value, Endian.big);
  return bytes;
}

Uint8List _ascii(String value) =>
    Uint8List.fromList(value.codeUnits.map((unit) => unit & 0xFF).toList());

Uint8List _concat(List<Uint8List> chunks) {
  final length = chunks.fold<int>(0, (sum, chunk) => sum + chunk.length);
  final bytes = Uint8List(length);
  var offset = 0;
  for (final chunk in chunks) {
    bytes.setRange(offset, offset + chunk.length, chunk);
    offset += chunk.length;
  }
  return bytes;
}
