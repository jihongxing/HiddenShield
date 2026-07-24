param(
  [Parameter(Mandatory = $true)]
  [string]$NsisPath,

  [Parameter(Mandatory = $true)]
[string]$MsiPath,

[Parameter(Mandatory = $true)]
[string]$ExpectedCertificateThumbprint,

  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"

function Get-FileEvidence {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $signature = Get-AuthenticodeSignature -LiteralPath $Path
  return [ordered]@{
    path = (Resolve-Path -LiteralPath $Path).Path
    sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    authenticodeStatus = $signature.Status.ToString()
    subject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
    thumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
  }
}

function Assert-InstalledPayload {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Kind,

    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedThumbprint
  )

  if (-not (Test-Path -LiteralPath $Path)) {
    throw "$Kind installation did not produce hidden_shield.exe: $Path"
  }

  $actual = Get-FileEvidence -Path $Path
  if ($actual.authenticodeStatus -ne "Valid") {
    throw "$Kind installed hidden_shield.exe is not Authenticode Valid: $($actual.authenticodeStatus)"
  }
  if (($actual.thumbprint -replace '\s', '').ToUpperInvariant() -ne $ExpectedThumbprint) {
    throw "$Kind installed hidden_shield.exe signer does not match the release certificate"
  }

  return $actual
}

$nsis = (Resolve-Path -LiteralPath $NsisPath).Path
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$expectedThumbprint = ($ExpectedCertificateThumbprint -replace '\s', '').ToUpperInvariant()

$output = [System.IO.Path]::GetFullPath($OutputDirectory)
$nsisDirectory = Join-Path $output "nsis-installed"
$msiDirectory = Join-Path $output "msi-installed"
$msiLogPath = Join-Path $output "msi-install.log"
if ((Test-Path -LiteralPath $nsisDirectory) -or (Test-Path -LiteralPath $msiDirectory)) {
  throw "Installed-payload output directories already exist; use a new immutable run ID"
}
New-Item -ItemType Directory -Path $output -Force | Out-Null

$nsisProcess = Start-Process -FilePath $nsis `
  -ArgumentList @("/S", "/D=$nsisDirectory") `
  -Wait `
  -PassThru `
  -WindowStyle Hidden
if ($nsisProcess.ExitCode -ne 0) {
  throw "NSIS installation failed with exit code $($nsisProcess.ExitCode)"
}
$nsisExe = Join-Path $nsisDirectory "hidden_shield.exe"
$nsisEvidence = Assert-InstalledPayload -Kind "NSIS" -Path $nsisExe -ExpectedThumbprint $expectedThumbprint

$msiArguments = @(
  "/i",
  "`"$msi`"",
  "/qn",
  "/norestart",
  "/L*v",
  "`"$msiLogPath`"",
  "INSTALLDIR=`"$msiDirectory`""
)
$msiProcess = Start-Process -FilePath "msiexec.exe" `
  -ArgumentList $msiArguments `
  -Wait `
  -PassThru `
  -WindowStyle Hidden
if ($msiProcess.ExitCode -ne 0) {
  throw "MSI installation failed with exit code $($msiProcess.ExitCode); see $msiLogPath"
}
$msiExe = Join-Path $msiDirectory "hidden_shield.exe"
$msiEvidence = Assert-InstalledPayload -Kind "MSI" -Path $msiExe -ExpectedThumbprint $expectedThumbprint

$summary = [ordered]@{
  schemaVersion = 1
  gate = "windows_installed_payload_signature"
  generatedAt = [DateTime]::UtcNow.ToString("o")
  expectedCertificateThumbprint = $expectedThumbprint
  nsis = [ordered]@{
    installerPath = $nsis
    installDirectory = $nsisDirectory
    installedExecutable = $nsisEvidence
  }
  msi = [ordered]@{
    installerPath = $msi
    installDirectory = $msiDirectory
    installLog = $msiLogPath
    installedExecutable = $msiEvidence
  }
  status = "passed"
}
$summaryPath = Join-Path $output "installed-payload-signature.json"
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $summaryPath -Encoding UTF8
Write-Host "Windows installed-payload signature Gate passed: $summaryPath"
