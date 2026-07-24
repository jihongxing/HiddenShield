param(
  [string]$CertificateBase64,

  [string]$CertificatePassword,

  [Parameter(Mandatory = $true)]
  [string]$ConfigPath,

  [Parameter(Mandatory = $true)]
  [string]$TimestampUrl,

  [string]$DigestAlgorithm = "sha256",

  [bool]$UseTsp = $false,

  [string]$ExistingCertificateThumbprint,

  [string]$ExpectedKspProvider,

  [switch]$AllowSelfSignedCertificate,

  [switch]$AllowInternalQaCertificate,

  [switch]$PrepareOnly
)

$ErrorActionPreference = "Stop"

$useExistingCertificate = -not [string]::IsNullOrWhiteSpace($ExistingCertificateThumbprint)
$usePfxImport = -not [string]::IsNullOrWhiteSpace($CertificateBase64)
if ($useExistingCertificate -eq $usePfxImport) {
  throw "Provide exactly one Authenticode source: CertificateBase64 or ExistingCertificateThumbprint"
}
if ($usePfxImport -and [string]::IsNullOrWhiteSpace($CertificatePassword)) {
  throw "WINDOWS_CERTIFICATE_PASSWORD is required for PFX import"
}

if ([string]::IsNullOrWhiteSpace($TimestampUrl)) {
  throw "WINDOWS_TIMESTAMP_URL is required"
}

$allowedHttpTimestampUrls = @(
  "http://timestamp.digicert.com",
  "http://timestamp.sectigo.com"
)
if (
  -not $TimestampUrl.StartsWith("https://") -and
  $TimestampUrl -notin $allowedHttpTimestampUrls
) {
  throw "WINDOWS_TIMESTAMP_URL must use HTTPS or an explicitly allowed signed timestamp endpoint"
}

$importedCertificates = @()
if ($usePfxImport) {
  $workDir = Join-Path $PWD "certificate"
  New-Item -ItemType Directory -Force -Path $workDir | Out-Null
  $tempCertPath = Join-Path $workDir "tempCert.txt"
  $pfxPath = Join-Path $workDir "certificate.pfx"

  try {
    Set-Content -Path $tempCertPath -Value $CertificateBase64
    certutil -decode $tempCertPath $pfxPath | Out-Null
    $securePassword = ConvertTo-SecureString -String $CertificatePassword -Force -AsPlainText
    $importedCertificates = @(
      Import-PfxCertificate `
        -FilePath $pfxPath `
        -CertStoreLocation Cert:\CurrentUser\My `
        -Password $securePassword
    )
    if ($importedCertificates.Count -eq 0) {
      throw "Failed to import Windows code-signing certificate"
    }
  }
  finally {
    Remove-Item -LiteralPath $tempCertPath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $pfxPath -Force -ErrorAction SilentlyContinue
  }
}
else {
  $normalizedThumbprint = ($ExistingCertificateThumbprint -replace '\s', '').ToUpperInvariant()
  $importedCertificates = @(
    Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -ErrorAction SilentlyContinue |
      Where-Object {
        ($_.Thumbprint -replace '\s', '').ToUpperInvariant() -eq $normalizedThumbprint
      }
  )
  if ($importedCertificates.Count -ne 1) {
    throw "Existing Authenticode certificate thumbprint must resolve to exactly one certificate"
  }
}

try {
  $codeSigningOid = "1.3.6.1.5.5.7.3.3"
  $signingCertificate = $null

  foreach ($certificate in $importedCertificates) {
    if (-not $certificate.HasPrivateKey) {
      continue
    }

    $ekuExtension = $certificate.Extensions |
      Where-Object { $_.Oid.Value -eq "2.5.29.37" } |
      Select-Object -First 1

    if (-not $ekuExtension) {
      continue
    }

    $decodedEku = [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new()
    $decodedEku.CopyFrom($ekuExtension)
    $hasCodeSigningEku = $decodedEku.EnhancedKeyUsages |
      Where-Object { $_.Value -eq $codeSigningOid } |
      Select-Object -First 1

    if ($hasCodeSigningEku) {
      $signingCertificate = $certificate
      break
    }
  }

  if (-not $signingCertificate) {
    throw "PFX does not contain a certificate with a private key and the Code Signing EKU"
  }

  $now = Get-Date
  if ($signingCertificate.NotBefore -gt $now -or $signingCertificate.NotAfter -le $now) {
    throw "Windows code-signing certificate is not currently valid"
  }

  $isSelfSigned = $signingCertificate.Subject -eq $signingCertificate.Issuer
  $isInternalQa = $signingCertificate.Subject -match "HiddenShield Internal QA"
  if ($isInternalQa -and -not $AllowInternalQaCertificate) {
    throw "HiddenShield Internal QA certificates are rejected"
  }
  if (
    $isSelfSigned -and
    -not $AllowSelfSignedCertificate -and
    -not $AllowInternalQaCertificate
  ) {
    throw "Self-signed Authenticode certificates require AllowSelfSignedCertificate"
  }
  if ($AllowSelfSignedCertificate -and -not $isSelfSigned) {
    throw "The free Authenticode baseline requires a self-signed certificate"
  }
  if ($AllowSelfSignedCertificate -and $signingCertificate.Subject -notmatch "HiddenShield Release Signing") {
    throw "Self-signed release certificate subject must contain HiddenShield Release Signing"
  }

  $thumbprint = ($signingCertificate.Thumbprint -replace '\s', '').ToUpperInvariant()
  if (-not $thumbprint) {
    throw "Unable to resolve certificate thumbprint"
  }

  if (-not [string]::IsNullOrWhiteSpace($ExpectedKspProvider)) {
    $certutilArguments = @("-store", "My", $thumbprint)
    if ($signingCertificate.PSPath -match "CurrentUser") {
      $certutilArguments = @("-user") + $certutilArguments
    }
    $certificateDetails = (& certutil.exe @certutilArguments 2>&1) -join "`n"
    if ($certificateDetails -notmatch [regex]::Escape($ExpectedKspProvider)) {
      throw "Authenticode certificate is not backed by expected KSP provider: $ExpectedKspProvider"
    }
  }

  if ($AllowSelfSignedCertificate) {
    $temporaryCertificate = Join-Path $env:TEMP "$thumbprint.cer"
    try {
      Export-Certificate -Cert $signingCertificate -FilePath $temporaryCertificate | Out-Null
      Import-Certificate `
        -FilePath $temporaryCertificate `
        -CertStoreLocation "Cert:\CurrentUser\Root" | Out-Null
      Import-Certificate `
        -FilePath $temporaryCertificate `
        -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" | Out-Null
    }
    finally {
      Remove-Item -LiteralPath $temporaryCertificate -Force -ErrorAction SilentlyContinue
    }
  }
}
catch {
  throw
}

if ($PrepareOnly) {
  if ($env:GITHUB_OUTPUT) {
    "thumbprint=$thumbprint" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
  }
  Write-Host "Windows signing certificate prepared without mutating Tauri configuration"
  exit 0
}

$config = Get-Content -Raw $ConfigPath | ConvertFrom-Json
if (-not $config.bundle) {
  $config | Add-Member -MemberType NoteProperty -Name bundle -Value ([pscustomobject]@{})
}
if (-not $config.bundle.windows) {
  $config.bundle | Add-Member -MemberType NoteProperty -Name windows -Value ([pscustomobject]@{})
}

$config.bundle.windows |
  Add-Member -MemberType NoteProperty -Name certificateThumbprint -Value $thumbprint -Force
$config.bundle.windows |
  Add-Member -MemberType NoteProperty -Name digestAlgorithm -Value $DigestAlgorithm -Force
$config.bundle.windows |
  Add-Member -MemberType NoteProperty -Name timestampUrl -Value $TimestampUrl -Force
$config.bundle.windows |
  Add-Member -MemberType NoteProperty -Name tsp -Value $UseTsp -Force

$configJson = $config | ConvertTo-Json -Depth 100
[System.IO.File]::WriteAllText(
  $ConfigPath,
  $configJson,
  [System.Text.UTF8Encoding]::new($false)
)

if ($env:GITHUB_OUTPUT) {
  "thumbprint=$thumbprint" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
}

Write-Host "Windows signing config injected with certificate thumbprint $thumbprint"
if ($AllowSelfSignedCertificate) {
  Write-Warning "This self-signed publisher is trusted only where the certificate is installed in the Windows trust store."
}
