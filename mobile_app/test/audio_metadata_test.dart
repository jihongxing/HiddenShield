import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/workspace/audio_metadata.dart';

void main() {
  test('inspectAudioMetadata reads wav duration and channel count', () {
    final bytes = _makeWavBytes(seconds: 31, sampleRate: 44100, channels: 1);

    final metadata = inspectAudioMetadata(bytes, fileName: 'song.wav');

    expect(metadata.durationSeconds, closeTo(31, 0.01));
    expect(metadata.sampleRate, 44100);
    expect(metadata.channels, 1);
    expect(metadata.container, 'wav');
  });

  test('inspectAudioMetadata reads mp3 duration from xing frame count', () {
    final bytes = _makeMp3WithXing(frames: 1200);

    final metadata = inspectAudioMetadata(bytes, fileName: 'song.mp3');

    expect(metadata.durationSeconds, closeTo(31.34, 0.01));
    expect(metadata.sampleRate, 44100);
    expect(metadata.channels, 2);
    expect(metadata.container, 'mp3');
  });

  test('inspectAudioMetadata returns unknown for unrecognized audio bytes', () {
    final metadata = inspectAudioMetadata(Uint8List.fromList([1, 2, 3]));

    expect(metadata.durationSeconds, isNull);
    expect(metadata.hasConfirmedDuration, isFalse);
  });

  test('audio protection preflight keeps the shared specification boundary', () {
    expect(
      audioProtectionPreflight(
        const AudioMetadata(durationSeconds: 30, sampleRate: 8000, channels: 1),
      ),
      AudioProtectionPreflightCode.ok,
    );
    expect(
      audioProtectionPreflight(
        const AudioMetadata(durationSeconds: 30, sampleRate: 48000, channels: 2),
      ),
      AudioProtectionPreflightCode.ok,
    );
    expect(
      audioProtectionPreflight(
        const AudioMetadata(durationSeconds: 30, sampleRate: 7999, channels: 1),
      ),
      AudioProtectionPreflightCode.sampleRateTooLow,
    );
    expect(
      audioProtectionPreflight(
        const AudioMetadata(durationSeconds: 30, sampleRate: 48000, channels: 3),
      ),
      AudioProtectionPreflightCode.channelsUnsupported,
    );
    expect(
      audioProtectionPreflight(
        const AudioMetadata(durationSeconds: 29, sampleRate: 48000, channels: 2),
      ),
      AudioProtectionPreflightCode.tooShort,
    );
  });
}

Uint8List _makeWavBytes({
  required int seconds,
  required int sampleRate,
  required int channels,
}) {
  const bitsPerSample = 16;
  final dataSize = seconds * sampleRate * channels * (bitsPerSample ~/ 8);
  final bytes = Uint8List(44 + dataSize);
  final data = ByteData.sublistView(bytes);
  _writeAscii(bytes, 0, 'RIFF');
  data.setUint32(4, 36 + dataSize, Endian.little);
  _writeAscii(bytes, 8, 'WAVE');
  _writeAscii(bytes, 12, 'fmt ');
  data.setUint32(16, 16, Endian.little);
  data.setUint16(20, 1, Endian.little);
  data.setUint16(22, channels, Endian.little);
  data.setUint32(24, sampleRate, Endian.little);
  data.setUint32(
    28,
    sampleRate * channels * (bitsPerSample ~/ 8),
    Endian.little,
  );
  data.setUint16(32, channels * (bitsPerSample ~/ 8), Endian.little);
  data.setUint16(34, bitsPerSample, Endian.little);
  _writeAscii(bytes, 36, 'data');
  data.setUint32(40, dataSize, Endian.little);
  return bytes;
}

Uint8List _makeMp3WithXing({required int frames}) {
  final bytes = Uint8List(512);
  final data = ByteData.sublistView(bytes);
  data.setUint32(0, 0xFFFB9000, Endian.big);
  _writeAscii(bytes, 36, 'Xing');
  data.setUint32(40, 1, Endian.big);
  data.setUint32(44, frames, Endian.big);
  return bytes;
}

void _writeAscii(Uint8List bytes, int offset, String value) {
  for (var i = 0; i < value.length; i++) {
    bytes[offset + i] = value.codeUnitAt(i);
  }
}
