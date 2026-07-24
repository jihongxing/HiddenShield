import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/public_rights/public_metadata_embedder.dart';

void main() {
  test('PNG embedding writes iTXt XMP with public rights identifiers', () {
    final embedded = embedPublicRightsMetadataInImage(
      sourceBytes: _minimalPng(),
      metadata: _metadata(),
    );
    final checks = checkEmbeddedPublicMetadataBytes(
      bytes: embedded.bytes,
      format: PublicMetadataImageFormat.png,
      watermarkUid: 'HS-TEST-UID',
      manifestHash: 'sha256:manifest-test',
    );

    expect(embedded.format, PublicMetadataImageFormat.png);
    expect(embedded.legalConclusion, isFalse);
    expect(checks.pass, isTrue);
  });

  test('JPEG embedding writes APP1 XMP with public rights identifiers', () {
    final embedded = embedPublicRightsMetadataInImage(
      sourceBytes: Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xD9]),
      metadata: _metadata(),
    );
    final checks = checkEmbeddedPublicMetadataBytes(
      bytes: embedded.bytes,
      format: PublicMetadataImageFormat.jpeg,
      watermarkUid: 'HS-TEST-UID',
      manifestHash: 'sha256:manifest-test',
    );

    expect(embedded.format, PublicMetadataImageFormat.jpeg);
    expect(embedded.bytes.take(4), [0xFF, 0xD8, 0xFF, 0xE1]);
    expect(checks.pass, isTrue);
  });

  test('embedding rejects legalConclusion true', () {
    expect(
      () => embedPublicRightsMetadataInImage(
        sourceBytes: _minimalPng(),
        metadata: {..._metadata(), 'legalConclusion': true},
      ),
      throwsArgumentError,
    );
  });
}

Map<String, Object?> _metadata() => {
  'watermarkUid': 'HS-TEST-UID',
  'manifestHash': 'sha256:manifest-test',
  'legalConclusion': false,
  'xmp': {'xmpRights:Marked': true},
  'iptc': {'plus:DataMining': 'http://ns.useplus.org/ldf/vocab/DMI-PLAN'},
  'jsonLd': {'hs:trainingPolicy': 'separate_license_required'},
  'c2paAssertions': [
    {'label': 'cawg.training-mining'},
  ],
};

Uint8List _minimalPng() {
  final bytes = BytesBuilder(copy: false)
    ..add([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
    ..add(_chunk('IHDR', [0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]))
    ..add(_chunk('IDAT', [0x78, 0x9C, 0x63, 0, 0, 0, 2, 0, 1]))
    ..add(_chunk('IEND', const []));
  return bytes.toBytes();
}

List<int> _chunk(String kind, List<int> data) {
  final kindBytes = ascii.encode(kind);
  final bytes = BytesBuilder(copy: false)
    ..add(_uint32be(data.length))
    ..add(kindBytes)
    ..add(data)
    ..add(_uint32be(_crc32([...kindBytes, ...data])));
  return bytes.toBytes();
}

List<int> _uint32be(int value) {
  final data = ByteData(4)..setUint32(0, value, Endian.big);
  return data.buffer.asUint8List();
}

int _crc32(List<int> bytes) {
  var crc = 0xFFFFFFFF;
  for (final byte in bytes) {
    crc ^= byte;
    for (var i = 0; i < 8; i++) {
      final mask = -(crc & 1);
      crc = (crc >> 1) ^ (0xEDB88320 & mask);
    }
  }
  return (crc ^ 0xFFFFFFFF) & 0xFFFFFFFF;
}
