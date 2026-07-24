import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const root = process.cwd();
const mode = process.argv.includes("--candidate") ? "candidate" : "contract";
const runId = new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const outputDir = path.resolve("artifacts", "hslic1-signer-gate", runId);
mkdirSync(outputDir, { recursive: true });

if (mode === "contract") {
  run("node", ["scripts/verify-managed-signing-contract.mjs"]);
  writeSummary({
    status: "passed_contract_only",
    signer: {
      status: "managed_kms_contract_verified",
      productionEvidence: false,
    },
  });
  console.log(`HSLIC1 Signer Gate passed_contract_only: ${outputDir}`);
  process.exit(0);
}

const configPath = process.env.HIDDENSHIELD_HSLIC1_SIGNER_CONFIG;
const requestPath = process.env.HIDDENSHIELD_HSLIC1_REQUEST_PATH;
const missingInputs = [];
if (!configPath || !existsSync(configPath)) missingInputs.push("signerConfig");
if (!requestPath || !existsSync(requestPath)) missingInputs.push("hsreq1");
if (missingInputs.length > 0) {
  block("real HSLIC1 signer configuration and HSREQ1 are required", {
    missingInputs,
  });
}

const config = JSON.parse(readFileSync(configPath, "utf8"));
assertProductionConfig(config, configPath);
const binary = buildProductionIssuer();
const tempRoot = mkdtempSync(
  path.join(tmpdir(), "hiddenshield-hslic1-signer-gate-"),
);

try {
  const issuedAt = new Date();
  issuedAt.setUTCMilliseconds(0);
  const expiresAt = new Date(
    issuedAt.getTime() + 365 * 24 * 60 * 60 * 1000,
  );
  const issuedAtTimestamp = toProtocolTimestamp(issuedAt);
  const expiresAtTimestamp = toProtocolTimestamp(expiresAt);
  const licensePath = path.join(tempRoot, "candidate.hslicense");
  const licenseAuditPath = path.join(tempRoot, "candidate-license-audit.json");
  const issueResult = runJson(binary, [
    "issue",
    "--isolated-signer-config",
    path.resolve(configPath),
    "--request",
    path.resolve(requestPath),
    "--issued-at",
    issuedAtTimestamp,
    "--expires-at",
    expiresAtTimestamp,
    "--operator-id",
    process.env.HIDDENSHIELD_HSLIC1_OPERATOR_ID || "release-gate",
    "--output",
    licensePath,
    "--audit-output",
    licenseAuditPath,
  ]);
  const licenseVerification = runJson(binary, [
    "verify-license",
    "--license",
    licensePath,
    "--public-key",
    config.publicKeyBase64Url,
  ]);

  const revocationDraftPath = path.join(tempRoot, "revocation-draft.json");
  writeFileSync(
    revocationDraftPath,
    JSON.stringify(
      {
        listId: `rvl_release_gate_${runId}`,
        generatedAt: toProtocolTimestamp(new Date()),
        sequence: 1,
        revokedLicenseIds: [issueResult.licenseId],
      },
      null,
      2,
    ),
  );
  const revocationPath = path.join(tempRoot, "candidate.hsrvl");
  const revocationAuditPath = path.join(
    tempRoot,
    "candidate-revocation-audit.json",
  );
  const revocationResult = runJson(binary, [
    "sign-revocations",
    "--isolated-signer-config",
    path.resolve(configPath),
    "--input",
    revocationDraftPath,
    "--operator-id",
    process.env.HIDDENSHIELD_HSLIC1_OPERATOR_ID || "release-gate",
    "--output",
    revocationPath,
    "--audit-output",
    revocationAuditPath,
  ]);
  const revocationVerification = runJson(binary, [
    "verify-revocations",
    "--revocations",
    revocationPath,
    "--public-key",
    config.publicKeyBase64Url,
  ]);
  const licenseAudit = JSON.parse(readFileSync(licenseAuditPath, "utf8"));
  const revocationAudit = JSON.parse(readFileSync(revocationAuditPath, "utf8"));
  assert(licenseVerification.status === "valid", "HSLIC1 verification failed");
  assert(
    revocationVerification.status === "valid",
    "HSRVL1 verification failed",
  );
  assert(
    licenseAudit.signerType === "managed_kms" &&
      revocationAudit.signerType === "managed_kms",
    "candidate was not signed through managed KMS interface",
  );
  writeSummary({
    status: "passed",
    signer: {
      status: "managed_kms_signer_verified",
      keyId: config.keyId,
      keyHandle: config.keyHandle,
      command: config.command,
      configSha256: sha256(configPath),
      privateKeyExportedToHiddenShield: false,
    },
    evidence: {
      requestSha256: sha256(requestPath),
      licenseId: issueResult.licenseId,
      licenseTokenSha256: licenseAudit.tokenSha256,
      revocationListId: revocationResult.listId,
      revocationTokenSha256: revocationAudit.tokenSha256,
      annualTerm: {
        issuedAt: issuedAtTimestamp,
        expiresAt: expiresAtTimestamp,
      },
    },
  });
  console.log(`HSLIC1 Signer Gate passed: ${outputDir}`);
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

function assertProductionConfig(config, candidatePath) {
  const serialized = JSON.stringify(config).toLowerCase();
  const forbidden = [
    "privatekey",
    "private_key",
    "seed",
    "mnemonic",
    "password",
    "pfx",
    "pem",
  ];
  assert(
    config.schemaVersion === 1 &&
    config.signerType === "managed_kms" &&
      typeof config.keyId === "string" &&
      typeof config.publicKeyBase64Url === "string" &&
      typeof config.keyHandle === "string" &&
      typeof config.command === "string",
    "invalid external hardware signer configuration",
  );
  assert(path.isAbsolute(config.command), "signer command must be absolute");
  assert(existsSync(config.command), "signer command does not exist");
  assert(
    !forbidden.some((field) => serialized.includes(`"${field}"`)),
    "signer configuration contains forbidden private material",
  );
  const combined = `${candidatePath} ${config.command} ${config.keyHandle}`.toLowerCase();
  assert(
    !combined.includes("fixture") && !combined.includes("mock"),
    "fixture or mock signer cannot pass the candidate Gate",
  );
  const adapterContract = `${config.keyHandle} ${(config.arguments || []).join(" ")}`.toLowerCase();
  assert(
    config.keyHandle.startsWith("gcp-kms://") &&
      adapterContract.includes("hslic1-google-cloud-kms-signer.mjs") &&
      !adapterContract.includes("localhost") &&
      !adapterContract.includes("127.0.0.1") &&
      !adapterContract.includes("test-mode"),
    "candidate must use the Google Cloud KMS Ed25519 adapter",
  );
}

function buildProductionIssuer() {
  run("cargo", [
    "build",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--example",
    "offline_license_issuer",
  ]);
  const executable = path.resolve(
    "src-tauri",
    "target",
    "debug",
    "examples",
    process.platform === "win32"
      ? "offline_license_issuer.exe"
      : "offline_license_issuer",
  );
  assert(existsSync(executable), "production issuer binary was not built");
  return executable;
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed:\n${result.stderr}\n${result.stdout}`,
    );
  }
  return result.stdout.trim();
}

function runJson(command, args) {
  return JSON.parse(run(command, args));
}

function sha256(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

function toProtocolTimestamp(date) {
  return new Date(date).toISOString().replace(/\.\d{3}Z$/, "Z");
}

function writeSummary(details) {
  const summary = {
    schemaVersion: 1,
    gate: "hslic1_signer_gate",
    generatedAt: new Date().toISOString(),
    mode,
    scope: {
      signs: ["hslic1", "hsrvl1"],
      excludes: ["windows_exe", "windows_msi", "windows_nsis"],
    },
    keyPolicy: {
      algorithm: "Ed25519",
      privateKeyPurpose: "offline_license_issuance_only",
      isolation: "managed_kms_signing_api",
      serviceProviderIdentityOnly: true,
      mustBeDistinctFromAuthenticode: true,
    },
    ...details,
  };
  writeFileSync(
    path.join(outputDir, "hslic1-signer-gate.json"),
    `${JSON.stringify(summary, null, 2)}\n`,
    "utf8",
  );
}

function block(message, details) {
  writeSummary({ status: "blocked", reason: message, ...details });
  console.error(`HSLIC1 Signer Gate blocked: ${outputDir}`);
  throw new Error(message);
}

function assert(condition, message) {
  if (!condition) throw new Error(`HSLIC1 Signer Gate failed: ${message}`);
}
