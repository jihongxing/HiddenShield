import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

const root = process.cwd();
const mode = process.argv.includes("--candidate") ? "candidate" : "contract";
const runId = new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const outputDir = path.resolve(
  "artifacts",
  "offline-license-security-gate",
  runId,
);
mkdirSync(outputDir, { recursive: true });

runCargoTest(
  "db::offline_license::tests::copied_identity_metadata_rejects_a_different_keyring_secret",
);
runCargoTest(
  "entitlements::tests::full_snapshot_rollback_is_known_limit_without_external_anchor",
);

const authenticodeGate =
  mode === "candidate"
    ? runSubGate("scripts/run-authenticode-gate.mjs")
    : { status: "separate_gate_not_run_in_contract_mode" };
const hslic1SignerGate =
  mode === "candidate"
    ? runSubGate("scripts/run-software-hslic1-signer-gate.mjs")
    : { status: "separate_gate_not_run_in_contract_mode" };

const documentation = [
  readFileSync("docs/商业化落地Roadmap.md", "utf8"),
  readFileSync("docs/当前真实能力边界说明.md", "utf8"),
  readFileSync("docs/CDKEY离线激活与本地许可证设计.md", "utf8"),
].join("\n");
assert(
  documentation.includes("不建设后端在线许可证验证"),
  "documents must freeze the no-online-license-validation decision",
);
assert(
  documentation.includes("完整快照回滚") &&
    documentation.includes("已知限制"),
  "documents must record full snapshot rollback as a known limitation",
);

const summary = {
  schemaVersion: 1,
  gate: "offline_license_security_attack_gate",
  generatedAt: new Date().toISOString(),
  mode,
  status:
    mode !== "candidate" ||
    (authenticodeGate.status === "passed" &&
      hslic1SignerGate.status === "passed")
      ? "passed_with_full_snapshot_known_limitation"
      : "blocked_candidate_evidence",
  decisions: {
    backendOnlineLicenseValidation: "not_planned",
    authenticodePrivateKey:
      "self_signed_exportable_pfx_distinct_from_hslic1",
    hslic1PrivateKey:
      "password_encrypted_ed25519_software_key",
    fullSnapshotRollback: "known_limitation_without_external_anchor",
  },
  gates: {
    authenticode: authenticodeGate,
    hslic1Signer: hslic1SignerGate,
    copiedDatabase: {
      status: "passed",
      expectedResult: "offline_license_installation_identity_mismatch",
    },
    fullSnapshotRollback: {
      status: "known_limitation_reproduced",
      releaseMeaning:
        "A complete rollback of database, secure storage, and time cannot be reliably detected by the pure offline client.",
    },
  },
};
writeFileSync(
  path.join(outputDir, "offline-license-security-gate.json"),
  `${JSON.stringify(summary, null, 2)}\n`,
  "utf8",
);
console.log(
  `Offline license security attack gate ${summary.status}: ${outputDir}`,
);
if (mode === "candidate" && summary.status === "blocked_candidate_evidence") {
  throw new Error(
    "candidate Gate blocked: self-signed Authenticode and software HSLIC1 Signer Gates must both pass",
  );
}

function runCargoTest(testName) {
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--lib",
      testName,
      "--",
      "--exact",
    ],
    {
      cwd: root,
      encoding: "utf8",
      windowsHide: true,
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `cargo test ${testName} failed:\n${result.stderr || result.stdout}`,
    );
  }
}

function runSubGate(scriptPath) {
  const result = spawnSync("node", [scriptPath, "--candidate"], {
    cwd: root,
    encoding: "utf8",
    env: process.env,
    windowsHide: true,
  });
  return {
    status: result.status === 0 ? "passed" : "blocked",
    command: `node ${scriptPath} --candidate`,
    output: meaningfulLine(result.stdout),
    error: meaningfulLine(result.stderr),
  };
}

function meaningfulLine(value) {
  const lines = (value || "")
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(
      (line) =>
        line &&
        !line.startsWith("at ") &&
        !line.startsWith("file:///") &&
        !line.startsWith("Node.js v"),
    );
  return (
    lines.find((line) => line.includes("Gate blocked")) ||
    lines.find((line) => line.startsWith("Error:")) ||
    lines.at(-1) ||
    ""
  );
}

function assert(condition, message) {
  if (!condition) throw new Error(`Offline license security gate failed: ${message}`);
}
