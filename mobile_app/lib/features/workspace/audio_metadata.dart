import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';

const int minimumAudioProtectionSeconds = 30;
const int minimumSupportedAudioSampleRate = 8000;
const int maximumSupportedAudioSampleRate = 48000;
const int minimumSupportedAudioChannels = 1;
const int maximumSupportedAudioChannels = 2;

enum AudioProtectionPreflightCode {
  ok,
  durationUnknown,
  tooShort,
  specUnknown,
  sampleRateTooLow,
  sampleRateTooHigh,
  channelsUnsupported,
}

class AudioMetadata {
  const AudioMetadata({
    required this.durationSeconds,
    this.sampleRate,
    this.channels,
    this.container,
  });

  final double? durationSeconds;
  final int? sampleRate;
  final int? channels;
  final String? container;

  bool get hasConfirmedDuration => durationSeconds != null;
}

AudioProtectionPreflightCode audioProtectionPreflight(AudioMetadata? metadata) {
  if (metadata == null || metadata.durationSeconds == null) {
    return AudioProtectionPreflightCode.durationUnknown;
  }
  if (metadata.durationSeconds! < minimumAudioProtectionSeconds) {
    return AudioProtectionPreflightCode.tooShort;
  }
  final sampleRate = metadata.sampleRate;
  final channels = metadata.channels;
  if (sampleRate == null || channels == null) {
    return AudioProtectionPreflightCode.specUnknown;
  }
  if (sampleRate < minimumSupportedAudioSampleRate) {
    return AudioProtectionPreflightCode.sampleRateTooLow;
  }
  if (sampleRate > maximumSupportedAudioSampleRate) {
    return AudioProtectionPreflightCode.sampleRateTooHigh;
  }
  if (channels < minimumSupportedAudioChannels ||
      channels > maximumSupportedAudioChannels) {
    return AudioProtectionPreflightCode.channelsUnsupported;
  }
  return AudioProtectionPreflightCode.ok;
}

AudioMetadata inspectAudioMetadata(Uint8List bytes, {String? fileName}) {
  if (_isWav(bytes)) {
    return _inspectWav(bytes);
  }
  if (_isFlac(bytes)) {
    return _inspectFlac(bytes);
  }
  if (_isOgg(bytes)) {
    return _inspectOgg(bytes);
  }
  if (_isMp4(bytes) || _looksLikeMp4Name(fileName)) {
    return _inspectMp4(bytes);
  }
  if (_isMp3(bytes) || _looksLikeMp3Name(fileName)) {
    return _inspectMp3(bytes);
  }
  if (_looksLikeAacName(fileName)) {
    return _inspectAdtsAac(bytes);
  }
  return const AudioMetadata(durationSeconds: null);
}

bool _isWav(Uint8List bytes) =>
    bytes.length >= 12 &&
    _ascii(bytes, 0, 4) == 'RIFF' &&
    _ascii(bytes, 8, 12) == 'WAVE';

bool _isFlac(Uint8List bytes) =>
    bytes.length >= 4 && _ascii(bytes, 0, 4) == 'fLaC';

bool _isOgg(Uint8List bytes) =>
    bytes.length >= 4 && _ascii(bytes, 0, 4) == 'OggS';

bool _isMp4(Uint8List bytes) =>
    bytes.length >= 12 && _ascii(bytes, 4, 8) == 'ftyp';

bool _isMp3(Uint8List bytes) {
  final offset = _skipId3(bytes);
  if (offset + 2 >= bytes.length) {
    return false;
  }
  return bytes[offset] == 0xFF && (bytes[offset + 1] & 0xE0) == 0xE0;
}

bool _looksLikeMp3Name(String? fileName) =>
    fileName?.toLowerCase().endsWith('.mp3') == true;

bool _looksLikeAacName(String? fileName) =>
    fileName?.toLowerCase().endsWith('.aac') == true;

bool _looksLikeMp4Name(String? fileName) {
  final lower = fileName?.toLowerCase();
  return lower?.endsWith('.m4a') == true || lower?.endsWith('.mp4') == true;
}

AudioMetadata _inspectWav(Uint8List bytes) {
  if (bytes.length < 44) {
    return const AudioMetadata(durationSeconds: null, container: 'wav');
  }
  final data = ByteData.sublistView(bytes);
  int? channels;
  int? sampleRate;
  int? bitsPerSample;
  int? dataSize;
  var offset = 12;

  while (offset + 8 <= bytes.length) {
    final chunkId = _ascii(bytes, offset, offset + 4);
    final chunkSize = data.getUint32(offset + 4, Endian.little);
    final chunkDataOffset = offset + 8;
    if (chunkDataOffset + chunkSize > bytes.length) {
      break;
    }

    if (chunkId == 'fmt ' && chunkSize >= 16) {
      channels = data.getUint16(chunkDataOffset + 2, Endian.little);
      sampleRate = data.getUint32(chunkDataOffset + 4, Endian.little);
      bitsPerSample = data.getUint16(chunkDataOffset + 14, Endian.little);
    } else if (chunkId == 'data') {
      dataSize = chunkSize;
      break;
    }

    offset = chunkDataOffset + chunkSize + (chunkSize.isOdd ? 1 : 0);
  }

  if (channels == null ||
      sampleRate == null ||
      bitsPerSample == null ||
      dataSize == null ||
      channels <= 0 ||
      sampleRate <= 0 ||
      bitsPerSample <= 0) {
    return AudioMetadata(
      durationSeconds: null,
      sampleRate: sampleRate,
      channels: channels,
      container: 'wav',
    );
  }

  final bytesPerSecond = sampleRate * channels * (bitsPerSample / 8);
  return AudioMetadata(
    durationSeconds: bytesPerSecond <= 0 ? null : dataSize / bytesPerSecond,
    sampleRate: sampleRate,
    channels: channels,
    container: 'wav',
  );
}

AudioMetadata _inspectFlac(Uint8List bytes) {
  var offset = 4;
  while (offset + 4 <= bytes.length) {
    final header = bytes[offset];
    final blockType = header & 0x7F;
    final length =
        (bytes[offset + 1] << 16) |
        (bytes[offset + 2] << 8) |
        bytes[offset + 3];
    final dataOffset = offset + 4;
    if (dataOffset + length > bytes.length) {
      break;
    }
    if (blockType == 0 && length >= 18) {
      final b = bytes;
      final sampleRate =
          (b[dataOffset + 10] << 12) |
          (b[dataOffset + 11] << 4) |
          ((b[dataOffset + 12] & 0xF0) >> 4);
      final channels = ((b[dataOffset + 12] & 0x0E) >> 1) + 1;
      final totalSamples =
          (BigInt.from(b[dataOffset + 13] & 0x0F) << 32) |
          (BigInt.from(b[dataOffset + 14]) << 24) |
          (BigInt.from(b[dataOffset + 15]) << 16) |
          (BigInt.from(b[dataOffset + 16]) << 8) |
          BigInt.from(b[dataOffset + 17]);
      return AudioMetadata(
        durationSeconds: sampleRate <= 0
            ? null
            : totalSamples.toDouble() / sampleRate,
        sampleRate: sampleRate,
        channels: channels,
        container: 'flac',
      );
    }
    offset = dataOffset + length;
  }
  return const AudioMetadata(durationSeconds: null, container: 'flac');
}

AudioMetadata _inspectOgg(Uint8List bytes) {
  int? sampleRate;
  int? channels;
  BigInt? lastGranule;
  var offset = 0;
  while (offset + 27 <= bytes.length) {
    if (_ascii(bytes, offset, offset + 4) != 'OggS') {
      break;
    }
    final pageSegments = bytes[offset + 26];
    final segmentTableOffset = offset + 27;
    if (segmentTableOffset + pageSegments > bytes.length) {
      break;
    }
    var payloadLength = 0;
    for (var i = 0; i < pageSegments; i++) {
      payloadLength += bytes[segmentTableOffset + i];
    }
    final payloadOffset = segmentTableOffset + pageSegments;
    if (payloadOffset + payloadLength > bytes.length) {
      break;
    }

    final granule = _readUint64Le(bytes, offset + 6);
    if (granule >= BigInt.zero) {
      lastGranule = granule;
    }

    if (sampleRate == null && payloadLength >= 16) {
      if (bytes[payloadOffset] == 1 &&
          _ascii(bytes, payloadOffset + 1, payloadOffset + 7) == 'vorbis') {
        channels = bytes[payloadOffset + 11];
        sampleRate = ByteData.sublistView(
          bytes,
        ).getUint32(payloadOffset + 12, Endian.little);
      } else if (_ascii(bytes, payloadOffset, payloadOffset + 8) ==
          'OpusHead') {
        channels = bytes[payloadOffset + 9];
        sampleRate = 48000;
      }
    }
    offset = payloadOffset + payloadLength;
  }
  return AudioMetadata(
    durationSeconds:
        sampleRate == null || sampleRate <= 0 || lastGranule == null
        ? null
        : lastGranule.toDouble() / sampleRate,
    sampleRate: sampleRate,
    channels: channels,
    container: 'ogg',
  );
}

AudioMetadata _inspectMp4(Uint8List bytes) {
  final state = _Mp4InspectState();
  _walkMp4Atoms(bytes, 0, bytes.length, state);
  return AudioMetadata(
    durationSeconds: state.durationSeconds,
    sampleRate: state.sampleRate,
    channels: state.channels,
    container: 'm4a',
  );
}

void _walkMp4Atoms(
  Uint8List bytes,
  int start,
  int end,
  _Mp4InspectState state,
) {
  var offset = start;
  final data = ByteData.sublistView(bytes);
  while (offset + 8 <= end && offset + 8 <= bytes.length) {
    var size = data.getUint32(offset, Endian.big);
    final type = _ascii(bytes, offset + 4, offset + 8);
    var header = 8;
    if (size == 1 && offset + 16 <= bytes.length) {
      final largeSize = data.getUint64(offset + 8, Endian.big);
      if (largeSize > 0x7fffffff) {
        break;
      }
      size = largeSize;
      header = 16;
    } else if (size == 0) {
      size = end - offset;
    }
    if (size < header || offset + size > end || offset + size > bytes.length) {
      break;
    }
    final payloadStart = offset + header;
    final payloadEnd = offset + size;

    if (type == 'mvhd') {
      _readMvhd(bytes, payloadStart, payloadEnd, state);
    } else if (type == 'mdhd') {
      _readMdhd(bytes, payloadStart, payloadEnd, state);
    } else if (type == 'mp4a') {
      _readMp4a(bytes, payloadStart, payloadEnd, state);
    }

    if (_mp4ContainerTypes.contains(type)) {
      _walkMp4Atoms(bytes, payloadStart, payloadEnd, state);
    }
    offset += size;
  }
}

const Set<String> _mp4ContainerTypes = {
  'moov',
  'trak',
  'mdia',
  'minf',
  'stbl',
  'stsd',
  'edts',
};

void _readMvhd(Uint8List bytes, int start, int end, _Mp4InspectState state) {
  if (start + 20 > end) {
    return;
  }
  final data = ByteData.sublistView(bytes);
  final version = bytes[start];
  if (version == 1) {
    if (start + 32 > end) return;
    final timescale = data.getUint32(start + 20, Endian.big);
    final duration = data.getUint64(start + 24, Endian.big);
    state.durationSeconds ??= timescale <= 0
        ? null
        : duration.toDouble() / timescale;
  } else {
    final timescale = data.getUint32(start + 12, Endian.big);
    final duration = data.getUint32(start + 16, Endian.big);
    state.durationSeconds ??= timescale <= 0
        ? null
        : duration.toDouble() / timescale;
  }
}

void _readMdhd(Uint8List bytes, int start, int end, _Mp4InspectState state) {
  if (start + 20 > end) {
    return;
  }
  final data = ByteData.sublistView(bytes);
  final version = bytes[start];
  if (version == 1) {
    if (start + 32 > end) return;
    final timescale = data.getUint32(start + 20, Endian.big);
    final duration = data.getUint64(start + 24, Endian.big);
    state.durationSeconds ??= timescale <= 0
        ? null
        : duration.toDouble() / timescale;
    state.sampleRate ??= timescale > 8000 ? timescale : null;
  } else {
    final timescale = data.getUint32(start + 12, Endian.big);
    final duration = data.getUint32(start + 16, Endian.big);
    state.durationSeconds ??= timescale <= 0
        ? null
        : duration.toDouble() / timescale;
    state.sampleRate ??= timescale > 8000 ? timescale : null;
  }
}

void _readMp4a(Uint8List bytes, int start, int end, _Mp4InspectState state) {
  if (start + 28 > end) {
    return;
  }
  final data = ByteData.sublistView(bytes);
  state.channels ??= data.getUint16(start + 16, Endian.big);
  final fixedSampleRate = data.getUint32(start + 24, Endian.big);
  if (fixedSampleRate > 0) {
    state.sampleRate ??= fixedSampleRate >> 16;
  }
}

AudioMetadata _inspectMp3(Uint8List bytes) {
  final offset = _skipId3(bytes);
  final data = ByteData.sublistView(bytes);
  for (var i = offset; i + 4 <= bytes.length; i++) {
    final header = data.getUint32(i, Endian.big);
    final frame = _parseMp3FrameHeader(header);
    if (frame == null) {
      continue;
    }
    final xingOffset = i + frame.xingOffset;
    final frames =
        _readXingFrameCount(bytes, xingOffset) ??
        _readVbriFrameCount(bytes, i + 36);
    if (frames != null) {
      return AudioMetadata(
        durationSeconds: frames * frame.samplesPerFrame / frame.sampleRate,
        sampleRate: frame.sampleRate,
        channels: frame.channels,
        container: 'mp3',
      );
    }
    final estimated = _estimateMp3DurationFromFrames(bytes, i, frame);
    return AudioMetadata(
      durationSeconds: estimated,
      sampleRate: frame.sampleRate,
      channels: frame.channels,
      container: 'mp3',
    );
  }
  return const AudioMetadata(durationSeconds: null, container: 'mp3');
}

AudioMetadata _inspectAdtsAac(Uint8List bytes) {
  for (var i = 0; i + 7 <= bytes.length; i++) {
    if (bytes[i] != 0xFF || (bytes[i + 1] & 0xF0) != 0xF0) {
      continue;
    }
    final sampleRateIndex = (bytes[i + 2] & 0x3C) >> 2;
    final sampleRate = _aacSampleRates[sampleRateIndex];
    final channels =
        ((bytes[i + 2] & 0x01) << 2) | ((bytes[i + 3] & 0xC0) >> 6);
    if (sampleRate == null || sampleRate <= 0) {
      return const AudioMetadata(durationSeconds: null, container: 'aac');
    }
    var offset = i;
    var frames = 0;
    while (offset + 7 <= bytes.length &&
        bytes[offset] == 0xFF &&
        (bytes[offset + 1] & 0xF0) == 0xF0) {
      final frameLength =
          ((bytes[offset + 3] & 0x03) << 11) |
          (bytes[offset + 4] << 3) |
          ((bytes[offset + 5] & 0xE0) >> 5);
      if (frameLength <= 0 || offset + frameLength > bytes.length) {
        break;
      }
      frames++;
      offset += frameLength;
    }
    return AudioMetadata(
      durationSeconds: frames <= 0 ? null : frames * 1024 / sampleRate,
      sampleRate: sampleRate,
      channels: channels == 0 ? null : channels,
      container: 'aac',
    );
  }
  return const AudioMetadata(durationSeconds: null, container: 'aac');
}

int _skipId3(Uint8List bytes) {
  if (bytes.length < 10 || _ascii(bytes, 0, 3) != 'ID3') {
    return 0;
  }
  final size =
      ((bytes[6] & 0x7F) << 21) |
      ((bytes[7] & 0x7F) << 14) |
      ((bytes[8] & 0x7F) << 7) |
      (bytes[9] & 0x7F);
  return math.min(bytes.length, 10 + size);
}

_Mp3Frame? _parseMp3FrameHeader(int header) {
  if ((header & 0xFFE00000) != 0xFFE00000) {
    return null;
  }
  final versionBits = (header >> 19) & 0x03;
  final layerBits = (header >> 17) & 0x03;
  final bitrateIndex = (header >> 12) & 0x0F;
  final sampleRateIndex = (header >> 10) & 0x03;
  final padding = (header >> 9) & 0x01;
  final channelMode = (header >> 6) & 0x03;
  if (versionBits == 1 ||
      layerBits == 0 ||
      bitrateIndex == 0 ||
      bitrateIndex == 0x0F ||
      sampleRateIndex == 0x03) {
    return null;
  }
  final version = switch (versionBits) {
    3 => 1,
    2 => 2,
    _ => 25,
  };
  final layer = 4 - layerBits;
  final sampleRate = _mp3SampleRate(version, sampleRateIndex);
  final bitrate = _mp3Bitrate(version, layer, bitrateIndex);
  if (sampleRate <= 0 || bitrate <= 0) {
    return null;
  }
  final samplesPerFrame = layer == 1
      ? 384
      : (layer == 3 && version != 1)
      ? 576
      : 1152;
  final frameLength = layer == 1
      ? (((12 * bitrate * 1000) ~/ sampleRate) + padding) * 4
      : ((samplesPerFrame ~/ 8 * bitrate * 1000) ~/ sampleRate) + padding;
  if (frameLength <= 0) {
    return null;
  }
  return _Mp3Frame(
    sampleRate: sampleRate,
    bitrateKbps: bitrate,
    channels: channelMode == 3 ? 1 : 2,
    samplesPerFrame: samplesPerFrame,
    frameLength: frameLength,
    xingOffset: version == 1
        ? (channelMode == 3 ? 21 : 36)
        : (channelMode == 3 ? 13 : 21),
  );
}

int? _readXingFrameCount(Uint8List bytes, int offset) {
  if (offset + 16 > bytes.length) {
    return null;
  }
  final tag = _ascii(bytes, offset, offset + 4);
  if (tag != 'Xing' && tag != 'Info') {
    return null;
  }
  final data = ByteData.sublistView(bytes);
  final flags = data.getUint32(offset + 4, Endian.big);
  if ((flags & 0x01) == 0) {
    return null;
  }
  return data.getUint32(offset + 8, Endian.big);
}

int? _readVbriFrameCount(Uint8List bytes, int offset) {
  if (offset + 18 > bytes.length ||
      _ascii(bytes, offset, offset + 4) != 'VBRI') {
    return null;
  }
  return ByteData.sublistView(bytes).getUint32(offset + 14, Endian.big);
}

double? _estimateMp3DurationFromFrames(
  Uint8List bytes,
  int firstFrameOffset,
  _Mp3Frame firstFrame,
) {
  var offset = firstFrameOffset;
  var frames = 0;
  while (offset + 4 <= bytes.length) {
    final frame = _parseMp3FrameHeader(
      ByteData.sublistView(bytes).getUint32(offset, Endian.big),
    );
    if (frame == null) {
      break;
    }
    frames++;
    offset += frame.frameLength;
  }
  if (frames > 1) {
    return frames * firstFrame.samplesPerFrame / firstFrame.sampleRate;
  }
  final audioBytes = bytes.length - firstFrameOffset;
  return (audioBytes * 8) / (firstFrame.bitrateKbps * 1000);
}

int _mp3SampleRate(int version, int index) {
  const mpeg1 = [44100, 48000, 32000];
  final base = mpeg1[index];
  return switch (version) {
    1 => base,
    2 => base ~/ 2,
    _ => base ~/ 4,
  };
}

int _mp3Bitrate(int version, int layer, int index) {
  const mpeg1Layer1 = [
    0,
    32,
    64,
    96,
    128,
    160,
    192,
    224,
    256,
    288,
    320,
    352,
    384,
    416,
    448,
  ];
  const mpeg1Layer2 = [
    0,
    32,
    48,
    56,
    64,
    80,
    96,
    112,
    128,
    160,
    192,
    224,
    256,
    320,
    384,
  ];
  const mpeg1Layer3 = [
    0,
    32,
    40,
    48,
    56,
    64,
    80,
    96,
    112,
    128,
    160,
    192,
    224,
    256,
    320,
  ];
  const mpeg2Layer1 = [
    0,
    32,
    48,
    56,
    64,
    80,
    96,
    112,
    128,
    144,
    160,
    176,
    192,
    224,
    256,
  ];
  const mpeg2Layers23 = [
    0,
    8,
    16,
    24,
    32,
    40,
    48,
    56,
    64,
    80,
    96,
    112,
    128,
    144,
    160,
  ];
  if (version == 1 && layer == 1) return mpeg1Layer1[index];
  if (version == 1 && layer == 2) return mpeg1Layer2[index];
  if (version == 1) return mpeg1Layer3[index];
  if (layer == 1) return mpeg2Layer1[index];
  return mpeg2Layers23[index];
}

const Map<int, int> _aacSampleRates = {
  0: 96000,
  1: 88200,
  2: 64000,
  3: 48000,
  4: 44100,
  5: 32000,
  6: 24000,
  7: 22050,
  8: 16000,
  9: 12000,
  10: 11025,
  11: 8000,
  12: 7350,
};

String _ascii(Uint8List bytes, int start, int end) {
  if (start < 0 || end > bytes.length || start > end) {
    return '';
  }
  return ascii.decode(bytes.sublist(start, end), allowInvalid: true);
}

BigInt _readUint64Le(Uint8List bytes, int offset) {
  var value = BigInt.zero;
  for (var i = 7; i >= 0; i--) {
    value = (value << 8) | BigInt.from(bytes[offset + i]);
  }
  return value;
}

class _Mp3Frame {
  const _Mp3Frame({
    required this.sampleRate,
    required this.bitrateKbps,
    required this.channels,
    required this.samplesPerFrame,
    required this.frameLength,
    required this.xingOffset,
  });

  final int sampleRate;
  final int bitrateKbps;
  final int channels;
  final int samplesPerFrame;
  final int frameLength;
  final int xingOffset;
}

class _Mp4InspectState {
  double? durationSeconds;
  int? sampleRate;
  int? channels;
}
