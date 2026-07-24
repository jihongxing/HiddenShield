import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  desktopVerify: readFileSync('src/views/VerifyView.vue', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileVerify: readFileSync('mobile_app/lib/features/verify/verify_page.dart', 'utf8'),
  mobileCloudClient: readFileSync('mobile_app/lib/sync/cloud_account_client.dart', 'utf8'),
  mobileVaultStore: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  desktopReportCommand: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  desktopReportPdf: readFileSync('src-tauri/src/report_pdf.rs', 'utf8'),
  desktopReportWorker: readFileSync('src-tauri/resources/report-pdf/chromium-worker.mjs', 'utf8'),
  desktopTauriConfig: readFileSync('src-tauri/tauri.conf.json', 'utf8'),
  mobileReportVerifier: readFileSync(
    'mobile_app/lib/features/vault/report_bundle_verifier.dart',
    'utf8',
  ),
  mobileReportHandoff: readFileSync(
    'mobile_app/lib/features/vault/report_handoff_bundle.dart',
    'utf8',
  ),
  mobileReportHandoffTest: readFileSync(
    'mobile_app/test/report_handoff_bundle_test.dart',
    'utf8',
  ),
  mobileReportHandoffManifest: readFileSync(
    'mobile_app/test/fixtures/report_handoff_r3/mobile-image/manifest.json',
    'utf8',
  ),
  mobileHandoffRuntimeQa: readFileSync(
    'scripts/run-report-mobile-handoff-runtime-qa.mjs',
    'utf8',
  ),
  mobileHandoffRuntimeQaBin: readFileSync(
    'src-tauri/examples/report_mobile_handoff_runtime_qa.rs',
    'utf8',
  ),
  mobileReportVerifierTest: readFileSync(
    'mobile_app/test/report_bundle_verifier_test.dart',
    'utf8',
  ),
  mobileReportAndroidTest: readFileSync(
    'mobile_app/integration_test/report_bundle_android_test.dart',
    'utf8',
  ),
  reportR3FixtureGenerator: readFileSync('scripts/generate-report-r3-fixtures.mjs', 'utf8'),
  reportR3FixtureIndex: readFileSync(
    'mobile_app/test/fixtures/report_bundles_r3/fixture-index.json',
    'utf8',
  ),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  commercialModel: readFileSync('docs/商业模式规划.md', 'utf8'),
  freeReportDesign: readFileSync('docs/Phase 8 Free单份报告付费设计.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
};

assert(
  sources.desktopVerify.includes('report_export') &&
    sources.desktopVerify.includes('canExportFormalReports'),
  'desktop verify page must gate formal report export with report_export',
);
assert(
  sources.desktopVault.includes('report_export') &&
    sources.desktopVault.includes('canExportFormalReports'),
  'desktop vault page must gate formal report export with report_export',
);
assert(
  sources.desktopApi.includes('基础验证摘要') &&
    !sources.desktopApi.includes('HiddenShield 数字版权存证报告'),
  'desktop copied verification text must be a basic summary, not a formal report',
);
assert(
  sources.mobileState.includes('canExportFormalReports') &&
    sources.mobileState.includes("entitlementFeatures['report_export'] == true"),
  'mobile app state must expose report_export entitlement gate',
);
assert(
  sources.mobileVault.includes('canExportFormalReports') &&
    sources.mobileVault.includes('buildFormalReportDraft') &&
    sources.mobileVault.includes('Creator 正式报告'),
  'mobile vault page must surface Creator formal report gate',
);
assert(
  sources.mobileVerify.includes('canExportFormalReports') &&
    sources.mobileVerify.includes('Creator 导出正式报告'),
  'mobile verify page must surface Creator formal report gate',
);
assert(
  sources.desktopReportCommand.includes('export_vault_formal_report') &&
    sources.desktopReportCommand.includes('export_vault_batch_summary_report'),
  'desktop must expose formal and batch report export commands',
);
assert(
  sources.desktopReportCommand.includes('report_dir') &&
    sources.desktopApi.includes('reportDir: string') &&
    sources.desktopApi.includes('pdfPath: string') &&
    sources.desktopApi.includes('jsonPath: string') &&
    sources.desktopApi.includes('manifestPath: string'),
  'desktop report export result must include reportDir for opening exported reports',
);
assert(
  sources.desktopReportCommand.includes('report.pdf') &&
    sources.desktopReportCommand.includes('report.json') &&
    sources.desktopReportCommand.includes('manifest.json') &&
    sources.desktopReportCommand.includes('build_report_manifest') &&
    sources.desktopReportCommand.includes('sha256_hex'),
  'desktop report export must atomically generate PDF, JSON, and Manifest from one FormalReportDocument',
);
assert(
  sources.desktopReportCommand.includes('verify_formal_report_bundle') &&
    sources.desktopReportCommand.includes('build_integrity_chain') &&
    sources.desktopReportCommand.includes('verify_integrity_chain') &&
    sources.desktopReportCommand.includes('report_bundle_verification_detects_file_tampering'),
  'desktop report bundles must expose offline verification, a SHA-256 chain, and a tamper regression test',
);
assert(
  sources.desktopReportCommand.includes('bundle_version') &&
    sources.desktopReportCommand.includes('supersedes_report_id') &&
    sources.desktopReportCommand.includes('find_previous_report_manifest'),
  'desktop report regeneration must record bundle versions and superseded report ids',
);
assert(
  sources.desktopReportCommand.includes('"not_signed"') &&
    sources.desktopReportCommand.includes('"not_timestamped"') &&
    sources.desktopReportCommand.includes('"present_unverified"') &&
    sources.desktopReportCommand.includes('"not_issued"'),
  'report verification must distinguish integrity, signature, trusted time, and unavailable QR status',
);
assert(
  sources.desktopReportPdf.includes('ReportPdfWorkerManager') &&
    sources.desktopReportPdf.includes('REPORT_PDF_GENERATION_BUDGET_MS: u64 = 3_000') &&
    sources.desktopReportPdf.includes('worker: Option<ReportPdfWorker>') &&
    sources.desktopReportCommand.includes('persistent_warm_worker') &&
    sources.desktopReportWorker.includes('HiddenShieldReportTemplate') &&
    sources.desktopReportWorker.includes('REPORT_PDF_GENERATION_BUDGET_EXCEEDED'),
  'desktop report renderer must use the persistent Chromium worker and enforce the 3 second budget',
);
assert(
  sources.desktopReportWorker.includes('NotoSansSC-Controlled.ttf') &&
    sources.desktopReportWorker.includes('NotoSerifSC-Controlled.ttf') &&
    sources.desktopTauriConfig.includes('resources/report-pdf/**/*'),
  'desktop PDF renderer must load and bundle controlled Chinese fonts',
);
assert(
  sources.mobileReportVerifier.includes('if (schemaVersion != 2)') &&
    sources.mobileReportVerifier.includes('sha256_chain_v1') &&
    sources.mobileReportVerifier.includes('report.pdf') &&
    sources.mobileReportVerifier.includes('report.json') &&
    sources.mobileReportVerifier.includes('manifest.json') &&
    sources.mobileReportVerifier.includes(
      'Manifest v2 只允许 report.pdf 和 report.json 两个受校验文件',
    ) &&
    !sources.mobileReportVerifier.includes('watermark'),
  'mobile report verification must be a read-only Manifest schema v2 integrity check without watermark inference',
);
assert(
  sources.mobileReportVerifier.includes('integrityStatus') &&
    sources.mobileReportVerifier.includes('manifestChainStatus') &&
    sources.mobileReportVerifier.includes('documentContractStatus') &&
    sources.mobileReportVerifier.includes('signatureStatus') &&
    sources.mobileReportVerifier.includes('trustedTimeStatus') &&
    sources.mobileReportVerifier.includes("'not_signed'") &&
    sources.mobileReportVerifier.includes("'not_timestamped'"),
  'mobile report verification must distinguish file integrity, chain, document contract, signature, and trusted time',
);
assert(
  sources.mobileVault.includes('校验桌面报告包') &&
    sources.mobileVault.includes('移动端只读复算文件摘要与 Manifest 链') &&
    sources.mobileVault.includes('不判断签名可信'),
  'mobile vault must expose desktop report bundle verification with explicit trust boundaries',
);
assert(
  sources.mobileReportVerifierTest.includes("'image'") &&
    sources.mobileReportVerifierTest.includes("'audio'") &&
    sources.mobileReportVerifierTest.includes("'l2-video'") &&
    sources.mobileReportVerifierTest.includes('detects a tampered desktop PDF on mobile') &&
    sources.mobileReportVerifierTest.includes(
      'rejects manifest entries outside the read-only report bundle',
    ) &&
    sources.mobileReportVerifierTest.includes("expect(result.integrityStatus, 'mismatch')") &&
    sources.mobileReportAndroidTest.includes(
      'Android verifies desktop image audio and L2 report bundles',
    ) &&
    sources.reportR3FixtureGenerator.includes('sha256_chain_v1') &&
    sources.reportR3FixtureIndex.includes('"mediaKind": "image"') &&
    sources.reportR3FixtureIndex.includes('"mediaKind": "audio"') &&
    sources.reportR3FixtureIndex.includes('"mediaKind": "l2_video"'),
  'R3 must keep desktop-generated image, audio, and L2 video fixtures plus host tamper and Android runtime coverage',
);
assert(
  sources.mobileReportHandoff.includes('formal_report_handoff') &&
    sources.mobileReportHandoff.includes('mobile_handoff') &&
    sources.mobileReportHandoff.includes('not_rendered') &&
    sources.mobileReportHandoff.includes('sha256_chain_v1') &&
    sources.mobileReportHandoff.includes('awaiting_desktop_render') &&
    sources.mobileReportHandoffTest.includes(
      'builds a mobile Manifest v2 desktop render handoff',
    ) &&
    sources.mobileReportHandoffManifest.includes(
      '"reportType": "formal_report_handoff"',
    ) &&
    sources.desktopReportCommand.includes(
      'desktop_verifies_mobile_generated_report_handoff_fixture',
    ) &&
    sources.desktopReportCommand.includes('import_mobile_report_handoff') &&
    sources.desktopReportCommand.includes('prepare_mobile_report_handoff_import') &&
    sources.desktopReportCommand.includes('source_handoff_root_digest') &&
    sources.desktopReportCommand.includes(
      'imported_manifest_records_mobile_handoff_root_digest',
    ) &&
    sources.desktopReportCommand.includes(
      'mobile_handoff_import_rejects_tampered_report_json',
    ) &&
    sources.desktopReportCommand.includes('ensure_report_export_entitled') &&
    sources.desktopReportCommand.includes('persistent_warm_worker') &&
    sources.desktopReportCommand.includes('document_contract_status') &&
    sources.desktopVerify.includes('跨端报告包校验') &&
    sources.desktopVerify.includes('生成最终 PDF') &&
    sources.desktopApi.includes('importMobileReportHandoff') &&
    sources.mobileVault.includes('生成桌面签发交接包'),
  'R3 reverse flow must verify a mobile Manifest v2 handoff, render it through the persistent Chromium worker, and record the source root digest without claiming signatures exist',
);
assert(
  sources.packageJson.includes('"report:mobile-handoff-runtime-qa"') &&
    sources.mobileHandoffRuntimeQa.includes('report_mobile_handoff_runtime_qa') &&
    sources.mobileHandoffRuntimeQa.includes('import_mobile_report_handoff') &&
    sources.mobileHandoffRuntimeQa.includes('report.pdf') &&
    sources.mobileHandoffRuntimeQa.includes('report.json') &&
    sources.mobileHandoffRuntimeQa.includes('manifest.json') &&
    sources.mobileHandoffRuntimeQa.includes('sourceHandoffRootDigest') &&
    sources.mobileHandoffRuntimeQa.includes('sha256_chain_v1') &&
    sources.mobileHandoffRuntimeQa.includes('pdfGenerationMs <= 3_000') &&
    sources.mobileHandoffRuntimeQaBin.includes('tauri::test::mock_app()') &&
    sources.mobileHandoffRuntimeQaBin.includes('run_mobile_report_handoff_runtime_qa'),
  'R3 must keep a Tauri runtime QA that imports the mobile fixture and verifies the final PDF, JSON, Manifest, source root digest, and SHA-256 chain',
);
assert(
  sources.desktopVault.includes('recentReportExports') &&
    sources.desktopVault.includes('openOutputDir') &&
    sources.desktopVault.includes('copyReportPath') &&
    sources.desktopVault.includes('verifyReportBundle'),
  'desktop vault page must keep recent report exports with open/copy actions',
);
assert(
  sources.desktopVault.includes('loadRecentReportExports') &&
    sources.desktopVault.includes('saveRecentReportExport'),
  'desktop vault page must restore recent report exports across launches',
);
assert(
  sources.desktopVerify.includes('latestReportExport') &&
    sources.desktopVerify.includes('openOutputDir') &&
    sources.desktopVerify.includes('copyReportPath') &&
    sources.desktopVerify.includes('verifyLatestReportBundle'),
  'desktop verify page must expose open/copy actions after report export',
);
assert(
  sources.desktopVerify.includes('saveRecentReportExport'),
  'desktop verify page must persist its latest report export into recent history',
);
assert(
  sources.desktopReportCommand.includes('excludes_local_media_paths: true') &&
    sources.desktopReportCommand.includes('record_report_usage') &&
    sources.desktopReportCommand.includes('"report_export"'),
  'desktop report export must omit local paths and record report_export usage',
);
assert(
  sources.desktopReportCommand.includes('FormalReportVideoNotary') &&
    sources.desktopReportCommand.includes('video_notary: FormalReportVideoNotary') &&
    sources.desktopReportCommand.includes('video_notary_receipt_signature') &&
    sources.desktopReportCommand.includes('video_fingerprint_root') &&
    sources.desktopReportCommand.includes('video_bundle_sha256') &&
    sources.desktopReportCommand.includes('video_frame_sample_policy') &&
    sources.desktopReportCommand.includes('### 视频指纹存证'),
  'desktop formal report must include L2 video notary receipt and bundle metadata',
);
assert(
  sources.mobileState.includes('UsageMediaType.report') &&
    sources.mobileState.includes("featureName: 'report_export'") &&
    !sources.mobileState.includes('static const report = WatermarkAssetKind.video'),
  'mobile report export must use report media type without polluting video usage',
);
assert(
  sources.mobileState.includes('## 视频指纹存证') &&
    sources.mobileState.includes('record.videoNotaryReceiptSignature') &&
    sources.mobileState.includes('record.videoFingerprintRoot') &&
    sources.mobileState.includes('record.videoBundleSha256') &&
    sources.mobileState.includes('record.videoFrameSamplePolicy'),
  'mobile formal report draft must include the same L2 video notary receipt and bundle metadata',
);
assert(
  !sources.desktopReportCommand.includes('bundlePath') &&
    !sources.mobileState.includes('bundlePath') &&
    !sources.desktopReportCommand.includes('originalVideoPath') &&
    !sources.mobileState.includes('originalVideoPath'),
  'formal reports must not expose local bundle paths or original video paths',
);
assert(
  sources.roadmap.includes('Phase 5') &&
    sources.roadmap.includes('report_export'),
  'roadmap must record report_export Phase 5 progress',
);

assert(
  sources.freeReportDesign.includes('copyright_report_single') &&
    sources.freeReportDesign.includes('rights_evidence_pack_single') &&
    sources.freeReportDesign.includes('19.9 元 / 份') &&
    sources.freeReportDesign.includes('49.9 元 / 份') &&
    sources.freeReportDesign.includes('report_purchase_grants') &&
    sources.freeReportDesign.includes('Free 可复制基础摘要') &&
    sources.freeReportDesign.includes('未购买时不能导出正式报告或证据包') &&
    sources.freeReportDesign.includes('已进入双端版权库 UI，并完成双端记录级导出核销') &&
    sources.backendSchema.includes('ReportPurchaseSessionRequest') &&
    sources.backendSchema.includes('ReportPurchaseGrant') &&
    sources.backendLib.includes('/v1/billing/report-purchase-sessions') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS report_purchase_grants') &&
    sources.backendStorage.includes('price_cents, currency, payment_session_id') &&
    sources.backendStorage.includes('entitlement.plan_code, "free"') &&
    sources.backendStorage.includes('entitlement.features["report_export"], false') &&
    sources.commercialModel.includes('单份版权详细报告 | 19.9 元 / 份') &&
    sources.commercialModel.includes('维权证据包 | 49.9 元 / 份') &&
    sources.roadmap.includes('Free 单份付费报告') &&
    sources.roadmap.includes('双端版权库已接入购买入口与单记录导出核销') &&
    sources.desktopVault.includes('createReportPurchaseSession') &&
    sources.desktopVault.includes('购买版权详细报告') &&
    sources.desktopVault.includes('购买维权证据包') &&
    sources.desktopApi.includes('createReportPurchaseSession') &&
    sources.desktopApi.includes('reconcileReportPurchaseSession') &&
    sources.mobileCloudClient.includes('createReportPurchaseSession') &&
    sources.mobileCloudClient.includes('getReportPurchaseSessionStatus') &&
    sources.mobileCloudClient.includes('reconcileReportPurchaseSession') &&
    sources.mobileState.includes('createReportPurchaseSession') &&
    sources.mobileState.includes('reconcileReportPurchaseSession') &&
    sources.mobileState.includes('canExportFormalReportForRecord') &&
    sources.mobileState.includes('reportPurchaseGrantsJson') &&
    sources.mobileVaultStore.includes('report_purchase_grants_json') &&
    sources.mobileVault.includes('购买版权详细报告') &&
    sources.mobileVault.includes('购买维权证据包') &&
    sources.mobileVault.includes('createReportPurchaseSession') &&
    sources.mobileVault.includes('reconcileReportPurchaseSession') &&
    sources.desktopReportCommand.includes('ensure_single_report_export_entitled') &&
    sources.desktopReportCommand.includes('has_active_report_purchase_grant') &&
    sources.capabilityBoundary.includes('双端版权库已接入购买入口和记录级导出核销') &&
    sources.capabilityBoundary.includes('Free 单份版权详细报告和维权证据包纳入本版封版范围') &&
    sources.capabilityBoundary.includes('未配置时展示支付通道未完成配置'),
  'Free one-off report purchase must be documented with fixed products, prices, grant model, double-end fixture entry, and backend fixture boundary',
);

console.log('Report export contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Report export contract failed: ${message}`);
    process.exit(1);
  }
}
