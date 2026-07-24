import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const runId = process.env.HIDDENSHIELD_PUBLIC_METADATA_AV_QA_RUN_ID ?? `${Date.now()}`;
const outputDir = resolve('tmp-ui-qa', 'public-metadata-embedded-av', runId);
const sourceDir = join(outputDir, 'source');
const metadataDir = join(outputDir, 'metadata');
const embeddedDir = join(outputDir, 'embedded');
mkdirSync(sourceDir, { recursive: true });
mkdirSync(metadataDir, { recursive: true });
mkdirSync(embeddedDir, { recursive: true });

const metadata = {
  watermarkUid: `wm_av_metadata_${runId}`,
  manifestHash: `sha256:${'a'.repeat(64)}`,
  legalConclusion: false,
  signedManifestStore: {
    format: 'hidden-shield-signed-c2pa-manifest-store-json',
    manifestStoreHash: `sha256:${'b'.repeat(64)}`,
    signatureAlgorithm: 'HMAC-SHA256',
    signature: 'fixture-signature',
    verificationStatus: 'signed_by_hiddenshield_registry_key',
    legalConclusion: false,
  },
  jsonLd: {
    'hs:trainingPolicy': 'separate_authorization_required',
    'hs:legalConclusion': false,
  },
};
const metadataPath = join(metadataDir, 'metadata.json');
writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`, 'utf8');

const cases = [
  {
    format: 'wav',
    sourcePath: join(sourceDir, 'fixture.wav'),
    outputPath: join(embeddedDir, 'fixture.embedded.wav'),
    create: () => createWav(join(sourceDir, 'fixture.wav')),
  },
  {
    format: 'mp4',
    sourcePath: join(sourceDir, 'fixture.mp4'),
    outputPath: join(embeddedDir, 'fixture.embedded.mp4'),
    create: () => createMp4(join(sourceDir, 'fixture.mp4')),
  },
];

const rows = [];
for (const item of cases) {
  item.create();
  const checkJsonPath = join(embeddedDir, `${item.format}.checks.json`);
  run('cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'src-tauri/Cargo.toml',
    '--features',
    'internal-qa',
    '--example',
    'public_metadata_embed_qa',
    '--',
    '--source',
    item.sourcePath,
    '--metadata',
    metadataPath,
    '--output',
    item.outputPath,
    '--format',
    item.format,
    '--json-out',
    checkJsonPath,
  ]);
  const checks = JSON.parse(readFileSync(checkJsonPath, 'utf8'));
  rows.push({
    format: item.format,
    sourcePath: item.sourcePath,
    outputPath: item.outputPath,
    checkJsonPath,
    propagationLayer: checks.propagationLayer,
    c2paManifestHash: checks.c2paManifestHash,
    c2paSignerStatus: checks.c2paSignerStatus,
    checks: checks.checks,
    pass: Object.values(checks.checks).every((value) => value === true),
  });
}

const result = {
  runId,
  outputDir,
  metadataPath,
  rows,
  pass: rows.every((row) => row.pass),
  completedAt: new Date().toISOString(),
};
const qaJsonPath = join(outputDir, `public-metadata-embedded-av-qa-${runId}.json`);
const qaMdPath = join(outputDir, `public-metadata-embedded-av-qa-${runId}.md`);
writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
if (!result.pass) {
  throw new Error(`public metadata embedded AV runtime QA failed: ${qaJsonPath}`);
}
console.log('Public metadata embedded audio/video runtime QA OK');
console.log(`QA JSON: ${qaJsonPath}`);
console.log(`QA Markdown: ${qaMdPath}`);

function createWav(outputPath) {
  run('ffmpeg', [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'sine=frequency=880:duration=1',
    '-ac',
    '1',
    '-ar',
    '44100',
    outputPath,
  ]);
}

function createMp4(outputPath) {
  run('ffmpeg', [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'testsrc=size=160x90:rate=10:duration=1',
    '-f',
    'lavfi',
    '-i',
    'sine=frequency=440:duration=1',
    '-c:v',
    'libx264',
    '-pix_fmt',
    'yuv420p',
    '-c:a',
    'aac',
    '-movflags',
    '+faststart',
    '-shortest',
    outputPath,
  ]);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed with ${result.status}\nSTDOUT:\n${result.stdout}\nSTDERR:\n${result.stderr}`,
    );
  }
}

function renderMarkdown(result) {
  const lines = [
    '# HiddenShield 公开元数据嵌入音频/视频运行态 QA',
    '',
    `- Run ID: \`${result.runId}\``,
    `- 完成时间: ${result.completedAt}`,
    '',
    '| 格式 | 传播层 | 官方 C2PA active manifest | signer | UID | manifestHash | signedManifestHash | legalConclusion=false | 结果 |',
    '| --- | --- | --- | --- | --- | --- | --- | --- | --- |',
  ];
  for (const row of result.rows) {
    lines.push(
      `| ${row.format} | ${row.propagationLayer} | ${mark(row.checks.hasC2paActiveManifest)} | ${row.c2paSignerStatus ?? 'n/a'} | ${mark(row.checks.hasWatermarkUid)} | ${mark(row.checks.hasManifestHash)} | ${mark(row.checks.hasSignedManifestHash)} | ${mark(row.checks.hasLegalConclusionFalse)} | ${row.pass ? 'PASS' : 'FAIL'} |`,
    );
  }
  return `${lines.join('\n')}\n`;
}

function mark(value) {
  return value ? 'PASS' : 'FAIL';
}
