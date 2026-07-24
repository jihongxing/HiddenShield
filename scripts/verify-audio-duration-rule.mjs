import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import ts from "typescript";

const source = await readFile("src/lib/tauri-api.ts", "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ES2020,
    target: ts.ScriptTarget.ES2020,
  },
});
const tmp = await mkdtemp(join(tmpdir(), "hiddenshield-audio-rule-"));
const modulePath = join(tmp, "tauri-api.mjs");
await writeFile(modulePath, transpiled.outputText);
const api = await import(pathToFileURL(modulePath).href);

const baseMeta = {
  fileName: "sample.wav",
  path: "sample.wav",
  width: 0,
  height: 0,
  fps: 0,
  durationSecs: 30,
  durationConfirmed: true,
  sampleRate: 44_100,
  channels: 2,
  fileSizeBytes: 1024,
  fileSizeMb: 1,
  isHdr: false,
  colorProfile: "audio",
  sha256: "mock",
  fileType: "audio",
};

assert.equal(api.MIN_AUDIO_PROTECTION_SECONDS, 30);
assert.equal(api.MAX_AUDIO_PROTECTION_SECONDS, 1200);
assert.equal(api.MAX_AUDIO_PROTECTION_BYTES, 512 * 1024 * 1024);
assert.equal(api.isStandaloneAudioTooShort(null), false);
assert.equal(api.isStandaloneAudioTooShort({ ...baseMeta, durationSecs: 10 }), true);
assert.equal(api.isStandaloneAudioTooShort({ ...baseMeta, durationSecs: 29.99 }), true);
assert.equal(api.isStandaloneAudioTooShort({ ...baseMeta, durationSecs: 30 }), false);
assert.equal(api.isStandaloneAudioTooShort({ ...baseMeta, durationSecs: 42 }), false);
assert.equal(api.standaloneAudioProtectionPreflight({ ...baseMeta, durationSecs: 1200 }), "ok");
assert.equal(
  api.standaloneAudioProtectionPreflight({ ...baseMeta, durationSecs: 1200.001 }),
  "audio_too_long",
);
assert.equal(
  api.standaloneAudioProtectionPreflight({
    ...baseMeta,
    fileSizeBytes: 512 * 1024 * 1024,
  }),
  "ok",
);
assert.equal(
  api.standaloneAudioProtectionPreflight({
    ...baseMeta,
    fileSizeBytes: 512 * 1024 * 1024 + 1,
  }),
  "audio_file_too_large",
);
assert.equal(
  api.isStandaloneAudioTooShort({ ...baseMeta, fileType: "video", durationSecs: 10 }),
  false,
);

console.log("Audio duration rule verified");
