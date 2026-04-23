param(
  [Parameter(Mandatory = $true)]
  [string]$CertificateBase64,

  [Parameter(Mandatory = $true)]
  [string]$CertificatePassword,

  [Parameter(Mandatory = $true)]
  [string]$ConfigPath,

  [Parameter(Mandatory = $true)]
  [string]$TimestampUrl,

  [string]$DigestAlgorithm = "sha256",

  [bool]$UseTsp = $false
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($CertificateBase64)) {
  throw "WINDOWS_CERTIFICATE is required"
}

if ([string]::IsNullOrWhiteSpace($CertificatePassword)) {
  throw "WINDOWS_CERTIFICATE_PASSWORD is required"
}

if ([string]::IsNullOrWhiteSpace($TimestampUrl)) {
  throw "WINDOWS_TIMESTAMP_URL is required"
}

if (-not $TimestampUrl.StartsWith("https://")) {
  throw "WINDOWS_TIMESTAMP_URL must use HTTPS"
}

$workDir = Join-Path $PWD "certificate"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

$tempCertPath = Join-Path $workDir "tempCert.txt"
$pfxPath = Join-Path $workDir "certificate.pfx"

Set-Content -Path $tempCertPath -Value $CertificateBase64
certutil -decode $tempCertPath $pfxPath | Out-Null
Remove-Item $tempCertPath -Force

$securePassword = ConvertTo-SecureString -String $CertificatePassword -Force -AsPlainText
$imported = Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $securePassword

if (-not $imported) {
  throw "Failed to import Windows code-signing certificate"
}

$thumbprint = ($imported.Thumbprint -replace '\s', '').ToUpperInvariant()
if (-not $thumbprint) {
  throw "Unable to resolve certificate thumbprint"
}

$config = Get-Content -Raw $ConfigPath | ConvertFrom-Json
if (-not $config.bundle) {
  $config | Add-Member -MemberType NoteProperty -Name bundle -Value ([pscustomobject]@{})
}
if (-not $config.bundle.windows) {
  $config.bundle | Add-Member -MemberType NoteProperty -Name windows -Value ([pscustomobject]@{})
}

$config.bundle.windows.certificateThumbprint = $thumbprint
$config.bundle.windows.digestAlgorithm = $DigestAlgorithm
$config.bundle.windows.timestampUrl = $TimestampUrl
$config.bundle.windows.tsp = $UseTsp

$config | ConvertTo-Json -Depth 100 | Set-Content -Path $ConfigPath -Encoding utf8

if ($env:GITHUB_OUTPUT) {
  "thumbprint=$thumbprint" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
}

Write-Host "Windows signing config injected with certificate thumbprint $thumbprint"
