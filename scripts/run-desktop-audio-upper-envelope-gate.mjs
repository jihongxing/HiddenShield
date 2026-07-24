import { createHash } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_AUDIO_UPPER_ENVELOPE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const evidenceDir = resolve("artifacts/desktop-audio-upper-envelope-gate", runId);
const fixtureDir = resolve("tmp-ui-qa/desktop-audio-upper-envelope", runId);
const summaryPath = join(evidenceDir, "summary.json");
const inputPath = join(fixtureDir, "rc-media-004-20m-48k-stereo-24bit.flac");
const cancellationInputPath = join(
  fixtureDir,
  "rc-media-004-20m-48k-stereo-24bit-cancel.flac",
);
const installedExe = resolve(
  process.env.HIDDENSHIELD_INSTALLED_EXE ??
    "tmp-ui-qa/desktop-installer-self-contained/20260722-webp-q60-core-fix/installed/hidden_shield.exe",
);
const coreReader = resolve("watermark-core/target/release/desktop_audio_read_qa.exe");
const ffmpeg = process.env.FFMPEG_PATH ?? "ffmpeg.exe";
const ffprobe = process.env.FFPROBE_PATH ?? "ffprobe.exe";
const debugPort = 10_800 + Math.floor(Math.random() * 400);
const maximumSourceBytes = 512 * 1024 * 1024;

const summary = {
  schemaVersion: "desktop_audio_upper_envelope_gate_v1",
  runId,
  startedAt: new Date().toISOString(),
  status: "running",
  product: {
    endpoint: "installed-desktop",
    installedExecutable: relative(root, installedExe),
    installedExecutableSha256: null,
    inputContainer: "flac",
    outputContainer: "wav",
    durationSeconds: 1200,
    sampleRate: 48_000,
    channels: 2,
    effectiveBitDepth: 24,
    maximumSourceBytes,
    payloadProtocolVersion: 3,
    mobileFrozen: true,
  },
  fixture: null,
  cancellation: null,
  completion: null,
  checks: {},
};

mkdirSync(evidenceDir, { recursive: true });
mkdirSync(fixtureDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  assert(existsSync(installedExe), `Installed executable not found: ${installedExe}`);
  assert(existsSync(coreReader), `Independent core reader not found: ${coreReader}`);
  summary.product.installedExecutableSha256 = sha256(installedExe);

  generateFixture();
  copyFileSync(inputPath, cancellationInputPath);
  const input = describeProbe(probe(inputPath), inputPath);
  const cancellationInput = describeProbe(probe(cancellationInputPath), cancellationInputPath);
  summary.fixture = {
    generationCommand:
      "ffmpeg lavfi dual-sine 1200s 48kHz stereo -> FLAC s32 bits_per_raw_sample=24",
    input,
    inputSha256: sha256(inputPath),
    cancellationInput,
    cancellationInputSha256: sha256(cancellationInputPath),
  };
  summary.checks.fixture = {
    passed:
      input.durationSeconds === 1200 &&
      input.sampleRate === 48_000 &&
      input.channels === 2 &&
      input.effectiveBitDepth === 24 &&
      input.bytes <= maximumSourceBytes &&
      cancellationInput.bytes === input.bytes,
    details: summary.fixture,
  };
  assert(summary.checks.fixture.passed, "Upper-envelope audio fixture is invalid.");

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
      details: { url: target.url, title: target.title, processId: child.pid },
    };
    assert(summary.checks.installedUiLoaded.passed, "Installed UI did not load.");

    const identity = await cdpInvoke(target.webSocketDebuggerUrl, "get_identity_status");
    if (!identity?.initialized) {
      await cdpInvoke(target.webSocketDebuggerUrl, "setup_identity", {
        creatorInput: `HiddenShield RC-MEDIA-004 ${runId}`,
      });
    }

    summary.cancellation = await runCancellationScenario(
      target.webSocketDebuggerUrl,
      child.pid,
    );
    writeSummary();
    assert(summary.cancellation.status === "passed", "Cancellation scenario failed.");

    summary.completion = await runCompletionScenario(
      target.webSocketDebuggerUrl,
      child.pid,
    );
    writeSummary();
    assert(summary.completion.status === "passed", "Completion scenario failed.");
  } finally {
    if (child.exitCode === null) child.kill();
  }

  summary.checks.gate = {
    passed:
      summary.cancellation?.status === "passed" &&
      summary.completion?.status === "passed",
    cancellationPassed: summary.cancellation?.status === "passed",
    completionPassed: summary.completion?.status === "passed",
  };
  assert(summary.checks.gate.passed, "RC-MEDIA-004 Gate failed.");
  summary.status = "passed";
  summary.finishedAt = new Date().toISOString();
  summary.elapsedMs =
    new Date(summary.finishedAt).getTime() - new Date(summary.startedAt).getTime();
  writeSummary();
  console.log(`Desktop audio upper-envelope Gate: ${summary.status}`);
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
} finally {
  rmSync(inputPath, { force: true });
  rmSync(cancellationInputPath, { force: true });
}

function generateFixture() {
  run(
    ffmpeg,
    [
      "-y",
      "-f",
      "lavfi",
      "-i",
      "sine=frequency=440:sample_rate=48000:duration=1200",
      "-f",
      "lavfi",
      "-i",
      "sine=frequency=997:sample_rate=48000:duration=1200",
      "-filter_complex",
      "[0:a]volume=0.3[left];[1:a]volume=0.3[right];[left][right]amerge=inputs=2[a]",
      "-map",
      "[a]",
      "-ac",
      "2",
      "-ar",
      "48000",
      "-c:a",
      "flac",
      "-sample_fmt",
      "s32",
      "-bits_per_raw_sample",
      "24",
      inputPath,
    ],
    10 * 60_000,
  );
}

async function runCancellationScenario(webSocketUrl, processId) {
  const startedAt = Date.now();
  const baselineMemory = sampleProcessMemory(processId);
  const result = {
    inputPath: relative(root, cancellationInputPath),
    baselineMemory,
    status: "running",
  };
  try {
    const sourceMeta = await cdpInvoke(
      webSocketUrl,
      "probe_source",
      { path: cancellationInputPath },
      120_000,
    );
    assertUpperEnvelopeMeta(sourceMeta, cancellationInputPath);
    result.sourceMeta = sourceMeta;

    const recordsBefore = await cdpInvoke(webSocketUrl, "list_vault_records");
    const priorIds = new Set((recordsBefore ?? []).map((candidate) => candidate.id));
    const pipeline = await cdpInvoke(webSocketUrl, "start_pipeline", {
      inputPath: cancellationInputPath,
      platforms: [],
      options: {
        aspectStrategy: "letterbox",
        encodingMode: "high_quality_cpu",
        allowRewrite: false,
      },
    });
    result.pipelineId = pipeline.pipelineId;

    const samples = [];
    await delay(1_000);
    samples.push(sampleProcessMemory(processId));
    const cancelStartedAt = Date.now();
    await cdpInvoke(webSocketUrl, "cancel_pipeline", {
      pipelineId: pipeline.pipelineId,
    });
    await waitUntilInactive(webSocketUrl, pipeline.pipelineId, 30_000);
    result.cancelAcknowledgeMs = Date.now() - cancelStartedAt;

    const quiescenceStartedAt = Date.now();
    const quiescenceDeadline = quiescenceStartedAt + 120_000;
    let consecutiveQuiescentSamples = 0;
    let previousSample = null;
    while (Date.now() < quiescenceDeadline) {
      const sample = sampleProcessMemory(processId);
      samples.push(sample);
      const rootCpuDelta =
        previousSample === null
          ? Number.POSITIVE_INFINITY
          : Number(sample.rootCpuSeconds ?? 0) -
            Number(previousSample.rootCpuSeconds ?? 0);
      const treeCpuDelta =
        previousSample === null
          ? Number.POSITIVE_INFINITY
          : Number(sample.treeCpuSeconds ?? 0) -
            Number(previousSample.treeCpuSeconds ?? 0);
      const quiescent =
        Number(sample.ffmpegProcessCount ?? 0) === 0 &&
        rootCpuDelta >= 0 &&
        rootCpuDelta <= 0.5 &&
        treeCpuDelta >= 0 &&
        treeCpuDelta <= 1.5;
      consecutiveQuiescentSamples = quiescent ? consecutiveQuiescentSamples + 1 : 0;
      previousSample = sample;
      if (consecutiveQuiescentSamples >= 3) break;
      await delay(1_000);
    }
    result.resources = summarizeMemorySamples(samples);
    assert(
      consecutiveQuiescentSamples >= 3,
      "Cancelled pipeline did not become CPU-quiescent within 120 seconds.",
    );
    result.workerQuiescenceMs = Date.now() - quiescenceStartedAt;
    result.workerQuiescenceThresholds = {
      rootCpuDeltaSeconds: 0.5,
      processTreeCpuDeltaSeconds: 1.5,
      ffmpegProcessCount: 0,
      consecutiveSamplesRequired: 3,
    };

    await delay(2_000);
    const recordsAfter = await cdpInvoke(webSocketUrl, "list_vault_records");
    const unexpectedRecord = (recordsAfter ?? []).find(
      (candidate) =>
        !priorIds.has(candidate.id) &&
        candidate.fileName === basename(cancellationInputPath),
    );
    assert(!unexpectedRecord, "Cancelled pipeline created a vault record.");

    result.status = "passed";
    result.vaultRecordCreated = false;
  } catch (error) {
    result.status = "failed";
    result.error = String(error?.stack ?? error);
  }
  result.elapsedMs = Date.now() - startedAt;
  return result;
}

async function runCompletionScenario(webSocketUrl, processId) {
  const startedAt = Date.now();
  const input = describeProbe(probe(inputPath), inputPath);
  const result = {
    inputPath: relative(root, inputPath),
    input,
    inputSha256: sha256(inputPath),
    status: "running",
  };
  try {
    const sourceMeta = await cdpInvoke(
      webSocketUrl,
      "probe_source",
      { path: inputPath },
      120_000,
    );
    assertUpperEnvelopeMeta(sourceMeta, inputPath);
    result.sourceMeta = sourceMeta;

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
    result.pipelineId = pipeline.pipelineId;
    result.resources = await waitForPipelineWithResources(
      webSocketUrl,
      pipeline.pipelineId,
      processId,
      15 * 60_000,
    );

    const records = await cdpInvoke(webSocketUrl, "list_vault_records");
    const record = [...(records ?? [])]
      .filter(
        (candidate) =>
          !priorIds.has(candidate.id) && candidate.fileName === basename(inputPath),
      )
      .sort((left, right) =>
        String(right.createdAt ?? "").localeCompare(String(left.createdAt ?? "")),
      )[0];
    assert(record, "No vault record found for completed upper-envelope pipeline.");
    assert(
      record.protectedCopyPath && existsSync(record.protectedCopyPath),
      "Completed pipeline did not produce a protected copy.",
    );
    assert(extname(record.protectedCopyPath).toLowerCase() === ".wav", "Protected copy is not WAV.");

    const output = describeProbe(probe(record.protectedCopyPath), record.protectedCopyPath);
    const specification = compareSpecification(input, output);
    assert(specification.passed, `Output specification changed: ${JSON.stringify(specification)}`);
    assert(record.writeVerificationStatus === "verified", "Installed write-after-read failed.");
    assert(record.payloadProtocolVersion === 3, "Installed record is not V3.");
    assert(record.payloadBytesLength === 39, "Installed record payload length is not 39.");

    const coreRead = runCoreRead(record.protectedCopyPath);
    assert(coreRead.passed, `Independent core read failed: ${coreRead.stderrTail}`);
    assert(coreRead.result?.watermarkUid === record.watermarkUid, "Independent core UID mismatch.");
    assert(coreRead.result?.payloadProtocolVersion === 3, "Independent core read is not V3.");

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

function assertUpperEnvelopeMeta(sourceMeta, path) {
  assert(sourceMeta?.fileType === "audio", "Installed probe did not classify fixture as audio.");
  assert(sourceMeta.fileSizeBytes === statSync(path).size, "Installed probe lost exact byte size.");
  assert(sourceMeta.durationSecs === 1200, "Installed probe changed the 20-minute duration.");
  assert(sourceMeta.sampleRate === 48_000, "Installed probe changed the sample rate.");
  assert(sourceMeta.channels === 2, "Installed probe changed the channel count.");
}

async function waitForPipelineWithResources(webSocketUrl, pipelineId, processId, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  const samples = [];
  while (Date.now() < deadline) {
    samples.push(sampleProcessMemory(processId));
    const active = await cdpInvoke(webSocketUrl, "check_active_pipelines");
    if (!active.includes(pipelineId)) return summarizeMemorySamples(samples);
    await delay(750);
  }
  throw new Error(`Pipeline ${pipelineId} did not finish within ${timeoutMs}ms.`);
}

async function waitUntilInactive(webSocketUrl, pipelineId, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const active = await cdpInvoke(webSocketUrl, "check_active_pipelines");
    if (!active.includes(pipelineId)) return;
    await delay(250);
  }
  throw new Error(`Cancelled pipeline ${pipelineId} remained active.`);
}

function sampleProcessMemory(processId) {
  const command = [
    "$rootId=",
    String(processId),
    ";$all=Get-CimInstance Win32_Process;",
    "$ids=New-Object 'System.Collections.Generic.HashSet[int]';",
    "[void]$ids.Add($rootId);",
    "do{$added=$false;foreach($p in $all){if($ids.Contains([int]$p.ParentProcessId)-and $ids.Add([int]$p.ProcessId)){$added=$true}}}while($added);",
    "$rows=@();foreach($id in $ids){$p=Get-Process -Id $id -ErrorAction SilentlyContinue;if($p){$rows+=[pscustomobject]@{pid=$id;name=$p.ProcessName;workingSetBytes=[int64]$p.WorkingSet64;cpuSeconds=[double]$p.CPU}}};",
    "$root=($rows|Where-Object pid -eq $rootId|Select-Object -First 1).workingSetBytes;",
    "$rootCpu=($rows|Where-Object pid -eq $rootId|Select-Object -First 1).cpuSeconds;",
    "$tree=($rows|Measure-Object workingSetBytes -Sum).Sum;",
    "$treeCpu=($rows|Measure-Object cpuSeconds -Sum).Sum;",
    "$ffmpegCount=@($rows|Where-Object name -eq 'ffmpeg').Count;",
    "$names=@($rows.name|Sort-Object -Unique);",
    "if($null -eq $root){$root=0};if($null -eq $tree){$tree=0};if($null -eq $rootCpu){$rootCpu=0};if($null -eq $treeCpu){$treeCpu=0};",
    "[pscustomobject]@{capturedAt=(Get-Date).ToUniversalTime().ToString('o');rootWorkingSetBytes=[int64]$root;treeWorkingSetBytes=[int64]$tree;rootCpuSeconds=[double]$rootCpu;treeCpuSeconds=[double]$treeCpu;ffmpegProcessCount=[int]$ffmpegCount;processCount=$rows.Count;processNames=$names}|ConvertTo-Json -Compress",
  ].join("");
  const result = spawnSync("powershell.exe", ["-NoProfile", "-Command", command], {
    cwd: root,
    encoding: "utf8",
    timeout: 20_000,
  });
  if (result.status !== 0 || !result.stdout.trim()) {
    return {
      capturedAt: new Date().toISOString(),
      rootWorkingSetBytes: 0,
      treeWorkingSetBytes: 0,
      rootCpuSeconds: 0,
      treeCpuSeconds: 0,
      ffmpegProcessCount: 0,
      processCount: 0,
      error: tail(result.stderr),
    };
  }
  return JSON.parse(result.stdout);
}

function summarizeMemorySamples(samples) {
  return {
    sampleCount: samples.length,
    peakRootWorkingSetBytes: Math.max(
      0,
      ...samples.map((sample) => Number(sample.rootWorkingSetBytes ?? 0)),
    ),
    peakProcessTreeWorkingSetBytes: Math.max(
      0,
      ...samples.map((sample) => Number(sample.treeWorkingSetBytes ?? 0)),
    ),
    peakProcessCount: Math.max(
      0,
      ...samples.map((sample) => Number(sample.processCount ?? 0)),
    ),
    peakFfmpegProcessCount: Math.max(
      0,
      ...samples.map((sample) => Number(sample.ffmpegProcessCount ?? 0)),
    ),
    finalRootWorkingSetBytes: Number(samples.at(-1)?.rootWorkingSetBytes ?? 0),
    finalProcessTreeWorkingSetBytes: Number(samples.at(-1)?.treeWorkingSetBytes ?? 0),
    samples,
  };
}

function runCoreRead(path) {
  const command = spawnSync(coreReader, [path], {
    cwd: root,
    encoding: "utf8",
    timeout: 10 * 60_000,
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

function compareSpecification(input, output) {
  const fields = ["durationSeconds", "sampleRate", "channels", "effectiveBitDepth"];
  const differences = Object.fromEntries(
    fields
      .filter((field) => input[field] !== output[field])
      .map((field) => [field, { input: input[field], output: output[field] }]),
  );
  return { passed: Object.keys(differences).length === 0, fields, differences };
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
