import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  ci: readFileSync('.github/workflows/ci.yml', 'utf8'),
  plan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  roadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
};

const mode = parseMode(process.argv.slice(2));
const includeL3ReleasePool = process.env.HIDDENSHIELD_L3_FULL_RELEASE_POOL === '1';

const requiredTests = [
  'audio_container_fixtures_are_valid',
  'mobile_image_output_is_desktop_core_extractable',
  'desktop_core_image_output_is_mobile_extractable',
  'mobile_jpeg_image_input_is_desktop_core_extractable',
  'desktop_core_jpeg_image_input_is_mobile_extractable',
  'mobile_webp_image_input_is_desktop_core_extractable',
  'desktop_core_webp_image_input_is_mobile_extractable',
  'mobile_audio_output_is_desktop_core_extractable',
  'desktop_core_audio_output_is_mobile_extractable',
  'mobile_flac_audio_input_is_desktop_core_extractable',
  'mobile_mp3_audio_input_is_desktop_core_extractable',
  'mobile_ogg_audio_input_is_desktop_core_extractable',
  'mobile_m4a_audio_input_is_desktop_core_extractable',
  'mobile_aac_audio_input_is_desktop_core_extractable',
  'mobile_flac_audio_input_normalizes_to_desktop_core_payload',
  'mobile_mp3_audio_input_normalizes_to_desktop_core_payload',
  'mobile_ogg_audio_input_normalizes_to_desktop_core_payload',
  'mobile_m4a_audio_input_normalizes_to_desktop_core_payload',
  'mobile_aac_audio_input_normalizes_to_desktop_core_payload',
  'cross_end_image_bridge_contract_group',
  'cross_end_wav_core_algorithm_group',
  'cross_end_non_wav_mobile_normalize_group',
  'cross_end_non_wav_bridge_contract_group',
  'desktop_transcode_audio_fixtures_extract_to_core_wav',
  'l1_video_audio_track_roundtrip_extracts_core_watermark',
  'l3_decoded_video_y_plane_fixture_enters_watermark_core',
  'l3_decoded_video_y_plane_fixture_roundtrips_dct_in_watermark_core',
  'l3_encoded_video_y_plane_fixture_self_checks_after_ffmpeg_roundtrip',
  'l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary',
  'l3_target_platform_transcode_matrix_classifies_dct_survival',
  'l3_main_resolution_transcode_matrix_covers_720p_1080p_2k',
  'l3_main_resolution_platform_profiles_cover_720p_1080p_2k',
  'l3_mainstream_bitrate_floor_matrix_covers_720p_1080p_2k',
  'l3_30s_commercial_sampling_performance_records_cost_breakdown',
  'l3_bilibili_hevc_mainstream_floor_records_cost_breakdown',
  'l3_bilibili_h264_hevc_cost_comparison_records_budget',
  'l3_2k_h264_strategy_density_budget_records_confidence_curve',
  'l3_2k_h264_sample_count_budget_records_confidence_curve',
  'l3_2k_h264_region_quality_budget_records_confidence_curve',
  'l3_platform_timing_budget_records_16frame_seeded_costs',
];

const testGroups = [
  {
    code: 'fixture_invalid',
    label: 'audio fixture validity',
    filter: 'audio_container_fixtures_are_valid',
    modes: ['fast', 'release'],
  },
  {
    code: 'bridge_contract',
    label: 'image bridge extraction contract',
    filter: 'cross_end_image_bridge_contract_group',
    modes: ['fast', 'release'],
  },
  {
    code: 'core_algorithm',
    label: 'WAV core cross-end extraction',
    filter: 'cross_end_wav_core_algorithm_group',
    modes: ['fast', 'release'],
  },
  {
    code: 'mobile_normalize',
    label: 'non-WAV mobile normalization',
    filter: 'cross_end_non_wav_mobile_normalize_group',
    modes: ['release'],
  },
  {
    code: 'bridge_contract',
    label: 'non-WAV mobile output desktop extraction',
    filter: 'cross_end_non_wav_bridge_contract_group',
    modes: ['release'],
  },
  {
    code: 'desktop_transcode',
    label: 'desktop FFmpeg audio transcode to core WAV',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'desktop_transcode_audio_fixtures_extract_to_core_wav',
    modes: ['release'],
  },
  {
    code: 'desktop_transcode',
    label: 'L1 video audio track core watermark roundtrip',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l1_video_audio_track_roundtrip_extracts_core_watermark',
    modes: ['release'],
  },
  {
    code: 'desktop_transcode',
    label: 'L1 video audio track release container matrix',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l1_video_audio_track_accepts_release_input_containers',
    modes: ['release'],
  },
  {
    code: 'desktop_transcode',
    label: 'L3 decoded video Y-plane enters watermark-core',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_decoded_video_y_plane_fixture_enters_watermark_core',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 decoded video Y-plane DCT staged core roundtrip',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_decoded_video_y_plane_fixture_roundtrips_dct_in_watermark_core',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 encoded video Y-plane DCT staged self-check',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_encoded_video_y_plane_fixture_self_checks_after_ffmpeg_roundtrip',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 lossy video Y-plane DCT staged survival boundary',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 target-platform transcode DCT staged survival matrix',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_target_platform_transcode_matrix_classifies_dct_survival',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 main-resolution 720p/1080p/2K DCT staged matrix',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_main_resolution_transcode_matrix_covers_720p_1080p_2k',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 platform-profile 720p/1080p/2K DCT staged matrix',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_main_resolution_platform_profiles_cover_720p_1080p_2k',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 mainstream bitrate floor DCT staged matrix',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_mainstream_bitrate_floor_matrix_covers_720p_1080p_2k',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 30s commercial sampling performance and boundary record',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_30s_commercial_sampling_performance_records_cost_breakdown',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 Bilibili HEVC mainstream floor probe',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_bilibili_hevc_mainstream_floor_records_cost_breakdown',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 Bilibili H.264/HEVC 30s cost comparison',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_bilibili_h264_hevc_cost_comparison_records_budget',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 2K H.264 strategy-density confidence curve',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_2k_h264_strategy_density_budget_records_confidence_curve',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 2K H.264 sample-count confidence curve',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_2k_h264_sample_count_budget_records_confidence_curve',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 2K H.264 region-quality confidence curve',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_2k_h264_region_quality_budget_records_confidence_curve',
    modes: ['release'],
  },
  {
    code: 'core_algorithm',
    label: 'L3 platform timing budget for 16-frame seeded strategy',
    manifest: 'src-tauri/Cargo.toml',
    filter: 'l3_platform_timing_budget_records_16frame_seeded_costs',
    modes: ['release'],
  },
];

assert(
  sources.packageJson.includes('"watermark:cross-end-fast"') &&
    sources.packageJson.includes('"watermark:cross-end-contract"') &&
    sources.ci.includes('npm run watermark:cross-end-contract'),
  'static_contract',
  'cross-end watermark fast/release contracts must be exposed in package.json, and release must run in CI',
);

assert(
  sources.plan.includes('Phase I-2：跨端金样本 fixtures') &&
    sources.plan.includes('watermark:cross-end-contract') &&
    sources.roadmap.includes('watermark:cross-end-contract'),
  'static_contract',
  'Phase I docs must record the cross-end watermark contract gate',
);

assert(
  sources.plan.includes('core_algorithm') &&
    sources.plan.includes('mobile_normalize') &&
    sources.plan.includes('desktop_transcode') &&
    sources.plan.includes('bridge_contract') &&
    sources.plan.includes('fixture_invalid'),
  'static_contract',
  'Phase I docs must define cross-end failure attribution codes',
);

assert(
  sources.plan.includes('L1 视频音轨水印') &&
    sources.plan.includes('L2 视频指纹存证') &&
    sources.plan.includes('L3 端云协同画面盲水印') &&
    sources.desktopScheduler.includes('AudioProtectionMode::VideoTrack') &&
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
    sources.desktopScheduler.includes('l3_2k_h264_strategy_density_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_2k_h264_sample_count_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_2k_h264_region_quality_budget_records_confidence_curve') &&
    sources.desktopScheduler.includes('l3_platform_timing_budget_records_16frame_seeded_costs') &&
    sources.desktopScheduler.includes('video_frame_plane_from_decoded_luma'),
  'static_contract',
  'Phase I video consistency must separate L1/L2/L3 and keep L1 video audio plus L3 decoded Y-plane/DCT staged boundaries on watermark-core',
);

for (const testName of requiredTests) {
  assert(
    sources.mobileRustApi.includes(testName) ||
      sources.desktopScheduler.includes(testName),
    'bridge_contract',
    `cross-end contract is missing required test ${testName}`,
  );
}

for (const group of testGroups.filter((group) => group.modes.includes(mode))) {
  if (isL3Group(group) && !includeL3ReleasePool) {
    console.log(
      `Skipping ${group.label}: L3 is frozen for the current dual-end release. Set HIDDENSHIELD_L3_FULL_RELEASE_POOL=1 to run this internal pool.`,
    );
    continue;
  }
  runCargoTests(group);
}

console.log(`Watermark cross-end ${mode} contract OK`);

function runCargoTests(group) {
  const startedAt = performance.now();
  const profileArgs = mode === 'release' ? ['--release'] : [];
  console.log(
    `[cross-end] ${group.label}: starting ${mode} Cargo test${mode === 'release' ? ' build/run' : ''}`,
  );
  const result = spawnSync(
    'cargo',
    [
      'test',
      ...profileArgs,
      '--manifest-path',
      group.manifest ?? 'mobile_app/rust/Cargo.toml',
      group.filter,
      '--',
      '--nocapture',
    ],
    { encoding: 'utf8' },
  );

  if (result.stdout) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.status !== 0) {
    fail(
      group.code,
      `${group.label} failed while running ${group.filter}; inspect the Rust test output above`,
    );
  }

  console.log(
    `[cross-end] ${group.label}: completed in ${((performance.now() - startedAt) / 1000).toFixed(1)}s`,
  );

  const passedPattern = new RegExp(
    `test .*${escapeRegExp(group.filter)}.*\\.\\.\\. ok`,
  );
  if (!passedPattern.test(result.stdout)) {
    fail(
      'static_contract',
      `${group.label} did not report a passing Rust test for filter ${group.filter}`,
    );
  }
}

function isL3Group(group) {
  return group.filter.startsWith('l3_') || group.label.startsWith('L3 ');
}

function assert(condition, code, message) {
  if (!condition) {
    fail(code, message);
  }
}

function fail(code, message) {
  console.error(`Watermark cross-end contract failed [${code}]: ${message}`);
  process.exit(1);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function parseMode(args) {
  const value = args
    .find((arg) => arg.startsWith('--mode='))
    ?.slice('--mode='.length);
  if (!value || value === 'release') {
    return 'release';
  }
  if (value === 'fast') {
    return 'fast';
  }
  fail('static_contract', `unknown cross-end contract mode ${value}`);
}
