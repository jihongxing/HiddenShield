param(
  [Parameter(Mandatory = $true)]
  [string]$RunId,

  [Parameter(Mandatory = $true)]
  [string]$CertificateThumbprint,

  [string]$TimestampUrl = "http://timestamp.digicert.com",

  [string]$OutputRoot = "artifacts/windows-signed-release-candidate",

  [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$outputDirectory = Join-Path $workspace (Join-Path $OutputRoot $RunId)
$manifestPath = Join-Path $outputDirectory "candidate-manifest.json"
$normalizedThumbprint = ($CertificateThumbprint -replace '\s', '').ToUpperInvariant()

function Get-FileEvidence {
  param([Parameter(Mandatory = $true)][string]$Path)

  $signature = Get-AuthenticodeSignature -LiteralPath $Path
  return [ordered]@{
    path = (Resolve-Path -LiteralPath $Path).Path
    sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    authenticodeStatus = $signature.Status.ToString()
    subject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
    thumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
  }
}

function Invoke-Checked {
  param([Parameter(Mandatory = $true)][string]$FilePath, [string[]]$Arguments)

  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
  }
}

if (Test-Path -LiteralPath $outputDirectory) {
  throw "Candidate output directory already exists; choose a new immutable run ID: $outputDirectory"
}

$certificate = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object {
    ($_.Thumbprint -replace '\s', '').ToUpperInvariant() -eq $normalizedThumbprint
  } |
  Select-Object -First 1
if (-not $certificate -or -not $certificate.HasPrivateKey) {
  throw "Authenticode certificate with private key is unavailable"
}

$worktreeChanges = @(& git -C $workspace status --porcelain)
if ($worktreeChanges.Count -gt 0) {
  throw "Windows signed release candidates require a clean Git worktree"
}

$sourceCommit = (& git -C $workspace rev-parse HEAD).Trim()
$version = (Get-Content -Raw (Join-Path $workspace "src-tauri\tauri.conf.json") | ConvertFrom-Json).version
$manifest = [ordered]@{
  schemaVersion = 1
  candidateKind = "windows_signed_release_candidate"
  runId = $RunId
  generatedAt = [DateTime]::UtcNow.ToString("o")
  sourceCommit = $sourceCommit
  version = $version
  certificate = [ordered]@{
    subject = $certificate.Subject
    thumbprint = $normalizedThumbprint
    timestampUrl = $TimestampUrl
  }
  rebuildProhibitedAfterManifest = $true
  status = if ($PlanOnly) { "planned" } else { "running" }
}
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

if ($PlanOnly) {
  Write-Host "Windows signed release candidate plan written: $manifestPath"
  exit 0
}

$payloadGate = Join-Path $workspace "scripts\release\verify-windows-installed-payload.ps1"
$evidenceWriter = Join-Path $workspace "scripts\release\write-self-signed-authenticode-evidence.ps1"
$signingConfigPath = Join-Path $outputDirectory "tauri.windows-signing.json"
$signingConfig = [ordered]@{
  bundle = [ordered]@{
    windows = [ordered]@{
      certificateThumbprint = $normalizedThumbprint
      digestAlgorithm = "sha256"
      timestampUrl = $TimestampUrl
      tsp = $true
    }
  }
}
$signingConfig | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $signingConfigPath -Encoding UTF8

Push-Location $workspace
try {
  $trustPolicyOutput = @(
    & node (Join-Path $workspace "scripts\release\export-offline-license-trust-policy-env.mjs")
  )
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to load the desktop offline license trust policy"
  }
  $trustPolicyAssignment = ($trustPolicyOutput | Where-Object {
    $_ -like "HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON=*"
  } | Select-Object -First 1)
  if (-not $trustPolicyAssignment) {
    throw "Desktop offline license trust policy export returned no environment assignment"
  }
  $env:HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON = $trustPolicyAssignment.Substring(
    "HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON=".Length
  )

  Invoke-Checked -FilePath "npx.cmd" -Arguments @(
    "tauri",
    "build",
    "--bundles",
    "msi,nsis",
    "--config",
    $signingConfigPath,
    "--",
    "--bin",
    "hidden_shield"
  )

  $nsis = Get-ChildItem (Join-Path $workspace "src-tauri\target\release\bundle\nsis") -Filter *.exe |
    Select-Object -First 1
  $msi = Get-ChildItem (Join-Path $workspace "src-tauri\target\release\bundle\msi") -Filter *.msi |
    Select-Object -First 1
  if (-not $nsis -or -not $msi) {
    throw "NSIS or MSI package was not generated"
  }

  $manifest.sourceArtifacts = [ordered]@{
    nsis = Get-FileEvidence -Path $nsis.FullName
    msi = Get-FileEvidence -Path $msi.FullName
  }
  $candidateArtifacts = Join-Path $outputDirectory "source-artifacts"
  New-Item -ItemType Directory -Path $candidateArtifacts -Force | Out-Null
  $frozenNsis = Join-Path $candidateArtifacts $nsis.Name
  $frozenMsi = Join-Path $candidateArtifacts $msi.Name
  Copy-Item -LiteralPath $nsis.FullName -Destination $frozenNsis -Force
  Copy-Item -LiteralPath $msi.FullName -Destination $frozenMsi -Force
  $manifest.frozenSourceArtifacts = [ordered]@{
    nsis = Get-FileEvidence -Path $frozenNsis
    msi = Get-FileEvidence -Path $frozenMsi
  }

  $payloadDirectory = Join-Path $outputDirectory "installed-payloads"
  & $payloadGate `
    -NsisPath $frozenNsis `
    -MsiPath $frozenMsi `
    -ExpectedCertificateThumbprint $normalizedThumbprint `
    -OutputDirectory $payloadDirectory
  if ($LASTEXITCODE -ne 0) {
    throw "Installed-payload signature Gate failed"
  }
  $payloadEvidencePath = Join-Path $payloadDirectory "installed-payload-signature.json"
  $payloadEvidence = Get-Content -Raw $payloadEvidencePath | ConvertFrom-Json
  $manifest.installedPayloads = $payloadEvidence

  $signingEvidence = Join-Path $outputDirectory "self-signed-authenticode-evidence.json"
  & $evidenceWriter `
    -Files @($frozenNsis, $frozenMsi, $payloadEvidence.nsis.installedExecutable.path, $payloadEvidence.msi.installedExecutable.path) `
    -EvidenceOutput $signingEvidence
  if ($LASTEXITCODE -ne 0) {
    throw "Self-signed Authenticode evidence generation failed"
  }

  $env:HIDDENSHIELD_SIGNED_NSIS_PATH = $frozenNsis
  $env:HIDDENSHIELD_SIGNED_MSI_PATH = $frozenMsi
  $env:HIDDENSHIELD_SIGNED_NSIS_INSTALLED_EXE_PATH = $payloadEvidence.nsis.installedExecutable.path
  $env:HIDDENSHIELD_SIGNED_MSI_INSTALLED_EXE_PATH = $payloadEvidence.msi.installedExecutable.path
  $env:HIDDENSHIELD_AUTHENTICODE_PROVIDER = "self_signed_authenticode"
  $env:HIDDENSHIELD_AUTHENTICODE_SIGNING_EVIDENCE_PATH = $signingEvidence
  $env:HIDDENSHIELD_AUTHENTICODE_RUN_ID = $RunId
  Invoke-Checked -FilePath "npm.cmd" -Arguments @("run", "release:authenticode-gate:candidate")

  $manifest.status = "passed"
  $manifest.authenticodeGate = Join-Path $workspace (Join-Path "artifacts\authenticode-gate" (Join-Path $RunId "authenticode-gate.json"))
}
catch {
  $manifest.status = "failed"
  $manifest.error = $_.Exception.Message
  throw
}
finally {
  $manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
  Pop-Location
}

Write-Host "Windows signed release candidate passed: $manifestPath"
