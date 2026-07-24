import {
  existsSync,
  mkdirSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const adbSerial = process.env.ANDROID_DEVICE_ID ?? "emulator-5554";
const packageName = "com.hiddenshield.hidden_shield_mobile";
const activityName = `${packageName}/.MainActivity`;
const providerPackage = "com.hiddenshield.qa.documentsprovider";
const documentsUiPackage = "com.google.android.documentsui";
const fixtureRoot = resolve(
  "docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001",
);
const remoteQaRoot = "/sdcard/Download/HiddenShield-R4-QA";
const remoteFixture = `${remoteQaRoot}/${basename(fixtureRoot)}`;
const movedRemoteFixture = `${remoteFixture}-moved`;
const deletedAttachmentRelative =
  "attachments/capture/ATT-03-disputed-page-capture.txt";
const deletedAttachmentRemote =
  `${remoteFixture}/${deletedAttachmentRelative}`;
const deletedAttachmentLocal = join(
  fixtureRoot,
  ...deletedAttachmentRelative.split("/"),
);
const appApkPath = resolve(
  "mobile_app/build/app/outputs/flutter-apk/app-debug.apk",
);
const providerApkPath = resolve(
  "mobile_app/build/qa_documents_provider/outputs/apk/debug/qa_documents_provider-debug.apk",
);
const runId = new Date().toISOString().replaceAll(/[:.]/g, "-");
const outputDir = resolve(
  "artifacts/rights-evidence-pack-r4-saf-failure-matrix",
  runId,
);
const expectedRootDigest =
  "4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33";
const expectedFailures = {
  authorizationRevoked: {
    code: "evidence_pack_authorization_revoked",
    userMessage: "目录授权已失效，请重新选择案件包目录。",
  },
  directoryMissing: {
    code: "evidence_pack_directory_missing",
    userMessage: "案件包目录已移动或删除，请重新选择。",
  },
  attachmentMissing: {
    code: "evidence_pack_attachment_missing",
    userMessage: "案件包附件缺失，请恢复原目录内容后重试。",
  },
  providerUnavailable: {
    code: "evidence_pack_provider_unavailable",
    userMessage: "文件提供方当前不可用，请恢复对应应用或改选本地目录。",
  },
};

if (!existsSync(fixtureRoot)) {
  throw new Error(`R4 fixture missing: ${fixtureRoot}`);
}
mkdirSync(outputDir, { recursive: true });

const screenshots = {};
const results = {};
let providerDisabled = false;

try {
  prepareDownloadFixture();
  buildApks();
  installQaApps();

  selectDownloadFixture();
  results.downloadBaseline = waitForResult(60_000);
  assertMatchedResult(results.downloadBaseline, "Download baseline");
  screenshots.downloadBaseline = screenshot("01-download-baseline.png");

  adb(["shell", "rm", "-f", deletedAttachmentRemote]);
  clearLogcat();
  tapAuthorizedVerify();
  results.attachmentMissing = waitForFailure(60_000);
  assertFailure(
    results.attachmentMissing,
    expectedFailures.attachmentMissing,
    "attachment deletion",
  );
  waitForContainedText(expectedFailures.attachmentMissing.userMessage, 20_000);
  screenshots.attachmentMissing = screenshot("02-attachment-missing.png");

  adb([
    "push",
    deletedAttachmentLocal,
    `${remoteFixture}/${dirname(deletedAttachmentRelative).replaceAll("\\", "/")}`,
  ]);
  clearLogcat();
  tapAuthorizedVerify();
  results.attachmentRecovery = waitForResult(60_000);
  assertMatchedResult(results.attachmentRecovery, "attachment recovery");

  adb(["shell", "mv", remoteFixture, movedRemoteFixture]);
  clearLogcat();
  tapAuthorizedVerify();
  results.directoryMissing = waitForFailure(60_000);
  assertFailure(
    results.directoryMissing,
    expectedFailures.directoryMissing,
    "directory move",
  );
  waitForContainedText(expectedFailures.directoryMissing.userMessage, 20_000);
  screenshots.directoryMissing = screenshot("03-directory-missing.png");

  adb(["shell", "mv", movedRemoteFixture, remoteFixture]);
  clearLogcat();
  tapAuthorizedVerify();
  results.directoryRecovery = waitForResult(60_000);
  assertMatchedResult(results.directoryRecovery, "directory recovery");

  clearLogcat();
  tapByText("QA 撤销目录授权");
  waitForLogText(
    "RIGHTS_EVIDENCE_PACK_SAF_QA_CONTROL:authorization_cleared",
    30_000,
  );
  tapAuthorizedVerify();
  results.authorizationRevoked = waitForFailure(60_000);
  assertFailure(
    results.authorizationRevoked,
    expectedFailures.authorizationRevoked,
    "authorization revocation",
  );
  waitForContainedText(
    expectedFailures.authorizationRevoked.userMessage,
    20_000,
  );
  screenshots.authorizationRevoked = screenshot(
    "04-authorization-revoked.png",
  );

  clearLogcat();
  selectThirdPartyProviderFixture();
  results.thirdPartyBaseline = waitForResult(60_000);
  assertMatchedResult(results.thirdPartyBaseline, "third-party baseline");
  screenshots.thirdPartyBaseline = screenshot("05-third-party-baseline.png");

  adb(["shell", "pm", "disable-user", "--user", "0", providerPackage]);
  providerDisabled = true;
  clearLogcat();
  tapAuthorizedVerify();
  results.providerUnavailable = waitForFailure(60_000);
  assertFailure(
    results.providerUnavailable,
    expectedFailures.providerUnavailable,
    "provider disable",
  );
  waitForContainedText(
    expectedFailures.providerUnavailable.userMessage,
    20_000,
  );
  screenshots.providerUnavailable = screenshot(
    "06-provider-unavailable.png",
  );

  const summary = {
    schemaVersion: "rights_evidence_pack_r4_saf_failure_matrix_v1",
    runId,
    completedAt: new Date().toISOString(),
    adbSerial,
    downloadFixture: remoteFixture,
    thirdPartyProvider: {
      packageName: providerPackage,
      authority: "com.hiddenshield.qa.documentsprovider.documents",
      displayName: "HiddenShield QA Provider",
      fixtureDirectory: "case-fixture-r4-provider",
    },
    frozenFailures: expectedFailures,
    results,
    screenshots,
    pass: true,
  };
  const jsonPath = join(outputDir, "summary.json");
  const markdownPath = join(outputDir, "summary.md");
  writeFileSync(jsonPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  writeFileSync(markdownPath, renderMarkdown(summary), "utf8");
  console.log(`R4 SAF failure matrix passed: ${jsonPath}`);
} finally {
  if (providerDisabled) {
    try {
      adb(["shell", "pm", "enable", providerPackage]);
    } catch {
    }
  }
  try {
    const movedCheck = run(
      "adb",
      ["-s", adbSerial, "shell", "test", "-d", movedRemoteFixture],
      { timeoutMs: 60_000, silent: true, allowFailure: true },
    );
    if (movedCheck.status === 0) {
      adb(["shell", "mv", movedRemoteFixture, remoteFixture]);
    }
  } catch {
  }
}

function prepareDownloadFixture() {
  if (!remoteQaRoot.startsWith("/sdcard/Download/HiddenShield-R4-QA")) {
    throw new Error(`unsafe remote QA path: ${remoteQaRoot}`);
  }
  adb(["shell", "am", "force-stop", packageName]);
  adb(["shell", "am", "force-stop", documentsUiPackage]);
  adb(["shell", "rm", "-rf", remoteQaRoot]);
  adb(["shell", "mkdir", "-p", remoteQaRoot]);
  adb(["push", fixtureRoot, remoteQaRoot]);
  adb(["shell", "test", "-f", `${remoteFixture}/case-manifest.json`]);
}

function buildApks() {
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
  run(
    process.platform === "win32" ? "cmd.exe" : "./gradlew",
    process.platform === "win32"
      ? ["/d", "/c", "gradlew.bat", ":qa_documents_provider:assembleDebug"]
      : [":qa_documents_provider:assembleDebug"],
    { cwd: resolve("mobile_app/android"), timeoutMs: 300_000 },
  );
  if (!existsSync(appApkPath) || !existsSync(providerApkPath)) {
    throw new Error("SAF failure matrix APK build output is missing");
  }
}

function installQaApps() {
  adb(["install", "-r", providerApkPath], 180_000);
  adb(["shell", "pm", "enable", providerPackage]);
  adb(["shell", "am", "force-stop", documentsUiPackage]);
  adb(["install", "-r", appApkPath], 180_000);
  adb(["shell", "am", "force-stop", packageName]);
  adb(["shell", "pm", "clear", packageName]);
  clearLogcat();
  adb(["shell", "am", "start", "-S", "-n", activityName]);
  waitForText(["选择案件包目录"], 45_000, { scroll: true });
}

function selectDownloadFixture() {
  clearLogcat();
  tapByText("选择案件包目录");
  navigateToDownloadFixture();
  tapByText("HiddenShield-R4-QA");
  waitForText(["case-fixture-r4-0001"], 30_000);
  tapByText("case-fixture-r4-0001");
  confirmTreeSelection();
}

function selectThirdPartyProviderFixture() {
  scrollToText("重新选择案件包目录");
  tapByText("重新选择案件包目录");
  waitForDocumentsUi();
  const initialXml = dumpUi();
  if (!findBounds(initialXml, "HiddenShield QA Provider")) {
    tapByAnyText([
      "Show roots",
      "Open navigation drawer",
      "显示根目录",
      "打开导航抽屉",
    ]);
  }
  waitForText(["HiddenShield QA Provider"], 30_000);
  tapByText("HiddenShield QA Provider");
  waitForText(["case-fixture-r4-provider"], 30_000);
  tapByText("case-fixture-r4-provider");
  confirmTreeSelection();
}

function confirmTreeSelection() {
  waitForText(["使用此文件夹", "Use this folder", "USE THIS FOLDER"], 30_000);
  tapByAnyText(["使用此文件夹", "Use this folder", "USE THIS FOLDER"]);
  if (waitForAnyText(["允许", "Allow", "ALLOW"], 8_000, false)) {
    tapByAnyText(["允许", "Allow", "ALLOW"]);
  }
}

function tapAuthorizedVerify() {
  scrollToText("校验已授权目录");
  tapByText("校验已授权目录");
}

function waitForResult(timeoutMs) {
  return waitForJsonMarker("RIGHTS_EVIDENCE_PACK_SAF_QA_RESULT:", timeoutMs);
}

function waitForFailure(timeoutMs) {
  return waitForJsonMarker("RIGHTS_EVIDENCE_PACK_SAF_QA_FAILURE:", timeoutMs);
}

function waitForJsonMarker(marker, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const log = adbText(["logcat", "-d", "-v", "raw"]);
    const lines = log.split(/\r?\n/).filter((line) => line.includes(marker));
    if (lines.length > 0) {
      return JSON.parse(lines.at(-1).split(marker)[1]);
    }
    sleep(500);
  }
  throw new Error(`timed out waiting for log marker ${marker}`);
}

function waitForLogText(text, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (adbText(["logcat", "-d", "-v", "raw"]).includes(text)) {
      return;
    }
    sleep(500);
  }
  throw new Error(`timed out waiting for log text ${text}`);
}

function assertMatchedResult(result, phase) {
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

function assertFailure(actual, expected, phase) {
  if (
    actual.code !== expected.code ||
    actual.userMessage !== expected.userMessage
  ) {
    throw new Error(
      `${phase} failure mismatch: ${JSON.stringify(actual)} !== ${JSON.stringify(expected)}`,
    );
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

function waitForDocumentsUi() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (dumpUi().includes("com.google.android.documentsui")) {
      return;
    }
    sleep(500);
  }
  throw new Error("DocumentsUI did not open");
}

function waitForText(texts, timeoutMs, options = {}) {
  if (!waitForAnyText(texts, timeoutMs, options.scroll === true)) {
    throw new Error(`timed out waiting for UI text: ${texts.join(" / ")}`);
  }
}

function waitForContainedText(text, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const xml = dumpUi();
    if (xml.includes(text)) {
      return;
    }
    adb(["shell", "input", "swipe", "540", "1700", "540", "500", "350"]);
    sleep(500);
  }
  throw new Error(`timed out waiting for semantic text: ${text}`);
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

function adbText(args, allowFailure = false) {
  return run("adb", ["-s", adbSerial, ...args], {
    timeoutMs: 60_000,
    silent: true,
    allowFailure,
  }).stdout;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? process.cwd(),
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    timeout: options.timeoutMs ?? 60_000,
    windowsHide: true,
  });
  if (!options.silent && result.stdout) process.stdout.write(result.stdout);
  if (!options.silent && result.stderr) process.stderr.write(result.stderr);
  if (!options.allowFailure && result.status !== 0) {
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
  return `# HiddenShield R4 Android SAF 失败矩阵

- Download 基线：PASS
- 附件删除：\`${summary.results.attachmentMissing.code}\`
- 目录移动：\`${summary.results.directoryMissing.code}\`
- 授权撤销：\`${summary.results.authorizationRevoked.code}\`
- 第三方 Provider 基线：PASS
- Provider 禁用：\`${summary.results.providerUnavailable.code}\`
- root digest：\`${summary.results.thirdPartyBaseline.computedRootDigest}\`

第三方矩阵使用独立 APK \`${summary.thirdPartyProvider.packageName}\` 暴露只读 DocumentsProvider。结果只冻结文件访问错误码与用户提示，不改变数字签名、可信时间、水印或法律结论边界。
`;
}
