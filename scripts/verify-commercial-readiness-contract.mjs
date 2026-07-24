import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  checklist: readFileSync('docs/Phase 9 商业化上线验收Checklist.md', 'utf8'),
  releasePlan: readFileSync('docs/双端现有能力发布计划.md', 'utf8'),
  qaRecord: readFileSync('docs/Phase 9 商业化双端QA记录.md', 'utf8'),
  privacyDraft: readFileSync('docs/Phase 9 隐私政策草案.md', 'utf8'),
  termsDraft: readFileSync('docs/Phase 9 用户协议草案.md', 'utf8'),
  billingTermsDraft: readFileSync('docs/Phase 9 支付与订阅条款草案.md', 'utf8'),
  commercialContract: readFileSync('docs/商业化契约与权益模型.md', 'utf8'),
  enterprisePublicRightsApiKeyDesign: readFileSync('docs/Enterprise公开扫描API Key与额度账本模型草案.md', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  enterpriseInternalAdminCli: readFileSync('scripts/enterprise-internal-admin.mjs', 'utf8'),
  desktopApp: readFileSync('src/App.vue', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopWorkspaceContext: readFileSync('src/lib/workspace-context.ts', 'utf8'),
  desktopLegal: readFileSync('src/content/legal.ts', 'utf8'),
  desktopSettings: readFileSync('src/components/SettingsPanel.vue', 'utf8'),
  desktopSubscriptionPanel: readFileSync('src/components/SubscriptionPanel.vue', 'utf8'),
  desktopOfflineLicensePanel: readFileSync('src/components/OfflineLicensePanel.vue', 'utf8'),
  desktopEntitlements: readFileSync('src-tauri/src/entitlements.rs', 'utf8'),
  offlineLicenseDesign: readFileSync('docs/CDKEY离线激活与本地许可证设计.md', 'utf8'),
  mobileSettings: readFileSync('mobile_app/lib/features/settings/settings_page.dart', 'utf8'),
  commercialCi: readFileSync('scripts/run-commercial-ci.mjs', 'utf8'),
};

assert(
  sources.packageJson.includes('"commercial:contract"') &&
    sources.packageJson.includes('"commercial:ci"'),
  'package.json must expose commercial:contract and commercial:ci',
);

assert(
  sources.roadmap.includes('Phase 9 商业化验收 checklist') &&
    sources.roadmap.includes('docs/Phase 9 商业化上线验收Checklist.md') &&
    sources.roadmap.includes('docs/双端现有能力发布计划.md') &&
    sources.roadmap.includes('docs/Phase 9 商业化双端QA记录.md') &&
    sources.roadmap.includes('commercial:contract') &&
    sources.roadmap.includes('commercial:ci'),
  'roadmap must record Phase 9 checklist, QA record, commercial contract, and commercial CI gate',
);

assert(
  sources.releasePlan.includes('状态：发布主线，L3 冻结为内部储备') &&
    sources.releasePlan.includes('图片盲水印写入 / 验证') &&
    sources.releasePlan.includes('音频盲水印写入 / 验证') &&
    sources.releasePlan.includes('移动端保护副本出口') &&
    sources.releasePlan.includes('版权库与存证摘要') &&
    sources.releasePlan.includes('正式云同步') &&
    sources.releasePlan.includes('L1 视频音轨水印') &&
    sources.releasePlan.includes('L2 视频指纹存证') &&
    sources.releasePlan.includes('L3 视频画面盲水印') &&
    sources.releasePlan.includes('本版冻结，不发布') &&
    sources.releasePlan.includes('npm run commercial:ci') &&
    sources.releasePlan.includes('npm run watermark:cross-end-release') &&
    sources.releasePlan.includes('短期不继续执行') &&
    sources.releasePlan.includes('HIDDENSHIELD_L3_FULL_RELEASE_POOL=1') &&
    sources.releasePlan.includes('下一步不继续写 L3 算法'),
  'release plan must freeze short-term L3 work and focus launch scope on current dual-end committed capabilities',
);

assert(
  sources.checklist.includes('Free / Creator / Studio / Enterprise') &&
    sources.checklist.includes('客户端不得自行写入正式权益') &&
    sources.checklist.includes('不默认同步原始图片') &&
    sources.checklist.includes('云端视频画面盲水印仍是未来能力') &&
    sources.checklist.includes('docs/双端现有能力发布计划.md') &&
    sources.checklist.includes('L1 视频音轨水印可以按“视频音轨水印”表达') &&
    sources.checklist.includes('L3 视频画面盲水印不作为本版上线阻断项') &&
    sources.checklist.includes('真实微信商户沙箱 / 生产联调'),
  'checklist must freeze commercial terminology, entitlement authority, media privacy, video boundary, and WeChat联调 boundary',
);

assert(
  sources.checklist.includes('npm run build') &&
    sources.checklist.includes('cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip l3') &&
    sources.checklist.includes('cargo test --manifest-path feedback-backend/Cargo.toml --lib') &&
    sources.checklist.includes('flutter analyze') &&
    sources.checklist.includes('flutter test') &&
    sources.checklist.includes('npm run cloud:ci') &&
    sources.checklist.includes('npm run billing:contract') &&
    sources.checklist.includes('npm run usage:contract') &&
    sources.checklist.includes('npm run report:contract') &&
    sources.checklist.includes('npm run team:contract') &&
    sources.checklist.includes('npm run cloud-video:ci'),
  'checklist must enumerate all current automated commercial readiness gates',
);

assert(
  sources.checklist.includes('桌面端人工验收') &&
    sources.checklist.includes('移动端人工验收') &&
    sources.checklist.includes('后端与支付验收') &&
    sources.checklist.includes('法务与文案验收') &&
    sources.checklist.includes('指标看板验收') &&
    sources.checklist.includes('上线阻断项'),
  'checklist must cover desktop, mobile, backend/payment, legal copy, metrics, and blockers',
);

assert(
  sources.checklist.includes('微信支付商户号') &&
    sources.checklist.includes('可公网访问的 HTTPS 回调域名') &&
    sources.checklist.includes('Native 下单真实返回二维码') &&
    sources.checklist.includes('退款 / 撤销后 entitlement 降级'),
  'checklist must explicitly list user-provided WeChat materials and post-material acceptance checks',
);

assert(
  sources.qaRecord.includes('桌面端订阅页面 | PASS') &&
    sources.qaRecord.includes('移动端订阅页面 | PASS') &&
    sources.qaRecord.includes('权益门禁 | PASS') &&
    sources.qaRecord.includes('本地批量订阅服务 | PASS') &&
    sources.qaRecord.includes('云同步订阅门禁 | PASS') &&
    sources.qaRecord.includes('商业指标看板 | PASS') &&
    sources.qaRecord.includes('微信支付真实联调 | BLOCKED') &&
    sources.qaRecord.includes('法务审阅 | BLOCKED'),
  'QA record must state desktop/mobile commercial QA pass results and remaining blockers',
);

assert(
  sources.qaRecord.includes('不显示“桥接层已接入”和“临时直连”') &&
    sources.qaRecord.includes('客户端不自行写正式权益') &&
    sources.qaRecord.includes('真实微信商户联调') &&
    sources.qaRecord.includes('隐私政策') &&
    sources.qaRecord.includes('HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN'),
  'QA record must cover bridge cleanup, entitlement authority, WeChat联调, legal copy, and metrics production config',
);

assert(
  sources.privacyDraft.includes('默认不同步') &&
    sources.privacyDraft.includes('原始图片') &&
    sources.privacyDraft.includes('加水印后的音频') &&
    sources.privacyDraft.includes('本地文件路径') &&
    sources.privacyDraft.includes('L2 视频能力是“画面指纹存证 / 相似性验证增强”，不是视频画面盲水印') &&
    sources.privacyDraft.includes('客户端点击“确认支付”只触发查单或刷新'),
  'privacy draft must cover media/path privacy, L2 boundary, and payment confirmation boundary',
);

assert(
  sources.termsDraft.includes('不承诺“绝对防盗”') &&
    sources.termsDraft.includes('不构成法律意见、司法鉴定意见或诉讼结果承诺') &&
    sources.termsDraft.includes('默认不同步原始图片') &&
    sources.termsDraft.includes('L2 视频能力是画面指纹存证和相似性验证增强，不是视频画面盲水印'),
  'terms draft must cover copyright protection limits, report legal boundary, sync privacy, and L2 wording',
);

assert(
  sources.billingTermsDraft.includes('本地批量处理是 Creator 订阅权益，不提供 Free 小批量试用') &&
    sources.billingTermsDraft.includes('客户端不保存商户私钥') &&
    sources.billingTermsDraft.includes('确认支付') &&
    sources.billingTermsDraft.includes('不会绕过后端直接开通 Creator / Studio') &&
    sources.billingTermsDraft.includes('在真实微信商户沙箱 / 生产联调完成前，产品不得声明“支付已正式上线”'),
  'billing terms draft must cover subscription gate, provider secrets, payment confirmation, and WeChat launch boundary',
);

assert(
    sources.roadmap.includes('Enterprise 公开扫描 API key / quota ledger 数据模型草案') &&
    sources.roadmap.includes('后端 schema 新增 `enterprise_api_keys`') &&
    sources.roadmap.includes('外部客户侧仅开放 `POST /v1/enterprise/public-rights/batch`') &&
    sources.packageJson.includes('"enterprise:internal-admin"') &&
    sources.commercialContract.includes('public_rights_scan_units') &&
    sources.commercialContract.includes('Enterprise公开扫描API Key与额度账本模型草案.md') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('enterprise_api_keys') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('enterprise_quota_balances') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('enterprise_quota_ledger') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('enterprise_api_audit_events') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('public_rights:batch_read') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('public_rights_scan_units') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('内部只读审计查询入口') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('GET /internal/enterprise/admin-audit-events') &&
    sources.backendSchema.includes('EnterpriseApiKeyCreateRequest') &&
    sources.backendSchema.includes('EnterpriseApiKeyIssueRequest') &&
    sources.backendSchema.includes('EnterpriseApiKeyIssueResponse') &&
    sources.backendSchema.includes('EnterpriseApiKeyRotateRequest') &&
    sources.backendSchema.includes('EnterpriseApiKeyRotateResponse') &&
    sources.backendSchema.includes('EnterpriseExpiredRotationRevokeRequest') &&
    sources.backendSchema.includes('EnterpriseExpiredRotationRevokeResponse') &&
    sources.backendSchema.includes('EnterpriseApiKeyListQuery') &&
    sources.backendSchema.includes('EnterpriseApiKeyStatusChangeRequest') &&
    sources.backendSchema.includes('EnterpriseAdminAuditEventQuery') &&
    sources.backendSchema.includes('EnterpriseAdminAuditEventRecord') &&
    sources.backendSchema.includes('EnterpriseAdminAuditEventListResponse') &&
    sources.backendSchema.includes('EnterpriseQuotaBalanceInitRequest') &&
    sources.backendSchema.includes('EnterpriseQuotaBalanceRecord') &&
    sources.backendSchema.includes('EnterpriseGatewayAuthContext') &&
    sources.backendSchema.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.backendSchema.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.backendSchema.includes('EnterpriseGatewayAuditContract') &&
    sources.backendSchema.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.backendSchema.includes('ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE') &&
    sources.backendSchema.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.backendSchema.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.backendLib.includes('/internal/enterprise/api-keys') &&
    sources.backendLib.includes('/internal/enterprise/api-key-issuances') &&
    sources.backendLib.includes('/internal/enterprise/api-keys/:api_key_id') &&
    sources.backendLib.includes('/internal/enterprise/api-keys/:api_key_id/pause') &&
    sources.backendLib.includes('/internal/enterprise/api-keys/:api_key_id/rotate') &&
    sources.backendLib.includes('/internal/enterprise/api-keys/:api_key_id/revoke') &&
    sources.backendLib.includes('/internal/enterprise/api-key-rotations/revoke-expired') &&
    sources.backendLib.includes('/internal/enterprise/quota-balances') &&
    sources.backendLib.includes('/internal/enterprise/admin-audit-events') &&
    sources.backendLib.includes('create_enterprise_api_key_internal') &&
    sources.backendLib.includes('issue_enterprise_api_key_internal') &&
    sources.backendLib.includes('issue_enterprise_api_key_with_custody') &&
    sources.backendLib.includes('rotate_enterprise_api_key_internal') &&
    sources.backendLib.includes('rotate_enterprise_api_key_with_custody') &&
    sources.backendLib.includes('revoke_expired_enterprise_rotations_internal') &&
    sources.backendLib.includes('revoke_expired_enterprise_rotations') &&
    sources.backendLib.includes('list_enterprise_api_keys_internal') &&
    sources.backendLib.includes('get_enterprise_api_key_internal') &&
    sources.backendLib.includes('pause_enterprise_api_key_internal') &&
    sources.backendLib.includes('revoke_enterprise_api_key_internal') &&
    sources.backendLib.includes('initialize_enterprise_quota_balance_internal') &&
    sources.backendLib.includes('list_enterprise_admin_audit_events_internal') &&
    sources.backendLib.includes('record_enterprise_admin_operation') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_api_keys') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_quota_balances') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_quota_ledger') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_api_audit_events') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS enterprise_admin_audit_events') &&
    sources.backendStorage.includes('idx_enterprise_admin_audit_operation_time') &&
    sources.backendStorage.includes('create_enterprise_api_key_internal') &&
    sources.backendStorage.includes('list_enterprise_api_keys_internal') &&
    sources.backendStorage.includes('get_enterprise_api_key_internal') &&
    sources.backendStorage.includes('pause_enterprise_api_key_internal') &&
    sources.backendStorage.includes('revoke_enterprise_api_key_internal') &&
    sources.backendStorage.includes('initialize_enterprise_quota_balance_internal') &&
    sources.backendStorage.includes('record_enterprise_quota_ledger_internal') &&
    sources.backendStorage.includes('record_enterprise_api_audit_event_internal') &&
    sources.backendStorage.includes('record_enterprise_admin_audit_event_internal') &&
    sources.backendStorage.includes('list_enterprise_admin_audit_events_internal') &&
    sources.backendStorage.includes('enterprise_admin_audit_event_record_from_sql') &&
    sources.backendStorage.includes('enterprise_admin_audit_events_record_specific_internal_operations') &&
    sources.backendStorage.includes('enterprise_admin_audit_events_can_be_filtered_read_only') &&
    sources.backendStorage.includes('ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE') &&
    sources.backendStorage.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.backendStorage.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.backendStorage.includes('create_api_key') &&
    sources.backendStorage.includes('issue_api_key') &&
    sources.backendStorage.includes('rotate_api_key') &&
    sources.backendStorage.includes('revoke_expired_rotations') &&
    sources.backendStorage.includes('list_api_keys') &&
    sources.backendStorage.includes('get_api_key') &&
    sources.backendStorage.includes('pause_api_key') &&
    sources.backendStorage.includes('revoke_api_key') &&
    sources.backendStorage.includes('init_quota_balance') &&
    sources.backendStorage.includes('enterprise_api_key_internal_list_get_pause_and_revoke_work_without_hash_exposure') &&
    sources.backendStorage.includes('enterprise_quota_balance_initialization_is_idempotent_without_resetting_usage') &&
    sources.backendStorage.includes('enterprise_public_rights_internal_models_persist_without_external_api') &&
    sources.backendStorage.includes('enterprise_gateway_readonly_contract_freezes_auth_rate_limit_quota_and_audit') &&
    sources.enterpriseInternalAdminCli.includes("case 'list-api-keys'") &&
    sources.enterpriseInternalAdminCli.includes("case 'issue-api-key'") &&
    sources.enterpriseInternalAdminCli.includes('/internal/enterprise/api-key-issuances') &&
    sources.enterpriseInternalAdminCli.includes("case 'rotate-api-key'") &&
    sources.enterpriseInternalAdminCli.includes('}/rotate') &&
    sources.enterpriseInternalAdminCli.includes("case 'revoke-expired-rotations'") &&
    sources.enterpriseInternalAdminCli.includes('/internal/enterprise/api-key-rotations/revoke-expired') &&
    sources.enterpriseInternalAdminCli.includes("case 'get-api-key'") &&
    sources.enterpriseInternalAdminCli.includes("case 'pause-api-key'") &&
    sources.enterpriseInternalAdminCli.includes("case 'revoke-api-key'") &&
    sources.enterpriseInternalAdminCli.includes("case 'list-admin-audit-events'") &&
    sources.enterpriseInternalAdminCli.includes('/internal/enterprise/admin-audit-events') &&
    !sources.desktopApp.includes('EnterpriseAuditView') &&
    !sources.desktopApp.includes('enterpriseAudit') &&
    !sources.desktopWorkspaceContext.includes('Enterprise 内部管理') &&
    sources.desktopApi.includes('fetchEnterpriseAdminAuditEvents') &&
    sources.desktopApi.includes('/internal/enterprise/admin-audit-events') &&
    sources.desktopApi.includes('EnterpriseAdminAuditEventQuery') &&
    sources.desktopApi.includes('createEnterpriseApiKeyInternal') &&
    sources.desktopApi.includes('listEnterpriseApiKeysInternal') &&
    sources.desktopApi.includes('getEnterpriseApiKeyInternal') &&
    sources.desktopApi.includes('pauseEnterpriseApiKeyInternal') &&
    sources.desktopApi.includes('revokeEnterpriseApiKeyInternal') &&
    sources.desktopApi.includes('initializeEnterpriseQuotaBalanceInternal') &&
    sources.desktopApi.includes('/internal/enterprise/api-keys') &&
    sources.desktopApi.includes('/internal/enterprise/quota-balances') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayAuthContext') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayRateLimitPolicy') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayQuotaChargePlan') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayAuditContract') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayReadOnlyScanContract') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayDryRunRequest') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('EnterpriseGatewayDryRunDecision') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('ENTERPRISE_GATEWAY_REQUIRED_STEPS') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('ENTERPRISE_GATEWAY_STABLE_ERROR_CODES') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('authenticate_api_key') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('apply_rate_limit') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('record_quota_ledger') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('record_api_audit_event') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('rate_limited') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('quota_exhausted') &&
    sources.enterprisePublicRightsApiKeyDesign.includes('api_access_disabled') &&
    sources.backendStorage.includes('dry_run_enterprise_gateway_readonly_scan') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_outputs_auth_rate_limit_quota_and_audit_decisions') &&
    sources.backendStorage.includes('enterprise_gateway_dry_run_helper_denies_without_charging_or_legal_conclusion') &&
    sources.backendLib.includes('/internal/enterprise/gateway-dry-run') &&
    sources.backendLib.includes('dry_run_gateway') &&
    sources.enterpriseInternalAdminCli.includes("case 'dry-run-gateway'") &&
    sources.enterpriseInternalAdminCli.includes('/internal/enterprise/gateway-dry-run') &&
    sources.backendLib.includes('/v1/enterprise/public-rights/batch') &&
    sources.backendStorage.includes('enterprise_public_rights_external_batch_charges_quota_and_audits') &&
    !sources.desktopApi.includes('/v1/enterprise/'),
  'Enterprise public rights infrastructure must remain backend/CLI internal and must not expose a desktop product entry',
);

assert(
  sources.roadmap.includes('当前商业权益矩阵') &&
    sources.roadmap.includes('报告授权必须是记录级 purchase grant') &&
    sources.commercialContract.includes('2026-07-16 冻结权益矩阵') &&
    sources.commercialContract.includes('`report_export` | false | false') &&
    sources.offlineLicenseDesign.includes('HSLIC1 不得直接授权 `report_export`') &&
    sources.desktopEntitlements.includes('features.insert("report_export".to_string(), false)') &&
    sources.backendStorage.includes('"report_export": false') &&
    sources.desktopOfflineLicensePanel.includes('一年期注册码只开放本地批量处理'),
  'current commercialization baseline must freeze annual batch-only entitlement and per-record reports',
);

assert(
  sources.desktopLegal.includes('默认在本机完成图片、音频水印写入与验证') &&
    sources.desktopLegal.includes('当前版本不开放任何视频水印、视频存证或云端视频处理入口') &&
    sources.desktopLegal.includes('不构成法律意见或司法鉴定') &&
    sources.desktopLegal.includes('客户端点击“确认支付”只触发查单或刷新'),
  'desktop legal copy must expose privacy, hidden-video, report, and payment boundaries',
);

assert(
  sources.desktopSettings.includes('隐私政策') &&
    sources.desktopSettings.includes('默认不同步原始图片、加水印图片') &&
    sources.desktopSettings.includes('移动端开发与全部视频能力均已暂停') &&
    sources.desktopSettings.includes('确认支付只触发查单或刷新'),
  'desktop settings must show the current desktop release boundary copy',
);

assert(
  sources.desktopSubscriptionPanel.includes('当前只发布桌面端图片 / 音频与后端云服务') &&
    sources.desktopSubscriptionPanel.includes('未来视频将独立收费') &&
    sources.desktopSubscriptionPanel.includes('未付费和年费已激活用户都必须按记录单独购买正式报告') &&
    sources.desktopSubscriptionPanel.includes('createBillingPaymentSession(plan, "yearly"'),
  'desktop subscription panel must show the current annual base, per-record report, and future-video model',
);

assert(
  sources.mobileSettings.includes("title: '条款与边界'") &&
    sources.mobileSettings.includes('默认不同步原始媒体、加水印媒体和本地文件路径') &&
    sources.mobileSettings.includes('报告、时间戳和指纹存证是技术辅助材料') &&
    sources.mobileSettings.includes('当前是视频指纹存证，不是视频画面盲水印') &&
    sources.mobileSettings.includes('确认支付只触发查单或刷新'),
  'mobile settings and subscription sheet must show Phase 9 legal boundary copy',
);

assert(
  sources.commercialCi.includes("['Commercial readiness contract', 'npm', ['run', 'commercial:contract']]") &&
    sources.commercialCi.includes("['Billing contract', 'npm', ['run', 'billing:contract']]") &&
    sources.commercialCi.includes("['Usage ledger contract', 'npm', ['run', 'usage:contract']]") &&
    sources.commercialCi.includes("['Report export contract', 'npm', ['run', 'report:contract']]") &&
    sources.commercialCi.includes("['Team workspace contract', 'npm', ['run', 'team:contract']]") &&
    sources.commercialCi.includes("['Desktop web build', 'npm', ['run', 'build']]") &&
    sources.commercialCi.includes("['Backend tests', 'cargo', ['test', '--manifest-path', 'feedback-backend/Cargo.toml', '--lib']]") &&
    sources.commercialCi.includes("'Tauri desktop release-scope tests'") &&
    sources.commercialCi.includes("['test', '--manifest-path', 'src-tauri/Cargo.toml', '--lib', '--', '--skip', 'l3']") &&
    sources.commercialCi.includes("['Flutter analyze', 'flutter', ['analyze'], { cwd: 'mobile_app' }]") &&
    sources.commercialCi.includes("['Flutter tests', 'flutter', ['test'], { cwd: 'mobile_app' }]") &&
    sources.commercialCi.includes("['Cloud sync CI', 'npm', ['run', 'cloud:ci']]") &&
    sources.commercialCi.includes("['Cloud video CI', 'npm', ['run', 'cloud-video:ci']]") &&
    sources.commercialCi.indexOf("['Cloud sync CI', 'npm', ['run', 'cloud:ci']]") <
      sources.commercialCi.indexOf("['Cloud video CI', 'npm', ['run', 'cloud-video:ci']]") &&
    sources.commercialCi.includes('run serially to avoid port conflicts'),
  'commercial CI must run all automated readiness gates and keep cloud sync/video CI serial',
);

console.log('Commercial readiness contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Commercial readiness contract failed: ${message}`);
    process.exit(1);
  }
}
