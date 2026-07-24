import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

const fixture = JSON.parse(
  readFileSync("docs/contracts/rights-evidence-pack-v1.fixture.json", "utf8"),
);
const design = readFileSync(
  "docs/Phase R4 维权证据包案件级Schema与附件目录合同.md",
  "utf8",
);
const prototype = readFileSync(
  "docs/prototypes/rights-evidence-pack-r4/finalized.html",
  "utf8",
);
const prototypeMetrics = JSON.parse(
  readFileSync(
    "docs/prototypes/rights-evidence-pack-r4/finalized.json",
    "utf8",
  ),
);
const prototypePdfPath =
  "docs/prototypes/rights-evidence-pack-r4/finalized.pdf";
const bundleSource = readFileSync(
  "docs/contracts/rights-evidence-pack-bundle-v1.source.json",
  "utf8",
);
const bundleGenerator = readFileSync(
  "scripts/generate-rights-evidence-pack-r4-bundle.mjs",
  "utf8",
);
const bundleVerifier = readFileSync(
  "scripts/verify-rights-evidence-pack-r4-bundle.mjs",
  "utf8",
);
const bundleManifest = JSON.parse(
  readFileSync(
    "docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001/case-manifest.json",
    "utf8",
  ),
);
const desktopReportCommand = readFileSync(
  "src-tauri/src/commands/report.rs",
  "utf8",
);
const desktopTauriLib = readFileSync("src-tauri/src/lib.rs", "utf8");
const desktopApi = readFileSync("src/lib/tauri-api.ts", "utf8");
const packageJson = readFileSync("package.json", "utf8");
const runtimeQa = readFileSync(
  "src-tauri/examples/rights_evidence_pack_runtime_qa.rs",
  "utf8",
);
const desktopVerifyView = readFileSync("src/views/VerifyView.vue", "utf8");
const mobileVerifier = readFileSync(
  "mobile_app/lib/features/vault/rights_evidence_pack_verifier.dart",
  "utf8",
);
const mobileVerifierTest = readFileSync(
  "mobile_app/test/rights_evidence_pack_verifier_test.dart",
  "utf8",
);
const mobileAndroidTest = readFileSync(
  "mobile_app/integration_test/rights_evidence_pack_android_test.dart",
  "utf8",
);
const mobileVerifyPage = readFileSync(
  "mobile_app/lib/features/verify/verify_page.dart",
  "utf8",
);
const mobileWidgetTest = readFileSync("mobile_app/test/widget_test.dart", "utf8");
const mobileFixtureSync = readFileSync(
  "scripts/sync-rights-evidence-pack-r4-mobile-fixture.mjs",
  "utf8",
);
const mobileExternalQa = readFileSync(
  "scripts/run-rights-evidence-pack-r4-android-external-qa.mjs",
  "utf8",
);
const mobilePubspec = readFileSync("mobile_app/pubspec.yaml", "utf8");
const mobileAndroidMainActivity = readFileSync(
  "mobile_app/android/app/src/main/kotlin/com/hiddenshield/hidden_shield_mobile/MainActivity.kt",
  "utf8",
);
const mobileAndroidBuild = readFileSync(
  "mobile_app/android/app/build.gradle.kts",
  "utf8",
);
const mobileSafBridge = readFileSync(
  "mobile_app/lib/features/vault/rights_evidence_pack_saf_bridge.dart",
  "utf8",
);
const mobileSafBridgeTest = readFileSync(
  "mobile_app/test/rights_evidence_pack_saf_bridge_test.dart",
  "utf8",
);
const mobileSafClickQa = readFileSync(
  "scripts/run-rights-evidence-pack-r4-saf-click-qa.mjs",
  "utf8",
);
const mobileSafQaTarget = readFileSync(
  "mobile_app/tool/rights_evidence_pack_saf_click_qa.dart",
  "utf8",
);
const mobileSafAccessFailure = readFileSync(
  "mobile_app/lib/features/vault/rights_evidence_pack_access_failure.dart",
  "utf8",
);
const mobileSafFailureMatrix = readFileSync(
  "scripts/run-rights-evidence-pack-r4-saf-failure-matrix-qa.mjs",
  "utf8",
);
const qaDocumentsProviderBuild = readFileSync(
  "mobile_app/android/qa_documents_provider/build.gradle.kts",
  "utf8",
);
const qaDocumentsProviderManifest = readFileSync(
  "mobile_app/android/qa_documents_provider/src/main/AndroidManifest.xml",
  "utf8",
);
const qaDocumentsProviderSource = readFileSync(
  "mobile_app/android/qa_documents_provider/src/main/java/com/hiddenshield/qa/documentsprovider/RightsEvidenceQaDocumentsProvider.java",
  "utf8",
);

assert(fixture.schemaVersion === 1, "schemaVersion must be 1");
assert(fixture.documentType === "rights_evidence_pack", "documentType mismatch");
assert(["draft", "review_ready"].includes(fixture.status), "unsupported pack status");
assert(fixture.case?.purpose === "rights_enforcement_support", "case purpose mismatch");
assert(fixture.copyrightFacts?.length > 0, "copyright facts are required");
assert(
  fixture.copyrightFacts.every(
    (fact) =>
      isSha256(fact.reportRootDigest) &&
      typeof fact.reportId === "string" &&
      typeof fact.recordId === "string" &&
      typeof fact.watermarkUid === "string",
  ),
  "copyright facts must reference a formal report root digest and record facts",
);

const attachments = new Map(
  fixture.attachments.map((attachment) => [attachment.attachmentId, attachment]),
);
assert(attachments.size === fixture.attachments.length, "attachment IDs must be unique");
assert(
  fixture.infringementSamples?.length > 0 &&
    fixture.infringementSamples.every(
      (sample) =>
        sample.source?.kind &&
        sample.source?.value &&
        sample.capturedAt &&
        ["device_claimed", "unverified", "trusted"].includes(sample.captureTimeStatus) &&
        isSha256(sample.sha256) &&
        Number.isInteger(sample.bytes) &&
        sample.bytes >= 0 &&
        attachments.has(sample.attachmentId),
    ),
  "every infringement sample must have source, time status, digest, bytes, and attachment",
);
assert(
  fixture.infringementSamples.every((sample) => {
    const attachment = attachments.get(sample.attachmentId);
    return attachment.sha256 === sample.sha256 && attachment.bytes === sample.bytes;
  }),
  "sample and attachment integrity metadata must match",
);
assert(
  fixture.collectionEvents.every((event) =>
    event.attachmentIds.every((attachmentId) => attachments.has(attachmentId)),
  ),
  "collection events must reference known attachments",
);
assert(
  fixture.automatedFindings.every(
    (finding) =>
      finding.method &&
      finding.observation &&
      finding.limitations &&
      finding.inputAttachmentIds.every((attachmentId) => attachments.has(attachmentId)),
  ),
  "automated findings must declare method, inputs, observation, and limitations",
);
assert(
  fixture.humanStatements.every(
    (statement) =>
      statement.authorDisplayName &&
      statement.authorRole &&
      statement.statement &&
      ["not_signed", "signed_external"].includes(statement.signatureStatus),
  ),
  "human statements must remain explicit and independently attributed",
);

const serialized = JSON.stringify(fixture);
for (const forbiddenClaim of ["侵权成立", "司法认可", "公证完成", "确权成功"]) {
  assert(!serialized.includes(forbiddenClaim), `forbidden legal claim: ${forbiddenClaim}`);
}
assert(
  design.includes("自动结论与人工陈述分离") &&
    design.includes("reportRootDigest") &&
    design.includes("只能内部测试") &&
    design.includes("不构成法律意见"),
  "R4 design must freeze lineage, separation, and capability boundaries",
);
assert(
  prototype.includes("../../contracts/rights-evidence-pack-v1.fixture.json") &&
    prototype.includes("案件封面") &&
    prototype.includes("证据目录") &&
    prototype.includes("版权事实") &&
    prototype.includes("争议对象与侵权样本") &&
    prototype.includes("采集过程与操作记录") &&
    prototype.includes("自动技术观察") &&
    prototype.includes("人工陈述与确认页") &&
    prototype.includes("限制说明与附件索引") &&
    prototype.includes("NotoSansSC-Controlled.ttf") &&
    prototype.includes("NotoSerifSC-Controlled.ttf"),
  "R4 prototype must render the eight fixture-driven pages with controlled fonts",
);
assert(
  existsSync(prototypePdfPath) &&
    statSync(prototypePdfPath).size > 100_000 &&
    prototypeMetrics.status === "passed" &&
    prototypeMetrics.pageCount === 8 &&
    prototypeMetrics.overflow.every((page) => page.overflow === false) &&
    prototypeMetrics.capabilityBoundary.signatureStatus === "not_signed" &&
    prototypeMetrics.capabilityBoundary.trustedTimeStatus === "not_timestamped" &&
    prototypeMetrics.capabilityBoundary.legalConclusionStatus === "not_evaluated",
  "R4 PDF prototype must remain eight stable unsigned pages without legal conclusions",
);
assert(
  bundleSource.includes('"original"') &&
    bundleSource.includes('"working_copy"') &&
    bundleSource.includes('"capture"') &&
    bundleSource.includes('"external_receipt"') &&
    bundleGenerator.includes("sha256_append_chain_v1") &&
    bundleGenerator.includes("HiddenShield-Rights-Evidence-Pack-Event-Chain-v1") &&
    bundleGenerator.includes("HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1") &&
    bundleVerifier.includes("appendOnlyEventPrefixPreserved") &&
    bundleVerifier.includes("symbolic links are forbidden") &&
    bundleManifest.eventChain.algorithm === "sha256_append_chain_v1" &&
    bundleManifest.attachmentChain.algorithm === "sha256_append_chain_v1" &&
    bundleManifest.signature.status === "not_signed" &&
    bundleManifest.trustedTime.status === "not_timestamped",
  "R4 bundle must freeze four attachment roles and keep independent append-only event and attachment chains",
);
assert(
  desktopReportCommand.includes("verify_rights_evidence_pack") &&
    desktopReportCommand.includes("directory_contract_status") &&
    desktopReportCommand.includes("attachment_integrity_status") &&
    desktopReportCommand.includes("event_chain_status") &&
    desktopReportCommand.includes("attachment_chain_status") &&
    desktopReportCommand.includes("signature_status") &&
    desktopReportCommand.includes("trusted_time_status") &&
    desktopReportCommand.includes(
      "verifies_rights_evidence_pack_fixture_with_six_independent_statuses",
    ) &&
    desktopReportCommand.includes(
      "rights_evidence_pack_attachment_tamper_only_breaks_attachment_integrity",
    ) &&
    desktopTauriLib.includes("commands::report::verify_rights_evidence_pack") &&
    desktopApi.includes("verifyRightsEvidencePack") &&
    packageJson.includes('"report:r4-tauri-contract"'),
  "R4 must expose a registered read-only Tauri verifier with six independent statuses and fixture tamper tests",
);
assert(
  runtimeQa.includes("tauri::test::get_ipc_response") &&
    runtimeQa.includes('cmd: "verify_rights_evidence_pack"') &&
    runtimeQa.includes('"directoryContractStatus"') &&
    runtimeQa.includes('"attachmentIntegrityStatus"') &&
    runtimeQa.includes('"eventChainStatus"') &&
    runtimeQa.includes('"attachmentChainStatus"') &&
    runtimeQa.includes('"signatureStatus"') &&
    runtimeQa.includes('"trustedTimeStatus"') &&
    runtimeQa.includes("directory_contract_status") &&
    packageJson.includes('"report:r4-runtime-qa"') &&
    desktopVerifyView.includes("verifyRightsEvidencePack") &&
    desktopVerifyView.includes('data-testid="rights-evidence-pack-verifier"') &&
    desktopVerifyView.includes('data-testid="rights-evidence-pack-status-grid"') &&
    desktopVerifyView.includes("校验只复算目录、文件与摘要链") &&
    desktopVerifyView.includes("不读取媒体水印"),
  "R4 must keep a MockRuntime IPC serialization QA and a desktop six-status read-only verifier UI",
);
assert(
  mobileVerifier.includes("class RightsEvidencePackVerifier") &&
    mobileVerifier.includes("stableRightsEvidenceJsonString") &&
    mobileVerifier.includes("sha256_append_chain_v1") &&
    mobileVerifier.includes("sha256_case_event_attachment_roots_v1") &&
    mobileVerifier.includes("directoryContractStatus") &&
    mobileVerifier.includes("attachmentIntegrityStatus") &&
    mobileVerifier.includes("eventChainStatus") &&
    mobileVerifier.includes("attachmentChainStatus") &&
    mobileVerifier.includes("signatureStatus") &&
    mobileVerifier.includes("trustedTimeStatus") &&
    mobileVerifierTest.includes("detects attachment tampering independently") &&
    mobileVerifierTest.includes("detects event tampering and root digest mismatch") &&
    mobileVerifierTest.includes("rejects an unregistered physical attachment") &&
    mobileAndroidTest.includes("Android verifies the desktop R4 rights evidence pack") &&
    mobileAndroidTest.includes("AssetManifest.loadFromAssetBundle") &&
    mobileAndroidTest.includes(
      "Android verify page reads an adb-pushed external R4 case directory",
    ) &&
    mobileAndroidTest.includes("getExternalStorageDirectory") &&
    mobileAndroidTest.includes("RIGHTS_EVIDENCE_PACK_EXTERNAL_READY") &&
    mobileVerifyPage.includes("FilePicker.getDirectoryPath") &&
    mobileVerifyPage.includes(
      "ValueKey('verify-rights-evidence-pack-button')",
    ) &&
    mobileVerifyPage.includes("rights-evidence-status-${item.key}") &&
    mobileVerifyPage.includes("key: 'directory'") &&
    mobileVerifyPage.includes("key: 'trusted-time'") &&
    mobileVerifyPage.includes("校验只复算目录、文件与摘要链") &&
    mobileVerifyPage.includes("不读取媒体水印") &&
    mobileWidgetTest.includes(
      "verifies an R4 rights evidence pack from the verify page",
    ) &&
    mobileExternalQa.includes("adb") &&
    mobileExternalQa.includes("push") &&
    mobileExternalQa.includes("pushedFileCount") &&
    mobileFixtureSync.includes("docs") &&
    mobileFixtureSync.includes("mobile_app") &&
    mobilePubspec.includes(
      "test/fixtures/rights_evidence_pack_r4/case-fixture-r4-0001/",
    ) &&
    mobilePubspec.includes(
      "case-fixture-r4-0001/attachments/original/",
    ) &&
    mobilePubspec.includes(
      "case-fixture-r4-0001/attachments/working-copy/",
    ) &&
    mobilePubspec.includes(
      "case-fixture-r4-0001/attachments/capture/",
    ) &&
    mobilePubspec.includes(
      "case-fixture-r4-0001/attachments/external-receipt/",
    ) &&
    packageJson.includes('"mobile:report-r4-contract"') &&
    packageJson.includes('"mobile:report-r4-android"') &&
    packageJson.includes('"mobile:report-r4-external-android"') &&
    packageJson.includes('"mobile:report-r4-saf-click-android"') &&
    mobileAndroidBuild.includes(
      'androidx.documentfile:documentfile:1.0.1',
    ) &&
    mobileAndroidMainActivity.includes("Intent.ACTION_OPEN_DOCUMENT_TREE") &&
    mobileAndroidMainActivity.includes("takePersistableUriPermission") &&
    mobileAndroidMainActivity.includes("persistedUriPermissions") &&
    mobileAndroidMainActivity.includes("DocumentFile.fromTreeUri") &&
    mobileAndroidMainActivity.includes("getSharedPreferences") &&
    mobileAndroidMainActivity.includes(
      "com.hiddenshield.hidden_shield_mobile/rights_evidence_saf",
    ) &&
    mobileSafBridge.includes("class RightsEvidencePackSafBridge") &&
    mobileSafBridge.includes("getPersistedDirectory") &&
    mobileSafBridge.includes("RightsEvidencePackDirectoryListing") &&
    mobileSafBridgeTest.includes("maps persisted tree descriptors") &&
    mobileVerifyPage.includes("校验已授权目录") &&
    mobileVerifyPage.includes("重新选择案件包目录") &&
    mobileSafClickQa.includes("/sdcard/Download/HiddenShield-R4-QA") &&
    mobileSafClickQa.includes("uiautomator") &&
    mobileSafClickQa.includes("force-stop") &&
    mobileSafQaTarget.includes("RIGHTS_EVIDENCE_PACK_SAF_QA_RESULT") &&
    mobileSafQaTarget.includes("RIGHTS_EVIDENCE_PACK_SAF_QA_FAILURE") &&
    mobileSafAccessFailure.includes(
      "evidence_pack_authorization_revoked",
    ) &&
    mobileSafAccessFailure.includes("evidence_pack_directory_missing") &&
    mobileSafAccessFailure.includes("evidence_pack_attachment_missing") &&
    mobileSafAccessFailure.includes(
      "evidence_pack_provider_unavailable",
    ) &&
    mobileSafFailureMatrix.includes("attachment deletion") &&
    mobileSafFailureMatrix.includes("directory move") &&
    mobileSafFailureMatrix.includes("authorization revocation") &&
    mobileSafFailureMatrix.includes("provider disable") &&
    mobileSafFailureMatrix.includes("HiddenShield QA Provider") &&
    qaDocumentsProviderBuild.includes(
      "docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001",
    ) &&
    qaDocumentsProviderManifest.includes(
      "android.content.action.DOCUMENTS_PROVIDER",
    ) &&
    qaDocumentsProviderManifest.includes(
      "com.hiddenshield.qa.documentsprovider.documents",
    ) &&
    qaDocumentsProviderSource.includes("extends DocumentsProvider") &&
    qaDocumentsProviderSource.includes("case-fixture-r4-provider") &&
    packageJson.includes('"mobile:report-r4-saf-failure-matrix"'),
  "R4 must keep a Flutter verifier, frozen SAF failure contract, and Android Download plus independent DocumentsProvider QA",
);
assert(
  fixtureDirectoriesMatch(
    "docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001",
    "mobile_app/test/fixtures/rights_evidence_pack_r4/case-fixture-r4-0001",
  ),
  "mobile R4 fixture must remain byte-identical to the desktop-generated fixture",
);

console.log("Rights evidence pack R4 contract OK");

function isSha256(value) {
  return typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Rights evidence pack R4 contract failed: ${message}`);
    process.exit(1);
  }
}

function fixtureDirectoriesMatch(leftRoot, rightRoot) {
  const leftFiles = listFiles(leftRoot);
  const rightFiles = listFiles(rightRoot);
  if (JSON.stringify(leftFiles) !== JSON.stringify(rightFiles)) return false;
  return leftFiles.every((relativePath) =>
    readFileSync(path.join(leftRoot, relativePath)).equals(
      readFileSync(path.join(rightRoot, relativePath)),
    ),
  );
}

function listFiles(root, directory = root) {
  return readdirSync(directory, { withFileTypes: true })
    .flatMap((entry) => {
      const absolutePath = path.join(directory, entry.name);
      if (entry.isDirectory()) return listFiles(root, absolutePath);
      if (!entry.isFile()) return [];
      return [path.relative(root, absolutePath).replaceAll("\\", "/")];
    })
    .sort();
}
