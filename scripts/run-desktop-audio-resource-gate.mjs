import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import {
  appendFileSync,
  closeSync,
  copyFileSync,
  existsSync,
  ftruncateSync,
  mkdirSync,
  openSync,
  readSync,
  statSync,
  writeFileSync,
  writeSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_AUDIO_RESOURCE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve("artifacts/desktop-audio-resource-gate", runId);
const fixtureDir = resolve("tmp-ui-qa/desktop-audio-resource", runId);
const installedExe = process.env.HIDDENSHIELD_INSTALLED_EXE
  ? resolve(process.env.HIDDENSHIELD_INSTALLED_EXE)
  : newestInstalledExecutable();
const ffmpeg = process.env.FFMPEG_PATH ?? "ffmpeg.exe";
const ffprobe = process.env.FFPROBE_PATH ?? "ffprobe.exe";
const summaryPath = join(evidenceDir, "summary.json");
const debugPort = 10_000 + Math.floor(Math.random() * 400);
const maximumBytes = 512 * 1024 * 1024;
const maximumDurationSeconds = 20 * 60;

const summary = {
  schemaVersion: "desktop_audio_resource_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  product: {
    endpoint: "installed-desktop",
    installedExecutable: relative(root, installedExe),
    supportedDurationSeconds: { minimum: 30, maximum: maximumDurationSeconds },
    maximumSourceBytes: maximumBytes,
    outputContainer: "wav",
    imageAlgorithmFrozen: true,
    audioCarrierAlgorithm: "watermark-core standalone audio carrier",
    usesImageSpatialRecoveryV1: false,
  },
  fixtures: [],
  checks: {},
  limitations: [
    "The duration and capacity boundaries are intentionally tested as separate fixtures to avoid conflating two independent resource limits.",
    "The exact 512 MiB WAV uses a standards-compliant trailing JUNK chunk; its audible PCM payload remains 31 seconds.",
  ],
};

mkdirSync(evidenceDir, { recursive: true });
mkdirSync(fixtureDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  assert(existsSync(installedExe), `Installed executable not found: ${installedExe}`);
  const fixtures = generateFixtures();
  summary.checks.fixturesGenerated = {
    passed:
      statSync(fixtures.durationAllowed).size < maximumBytes &&
      statSync(fixtures.durationRejected).size < maximumBytes &&
      statSync(fixtures.capacityAllowed).size === maximumBytes &&
      statSync(fixtures.capacityRejected).size === maximumBytes + 1,
    details: Object.fromEntries(
      Object.entries(fixtures).map(([name, path]) => [
        name,
        { path: relative(root, path), bytes: statSync(path).size, probe: describeProbe(probe(path), path) },
      ]),
    ),
  };
  assert(summary.checks.fixturesGenerated.passed, "Audio resource fixtures are invalid.");

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
      passed: target.title === "HiddenShield",
      details: { url: target.url, title: target.title },
    };
    assert(summary.checks.installedUiLoaded.passed, "Installed UI did not load.");
    const identity = await cdpInvoke(target.webSocketDebuggerUrl, "get_identity_status");
    if (!identity?.initialized) {
      await cdpInvoke(target.webSocketDebuggerUrl, "setup_identity", {
        creatorInput: `HiddenShield audio resource QA ${runId}`,
      });
    }

    for (const [boundary, path] of [
      ["duration_exact_20_minutes", fixtures.durationAllowed],
      ["capacity_exact_512_mib", fixtures.capacityAllowed],
    ]) {
      const result = await processAllowedFixture(target.webSocketDebuggerUrl, boundary, path);
      summary.fixtures.push(result);
      writeSummary();
      assert(result.status === "passed", `${boundary} failed.`);
    }

    for (const [boundary, path, expected] of [
      ["duration_20_minutes_plus_1_second", fixtures.durationRejected, "audio_too_long"],
      ["capacity_512_mib_plus_1_byte", fixtures.capacityRejected, "audio_file_too_large"],
    ]) {
      const result = await processRejectedFixture(
        target.webSocketDebuggerUrl,
        boundary,
        path,
        expected,
      );
      summary.fixtures.push(result);
      writeSummary();
      assert(result.status === "passed", `${boundary} rejection failed.`);
    }
  } finally {
    if (child.exitCode === null) child.kill();
  }

  summary.status = "passed";
  summary.completedAt = new Date().toISOString();
  writeSummary();
  console.log(`Desktop audio resource Gate passed: ${summaryPath}`);
} catch (error) {
  summary.status = "failed";
  summary.completedAt = new Date().toISOString();
  summary.error = String(error?.stack ?? error);
  writeSummary();
  console.error(`Desktop audio resource Gate failed: ${summaryPath}`);
  throw error;
}

function generateFixtures() {
  const durationAllowed = join(fixtureDir, "audio-duration-1200s.wav");
  const durationRejected = join(fixtureDir, "audio-duration-1201s.wav");
  const capacityBase = join(fixtureDir, "audio-capacity-base-31s.wav");
  const capacityAllowed = join(fixtureDir, "audio-capacity-512mib.wav");
  const capacityRejected = join(fixtureDir, "audio-capacity-512mib-plus-1.wav");

  generateSineWav(durationAllowed, maximumDurationSeconds, 8_000, 1);
  generateSineWav(durationRejected, maximumDurationSeconds + 1, 8_000, 1);
  generateSineWav(capacityBase, 31, 48_000, 2);
  copyFileSync(capacityBase, capacityAllowed);
  padWavWithJunkChunk(capacityAllowed, maximumBytes);
  copyFileSync(capacityAllowed, capacityRejected);
  appendFileSync(capacityRejected, Buffer.from([0]));

  return { durationAllowed, durationRejected, capacityAllowed, capacityRejected };
}

function generateSineWav(path, durationSeconds, sampleRate, channels) {
  const source = `sine=frequency=523.25:sample_rate=${sampleRate}:duration=${durationSeconds}`;
  const args = ["-y", "-f", "lavfi", "-i", source];
  if (channels === 2) {
    args.push("-ac", "2");
  } else {
    args.push("-ac", "1");
  }
  args.push("-c:a", "pcm_s16le", path);
  run(ffmpeg, args, 10 * 60_000);
}

function padWavWithJunkChunk(path, targetBytes) {
  const originalBytes = statSync(path).size;
  const junkBytes = targetBytes - originalBytes - 8;
  assert(junkBytes >= 0 && junkBytes <= 0xffff_ffff, "Invalid WAV JUNK chunk size.");
  assert(junkBytes % 2 === 0, "WAV JUNK chunk must have an even payload size.");

  const file = openSync(path, "r+");
  try {
    const chunkHeader = Buffer.alloc(8);
    chunkHeader.write("JUNK", 0, "ascii");
    chunkHeader.writeUInt32LE(junkBytes, 4);
    writeSync(file, chunkHeader, 0, chunkHeader.length, originalBytes);
    ftruncateSync(file, targetBytes);
    const riffSize = Buffer.alloc(4);
    riffSize.writeUInt32LE(targetBytes - 8, 0);
    writeSync(file, riffSize, 0, riffSize.length, 4);
  } finally {
    closeSync(file);
  }
}

async function processAllowedFixture(webSocketUrl, boundary, inputPath) {
  const startedAt = Date.now();
  const inputProbe = probe(inputPath);
  const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path: inputPath }, 120_000);
  const result = {
    boundary,
    expectation: "accepted",
    inputPath: relative(root, inputPath),
    input: describeProbe(inputProbe, inputPath),
    sourceMeta,
    status: "running",
  };
  try {
    assert(sourceMeta?.fileType === "audio", "Installed probe did not classify fixture as audio.");
    assert(sourceMeta.fileSizeBytes === statSync(inputPath).size, "Installed probe lost exact byte size.");
    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath,
      platforms: [],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    await waitForPipeline(webSocketUrl, pipeline.pipelineId, 15 * 60_000);
    const records = await cdpInvoke(webSocketUrl, "list_vault_records");
    const record = [...(records ?? [])]
      .filter((candidate) => !priorIds.has(candidate.id) && candidate.fileName === basename(inputPath))
      .sort((left, right) => String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")))[0];
    assert(record, "No vault record found for accepted installed pipeline.");
    assert(
      record.protectedCopyPath && existsSync(record.protectedCopyPath),
      "Accepted pipeline did not produce a protected copy.",
    );

    const outputProbe = probe(record.protectedCopyPath);
    const specification = compareSpecification(inputProbe, outputProbe);
    assert(specification.passed, `Output specification changed: ${JSON.stringify(specification)}`);
    assert(record.writeVerificationStatus === "verified", "Installed write-after-read failed.");
    const coreRead = runCoreRead(record.protectedCopyPath);
    assert(coreRead.passed, "Independent core read failed.");
    const readonlyVerification = await cdpInvoke(
      webSocketUrl,
      "verify_suspect_readonly_candidate",
      { path: record.protectedCopyPath },
      10 * 60_000,
    );
    assert(readonlyVerification?.matched === true, "Installed read-only verification failed.");

    result.status = "passed";
    result.pipelineId = pipeline.pipelineId;
    result.outputPath = relative(root, record.protectedCopyPath);
    result.output = describeProbe(outputProbe, record.protectedCopyPath);
    result.specification = specification;
    result.writeAfterRead = {
      recordStatus: record.writeVerificationStatus,
      recordMessage: record.writeVerificationMessage,
      independentCoreRead: coreRead,
    };
    result.readOnlyVerification = readonlyVerification;
    result.inputSha256 = sha256(inputPath);
    result.outputSha256 = sha256(record.protectedCopyPath);
  } catch (error) {
    result.status = "failed";
    result.error = String(error?.stack ?? error);
  }
  result.elapsedMs = Date.now() - startedAt;
  return result;
}

async function processRejectedFixture(webSocketUrl, boundary, inputPath, expectedPreflightCode) {
  const startedAt = Date.now();
  const sourceMeta = await cdpInvoke(webSocketUrl, "probe_source", { path: inputPath }, 120_000);
  const result = {
    boundary,
    expectation: "rejected",
    expectedPreflightCode,
    inputPath: relative(root, inputPath),
    input: describeProbe(probe(inputPath), inputPath),
    sourceMeta,
    status: "running",
  };
  try {
    assert(sourceMeta.fileSizeBytes === statSync(inputPath).size, "Installed probe lost exact byte size.");
    const frontendPreflightCode = classifyPreflight(sourceMeta);
    assert(
      frontendPreflightCode === expectedPreflightCode,
      `Unexpected frontend preflight code: ${frontendPreflightCode}`,
    );

    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath,
      platforms: [],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    await waitForPipeline(webSocketUrl, pipeline.pipelineId, 120_000);
    const recordsAfter = await cdpInvoke(webSocketUrl, "list_vault_records");
    const unexpectedRecord = (recordsAfter ?? []).find(
      (candidate) => !priorIds.has(candidate.id) && candidate.fileName === basename(inputPath),
    );
    assert(!unexpectedRecord, "Rejected boundary unexpectedly created a vault record.");

    result.status = "passed";
    result.pipelineId = pipeline.pipelineId;
    result.frontendPreflightCode = frontendPreflightCode;
    result.executionBoundary = {
      passed: true,
      vaultRecordCreated: false,
    };
  } catch (error) {
    result.status = "failed";
    result.error = String(error?.stack ?? error);
  }
  result.elapsedMs = Date.now() - startedAt;
  return result;
}

function classifyPreflight(meta) {
  if (meta.durationConfirmed === false) return "audio_duration_unknown";
  if (meta.durationSecs < 30) return "audio_too_short";
  if (meta.durationSecs > maximumDurationSeconds) return "audio_too_long";
  if (meta.fileSizeBytes > maximumBytes) return "audio_file_too_large";
  if (!meta.sampleRate || !meta.channels) return "audio_spec_unknown";
  if (meta.sampleRate < 8_000) return "audio_sample_rate_too_low";
  if (meta.sampleRate > 48_000) return "audio_sample_rate_too_high";
  if (meta.channels < 1 || meta.channels > 2) return "audio_channels_unsupported";
  return "ok";
}

async function waitForPipeline(webSocketUrl, pipelineId, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const active = await cdpInvoke(webSocketUrl, "check_active_pipelines");
    if (!active.includes(pipelineId)) return;
    await delay(500);
  }
  throw new Error(`Pipeline ${pipelineId} did not finish within ${timeoutMs}ms.`);
}

function runCoreRead(path) {
  const result = spawnSync(
    "cargo",
    [
      "run",
      "--release",
      "--manifest-path",
      "watermark-core/Cargo.toml",
      "--bin",
      "desktop_audio_read_qa",
      "--",
      path,
    ],
    { cwd: root, encoding: "utf8", timeout: 10 * 60_000 },
  );
  return {
    passed: result.status === 0,
    exitCode: result.status,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
  };
}

function probe(path) {
  const result = spawnSync(
    ffprobe,
    [
      "-v",
      "error",
      "-show_entries",
      "format=duration,size:stream=codec_type,sample_rate,channels,sample_fmt,bits_per_sample,bits_per_raw_sample",
      "-of",
      "json",
      path,
    ],
    { cwd: root, encoding: "utf8", timeout: 120_000 },
  );
  assert(result.status === 0, `ffprobe failed for ${path}: ${tail(result.stderr)}`);
  return JSON.parse(result.stdout);
}

function describeProbe(result, path) {
  const stream = result.streams?.find((candidate) => candidate.codec_type === "audio") ?? {};
  return {
    path: relative(root, path),
    bytes: statSync(path).size,
    durationSeconds: Number(result.format?.duration ?? 0),
    sampleRate: Number(stream.sample_rate ?? 0),
    channels: Number(stream.channels ?? 0),
    sampleFmt: stream.sample_fmt ?? null,
    bitsPerSample: Number(stream.bits_per_sample ?? 0),
    bitsPerRawSample: Number(stream.bits_per_raw_sample ?? 0),
  };
}

function compareSpecification(input, output) {
  const left = probeSpecification(input);
  const right = probeSpecification(output);
  const fields = ["sampleRate", "channels", "sampleFmt", "bitsPerSample", "bitsPerRawSample"];
  const differences = Object.fromEntries(
    fields
      .filter((field) => left[field] !== right[field])
      .map((field) => [field, { input: left[field], output: right[field] }]),
  );
  return { passed: Object.keys(differences).length === 0, differences };
}

function probeSpecification(result) {
  const stream = result.streams?.find((candidate) => candidate.codec_type === "audio") ?? {};
  return {
    sampleRate: Number(stream.sample_rate ?? 0),
    channels: Number(stream.channels ?? 0),
    sampleFmt: stream.sample_fmt ?? null,
    bitsPerSample: Number(stream.bits_per_sample ?? 0),
    bitsPerRawSample: Number(stream.bits_per_raw_sample ?? 0),
  };
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
        if (page.title === "HiddenShield" && page.readyState === "complete") {
          return { ...target, ...page };
        }
      }
    } catch {}
    await delay(500);
  }
  throw new Error("Installed desktop target did not appear on the CDP port.");
}

async function cdpInvoke(webSocketUrl, command, args = undefined, timeoutMs = 30_000) {
  const expression = `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${args === undefined ? "undefined" : JSON.stringify(args)})`;
  const value = await evaluateCdp(webSocketUrl, expression, timeoutMs);
  if (value?.error) throw new Error(`${command} failed: ${JSON.stringify(value.error)}`);
  return value;
}

function evaluateCdp(webSocketUrl, expression, timeoutMs = 30_000) {
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = new WebSocket(webSocketUrl);
    const timeout = setTimeout(() => {
      socket.close();
      rejectPromise(new Error("CDP evaluation timed out"));
    }, timeoutMs);
    socket.addEventListener("open", () =>
      socket.send(
        JSON.stringify({
          id: 1,
          method: "Runtime.evaluate",
          params: { expression, awaitPromise: true, returnByValue: true },
        }),
      ),
    );
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id !== 1) return;
      clearTimeout(timeout);
      socket.close();
      if (message.error) rejectPromise(new Error(JSON.stringify(message.error)));
      else if (message.result?.exceptionDetails) {
        rejectPromise(new Error(JSON.stringify(message.result.exceptionDetails)));
      } else {
        resolvePromise(message.result?.result?.value);
      }
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      rejectPromise(new Error("CDP WebSocket connection failed"));
    });
  });
}

function newestInstalledExecutable() {
  const base = resolve("tmp-ui-qa/desktop-installer-self-contained");
  const candidates = [
    join(base, "20260722-audio-resource", "installed", "hidden_shield.exe"),
    join(base, "20260722-image-complete-final", "installed", "hidden_shield.exe"),
  ].filter(existsSync);
  if (candidates.length === 0) {
    throw new Error("Set HIDDENSHIELD_INSTALLED_EXE to the current installed candidate.");
  }
  return candidates[0];
}

function run(command, args, timeout) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    timeout,
  });
  assert(result.status === 0, `${command} ${args.join(" ")} failed: ${tail(result.stderr)}`);
}

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function sha256(path) {
  const hash = createHash("sha256");
  const file = openSync(path, "r");
  const buffer = Buffer.allocUnsafe(4 * 1024 * 1024);
  try {
    let bytesRead = 0;
    do {
      bytesRead = readSync(file, buffer, 0, buffer.length, null);
      if (bytesRead > 0) hash.update(buffer.subarray(0, bytesRead));
    } while (bytesRead > 0);
  } finally {
    closeSync(file);
  }
  return hash.digest("hex");
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
