param(
  [string]$Pkcs11Module,
  [string]$ExpectedKsp,
  [string]$OutputPath
)

$ErrorActionPreference = "Stop"

$providers = @()
$certutilOutput = & certutil.exe -csplist 2>&1
foreach ($line in $certutilOutput) {
  $text = "$line".Trim()
  if ($text.StartsWith(": ")) {
    $provider = $text.Substring(2)
    if ($provider -notmatch "^\d+\s+-\s+PROV_") {
      $providers += $provider
    }
  }
}

$codeSigningOid = "1.3.6.1.5.5.7.3.3"
$certificates = @(
  Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -ErrorAction SilentlyContinue |
    Where-Object {
      $_.EnhancedKeyUsageList.ObjectId -contains $codeSigningOid
    } |
    ForEach-Object {
      [pscustomobject]@{
        subject = $_.Subject
        issuer = $_.Issuer
        thumbprint = $_.Thumbprint
        notAfter = $_.NotAfter.ToUniversalTime().ToString("o")
        hasPrivateKey = $_.HasPrivateKey
        selfSigned = $_.Subject -eq $_.Issuer
      }
    }
)

$pkcs11 = [pscustomobject]@{
  configured = -not [string]::IsNullOrWhiteSpace($Pkcs11Module)
  path = $Pkcs11Module
  exists = if ($Pkcs11Module) { Test-Path -LiteralPath $Pkcs11Module } else { $false }
}

$summary = [pscustomobject]@{
  schemaVersion = 1
  generatedAt = (Get-Date).ToUniversalTime().ToString("o")
  authenticode = [pscustomobject]@{
    expectedKsp = $ExpectedKsp
    expectedKspPresent = if ($ExpectedKsp) { $providers -contains $ExpectedKsp } else { $null }
    providers = $providers
    codeSigningCertificates = $certificates
  }
  hslic1Signer = [pscustomobject]@{
    requiredAlgorithm = "Ed25519"
    recommendedInterface = "PKCS#11 CKM_EDDSA"
    pkcs11Module = $pkcs11
    customerHardwareRequired = $false
  }
}

$json = $summary | ConvertTo-Json -Depth 8
if ($OutputPath) {
  $resolved = [System.IO.Path]::GetFullPath($OutputPath)
  $directory = Split-Path -Parent $resolved
  if ($directory) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
  }
  Set-Content -LiteralPath $resolved -Value $json -Encoding utf8
}
$json
