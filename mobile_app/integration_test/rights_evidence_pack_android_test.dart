import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/local_preview_watermark_bridge.dart';
import 'package:hidden_shield_mobile/features/verify/verify_page.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_file_reader.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_verifier.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Android verifies the desktop R4 rights evidence pack', (
    tester,
  ) async {
    const fixtureDir = 'case-fixture-r4-0001';
    const fixturePrefix = 'test/fixtures/rights_evidence_pack_r4/$fixtureDir/';
    final assetManifest = await AssetManifest.loadFromAssetBundle(rootBundle);
    final fixtureFiles =
        assetManifest
            .listAssets()
            .where((asset) => asset.startsWith(fixturePrefix))
            .map((asset) => asset.substring(fixturePrefix.length))
            .toList()
          ..sort();
    final topLevelEntries =
        fixtureFiles
            .map((relativePath) => relativePath.split('/').first)
            .toSet()
            .toList()
          ..sort();
    final attachmentPaths =
        fixtureFiles
            .where((relativePath) => relativePath.startsWith('attachments/'))
            .toList()
          ..sort();

    final verifier = RightsEvidencePackVerifier(
      readBytes: (_, relativePath) async {
        final data = await rootBundle.load('$fixturePrefix$relativePath');
        return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
      },
      readDirectory: (_) async => RightsEvidencePackDirectoryListing(
        topLevelEntries: topLevelEntries,
        attachmentPaths: attachmentPaths,
        caseFileSafe: fixtureFiles.contains('case.json'),
        manifestFileSafe: fixtureFiles.contains('case-manifest.json'),
        attachmentTreeSafe: true,
      ),
    );

    final result = await verifier.verify(fixtureDir);
    final serialized = jsonDecode(jsonEncode(result.toJson()));
    tester.printToConsole(
      jsonEncode({
        'fixtureFiles': fixtureFiles,
        'topLevelEntries': topLevelEntries,
        'result': result.toJson(),
      }),
    );

    expect(result.directoryContractStatus, 'matched');
    expect(result.attachmentIntegrityStatus, 'matched');
    expect(result.eventChainStatus, 'matched');
    expect(result.attachmentChainStatus, 'matched');
    expect(result.signatureStatus, 'not_signed');
    expect(result.trustedTimeStatus, 'not_timestamped');
    expect(result.attachments, hasLength(4));
    expect(
      result.computedRootDigest,
      '4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33',
    );
    expect(result.computedRootDigest, result.declaredRootDigest);
    expect(serialized['directoryContractStatus'], 'matched');
    expect(serialized.containsKey('directory_contract_status'), isFalse);
  });

  const runExternalQa = bool.fromEnvironment(
    'RUN_RIGHTS_EVIDENCE_PACK_EXTERNAL_QA',
  );
  testWidgets(
    'Android verify page reads an adb-pushed external R4 case directory',
    (tester) async {
      final externalRoot = await getExternalStorageDirectory();
      expect(externalRoot, isNotNull);
      final externalCaseDir = path.join(
        externalRoot!.path,
        'rights-evidence-pack-qa',
        'case-fixture-r4-0001',
      );
      final externalDirectory = Directory(externalCaseDir);
      if (await externalDirectory.exists()) {
        await externalDirectory.delete(recursive: true);
      }
      await externalDirectory.create(recursive: true);
      tester.printToConsole(
        'RIGHTS_EVIDENCE_PACK_EXTERNAL_READY:$externalCaseDir',
      );
      final caseFile = File(
        '$externalCaseDir${Platform.pathSeparator}case.json',
      );
      final deadline = DateTime.now().add(const Duration(seconds: 45));
      while (!await caseFile.exists() && DateTime.now().isBefore(deadline)) {
        await Future<void>.delayed(const Duration(milliseconds: 250));
      }
      expect(
        await caseFile.exists(),
        isTrue,
        reason: '主机未在等待窗口内推入外部案件包 fixture',
      );

      final directResult = await const RightsEvidencePackVerifier().verify(
        externalCaseDir,
      );
      expect(directResult.directoryContractStatus, 'matched');
      expect(directResult.attachmentIntegrityStatus, 'matched');
      expect(directResult.eventChainStatus, 'matched');
      expect(directResult.attachmentChainStatus, 'matched');
      expect(directResult.signatureStatus, 'not_signed');
      expect(directResult.trustedTimeStatus, 'not_timestamped');
      expect(
        directResult.computedRootDigest,
        '4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33',
      );

      final appState = MobileAppState(vaultStore: MemoryVaultStore());
      await appState.load();
      await tester.pumpWidget(
        MaterialApp(
          home: VerifyPage(
            bridge: const PreviewWatermarkBridge(),
            appState: appState,
            pickRightsEvidencePackDirectory: () async => externalCaseDir,
          ),
        ),
      );
      await tester.pumpAndSettle();

      final button = find.byKey(
        const ValueKey('verify-rights-evidence-pack-button'),
      );
      await tester.scrollUntilVisible(
        button,
        220,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(button);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('rights-evidence-status-directory')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-status-attachments')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-status-events')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-status-attachment-chain')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-status-signature')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-status-trusted-time')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-pack-error')),
        findsNothing,
      );
      expect(
        find.byKey(const ValueKey('rights-evidence-pack-boundary')),
        findsOneWidget,
      );
      tester.printToConsole(
        jsonEncode({
          'status': 'passed',
          'source': 'android_external_directory',
          'caseDir': externalCaseDir,
          'result': directResult.toJson(),
        }),
      );
    },
    skip: !runExternalQa,
  );
}
