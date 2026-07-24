import 'dart:convert';
import 'dart:typed_data';

const publicRightsEmbeddedImageExportLabel = '导出嵌入元数据图片副本';
const publicRightsEmbeddedImageExportRequiresFileMessage =
    '移动端可生成 PNG / JPEG 嵌入元数据副本，但需要当前保护副本文件字节；历史版权库记录仅保存名称和摘要，需先重新选择保护副本。';

const _pngSignature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const _pngItxtKeyword = 'XML:com.adobe.xmp';
const _jpegXmpNamespace = 'http://ns.adobe.com/xap/1.0/\u0000';
const _embedBoundary =
    'creator_declaration_registry_snapshot_not_legal_advice_public_metadata_copy';

enum PublicMetadataImageFormat { png, jpeg }

class PublicMetadataEmbeddedImage {
  const PublicMetadataEmbeddedImage({
    required this.bytes,
    required this.format,
    required this.embeddedStandards,
    required this.legalConclusion,
    required this.boundary,
  });

  final Uint8List bytes;
  final PublicMetadataImageFormat format;
  final List<String> embeddedStandards;
  final bool legalConclusion;
  final String boundary;
}

class PublicMetadataByteCheck {
  const PublicMetadataByteCheck({
    required this.hasContainer,
    required this.hasNamespace,
    required this.hasWatermarkUid,
    required this.hasManifestHash,
    required this.hasLegalConclusionFalse,
  });

  final bool hasContainer;
  final bool hasNamespace;
  final bool hasWatermarkUid;
  final bool hasManifestHash;
  final bool hasLegalConclusionFalse;

  bool get pass =>
      hasContainer &&
      hasNamespace &&
      hasWatermarkUid &&
      hasManifestHash &&
      hasLegalConclusionFalse;

  Map<String, Object?> toJson() => {
    'hasContainer': hasContainer,
    'hasNamespace': hasNamespace,
    'hasWatermarkUid': hasWatermarkUid,
    'hasManifestHash': hasManifestHash,
    'hasLegalConclusionFalse': hasLegalConclusionFalse,
    'pass': pass,
  };
}

PublicMetadataEmbeddedImage embedPublicRightsMetadataInImage({
  required Uint8List sourceBytes,
  required Map<String, Object?> metadata,
  PublicMetadataImageFormat? format,
}) {
  if (metadata['legalConclusion'] == true) {
    throw ArgumentError('公开元数据不能声明 legalConclusion=true');
  }
  final detectedFormat = format ?? detectPublicMetadataImageFormat(sourceBytes);
  final xmpPacket = buildPublicRightsXmpPacket(metadata);
  final bytes = switch (detectedFormat) {
    PublicMetadataImageFormat.png => _embedPngXmp(sourceBytes, xmpPacket),
    PublicMetadataImageFormat.jpeg => _embedJpegXmp(sourceBytes, xmpPacket),
  };
  return PublicMetadataEmbeddedImage(
    bytes: bytes,
    format: detectedFormat,
    embeddedStandards: const [
      'XMP',
      'IPTC/PLUS JSON-LD mapping',
      'C2PA/CAWG JSON-LD mapping',
    ],
    legalConclusion: false,
    boundary: _embedBoundary,
  );
}

PublicMetadataImageFormat detectPublicMetadataImageFormat(Uint8List bytes) {
  if (_startsWith(bytes, _pngSignature)) {
    return PublicMetadataImageFormat.png;
  }
  if (bytes.length >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8) {
    return PublicMetadataImageFormat.jpeg;
  }
  throw ArgumentError('暂仅支持 PNG / JPEG 图片嵌入公开元数据。');
}

Uint8List buildPublicRightsXmpPacket(Map<String, Object?> metadata) {
  final jsonText = jsonEncode(metadata);
  final xmp = metadata['xmp'] ?? const <String, Object?>{};
  final iptc = metadata['iptc'] ?? const <String, Object?>{};
  final jsonLd =
      metadata['jsonLd'] ?? metadata['json_ld'] ?? const <String, Object?>{};
  final c2paAssertions =
      metadata['c2paAssertions'] ??
      metadata['c2pa_assertions'] ??
      const <Object?>[];
  final trainingPolicy = jsonLd is Map
      ? (jsonLd['hs:trainingPolicy']?.toString() ?? '')
      : '';
  final packet =
      '''<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hs="https://hiddenshield.local/ns#" xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      hs:boundary="${_xmlEscape(_embedBoundary)}"
      hs:watermarkUid="${_xmlEscape(metadata['watermarkUid']?.toString() ?? '')}"
      hs:manifestHash="${_xmlEscape(metadata['manifestHash']?.toString() ?? '')}"
      hs:trainingPolicy="${_xmlEscape(trainingPolicy)}"
      hs:legalConclusion="false">
      <hs:xmp>${_xmlEscape(jsonEncode(xmp))}</hs:xmp>
      <hs:iptcPlus>${_xmlEscape(jsonEncode(iptc))}</hs:iptcPlus>
      <hs:c2paCawg>${_xmlEscape(jsonEncode(c2paAssertions))}</hs:c2paCawg>
      <hs:jsonLd>${_xmlEscape(jsonEncode(jsonLd))}</hs:jsonLd>
      <hs:metadataExport>${_xmlEscape(jsonText)}</hs:metadataExport>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>''';
  return Uint8List.fromList(utf8.encode(packet));
}

PublicMetadataByteCheck checkEmbeddedPublicMetadataBytes({
  required Uint8List bytes,
  required PublicMetadataImageFormat format,
  required String watermarkUid,
  required String manifestHash,
}) {
  final text = latin1.decode(bytes, allowInvalid: true);
  final hasContainer = switch (format) {
    PublicMetadataImageFormat.png => text.contains('iTXt'),
    PublicMetadataImageFormat.jpeg => bytes.asMap().entries.any(
      (entry) =>
          entry.value == 0xFF &&
          bytes.length > entry.key + 1 &&
          bytes[entry.key + 1] == 0xE1,
    ),
  };
  final hasNamespace = switch (format) {
    PublicMetadataImageFormat.png => text.contains(_pngItxtKeyword),
    PublicMetadataImageFormat.jpeg => text.contains(
      'http://ns.adobe.com/xap/1.0/',
    ),
  };
  return PublicMetadataByteCheck(
    hasContainer: hasContainer,
    hasNamespace: hasNamespace,
    hasWatermarkUid: text.contains(watermarkUid),
    hasManifestHash: text.contains(manifestHash),
    hasLegalConclusionFalse:
        text.contains('legalConclusion="false"') ||
        text.contains('&quot;legalConclusion&quot;:false'),
  );
}

Uint8List _embedPngXmp(Uint8List bytes, Uint8List xmpPacket) {
  if (!_startsWith(bytes, _pngSignature)) {
    throw ArgumentError('PNG 文件头无效');
  }
  var offset = _pngSignature.length;
  final output = BytesBuilder(copy: false)..add(_pngSignature);
  var inserted = false;
  while (offset + 12 <= bytes.length) {
    final length = ByteData.sublistView(
      bytes,
      offset,
      offset + 4,
    ).getUint32(0, Endian.big);
    final chunkTypeStart = offset + 4;
    final dataStart = offset + 8;
    final chunkEnd = dataStart + length + 4;
    if (chunkEnd > bytes.length) {
      throw ArgumentError('PNG chunk 超出文件边界');
    }
    final chunkType = bytes.sublist(chunkTypeStart, chunkTypeStart + 4);
    if (!inserted && _bytesEqual(chunkType, ascii.encode('IDAT'))) {
      output.add(_pngItxtChunk(xmpPacket));
      inserted = true;
    }
    output.add(bytes.sublist(offset, chunkEnd));
    offset = chunkEnd;
    if (_bytesEqual(chunkType, ascii.encode('IEND'))) {
      break;
    }
  }
  if (!inserted) {
    throw ArgumentError('未找到 PNG IDAT chunk，无法嵌入 XMP。');
  }
  return output.toBytes();
}

Uint8List _pngItxtChunk(Uint8List xmpPacket) {
  final data = BytesBuilder(copy: false)
    ..add(ascii.encode(_pngItxtKeyword))
    ..addByte(0)
    ..addByte(0)
    ..addByte(0)
    ..addByte(0)
    ..addByte(0)
    ..add(xmpPacket);
  final dataBytes = data.toBytes();
  final chunk = BytesBuilder(copy: false)
    ..add(_uint32be(dataBytes.length))
    ..add(ascii.encode('iTXt'))
    ..add(dataBytes)
    ..add(_uint32be(_crc32([...ascii.encode('iTXt'), ...dataBytes])));
  return chunk.toBytes();
}

Uint8List _embedJpegXmp(Uint8List bytes, Uint8List xmpPacket) {
  if (bytes.length < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8) {
    throw ArgumentError('JPEG 文件头无效');
  }
  final namespace = latin1.encode(_jpegXmpNamespace);
  final segmentLength = namespace.length + xmpPacket.length + 2;
  if (segmentLength > 0xFFFF) {
    throw ArgumentError('XMP packet 超出 JPEG APP1 segment 限制');
  }
  final output = BytesBuilder(copy: false)
    ..add(bytes.sublist(0, 2))
    ..add([0xFF, 0xE1])
    ..add(_uint16be(segmentLength))
    ..add(namespace)
    ..add(xmpPacket)
    ..add(bytes.sublist(2));
  return output.toBytes();
}

Uint8List _uint16be(int value) {
  final data = ByteData(2)..setUint16(0, value, Endian.big);
  return data.buffer.asUint8List();
}

Uint8List _uint32be(int value) {
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

bool _startsWith(Uint8List bytes, List<int> prefix) {
  if (bytes.length < prefix.length) return false;
  for (var i = 0; i < prefix.length; i++) {
    if (bytes[i] != prefix[i]) return false;
  }
  return true;
}

bool _bytesEqual(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var i = 0; i < left.length; i++) {
    if (left[i] != right[i]) return false;
  }
  return true;
}

String _xmlEscape(String value) => value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&apos;');
