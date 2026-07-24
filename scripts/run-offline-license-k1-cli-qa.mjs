import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const workspace = process.cwd();
const qaRoot = mkdtempSync(path.join(tmpdir(), "hiddenshield-license-k1-"));
const binary = path.join(
  workspace,
  "src-tauri",
  "target",
  "debug",
  "examples",
  process.platform === "win32"
    ? "offline_license_issuer.exe"
    : "offline_license_issuer",
);
const passwordEnv = "HS_LICENSE_QA_PASSWORD";
const password = "correct horse battery staple";

try {
  run("cargo", [
    "build",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--example",
    "offline_license_issuer",
  ]);

  const fixture = JSON.parse(
    readFileSync(
      "docs/fixtures/offline-license-k0/hsreq1-v1-valid.json",
      "utf8",
    ),
  );
  const requestPath = path.join(qaRoot, "request.hsreq");
  writeFileSync(requestPath, fixture.token, "utf8");

  const keyPath = path.join(qaRoot, "issuer-key.json");
  const keygen = runJson(
    binary,
    [
      "keygen",
      "--output",
      keyPath,
      "--key-id",
      "offline-qa-k1",
      "--password-env",
      passwordEnv,
    ],
    { [passwordEnv]: password },
  );

  const licensePath = path.join(qaRoot, "license.hslicense");
  const licenseAudit = path.join(qaRoot, "license-audit.json");
  runJson(
    binary,
    [
      "issue",
      "--key",
      keyPath,
      "--password-env",
      passwordEnv,
      "--request",
      requestPath,
      "--issued-at",
      "2026-07-15T00:00:00Z",
      "--expires-at",
      "2027-07-15T00:00:00Z",
      "--operator-id",
      "qa-operator",
      "--output",
      licensePath,
      "--audit-output",
      licenseAudit,
    ],
    { [passwordEnv]: password },
  );
  const licenseVerification = runJson(binary, [
    "verify-license",
    "--license",
    licensePath,
    "--public-key",
    keygen.publicKeyBase64Url,
  ]);
  const hardwareSignerConfig = path.join(qaRoot, "hardware-signer.json");
  const hardwareFixture = JSON.parse(
    readFileSync(
      "docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json",
      "utf8",
    ),
  );
  writeFileSync(
    hardwareSignerConfig,
    JSON.stringify(
      {
        schemaVersion: 1,
        signerType: "external_hardware",
        keyId: "offline-test-k0",
        publicKeyBase64Url: hardwareFixture.publicKeyBase64Url,
        keyHandle: "fixture://offline-test-k0",
        command: process.execPath,
        arguments: [
          path.join(
            workspace,
            "scripts",
            "fixtures",
            "offline-license-mock-hardware-signer.mjs",
          ),
        ],
      },
      null,
      2,
    ),
    "utf8",
  );
  const hardwareLicensePath = path.join(qaRoot, "hardware-license.hslicense");
  const hardwareLicenseAudit = path.join(
    qaRoot,
    "hardware-license-audit.json",
  );
  runJson(binary, [
    "issue",
    "--hardware-signer-config",
    hardwareSignerConfig,
    "--request",
    requestPath,
    "--issued-at",
    "2026-07-15T00:00:00Z",
    "--expires-at",
    "2027-07-15T00:00:00Z",
    "--operator-id",
    "qa-hardware-operator",
    "--output",
    hardwareLicensePath,
    "--audit-output",
    hardwareLicenseAudit,
  ]);
  const hardwareLicenseVerification = runJson(binary, [
    "verify-license",
    "--license",
    hardwareLicensePath,
    "--public-key",
    hardwareFixture.publicKeyBase64Url,
  ]);

  const revocationDraft = path.join(qaRoot, "revocation-draft.json");
  writeFileSync(
    revocationDraft,
    JSON.stringify(
      {
        listId: "rvl_qa_k1",
        generatedAt: "2026-07-15T00:00:00Z",
        sequence: 1,
        revokedLicenseIds: ["lic_revoked_0002", "lic_revoked_0001"],
      },
      null,
      2,
    ),
    "utf8",
  );
  const revocationPath = path.join(qaRoot, "revocations.hsrvl");
  const revocationAudit = path.join(qaRoot, "revocation-audit.json");
  runJson(
    binary,
    [
      "sign-revocations",
      "--key",
      keyPath,
      "--password-env",
      passwordEnv,
      "--input",
      revocationDraft,
      "--operator-id",
      "qa-operator",
      "--output",
      revocationPath,
      "--audit-output",
      revocationAudit,
    ],
    { [passwordEnv]: password },
  );
  const revocationVerification = runJson(binary, [
    "verify-revocations",
    "--revocations",
    revocationPath,
    "--public-key",
    keygen.publicKeyBase64Url,
  ]);
  const hardwareRevocationPath = path.join(
    qaRoot,
    "hardware-revocations.hsrvl",
  );
  const hardwareRevocationAudit = path.join(
    qaRoot,
    "hardware-revocation-audit.json",
  );
  runJson(binary, [
    "sign-revocations",
    "--hardware-signer-config",
    hardwareSignerConfig,
    "--input",
    revocationDraft,
    "--operator-id",
    "qa-hardware-operator",
    "--output",
    hardwareRevocationPath,
    "--audit-output",
    hardwareRevocationAudit,
  ]);
  const hardwareRevocationVerification = runJson(binary, [
    "verify-revocations",
    "--revocations",
    hardwareRevocationPath,
    "--public-key",
    hardwareFixture.publicKeyBase64Url,
  ]);

  const wrongPassword = runExpectFailure(
    binary,
    [
      "issue",
      "--key",
      keyPath,
      "--password-env",
      passwordEnv,
      "--request",
      requestPath,
      "--issued-at",
      "2026-07-15T00:00:00Z",
      "--expires-at",
      "2027-07-15T00:00:00Z",
      "--operator-id",
      "qa-operator",
      "--output",
      path.join(qaRoot, "bad.hslicense"),
      "--audit-output",
      path.join(qaRoot, "bad-audit.json"),
    ],
    "offline_license_issuer_wrong_password_or_corrupt_key",
    { [passwordEnv]: "incorrect password 123" },
  );
  const unknownTemplate = runExpectFailure(
    binary,
    [
      "inspect-request",
      "--request",
      requestPath,
      "--template",
      "studio",
    ],
    "offline_license_issuer_unknown_option:template",
  );
  const invalidRequestPath = path.join(qaRoot, "invalid-request.hsreq");
  writeFileSync(
    invalidRequestPath,
    fixture.token.replace("HSREQ1", "HSREQ2"),
    "utf8",
  );
  const invalidRequest = runExpectFailure(
    binary,
    ["inspect-request", "--request", invalidRequestPath],
    "offline_license_request_invalid_format",
  );
  const licenseAuditPayload = JSON.parse(readFileSync(licenseAudit, "utf8"));
  const revocationAuditPayload = JSON.parse(
    readFileSync(revocationAudit, "utf8"),
  );
  const hardwareLicenseAuditPayload = JSON.parse(
    readFileSync(hardwareLicenseAudit, "utf8"),
  );
  const hardwareRevocationAuditPayload = JSON.parse(
    readFileSync(hardwareRevocationAudit, "utf8"),
  );

  const result = {
    keyId: keygen.keyId,
    licenseTokenLength: readFileSync(licensePath, "utf8").trim().length,
    revocationTokenLength: readFileSync(revocationPath, "utf8").trim().length,
    licenseVerified: licenseVerification.status === "valid",
    revocationVerified: revocationVerification.status === "valid",
    softwareSignerAudited:
      licenseAuditPayload.signerType === "software_encrypted_file" &&
      revocationAuditPayload.signerType === "software_encrypted_file",
    hardwareLicenseVerified: hardwareLicenseVerification.status === "valid",
    hardwareRevocationVerified:
      hardwareRevocationVerification.status === "valid",
    hardwareSignerAudited:
      hardwareLicenseAuditPayload.signerType === "external_hardware" &&
      hardwareRevocationAuditPayload.signerType === "external_hardware",
    wrongPasswordRejected: wrongPassword,
    unknownTemplateRejected: unknownTemplate,
    invalidRequestRejected: invalidRequest,
    auditFilesPresent: existsSync(licenseAudit) && existsSync(revocationAudit),
    auditOperatorRecorded:
      licenseAuditPayload.operatorId === "qa-operator" &&
      revocationAuditPayload.operatorId === "qa-operator",
    auditSerialRecorded:
      typeof licenseAuditPayload.serialNumber === "string" &&
      licenseAuditPayload.serialNumber.startsWith("serial_"),
    auditPayloadDigestsRecorded:
      /^[a-f0-9]{64}$/.test(licenseAuditPayload.payloadSha256) &&
      /^[a-f0-9]{64}$/.test(revocationAuditPayload.payloadSha256),
  };
  assert(
    Object.values(result).every((value) => value !== false),
    "runtime result contains a failed assertion",
  );
  console.log(`Offline license K1 CLI QA OK ${JSON.stringify(result)}`);
} finally {
  rmSync(qaRoot, { recursive: true, force: true });
}

function run(command, args, extraEnv = {}) {
  const result = spawnSync(command, args, {
    cwd: workspace,
    encoding: "utf8",
    env: { ...process.env, ...extraEnv },
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed:\n${result.stderr || result.stdout}`,
    );
  }
  return result.stdout.trim();
}

function runJson(command, args, extraEnv = {}) {
  return JSON.parse(run(command, args, extraEnv));
}

function runExpectFailure(command, args, expectedError, extraEnv = {}) {
  const result = spawnSync(command, args, {
    cwd: workspace,
    encoding: "utf8",
    env: { ...process.env, ...extraEnv },
    windowsHide: true,
  });
  assert(result.status !== 0, `${expectedError} unexpectedly succeeded`);
  assert(
    result.stderr.trim() === expectedError,
    `expected ${expectedError}, got ${result.stderr.trim()}`,
  );
  return true;
}

function assert(condition, message) {
  if (!condition) throw new Error(`Offline license K1 CLI QA failed: ${message}`);
}
