import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_saf_bridge.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel(
    'com.hiddenshield.hidden_shield_mobile/rights_evidence_saf',
  );
  const bridge = RightsEvidencePackSafBridge(channel: channel);
  final calls = <MethodCall>[];

  setUp(() {
    calls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          return switch (call.method) {
            'pickTree' || 'getPersistedTree' => {
              'treeUri': 'content://downloads/tree/fixture',
              'displayName': 'case-fixture-r4-0001',
              'persisted': true,
            },
            'readFile' => Uint8List.fromList([1, 2, 3]),
            'listDirectory' => {
              'topLevelEntries': [
                'attachments',
                'case-manifest.json',
                'case.json',
              ],
              'attachmentPaths': [
                'attachments/original/ATT-01-original-work.txt',
              ],
              'caseFileSafe': true,
              'manifestFileSafe': true,
              'attachmentTreeSafe': true,
            },
            'clearPersistedTree' => null,
            _ => throw MissingPluginException(),
          };
        });
  });

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test('maps persisted tree descriptors and byte reads', () async {
    final picked = await bridge.pickDirectory();
    final persisted = await bridge.getPersistedDirectory();
    final bytes = await bridge.readBytes(picked!.treeUri, 'case-manifest.json');

    expect(picked.displayName, 'case-fixture-r4-0001');
    expect(picked.persisted, isTrue);
    expect(persisted?.treeUri, picked.treeUri);
    expect(bytes, Uint8List.fromList([1, 2, 3]));
    expect(calls.map((call) => call.method), [
      'pickTree',
      'getPersistedTree',
      'readFile',
    ]);
    expect(calls.last.arguments, {
      'treeUri': 'content://downloads/tree/fixture',
      'relativePath': 'case-manifest.json',
    });
  });

  test('maps SAF directory safety fields', () async {
    final listing = await bridge.listDirectory(
      'content://downloads/tree/fixture',
    );

    expect(listing.topLevelEntries, [
      'attachments',
      'case-manifest.json',
      'case.json',
    ]);
    expect(listing.attachmentPaths, [
      'attachments/original/ATT-01-original-work.txt',
    ]);
    expect(listing.caseFileSafe, isTrue);
    expect(listing.manifestFileSafe, isTrue);
    expect(listing.attachmentTreeSafe, isTrue);
  });

  test('freezes platform failure codes and user messages', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          throw PlatformException(
            code: 'evidence_pack_provider_unavailable',
            message: 'provider disabled',
          );
        });

    await expectLater(
      bridge.listDirectory('content://provider/tree/case'),
      throwsA(
        isA<RightsEvidencePackAccessException>()
            .having(
              (error) => error.code,
              'code',
              RightsEvidencePackAccessFailureCode.providerUnavailable,
            )
            .having(
              (error) => error.userMessage,
              'userMessage',
              '文件提供方当前不可用，请恢复对应应用或改选本地目录。',
            ),
      ),
    );

    expect(
      RightsEvidencePackAccessFailureCode.values
          .where((code) => code != RightsEvidencePackAccessFailureCode.unknown)
          .map((code) => code.wireCode),
      [
        'evidence_pack_authorization_revoked',
        'evidence_pack_directory_missing',
        'evidence_pack_attachment_missing',
        'evidence_pack_provider_unavailable',
      ],
    );
  });
}
