import 'dart:math' as math;
import 'dart:typed_data';

const String trustedVideoMetadataProbe = 'trusted_video_metadata_probe_v1';

class VideoMetadata {
  const VideoMetadata({
    required this.container,
    required this.durationSeconds,
    required this.width,
    required this.height,
    required this.frameCount,
    required this.frameRate,
    this.probeSchema = trustedVideoMetadataProbe,
  });

  final String container;
  final double durationSeconds;
  final int width;
  final int height;
  final int frameCount;
  final double frameRate;
  final String probeSchema;

  bool get hasTrustedProbe => probeSchema == trustedVideoMetadataProbe;
}

VideoMetadata? inspectVideoMetadata(Uint8List bytes, {String? fileName}) {
  if (!_looksLikeIsoBmff(bytes, fileName)) {
    return null;
  }
  final reader = _Mp4Reader(bytes);
  final boxes = reader.children(0, bytes.length);
  final ftyp = boxes.where((box) => box.type == 'ftyp').firstOrNull;
  final moov = boxes.where((box) => box.type == 'moov').firstOrNull;
  if (ftyp == null || moov == null) {
    return null;
  }

  double? movieDuration;
  int? width;
  int? height;
  int? frameCount;
  double? trackDuration;

  for (final child in reader.children(moov.payloadStart, moov.end)) {
    if (child.type == 'mvhd') {
      movieDuration = reader.readMovieHeaderDuration(child);
    } else if (child.type == 'trak') {
      final track = _inspectTrack(reader, child);
      width ??= track.width;
      height ??= track.height;
      frameCount ??= track.frameCount;
      trackDuration ??= track.durationSeconds;
    }
  }

  final durationSeconds = trackDuration ?? movieDuration;
  if (durationSeconds == null ||
      durationSeconds <= 0 ||
      width == null ||
      height == null ||
      width <= 0 ||
      height <= 0 ||
      frameCount == null ||
      frameCount <= 0) {
    return null;
  }

  return VideoMetadata(
    container: _containerFor(ftyp, bytes, fileName),
    durationSeconds: durationSeconds,
    width: width,
    height: height,
    frameCount: frameCount,
    frameRate: frameCount / durationSeconds,
  );
}

_TrackMetadata _inspectTrack(_Mp4Reader reader, _Mp4Box trackBox) {
  int? width;
  int? height;
  int? frameCount;
  double? durationSeconds;

  for (final child in reader.children(trackBox.payloadStart, trackBox.end)) {
    if (child.type == 'tkhd') {
      final size = reader.readTrackHeaderSize(child);
      width = size?.$1;
      height = size?.$2;
    } else if (child.type == 'mdia') {
      for (final mediaChild in reader.children(child.payloadStart, child.end)) {
        if (mediaChild.type == 'mdhd') {
          durationSeconds = reader.readMediaHeaderDuration(mediaChild);
        } else if (mediaChild.type == 'minf') {
          frameCount = _inspectMinfFrameCount(reader, mediaChild);
        }
      }
    }
  }

  return _TrackMetadata(
    width: width,
    height: height,
    frameCount: frameCount,
    durationSeconds: durationSeconds,
  );
}

int? _inspectMinfFrameCount(_Mp4Reader reader, _Mp4Box minfBox) {
  for (final child in reader.children(minfBox.payloadStart, minfBox.end)) {
    if (child.type != 'stbl') {
      continue;
    }
    int? sttsFrameCount;
    int? stszFrameCount;
    for (final sampleChild in reader.children(child.payloadStart, child.end)) {
      if (sampleChild.type == 'stts') {
        sttsFrameCount = reader.readTimeToSampleCount(sampleChild);
      } else if (sampleChild.type == 'stsz') {
        stszFrameCount = reader.readSampleSizeCount(sampleChild);
      }
    }
    return stszFrameCount ?? sttsFrameCount;
  }
  return null;
}

bool _looksLikeIsoBmff(Uint8List bytes, String? fileName) {
  if (bytes.length >= 12 && _ascii(bytes, 4, 8) == 'ftyp') {
    return true;
  }
  final lower = fileName?.toLowerCase();
  return lower?.endsWith('.mp4') == true || lower?.endsWith('.mov') == true;
}

String _containerFor(_Mp4Box ftyp, Uint8List bytes, String? fileName) {
  final brand = ftyp.payloadStart + 4 <= ftyp.end
      ? _ascii(bytes, ftyp.payloadStart, ftyp.payloadStart + 4).trim()
      : '';
  final lower = fileName?.toLowerCase();
  if (lower?.endsWith('.mov') == true || brand == 'qt') {
    return 'mov';
  }
  return 'mp4';
}

class _TrackMetadata {
  const _TrackMetadata({
    required this.width,
    required this.height,
    required this.frameCount,
    required this.durationSeconds,
  });

  final int? width;
  final int? height;
  final int? frameCount;
  final double? durationSeconds;
}

class _Mp4Box {
  const _Mp4Box({
    required this.type,
    required this.start,
    required this.payloadStart,
    required this.end,
  });

  final String type;
  final int start;
  final int payloadStart;
  final int end;
}

class _Mp4Reader {
  const _Mp4Reader(this.bytes);

  final Uint8List bytes;

  List<_Mp4Box> children(int start, int end) {
    final result = <_Mp4Box>[];
    var offset = math.max(0, start);
    final limit = math.min(end, bytes.length);
    while (offset + 8 <= limit) {
      final size32 = _uint32(offset);
      final type = _ascii(bytes, offset + 4, offset + 8);
      var headerSize = 8;
      int boxEnd;
      if (size32 == 1) {
        if (offset + 16 > limit) break;
        headerSize = 16;
        final largeSize = _uint64(offset + 8);
        if (largeSize > BigInt.from(0x7fffffff)) break;
        boxEnd = offset + largeSize.toInt();
      } else if (size32 == 0) {
        boxEnd = limit;
      } else {
        boxEnd = offset + size32;
      }
      if (boxEnd <= offset + headerSize || boxEnd > limit) {
        break;
      }
      result.add(
        _Mp4Box(
          type: type,
          start: offset,
          payloadStart: offset + headerSize,
          end: boxEnd,
        ),
      );
      offset = boxEnd + (boxEnd.isOdd ? 1 : 0);
    }
    return result;
  }

  double? readMovieHeaderDuration(_Mp4Box box) {
    if (box.payloadStart + 20 > box.end) return null;
    final version = bytes[box.payloadStart];
    if (version == 1) {
      if (box.payloadStart + 32 > box.end) return null;
      final timescale = _uint32(box.payloadStart + 20);
      final duration = _uint64(box.payloadStart + 24).toDouble();
      return timescale <= 0 ? null : duration / timescale;
    }
    final timescale = _uint32(box.payloadStart + 12);
    final duration = _uint32(box.payloadStart + 16);
    return timescale <= 0 ? null : duration / timescale;
  }

  double? readMediaHeaderDuration(_Mp4Box box) {
    if (box.payloadStart + 20 > box.end) return null;
    final version = bytes[box.payloadStart];
    if (version == 1) {
      if (box.payloadStart + 32 > box.end) return null;
      final timescale = _uint32(box.payloadStart + 20);
      final duration = _uint64(box.payloadStart + 24).toDouble();
      return timescale <= 0 ? null : duration / timescale;
    }
    final timescale = _uint32(box.payloadStart + 12);
    final duration = _uint32(box.payloadStart + 16);
    return timescale <= 0 ? null : duration / timescale;
  }

  (int, int)? readTrackHeaderSize(_Mp4Box box) {
    if (box.payloadStart + 4 > box.end) return null;
    final version = bytes[box.payloadStart];
    final widthOffset = box.payloadStart + (version == 1 ? 88 : 76);
    final heightOffset = widthOffset + 4;
    if (heightOffset + 4 > box.end) return null;
    return (_fixed16_16(widthOffset), _fixed16_16(heightOffset));
  }

  int? readTimeToSampleCount(_Mp4Box box) {
    if (box.payloadStart + 8 > box.end) return null;
    final entryCount = _uint32(box.payloadStart + 4);
    var offset = box.payloadStart + 8;
    var samples = 0;
    for (var i = 0; i < entryCount; i++) {
      if (offset + 8 > box.end) return null;
      samples += _uint32(offset);
      offset += 8;
    }
    return samples;
  }

  int? readSampleSizeCount(_Mp4Box box) {
    if (box.payloadStart + 12 > box.end) return null;
    return _uint32(box.payloadStart + 8);
  }

  int _fixed16_16(int offset) => _uint32(offset) >> 16;

  int _uint32(int offset) =>
      ByteData.sublistView(bytes).getUint32(offset, Endian.big);

  BigInt _uint64(int offset) =>
      (BigInt.from(_uint32(offset)) << 32) | BigInt.from(_uint32(offset + 4));
}

String _ascii(Uint8List bytes, int start, int end) {
  if (start < 0 || end > bytes.length || end < start) {
    return '';
  }
  return String.fromCharCodes(bytes.sublist(start, end));
}
