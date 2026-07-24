import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/vault/report_bundle_verifier.dart';
import 'package:hidden_shield_mobile/features/vault/report_handoff_bundle.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Android verifies desktop image audio and L2 report bundles', (
    tester,
  ) async {
    const expected = {
      'image': 'hsr-r3-image-desktop',
      'audio': 'hsr-r3-audio-desktop',
      'l2-video': 'hsr-r3-l2-video-desktop',
    };

    for (final entry in expected.entries) {
      final result = await verifyMobileReportBundle(
        entry.key,
        readBytes: (directory, relativePath) async {
          final data = await rootBundle.load(
            'test/fixtures/report_bundles_r3/$directory/$relativePath',
          );
          return data.buffer.asUint8List(
            data.offsetInBytes,
            data.lengthInBytes,
          );
        },
      );

      expect(result.reportId, entry.value);
      expect(result.integrityStatus, 'matched');
      expect(result.manifestChainStatus, 'matched');
      expect(result.documentContractStatus, 'matched');
      expect(result.signatureStatus, 'not_signed');
      expect(result.trustedTimeStatus, 'not_timestamped');
      expect(result.files.every((file) => file.status == 'matched'), isTrue);
    }
  });

  testWidgets('Android generates a desktop-verifiable Manifest v2 handoff', (
    tester,
  ) async {
    final record = VaultRecord(
      id: 'android-r3-handoff-501',
      kind: WatermarkAssetKind.image,
      title: 'android-r3-handoff.png',
      watermarkUid: 'HS-ANDROID-R3-HANDOFF-000501',
      revision: 1,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.localOnly,
      createdAt: DateTime.parse('2026-07-14T03:00:00Z'),
      sha256: List.filled(64, 'c').join(),
    );
    final draft = FormalReportDraft.fromRecord(
      record: record,
      exportedAt: DateTime.parse('2026-07-14T03:10:00Z'),
      appVersion: 'mobile',
    );

    final bundle = buildMobileReportHandoffBundle(record: record, draft: draft);
    final manifest =
        jsonDecode(utf8.decode(bundle.manifestJsonBytes))
            as Map<String, dynamic>;

    expect(manifest['schemaVersion'], 2);
    expect(manifest['reportType'], 'formal_report_handoff');
    expect(
      (manifest['renderer'] as Map<String, dynamic>)['workerMode'],
      'not_rendered',
    );
    expect(
      (manifest['integrity'] as Map<String, dynamic>)['algorithm'],
      'sha256_chain_v1',
    );
  });
}
