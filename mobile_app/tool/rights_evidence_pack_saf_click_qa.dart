import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/local_preview_watermark_bridge.dart';
import 'package:hidden_shield_mobile/features/verify/verify_page.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_saf_bridge.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_verifier.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final appState = MobileAppState(vaultStore: MemoryVaultStore());
  await appState.load();
  runApp(_RightsEvidencePackSafQaApp(appState: appState));
}

class _RightsEvidencePackSafQaApp extends StatefulWidget {
  const _RightsEvidencePackSafQaApp({required this.appState});

  final MobileAppState appState;

  @override
  State<_RightsEvidencePackSafQaApp> createState() =>
      _RightsEvidencePackSafQaAppState();
}

class _RightsEvidencePackSafQaAppState
    extends State<_RightsEvidencePackSafQaApp> {
  final RightsEvidencePackSafBridge _safBridge =
      const RightsEvidencePackSafBridge();

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Stack(
        children: [
          VerifyPage(
            bridge: const PreviewWatermarkBridge(),
            appState: widget.appState,
            rightsEvidencePackSafBridge: _safBridge,
            onRightsEvidencePackVerified: _emitResult,
            onRightsEvidencePackAccessFailure: _emitFailure,
          ),
          SafeArea(
            child: Align(
              alignment: Alignment.topLeft,
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Material(
                  child: OutlinedButton(
                    onPressed: _clearAuthorization,
                    child: const Text('QA 撤销目录授权'),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _clearAuthorization() async {
    await _safBridge.clearPersistedDirectory();
    debugPrint('RIGHTS_EVIDENCE_PACK_SAF_QA_CONTROL:authorization_cleared');
  }

  void _emitResult(RightsEvidencePackVerificationResult result) {
    debugPrint(
      'RIGHTS_EVIDENCE_PACK_SAF_QA_RESULT:${jsonEncode({'caseId': result.caseId, 'packId': result.packId, 'directoryContractStatus': result.directoryContractStatus, 'attachmentIntegrityStatus': result.attachmentIntegrityStatus, 'eventChainStatus': result.eventChainStatus, 'attachmentChainStatus': result.attachmentChainStatus, 'signatureStatus': result.signatureStatus, 'trustedTimeStatus': result.trustedTimeStatus, 'declaredRootDigest': result.declaredRootDigest, 'computedRootDigest': result.computedRootDigest, 'matchedAttachmentCount': result.attachments.where((attachment) => attachment.status == 'matched').length, 'attachmentCount': result.attachments.length})}',
    );
  }

  void _emitFailure(RightsEvidencePackAccessException failure) {
    debugPrint(
      'RIGHTS_EVIDENCE_PACK_SAF_QA_FAILURE:${jsonEncode({'code': failure.code.wireCode, 'userMessage': failure.userMessage, 'technicalMessage': failure.technicalMessage})}',
    );
  }
}
