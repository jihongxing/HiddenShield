use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use watermark_core::{
    watermark_id_from_uid, AIContentFlags, AuthenticityClaim, EmbedOptions, GenerationMethod,
    ImageOutputFormat, MediaInput, MediaOutput, ModificationLevel, PayloadV2BuildInput,
    TrainingPermission, WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkService,
};

use crate::ai_transparency_confirm_command::{
    execute_postgres_confirm_marking_command, ConfirmEvidence, ConfirmExplicitLabelReceipt,
    ConfirmFailureInjection, ConfirmMarker, ConfirmMarkingCommand, ConfirmMarkingError,
};

pub const ANCHOR_PROFILE_ID: &str = "hiddenshield_v3_image_anchor_v1";
pub const REASON_EXECUTOR_SESSION_INVALID: &str = "ai_executor_session_invalid";
pub const REASON_EXECUTOR_PROFILE_INVALID: &str = "ai_executor_profile_invalid";
pub const REASON_EXECUTOR_CONFIRM_REJECTED: &str = "ai_executor_confirm_rejected";

#[derive(Debug, Clone)]
pub struct InternalImageMarkingCommand {
    pub marking_session_id: String,
    pub execution_id: String,
    pub watermark_uid: String,
    pub source_image_bytes: Vec<u8>,
    pub provider_id: String,
    pub system_name: String,
    pub system_version: String,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub generation_mode: String,
    pub generated_at: DateTime<Utc>,
    pub operations: Value,
    pub parent_subjects: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InternalExplicitLabelPlan {
    pub profile_id: String,
    pub locale: String,
    pub label_text: String,
}

#[derive(Debug, Clone)]
pub struct InternalImageMarkingOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub protected_image_bytes: Option<Vec<u8>>,
    pub protected_image_sha256: Option<String>,
    pub explicit_label_plans: Vec<InternalExplicitLabelPlan>,
}

#[derive(Debug, Clone)]
pub struct PreparedInternalImageMarking {
    pub protected_image_bytes: Vec<u8>,
    pub protected_image_sha256: String,
    pub source_image_sha256: String,
    pub marker_evidence_digest: String,
    pub explicit_label_receipt_digest: String,
    pub explicit_label_plans: Vec<InternalExplicitLabelPlan>,
    pub confirm_command: ConfirmMarkingCommand,
}

#[derive(Debug, thiserror::Error)]
pub enum InternalImageMarkingError {
    #[error("PostgreSQL internal image marking executor failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("watermark-core internal image marking executor failed: {0}")]
    Watermark(#[from] watermark_core::WatermarkError),
    #[error("confirm command failed: {0}")]
    Confirm(#[from] ConfirmMarkingError),
}

pub async fn execute_postgres_internal_image_marking(
    connection: &mut PgConnection,
    command: &InternalImageMarkingCommand,
) -> Result<InternalImageMarkingOutcome, InternalImageMarkingError> {
    let session = sqlx::query(
        "SELECT status, claim_type, requested_profile_ids_json
         FROM ai_marking_sessions WHERE marking_session_id = $1",
    )
    .bind(&command.marking_session_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(session) = session else {
        return Ok(rejected(REASON_EXECUTOR_SESSION_INVALID));
    };
    if session.get::<String, _>("status") != "ready_to_confirm"
        || session.get::<String, _>("claim_type") != "ai_generated"
    {
        return Ok(rejected(REASON_EXECUTOR_SESSION_INVALID));
    }
    let requested_profile_ids = profile_ids(session.get("requested_profile_ids_json"));
    if !requested_profile_ids
        .iter()
        .any(|profile_id| profile_id == ANCHOR_PROFILE_ID)
    {
        return Ok(rejected(REASON_EXECUTOR_PROFILE_INVALID));
    }

    let prepared = prepare_internal_image_marking(command, &requested_profile_ids)?;
    let confirmation =
        execute_postgres_confirm_marking_command(connection, &prepared.confirm_command).await?;
    if !confirmation.succeeded {
        return Ok(rejected(
            confirmation
                .reason_code
                .as_deref()
                .unwrap_or(REASON_EXECUTOR_CONFIRM_REJECTED),
        ));
    }
    Ok(InternalImageMarkingOutcome {
        succeeded: true,
        reason_code: None,
        protected_image_bytes: Some(prepared.protected_image_bytes),
        protected_image_sha256: Some(prepared.protected_image_sha256),
        explicit_label_plans: prepared.explicit_label_plans,
    })
}

pub fn prepare_internal_image_marking(
    command: &InternalImageMarkingCommand,
    requested_profile_ids: &[String],
) -> Result<PreparedInternalImageMarking, InternalImageMarkingError> {
    if !requested_profile_ids
        .iter()
        .any(|profile_id| profile_id == ANCHOR_PROFILE_ID)
    {
        return Err(watermark_core::WatermarkError::EmbedFailed(
            REASON_EXECUTOR_PROFILE_INVALID.to_string(),
        )
        .into());
    }
    let source_sha256 = sha256_hex(&command.source_image_bytes);
    let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id: watermark_id_from_uid(&command.watermark_uid)?,
        parent_watermark_id: None,
        revision: 1,
        issued_at: command.generated_at.timestamp().max(0) as u64,
        original_sha256: sha256_bytes(&command.source_image_bytes),
        ai_flags: AIContentFlags {
            is_ai_generated: true,
            training_permission: TrainingPermission::Prohibited,
            generation_method: GenerationMethod::TextToImage,
            human_modification_level: ModificationLevel::PureAI,
            authenticity_claim: AuthenticityClaim::Synthetic,
            reserved: 0,
        },
        issue_mode: WatermarkIssueMode::ServerConfirmed,
        media_type: WatermarkMediaType::Image,
        registry_proof_hash: None,
        creator_binding: Some("HiddenShield internal image marking executor"),
    })?;
    let output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: command.source_image_bytes.clone(),
        },
        &payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: false,
            ..EmbedOptions::default()
        },
    )?;
    let MediaOutput::ImageBytes {
        bytes: protected_image_bytes,
        format: ImageOutputFormat::Png,
    } = output
    else {
        return Err(watermark_core::WatermarkError::EmbedFailed(
            REASON_EXECUTOR_PROFILE_INVALID.to_string(),
        )
        .into());
    };
    let decoded = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: protected_image_bytes.clone(),
    })?;
    if !decoded.is_v3_minimal_anchor()
        || decoded.payload_auth_status() != "verified"
        || decoded.watermark_uid() != command.watermark_uid
    {
        return Err(watermark_core::WatermarkError::ExtractFailed(
            REASON_EXECUTOR_PROFILE_INVALID.to_string(),
        )
        .into());
    }

    let protected_image_sha256 = sha256_hex(&protected_image_bytes);
    let explicit_label_plans = requested_profile_ids
        .iter()
        .filter(|profile_id| profile_id.as_str() != ANCHOR_PROFILE_ID)
        .map(|profile_id| InternalExplicitLabelPlan {
            profile_id: profile_id.clone(),
            locale: "zh-CN".to_string(),
            label_text: "AI 生成（内部测试）".to_string(),
        })
        .collect::<Vec<_>>();
    let marker_evidence_digest = sha256_hex(
        format!(
            "{}|{}|{}|{}",
            command.watermark_uid, source_sha256, protected_image_sha256, ANCHOR_PROFILE_ID
        )
        .as_bytes(),
    );
    let explicit_label_receipt_digest =
        sha256_hex(serde_json::to_vec(&explicit_label_plans).unwrap_or_default());
    let confirm_command = ConfirmMarkingCommand {
        marking_session_id: command.marking_session_id.clone(),
        transparency_manifest_id: format!("manifest-{}", command.execution_id),
        ledger_entry_id: format!("ledger-{}", command.execution_id),
        audit_event_id: format!("audit-{}", command.execution_id),
        watermark_uid: command.watermark_uid.clone(),
        subject_digest: protected_image_sha256.clone(),
        generation_mode: command.generation_mode.clone(),
        provider_id: command.provider_id.clone(),
        system_name: command.system_name.clone(),
        system_version: command.system_version.clone(),
        model_id: command.model_id.clone(),
        model_version: command.model_version.clone(),
        generated_at: command.generated_at,
        operations: command.operations.clone(),
        parent_subjects: command.parent_subjects.clone(),
        profile_statuses: Value::Array(
            requested_profile_ids
                .iter()
                .map(|profile_id| json!({"profileId": profile_id, "status": "applied_internal_only"}))
                .collect(),
        ),
        evidence: ConfirmEvidence {
            evidence_id: format!("evidence-{}", command.execution_id),
            evidence_level: "self_declared".to_string(),
            evidence_source: "internal_image_marking_executor".to_string(),
            issuer_id: None,
            key_id: None,
            proof_type: "watermark_core_write_after_read".to_string(),
            signature_algorithm: None,
            signature: None,
        },
        markers: vec![ConfirmMarker {
            marker_binding_id: format!("marker-{}", command.execution_id),
            marker_type: "blind_watermark".to_string(),
            marker_profile_id: ANCHOR_PROFILE_ID.to_string(),
            marker_version: "v3".to_string(),
            embed_status: "verified".to_string(),
            verify_status: "verified".to_string(),
            binding_digest: Some(marker_evidence_digest.clone()),
        }],
        explicit_label_receipts: explicit_label_plans
            .iter()
            .map(|label| ConfirmExplicitLabelReceipt {
                receipt_id: format!("label-{}-{}", command.execution_id, label.profile_id),
                profile_id: label.profile_id.clone(),
                required_surface: "platform_ui".to_string(),
                render_mode: "internal_label_plan".to_string(),
                rendered_asset_digest: None,
                placement: json!({"surface": "platform_ui"}),
                locale: label.locale.clone(),
                label_text: label.label_text.clone(),
                applied_at: Utc::now(),
                applied_by: "internal_image_marking_executor".to_string(),
                verification_status: "verified".to_string(),
            })
            .collect(),
        write_after_read_verified: true,
        failure_injection: ConfirmFailureInjection::None,
    };
    Ok(PreparedInternalImageMarking {
        protected_image_bytes,
        protected_image_sha256,
        source_image_sha256: source_sha256,
        marker_evidence_digest,
        explicit_label_receipt_digest,
        explicit_label_plans,
        confirm_command,
    })
}

fn profile_ids(value: Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn rejected(reason_code: &str) -> InternalImageMarkingOutcome {
    InternalImageMarkingOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        protected_image_bytes: None,
        protected_image_sha256: None,
        explicit_label_plans: Vec::new(),
    }
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
