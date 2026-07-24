import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const fixtureDir = path.join(
  repoRoot,
  "mobile_app",
  "test",
  "fixtures",
  "report_handoff_r3",
  "mobile-image",
);
const runId = `run-${Date.now()}`;
const outputRoot = path.join(repoRoot, "tmp", "report-mobile-handoff-runtime-qa", runId);
await mkdir(outputRoot, { recursive: true });

const sourceManifest = JSON.parse(
  await readFile(path.join(fixtureDir, "manifest.json"), "utf8"),
);
const sourceReport = JSON.parse(
  await readFile(path.join(fixtureDir, "report.json"), "utf8"),
);
const resourceDir = path.join(repoRoot, "src-tauri", "resources", "report-pdf");
const result = spawnSync(
  "cargo",
  [
    "run",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--features",
    "runtime-qa",
    "--example",
    "report_mobile_handoff_runtime_qa",
    "--",
    "--fixture-dir",
    fixtureDir,
    "--output-dir",
    outputRoot,
  ],
  {
    cwd: repoRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      HIDDENSHIELD_NODE_PATH: process.execPath,
      HIDDENSHIELD_REPORT_PDF_RESOURCE_DIR: resourceDir,
    },
    windowsHide: true,
  },
);
if (result.status !== 0) {
  process.stderr.write(result.stderr);
  process.stdout.write(result.stdout);
  throw new Error(`report mobile handoff runtime QA failed with status ${result.status}`);
}
const resultLine = result.stdout
  .trim()
  .split(/\r?\n/)
  .reverse()
  .find((line) => line.trim().startsWith("{"));
assert(resultLine, "runtime QA binary must return JSON");
const exported = JSON.parse(resultLine);

const reportDir = path.resolve(exported.reportDir);
assert(
  reportDir.startsWith(path.resolve(outputRoot) + path.sep),
  "generated report directory must stay inside the requested QA output root",
);
const expectedPaths = {
  pdf: path.join(reportDir, "report.pdf"),
  json: path.join(reportDir, "report.json"),
  manifest: path.join(reportDir, "manifest.json"),
};
assert(path.resolve(exported.pdfPath) === expectedPaths.pdf, "PDF result path mismatch");
assert(path.resolve(exported.jsonPath) === expectedPaths.json, "JSON result path mismatch");
assert(
  path.resolve(exported.manifestPath) === expectedPaths.manifest,
  "Manifest result path mismatch",
);

const [pdfBytes, reportJsonBytes, manifestBytes] = await Promise.all([
  readFile(expectedPaths.pdf),
  readFile(expectedPaths.json),
  readFile(expectedPaths.manifest),
]);
const finalReport = JSON.parse(reportJsonBytes.toString("utf8"));
const finalManifest = JSON.parse(manifestBytes.toString("utf8"));

assert(pdfBytes.subarray(0, 4).toString("ascii") === "%PDF", "report.pdf must be a PDF");
assert(pdfBytes.length > 100_000, "report.pdf must contain a rendered high-fidelity report");
assert(exported.reportType === "formal_report", "import result must be a formal report");
assert(exported.pdfPageCount === 4, "imported PDF must remain four pages");
assert(exported.pdfGenerationMs <= 3_000, "imported PDF must pass the 3 second gate");
assert(finalReport.schemaVersion === 2, "final report.json must use schema v2");
assert(finalReport.reportId === exported.reportId, "final report ID mismatch");
assert(finalReport.reportType === "formal_report", "final report type mismatch");
assert(
  finalReport.records?.[0]?.watermarkUid === sourceReport.records?.[0]?.watermarkUid,
  "final report must preserve the mobile watermark UID fact",
);
assert(finalManifest.schemaVersion === 2, "final Manifest must use schema v2");
assert(finalManifest.reportId === exported.reportId, "final Manifest report ID mismatch");
assert(finalManifest.reportType === "formal_report", "final Manifest report type mismatch");
assert(
  finalManifest.bundle.sourceHandoffReportId === sourceManifest.reportId,
  "source handoff report ID mismatch",
);
assert(
  finalManifest.bundle.sourceHandoffSourceKey === sourceManifest.bundle.sourceKey,
  "source handoff source key mismatch",
);
assert(
  finalManifest.bundle.sourceHandoffRootDigest === sourceManifest.integrity.rootDigest,
  "source handoff root digest mismatch",
);
assert(
  finalManifest.bundle.sourceHandoffPlatform === sourceReport.sourcePlatform,
  "source handoff platform mismatch",
);
assert(
  finalManifest.renderer.workerMode === "persistent_warm_worker",
  "final PDF must use the persistent Chromium worker",
);
assert(finalManifest.renderer.pageCount === 4, "final Manifest page count mismatch");
assert(
  finalManifest.renderer.generationMs <= finalManifest.renderer.generationBudgetMs,
  "final Manifest generation budget mismatch",
);
assert(
  finalManifest.renderer.controlledFonts.includes("NotoSansSC-Controlled.ttf") &&
    finalManifest.renderer.controlledFonts.includes("NotoSerifSC-Controlled.ttf"),
  "controlled Chinese fonts must be recorded",
);
assert(finalManifest.signature.status === "not_signed", "final report must remain unsigned");
assert(
  finalManifest.trustedTime.packageTimestampPresent === false,
  "final report must not claim a package trusted timestamp",
);

const filesByPath = new Map(finalManifest.files.map((file) => [file.path, file]));
for (const [relativePath, bytes] of [
  ["report.pdf", pdfBytes],
  ["report.json", reportJsonBytes],
]) {
  const entry = filesByPath.get(relativePath);
  assert(entry, `Manifest missing ${relativePath}`);
  assert(entry.bytes === bytes.length, `${relativePath} byte count mismatch`);
  assert(entry.sha256 === sha256(bytes), `${relativePath} SHA-256 mismatch`);
}
assert(verifyIntegrityChain(finalManifest.files, finalManifest.integrity), "final SHA-256 chain mismatch");
assert((await stat(expectedPaths.pdf)).size === exportedFileSize(finalManifest, "report.pdf"), "PDF stat mismatch");

console.log(
  JSON.stringify(
    {
      status: "passed",
      command: "import_mobile_report_handoff",
      fixtureDir,
      reportDir,
      reportId: exported.reportId,
      sourceHandoffReportId: finalManifest.bundle.sourceHandoffReportId,
      sourceHandoffRootDigest: finalManifest.bundle.sourceHandoffRootDigest,
      finalRootDigest: finalManifest.integrity.rootDigest,
      pdfBytes: pdfBytes.length,
      pdfPageCount: exported.pdfPageCount,
      pdfGenerationMs: exported.pdfGenerationMs,
      outputRoot,
    },
    null,
    2,
  ),
);

function verifyIntegrityChain(files, integrity) {
  if (integrity.algorithm !== "sha256_chain_v1" || integrity.entries.length !== files.length) {
    return false;
  }
  let previousChainDigest = sha256(Buffer.from(integrity.genesis));
  for (let index = 0; index < files.length; index += 1) {
    const file = files[index];
    const entry = integrity.entries[index];
    const sequence = index + 1;
    if (
      entry.sequence !== sequence ||
      entry.path !== file.path ||
      entry.fileSha256 !== file.sha256 ||
      entry.fileBytes !== file.bytes ||
      entry.previousChainDigest !== previousChainDigest
    ) {
      return false;
    }
    const chainDigest = sha256(
      Buffer.from(
        `${sequence}\n${file.path}\n${file.bytes}\n${file.sha256}\n${previousChainDigest}`,
      ),
    );
    if (entry.chainDigest !== chainDigest) return false;
    previousChainDigest = chainDigest;
  }
  return previousChainDigest === integrity.rootDigest;
}

function exportedFileSize(manifest, relativePath) {
  return manifest.files.find((file) => file.path === relativePath)?.bytes ?? -1;
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assert(condition, message) {
  if (!condition) throw new Error(`Report mobile handoff runtime QA failed: ${message}`);
}
