import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  createHash,
} from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

const root = process.cwd();
const runId =
  process.env.HIDDENSHIELD_INSTALLER_GATE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const outputDir = resolve("artifacts/desktop-installer-self-contained", runId);
const runtimeDir = resolve("tmp-ui-qa/desktop-installer-self-contained", runId);
const installDir = resolve(runtimeDir, "installed");
const evidencePath = join(outputDir, "desktop-installer-self-contained-gate.json");
const skipBuild = process.argv.includes("--skip-build");
const minimumOfflineInstallerBytes = 90 * 1024 * 1024;
const evidence = {
  schemaVersion: "desktop_installer_self_contained_gate_v1",
  runId,
  generatedAt: new Date().toISOString(),
  status: "running",
  checks: {},
  artifacts: {},
  environment: {},
  limitations: [],
};

mkdirSync(outputDir, { recursive: true });

try {
  assert(process.platform === "win32", "This Gate only supports Windows.");
  const tauriConfig = JSON.parse(
    readFileSync(resolve("src-tauri/tauri.conf.json"), "utf8"),
  );
  const webviewInstallMode =
    tauriConfig.bundle?.windows?.webviewInstallMode ?? null;

  evidence.environment = {
    platform: process.platform,
    architecture: process.arch,
    webView2Runtime: detectWebView2Runtime(),
    physicalWebView2RemovalAttempted: false,
    physicalNetworkAdapterDisableAttempted: false,
  };
  evidence.limitations.push(
    "The host already has WebView2. The Gate does not uninstall the machine runtime because doing so would modify shared system state.",
    "Physical network adapters are not disabled by automation because that could interrupt the active QA session. Offline WebView2 coverage is established by embedding the official offline installer and checking installer size; a clean offline VM remains the GA proof.",
  );

  check(
    "frontendAssetsConfigured",
    tauriConfig.build?.frontendDist === "../dist" &&
      tauriConfig.build?.beforeBuildCommand === "npm run build",
    {
      frontendDist: tauriConfig.build?.frontendDist ?? null,
      beforeBuildCommand: tauriConfig.build?.beforeBuildCommand ?? null,
      devUrl: tauriConfig.build?.devUrl ?? null,
    },
  );
  check(
    "offlineWebView2Configured",
    webviewInstallMode?.type === "offlineInstaller" &&
      webviewInstallMode?.silent === true,
    webviewInstallMode,
  );

  stopPort1420Listeners();
  check("vitePortClosedBeforeBuild", !isPort1420Listening(), {
    endpoint: "localhost:1420",
  });

  const buildStartedAt = Date.now();
  if (!skipBuild) {
    resetBundleStaging();
    run("cmd.exe", ["/d", "/s", "/c", "npm run tauri:build"], {
      timeout: 60 * 60 * 1000,
      stdio: "inherit",
    });
  }

  stopPort1420Listeners();
  check("vitePortClosedBeforeInstall", !isPort1420Listening(), {
    endpoint: "localhost:1420",
  });

  const bundleRoot = resolve("src-tauri/target/release/bundle");
  const nsisInstaller = newest(
    findFiles(bundleRoot, (name) => /_x64-setup\.exe$/i.test(name)),
  );
  const msiInstaller = newest(
    findFiles(bundleRoot, (name) => /\.msi$/i.test(name)),
  );
  const releaseExe = resolve("src-tauri/target/release/hidden_shield.exe");

  check("nsisGenerated", Boolean(nsisInstaller), describeFile(nsisInstaller));
  check("msiGenerated", Boolean(msiInstaller), describeFile(msiInstaller));
  check("releaseExecutableGenerated", existsSync(releaseExe), describeFile(releaseExe));

  if (!skipBuild) {
    check(
      "installersBelongToCurrentBuild",
      statSync(nsisInstaller).mtimeMs >= buildStartedAt - 2000 &&
        statSync(msiInstaller).mtimeMs >= buildStartedAt - 2000,
      {
        buildStartedAt: new Date(buildStartedAt).toISOString(),
        nsisModifiedAt: statSync(nsisInstaller).mtime.toISOString(),
        msiModifiedAt: statSync(msiInstaller).mtime.toISOString(),
      },
    );
  }

  const nsisDescription = describeFile(nsisInstaller, true);
  const msiDescription = describeFile(msiInstaller, true);
  evidence.artifacts = {
    nsis: nsisDescription,
    msi: msiDescription,
    releaseExecutable: describeFile(releaseExe, true),
  };
  check(
    "offlineWebView2PayloadPresent",
    nsisDescription.bytes >= minimumOfflineInstallerBytes &&
      msiDescription.bytes >= minimumOfflineInstallerBytes,
    {
      minimumBytes: minimumOfflineInstallerBytes,
      nsisBytes: nsisDescription.bytes,
      msiBytes: msiDescription.bytes,
      basis:
        "Tauri offlineInstaller embeds the WebView2 offline runtime; both installers must exceed the self-contained package floor.",
    },
  );

  if (existsSync(installDir)) {
    rmSync(installDir, { recursive: true, force: true });
  }
  mkdirSync(installDir, { recursive: true });

  const installResult = spawnSync(
    nsisInstaller,
    ["/S", `/D=${installDir}`],
    {
      cwd: dirname(nsisInstaller),
      encoding: "utf8",
      timeout: 10 * 60 * 1000,
    },
  );
  check("nsisSilentInstallCompleted", installResult.status === 0, {
    exitCode: installResult.status,
    signal: installResult.signal,
    error: installResult.error ? String(installResult.error) : null,
    stdoutTail: tail(installResult.stdout),
    stderrTail: tail(installResult.stderr),
  });

  const installedExe = newest(
    findFiles(installDir, (name) => /^hidden_shield\.exe$/i.test(name)),
  );
  check("installedExecutableLocated", Boolean(installedExe), {
    installDir: relative(root, installDir),
    installedExecutable: installedExe ? relative(root, installedExe) : null,
  });
  evidence.artifacts.installedExecutable = describeFile(installedExe, true);
  const installedExecutables = findFiles(
    installDir,
    (name) => name.toLowerCase().endsWith(".exe"),
  ).map((path) => relative(installDir, path));
  check(
    "installerContainsOnlyProductExecutables",
    installedExecutables.every((path) =>
      ["hidden_shield.exe", "uninstall.exe"].includes(path.toLowerCase()),
    ),
    { installedExecutables },
  );

  stopPort1420Listeners();
  check("vitePortClosedBeforeLaunch", !isPort1420Listening(), {
    endpoint: "localhost:1420",
  });

  const uiAutomation = await runInstalledUiAutomation(installedExe);
  check(
    "installedUiLoadedWithoutVite",
    uiAutomation.processAlive === true &&
      uiAutomation.productUiDetected === true &&
      uiAutomation.localhostErrorDetected === false,
    uiAutomation,
  );
  check("vitePortRemainedClosed", !isPort1420Listening(), {
    endpoint: "localhost:1420",
  });

  evidence.status = "passed_with_ga_environment_limitations";
  evidence.completedAt = new Date().toISOString();
  writeEvidence();
  console.log(`Desktop installer self-contained Gate passed: ${evidencePath}`);
} catch (error) {
  evidence.status = "failed";
  evidence.completedAt = new Date().toISOString();
  evidence.error = String(error?.stack ?? error);
  writeEvidence();
  console.error(`Desktop installer self-contained Gate failed: ${evidencePath}`);
  throw error;
}

function check(name, passed, details) {
  evidence.checks[name] = { passed: Boolean(passed), details };
  writeEvidence();
  assert(passed, `${name} failed`);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with ${result.status}: ${
        result.error ? String(result.error) : "no spawn error"
      }\n${tail(
        result.stdout,
      )}\n${tail(result.stderr)}`,
    );
  }
  return result;
}

function resetBundleStaging() {
  const releaseRoot = resolve("src-tauri/target/release");
  for (const path of [
    resolve(releaseRoot, "wix"),
    resolve(releaseRoot, "nsis"),
    resolve(releaseRoot, "bundle"),
  ]) {
    assert(
      path.startsWith(`${releaseRoot}\\`) || path.startsWith(`${releaseRoot}/`),
      `Refusing to remove bundle staging outside ${releaseRoot}: ${path}`,
    );
    if (existsSync(path)) {
      rmSync(path, { recursive: true, force: true });
    }
  }
  for (const name of [
    "public_metadata_embed_qa.exe",
    "v3_readonly_fixture_qa.exe",
    "v3_readonly_candidate_runtime_qa.exe",
    "v3_internal_qa_write_runtime_qa.exe",
    "audio_noise_floor_migration_desktop_fixture.exe",
    "report_mobile_handoff_runtime_qa.exe",
    "rights_evidence_pack_runtime_qa.exe",
    "offline_license_issuer.exe",
    "desktop_offline_release_gate.exe",
  ]) {
    const path = resolve(releaseRoot, name);
    assert(
      path.startsWith(`${releaseRoot}\\`) || path.startsWith(`${releaseRoot}/`),
      `Refusing to remove release binary outside ${releaseRoot}: ${path}`,
    );
    if (existsSync(path)) {
      rmSync(path, { force: true });
    }
  }
}

function stopPort1420Listeners() {
  execFileSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-Command",
      [
        "$listeners=Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue",
        "if($listeners){$listeners|Select-Object -ExpandProperty OwningProcess -Unique|ForEach-Object{Stop-Process -Id $_ -Force -ErrorAction Stop}}",
        "exit 0",
      ].join(";"),
    ],
    { cwd: root, stdio: "ignore" },
  );
}

function isPort1420Listening() {
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-Command",
      "if(Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue){exit 0}else{exit 1}",
    ],
    { cwd: root, stdio: "ignore" },
  );
  return result.status === 0;
}

function detectWebView2Runtime() {
  const script = [
    "$path='HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'",
    "if(Test-Path $path){$item=Get-ItemProperty $path;[pscustomobject]@{path=$path;version=$item.pv;name=$item.name}|ConvertTo-Json -Compress}",
  ].join(";");
  const result = spawnSync("powershell.exe", ["-NoProfile", "-Command", script], {
    cwd: root,
    encoding: "utf8",
  });
  const text = result.stdout.trim();
  if (!text) return { detected: false, registrations: [] };
  const parsed = JSON.parse(text);
  return {
    detected: true,
    registrations: Array.isArray(parsed) ? parsed : [parsed],
  };
}

async function runInstalledUiAutomation(exe) {
  const resultPath = join(outputDir, "installed-ui-automation.json");
  const debugPort = 9300 + Math.floor(Math.random() * 500);
  const child = spawn(exe, [], {
    cwd: dirname(exe),
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
    },
    stdio: "ignore",
    windowsHide: true,
  });
  let target = null;
  let bodyText = "";
  let documentTitle = "";
  let documentReadyState = "";
  let automationError = null;
  try {
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      try {
        const response = await fetch(`http://127.0.0.1:${debugPort}/json/list`);
        const targets = await response.json();
        target = targets.find((candidate) => candidate.type === "page") ?? null;
        if (target?.webSocketDebuggerUrl) {
          const documentState = JSON.parse(
            await evaluateCdp(
              target.webSocketDebuggerUrl,
              "JSON.stringify({ title: document.title, readyState: document.readyState, bodyText: document.body?.innerText ?? '' })",
            ),
          );
          documentTitle = documentState.title ?? "";
          documentReadyState = documentState.readyState ?? "";
          bodyText = documentState.bodyText ?? "";
          if (
            documentReadyState === "complete" &&
            documentTitle === "HiddenShield" &&
            bodyText.trim().length > 20
          ) {
            break;
          }
        }
      } catch {
        target = null;
      }
      await delay(500);
    }
  } catch (error) {
    automationError = String(error?.stack ?? error);
  } finally {
    if (child.exitCode === null) child.kill();
  }
  const pageUrl = target?.url ?? "";
  const errorTerms = [
    "localhost:1420",
    "拒绝连接",
    "无法访问此页面",
    "ERR_CONNECTION_REFUSED",
  ];
  const result = {
    processId: child.pid ?? null,
    processAlive: target !== null,
    mainWindowDetected: target !== null,
    mainWindowTitle: documentTitle || target?.title || "",
    pageUrl,
    documentReadyState,
    productUiDetected:
      ["http://tauri.localhost/", "tauri://localhost/"].includes(pageUrl) &&
      documentTitle === "HiddenShield" &&
      documentReadyState === "complete" &&
      bodyText.trim().length > 20,
    localhostErrorDetected: errorTerms.some(
      (term) => pageUrl.includes(term) || bodyText.includes(term),
    ),
    bodyTextPreview: bodyText.slice(0, 2000),
    automationError,
  };
  writeFileSync(resultPath, `${JSON.stringify(result, null, 2)}\n`, "utf8");
  return result;
}

function evaluateCdp(webSocketUrl, expression) {
  return new Promise((resolvePromise, rejectPromise) => {
    const socket = new WebSocket(webSocketUrl);
    const timeout = setTimeout(() => {
      socket.close();
      rejectPromise(new Error("CDP Runtime.evaluate timed out"));
    }, 15_000);
    socket.addEventListener("open", () => {
      socket.send(
        JSON.stringify({
          id: 1,
          method: "Runtime.evaluate",
          params: { expression, returnByValue: true },
        }),
      );
    });
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id !== 1) return;
      clearTimeout(timeout);
      socket.close();
      if (message.error) {
        rejectPromise(new Error(JSON.stringify(message.error)));
        return;
      }
      resolvePromise(message.result?.result?.value ?? "");
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      rejectPromise(new Error("CDP WebSocket connection failed"));
    });
  });
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}

function findFiles(directory, predicate) {
  if (!existsSync(directory)) return [];
  const files = [];
  const pending = [directory];
  while (pending.length > 0) {
    const current = pending.pop();
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) pending.push(path);
      else if (predicate(entry.name)) files.push(path);
    }
  }
  return files;
}

function newest(files) {
  return files
    .map((path) => ({ path, modifiedAt: statSync(path).mtimeMs }))
    .sort((left, right) => right.modifiedAt - left.modifiedAt)[0]?.path;
}

function describeFile(path, includeHash = false) {
  if (!path || !existsSync(path)) return null;
  const stat = statSync(path);
  return {
    name: basename(path),
    path: relative(root, path),
    bytes: stat.size,
    modifiedAt: stat.mtime.toISOString(),
    sha256: includeHash ? sha256(path) : undefined,
    authenticodeStatus: authenticodeStatus(path),
  };
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function authenticodeStatus(path) {
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-Command",
      `(Get-AuthenticodeSignature -LiteralPath ${JSON.stringify(path)}).Status`,
    ],
    { cwd: root, encoding: "utf8" },
  );
  return result.stdout.trim() || "Unknown";
}

function writeEvidence() {
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

function tail(value, maximum = 4000) {
  const text = String(value ?? "");
  return text.length > maximum ? text.slice(-maximum) : text;
}
