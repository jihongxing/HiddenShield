import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline";

const repoRoot = path.resolve(import.meta.dirname, "..");
const resourceDir = path.join(repoRoot, "src-tauri", "resources", "report-pdf");
const fixtureRoot = path.join(
  repoRoot,
  "mobile_app",
  "test",
  "fixtures",
  "report_bundles_r3",
);
const documents = buildDocuments();

await rm(fixtureRoot, { recursive: true, force: true });
await mkdir(fixtureRoot, { recursive: true });

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
const index = {
  schemaVersion: 1,
  generatedBy: "HiddenShield desktop Chromium worker",
  workerLaunchMs: ready.launchMs,
  fixtures: []
};

for (let indexValue = 0; indexValue < documents.length; indexValue += 1) {
  const fixture = documents[indexValue];
  const fixtureDir = path.join(fixtureRoot, fixture.directory);
  const pdfPath = path.join(fixtureDir, "report.pdf");
  const jsonPath = path.join(fixtureDir, "report.json");
  const manifestPath = path.join(fixtureDir, "manifest.json");
  await mkdir(fixtureDir, { recursive: true });
  const reportJson = Buffer.from(JSON.stringify(fixture.document, null, 2));
  await writeFile(jsonPath, reportJson);

  worker.stdin.write(`${JSON.stringify({
    type: "render",
    requestId: indexValue + 1,
    document: fixture.document,
    outputPath: pdfPath
  })}\n`);
  const result = await nextMessage(iterator, "result");
  assert(result.ok === true, result.error ?? `${fixture.directory} render failed`);
  const files = [
    {
      path: "report.pdf",
      mediaType: "application/pdf",
      bytes: result.bytes,
      sha256: result.sha256
    },
    {
      path: "report.json",
      mediaType: "application/json",
      bytes: reportJson.length,
      sha256: sha256(reportJson)
    }
  ];
  const record = fixture.document.records[0];
  const manifest = {
    schemaVersion: 2,
    reportId: fixture.document.reportId,
    reportType: fixture.document.reportType,
    generatedAt: fixture.document.exportedAt,
    sourceSchemaVersion: fixture.document.schemaVersion,
    bundle: {
      sourceKey: sha256(Buffer.from(`${fixture.document.reportType}|${record.recordId}`)),
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
      paginationStable: result.pageOverflow.every((page) => page.overflow === false)
    },
    files,
    integrity: buildIntegrityChain(files),
    signature: {
      status: "not_signed",
      profile: null,
      signerKeyId: null,
      certificateChainStatus: "not_evaluated",
      revocationStatus: "not_applicable",
      signedAt: null,
      note: "R3 cross-end fixture is not signed."
    },
    trustedTime: {
      status: "not_verified",
      packageTimestampPresent: false,
      recordMaterialTokenPresent: record.trustedTime.tsaTokenPresent === true,
      note: "记录级 TSA 材料状态不等于报告包已获得可信时间戳。"
    },
    verification: {
      offlineMode: "sha256_chain_v1",
      onlineStatus: "not_deployed",
      qrStatus: "not_issued",
      onlineVerificationUrl: null
    }
  };
  await writeFile(manifestPath, Buffer.from(JSON.stringify(manifest, null, 2)));
  index.fixtures.push({
    directory: fixture.directory,
    mediaKind: fixture.mediaKind,
    reportId: fixture.document.reportId,
    recordId: record.recordId,
    watermarkUid: record.watermarkUid,
    generationMs: result.generationMs,
    pageCount: result.pageCount,
    pdfBytes: result.bytes,
    pdfSha256: result.sha256
  });
}

await writeFile(
  path.join(fixtureRoot, "fixture-index.json"),
  Buffer.from(JSON.stringify(index, null, 2)),
);
worker.stdin.write('{"type":"shutdown"}\n');
worker.stdin.end();
await new Promise((resolve, reject) => {
  worker.once("exit", (code) => code === 0 ? resolve() : reject(new Error(`worker exited ${code}`)));
});
console.log(JSON.stringify(index, null, 2));

function buildDocuments() {
  return [
    {
      directory: "image",
      mediaKind: "image",
      document: buildDocument({
        reportId: "hsr-r3-image-desktop",
        recordId: 301,
        fileName: "城市晨雾摄影.png",
        watermarkUid: "HS-V3-IMG-R3-000301",
        resolution: "4032 × 3024",
        durationSecs: 0,
        originalHash: "8af42e0d2cd1d75b0fe72ae82cf5e5ed88f02a2a50ce08f91376a4ce",
        protectedHash: "d2977a4fe51babc978309ba57fbac02a632bf55d29f0a1b88dc07259",
        videoNotary: {}
      })
    },
    {
      directory: "audio",
      mediaKind: "audio",
      document: buildDocument({
        reportId: "hsr-r3-audio-desktop",
        recordId: 302,
        fileName: "独立音乐片段.wav",
        watermarkUid: "HS-V3-AUD-R3-000302",
        resolution: "48 kHz · 24 bit · Stereo",
        durationSecs: 48.25,
        originalHash: "1af42e0d2cd1d75b0fe72ae82cf5e5ed88f02a2a50ce08f91376a4ce",
        protectedHash: "e2977a4fe51babc978309ba57fbac02a632bf55d29f0a1b88dc07259",
        videoNotary: {}
      })
    },
    {
      directory: "l2-video",
      mediaKind: "l2_video",
      document: buildDocument({
        reportId: "hsr-r3-l2-video-desktop",
        recordId: 303,
        fileName: "品牌短片_L2.mp4",
        watermarkUid: "HS-V3-VID-R3-000303",
        resolution: "1920 × 1080 · 30 fps",
        durationSecs: 32.8,
        originalHash: "2af42e0d2cd1d75b0fe72ae82cf5e5ed88f02a2a50ce08f91376a4ce",
        protectedHash: "f2977a4fe51babc978309ba57fbac02a632bf55d29f0a1b88dc07259",
        videoNotary: {
          notaryId: "vfn_r3_000303",
          notaryAt: "2026-07-14T10:02:00+08:00",
          receiptSignature: "fixture-receipt-signature",
          usageLedgerId: "usage_r3_000303",
          fingerprintRoot: "sha256:l2-fingerprint-root-r3",
          bundleSha256: "sha256:l2-bundle-r3",
          bundleBytes: 8192,
          bundleSceneCount: 8,
          bundleElapsedMs: 1420,
          frameSamplePolicy: "8 evenly spaced frames"
        }
      })
    }
  ];
}

function buildDocument(input) {
  return {
    schemaVersion: 2,
    reportId: input.reportId,
    reportType: "formal_report",
    exportedAt: "2026-07-14T10:10:00+08:00",
    appVersion: "0.1.0",
    records: [{
      recordId: input.recordId,
      fileName: input.fileName,
      watermarkUid: input.watermarkUid,
      creatorDisplayName: "HiddenShield R3 跨端测试",
      originalHash: input.originalHash,
      resolution: input.resolution,
      durationSecs: input.durationSecs,
      createdAt: "2026-07-14T10:00:00+08:00",
      revision: 1,
      parentWatermarkUid: null,
      rewriteReason: null,
      writeVerificationStatus: "verified",
      writeVerificationMessage: "桌面写后读取验证通过",
      writeVerificationAt: "2026-07-14T10:01:00+08:00",
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
        name: `${input.fileName}.protected`,
        hash: input.protectedHash,
        outputStrategy: "minimal_required_change"
      },
      trustedTime: {
        networkTime: "2026-07-14T10:01:30+08:00",
        tsaSource: "fixture-tsa",
        tsaTokenPresent: true
      },
      rightsDeclaration: {
        workSourceDeclaration: "original",
        trainingPermissionDeclaration: "prohibited",
        creationMethodDeclaration: "human_created",
        humanEditLevelDeclaration: "light_edit",
        authenticityClaimDeclaration: "declared",
        customRightsStatement: null
      },
      videoNotary: input.videoNotary,
      videoVisualWatermark: {}
    }],
    privacy: {
      excludesOriginalMedia: true,
      excludesWatermarkedMedia: true,
      excludesLocalMediaPaths: true,
      includedFields: [
        "file_name",
        "watermark_uid",
        "revision",
        "hashes",
        "verification_status",
        "payload_registry",
        "trusted_time_status",
        "video_notary_receipt"
      ]
    },
    disclaimer: "本报告不构成法律意见、司法鉴定意见或诉讼结果承诺。"
  };
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

async function nextMessage(iterator, expectedType) {
  const next = await iterator.next();
  if (next.done) throw new Error(`worker closed before ${expectedType}`);
  const message = JSON.parse(next.value);
  assert(message.type === expectedType, `expected ${expectedType}, got ${message.type}`);
  return message;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
