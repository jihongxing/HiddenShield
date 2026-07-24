param(
  [string]$Subject = "CN=HiddenShield Release Signing",

  [Parameter(Mandatory = $true)]
  [string]$PfxOutput,

  [Parameter(Mandatory = $true)]
  [string]$CertificatePassword,

  [int]$ValidityYears = 3
)

$ErrorActionPreference = "Stop"
if ($CertificatePassword.Length -lt 16) {
  throw "Self-signed certificate password must contain at least 16 characters"
}
if ($ValidityYears -lt 1 -or $ValidityYears -gt 5) {
  throw "ValidityYears must be between 1 and 5"
}
if (Test-Path -LiteralPath $PfxOutput) {
  throw "Refusing to overwrite existing PFX: $PfxOutput"
}

$certificate = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject $Subject `
  -FriendlyName "HiddenShield Release Signing" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -KeyAlgorithm RSA `
  -KeyLength 3072 `
  -HashAlgorithm SHA256 `
  -KeyExportPolicy Exportable `
  -NotAfter (Get-Date).AddYears($ValidityYears)

$temporaryCertificate = Join-Path $env:TEMP "$($certificate.Thumbprint).cer"
try {
  Export-Certificate -Cert $certificate -FilePath $temporaryCertificate | Out-Null
  Import-Certificate `
    -FilePath $temporaryCertificate `
    -CertStoreLocation "Cert:\CurrentUser\Root" | Out-Null
  Import-Certificate `
    -FilePath $temporaryCertificate `
    -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" | Out-Null

  $parent = Split-Path -Parent $PfxOutput
  if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  $securePassword = ConvertTo-SecureString `
    -String $CertificatePassword `
    -AsPlainText `
    -Force
  Export-PfxCertificate `
    -Cert $certificate `
    -FilePath $PfxOutput `
    -Password $securePassword | Out-Null
}
finally {
  Remove-Item -LiteralPath $temporaryCertificate -Force -ErrorAction SilentlyContinue
}

[ordered]@{
  status = "created"
  subject = $certificate.Subject
  thumbprint = $certificate.Thumbprint
  notBefore = $certificate.NotBefore.ToUniversalTime().ToString("o")
  notAfter = $certificate.NotAfter.ToUniversalTime().ToString("o")
  pfxPath = (Resolve-Path -LiteralPath $PfxOutput).Path
  publicTrust = $false
} | ConvertTo-Json
