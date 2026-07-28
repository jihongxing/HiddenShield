use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const AI_DELIVERY_ENVELOPE_SCHEMA_VERSION: &str =
    "hs-ai-confirmed-artifact-delivery-envelope-v1";
pub const AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION: &str =
    "hs-ai-delivery-retrieval-receipt-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeliveryProfileIdentity {
    pub entitlement_version: i32,
    pub entitlement_digest: String,
    pub technical_profile_ids: Vec<String>,
    pub regional_profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiConfirmedArtifactDeliveryEnvelope {
    pub schema_version: String,
    pub delivery_envelope_id: String,
    pub execution_id: String,
    pub marking_session_id: String,
    pub transparency_manifest_id: String,
    pub license_id: String,
    pub watermark_uid: String,
    pub media_type: String,
    pub claim_type: String,
    pub signing_status: String,
    pub artifact_status: String,
    pub recovery_state: String,
    pub worker_recovery_attempts: i32,
    pub recovery_control_version: i32,
    pub final_file_sha256: String,
    pub artifact_ref: String,
    pub artifact_object_version: String,
    pub signer_receipt_id: String,
    pub signer_receipt_sha256: String,
    pub artifact_finalize_receipt_id: String,
    pub artifact_finalize_receipt_sha256: String,
    pub profile_identity: AiDeliveryProfileIdentity,
    pub profile_identity_digest: String,
    pub finalized_at: String,
    pub envelope_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeliveryEnvelopeValidationResult {
    pub accepted: bool,
    pub envelope_digest: String,
    pub final_file_sha256: String,
    pub watermark_uid: String,
    pub profile_identity_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeliveryRetrievalReceipt {
    pub schema_version: String,
    pub retrieval_receipt_id: String,
    pub authorization_id: String,
    pub delivery_envelope_id: String,
    pub execution_id: String,
    pub envelope_digest: String,
    pub final_file_sha256: String,
    pub artifact_finalize_receipt_sha256: String,
    pub retrieved_at: String,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeliveryImportAdmission {
    pub admitted: bool,
    pub authorization_id: String,
    pub retrieval_receipt_id: String,
    pub envelope_digest: String,
    pub final_file_sha256: String,
    pub watermark_uid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDeliveryEnvelopeErrorCode {
    InvalidContract,
    InvalidSchemaVersion,
    SigningNotConfirmed,
    ArtifactNotFinalized,
    RecoveryNotCompleted,
    FinalFileHashMismatch,
    SignerReceiptHashMismatch,
    ArtifactFinalizeReceiptHashMismatch,
    SignerReceiptBindingMismatch,
    ArtifactFinalizeReceiptBindingMismatch,
    ProfileIdentityDigestMismatch,
    EnvelopeDigestMismatch,
    RetrievalReceiptMismatch,
}

impl AiDeliveryEnvelopeErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidContract => "ai_delivery_envelope_invalid_contract",
            Self::InvalidSchemaVersion => "ai_delivery_envelope_schema_version_invalid",
            Self::SigningNotConfirmed => "ai_delivery_envelope_signing_not_confirmed",
            Self::ArtifactNotFinalized => "ai_delivery_envelope_artifact_not_finalized",
            Self::RecoveryNotCompleted => "ai_delivery_envelope_recovery_not_completed",
            Self::FinalFileHashMismatch => "ai_delivery_envelope_final_file_hash_mismatch",
            Self::SignerReceiptHashMismatch => "ai_delivery_envelope_signer_receipt_hash_mismatch",
            Self::ArtifactFinalizeReceiptHashMismatch => {
                "ai_delivery_envelope_finalize_receipt_hash_mismatch"
            }
            Self::SignerReceiptBindingMismatch => {
                "ai_delivery_envelope_signer_receipt_binding_mismatch"
            }
            Self::ArtifactFinalizeReceiptBindingMismatch => {
                "ai_delivery_envelope_finalize_receipt_binding_mismatch"
            }
            Self::ProfileIdentityDigestMismatch => {
                "ai_delivery_envelope_profile_identity_digest_mismatch"
            }
            Self::EnvelopeDigestMismatch => "ai_delivery_envelope_digest_mismatch",
            Self::RetrievalReceiptMismatch => "ai_delivery_retrieval_receipt_mismatch",
        }
    }
}

pub fn seal_ai_delivery_retrieval_receipt(
    mut receipt: AiDeliveryRetrievalReceipt,
) -> Result<AiDeliveryRetrievalReceipt, AiDeliveryEnvelopeError> {
    validate_retrieval_receipt_static(&receipt)?;
    receipt.receipt_digest = ai_delivery_retrieval_receipt_digest(&receipt);
    Ok(receipt)
}

pub fn validate_ai_delivery_import(
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
    final_media_bytes: &[u8],
    signer_receipt_json: &str,
    artifact_finalize_receipt_json: &str,
    retrieval_receipt: &AiDeliveryRetrievalReceipt,
) -> Result<AiDeliveryImportAdmission, AiDeliveryEnvelopeError> {
    let validated = validate_ai_delivery_envelope(
        envelope,
        final_media_bytes,
        signer_receipt_json,
        artifact_finalize_receipt_json,
    )?;
    validate_retrieval_receipt_static(retrieval_receipt)?;
    if retrieval_receipt.delivery_envelope_id != envelope.delivery_envelope_id
        || retrieval_receipt.execution_id != envelope.execution_id
        || retrieval_receipt.envelope_digest != envelope.envelope_digest
        || retrieval_receipt.final_file_sha256 != envelope.final_file_sha256
        || retrieval_receipt.artifact_finalize_receipt_sha256
            != envelope.artifact_finalize_receipt_sha256
        || ai_delivery_retrieval_receipt_digest(retrieval_receipt)
            != retrieval_receipt.receipt_digest
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::RetrievalReceiptMismatch,
            "retrieval receipt does not bind to the verified delivery envelope",
        ));
    }
    Ok(AiDeliveryImportAdmission {
        admitted: true,
        authorization_id: retrieval_receipt.authorization_id.clone(),
        retrieval_receipt_id: retrieval_receipt.retrieval_receipt_id.clone(),
        envelope_digest: validated.envelope_digest,
        final_file_sha256: validated.final_file_sha256,
        watermark_uid: validated.watermark_uid,
    })
}

pub fn ai_delivery_retrieval_receipt_digest(receipt: &AiDeliveryRetrievalReceipt) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            receipt.schema_version,
            receipt.retrieval_receipt_id,
            receipt.authorization_id,
            receipt.delivery_envelope_id,
            receipt.execution_id,
            receipt.envelope_digest,
            receipt.final_file_sha256,
            receipt.artifact_finalize_receipt_sha256,
            receipt.retrieved_at
        ]))
        .expect("serializable delivery retrieval receipt")
        .as_bytes(),
    )
}

#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AiDeliveryEnvelopeError {
    pub code: &'static str,
    pub message: String,
}

impl AiDeliveryEnvelopeError {
    fn new(code: AiDeliveryEnvelopeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str(),
            message: message.into(),
        }
    }
}

pub fn seal_ai_delivery_envelope(
    mut envelope: AiConfirmedArtifactDeliveryEnvelope,
) -> Result<AiConfirmedArtifactDeliveryEnvelope, AiDeliveryEnvelopeError> {
    validate_static_contract(&envelope)?;
    envelope.profile_identity_digest =
        ai_delivery_profile_identity_digest(&envelope.profile_identity);
    envelope.envelope_digest = ai_delivery_envelope_digest(&envelope);
    Ok(envelope)
}

pub fn validate_ai_delivery_envelope(
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
    final_media_bytes: &[u8],
    signer_receipt_json: &str,
    artifact_finalize_receipt_json: &str,
) -> Result<AiDeliveryEnvelopeValidationResult, AiDeliveryEnvelopeError> {
    validate_static_contract(envelope)?;
    if envelope.signing_status != "confirmed" {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::SigningNotConfirmed,
            "signing status must be confirmed",
        ));
    }
    if envelope.artifact_status != "finalized" {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::ArtifactNotFinalized,
            "artifact status must be finalized",
        ));
    }
    if envelope.recovery_state != "completed" {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::RecoveryNotCompleted,
            "recovery state must be completed",
        ));
    }
    if sha256_hex(final_media_bytes) != envelope.final_file_sha256 {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::FinalFileHashMismatch,
            "final media bytes do not match the envelope",
        ));
    }
    let signer_receipt = canonical_json(signer_receipt_json)?;
    if sha256_hex(&signer_receipt) != envelope.signer_receipt_sha256 {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::SignerReceiptHashMismatch,
            "signer receipt digest does not match the envelope",
        ));
    }
    let finalize_receipt = canonical_json(artifact_finalize_receipt_json)?;
    if sha256_hex(&finalize_receipt) != envelope.artifact_finalize_receipt_sha256 {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::ArtifactFinalizeReceiptHashMismatch,
            "artifact finalize receipt digest does not match the envelope",
        ));
    }
    let signer_value: Value = serde_json::from_slice(&signer_receipt).map_err(|error| {
        AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidContract,
            format!("parse canonical signer receipt: {error}"),
        )
    })?;
    validate_signer_receipt_binding(envelope, &signer_value)?;
    let finalize_value: Value = serde_json::from_slice(&finalize_receipt).map_err(|error| {
        AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidContract,
            format!("parse canonical finalize receipt: {error}"),
        )
    })?;
    validate_finalize_receipt_binding(envelope, &finalize_value)?;
    if ai_delivery_profile_identity_digest(&envelope.profile_identity)
        != envelope.profile_identity_digest
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::ProfileIdentityDigestMismatch,
            "profile identity digest does not match the envelope",
        ));
    }
    if ai_delivery_envelope_digest(envelope) != envelope.envelope_digest {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::EnvelopeDigestMismatch,
            "envelope digest does not match the canonical envelope",
        ));
    }
    Ok(AiDeliveryEnvelopeValidationResult {
        accepted: true,
        envelope_digest: envelope.envelope_digest.clone(),
        final_file_sha256: envelope.final_file_sha256.clone(),
        watermark_uid: envelope.watermark_uid.clone(),
        profile_identity_digest: envelope.profile_identity_digest.clone(),
    })
}

pub fn ai_delivery_profile_identity_digest(profile: &AiDeliveryProfileIdentity) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            profile.entitlement_version,
            profile.entitlement_digest,
            profile.technical_profile_ids,
            profile.regional_profile_id
        ]))
        .expect("serializable delivery profile identity")
        .as_bytes(),
    )
}

pub fn ai_delivery_envelope_digest(envelope: &AiConfirmedArtifactDeliveryEnvelope) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            envelope.schema_version,
            envelope.delivery_envelope_id,
            envelope.execution_id,
            envelope.marking_session_id,
            envelope.transparency_manifest_id,
            envelope.license_id,
            envelope.watermark_uid,
            envelope.media_type,
            envelope.claim_type,
            envelope.signing_status,
            envelope.artifact_status,
            envelope.recovery_state,
            envelope.worker_recovery_attempts,
            envelope.recovery_control_version,
            envelope.final_file_sha256,
            envelope.artifact_ref,
            envelope.artifact_object_version,
            envelope.signer_receipt_id,
            envelope.signer_receipt_sha256,
            envelope.artifact_finalize_receipt_id,
            envelope.artifact_finalize_receipt_sha256,
            envelope.profile_identity.entitlement_version,
            envelope.profile_identity.entitlement_digest,
            envelope.profile_identity.technical_profile_ids,
            envelope.profile_identity.regional_profile_id,
            envelope.profile_identity_digest,
            envelope.finalized_at
        ]))
        .expect("serializable delivery envelope")
        .as_bytes(),
    )
}

pub fn canonical_json_sha256(json_text: &str) -> Result<String, AiDeliveryEnvelopeError> {
    canonical_json(json_text).map(|bytes| sha256_hex(&bytes))
}

fn validate_static_contract(
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
) -> Result<(), AiDeliveryEnvelopeError> {
    if envelope.schema_version != AI_DELIVERY_ENVELOPE_SCHEMA_VERSION {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidSchemaVersion,
            "unsupported delivery envelope schema version",
        ));
    }
    if envelope.delivery_envelope_id.is_empty()
        || envelope.execution_id.is_empty()
        || envelope.marking_session_id.is_empty()
        || envelope.transparency_manifest_id.is_empty()
        || envelope.license_id.is_empty()
        || envelope.watermark_uid.is_empty()
        || envelope.media_type != "image/png"
        || envelope.claim_type.is_empty()
        || envelope.artifact_ref.is_empty()
        || envelope.artifact_object_version.is_empty()
        || envelope.signer_receipt_id.is_empty()
        || envelope.artifact_finalize_receipt_id.is_empty()
        || envelope.finalized_at.is_empty()
        || envelope.worker_recovery_attempts < 0
        || envelope.recovery_control_version < 1
        || envelope.profile_identity.entitlement_version < 1
        || envelope.profile_identity.technical_profile_ids.is_empty()
        || envelope.profile_identity.regional_profile_id.is_empty()
        || !is_sha256(&envelope.final_file_sha256)
        || !is_sha256(&envelope.signer_receipt_sha256)
        || !is_sha256(&envelope.artifact_finalize_receipt_sha256)
        || !is_sha256(&envelope.profile_identity.entitlement_digest)
        || (!envelope.profile_identity_digest.is_empty()
            && !is_sha256(&envelope.profile_identity_digest))
        || (!envelope.envelope_digest.is_empty() && !is_sha256(&envelope.envelope_digest))
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidContract,
            "delivery envelope contains invalid or missing fields",
        ));
    }
    Ok(())
}

fn validate_retrieval_receipt_static(
    receipt: &AiDeliveryRetrievalReceipt,
) -> Result<(), AiDeliveryEnvelopeError> {
    if receipt.schema_version != AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION
        || receipt.retrieval_receipt_id.is_empty()
        || receipt.authorization_id.is_empty()
        || receipt.delivery_envelope_id.is_empty()
        || receipt.execution_id.is_empty()
        || receipt.retrieved_at.is_empty()
        || !is_sha256(&receipt.envelope_digest)
        || !is_sha256(&receipt.final_file_sha256)
        || !is_sha256(&receipt.artifact_finalize_receipt_sha256)
        || (!receipt.receipt_digest.is_empty() && !is_sha256(&receipt.receipt_digest))
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::RetrievalReceiptMismatch,
            "retrieval receipt contains invalid or missing fields",
        ));
    }
    Ok(())
}

fn validate_signer_receipt_binding(
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
    receipt: &Value,
) -> Result<(), AiDeliveryEnvelopeError> {
    if string_field(receipt, "signerReceiptId") != Some(envelope.signer_receipt_id.as_str())
        || string_field(receipt, "executionId") != Some(envelope.execution_id.as_str())
        || string_field(receipt, "markingSessionId") != Some(envelope.marking_session_id.as_str())
        || string_field(receipt, "watermarkUid") != Some(envelope.watermark_uid.as_str())
        || string_field(receipt, "finalSignedPngSha256")
            != Some(envelope.final_file_sha256.as_str())
        || string_field(receipt, "profileEntitlementDigest")
            != Some(envelope.profile_identity.entitlement_digest.as_str())
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::SignerReceiptBindingMismatch,
            "signer receipt fields do not bind to the delivery envelope",
        ));
    }
    Ok(())
}

fn validate_finalize_receipt_binding(
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
    receipt: &Value,
) -> Result<(), AiDeliveryEnvelopeError> {
    if string_field(receipt, "artifactReceiptId")
        != Some(envelope.artifact_finalize_receipt_id.as_str())
        || string_field(receipt, "executionId") != Some(envelope.execution_id.as_str())
        || string_field(receipt, "artifactRef") != Some(envelope.artifact_ref.as_str())
        || string_field(receipt, "objectVersion") != Some(envelope.artifact_object_version.as_str())
        || string_field(receipt, "finalSignedPngSha256")
            != Some(envelope.final_file_sha256.as_str())
        || string_field(receipt, "operation") != Some("finalize")
        || string_field(receipt, "durabilityStatus") != Some("finalized")
    {
        return Err(AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::ArtifactFinalizeReceiptBindingMismatch,
            "artifact finalize receipt fields do not bind to the delivery envelope",
        ));
    }
    Ok(())
}

fn canonical_json(json_text: &str) -> Result<Vec<u8>, AiDeliveryEnvelopeError> {
    let value: Value = serde_json::from_str(json_text).map_err(|error| {
        AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidContract,
            format!("invalid receipt JSON: {error}"),
        )
    })?;
    serde_json::to_vec(&sort_json_value(value)).map_err(|error| {
        AiDeliveryEnvelopeError::new(
            AiDeliveryEnvelopeErrorCode::InvalidContract,
            format!("canonicalize receipt JSON: {error}"),
        )
    })
}

fn sort_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json_value).collect()),
        Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = serde_json::Map::new();
            for (key, value) in entries {
                sorted.insert(key, sort_json_value(value));
            }
            Value::Object(sorted)
        }
        other => other,
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_json(media_digest: &str, entitlement_digest: &str) -> (String, String) {
        (
            json!({
                "signerReceiptId": "signer-receipt-1",
                "executionId": "execution-1",
                "markingSessionId": "session-1",
                "watermarkUid": "HS-DELIVERY-1",
                "finalSignedPngSha256": media_digest,
                "profileEntitlementDigest": entitlement_digest
            })
            .to_string(),
            json!({
                "artifactReceiptId": "finalize-receipt-1",
                "executionId": "execution-1",
                "artifactRef": "artifact://execution-1",
                "objectVersion": "version-1",
                "finalSignedPngSha256": media_digest,
                "operation": "finalize",
                "durabilityStatus": "finalized"
            })
            .to_string(),
        )
    }

    fn envelope(
        media: &[u8],
        signer_receipt_json: &str,
        finalize_receipt_json: &str,
    ) -> AiConfirmedArtifactDeliveryEnvelope {
        let entitlement_digest = "c".repeat(64);
        seal_ai_delivery_envelope(AiConfirmedArtifactDeliveryEnvelope {
            schema_version: AI_DELIVERY_ENVELOPE_SCHEMA_VERSION.to_string(),
            delivery_envelope_id: "delivery-1".to_string(),
            execution_id: "execution-1".to_string(),
            marking_session_id: "session-1".to_string(),
            transparency_manifest_id: "manifest-1".to_string(),
            license_id: "license-1".to_string(),
            watermark_uid: "HS-DELIVERY-1".to_string(),
            media_type: "image/png".to_string(),
            claim_type: "ai_generated".to_string(),
            signing_status: "confirmed".to_string(),
            artifact_status: "finalized".to_string(),
            recovery_state: "completed".to_string(),
            worker_recovery_attempts: 1,
            recovery_control_version: 2,
            final_file_sha256: sha256_hex(media),
            artifact_ref: "artifact://execution-1".to_string(),
            artifact_object_version: "version-1".to_string(),
            signer_receipt_id: "signer-receipt-1".to_string(),
            signer_receipt_sha256: canonical_json_sha256(signer_receipt_json).unwrap(),
            artifact_finalize_receipt_id: "finalize-receipt-1".to_string(),
            artifact_finalize_receipt_sha256: canonical_json_sha256(finalize_receipt_json).unwrap(),
            profile_identity: AiDeliveryProfileIdentity {
                entitlement_version: 3,
                entitlement_digest,
                technical_profile_ids: vec![
                    "hiddenshield_v3_image_anchor_v1".to_string(),
                    "c2pa_post_embed_signing_v1".to_string(),
                ],
                regional_profile_id: "cn_aigc_label_2025_image_export_v1".to_string(),
            },
            profile_identity_digest: String::new(),
            finalized_at: "2026-07-28T00:00:00Z".to_string(),
            envelope_digest: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn confirmed_finalized_envelope_is_accepted() {
        let media = b"final-png";
        let entitlement_digest = "c".repeat(64);
        let (signer, finalize) = receipt_json(&sha256_hex(media), &entitlement_digest);
        let envelope = envelope(media, &signer, &finalize);
        let result = validate_ai_delivery_envelope(&envelope, media, &signer, &finalize).unwrap();
        assert!(result.accepted);
    }

    #[test]
    fn status_and_digest_mismatches_fail_closed() {
        let media = b"final-png";
        let entitlement_digest = "c".repeat(64);
        let (signer, finalize) = receipt_json(&sha256_hex(media), &entitlement_digest);
        let base = envelope(media, &signer, &finalize);

        let mut pending = base.clone();
        pending.artifact_status = "pending_finalize".to_string();
        assert_eq!(
            validate_ai_delivery_envelope(&pending, media, &signer, &finalize)
                .unwrap_err()
                .code,
            AiDeliveryEnvelopeErrorCode::ArtifactNotFinalized.as_str()
        );

        assert_eq!(
            validate_ai_delivery_envelope(&base, b"tampered", &signer, &finalize)
                .unwrap_err()
                .code,
            AiDeliveryEnvelopeErrorCode::FinalFileHashMismatch.as_str()
        );

        let mut wrong_profile = base.clone();
        wrong_profile.profile_identity.regional_profile_id = "eu-ai-act-image-v1".to_string();
        assert_eq!(
            validate_ai_delivery_envelope(&wrong_profile, media, &signer, &finalize)
                .unwrap_err()
                .code,
            AiDeliveryEnvelopeErrorCode::ProfileIdentityDigestMismatch.as_str()
        );

        let wrong_signer = json!({"signerReceiptId": "other"}).to_string();
        assert_eq!(
            validate_ai_delivery_envelope(&base, media, &wrong_signer, &finalize)
                .unwrap_err()
                .code,
            AiDeliveryEnvelopeErrorCode::SignerReceiptHashMismatch.as_str()
        );
    }

    #[test]
    fn retrieval_receipt_admits_verified_import() {
        let media = b"final-png";
        let entitlement_digest = "c".repeat(64);
        let (signer, finalize) = receipt_json(&sha256_hex(media), &entitlement_digest);
        let envelope = envelope(media, &signer, &finalize);
        let receipt = seal_ai_delivery_retrieval_receipt(AiDeliveryRetrievalReceipt {
            schema_version: AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION.to_string(),
            retrieval_receipt_id: "retrieval-receipt-1".to_string(),
            authorization_id: "delivery-authorization-1".to_string(),
            delivery_envelope_id: envelope.delivery_envelope_id.clone(),
            execution_id: envelope.execution_id.clone(),
            envelope_digest: envelope.envelope_digest.clone(),
            final_file_sha256: envelope.final_file_sha256.clone(),
            artifact_finalize_receipt_sha256: envelope.artifact_finalize_receipt_sha256.clone(),
            retrieved_at: "2026-07-28T00:05:00Z".to_string(),
            receipt_digest: String::new(),
        })
        .unwrap();

        let admission =
            validate_ai_delivery_import(&envelope, media, &signer, &finalize, &receipt).unwrap();
        assert!(admission.admitted);
        assert_eq!(admission.authorization_id, "delivery-authorization-1");
    }

    #[test]
    fn retrieval_receipt_tamper_fails_closed() {
        let media = b"final-png";
        let entitlement_digest = "c".repeat(64);
        let (signer, finalize) = receipt_json(&sha256_hex(media), &entitlement_digest);
        let envelope = envelope(media, &signer, &finalize);
        let mut receipt = seal_ai_delivery_retrieval_receipt(AiDeliveryRetrievalReceipt {
            schema_version: AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION.to_string(),
            retrieval_receipt_id: "retrieval-receipt-1".to_string(),
            authorization_id: "delivery-authorization-1".to_string(),
            delivery_envelope_id: envelope.delivery_envelope_id.clone(),
            execution_id: envelope.execution_id.clone(),
            envelope_digest: envelope.envelope_digest.clone(),
            final_file_sha256: envelope.final_file_sha256.clone(),
            artifact_finalize_receipt_sha256: envelope.artifact_finalize_receipt_sha256.clone(),
            retrieved_at: "2026-07-28T00:05:00Z".to_string(),
            receipt_digest: String::new(),
        })
        .unwrap();
        receipt.authorization_id = "delivery-authorization-tampered".to_string();

        assert_eq!(
            validate_ai_delivery_import(&envelope, media, &signer, &finalize, &receipt)
                .unwrap_err()
                .code,
            AiDeliveryEnvelopeErrorCode::RetrievalReceiptMismatch.as_str()
        );
    }
}
