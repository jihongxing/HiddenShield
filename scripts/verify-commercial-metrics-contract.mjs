import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  metricsDoc: readFileSync('docs/Phase 9 商业指标看板设计.md', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  desktopSettings: readFileSync('src/components/SettingsPanel.vue', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileSettings: readFileSync('mobile_app/lib/features/settings/settings_page.dart', 'utf8'),
  commercialCi: readFileSync('scripts/run-commercial-ci.mjs', 'utf8'),
};

assert(
  sources.packageJson.includes('"commercial:metrics"') &&
    sources.packageJson.includes('verify-commercial-metrics-contract.mjs'),
  'package.json must expose commercial:metrics',
);

assert(
  sources.metricsDoc.includes('GET /v1/commercial/metrics/overview') &&
    sources.metricsDoc.includes('今日新增继续账户数') &&
    sources.metricsDoc.includes('Free / Creator / Studio / Enterprise') &&
    sources.metricsDoc.includes('支付会话') &&
    sources.metricsDoc.includes('本地批量') &&
    sources.metricsDoc.includes('正式报告导出') &&
    sources.metricsDoc.includes('云同步') &&
    sources.metricsDoc.includes('L2 视频指纹存证') &&
    sources.metricsDoc.includes('不采集原始媒体') &&
    sources.metricsDoc.includes('不采集原始媒体、加水印后的媒体、本地文件路径、文件名或完整媒体哈希') &&
    sources.metricsDoc.includes('HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN') &&
    sources.metricsDoc.includes('admin_audit_events') &&
    sources.metricsDoc.includes('未配置管理员 token 时，指标接口默认拒绝访问'),
  'metrics doc must define endpoint, dimensions, privacy boundary, admin auth, and audit log',
);

assert(
  sources.backendSchema.includes('CommercialMetricsOverviewResponse') &&
    sources.backendSchema.includes('CommercialMetricsPrivacyBoundary') &&
    sources.backendSchema.includes('excludes_original_media') &&
    sources.backendSchema.includes('excludes_watermarked_media') &&
    sources.backendSchema.includes('excludes_local_paths') &&
    sources.backendSchema.includes('excludes_file_names') &&
    sources.backendSchema.includes('excludes_full_media_hashes') &&
    sources.backendSchema.includes('CommercialPaymentSessionMetrics') &&
    sources.backendSchema.includes('CommercialAnonymousFailureRow'),
  'backend schema must expose commercial metrics overview and privacy boundary',
);

assert(
  sources.backendStorage.includes('commercial_metrics_overview') &&
    sources.backendStorage.includes('record_admin_audit_event') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS admin_audit_events') &&
    sources.backendStorage.includes('idx_admin_audit_events_endpoint') &&
    sources.backendStorage.includes('cloud_accounts') &&
    sources.backendStorage.includes('billing_payment_sessions') &&
    sources.backendStorage.includes('cloud_usage_ledger') &&
    sources.backendStorage.includes('video_fingerprint_notaries') &&
    sources.backendStorage.includes('feedback_events') &&
    sources.backendStorage.includes('commercial_metrics_overview_aggregates_without_media_identifiers') &&
    sources.backendStorage.includes('excludes_original_media: true') &&
    sources.backendStorage.includes('excludes_full_media_hashes: true'),
  'backend storage must aggregate metrics from existing safe tables and test privacy flags',
);

assert(
  sources.backendLib.includes('/v1/commercial/metrics/overview') &&
    sources.backendLib.includes('get_commercial_metrics_overview') &&
    sources.backendLib.includes('HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN') &&
    sources.backendLib.includes('commercial_metrics_admin_token') &&
    sources.backendLib.includes('validate_commercial_metrics_admin') &&
    sources.backendLib.includes('admin_token_not_configured') &&
    sources.backendLib.includes('admin_token_invalid') &&
    sources.backendLib.includes('record_admin_audit_event') &&
    sources.backendLib.indexOf('validate_commercial_metrics_admin(&state, &headers)?') <
      sources.backendLib.indexOf('state.storage.commercial_metrics_overview()?'),
  'backend router must protect commercial metrics with configured admin token and audit access',
);

assert(
  sources.desktopSettings.includes('商业健康摘要') &&
    sources.desktopSettings.includes('云端看板负责全局账户、支付会话和权益分布') &&
    sources.desktopSettings.includes('本地批量') &&
    sources.desktopSettings.includes('正式报告') &&
    sources.desktopSettings.includes('L2 视频存证') &&
    sources.desktopSettings.includes('不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希'),
  'desktop settings must show commercial health summary with privacy wording',
);

assert(
  sources.mobileState.includes('CommercialHealthSummary') &&
    sources.mobileState.includes('commercialHealthSummary') &&
    sources.mobileState.includes('l2VideoNotaryCount') &&
    sources.mobileState.includes('latestPaymentSessionStatus'),
  'mobile app state must expose the same commercial summary contract',
);

assert(
  sources.mobileSettings.includes('商业健康摘要') &&
    sources.mobileSettings.includes('云端看板负责全局账户、支付会话和权益分布') &&
    sources.mobileSettings.includes('本地批量') &&
    sources.mobileSettings.includes('正式报告') &&
    sources.mobileSettings.includes('L2 视频存证') &&
    sources.mobileSettings.includes('summary.privacyNote') &&
    sources.mobileState.includes('不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希'),
  'mobile settings must show commercial health summary with privacy wording',
);

assert(
  sources.commercialCi.includes("['Commercial metrics contract', 'npm', ['run', 'commercial:metrics']]"),
  'commercial CI must run commercial:metrics',
);

assert(
  sources.roadmap.includes('docs/Phase 9 商业指标看板设计.md') &&
    sources.roadmap.includes('commercial:metrics') &&
    sources.roadmap.includes('指标看板') &&
    sources.roadmap.includes('HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN') &&
    sources.roadmap.includes('管理员 token'),
  'roadmap must record commercial metrics design, contract, and admin-token deployment boundary',
);

console.log('Commercial metrics contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Commercial metrics contract failed: ${message}`);
    process.exit(1);
  }
}
