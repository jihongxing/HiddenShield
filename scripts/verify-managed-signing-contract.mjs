import { spawn, spawnSync } from "node:child_process";
import {
  createPublicKey,
  generateKeyPairSync,
  sign,
  verify,
} from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";

const root = process.cwd();
const tempRoot = mkdtempSync(
  path.join(tmpdir(), "hiddenshield-managed-signing-contract-"),
);
const resourceName =
  "projects/hiddenshield-prod/locations/global/keyRings/license/cryptoKeys/hslic1/cryptoKeyVersions/1";
const keyId = "offline-production-contract";
const { privateKey, publicKey } = generateKeyPairSync("ed25519");
const publicPem = publicKey.export({ format: "pem", type: "spki" });
const publicDer = createPublicKey(publicPem).export({
  format: "der",
  type: "spki",
});
const publicKeyBase64Url = publicDer.subarray(-32).toString("base64url");

const server = createServer(async (request, response) => {
  if (
    request.method === "GET" &&
    request.url === `/v1/${resourceName}/publicKey`
  ) {
    return json(response, {
      pem: publicPem,
      algorithm: "EC_SIGN_ED25519",
      name: resourceName,
      protectionLevel: "SOFTWARE",
    });
  }
  if (
    request.method === "POST" &&
    request.url === `/v1/${resourceName}:asymmetricSign`
  ) {
    const body = JSON.parse(await readBody(request));
    const data = Buffer.from(body.data, "base64");
    if (Number(body.dataCrc32c) !== crc32c(data)) {
      response.writeHead(400).end();
      return;
    }
    const signature = sign(null, data, privateKey);
    return json(response, {
      signature: signature.toString("base64"),
      signatureCrc32c: String(crc32c(signature)),
      verifiedDataCrc32c: true,
      name: resourceName,
      protectionLevel: "SOFTWARE",
    });
  }
  response.writeHead(404).end();
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
try {
  const address = server.address();
  const message = Buffer.from("HiddenShield managed KMS contract");
  const signerResult = await runChild(
    process.execPath,
    [
      "scripts/signers/hslic1-google-cloud-kms-signer.mjs",
      "--crypto-key-version",
      resourceName,
      "--key-id",
      keyId,
      "--expected-public-key-base64url",
      publicKeyBase64Url,
      "--allowed-protection-levels",
      "SOFTWARE",
      "--api-base-url",
      `http://127.0.0.1:${address.port}`,
    ],
    {
      cwd: root,
      input: JSON.stringify({
        schemaVersion: 1,
        operation: "ed25519_sign",
        keyId,
        keyHandle: `gcp-kms://${resourceName}`,
        purpose: "license",
        messageBase64Url: message.toString("base64url"),
      }),
      env: {
        ...process.env,
        HIDDENSHIELD_GOOGLE_KMS_TEST_MODE: "1",
        HIDDENSHIELD_GOOGLE_KMS_TEST_TOKEN: "contract-token",
      },
      windowsHide: true,
    },
  );
  assert(
    signerResult.status === 0,
    `Google KMS adapter failed: ${signerResult.stderr}`,
  );
  const signerResponse = JSON.parse(signerResult.stdout);
  assert(
    verify(
      null,
      message,
      publicKey,
      Buffer.from(signerResponse.signatureBase64Url, "base64url"),
    ),
    "Google KMS adapter signature did not verify",
  );

  const issuerBinary = path.resolve(
    "src-tauri",
    "target",
    "debug",
    "examples",
    process.platform === "win32"
      ? "offline_license_issuer.exe"
      : "offline_license_issuer",
  );
  const buildResult = spawnSync(
    "cargo",
    [
      "build",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--example",
      "offline_license_issuer",
    ],
    { cwd: root, encoding: "utf8", windowsHide: true },
  );
  assert(
    buildResult.status === 0,
    `managed KMS issuer build failed: ${buildResult.stderr}`,
  );
  const requestFixture = JSON.parse(
    readFileSync(
      "docs/fixtures/offline-license-k0/hsreq1-v1-valid.json",
      "utf8",
    ),
  );
  const requestPath = path.join(tempRoot, "managed-kms-request.hsreq");
  const configPath = path.join(tempRoot, "managed-kms-signer.json");
  const licensePath = path.join(tempRoot, "managed-kms-license.hslicense");
  const auditPath = path.join(tempRoot, "managed-kms-license-audit.json");
  writeFileSync(requestPath, requestFixture.token);
  writeFileSync(
    configPath,
    JSON.stringify(
      {
        schemaVersion: 1,
        signerType: "managed_kms",
        keyId,
        publicKeyBase64Url,
        keyHandle: `gcp-kms://${resourceName}`,
        command: process.execPath,
        arguments: [
          path.resolve(
            "scripts/signers/hslic1-google-cloud-kms-signer.mjs",
          ),
          "--crypto-key-version",
          resourceName,
          "--key-id",
          keyId,
          "--expected-public-key-base64url",
          publicKeyBase64Url,
          "--allowed-protection-levels",
          "SOFTWARE",
          "--api-base-url",
          `http://127.0.0.1:${address.port}`,
        ],
      },
      null,
      2,
    ),
  );
  const issuerResult = await runChild(
    issuerBinary,
    [
      "issue",
      "--isolated-signer-config",
      configPath,
      "--request",
      requestPath,
      "--issued-at",
      "2026-07-17T00:00:00Z",
      "--expires-at",
      "2027-07-17T00:00:00Z",
      "--operator-id",
      "managed-kms-contract",
      "--output",
      licensePath,
      "--audit-output",
      auditPath,
    ],
    {
      cwd: root,
      input: "",
      env: {
        ...process.env,
        HIDDENSHIELD_GOOGLE_KMS_TEST_MODE: "1",
        HIDDENSHIELD_GOOGLE_KMS_TEST_TOKEN: "contract-token",
      },
      windowsHide: true,
    },
  );
  assert(
    issuerResult.status === 0,
    `managed KMS issuer flow failed: ${issuerResult.stderr}`,
  );
  const issuerAudit = JSON.parse(readFileSync(auditPath, "utf8"));
  assert(
    issuerAudit.signerType === "managed_kms",
    "issuer audit must record managed_kms signer type",
  );

  const fakeSigntool = path.join(tempRoot, "signtool.exe");
  const fakeDlib = path.join(tempRoot, "Azure.CodeSigning.Dlib.dll");
  const fakeExe = path.join(tempRoot, "candidate.exe");
  const evidencePath = path.join(tempRoot, "azure-contract.json");
  writeFileSync(fakeSigntool, "contract");
  writeFileSync(fakeDlib, "contract");
  writeFileSync(fakeExe, Buffer.alloc(9000, 0x48));
  const azureResult = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      "scripts/release/sign-with-azure-artifact-signing.ps1",
      "-SigntoolPath",
      fakeSigntool,
      "-DlibPath",
      fakeDlib,
      "-Endpoint",
      "https://eus.codesigning.azure.net",
      "-CodeSigningAccountName",
      "hiddenshield-contract",
      "-CertificateProfileName",
      "windows-public-trust",
      "-Files",
      fakeExe,
      "-EvidenceOutput",
      evidencePath,
      "-ContractOnly",
    ],
    { cwd: root, encoding: "utf8", windowsHide: true },
  );
  assert(
    azureResult.status === 0,
    `Azure Artifact Signing contract failed: ${azureResult.stderr}`,
  );
  const azureEvidence = JSON.parse(readFileSync(evidencePath, "utf8"));
  assert(
    azureEvidence.provider === "azure_artifact_signing" &&
      azureEvidence.status === "contract_ready",
    "Azure Artifact Signing evidence contract is invalid",
  );
  const tauriConfigPath = path.join(tempRoot, "tauri.conf.json");
  writeFileSync(
    tauriConfigPath,
    JSON.stringify({ bundle: { windows: {} } }, null, 2),
  );
  const injectionResult = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      "scripts/release/inject-azure-artifact-signing.ps1",
      "-ConfigPath",
      tauriConfigPath,
    ],
    { cwd: root, encoding: "utf8", windowsHide: true },
  );
  assert(
    injectionResult.status === 0,
    `Tauri Azure signCommand injection failed: ${injectionResult.stderr}`,
  );
  const injectedConfig = JSON.parse(readFileSync(tauriConfigPath, "utf8"));
  assert(
    injectedConfig.bundle.windows.signCommand.cmd === "powershell.exe" &&
      injectedConfig.bundle.windows.signCommand.args.includes("%1") &&
      !("certificateThumbprint" in injectedConfig.bundle.windows),
    "Tauri config must use Azure signCommand without PFX thumbprint",
  );

  const issuerSource = readFileSync(
    "src-tauri/examples/offline_license_issuer.rs",
    "utf8",
  );
  assert(
    issuerSource.includes('"isolated-signer-config"') &&
      issuerSource.includes('"managed_kms"'),
    "issuer must accept managed KMS isolated signer config",
  );
  const authenticodeGate = readFileSync(
    "scripts/run-authenticode-gate.mjs",
    "utf8",
  );
  assert(
    authenticodeGate.includes("self_signed_authenticode") &&
      authenticodeGate.includes(
        "HIDDENSHIELD_AUTHENTICODE_SIGNING_EVIDENCE_PATH",
      ),
    "current Authenticode Gate must require self-signed signing evidence",
  );
  const releaseWorkflow = readFileSync(
    ".github/workflows/release.yml",
    "utf8",
  );
  assert(
    releaseWorkflow.includes("WINDOWS_SELF_SIGNED_CERTIFICATE") &&
      releaseWorkflow.includes("inject-windows-signing.ps1") &&
      releaseWorkflow.includes("release:authenticode-gate:candidate") &&
      !releaseWorkflow.includes("azure/login@v2"),
    "release workflow must use the free self-signed Authenticode baseline",
  );
  console.log(
    "Managed signing compatibility contract OK: Google Cloud KMS and Azure Artifact Signing adapters remain isolated optional enhancements",
  );
} finally {
  server.close();
  rmSync(tempRoot, { recursive: true, force: true });
}

function json(response, body) {
  response.writeHead(200, { "Content-Type": "application/json" });
  response.end(JSON.stringify(body));
}

async function readBody(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  return Buffer.concat(chunks).toString("utf8");
}

function crc32c(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0x82f63b78 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function assert(condition, message) {
  if (!condition) throw new Error(`Managed signing contract failed: ${message}`);
}

function runChild(command, args, options) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      windowsHide: options.windowsHide,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (status) => resolve({ status, stdout, stderr }));
    child.stdin.end(options.input);
  });
}
