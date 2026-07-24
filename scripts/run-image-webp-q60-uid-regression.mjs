import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_IMAGE_WEBP_Q60_UID_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const sourcePath = resolve(
  process.env.HIDDENSHIELD_IMAGE_WEBP_Q60_SOURCE ??
    "C:/Windows/Web/Wallpaper/ThemeC/img29.jpg",
);
const outputDir = resolve("tmp-ui-qa/image-webp-q60-uid-regression", runId);
const evidenceDir = resolve("artifacts/image-webp-q60-uid-regression", runId);
const summaryPath = join(evidenceDir, "summary.json");
const writer = resolve("watermark-core/target/release/desktop_image_write_qa.exe");
const reader = resolve("watermark-core/target/release/desktop_image_read_qa.exe");
const diagnostic = resolve(
  "watermark-core/target/release/desktop_image_spatial_diagnose.exe",
);
const uids = [
  "HS-9214D504-63C9EFDF-5376CA9B-9A81A854",
  "HS-085341C0-E0B3FF00-89F50E28-66070263",
  "HS-D5A73850-76EE8547-1AC43822-9648BCF3",
];

const summary = {
  schemaVersion: "image_webp_q60_uid_regression_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  investigation: {
    incident: "RC-MEDIA-001",
    rootCause:
      "The exact reader returned the first checksum-valid packet before evaluating the 25-packet consensus. WebP q60 flipped UID bits 73 and 95 in packet variant 0 while preserving the legacy 8-bit checksum.",
    sharedCoreFix:
      "Evaluate direct and soft-corrected multi-packet consensus before accepting an individual packet.",
    productDecision:
      "Keep WebP quality 60 in the desktop recovery promise; do not narrow the boundary to quality 75.",
  },
  source: {
    path: relative(root, sourcePath),
    sha256: null,
    bytes: null,
  },
  cases: [],
};

mkdirSync(outputDir, { recursive: true });
mkdirSync(evidenceDir, { recursive: true });

try {
  assert(existsSync(sourcePath), `Fixed source photo not found: ${sourcePath}`);
  buildQaBinaries();
  summary.source.sha256 = sha256(sourcePath);
  summary.source.bytes = statSync(sourcePath).size;

  for (const uid of uids) {
    const label = uid.replaceAll("-", "").slice(-12);
    const protectedPath = join(outputDir, `${label}-protected.png`);
    const webpPath = join(outputDir, `${label}-webp-q60.webp`);
    const startedAt = Date.now();
    const write = run(writer, [sourcePath, protectedPath, uid], 180_000);
    const protectedRead = parseReaderOutput(run(reader, [protectedPath], 120_000).stdout);
    run(
      "python",
      [
        "scripts/create-image-transform-fixture.py",
        protectedPath,
        webpPath,
        "webp",
        "60",
      ],
      180_000,
    );
    const transformedRead = parseReaderOutput(run(reader, [webpPath], 120_000).stdout);
    const spatialDiagnostic = parseReaderOutput(
      run(diagnostic, [webpPath, uid], 120_000).stdout,
    );
    const passed =
      protectedRead.watermarkUid === uid &&
      transformedRead.watermarkUid === uid;
    summary.cases.push({
      uid,
      status: passed ? "passed" : "failed",
      protectedPath: relative(root, protectedPath),
      protectedSha256: sha256(protectedPath),
      protectedReadUid: protectedRead.watermarkUid,
      transformedPath: relative(root, webpPath),
      transformedSha256: sha256(webpPath),
      transformedBytes: statSync(webpPath).size,
      transformedReadUid: transformedRead.watermarkUid,
      differingUidBits: differingUidBits(uid, transformedRead.watermarkUid),
      spatialDiagnostic,
      writerOutput: write.stdout.trim(),
      elapsedMs: Date.now() - startedAt,
    });
    writeSummary();
  }

  summary.failed = summary.cases.filter((item) => item.status === "failed").length;
  summary.passed = summary.cases.length - summary.failed;
  summary.status = summary.failed === 0 ? "passed" : "failed";
  summary.completedAt = new Date().toISOString();
  writeSummary();
  console.log(JSON.stringify({
    status: summary.status,
    passed: summary.passed,
    failed: summary.failed,
    cases: summary.cases.map((item) => ({
      uid: item.uid,
      transformedReadUid: item.transformedReadUid,
      differingUidBits: item.differingUidBits,
      status: item.status,
    })),
    summaryPath,
  }, null, 2));
  if (summary.failed > 0) process.exitCode = 1;
} catch (error) {
  summary.status = "error";
  summary.completedAt = new Date().toISOString();
  summary.error = String(error?.stack ?? error);
  writeSummary();
  throw error;
}

function buildQaBinaries() {
  run(
    "cargo",
    [
      "build",
      "--release",
      "--manifest-path",
      "watermark-core/Cargo.toml",
      "--bin",
      "desktop_image_write_qa",
      "--bin",
      "desktop_image_read_qa",
      "--bin",
      "desktop_image_spatial_diagnose",
    ],
    10 * 60_000,
  );
}

function run(command, args, timeout) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    timeout,
  });
  assert(
    result.status === 0,
    `${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
  );
  return result;
}

function parseReaderOutput(value) {
  return JSON.parse(value.trim());
}

function differingUidBits(expected, actual) {
  const left = uidBytes(expected);
  const right = uidBytes(actual);
  const differences = [];
  for (let byteIndex = 0; byteIndex < left.length; byteIndex += 1) {
    const delta = left[byteIndex] ^ right[byteIndex];
    for (let bit = 0; bit < 8; bit += 1) {
      if (((delta >> (7 - bit)) & 1) === 1) {
        differences.push(byteIndex * 8 + bit);
      }
    }
  }
  return differences;
}

function uidBytes(uid) {
  return Buffer.from(uid.replace(/^HS-/, "").replaceAll("-", ""), "hex");
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
