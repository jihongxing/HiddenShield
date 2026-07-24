#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function envPath(name) {
  const value = process.env[name]?.trim();
  return value ? resolve(value) : null;
}

const runId = process.env.HIDDENSHIELD_PUBLIC_RIGHTS_COMPLETION_RUN_ID ?? `${Date.now()}`;
const requireComplete = process.env.HIDDENSHIELD_PUBLIC_RIGHTS_REQUIRE_COMPLETE === '1';
const outputDir = resolve('tmp-ui-qa', 'public-rights-completion');
const jsonPath = join(outputDir, `public-rights-completion-gate-${runId}.json`);
const mdPath = join(outputDir, `public-rights-completion-gate-${runId}.md`);
mkdirSync(outputDir, { recursive: true });

const sources = {
  packageJson: readText('package.json'),
  c2paCommand: readText('src-tauri/src/commands/public_metadata.rs'),
  sdkPackageJson: readText('packages/public-rights-sdk/package.json'),
  sdkReadme: readText('packages/public-rights-sdk/README.md'),
  iosQaScript: readText('scripts/verify-ios-public-rights-v3-runtime-qa.mjs'),
  iosQaTool: readText('mobile_app/tool/ios_public_rights_v3_runtime_qa.dart'),
  protocolDoc: readText('docs/公开权利信号与训练许可扫描协议设计.md'),
  remainingTasks: readText('docs/公开权利信号与训练许可扫描剩余任务清单.md'),
  capabilityBoundary: readText('docs/当前真实能力边界说明.md'),
};

const staticChecks = [
  check(
    'production_c2pa_tsa_injection_points',
    sources.c2paCommand.includes('HIDDENSHIELD_C2PA_SIGN_CERT_PEM') &&
      sources.c2paCommand.includes('HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM') &&
      sources.c2paCommand.includes('HIDDENSHIELD_C2PA_TSA_URL') &&
      sources.c2paCommand.includes('configured_certificate_chain') &&
      sources.c2paCommand.includes('ephemeral_development_certificate_not_publicly_trusted'),
    'Desktop C2PA signer must support production certificate chain/TSA injection and expose signer status.',
  ),
  check(
    'ios_public_rights_v3_runtime_qa_entry',
    sources.packageJson.includes('"rights:ios-public-rights-v3-runtime-qa"') &&
      sources.iosQaScript.includes('ios_public_rights_v3_runtime_qa.dart') &&
      sources.iosQaTool.includes('HIDDENSHIELD_IOS_PUBLIC_RIGHTS_QA_RESULT') &&
      sources.iosQaTool.includes('embedPublicRightsMetadataInImage') &&
      sources.iosQaTool.includes("rights.registry.anchorProtocol == 'v3_minimal_anchor'") &&
      sources.iosQaTool.includes('record.payloadProtocolVersion == 3') &&
      sources.iosQaTool.includes('record.payloadBytesLength == 39'),
    'iOS gate must cover public rights JSON, metadata JSON, embedded image bytes, and default V3/39 write/read.',
  ),
  check(
    'external_sdk_package_ready_not_legal_conclusion',
    sources.sdkPackageJson.includes('@hiddenshield/public-rights-sdk') &&
      sources.sdkPackageJson.includes('"exports"') &&
      sources.sdkReadme.includes('legalConclusion') &&
      sources.sdkReadme.includes('always `false`') &&
      sources.sdkReadme.includes('not published'),
    'SDK package must be externally packageable while preserving non-legal-conclusion boundary before npm publish.',
  ),
  check(
    'completion_gate_script_exposed',
    sources.packageJson.includes('"public-rights:completion-gate"') &&
      sources.remainingTasks.includes('public-rights:completion-gate') &&
      sources.capabilityBoundary.includes('public-rights:completion-gate'),
    'Completion gate must be visible in npm scripts and boundary docs.',
  ),
];

const artifactInputs = {
  c2paTsa: envPath('HIDDENSHIELD_PUBLIC_RIGHTS_C2PA_TSA_VALIDATION_JSON'),
  iosQa: envPath('HIDDENSHIELD_PUBLIC_RIGHTS_IOS_QA_JSON'),
  sdkPublish: envPath('HIDDENSHIELD_PUBLIC_RIGHTS_SDK_NPM_PUBLISH_JSON'),
  releaseSamplePool: envPath('HIDDENSHIELD_PUBLIC_RIGHTS_RELEASE_SAMPLE_POOL_JSON'),
  customerSignoff: envPath('HIDDENSHIELD_PUBLIC_RIGHTS_CUSTOMER_SIGNOFF_JSON'),
};

const artifactChecks = [
  validateArtifact('production_c2pa_tsa', artifactInputs.c2paTsa, validateC2paTsa),
  validateArtifact('ios_public_rights_v3_runtime', artifactInputs.iosQa, validateIosQa),
  validateArtifact('external_sdk_npm_publish', artifactInputs.sdkPublish, validateSdkPublish),
  validateArtifact(
    'release_sample_pool',
    artifactInputs.releaseSamplePool,
    validateReleaseSamplePool,
  ),
  validateArtifact('customer_signoff', artifactInputs.customerSignoff, validateCustomerSignoff),
];

const passedStatic = staticChecks.every((item) => item.pass);
const passedArtifacts = artifactChecks.every((item) => item.pass);
const status = passedStatic && passedArtifacts ? 'passed' : 'blocked';
const result = {
  runId,
  gate: 'public-rights:completion-gate',
  status,
  requireComplete,
  legalConclusion: false,
  staticChecks,
  artifactInputs,
  artifactChecks,
  blockers: [
    ...staticChecks.filter((item) => !item.pass).map((item) => item.id),
    ...artifactChecks.filter((item) => !item.pass).map((item) => item.id),
  ],
  completedAt: new Date().toISOString(),
};
writeFileSync(jsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(mdPath, renderMarkdown(result), 'utf8');

if (status === 'passed') {
  console.log(`Public rights completion gate OK: ${mdPath}`);
} else {
  console.log(`Public rights completion gate BLOCKED: ${mdPath}`);
  if (requireComplete) {
    process.exitCode = 1;
  }
}

function check(id, pass, message) {
  return { id, pass: Boolean(pass), message };
}

function validateArtifact(id, path, validator) {
  if (!path) {
    return {
      id,
      pass: false,
      status: 'missing_env_path',
      message: `Set ${envNameForArtifact(id)} to a JSON evidence artifact.`,
    };
  }
  if (!existsSync(path)) {
    return { id, pass: false, status: 'missing_file', path };
  }
  try {
    const json = readJson(path);
    validator(json);
    return { id, pass: true, status: 'passed', path };
  } catch (error) {
    return { id, pass: false, status: 'invalid_artifact', path, error: String(error.message ?? error) };
  }
}

function validateC2paTsa(json) {
  const signerRows = json.signerRows ?? json.rows ?? [];
  assert(json.status === 'passed', 'C2PA/TSA artifact status must be passed');
  assert(json.legalConclusion === false, 'C2PA/TSA artifact must keep legalConclusion=false');
  assert(Array.isArray(signerRows) && signerRows.length >= 2, 'must include PNG/JPEG signer rows');
  for (const row of signerRows) {
    assert(
      row.c2paSignerStatus === 'configured_certificate_chain',
      'every signer row must use configured_certificate_chain',
    );
    assert(row.hasC2paActiveManifest === true, 'every signer row must have active C2PA manifest');
    assert(isSha256Digest(row.c2paManifestHash), 'manifest hash must be sha256 digest');
  }
}

function validateIosQa(json) {
  const result = json.result ?? json;
  assert(json.status === 'passed' || result.passed === true, 'iOS artifact must pass');
  assert(result.platform === 'ios', 'iOS artifact must declare platform=ios');
  assert(result.publicRightsJsonPass === true, 'public rights JSON check must pass');
  assert(result.publicMetadataJsonPass === true, 'public metadata JSON check must pass');
  assert(result.embeddedImagePass === true, 'embedded image byte check must pass');
  assert(result.v3DefaultWriteReadPass === true, 'V3 default write/read check must pass');
  assert(result.payloadProtocolVersion === 3, 'payloadProtocolVersion must be 3');
  assert(result.payloadBytesLength === 39, 'payloadBytesLength must be 39');
  assert(result.legalConclusion === false, 'iOS artifact must keep legalConclusion=false');
}

function isSha256Digest(value) {
  const text = String(value ?? '').trim();
  return /^[a-f0-9]{64}$/i.test(text) || /^sha256:[a-f0-9]{64}$/i.test(text);
}

function validateSdkPublish(json) {
  assert(json.kind === 'public_rights_sdk_npm_publish_v1', 'SDK artifact kind mismatch');
  assert(json.packageName === '@hiddenshield/public-rights-sdk', 'package name mismatch');
  assert(String(json.registry ?? '').startsWith('https://registry.npmjs.org'), 'registry must be npmjs');
  assert(Boolean(json.version), 'published version is required');
  assert(Boolean(json.publishedAt), 'publishedAt is required');
  assert(json.legalConclusionBoundary === false, 'SDK must not publish a legal conclusion boundary');
  assert(json.canTreatAsTrainingAllowed === false, 'SDK must keep canTreatAsTrainingAllowed=false');
}

function validateReleaseSamplePool(json) {
  assert(json.kind === 'public_rights_release_sample_pool_v1', 'sample pool artifact kind mismatch');
  assert(json.status === 'passed', 'sample pool status must be passed');
  assert(json.legalConclusion === false, 'sample pool must keep legalConclusion=false');
  assert(Number(json.images?.total ?? 0) >= 24, 'image sample pool must include at least 24 samples');
  assert(Number(json.audio?.total ?? 0) >= 12, 'audio sample pool must include at least 12 samples');
  assert(
    Number(json.l1VideoAudioTrack?.total ?? 0) >= 6,
    'L1 video audio track sample pool must include at least 6 samples',
  );
  for (const key of ['images', 'audio', 'l1VideoAudioTrack']) {
    assert(json[key].passed === json[key].total, `${key} must have all samples passed`);
    assert(Number.isFinite(Number(json[key].p95Ms)), `${key} must record p95Ms`);
  }
}

function validateCustomerSignoff(json) {
  assert(json.kind === 'enterprise_customer_signoff_v1', 'customer signoff kind mismatch');
  assert(json.status === 'signed', 'customer signoff must be signed');
  assert(Boolean(json.customerId), 'customerId is required');
  assert(Boolean(json.signedBy), 'signedBy is required');
  assert(Boolean(json.signedAt), 'signedAt is required');
  assert(json.slaAccepted === true, 'SLA must be accepted');
  assert(json.supportContactsAccepted === true, 'support contacts must be accepted');
  assert(json.rollbackWindowAccepted === true, 'rollback window must be accepted');
  assert(json.legalConclusionBoundary === false, 'customer signoff must preserve legalConclusion=false');
}

function envNameForArtifact(id) {
  return {
    production_c2pa_tsa: 'HIDDENSHIELD_PUBLIC_RIGHTS_C2PA_TSA_VALIDATION_JSON',
    ios_public_rights_v3_runtime: 'HIDDENSHIELD_PUBLIC_RIGHTS_IOS_QA_JSON',
    external_sdk_npm_publish: 'HIDDENSHIELD_PUBLIC_RIGHTS_SDK_NPM_PUBLISH_JSON',
    release_sample_pool: 'HIDDENSHIELD_PUBLIC_RIGHTS_RELEASE_SAMPLE_POOL_JSON',
    customer_signoff: 'HIDDENSHIELD_PUBLIC_RIGHTS_CUSTOMER_SIGNOFF_JSON',
  }[id];
}

function renderMarkdown(result) {
  const lines = [
    '# Public Rights Completion Gate',
    '',
    `- runId: \`${result.runId}\``,
    `- status: \`${result.status}\``,
    `- requireComplete: \`${result.requireComplete}\``,
    `- legalConclusion: \`${result.legalConclusion}\``,
    `- completedAt: \`${result.completedAt}\``,
    '',
    '## Static Checks',
    '',
    '| check | result |',
    '| --- | --- |',
  ];
  for (const item of result.staticChecks) {
    lines.push(`| \`${item.id}\` | ${item.pass ? 'PASS' : 'FAIL'} |`);
  }
  lines.push('', '## Artifact Checks', '', '| artifact | status | path / message |', '| --- | --- | --- |');
  for (const item of result.artifactChecks) {
    lines.push(
      `| \`${item.id}\` | ${item.pass ? 'PASS' : 'BLOCKED'} | ${item.path ? `\`${item.path}\`` : item.message ?? item.status} |`,
    );
  }
  if (result.blockers.length) {
    lines.push('', '## Blockers', '', ...result.blockers.map((item) => `- \`${item}\``));
  }
  lines.push(
    '',
    '## Next Step',
    '',
    result.status === 'passed'
      ? 'Public rights protocol completion can be reviewed for production release wording, while still preserving legalConclusion=false.'
      : 'Provide the five real completion artifacts through the documented environment variables and rerun with HIDDENSHIELD_PUBLIC_RIGHTS_REQUIRE_COMPLETE=1.',
    '',
  );
  return `${lines.join('\n')}\n`;
}
