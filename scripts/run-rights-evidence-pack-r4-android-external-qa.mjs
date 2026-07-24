import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const mobileRoot = path.join(repoRoot, "mobile_app");
const fixtureRoot = path.join(
  repoRoot,
  "docs",
  "fixtures",
  "rights-evidence-pack-r4",
  "case-fixture-r4-0001",
);
const serial = process.env.ANDROID_SERIAL || "emulator-5554";
const packageName = "com.hiddenshield.hidden_shield_mobile";

run(tool("adb"), ["-s", serial, "wait-for-device"]);
const qaResult = await runFlutterTestAndPushFixture(
  [
    "test",
    "integration_test/rights_evidence_pack_android_test.dart",
    "-d",
    serial,
    "--dart-define=RUN_RIGHTS_EVIDENCE_PACK_EXTERNAL_QA=true",
  ],
);

console.log(
  JSON.stringify(
    {
      status: "passed",
      serial,
      externalCaseDir: qaResult.externalCaseDir,
      pushedFileCount: qaResult.pushedFileCount,
      fixtureSource: path
        .relative(repoRoot, fixtureRoot)
        .replaceAll("\\", "/"),
    },
    null,
    2,
  ),
);

async function runFlutterTestAndPushFixture(args) {
  const command = tool("flutter");
  const usesWindowsBatch =
    process.platform === "win32" && command.toLowerCase().endsWith(".bat");
  const executable = usesWindowsBatch
    ? process.env.ComSpec || "cmd.exe"
    : command;
  const executableArgs = usesWindowsBatch
    ? ["/d", "/s", "/c", command, ...args]
    : args;
  const child = spawn(executable, executableArgs, {
    cwd: mobileRoot,
    shell: false,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let pushed = false;
  let pushedFileCount = 0;
  let externalCaseDir = "";
  let outputBuffer = "";

  const handleOutput = (chunk, stream) => {
    const text = chunk.toString();
    stream.write(text);
    outputBuffer = `${outputBuffer}${text}`.slice(-8192);
    const readyMatch = outputBuffer.match(
      /RIGHTS_EVIDENCE_PACK_EXTERNAL_READY:([^\r\n]+)/,
    );
    if (!pushed && readyMatch) {
      externalCaseDir = readyMatch[1].trim();
      const normalized = externalCaseDir.replaceAll("\\", "/");
      const allowedFragments = [
        `/Android/data/${packageName}/files/rights-evidence-pack-qa/`,
      ];
      if (
        !allowedFragments.some((fragment) => normalized.includes(fragment))
      ) {
        child.kill();
        return;
      }
      pushed = true;
      run(tool("adb"), [
        "-s",
        serial,
        "push",
        `${fixtureRoot}${path.sep}.`,
        `${externalCaseDir}/`,
      ]);
      pushedFileCount = run(
        tool("adb"),
        ["-s", serial, "shell", "find", externalCaseDir, "-type", "f"],
        { capture: true },
      ).stdout
        .split(/\r?\n/)
        .filter(Boolean).length;
      if (pushedFileCount !== 6) {
        child.kill();
      }
    }
  };
  child.stdout.on("data", (chunk) => handleOutput(chunk, process.stdout));
  child.stderr.on("data", (chunk) => handleOutput(chunk, process.stderr));

  const exitCode = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", resolve);
  });
  if (!pushed) {
    throw new Error("Android integration test never requested the external fixture");
  }
  if (pushedFileCount !== 6) {
    throw new Error(
      `expected 6 physical fixture files after adb push, got ${pushedFileCount}`,
    );
  }
  if (exitCode !== 0) {
    throw new Error(`${command} exited with status ${exitCode}`);
  }
  return { pushedFileCount, externalCaseDir };
}

function run(command, args, options = {}) {
  const usesWindowsBatch =
    process.platform === "win32" && command.toLowerCase().endsWith(".bat");
  const executable = usesWindowsBatch
    ? process.env.ComSpec || "cmd.exe"
    : command;
  const executableArgs = usesWindowsBatch
    ? ["/d", "/s", "/c", command, ...args]
    : args;
  const result = spawnSync(executable, executableArgs, {
    cwd: options.cwd ?? repoRoot,
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0 && !options.allowFailure) {
    if (options.capture) {
      process.stderr.write(result.stdout ?? "");
      process.stderr.write(result.stderr ?? "");
    }
    throw new Error(`${command} exited with status ${result.status}`);
  }
  return {
    status: result.status,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
  };
}

function tool(name) {
  if (process.platform !== "win32") return name;
  return name === "flutter" ? "flutter.bat" : `${name}.exe`;
}
