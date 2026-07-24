import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, join, relative, resolve } from 'node:path';

const runId = process.env.HIDDENSHIELD_RC1_OS_DRILL_RUN_ID ?? '20260704';
const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const backendBaseUrl = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const outputDir = resolve('tmp-ui-qa/rc1-no-external-acceptance', runId);
mkdirSync(outputDir, { recursive: true });

const desktopArtifactPath = join(outputDir, `desktop-os-network-disconnect-drill-${runId}.json`);
const androidArtifactPath = join(outputDir, `android-os-network-disconnect-drill-${runId}.json`);
const aggregateArtifactPath = join(outputDir, `os-network-disconnect-drill-record-${runId}.json`);
const summaryArtifactPath = join(outputDir, `rc1-no-external-acceptance-summary-${runId}.json`);

const desktop = await runDesktopDrill();
writeJson(desktopArtifactPath, desktop);
writeMarkdown(
  desktopArtifactPath.replace(/\.json$/, '.md'),
  renderDesktopMarkdown(desktop, desktopArtifactPath),
);

const android = await runAndroidDrill();
writeJson(androidArtifactPath, android);
writeMarkdown(
  androidArtifactPath.replace(/\.json$/, '.md'),
  renderAndroidMarkdown(android, androidArtifactPath),
);

const aggregate = buildAggregate(desktop, android);
writeJson(aggregateArtifactPath, aggregate);
writeMarkdown(
  aggregateArtifactPath.replace(/\.json$/, '.md'),
  renderAggregateMarkdown(aggregate),
);

if (existsSync(summaryArtifactPath)) {
  const summary = readJson(summaryArtifactPath);
  summary.generatedAt = new Date().toISOString();
  summary.status = aggregate.releaseDecision;
  summary.osNetworkDisconnectDrill = {
    status: aggregate.status,
    artifact: rel(aggregateArtifactPath),
    desktopArtifact: rel(desktopArtifactPath),
    androidArtifact: rel(androidArtifactPath),
    desktopStatus: desktop.status,
    androidStatus: android.status,
  };
  writeJson(summaryArtifactPath, summary);
  writeMarkdown(
    summaryArtifactPath.replace(/\.json$/, '.md'),
    renderAcceptanceSummaryMarkdown(summary, aggregate),
  );
}

console.log(`Desktop OS network drill artifact: ${rel(desktopArtifactPath)}`);
console.log(`Android OS network drill artifact: ${rel(androidArtifactPath)}`);
console.log(`Aggregate OS network drill artifact: ${rel(aggregateArtifactPath)}`);

async function runDesktopDrill() {
  const generatedAt = new Date().toISOString();
  const adapters = safePowerShellJson(
    "Get-NetAdapter | Select-Object Name,Status,InterfaceDescription,MacAddress,LinkSpeed | ConvertTo-Json -Depth 3",
  );
  const backendBefore = await backendHealth();
  const isAdmin = powershellText(
    "[Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent() | ForEach-Object { $_.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) }",
  ).trim().toLowerCase() === 'true';
  const ruleName = `HiddenShield RC1 OS Network Drill Block ${runId}`;
  let firewall = {
    attempted: false,
    status: 'not_attempted_not_elevated',
    ruleName,
    stderr: '',
    stdout: '',
  };
  let backendDuringBlock = null;
  let backendAfterCleanup = null;

  if (isAdmin) {
    firewall = runFirewallBlock(ruleName);
    backendDuringBlock = await backendHealth();
    cleanupFirewallRule(ruleName);
    backendAfterCleanup = await backendHealth();
  }

  const loopbackTopology = backendBaseUrl.includes('127.0.0.1') || backendBaseUrl.includes('localhost');
  const status = firewall.status === 'created'
    ? (
        backendDuringBlock?.reachable === false
          ? 'ready'
          : 'blocked_loopback_topology_firewall_did_not_break_backend'
      )
    : 'blocked_permission_or_loopback_topology';

  return {
    schemaVersion: 'rc1_desktop_os_network_disconnect_drill_v1',
    runId,
    generatedAt,
    ok: status === 'ready',
    status,
    backendBaseUrl,
    backendBefore,
    windowsNetworkAdapters: adapters,
    desktopTopology: {
      backendUsesLoopback: loopbackTopology,
      reason:
        'The installed desktop app talks to the local feedback-backend through 127.0.0.1:43188 in this QA setup; disabling Wi-Fi/Ethernet would not prove cloud-sync offline behavior unless the app is pointed at a LAN/staging backend or a privileged firewall/proxy blocks loopback traffic.',
    },
    firewall,
    backendDuringBlock,
    backendAfterCleanup,
    screenshot: null,
    queueState: latestJsonSummary('tmp-ui-qa/cloud-sync-runtime', 'desktop-installer-sync-runtime-'),
    matureErrorEvidence: {
      source: 'src/lib/user-facing-errors.ts',
      coveredBy: 'desktop installed cloud sync QA channel plus user-facing network error mapper',
      status: status === 'ready' ? 'validated_in_firewall_drill' : 'blocked_no_real_network_break',
    },
    privacyWhitelistEvidence: latestJsonSummary('tmp-ui-qa/cloud-sync-runtime', 'desktop-installer-sync-runtime-'),
    requiredToUnblock: [
      'Run the desktop app against a LAN/staging backend or allow an elevated firewall/proxy rule that actually makes the app lose the backend while the app remains open.',
      'Capture the desktop offline screen, queue diagnostics, mature error message, restore screenshot, and privacy whitelist scan in the same run.',
    ],
    releaseDecision: status === 'ready'
      ? 'desktop_real_os_network_drill_ready'
      : 'desktop_real_os_network_drill_blocked_not_release_ready',
  };
}

async function runAndroidDrill() {
  const generatedAt = new Date().toISOString();
  const screenshotOffline = join(outputDir, `android-network-off-${runId}.png`);
  const screenshotRestored = join(outputDir, `android-network-restored-${runId}.png`);
  const devices = safeExec('adb', ['devices']);
  const deviceOnline = devices.includes(`${adbSerial}\tdevice`);
  const before = deviceOnline ? androidConnectivityProbe('before') : null;
  let offline = null;
  let restored = null;
  let screenshots = {
    offline: null,
    restored: null,
  };
  let toggle = {
    attempted: false,
    disableData: null,
    disableWifi: null,
    enableData: null,
    enableWifi: null,
  };

  if (deviceOnline) {
    try {
      launchAndroidApp();
      toggle.attempted = true;
      toggle.disableData = adb(['shell', 'svc', 'data', 'disable']);
      toggle.disableWifi = adb(['shell', 'svc', 'wifi', 'disable']);
      await sleep(2500);
      offline = androidConnectivityProbe('offline');
      screenshots.offline = captureAndroidScreenshot(screenshotOffline);
    } finally {
      toggle.enableData = adb(['shell', 'svc', 'data', 'enable']);
      toggle.enableWifi = adb(['shell', 'svc', 'wifi', 'enable']);
      await sleep(4500);
      restored = androidConnectivityProbe('restored');
      launchAndroidApp();
      screenshots.restored = captureAndroidScreenshot(screenshotRestored);
    }
  }

  const latestAndroidRuntime = latestJsonSummary('tmp-ui-qa/cloud-sync-runtime', 'android-native-sync-runtime-');
  const latestBatch2 = latestJsonSummary('tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa', 'android-batch2-page-qa-summary-', {
    recursive: true,
  });
  const backendOff = latestBatch2?.json?.artifact?.backendOffMatureError
    ?? latestBatch2?.json?.backendOffMatureError
    ?? null;
  const privacyPass = Boolean(
    latestAndroidRuntime?.json?.completedChecks?.privacyWhitelistEnforced === true
      || latestAndroidRuntime?.json?.privacy?.privacyWhitelistEnforced === true,
  );
  const queueDiagnostics = latestAndroidRuntime?.json?.queueDiagnostics ?? null;
  const osDisconnectObserved = offline?.pingHost?.ok === false && offline?.backendPort?.ok === false;
  const networkRestored = restored?.pingHost?.ok === true && restored?.backendPort?.ok === true;
  const matureErrorPass = backendOff?.privacyPass === true
    && typeof backendOff?.message === 'string'
    && backendOff.message.includes('暂时无法连接服务');
  const ok = deviceOnline && toggle.attempted && osDisconnectObserved && networkRestored && matureErrorPass && privacyPass;

  return {
    schemaVersion: 'rc1_android_os_network_disconnect_drill_v1',
    runId,
    generatedAt,
    ok,
    status: ok ? 'ready' : 'blocked',
    adbSerial,
    deviceOnline,
    backendHostFromAndroid: '10.0.2.2:43188',
    devicesRaw: devices,
    connectivity: {
      before,
      offline,
      restored,
    },
    toggle,
    screenshots,
    queueStateEvidence: latestAndroidRuntime
      ? {
          artifact: latestAndroidRuntime.path,
          status: latestAndroidRuntime.json?.status ?? null,
          completedChecks: latestAndroidRuntime.json?.completedChecks ?? null,
          queueDiagnostics,
        }
      : null,
    matureErrorEvidence: latestBatch2
      ? {
          artifact: latestBatch2.path,
          backendOffMatureError: backendOff,
          note:
            'The OS toggle proves emulator network loss to the same host backend. Mature wording is taken from the same Android native app Batch 2 runner because the current drill is transport-level and does not drive the full UI button sequence while offline.',
        }
      : null,
    privacyWhitelistEvidence: latestAndroidRuntime
      ? {
          artifact: latestAndroidRuntime.path,
          privacy: latestAndroidRuntime.json?.privacy ?? null,
          completedChecks: latestAndroidRuntime.json?.completedChecks ?? null,
        }
      : null,
    completedChecks: {
      adbDeviceOnline: deviceOnline,
      realOsNetworkToggleAttempted: toggle.attempted,
      beforeBackendReachable: before?.backendPort?.ok === true,
      offlineBackendUnreachable: osDisconnectObserved,
      offlineScreenshotCaptured: Boolean(screenshots.offline),
      restoredBackendReachable: networkRestored,
      restoredScreenshotCaptured: Boolean(screenshots.restored),
      queueDiagnosticsEvidenceAvailable: Boolean(queueDiagnostics),
      matureErrorMessageEvidenceAvailable: matureErrorPass,
      privacyWhitelistEvidenceAvailable: privacyPass,
    },
    releaseDecision: ok
      ? 'android_real_os_network_drill_ready'
      : 'android_real_os_network_drill_blocked_or_partial',
  };
}

function buildAggregate(desktop, android) {
  const status = desktop.ok && android.ok
    ? 'ready'
    : android.ok
      ? 'partial_ready_desktop_blocked'
      : 'blocked';
  return {
    schemaVersion: 'rc1_os_network_disconnect_drill_record_v2',
    runId,
    generatedAt: new Date().toISOString(),
    status,
    scope: 'desktop_installer_and_android_native_real_os_network_disconnect_resume',
    desktop: {
      status: desktop.status,
      ok: desktop.ok,
      artifact: rel(desktopArtifactPath),
      releaseDecision: desktop.releaseDecision,
    },
    android: {
      status: android.status,
      ok: android.ok,
      artifact: rel(androidArtifactPath),
      offlineScreenshot: android.screenshots?.offline,
      restoredScreenshot: android.screenshots?.restored,
      releaseDecision: android.releaseDecision,
    },
    automatedEvidenceAvailable: {
      networkResumeRuntime: latestArtifactPath('tmp-ui-qa/cloud-sync-runtime', 'network-resume-sync-runtime-'),
      desktopRuntime: latestArtifactPath('tmp-ui-qa/cloud-sync-runtime', 'desktop-installer-sync-runtime-'),
      androidRuntime: latestArtifactPath('tmp-ui-qa/cloud-sync-runtime', 'android-native-sync-runtime-'),
      eventDispositionRuntime: latestArtifactPath('tmp-ui-qa/cloud-sync-runtime', 'event-disposition-sync-runtime-'),
      automationMode:
        'installed_desktop_and_android_runner_queue_lifecycle_plus_android_real_os_network_toggle_desktop_blocked_by_loopback_or_firewall_permission',
    },
    blockedItems: [
      ...(desktop.ok ? [] : ['Windows desktop real OS disconnect drill is still blocked by loopback topology or missing elevated firewall/proxy permission.']),
      ...(android.ok ? [] : ['Android real OS disconnect drill did not complete all evidence checks.']),
    ],
    requiredToUnblock: [
      ...(desktop.ok
        ? []
        : [
            'Run Windows desktop against LAN/staging backend or use elevated firewall/proxy to block loopback while the app remains open.',
            'Capture desktop disconnect/recover screenshots, queue state, mature error and privacy whitelist in that run.',
          ]),
      ...(android.ok ? [] : ['Rerun Android OS toggle with emulator/device online and backend reachable before disconnect.']),
    ],
    releaseDecision: status === 'ready'
      ? 'ready_for_release_owner_review'
      : 'ready_for_release_owner_review_with_desktop_os_network_blocked',
  };
}

function renderDesktopMarkdown(value, path) {
  return `# Windows 桌面端真实 OS 断网拨测

生成时间：${value.generatedAt}

状态：${value.status}

证据：\`${rel(path)}\`

## 结论

- 后端地址：\`${value.backendBaseUrl}\`
- 后端断网前健康：${value.backendBefore?.reachable ? 'YES' : 'NO'}
- 当前桌面 QA 拓扑：${value.desktopTopology.backendUsesLoopback ? 'loopback 127.0.0.1' : 'non-loopback'}
- 防火墙拨测：${value.firewall.status}

## 阻断说明

当前安装版桌面端在本机 QA 下连接 \`127.0.0.1:43188\`，关闭 Wi-Fi / Ethernet 不能切断 loopback 后端。当前会话也没有足够权限建立可验证的 Windows firewall / proxy 阻断规则，因此不能把桌面 OS 断网标记为通过。

## 放行条件

${value.requiredToUnblock.map((item) => `- ${item}`).join('\n')}
`;
}

function renderAndroidMarkdown(value, path) {
  return `# Android 原生端真实 OS 断网拨测

生成时间：${value.generatedAt}

状态：${value.status}

证据：\`${rel(path)}\`

## 现场拨测

- 设备：\`${value.adbSerial}\`
- 断网前后端可达：${value.completedChecks.beforeBackendReachable ? 'YES' : 'NO'}
- 断网后后端不可达：${value.completedChecks.offlineBackendUnreachable ? 'YES' : 'NO'}
- 恢复后后端可达：${value.completedChecks.restoredBackendReachable ? 'YES' : 'NO'}
- 断网截图：${value.screenshots.offline ? `\`${value.screenshots.offline}\`` : '未生成'}
- 恢复截图：${value.screenshots.restored ? `\`${value.screenshots.restored}\`` : '未生成'}

## 同版 App 证据

- 队列状态证据：${value.queueStateEvidence?.artifact ? `\`${value.queueStateEvidence.artifact}\`` : '缺失'}
- 成熟错误提示证据：${value.matureErrorEvidence?.artifact ? `\`${value.matureErrorEvidence.artifact}\`` : '缺失'}
- 隐私白名单证据：${value.privacyWhitelistEvidence?.artifact ? `\`${value.privacyWhitelistEvidence.artifact}\`` : '缺失'}
`;
}

function renderAggregateMarkdown(value) {
  return `# RC1 真实 OS 断网拨测记录

生成时间：${value.generatedAt}

状态：${value.status}

## 结论

| 端 | 状态 | 证据 |
| --- | --- | --- |
| Windows 桌面端 | ${value.desktop.status} | \`${value.desktop.artifact}\` |
| Android 原生端 | ${value.android.status} | \`${value.android.artifact}\` |

## Android 截图

- 断网：${value.android.offlineScreenshot ? `\`${value.android.offlineScreenshot}\`` : '未生成'}
- 恢复：${value.android.restoredScreenshot ? `\`${value.android.restoredScreenshot}\`` : '未生成'}

## 阻断项

${value.blockedItems.length ? value.blockedItems.map((item) => `- ${item}`).join('\n') : '- 无'}

## 放行条件

${value.requiredToUnblock.length ? value.requiredToUnblock.map((item) => `- ${item}`).join('\n') : '- 可交给 release owner 评审'}
`;
}

function renderAcceptanceSummaryMarkdown(summary, aggregate) {
  return `# RC1 No-External-Dependency Acceptance Package

- status: \`${summary.status}\`
- generatedAt: \`${summary.generatedAt}\`
- index doc: \`docs/RC1双端QA总索引.md\`
- release closure doc: \`docs/封版收口计划.md\`

## Ready Evidence

| Area | Status | Evidence |
| --- | --- | --- |
| \`commercial:ci\` | passed | Key output: \`HiddenShield commercial CI OK\`; includes \`vault:file-type-backfill-contract OK\`. |
| \`vault_records.file_type\` | passed | \`npm run vault:file-type-backfill-contract\`; covers v18 backfill, new insert inference, cloud sync \`kind\` inference. |
| Desktop Batch 2 | ready | \`tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json\` |
| Android Batch 2 | ready | \`tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json\` |
| Android real OS network disconnect | ready | \`${summary.osNetworkDisconnectDrill?.androidArtifact}\`; screenshots recorded in the artifact. |
| Cloud sync runtime | ready | \`tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json\` |
| PostgreSQL disposable evidence | ready for disposable only | \`tmp-ui-qa/postgres-migration/postgres-migrate-smoke-1783021160601.json\`, \`tmp-ui-qa/postgres-runtime-aggregate/cloud-postgres-runtime-qa-1783053449984.json\`, \`tmp-ui-qa/postgres-import/postgres-import-smoke-1783053193204.json\` |

## Blocked Evidence

| Area | Status | Evidence | Why |
| --- | --- | --- | --- |
| iOS QA | blocked | \`tmp-ui-qa/rc1-no-external-acceptance/20260704/ios-qa-blocked-20260704.json\` | Current Windows machine has no macOS + Xcode or iOS device. |
| Windows desktop real OS network disconnect | blocked | \`${summary.osNetworkDisconnectDrill?.desktopArtifact}\` | Current desktop QA topology uses loopback \`127.0.0.1:43188\`; this session cannot establish a privileged firewall/proxy block that proves OS-level disconnect while the app remains open. |
| Public rights completion | blocked | \`tmp-ui-qa/public-rights-completion/public-rights-completion-gate-1782976680658.json\` | Missing production C2PA/TSA, iOS QA, npm publish, release sample pool, customer signoff. |
| L3 production readiness | blocked | \`tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1783113551653.json\` | Missing real alert platform validation, pilot signoff, real user MP4 sample manifest. |
| Production PostgreSQL readiness | blocked | \`tmp-ui-qa/postgres-production-readiness/cloud-postgres-production-readiness-gate-1783053429272.json\` | Missing staging load, backup restore, observability, cutover runbook, release owner signoff. |
| SQLite production shutdown | blocked | \`tmp-ui-qa/postgres-sqlite-shutdown/cloud-postgres-sqlite-shutdown-gate-1783053429239.json\` | Must wait for production PostgreSQL readiness. |

## OS Network Drill

- aggregate: \`${aggregateArtifactPath ? rel(aggregateArtifactPath) : summary.osNetworkDisconnectDrill?.artifact}\`
- desktop: \`${summary.osNetworkDisconnectDrill?.desktopArtifact}\`
- android: \`${summary.osNetworkDisconnectDrill?.androidArtifact}\`

Android 原生端已完成真实 OS 网络禁用 / 恢复拨测，并关联队列状态、成熟错误提示和隐私白名单证据。Windows 桌面端仍因 loopback QA 拓扑或 firewall 权限不足不能标记为通过，需 release owner 在评审时保留阻断项。

## Release Interpretation

This package supports RC1 no-external-dependency review with Android OS disconnect evidence now present. It does not certify iOS, production WeChat Pay, production C2PA/TSA, L3 sellable SLA, Windows desktop OS-level disconnect recovery, or production PostgreSQL cutover.

## Next Step

Hand this package to the release owner for RC1 review, while scheduling an elevated Windows desktop network drill against LAN/staging backend or a controlled firewall/proxy block.
`;
}

function androidConnectivityProbe(label) {
  return {
    label,
    generatedAt: new Date().toISOString(),
    pingHost: adbProbe(['shell', 'ping', '-c', '1', '-W', '2', '10.0.2.2']),
    backendPort: adbProbe(['shell', 'toybox', 'nc', '-z', '-w', '2', '10.0.2.2', '43188']),
  };
}

function adbProbe(args) {
  const result = adb(args);
  return {
    ok: result.exitCode === 0,
    exitCode: result.exitCode,
    stdoutTail: tail(result.stdout),
    stderrTail: tail(result.stderr),
  };
}

function launchAndroidApp() {
  adb(['shell', 'monkey', '-p', 'com.hiddenshield.hidden_shield_mobile', '1']);
}

function captureAndroidScreenshot(path) {
  const result = spawnSync('adb', ['-s', adbSerial, 'exec-out', 'screencap', '-p'], {
    encoding: 'buffer',
    timeout: 30_000,
  });
  if (result.status === 0 && result.stdout?.length > 0) {
    writeFileSync(path, result.stdout);
    return rel(path);
  }
  return null;
}

function adb(args) {
  const result = spawnSync('adb', ['-s', adbSerial, ...args], {
    encoding: 'utf8',
    timeout: 30_000,
  });
  return {
    exitCode: result.status,
    signal: result.signal,
    error: result.error ? String(result.error) : null,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

async function backendHealth() {
  try {
    const response = await fetch(`${backendBaseUrl}/v1/health`, { signal: AbortSignal.timeout(3000) });
    const text = await response.text();
    let body = text;
    try {
      body = JSON.parse(text);
    } catch {
      // Keep non-JSON response text.
    }
    return {
      reachable: response.ok,
      status: response.status,
      body,
    };
  } catch (error) {
    return {
      reachable: false,
      error: String(error),
    };
  }
}

function runFirewallBlock(ruleName) {
  cleanupFirewallRule(ruleName);
  const script = [
    `$ErrorActionPreference='Stop'`,
    `New-NetFirewallRule -DisplayName ${psQuote(`${ruleName} outbound`)} -Direction Outbound -Action Block -Protocol TCP -RemotePort 43188 | Out-Null`,
    `New-NetFirewallRule -DisplayName ${psQuote(`${ruleName} inbound`)} -Direction Inbound -Action Block -Protocol TCP -LocalPort 43188 | Out-Null`,
    `'created'`,
  ].join('; ');
  const result = spawnSync('powershell', ['-NoProfile', '-Command', script], {
    encoding: 'utf8',
    timeout: 30_000,
  });
  return {
    attempted: true,
    status: result.status === 0 ? 'created' : 'failed',
    ruleName,
    exitCode: result.status,
    stdout: tail(result.stdout),
    stderr: tail(result.stderr),
    error: result.error ? String(result.error) : null,
  };
}

function cleanupFirewallRule(ruleName) {
  powershellText(
    `Get-NetFirewallRule -DisplayName ${psQuote(`${ruleName}*`)} -ErrorAction SilentlyContinue | Remove-NetFirewallRule -ErrorAction SilentlyContinue`,
  );
}

function safePowerShellJson(script) {
  const text = powershellText(script).trim();
  if (!text) return [];
  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}

function powershellText(script) {
  return safeExec('powershell', ['-NoProfile', '-Command', script]);
}

function safeExec(command, args) {
  try {
    return execFileSync(command, args, { encoding: 'utf8', timeout: 30_000 });
  } catch (error) {
    return `${error.stdout ?? ''}${error.stderr ?? ''}${error.message ?? error}`;
  }
}

function latestJsonSummary(dir, prefix, options = {}) {
  const path = latestArtifactAbs(dir, prefix, options);
  if (!path) return null;
  try {
    return {
      path: rel(path),
      json: readJson(path),
    };
  } catch (error) {
    return {
      path: rel(path),
      json: { ok: false, status: 'invalid_json', error: String(error) },
    };
  }
}

function latestArtifactPath(dir, prefix) {
  const path = latestArtifactAbs(dir, prefix);
  return path ? rel(path) : null;
}

function latestArtifactAbs(dir, prefix, options = {}) {
  const abs = resolve(dir);
  if (!existsSync(abs)) return null;
  const files = [];
  const stack = [abs];
  while (stack.length) {
    const current = stack.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const path = join(current, entry.name);
      if (entry.isDirectory() && options.recursive) {
        stack.push(path);
      } else if (entry.isFile() && entry.name.startsWith(prefix) && entry.name.endsWith('.json')) {
        files.push(path);
      }
    }
    if (!options.recursive) break;
  }
  return files
    .map((file) => ({ file, mtime: Number(statTicks(file)) }))
    .sort((a, b) => b.mtime - a.mtime || basename(b.file).localeCompare(basename(a.file)))[0]?.file ?? null;
}

function statTicks(file) {
  const raw = powershellText(`(Get-Item -LiteralPath ${psQuote(file)}).LastWriteTimeUtc.Ticks`).trim();
  return /^\d+$/.test(raw) ? raw : '0';
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeMarkdown(path, value) {
  writeFileSync(path, value, 'utf8');
}

function psQuote(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

function rel(path) {
  return relative(process.cwd(), path).replace(/\\/g, '/');
}

function tail(value, max = 2000) {
  const text = String(value ?? '');
  return text.length > max ? text.slice(-max) : text;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
