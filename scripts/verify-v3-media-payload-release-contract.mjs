import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  qaBin: readFileSync('watermark-core/src/bin/v3_media_payload_release_qa.rs', 'utf8'),
  boundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  sharedPlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
  videoContract: readFileSync('scripts/verify-watermark-video-phase-contract.mjs', 'utf8'),
};

assert(
  sources.packageJson.includes('rights:v3-media-payload-release-qa') &&
    sources.packageJson.includes('v3_media_payload_release_qa'),
  'package.json must expose rights:v3-media-payload-release-qa',
);

for (const token of [
  'image_png',
  'image_jpeg',
  'image_webp',
  'image_bmp',
  'ImageOutputFormat::Png',
  'ImageOutputFormat::Jpeg',
  'ImageOutputFormat::WebP',
  'ImageOutputFormat::Bmp',
  'video_l1_audio_track',
  'video_l2_fingerprint_notary',
  'AudioProtectionMode::VideoTrack',
  'shouldHaveMediaPayload\\\":false',
  'not_applicable_l2_fingerprint',
  'PAYLOAD_V3_MINIMAL_ANCHOR_BYTES',
  'payloadProtocolVersion',
  'payloadBytesLength',
  'v3_minimal_anchor',
]) {
  assert(sources.qaBin.includes(token), `V3 media payload QA must include ${token}`);
}

assert(
  sources.boundary.includes('L1 视频音轨水印') &&
    sources.boundary.includes('L2 视频指纹存证') &&
    sources.boundary.includes('L2 不是盲水印') &&
    sources.boundary.includes('默认媒体锚点已切到 V3/39'),
  'capability boundary must preserve V3 payload and L1/L2 split',
);

assert(
  sources.sharedPlan.includes('V3/39') &&
    sources.sharedPlan.includes('L3 保持 staged / internal') &&
    sources.videoContract.includes('L2 must preserve three-layer irreversible VideoFingerprintBundle'),
  'shared plan and video contract must preserve V3 and L2/L3 boundary',
);

console.log('V3 media payload release contract passed');
