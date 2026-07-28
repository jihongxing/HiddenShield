use chrono::{DateTime, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::ai_transparency_change_command::{
    ActorAuthorizationDecision, ActorAuthorizationInput, ApprovalReferenceAdapter,
    ApprovalReferenceDecision, ApprovalReferenceInput, ApprovalReferenceType,
    InternalIamAuthorizationAdapter,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderReceiptKind {
    Iam,
    Reference,
}

impl ProviderReceiptKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Iam => "iam",
            Self::Reference => "reference",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignedProviderReceipt {
    pub provider_id: String,
    pub key_id: String,
    pub receipt_id: String,
    pub kind: ProviderReceiptKind,
    pub granted: bool,
    pub status: String,
    pub scope_digest: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct InternalProviderClientConfig {
    pub provider_id: String,
    pub key_id: String,
    pub hmac_secret: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderHealth {
    Healthy,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTransportError {
    Unavailable,
}

pub trait InternalProviderTransport: Send + Sync {
    fn health(&self) -> ProviderHealth;

    fn fetch_iam_receipt(
        &self,
        input: &ActorAuthorizationInput<'_>,
    ) -> Result<SignedProviderReceipt, ProviderTransportError>;

    fn fetch_reference_receipt(
        &self,
        input: &ApprovalReferenceInput<'_>,
    ) -> Result<SignedProviderReceipt, ProviderTransportError>;
}

pub struct ControlledInternalProviderClient<T> {
    transport: T,
    config: InternalProviderClientConfig,
}

impl<T> ControlledInternalProviderClient<T> {
    pub fn new(transport: T, config: InternalProviderClientConfig) -> Self {
        Self { transport, config }
    }
}

impl<T> InternalIamAuthorizationAdapter for ControlledInternalProviderClient<T>
where
    T: InternalProviderTransport,
{
    fn verify_actor_authorization(
        &self,
        input: &ActorAuthorizationInput<'_>,
    ) -> ActorAuthorizationDecision {
        if self.transport.health() != ProviderHealth::Healthy {
            return denied_actor("iam_unavailable");
        }
        let receipt = match self.transport.fetch_iam_receipt(input) {
            Ok(receipt) => receipt,
            Err(ProviderTransportError::Unavailable) => return denied_actor("iam_unavailable"),
        };
        match validate_receipt(
            &receipt,
            &self.config,
            ProviderReceiptKind::Iam,
            &iam_scope_digest(input),
            Utc::now(),
        ) {
            ReceiptValidation::Valid => ActorAuthorizationDecision {
                authorized: true,
                reason_code: None,
                verification_receipt_id: Some(receipt.receipt_id),
            },
            ReceiptValidation::Expired => denied_actor("iam_token_expired"),
            ReceiptValidation::ScopeMismatch => denied_actor("iam_scope_denied"),
            ReceiptValidation::Invalid => denied_actor("iam_token_invalid"),
        }
    }
}

impl<T> ApprovalReferenceAdapter for ControlledInternalProviderClient<T>
where
    T: InternalProviderTransport,
{
    fn verify_approval_reference(
        &self,
        input: &ApprovalReferenceInput<'_>,
    ) -> ApprovalReferenceDecision {
        if self.transport.health() != ProviderHealth::Healthy {
            return denied_reference("reference_unavailable");
        }
        let receipt = match self.transport.fetch_reference_receipt(input) {
            Ok(receipt) => receipt,
            Err(ProviderTransportError::Unavailable) => {
                return denied_reference("reference_unavailable")
            }
        };
        match validate_receipt(
            &receipt,
            &self.config,
            ProviderReceiptKind::Reference,
            &reference_scope_digest(input),
            Utc::now(),
        ) {
            ReceiptValidation::Valid => ApprovalReferenceDecision {
                verified: true,
                reason_code: None,
                verification_receipt_id: Some(receipt.receipt_id),
            },
            ReceiptValidation::Expired => denied_reference("reference_expired"),
            ReceiptValidation::ScopeMismatch => denied_reference("reference_scope_mismatch"),
            ReceiptValidation::Invalid => denied_reference("reference_authority_untrusted"),
        }
    }
}

pub fn iam_scope_digest(input: &ActorAuthorizationInput<'_>) -> String {
    sha256_hex(
        format!(
            "hs-internal-provider-scope-v1|iam|{}|{}|{}|{}|{}|{}",
            input.token_hash,
            input.required_role,
            input.tenant_id,
            input.workspace_id,
            input.environment,
            input.operation
        )
        .as_bytes(),
    )
}

pub fn reference_scope_digest(input: &ApprovalReferenceInput<'_>) -> String {
    sha256_hex(
        format!(
            "hs-internal-provider-scope-v1|reference|{}|{}|{}|{}|{}|{}",
            reference_type_name(input.reference_type),
            input.reference_id,
            input.tenant_id,
            input.workspace_id,
            input.environment,
            input.operation
        )
        .as_bytes(),
    )
}

pub fn sign_provider_receipt(
    config: &InternalProviderClientConfig,
    receipt: &SignedProviderReceipt,
) -> String {
    let mut mac =
        HmacSha256::new_from_slice(&config.hmac_secret).expect("HMAC accepts arbitrary key length");
    mac.update(receipt_payload(receipt).as_bytes());
    hex_lower(&mac.finalize().into_bytes())
}

pub fn new_signed_receipt(
    config: &InternalProviderClientConfig,
    receipt_id: impl Into<String>,
    kind: ProviderReceiptKind,
    scope_digest: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> SignedProviderReceipt {
    let mut receipt = SignedProviderReceipt {
        provider_id: config.provider_id.clone(),
        key_id: config.key_id.clone(),
        receipt_id: receipt_id.into(),
        kind,
        granted: true,
        status: "active".to_string(),
        scope_digest,
        issued_at,
        expires_at,
        signature: String::new(),
    };
    receipt.signature = sign_provider_receipt(config, &receipt);
    receipt
}

enum ReceiptValidation {
    Valid,
    Expired,
    ScopeMismatch,
    Invalid,
}

fn validate_receipt(
    receipt: &SignedProviderReceipt,
    config: &InternalProviderClientConfig,
    expected_kind: ProviderReceiptKind,
    expected_scope_digest: &str,
    now: DateTime<Utc>,
) -> ReceiptValidation {
    if receipt.expires_at <= now || receipt.issued_at > now {
        return ReceiptValidation::Expired;
    }
    if receipt.scope_digest != expected_scope_digest {
        return ReceiptValidation::ScopeMismatch;
    }
    if receipt.provider_id != config.provider_id
        || receipt.key_id != config.key_id
        || receipt.kind != expected_kind
        || !receipt.granted
        || receipt.status != "active"
        || !constant_time_equal(
            receipt.signature.as_bytes(),
            sign_provider_receipt(config, receipt).as_bytes(),
        )
    {
        return ReceiptValidation::Invalid;
    }
    ReceiptValidation::Valid
}

fn denied_actor(reason_code: &str) -> ActorAuthorizationDecision {
    ActorAuthorizationDecision {
        authorized: false,
        reason_code: Some(reason_code.to_string()),
        verification_receipt_id: None,
    }
}

fn denied_reference(reason_code: &str) -> ApprovalReferenceDecision {
    ApprovalReferenceDecision {
        verified: false,
        reason_code: Some(reason_code.to_string()),
        verification_receipt_id: None,
    }
}

fn receipt_payload(receipt: &SignedProviderReceipt) -> String {
    format!(
        "hs-internal-provider-receipt-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.provider_id,
        receipt.key_id,
        receipt.receipt_id,
        receipt.kind.as_str(),
        receipt.granted,
        receipt.status,
        receipt.scope_digest,
        receipt
            .issued_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        receipt
            .expires_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
    )
}

fn reference_type_name(reference_type: ApprovalReferenceType) -> &'static str {
    match reference_type {
        ApprovalReferenceType::Contract => "contract",
        ApprovalReferenceType::LegalReview => "legal_review",
        ApprovalReferenceType::SecurityReview => "security_review",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}
