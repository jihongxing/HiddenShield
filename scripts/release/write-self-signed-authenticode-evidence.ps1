param(
  [Parameter(Mandatory = $true)]
  [string[]]$Files,

  [Parameter(Mandatory = $true)]
  [string]$EvidenceOutput
)

$ErrorActionPreference = "Stop"
$evidenceFiles = @()

foreach ($file in $Files) {
  $resolved = (Resolve-Path -LiteralPath $file).Path
  $signature = Get-AuthenticodeSignature -LiteralPath $resolved
  if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
    throw "Self-signed Authenticode candidate is not Valid: $resolved ($($signature.Status))"
  }
  if (-not $signature.SignerCertificate) {
    throw "Self-signed Authenticode candidate has no signer certificate: $resolved"
  }
  if ($signature.SignerCertificate.Subject -ne $signature.SignerCertificate.Issuer) {
    throw "Expected a self-signed Authenticode certificate: $resolved"
  }

  $evidenceFiles += [ordered]@{
    path = $resolved
    sha256 = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
    status = $signature.Status.ToString()
    subject = $signature.SignerCertificate.Subject
    issuer = $signature.SignerCertificate.Issuer
    thumbprint = $signature.SignerCertificate.Thumbprint
    notBefore = $signature.SignerCertificate.NotBefore.ToUniversalTime().ToString("o")
    notAfter = $signature.SignerCertificate.NotAfter.ToUniversalTime().ToString("o")
  }
}

$evidence = [ordered]@{
  schemaVersion = 1
  provider = "self_signed_authenticode"
  status = "signed"
  generatedAt = (Get-Date).ToUniversalTime().ToString("o")
  trustScope = "service_provider_and_managed_customer_trust_store"
  publicTrust = $false
  files = $evidenceFiles
}

$parent = Split-Path -Parent $EvidenceOutput
if ($parent) {
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$evidence | ConvertTo-Json -Depth 10 |
  Set-Content -LiteralPath $EvidenceOutput -Encoding utf8

Write-Host "Self-signed Authenticode evidence written to $EvidenceOutput"
