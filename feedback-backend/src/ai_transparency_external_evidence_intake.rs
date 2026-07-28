use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::ai_transparency_change_command::{
    ActorAuthorizationInput, ApprovalReferenceInput, ApprovalReferenceType, ChangeCommandPreflight,
};

pub const REASON_EVIDENCE_RECEIVED: &str = "ai_external_evidence_received_for_review";

#[derive(Debug, Clone)]
pub struct SubmitExternalEvidenceIntakeCommand {
    pub source_kind: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub source_reference: String,
    pub evidence_reference: String,
    pub evidence_sha256: String,
    pub signer_reference: String,
    pub contract_reference: String,
    pub security_review_reference: String,
    pub submitter_snapshot_id: String,
    pub submitter_token_hash: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEvidenceIntakeOutcome {
    pub evidence_intake_id: String,
    pub reason_code: &'static str,
}

#[derive(Debug, Error)]
pub enum ExternalEvidenceIntakeError {
    #[error("external evidence intake is invalid: {0}")]
    Invalid(&'static str),
    #[error("external evidence intake authorization denied: {0}")]
    AuthorizationDenied(String),
    #[error("external evidence intake reference denied: {0}")]
    ReferenceDenied(String),
    #[error("external evidence intake database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn submit_external_evidence_intake(
    pool: &PgPool,
    preflight: &ChangeCommandPreflight<'_>,
    command: &SubmitExternalEvidenceIntakeCommand,
) -> Result<ExternalEvidenceIntakeOutcome, ExternalEvidenceIntakeError> {
    validate(command)?;
    authorize(preflight, command)?;

    let evidence_intake_id = Uuid::new_v4().to_string();
    let audit_event_id = Uuid::new_v4().to_string();
    let event_digest = event_digest(command, &evidence_intake_id);
    let mut transaction = pool.begin().await?;

    sqlx::query(
        "INSERT INTO ai_transparency_external_evidence_intakes (
            evidence_intake_id, source_kind, tenant_id, workspace_id, environment,
            source_reference, evidence_reference, evidence_sha256, signer_reference,
            contract_reference, security_review_reference, submitter_snapshot_id,
            valid_from, valid_until, received_at, status, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
            $13, $14, $15, 'received_for_review', $15
        )",
    )
    .bind(&evidence_intake_id)
    .bind(&command.source_kind)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(&command.source_reference)
    .bind(&command.evidence_reference)
    .bind(&command.evidence_sha256)
    .bind(&command.signer_reference)
    .bind(&command.contract_reference)
    .bind(&command.security_review_reference)
    .bind(&command.submitter_snapshot_id)
    .bind(command.valid_from)
    .bind(command.valid_until)
    .bind(command.received_at)
    .execute(&mut *transaction)
    .await?;

    insert_audit(
        &mut transaction,
        &audit_event_id,
        &evidence_intake_id,
        &command.submitter_snapshot_id,
        &event_digest,
        command.received_at,
    )
    .await?;
    transaction.commit().await?;

    Ok(ExternalEvidenceIntakeOutcome {
        evidence_intake_id,
        reason_code: REASON_EVIDENCE_RECEIVED,
    })
}

fn authorize(
    preflight: &ChangeCommandPreflight<'_>,
    command: &SubmitExternalEvidenceIntakeCommand,
) -> Result<(), ExternalEvidenceIntakeError> {
    let actor = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.submitter_token_hash,
            required_role: "ai_transparency_security_approver",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "submit_external_evidence",
        });
    if !actor.authorized {
        return Err(ExternalEvidenceIntakeError::AuthorizationDenied(
            actor
                .reason_code
                .unwrap_or_else(|| "iam_scope_denied".to_string()),
        ));
    }
    for (reference_type, reference_id) in [
        (ApprovalReferenceType::Contract, &command.contract_reference),
        (
            ApprovalReferenceType::SecurityReview,
            &command.security_review_reference,
        ),
    ] {
        let decision = preflight
            .references
            .verify_approval_reference(&ApprovalReferenceInput {
                reference_type,
                reference_id,
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                operation: "submit_external_evidence",
            });
        if !decision.verified {
            return Err(ExternalEvidenceIntakeError::ReferenceDenied(
                decision
                    .reason_code
                    .unwrap_or_else(|| "reference_unavailable".to_string()),
            ));
        }
    }
    Ok(())
}

fn validate(
    command: &SubmitExternalEvidenceIntakeCommand,
) -> Result<(), ExternalEvidenceIntakeError> {
    if !matches!(
        command.source_kind.as_str(),
        "provider_recovery" | "design_partner_sandbox"
    ) {
        return Err(ExternalEvidenceIntakeError::Invalid("source_kind"));
    }
    if !matches!(command.environment.as_str(), "sandbox" | "production") {
        return Err(ExternalEvidenceIntakeError::Invalid("environment"));
    }
    if !(command.source_reference.starts_with("provider://")
        || command.source_reference.starts_with("partner://"))
    {
        return Err(ExternalEvidenceIntakeError::Invalid("source_reference"));
    }
    if !is_sha256(&command.evidence_sha256)
        || command.evidence_reference != format!("evidence://sha256/{}", command.evidence_sha256)
    {
        return Err(ExternalEvidenceIntakeError::Invalid("evidence_reference"));
    }
    if !(command.signer_reference.starts_with("approval://")
        || command.signer_reference.starts_with("receipt://"))
        || !command.contract_reference.starts_with("approval://")
        || !command.security_review_reference.starts_with("approval://")
    {
        return Err(ExternalEvidenceIntakeError::Invalid("approval_reference"));
    }
    if command.valid_until <= command.valid_from || contains_secret_or_placeholder(command) {
        return Err(ExternalEvidenceIntakeError::Invalid(
            "evidence_window_or_reference",
        ));
    }
    Ok(())
}

fn contains_secret_or_placeholder(command: &SubmitExternalEvidenceIntakeCommand) -> bool {
    [
        &command.source_reference,
        &command.evidence_reference,
        &command.signer_reference,
        &command.contract_reference,
        &command.security_review_reference,
    ]
    .iter()
    .any(|value| {
        let normalized = value.to_ascii_lowercase();
        normalized.contains("replace-me")
            || normalized.contains("placeholder")
            || normalized.contains("secret=")
            || normalized.contains("token=")
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn event_digest(command: &SubmitExternalEvidenceIntakeCommand, evidence_intake_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{}|{}|{}|{}|{}|{}",
        evidence_intake_id,
        command.source_kind,
        command.source_reference,
        command.evidence_sha256,
        command.signer_reference,
        command.received_at.to_rfc3339(),
    ));
    format!("{:x}", hasher.finalize())
}

async fn insert_audit(
    transaction: &mut Transaction<'_, Postgres>,
    audit_event_id: &str,
    evidence_intake_id: &str,
    actor_snapshot_id: &str,
    event_digest: &str,
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_external_evidence_intake_audit_events (
            evidence_intake_audit_event_id, evidence_intake_id, event_type,
            actor_snapshot_id, event_digest, occurred_at
        ) VALUES ($1, $2, 'evidence_received', $3, $4, $5)",
    )
    .bind(audit_event_id)
    .bind(evidence_intake_id)
    .bind(actor_snapshot_id)
    .bind(event_digest)
    .bind(occurred_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn command() -> SubmitExternalEvidenceIntakeCommand {
        let now = Utc::now();
        SubmitExternalEvidenceIntakeCommand {
            source_kind: "provider_recovery".to_string(),
            tenant_id: "tenant".to_string(),
            workspace_id: "workspace".to_string(),
            environment: "sandbox".to_string(),
            source_reference: "provider://sandbox/recovery-v1".to_string(),
            evidence_reference: format!("evidence://sha256/{}", "a".repeat(64)),
            evidence_sha256: "a".repeat(64),
            signer_reference: "receipt://signer/recovery-v1".to_string(),
            contract_reference: "approval://contract/recovery-v1".to_string(),
            security_review_reference: "approval://security/recovery-v1".to_string(),
            submitter_snapshot_id: "snapshot".to_string(),
            submitter_token_hash: "token-hash".to_string(),
            valid_from: now,
            valid_until: now + Duration::hours(1),
            received_at: now,
        }
    }

    #[test]
    fn intake_validation_rejects_non_immutable_evidence_reference() {
        let mut input = command();
        input.evidence_reference = "evidence://sha256/not-a-digest".to_string();
        assert!(matches!(
            validate(&input),
            Err(ExternalEvidenceIntakeError::Invalid("evidence_reference"))
        ));
    }

    #[test]
    fn intake_validation_rejects_secret_or_placeholder_references() {
        let mut input = command();
        input.signer_reference = "receipt://replace-me/signature".to_string();
        assert!(matches!(
            validate(&input),
            Err(ExternalEvidenceIntakeError::Invalid(
                "evidence_window_or_reference"
            ))
        ));
    }
}
