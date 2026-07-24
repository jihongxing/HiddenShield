import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve, relative } from 'node:path';

const runId = process.env.HIDDENSHIELD_CLOUD_SYNC_RUNTIME_RUN_ID ?? `${Date.now()}`;
const baseUrl = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const outputDir = resolve('tmp-ui-qa/cloud-sync-runtime');
mkdirSync(outputDir, { recursive: true });

const artifacts = {
  desktop: join(outputDir, `desktop-installer-sync-runtime-${runId}.json`),
  android: join(outputDir, `android-native-sync-runtime-${runId}.json`),
  network: join(outputDir, `network-resume-sync-runtime-${runId}.json`),
  eventDisposition: join(outputDir, `event-disposition-sync-runtime-${runId}.json`),
};

await waitForHealth();

const eventDisposition = await runEventDispositionQa();
writeJson(artifacts.eventDisposition, eventDisposition);

const desktop = runDesktopInstallerQa();
writeJson(artifacts.desktop, desktop);

const android = runAndroidNativeQa();
writeJson(artifacts.android, android);

const network = runNetworkResumeQa();
writeJson(artifacts.network, network);

console.log(`Desktop artifact: ${artifacts.desktop}`);
console.log(`Android artifact: ${artifacts.android}`);
console.log(`Network artifact: ${artifacts.network}`);
console.log(`Event disposition artifact: ${artifacts.eventDisposition}`);

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

async function waitForHealth() {
  const deadline = Date.now() + 120_000;
  let lastError = '';
  while (Date.now() < deadline) {
    try {
      const health = await request('GET', '/v1/health');
      if (health.status === 200 && health.body?.ok === true && health.body?.cloudSync === true) {
        return;
      }
      lastError = `unexpected health response ${health.status}`;
    } catch (error) {
      lastError = String(error);
    }
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }
  throw new Error(`feedback-backend did not become healthy: ${lastError}`);
}

async function runEventDispositionQa() {
  const startedAt = new Date().toISOString();
  const identifier = `event-disposition-${runId}@hiddenshield.local`;
  const password = `qa-${runId}`;
  const desktop = await continueAccount({
    identifier,
    password,
    deviceId: `desktop-event-${runId}`,
    name: 'Desktop Event Disposition QA',
    platform: 'windows',
  });
  await upgradeToCreator(desktop);
  const session = await continueAccount({
    identifier,
    password,
    deviceId: `desktop-event-${runId}`,
    name: 'Desktop Event Disposition QA',
    platform: 'windows',
  });
  const event = {
    clientEventId: `event-disposition-${runId}`,
    operation: 'upsertVaultRecord',
    entityType: 'vaultRecord',
    entityId: `record-event-disposition-${runId}`,
    payload: {
      id: `record-event-disposition-${runId}`,
      kind: 'image',
      title: 'event-disposition.png',
      watermark_uid: `HS-${runId.slice(-8).padStart(8, '0')}-11111111-22222222-33333333`,
      revision: 1,
      sha256: `sha256:event-disposition-${runId}`,
      created_at: new Date().toISOString(),
    },
  };
  assertNoForbiddenPayloadFields(event.payload);

  const first = await pushBatch(session, [event]);
  assertDisposition(first.body, 'accepted');
  const duplicate = await pushBatch(session, [event]);
  assertDisposition(duplicate.body, 'duplicate');
  const changed = structuredClone(event);
  changed.payload = { ...changed.payload, revision: 2 };
  const conflict = await pushBatch(session, [changed]);
  assertDisposition(conflict.body, 'conflict_payload_changed');
  assert(
    !conflict.body.acceptedEventIds?.includes(event.clientEventId),
    'changed payload conflict must not be accepted',
  );

  const postgresArtifact = latestArtifact('tmp-ui-qa/postgres-sync-runtime', 'cloud-sync-postgres-runtime-qa-');
  return {
    schemaVersion: 'cloud_sync_event_disposition_runtime_qa_v1',
    runId,
    generatedAt: new Date().toISOString(),
    startedAt,
    completedAt: new Date().toISOString(),
    ok: true,
    backendBaseUrl: baseUrl,
    sqliteDevAdapter: {
      accepted: first.body.eventResults?.[0] ?? null,
      duplicate: duplicate.body.eventResults?.[0] ?? null,
      conflict: conflict.body.eventResults?.[0] ?? null,
      acceptedEventIdsAfterConflict: conflict.body.acceptedEventIds ?? [],
    },
    postgresDisposableArtifact: postgresArtifact
      ? relative(process.cwd(), postgresArtifact)
      : 'not_rerun_in_this_script_latest_artifact_missing',
    privacyBoundary:
      'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  };
}

function runDesktopInstallerQa() {
  const installer = findNewestDesktopInstaller();
  const exe = findNewestDesktopExe();
  const installedExe = findInstalledDesktopExe();
  const processSmoke = exe ? launchDesktopExeSmoke(exe) : null;
  const installedProcessSmoke = installedExe ? launchDesktopExeSmoke(installedExe) : null;
  const automation = runDesktopAutomationQa(installedExe ?? exe);
  const ok = automation?.ok === true;
  return {
    schemaVersion: 'cloud_sync_desktop_installer_runtime_qa_v1',
    runId,
    generatedAt: new Date().toISOString(),
    ok,
    status: ok ? 'ready' : 'blocked',
    installerPath: installer ? relative(process.cwd(), installer) : null,
    executablePath: exe ? relative(process.cwd(), exe) : null,
    installedExecutablePath: installedExe ?? null,
    processSmoke,
    installedProcessSmoke,
    automation,
    completedChecks: {
      feedbackBackendHealthy: true,
      latestInstallerLocated: Boolean(installer),
      latestExecutableLocated: Boolean(exe),
      installedExecutableLocated: Boolean(installedExe),
      desktopProcessLaunchSmoke: processSmoke?.launched === true,
      installedDesktopProcessLaunchSmoke: installedProcessSmoke?.launched === true,
      automationChannelCompleted: automation?.ok === true,
      creatorPullFlushPull: automation?.completedChecks?.creatorPeerPullReceived === true,
      freeBlockedByEntitlement: automation?.completedChecks?.freeBlockedByEntitlement === true,
      queueDiagnosticsExported: automation?.completedChecks?.queueDiagnosticsExported === true,
      privacyWhitelistEnforced: automation?.completedChecks?.privacyWhitelistEnforced === true,
    },
    creatorPullFlushPull: automation?.creatorPullFlushPull ?? null,
    freeBlockedByEntitlement: automation?.freeBlockedByEntitlement ?? null,
    queueDiagnostics: automation?.queueDiagnostics ?? null,
    privacy: automation?.privacy ?? null,
    missingChecks: ok
      ? []
      : [
          'Desktop installed automation channel did not complete Creator pull/flush/pull, Free blocked_by_entitlement, queue diagnostics, and privacy whitelist assertions.',
        ],
    privacyBoundary:
      'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  };
}

function runAndroidNativeQa() {
  const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
  const devices = safeExec('adb', ['devices']);
  const deviceOnline = devices.includes(`${adbSerial}\tdevice`);
  const screenshotPath = join(outputDir, `android-native-sync-runtime-${runId}.png`);
  let screenshotCaptured = false;
  let flutterRun = {
    attempted: false,
    status: 'not_attempted_device_offline',
    stdoutTail: '',
    stderrTail: '',
  };
  let nativeArtifact = null;
  if (deviceOnline) {
    safeExec('adb', ['-s', adbSerial, 'reverse', 'tcp:43188', 'tcp:43188']);
    const runResult = runFlutterMobileQa(adbSerial);
    flutterRun = runResult.flutterRun;
    nativeArtifact = runResult.artifact;
    const png = spawnSync('adb', ['-s', adbSerial, 'exec-out', 'screencap', '-p'], {
      encoding: 'buffer',
      timeout: 30_000,
    });
    if (png.status === 0 && png.stdout?.length > 0) {
      writeFileSync(screenshotPath, png.stdout);
      screenshotCaptured = true;
    }
  }
  const ok = nativeArtifact?.ok === true;
  return {
    schemaVersion: 'cloud_sync_android_native_runtime_qa_v1',
    runId,
    generatedAt: new Date().toISOString(),
    ok,
    status: ok ? 'ready' : 'blocked',
    adbSerial,
    deviceOnline,
    flutterTool: 'mobile_app/tool/cloud_sync_runtime_qa.dart',
    flutterRun,
    nativeArtifact,
    screenshotPath: screenshotCaptured ? relative(process.cwd(), screenshotPath) : null,
    completedChecks: {
      adbDeviceOnline: deviceOnline,
      backendReverseConfigured: deviceOnline,
      nativeFlutterToolAttempted: flutterRun.attempted,
      screenshotCaptured,
      nativeRunnerCompleted: nativeArtifact?.completedChecks?.nativeRunnerCompleted === true,
      creatorPullFlushPull: nativeArtifact?.completedChecks?.creatorPeerPullReceived === true,
      freeBlockedByEntitlement: nativeArtifact?.completedChecks?.freeBlockedByEntitlement === true,
      queueDiagnosticsExported: nativeArtifact?.completedChecks?.queueDiagnosticsExported === true,
      privacyWhitelistEnforced: nativeArtifact?.completedChecks?.privacyWhitelistEnforced === true,
    },
    creatorPullFlushPull: nativeArtifact?.creatorPullFlushPull ?? null,
    freeBlockedByEntitlement: nativeArtifact?.freeBlockedByEntitlement ?? null,
    queueDiagnostics: nativeArtifact?.queueDiagnostics ?? null,
    privacy: nativeArtifact?.privacy ?? null,
    missingChecks: ok
      ? []
      : [
          'Android native cloud sync runner did not complete Creator pull/flush/pull, Free blocked_by_entitlement, queue diagnostics, and privacy whitelist assertions.',
        ],
    privacyBoundary:
      'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  };
}

function runNetworkResumeQa() {
  const desktopArtifact = readJson(artifacts.desktop);
  const androidArtifact = readJson(artifacts.android);
  const eventArtifact = readJson(artifacts.eventDisposition);
  const ok = desktopArtifact.ok === true && androidArtifact.ok === true && eventArtifact.ok === true;
  return {
    schemaVersion: 'cloud_sync_network_resume_runtime_qa_v1',
    runId,
    generatedAt: new Date().toISOString(),
    ok,
    status: ok ? 'ready' : 'blocked',
    completedChecks: {
      desktopQueueRecoveredStale:
        desktopArtifact.queueDiagnostics?.recoveredStale === 1,
      desktopSyncedItemNotRetransmitted:
        desktopArtifact.queueDiagnostics?.creatorAfterFlush?.synced === 1,
      androidQueueRecoveredStale:
        androidArtifact.queueDiagnostics?.recoveredStale === 1,
      androidSyncedItemNotRetransmitted:
        androidArtifact.queueDiagnostics?.creatorAfterFlush?.synced === 1,
      backendEventDispositionRuntime: relative(process.cwd(), artifacts.eventDisposition),
      automationMode:
        'installed_desktop_and_android_runner_queue_lifecycle_simulation_no_os_network_toggle',
    },
    desktopArtifact: relative(process.cwd(), artifacts.desktop),
    androidArtifact: relative(process.cwd(), artifacts.android),
    missingChecks: ok
      ? []
      : [
          'Network/lifecycle readiness requires both desktop installed automation and Android native runner artifacts to be ready.',
        ],
    privacyBoundary:
      'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url',
  };
}

async function continueAccount({ identifier, password, deviceId, name, platform }) {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name,
      platform,
      appVersion: 'cloud-sync-runtime-evidence',
    },
    localCreatorProfile: {
      displayName: 'Cloud Sync Runtime QA',
      creatorSeedRef: `qa-seed-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `${name} auth/sessions failed: ${response.status}`);
  return response.body;
}

async function upgradeToCreator(session) {
  const payment = await request(
    'POST',
    '/v1/billing/payment-sessions',
    {
      accountId: session.account.id,
      workspaceId: session.workspace.id,
      planCode: 'creator',
      billingCycle: 'monthly',
      preferredProvider: 'fixture',
    },
    session.accessToken,
  );
  assert(payment.status === 200, `fixture creator payment failed: ${payment.status}`);
  const reconcile = await request(
    'POST',
    `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
    {},
    session.accessToken,
  );
  assert(reconcile.status === 200, `fixture creator reconcile failed: ${reconcile.status}`);
}

async function pushBatch(session, events) {
  const response = await request(
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: session.workspace.id,
      events,
    },
    session.accessToken,
  );
  assert(response.status === 200, `events:batch failed: ${response.status}`);
  return response;
}

function assertDisposition(body, disposition) {
  const actual = body.eventResults?.[0]?.disposition;
  assert(actual === disposition, `expected disposition ${disposition}, got ${actual}`);
  if (disposition !== 'conflict_payload_changed') {
    assert(
      body.eventResults?.[0]?.payloadHash?.startsWith('sha256:'),
      `${disposition} must return payloadHash`,
    );
  }
}

async function request(method, path, body = null, token = null) {
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers: {
      ...(body ? { 'content-type': 'application/json' } : {}),
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = text;
  }
  return { status: response.status, body: parsed };
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertNoForbiddenPayloadFields(payload) {
  for (const key of [
    'originalPath',
    'original_path',
    'protectedCopyPath',
    'protected_copy_path',
    'localPath',
    'local_path',
    'objectRef',
    'object_ref',
    'signedUrl',
    'signed_url',
    'mediaBytes',
    'media_bytes',
  ]) {
    assert(!(key in payload), `forbidden sync payload key present: ${key}`);
  }
}

function findNewestDesktopInstaller() {
  const candidates = findFiles('src-tauri/target/release/bundle', /\.(msi|exe)$/i);
  return newest(candidates);
}

function findNewestDesktopExe() {
  const candidates = findFiles('src-tauri/target/release', /^hidden_shield\.exe$/i)
    .filter((file) => !file.includes(`${join('target', 'release', 'build')}`));
  return newest(candidates);
}

function findInstalledDesktopExe() {
  const registry = safeExec('powershell', [
    '-NoProfile',
    '-Command',
    `$items=Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKLM:\\Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'HiddenShield' }; $paths=@(); foreach ($item in $items) { if ($item.DisplayIcon) { $paths += ($item.DisplayIcon -replace '^"|"$','') }; if ($item.InstallLocation) { $loc=($item.InstallLocation -replace '^"|"$',''); $paths += (Join-Path $loc 'hidden_shield.exe') } }; $paths | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1`,
  ]).trim();
  return registry || null;
}

function runDesktopAutomationQa(exe) {
  if (!exe) return null;
  const automationArtifact = join(outputDir, `desktop-installed-cloud-sync-automation-${runId}.json`);
  const result = spawnSync(
    exe,
    ['--cloud-sync-runtime-qa', automationArtifact],
    {
      encoding: 'utf8',
      timeout: 180_000,
      env: {
        ...process.env,
        HIDDENSHIELD_DESKTOP_CLOUD_SYNC_QA_ARTIFACT: automationArtifact,
        HIDDENSHIELD_CLOUD_SYNC_QA_BACKEND_URL: baseUrl,
        HIDDENSHIELD_CLOUD_SYNC_RUNTIME_RUN_ID: runId,
      },
      windowsHide: true,
    },
  );
  let artifact = null;
  if (existsSync(automationArtifact)) {
    try {
      artifact = readJson(automationArtifact);
    } catch (error) {
      artifact = { ok: false, status: 'invalid_json', error: String(error) };
    }
  }
  return {
    ...(artifact ?? { ok: false, status: 'missing_artifact' }),
    automationArtifact: relative(process.cwd(), automationArtifact),
    process: {
      attempted: true,
      executable: exe,
      exitCode: result.status,
      signal: result.signal,
      error: result.error ? String(result.error) : null,
      stdoutTail: tail(result.stdout),
      stderrTail: tail(result.stderr),
    },
  };
}

function findFiles(root, pattern) {
  const abs = resolve(root);
  if (!existsSync(abs)) return [];
  const out = [];
  const stack = [abs];
  while (stack.length) {
    const dir = stack.pop();
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) stack.push(path);
      else if (pattern.test(entry.name)) out.push(path);
    }
  }
  return out;
}

function newest(files) {
  return files
    .map((file) => ({ file, mtime: statMtime(file) }))
    .sort((a, b) => b.mtime - a.mtime)[0]?.file ?? null;
}

function statMtime(file) {
  try {
    return execFileSync('powershell', [
      '-NoProfile',
      '-Command',
      `(Get-Item -LiteralPath ${JSON.stringify(file)}).LastWriteTimeUtc.Ticks`,
    ], { encoding: 'utf8' }).trim();
  } catch {
    return '0';
  }
}

function launchDesktopExeSmoke(exe) {
  try {
    const child = spawnSync('powershell', [
      '-NoProfile',
      '-Command',
      `$p=Start-Process -FilePath ${JSON.stringify(exe)} -PassThru; Start-Sleep -Seconds 8; $alive=Get-Process -Id $p.Id -ErrorAction SilentlyContinue; if ($alive) { Stop-Process -Id $p.Id -Force }; [pscustomobject]@{pid=$p.Id; alive=[bool]$alive} | ConvertTo-Json`,
    ], { encoding: 'utf8', timeout: 30_000 });
    const parsed = JSON.parse(child.stdout.trim());
    return {
      launched: child.status === 0,
      processObserved: parsed.alive === true,
      pid: parsed.pid ?? null,
      stdout: child.stdout.trim(),
      stderr: child.stderr.trim(),
    };
  } catch (error) {
    return {
      launched: false,
      processObserved: false,
      error: String(error),
    };
  }
}

function runFlutterMobileQa(adbSerial) {
  const deviceArtifactName = `cloud_sync_runtime_qa_${runId}.json`;
  const deviceArtifactPath = `/data/user/0/com.hiddenshield.hidden_shield_mobile/files/${deviceArtifactName}`;
  const hostArtifactPath = join(outputDir, `android-native-cloud-sync-runner-${runId}.json`);
  const flutterArgs = [
    'run',
    '-d',
    adbSerial,
    '-t',
    'tool/cloud_sync_runtime_qa.dart',
    '--dart-define',
    'HIDDENSHIELD_QA_BACKEND_URL=http://10.0.2.2:43188',
    '--dart-define',
    `HIDDENSHIELD_CLOUD_SYNC_ANDROID_QA_ARTIFACT_PATH=${deviceArtifactPath}`,
    '--dart-define',
    `HIDDENSHIELD_CLOUD_SYNC_RUNTIME_RUN_ID=${runId}`,
  ];
  const command = process.platform === 'win32' ? 'cmd.exe' : 'flutter';
  const args = process.platform === 'win32' ? ['/c', 'flutter', ...flutterArgs] : flutterArgs;
  const result = spawnSync(
    command,
    args,
    {
      cwd: resolve('mobile_app'),
      encoding: 'utf8',
      timeout: 240_000,
    },
  );
  let artifact = null;
  const pulled = spawnSync(
    'adb',
    [
      '-s',
      adbSerial,
      'exec-out',
      'run-as',
      'com.hiddenshield.hidden_shield_mobile',
      'cat',
      `files/${deviceArtifactName}`,
    ],
    {
      encoding: 'utf8',
      timeout: 30_000,
    },
  );
  if (pulled.status === 0 && pulled.stdout.trim().startsWith('{')) {
    writeFileSync(hostArtifactPath, pulled.stdout, 'utf8');
    try {
      artifact = JSON.parse(pulled.stdout);
    } catch (error) {
      artifact = { ok: false, status: 'invalid_json', error: String(error) };
    }
  }
  return {
    artifact: artifact
      ? {
          ...artifact,
          hostArtifactPath: relative(process.cwd(), hostArtifactPath),
          deviceArtifactPath,
        }
      : null,
    flutterRun: {
    attempted: true,
    status: result.status === 0 ? 'completed' : 'failed_or_timed_out',
    exitCode: result.status,
    signal: result.signal,
    error: result.error ? String(result.error) : null,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
    },
  };
}

function safeExec(command, args) {
  try {
    return execFileSync(command, args, { encoding: 'utf8' });
  } catch (error) {
    return `${error.stdout ?? ''}\n${error.stderr ?? ''}\n${error.message ?? error}`;
  }
}

function latestArtifact(dir, prefix) {
  const abs = resolve(dir);
  if (!existsSync(abs)) return null;
  return newest(
    readdirSync(abs)
      .filter((name) => name.startsWith(prefix) && name.endsWith('.json'))
      .map((name) => join(abs, name)),
  );
}

function tail(value, max = 3000) {
  const text = String(value ?? '');
  return text.length > max ? text.slice(-max) : text;
}
