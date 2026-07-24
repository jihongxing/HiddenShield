import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_AUDIO_FORMAT_CHANNEL_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve("artifacts/desktop-audio-format-channel-gate", runId);
const summaryPath = join(evidenceDir, "summary.json");
const fixtureDir = resolve(
  process.env.HIDDENSHIELD_AUDIO_FORMAT_CHANNEL_FIXTURE_DIR ??
    "tmp-ui-qa/watermark-real-file-matrix/20260721/audio-medium-baseline-pass",
);
const installedExe = resolve(
  process.env.HIDDENSHIELD_INSTALLED_EXE ??
    "tmp-ui-qa/desktop-installer-self-contained/20260722-webp-q60-core-fix/installed/hidden_shield.exe",
);
const coreReader = resolve("watermark-core/target/release/desktop_audio_read_qa.exe");
const ffprobe = process.env.FFPROBE_PATH ?? "ffprobe.exe";
const debugPort = 10_400 + Math.floor(Math.random() * 400);
const formats = ["wav", "mp3", "flac", "ogg", "m4a"];
const channelModes = [
  { name: "mono", channels: 1 },
  { name: "stereo", channels: 2 },
];

const summary = {
  schemaVersion: "desktop_audio_format_channel_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  product: {
    endpoint: "installed-desktop",
    installedExecutable: relative(root, installedExe),
    installedExecutableSha256: null,
    inputFormats: formats,
    channelModes: channelModes.map((mode) => mode.name),
    sampleRate: 48_000,
    durationSeconds: 30,
    outputContainer: "wav",
    specificationPromise: "preserve source sample rate and channels",
    payloadProtocolVersion: 3,
    mobileFrozen: true,
  },
  fixtures: [],
  checks: {},
};

mkdirSync(evidenceDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  assert(existsSync(installedExe), `Installed executable not found: ${installedExe}`);
  assert(existsSync(coreReader), `Independent core reader not found: ${coreReader}`);
  assert(existsSync(fixtureDir), `Fixture directory not found: ${fixtureDir}`);
  summary.product.installedExecutableSha256 = sha256(installedExe);

  const fixtures = buildFixtureMatrix();
  summary.checks.fixtureMatrix = {
    passed: fixtures.length === 10,
    count: fixtures.length,
    expectedCount: 10,
    details: fixtures.map((fixture) => ({
      format: fixture.format,
      channelMode: fixture.channelMode,
      inputPath: relative(root, fixture.inputPath),
      inputSha256: sha256(fixture.inputPath),
      input: describeProbe(probe(fixture.inputPath), fixture.inputPath),
    })),
  };
  assert(summary.checks.fixtureMatrix.passed, "Audio fixture matrix is incomplete.");

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
        creatorInput: `HiddenShield RC-MEDIA-003 ${runId}`,
      });
    }

    for (const fixture of fixtures) {
      const result = await processFixture(target.webSocketDebuggerUrl, fixture);
      summary.fixtures.push(result);
      writeSummary();
      assert(result.status === "passed", `${fixture.format}/${fixture.channelMode} failed.`);
    }
  } finally {
    if (child.exitCode === null) child.kill();
  }

  summary.checks.matrix = {
    passed:
      summary.fixtures.length === 10 &&
      summary.fixtures.every((fixture) => fixture.status === "passed"),
    total: summary.fixtures.length,
    passedCount: summary.fixtures.filter((fixture) => fixture.status === "passed").length,
    failedCount: summary.fixtures.filter((fixture) => fixture.status !== "passed").length,
  };
  assert(summary.checks.matrix.passed, "Installed audio format/channel matrix failed.");
  summary.status = "passed";
  summary.finishedAt = new Date().toISOString();
  summary.elapsedMs =
    new Date(summary.finishedAt).getTime() - new Date(summary.startedAt).getTime();
  writeSummary();
  console.log(`Desktop audio format/channel Gate: ${summary.status}`);
  console.log(summaryPath);
} catch (error) {
  summary.status = "failed";
  summary.error = String(error?.stack ?? error);
  summary.finishedAt = new Date().toISOString();
  summary.elapsedMs =
    new Date(summary.finishedAt).getTime() - new Date(summary.startedAt).getTime();
  writeSummary();
  console.error(summary.error);
  console.error(summaryPath);
  process.exitCode = 1;
}

function buildFixtureMatrix() {
  return channelModes.flatMap((mode) =>
    formats.map((format) => {
      const inputPath = join(fixtureDir, `audio-medium-${mode.name}-${format}.${format}`);
      assert(existsSync(inputPath), `Missing fixture: ${inputPath}`);
      const input = describeProbe(probe(inputPath), inputPath);
      assert(input.sampleRate === 48_000, `${basename(inputPath)} is not 48 kHz.`);
      assert(input.channels === mode.channels, `${basename(inputPath)} channel count is invalid.`);
      assert(input.durationSeconds >= 30, `${basename(inputPath)} is shorter than 30 seconds.`);
      return { format, channelMode: mode.name, inputPath };
    }),
  );
}

async function processFixture(webSocketUrl, fixture) {
  const startedAt = Date.now();
  const input = describeProbe(probe(fixture.inputPath), fixture.inputPath);
  const result = {
    format: fixture.format,
    channelMode: fixture.channelMode,
    inputPath: relative(root, fixture.inputPath),
    input,
    inputSha256: sha256(fixture.inputPath),
    status: "running",
  };

  try {
    const sourceMeta = await cdpInvoke(
      webSocketUrl,
      "probe_source",
      { path: fixture.inputPath },
      120_000,
    );
    result.sourceMeta = sourceMeta;
    assert(sourceMeta?.fileType === "audio", "Installed probe did not classify fixture as audio.");
    assert(sourceMeta.fileSizeBytes === statSync(fixture.inputPath).size, "Installed probe lost exact byte size.");
    assert(sourceMeta.sampleRate === input.sampleRate, "Installed probe changed sample rate.");
    assert(sourceMeta.channels === input.channels, "Installed probe changed channel count.");
    assert(sourceMeta.durationSecs >= 30, "Installed preflight reported a short audio input.");

    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath: fixture.inputPath,
      platforms: [],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    await waitForPipeline(webSocketUrl, pipeline.pipelineId, 5 * 60_000);

    const records = await cdpInvoke(webSocketUrl, "list_vault_records");
    const record = [...(records ?? [])]
      .filter(
        (candidate) =>
          !priorIds.has(candidate.id) && candidate.fileName === basename(fixture.inputPath),
      )
      .sort((left, right) =>
        String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")),
      )[0];
    assert(record, "No vault record found for accepted installed pipeline.");
    assert(
      record.protectedCopyPath && existsSync(record.protectedCopyPath),
      "Installed pipeline did not produce a protected copy.",
    );
    assert(extname(record.protectedCopyPath).toLowerCase() === ".wav", "Protected copy is not WAV.");

    const output = describeProbe(probe(record.protectedCopyPath), record.protectedCopyPath);
    const specification = comparePromisedSpecification(input, output);
    assert(specification.passed, `Output specification changed: ${JSON.stringify(specification)}`);
    assert(record.writeVerificationStatus === "verified", "Installed write-after-read failed.");
    assert(record.payloadProtocolVersion === 3, "Installed record is not V3.");
    assert(record.payloadBytesLength === 39, "Installed record payload length is not 39.");

    const coreRead = runCoreRead(record.protectedCopyPath);
    assert(coreRead.passed, `Independent core read failed: ${coreRead.stderrTail}`);
    assert(coreRead.result?.payloadProtocolVersion === 3, "Independent core read is not V3.");
    assert(coreRead.result?.watermarkUid === record.watermarkUid, "Independent core UID mismatch.");

    const readonlyVerification = await cdpInvoke(
      webSocketUrl,
      "verify_suspect_readonly_candidate",
      { path: record.protectedCopyPath },
      10 * 60_000,
    );
    assert(readonlyVerification?.matched === true, "Installed read-only verification failed.");
    assert(
      readonlyVerification.watermarkUid === record.watermarkUid,
      "Installed read-only UID mismatch.",
    );
    assert(
      readonlyVerification.payloadProtocolVersion === 3,
      "Installed read-only verification is not V3.",
    );

    result.status = "passed";
    result.pipelineId = pipeline.pipelineId;
    result.vaultRecordId = record.id;
    result.watermarkUid = record.watermarkUid;
    result.outputPath = relative(root, record.protectedCopyPath);
    result.output = output;
    result.outputSha256 = sha256(record.protectedCopyPath);
    result.specification = specification;
    result.writeAfterRead = {
      status: record.writeVerificationStatus,
      message: record.writeVerificationMessage,
      payloadProtocolVersion: record.payloadProtocolVersion,
      payloadBytesLength: record.payloadBytesLength,
    };
    result.independentCoreRead = coreRead;
    result.readOnlyVerification = {
      matched: readonlyVerification.matched,
      watermarkUid: readonlyVerification.watermarkUid,
      confidence: readonlyVerification.confidence,
      payloadProtocolVersion: readonlyVerification.payloadProtocolVersion,
      payloadBytesLength: readonlyVerification.payloadBytesLength,
      payloadAuthStatus: readonlyVerification.payloadAuthStatus,
      mediaPayloadRole: readonlyVerification.mediaPayloadRole,
      reasonCode: readonlyVerification.reasonCode,
      durationMs: readonlyVerification.durationMs,
    };
  } catch (error) {
    result.status = "failed";
    result.error = String(error?.stack ?? error);
  }

  result.elapsedMs = Date.now() - startedAt;
  return result;
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
  const command = spawnSync(coreReader, [path], {
    cwd: root,
    encoding: "utf8",
    timeout: 120_000,
  });
  let result = null;
  if (command.status === 0) {
    const line = String(command.stdout ?? "").trim().split(/\r?\n/).filter(Boolean).at(-1);
    if (line) result = JSON.parse(line);
  }
  return {
    passed: command.status === 0 && result?.status === "verified",
    exitCode: command.status,
    result,
    stdoutTail: tail(command.stdout),
    stderrTail: tail(command.stderr),
  };
}

function probe(path) {
  const result = spawnSync(
    ffprobe,
    [
      "-v",
      "error",
      "-show_entries",
      "format=format_name,duration,size:stream=codec_type,codec_name,sample_rate,channels,channel_layout,sample_fmt,bits_per_sample,bits_per_raw_sample",
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
  const bitsPerSample = Number(stream.bits_per_sample ?? 0);
  const bitsPerRawSample = Number(stream.bits_per_raw_sample ?? 0);
  const sampleFmt = stream.sample_fmt ?? null;
  return {
    path: relative(root, path),
    bytes: statSync(path).size,
    formatName: result.format?.format_name ?? null,
    codecName: stream.codec_name ?? null,
    durationSeconds: Number(result.format?.duration ?? 0),
    sampleRate: Number(stream.sample_rate ?? 0),
    channels: Number(stream.channels ?? 0),
    channelLayout: stream.channel_layout ?? null,
    sampleFmt,
    bitsPerSample,
    bitsPerRawSample,
    effectiveBitDepth:
      bitsPerRawSample || bitsPerSample || inferredBitDepth(sampleFmt),
  };
}

function comparePromisedSpecification(input, output) {
  const losslessDepthMustMatch =
    input.codecName === "flac" || String(input.codecName ?? "").startsWith("pcm_");
  const promisedFields = [
    "sampleRate",
    "channels",
    ...(losslessDepthMustMatch ? ["effectiveBitDepth"] : []),
  ];
  const differences = Object.fromEntries(
    promisedFields
      .filter((field) => input[field] !== output[field])
      .map((field) => [field, { input: input[field], output: output[field] }]),
  );
  return {
    passed: Object.keys(differences).length === 0,
    promisedFields,
    differences,
    inputEncoding: {
      formatName: input.formatName,
      codecName: input.codecName,
      sampleFmt: input.sampleFmt,
    },
    outputEncoding: {
      formatName: output.formatName,
      codecName: output.codecName,
      sampleFmt: output.sampleFmt,
    },
    losslessDepthMustMatch,
  };
}

function inferredBitDepth(sampleFmt) {
  if (sampleFmt === "dbl" || sampleFmt === "dblp" || sampleFmt === "s64" || sampleFmt === "s64p") {
    return 64;
  }
  if (sampleFmt === "flt" || sampleFmt === "fltp" || sampleFmt === "s32" || sampleFmt === "s32p") {
    return 32;
  }
  if (sampleFmt === "s16" || sampleFmt === "s16p") return 16;
  if (sampleFmt === "u8" || sampleFmt === "u8p") return 8;
  return 0;
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

function writeSummary() {
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
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
