import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const adbSerial = process.env.ANDROID_DEVICE_ID ?? "emulator-5554";
const packageName = "com.hiddenshield.hidden_shield_mobile";
const activityName = `${packageName}/.MainActivity`;
const fixtureRoot = resolve(
  "docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001",
);
const remoteQaRoot = "/sdcard/Download/HiddenShield-R4-QA";
const remoteFixture = `${remoteQaRoot}/${basename(fixtureRoot)}`;
const apkPath = resolve(
  "mobile_app/build/app/outputs/flutter-apk/app-debug.apk",
);
const runId = new Date().toISOString().replaceAll(/[:.]/g, "-");
const outputDir = resolve(
  "artifacts/rights-evidence-pack-r4-saf-click-qa",
  runId,
);
const expectedRootDigest =
  "4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33";

if (!existsSync(fixtureRoot)) {
  throw new Error(`R4 fixture missing: ${fixtureRoot}`);
}
mkdirSync(outputDir, { recursive: true });

prepareDownloadFixture();
buildApk();
installFreshApp();

const screenshots = {};
scrollToText("选择案件包目录");
screenshots.beforePicker = screenshot("01-before-picker.png");
clearLogcat();
tapByText("选择案件包目录");

navigateToDownloadFixture();
screenshots.downloadPicker = screenshot("02-download-picker.png");
tapByText("HiddenShield-R4-QA");
waitForText(["case-fixture-r4-0001"], 30_000);
tapByText("case-fixture-r4-0001");
waitForText(["使用此文件夹", "Use this folder", "USE THIS FOLDER"], 30_000);
screenshots.caseFolder = screenshot("03-case-folder.png");
tapByAnyText(["使用此文件夹", "Use this folder", "USE THIS FOLDER"]);
if (waitForAnyText(["允许", "Allow", "ALLOW"], 8_000, false)) {
  tapByAnyText(["允许", "Allow", "ALLOW"]);
}

const firstResult = waitForResult(60_000);
assertResult(firstResult, "first selection");
screenshots.firstResult = screenshot("04-first-result.png");

adb(["shell", "am", "force-stop", packageName]);
clearLogcat();
adb(["shell", "am", "start", "-S", "-n", activityName]);
waitForText(["校验已授权目录"], 45_000, { scroll: true });
screenshots.persistedDirectory = screenshot("05-persisted-directory.png");
tapByText("校验已授权目录");
const restartResult = waitForResult(60_000);
assertResult(restartResult, "restart verification");
screenshots.restartResult = screenshot("06-restart-result.png");

const persistedUriPermissions = adbText([
  "shell",
  "dumpsys",
  "package",
  packageName,
]);
const summary = {
  schemaVersion: "rights_evidence_pack_r4_saf_click_qa_v1",
  runId,
  completedAt: new Date().toISOString(),
  adbSerial,
  fixtureRoot,
  remoteFixture,
  firstResult,
  restartResult,
  persistedGrantReusedAfterRestart: true,
  persistedGrantVisibleInPackageDump:
    persistedUriPermissions.includes("content://") &&
    persistedUriPermissions.includes("downloads"),
  screenshots,
  pass: true,
};
const jsonPath = join(outputDir, "summary.json");
const markdownPath = join(outputDir, "summary.md");
writeFileSync(jsonPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
writeFileSync(markdownPath, renderMarkdown(summary), "utf8");
console.log(`R4 SAF click QA passed: ${jsonPath}`);

function prepareDownloadFixture() {
  if (!remoteQaRoot.startsWith("/sdcard/Download/HiddenShield-R4-QA")) {
    throw new Error(`unsafe remote QA path: ${remoteQaRoot}`);
  }
  adb(["shell", "rm", "-rf", remoteQaRoot]);
  adb(["shell", "mkdir", "-p", remoteQaRoot]);
  adb(["push", fixtureRoot, remoteQaRoot]);
  const remoteManifest = `${remoteFixture}/case-manifest.json`;
  adb(["shell", "test", "-f", remoteManifest]);
}

function buildApk() {
  const flutterArgs = [
    "build",
    "apk",
    "--debug",
    "--target-platform",
    "android-x64",
    "-t",
    "tool/rights_evidence_pack_saf_click_qa.dart",
  ];
  run(
    process.platform === "win32" ? "cmd.exe" : "flutter",
    process.platform === "win32"
      ? ["/d", "/c", "flutter", ...flutterArgs]
      : flutterArgs,
    { cwd: resolve("mobile_app"), timeoutMs: 900_000 },
  );
  if (!existsSync(apkPath)) {
    throw new Error(`debug APK missing: ${apkPath}`);
  }
}

function installFreshApp() {
  adb(["install", "-r", apkPath], 180_000);
  adb(["shell", "am", "force-stop", packageName]);
  adb(["shell", "pm", "clear", packageName]);
  clearLogcat();
  adb(["shell", "am", "start", "-S", "-n", activityName]);
  waitForText(["选择案件包目录"], 45_000, { scroll: true });
}

function waitForResult(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const log = adbText(["logcat", "-d", "-v", "raw"]);
    const lines = log
      .split(/\r?\n/)
      .filter((line) => line.includes("RIGHTS_EVIDENCE_PACK_SAF_QA_RESULT:"));
    if (lines.length > 0) {
      const payload = lines.at(-1).split(
        "RIGHTS_EVIDENCE_PACK_SAF_QA_RESULT:",
      )[1];
      return JSON.parse(payload);
    }
    sleep(500);
  }
  throw new Error("timed out waiting for SAF verification result");
}

function assertResult(result, phase) {
  const expected = {
    directoryContractStatus: "matched",
    attachmentIntegrityStatus: "matched",
    eventChainStatus: "matched",
    attachmentChainStatus: "matched",
    signatureStatus: "not_signed",
    trustedTimeStatus: "not_timestamped",
    declaredRootDigest: expectedRootDigest,
    computedRootDigest: expectedRootDigest,
    matchedAttachmentCount: 4,
    attachmentCount: 4,
  };
  for (const [key, value] of Object.entries(expected)) {
    if (result[key] !== value) {
      throw new Error(`${phase} ${key} mismatch: ${result[key]} !== ${value}`);
    }
  }
}

function waitForText(texts, timeoutMs, options = {}) {
  if (!waitForAnyText(texts, timeoutMs, options.scroll === true)) {
    throw new Error(`timed out waiting for UI text: ${texts.join(" / ")}`);
  }
}

function navigateToDownloadFixture() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    const xml = dumpUi();
    if (findBounds(xml, "HiddenShield-R4-QA")) {
      return;
    }
    const downloadBounds =
      findBounds(xml, "Download") ?? findBounds(xml, "Downloads");
    if (downloadBounds) {
      adb([
        "shell",
        "input",
        "tap",
        String(downloadBounds.x),
        String(downloadBounds.y),
      ]);
      waitForText(["HiddenShield-R4-QA"], 20_000);
      return;
    }
    sleep(500);
  }
  throw new Error("timed out navigating DocumentsUI to Download fixture");
}

function waitForAnyText(texts, timeoutMs, scroll = false) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const xml = dumpUi();
    if (
      xml.includes("System UI isn&apos;t responding") ||
      xml.includes("System UI isn't responding")
    ) {
      const waitButton = findBounds(xml, "Wait");
      if (waitButton) {
        adb([
          "shell",
          "input",
          "tap",
          String(waitButton.x),
          String(waitButton.y),
        ]);
        sleep(3000);
        continue;
      }
    }
    if (texts.some((text) => findBounds(xml, text))) {
      return true;
    }
    if (scroll) {
      adb(["shell", "input", "swipe", "540", "1700", "540", "500", "350"]);
    }
    sleep(500);
  }
  return false;
}

function scrollToText(text) {
  waitForText([text], 30_000, { scroll: true });
}

function tapByAnyText(texts) {
  const xml = dumpUi();
  for (const text of texts) {
    const bounds = findBounds(xml, text);
    if (bounds) {
      adb(["shell", "input", "tap", String(bounds.x), String(bounds.y)]);
      return;
    }
  }
  throw new Error(`UI text not tappable: ${texts.join(" / ")}`);
}

function tapByText(text) {
  tapByAnyText([text]);
}

function dumpUi() {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const dump = spawnSync(
      "adb",
      [
        "-s",
        adbSerial,
        "shell",
        "uiautomator",
        "dump",
        "/data/local/tmp/rights-evidence-window.xml",
      ],
      { encoding: "utf8", timeout: 15_000, windowsHide: true },
    );
    if (dump.status === 0) {
      const read = spawnSync(
        "adb",
        [
          "-s",
          adbSerial,
          "shell",
          "cat",
          "/data/local/tmp/rights-evidence-window.xml",
        ],
        { encoding: "utf8", timeout: 15_000, windowsHide: true },
      );
      if (read.status === 0 && read.stdout.includes("<hierarchy")) {
        return read.stdout;
      }
    }
    sleep(500);
  }
  return "";
}

function findBounds(xml, text) {
  const escaped = escapeRegExp(text);
  const pattern = new RegExp(
    `<node[^>]*(?:text|content-desc)="${escaped}"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`,
  );
  const reversePattern = new RegExp(
    `<node[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"[^>]*(?:text|content-desc)="${escaped}"`,
  );
  const match = xml.match(pattern) ?? xml.match(reversePattern);
  if (!match) return null;
  return {
    x: Math.round((Number(match[1]) + Number(match[3])) / 2),
    y: Math.round((Number(match[2]) + Number(match[4])) / 2),
  };
}

function screenshot(fileName) {
  const outputPath = join(outputDir, fileName);
  const result = spawnSync(
    "adb",
    ["-s", adbSerial, "exec-out", "screencap", "-p"],
    { maxBuffer: 32 * 1024 * 1024, windowsHide: true },
  );
  if (result.status !== 0) {
    throw new Error(`screenshot failed: ${fileName}`);
  }
  writeFileSync(outputPath, result.stdout);
  return outputPath;
}

function clearLogcat() {
  adb(["logcat", "-c"]);
}

function adb(args, timeoutMs = 60_000) {
  return run("adb", ["-s", adbSerial, ...args], { timeoutMs });
}

function adbText(args) {
  return run("adb", ["-s", adbSerial, ...args], {
    timeoutMs: 60_000,
    silent: true,
  }).stdout;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? process.cwd(),
    encoding: options.binary ? undefined : "utf8",
    maxBuffer: 64 * 1024 * 1024,
    timeout: options.timeoutMs ?? 60_000,
    windowsHide: true,
  });
  if (!options.silent && result.stdout) process.stdout.write(result.stdout);
  if (!options.silent && result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.status} ${result.error ?? ""}`,
    );
  }
  return result;
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function renderMarkdown(summary) {
  return `# HiddenShield R4 Android SAF 点击 QA

- ADB: \`${summary.adbSerial}\`
- Download fixture: \`${summary.remoteFixture}\`
- 首次选择校验: PASS
- 重启持久授权校验: PASS
- root digest: \`${summary.restartResult.computedRootDigest}\`
- 重启后授权复用: ${summary.persistedGrantReusedAfterRestart ? "PASS" : "FAIL"}
- package dumpsys 可见性: ${summary.persistedGrantVisibleInPackageDump ? "已观察到" : "当前系统镜像未输出"}

该 QA 只证明 Android DocumentsUI Download 目录、持久读取授权与 R4 技术完整性校验链路；不证明任意 DocumentsProvider、数字签名、可信时间或司法采信。
`;
}
