import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const visualFixtureDirectory = 'docs/fixtures/ai-transparency-third-party-visual-watermark-v1';
const visualManifest = JSON.parse(
  readFileSync(`${visualFixtureDirectory}/manifest.json`, 'utf8'),
);
const visualInput = `${visualFixtureDirectory}/${visualManifest.file.path}`;
const output = 'tmp-ui-qa/ai-transparency-third-party-layered-benchmark/hiddenshield-v3.png';

assert(visualManifest.source.license === 'MIT', 'visual watermark fixture license must be MIT');
assert(visualManifest.expected.visibleExternalWatermarkPresent, 'visual watermark fixture must declare a visible external watermark');
assert(!visualManifest.expected.hiddenShieldV3AnchorPresentBeforeWrite, 'visual watermark fixture must start without a HiddenShield anchor');
assert(!visualManifest.expected.externalPlatformAcceptanceAuthorized, 'fixture must not imply external platform acceptance');
assert(!visualManifest.expected.legalConclusion, 'fixture must not make a legal conclusion');
assert(existsSync(visualInput), 'visual watermark fixture is missing');
assert(
  createHash('sha256').update(readFileSync(visualInput)).digest('hex') === visualManifest.file.sha256,
  'visual watermark fixture SHA-256 mismatch',
);

run('cargo', [
  'run',
  '--manifest-path',
  'src-tauri/Cargo.toml',
  '--bin',
  'ai_transparency_third_party_c2pa_fixture_qa',
]);

const prewrite = run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'desktop_image_read_qa',
  '--',
  visualInput,
], false);
assert(prewrite.status !== 0, 'visual watermark fixture must not contain a HiddenShield anchor before write');

run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'desktop_image_write_qa',
  '--',
  visualInput,
  output,
  'HS-89ABCDEF-01234567-89ABCDEF-01234567',
]);
run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'desktop_image_read_qa',
  '--',
  output,
]);

console.log('AI Transparency third-party layered benchmark passed');

function run(command, args, required = true) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: 'inherit',
  });
  if (required && result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed`);
  }
  return result;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
