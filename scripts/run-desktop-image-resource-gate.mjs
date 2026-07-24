import { createHash } from "node:crypto";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { deflateSync } from "node:zlib";
import {
  existsSync,
  closeSync,
  copyFileSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  statfsSync,
  writeSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";

const root = process.cwd();
const spatialComprehensive = process.argv.includes("--spatial-comprehensive");
const spatialRecoveryOnly =
  process.argv.includes("--spatial-recovery") || spatialComprehensive;
const spatialVisualOnly = process.argv.includes("--spatial-visual-only");
const spatialResourceOnly = process.argv.includes("--spatial-resource-only");
const runId =
  process.env.HIDDENSHIELD_IMAGE_RESOURCE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve(
  spatialRecoveryOnly
    ? "artifacts/desktop-image-spatial-recovery-gate"
    : "artifacts/desktop-image-resource-gate",
  runId,
);
const runtimeDir = resolve(
  spatialRecoveryOnly
    ? "tmp-ui-qa/desktop-image-spatial-recovery"
    : "tmp-ui-qa/desktop-image-resource",
  runId,
);
const fixtureDir = join(runtimeDir, "fixtures");
const outputDir = join(runtimeDir, "outputs");
const installedExe = resolve(
  process.env.HIDDENSHIELD_INSTALLED_EXE ??
    "tmp-ui-qa/desktop-installer-self-contained/20260722-image-resource/installed/hidden_shield.exe",
);
const ffmpeg = process.env.FFMPEG_PATH ?? "ffmpeg.exe";
const ffprobe = process.env.FFPROBE_PATH ?? "ffprobe.exe";
const summaryPath = join(evidenceDir, "summary.json");
const debugPort = 10_100 + Math.floor(Math.random() * 300);
const rejectionOnly = process.argv.includes("--rejection-only");
const enrichOnly = process.argv.includes("--enrich-record-times");
const fileSizeOnly = process.argv.includes("--file-size-only");
const smokeOnly = process.argv.includes("--smoke-only");
const resumeExisting = rejectionOnly || enrichOnly || fileSizeOnly;

const ordinarySizes = [
  { label: "landscape", width: 1920, height: 1080 },
  { label: "portrait", width: 1080, height: 1920 },
  { label: "square", width: 2048, height: 2048 },
];
const formats = ["png", "jpeg", "webp"];
const fixtures = [
  ...formats.flatMap((format) =>
    ordinarySizes.map((size) => ({ ...size, format, tier: "ordinary" }))),
  ...formats.map((format) => ({
    format,
    tier: "near_100mp",
    label: "near-100mp",
    width: 9992,
    height: 10000,
  })),
];

const existingSummary = resumeExisting && existsSync(summaryPath)
  ? JSON.parse(readFileSync(summaryPath, "utf8"))
  : null;
const summary = existingSummary
  ? {
      ...existingSummary,
      status: "running",
      completedAt: undefined,
      error: undefined,
      rejectionChecks: rejectionOnly
        ? existingSummary.rejectionChecks.filter((row) => row.name !== "png-over-100mp-10001x10000.png")
        : existingSummary.rejectionChecks,
      fixtures: fileSizeOnly
        ? existingSummary.fixtures.filter((row) => row.tier !== "near_512mib")
        : existingSummary.fixtures,
    }
  : {
  schemaVersion: spatialRecoveryOnly
    ? "desktop_image_spatial_recovery_gate_v1"
    : "desktop_image_resource_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  product: {
    endpoint: "installed-desktop",
    installedExecutable: relative(root, installedExe),
    inputFormats: formats,
    outputFormat: "png",
    maximumPixels: 100_000_000,
    maximumBytes: 512 * 1024 * 1024,
    spatialRecoveryLayout: spatialRecoveryOnly ? "spatial-recovery-v1" : null,
  },
  fixtures: [],
  rejectionChecks: [],
  checks: {},
  };

mkdirSync(evidenceDir, { recursive: true });
mkdirSync(fixtureDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  assert(existsSync(installedExe), `Installed executable not found: ${installedExe}`);
  const child = spawn(installedExe, [], {
    cwd: dirname(installedExe),
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    },
    stdio: "ignore",
    windowsHide: true,
  });
  let previousPreferences = null;
  try {
    const target = await waitForTarget();
    const webSocketUrl = target.webSocketDebuggerUrl;
    summary.checks.installedUiLoaded = {
      passed: target.url === "http://tauri.localhost/" && target.title === "HiddenShield",
      details: { url: target.url, title: target.title },
    };
    assert(summary.checks.installedUiLoaded.passed, "Installed UI did not load.");
    const identity = await cdpInvoke(webSocketUrl, "get_identity_status");
    if (!identity?.initialized) {
      await cdpInvoke(webSocketUrl, "setup_identity", {
        creatorInput: `HiddenShield image resource QA ${runId}`,
      });
    }
    previousPreferences = await cdpInvoke(webSocketUrl, "get_preferences");
    const preferences = await cdpInvoke(webSocketUrl, "save_preferences", {
      input: { defaultOutputDir: outputDir, onboardingCompleted: true },
    });
    assert(preferences?.defaultOutputDirWritable === true, "QA output directory is not writable.");

    if (spatialRecoveryOnly) {
      await runSpatialRecoveryGate(webSocketUrl, child.pid);
    } else if (enrichOnly) {
      const records = await cdpInvoke(webSocketUrl, "list_vault_records");
      for (const row of summary.fixtures) {
        const record = [...(records ?? [])]
          .filter((candidate) => candidate.fileName === row.name)
          .sort((left, right) => String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")))[0];
        assert(record, `Missing vault record for ${row.name}`);
        row.productProcessTimeMs = record.processTimeMs;
      }
    } else if (fileSizeOnly) {
      const spec = {
        format: "png",
        tier: "near_512mib",
        label: "near-512mib",
        width: 1920,
        height: 1080,
      };
      const inputPath = generateFixture(spec);
      padPngToExactSize(inputPath, 512 * 1024 * 1024);
      const row = await processFixture(webSocketUrl, child.pid, inputPath, spec);
      summary.fixtures.push(row);
      writeSummary();
      cleanupFixtureFiles(inputPath, row.outputAbsolutePath);
      delete row.outputAbsolutePath;
      assert(row.status === "passed", `${basename(inputPath)} failed.`);
    } else if (!rejectionOnly) {
      const selectedFixtures = smokeOnly
        ? fixtures.filter((spec) => spec.tier === "ordinary" && spec.label === "landscape")
        : fixtures;
      for (const spec of selectedFixtures) {
        console.log(
          `[image-resource] ${spec.format} ${spec.label} ${spec.width}x${spec.height} generating`,
        );
        const inputPath = generateFixture(spec);
        const row = await processFixture(webSocketUrl, child.pid, inputPath, spec);
        summary.fixtures.push(row);
        writeSummary();
        cleanupFixtureFiles(inputPath, row.outputAbsolutePath);
        delete row.outputAbsolutePath;
        assert(row.status === "passed", `${basename(inputPath)} failed.`);
        console.log(
          `[image-resource] ${basename(inputPath)} passed in ${row.elapsedMs} ms, peak ${row.resources.peakWorkingSetBytes} bytes`,
        );
      }
    }

    if (fileSizeOnly) {
      const rejection = await runOverFileSizeRejection(webSocketUrl);
      summary.rejectionChecks.push(rejection);
      assert(rejection.passed, "512 MiB + 1 byte rejection failed.");
    } else if (!enrichOnly) {
      const rejection = await runOverPixelRejection(webSocketUrl);
      summary.rejectionChecks.push(rejection);
      assert(rejection.passed, "100 MP + 1 rejection failed.");
    }
  } finally {
    if (previousPreferences) {
      try {
        const targets = await fetch(`http://127.0.0.1:${debugPort}/json/list`).then((response) =>
          response.json());
        const target = targets.find((candidate) => candidate.type === "page");
        if (target?.webSocketDebuggerUrl) {
          await cdpInvoke(target.webSocketDebuggerUrl, "save_preferences", {
            input: {
              defaultOutputDir: previousPreferences.defaultOutputDir ?? "",
              onboardingCompleted: previousPreferences.onboardingCompleted,
            },
          });
        }
      } catch {}
    }
    if (child.exitCode === null) child.kill();
  }

  summary.status = "passed";
  summary.completedAt = new Date().toISOString();
  writeSummary();
  console.log(`Desktop image resource Gate passed: ${summaryPath}`);
} catch (error) {
  summary.status = "failed";
  summary.completedAt = new Date().toISOString();
  summary.error = String(error?.stack ?? error);
  writeSummary();
  console.error(`Desktop image resource Gate failed: ${summaryPath}`);
  throw error;
}

function generateFixture(spec) {
  const extension = spec.format === "jpeg" ? "jpg" : spec.format;
  const output = join(
    fixtureDir,
    `${spec.format}-${spec.label}-${spec.width}x${spec.height}.${extension}`,
  );
  const codecArgs = {
    png: ["-c:v", "png", "-pix_fmt", "rgb24"],
    jpeg: ["-c:v", "mjpeg", "-q:v", "2", "-pix_fmt", "yuvj444p"],
    webp: ["-c:v", "libwebp", "-q:v", "90", "-compression_level", "4"],
  }[spec.format];
  run(ffmpeg, [
    "-y",
    "-f", "lavfi",
    "-i", `testsrc2=size=${spec.width}x${spec.height}:rate=1`,
    "-frames:v", "1",
    ...codecArgs,
    output,
  ], 20 * 60_000);
  return output;
}

async function runSpatialRecoveryGate(webSocketUrl, processId) {
  summary.checks.visualThresholds = {
    passed: true,
    details: {
      minimumPsnr: 38,
      minimumSsim: 0.99,
      basis: "Existing HiddenShield full image quality gate thresholds.",
    },
  };
  const realPhotos = [
    {
      label: "windows-mi-default",
      sourcePath: "C:\\Windows\\Web\\Wallpaper\\MI\\Default.jpg",
    },
    {
      label: "windows-mi-sunset",
      sourcePath: "C:\\Windows\\Web\\Wallpaper\\MI\\Sunset.jpg",
    },
    {
      label: "windows-theme-c-img29",
      sourcePath: "C:\\Windows\\Web\\Wallpaper\\ThemeC\\img29.jpg",
    },
  ];
  if (!spatialResourceOnly) {
    for (const photo of realPhotos) {
      assert(existsSync(photo.sourcePath), `Real photo fixture not found: ${photo.sourcePath}`);
      const inputPath = join(fixtureDir, `${photo.label}.jpg`);
      copyFileSync(photo.sourcePath, inputPath);
      const input = probeImage(inputPath);
      const spec = {
        format: "jpeg",
        tier: "real_photo_visual",
        label: photo.label,
        width: input.width,
        height: input.height,
      };
      console.log(`[image-spatial] ${photo.label} installed write/read starting`);
      const row = await processFixture(webSocketUrl, processId, inputPath, spec);
      row.fixtureOrigin = {
        kind: "windows_bundled_photography",
        sourcePath: photo.sourcePath,
        sourceSha256: sha256(photo.sourcePath),
      };
      row.visualQuality = measureVisualQuality(inputPath, row.outputAbsolutePath);
      assert(
        row.visualQuality.psnr >= 38,
        `${photo.label} PSNR ${row.visualQuality.psnr} is below 38 dB.`,
      );
      assert(
        row.visualQuality.ssim >= 0.99,
        `${photo.label} SSIM ${row.visualQuality.ssim} is below 0.990.`,
      );
      row.cropRecovery = await runInstalledCropRecovery(
        webSocketUrl,
        row.outputAbsolutePath,
        row.readOnlyVerification.watermarkUid,
        photo.label,
      );
      assert(row.cropRecovery.exactGridPassed, `${photo.label} exact-grid crop Gate failed.`);
      assert(row.cropRecovery.slidingPassed, `${photo.label} sliding crop Gate failed.`);
      if (spatialComprehensive) {
        row.transformRecovery = await runInstalledTransformRecovery(
          webSocketUrl,
          row.outputAbsolutePath,
          row.readOnlyVerification.watermarkUid,
          photo.label,
        );
        assert(
          row.transformRecovery.allPassed,
          `${photo.label} transform recovery Gate failed.`,
        );
      }
      summary.fixtures.push(row);
      writeSummary();
      cleanupFixtureFiles(inputPath, row.outputAbsolutePath);
      row.cleanedUpAfterGate = true;
      delete row.outputAbsolutePath;
      writeSummary();
    }
  }

  if (!spatialVisualOnly) {
    const resourceSpec = {
      format: "png",
      tier: "near_100mp_spatial_recovery",
      label: "near-100mp",
      width: 9992,
      height: 10000,
    };
    console.log("[image-spatial] near-100mp installed resource Gate starting");
    const resourceInput = generateFixture(resourceSpec);
    const resourceRow = await processFixture(
      webSocketUrl,
      processId,
      resourceInput,
      resourceSpec,
    );
    resourceRow.spatialRecoveryReadVerified =
      resourceRow.independentCoreRead.passed &&
      resourceRow.readOnlyVerification.matched === true;
    assert(resourceRow.spatialRecoveryReadVerified, "Near-100MP spatial recovery read failed.");
    summary.fixtures.push(resourceRow);
    writeSummary();
    cleanupFixtureFiles(resourceInput, resourceRow.outputAbsolutePath);
    resourceRow.cleanedUpAfterGate = true;
    delete resourceRow.outputAbsolutePath;
    writeSummary();
  }
}

async function runInstalledTransformRecovery(
  webSocketUrl,
  protectedPath,
  expectedUid,
  label,
) {
  const transformDir = join(runtimeDir, "transforms", label);
  mkdirSync(transformDir, { recursive: true });
  const cases = [
    { name: "rotate-90", extension: "png", operation: "rotate", value: "90" },
    { name: "rotate-180", extension: "png", operation: "rotate", value: "180" },
    { name: "rotate-270", extension: "png", operation: "rotate", value: "270" },
    { name: "scale-85", extension: "png", operation: "scale", value: "0.85" },
    { name: "jpeg-q75", extension: "jpg", operation: "jpeg", value: "75" },
    { name: "jpeg-q60", extension: "jpg", operation: "jpeg", value: "60" },
    { name: "webp-q75", extension: "webp", operation: "webp", value: "75" },
    { name: "webp-q60", extension: "webp", operation: "webp", value: "60" },
  ];
  const results = [];
  for (const transform of cases) {
    const path = join(transformDir, `${transform.name}.${transform.extension}`);
    run("python", [
      "scripts/create-image-transform-fixture.py",
      protectedPath,
      path,
      transform.operation,
      transform.value,
    ], 5 * 60_000);
    const startedAt = Date.now();
    const readonly = await cdpInvoke(
      webSocketUrl,
      "verify_suspect_readonly_candidate",
      { path },
    );
    const coreRead = runCoreRead(path);
    const passed =
      readonly?.matched === true &&
      readonly?.watermarkUid === expectedUid &&
      coreRead.passed;
    results.push({
      ...transform,
      path: relative(root, path),
      sha256: sha256(path),
      bytes: statSync(path).size,
      passed,
      watermarkUid: readonly?.watermarkUid ?? null,
      reasonCode: readonly?.reasonCode ?? null,
      independentCoreRead: coreRead,
      elapsedMs: Date.now() - startedAt,
    });
    rmSync(path, { force: true });
  }
  return {
    expectedUid,
    caseCount: results.length,
    allPassed: results.every((result) => result.passed),
    cases: results,
  };
}

async function runInstalledCropRecovery(
  webSocketUrl,
  protectedPath,
  expectedUid,
  label,
) {
  const image = probeImage(protectedPath);
  const cropDir = join(runtimeDir, "crops", label);
  mkdirSync(cropDir, { recursive: true });
  const exactCrops = [];
  for (let row = 0; row < 4; row += 1) {
    for (let column = 0; column < 4; column += 1) {
      const x = Math.floor((column * image.width) / 4);
      const y = Math.floor((row * image.height) / 4);
      const right = Math.floor(((column + 1) * image.width) / 4);
      const bottom = Math.floor(((row + 1) * image.height) / 4);
      exactCrops.push({
        name: `grid-${row}-${column}`,
        x,
        y,
        width: right - x,
        height: bottom - y,
      });
    }
  }
  const cropWidth = Math.floor(image.width / 4);
  const cropHeight = Math.floor(image.height / 4);
  const maximumX = image.width - cropWidth;
  const maximumY = image.height - cropHeight;
  const xPositions = [
    0,
    1,
    Math.floor(maximumX / 3),
    Math.floor(maximumX / 2),
    maximumX - 1,
    maximumX,
  ];
  const yPositions = [
    0,
    1,
    Math.floor(maximumY / 3),
    Math.floor(maximumY / 2),
    maximumY - 1,
    maximumY,
  ];
  const slidingCrops = yPositions.flatMap((y, row) =>
    xPositions.map((x, column) => ({
      name: `sliding-${row}-${column}`,
      x,
      y,
      width: cropWidth,
      height: cropHeight,
    })),
  );
  const exactRows = [];
  for (const crop of exactCrops) {
    exactRows.push(
      await verifyInstalledCrop(webSocketUrl, protectedPath, expectedUid, cropDir, crop),
    );
  }
  const slidingRows = [];
  for (const crop of slidingCrops) {
    slidingRows.push(
      await verifyInstalledCrop(webSocketUrl, protectedPath, expectedUid, cropDir, crop),
    );
  }
  return {
    expectedUid,
    exactGridCount: exactRows.length,
    exactGridPassed: exactRows.every((row) => row.passed),
    exactGrid: exactRows,
    slidingCount: slidingRows.length,
    slidingPassed: slidingRows.every((row) => row.passed),
    sliding: slidingRows,
  };
}

async function verifyInstalledCrop(
  webSocketUrl,
  protectedPath,
  expectedUid,
  cropDir,
  crop,
) {
  const cropPath = join(cropDir, `${crop.name}.png`);
  run(ffmpeg, [
    "-y",
    "-i", protectedPath,
    "-vf", `crop=${crop.width}:${crop.height}:${crop.x}:${crop.y}`,
    "-frames:v", "1",
    "-c:v", "png",
    cropPath,
  ], 5 * 60_000);
  const startedAt = Date.now();
  const readonly = await cdpInvoke(
    webSocketUrl,
    "verify_suspect_readonly_candidate",
    { path: cropPath },
  );
  const passed =
    readonly?.matched === true && readonly?.watermarkUid === expectedUid;
  rmSync(cropPath, { force: true });
  return {
    ...crop,
    passed,
    watermarkUid: readonly?.watermarkUid ?? null,
    reasonCode: readonly?.reasonCode ?? null,
    elapsedMs: Date.now() - startedAt,
  };
}

function measureVisualQuality(sourcePath, protectedPath) {
  const psnrOutput = runCapture(ffmpeg, [
    "-hide_banner",
    "-i", protectedPath,
    "-i", sourcePath,
    "-lavfi", "[0:v][1:v]psnr",
    "-f", "null",
    "-",
  ], 10 * 60_000);
  const ssimOutput = runCapture(ffmpeg, [
    "-hide_banner",
    "-i", protectedPath,
    "-i", sourcePath,
    "-lavfi", "[0:v][1:v]ssim",
    "-f", "null",
    "-",
  ], 10 * 60_000);
  const psnrMatch = psnrOutput.match(/average:([0-9.]+)/);
  const ssimMatch = ssimOutput.match(/All:([0-9.]+)/);
  assert(psnrMatch, `Unable to parse PSNR for ${basename(sourcePath)}.`);
  assert(ssimMatch, `Unable to parse SSIM for ${basename(sourcePath)}.`);
  return {
    psnr: Number(psnrMatch[1]),
    minimumPsnr: 38,
    ssim: Number(ssimMatch[1]),
    minimumSsim: 0.99,
    passed: Number(psnrMatch[1]) >= 38 && Number(ssimMatch[1]) >= 0.99,
  };
}

function runCapture(command, args, timeout) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    timeout,
  });
  assert(
    result.status === 0,
    `${command} failed: ${result.stderr || result.stdout || result.error}`,
  );
  return `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
}

async function processFixture(webSocketUrl, processId, inputPath, spec) {
  const startedAt = Date.now();
  const input = probeImage(inputPath);
  const row = {
    name: basename(inputPath),
    tier: spec.tier,
    format: spec.format,
    inputPath: relative(root, inputPath),
    input,
    status: "running",
  };
  try {
    assert(input.width === spec.width && input.height === spec.height, "Generated dimensions differ.");
    const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path: inputPath });
    assert(sourceMeta?.fileType === "image", "Installed probe did not classify fixture as image.");
    assert(sourceMeta?.watermarkEligible === true, "Installed preflight rejected allowed image.");
    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const diskFreeBefore = diskFreeBytes(outputDir);
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath,
      platforms: ["douyin"],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    const runtime = await waitForPipeline(webSocketUrl, pipeline.pipelineId, processId);
    const records = await cdpInvoke(webSocketUrl, "list_vault_records");
    const record = [...(records ?? [])]
      .filter((candidate) => !priorIds.has(candidate.id) && candidate.fileName === basename(inputPath))
      .sort((left, right) => String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")))[0];
    assert(record, "No vault record found for installed image pipeline.");
    const outputPath = record.protectedCopyPath;
    assert(outputPath && existsSync(outputPath), "Installed image pipeline produced no protected copy.");
    const output = probeImage(outputPath);
    assert(output.codecName === "png", `Protected copy is not PNG: ${output.codecName}`);
    assert(
      output.width === input.width && output.height === input.height,
      "Protected copy dimensions changed.",
    );
    assert(record.writeVerificationStatus === "verified", "Write-after-read record is not verified.");
    const coreRead = runCoreRead(outputPath);
    assert(coreRead.passed, "Independent core image read failed.");
    const readonly = await cdpInvoke(
      webSocketUrl,
      "verify_suspect_readonly_candidate",
      { path: outputPath },
    );
    assert(readonly?.matched === true, "Installed read-only verification failed.");
    row.status = "passed";
    row.pipelineId = pipeline.pipelineId;
    row.outputPath = relative(root, outputPath);
    row.outputAbsolutePath = outputPath;
    row.output = output;
    row.inputSha256 = sha256(inputPath);
    row.outputSha256 = sha256(outputPath);
    row.writeAfterRead = {
      status: record.writeVerificationStatus,
      message: record.writeVerificationMessage,
    };
    row.productProcessTimeMs = record.processTimeMs;
    row.independentCoreRead = coreRead;
    row.readOnlyVerification = {
      matched: readonly.matched,
      watermarkUid: readonly.watermarkUid,
      reasonCode: readonly.reasonCode,
    };
    row.resources = {
      peakWorkingSetBytes: runtime.peakWorkingSetBytes,
      diskFreeBeforeBytes: diskFreeBefore,
      diskFreeAfterBytes: diskFreeBytes(outputDir),
      inputBytes: statSync(inputPath).size,
      outputBytes: statSync(outputPath).size,
    };
    row.elapsedMs = Date.now() - startedAt;
    return row;
  } catch (error) {
    row.status = "failed";
    row.error = String(error?.stack ?? error);
    row.elapsedMs = Date.now() - startedAt;
    return row;
  }
}

async function runOverPixelRejection(webSocketUrl) {
  const path = join(fixtureDir, "png-over-100mp-10001x10000.png");
  writeMinimalPng(path, 10001, 10000);
  try {
    const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path });
    return {
      name: basename(path),
      width: sourceMeta.width,
      height: sourceMeta.height,
      pixels: sourceMeta.width * sourceMeta.height,
      watermarkEligible: sourceMeta.watermarkEligible,
      passed:
        sourceMeta.width === 10001 &&
        sourceMeta.height === 10000 &&
        sourceMeta.watermarkEligible === false,
    };
  } finally {
    rmSync(path, { force: true });
  }
}

async function runOverFileSizeRejection(webSocketUrl) {
  const spec = {
    format: "png",
    tier: "over_512mib",
    label: "over-512mib",
    width: 1920,
    height: 1080,
  };
  const path = generateFixture(spec);
  padPngToExactSize(path, 512 * 1024 * 1024 + 1);
  try {
    const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path });
    return {
      name: basename(path),
      bytes: statSync(path).size,
      maximumBytes: 512 * 1024 * 1024,
      watermarkEligible: sourceMeta.watermarkEligible,
      passed:
        statSync(path).size === 512 * 1024 * 1024 + 1 &&
        sourceMeta.watermarkEligible === false,
    };
  } finally {
    rmSync(path, { force: true });
  }
}

function writeMinimalPng(path, width, height) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 1;
  ihdr[9] = 0;
  const chunk = pngChunk("IHDR", ihdr);
  const rowBytes = Math.ceil(width / 8);
  const raw = Buffer.alloc((rowBytes + 1) * height);
  const imageData = pngChunk("IDAT", deflateSync(raw, { level: 9 }));
  const end = pngChunk("IEND", Buffer.alloc(0));
  writeFileSync(path, Buffer.concat([signature, chunk, imageData, end]));
}

function padPngToExactSize(path, targetBytes) {
  const bytes = readFileSync(path);
  const iendOffset = bytes.lastIndexOf(Buffer.from("IEND", "ascii")) - 4;
  assert(iendOffset >= 8, "PNG IEND chunk not found.");
  const prefix = bytes.subarray(0, iendOffset);
  const iend = bytes.subarray(iendOffset);
  const chunkOverhead = 12;
  const dataLength = targetBytes - prefix.length - chunkOverhead - iend.length;
  assert(dataLength >= 0 && dataLength <= 0xffffffff, "Requested PNG padding is invalid.");
  const temporaryPath = `${path}.padding`;
  const handle = openSync(temporaryPath, "w");
  try {
    writeSync(handle, prefix);
    const length = Buffer.alloc(4);
    length.writeUInt32BE(dataLength);
    const type = Buffer.from("hsQa", "ascii");
    writeSync(handle, length);
    writeSync(handle, type);
    const zeroChunk = Buffer.alloc(1024 * 1024);
    let remaining = dataLength;
    let crc = crc32Update(0xffffffff, type);
    while (remaining > 0) {
      const size = Math.min(remaining, zeroChunk.length);
      const slice = zeroChunk.subarray(0, size);
      writeSync(handle, slice);
      crc = crc32Update(crc, slice);
      remaining -= size;
    }
    const checksum = Buffer.alloc(4);
    checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0);
    writeSync(handle, checksum);
    writeSync(handle, iend);
  } finally {
    closeSync(handle);
  }
  rmSync(path, { force: true });
  renameSync(temporaryPath, path);
  assert(statSync(path).size === targetBytes, "Padded PNG size does not match target.");
}

function pngChunk(type, data) {
  const typeBytes = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])));
  return Buffer.concat([length, typeBytes, data, crc]);
}

function crc32(bytes) {
  return (crc32Update(0xffffffff, bytes) ^ 0xffffffff) >>> 0;
}

function crc32Update(initial, bytes) {
  let crc = initial;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return crc >>> 0;
}

function probeImage(path) {
  const raw = execFileSync(ffprobe, [
    "-v", "error",
    "-select_streams", "v:0",
    "-show_entries", "stream=codec_name,pix_fmt,width,height:format=size",
    "-of", "json",
    path,
  ], { cwd: root, encoding: "utf8" });
  const parsed = JSON.parse(raw);
  const stream = parsed.streams?.[0] ?? {};
  return {
    codecName: stream.codec_name ?? extname(path).slice(1),
    pixelFormat: stream.pix_fmt ?? null,
    width: Number(stream.width ?? 0),
    height: Number(stream.height ?? 0),
    pixels: Number(stream.width ?? 0) * Number(stream.height ?? 0),
    sizeBytes: Number(parsed.format?.size ?? statSync(path).size),
  };
}

function runCoreRead(path) {
  const result = spawnSync(
    "cargo",
    ["run", "--release", "--manifest-path", "watermark-core/Cargo.toml", "--bin", "desktop_image_read_qa", "--", path],
    { cwd: root, encoding: "utf8", timeout: 20 * 60_000 },
  );
  return {
    passed: result.status === 0,
    exitCode: result.status,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
  };
}

async function waitForPipeline(webSocketUrl, pipelineId, processId) {
  const deadline = Date.now() + 30 * 60_000;
  let peakWorkingSetBytes = 0;
  while (Date.now() < deadline) {
    peakWorkingSetBytes = Math.max(peakWorkingSetBytes, processWorkingSetBytes(processId));
    const active = await cdpInvoke(webSocketUrl, "check_active_pipelines");
    if (!active.includes(pipelineId)) return { peakWorkingSetBytes };
    await delay(750);
  }
  throw new Error(`Pipeline ${pipelineId} did not finish within 30 minutes.`);
}

function processWorkingSetBytes(processId) {
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-Command", `(Get-Process -Id ${processId} -ErrorAction SilentlyContinue).WorkingSet64`],
    { cwd: root, encoding: "utf8", timeout: 10_000 },
  );
  return Number(result.stdout.trim() || 0);
}

function diskFreeBytes(path) {
  const stats = statfsSync(path);
  return Number(stats.bavail) * Number(stats.bsize);
}

function cleanupFixtureFiles(inputPath, outputPath) {
  rmSync(inputPath, { force: true });
  if (outputPath) rmSync(outputPath, { force: true });
}

async function waitForTarget() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${debugPort}/json/list`);
      const targets = await response.json();
      const target = targets.find((candidate) => candidate.type === "page");
      if (target?.webSocketDebuggerUrl) {
        const state = JSON.parse(await evaluateCdp(
          target.webSocketDebuggerUrl,
          "JSON.stringify({ title: document.title, readyState: document.readyState, url: location.href })",
        ));
        if (
          state.title === "HiddenShield" &&
          state.readyState === "complete" &&
          ["http://tauri.localhost/", "tauri://localhost/"].includes(state.url)
        ) {
          return { ...target, ...state };
        }
      }
    } catch {}
    await delay(500);
  }
  throw new Error("Installed desktop target did not appear on the CDP port.");
}

async function cdpInvoke(webSocketUrl, command, args = undefined) {
  const expression = `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${args === undefined ? "undefined" : JSON.stringify(args)})`;
  return evaluateCdp(webSocketUrl, expression);
}

function evaluateCdp(webSocketUrl, expression) {
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = new WebSocket(webSocketUrl);
    const timeout = setTimeout(() => {
      socket.close();
      rejectPromise(new Error("CDP evaluation timed out"));
    }, 60_000);
    socket.addEventListener("open", () => socket.send(JSON.stringify({
      id: 1,
      method: "Runtime.evaluate",
      params: { expression, awaitPromise: true, returnByValue: true },
    })));
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id !== 1) return;
      clearTimeout(timeout);
      socket.close();
      if (message.error) rejectPromise(new Error(JSON.stringify(message.error)));
      else if (message.result?.exceptionDetails) rejectPromise(new Error(JSON.stringify(message.result.exceptionDetails)));
      else resolvePromise(message.result?.result?.value);
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      rejectPromise(new Error("CDP WebSocket connection failed"));
    });
  });
}

function run(command, args, timeout) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    timeout,
  });
  assert(result.status === 0, `${command} ${args.join(" ")} failed: ${tail(result.stderr)}`);
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function tail(value, maximum = 3000) {
  const text = String(value ?? "");
  return text.length > maximum ? text.slice(-maximum) : text;
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
