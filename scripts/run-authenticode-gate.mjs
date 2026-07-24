import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const mode = process.argv.includes("--candidate") ? "candidate" : "contract";
const runId =
  process.env.HIDDENSHIELD_AUTHENTICODE_RUN_ID ??
  new Date().toISOString().replaceAll(/[-:.TZ]/g, "").slice(0, 14);
const outputDir = path.resolve("artifacts", "authenticode-gate", runId);
mkdirSync(outputDir, { recursive: true });

const candidateArtifacts = [
  [
    "nsis_installed_exe",
    process.env.HIDDENSHIELD_SIGNED_NSIS_INSTALLED_EXE_PATH,
  ],
  [
    "msi_installed_exe",
    process.env.HIDDENSHIELD_SIGNED_MSI_INSTALLED_EXE_PATH,
  ],
  ["msi", process.env.HIDDENSHIELD_SIGNED_MSI_PATH],
  ["nsis", process.env.HIDDENSHIELD_SIGNED_NSIS_PATH],
];
const missingArtifacts = candidateArtifacts
  .filter(([, candidatePath]) => !candidatePath || !existsSync(candidatePath))
  .map(([kind]) => kind);

let artifactGate;
if (missingArtifacts.length > 0) {
  artifactGate = {
    status: "blocked_formal_authenticode_candidate_required",
    missingArtifacts,
  };
} else {
  const providerEvidence = loadSelfSignedAuthenticodeEvidence();
  const tempRoot = mkdtempSync(
    path.join(tmpdir(), "hiddenshield-authenticode-gate-"),
  );
  try {
    artifactGate = {
      status: "passed",
      provider: "self_signed_authenticode",
      signingEvidence: providerEvidence.path,
      artifacts: candidateArtifacts.map(([kind, candidatePath]) =>
        verifyTamperDetection(
          kind,
          path.resolve(candidatePath),
          tempRoot,
          providerEvidence,
        ),
      ),
    };
    assertInstalledPayloadTopology(artifactGate.artifacts);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
}

const summary = {
  schemaVersion: 1,
  gate: "authenticode_gate",
  generatedAt: new Date().toISOString(),
  mode,
  status:
    artifactGate.status === "passed"
      ? "passed"
      : mode === "candidate"
        ? "blocked"
        : "contract_ready_candidate_evidence_blocked",
  scope: {
    signs: [
      "windows_nsis_installed_exe",
      "windows_msi_installed_exe",
      "windows_msi",
      "windows_nsis",
    ],
    excludes: ["hslic1", "hsrvl1"],
  },
  keyPolicy: {
    privateKeyPurpose: "windows_code_signing_only",
    provider: "self_signed_authenticode",
    privateKeyCustody: "password_protected_exportable_pfx",
    trustScope: "service_provider_and_managed_customer_trust_store",
    knownLimitation:
      "general Windows clients do not trust the self-signed publisher by default",
    mustBeDistinctFromHslic1Signer: true,
  },
  artifactGate,
};
writeFileSync(
  path.join(outputDir, "authenticode-gate.json"),
  `${JSON.stringify(summary, null, 2)}\n`,
  "utf8",
);
console.log(`Authenticode Gate ${summary.status}: ${outputDir}`);

if (mode === "candidate" && summary.status !== "passed") {
  throw new Error(
    "formal Authenticode EXE, NSIS/MSI, and both installer-produced EXE candidates are required",
  );
}

function verifyTamperDetection(kind, candidatePath, tempRoot, providerEvidence) {
  const originalSignature = authenticodeStatus(candidatePath);
  assert(
    originalSignature.Status === "Valid",
    `${kind} candidate must have Valid Authenticode`,
  );
  const candidateSha256 = sha256(candidatePath);
  const evidenceEntry = providerEvidence.files.find(
    (entry) =>
      path.resolve(entry.path) === candidatePath &&
      entry.status === "Valid" &&
      entry.sha256.toLowerCase() === candidateSha256,
  );
  assert(evidenceEntry, `${kind} is missing from self-signed signing evidence`);
  const tamperedPath = path.join(
    tempRoot,
    `${kind}-tampered${path.extname(candidatePath)}`,
  );
  copyFileSync(candidatePath, tamperedPath);
  const bytes = readFileSync(tamperedPath);
  assert(bytes.length > 8192, `${kind} candidate is unexpectedly small`);
  if (kind === "msi") {
    tamperMsiDatabase(tamperedPath);
  } else {
    bytes[Math.min(4096, bytes.length - 1)] ^= 0x01;
    writeFileSync(tamperedPath, bytes);
  }
  const tamperedSignature = authenticodeStatus(tamperedPath);
  assert(
    tamperedSignature.Status !== "Valid",
    `${kind} tampering must invalidate Authenticode`,
  );
  return {
    kind,
    path: candidatePath,
    sha256: candidateSha256,
    signerSubject: originalSignature.Subject,
    signerThumbprint: originalSignature.Thumbprint,
    originalStatus: originalSignature.Status,
    tamperedStatus: tamperedSignature.Status,
  };
}

function tamperMsiDatabase(filePath) {
  const command =
    "$installer = New-Object -ComObject WindowsInstaller.Installer;" +
    "$database = $installer.OpenDatabase($env:HS_GATE_FILE, 1);" +
    "$view = $database.OpenView(\"UPDATE `Property` SET `Value`='HiddenShield Tampered' WHERE `Property`='ProductName'\");" +
    "$view.Execute();" +
    "$database.Commit();";
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-Command", command],
    {
      encoding: "utf8",
      windowsHide: true,
      env: { ...process.env, HS_GATE_FILE: filePath },
    },
  );
  if (result.status !== 0) {
    throw new Error(`MSI tamper operation failed: ${result.stderr}`);
  }
}

function authenticodeStatus(filePath) {
  const command =
    "$signature = Get-AuthenticodeSignature -LiteralPath $env:HS_GATE_FILE;" +
    "[pscustomobject]@{" +
    "Status=$signature.Status.ToString();" +
    "Subject=if($signature.SignerCertificate){$signature.SignerCertificate.Subject}else{''};" +
    "Thumbprint=if($signature.SignerCertificate){$signature.SignerCertificate.Thumbprint}else{''}" +
    "}|ConvertTo-Json -Compress";
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-Command", command],
    {
      encoding: "utf8",
      windowsHide: true,
      env: { ...process.env, HS_GATE_FILE: filePath },
    },
  );
  if (result.status !== 0) {
    throw new Error(`Authenticode inspection failed: ${result.stderr}`);
  }
  return JSON.parse(result.stdout.trim());
}

function sha256(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

function assertInstalledPayloadTopology(artifacts) {
  const nsisInstalled = artifacts.find(
    (artifact) => artifact.kind === "nsis_installed_exe",
  );
  const msiInstalled = artifacts.find(
    (artifact) => artifact.kind === "msi_installed_exe",
  );
  assert(nsisInstalled, "NSIS-installed EXE evidence is missing");
  assert(msiInstalled, "MSI-installed EXE evidence is missing");
  const installers = artifacts.filter(
    (artifact) => artifact.kind === "nsis" || artifact.kind === "msi",
  );
  for (const installed of [nsisInstalled, msiInstalled, ...installers]) {
    assert(
      installed.signerSubject === nsisInstalled.signerSubject &&
        installed.signerThumbprint === nsisInstalled.signerThumbprint,
      `${installed.kind} signer must match the NSIS-installed EXE`,
    );
  }
}

function loadSelfSignedAuthenticodeEvidence() {
  assert(
    process.env.HIDDENSHIELD_AUTHENTICODE_PROVIDER ===
      "self_signed_authenticode",
    "candidate must declare self-signed Authenticode provider",
  );
  const evidencePath =
    process.env.HIDDENSHIELD_AUTHENTICODE_SIGNING_EVIDENCE_PATH;
  assert(
    evidencePath && existsSync(evidencePath),
    "self-signed Authenticode evidence is required",
  );
  const resolvedEvidencePath = path.resolve(evidencePath);
  const evidenceFiles = statSync(resolvedEvidencePath).isDirectory()
    ? readdirSync(resolvedEvidencePath)
        .filter((name) => name.endsWith(".json"))
        .map((name) => path.join(resolvedEvidencePath, name))
    : [resolvedEvidencePath];
  assert(
    evidenceFiles.length > 0,
    "self-signed signing evidence directory is empty",
  );
  const evidenceDocuments = evidenceFiles.map((filePath) =>
    JSON.parse(readFileSync(filePath, "utf8").replace(/^\uFEFF/, "")),
  );
  for (const evidence of evidenceDocuments) {
    assert(
      evidence.schemaVersion === 1 &&
        evidence.provider === "self_signed_authenticode" &&
        evidence.status === "signed" &&
        Array.isArray(evidence.files),
      "invalid self-signed Authenticode evidence",
    );
  }
  const evidence = {
    schemaVersion: 1,
    provider: "self_signed_authenticode",
    status: "signed",
    files: evidenceDocuments.flatMap((document) => document.files),
  };
  return { ...evidence, path: resolvedEvidencePath };
}

function assert(condition, message) {
  if (!condition) throw new Error(`Authenticode Gate failed: ${message}`);
}
