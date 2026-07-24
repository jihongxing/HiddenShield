import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'v3-readonly-fixtures', runId);
mkdirSync(outputDir, { recursive: true });

run('cargo', [
  'run',
  '--manifest-path',
  'src-tauri/Cargo.toml',
  '--features',
  'internal-qa',
  '--example',
  'v3_readonly_fixture_qa',
  '--',
  '--out-dir',
  outputDir,
]);

run('cargo', [
  'test',
  '--manifest-path',
  'mobile_app/rust/Cargo.toml',
  'mobile_v3_readonly',
  '--',
  '--nocapture',
]);

const desktop = JSON.parse(readFileSync(join(outputDir, 'desktop-v3-readonly-fixtures.json'), 'utf8'));
const rows = [
  {
    direction: 'desktop bridge readonly payload bytes',
    kind: 'image',
    ...desktop.desktop.image,
  },
  {
    direction: 'desktop bridge readonly payload bytes',
    kind: 'audio',
    ...desktop.desktop.audio,
  },
  {
    direction: 'desktop bridge readonly media container',
    kind: 'image',
    ...desktop.desktop.imageMedia,
  },
  {
    direction: 'desktop bridge readonly media container',
    kind: 'audio',
    ...desktop.desktop.audioMedia,
  },
];

for (const row of rows) {
  assert(row.watermarkUid.startsWith('HS-'), `${row.kind} watermarkUid must be present`);
  assert(row.payloadProtocolVersion === 3, `${row.kind} payloadProtocolVersion must be 3`);
  assert(row.payloadBytesLength === 39, `${row.kind} payloadBytesLength must be 39`);
  assert(row.payloadAuthStatus === 'verified', `${row.kind} payloadAuthStatus must be verified`);
}
assert(desktop.defaultV3WriteEnabled === true, 'V3 default write must be enabled');

const result = {
  runId,
  outputDir,
  desktop,
  mobile: {
    cargoTest: 'mobile_v3_readonly_fixture_preserves_anchor_fields',
    cargoMediaTest: 'mobile_v3_readonly_media_fixture_preserves_anchor_fields',
    expectedBridgeFields: {
      payloadProtocolVersion: 3,
      payloadBytesLength: 39,
      payloadAuthStatus: 'verified',
    },
  },
  boundary:
    'Readonly QA uses controlled V3 minimal anchor bytes and staged PNG/WAV fixture containers. It does not enable default V3 writes or route formal image/audio extraction through V3.',
  pass: true,
};
const qaJsonPath = join(outputDir, `v3-readonly-fixture-qa-${runId}.json`);
const qaMdPath = join(outputDir, `v3-readonly-fixture-qa-${runId}.md`);
writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(qaMdPath, renderMarkdown(result, rows), 'utf8');

console.log(`V3 readonly fixture QA JSON: ${qaJsonPath}`);
console.log(`V3 readonly fixture QA Markdown: ${qaMdPath}`);

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: resolve('.'),
    env: process.env,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function renderMarkdown(result, rows) {
  const lines = [
    '# HiddenShield V3 Readonly Fixture QA',
    '',
    `- Run ID: ${result.runId}`,
    `- Output: ${result.outputDir}`,
    `- Boundary: ${result.boundary}`,
    '',
    '| Direction | Kind | Watermark UID | Payload | Auth | Pass |',
    '| --- | --- | --- | --- | --- | --- |',
    ...rows.map(
      (row) =>
        `| ${row.direction} | ${row.kind} | ${row.watermarkUid} | V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes | ${row.payloadAuthStatus} | PASS |`,
    ),
    '',
    'Mobile Rust bridge cargo test:',
    '',
    `- ${result.mobile.cargoTest}: PASS`,
    `- ${result.mobile.cargoMediaTest}: PASS`,
  ];
  return `${lines.join('\n')}\n`;
}
