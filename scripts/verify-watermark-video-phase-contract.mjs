import { readFileSync } from 'node:fs';

const sources = {
  agents: readFileSync('AGENTS.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  plan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  audit: readFileSync('docs/共享水印核心算法审计.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  l3Design: readFileSync('docs/Phase I-6 L3视频画面盲水印同核与云端策略设计.md', 'utf8'),
  l3AlgorithmDesign: readFileSync('docs/Phase I-6 L3真实鲁棒画面盲水印算法设计.md', 'utf8'),
  l3CostModel: readFileSync('docs/Phase I-6 L3平台成本模型.md', 'utf8'),
  l3ReleaseSamplePool: readFileSync('docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md', 'utf8'),
  l3ReleaseGateQa: readFileSync('docs/L3视频画面盲水印release_gate_QA记录.md', 'utf8'),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  desktopCloudClient: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopVideoFingerprint: readFileSync('src-tauri/src/video_fingerprint.rs', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  mobileWorkspace: readFileSync('mobile_app/lib/features/workspace/workspace_page.dart', 'utf8'),
  mobileBridge: readFileSync('mobile_app/lib/bridge/rust_watermark_bridge.dart', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  cloudVideoContract: readFileSync('scripts/verify-cloud-video-contract.mjs', 'utf8'),
  crossEndContract: readFileSync('scripts/verify-watermark-cross-end-contract.mjs', 'utf8'),
  architectureContract: readFileSync('scripts/verify-watermark-architecture-contract.mjs', 'utf8'),
  coreLib: readFileSync('watermark-core/src/lib.rs', 'utf8'),
  coreError: readFileSync('watermark-core/src/error.rs', 'utf8'),
  coreVideoVisual: readFileSync('watermark-core/src/video_visual.rs', 'utf8'),
};

assert(
  sources.packageJson.includes('"watermark:video-phase-contract"') &&
    sources.packageJson.includes('verify-watermark-video-phase-contract.mjs'),
  'package.json must expose watermark:video-phase-contract',
);

assert(
  sources.capabilityBoundary.includes('## 2. 可对用户承诺') &&
    sources.capabilityBoundary.includes('## 3. 只能内部测试') &&
    sources.capabilityBoundary.includes('## 4. 明确不能承诺') &&
    sources.capabilityBoundary.includes('L1 视频音轨水印') &&
    sources.capabilityBoundary.includes('L2 视频指纹存证') &&
    sources.capabilityBoundary.includes('L3 视频画面盲水印 release 候选') &&
    sources.capabilityBoundary.includes('watermark:l3-video-visual-release-gate') &&
    sources.capabilityBoundary.includes('confidence >= threshold') &&
    sources.capabilityBoundary.includes('output-download-authorizations') &&
    sources.capabilityBoundary.includes('桌面 / 移动已接入 Studio / Enterprise 创建上传向导和 succeeded task 领取入口') &&
    sources.capabilityBoundary.includes('复核 `watermarkedMediaHash` / 字节数') &&
    sources.capabilityBoundary.includes('写入版权库 `video_visual_*` 收据字段并进入同步队列') &&
    sources.capabilityBoundary.includes('当前入口仍是 MP4-only release gate') &&
    sources.capabilityBoundary.includes('真实告警平台配置验证、首个试点客户签字验收和更大真实用户 MP4 样本池仍未完成') &&
    sources.capabilityBoundary.includes('不代表 L3 已可正式销售') &&
    sources.capabilityBoundary.includes('L2 不是盲水印') &&
    sources.capabilityBoundary.includes('L1 只处理视频音轨中的音频水印') &&
    sources.capabilityBoundary.includes('Web preview bridge 未接同核正式能力时只能作为 UI 预览') &&
    sources.agents.includes('Current capability boundary statements must follow `docs/当前真实能力边界说明.md`') &&
    sources.plan.includes('当前真实能力边界以 `docs/当前真实能力边界说明.md` 为准') &&
    sources.dualRoadmap.includes('docs/当前真实能力边界说明.md') &&
    sources.commercialRoadmap.includes('当前真实能力边界以 `docs/当前真实能力边界说明.md` 为准'),
  'current real capability boundary document must govern L1/L2/L3 user-committable, internal-test-only, and forbidden claims',
);

assert(
  sources.plan.includes('Phase I-6：视频一致性与互验设计') &&
    sources.plan.includes('L1 视频音轨水印') &&
    sources.plan.includes('L2 视频指纹存证') &&
    sources.plan.includes('L3 端云协同画面盲水印') &&
    sources.audit.includes('视频同核设计') &&
    sources.dualRoadmap.includes('视频一致性纳入 Phase I'),
  'Phase I docs must split video into L1 audio-track watermark, L2 fingerprint notary, and L3 visual watermark',
);

assert(
    sources.desktopScheduler.includes('AudioProtectionMode::VideoTrack') &&
    sources.desktopScheduler.includes('WatermarkService::embed') &&
    sources.desktopScheduler.includes('WatermarkService::extract') &&
    sources.desktopScheduler.includes('l1_video_audio_track_roundtrip_extracts_core_watermark') &&
    sources.desktopScheduler.includes('l3_decoded_video_y_plane_fixture_enters_watermark_core') &&
    sources.desktopScheduler.includes('l3_decoded_video_y_plane_fixture_roundtrips_dct_in_watermark_core') &&
    sources.desktopScheduler.includes('l3_encoded_video_y_plane_fixture_self_checks_after_ffmpeg_roundtrip') &&
    sources.desktopScheduler.includes('l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary') &&
    sources.desktopScheduler.includes('l3_target_platform_transcode_matrix_classifies_dct_survival') &&
    sources.desktopScheduler.includes('l3_main_resolution_transcode_matrix_covers_720p_1080p_2k') &&
    sources.desktopScheduler.includes('l3_main_resolution_platform_profiles_cover_720p_1080p_2k') &&
    sources.desktopScheduler.includes('l3_mainstream_bitrate_floor_matrix_covers_720p_1080p_2k') &&
    sources.desktopScheduler.includes('l3_30s_commercial_sampling_performance_records_cost_breakdown') &&
    sources.desktopScheduler.includes('l3_bilibili_hevc_mainstream_floor_records_cost_breakdown') &&
    sources.desktopScheduler.includes('l3_bilibili_h264_hevc_cost_comparison_records_budget') &&
    sources.desktopScheduler.includes('l3_bilibili_hevc_texture_aware_records_cost_budget') &&
    sources.desktopScheduler.includes('l3_2k_h264_strategy_density_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_2k_h264_sample_count_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_2k_h264_region_quality_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes') &&
    sources.desktopScheduler.includes('l3_platform_timing_budget_records_16frame_seeded_costs') &&
    sources.desktopScheduler.includes('video_frame_plane_from_decoded_luma') &&
    sources.desktopScheduler.includes('embed_video_visual_dct_frames') &&
    sources.desktopScheduler.includes('extract_video_visual_dct_from_frames') &&
    sources.desktopScheduler.includes('WatermarkErrorCode::SelfCheckFailed') &&
    sources.desktopScheduler.includes('gray10le') &&
    sources.crossEndContract.includes('l1_video_audio_track_roundtrip_extracts_core_watermark'),
  'L1 video audio-track watermarking and L3 decoded Y-plane/DCT staged fixtures must use watermark-core and be covered by release gates',
);

assert(
  sources.commercialRoadmap.includes('L1 继续复用本地 Tauri 2 + Rust + FFmpeg + 音频 QIM 盲水印，不进入 quota ledger') &&
    sources.desktopScheduler.includes('"watermark_video"') &&
    sources.desktopScheduler.includes('"video"') &&
    !sources.desktopScheduler.includes('"video_minutes"'),
  'L1 must stay local, reuse watermark-core audio watermarking, and not consume cloud video minutes',
);

assert(
  sources.desktopVideoFingerprint.includes('VideoFingerprintBundleGeneration') &&
    sources.desktopVideoFingerprint.includes('local_block_fingerprints') &&
    sources.desktopVideoFingerprint.includes('crop_window_fingerprints') &&
    sources.desktopCloudClient.includes('video_fingerprint_bundle_to_notary_request') &&
    sources.desktopCloudClient.includes('local_block_fingerprint_root') &&
    sources.desktopCloudClient.includes('crop_window_fingerprint_root') &&
    sources.cloudVideoContract.includes('localBlockFingerprintRoot') &&
    sources.cloudVideoContract.includes('cropWindowFingerprintRoot'),
  'L2 must preserve three-layer irreversible VideoFingerprintBundle and notary mapping',
);

assert(
  sources.desktopCloudClient.includes('contains_original_video') &&
    sources.desktopCloudClient.includes('contains_watermarked_video') &&
    sources.desktopCloudClient.includes('contains_local_paths') &&
    sources.backendStorage.includes('original_video_forbidden') &&
    sources.backendStorage.includes('watermarked_video_forbidden') &&
    sources.backendStorage.includes('local_path_forbidden') &&
    sources.backendStorage.includes("'video_fingerprint_notary', 'usage_ledger', NULL, 0"),
  'L2 notary must reject media/path manifests and stay out of video_minutes quota',
);

assert(
    sources.desktopWorkbench.includes('视频指纹存证') &&
    sources.desktopWorkbench.includes('L1 本地写入') &&
    sources.desktopWorkbench.includes('L3 对象上传入口') &&
    sources.desktopWorkbench.includes('已 succeeded 的 L3 对象任务') &&
    sources.desktopWorkbench.includes('下载并保存版权库') &&
    sources.mobileWorkspace.includes('视频指纹存证与 L3 对象上传入口') &&
    sources.mobileWorkspace.includes('Studio / Enterprise release gate') &&
    sources.mobileWorkspace.includes('下载并保存版权库') &&
    sources.mobileWorkspace.includes('对象上传入口') &&
    sources.mobileWorkspace.includes('L1 视频音轨水印') &&
    sources.mobileBridge.includes('WatermarkAssetKind.video => await rust_api.extractAudioWavForMobile') &&
    sources.mobileBridge.includes('视频音轨可在移动端验证') &&
    sources.mobileBridge.includes('Mobile local video watermarking is disabled.'),
  'current UI must present L3 only as a Studio/Enterprise object-upload release-gate entry and mobile L1 must consume video audio-track verification while keeping mobile L1 video writing disabled until a real remux/encoder path exists',
);

assert(
  sources.audit.includes('L3 在写代码前必须先完成 `watermark-core` 视频画面算法') &&
    sources.audit.includes('策略包、防逆向、密钥边界、客户端自检') &&
    sources.l3Design.includes('状态：设计冻结，未进入实现') &&
    sources.l3Design.includes('L3 的画面盲水印写入、读取、payload 编码、同步标记、鲁棒性参数和恢复逻辑必须位于 `watermark-core`') &&
    sources.l3Design.includes('云端只能提供策略生成、密钥托管、任务调度、权益校验、额度账本、策略签名和自检编排') &&
    sources.commercialRoadmap.includes('L3 端云协同画面盲水印已进入 release candidate 准备') &&
    sources.commercialRoadmap.includes('watermarkedMediaHash') &&
    sources.commercialRoadmap.includes('成功完成后才扣额度') &&
    sources.architectureContract.includes('future video-visual blind-watermark algorithms must live in `watermark-core`'),
  'L3 must remain non-user-committable until release gate, trusted worker, UI, reports, and cross-end validation pass',
);

assert(
  sources.l3Design.includes('VideoVisualPayloadBuildInput') &&
    sources.l3Design.includes('VideoFeatureBundle') &&
    sources.l3Design.includes('VideoVisualStrategy') &&
    sources.l3Design.includes('VideoVisualSelfCheckResult') &&
    sources.l3Design.includes('build_video_visual_payload') &&
    sources.l3Design.includes('derive_video_visual_strategy') &&
    sources.l3Design.includes('embed_video_visual_frame') &&
    sources.l3Design.includes('extract_video_visual_watermark') &&
    sources.l3Design.includes('self_check_video_visual_watermark') &&
    sources.l3Design.includes('WatermarkErrorCode') &&
    sources.l3Design.includes('strategy_invalid') &&
    sources.l3Design.includes('self_check_failed'),
  'L3 design must define watermark-core visual payload, strategy, frame embed/extract, self-check, and error-code contracts',
);

assert(
  sources.coreLib.includes('mod video_visual') &&
    sources.coreLib.includes('build_video_feature_bundle') &&
    sources.coreLib.includes('build_video_visual_payload') &&
    sources.coreLib.includes('derive_video_visual_strategy') &&
    sources.coreLib.includes('derive_video_visual_strategy_with_region_selection') &&
    sources.coreLib.includes('derive_video_visual_complexity_budget') &&
    sources.coreLib.includes('embed_video_visual_dct_frames') &&
    sources.coreLib.includes('embed_video_visual_frame') &&
    sources.coreLib.includes('embed_video_visual_frames') &&
    sources.coreLib.includes('extract_video_visual_dct_from_frames') &&
    sources.coreLib.includes('extract_video_visual_watermark') &&
    sources.coreLib.includes('extract_video_visual_watermark_from_frames') &&
    sources.coreLib.includes('sample_video_visual_frame_indices') &&
    sources.coreLib.includes('self_check_video_visual_dct_frames') &&
    sources.coreLib.includes('self_check_video_visual_frames') &&
    sources.coreLib.includes('self_check_video_visual_watermark') &&
    sources.coreLib.includes('video_frame_plane_from_decoded_luma') &&
    sources.coreLib.includes('DecodedVideoLumaPlane') &&
    sources.coreLib.includes('VideoFeatureBundle') &&
    sources.coreLib.includes('VideoFramePlane') &&
    sources.coreLib.includes('VideoLumaBitDepth') &&
    sources.coreLib.includes('VideoLumaColorRange') &&
    sources.coreLib.includes('VideoVisualComplexityBudget') &&
    sources.coreLib.includes('VideoVisualComplexityTier') &&
    sources.coreLib.includes('VideoVisualRegionSelectionMode') &&
    sources.coreLib.includes('VideoVisualStrategy') &&
    sources.coreLib.includes('VideoVisualSelfCheckResult') &&
    sources.coreLib.includes('VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION'),
  'L3 core spike must export the minimal video frame, feature bundle, strategy, payload, and self-check contracts from watermark-core',
);

assert(
  sources.coreVideoVisual.includes('VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION') &&
    sources.coreVideoVisual.includes('video_strategy_v1') &&
    sources.coreVideoVisual.includes('LumaDctMidBandV1') &&
    sources.coreVideoVisual.includes('pub struct VideoFramePlane') &&
    sources.coreVideoVisual.includes('pub struct VideoVisualPayloadBuildInput') &&
    sources.coreVideoVisual.includes('pub struct VideoFeatureBundle') &&
    sources.coreVideoVisual.includes('pub struct VideoVisualStrategy') &&
    sources.coreVideoVisual.includes('pub enum VideoVisualRegionSelectionMode') &&
    sources.coreVideoVisual.includes('pub struct VideoVisualSelfCheckResult') &&
    sources.coreVideoVisual.includes('pub fn build_video_feature_bundle') &&
    sources.coreVideoVisual.includes('pub fn build_video_visual_payload') &&
    sources.coreVideoVisual.includes('pub fn derive_video_visual_strategy') &&
    sources.coreVideoVisual.includes('pub fn derive_video_visual_strategy_with_region_selection') &&
    sources.coreVideoVisual.includes('pub fn derive_video_visual_complexity_budget') &&
    sources.coreVideoVisual.includes('pub fn sample_video_visual_frame_indices') &&
    sources.coreVideoVisual.includes('pub fn video_frame_plane_from_decoded_luma') &&
    sources.coreVideoVisual.includes('pub fn embed_video_visual_dct_frames') &&
    sources.coreVideoVisual.includes('pub fn extract_video_visual_dct_from_frames') &&
    sources.coreVideoVisual.includes('pub fn self_check_video_visual_dct_frames') &&
    sources.coreVideoVisual.includes('pub fn embed_video_visual_frame') &&
    sources.coreVideoVisual.includes('pub fn embed_video_visual_frames') &&
    sources.coreVideoVisual.includes('pub fn extract_video_visual_watermark') &&
    sources.coreVideoVisual.includes('pub fn extract_video_visual_watermark_from_frames') &&
    sources.coreVideoVisual.includes('pub fn self_check_video_visual_frames') &&
    sources.coreVideoVisual.includes('pub fn self_check_video_visual_watermark') &&
    sources.coreVideoVisual.includes('video_visual_synthetic_frame_embed_extract_roundtrips_payload') &&
    sources.coreVideoVisual.includes('video_visual_multiframe_embed_extract_and_self_check_roundtrip') &&
    sources.coreVideoVisual.includes('video_visual_multiframe_self_check_fails_when_confidence_drops') &&
    sources.coreVideoVisual.includes('video_visual_robustness_tolerates_missing_synthetic_frames') &&
    sources.coreVideoVisual.includes('video_visual_robustness_tolerates_luma_offset_when_lsb_survives') &&
    sources.coreVideoVisual.includes('video_visual_robustness_detects_local_erasure') &&
    sources.coreVideoVisual.includes('video_visual_robustness_tolerates_edge_crop_simulation') &&
    sources.coreVideoVisual.includes('video_visual_robustness_tolerates_quantized_compression_when_lsb_survives') &&
    sources.coreVideoVisual.includes('video_visual_robustness_detects_destructive_compression_simulation') &&
    sources.coreVideoVisual.includes('video_visual_performance_baseline_for_synthetic_multiframe_roundtrip') &&
    sources.coreVideoVisual.includes('fn dct_8x8_forward') &&
    sources.coreVideoVisual.includes('fn dct_8x8_inverse') &&
    sources.coreVideoVisual.includes('fn embed_luma_dct_mid_band_bit') &&
    sources.coreVideoVisual.includes('fn extract_luma_dct_mid_band_bit') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_SYNC_MARKER_V1') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_ECC_REPEAT') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_SYNC_MAX_BIT_ERRORS') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_DCT_EMBED_DELTA') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_DCT_MAX_STREAM_REPEATS') &&
    sources.coreVideoVisual.includes('fn encode_video_visual_bitstream') &&
    sources.coreVideoVisual.includes('fn decode_video_visual_bitstream') &&
    sources.coreVideoVisual.includes('fn locate_video_visual_sync_marker') &&
    sources.coreVideoVisual.includes('fn extract_luma_dct_mid_band_payload_from_streams') &&
    sources.coreVideoVisual.includes('fn embed_luma_dct_mid_band_bitstream') &&
    sources.coreVideoVisual.includes('fn extract_luma_dct_mid_band_bitstream') &&
    sources.coreVideoVisual.includes('fn embed_luma_dct_mid_band_frame') &&
    sources.coreVideoVisual.includes('fn extract_luma_dct_mid_band_frame') &&
    sources.coreVideoVisual.includes('fn embed_luma_dct_mid_band_frames') &&
    sources.coreVideoVisual.includes('fn extract_luma_dct_mid_band_from_frames') &&
    sources.coreVideoVisual.includes('fn self_check_luma_dct_mid_band_frames') &&
    sources.coreVideoVisual.includes('pub enum VideoVisualComplexityTier') &&
    sources.coreVideoVisual.includes('pub struct VideoVisualComplexityBudget') &&
    sources.coreVideoVisual.includes('pub enum VideoLumaBitDepth') &&
    sources.coreVideoVisual.includes('pub enum VideoLumaColorRange') &&
    sources.coreVideoVisual.includes('pub struct DecodedVideoLumaPlane') &&
    sources.coreVideoVisual.includes('video_visual_luma_dct_mid_band_profile_is_exposed') &&
    sources.coreVideoVisual.includes('video_visual_decoded_y_plane_normalizes_limited_10_bit_with_stride') &&
    sources.coreVideoVisual.includes('video_visual_decoded_y_plane_rejects_short_buffer') &&
    sources.coreVideoVisual.includes('video_visual_decoded_y_plane_rejects_synthetic_profile') &&
    sources.coreVideoVisual.includes('video_visual_fixed_y_plane_fixture_roundtrips_dct_payload') &&
    sources.coreVideoVisual.includes('video_visual_dct_complexity_tiers_are_explicit') &&
    sources.coreVideoVisual.includes('video_visual_dct_frame_sampling_is_even_and_deterministic') &&
    sources.coreVideoVisual.includes('video_visual_dct_complexity_rejects_non_dct_profile') &&
    sources.coreVideoVisual.includes('video_visual_dct_8x8_roundtrips_luma_block') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_bit_embed_extracts_both_values') &&
    sources.coreVideoVisual.includes('video_visual_bitstream_sync_marker_and_ecc_recover_payload_bits') &&
    sources.coreVideoVisual.includes('video_visual_bitstream_tolerates_small_sync_marker_damage') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_bitstream_embed_extracts_with_sync_and_ecc') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_frame_roundtrips_payload_with_sync_and_ecc') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_frame_rejects_insufficient_capacity') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_multiframe_self_check_roundtrips') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_multiframe_fuses_corrupted_payload_streams') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_frame_fuses_repeated_payload_streams') &&
    sources.coreVideoVisual.includes('video_visual_dct_public_staged_api_roundtrips') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_multiframe_tolerates_missing_frames') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_multiframe_detects_erased_frames') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_tolerates_uniform_luma_shift') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_tolerates_conservative_quantization') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_reports_v2_nearest_resample_boundary') &&
    sources.coreVideoVisual.includes('video_visual_dct_mid_band_performance_baseline_for_multiframe_roundtrip') &&
    sources.coreVideoVisual.includes('video_feature_bundle_is_deterministic_for_synthetic_frames') &&
    sources.coreVideoVisual.includes('video_visual_strategy_is_deterministic_and_self_checks') &&
    sources.coreVideoVisual.includes('video_visual_region_selection_modes_are_explicit') &&
    sources.coreVideoVisual.includes('VideoVisualTextureHint') &&
    sources.coreVideoVisual.includes('texture_hints') &&
    sources.coreVideoVisual.includes('TextureAware') &&
    sources.coreVideoVisual.includes('texture_aware') &&
    sources.coreVideoVisual.includes('default_video_visual_region_selection') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_LONG_EDGE') &&
    sources.coreVideoVisual.includes('VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_SHORT_EDGE') &&
    sources.coreVideoVisual.includes('collect_video_visual_texture_hints') &&
    sources.coreVideoVisual.includes('luma_texture_score_8x8') &&
    sources.coreVideoVisual.includes('video_visual_texture_hints_are_core_derived_and_deterministic') &&
    sources.coreVideoVisual.includes('video_visual_default_region_selection_uses_transcode_stable_for_main_battlefield') &&
    sources.coreVideoVisual.includes('video_visual_transcode_stable_regions_do_not_drift_with_task_id') &&
    sources.coreVideoVisual.includes('video_visual_spike_returns_l3_error_codes'),
  'L3 core spike must live in watermark-core with synthetic fixture tests before any UI or cloud task is wired',
);

assert(
  sources.coreError.includes('StrategyInvalid') &&
    sources.coreError.includes('FeatureBundleInvalid') &&
    sources.coreError.includes('SelfCheckFailed') &&
    sources.coreError.includes('VisualExtractFailed') &&
    sources.coreError.includes('UnsupportedVideoProfile') &&
    sources.coreError.includes('strategy_invalid') &&
    sources.coreError.includes('feature_bundle_invalid') &&
    sources.coreError.includes('self_check_failed') &&
    sources.coreError.includes('visual_extract_failed') &&
    sources.coreError.includes('unsupported_video_profile'),
  'L3 error codes must be first-class WatermarkErrorCode values in watermark-core',
);

assert(
  sources.l3AlgorithmDesign.includes('状态：设计冻结，未进入实现') &&
    sources.l3AlgorithmDesign.includes('LumaDctMidBandV1') &&
    sources.l3AlgorithmDesign.includes('8x8 DCT') &&
    sources.l3AlgorithmDesign.includes('中频系数') &&
    sources.l3AlgorithmDesign.includes('sync_marker_v1') &&
    sources.l3AlgorithmDesign.includes('ECC') &&
    sources.l3AlgorithmDesign.includes('O(sampled_frames * candidate_blocks_per_frame * selected_coeff_pairs)') &&
    sources.l3AlgorithmDesign.includes('VideoVisualComplexityTier') &&
    sources.l3AlgorithmDesign.includes('VideoVisualComplexityBudget') &&
    sources.l3AlgorithmDesign.includes('derive_video_visual_complexity_budget') &&
    sources.l3AlgorithmDesign.includes('sample_video_visual_frame_indices') &&
    sources.l3AlgorithmDesign.includes('DecodedVideoLumaPlane') &&
    sources.l3AlgorithmDesign.includes('VideoLumaBitDepth') &&
    sources.l3AlgorithmDesign.includes('VideoLumaColorRange') &&
    sources.l3AlgorithmDesign.includes('video_frame_plane_from_decoded_luma') &&
    sources.l3AlgorithmDesign.includes('Small | 4 | 512 / frame | 3 | 6,144 | < 1.5s') &&
    sources.l3AlgorithmDesign.includes('Standard | 8 | 768 / frame | 3 | 18,432 | < 3s') &&
    sources.l3AlgorithmDesign.includes('High | 12 | 1,024 / frame | 3 | 36,864 | < 6s') &&
    sources.l3AlgorithmDesign.includes('小视频') &&
    sources.l3AlgorithmDesign.includes('标准视频') &&
    sources.l3AlgorithmDesign.includes('高阶视频') &&
    sources.l3AlgorithmDesign.includes('feature_bundle_invalid') &&
    sources.l3AlgorithmDesign.includes('strategy_invalid') &&
    sources.l3AlgorithmDesign.includes('visual_extract_failed') &&
    sources.l3AlgorithmDesign.includes('self_check_failed') &&
    sources.l3AlgorithmDesign.includes('从 Synthetic Spike 到真实算法的替换边界') &&
    sources.l3AlgorithmDesign.includes('禁止') &&
    sources.l3AlgorithmDesign.includes('在桌面端 Tauri、移动端 Flutter/Rust bridge、后端 handler 或脚本中实现 DCT / QIM / bitstream / ECC') &&
    sources.l3AlgorithmDesign.includes('主流码率地板矩阵') &&
    sources.l3AlgorithmDesign.includes('低于主流地板的码率只记录风险边界') &&
    sources.l3AlgorithmDesign.includes('中心裁切后补边再 CRF 23 二压也必须通过自检') &&
    sources.desktopScheduler.includes('main_720p_h264_crf28') &&
    sources.desktopScheduler.includes('main_1080p_h264_crf28') &&
    sources.desktopScheduler.includes('main_2k_h264_crf28') &&
    sources.desktopScheduler.includes('main_720p_center_crop_pad_crf23') &&
    sources.desktopScheduler.includes('main_1080p_center_crop_pad_crf23') &&
    sources.desktopScheduler.includes('main_2k_center_crop_pad_crf23') &&
    sources.desktopScheduler.includes('douyin_720p_vertical_h264_high_crf18') &&
    sources.desktopScheduler.includes('douyin_1080p_vertical_h264_high_crf18') &&
    sources.desktopScheduler.includes('bilibili_2k_landscape_h264_high_crf18') &&
    sources.desktopScheduler.includes('xiaohongshu_1080p_vertical_h264_high_crf17') &&
    sources.desktopScheduler.includes('mainstream_floor_720p_h264_2500k') &&
    sources.desktopScheduler.includes('mainstream_floor_1080p_h264_4500k') &&
    sources.desktopScheduler.includes('mainstream_floor_2k_h264_8000k') &&
    sources.desktopScheduler.includes('commercial_30s_720p_12frames_h264_2500k') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_12frames_hevc_4000k') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_hevc_6500k') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_12frames_h264_4500k_cost') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_12frames_hevc_4000k_cost') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_h264_8000k_cost') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_hevc_6500k_cost') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_hevc_4000k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_hevc_6500k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('l3_default_transcode_stable_h264_hevc_regression_records_cost_budget') &&
    sources.desktopScheduler.includes('default_30s_720p_h264_2500k_12frames_core_default') &&
    sources.desktopScheduler.includes('default_30s_1080p_h264_6000k_16frames_core_default') &&
    sources.desktopScheduler.includes('default_30s_2k_h264_8000k_16frames_core_default') &&
    sources.desktopScheduler.includes('default_30s_1080p_hevc_4000k_16frames_core_default') &&
    sources.desktopScheduler.includes('default_30s_2k_hevc_6500k_16frames_core_default') &&
    sources.desktopScheduler.includes('l3_default_strategy_texture_diversity_records_cost_budget') &&
    sources.desktopScheduler.includes('default_30s_1080p_h264_6000k_low_texture_grid') &&
    sources.desktopScheduler.includes('default_30s_1080p_h264_6000k_high_texture') &&
    sources.desktopScheduler.includes('default_30s_1080p_vertical_h264_6000k_high_texture') &&
    sources.desktopScheduler.includes('default_30s_2k_h264_8000k_low_texture_grid') &&
    sources.desktopScheduler.includes('l3_default_strategy_real_content_risk_boundary_records_outcomes') &&
    sources.desktopScheduler.includes('risk_30s_1080p_vertical_h264_4500k_high_detail') &&
    sources.desktopScheduler.includes('risk_30s_1080p_h264_6000k_extreme_high_frequency') &&
    sources.desktopScheduler.includes('risk_30s_1080p_h264_6000k_temporal_noise') &&
    sources.desktopScheduler.includes('l3_platform_second_pass_transcode_risk_records_outcomes') &&
    sources.desktopScheduler.includes('second_pass_30s_1080p_vertical_high_detail_6000k_to_4500k') &&
    sources.desktopScheduler.includes('second_pass_30s_2k_landscape_8000k_to_6500k') &&
    sources.desktopScheduler.includes('l3_platform_second_pass_stability_diagnostics_records_budget_curve') &&
    sources.desktopScheduler.includes('second_pass_diag_1080p_vertical_6000k_to_4500k_20frames_96regions') &&
    sources.desktopScheduler.includes('second_pass_diag_1080p_vertical_6000k_to_4500k_16frames_128regions') &&
    sources.desktopScheduler.includes('second_pass_diag_1080p_vertical_6000k_to_4500k_16frames_96regions_transcode_stable') &&
    sources.desktopScheduler.includes('second_pass_diag_2k_8000k_to_6500k_20frames_96regions') &&
    sources.desktopScheduler.includes('l3_transcode_stable_second_pass_platform_matrix_records_generalization') &&
    sources.desktopScheduler.includes('core_default_30s_720p_h264_4000k_to_3000k_16frames') &&
    sources.desktopScheduler.includes('transcode_stable_30s_1080p_h264_6000k_to_4500k_16frames') &&
    sources.desktopScheduler.includes('transcode_stable_30s_2k_h264_8000k_to_6500k_16frames') &&
    sources.desktopScheduler.includes('transcode_stable_30s_1080p_hevc_4000k_to_3200k_16frames') &&
    sources.desktopScheduler.includes('transcode_stable_30s_2k_hevc_6500k_to_5200k_16frames') &&
    sources.desktopScheduler.includes('l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight') &&
    sources.desktopScheduler.includes('default_30s_720p_h264_4000k_to_3000k_16frames') &&
    sources.desktopScheduler.includes('default_30s_1080p_h264_6000k_to_4500k_16frames') &&
    sources.desktopScheduler.includes('default_30s_2k_h264_8000k_to_6500k_16frames') &&
    sources.desktopScheduler.includes('default_30s_1080p_hevc_4000k_to_3200k_16frames') &&
    sources.desktopScheduler.includes('default_30s_2k_hevc_6500k_to_5200k_16frames') &&
    sources.desktopScheduler.includes('core default TranscodeStable second-pass matrix') &&
    sources.desktopScheduler.includes('core default second-pass risk boundary') &&
    sources.desktopScheduler.includes('l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes') &&
    sources.desktopScheduler.includes('real_content_default_30s_1080p_landscape_h264_6000k_to_4500k_16frames') &&
    sources.desktopScheduler.includes('real_content_default_30s_1080p_vertical_h264_6000k_to_4500k_16frames') &&
    sources.desktopScheduler.includes('real_content_default_30s_2k_regular_h264_8000k_to_6500k_16frames') &&
    sources.desktopScheduler.includes('real_content_default_30s_2k_high_detail_h264_8000k_to_6500k_16frames') &&
    sources.desktopScheduler.includes('budget_2k_high_detail_h264_8000k_to_6500k_20frames_96regions') &&
    sources.desktopScheduler.includes('budget_2k_high_detail_h264_8000k_to_6500k_16frames_128regions') &&
    sources.desktopScheduler.includes('budget_2k_high_detail_h264_10000k_to_8000k_16frames_96regions') &&
    sources.desktopScheduler.includes('2K high-detail H.264 budget risk boundary') &&
    sources.desktopScheduler.includes('l3_2k_high_bitrate_content_candidate_matrix_records_outcomes') &&
    sources.desktopScheduler.includes('candidate_2k_high_detail_h264_10000k_to_8000k_16frames_96regions') &&
    sources.desktopScheduler.includes('candidate_2k_low_texture_h264_10000k_to_8000k_16frames_96regions') &&
    sources.desktopScheduler.includes('candidate_2k_motion_texture_h264_10000k_to_8000k_16frames_96regions') &&
    sources.desktopScheduler.includes('candidate_2k_high_detail_hevc_8000k_to_6500k_16frames_96regions') &&
    sources.desktopScheduler.includes('real-content second-pass risk boundary') &&
    sources.desktopScheduler.includes('recorded second-pass risk boundary') &&
    sources.desktopScheduler.includes('region_selection=core_default') &&
    sources.desktopScheduler.includes('checked_frames') &&
    sources.coreVideoVisual.includes('TranscodeStable') &&
    sources.coreVideoVisual.includes('transcode_stable') &&
    sources.coreVideoVisual.includes('derive_transcode_stable_regions') &&
    sources.desktopScheduler.includes('ffmpeg_first_pass_ms') &&
    sources.desktopScheduler.includes('ffmpeg_second_pass_ms') &&
    sources.desktopScheduler.includes('"self_check_failed"') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_h264_8000k_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_h264_8000k_regions_128') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_12frames_h264_8000k_regions_160') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_12frames_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_16frames_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_20frames_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_16frames_seeded_random_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_16frames_center_safe_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_16frames_distributed_regions_96') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_h264_8000k_16frames_texture_aware_regions_96') &&
    sources.desktopScheduler.includes('douyin_30s_1080p_vertical_h264_4500k_16frames_seeded') &&
    sources.desktopScheduler.includes('xiaohongshu_30s_1080p_vertical_h264_6000k_16frames_seeded') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_landscape_h264_6000k_16frames_seeded') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_landscape_h264_8000k_16frames_seeded') &&
    sources.desktopScheduler.includes('douyin_30s_1080p_vertical_h264_4500k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('xiaohongshu_30s_1080p_vertical_h264_6000k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('bilibili_30s_1080p_landscape_h264_6000k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('bilibili_30s_2k_landscape_h264_8000k_16frames_texture_aware') &&
    sources.desktopScheduler.includes('ffmpeg_encoder_available(&paths.ffmpeg, "libx265")') &&
    sources.desktopScheduler.includes('max_regions: 96') &&
    sources.desktopScheduler.includes('max_regions: case.max_regions') &&
    sources.desktopScheduler.includes('sampled_frames: 16') &&
    sources.desktopScheduler.includes('sampled_frames: 20') &&
    sources.l3AlgorithmDesign.includes('30 秒商业采样性能矩阵') &&
    sources.l3AlgorithmDesign.includes('B站 HEVC 主流码率地板矩阵') &&
    sources.l3AlgorithmDesign.includes('B站 H.264 / HEVC 成本对照矩阵') &&
    sources.l3AlgorithmDesign.includes('2K H.264 策略密度预算矩阵') &&
    sources.l3AlgorithmDesign.includes('2K H.264 抽帧数量预算矩阵') &&
    sources.l3AlgorithmDesign.includes('2K H.264 区域质量预算矩阵') &&
    sources.l3AlgorithmDesign.includes('平台矩阵耗时预算') &&
    sources.l3AlgorithmDesign.includes('6Mbps 成为 1080p 平台候选预算') &&
    sources.l3AlgorithmDesign.includes('VideoVisualTextureHint') &&
    sources.l3AlgorithmDesign.includes('TextureAware') &&
    sources.l3AlgorithmDesign.includes('confidence 1.000') &&
    sources.l3AlgorithmDesign.includes('总耗时约 55.6s') &&
    sources.l3AlgorithmDesign.includes('TextureAware 平台矩阵耗时预算') &&
    sources.l3AlgorithmDesign.includes('B站 HEVC TextureAware 对照矩阵') &&
    sources.l3AlgorithmDesign.includes('默认策略切换回归矩阵') &&
    sources.l3AlgorithmDesign.includes('默认策略真实素材多样性回归矩阵') &&
    sources.l3AlgorithmDesign.includes('真实素材风险边界矩阵') &&
    sources.l3AlgorithmDesign.includes('平台二压风险矩阵') &&
    sources.l3AlgorithmDesign.includes('平台二压稳定性诊断矩阵') &&
    sources.l3AlgorithmDesign.includes('TranscodeStable') &&
    sources.l3AlgorithmDesign.includes('TranscodeStable 平台泛化矩阵') &&
    sources.l3AlgorithmDesign.includes('默认 TranscodeStable 平台二压成本权重复核矩阵') &&
    sources.l3AlgorithmDesign.includes('默认 TranscodeStable 真实内容二压矩阵') &&
    sources.l3AlgorithmDesign.includes('2K 高细节 H.264 二压预算策略矩阵') &&
    sources.l3AlgorithmDesign.includes('总耗时约 33.0s、26.5s、33.9s、55.8s') &&
    sources.l3AlgorithmDesign.includes('总耗时约 35.1s、57.7s') &&
    sources.l3AlgorithmDesign.includes('总耗时约 17.7s') &&
    sources.l3AlgorithmDesign.includes('总耗时约 56.1s、36.6s、58.4s') &&
    sources.l3AlgorithmDesign.includes('总耗时约 50.2s、39.7s、40.3s、79.2s') &&
    sources.l3AlgorithmDesign.includes('confidence 1.000，总耗时约 35.1s') &&
    sources.l3AlgorithmDesign.includes('低码率竖屏高细节通过但 confidence 0.875') &&
    sources.l3AlgorithmDesign.includes('极端程序化高频纹理和逐帧随机噪声均稳定归因为 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('1080p 竖屏高细节 6Mbps 再二压到 4.5Mbps 稳定归因为 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('2K 8Mbps 再二压到 6.5Mbps 当前压线通过，confidence 0.750') &&
    sources.l3AlgorithmDesign.includes('2K 20 帧 / 96 区域二压 confidence 提升到 0.950') &&
    sources.l3AlgorithmDesign.includes('1080p 竖屏高细节 TranscodeStable 16 帧 / 96 区域二压通过，confidence 0.812') &&
    sources.l3AlgorithmDesign.includes('720p H.264 4Mbps -> 3Mbps 真实二压仍为 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压全部通过') &&
    sources.l3AlgorithmDesign.includes('1080p H.264、2K H.264、1080p HEVC、2K HEVC confidence 分别为 1.000、0.875、1.000、1.000') &&
    sources.l3AlgorithmDesign.includes('2K 高细节在 8Mbps -> 6.5Mbps 二压下稳定返回 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('20 帧 / 96 区域仍 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('16 帧 / 128 区域仍 `self_check_failed`') &&
    sources.l3AlgorithmDesign.includes('提高到 10Mbps -> 8Mbps 后通过但 confidence 0.875') &&
    sources.l3AlgorithmDesign.includes('加帧 / 加区域无效，高码率候选可过但未达 SLA') &&
    sources.l3AlgorithmDesign.includes('2K 高码率内容候选矩阵') &&
    sources.l3AlgorithmDesign.includes('低纹理和运动纹理 H.264 在 10Mbps -> 8Mbps 下均达到 confidence 1.000') &&
    sources.l3AlgorithmDesign.includes('HEVC 高细节 8Mbps -> 6.5Mbps 达到 confidence 1.000') &&
    sources.l3AlgorithmDesign.includes('12 个采样帧 / 96 个策略区域') &&
    sources.l3AlgorithmDesign.includes('首版平台 profile 矩阵'),
  'L3 robust visual algorithm design must freeze DCT mid-band route, sync/ECC, complexity budgets, error attribution, and synthetic-spike replacement boundary',
);

assert(
  sources.l3Design.includes('video_strategy_v1') &&
    sources.l3Design.includes('strategy_digest') &&
    sources.l3Design.includes('server_signature') &&
    sources.l3Design.includes('策略包必须是一次性的') &&
    sources.l3Design.includes('服务端主密钥') &&
    sources.l3Design.includes('任务派生密钥') &&
    sources.l3Design.includes('客户端只能执行策略包') &&
    sources.l3Design.includes('完整策略包'),
  'L3 design must freeze one-time strategy packet, signature, digest, key custody, and anti-reverse-engineering boundaries',
);

assert(
  sources.l3Design.includes('必须在成品视频上执行') &&
    sources.l3Design.includes('checked_frames') &&
    sources.l3Design.includes('confidence') &&
    sources.l3Design.includes('self_check_threshold') &&
    sources.l3Design.includes('自检失败不能扣费') &&
    sources.l3Design.includes('策略包生成成功') &&
    sources.l3Design.includes('客户端本地渲染成功') &&
    sources.l3Design.includes('成品视频完成后自检通过') &&
    sources.l3Design.includes('quota_units = ceil(duration_ms / 60000)'),
  'L3 design must freeze client self-check, success gates, and video-minute charging semantics',
);

assert(
  sources.l3CostModel.includes('状态：设计冻结，未进入实现') &&
    sources.l3CostModel.includes('l3_platform_timing_budget_records_16frame_seeded_costs') &&
    sources.l3CostModel.includes('30 秒') &&
    sources.l3CostModel.includes('16') &&
    sources.l3CostModel.includes('96') &&
    sources.l3CostModel.includes('SeededRandom') &&
    sources.l3CostModel.includes('H.264 4.5Mbps / CRF23') &&
    sources.l3CostModel.includes('H.264 6Mbps / CRF20') &&
    sources.l3CostModel.includes('H.264 8Mbps / CRF23') &&
    sources.l3CostModel.includes('处理倍率') &&
    sources.l3CostModel.includes('l3_cost_units') &&
    sources.l3CostModel.includes('platform_weight') &&
    sources.l3CostModel.includes('strategy_weight') &&
    sources.l3CostModel.includes('16 帧 / 96 区域 / TextureAware') &&
    sources.l3CostModel.includes('B站 HEVC TextureAware 复测结果') &&
    sources.l3CostModel.includes('默认策略切换回归结果') &&
    sources.l3CostModel.includes('默认策略真实素材多样性回归结果') &&
    sources.l3CostModel.includes('真实素材风险边界结果') &&
    sources.l3CostModel.includes('平台二压风险矩阵结果') &&
    sources.l3CostModel.includes('平台二压稳定性诊断结果') &&
    sources.l3CostModel.includes('TranscodeStable') &&
    sources.l3CostModel.includes('TranscodeStable 平台泛化结果') &&
    sources.l3CostModel.includes('默认 TranscodeStable 平台二压成本权重复核结果') &&
    sources.l3CostModel.includes('默认 TranscodeStable 真实内容二压结果') &&
    sources.l3CostModel.includes('2K 高细节 H.264 二压预算策略结果') &&
    sources.l3CostModel.includes('2K 高码率内容候选结果') &&
    sources.l3CostModel.includes('720p H.264 二压风险边界') &&
    sources.l3CostModel.includes('HEVC 4Mbps / CRF20') &&
    sources.l3CostModel.includes('HEVC 6.5Mbps / CRF20') &&
    sources.l3CostModel.includes('HEVC 5.2Mbps / CRF24') &&
    sources.l3CostModel.includes('strategy_weight` 暂定为 1.00') &&
    sources.l3CostModel.includes('只能用于内部容量规划、定价测算和套餐边界设计') &&
    sources.l3CostModel.includes('不能在当前阶段进入 UI、后端账本或用户报告') &&
    sources.l3CostModel.includes('只有成功完成后才扣额度') &&
    sources.l3CostModel.includes('self_check_failed') &&
    sources.l3ReleaseGateQa.includes('watermark:l3-video-visual-release-gate') &&
    sources.l3ReleaseGateQa.includes('watermarkedMediaHash') &&
    sources.l3ReleaseGateQa.includes('完整 24 个 2K 样本池已跑完并过线') &&
    sources.l3ReleaseGateQa.includes('1782888912515') &&
    sources.l3ReleaseGateQa.includes('H.264-HD：6/6 通过') &&
    sources.l3CostModel.includes('真实素材风险边界矩阵') &&
    sources.l3CostModel.includes('2K 二压当前只是阈值线上的通过证据') &&
    sources.l3CostModel.includes('2K 20 帧诊断可把二压 confidence 提升到 0.950') &&
    sources.l3CostModel.includes('1080p TranscodeStable 可在不加帧、不加区域的情况下恢复到 passed:0.812') &&
    sources.l3CostModel.includes('2K H.264 confidence 0.875') &&
    sources.l3CostModel.includes('2K HEVC confidence 1.000') &&
    sources.l3CostModel.includes('2K 高细节 H.264 风险边界') &&
    sources.l3CostModel.includes('2K 高细节不能进入当前默认商业承诺') &&
    sources.l3CostModel.includes('提高码率：16 帧 / 96 区域') &&
    sources.l3CostModel.includes('H.264 10Mbps / CRF21') &&
    sources.l3CostModel.includes('H.264 8Mbps / CRF23') &&
    sources.l3CostModel.includes('10Mbps -> 8Mbps') &&
    sources.l3CostModel.includes('confidence 仍只有 0.875') &&
    sources.l3CostModel.includes('H.264 低纹理和运动纹理已到 confidence 1.000') &&
    sources.l3CostModel.includes('HEVC 高细节在 8Mbps -> 6.5Mbps 下达到 confidence 1.000') &&
    sources.l3CostModel.includes('720p 真实二压仍是当前失败边界') &&
    sources.l3CostModel.includes('1080p / 2K 默认 TranscodeStable') &&
    sources.l3CostModel.includes('16 帧 / 96 区域 / TranscodeStable') &&
    sources.l3CostModel.includes('当前不需要为 TranscodeStable 单独提高 `strategy_weight`') &&
    sources.l3AlgorithmDesign.includes('L3 30 秒平台成本模型') &&
    sources.l3AlgorithmDesign.includes('docs/Phase I-6 L3平台成本模型.md') &&
    sources.plan.includes('L3 已有 staged API、平台矩阵、release 样本池门禁和边界文档全部保留') &&
    sources.capabilityBoundary.includes('当前执行主线已重新打开 L3 正式化准备') &&
    sources.capabilityBoundary.includes('npm run watermark:l3-video-visual-release-gate') &&
    sources.plan.includes('TextureAware 核心候选') &&
    sources.plan.includes('TextureAware 完整平台耗时矩阵') &&
    sources.plan.includes('HEVC TextureAware 对照矩阵') &&
    sources.plan.includes('默认 TranscodeStable 策略切换回归矩阵') &&
    sources.plan.includes('默认策略真实素材多样性回归矩阵') &&
    sources.plan.includes('真实素材风险边界矩阵') &&
    sources.plan.includes('平台二压风险矩阵') &&
    sources.plan.includes('平台二压稳定性诊断矩阵') &&
    sources.plan.includes('TranscodeStable 平台泛化矩阵') &&
    sources.plan.includes('默认 TranscodeStable 平台二压成本权重复核') &&
    sources.plan.includes('默认 TranscodeStable 真实内容二压矩阵') &&
    sources.plan.includes('720p 真实二压失败边界') &&
    sources.plan.includes('2K HEVC 二压') &&
    sources.plan.includes('2K 高细节 H.264 二压预算策略矩阵') &&
    sources.plan.includes('2K 高码率内容候选矩阵') &&
    sources.audit.includes('TextureAware 区域质量候选') &&
    sources.audit.includes('TextureAware 完整平台耗时矩阵') &&
    sources.audit.includes('HEVC TextureAware 对照矩阵') &&
    sources.audit.includes('默认 TranscodeStable 策略切换回归矩阵') &&
    sources.audit.includes('默认策略真实素材多样性回归矩阵') &&
    sources.audit.includes('真实素材风险边界矩阵') &&
    sources.audit.includes('平台二压风险矩阵') &&
    sources.audit.includes('平台二压稳定性诊断矩阵') &&
    sources.audit.includes('TranscodeStable 平台泛化矩阵') &&
    sources.audit.includes('默认 TranscodeStable 平台二压成本权重复核') &&
    sources.audit.includes('默认 TranscodeStable 真实内容二压矩阵') &&
    sources.audit.includes('720p 真实二压失败边界') &&
    sources.audit.includes('2K HEVC 二压') &&
    sources.audit.includes('2K 高细节 H.264 二压预算策略') &&
    sources.audit.includes('2K 高码率内容候选矩阵') &&
    sources.dualRoadmap.includes('TextureAware 核心候选') &&
    sources.dualRoadmap.includes('TextureAware 完整平台耗时矩阵') &&
    sources.dualRoadmap.includes('HEVC TextureAware 对照矩阵') &&
    sources.dualRoadmap.includes('默认 TranscodeStable 策略切换回归矩阵') &&
    sources.dualRoadmap.includes('默认策略真实素材多样性回归矩阵') &&
    sources.dualRoadmap.includes('真实素材风险边界矩阵') &&
    sources.dualRoadmap.includes('平台二压风险矩阵') &&
    sources.dualRoadmap.includes('平台二压稳定性诊断矩阵') &&
    sources.dualRoadmap.includes('TranscodeStable 平台泛化矩阵') &&
    sources.dualRoadmap.includes('默认 TranscodeStable 平台二压成本权重复核') &&
    sources.dualRoadmap.includes('默认 TranscodeStable 真实内容二压矩阵') &&
    sources.dualRoadmap.includes('720p 真实二压失败边界') &&
    sources.dualRoadmap.includes('2K HEVC 二压') &&
    sources.dualRoadmap.includes('2K 高细节 H.264 二压预算策略') &&
    sources.dualRoadmap.includes('2K 高码率内容候选矩阵') &&
    sources.commercialRoadmap.includes('TextureAware 完整平台矩阵') &&
    sources.commercialRoadmap.includes('TextureAware HEVC 对照矩阵') &&
    sources.commercialRoadmap.includes('默认 TranscodeStable 策略切换回归矩阵') &&
    sources.commercialRoadmap.includes('默认策略真实素材多样性回归矩阵') &&
    sources.commercialRoadmap.includes('真实素材风险边界矩阵') &&
    sources.commercialRoadmap.includes('平台二压风险矩阵') &&
    sources.commercialRoadmap.includes('平台二压稳定性诊断矩阵') &&
    sources.commercialRoadmap.includes('TranscodeStable 平台泛化矩阵') &&
    sources.commercialRoadmap.includes('默认 TranscodeStable 平台二压成本权重复核') &&
    sources.commercialRoadmap.includes('默认 TranscodeStable 真实内容二压矩阵') &&
    sources.commercialRoadmap.includes('720p 真实二压失败边界') &&
    sources.commercialRoadmap.includes('2K HEVC 二压') &&
    sources.commercialRoadmap.includes('2K 高细节 H.264 二压预算策略') &&
    sources.commercialRoadmap.includes('2K 高码率内容候选矩阵') &&
    sources.commercialRoadmap.includes('L3 30 秒平台成本模型'),
  'L3 cost model must stay as a staged internal capacity model and must not open UI, backend ledger, or video_minutes charging',
);

assert(
  sources.l3ReleaseSamplePool.includes('状态：测试门禁已落地，未进入商业实现') &&
    sources.l3ReleaseSamplePool.includes('首版 release 样本池至少 24 个样本') &&
    sources.l3ReleaseSamplePool.includes('H.264 10Mbps -> 8Mbps') &&
    sources.l3ReleaseSamplePool.includes('HEVC 8Mbps -> 6.5Mbps') &&
    sources.l3ReleaseSamplePool.includes('H264-HD') &&
    sources.l3ReleaseSamplePool.includes('H264-LT') &&
    sources.l3ReleaseSamplePool.includes('H264-MT') &&
    sources.l3ReleaseSamplePool.includes('H264-RISK') &&
    sources.l3ReleaseSamplePool.includes('HEVC-HD') &&
    sources.l3ReleaseSamplePool.includes('HEVC-MIX') &&
    sources.l3ReleaseSamplePool.includes('H.264 非风险样本最低 confidence 门槛为 0.950') &&
    sources.l3ReleaseSamplePool.includes('`H264-HD` 分组均值 confidence >= 0.970') &&
    sources.l3ReleaseSamplePool.includes('`H264-LT` 和 `H264-MT` 分组均值 confidence >= 0.980') &&
    sources.l3ReleaseSamplePool.includes('HEVC 进入 release-blocking 门禁') &&
    sources.l3ReleaseSamplePool.includes('confidence >= 0.970') &&
    sources.l3ReleaseSamplePool.includes('分组均值 confidence >= 0.990') &&
    sources.l3ReleaseSamplePool.includes('payload_mismatch') &&
    sources.l3ReleaseSamplePool.includes('confidence_below_threshold') &&
    sources.l3ReleaseSamplePool.includes('risk_boundary_expected') &&
    sources.l3ReleaseSamplePool.includes('禁止商业包装') &&
    sources.l3ReleaseSamplePool.includes('l3_2k_high_bitrate_release_sample_pool_records_thresholds') &&
    sources.l3ReleaseSamplePool.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL=1') &&
    sources.l3ReleaseSamplePool.includes('默认本地 / CI 门禁运行每个分组 1 个代表样本') &&
    sources.l3ReleaseSamplePool.includes('完整 24 样本池属于长跑 release evidence gate') &&
    sources.l3ReleaseSamplePool.includes('confidence 0.875') &&
    sources.l3AlgorithmDesign.includes('docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md') &&
    sources.l3AlgorithmDesign.includes('l3_2k_high_bitrate_release_sample_pool_records_thresholds') &&
    sources.l3AlgorithmDesign.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL=1') &&
    sources.l3CostModel.includes('docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md') &&
    sources.l3CostModel.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL=1') &&
    sources.capabilityBoundary.includes('首版 release 样本池至少 24 个 2K 样本') &&
    sources.capabilityBoundary.includes('l3_2k_high_bitrate_release_sample_pool_records_thresholds') &&
    sources.capabilityBoundary.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL=1') &&
    sources.plan.includes('2K 高码率 release 样本池与阈值策略冻结') &&
    sources.plan.includes('l3_2k_high_bitrate_release_sample_pool_records_thresholds') &&
    sources.audit.includes('L3 2K 高码率 release 样本池与阈值策略冻结') &&
    sources.audit.includes('L3 2K 高码率 release 样本池门禁落地') &&
    sources.dualRoadmap.includes('2K 高码率 release 样本池与阈值策略冻结') &&
    sources.dualRoadmap.includes('2K 高码率 release 样本池门禁') &&
    sources.commercialRoadmap.includes('2K 高码率 release 样本池与阈值策略冻结') &&
    sources.commercialRoadmap.includes('2K 高码率 release 样本池门禁') &&
    sources.commercialRoadmap.includes('该策略仍不是可销售 SLA'),
  'L3 2K high-bitrate release sample pool must freeze sample counts, H.264/HEVC thresholds, failure attribution, and commercial packaging gates',
);

assert(
  sources.desktopScheduler.includes('l3_2k_high_bitrate_release_sample_pool_records_thresholds') &&
    sources.desktopScheduler.includes('l3_2k_high_bitrate_release_sample_pool_cases') &&
    sources.desktopScheduler.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL') &&
    sources.desktopScheduler.includes('first_sample_per_group') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "H264-HD", 6)') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "H264-LT", 4)') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "H264-MT", 4)') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "H264-RISK", 2)') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "HEVC-HD", 4)') &&
    sources.desktopScheduler.includes('assert_sample_definition_count(&samples, "HEVC-MIX", 4)') &&
    sources.desktopScheduler.includes('confidence_below_threshold') &&
    sources.desktopScheduler.includes('risk_boundary_expected') &&
    sources.desktopScheduler.includes('encoder_unavailable') &&
    sources.desktopScheduler.includes('release_blocked_h264_hd_confidence_below_threshold'),
  'Tauri L3 release sample pool gate must define the full 24-sample pool, default smoke behavior, full-pool env gate, and release-blocking attribution',
);

assert(
  sources.l3Design.includes('禁止同步') &&
    sources.l3Design.includes('原始视频') &&
    sources.l3Design.includes('加水印视频') &&
    sources.l3Design.includes('本地文件路径') &&
    sources.l3Design.includes('服务端密钥或任务派生密钥') &&
    sources.l3Design.includes('不把 L2 指纹匹配结果当作 L3 水印命中') &&
    sources.l3Design.includes('不在本阶段实现视频画面盲水印算法') &&
    sources.l3Design.includes('不在本阶段开放云端视频任务'),
  'L3 design must freeze privacy boundaries and current non-goals',
);

assert(
  sources.commercialRoadmap.includes('不能把 L2 指纹存证包装成 L3 视频画面盲水印') &&
    sources.commercialRoadmap.includes('L2 相似性证据') &&
    sources.commercialRoadmap.includes('水印命中互验') &&
    sources.plan.includes('不能把 L2 指纹存证包装成 L3 视频画面盲水印') &&
    sources.plan.includes('L2 相似性证据') &&
    sources.plan.includes('水印命中互验') &&
    sources.plan.includes('L2：三层摘要要求不回退') &&
    sources.plan.includes('L3：不出现桌面端、移动端、后端或云任务各自实现视频画面水印算法'),
  'commercial roadmap and Phase I plan must prevent L2/L3 product or algorithm conflation',
);

console.log('Watermark video Phase I contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Watermark video Phase I contract failed: ${message}`);
    process.exit(1);
  }
}
