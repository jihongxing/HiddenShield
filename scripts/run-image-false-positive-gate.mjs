import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, extname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_IMAGE_RESOURCE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve(
  "artifacts/desktop-image-spatial-recovery-gate",
  runId,
);
const runtimeDir = resolve(
  "tmp-ui-qa/desktop-image-spatial-recovery",
  runId,
  "false-positive",
);
const summaryPath = join(evidenceDir, "false-positive-summary.json");
const readerExe = resolve(
  "watermark-core/target/release/desktop_image_read_qa.exe",
);
const sourceRoots = [
  "C:\\Windows\\Web\\Wallpaper",
  "C:\\Windows\\Web\\Screen",
];
const variants = [
  {
    name: "png-1920",
    extension: "png",
    codecArgs: ["-c:v", "png"],
  },
  {
    name: "jpeg-qscale-8",
    extension: "jpg",
    codecArgs: ["-c:v", "mjpeg", "-q:v", "8", "-pix_fmt", "yuvj444p"],
  },
  {
    name: "webp-q60",
    extension: "webp",
    codecArgs: ["-c:v", "libwebp", "-q:v", "60", "-compression_level", "4"],
  },
];

mkdirSync(evidenceDir, { recursive: true });
mkdirSync(runtimeDir, { recursive: true });

const summary = {
  schemaVersion: "desktop_image_false_positive_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  sourceRoots,
  minimumSamples: 100,
  samples: [],
};

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  buildReader();
  assert(existsSync(readerExe), `Core reader not found: ${readerExe}`);
  const sources = sourceRoots
    .flatMap((sourceRoot) => walkImages(sourceRoot))
    .sort((left, right) => left.localeCompare(right));
  assert(sources.length >= 34, `Expected at least 34 Windows photo sources, got ${sources.length}.`);

  for (const [sourceIndex, sourcePath] of sources.entries()) {
    for (const variant of variants) {
      const outputPath = join(
        runtimeDir,
        `${String(sourceIndex).padStart(2, "0")}-${basename(sourcePath, extname(sourcePath))}-${variant.name}.${variant.extension}`,
      );
      generateVariant(sourcePath, outputPath, variant.codecArgs);
      const startedAt = Date.now();
      const result = spawnSync(readerExe, [outputPath], {
        cwd: root,
        encoding: "utf8",
        timeout: 2 * 60_000,
        windowsHide: true,
      });
      const falsePositive = result.status === 0;
      summary.samples.push({
        sourcePath,
        sourceSha256: sha256(sourcePath),
        sourceBytes: statSync(sourcePath).size,
        variant: variant.name,
        format: variant.extension,
        path: relative(root, outputPath),
        sha256: sha256(outputPath),
        bytes: statSync(outputPath).size,
        elapsedMs: Date.now() - startedAt,
        exitCode: result.status,
        falsePositive,
        stdoutTail: tail(result.stdout),
        stderrTail: tail(result.stderr),
      });
      writeSummary();
      rmSync(outputPath, { force: true });
      if (falsePositive) {
        throw new Error(`False positive detected for ${sourcePath} (${variant.name}).`);
      }
    }
  }

  summary.sampleCount = summary.samples.length;
  summary.sourceCount = sources.length;
  summary.falsePositiveCount = summary.samples.filter((sample) => sample.falsePositive).length;
  summary.maximumElapsedMs = Math.max(...summary.samples.map((sample) => sample.elapsedMs));
  summary.averageElapsedMs = Math.round(
    summary.samples.reduce((sum, sample) => sum + sample.elapsedMs, 0) /
      summary.samples.length,
  );
  assert(summary.sampleCount >= summary.minimumSamples, "False-positive sample count is too low.");
  assert(summary.falsePositiveCount === 0, "False-positive Gate detected a watermark.");
  summary.status = "passed";
  summary.completedAt = new Date().toISOString();
  writeSummary();
  console.log(JSON.stringify(summary, null, 2));
} catch (error) {
  summary.status = "failed";
  summary.completedAt = new Date().toISOString();
  summary.error = String(error?.stack ?? error);
  writeSummary();
  console.error(summary.error);
  process.exitCode = 1;
} finally {
  rmSync(runtimeDir, { recursive: true, force: true });
}

function buildReader() {
  const result = spawnSync(
    "cargo",
    [
      "build",
      "--release",
      "--manifest-path",
      "watermark-core/Cargo.toml",
      "--bin",
      "desktop_image_read_qa",
    ],
    {
      cwd: root,
      encoding: "utf8",
      timeout: 10 * 60_000,
      windowsHide: true,
    },
  );
  assert(
    result.status === 0,
    `Failed to build core reader: ${result.stderr || result.stdout || result.error}`,
  );
}

function walkImages(directory) {
  if (!existsSync(directory)) return [];
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkImages(path));
    } else if ([".jpg", ".jpeg", ".png", ".webp"].includes(extname(entry.name).toLowerCase())) {
      files.push(path);
    }
  }
  return files;
}

function generateVariant(sourcePath, outputPath, codecArgs) {
  const result = spawnSync(
    "ffmpeg.exe",
    [
      "-y",
      "-hide_banner",
      "-loglevel",
      "error",
      "-i",
      sourcePath,
      "-vf",
      "scale=1920:1080:force_original_aspect_ratio=decrease",
      "-frames:v",
      "1",
      ...codecArgs,
      outputPath,
    ],
    {
      cwd: root,
      encoding: "utf8",
      timeout: 2 * 60_000,
      windowsHide: true,
    },
  );
  assert(
    result.status === 0 && existsSync(outputPath),
    `Failed to generate ${outputPath}: ${result.stderr || result.stdout || result.error}`,
  );
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
}

function tail(value, maximum = 1200) {
  const text = String(value ?? "");
  return text.length > maximum ? text.slice(-maximum) : text;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
