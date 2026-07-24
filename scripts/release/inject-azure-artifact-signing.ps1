param(
  [Parameter(Mandatory = $true)]
  [string]$ConfigPath
)

$ErrorActionPreference = "Stop"

$resolvedConfig = [System.IO.Path]::GetFullPath($ConfigPath)
$wrapperPath = [System.IO.Path]::GetFullPath(
  (Join-Path $PSScriptRoot "tauri-azure-artifact-signing-command.ps1")
)
if (-not (Test-Path -LiteralPath $resolvedConfig -PathType Leaf)) {
  throw "Tauri config does not exist: $resolvedConfig"
}
if (-not (Test-Path -LiteralPath $wrapperPath -PathType Leaf)) {
  throw "Azure Artifact Signing Tauri wrapper does not exist"
}

$config = Get-Content -Raw -LiteralPath $resolvedConfig | ConvertFrom-Json
if (-not $config.bundle) {
  $config | Add-Member -MemberType NoteProperty -Name bundle -Value ([pscustomobject]@{})
}
if (-not $config.bundle.windows) {
  $config.bundle | Add-Member -MemberType NoteProperty -Name windows -Value ([pscustomobject]@{})
}

$signCommand = [ordered]@{
  cmd = "powershell.exe"
  args = @(
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    $wrapperPath,
    "-File",
    "%1"
  )
}
$config.bundle.windows |
  Add-Member -MemberType NoteProperty -Name signCommand -Value $signCommand -Force
$config.bundle.windows.PSObject.Properties.Remove("certificateThumbprint")
$config.bundle.windows.PSObject.Properties.Remove("timestampUrl")
$config.bundle.windows.PSObject.Properties.Remove("digestAlgorithm")
$config.bundle.windows.PSObject.Properties.Remove("tsp")

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText(
  $resolvedConfig,
  ($config | ConvertTo-Json -Depth 100),
  $utf8NoBom
)

Write-Host "Tauri Windows signCommand configured for Azure Artifact Signing"
