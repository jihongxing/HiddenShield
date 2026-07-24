import { createHash } from "node:crypto";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_HIGH_BIT_DEPTH_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve("artifacts/desktop-high-bit-depth-audio-gate", runId);
const fixtureDir = resolve("tmp-ui-qa/desktop-high-bit-depth-audio", runId);
const installedExe = process.env.HIDDENSHIELD_INSTALLED_EXE
  ? resolve(process.env.HIDDENSHIELD_INSTALLED_EXE)
  : newestInstalledExecutable();
const ffmpeg = process.env.FFMPEG_PATH ?? "ffmpeg.exe";
const ffprobe = process.env.FFPROBE_PATH ?? "ffprobe.exe";
const summaryPath = join(evidenceDir, "summary.json");
const debugPort = 9700 + Math.floor(Math.random() * 300);

const summary = {
  schemaVersion: "desktop_high_bit_depth_audio_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  product: {
    endpoint: "installed-desktop",
    installedExecutable: relative(root, installedExe),
    outputContainer: "wav",
    sourceSpecificationsPreserved: ["sample_rate", "channels", "bit_depth", "sample_fmt"],
  },
  fixtures: [],
  checks: {},
  limitations: [
    "PCM differences include intentional watermark modification; the error report is diagnostic and is not a zero-difference claim.",
  ],
};

mkdirSync(evidenceDir, { recursive: true });
mkdirSync(fixtureDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  assert(existsSync(installedExe), `Installed executable not found: ${installedExe}`);
  const fixturePaths = generateFixtures();
  summary.checks.fixturesGenerated = {
    passed: fixturePaths.length === 6,
    details: fixturePaths.map((path) => relative(root, path)),
  };
  assert(fixturePaths.length === 6, "Expected six high-bit-depth fixtures.");

  const child = spawn(installedExe, [], {
    cwd: dirname(installedExe),
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    },
    stdio: "ignore",
    windowsHide: true,
  });

  try {
    const target = await waitForTarget();
    summary.checks.installedUiLoaded = {
      passed: target.url === "http://tauri.localhost/" && target.title === "HiddenShield",
      details: { url: target.url, title: target.title },
    };
    assert(summary.checks.installedUiLoaded.passed, "Installed UI did not load.");
    const identity = await cdpInvoke(target.webSocketDebuggerUrl, "get_identity_status");
    if (!identity?.initialized) {
      await cdpInvoke(target.webSocketDebuggerUrl, "setup_identity", {
        creatorInput: `HiddenShield high-bit-depth QA ${runId}`,
      });
    }
    summary.checks.creatorIdentityReady = {
      passed: true,
      details: { initializedBeforeRun: identity?.initialized === true },
    };

    for (const fixturePath of fixturePaths) {
      const result = await processFixture(target.webSocketDebuggerUrl, fixturePath);
      summary.fixtures.push(result);
      writeSummary();
      assert(result.status === "passed", `${basename(fixturePath)} failed.`);
    }
  } finally {
    if (child.exitCode === null) child.kill();
  }

  summary.status = "passed";
  summary.completedAt = new Date().toISOString();
  writeSummary();
  console.log(`Desktop high-bit-depth audio Gate passed: ${summaryPath}`);
} catch (error) {
  summary.status = "failed";
  summary.completedAt = new Date().toISOString();
  summary.error = String(error?.stack ?? error);
  writeSummary();
  console.error(`Desktop high-bit-depth audio Gate failed: ${summaryPath}`);
  throw error;
}

function generateFixtures() {
  const specs = [
    { container: "wav", encoding: "pcm_s24le", sampleFmt: "s32", suffix: "24bit-wav" },
    { container: "flac", encoding: "flac", sampleFmt: "s32", suffix: "24bit-flac" },
    { container: "wav", encoding: "pcm_f32le", sampleFmt: "flt", suffix: "float32-wav" },
  ];
  const paths = [];
  for (const channels of [1, 2]) {
    for (const spec of specs) {
      const output = join(fixtureDir, `${spec.suffix}-${channels === 1 ? "mono" : "stereo"}.${spec.container}`);
      const channelLayout = channels === 1 ? "mono" : "stereo";
      const source = [
        "sine=frequency=440:sample_rate=48000:duration=31",
        "sine=frequency=997:sample_rate=48000:duration=31",
      ];
      const filter =
        channels === 1
          ? "[0:a]volume=0.3[a]"
          : "[0:a][1:a]amerge=inputs=2,pan=stereo|c0=0.8*c0|c1=0.8*c1[a]";
      const inputs = channels === 1
        ? ["-f", "lavfi", "-i", source[0]]
        : ["-f", "lavfi", "-i", source[0], "-f", "lavfi", "-i", source[1]];
      run(ffmpeg, [
        "-y",
        ...inputs,
        "-filter_complex", filter,
        "-map", "[a]",
        "-ac", String(channels),
        "-ar", "48000",
        "-c:a", spec.encoding,
        ...(spec.container === "flac" ? ["-sample_fmt", spec.sampleFmt, "-bits_per_raw_sample", "24"] : []),
        output,
      ]);
      paths.push(output);
      void channelLayout;
    }
  }
  return paths;
}

async function processFixture(webSocketUrl, inputPath) {
  const startedAt = Date.now();
  const inputProbe = probe(inputPath);
  const inputRecord = {
    inputPath: relative(root, inputPath),
    input: describeProbe(inputProbe, inputPath),
    status: "running",
  };
  try {
    const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path: inputPath });
    assert(sourceMeta?.fileType === "audio", "Installed probe did not classify fixture as audio.");
    assert(sourceMeta?.watermarkEligible !== false, "Installed preflight rejected high-bit-depth fixture.");

    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath,
      platforms: ["douyin"],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    const pipelineId = pipeline.pipelineId;
    await waitForPipeline(webSocketUrl, pipelineId);
    const records = await cdpInvoke(webSocketUrl, "list_vault_records");
    const record = [...(records ?? [])]
      .filter((candidate) => !priorIds.has(candidate.id) && candidate.fileName === basename(inputPath))
      .sort((left, right) => String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")))[0];
    assert(record, "No vault record found for installed pipeline.");
    const outputPath = record.protectedCopyPath;
    assert(outputPath && existsSync(outputPath), "Installed pipeline did not produce a protected copy.");

    const outputProbe = probe(outputPath);
    const specification = compareSpecification(inputProbe, outputProbe);
    assert(specification.passed, `Output specification changed: ${JSON.stringify(specification)}`);

    const readBack = runCoreRead(outputPath);
    assert(readBack.passed, "Write-after-read core extraction failed.");
    assert(record.writeVerificationStatus === "verified", `Write-after-read record failed: ${record.writeVerificationMessage}`);
    const readonlyVerification = await cdpInvoke(
      webSocketUrl,
      "verify_suspect_readonly_candidate",
      { path: outputPath },
    );
    const verificationPassed = readonlyVerification?.matched === true &&
      typeof readonlyVerification?.watermarkUid === "string";
    assert(verificationPassed, `Installed read-only verification failed: ${JSON.stringify(readonlyVerification)}`);

    const error = measurePcmDifference(inputPath, outputPath);
    inputRecord.status = "passed";
    inputRecord.pipelineId = pipelineId;
    inputRecord.outputPath = relative(root, outputPath);
    inputRecord.output = describeProbe(outputProbe, outputPath);
    inputRecord.inputSha256 = sha256(inputPath);
    inputRecord.outputSha256 = sha256(outputPath);
    inputRecord.specification = specification;
    inputRecord.writeAfterRead = {
      recordStatus: record.writeVerificationStatus,
      recordMessage: record.writeVerificationMessage,
      independentCoreRead: readBack,
    };
    inputRecord.readOnlyVerification = readonlyVerification;
    inputRecord.pcmDifference = error;
    inputRecord.elapsedMs = Date.now() - startedAt;
    return inputRecord;
  } catch (error) {
    inputRecord.status = "failed";
    inputRecord.error = String(error?.stack ?? error);
    inputRecord.elapsedMs = Date.now() - startedAt;
    return inputRecord;
  }
}

async function waitForPipeline(webSocketUrl, pipelineId) {
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline) {
    const active = await cdpInvoke(webSocketUrl, "check_active_pipelines");
    if (!active.includes(pipelineId)) return;
    await delay(500);
  }
  throw new Error(`Pipeline ${pipelineId} did not finish within 180 seconds.`);
}

function runCoreRead(path) {
  const result = spawnSync(
    "cargo",
    ["run", "--release", "--manifest-path", "watermark-core/Cargo.toml", "--bin", "desktop_audio_read_qa", "--", path],
    { cwd: root, encoding: "utf8", timeout: 180_000 },
  );
  return {
    passed: result.status === 0,
    exitCode: result.status,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
  };
}

function measurePcmDifference(inputPath, outputPath) {
  const inputRaw = join(evidenceDir, `${basename(inputPath)}.input.f32`);
  const outputRaw = join(evidenceDir, `${basename(outputPath)}.output.f32`);
  decodeToFloat(inputPath, inputRaw);
  decodeToFloat(outputPath, outputRaw);
  const input = readFloat32(inputRaw);
  const output = readFloat32(outputRaw);
  rmSync(inputRaw, { force: true });
  rmSync(outputRaw, { force: true });
  const count = Math.min(input.length, output.length);
  let sumSquare = 0;
  let signalSquare = 0;
  let maxAbs = 0;
  for (let index = 0; index < count; index += 1) {
    const difference = output[index] - input[index];
    maxAbs = Math.max(maxAbs, Math.abs(difference));
    sumSquare += difference * difference;
    signalSquare += input[index] * input[index];
  }
  const rms = Math.sqrt(sumSquare / Math.max(count, 1));
  const signalRms = Math.sqrt(signalSquare / Math.max(count, 1));
  return {
    inputSamples: input.length,
    outputSamples: output.length,
    comparedSamples: count,
    sampleCountEqual: input.length === output.length,
    maxAbsoluteDifference: maxAbs,
    rmsDifference: rms,
    snrDb: rms > 0 ? 20 * Math.log10(signalRms / rms) : Number.POSITIVE_INFINITY,
    finite: Number.isFinite(maxAbs) && Number.isFinite(rms),
  };
}

function decodeToFloat(inputPath, outputPath) {
  run(ffmpeg, ["-y", "-i", inputPath, "-f", "f32le", "-acodec", "pcm_f32le", outputPath]);
}

function readFloat32(path) {
  const bytes = readFileSync(path);
  return new Float32Array(bytes.buffer, bytes.byteOffset, Math.floor(bytes.byteLength / 4));
}

function probe(path) {
  const raw = execFileSync(ffprobe, [
    "-v", "error", "-show_streams", "-show_format", "-of", "json", path,
  ], { cwd: root, encoding: "utf8" });
  const parsed = JSON.parse(raw);
  const stream = parsed.streams?.find((candidate) => candidate.codec_type === "audio") ?? parsed.streams?.[0];
  return { stream, format: parsed.format };
}

function describeProbe(result, path) {
  const stream = result.stream ?? {};
  return {
    path: relative(root, path),
    codecName: stream.codec_name ?? null,
    sampleFmt: stream.sample_fmt ?? null,
    sampleRate: Number(stream.sample_rate ?? 0),
    channels: Number(stream.channels ?? 0),
    bitsPerSample: Number(stream.bits_per_sample ?? 0),
    bitsPerRawSample: Number(stream.bits_per_raw_sample ?? 0),
    effectiveBitDepth: Number(stream.bits_per_raw_sample ?? 0) ||
      Number(stream.bits_per_sample ?? 0) ||
      inferredBitDepth(stream.sample_fmt),
    durationSeconds: Number(stream.duration ?? result.format?.duration ?? 0),
    sizeBytes: Number(result.format?.size ?? 0),
  };
}

function compareSpecification(input, output) {
  const left = describeProbe(input, "");
  const right = describeProbe(output, "");
  const fields = ["sampleRate", "channels", "sampleFmt", "effectiveBitDepth"];
  const differences = Object.fromEntries(fields
    .filter((field) => left[field] !== right[field])
    .map((field) => [field, { input: left[field], output: right[field] }]));
  return { passed: Object.keys(differences).length === 0, differences };
}

async function waitForTarget() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${debugPort}/json/list`);
      const targets = await response.json();
      const target = targets.find((candidate) => candidate.type === "page");
      if (target?.webSocketDebuggerUrl) {
        const state = await evaluateCdp(
          target.webSocketDebuggerUrl,
          "JSON.stringify({ title: document.title, readyState: document.readyState, url: location.href })",
        );
        const page = JSON.parse(state);
        if (
          page.title === "HiddenShield" &&
          page.readyState === "complete" &&
          ["http://tauri.localhost/", "tauri://localhost/"].includes(page.url)
        ) {
          return { ...target, ...page };
        }
      }
    } catch {}
    await delay(500);
  }
  throw new Error("Installed desktop target did not appear on the CDP port.");
}

async function cdpInvoke(webSocketUrl, command, args = undefined) {
  const expression = `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${args === undefined ? "undefined" : JSON.stringify(args)})`;
  const value = await evaluateCdp(webSocketUrl, expression);
  if (value?.error) throw new Error(`${command} failed: ${JSON.stringify(value.error)}`);
  return value;
}

function evaluateCdp(webSocketUrl, expression) {
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = new WebSocket(webSocketUrl);
    const timeout = setTimeout(() => {
      socket.close();
      rejectPromise(new Error("CDP evaluation timed out"));
    }, 30_000);
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

function newestInstalledExecutable() {
  const base = resolve("tmp-ui-qa/desktop-installer-self-contained");
  const candidates = [];
  for (const run of ["20260722-high-bit-depth"]) {
    const path = join(base, run, "installed", "hidden_shield.exe");
    if (existsSync(path)) candidates.push(path);
  }
  if (candidates.length === 0) {
    throw new Error("Set HIDDENSHIELD_INSTALLED_EXE to the current installed candidate.");
  }
  return candidates[0];
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: "utf8", timeout: 180_000 });
  assert(result.status === 0, `${command} ${args.join(" ")} failed: ${tail(result.stderr)}`);
}

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function inferredBitDepth(sampleFmt) {
  if (sampleFmt === "flt" || sampleFmt === "fltp" || sampleFmt === "s32" || sampleFmt === "s32p") return 32;
  if (sampleFmt === "s16" || sampleFmt === "s16p") return 16;
  return 0;
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
