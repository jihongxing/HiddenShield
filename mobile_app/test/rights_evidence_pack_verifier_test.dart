import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_file_reader.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_verifier.dart';

void main() {
  final fixtureRoot =
      '${Directory.current.path}${Platform.pathSeparator}'
      'test${Platform.pathSeparator}fixtures${Platform.pathSeparator}'
      'rights_evidence_pack_r4${Platform.pathSeparator}'
      'case-fixture-r4-0001';

  test('verifies the desktop R4 rights evidence pack fixture', () async {
    final result = await const RightsEvidencePackVerifier().verify(fixtureRoot);

    expect(result.packId, 'hsep-fixture-r4-0001');
    expect(result.caseId, 'case-fixture-r4-0001');
    expect(result.manifestSchemaVersion, 1);
    expect(result.directoryContractStatus, 'matched');
    expect(result.attachmentIntegrityStatus, 'matched');
    expect(result.eventChainStatus, 'matched');
    expect(result.attachmentChainStatus, 'matched');
    expect(result.signatureStatus, 'not_signed');
    expect(result.trustedTimeStatus, 'not_timestamped');
    expect(
      result.declaredRootDigest,
      '4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33',
    );
    expect(result.computedRootDigest, result.declaredRootDigest);
    expect(result.attachments, hasLength(4));
    expect(
      result.attachments.every((attachment) => attachment.status == 'matched'),
      isTrue,
    );

    final json = result.toJson();
    expect(json['directoryContractStatus'], 'matched');
    expect(json['attachmentIntegrityStatus'], 'matched');
    expect(json['eventChainStatus'], 'matched');
    expect(json['attachmentChainStatus'], 'matched');
    expect(json['signatureStatus'], 'not_signed');
    expect(json['trustedTimeStatus'], 'not_timestamped');
    expect(json.containsKey('directory_contract_status'), isFalse);
  });

  test('detects attachment tampering independently', () async {
    final verifier = RightsEvidencePackVerifier(
      readBytes: (caseDir, relativePath) async {
        if (relativePath == 'attachments/original/ATT-01-original-work.txt') {
          return Uint8List.fromList(utf8.encode('tampered attachment'));
        }
        return File(
          '$caseDir${Platform.pathSeparator}'
          '${relativePath.replaceAll('/', Platform.pathSeparator)}',
        ).readAsBytes();
      },
    );

    final result = await verifier.verify(fixtureRoot);

    expect(result.directoryContractStatus, 'matched');
    expect(result.attachmentIntegrityStatus, 'mismatch');
    expect(result.eventChainStatus, 'matched');
    expect(result.attachmentChainStatus, 'matched');
  });

  test('detects event tampering and root digest mismatch', () async {
    final verifier = RightsEvidencePackVerifier(
      readBytes: (caseDir, relativePath) async {
        final file = File(
          '$caseDir${Platform.pathSeparator}'
          '${relativePath.replaceAll('/', Platform.pathSeparator)}',
        );
        final bytes = await file.readAsBytes();
        if (relativePath != 'case.json') return bytes;
        final document = jsonDecode(utf8.decode(bytes)) as Map<String, dynamic>;
        final events = document['collectionEvents'] as List<dynamic>;
        final first = events.first as Map<String, dynamic>;
        first['note'] = 'tampered event';
        return Uint8List.fromList(utf8.encode(jsonEncode(document)));
      },
    );

    final result = await verifier.verify(fixtureRoot);

    expect(result.directoryContractStatus, 'mismatch');
    expect(result.attachmentIntegrityStatus, 'matched');
    expect(result.eventChainStatus, 'mismatch');
    expect(result.attachmentChainStatus, 'matched');
  });

  test('rejects an unregistered physical attachment', () async {
    final listing = await listRightsEvidencePackDirectory(fixtureRoot);
    final verifier = RightsEvidencePackVerifier(
      readDirectory: (_) async => RightsEvidencePackDirectoryListing(
        topLevelEntries: listing.topLevelEntries,
        attachmentPaths: [
          ...listing.attachmentPaths,
          'attachments/capture/UNREGISTERED.txt',
        ],
        caseFileSafe: listing.caseFileSafe,
        manifestFileSafe: listing.manifestFileSafe,
        attachmentTreeSafe: listing.attachmentTreeSafe,
      ),
    );

    final result = await verifier.verify(fixtureRoot);

    expect(result.directoryContractStatus, 'mismatch');
    expect(result.attachmentIntegrityStatus, 'mismatch');
    expect(result.eventChainStatus, 'matched');
    expect(result.attachmentChainStatus, 'matched');
  });

  test('stable JSON sorts nested keys and normalizes integral doubles', () {
    expect(
      stableRightsEvidenceJsonString({
        'z': 1.0,
        'a': [
          {'b': true, 'a': '值'},
        ],
      }),
      '{"a":[{"a":"值","b":true}],"z":1}',
    );
    expect(
      () => stableRightsEvidenceJsonString({1: 'invalid key'}),
      throwsFormatException,
    );
  });
}
