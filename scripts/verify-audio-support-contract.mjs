import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const contract = JSON.parse(
  await readFile("watermark-core/fixtures/audio-support-contract.json", "utf8"),
);
const sources = {
  core: await readFile("watermark-core/src/audio.rs", "utf8"),
  coreExports: await readFile("watermark-core/src/lib.rs", "utf8"),
  desktopApi: await readFile("src/lib/tauri-api.ts", "utf8"),
  desktopProbe: await readFile("src-tauri/src/commands/probe.rs", "utf8"),
  desktopErrors: await readFile("src-tauri/src/pipeline/error.rs", "utf8"),
  desktopMessages: await readFile("src-tauri/src/commands/transcode.rs", "utf8"),
  desktopWorkbench: await readFile("src/views/WorkbenchView.vue", "utf8"),
  dualRoadmap: await readFile("docs/双端能力一致性Roadmap.md", "utf8"),
};

assert.equal(contract.minimumSampleRate, 8000);
assert.equal(contract.maximumSampleRate, 48000);
assert.deepEqual(contract.supportedChannels, [1, 2]);
assert.equal(contract.minimumProtectionSeconds, 30);
assert.equal(contract.preserveSourceSpec, true);
assert.equal(contract.desktopOutput.container, "wav");
assert.deepEqual(contract.desktopOutput.supportedPcmCodecs, [
  "pcm_s16le",
  "pcm_s24le",
  "pcm_s32le",
  "pcm_f32le",
]);
assert.deepEqual(contract.desktopOutput.preserves, ["sampleRate", "channels"]);
assert.deepEqual(contract.desktopImageReferenceMatrix.formats, ["png", "jpeg", "webp"]);
assert.equal(contract.desktopImageReferenceMatrix.successfulReference.width, 1920);
assert.equal(contract.desktopImageReferenceMatrix.successfulReference.height, 1080);
assert.equal(contract.desktopImageReferenceMatrix.notAPublicMinimum, true);

assert.match(sources.core, /MIN_SUPPORTED_AUDIO_SAMPLE_RATE/);
assert.match(sources.core, /validate_audio_protection_input/);
assert.match(sources.coreExports, /validate_audio_protection_input/);
assert.match(sources.coreExports, /image_embed_capacity_sufficient/);
assert.match(sources.desktopApi, /standaloneAudioProtectionPreflight/);
assert.match(sources.desktopApi, /MIN_SUPPORTED_AUDIO_SAMPLE_RATE = 8_000/);
assert.match(sources.desktopProbe, /sample_rate/);
assert.match(sources.desktopProbe, /channels/);
assert.match(sources.desktopProbe, /watermark_eligible/);
assert.match(sources.desktopErrors, /audio_sample_rate_too_low/);
assert.match(sources.desktopErrors, /audio_channels_unsupported/);
assert.match(sources.desktopErrors, /image_capacity_insufficient/);
assert.match(sources.desktopMessages, /8–48 kHz/);
assert.match(sources.desktopWorkbench, /音频采样率暂不支持/);
assert.match(sources.desktopWorkbench, /音频声道暂不支持/);
assert.match(sources.desktopWorkbench, /图片可用水印容量不足/);
assert.match(sources.dualRoadmap, /8–48 kHz/);
assert.match(sources.dualRoadmap, /移动端冻结/);

for (const sample of contract.releaseCases) {
  const rateOk =
    sample.sampleRate >= contract.minimumSampleRate &&
    sample.sampleRate <= contract.maximumSampleRate;
  const channelsOk = contract.supportedChannels.includes(sample.channels);
  const durationOk = sample.durationSeconds >= contract.minimumProtectionSeconds;
  const expected =
    !rateOk
      ? sample.sampleRate < contract.minimumSampleRate
        ? contract.errorCodes.sampleRateTooLow
        : contract.errorCodes.sampleRateTooHigh
      : !channelsOk
        ? contract.errorCodes.channelsUnsupported
        : !durationOk
          ? contract.errorCodes.tooShort
          : "pass";
  assert.equal(expected, sample.expected, sample.id);
}

console.log("Audio support contract verified");
