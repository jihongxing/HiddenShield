import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { extname, join } from 'node:path';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function read(path) {
  return readFileSync(path, 'utf8');
}

const sources = {
  packageJson: read('package.json'),
  migrationDesign: read('docs/音频噪声底跨端可读频带策略迁移设计.md'),
  bandSelectionDesign: read('docs/音频噪声底频带选择实验设计.md'),
  tuningTask: read('docs/音频V3强度与噪声段处理调参任务.md'),
  sharedCorePlan: read('docs/共享水印核心与跨端互验推进计划.md'),
  dualRoadmap: read('docs/双端能力一致性Roadmap.md'),
  capabilityBoundary: read('docs/当前真实能力边界说明.md'),
  qualityDoc: read('docs/感知质量发布门禁设计.md'),
  releaseQualityGate: read('watermark-core/src/bin/v3_quality_release_gate.rs'),
  bandSelectionBin: read('watermark-core/src/bin/audio_noise_floor_band_selection_experiment.rs'),
  readCompatBin: read('watermark-core/src/bin/audio_noise_floor_migration_read_compat.rs'),
  audioCore: read('watermark-core/src/audio.rs'),
  migrationManifestSchema: read(
    'watermark-core/fixtures/audio-noise-floor-migration/manifest.schema.json',
  ),
  migrationManifestExample: read(
    'watermark-core/fixtures/audio-noise-floor-migration/manifest.example.json',
  ),
  newCandidateManifestDraft: read(
    'watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate/manifest.draft.json',
  ),
  architectureContract: read('scripts/verify-watermark-architecture-contract.mjs'),
};

assert(
  sources.packageJson.includes('"watermark:audio-noise-floor-migration-contract"') &&
    sources.packageJson.includes('verify-audio-noise-floor-migration-contract.mjs') &&
    sources.packageJson.includes('"watermark:audio-noise-floor-migration-read-compat"') &&
    sources.packageJson.includes('verify-audio-noise-floor-migration-read-compat.mjs'),
  'package.json must expose watermark:audio-noise-floor-migration-contract and read-compat',
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
  '本文是 `field-noise` / 稳定噪声底 profile 的下一阶段设计闸门',
  '不进入真正的 `watermark-core` 频带策略迁移实现',
  '不改变 V3/39 payload 格式',
  '不改变版权编号格式',
  '新旧 extractor 兼容',
  'desktop old write -> mobile new read',
  'mobile new write -> desktop new read',
  'fixtureSchemaVersion',
  'audioStrategyVersion',
  'expectedExtractorPath',
  'rollbackPolicy',
  'watermark:audio-noise-floor-migration-contract',
  'watermark:audio-noise-floor-migration-read-compat',
  'field-noise >= 44 dB',
  'extractionConfidence >= 0.99',
  'platform_algorithm_drift',
  '禁止平台层算法漂移',
  'read-only candidate 扫描最小实现',
  'candidateScanAttempted=true',
  'candidateScanProfiles',
  'candidate_payload_not_found',
  '不接正式 UI / mock / release gate 默认路径',
  'protected-new-candidate manifest 草案与阻断矩阵',
  'audio_noise_floor_new_candidate_manifest_draft_v1',
  'legacy_misdetected_as_new_candidate',
  'new_candidate_not_read_by_candidate',
  'candidate_scan_regressed_to_stub',
]) {
  assert(
    sources.migrationDesign.includes(token),
    `migration design must include required contract token: ${token}`,
  );
}

for (const token of [
  'audio_noise_floor_migration_manifest_v1',
  'read_compat_legacy_v3',
  'generated_legacy_v3_wav',
  'file_backed_legacy_v3_wav',
  'watermark_core_legacy',
  'desktop_legacy',
  'mobile_legacy',
  'android_native_legacy',
  'v3_recovery_2_8k_legacy',
  'current_default_extractor',
  'legacy_v3_readonly_candidate',
  'plannedExtractorReadOrder',
  'plannedReportFields',
  'candidateFailureCode',
  'candidateFailureMessage',
  'candidateFailureMatrix',
  'candidateScanAttempted',
  'candidateScanProfiles',
]) {
  assert(
    sources.migrationManifestSchema.includes(token) &&
      sources.migrationManifestExample.includes(token),
    `migration manifest schema/example must include ${token}`,
  );
}

const newCandidateDraft = JSON.parse(sources.newCandidateManifestDraft);
assert(
  newCandidateDraft.schemaVersion === 'audio_noise_floor_new_candidate_manifest_draft_v1' &&
    newCandidateDraft.draftOnly === true &&
    newCandidateDraft.mediaMutationAllowedInThisTask === false &&
    newCandidateDraft.writingImplementationAllowed === false &&
    newCandidateDraft.formalUiMockReleaseDefaultPathAllowed === false,
  'new candidate manifest draft must stay draft-only and forbid writing/UI/release default paths',
);
assert(
  newCandidateDraft.artifactRoot ===
    'watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate' &&
    newCandidateDraft.candidateStrategyVersion === 'v3_noise_floor_migrated_band_v1_candidate' &&
    newCandidateDraft.legacyFallbackPath === 'v3_recovery_2_8k_legacy',
  'new candidate manifest draft must target the protected-new-candidate artifact root and candidate strategy',
);
assert(
  newCandidateDraft.payloadProtocolVersion === 3 &&
    newCandidateDraft.payloadBytesLength === 39 &&
    newCandidateDraft.formalThresholds?.fieldNoiseMinSnrDb === 44.0 &&
    newCandidateDraft.formalThresholds?.extractionConfidenceMin === 0.99 &&
    newCandidateDraft.formalThresholds?.thresholdsMustNotDrop === true,
  'new candidate manifest draft must preserve V3/39 and formal threshold floors',
);
assert(
  Array.isArray(newCandidateDraft.candidateScanProfiles) &&
    newCandidateDraft.candidateScanProfiles.length >= 3 &&
    newCandidateDraft.candidateScanProfiles.every((profile) => profile.readOnly === true),
  'new candidate manifest draft must preserve read-only candidate scan profiles',
);
const fixtureClasses = new Map(
  (newCandidateDraft.fixtureClasses ?? []).map((entry) => [entry.classId, entry]),
);
assert(
  fixtureClasses.get('legacy_v3_fixture')?.expectedCandidateFailureCode ===
    'candidate_payload_not_found' &&
    fixtureClasses.get('legacy_v3_fixture')?.legacyFallbackAllowed === true,
  'legacy V3 fixture class must require candidate miss plus legacy fallback',
);
assert(
  fixtureClasses.get('new_candidate_fixture')?.expectedCandidateStatus ===
    'candidate_read_succeeded' &&
    fixtureClasses.get('new_candidate_fixture')?.legacyFallbackOnlyAllowed === false,
  'new candidate fixture class must require candidate hit and forbid fallback-only pass',
);
const matrixDisposition = new Set(
  (newCandidateDraft.readCompatBlockingMatrix ?? []).map((entry) => entry.disposition),
);
for (const disposition of [
  'pass',
  'block_legacy_misdetected_as_new_candidate',
  'block_new_candidate_not_read_by_candidate',
  'block_candidate_payload_invalid',
  'block_candidate_scan_regressed_to_stub',
  'block_fixture_or_parser_regression',
]) {
  assert(matrixDisposition.has(disposition), `new candidate blocking matrix must include ${disposition}`);
}
for (const prerequisite of [
  'field_noise_release_blocker_resolved_without_threshold_drop',
  'explicit_post_rc1_writer_experiment_approval',
  'implement_writer_in_watermark_core_only_after_reapproval',
  'create_real_file_backed_new_candidate_fixtures',
  'extend_manifest_schema_from_draft_to_checked_fixture_class',
  'extend_read_compat_to_require_candidate_hit_for_new_candidate_fixture',
  'rerun_watermark_quality_gate_release_and_full_without_threshold_drop',
]) {
  assert(
    newCandidateDraft.promotionPrerequisites?.includes(prerequisite),
    `new candidate manifest draft must include promotion prerequisite ${prerequisite}`,
  );
}

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
  'file_backed_legacy_v3_wav',
  'planned_extractor_read_order',
  'planned_report_fields',
]) {
  assert(sources.readCompatBin.includes(token), `read compat bin must include ${token}`);
}

for (const token of [
  'AudioNoiseFloorMigrationCandidateFailureCode',
  'AudioNoiseFloorMigrationCandidateReadError',
  'extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes',
  'candidate_input_invalid',
  'candidate_audio_too_short',
  'candidate_not_implemented_no_frequency_strategy',
  'candidate_payload_not_found',
  'candidate_payload_invalid',
]) {
  assert(sources.audioCore.includes(token), `watermark-core audio candidate interface must include ${token}`);
}

for (const token of [
  'WatermarkService::embed',
  'WatermarkService::extract',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'AUDIO_FULL_MIN_SNR',
  'MIN_EXTRACTION_CONFIDENCE',
  'official_ui_or_mock_path_touched',
  'false',
  'cross_end_readable_frequency_strategy_migration',
]) {
  assert(
    sources.bandSelectionBin.includes(token),
    `band selection experiment must preserve isolated hard-constraint token: ${token}`,
  );
}

for (const token of [
  'codex-noise-band-selection-c3',
  'masked_pair_budget_cap',
  '21.1644 dB',
  '0.999989',
  '`docs/音频噪声底跨端可读频带策略迁移设计.md` 已新建',
  'watermark:audio-noise-floor-migration-contract',
  'watermark:audio-noise-floor-migration-read-compat',
  '当前 2-8 kHz extractor 可读 lane 内微调停止',
]) {
  assert(
    sources.bandSelectionDesign.includes(token),
    `band selection design must record A/B/C closure before migration: ${token}`,
  );
}

for (const token of [
  'watermark:audio-noise-floor-migration-contract',
  'manifest.schema.json',
  'manifest.example.json',
  'watermark:audio-noise-floor-migration-read-compat',
  '新候选 writer 实验在 RC1 暂停',
  '封版主线回到 RC1 人工 QA 与无外部依赖验收',
]) {
  assert(
    sources.tuningTask.includes(token),
    `audio V3 tuning task must record RC1 blocker/writer pause state: ${token}`,
  );
}

for (const token of [
  'Phase I-2.1：稳定噪声底音频频带策略迁移设计',
  '设计闸门、旧策略读取兼容门禁、只读 candidate interface、read-only candidate 扫描和 protected-new-candidate draft manifest 已建立，写入迁移实现未开始',
  'docs/音频噪声底跨端可读频带策略迁移设计.md',
  '实现 `watermark:audio-noise-floor-migration-contract`',
  'watermark:audio-noise-floor-migration-read-compat',
  '不得降低阈值',
  'candidateScanAttempted=true',
  'candidate_payload_not_found',
  'protected-new-candidate/manifest.draft.json',
]) {
  assert(
    sources.sharedCorePlan.includes(token),
    `shared core plan must track migration contract status: ${token}`,
  );
}

for (const token of [
  'docs/音频噪声底跨端可读频带策略迁移设计.md',
  'protected-new-candidate/manifest.draft.json',
  'new candidate draft 完成，writer 暂停，RC1 QA 继续',
  '不改变桌面 / 移动正式音频能力边界',
]) {
  assert(
    sources.dualRoadmap.includes(token),
    `dual roadmap must track design-only migration state: ${token}`,
  );
}

assert(
  sources.capabilityBoundary.includes('docs/音频噪声底跨端可读频带策略迁移设计.md') &&
    sources.capabilityBoundary.includes('跨端可读频带策略迁移当前只是设计 / read-only scan 准备，不是正式音频算法能力') &&
    sources.capabilityBoundary.includes('RC1 已暂停新频带 writer 实验'),
  'capability boundary must state the migration is design/read-only scan only, writer-paused, and not a formal audio algorithm capability',
);

assert(
  sources.migrationDesign.includes('新频带 writer 实验不再是 RC1 主线') &&
    sources.migrationDesign.includes('桌面安装版完整人工 QA') &&
    sources.bandSelectionDesign.includes('field-noise` 当前标记为 release blocker / known limitation') &&
    sources.bandSelectionDesign.includes('新候选 writer 实验在 RC1 暂停') &&
    sources.dualRoadmap.includes('writer 暂停，RC1 QA 继续'),
  'noise-floor migration docs must pause writer work and return RC1 to manual QA/no-external-dependency acceptance',
);

assert(
  sources.qualityDoc.includes('该扫描不代表写入迁移已实现') &&
    sources.qualityDoc.includes('A/B/C 均不能晋级正式算法候选'),
  'quality gate doc must not imply the read-only scan is a write migration or promotable',
);

assert(
  sources.releaseQualityGate.includes('const AUDIO_FULL_MIN_SNR: f64 = 44.0;') &&
    sources.releaseQualityGate.includes('const AUDIO_FULL_MAX_PEAK_DELTA: f64 = 0.8;') &&
    sources.releaseQualityGate.includes('const AUDIO_FULL_MAX_LUFS_DELTA: f64 = 0.5;') &&
    sources.releaseQualityGate.includes('field-noise') &&
    sources.releaseQualityGate.includes('"snr_below_threshold"'),
  'full quality gate thresholds and field-noise blocking behavior must not be lowered or removed',
);

assert(
  sources.bandSelectionBin.includes('const AUDIO_FULL_MIN_SNR: f64 = 44.0;') &&
    sources.bandSelectionBin.includes('const MIN_EXTRACTION_CONFIDENCE: f32 = 0.99;'),
  'noise-floor experiment must keep the same SNR and extraction confidence floors',
);

assert(
  sources.architectureContract.includes('forbiddenAlgorithmPatterns') &&
    sources.architectureContract.includes('blind-watermark algorithm code must stay in watermark-core') &&
    sources.architectureContract.includes('feedback-backend/src') &&
    sources.architectureContract.includes('mobile_app/lib') &&
    sources.architectureContract.includes('src-tauri/src'),
  'architecture contract must continue scanning platform layers for non-core algorithm drift',
);

const platformDriftHits = scanPlatformSourcesForMigrationDrift();
assert(
  platformDriftHits.length === 0,
  `audio noise-floor migration must remain design-only; platform/source drift found:\n${platformDriftHits.join('\n')}`,
);

console.log('Audio noise-floor migration contract passed');

function scanPlatformSourcesForMigrationDrift() {
  const roots = ['src', 'src-tauri/src', 'mobile_app/lib', 'mobile_app/rust/src', 'feedback-backend/src'];
  const allowedQaFixtureGenerators = new Set([
    'src-tauri/examples/audio_noise_floor_migration_desktop_fixture.rs',
    'mobile_app/rust/src/bin/audio_noise_floor_migration_android_fixture.rs',
  ]);
  const allowedExtensions = new Set(['.rs', '.ts', '.tsx', '.js', '.mjs', '.vue', '.dart']);
  const forbidden = [
    /audioStrategyVersion/,
    /v3_noise_floor_migrated_band_v1/,
    /v3_recovery_2_8k_legacy/,
    /audio[-_]noise[-_]floor[-_]migration/i,
    /noise[_-]floor[_-]migrated/i,
    /field_noise_still_blocked/,
    /platform_algorithm_drift/,
    /expectedExtractorPath/,
    /rollbackPolicy/,
  ];
  const hits = [];
  for (const file of listFiles(roots)) {
    if (!allowedExtensions.has(extname(file))) continue;
    if (allowedQaFixtureGenerators.has(toPosix(file))) continue;
    const text = read(file);
    for (const pattern of forbidden) {
      if (pattern.test(text)) {
        hits.push(`${toPosix(file)} matches ${pattern}`);
      }
    }
  }
  return hits;
}

function listFiles(roots) {
  const files = [];
  for (const root of roots) {
    if (!existsSync(root)) continue;
    walk(root, files);
  }
  return files;
}

function walk(path, files) {
  const stat = statSync(path);
  if (stat.isDirectory()) {
    for (const entry of readdirSync(path)) {
      if (entry === 'target' || entry === 'node_modules' || entry === 'dist' || entry === 'build') {
        continue;
      }
      walk(join(path, entry), files);
    }
    return;
  }
  files.push(path);
}

function toPosix(path) {
  return path.replaceAll('\\', '/');
}
