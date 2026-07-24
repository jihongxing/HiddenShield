import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  payload: readFileSync('watermark-core/src/payload.rs', 'utf8'),
  coreLib: readFileSync('watermark-core/src/lib.rs', 'utf8'),
  service: readFileSync('watermark-core/src/service.rs', 'utf8'),
  image: readFileSync('watermark-core/src/image.rs', 'utf8'),
  audio: readFileSync('watermark-core/src/audio.rs', 'utf8'),
  desktopVerify: readFileSync('src-tauri/src/commands/verify.rs', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  videoVisual: readFileSync('watermark-core/src/video_visual.rs', 'utf8'),
  protocolDoc: readFileSync('docs/公开权利信号与训练许可扫描协议设计.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  watermarkPlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
};

assert(
  sources.packageJson.includes('"rights:v3-minimal-anchor-contract"') &&
    sources.packageJson.includes('verify-v3-minimal-anchor-contract.mjs'),
  'package.json must expose the V3 minimal anchor contract',
);

assert(
  sources.payload.includes('pub const PAYLOAD_BYTES: usize = 119;') &&
    sources.payload.includes('pub const PAYLOAD_V3_MINIMAL_ANCHOR_BYTES') &&
    sources.payload.includes('PAYLOAD_V3_MINIMAL_ANCHOR_BYTES, 39') &&
    sources.payload.includes('WatermarkPayloadV3MinimalAnchor') &&
    sources.payload.includes('WatermarkDecodedPayload') &&
    sources.payload.includes('PayloadV3MinimalAnchorBuildInput') &&
    sources.payload.includes('encode_payload_v3_minimal_anchor') &&
    sources.payload.includes('decode_payload_v3_minimal_anchor') &&
    sources.payload.includes('decode_watermark_payload_readonly') &&
    sources.payload.includes('v3_minimal_anchor_roundtrips_without_expanding_v2_payload') &&
    sources.payload.includes('v3_minimal_anchor_rejects_tampered_uid'),
  'watermark-core payload layer must define a standalone V3 minimal anchor codec while keeping V2/119 fixed',
);

assert(
  sources.coreLib.includes('WatermarkPayloadV3MinimalAnchor') &&
    sources.coreLib.includes('WatermarkDecodedPayload') &&
    sources.coreLib.includes('decode_watermark_payload_readonly') &&
    sources.coreLib.includes('PAYLOAD_V3_MINIMAL_ANCHOR_BYTES'),
  'watermark-core lib exports must make V3 minimal anchor available for future migration fixtures',
);

assert(
  sources.image.includes('encode_image_sync_packet_v3_readonly') &&
    sources.image.includes('extract_image_watermark_readonly_candidate_bytes') &&
    sources.audio.includes('encode_audio_recovery_packet_v3_readonly') &&
    sources.audio.includes('extract_watermark_wav_readonly_candidate_bytes') &&
    sources.coreLib.includes('extract_image_watermark_readonly_candidate_bytes') &&
    sources.coreLib.includes('extract_watermark_samples_readonly_candidate') &&
    sources.coreLib.includes('build_v3_readonly_candidate_image_fixture_png_bytes') &&
    sources.coreLib.includes('build_v3_readonly_candidate_audio_fixture_wav_bytes') &&
    sources.desktopVerify.includes('verify_suspect_readonly_candidate') &&
    sources.mobileRustApi.includes('extract_image_readonly_candidate_for_mobile') &&
    sources.mobileRustApi.includes('extract_audio_wav_readonly_candidate_for_mobile') &&
    sources.service.includes('DefaultV3') &&
    sources.service.includes('ForceV2Rollback') &&
    sources.service.includes('require_v3_default') &&
    sources.service.includes('v3_anchor_from_v2_payload') &&
    sources.service.includes('pub fn extract_v2') &&
    !sources.videoVisual.includes('encode_payload_v3_minimal_anchor') &&
    !sources.videoVisual.includes('decode_watermark_payload_readonly') &&
    !sources.videoVisual.includes('decode_payload_v3_minimal_anchor'),
  'V3 minimal anchor must be the default image/audio service payload while video visual paths stay outside this migration',
);

assert(
  sources.protocolDoc.includes('默认图片 / 音频正式路径已切到 V3/39') &&
    sources.capabilityBoundary.includes('V3 最小锚点默认 codec') &&
    sources.watermarkPlan.includes('V3 最小锚点默认 codec') &&
    sources.dualRoadmap.includes('图片 / 音频 V3 默认算法迁移'),
  'docs must state that V3 minimal anchor is now the default image/audio codec with iOS and non-image/audio gaps still pending',
);

console.log('V3 minimal anchor contract passed');
