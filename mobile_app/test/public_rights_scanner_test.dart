import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/public_rights/public_rights_scanner.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';

void main() {
  test(
    'resolvePolicy never treats registry declaration as legal conclusion',
    () {
      final policy = resolvePublicRightsPolicy(_rights());

      expect(policy.trainingPolicy, 'commercial_training_allowed');
      expect(policy.legalConclusion, isFalse);
      expect(policy.canTreatAsTrainingAllowed, isFalse);
      expect(policy.requiresHumanReview, isFalse);
    },
  );

  test('formatUserMessage maps backfill pending to explicit wording', () {
    final scan = _rights(warnings: const ['backfill_pending']);
    final result = PublicRightsSdkResult(
      status: PublicRightsSdkStatus.error,
      scan: scan,
      error: PublicRightsSdkErrorCode.backfillPending,
      warnings: scan.warnings,
      message: '',
      policy: resolvePublicRightsPolicy(scan),
    );

    expect(formatPublicRightsUserMessage(result), contains('尚未完成回填'));
  });

  test(
    'classifyPublicRightsError maps registry unavailable without leaking HTTP',
    () {
      expect(
        classifyPublicRightsError(Exception('HTTP 403 forbidden')),
        PublicRightsSdkErrorCode.registryUnavailable,
      );
      expect(
        classifyPublicRightsError(Exception('payload_invalid')),
        PublicRightsSdkErrorCode.payloadInvalid,
      );
    },
  );
}

PublicRightsQueryResponse _rights({List<String> warnings = const []}) {
  return PublicRightsQueryResponse(
    watermarkUid: 'HS-TEST',
    scanStatus: 'registry_active',
    registry: const PublicRightsRegistrySnapshot(
      registryStatus: 'registered',
      payloadAuthStatus: 'verified',
      watermarkIdIssueMode: 'server_confirmed',
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      anchorProtocol: 'v2_migration_anchor',
      mediaPayloadRole: 'legacy_bridge_anchor',
      rightsSource: 'creator_declaration_registry',
    ),
    rightsManifest: RightsManifestResponse(
      rightsManifestId: 'manifest-1',
      manifestVersion: 1,
      status: 'active',
      trainingPolicy: 'commercial_training_allowed',
      manifestSha256: 'hash',
      effectiveAt: DateTime.utc(2026, 6, 29),
    ),
    publicMetadata: const PublicRightsMetadata(consistency: 'registry_only'),
    trainingPermission: const PublicTrainingPermissionSnapshot(
      policy: 'commercial_training_allowed',
      label: '允许商业训练',
      effectiveSource: 'registry',
      legalConclusion: false,
    ),
    warnings: warnings,
    resolvedAt: DateTime.utc(2026, 6, 29),
  );
}
