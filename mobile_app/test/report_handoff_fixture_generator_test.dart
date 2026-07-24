import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/vault/report_handoff_bundle.dart';

void main() {
  test('generates deterministic mobile report handoff fixture', () async {
    const generateFixture = bool.fromEnvironment(
      'GENERATE_REPORT_HANDOFF_FIXTURE',
    );
    if (!generateFixture) return;

    final exportedAt = DateTime.parse('2026-07-14T02:00:00Z');
    final record = VaultRecord(
      id: 'mobile-r3-image-401',
      kind: WatermarkAssetKind.image,
      title: 'mobile-r3-image.png',
      watermarkUid: 'HS-MOBILE-R3-IMAGE-000401',
      revision: 2,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.localOnly,
      createdAt: DateTime.parse('2026-07-14T01:00:00Z'),
      creatorDisplayName: '移动端创作者',
      sha256: List.filled(64, 'a').join(),
      protectedCopyName: 'mobile-r3-image-protected.png',
      protectedCopyHash: List.filled(64, 'b').join(),
      writeVerificationStatus: WriteVerificationStatus.verified,
      writeVerificationMessage: '移动端写入后读取验证通过',
      writeVerificationAt: DateTime.parse('2026-07-14T01:05:00Z'),
      trustedTimeStatus: 'recorded',
      trustedTimeSource: 'mobile_network_time',
      trustedTimeAt: DateTime.parse('2026-07-14T01:06:00Z'),
    );
    final draft = FormalReportDraft.fromRecord(
      record: record,
      exportedAt: exportedAt,
      appVersion: 'mobile',
    );
    final bundle = buildMobileReportHandoffBundle(record: record, draft: draft);
    final outputDir = Directory('test/fixtures/report_handoff_r3/mobile-image');
    await outputDir.create(recursive: true);
    await File(
      '${outputDir.path}${Platform.pathSeparator}report.json',
    ).writeAsBytes(bundle.reportJsonBytes);
    await File(
      '${outputDir.path}${Platform.pathSeparator}manifest.json',
    ).writeAsBytes(bundle.manifestJsonBytes);

    expect(await File('${outputDir.path}/report.json').exists(), isTrue);
    expect(await File('${outputDir.path}/manifest.json').exists(), isTrue);
  });
}
