param(
  [Parameter(Mandatory = $true)]
  [string]$CertificateThumbprint,

  [Parameter(Mandatory = $true)]
  [string[]]$Files,

  [string]$TimestampUrl = "http://timestamp.digicert.com",

  [string]$DigestAlgorithm = "SHA256",

  [ValidateRange(1, 5)]
  [int]$MaxAttempts = 5,

  [ValidateRange(1, 60)]
  [int]$RetryDelaySeconds = 15
)

$ErrorActionPreference = "Stop"
$normalizedThumbprint = ($CertificateThumbprint -replace '\s', '').ToUpperInvariant()
$certificate = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object {
    ($_.Thumbprint -replace '\s', '').ToUpperInvariant() -eq $normalizedThumbprint
  } |
  Select-Object -First 1

if (-not $certificate -or -not $certificate.HasPrivateKey) {
  throw "Self-signed Authenticode certificate with private key is unavailable"
}
if ($certificate.Subject -ne $certificate.Issuer) {
  throw "Expected a self-signed Authenticode certificate"
}
if ($certificate.Subject -notmatch "HiddenShield Release Signing") {
  throw "Self-signed release certificate subject must contain HiddenShield Release Signing"
}

$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" `
  -Recurse `
  -Filter signtool.exe `
  -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
  Sort-Object FullName -Descending |
  Select-Object -First 1
if (-not $signtool) {
  throw "x64 SignTool is unavailable"
}

foreach ($file in $Files) {
  $resolved = (Resolve-Path -LiteralPath $file).Path
  $existingSignature = Get-AuthenticodeSignature -LiteralPath $resolved
  if ($existingSignature.Status -eq [System.Management.Automation.SignatureStatus]::Valid) {
    Write-Host "Authenticode signature already Valid; skipping duplicate signature: $resolved"
    continue
  }
  $signed = $false
  for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
    & $signtool.FullName sign `
      /fd $DigestAlgorithm `
      /sha1 $normalizedThumbprint `
      /s My `
      /tr $TimestampUrl `
      /td $DigestAlgorithm `
      $resolved
    if ($LASTEXITCODE -eq 0) {
      $signed = $true
      break
    }
    if ($attempt -lt $MaxAttempts) {
      Start-Sleep -Seconds ($RetryDelaySeconds * $attempt)
    }
  }
  if (-not $signed) {
    throw "Self-signed Authenticode signing failed after $MaxAttempts attempts: $resolved"
  }
  $signature = Get-AuthenticodeSignature -LiteralPath $resolved
  if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
    throw "Self-signed Authenticode signature is not Valid: $resolved"
  }
}

Write-Host "Self-signed Authenticode signing completed for $($Files.Count) file(s)"
