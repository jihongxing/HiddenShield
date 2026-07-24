#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const runId = process.env.HIDDENSHIELD_PUBLIC_METADATA_PRODUCTION_STAGING_RUN_ID ?? String(Date.now());
const outputDir = resolve('tmp-ui-qa', 'public-metadata-production-staging', runId);
const qaJsonPath = join(outputDir, `public-metadata-production-staging-qa-${runId}.json`);
const qaMdPath = join(outputDir, `public-metadata-production-staging-qa-${runId}.md`);
mkdirSync(outputDir, { recursive: true });

const requiredSecrets = [
  'HIDDENSHIELD_C2PA_SIGN_CERT_PEM',
  'HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM',
  'HIDDENSHIELD_C2PA_SIGNING_ALG',
  'HIDDENSHIELD_C2PA_TSA_URL',
];

const secretStatus = Object.fromEntries(
  requiredSecrets.map((name) => {
    const value = process.env[name];
    return [name, { present: typeof value === 'string' && value.trim().length > 0 }];
  }),
);
const missingSecrets = Object.entries(secretStatus)
  .filter(([, status]) => !status.present)
  .map(([name]) => name);

if (missingSecrets.length > 0) {
  const result = {
    runId,
    status: 'blocked_missing_c2pa_tsa_secrets',
    legalConclusion: false,
    missingSecrets,
    secretStatus,
    completedAt: new Date().toISOString(),
  };
  writeEvidence(result);
  console.error(`Production C2PA/TSA staging QA blocked: missing ${missingSecrets.join(', ')}`);
  process.exit(1);
}

const childRunId = `production-staging-${runId}`;
const qa = spawnSync(
  process.platform === 'win32' ? 'npm.cmd' : 'npm',
  ['run', 'rights:metadata-embed-runtime-qa'],
  {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
    windowsHide: true,
    env: {
      ...process.env,
      HIDDENSHIELD_PUBLIC_METADATA_EMBED_QA_RUN_ID: childRunId,
    },
  },
);

const imageQaJsonPath = resolve(
  'tmp-ui-qa',
  'public-metadata-embedded-image',
  childRunId,
  `public-metadata-embedded-image-qa-${childRunId}.json`,
);

if (qa.status !== 0) {
  const result = {
    runId,
    status: 'failed_runtime_qa',
    legalConclusion: false,
    childRunId,
    imageQaJsonPath,
    stdoutTail: tail(qa.stdout),
    stderrTail: tail(qa.stderr),
    completedAt: new Date().toISOString(),
  };
  writeEvidence(result);
  throw new Error(`production C2PA/TSA staging runtime QA failed: ${qaJsonPath}`);
}

const imageQa = JSON.parse(readFileSync(imageQaJsonPath, 'utf8'));
const signerRows = imageQa.rows.map((row) => {
  const checks = JSON.parse(readFileSync(row.checkJsonPath, 'utf8'));
  return {
    format: row.format,
    watermarkUid: row.watermarkUid,
    checkJsonPath: row.checkJsonPath,
    c2paSignerStatus: checks.c2paSignerStatus,
    c2paManifestHash: checks.c2paManifestHash,
    hasC2paActiveManifest: checks.checks?.hasC2paActiveManifest === true,
  };
});

const badRows = signerRows.filter(
  (row) =>
    row.c2paSignerStatus !== 'configured_certificate_chain' ||
    row.hasC2paActiveManifest !== true ||
    !row.c2paManifestHash,
);
const result = {
  runId,
  status: badRows.length === 0 ? 'passed' : 'failed_signer_status',
  legalConclusion: false,
  childRunId,
  imageQaJsonPath,
  signerRows,
  badRows,
  completedAt: new Date().toISOString(),
};
writeEvidence(result);

if (badRows.length > 0) {
  throw new Error(`production C2PA/TSA staging QA did not use configured certificate chain: ${qaJsonPath}`);
}

console.log(`Production C2PA/TSA staging QA OK: ${qaMdPath}`);

function writeEvidence(result) {
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
}

function renderMarkdown(result) {
  const lines = [
    '# Public Metadata Production C2PA/TSA Staging QA',
    '',
    `- runId: \`${result.runId}\``,
    `- status: \`${result.status}\``,
    `- legalConclusion: \`${result.legalConclusion}\``,
    `- completedAt: \`${result.completedAt}\``,
    '',
  ];
  if (result.missingSecrets?.length) {
    lines.push('## Missing Secrets', '', ...result.missingSecrets.map((name) => `- \`${name}\``), '');
  }
  if (result.signerRows?.length) {
    lines.push(
      '## Signer Rows',
      '',
      '| format | watermarkUid | signerStatus | activeManifest | manifestHash |',
      '| --- | --- | --- | --- | --- |',
    );
    for (const row of result.signerRows) {
      lines.push(
        `| ${row.format} | \`${row.watermarkUid}\` | \`${row.c2paSignerStatus}\` | ${row.hasC2paActiveManifest ? 'PASS' : 'FAIL'} | \`${row.c2paManifestHash ?? ''}\` |`,
      );
    }
    lines.push('');
  }
  lines.push(
    '## Next Step',
    '',
    result.status === 'passed'
      ? 'Release owner should attach certificate-chain review and TSA availability records before making any production C2PA/TSA claim.'
      : 'Inject production-equivalent C2PA certificate chain, private key, signing algorithm, and TSA URL through staging secret manager, then rerun this command.',
    '',
  );
  return `${lines.join('\n')}\n`;
}

function tail(value) {
  return String(value ?? '')
    .split(/\r?\n/)
    .slice(-40)
    .join('\n');
}
