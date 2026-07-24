import { readFileSync } from 'node:fs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function includesAll(source, tokens, label) {
  for (const token of tokens) {
    assert(source.includes(token), `${label} must include ${token}`);
  }
}

const sources = {
  runbook: readFileSync('docs/生产C2PA证书链_TSA_SDK发布_Enterprise客户开通Runbook.md', 'utf8'),
  c2paSecretChecklist: readFileSync('docs/生产C2PA证书申请与Secret注入Checklist.md', 'utf8'),
  protocolDoc: readFileSync('docs/公开权利信号与训练许可扫描协议设计.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  publicMetadataCommand: readFileSync('src-tauri/src/commands/public_metadata.rs', 'utf8'),
  sdkPackageJson: readFileSync('packages/public-rights-sdk/package.json', 'utf8'),
  sdkReadme: readFileSync('packages/public-rights-sdk/README.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  enterpriseDesign: readFileSync('docs/Enterprise公开扫描API Key与额度账本模型草案.md', 'utf8'),
};

includesAll(
  sources.runbook,
  [
    'HIDDENSHIELD_C2PA_SIGN_CERT_PEM',
    'HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM',
    'HIDDENSHIELD_C2PA_SIGNING_ALG',
    'HIDDENSHIELD_C2PA_TSA_URL',
    'ephemeral_development_certificate_not_publicly_trusted',
    'configured_certificate_chain',
    'rights:metadata-embed-runtime-qa',
    'rights:metadata-embed-production-staging-qa',
    'rights:metadata-embed-av-runtime-qa',
    'legalConclusion=false',
    '音视频 C2PA active manifest QA',
    '不得宣称生产可信 C2PA trust chain 已上线',
  ],
  'production C2PA/TSA runbook',
);

includesAll(
  sources.c2paSecretChecklist,
  [
    'CA 选择',
    'CSR',
    '私钥托管',
    'TSA 开通',
    'Secret Manager 注入',
    'HIDDENSHIELD_C2PA_SIGN_CERT_PEM',
    'HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM',
    'HIDDENSHIELD_C2PA_SIGNING_ALG',
    'HIDDENSHIELD_C2PA_TSA_URL',
    'rights:metadata-embed-production-staging-qa',
    'configured_certificate_chain',
    'ephemeral_development_certificate_not_publicly_trusted',
    'legalConclusion=false',
    '不得把 self-signed / ephemeral cert 写成生产可信 C2PA 证书',
  ],
  'production C2PA certificate and secret injection checklist',
);

includesAll(
  sources.runbook,
  [
    'packages/public-rights-sdk',
    'createPublicRightsScanner',
    'scanOne',
    'scanBatch',
    'resolvePolicy',
    'formatUserMessage',
    'rights:sdk-pack-dry-run',
    'dist/index.js',
    'dist/index.d.ts',
    'not published',
    'canTreatAsTrainingAllowed=false',
  ],
  'SDK release runbook',
);

includesAll(
  sources.runbook,
  [
    'POST /v1/enterprise/public-rights/batch',
    'public_rights:batch_read',
    'api_access=true',
    'public_rights_scan_units',
    'quota ledger committed debit',
    'API audit',
    'HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET',
    'HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY',
    'clientFingerprintHash',
    'last used at',
    'ENTERPRISE_GATEWAY_STABLE_ERROR_CODES',
    'pause',
    'revoke',
    'rotate-api-key',
    'revoke-expired-rotations',
    '客户侧 key 管理和 quota 管理路由仍不开放',
  ],
  'Enterprise onboarding runbook',
);

assert(
  sources.publicMetadataCommand.includes('HIDDENSHIELD_C2PA_SIGN_CERT_PEM') &&
    sources.publicMetadataCommand.includes('HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM') &&
    sources.publicMetadataCommand.includes('HIDDENSHIELD_C2PA_SIGNING_ALG') &&
    sources.publicMetadataCommand.includes('HIDDENSHIELD_C2PA_TSA_URL') &&
    sources.publicMetadataCommand.includes('configured_certificate_chain') &&
    sources.publicMetadataCommand.includes('ephemeral_development_certificate_not_publicly_trusted'),
  'desktop C2PA implementation must expose production signer env names and signer status',
);

assert(
  sources.sdkPackageJson.includes('@hiddenshield/public-rights-sdk') &&
    sources.sdkPackageJson.includes('"exports"') &&
    sources.sdkPackageJson.includes('"types"') &&
    sources.sdkReadme.includes('legalConclusion') &&
    sources.sdkReadme.includes('always `false`') &&
    sources.sdkReadme.includes('not published'),
  'SDK package and README must preserve external package and non-legal-conclusion boundaries',
);

assert(
  sources.backendLib.includes('/v1/enterprise/public-rights/batch') &&
    sources.backendLib.includes('enterprise_public_rights_batch') &&
    !sources.backendLib.includes('route("/v1/enterprise/api-keys') &&
    !sources.backendLib.includes('route("/v1/enterprise/quotas') &&
    sources.backendStorage.includes('enterprise_public_rights_external_batch_charges_quota_and_audits') &&
    sources.backendStorage.includes('record_enterprise_quota_ledger_tx') &&
    sources.backendStorage.includes('record_enterprise_api_audit_event_tx') &&
    sources.backendStorage.includes('enterprise_rate_limit_windows'),
  'Enterprise production route must be read-only batch only with quota, rate-limit and audit controls',
);

assert(
  sources.backendLib.includes('HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET') &&
    sources.backendLib.includes('HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY') &&
    sources.backendLib.includes('x-hiddenshield-proxy-secret') &&
    sources.backendLib.includes('x-forwarded-for') &&
    sources.backendStorage.includes('client_fingerprint_hash') &&
    sources.backendStorage.includes('trusted_proxy_status'),
  'Enterprise production route must support trusted proxy hash-only fingerprint rate limiting',
);

includesAll(
  [sources.protocolDoc, sources.commercialRoadmap, sources.capabilityBoundary, sources.runbook].join('\n'),
  [
    '生产 C2PA',
    'TSA',
    'SDK',
    'Enterprise',
    'legalConclusion=false',
  ],
  'project docs',
);

assert(
  sources.enterpriseDesign.includes('POST /v1/enterprise/public-rights/batch') &&
    sources.enterpriseDesign.includes('不得记录明文 key 或 `keyHash`') &&
    sources.enterpriseDesign.includes('所有返回仍是训练许可声明和 registry 状态解释'),
  'Enterprise design must preserve key custody and registry-only output boundaries',
);

assert(
  sources.packageJson.includes('public-rights:production-readiness-contract') &&
    sources.packageJson.includes('rights:sdk-pack-dry-run') &&
    sources.packageJson.includes('rights:metadata-embed-production-staging-qa'),
  'root package.json must expose production readiness, SDK pack dry-run and production metadata staging scripts',
);

console.log('public rights production readiness contract passed');
