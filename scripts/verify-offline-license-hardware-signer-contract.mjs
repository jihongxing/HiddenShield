import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const root = process.cwd();
const tempRoot = mkdtempSync(
  path.join(tmpdir(), "hiddenshield-hardware-signer-contract-"),
);
const binary = path.join(
  root,
  "src-tauri",
  "target",
  "debug",
  "examples",
  process.platform === "win32"
    ? "offline_license_issuer.exe"
    : "offline_license_issuer",
);

try {
  run("cargo", [
    "build",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--example",
    "offline_license_issuer",
  ]);

  const requestFixture = JSON.parse(
    readFileSync(
      "docs/fixtures/offline-license-k0/hsreq1-v1-valid.json",
      "utf8",
    ),
  );
  const keyFixture = JSON.parse(
    readFileSync(
      "docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json",
      "utf8",
    ),
  );
  const requestPath = path.join(tempRoot, "request.hsreq");
  writeFileSync(requestPath, requestFixture.token, "utf8");

  const signerConfigPath = path.join(tempRoot, "hardware-signer.json");
  writeFileSync(
    signerConfigPath,
    JSON.stringify(
      {
        schemaVersion: 1,
        signerType: "external_hardware",
        keyId: "offline-test-k0",
        publicKeyBase64Url: keyFixture.publicKeyBase64Url,
        keyHandle: "fixture://offline-test-k0",
        command: process.execPath,
        arguments: [
          path.join(
            root,
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

  const licensePath = path.join(tempRoot, "license.hslicense");
  const licenseAuditPath = path.join(tempRoot, "license-audit.json");
  runJson(binary, [
    "issue",
    "--hardware-signer-config",
    signerConfigPath,
    "--request",
    requestPath,
    "--issued-at",
    "2026-07-15T00:00:00Z",
    "--expires-at",
    "2027-07-15T00:00:00Z",
    "--operator-id",
    "hardware-contract",
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
    keyFixture.publicKeyBase64Url,
  ]);

  const revocationDraftPath = path.join(tempRoot, "revocation-draft.json");
  writeFileSync(
    revocationDraftPath,
    JSON.stringify(
      {
        listId: "rvl_hardware_contract",
        generatedAt: "2026-07-15T00:00:00Z",
        sequence: 1,
        revokedLicenseIds: [],
      },
      null,
      2,
    ),
    "utf8",
  );
  const revocationPath = path.join(tempRoot, "revocations.hsrvl");
  const revocationAuditPath = path.join(tempRoot, "revocation-audit.json");
  runJson(binary, [
    "sign-revocations",
    "--hardware-signer-config",
    signerConfigPath,
    "--input",
    revocationDraftPath,
    "--operator-id",
    "hardware-contract",
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
    keyFixture.publicKeyBase64Url,
  ]);

  const forbiddenConfigPath = path.join(tempRoot, "forbidden-config.json");
  const forbiddenConfig = JSON.parse(readFileSync(signerConfigPath, "utf8"));
  forbiddenConfig.privateKeySeed = "forbidden";
  writeFileSync(
    forbiddenConfigPath,
    JSON.stringify(forbiddenConfig, null, 2),
    "utf8",
  );
  runExpectFailureIncludes(
    binary,
    [
      "issue",
      "--hardware-signer-config",
      forbiddenConfigPath,
      "--request",
      requestPath,
      "--expires-at",
      "2027-07-15T00:00:00Z",
      "--operator-id",
      "hardware-contract",
      "--output",
      path.join(tempRoot, "forbidden-config-license.hslicense"),
      "--audit-output",
      path.join(tempRoot, "forbidden-config-audit.json"),
    ],
    "offline_license_hardware_signer_config_invalid",
  );

  const licenseAudit = JSON.parse(readFileSync(licenseAuditPath, "utf8"));
  const revocationAudit = JSON.parse(
    readFileSync(revocationAuditPath, "utf8"),
  );
  assert(licenseVerification.status === "valid", "license verification failed");
  assert(
    revocationVerification.status === "valid",
    "revocation verification failed",
  );
  assert(
    licenseAudit.signerType === "external_hardware" &&
      revocationAudit.signerType === "external_hardware",
    "external signer type was not audited",
  );
  console.log(
    "Offline license hardware signer compatibility contract OK: external signatures remain available as an optional future enhancement",
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
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

function runExpectFailure(command, args, expectedError) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  });
  assert(result.status !== 0, `${expectedError} unexpectedly succeeded`);
  assert(
    result.stderr.trim() === expectedError,
    `expected ${expectedError}, got ${result.stderr.trim()}`,
  );
}

function runExpectFailureIncludes(command, args, expectedError) {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  });
  assert(result.status !== 0, `${expectedError} unexpectedly succeeded`);
  assert(
    result.stderr.includes(expectedError),
    `expected ${expectedError}, got ${result.stderr.trim()}`,
  );
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`Offline hardware signer contract failed: ${message}`);
  }
}
