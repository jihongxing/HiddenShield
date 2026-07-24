param(
  [Parameter(Mandatory = $true)]
  [string]$SigntoolPath,

  [Parameter(Mandatory = $true)]
  [string]$DlibPath,

  [Parameter(Mandatory = $true)]
  [string]$Endpoint,

  [Parameter(Mandatory = $true)]
  [string]$CodeSigningAccountName,

  [Parameter(Mandatory = $true)]
  [string]$CertificateProfileName,

  [Parameter(Mandatory = $true)]
  [string[]]$Files,

  [string]$TimestampUrl = "http://timestamp.acs.microsoft.com",

  [string[]]$ExcludeCredentials = @(),

  [string]$CorrelationId,

  [string]$EvidenceOutput,

  [switch]$ContractOnly
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $SigntoolPath -PathType Leaf)) {
  throw "Azure Artifact Signing requires an existing SignTool executable"
}
if (-not (Test-Path -LiteralPath $DlibPath -PathType Leaf)) {
  throw "Azure Artifact Signing requires Azure.CodeSigning.Dlib.dll"
}
if ($Endpoint -notmatch '^https://[a-z0-9-]+\.codesigning\.azure\.net/?$') {
  throw "Azure Artifact Signing endpoint must be a regional codesigning.azure.net HTTPS endpoint"
}
if ([string]::IsNullOrWhiteSpace($CodeSigningAccountName) -or
    [string]::IsNullOrWhiteSpace($CertificateProfileName)) {
  throw "Azure Artifact Signing account and certificate profile are required"
}

$resolvedFiles = @(
  foreach ($file in $Files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
      throw "Azure Artifact Signing input does not exist: $file"
    }
    [System.IO.Path]::GetFullPath($file)
  }
)

$metadata = [ordered]@{
  Endpoint = $Endpoint.TrimEnd("/")
  CodeSigningAccountName = $CodeSigningAccountName
  CertificateProfileName = $CertificateProfileName
}
if (-not [string]::IsNullOrWhiteSpace($CorrelationId)) {
  $metadata.CorrelationId = $CorrelationId
}
if ($ExcludeCredentials.Count -gt 0) {
  $metadata.ExcludeCredentials = $ExcludeCredentials
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hiddenshield-azure-signing-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
$metadataPath = Join-Path $tempRoot "metadata.json"
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText(
  $metadataPath,
  ($metadata | ConvertTo-Json -Depth 6),
  $utf8NoBom
)

try {
  $results = @()
  foreach ($file in $resolvedFiles) {
    $arguments = @(
      "sign",
      "/v",
      "/fd", "SHA256",
      "/tr", $TimestampUrl,
      "/td", "SHA256",
      "/dlib", [System.IO.Path]::GetFullPath($DlibPath),
      "/dmdf", $metadataPath,
      $file
    )

    if (-not $ContractOnly) {
      & $SigntoolPath @arguments
      if ($LASTEXITCODE -ne 0) {
        throw "Azure Artifact Signing failed for $file with exit code $LASTEXITCODE"
      }
      $signature = Get-AuthenticodeSignature -LiteralPath $file
      if ($signature.Status -ne "Valid") {
        throw "Azure Artifact Signing produced a non-valid Authenticode signature for $file"
      }
      $results += [ordered]@{
        path = $file
        sha256 = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
        status = $signature.Status.ToString()
        signerSubject = $signature.SignerCertificate.Subject
        signerThumbprint = $signature.SignerCertificate.Thumbprint
      }
    }
    else {
      $results += [ordered]@{
        path = $file
        status = "contract_only"
        command = "$SigntoolPath " + ($arguments -join " ")
      }
    }
  }

  $evidence = [ordered]@{
    schemaVersion = 1
    provider = "azure_artifact_signing"
    generatedAt = (Get-Date).ToUniversalTime().ToString("o")
    status = if ($ContractOnly) { "contract_ready" } else { "signed" }
    endpoint = $metadata.Endpoint
    codeSigningAccountName = $CodeSigningAccountName
    certificateProfileName = $CertificateProfileName
    timestampUrl = $TimestampUrl
    files = $results
  }

  $json = $evidence | ConvertTo-Json -Depth 8
  if (-not [string]::IsNullOrWhiteSpace($EvidenceOutput)) {
    $resolvedEvidence = [System.IO.Path]::GetFullPath($EvidenceOutput)
    $evidenceDirectory = Split-Path -Parent $resolvedEvidence
    if ($evidenceDirectory) {
      New-Item -ItemType Directory -Force -Path $evidenceDirectory | Out-Null
    }
    [System.IO.File]::WriteAllText($resolvedEvidence, $json, $utf8NoBom)
  }
  $json
}
finally {
  Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
