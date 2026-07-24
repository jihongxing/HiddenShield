import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  design: readFileSync('docs/Enterprise公开扫描API Key与额度账本模型草案.md', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  internalAdminCli: readFileSync('scripts/enterprise-internal-admin.mjs', 'utf8'),
  dryRunRuntimeQa: readFileSync('scripts/verify-enterprise-gateway-dry-run-runtime-qa.mjs', 'utf8'),
  keyIssuanceRuntimeQa: readFileSync('scripts/verify-enterprise-key-issuance-runtime-qa.mjs', 'utf8'),
  commercialCi: readFileSync('scripts/run-commercial-ci.mjs', 'utf8'),
  commercialContract: readFileSync('docs/商业化契约与权益模型.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
};

function includesAny(value, needles) {
  return needles.some((needle) => value.includes(needle));
}

assert(
  sources.design.includes('EnterpriseGatewayAuthContext') &&
    sources.design.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.design.includes('EnterpriseGatewayClientFingerprint') &&
    sources.design.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.design.includes('EnterpriseGatewayAuditContract') &&
    sources.design.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.design.includes('EnterpriseGatewayDryRunRequest') &&
    sources.design.includes('EnterpriseGatewayDryRunDecision') &&
    sources.design.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.design.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.design.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.design.includes('authenticate_api_key') &&
    sources.design.includes('authorize_scope') &&
    sources.design.includes('check_entitlement_api_access') &&
    sources.design.includes('apply_rate_limit') &&
    sources.design.includes('resolve_readonly_public_rights') &&
    sources.design.includes('record_quota_ledger') &&
    sources.design.includes('record_api_audit_event') &&
    sources.design.includes('enterprise_api_closed') &&
    sources.design.includes('quota_contract_missing') &&
    sources.design.includes('api_access_disabled') &&
    sources.design.includes('rate_limited') &&
    sources.design.includes('quota_exhausted') &&
    sources.design.includes('POST /v1/enterprise/public-rights/batch') &&
    sources.design.includes('chargeMetadataExport') &&
    sources.design.includes('chargeOnNotFound') &&
    sources.design.includes('API key 明文签发 / key custody 草案') &&
    sources.design.includes('POST /internal/enterprise/api-key-issuances') &&
    sources.design.includes('POST /internal/enterprise/api-keys/{apiKeyId}/rotate') &&
    sources.design.includes('issue_api_key') &&
    sources.design.includes('rotate_api_key') &&
    sources.design.includes('revoke_expired_rotations') &&
    sources.design.includes('只有后端可信执行环境中的 key custody 服务或内部运维 CLI 可以生成明文 API key') &&
    sources.design.includes('明文 API key 只允许在签发成功响应或受控终端输出中显示一次') &&
    sources.design.includes('keyHash` 推荐保存为带算法和 secret 版本的字符串') &&
    sources.design.includes('轮换不是修改原 key 明文，而是签发一个新的 active key') &&
    sources.design.includes('撤销必须是不可恢复状态变更') &&
    sources.design.includes('不得记录明文 key 或 `keyHash`') &&
    sources.design.includes('所有返回仍是训练许可声明和 registry 状态解释'),
  'enterprise gateway contract design must freeze auth, scope, entitlement, rate limit, quota and audit',
);

assert(
  sources.backendSchema.includes('EnterpriseGatewayAuthContext') &&
    sources.backendSchema.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.backendSchema.includes('EnterpriseGatewayClientFingerprint') &&
    sources.backendSchema.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.backendSchema.includes('EnterpriseGatewayAuditContract') &&
    sources.backendSchema.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.backendSchema.includes('EnterpriseGatewayDryRunRequest') &&
    sources.backendSchema.includes('EnterpriseGatewayDryRunDecision') &&
    sources.backendSchema.includes('ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE') &&
    sources.backendSchema.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.backendSchema.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.backendStorage.includes('enterprise_gateway_readonly_contract_freezes_auth_rate_limit_quota_and_audit') &&
    sources.backendStorage.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_outputs_auth_rate_limit_quota_and_audit_decisions') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_denies_without_charging_or_legal_conclusion') &&
    sources.backendStorage.includes('enterprise_public_rights_batch') &&
    sources.backendStorage.includes('enterprise_public_rights_external_batch_charges_quota_and_audits') &&
    sources.backendStorage.includes('enterprise_rate_limit_windows') &&
    sources.backendStorage.includes('normalize_enterprise_client_fingerprint') &&
    sources.backendStorage.includes('client_fingerprint_hash') &&
    sources.backendStorage.includes('trusted_proxy_status') &&
    sources.backendStorage.includes('record_enterprise_quota_ledger_tx') &&
    sources.backendStorage.includes('record_enterprise_api_audit_event_tx') &&
    sources.backendStorage.includes('legal_conclusion: false') &&
    sources.backendStorage.includes('ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE') &&
    sources.backendStorage.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.backendStorage.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.backendLib.includes('/internal/enterprise/api-keys') &&
    sources.backendLib.includes('/internal/enterprise/api-key-issuances') &&
    sources.backendLib.includes('/internal/enterprise/api-keys/:api_key_id/rotate') &&
    sources.backendLib.includes('/internal/enterprise/api-key-rotations/revoke-expired') &&
    sources.backendLib.includes('/internal/enterprise/quota-balances') &&
    sources.backendLib.includes('/internal/enterprise/admin-audit-events') &&
    sources.backendLib.includes('/internal/enterprise/gateway-dry-run') &&
    sources.backendLib.includes('/v1/enterprise/public-rights/batch') &&
    sources.backendLib.includes('enterprise_public_rights_batch') &&
    sources.backendLib.includes('extract_enterprise_api_key') &&
    sources.backendLib.includes('dry_run_enterprise_gateway_internal') &&
    sources.backendLib.includes('issue_enterprise_api_key_internal') &&
    sources.backendLib.includes('issue_enterprise_api_key_with_custody') &&
    sources.backendLib.includes('rotate_enterprise_api_key_internal') &&
    sources.backendLib.includes('rotate_enterprise_api_key_with_custody') &&
    sources.backendLib.includes('revoke_expired_enterprise_rotations_internal') &&
    sources.backendLib.includes('revoke_expired_enterprise_rotations') &&
    sources.backendLib.includes('HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET') &&
    sources.backendLib.includes('HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET') &&
    sources.backendLib.includes('HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY') &&
    sources.backendLib.includes('extract_enterprise_client_fingerprint') &&
    sources.backendLib.includes('x-hiddenshield-proxy-secret') &&
    sources.backendLib.includes('x-hiddenshield-client-fingerprint') &&
    sources.backendLib.includes('x-forwarded-for') &&
    sources.backendLib.includes('cleartext_api_key') &&
    sources.backendLib.includes('hmac-sha256:v1') &&
    sources.backendLib.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.backendLib.includes('dry_run_gateway') &&
    sources.backendStorage.includes('issue_api_key') &&
    sources.backendStorage.includes('rotate_api_key') &&
    sources.backendStorage.includes('revoke_expired_rotations') &&
    sources.internalAdminCli.includes("case 'dry-run-gateway'") &&
    sources.internalAdminCli.includes("case 'issue-api-key'") &&
    sources.internalAdminCli.includes("case 'rotate-api-key'") &&
    sources.internalAdminCli.includes("case 'revoke-expired-rotations'") &&
    sources.internalAdminCli.includes('/internal/enterprise/api-key-issuances') &&
    sources.internalAdminCli.includes('}/rotate') &&
    sources.internalAdminCli.includes('returns a cleartext API key, and it returns it once') &&
    sources.internalAdminCli.includes('/internal/enterprise/gateway-dry-run') &&
    sources.internalAdminCli.includes('does not write quota ledger') &&
    sources.packageJson.includes('enterprise:gateway-dry-run-runtime-qa') &&
    sources.packageJson.includes('enterprise:key-issuance-runtime-qa') &&
    sources.commercialCi.includes('Enterprise key issuance runtime QA') &&
    sources.commercialCi.includes('Enterprise gateway dry-run runtime QA') &&
    sources.keyIssuanceRuntimeQa.includes("['issue-api-key', '--json'") &&
    sources.keyIssuanceRuntimeQa.includes("'rotate-api-key'") &&
    sources.keyIssuanceRuntimeQa.includes("['list-admin-audit-events', '--operation', 'rotate_api_key'") &&
    sources.keyIssuanceRuntimeQa.includes("'revoke-expired-rotations'") &&
    sources.keyIssuanceRuntimeQa.includes("['list-admin-audit-events', '--operation', 'revoke_expired_rotations'") &&
    sources.keyIssuanceRuntimeQa.includes("['list-admin-audit-events', '--operation', 'issue_api_key'") &&
    sources.keyIssuanceRuntimeQa.includes('followupContainsCleartext') &&
    sources.dryRunRuntimeQa.includes("key: 'success'") &&
    sources.dryRunRuntimeQa.includes("key: 'scope_denied'") &&
    sources.dryRunRuntimeQa.includes("key: 'api_access_disabled'") &&
    sources.dryRunRuntimeQa.includes("key: 'rate_limited'") &&
    sources.dryRunRuntimeQa.includes("key: 'quota_exhausted'") &&
    sources.dryRunRuntimeQa.includes("key: 'api_key_revoked'") &&
    sources.dryRunRuntimeQa.includes("['dry-run-gateway', '--json'") &&
    sources.dryRunRuntimeQa.includes('clientFingerprint') &&
    sources.dryRunRuntimeQa.includes('trusted_proxy_x_hiddenshield_client_fingerprint') &&
    sources.dryRunRuntimeQa.includes("['list-admin-audit-events', '--operation', 'dry_run_gateway'"),
  'backend schema, storage and lib must contain real Enterprise gateway route with key, quota, rate-limit and audit controls',
);

assert(
    includesAny(sources.commercialContract, ['外部 Enterprise API 网关合同、内部 dry-run helper', '外部 Enterprise API 网关合同与内部 dry-run helper', '外部 Enterprise API 网关合同草案']) &&
    sources.commercialContract.includes('EnterpriseGatewayAuthContext') &&
  sources.commercialContract.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.commercialContract.includes('hash-only IP / 指纹限流') &&
    sources.commercialContract.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.commercialContract.includes('EnterpriseGatewayAuditContract') &&
    sources.commercialContract.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.commercialContract.includes('EnterpriseGatewayDryRunRequest') &&
    sources.commercialContract.includes('EnterpriseGatewayDryRunDecision') &&
    sources.commercialContract.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.commercialContract.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.commercialContract.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    includesAny(sources.commercialContract, ['dry-run helper / 测试门禁', 'dry-run helper', '测试门禁']) &&
    sources.commercialContract.includes('/v1/enterprise/...') &&
    includesAny(sources.roadmap, ['外部 Enterprise API 网关合同草案', '外部 Enterprise API 网关合同与内部 dry-run helper', 'Enterprise dry-run 网关校验内部入口和 CLI']) &&
    includesAny(sources.roadmap, ['dry-run helper / 测试门禁', 'dry-run helper', '测试门禁']) &&
    sources.roadmap.includes('/internal/enterprise/gateway-dry-run') &&
    sources.roadmap.includes('dry-run-gateway') &&
    sources.dualRoadmap.includes('外部 Enterprise API') &&
    sources.dualRoadmap.includes('/v1/enterprise/public-rights') &&
    sources.dualRoadmap.includes('/internal/enterprise/gateway-dry-run') &&
    sources.commercialContract.includes('不得表述为法律授权结论'),
  'commercial docs and roadmaps must describe gateway contract and legal boundary',
);

assert(
  sources.commercialContract.includes('已实现受管理员 token 保护的内部 API key 明文签发入口') &&
    sources.commercialContract.includes('明文只在签发响应或 CLI 输出中返回一次') &&
    sources.roadmap.includes('完成真实 Enterprise API key 明文签发 / key custody 流程草案') &&
    sources.roadmap.includes('完成 Enterprise API key 内部明文签发入口') &&
    sources.roadmap.includes('完成 Enterprise API key 内部轮换命令') &&
    sources.roadmap.includes('完成 Enterprise 过期轮换自动撤销内部巡检命令') &&
    sources.roadmap.includes('明文只显示一次') &&
    sources.roadmap.includes('/internal/enterprise/api-key-issuances') &&
    sources.roadmap.includes('/internal/enterprise/api-keys/{apiKeyId}/rotate') &&
    sources.roadmap.includes('/internal/enterprise/api-key-rotations/revoke-expired') &&
    sources.roadmap.includes('issue-api-key') &&
    sources.roadmap.includes('rotate-api-key') &&
    sources.roadmap.includes('revoke-expired-rotations'),
  'Enterprise key custody must have internal issuance, rotation and revoke-expired implementation',
);

assert(
  sources.backendLib.includes('/v1/enterprise/public-rights/batch') &&
    /\.route\(\s*["']\/v1\/enterprise\/public-rights\/batch["']\s*,\s*post\(enterprise_public_rights_batch\)/s.test(
      sources.backendLib,
    ) &&
    !sources.backendLib.includes('route("/v1/enterprise/api-keys') &&
    !sources.backendLib.includes('route("/v1/enterprise/quotas'),
  'enterprise customer scan route may be open, but customer key-management and quota-management routes must remain absent',
);

console.log('Enterprise gateway contract OK');

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
