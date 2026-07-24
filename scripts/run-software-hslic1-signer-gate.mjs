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
  run("node", ["scripts/run-offline-license-k1-cli-qa.mjs"]);
  writeSummary({
    status: "passed_contract_only",
    signer: {
      status: "encrypted_software_file_contract_verified",
      productionEvidence: false,
    },
  });
  console.log(`HSLIC1 Signer Gate passed_contract_only: ${outputDir}`);
  process.exit(0);
}

const keyPath = process.env.HIDDENSHIELD_HSLIC1_SOFTWARE_KEY_PATH;
const requestPath = process.env.HIDDENSHIELD_HSLIC1_REQUEST_PATH;
const passwordEnvName =
  process.env.HIDDENSHIELD_HSLIC1_SOFTWARE_KEY_PASSWORD_ENV ||
  "HIDDENSHIELD_HSLIC1_SOFTWARE_KEY_PASSWORD";
const missingInputs = [];
if (!keyPath || !existsSync(keyPath)) missingInputs.push("encryptedSoftwareKey");
if (!requestPath || !existsSync(requestPath)) missingInputs.push("hsreq1");
if (!process.env[passwordEnvName]) missingInputs.push(passwordEnvName);
if (missingInputs.length > 0) {
  block("encrypted HSLIC1 software key, password, and HSREQ1 are required", {
    missingInputs,
  });
}

const resolvedKeyPath = path.resolve(keyPath);
const resolvedRequestPath = path.resolve(requestPath);
const keyEnvelope = JSON.parse(readFileSync(resolvedKeyPath, "utf8"));
assertEncryptedKeyEnvelope(keyEnvelope);
assert(
  process.env[passwordEnvName].length >= 8,
  "software key password must contain at least 8 characters",
);

const binary = buildProductionIssuer();
const tempRoot = mkdtempSync(
  path.join(tmpdir(), "hiddenshield-software-hslic1-gate-"),
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
    "--key",
    resolvedKeyPath,
    "--password-env",
    passwordEnvName,
    "--request",
    resolvedRequestPath,
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
    keyEnvelope.publicKeyBase64Url,
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
    "--key",
    resolvedKeyPath,
    "--password-env",
    passwordEnvName,
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
    keyEnvelope.publicKeyBase64Url,
  ]);

  const wrongPasswordEnv = "HIDDENSHIELD_HSLIC1_GATE_WRONG_PASSWORD";
  runExpectFailure(
    binary,
    [
      "issue",
      "--key",
      resolvedKeyPath,
      "--password-env",
      wrongPasswordEnv,
      "--request",
      resolvedRequestPath,
      "--issued-at",
      issuedAtTimestamp,
      "--expires-at",
      expiresAtTimestamp,
      "--operator-id",
      "release-gate-wrong-password",
      "--output",
      path.join(tempRoot, "wrong-password.hslicense"),
      "--audit-output",
      path.join(tempRoot, "wrong-password-audit.json"),
    ],
    {
      [wrongPasswordEnv]: "incorrect release gate password",
    },
    "offline_license_issuer_wrong_password_or_corrupt_key",
  );

  const licenseAudit = JSON.parse(readFileSync(licenseAuditPath, "utf8"));
  const revocationAudit = JSON.parse(readFileSync(revocationAuditPath, "utf8"));
  assert(licenseVerification.status === "valid", "HSLIC1 verification failed");
  assert(
    revocationVerification.status === "valid",
    "HSRVL1 verification failed",
  );
  assert(
    licenseAudit.signerType === "software_encrypted_file" &&
      revocationAudit.signerType === "software_encrypted_file",
    "candidate was not signed with the encrypted software key",
  );
  writeSummary({
    status: "passed",
    signer: {
      type: "software_encrypted_file",
      keyId: keyEnvelope.keyId,
      keyFileSha256: sha256(resolvedKeyPath),
      publicKeyBase64Url: keyEnvelope.publicKeyBase64Url,
      passwordSource: "environment_variable",
      productionEvidence: true,
    },
    evidence: {
      requestSha256: sha256(resolvedRequestPath),
      licenseId: issueResult.licenseId,
      licenseTokenSha256: licenseAudit.tokenSha256,
      revocationListId: revocationResult.listId,
      revocationTokenSha256: revocationAudit.tokenSha256,
      wrongPasswordRejected: true,
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

function assertEncryptedKeyEnvelope(envelope) {
  const serialized = JSON.stringify(envelope).toLowerCase();
  assert(
    envelope.schemaVersion === 1 &&
      typeof envelope.keyId === "string" &&
      typeof envelope.publicKeyBase64Url === "string" &&
      envelope.kdf === "argon2id-v19-m19456-t2-p1" &&
      envelope.cipher === "xchacha20poly1305" &&
      typeof envelope.saltBase64Url === "string" &&
      typeof envelope.nonceBase64Url === "string" &&
      typeof envelope.ciphertextBase64Url === "string",
    "invalid encrypted software key envelope",
  );
  assert(
    !serialized.includes("privatekey") &&
      !serialized.includes("private_key") &&
      !serialized.includes('"seed"') &&
      !serialized.includes('"password"'),
    "encrypted key envelope contains plaintext private material",
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

function run(command, args, extraEnv = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...extraEnv },
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed:\n${result.stderr}\n${result.stdout}`,
    );
  }
  return result.stdout.trim();
}

function runJson(command, args, extraEnv = {}) {
  return JSON.parse(run(command, args, extraEnv));
}

function runExpectFailure(command, args, extraEnv, expectedMessage) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...extraEnv },
    windowsHide: true,
  });
  assert(result.status !== 0, "wrong software key password must be rejected");
  assert(
    `${result.stderr}\n${result.stdout}`.includes(expectedMessage),
    `wrong password failure must include ${expectedMessage}`,
  );
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
      custody: "password_encrypted_software_file",
      knownLimitation:
        "service-provider compromise can expose the exportable signing key",
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
