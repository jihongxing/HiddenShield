import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline";

const repoRoot = path.resolve(import.meta.dirname, "..");
const resourceDir = path.join(repoRoot, "src-tauri", "resources", "report-pdf");
const outputDir = path.join(repoRoot, "tmp", "report-pdf-r1-gate");
const pdfPath = path.join(outputDir, "report.pdf");
const reportJsonPath = path.join(outputDir, "report.json");
const manifestPath = path.join(outputDir, "manifest.json");

await rm(outputDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });

const document = buildFixtureDocument();
const reportJson = `${JSON.stringify(document, null, 2)}\n`;
await writeFile(reportJsonPath, reportJson);

const worker = spawn(
  process.env.HIDDENSHIELD_NODE_PATH ?? process.execPath,
  [
    path.join(resourceDir, "chromium-worker.mjs"),
    "--resourceDir",
    resourceDir,
    "--maxGenerationMs",
    "3000"
  ],
  {
    cwd: repoRoot,
    stdio: ["pipe", "pipe", "inherit"],
    windowsHide: true
  }
);
const messages = createInterface({ input: worker.stdout, crlfDelay: Infinity });
const iterator = messages[Symbol.asyncIterator]();

const ready = await nextMessage(iterator, "ready");
worker.stdin.write(`${JSON.stringify({
  type: "render",
  requestId: 1,
  document,
  outputPath: pdfPath
})}\n`);
const result = await nextMessage(iterator, "result");

assert(result.ok === true, result.error ?? "worker render failed");
assert(result.generationMs <= 3000, `generation budget exceeded: ${result.generationMs}ms`);
assert(result.pageCount === 4, `expected 4 pages, got ${result.pageCount}`);
assert(result.pageOverflow.every((page) => page.overflow === false), "page overflow detected");
assert(result.fontState.sansLoaded === true, "controlled sans font not loaded");
assert(result.fontState.serifLoaded === true, "controlled serif font not loaded");

const pdf = await readFile(pdfPath);
const pdfStat = await stat(pdfPath);
assert(pdfStat.size === result.bytes, "PDF size mismatch");
assert(sha256(pdf) === result.sha256, "PDF SHA-256 mismatch");

const manifestFiles = [
  {
    path: "report.pdf",
    mediaType: "application/pdf",
    bytes: result.bytes,
    sha256: result.sha256
  },
  {
    path: "report.json",
    mediaType: "application/json",
    bytes: Buffer.byteLength(reportJson),
    sha256: sha256(Buffer.from(reportJson))
  }
];
const manifest = {
  schemaVersion: 2,
  reportId: document.reportId,
  reportType: document.reportType,
  generatedAt: document.exportedAt,
  sourceSchemaVersion: document.schemaVersion,
  bundle: {
    sourceKey: sha256(Buffer.from(`${document.reportType}|7`)),
    bundleVersion: 1,
    supersedesReportId: null
  },
  renderer: {
    engine: "chromium",
    workerMode: "persistent_warm_worker",
    templateVersion: "R1.0",
    controlledFonts: [
      "NotoSansSC-Controlled.ttf",
      "NotoSerifSC-Controlled.ttf"
    ],
    generationMs: result.generationMs,
    generationBudgetMs: 3000,
    pageCount: result.pageCount,
    paginationStable: true
  },
  files: manifestFiles,
  integrity: buildIntegrityChain(manifestFiles),
  signature: {
    status: "not_signed",
    profile: null,
    signerKeyId: null,
    certificateChainStatus: "not_evaluated",
    revocationStatus: "not_applicable",
    signedAt: null,
    note: "R2 runtime gate fixture"
  },
  trustedTime: {
    status: "not_verified",
    packageTimestampPresent: false,
    recordMaterialTokenPresent: true,
    note: "记录级 TSA 材料状态不等于报告包已获得可信时间戳。"
  },
  verification: {
    offlineMode: "sha256_chain_v1",
    onlineStatus: "not_deployed",
    qrStatus: "not_issued",
    onlineVerificationUrl: null
  }
};
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

worker.stdin.write('{"type":"shutdown"}\n');
worker.stdin.end();
await new Promise((resolve, reject) => {
  worker.once("exit", (code) => code === 0 ? resolve() : reject(new Error(`worker exited ${code}`)));
});

console.log(JSON.stringify({
  status: "passed",
  workerLaunchMs: ready.launchMs,
  generationMs: result.generationMs,
  generationBudgetMs: 3000,
  pageCount: result.pageCount,
  bytes: result.bytes,
  sha256: result.sha256,
  outputDir
}, null, 2));

async function nextMessage(iterator, expectedType) {
  const next = await iterator.next();
  if (next.done) throw new Error(`worker closed before ${expectedType}`);
  const message = JSON.parse(next.value);
  assert(message.type === expectedType, `expected ${expectedType}, got ${message.type}`);
  return message;
}

function buildFixtureDocument() {
  return {
    schemaVersion: 2,
    reportId: "hsr-r1-runtime-gate-image",
    reportType: "formal_report",
    exportedAt: "2026-07-14T08:00:00+08:00",
    appVersion: "0.1.0",
    records: [{
      recordId: 7,
      fileName: "城市晨雾摄影.png",
      watermarkUid: "HS-V3-IMG-7F31A2D9",
      creatorDisplayName: "林川影像工作室",
      originalHash: "8af42e0d2cd1d75b0fe72ae82cf5e5ed88f02a2a50ce08f91376a4ce",
      resolution: "4032 × 3024",
      durationSecs: 0,
      createdAt: "2026-07-12T09:42:18+08:00",
      revision: 1,
      parentWatermarkUid: null,
      rewriteReason: null,
      writeVerificationStatus: "verified",
      writeVerificationMessage: "保护副本写后读取验证通过",
      writeVerificationAt: "2026-07-12T09:42:21+08:00",
      payloadRegistry: {
        payloadProtocolVersion: 3,
        payloadBytesLength: 39,
        mediaPayloadRole: "v3_minimal_anchor",
        watermarkIdIssueMode: "offline_generated",
        watermarkIdRegistryStatus: "pending_registration",
        watermarkIdRegistryReceipt: null,
        payloadAuthStatus: "verified"
      },
      protectedCopy: {
        name: "城市晨雾摄影_protected.png",
        hash: "d2977a4fe51babc978309ba57fbac02a632bf55d29f0a1b88dc07259",
        outputStrategy: "minimal_required_change"
      },
      trustedTime: {
        networkTime: "2026-07-12T09:42:22+08:00",
        tsaSource: "fixture-tsa",
        tsaTokenPresent: true
      },
      rightsDeclaration: {
        workSourceDeclaration: "原创拍摄",
        trainingPermissionDeclaration: "禁止",
        creationMethodDeclaration: "真人拍摄",
        humanEditLevelDeclaration: "轻度调色",
        authenticityClaimDeclaration: "由声明人确认",
        customRightsStatement: null
      },
      videoNotary: {},
      videoVisualWatermark: {}
    }],
    privacy: {
      excludesOriginalMedia: true,
      excludesWatermarkedMedia: true,
      excludesLocalMediaPaths: true,
      includedFields: ["file_name", "watermark_uid", "hashes"]
    },
    disclaimer: "本报告不构成法律意见、司法鉴定意见或诉讼结果承诺。"
  };
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function buildIntegrityChain(files) {
  const genesis = "HiddenShield-Report-Manifest-v2";
  let previousChainDigest = sha256(Buffer.from(genesis));
  const entries = files.map((file, index) => {
    const sequence = index + 1;
    const chainDigest = sha256(Buffer.from(
      `${sequence}\n${file.path}\n${file.bytes}\n${file.sha256}\n${previousChainDigest}`,
    ));
    const entry = {
      sequence,
      path: file.path,
      fileSha256: file.sha256,
      fileBytes: file.bytes,
      previousChainDigest,
      chainDigest
    };
    previousChainDigest = chainDigest;
    return entry;
  });
  return {
    algorithm: "sha256_chain_v1",
    genesis,
    entries,
    rootDigest: previousChainDigest
  };
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
