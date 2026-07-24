import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const runId = process.env.HIDDENSHIELD_L3_PRODUCTION_READINESS_RUN_ID ?? `${Date.now()}`;
const requireReady = process.env.HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY === '1';
const outputDir = join(process.cwd(), 'tmp-ui-qa', 'l3-video-visual-production-readiness');
mkdirSync(outputDir, { recursive: true });

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  sellableChecklist: readFileSync('docs/L3视频画面盲水印可售验收清单.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  sharedCorePlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
};

const requiredSourceTokens = [
  '真实告警平台配置验证',
  '首个试点客户签字验收',
  '更大真实用户 MP4',
  'HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY',
  'cloud-video:l3-production-readiness-contract',
];

assert(
  sources.packageJson.includes('"cloud-video:l3-production-readiness-contract"'),
  'package.json must expose cloud-video:l3-production-readiness-contract',
);

for (const [name, source] of Object.entries(sources)) {
  if (name === 'packageJson') continue;
  for (const token of requiredSourceTokens.slice(0, 3)) {
    assert(source.includes(token), `${name} must record L3 readiness blocker: ${token}`);
  }
}

const artifacts = {
  alertWebhook: process.env.HIDDENSHIELD_L3_ALERT_PLATFORM_WEBHOOK ?? '',
  alertValidationJson: process.env.HIDDENSHIELD_L3_ALERT_PLATFORM_VALIDATION_JSON ?? '',
  pilotSignoffMd: process.env.HIDDENSHIELD_L3_PILOT_SIGNOFF_MD ?? '',
  realUserSampleManifest: process.env.HIDDENSHIELD_L3_REAL_USER_SAMPLE_MANIFEST ?? '',
};

const checks = [
  {
    id: 'real_alert_platform_webhook',
    passed: /^https:\/\//.test(artifacts.alertWebhook),
    detail: artifacts.alertWebhook ? 'webhook configured' : 'missing HIDDENSHIELD_L3_ALERT_PLATFORM_WEBHOOK',
  },
  {
    id: 'alert_platform_validation_json',
    passed: jsonArtifactHasSchema(
      artifacts.alertValidationJson,
      'l3_alert_platform_real_delivery_validation_v1',
    ),
    detail: artifacts.alertValidationJson || 'missing HIDDENSHIELD_L3_ALERT_PLATFORM_VALIDATION_JSON',
  },
  {
    id: 'pilot_customer_signoff',
    passed: textArtifactIncludes(
      artifacts.pilotSignoffMd,
      [
        'l3_pilot_customer_signoff_v1',
        'customerAcceptedL3Mp4OnlyBoundary: true',
        'supportAndRollbackOwnerSigned: true',
      ],
    ),
    detail: artifacts.pilotSignoffMd || 'missing HIDDENSHIELD_L3_PILOT_SIGNOFF_MD',
  },
  {
    id: 'real_user_mp4_sample_manifest',
    passed: jsonArtifactHasMinimumSamples(artifacts.realUserSampleManifest, 24),
    detail: artifacts.realUserSampleManifest || 'missing HIDDENSHIELD_L3_REAL_USER_SAMPLE_MANIFEST',
  },
];

const status = checks.every((check) => check.passed) ? 'ready' : 'blocked';
const result = {
  schemaVersion: 'l3_production_readiness_contract_v1',
  runId,
  status,
  requireReady,
  checks,
  privacyBoundary: 'artifact_paths_only_no_media_no_signed_url_no_local_path_payload',
};

const jsonPath = join(outputDir, `l3-production-readiness-contract-${runId}.json`);
const mdPath = join(outputDir, `l3-production-readiness-contract-${runId}.md`);
writeFileSync(jsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(mdPath, renderMarkdown(result), 'utf8');

if (requireReady && status !== 'ready') {
  console.error('L3 production readiness contract failed: external readiness artifacts are incomplete');
  console.error(`Report: ${mdPath}`);
  process.exit(1);
}

console.log(`L3 production readiness contract ${status.toUpperCase()}`);
console.log(`Readiness JSON: ${jsonPath}`);
console.log(`Readiness Markdown: ${mdPath}`);

function jsonArtifactHasSchema(path, schemaVersion) {
  if (!path || !existsSync(path)) return false;
  try {
    const parsed = JSON.parse(readFileSync(path, 'utf8'));
    return parsed?.schemaVersion === schemaVersion && parsed?.status === 'passed';
  } catch (_) {
    return false;
  }
}

function jsonArtifactHasMinimumSamples(path, minimumSamples) {
  if (!path || !existsSync(path)) return false;
  try {
    const parsed = JSON.parse(readFileSync(path, 'utf8'));
    return (
      parsed?.schemaVersion === 'l3_real_user_mp4_sample_manifest_v1' &&
      parsed?.status === 'passed' &&
      Array.isArray(parsed?.samples) &&
      parsed.samples.length >= minimumSamples &&
      parsed.samples.every((sample) => sample?.result === 'succeeded' || sample?.result === 'input_rejected_documented')
    );
  } catch (_) {
    return false;
  }
}

function textArtifactIncludes(path, tokens) {
  if (!path || !existsSync(path)) return false;
  const source = readFileSync(path, 'utf8');
  return tokens.every((token) => source.includes(token));
}

function renderMarkdown(result) {
  const lines = [
    '# HiddenShield L3 Production Readiness Contract',
    '',
    `- Run ID: ${result.runId}`,
    `- Status: ${result.status}`,
    `- Require ready: ${result.requireReady}`,
    `- Privacy boundary: ${result.privacyBoundary}`,
    '',
    '| Check | Result | Detail |',
    '| --- | --- | --- |',
  ];
  for (const check of result.checks) {
    lines.push(`| ${check.id} | ${check.passed ? 'passed' : 'blocked'} | ${escapeCell(check.detail)} |`);
  }
  lines.push('');
  lines.push('When `HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY=1`, every check must pass before L3 can be treated as production-sellable.');
  return `${lines.join('\n')}\n`;
}

function escapeCell(value) {
  return String(value).replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function assert(condition, message) {
  if (!condition) {
    console.error(`L3 production readiness contract failed: ${message}`);
    process.exit(1);
  }
}
