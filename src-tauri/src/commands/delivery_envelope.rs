use serde::{Deserialize, Serialize};
use watermark_core::{
    validate_ai_delivery_envelope, validate_ai_delivery_import,
    AiConfirmedArtifactDeliveryEnvelope, AiDeliveryEnvelopeValidationResult,
    AiDeliveryRetrievalReceipt,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDeliveryEnvelopeValidationRequest {
    pub envelope_json: String,
    pub final_media_bytes: Vec<u8>,
    pub signer_receipt_json: String,
    pub artifact_finalize_receipt_json: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDeliveryEnvelopeValidationResponse {
    pub accepted: bool,
    pub reason_code: Option<String>,
    pub envelope_digest: Option<String>,
    pub final_file_sha256: Option<String>,
    pub watermark_uid: Option<String>,
    pub profile_identity_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDeliveryImportAdmissionRequest {
    pub envelope_json: String,
    pub final_media_bytes: Vec<u8>,
    pub signer_receipt_json: String,
    pub artifact_finalize_receipt_json: String,
    pub retrieval_receipt_json: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDeliveryImportAdmissionResponse {
    pub admitted: bool,
    pub reason_code: Option<String>,
    pub authorization_id: Option<String>,
    pub retrieval_receipt_id: Option<String>,
    pub envelope_digest: Option<String>,
    pub final_file_sha256: Option<String>,
    pub watermark_uid: Option<String>,
}

pub fn validate_ai_delivery_envelope_for_desktop(
    request: DesktopDeliveryEnvelopeValidationRequest,
) -> DesktopDeliveryEnvelopeValidationResponse {
    let envelope: AiConfirmedArtifactDeliveryEnvelope =
        match serde_json::from_str(&request.envelope_json) {
            Ok(envelope) => envelope,
            Err(_) => {
                return rejected("ai_delivery_envelope_invalid_contract");
            }
        };
    match validate_ai_delivery_envelope(
        &envelope,
        &request.final_media_bytes,
        &request.signer_receipt_json,
        &request.artifact_finalize_receipt_json,
    ) {
        Ok(result) => accepted(result),
        Err(error) => rejected(error.code),
    }
}

#[tauri::command]
pub async fn validate_ai_delivery_envelope_command(
    request: DesktopDeliveryEnvelopeValidationRequest,
) -> DesktopDeliveryEnvelopeValidationResponse {
    validate_ai_delivery_envelope_for_desktop(request)
}

pub fn admit_ai_delivery_for_desktop_vault_import(
    request: DesktopDeliveryImportAdmissionRequest,
) -> DesktopDeliveryImportAdmissionResponse {
    let envelope: AiConfirmedArtifactDeliveryEnvelope =
        match serde_json::from_str(&request.envelope_json) {
            Ok(envelope) => envelope,
            Err(_) => {
                return import_rejected("ai_delivery_envelope_invalid_contract");
            }
        };
    let retrieval_receipt: AiDeliveryRetrievalReceipt =
        match serde_json::from_str(&request.retrieval_receipt_json) {
            Ok(receipt) => receipt,
            Err(_) => {
                return import_rejected("ai_delivery_retrieval_receipt_mismatch");
            }
        };
    match validate_ai_delivery_import(
        &envelope,
        &request.final_media_bytes,
        &request.signer_receipt_json,
        &request.artifact_finalize_receipt_json,
        &retrieval_receipt,
    ) {
        Ok(admission) => DesktopDeliveryImportAdmissionResponse {
            admitted: admission.admitted,
            reason_code: None,
            authorization_id: Some(admission.authorization_id),
            retrieval_receipt_id: Some(admission.retrieval_receipt_id),
            envelope_digest: Some(admission.envelope_digest),
            final_file_sha256: Some(admission.final_file_sha256),
            watermark_uid: Some(admission.watermark_uid),
        },
        Err(error) => import_rejected(error.code),
    }
}

#[tauri::command]
pub async fn admit_ai_delivery_vault_import_command(
    request: DesktopDeliveryImportAdmissionRequest,
) -> DesktopDeliveryImportAdmissionResponse {
    admit_ai_delivery_for_desktop_vault_import(request)
}

fn accepted(
    result: AiDeliveryEnvelopeValidationResult,
) -> DesktopDeliveryEnvelopeValidationResponse {
    DesktopDeliveryEnvelopeValidationResponse {
        accepted: true,
        reason_code: None,
        envelope_digest: Some(result.envelope_digest),
        final_file_sha256: Some(result.final_file_sha256),
        watermark_uid: Some(result.watermark_uid),
        profile_identity_digest: Some(result.profile_identity_digest),
    }
}

fn rejected(reason_code: &str) -> DesktopDeliveryEnvelopeValidationResponse {
    DesktopDeliveryEnvelopeValidationResponse {
        accepted: false,
        reason_code: Some(reason_code.to_string()),
        envelope_digest: None,
        final_file_sha256: None,
        watermark_uid: None,
        profile_identity_digest: None,
    }
}

fn import_rejected(reason_code: &str) -> DesktopDeliveryImportAdmissionResponse {
    DesktopDeliveryImportAdmissionResponse {
        admitted: false,
        reason_code: Some(reason_code.to_string()),
        authorization_id: None,
        retrieval_receipt_id: None,
        envelope_digest: None,
        final_file_sha256: None,
        watermark_uid: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/contracts/ai-transparency-delivery-envelope/success-v1.fixture.json"
        ))
        .unwrap()
    }

    fn request(value: &Value) -> DesktopDeliveryEnvelopeValidationRequest {
        DesktopDeliveryEnvelopeValidationRequest {
            envelope_json: value["envelope"].to_string(),
            final_media_bytes: value["finalMediaUtf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec(),
            signer_receipt_json: value["signerReceipt"].to_string(),
            artifact_finalize_receipt_json: value["artifactFinalizeReceipt"].to_string(),
        }
    }

    fn import_request(value: &Value) -> DesktopDeliveryImportAdmissionRequest {
        DesktopDeliveryImportAdmissionRequest {
            envelope_json: value["envelope"].to_string(),
            final_media_bytes: value["finalMediaUtf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec(),
            signer_receipt_json: value["signerReceipt"].to_string(),
            artifact_finalize_receipt_json: value["artifactFinalizeReceipt"].to_string(),
            retrieval_receipt_json: value["retrievalReceipt"].to_string(),
        }
    }

    #[test]
    fn desktop_accepts_shared_delivery_fixture() {
        let result = validate_ai_delivery_envelope_for_desktop(request(&fixture()));
        assert!(result.accepted, "{result:?}");
        assert!(result.reason_code.is_none());
    }

    #[test]
    fn desktop_fails_closed_for_status_and_digest_mismatch() {
        let mut pending = fixture();
        pending["envelope"]["artifactStatus"] = Value::String("pending_finalize".to_string());
        let result = validate_ai_delivery_envelope_for_desktop(request(&pending));
        assert!(!result.accepted);
        assert_eq!(
            result.reason_code.as_deref(),
            Some("ai_delivery_envelope_artifact_not_finalized")
        );

        let mut tampered_media = request(&fixture());
        tampered_media.final_media_bytes = b"tampered".to_vec();
        let result = validate_ai_delivery_envelope_for_desktop(tampered_media);
        assert_eq!(
            result.reason_code.as_deref(),
            Some("ai_delivery_envelope_final_file_hash_mismatch")
        );

        let mut tampered_receipt = fixture();
        tampered_receipt["signerReceipt"]["signerReceiptId"] =
            Value::String("other-receipt".to_string());
        let result = validate_ai_delivery_envelope_for_desktop(request(&tampered_receipt));
        assert_eq!(
            result.reason_code.as_deref(),
            Some("ai_delivery_envelope_signer_receipt_hash_mismatch")
        );
    }

    #[test]
    fn desktop_admits_only_shared_retrieval_package() {
        let value: Value = serde_json::from_str(include_str!(
            "../../../docs/contracts/ai-transparency-delivery-retrieval/success-v1.fixture.json"
        ))
        .unwrap();
        let result = admit_ai_delivery_for_desktop_vault_import(import_request(&value));
        assert!(result.admitted, "{result:?}");
        assert_eq!(
            result.authorization_id.as_deref(),
            Some("delivery-auth-fixture")
        );
    }

    #[test]
    fn desktop_import_rejects_receipt_mismatch_without_metadata() {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../docs/contracts/ai-transparency-delivery-retrieval/success-v1.fixture.json"
        ))
        .unwrap();
        value["retrievalReceipt"]["authorizationId"] =
            Value::String("delivery-auth-tampered".to_string());
        let result = admit_ai_delivery_for_desktop_vault_import(import_request(&value));
        assert!(!result.admitted);
        assert_eq!(
            result.reason_code.as_deref(),
            Some("ai_delivery_retrieval_receipt_mismatch")
        );
        assert!(result.authorization_id.is_none());
        assert!(result.retrieval_receipt_id.is_none());
        assert!(result.envelope_digest.is_none());
        assert!(result.final_file_sha256.is_none());
        assert!(result.watermark_uid.is_none());
    }
}
