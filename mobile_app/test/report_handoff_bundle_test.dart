import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/vault/report_handoff_bundle.dart';

void main() {
  test('builds a mobile Manifest v2 desktop render handoff', () {
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
    final report =
        jsonDecode(utf8.decode(bundle.reportJsonBytes)) as Map<String, dynamic>;
    final manifest =
        jsonDecode(utf8.decode(bundle.manifestJsonBytes))
            as Map<String, dynamic>;

    expect(report['schemaVersion'], 2);
    expect(report['reportType'], 'formal_report_handoff');
    expect(report['reportId'], draft.reportId);
    expect(
      (report['handoff'] as Map<String, dynamic>)['status'],
      'awaiting_desktop_render',
    );
    expect(manifest['schemaVersion'], 2);
    expect(manifest['reportId'], draft.reportId);
    expect(manifest['reportType'], 'formal_report_handoff');
    expect((manifest['files'] as List<dynamic>).length, 1);
    expect(
      ((manifest['files'] as List<dynamic>).single
          as Map<String, dynamic>)['path'],
      'report.json',
    );
    expect(
      (manifest['integrity'] as Map<String, dynamic>)['algorithm'],
      'sha256_chain_v1',
    );
    expect(
      (manifest['signature'] as Map<String, dynamic>)['status'],
      'not_signed',
    );
    expect(
      (manifest['trustedTime']
          as Map<String, dynamic>)['packageTimestampPresent'],
      isFalse,
    );
  });
}
