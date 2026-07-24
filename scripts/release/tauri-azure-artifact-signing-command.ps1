param(
  [Parameter(Mandatory = $true)]
  [string]$File
)

$ErrorActionPreference = "Stop"

$required = @(
  "HIDDENSHIELD_AZURE_SIGNTOOL_PATH",
  "HIDDENSHIELD_AZURE_SIGNING_DLIB_PATH",
  "HIDDENSHIELD_AZURE_SIGNING_ENDPOINT",
  "HIDDENSHIELD_AZURE_SIGNING_ACCOUNT",
  "HIDDENSHIELD_AZURE_SIGNING_PROFILE",
  "HIDDENSHIELD_AZURE_SIGNING_EVIDENCE_DIR"
)
foreach ($name in $required) {
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
    throw "Missing Azure Artifact Signing environment variable: $name"
  }
}

$evidenceDirectory = [System.IO.Path]::GetFullPath($env:HIDDENSHIELD_AZURE_SIGNING_EVIDENCE_DIR)
New-Item -ItemType Directory -Force -Path $evidenceDirectory | Out-Null
$evidencePath = Join-Path $evidenceDirectory ("azure-signing-" + [guid]::NewGuid().ToString("N") + ".json")

& (Join-Path $PSScriptRoot "sign-with-azure-artifact-signing.ps1") `
  -SigntoolPath $env:HIDDENSHIELD_AZURE_SIGNTOOL_PATH `
  -DlibPath $env:HIDDENSHIELD_AZURE_SIGNING_DLIB_PATH `
  -Endpoint $env:HIDDENSHIELD_AZURE_SIGNING_ENDPOINT `
  -CodeSigningAccountName $env:HIDDENSHIELD_AZURE_SIGNING_ACCOUNT `
  -CertificateProfileName $env:HIDDENSHIELD_AZURE_SIGNING_PROFILE `
  -Files ([System.IO.Path]::GetFullPath($File)) `
  -EvidenceOutput $evidencePath
