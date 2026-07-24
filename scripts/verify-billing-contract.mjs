import { readFileSync } from 'node:fs';

const sources = {
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  design: readFileSync('docs/Phase 8 支付与订阅状态闭环设计.md', 'utf8'),
  freeReportDesign: readFileSync('docs/Phase 8 Free单份报告付费设计.md', 'utf8'),
  wechatOneTimeChecklist: readFileSync('docs/Phase 8 微信一次性商品联调Checklist.md', 'utf8'),
  compensationDesign: readFileSync('docs/Phase 8 支付状态补偿机制设计.md', 'utf8'),
  commercialContract: readFileSync('docs/商业化契约与权益模型.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendBilling: readFileSync('feedback-backend/src/billing.rs', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  desktopSyncCommands: readFileSync('src-tauri/src/commands/sync.rs', 'utf8'),
  desktopTauriApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopSubscriptionPanel: readFileSync('src/components/SubscriptionPanel.vue', 'utf8'),
  desktopBilling: readFileSync('src-tauri/src/db/billing.rs', 'utf8'),
  desktopCloudClient: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  mobileCloudClient: readFileSync('mobile_app/lib/sync/cloud_account_client.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileSettingsPage: readFileSync('mobile_app/lib/features/settings/settings_page.dart', 'utf8'),
};

assert(
  sources.packageJson.includes('"billing:contract"'),
  'package.json must expose billing:contract',
);

assert(
  sources.design.includes('BillingProvider') &&
    sources.design.includes('微信支付 APIv3') &&
    sources.design.includes('billing_source = "wechat_pay"') &&
    sources.design.includes('paymentAction') &&
    sources.design.includes('客户端不保存支付凭证'),
  'Phase 8 design must use a provider abstraction, choose WeChat Pay first, and keep payment credentials off clients',
);

assert(
  sources.design.includes('billing_customers') &&
    sources.design.includes('subscriptions') &&
    sources.design.includes('subscription_events') &&
    sources.design.includes('entitlements') &&
    sources.commercialContract.includes('subscription_events'),
  'Phase 8 design must define billing customer, subscription, subscription_events, and entitlement models',
);

assert(
  sources.design.includes('POST /v1/billing/payment-sessions') &&
    sources.design.includes('POST /v1/billing/portal-sessions') &&
    sources.design.includes('GET /v1/entitlements/current') &&
    sources.design.includes('GET /v1/billing/payment-sessions/{paymentSessionId}') &&
    sources.design.includes('POST /v1/billing/payment-sessions/{paymentSessionId}:reconcile') &&
    sources.design.includes('POST /v1/billing/webhooks/{provider}') &&
    sources.design.includes('POST /v1/billing/webhooks/wechat-pay'),
  'Phase 8 design must define payment session, management, entitlement, and provider webhook APIs',
);

assert(
  sources.compensationDesign.includes('billing_payment_sessions') &&
    sources.compensationDesign.includes('query_order') &&
    sources.compensationDesign.includes('reconcile_pending_payment_sessions') &&
    sources.compensationDesign.includes('POST /v1/billing/payment-sessions/{paymentSessionId}:reconcile') &&
    sources.compensationDesign.includes('GET /v1/billing/payment-sessions/{paymentSessionId}') &&
    sources.compensationDesign.includes('客户端禁止') &&
    sources.compensationDesign.includes('不能根据二维码打开、H5 返回、用户点击“已支付”直接开通 Creator / Studio') &&
    sources.compensationDesign.includes('查单成功后必须复用 `apply_billing_event`') &&
    sources.compensationDesign.includes('微信支付首期查单边界'),
  'Phase 8 compensation design must define payment session ledger, order query, reconcile APIs, backend compensation, and client authority limits',
);

assert(
  sources.freeReportDesign.includes('copyright_report_single') &&
    sources.freeReportDesign.includes('rights_evidence_pack_single') &&
    sources.freeReportDesign.includes('19.9 元 / 份') &&
    sources.freeReportDesign.includes('49.9 元 / 份') &&
    sources.freeReportDesign.includes('一次性报告购买不能复用“订阅权益生效”语义') &&
    sources.freeReportDesign.includes('report_purchase_grants') &&
    sources.freeReportDesign.includes('支付成功后只给对应 `vault_record_id` 授权，不改变用户订阅等级') &&
    sources.commercialContract.includes('Free 单份报告付费不进入 `features`') &&
    sources.commercialContract.includes('`copyright_report_single`') &&
    sources.commercialContract.includes('`rights_evidence_pack_single`') &&
    sources.commercialContract.includes('`report_purchase_grants`') &&
    sources.roadmap.includes('不能直接复用 Creator 订阅 `report_export`'),
  'billing contract must freeze Free one-off report purchase as separate products and grants, not subscription entitlement mutation',
);

assert(
  sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_APP_ID') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_MCH_ID') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_MERCHANT_SERIAL_NO') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_API_V3_KEY') &&
    sources.wechatOneTimeChecklist.includes('HIDDENSHIELD_WECHAT_PAY_NOTIFY_URL') &&
    sources.wechatOneTimeChecklist.includes('/v1/billing/webhooks/wechat-pay') &&
    sources.wechatOneTimeChecklist.includes('purchaseType=report_purchase') &&
    sources.wechatOneTimeChecklist.includes('copyright_report_single') &&
    sources.wechatOneTimeChecklist.includes('rights_evidence_pack_single') &&
    sources.wechatOneTimeChecklist.includes('priceCents=1990') &&
    sources.wechatOneTimeChecklist.includes('priceCents=4990') &&
    sources.wechatOneTimeChecklist.includes('report_purchase_grants') &&
    sources.wechatOneTimeChecklist.includes('report_export=false') &&
    sources.wechatOneTimeChecklist.includes('退款 / 撤销') &&
    sources.wechatOneTimeChecklist.includes('双端授权互认') &&
    sources.wechatOneTimeChecklist.includes('不得把商户私钥、平台公钥、APIv3 key 写入 Git、日志、报告或客户端配置') &&
    sources.wechatOneTimeChecklist.includes('不得对外开启正式购买'),
  'WeChat one-time report purchase checklist must freeze merchant config, webhook path, product prices, grant/revoke gates, double-end QA, and launch blockers',
);

assert(
  sources.design.includes('provider signature') &&
    sources.design.includes('微信支付回调必须校验') &&
    sources.design.includes('provider + provider_event_id') &&
    sources.design.includes('product id / price id') &&
    sources.design.includes('allowlist') &&
    sources.design.includes('重放') &&
    sources.design.includes('幂等'),
  'Phase 8 design must require provider webhook signature verification, event idempotency, and server-side product allowlist',
);

assert(
  sources.design.includes('payment.failed') &&
    sources.design.includes('subscription.expired') &&
    sources.design.includes('payment.succeeded') &&
    sources.design.includes('refund.succeeded') &&
    sources.design.includes('trade_state=SUCCESS') &&
    sources.design.includes('grace_ends_at') &&
    sources.design.includes('expired') &&
    sources.design.includes('降级为 Free feature map'),
  'Phase 8 design must define subscription lifecycle, grace, expired, and refund downgrade behavior',
);

assert(
  sources.design.includes('cloud_sync=false') &&
    sources.design.includes('batch_processing=false') &&
    sources.design.includes('report_export=false') &&
    sources.design.includes('team_workspace=false') &&
    sources.design.includes('cloud_video_processing') &&
    sources.design.includes('服务端校验 feature map'),
  'Phase 8 design must map entitlement changes to paid feature gates and keep server-side enforcement',
);

assert(
  sources.desktopBilling.includes('billing_source') &&
    sources.desktopBilling.includes('subscription_id') &&
    sources.desktopBilling.includes('current_period_ends_at') &&
    sources.desktopBilling.includes('grace_ends_at') &&
    sources.desktopCloudClient.includes('CloudEntitlement'),
  'desktop must already carry entitlement lifecycle fields needed by Phase 8',
);

assert(
  sources.mobileCloudClient.includes('CloudEntitlement') &&
    sources.mobileCloudClient.includes('planCode') &&
    sources.mobileCloudClient.includes('status') &&
    sources.mobileState.includes('EntitlementStatus') &&
    sources.mobileState.includes('entitlementFeatures'),
  'mobile must already carry entitlement lifecycle fields needed by Phase 8',
);

assert(
  sources.backendStorage.includes('entitlement_plan_code') &&
    sources.backendStorage.includes('entitlement_status') &&
    sources.backendStorage.includes('entitlement_features_json') &&
    sources.backendSchema.includes('CloudEntitlement'),
  'backend must already return entitlement snapshots that Phase 8 will update',
);

assert(
    sources.backendBilling.includes('pub trait BillingProvider') &&
    sources.backendBilling.includes('FixtureBillingProvider') &&
    sources.backendBilling.includes('WECHAT_PAY_PROVIDER') &&
    sources.backendBilling.includes('WechatPayNativeAdapter') &&
    sources.backendBilling.includes('HIDDENSHIELD_WECHAT_PAY_APP_ID') &&
    sources.backendBilling.includes('HIDDENSHIELD_WECHAT_PAY_MCH_ID') &&
    sources.backendBilling.includes('create_native_order') &&
    sources.backendBilling.includes('ReportPurchasePaymentInput') &&
    sources.backendBilling.includes('WechatPayNormalizedEvent') &&
    sources.backendBilling.includes('build_report_purchase_native_order_request') &&
    sources.backendBilling.includes('create_report_purchase_native_order') &&
    sources.backendBilling.includes('order_query_response_to_report_purchase_status') &&
    sources.backendBilling.includes('"purchaseType": "report_purchase"') &&
    sources.backendBilling.includes('https://api.mch.weixin.qq.com') &&
    sources.backendBilling.includes('build_native_order_http_request') &&
    sources.backendBilling.includes('build_query_order_http_request') &&
    sources.backendBilling.includes('query_order_by_out_trade_no') &&
    sources.backendBilling.includes('WechatOrderQueryResponse') &&
    sources.backendBilling.includes('order_query_response_to_status') &&
    sources.backendBilling.includes('wechat_trade_state_to_order_status') &&
    sources.backendBilling.includes('verify_and_normalize_notification') &&
    sources.backendBilling.includes('decrypt_wechat_resource') &&
    sources.backendBilling.includes('amount_mismatch') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS billing_customers') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS subscriptions') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS subscription_events') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS entitlements'),
  'backend must expose a provider-neutral billing layer, fixture provider, and Phase 8 billing schema',
);

assert(
    sources.backendLib.includes('/v1/billing/payment-sessions') &&
    sources.backendLib.includes('/v1/billing/payment-sessions/:payment_session_id') &&
    sources.backendLib.includes('/v1/billing/payment-sessions/:payment_session_id/reconcile') &&
    sources.backendLib.includes('/v1/billing/report-purchase-sessions') &&
    sources.backendLib.includes('/v1/billing/report-purchase-sessions/:payment_session_id') &&
    sources.backendLib.includes('/v1/billing/report-purchase-sessions/:payment_session_id/reconcile') &&
    sources.backendLib.includes('/v1/billing/webhooks/fixture') &&
    sources.backendLib.includes('/v1/billing/webhooks/wechat-pay') &&
    sources.backendLib.includes('/v1/entitlements/current') &&
    sources.backendLib.includes('wechat_pay_not_configured') &&
    sources.backendLib.includes('spawn_billing_reconcile_worker') &&
    sources.backendLib.includes('HIDDENSHIELD_BILLING_RECONCILE_INTERVAL_SECS') &&
    sources.backendLib.includes('HIDDENSHIELD_BILLING_RECONCILE_BATCH_SIZE') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS billing_payment_sessions') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS report_purchase_sessions') &&
    sources.backendStorage.includes('CREATE TABLE IF NOT EXISTS report_purchase_grants') &&
    sources.backendStorage.includes('copyright_report_single') &&
    sources.backendStorage.includes('rights_evidence_pack_single') &&
    sources.backendStorage.includes('=> Ok(1990)') &&
    sources.backendStorage.includes('=> Ok(4990)') &&
    sources.backendStorage.includes('create_report_purchase_session') &&
    sources.backendStorage.includes('persist_provider_report_purchase_session') &&
    sources.backendStorage.includes('reconcile_report_purchase_session') &&
    sources.backendStorage.includes('reconcile_report_purchase_order_status') &&
    sources.backendStorage.includes('apply_report_purchase_event') &&
    sources.backendStorage.includes('revoke_report_purchase_grant_tx') &&
    sources.backendStorage.includes('grant_report_purchase_from_payment') &&
    sources.backendStorage.includes('free_report_purchase_grants_single_record_without_upgrading_entitlement') &&
    sources.backendStorage.includes('wechat_report_purchase_order_status_grants_then_refund_revokes_without_entitlement_change') &&
    sources.backendStorage.includes('report_purchase_supports_evidence_pack_price_and_rejects_unknown_product') &&
    sources.desktopCloudClient.includes('create_report_purchase_session') &&
    sources.desktopCloudClient.includes('get_report_purchase_session_status') &&
    sources.desktopCloudClient.includes('reconcile_report_purchase_session') &&
    sources.desktopSyncCommands.includes('CreateReportPurchaseSessionInput') &&
    sources.desktopSyncCommands.includes('persist_report_purchase_grant') &&
    sources.desktopTauriApi.includes('createReportPurchaseSession') &&
    sources.desktopTauriApi.includes('reconcileReportPurchaseSession') &&
    sources.mobileCloudClient.includes('createReportPurchaseSession') &&
    sources.mobileCloudClient.includes('getReportPurchaseSessionStatus') &&
    sources.mobileCloudClient.includes('reconcileReportPurchaseSession') &&
    sources.mobileCloudClient.includes('ReportPurchaseSession') &&
    sources.mobileCloudClient.includes('ReportPurchaseGrant') &&
    sources.mobileState.includes('createReportPurchaseSession') &&
    sources.mobileState.includes('reconcileReportPurchaseSession') &&
    sources.mobileState.includes('reportPurchaseGrantsJson') &&
    sources.mobileState.includes('canExportFormalReportForRecord') &&
    sources.desktopSubscriptionPanel.includes('startPayment') &&
    sources.backendStorage.includes('billing_payment_session_status') &&
    sources.backendStorage.includes('reconcile_billing_payment_session') &&
    sources.backendStorage.includes('reconcile_pending_payment_sessions') &&
    sources.backendStorage.includes('due_payment_sessions_for_provider') &&
    sources.backendStorage.includes('reconcile_billing_order_status') &&
    sources.backendStorage.includes('load_due_billing_payment_sessions') &&
    sources.backendStorage.includes('defer_billing_payment_session_check') &&
    sources.backendStorage.includes('next_billing_payment_check_after') &&
    sources.backendStorage.includes('mark_billing_payment_session_checked_tx') &&
    sources.backendStorage.includes('current_entitlement') &&
    sources.backendStorage.includes('apply_billing_state_transition') &&
    sources.backendStorage.includes('payment.succeeded') &&
    sources.backendStorage.includes('payment.failed') &&
    sources.backendStorage.includes('refund.succeeded') &&
    sources.backendStorage.includes('duplicate'),
  'backend must implement fixture payment sessions, fixture webhook, idempotency, and entitlement state transitions',
);

assert(
  sources.backendBilling.includes('BillingOrderStatus') &&
    sources.backendBilling.includes('ReportPurchaseOrderStatus') &&
    sources.backendBilling.includes('BillingOrderStatusKind') &&
    sources.backendBilling.includes('query_order') &&
    sources.backendBilling.includes('event_for_order_status') &&
    sources.backendBilling.includes('wechat_query_order_request_is_signed_by_out_trade_no') &&
    sources.backendBilling.includes('wechat_order_query_response_maps_trade_state_and_validates_amount') &&
    sources.backendBilling.includes('wechat_report_purchase_order_maps_to_record_grant_status') &&
    sources.backendStorage.includes('fixture_billing_reconcile_recovers_payment_without_webhook') &&
    sources.backendStorage.includes('fixture_billing_background_reconcile_recovers_due_payment_without_webhook') &&
    sources.backendStorage.includes('background_reconcile_skips_wechat_pay_sessions_until_webhook_or_real_query_exists') &&
    sources.backendStorage.includes('wechat_order_status_reconcile_uses_standard_billing_event_path') &&
    sources.backendStorage.includes('WECHAT_PAY_PROVIDER'),
  'backend must implement fixture and WeChat order query compensation, report purchase grant/revoke, background sweep, provider throttle, and prove webhook-missing recovery through standard billing events',
);

assert(
  sources.desktopCloudClient.includes('create_billing_payment_session') &&
    sources.desktopCloudClient.includes('get_billing_payment_session_status') &&
    sources.desktopCloudClient.includes('reconcile_billing_payment_session') &&
    sources.desktopCloudClient.includes('get_current_entitlement') &&
    sources.desktopSyncCommands.includes('CreateBillingPaymentSessionInput') &&
    sources.desktopSyncCommands.includes('BillingPaymentSessionIdInput') &&
    sources.desktopSyncCommands.includes('get_billing_payment_session_status') &&
    sources.desktopSyncCommands.includes('reconcile_billing_payment_session') &&
    sources.desktopSyncCommands.includes('refresh_billing_entitlement') &&
    sources.desktopTauriApi.includes('createBillingPaymentSession') &&
    sources.desktopTauriApi.includes('getBillingPaymentSessionStatus') &&
    sources.desktopTauriApi.includes('reconcileBillingPaymentSession') &&
    sources.desktopTauriApi.includes('refreshBillingEntitlement') &&
    sources.desktopSubscriptionPanel.includes('startPayment') &&
    sources.desktopSubscriptionPanel.includes('startPaymentPolling') &&
    sources.desktopSubscriptionPanel.includes('pollPaymentSession') &&
    sources.desktopSubscriptionPanel.includes('确认支付') &&
    sources.desktopSubscriptionPanel.includes('wechat_pay'),
  'desktop must expose payment session creation, session status, reconcile, and lightweight payment polling from the subscription panel through Tauri',
);

assert(
  sources.mobileCloudClient.includes('createBillingPaymentSession') &&
    sources.mobileCloudClient.includes('getBillingPaymentSessionStatus') &&
    sources.mobileCloudClient.includes('reconcileBillingPaymentSession') &&
    sources.mobileCloudClient.includes('getCurrentEntitlement') &&
    sources.mobileState.includes('createBillingPaymentSession') &&
    sources.mobileState.includes('reconcileLatestPaymentSession') &&
    sources.mobileState.includes('_startPaymentPolling') &&
    sources.mobileState.includes('_pollLatestPaymentSession') &&
    sources.mobileState.includes('refreshBillingEntitlement') &&
    sources.mobileSettingsPage.includes('_startPayment') &&
    sources.mobileSettingsPage.includes('_confirmPayment') &&
    sources.mobileSettingsPage.includes('确认支付') &&
    sources.mobileSettingsPage.includes('支付会话已创建'),
  'mobile must expose payment session creation, session status, reconcile, and lightweight payment polling from the subscription sheet',
);

assert(
  sources.roadmap.includes('Phase 8') &&
    sources.roadmap.includes('支付 provider 抽象层') &&
    sources.roadmap.includes('微信支付') &&
    sources.roadmap.includes('支付 provider') &&
    sources.roadmap.includes('订阅 webhook') &&
    sources.roadmap.includes('entitlement 更新链路') &&
    sources.roadmap.includes('支付状态补偿机制') &&
    sources.roadmap.includes('billing_payment_sessions') &&
    sources.roadmap.includes('fixture `query_order`') &&
    sources.roadmap.includes('双端接入支付会话状态与轻量轮询') &&
    sources.roadmap.includes('确认支付') &&
    sources.roadmap.includes('Phase 8 微信一次性商品联调Checklist.md') &&
    sources.roadmap.includes('billing:contract'),
  'roadmap must record Phase 8 design, payment compensation, double-end polling, and billing contract gates',
);

console.log('Billing contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Billing contract failed: ${message}`);
    process.exit(1);
  }
}
