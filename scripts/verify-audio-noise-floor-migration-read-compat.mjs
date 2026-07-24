import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function read(path) {
  return readFileSync(path, 'utf8');
}

const paths = {
  schema: 'watermark-core/fixtures/audio-noise-floor-migration/manifest.schema.json',
  example: 'watermark-core/fixtures/audio-noise-floor-migration/manifest.example.json',
  newCandidateDraft:
    'watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate/manifest.draft.json',
  bin: 'watermark-core/src/bin/audio_noise_floor_migration_read_compat.rs',
  audio: 'watermark-core/src/audio.rs',
};

for (const path of Object.values(paths)) {
  assert(existsSync(path), `${path} must exist`);
}

const sources = {
  packageJson: read('package.json'),
  schema: read(paths.schema),
  example: read(paths.example),
  newCandidateDraft: read(paths.newCandidateDraft),
  bin: read(paths.bin),
  audio: read(paths.audio),
  migrationDesign: read('docs/音频噪声底跨端可读频带策略迁移设计.md'),
  sharedCorePlan: read('docs/共享水印核心与跨端互验推进计划.md'),
  dualRoadmap: read('docs/双端能力一致性Roadmap.md'),
  coreDoc: read('docs/watermark-core能力说明.md'),
  capabilityBoundary: read('docs/当前真实能力边界说明.md'),
  architectureContract: read('scripts/verify-watermark-architecture-contract.mjs'),
};

assert(
  sources.packageJson.includes('"watermark:audio-noise-floor-migration-read-compat"') &&
    sources.packageJson.includes('verify-audio-noise-floor-migration-read-compat.mjs'),
  'package.json must expose watermark:audio-noise-floor-migration-read-compat',
);

for (const blockedScript of [
  '"watermark:audio-noise-floor-migration-experiment"',
  '"watermark:audio-noise-floor-migration-release"',
]) {
  assert(
    !sources.packageJson.includes(blockedScript),
    `${blockedScript} must not be exposed before read compat and release promotion are complete`,
  );
}

for (const token of [
  'audio_noise_floor_migration_manifest_v1',
  'read_compat_legacy_v3',
  'v3_recovery_2_8k_legacy',
  'generated_legacy_v3_wav',
  'file_backed_legacy_v3_wav',
  'watermark_core_legacy',
  'desktop_legacy',
  'mobile_legacy',
  'android_native_legacy',
  'field_noise_noise_floor',
  'current_default_extractor',
  'legacy_v3_readonly_candidate',
  'plannedExtractorReadOrder',
  'plannedReportFields',
  'candidateScanAttempted',
  'candidateScanProfiles',
  'HS-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}',
]) {
  assert(sources.schema.includes(token), `manifest schema must include ${token}`);
}

const manifest = JSON.parse(sources.example);
const newCandidateDraft = JSON.parse(sources.newCandidateDraft);
assert(
  manifest.schemaVersion === 'audio_noise_floor_migration_manifest_v1' &&
    manifest.migrationPhase === 'read_compat_legacy_v3' &&
    manifest.payloadProtocolVersion === 3 &&
    manifest.payloadBytesLength === 39 &&
    manifest.audioStrategyVersion === 'v3_recovery_2_8k_legacy',
  'manifest example must stay on legacy V3/39 read compat',
);
assert(
  manifest.expectedExtractorPath ===
    'WatermarkService::extract -> audio::extract_watermark_wav_readonly_candidate_bytes_with_delta',
  'manifest example must document the shared-core extractor path',
);
assert(
  manifest.formalThresholds?.fieldNoiseMinSnrDb === 44.0 &&
    manifest.formalThresholds?.extractionConfidenceMin === 0.99 &&
    manifest.formalThresholds?.thresholdsMustNotDrop === true,
  'manifest example must preserve formal threshold floors',
);
assert(
  manifest.rollbackPolicy?.writeStrategyFlagRequired === true &&
    manifest.rollbackPolicy?.legacyFallbackRequired === true &&
    manifest.rollbackPolicy?.platformAlgorithmDriftForbidden === true &&
    manifest.rollbackPolicy?.formalThresholdsMustNotDrop === true,
  'manifest example must preserve rollback policy',
);
assert(
  manifest.plannedExtractorReadOrder?.join(' > ') ===
    [
      'v3_noise_floor_migrated_band_v1_candidate',
      'v3_recovery_2_8k_legacy',
      'legacy_v3_readonly_candidate',
      'v2_rollback_legacy',
    ].join(' > '),
  'manifest example must freeze the planned extractor read order',
);
for (const field of [
  'watermarkUid',
  'payloadProtocolVersion',
  'payloadBytesLength',
  'audioStrategyVersion',
  'extractorPath',
  'extractorFallbackPath',
  'candidateFailureCode',
  'candidateFailureMessage',
  'candidateFailureMatrix',
  'candidateScanAttempted',
  'candidateScanProfiles',
  'extractionConfidence',
  'readCompatibilityMode',
]) {
  assert(manifest.plannedReportFields?.includes(field), `manifest must include report field ${field}`);
}

for (const origin of ['watermark_core_legacy', 'desktop_legacy', 'mobile_legacy']) {
  assert(
    manifest.fixtures?.some((fixture) => fixture.originEndpoint === origin),
    `manifest example must include ${origin}`,
  );
}

for (const fixture of manifest.fixtures ?? []) {
  assert(
    ['generated_legacy_v3_wav', 'file_backed_legacy_v3_wav'].includes(fixture.artifactMode) &&
      fixture.audioProfile === 'field_noise_noise_floor' &&
      fixture.payloadProtocolVersion === 3 &&
      fixture.payloadBytesLength === 39 &&
      fixture.audioStrategyVersion === 'v3_recovery_2_8k_legacy' &&
      fixture.minExtractionConfidence >= 0.99,
    `${fixture.sampleId} must remain a legacy V3/39 field-noise read-compat fixture`,
  );
  assert(
    /^HS-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}$/.test(
      fixture.expectedWatermarkUid,
    ),
    `${fixture.sampleId} must use long-form HS UID`,
  );
  assert(
    fixture.expectedReadPaths?.includes('current_default_extractor') &&
      fixture.expectedReadPaths?.includes('legacy_v3_readonly_candidate'),
    `${fixture.sampleId} must cover both current and legacy read paths`,
  );
  if (fixture.artifactMode === 'file_backed_legacy_v3_wav') {
    assert(fixture.protectedPath && existsSync(fixture.protectedPath), `${fixture.sampleId} file must exist`);
    assert(/^[0-9a-f]{64}$/.test(fixture.sha256 ?? ''), `${fixture.sampleId} must include sha256`);
    assert(Number.isInteger(fixture.bytes) && fixture.bytes > 0, `${fixture.sampleId} must include bytes`);
    assert(fixture.generatedBy, `${fixture.sampleId} must include generatedBy`);
  }
}

for (const origin of ['desktop_legacy', 'android_native_legacy']) {
  assert(
    manifest.fixtures?.some(
      (fixture) =>
        fixture.originEndpoint === origin && fixture.artifactMode === 'file_backed_legacy_v3_wav',
    ),
    `manifest example must include file-backed ${origin}`,
  );
}

assert(
  newCandidateDraft.schemaVersion === 'audio_noise_floor_new_candidate_manifest_draft_v1' &&
    newCandidateDraft.draftOnly === true &&
    newCandidateDraft.artifactRoot ===
      'watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate' &&
    newCandidateDraft.mediaMutationAllowedInThisTask === false &&
    newCandidateDraft.writingImplementationAllowed === false,
  'new candidate draft manifest must remain design-only and must not trigger media mutation',
);
assert(
  newCandidateDraft.fixtureClasses?.some(
    (entry) =>
      entry.classId === 'legacy_v3_fixture' &&
      entry.expectedCandidateFailureCode === 'candidate_payload_not_found' &&
      entry.legacyFallbackAllowed === true,
  ) &&
    newCandidateDraft.fixtureClasses?.some(
      (entry) =>
        entry.classId === 'new_candidate_fixture' &&
        entry.expectedCandidateStatus === 'candidate_read_succeeded' &&
        entry.legacyFallbackOnlyAllowed === false,
    ),
  'new candidate draft manifest must distinguish legacy fallback from new-candidate required hit',
);
const draftDispositions = new Set(
  (newCandidateDraft.readCompatBlockingMatrix ?? []).map((entry) => entry.disposition),
);
for (const disposition of [
  'block_legacy_misdetected_as_new_candidate',
  'block_new_candidate_not_read_by_candidate',
  'block_candidate_payload_invalid',
  'block_candidate_scan_regressed_to_stub',
]) {
  assert(draftDispositions.has(disposition), `new candidate draft matrix must include ${disposition}`);
}
assert(
  (newCandidateDraft.plannedNewCandidateFixtures ?? []).every(
    (fixture) => fixture.status === 'paused_rc1_no_bytes_until_field_noise_blocker_is_resolved',
  ),
  'new candidate draft fixtures must remain paused RC1 placeholders until the field-noise blocker is resolved',
);

for (const token of [
  'WatermarkService::embed',
  'WatermarkService::extract',
  'extract_watermark_wav_readonly_candidate_bytes',
  'audio_v3_quality_diagnostics',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'run_new_extractor_candidate_read',
  'extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes',
  'v3_noise_floor_migrated_band_v1_candidate',
  'v3_recovery_2_8k_legacy',
  'extractorPath',
  'extractorFallbackPath',
  'candidateFailureCode',
  'candidateFailureMessage',
  'candidateFailureMatrix',
  'candidateScanAttempted',
  'candidateScanProfiles',
  'candidate_failure_matrix_json',
  'count_candidate_failure_code',
  'readCompatibilityMode',
  'AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE',
  'CandidateNotImplementedNoFrequencyStrategy',
  'CandidateInputInvalid',
  'CandidateAudioTooShort',
  'CandidatePayloadNotFound',
  'CandidatePayloadInvalid',
  'read_compat_legacy_v3',
  'file_backed_legacy_v3_wav',
  'planned_extractor_read_order',
  'planned_report_fields',
]) {
  assert(sources.bin.includes(token), `read compat bin must include ${token}`);
}

for (const token of [
  'AudioNoiseFloorMigrationCandidateFailureCode',
  'AudioNoiseFloorMigrationCandidateReadError',
  'extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes',
  'extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate',
  'candidate_input_invalid',
  'candidate_audio_too_short',
  'candidate_not_implemented_no_frequency_strategy',
  'candidate_payload_not_found',
  'candidate_payload_invalid',
  'legacy_v3_read_compat_candidate_interface_fallback',
]) {
  assert(sources.audio.includes(token), `watermark-core audio candidate interface must include ${token}`);
}

for (const token of [
  'watermark:audio-noise-floor-migration-read-compat',
  'manifest.schema.json',
  'manifest.example.json',
  '旧策略样本',
  '双端旧产物',
  '不进入实际算法迁移',
  'read-only candidate 扫描最小实现',
  'candidateScanAttempted=true',
  'candidate_payload_not_found',
]) {
  assert(sources.migrationDesign.includes(token), `migration design must mention ${token}`);
}

for (const token of [
  '新增 migration fixture manifest 和 read compat gate',
  'watermark:audio-noise-floor-migration-read-compat',
  '写入迁移实现未开始',
]) {
  assert(sources.sharedCorePlan.includes(token), `shared core plan must mention ${token}`);
}

assert(
  sources.dualRoadmap.includes('watermark:audio-noise-floor-migration-read-compat') &&
    sources.dualRoadmap.includes('写入迁移未开始'),
  'dual roadmap must record read compat as pre-algorithm migration work',
);

assert(
  sources.coreDoc.includes('watermark:audio-noise-floor-migration-read-compat') &&
    sources.coreDoc.includes('不实现新频带写入策略'),
  'watermark-core capability doc must record the read-compat gate boundary',
);

assert(
  sources.capabilityBoundary.includes('跨端可读频带策略迁移当前只是设计 / read-only scan 准备') &&
    sources.capabilityBoundary.includes('read-compat'),
  'capability boundary must keep migration as non-formal algorithm capability while mentioning read-compat',
);

assert(
  sources.architectureContract.includes('blind-watermark algorithm code must stay in watermark-core'),
  'architecture contract must continue forbidding platform-layer algorithm drift',
);

const result = spawnSync(
  'cargo',
  [
    'run',
    '--manifest-path',
    'watermark-core/Cargo.toml',
    '--bin',
    'audio_noise_floor_migration_read_compat',
    '--',
    '--manifest',
    paths.example,
  ],
  {
    stdio: ['ignore', 'pipe', 'inherit'],
    encoding: 'utf8',
    shell: process.platform === 'win32',
  },
);

if (result.status !== 0) {
  throw new Error(`audio noise-floor migration read compat failed with exit code ${result.status}`);
}

const report = JSON.parse(result.stdout);
assert(report.extractorPath === 'v3_recovery_2_8k_legacy', 'read compat report must hit legacy V3 fallback');
assert(
  report.extractorFallbackPath ===
    'v3_noise_floor_migrated_band_v1_candidate -> v3_recovery_2_8k_legacy',
  'read compat report must record candidate interface -> legacy fallback',
);
assert(
  report.readCompatibilityMode === 'legacy_v3_read_compat_candidate_interface_fallback',
  'read compat report must record candidate interface fallback mode',
);
assert(
  report.candidateScanAttempted === true &&
    Array.isArray(report.candidateScanProfiles) &&
    report.candidateScanProfiles.length >= 3,
  'read compat report must record read-only candidate scan profiles',
);
assert(
  report.candidateFailureCode === 'candidate_payload_not_found',
  'read compat report must record the current read-only scan miss failure code',
);
assert(
  report.candidateFailureMessage?.includes('candidate scan did not find'),
  'read compat report must record the current candidate scan miss message',
);
const matrix = report.candidateFailureMatrix ?? [];
const matrixByCode = new Map(matrix.map((entry) => [entry.code, entry]));
for (const code of [
  'candidate_not_implemented_no_frequency_strategy',
  'candidate_input_invalid',
  'candidate_audio_too_short',
  'candidate_payload_not_found',
  'candidate_payload_invalid',
]) {
  assert(matrixByCode.has(code), `candidate failure matrix must include ${code}`);
}
assert(
  matrixByCode.get('candidate_not_implemented_no_frequency_strategy')?.expectedHandling ===
    'fail_read_compat_gate',
  'not-implemented candidate failure must now be treated as a scan regression',
);
assert(
  matrixByCode.get('candidate_payload_not_found')?.currentObservedCount ===
    report.fixtures?.length,
  'all current legacy fixtures must observe payload-not-found candidate scan miss before fallback',
);
assert(
  matrixByCode.get('candidate_input_invalid')?.expectedHandling === 'fail_read_compat_gate' &&
    matrixByCode.get('candidate_audio_too_short')?.expectedHandling === 'fail_read_compat_gate',
  'invalid input and too-short candidate failures must fail the gate',
);
assert(
  matrixByCode.get('candidate_payload_not_found')?.expectedHandling ===
    'legacy_fixture_may_fallback_new_candidate_fixture_must_block' &&
    matrixByCode.get('candidate_payload_invalid')?.expectedHandling ===
      'legacy_fixture_may_fallback_new_candidate_fixture_must_block',
  'future payload miss/invalid failures must distinguish legacy fallback from new-candidate blocking',
);
for (const code of [
  'candidate_not_implemented_no_frequency_strategy',
  'candidate_input_invalid',
  'candidate_audio_too_short',
  'candidate_payload_invalid',
]) {
  assert(matrixByCode.get(code)?.currentObservedCount === 0, `${code} must not occur in current legacy fixtures`);
}
for (const fixture of report.fixtures ?? []) {
  assert(fixture.extractorPath === report.extractorPath, `${fixture.sampleId} must report legacy extractor path`);
  assert(
    fixture.extractorFallbackPath === report.extractorFallbackPath,
    `${fixture.sampleId} must report candidate interface fallback path`,
  );
  assert(
    fixture.readCompatibilityMode === report.readCompatibilityMode,
    `${fixture.sampleId} must report candidate interface fallback read compatibility mode`,
  );
  assert(
    fixture.candidateFailureCode === report.candidateFailureCode &&
      fixture.newExtractorCandidate?.status === 'candidate_failed_fallback_required' &&
      fixture.newExtractorCandidate?.readOnly === true &&
      fixture.newExtractorCandidate?.scanAttempted === true,
    `${fixture.sampleId} must keep new extractor candidate as a read-only scan with typed fallback failure`,
  );
}

console.log('Audio noise-floor migration read compatibility passed');
