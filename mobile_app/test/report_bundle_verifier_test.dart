import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/vault/report_bundle_verifier.dart';

void main() {
  final fixtureRoot =
      '${Directory.current.path}${Platform.pathSeparator}'
      'test${Platform.pathSeparator}fixtures${Platform.pathSeparator}'
      'report_bundles_r3';

  test('verifies desktop image audio and L2 video report bundles', () async {
    final expected = {
      'image': 'hsr-r3-image-desktop',
      'audio': 'hsr-r3-audio-desktop',
      'l2-video': 'hsr-r3-l2-video-desktop',
    };

    for (final entry in expected.entries) {
      final reportDir = '$fixtureRoot${Platform.pathSeparator}${entry.key}';
      final result = await verifyMobileReportBundle(reportDir);

      expect(result.reportId, entry.value);
      expect(result.reportType, 'formal_report');
      expect(result.bundleVersion, 1);
      expect(result.integrityStatus, 'matched');
      expect(result.manifestChainStatus, 'matched');
      expect(result.documentContractStatus, 'matched');
      expect(result.signatureStatus, 'not_signed');
      expect(result.trustedTimeStatus, 'not_timestamped');
      expect(result.files.map((file) => file.path), [
        'report.pdf',
        'report.json',
      ]);
      expect(result.files.every((file) => file.status == 'matched'), isTrue);
    }
  });

  test('detects a tampered desktop PDF on mobile', () async {
    final reportDir = '$fixtureRoot${Platform.pathSeparator}image';
    final result = await verifyMobileReportBundle(
      reportDir,
      readBytes: (directory, relativePath) async {
        if (relativePath == 'report.pdf') {
          return Uint8List.fromList('%PDF-TAMPERED'.codeUnits);
        }
        return File(
          '$directory${Platform.pathSeparator}$relativePath',
        ).readAsBytes();
      },
    );

    expect(result.integrityStatus, 'mismatch');
    expect(result.manifestChainStatus, 'matched');
    expect(
      result.files.firstWhere((file) => file.path == 'report.pdf').status,
      'mismatch',
    );
    expect(result.signatureStatus, 'not_signed');
  });

  test(
    'rejects manifest entries outside the read-only report bundle',
    () async {
      final reportDir = '$fixtureRoot${Platform.pathSeparator}image';

      await expectLater(
        verifyMobileReportBundle(
          reportDir,
          readBytes: (directory, relativePath) async {
            final bytes = await File(
              '$directory${Platform.pathSeparator}$relativePath',
            ).readAsBytes();
            if (relativePath != 'manifest.json') return bytes;
            final manifest =
                jsonDecode(utf8.decode(bytes)) as Map<String, dynamic>;
            final files = manifest['files'] as List<dynamic>;
            files.add({
              'path': 'original-media.png',
              'bytes': 1,
              'sha256': List.filled(64, '0').join(),
            });
            return Uint8List.fromList(utf8.encode(jsonEncode(manifest)));
          },
        ),
        throwsA(isA<FormatException>()),
      );
    },
  );
}
