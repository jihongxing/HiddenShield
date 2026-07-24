import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  qualityBin: readFileSync('watermark-core/src/bin/v3_quality_gate.rs', 'utf8'),
  releaseQualityBin: readFileSync('watermark-core/src/bin/v3_quality_release_gate.rs', 'utf8'),
  noiseFloorBandSelectionBin: readFileSync(
    'watermark-core/src/bin/audio_noise_floor_band_selection_experiment.rs',
    'utf8',
  ),
  audioCore: readFileSync('watermark-core/src/audio.rs', 'utf8'),
  l3ReleaseGate: readFileSync('scripts/verify-l3-video-visual-release-gate.mjs', 'utf8'),
  qualityDoc: readFileSync('docs/感知质量发布门禁设计.md', 'utf8'),
  noSensePlan: readFileSync('docs/盲水印无感测试计划.md', 'utf8'),
  boundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  noiseFloorBandSelectionDesign: readFileSync('docs/音频噪声底频带选择实验设计.md', 'utf8'),
  noiseFloorMigrationDesign: readFileSync(
    'docs/音频噪声底跨端可读频带策略迁移设计.md',
    'utf8',
  ),
};

for (const token of [
  'IMAGE_MIN_PSNR',
  'IMAGE_MIN_SSIM',
  'IMAGE_MAX_ROUNDTRIP_MS',
  'AUDIO_MIN_SNR',
  'AUDIO_MAX_PEAK_DELTA',
  'AUDIO_MAX_LUFS_DELTA',
  'WatermarkService::embed',
  'WatermarkService::extract',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'VMAF requires ffmpeg/libvmaf',
]) {
  assert(sources.qualityBin.includes(token), `V3 quality gate bin must include ${token}`);
}

for (const token of [
  'non_external_dependency_release_smoke',
  'perceptual_full_gate',
  'image_samples',
  'full_image_samples',
  'audio_samples',
  'full_audio_samples',
  'video_audio_track_samples',
  'full_video_audio_track_samples',
  'write_abx_templates',
  'videoFingerprintNotary',
  'videoVisualStaged',
  'not_applicable_no_media_mutation',
  'video_l2_fingerprint_notary',
  'watermark:l3-video-visual-release-gate',
  'futureFullGateMetrics',
  'Audio V3 Debug Metrics',
  'Audio Perceptual Diagnosis',
  'shortTimeRms',
  'modifiedPairRatio',
  'noiseFloorSparseRecovery',
  'extractionConfidence',
  'perceptualDiagnosis',
  'segmentedSnr',
  'bandEnergyShare',
  'dominantNoiseBand',
  'specific_watermark_band_energy_redistribution',
  'WatermarkService::embed',
  'WatermarkService::extract',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'AudioProtectionMode::VideoTrack',
  'watermark:l3-video-visual-release-gate',
  'delegated',
]) {
  assert(sources.releaseQualityBin.includes(token), `V3 release quality gate bin must include ${token}`);
}

for (const token of [
  'HIDDENSHIELD_L3_FULL_RELEASE_POOL',
  'l3_2k_high_bitrate_release_sample_pool_records_thresholds',
  'h264HdPerSampleMin',
  'h264HdGroupMeanMin',
  'hevcHdPerSampleMin',
  'hevcMixGroupMeanMin',
  'release_thresholds_met',
  'checkedFrames',
]) {
  assert(sources.l3ReleaseGate.includes(token), `L3 release gate must include ${token}`);
}

assert(
  !sources.releaseQualityBin.includes('videoVisual remains staged/internal and is not part of this release gate'),
  'V3 release gate must no longer mark L3 videoVisual as skipped/internal',
);

assert(
  sources.packageJson.includes('watermark:quality-gate:fast') &&
    sources.packageJson.includes('watermark:quality-gate:release') &&
    sources.packageJson.includes('watermark:quality-gate:full') &&
    sources.packageJson.includes('watermark:quality-gate:contract') &&
    sources.packageJson.includes('watermark:audio-noise-floor-band-selection-experiment') &&
    sources.packageJson.includes('watermark:l3-video-visual-release-gate'),
  'package.json must expose V3 quality gate scripts, the noise-floor experiment, and the independent L3 release gate',
);

assert(
  sources.qualityDoc.includes('watermark:quality-gate:fast') &&
    sources.qualityDoc.includes('PSNR') &&
    sources.qualityDoc.includes('SSIM') &&
    sources.qualityDoc.includes('SNR') &&
    sources.qualityDoc.includes('LUFS') &&
    sources.qualityDoc.includes('VMAF') &&
    sources.qualityDoc.includes('metrics.debug') &&
    sources.qualityDoc.includes('extractionConfidence') &&
    sources.qualityDoc.includes('noiseFloorSparseRecovery') &&
    sources.qualityDoc.includes('perceptualDiagnosis') &&
    sources.qualityDoc.includes('segmentedSnr') &&
    sources.qualityDoc.includes('bandEnergyShare'),
  'quality gate doc must describe image/audio/video metrics',
);

assert(
  sources.qualityDoc.includes('watermark:quality-gate:release') &&
    sources.qualityDoc.includes('non_external_dependency_release_smoke'),
  'quality gate doc must describe release smoke gate scope',
);

assert(
  sources.noSensePlan.includes('watermark:quality-gate:full') &&
    sources.noSensePlan.includes('ABX') &&
    sources.noSensePlan.includes('图片样本池') &&
    sources.noSensePlan.includes('音频样本池') &&
    sources.noSensePlan.includes('视频样本池') &&
    sources.noSensePlan.includes('videoFingerprintNotary') &&
    sources.noSensePlan.includes('videoVisualStaged') &&
    sources.noSensePlan.includes('not_applicable_no_media_mutation') &&
    sources.noSensePlan.includes('通过 / 阻断规则'),
  'no-sense watermark test plan must define full gate, ABX, sample pools, and pass/block rules',
);

assert(
  sources.boundary.includes('PSNR') &&
    sources.boundary.includes('SSIM') &&
    sources.boundary.includes('VMAF') &&
    sources.boundary.includes('field-noise') &&
    sources.boundary.includes('12.5383 dB') &&
    sources.boundary.includes('13.5115 dB') &&
    sources.boundary.includes('0.282321') &&
    sources.boundary.includes('specific_watermark_band_energy_redistribution'),
  'current capability boundary must mention perceptual quality metric boundary',
);

assert(
  sources.releaseQualityBin.includes('audio_v3_quality_diagnostics') &&
    sources.releaseQualityBin.includes('AudioV3QualityDiagnostics') &&
    sources.releaseQualityBin.includes('debug_markdown') &&
    sources.releaseQualityBin.includes('noiseFloorSparseRecovery'),
  'release/full quality gate must keep audio V3 diagnostics wired into JSON and Markdown reports',
);

for (const token of [
  'audio_noise_floor_band_selection_experiment',
  'inner_watermark_subband_sparse',
  'frame_stability_window_sparse',
  'masked_pair_budget_cap',
  '稳定噪声底 profile',
  'payload 格式不变',
  '版权编号不变',
  '跨端可读不变',
  '提取置信度不退',
  '质量阈值不降',
  'bandEnergyShare',
  'extractionConfidence',
  'audio_noise_floor_band_selection_abx_trials.csv',
  '44 dB',
]) {
  assert(
    sources.noiseFloorBandSelectionDesign.includes(token),
    `noise-floor band selection design must include ${token}`,
  );
}

assert(
  sources.qualityDoc.includes('audio_noise_floor_band_selection_experiment') &&
    sources.qualityDoc.includes('docs/音频噪声底频带选择实验设计.md'),
  'quality gate doc must link the noise-floor band selection experiment design',
);

for (const token of [
  'audio_noise_floor_band_selection_experiment',
  'frame_stability_window_sparse',
  'inner_watermark_subband_sparse',
  'masked_pair_budget_cap',
  'cross_end_readable_frequency_strategy_migration',
  'official_ui_or_mock_path_touched',
  'false',
  'full_gate_snr_threshold_db',
  'AUDIO_FULL_MIN_SNR',
  'MIN_EXTRACTION_CONFIDENCE',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'WatermarkService::embed',
  'WatermarkService::extract',
  'audio_noise_floor_band_selection_abx_trials.csv',
  'planned_not_executed',
  'cap_highest_diff_recovery_pairs_per_second_with_existing_extractor',
]) {
  assert(
    sources.noiseFloorBandSelectionBin.includes(token),
    `noise-floor band selection experiment bin must include ${token}`,
  );
}

for (const token of [
  'codex-noise-band-selection-c3',
  'masked_pair_budget_cap',
  '21.1644 dB',
  '0.999989',
  '跨端可读频带策略迁移设计',
]) {
  assert(
    sources.noiseFloorBandSelectionDesign.includes(token),
    `noise-floor band selection design must record C group result ${token}`,
  );
}

for (const token of [
  '音频噪声底跨端可读频带策略迁移设计',
  '不改变 V3/39 payload 格式',
  '不改变版权编号格式',
  '新旧 extractor 兼容',
  'desktop old write -> mobile new read',
  'mobile new write -> desktop new read',
  'fixtureSchemaVersion',
  'audioStrategyVersion',
  'rollbackPolicy',
  'watermark:audio-noise-floor-migration-contract',
  'candidate_payload_not_found',
  'candidateScanAttempted=true',
  'field-noise >= 44 dB',
  'extractionConfidence >= 0.99',
  '禁止平台层算法漂移',
]) {
  assert(
    sources.noiseFloorMigrationDesign.includes(token),
    `noise-floor migration design must include ${token}`,
  );
}

assert(
  sources.boundary.includes('docs/音频噪声底跨端可读频带策略迁移设计.md') &&
    sources.boundary.includes('跨端可读频带策略迁移当前只是设计 / read-only scan 准备，不是正式音频算法能力') &&
    sources.boundary.includes('RC1 已暂停新频带 writer 实验'),
  'current capability boundary must say noise-floor migration is design/read-only scan only and writer-paused for RC1',
);

assert(
  sources.audioCore.includes('is_noise_floor_sparse_recovery_profile') &&
    sources.audioCore.includes('embed_recovery_bit_sparse_lane_majority') &&
    sources.audioCore.includes('noise_floor_sparse_recovery'),
  'audio core must keep the noise-floor sparse recovery experiment feature-gated by content profile',
);

const readonlyCandidate = sources.audioCore.slice(
  sources.audioCore.indexOf('pub fn extract_watermark_samples_readonly_candidate_with_delta_and_rate'),
  sources.audioCore.indexOf('fn extract_watermark_samples_relative'),
);
assert(
  readonlyCandidate.indexOf('extract_watermark_samples_recovery_readonly_candidate') >= 0 &&
    readonlyCandidate.indexOf('extract_watermark_samples_recovery_readonly_candidate') <
      readonlyCandidate.indexOf('extract_watermark_samples_relative'),
  'audio readonly candidate extraction must try V3 recovery before V2/legacy extraction paths',
);

console.log('V3 quality gate contract passed');
