param(
  [Parameter(Mandatory = $true)]
  [string]$SignedBy,

  [Parameter(Mandatory = $true)]
  [string]$FormalHttpArtifact,

  [Parameter(Mandatory = $true)]
  [string]$LoadArtifact,

  [Parameter(Mandatory = $true)]
  [string]$RestoreArtifact,

  [Parameter(Mandatory = $true)]
  [string]$ObservabilityArtifact,

  [Parameter(Mandatory = $true)]
  [string]$DraftRunbookArtifact,

  [Parameter(Mandatory = $true)]
  [string]$OutputDirectory,

  [switch]$Approve
)

$ErrorActionPreference = "Stop"

if (-not $Approve) {
  throw "Release owner approval requires the explicit -Approve switch."
}

$signer = $SignedBy.Trim()
if ([string]::IsNullOrWhiteSpace($signer)) {
  throw "SignedBy must identify the human release owner."
}

function Read-PassingArtifact {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedSchema
  )

  $resolved = (Resolve-Path -LiteralPath $Path).Path
  $artifact = Get-Content -Raw -Encoding UTF8 -LiteralPath $resolved | ConvertFrom-Json
  if ($artifact.schemaVersion -ne $ExpectedSchema) {
    throw "Artifact schema mismatch for ${resolved}: expected ${ExpectedSchema}, got $($artifact.schemaVersion)"
  }
  if ($artifact.ok -ne $true -or $artifact.status -ne "passed") {
    throw "Artifact is not passing: ${resolved}"
  }
  return @{
    Path = $resolved
    Artifact = $artifact
  }
}

$formal = Read-PassingArtifact -Path $FormalHttpArtifact -ExpectedSchema "cloud_postgres_formal_http_gate_v1"
$load = Read-PassingArtifact -Path $LoadArtifact -ExpectedSchema "cloud_postgres_load_gate_artifact_v1"
$restore = Read-PassingArtifact -Path $RestoreArtifact -ExpectedSchema "cloud_postgres_restore_drill_artifact_v1"
$observability = Read-PassingArtifact -Path $ObservabilityArtifact -ExpectedSchema "cloud_postgres_observability_artifact_v1"

$draftPath = (Resolve-Path -LiteralPath $DraftRunbookArtifact).Path
$draft = Get-Content -Raw -Encoding UTF8 -LiteralPath $draftPath | ConvertFrom-Json
if ($draft.schemaVersion -ne "cloud_postgres_cutover_runbook_artifact_v1") {
  throw "Draft runbook schema mismatch: $draftPath"
}
if (-not $draft.steps -or -not $draft.rollbackTriggers) {
  throw "Draft runbook must contain cutover steps and rollback triggers."
}

$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
$timestamp = [DateTimeOffset]::UtcNow
$runId = "cloud-postgres-p5-release-owner-$($timestamp.ToUnixTimeMilliseconds())"
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)

$reviewedArtifacts = @(
  $formal.Path.Replace("\", "/"),
  $load.Path.Replace("\", "/"),
  $restore.Path.Replace("\", "/"),
  $observability.Path.Replace("\", "/"),
  $draftPath.Replace("\", "/")
)

$approvedRunbook = [ordered]@{
  schemaVersion = "cloud_postgres_cutover_runbook_artifact_v1"
  runId = $runId
  generatedAt = $timestamp.ToString("o")
  ok = $true
  status = "passed"
  productionDatabaseAllowed = $false
  environmentClass = "release_owner_reviewed_local_podman_staging_equivalent"
  scope = "cloud_copyright_core_auth_sync_registry"
  reviewStatus = "approved"
  releaseOwnerReviewed = $true
  reviewedBy = $signer
  reviewedAt = $timestamp.ToString("o")
  steps = @($draft.steps)
  rollbackTriggers = @($draft.rollbackTriggers)
  reviewedArtifacts = $reviewedArtifacts
  limitationsAccepted = @(
    "Local Podman is accepted as the staging-equivalent technical rehearsal for the cloud copyright core.",
    "This approval does not open independent cloud-vault UI, Enterprise, payment, team, cloud-video, public API or SLA capabilities."
  )
}

$runbookPath = Join-Path $outputPath "$runId-cutover-runbook.json"
[System.IO.File]::WriteAllText(
  $runbookPath,
  ($approvedRunbook | ConvertTo-Json -Depth 20),
  $utf8WithoutBom
)

$signoff = [ordered]@{
  schemaVersion = "cloud_postgres_release_owner_signoff_v1"
  runId = $runId
  generatedAt = $timestamp.ToString("o")
  ok = $true
  status = "passed"
  productionDatabaseAllowed = $false
  environmentClass = "manual_release_owner_approval"
  decision = "approved"
  humanAttestation = $true
  signedBy = $signer
  signedAt = $timestamp.ToString("o")
  approvedScope = "cloud_copyright_core_auth_sync_registry"
  reviewedArtifacts = $reviewedArtifacts + @($runbookPath.Replace("\", "/"))
  attestation = "I reviewed the formal PostgreSQL HTTP Gate, Podman load, PITR restore, observability evidence and cutover/rollback runbook for the cloud copyright core."
}

$signoffPath = Join-Path $outputPath "$runId-release-owner-signoff.json"
[System.IO.File]::WriteAllText(
  $signoffPath,
  ($signoff | ConvertTo-Json -Depth 20),
  $utf8WithoutBom
)

$env:HIDDENSHIELD_POSTGRES_FORMAL_HTTP_GATE_ARTIFACT = $formal.Path
$env:HIDDENSHIELD_POSTGRES_STAGING_LOAD_ARTIFACT = $load.Path
$env:HIDDENSHIELD_POSTGRES_BACKUP_RESTORE_ARTIFACT = $restore.Path
$env:HIDDENSHIELD_POSTGRES_OBSERVABILITY_ARTIFACT = $observability.Path
$env:HIDDENSHIELD_POSTGRES_CUTOVER_RUNBOOK_ARTIFACT = $runbookPath
$env:HIDDENSHIELD_POSTGRES_RELEASE_OWNER_SIGNOFF_ARTIFACT = $signoffPath
$env:HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY = "1"

Write-Output "Approved runbook artifact: $runbookPath"
Write-Output "Release owner signoff artifact: $signoffPath"
npm run cloud:postgres-production-readiness-gate
if ($LASTEXITCODE -ne 0) {
  throw "Cloud PostgreSQL production readiness Gate failed after release owner approval."
}
