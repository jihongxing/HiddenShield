use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use chrono::{Duration, Utc};
use rsa::pkcs1v15::{Signature as RsaSignature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingPaymentSessionInput {
    pub account_id: String,
    pub workspace_id: String,
    pub plan_code: String,
    pub billing_cycle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPurchasePaymentInput {
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub vault_record_id: String,
    pub product_code: String,
    pub price_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingPaymentSession {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
    pub action: BillingPaymentAction,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingPaymentAction {
    pub action_type: String,
    pub qr_code_url: Option<String>,
    pub h5_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingEventType {
    PaymentSucceeded,
    SubscriptionRenewed,
    PaymentFailed,
    SubscriptionCanceled,
    SubscriptionExpired,
    RefundSucceeded,
}

impl BillingEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PaymentSucceeded => "payment.succeeded",
            Self::SubscriptionRenewed => "subscription.renewed",
            Self::PaymentFailed => "payment.failed",
            Self::SubscriptionCanceled => "subscription.canceled",
            Self::SubscriptionExpired => "subscription.expired",
            Self::RefundSucceeded => "refund.succeeded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingEvent {
    pub provider: String,
    pub provider_event_id: String,
    pub provider_order_id: String,
    pub provider_transaction_id: Option<String>,
    pub account_id: String,
    pub workspace_id: String,
    pub plan_code: String,
    pub billing_cycle: String,
    pub amount_cents: i64,
    pub currency: String,
    pub event_type: BillingEventType,
    pub occurred_at: chrono::DateTime<Utc>,
    pub raw_payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportPurchaseEventType {
    PaymentSucceeded,
    RefundSucceeded,
}

impl ReportPurchaseEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PaymentSucceeded => "payment.succeeded",
            Self::RefundSucceeded => "refund.succeeded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPurchaseEvent {
    pub provider: String,
    pub provider_event_id: String,
    pub provider_order_id: String,
    pub provider_transaction_id: Option<String>,
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub vault_record_id: String,
    pub product_code: String,
    pub price_cents: i64,
    pub currency: String,
    pub event_type: ReportPurchaseEventType,
    pub occurred_at: chrono::DateTime<Utc>,
    pub raw_payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingOrderStatusKind {
    NotFound,
    Pending,
    Succeeded,
    Failed,
    Closed,
    Refunded,
}

impl BillingOrderStatusKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Closed => "closed",
            Self::Refunded => "refunded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingOrderStatus {
    pub provider: String,
    pub provider_order_id: String,
    pub provider_transaction_id: Option<String>,
    pub account_id: String,
    pub workspace_id: String,
    pub plan_code: String,
    pub billing_cycle: String,
    pub amount_cents: i64,
    pub currency: String,
    pub status: BillingOrderStatusKind,
    pub paid_at: Option<chrono::DateTime<Utc>>,
    pub raw_payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPurchaseOrderStatus {
    pub provider: String,
    pub provider_order_id: String,
    pub provider_transaction_id: Option<String>,
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub vault_record_id: String,
    pub product_code: String,
    pub price_cents: i64,
    pub currency: String,
    pub status: BillingOrderStatusKind,
    pub paid_at: Option<chrono::DateTime<Utc>>,
    pub raw_payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WechatPayNormalizedEvent {
    Billing(BillingEvent),
    ReportPurchase(ReportPurchaseEvent),
}

pub trait BillingProvider {
    fn provider(&self) -> &'static str;
    fn create_payment_session(&self, input: &BillingPaymentSessionInput) -> BillingPaymentSession;
}

pub const FIXTURE_PROVIDER: &str = "fixture";
pub const WECHAT_PAY_PROVIDER: &str = "wechat_pay";

#[derive(Debug, Default, Clone, Copy)]
pub struct FixtureBillingProvider;

impl BillingProvider for FixtureBillingProvider {
    fn provider(&self) -> &'static str {
        FIXTURE_PROVIDER
    }

    fn create_payment_session(&self, input: &BillingPaymentSessionInput) -> BillingPaymentSession {
        let order_seed = format!(
            "{}:{}:{}:{}",
            input.account_id, input.workspace_id, input.plan_code, input.billing_cycle
        );
        let order_hash = stable_short_id(&order_seed);
        let expires_at = Utc::now() + Duration::minutes(15);
        BillingPaymentSession {
            payment_session_id: format!("pay_sess_{order_hash}"),
            provider: self.provider().to_string(),
            provider_order_id: format!("fixture_order_{order_hash}"),
            action: BillingPaymentAction {
                action_type: "qr_code".to_string(),
                qr_code_url: Some(format!("fixture://pay/{}/{}", input.plan_code, order_hash)),
                h5_url: None,
            },
            expires_at,
        }
    }
}

impl FixtureBillingProvider {
    pub fn query_order(
        &self,
        session: &BillingPaymentSession,
        input: &BillingPaymentSessionInput,
    ) -> BillingOrderStatus {
        BillingOrderStatus {
            provider: self.provider().to_string(),
            provider_order_id: session.provider_order_id.clone(),
            provider_transaction_id: Some(format!(
                "fixture_txn_{}",
                stable_short_id(&session.provider_order_id)
            )),
            account_id: input.account_id.clone(),
            workspace_id: input.workspace_id.clone(),
            plan_code: input.plan_code.clone(),
            billing_cycle: input.billing_cycle.clone(),
            amount_cents: plan_amount_cents(&input.plan_code, &input.billing_cycle),
            currency: "CNY".to_string(),
            status: BillingOrderStatusKind::Succeeded,
            paid_at: Some(Utc::now()),
            raw_payload_json: format!(
                r#"{{"source":"order_query","provider":"fixture","providerOrderId":"{}","status":"succeeded"}}"#,
                session.provider_order_id
            ),
        }
    }

    pub fn event_for_order_status(&self, status: &BillingOrderStatus) -> Option<BillingEvent> {
        let event_type = match status.status {
            BillingOrderStatusKind::Succeeded => BillingEventType::PaymentSucceeded,
            BillingOrderStatusKind::Refunded => BillingEventType::RefundSucceeded,
            _ => return None,
        };
        Some(BillingEvent {
            provider: self.provider().to_string(),
            provider_event_id: format!(
                "order_query_{}_{}",
                self.provider(),
                stable_short_id(&format!(
                    "{}:{}:{}",
                    status.provider_order_id,
                    status
                        .provider_transaction_id
                        .as_deref()
                        .unwrap_or_default(),
                    event_type.as_str()
                ))
            ),
            provider_order_id: status.provider_order_id.clone(),
            provider_transaction_id: status.provider_transaction_id.clone(),
            account_id: status.account_id.clone(),
            workspace_id: status.workspace_id.clone(),
            plan_code: status.plan_code.clone(),
            billing_cycle: status.billing_cycle.clone(),
            amount_cents: status.amount_cents,
            currency: status.currency.clone(),
            event_type,
            occurred_at: status.paid_at.unwrap_or_else(Utc::now),
            raw_payload_json: status.raw_payload_json.clone(),
        })
    }

    pub fn event_for_session(
        &self,
        session: &BillingPaymentSession,
        input: &BillingPaymentSessionInput,
        event_type: BillingEventType,
    ) -> BillingEvent {
        let provider_event_id = format!(
            "{}_{}_{}",
            self.provider(),
            event_type.as_str().replace('.', "_"),
            stable_short_id(&format!(
                "{}:{}",
                session.provider_order_id,
                event_type.as_str()
            ))
        );
        BillingEvent {
            provider: self.provider().to_string(),
            provider_event_id,
            provider_order_id: session.provider_order_id.clone(),
            provider_transaction_id: Some(format!(
                "fixture_txn_{}",
                stable_short_id(&session.provider_order_id)
            )),
            account_id: input.account_id.clone(),
            workspace_id: input.workspace_id.clone(),
            plan_code: input.plan_code.clone(),
            billing_cycle: input.billing_cycle.clone(),
            amount_cents: plan_amount_cents(&input.plan_code, &input.billing_cycle),
            currency: "CNY".to_string(),
            event_type,
            occurred_at: Utc::now(),
            raw_payload_json: format!(
                r#"{{"provider":"fixture","eventType":"{}","providerOrderId":"{}"}}"#,
                event_type.as_str(),
                session.provider_order_id
            ),
        }
    }
}

pub fn plan_amount_cents(plan_code: &str, billing_cycle: &str) -> i64 {
    match (plan_code, billing_cycle) {
        ("creator", "yearly") => 19900,
        ("creator", _) => 1900,
        ("studio", "yearly") => 69900,
        ("studio", _) => 6900,
        _ => 0,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WechatPayError {
    #[error("wechat config invalid: {0}")]
    Config(String),
    #[error("wechat signature invalid")]
    SignatureInvalid,
    #[error("wechat payload invalid: {0}")]
    Payload(String),
    #[error("wechat resource decrypt failed")]
    DecryptFailed,
    #[error("wechat http failed: {0}")]
    Http(String),
}

#[derive(Debug, Clone)]
pub struct WechatPayConfig {
    pub app_id: String,
    pub mch_id: String,
    pub merchant_serial_no: String,
    pub merchant_private_key_pem: String,
    pub platform_public_key_pem: String,
    pub api_v3_key: String,
    pub notify_url: String,
}

impl WechatPayConfig {
    pub fn from_env() -> Result<Option<Self>, WechatPayError> {
        let app_id = env_var("HIDDENSHIELD_WECHAT_PAY_APP_ID");
        let mch_id = env_var("HIDDENSHIELD_WECHAT_PAY_MCH_ID");
        let merchant_serial_no = env_var("HIDDENSHIELD_WECHAT_PAY_MERCHANT_SERIAL_NO");
        let merchant_private_key_pem = env_var_or_file(
            "HIDDENSHIELD_WECHAT_PAY_MERCHANT_PRIVATE_KEY_PEM",
            "HIDDENSHIELD_WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH",
        )?;
        let platform_public_key_pem = env_var_or_file(
            "HIDDENSHIELD_WECHAT_PAY_PLATFORM_PUBLIC_KEY_PEM",
            "HIDDENSHIELD_WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH",
        )?;
        let api_v3_key = env_var("HIDDENSHIELD_WECHAT_PAY_API_V3_KEY");
        let notify_url = env_var("HIDDENSHIELD_WECHAT_PAY_NOTIFY_URL");

        let values = [
            app_id.as_ref(),
            mch_id.as_ref(),
            merchant_serial_no.as_ref(),
            merchant_private_key_pem.as_ref(),
            platform_public_key_pem.as_ref(),
            api_v3_key.as_ref(),
            notify_url.as_ref(),
        ];
        if values.iter().all(|value| value.is_none()) {
            return Ok(None);
        }
        let config = Self {
            app_id: app_id.ok_or_else(|| WechatPayError::Config("app_id_required".to_string()))?,
            mch_id: mch_id.ok_or_else(|| WechatPayError::Config("mch_id_required".to_string()))?,
            merchant_serial_no: merchant_serial_no
                .ok_or_else(|| WechatPayError::Config("merchant_serial_no_required".to_string()))?,
            merchant_private_key_pem: merchant_private_key_pem.ok_or_else(|| {
                WechatPayError::Config("merchant_private_key_required".to_string())
            })?,
            platform_public_key_pem: platform_public_key_pem.ok_or_else(|| {
                WechatPayError::Config("platform_public_key_required".to_string())
            })?,
            api_v3_key: api_v3_key
                .ok_or_else(|| WechatPayError::Config("api_v3_key_required".to_string()))?,
            notify_url: notify_url
                .ok_or_else(|| WechatPayError::Config("notify_url_required".to_string()))?,
        };
        validate_wechat_config(&config)?;
        Ok(Some(config))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatNativeOrderRequest {
    pub appid: String,
    pub mchid: String,
    pub description: String,
    pub out_trade_no: String,
    pub notify_url: String,
    pub amount: WechatAmount,
    pub attach: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatAmount {
    pub total: i64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatSignedRequest {
    pub method: String,
    pub path: String,
    pub body: String,
    pub authorization: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatNativeOrderResponse {
    pub code_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WechatOrderQueryResponse {
    pub appid: String,
    pub mchid: String,
    pub out_trade_no: String,
    pub transaction_id: Option<String>,
    pub trade_state: String,
    pub success_time: Option<String>,
    pub amount: WechatAmount,
    pub attach: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WechatPayHeaders {
    pub timestamp: String,
    pub nonce: String,
    pub signature: String,
    pub serial: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatPayNotification {
    id: String,
    create_time: String,
    event_type: String,
    resource: WechatEncryptedResource,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatEncryptedResource {
    algorithm: String,
    ciphertext: String,
    nonce: String,
    associated_data: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WechatTransaction {
    appid: String,
    mchid: String,
    out_trade_no: String,
    transaction_id: Option<String>,
    trade_state: String,
    success_time: Option<String>,
    amount: WechatAmount,
    attach: String,
}

#[derive(Debug, Clone)]
pub struct WechatPayNativeAdapter {
    config: WechatPayConfig,
}

impl WechatPayNativeAdapter {
    pub fn new(config: WechatPayConfig) -> Result<Self, WechatPayError> {
        validate_wechat_config(&config)?;
        Ok(Self { config })
    }

    pub fn from_env() -> Result<Option<Self>, WechatPayError> {
        let Some(config) = WechatPayConfig::from_env()? else {
            return Ok(None);
        };
        Ok(Some(Self::new(config)?))
    }

    pub fn build_native_order_request(
        &self,
        input: &BillingPaymentSessionInput,
    ) -> WechatNativeOrderRequest {
        let out_trade_no = format!(
            "hs_{}_{}",
            stable_short_id(&input.account_id),
            stable_short_id(&format!(
                "{}:{}:{}",
                input.workspace_id, input.plan_code, input.billing_cycle
            ))
        );
        let attach = serde_json::json!({
            "accountId": input.account_id,
            "workspaceId": input.workspace_id,
            "planCode": input.plan_code,
            "billingCycle": input.billing_cycle,
        })
        .to_string();
        WechatNativeOrderRequest {
            appid: self.config.app_id.clone(),
            mchid: self.config.mch_id.clone(),
            description: format!("HiddenShield {} {}", input.plan_code, input.billing_cycle),
            out_trade_no,
            notify_url: self.config.notify_url.clone(),
            amount: WechatAmount {
                total: plan_amount_cents(&input.plan_code, &input.billing_cycle),
                currency: "CNY".to_string(),
            },
            attach,
        }
    }

    pub fn build_report_purchase_native_order_request(
        &self,
        input: &ReportPurchasePaymentInput,
    ) -> WechatNativeOrderRequest {
        let out_trade_no = format!(
            "hsr_{}_{}",
            stable_short_id(&input.account_id),
            stable_short_id(&format!(
                "{}:{}:{}",
                input.workspace_id, input.vault_record_id, input.product_code
            ))
        );
        let attach = serde_json::json!({
            "purchaseType": "report_purchase",
            "accountId": input.account_id,
            "workspaceId": input.workspace_id,
            "creatorProfileId": input.creator_profile_id,
            "vaultRecordId": input.vault_record_id,
            "productCode": input.product_code,
        })
        .to_string();
        WechatNativeOrderRequest {
            appid: self.config.app_id.clone(),
            mchid: self.config.mch_id.clone(),
            description: format!("HiddenShield {}", report_product_label(&input.product_code)),
            out_trade_no,
            notify_url: self.config.notify_url.clone(),
            amount: WechatAmount {
                total: input.price_cents,
                currency: "CNY".to_string(),
            },
            attach,
        }
    }

    pub fn build_native_order_http_request(
        &self,
        input: &BillingPaymentSessionInput,
        timestamp: &str,
        nonce: &str,
    ) -> Result<WechatSignedRequest, WechatPayError> {
        let order = self.build_native_order_request(input);
        let body = serde_json::to_string(&order)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        let method = "POST".to_string();
        let path = "/v3/pay/transactions/native".to_string();
        let message = wechat_signing_message(&method, &path, timestamp, nonce, &body);
        let signature = sign_wechat_message(&self.config.merchant_private_key_pem, &message)?;
        let authorization = format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",nonce_str=\"{}\",signature=\"{}\",timestamp=\"{}\",serial_no=\"{}\"",
            self.config.mch_id, nonce, signature, timestamp, self.config.merchant_serial_no
        );
        Ok(WechatSignedRequest {
            method,
            path,
            body,
            authorization,
        })
    }

    pub fn build_report_purchase_native_order_http_request(
        &self,
        input: &ReportPurchasePaymentInput,
        timestamp: &str,
        nonce: &str,
    ) -> Result<WechatSignedRequest, WechatPayError> {
        let order = self.build_report_purchase_native_order_request(input);
        let body = serde_json::to_string(&order)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        let method = "POST".to_string();
        let path = "/v3/pay/transactions/native".to_string();
        let message = wechat_signing_message(&method, &path, timestamp, nonce, &body);
        let signature = sign_wechat_message(&self.config.merchant_private_key_pem, &message)?;
        let authorization = format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",nonce_str=\"{}\",signature=\"{}\",timestamp=\"{}\",serial_no=\"{}\"",
            self.config.mch_id, nonce, signature, timestamp, self.config.merchant_serial_no
        );
        Ok(WechatSignedRequest {
            method,
            path,
            body,
            authorization,
        })
    }

    pub async fn create_native_order(
        &self,
        client: &reqwest::Client,
        input: &BillingPaymentSessionInput,
    ) -> Result<WechatNativeOrderResponse, WechatPayError> {
        let timestamp = Utc::now().timestamp().to_string();
        let nonce = format!(
            "hs{}{}",
            timestamp,
            stable_short_id(&format!("{}{}", input.account_id, input.workspace_id))
        );
        let request = self.build_native_order_http_request(input, &timestamp, &nonce)?;
        let url = format!("https://api.mch.weixin.qq.com{}", request.path);
        let response = client
            .post(url)
            .header("Authorization", request.authorization)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(request.body)
            .send()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        if !status.is_success() {
            return Err(WechatPayError::Http(format!(
                "wechat_native_order_failed:{status}:{body}"
            )));
        }
        serde_json::from_str(&body).map_err(|error| WechatPayError::Payload(error.to_string()))
    }

    pub async fn create_report_purchase_native_order(
        &self,
        client: &reqwest::Client,
        input: &ReportPurchasePaymentInput,
    ) -> Result<WechatNativeOrderResponse, WechatPayError> {
        let timestamp = Utc::now().timestamp().to_string();
        let nonce = format!(
            "hs{}{}",
            timestamp,
            stable_short_id(&format!("{}{}", input.account_id, input.vault_record_id))
        );
        let request =
            self.build_report_purchase_native_order_http_request(input, &timestamp, &nonce)?;
        let url = format!("https://api.mch.weixin.qq.com{}", request.path);
        let response = client
            .post(url)
            .header("Authorization", request.authorization)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(request.body)
            .send()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        if !status.is_success() {
            return Err(WechatPayError::Http(format!(
                "wechat_native_order_failed:{status}:{body}"
            )));
        }
        serde_json::from_str(&body).map_err(|error| WechatPayError::Payload(error.to_string()))
    }

    pub fn build_query_order_http_request(
        &self,
        provider_order_id: &str,
        timestamp: &str,
        nonce: &str,
    ) -> Result<WechatSignedRequest, WechatPayError> {
        let provider_order_id = provider_order_id.trim();
        if provider_order_id.is_empty() {
            return Err(WechatPayError::Payload(
                "provider_order_id_required".to_string(),
            ));
        }
        let method = "GET".to_string();
        let path = format!(
            "/v3/pay/transactions/out-trade-no/{}?mchid={}",
            provider_order_id, self.config.mch_id
        );
        let body = String::new();
        let message = wechat_signing_message(&method, &path, timestamp, nonce, &body);
        let signature = sign_wechat_message(&self.config.merchant_private_key_pem, &message)?;
        let authorization = format!(
            "WECHATPAY2-SHA256-RSA2048 mchid=\"{}\",nonce_str=\"{}\",signature=\"{}\",timestamp=\"{}\",serial_no=\"{}\"",
            self.config.mch_id, nonce, signature, timestamp, self.config.merchant_serial_no
        );
        Ok(WechatSignedRequest {
            method,
            path,
            body,
            authorization,
        })
    }

    pub async fn query_order_by_out_trade_no(
        &self,
        client: &reqwest::Client,
        provider_order_id: &str,
    ) -> Result<BillingOrderStatus, WechatPayError> {
        let timestamp = Utc::now().timestamp().to_string();
        let nonce = format!(
            "hs{}{}",
            timestamp,
            stable_short_id(provider_order_id.trim())
        );
        let request = self.build_query_order_http_request(provider_order_id, &timestamp, &nonce)?;
        let url = format!("https://api.mch.weixin.qq.com{}", request.path);
        let response = client
            .get(url)
            .header("Authorization", request.authorization)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| WechatPayError::Http(error.to_string()))?;
        if !status.is_success() {
            return Err(WechatPayError::Http(format!(
                "wechat_order_query_failed:{status}:{body}"
            )));
        }
        let response: WechatOrderQueryResponse = serde_json::from_str(&body)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        self.order_query_response_to_status(&response, &body)
    }

    pub fn order_query_response_to_status(
        &self,
        response: &WechatOrderQueryResponse,
        raw_body: &str,
    ) -> Result<BillingOrderStatus, WechatPayError> {
        if response.appid != self.config.app_id || response.mchid != self.config.mch_id {
            return Err(WechatPayError::Payload("merchant_mismatch".to_string()));
        }
        if response.amount.currency != "CNY" || response.amount.total <= 0 {
            return Err(WechatPayError::Payload("amount_invalid".to_string()));
        }
        let attach: Value = serde_json::from_str(&response.attach)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        let plan_code = required_json_string(&attach, "planCode")?;
        let billing_cycle = required_json_string(&attach, "billingCycle")?;
        let expected_amount = plan_amount_cents(&plan_code, &billing_cycle);
        if expected_amount <= 0 || expected_amount != response.amount.total {
            return Err(WechatPayError::Payload("amount_mismatch".to_string()));
        }
        let paid_at = response
            .success_time
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc));
        Ok(BillingOrderStatus {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_order_id: response.out_trade_no.clone(),
            provider_transaction_id: response.transaction_id.clone(),
            account_id: required_json_string(&attach, "accountId")?,
            workspace_id: required_json_string(&attach, "workspaceId")?,
            plan_code,
            billing_cycle,
            amount_cents: response.amount.total,
            currency: response.amount.currency.clone(),
            status: wechat_trade_state_to_order_status(&response.trade_state)?,
            paid_at,
            raw_payload_json: raw_body.to_string(),
        })
    }

    pub fn order_query_response_to_report_purchase_status(
        &self,
        response: &WechatOrderQueryResponse,
        raw_body: &str,
    ) -> Result<ReportPurchaseOrderStatus, WechatPayError> {
        if response.appid != self.config.app_id || response.mchid != self.config.mch_id {
            return Err(WechatPayError::Payload("merchant_mismatch".to_string()));
        }
        if response.amount.currency != "CNY" || response.amount.total <= 0 {
            return Err(WechatPayError::Payload("amount_invalid".to_string()));
        }
        let attach: Value = serde_json::from_str(&response.attach)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        if required_json_string(&attach, "purchaseType")? != "report_purchase" {
            return Err(WechatPayError::Payload(
                "purchase_type_mismatch".to_string(),
            ));
        }
        let product_code = required_json_string(&attach, "productCode")?;
        let expected_amount = report_product_price_cents(&product_code)?;
        if expected_amount != response.amount.total {
            return Err(WechatPayError::Payload("amount_mismatch".to_string()));
        }
        let paid_at = response
            .success_time
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc));
        Ok(ReportPurchaseOrderStatus {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_order_id: response.out_trade_no.clone(),
            provider_transaction_id: response.transaction_id.clone(),
            account_id: required_json_string(&attach, "accountId")?,
            workspace_id: required_json_string(&attach, "workspaceId")?,
            creator_profile_id: required_json_string(&attach, "creatorProfileId")?,
            vault_record_id: required_json_string(&attach, "vaultRecordId")?,
            product_code,
            price_cents: response.amount.total,
            currency: response.amount.currency.clone(),
            status: wechat_trade_state_to_order_status(&response.trade_state)?,
            paid_at,
            raw_payload_json: raw_body.to_string(),
        })
    }

    pub fn verify_and_normalize_notification(
        &self,
        headers: &WechatPayHeaders,
        body: &str,
    ) -> Result<WechatPayNormalizedEvent, WechatPayError> {
        if headers.serial.trim().is_empty() {
            return Err(WechatPayError::SignatureInvalid);
        }
        let message = format!("{}\n{}\n{}\n", headers.timestamp, headers.nonce, body);
        verify_wechat_signature(
            &self.config.platform_public_key_pem,
            &message,
            &headers.signature,
        )?;
        let notification: WechatPayNotification = serde_json::from_str(body)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        if notification.resource.algorithm != "AEAD_AES_256_GCM" {
            return Err(WechatPayError::Payload(
                "unsupported_resource_algorithm".to_string(),
            ));
        }
        let plaintext = decrypt_wechat_resource(
            &self.config.api_v3_key,
            &notification.resource.nonce,
            &notification.resource.associated_data,
            &notification.resource.ciphertext,
        )?;
        let transaction: WechatTransaction = serde_json::from_str(&plaintext)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        self.transaction_to_normalized_event(&notification, &transaction, body)
    }

    fn transaction_to_normalized_event(
        &self,
        notification: &WechatPayNotification,
        transaction: &WechatTransaction,
        raw_body: &str,
    ) -> Result<WechatPayNormalizedEvent, WechatPayError> {
        if transaction.appid != self.config.app_id || transaction.mchid != self.config.mch_id {
            return Err(WechatPayError::Payload("merchant_mismatch".to_string()));
        }
        if transaction.amount.currency != "CNY" || transaction.amount.total <= 0 {
            return Err(WechatPayError::Payload("amount_invalid".to_string()));
        }
        let attach: Value = serde_json::from_str(&transaction.attach)
            .map_err(|error| WechatPayError::Payload(error.to_string()))?;
        if attach.get("purchaseType").and_then(Value::as_str) == Some("report_purchase") {
            let event = self.transaction_to_report_purchase_event(
                notification,
                transaction,
                &attach,
                raw_body,
            )?;
            return Ok(WechatPayNormalizedEvent::ReportPurchase(event));
        }
        let plan_code = required_json_string(&attach, "planCode")?;
        let billing_cycle = required_json_string(&attach, "billingCycle")?;
        let expected_amount = plan_amount_cents(&plan_code, &billing_cycle);
        if expected_amount <= 0 || expected_amount != transaction.amount.total {
            return Err(WechatPayError::Payload("amount_mismatch".to_string()));
        }
        if notification.event_type.as_str() != "TRANSACTION.SUCCESS" {
            return Err(WechatPayError::Payload(
                "event_type_unsupported".to_string(),
            ));
        }
        let order_status = wechat_trade_state_to_order_status(&transaction.trade_state)?;
        if order_status != BillingOrderStatusKind::Succeeded {
            return Err(WechatPayError::Payload(
                "trade_state_unsupported".to_string(),
            ));
        }
        let occurred_at = transaction
            .success_time
            .as_deref()
            .or(Some(notification.create_time.as_str()))
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        Ok(WechatPayNormalizedEvent::Billing(BillingEvent {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_event_id: notification.id.clone(),
            provider_order_id: transaction.out_trade_no.clone(),
            provider_transaction_id: transaction.transaction_id.clone(),
            account_id: required_json_string(&attach, "accountId")?,
            workspace_id: required_json_string(&attach, "workspaceId")?,
            plan_code,
            billing_cycle,
            amount_cents: transaction.amount.total,
            currency: transaction.amount.currency.clone(),
            event_type: BillingEventType::PaymentSucceeded,
            occurred_at,
            raw_payload_json: raw_body.to_string(),
        }))
    }

    fn transaction_to_report_purchase_event(
        &self,
        notification: &WechatPayNotification,
        transaction: &WechatTransaction,
        attach: &Value,
        raw_body: &str,
    ) -> Result<ReportPurchaseEvent, WechatPayError> {
        let product_code = required_json_string(attach, "productCode")?;
        let expected_amount = report_product_price_cents(&product_code)?;
        if expected_amount != transaction.amount.total {
            return Err(WechatPayError::Payload("amount_mismatch".to_string()));
        }
        let event_type = match notification.event_type.as_str() {
            "TRANSACTION.SUCCESS" => {
                let order_status = wechat_trade_state_to_order_status(&transaction.trade_state)?;
                if order_status != BillingOrderStatusKind::Succeeded {
                    return Err(WechatPayError::Payload(
                        "trade_state_unsupported".to_string(),
                    ));
                }
                ReportPurchaseEventType::PaymentSucceeded
            }
            "REFUND.SUCCESS" => ReportPurchaseEventType::RefundSucceeded,
            _ => {
                return Err(WechatPayError::Payload(
                    "event_type_unsupported".to_string(),
                ))
            }
        };
        let occurred_at = transaction
            .success_time
            .as_deref()
            .or(Some(notification.create_time.as_str()))
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        Ok(ReportPurchaseEvent {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_event_id: notification.id.clone(),
            provider_order_id: transaction.out_trade_no.clone(),
            provider_transaction_id: transaction.transaction_id.clone(),
            account_id: required_json_string(attach, "accountId")?,
            workspace_id: required_json_string(attach, "workspaceId")?,
            creator_profile_id: required_json_string(attach, "creatorProfileId")?,
            vault_record_id: required_json_string(attach, "vaultRecordId")?,
            product_code,
            price_cents: transaction.amount.total,
            currency: transaction.amount.currency.clone(),
            event_type,
            occurred_at,
            raw_payload_json: raw_body.to_string(),
        })
    }
}

fn report_product_label(product_code: &str) -> &'static str {
    match product_code {
        "rights_evidence_pack_single" => "维权证据包",
        _ => "版权详细报告",
    }
}

fn report_product_price_cents(product_code: &str) -> Result<i64, WechatPayError> {
    match product_code {
        "copyright_report_single" => Ok(1990),
        "rights_evidence_pack_single" => Ok(4990),
        _ => Err(WechatPayError::Payload(
            "report_product_not_allowed".to_string(),
        )),
    }
}

fn wechat_trade_state_to_order_status(
    trade_state: &str,
) -> Result<BillingOrderStatusKind, WechatPayError> {
    match trade_state.trim() {
        "SUCCESS" => Ok(BillingOrderStatusKind::Succeeded),
        "NOTPAY" | "USERPAYING" | "ACCEPT" => Ok(BillingOrderStatusKind::Pending),
        "CLOSED" | "REVOKED" | "PAYERROR" => Ok(BillingOrderStatusKind::Failed),
        "REFUND" => Ok(BillingOrderStatusKind::Refunded),
        "" => Err(WechatPayError::Payload("trade_state_required".to_string())),
        _ => Err(WechatPayError::Payload(
            "trade_state_unsupported".to_string(),
        )),
    }
}

fn validate_wechat_config(config: &WechatPayConfig) -> Result<(), WechatPayError> {
    for (name, value) in [
        ("app_id", &config.app_id),
        ("mch_id", &config.mch_id),
        ("merchant_serial_no", &config.merchant_serial_no),
        ("merchant_private_key_pem", &config.merchant_private_key_pem),
        ("platform_public_key_pem", &config.platform_public_key_pem),
        ("api_v3_key", &config.api_v3_key),
        ("notify_url", &config.notify_url),
    ] {
        if value.trim().is_empty() {
            return Err(WechatPayError::Config(format!("{name}_required")));
        }
    }
    if config.api_v3_key.as_bytes().len() != 32 {
        return Err(WechatPayError::Config(
            "api_v3_key_must_be_32_bytes".to_string(),
        ));
    }
    Ok(())
}

fn wechat_signing_message(
    method: &str,
    path: &str,
    timestamp: &str,
    nonce: &str,
    body: &str,
) -> String {
    format!("{method}\n{path}\n{timestamp}\n{nonce}\n{body}\n")
}

fn sign_wechat_message(private_key_pem: &str, message: &str) -> Result<String, WechatPayError> {
    let key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .map_err(|error| WechatPayError::Config(error.to_string()))?;
    let signing_key = SigningKey::<Sha256>::new(key);
    let signature = signing_key.sign(message.as_bytes());
    Ok(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()))
}

fn verify_wechat_signature(
    public_key_pem: &str,
    message: &str,
    signature_base64: &str,
) -> Result<(), WechatPayError> {
    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|error| WechatPayError::Config(error.to_string()))?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_base64)
        .map_err(|_| WechatPayError::SignatureInvalid)?;
    let signature = RsaSignature::try_from(signature_bytes.as_slice())
        .map_err(|_| WechatPayError::SignatureInvalid)?;
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| WechatPayError::SignatureInvalid)
}

fn decrypt_wechat_resource(
    api_v3_key: &str,
    nonce: &str,
    associated_data: &str,
    ciphertext_base64: &str,
) -> Result<String, WechatPayError> {
    let cipher = Aes256Gcm::new_from_slice(api_v3_key.as_bytes())
        .map_err(|_| WechatPayError::DecryptFailed)?;
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(ciphertext_base64)
        .map_err(|_| WechatPayError::DecryptFailed)?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce.as_bytes()),
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: associated_data.as_bytes(),
            },
        )
        .map_err(|_| WechatPayError::DecryptFailed)?;
    String::from_utf8(plaintext).map_err(|_| WechatPayError::DecryptFailed)
}

fn required_json_string(value: &Value, key: &str) -> Result<String, WechatPayError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| WechatPayError::Payload(format!("{key}_required")))
}

fn env_var(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_var_or_file(value_name: &str, path_name: &str) -> Result<Option<String>, WechatPayError> {
    if let Some(value) = env_var(value_name) {
        return Ok(Some(value.replace("\\n", "\n")));
    }
    let Some(path) = env_var(path_name) else {
        return Ok(None);
    };
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| WechatPayError::Config(format!("{path_name}:{error}")))
}

fn stable_short_id(input: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in input.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::Aead;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};

    fn test_config_with_platform_private_key() -> (WechatPayConfig, String) {
        let mut rng = rand::thread_rng();
        let merchant_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let platform_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let platform_public_key = RsaPublicKey::from(&platform_key);
        let platform_private_key_pem = platform_key
            .to_pkcs8_pem(LineEnding::LF)
            .unwrap()
            .to_string();
        (
            WechatPayConfig {
                app_id: "wx_app_123".to_string(),
                mch_id: "1900000001".to_string(),
                merchant_serial_no: "merchant-serial-1".to_string(),
                merchant_private_key_pem: merchant_key
                    .to_pkcs8_pem(LineEnding::LF)
                    .unwrap()
                    .to_string(),
                platform_public_key_pem: platform_public_key
                    .to_public_key_pem(LineEnding::LF)
                    .unwrap(),
                api_v3_key: "0123456789abcdef0123456789abcdef".to_string(),
                notify_url: "https://api.example.com/v1/billing/webhooks/wechat-pay".to_string(),
            },
            platform_private_key_pem,
        )
    }

    fn test_config() -> WechatPayConfig {
        test_config_with_platform_private_key().0
    }

    fn encrypt_resource(
        api_v3_key: &str,
        nonce: &str,
        associated_data: &str,
        plaintext: &str,
    ) -> String {
        let cipher = Aes256Gcm::new_from_slice(api_v3_key.as_bytes()).unwrap();
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(nonce.as_bytes()),
                aes_gcm::aead::Payload {
                    msg: plaintext.as_bytes(),
                    aad: associated_data.as_bytes(),
                },
            )
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(ciphertext)
    }

    fn sign_with_private_key(private_key_pem: &str, message: &str) -> String {
        let key = RsaPrivateKey::from_pkcs8_pem(private_key_pem).unwrap();
        let signing_key = SigningKey::<Sha256>::new(key);
        let signature = signing_key.sign(message.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
    }

    #[test]
    fn wechat_native_order_request_is_provider_specific_but_billing_neutral() {
        let adapter = WechatPayNativeAdapter::new(test_config()).unwrap();
        let input = BillingPaymentSessionInput {
            account_id: "acct_1".to_string(),
            workspace_id: "ws_1".to_string(),
            plan_code: "creator".to_string(),
            billing_cycle: "monthly".to_string(),
        };

        let order = adapter.build_native_order_request(&input);
        assert_eq!(order.appid, "wx_app_123");
        assert_eq!(order.mchid, "1900000001");
        assert_eq!(order.amount.total, 1900);
        assert_eq!(order.amount.currency, "CNY");
        assert!(order.out_trade_no.starts_with("hs_"));
        assert!(order.attach.contains("\"accountId\":\"acct_1\""));

        let signed = adapter
            .build_native_order_http_request(&input, "1718784000", "nonce-1")
            .unwrap();
        assert_eq!(signed.method, "POST");
        assert_eq!(signed.path, "/v3/pay/transactions/native");
        assert!(signed.authorization.contains("WECHATPAY2-SHA256-RSA2048"));
        assert!(signed.authorization.contains("mchid=\"1900000001\""));
    }

    #[test]
    fn wechat_query_order_request_is_signed_by_out_trade_no() {
        let adapter = WechatPayNativeAdapter::new(test_config()).unwrap();
        let signed = adapter
            .build_query_order_http_request("hs_order_1", "1718784000", "nonce-query")
            .unwrap();

        assert_eq!(signed.method, "GET");
        assert_eq!(
            signed.path,
            "/v3/pay/transactions/out-trade-no/hs_order_1?mchid=1900000001"
        );
        assert_eq!(signed.body, "");
        assert!(signed.authorization.contains("WECHATPAY2-SHA256-RSA2048"));
        assert!(signed.authorization.contains("mchid=\"1900000001\""));
        assert!(signed.authorization.contains("nonce_str=\"nonce-query\""));
    }

    #[test]
    fn wechat_order_query_response_maps_trade_state_and_validates_amount() {
        let config = test_config();
        let adapter = WechatPayNativeAdapter::new(config.clone()).unwrap();
        let response = WechatOrderQueryResponse {
            appid: config.app_id.clone(),
            mchid: config.mch_id.clone(),
            out_trade_no: "hs_order_query_1".to_string(),
            transaction_id: Some("wx_txn_query_1".to_string()),
            trade_state: "SUCCESS".to_string(),
            success_time: Some("2026-06-19T10:00:00+08:00".to_string()),
            amount: WechatAmount {
                total: 1900,
                currency: "CNY".to_string(),
            },
            attach: serde_json::json!({
                "accountId": "acct_1",
                "workspaceId": "ws_1",
                "planCode": "creator",
                "billingCycle": "monthly",
            })
            .to_string(),
        };

        let status = adapter
            .order_query_response_to_status(&response, r#"{"trade_state":"SUCCESS"}"#)
            .unwrap();
        assert_eq!(status.provider, WECHAT_PAY_PROVIDER);
        assert_eq!(status.provider_order_id, "hs_order_query_1");
        assert_eq!(
            status.provider_transaction_id.as_deref(),
            Some("wx_txn_query_1")
        );
        assert_eq!(status.status, BillingOrderStatusKind::Succeeded);
        assert_eq!(status.account_id, "acct_1");
        assert_eq!(status.workspace_id, "ws_1");
        assert_eq!(status.plan_code, "creator");
        assert_eq!(status.billing_cycle, "monthly");
        assert_eq!(status.amount_cents, 1900);
        assert!(status.paid_at.is_some());

        let pending = WechatOrderQueryResponse {
            trade_state: "NOTPAY".to_string(),
            transaction_id: None,
            success_time: None,
            ..response.clone()
        };
        let pending_status = adapter
            .order_query_response_to_status(&pending, r#"{"trade_state":"NOTPAY"}"#)
            .unwrap();
        assert_eq!(pending_status.status, BillingOrderStatusKind::Pending);
        assert_eq!(pending_status.provider_transaction_id, None);
        assert_eq!(pending_status.paid_at, None);

        let wrong_amount = WechatOrderQueryResponse {
            amount: WechatAmount {
                total: 1,
                currency: "CNY".to_string(),
            },
            ..response
        };
        assert!(matches!(
            adapter.order_query_response_to_status(&wrong_amount, "{}"),
            Err(WechatPayError::Payload(message)) if message == "amount_mismatch"
        ));
    }

    #[test]
    fn wechat_report_purchase_order_maps_to_record_grant_status() {
        let config = test_config();
        let adapter = WechatPayNativeAdapter::new(config.clone()).unwrap();
        let input = ReportPurchasePaymentInput {
            account_id: "acct_1".to_string(),
            workspace_id: "ws_1".to_string(),
            creator_profile_id: "creator_1".to_string(),
            vault_record_id: "vault_1".to_string(),
            product_code: "rights_evidence_pack_single".to_string(),
            price_cents: 4990,
        };

        let order = adapter.build_report_purchase_native_order_request(&input);
        assert_eq!(order.amount.total, 4990);
        assert!(order.out_trade_no.starts_with("hsr_"));
        assert!(order
            .attach
            .contains("\"purchaseType\":\"report_purchase\""));
        assert!(order.attach.contains("\"vaultRecordId\":\"vault_1\""));

        let response = WechatOrderQueryResponse {
            appid: config.app_id.clone(),
            mchid: config.mch_id.clone(),
            out_trade_no: order.out_trade_no,
            transaction_id: Some("wx_report_txn_1".to_string()),
            trade_state: "SUCCESS".to_string(),
            success_time: Some("2026-06-19T10:00:00+08:00".to_string()),
            amount: WechatAmount {
                total: 4990,
                currency: "CNY".to_string(),
            },
            attach: order.attach,
        };
        let status = adapter
            .order_query_response_to_report_purchase_status(
                &response,
                r#"{"trade_state":"SUCCESS"}"#,
            )
            .unwrap();

        assert_eq!(status.provider, WECHAT_PAY_PROVIDER);
        assert_eq!(status.status, BillingOrderStatusKind::Succeeded);
        assert_eq!(status.creator_profile_id, "creator_1");
        assert_eq!(status.vault_record_id, "vault_1");
        assert_eq!(status.product_code, "rights_evidence_pack_single");
        assert_eq!(status.price_cents, 4990);
    }

    #[test]
    fn wechat_trade_state_maps_failure_and_refund_without_granting_success() {
        assert_eq!(
            wechat_trade_state_to_order_status("USERPAYING").unwrap(),
            BillingOrderStatusKind::Pending
        );
        assert_eq!(
            wechat_trade_state_to_order_status("CLOSED").unwrap(),
            BillingOrderStatusKind::Failed
        );
        assert_eq!(
            wechat_trade_state_to_order_status("REFUND").unwrap(),
            BillingOrderStatusKind::Refunded
        );
        assert!(matches!(
            wechat_trade_state_to_order_status("UNKNOWN"),
            Err(WechatPayError::Payload(message)) if message == "trade_state_unsupported"
        ));
    }

    #[test]
    fn wechat_notification_verifies_decrypts_and_maps_to_billing_event() {
        let (config, platform_private_key_pem) = test_config_with_platform_private_key();
        let adapter = WechatPayNativeAdapter::new(config.clone()).unwrap();
        let transaction = serde_json::json!({
            "appid": config.app_id,
            "mchid": config.mch_id,
            "out_trade_no": "hs_order_1",
            "transaction_id": "wx_txn_1",
            "trade_state": "SUCCESS",
            "success_time": "2026-06-19T10:00:00+08:00",
            "amount": {"total": 1900, "currency": "CNY"},
            "attach": serde_json::json!({
                "accountId": "acct_1",
                "workspaceId": "ws_1",
                "planCode": "creator",
                "billingCycle": "monthly",
            }).to_string(),
        })
        .to_string();
        let nonce = "notify-nonce";
        let aad = "transaction";
        let ciphertext = encrypt_resource(&config.api_v3_key, nonce, aad, &transaction);
        let body = serde_json::json!({
            "id": "wechat_evt_1",
            "create_time": "2026-06-19T10:00:01+08:00",
            "event_type": "TRANSACTION.SUCCESS",
            "resource": {
                "algorithm": "AEAD_AES_256_GCM",
                "ciphertext": ciphertext,
                "nonce": nonce,
                "associated_data": aad,
            }
        })
        .to_string();
        let timestamp = "1718784000";
        let header_nonce = "header-nonce";
        let message = format!("{timestamp}\n{header_nonce}\n{body}\n");
        let signature = sign_with_private_key(&platform_private_key_pem, &message);

        let mut wrong_signature_headers = WechatPayHeaders {
            timestamp: timestamp.to_string(),
            nonce: header_nonce.to_string(),
            signature: "bad-signature".to_string(),
            serial: "platform-serial-1".to_string(),
        };
        assert!(matches!(
            adapter.verify_and_normalize_notification(&wrong_signature_headers, &body),
            Err(WechatPayError::SignatureInvalid)
        ));

        wrong_signature_headers.signature = signature;
        let normalized = adapter
            .verify_and_normalize_notification(&wrong_signature_headers, &body)
            .unwrap();
        let WechatPayNormalizedEvent::Billing(event) = normalized else {
            panic!("expected billing event");
        };
        assert_eq!(event.provider, WECHAT_PAY_PROVIDER);
        assert_eq!(event.provider_event_id, "wechat_evt_1");
        assert_eq!(event.provider_order_id, "hs_order_1");
        assert_eq!(event.provider_transaction_id.as_deref(), Some("wx_txn_1"));
        assert_eq!(event.account_id, "acct_1");
        assert_eq!(event.workspace_id, "ws_1");
        assert_eq!(event.plan_code, "creator");
        assert_eq!(event.billing_cycle, "monthly");
        assert_eq!(event.amount_cents, 1900);
        assert_eq!(event.currency, "CNY");
        assert_eq!(event.event_type, BillingEventType::PaymentSucceeded);
    }

    #[test]
    fn wechat_notification_rejects_amount_mismatch() {
        let (config, platform_private_key_pem) = test_config_with_platform_private_key();
        let adapter = WechatPayNativeAdapter::new(config.clone()).unwrap();
        let transaction = serde_json::json!({
            "appid": config.app_id,
            "mchid": config.mch_id,
            "out_trade_no": "hs_order_2",
            "transaction_id": "wx_txn_2",
            "trade_state": "SUCCESS",
            "amount": {"total": 1, "currency": "CNY"},
            "attach": serde_json::json!({
                "accountId": "acct_1",
                "workspaceId": "ws_1",
                "planCode": "creator",
                "billingCycle": "monthly",
            }).to_string(),
        })
        .to_string();
        let body = serde_json::json!({
            "id": "wechat_evt_2",
            "create_time": "2026-06-19T10:00:01+08:00",
            "event_type": "TRANSACTION.SUCCESS",
            "resource": {
                "algorithm": "AEAD_AES_256_GCM",
                "ciphertext": encrypt_resource(&config.api_v3_key, "notify-nonce", "transaction", &transaction),
                "nonce": "notify-nonce",
                "associated_data": "transaction",
            }
        })
        .to_string();
        let message = format!("1718784000\nheader-nonce\n{body}\n");
        let headers = WechatPayHeaders {
            timestamp: "1718784000".to_string(),
            nonce: "header-nonce".to_string(),
            signature: sign_with_private_key(&platform_private_key_pem, &message),
            serial: "platform-serial-1".to_string(),
        };
        assert!(matches!(
            adapter.verify_and_normalize_notification(&headers, &body),
            Err(WechatPayError::Payload(message)) if message == "amount_mismatch"
        ));
    }
}
