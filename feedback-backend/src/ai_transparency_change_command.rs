use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

pub const REASON_IDEMPOTENCY_REPLAY: &str = "idempotency_replay";
pub const REASON_CONFLICTING_REQUEST_EXISTS: &str = "conflicting_request_exists";
pub const REASON_TARGET_STATE_CONFLICT: &str = "target_state_conflict";
pub const REASON_TARGET_VERSION_CONFLICT: &str = "target_version_conflict";
pub const REASON_AUDIT_WRITE_FAILED: &str = "audit_write_failed";

#[derive(Debug, Clone)]
pub struct ActorAuthorizationInput<'a> {
    pub token_hash: &'a str,
    pub required_role: &'a str,
    pub tenant_id: &'a str,
    pub workspace_id: &'a str,
    pub environment: &'a str,
    pub operation: &'a str,
}

#[derive(Debug, Clone)]
pub struct ActorAuthorizationDecision {
    pub authorized: bool,
    pub reason_code: Option<String>,
    pub verification_receipt_id: Option<String>,
}

pub trait InternalIamAuthorizationAdapter: Send + Sync {
    fn verify_actor_authorization(
        &self,
        input: &ActorAuthorizationInput<'_>,
    ) -> ActorAuthorizationDecision;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalReferenceType {
    Contract,
    LegalReview,
    SecurityReview,
}

#[derive(Debug, Clone)]
pub struct ApprovalReferenceInput<'a> {
    pub reference_type: ApprovalReferenceType,
    pub reference_id: &'a str,
    pub tenant_id: &'a str,
    pub workspace_id: &'a str,
    pub environment: &'a str,
    pub operation: &'a str,
}

#[derive(Debug, Clone)]
pub struct ApprovalReferenceDecision {
    pub verified: bool,
    pub reason_code: Option<String>,
    pub verification_receipt_id: Option<String>,
}

pub trait ApprovalReferenceAdapter: Send + Sync {
    fn verify_approval_reference(
        &self,
        input: &ApprovalReferenceInput<'_>,
    ) -> ApprovalReferenceDecision;
}

pub struct ChangeCommandPreflight<'a> {
    pub iam: &'a dyn InternalIamAuthorizationAdapter,
    pub references: &'a dyn ApprovalReferenceAdapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalChangeCommandMode {
    SubmitRequest,
    ApplyProfileChange,
    ExecuteApprovedRequest,
}

#[derive(Debug, Clone)]
pub struct InternalChangeCommand {
    pub mode: InternalChangeCommandMode,
    pub change_request_id: String,
    pub approval_id: String,
    pub execution_id: String,
    pub entitlement_version_id: String,
    pub operation: String,
    pub target_scope_key: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub license_id: String,
    pub profile_id: String,
    pub profile_kind: String,
    pub expected_target_version: i32,
    pub desired_next_version: i32,
    pub desired_status: String,
    pub terms_version: String,
    pub contract_reference: Option<String>,
    pub legal_review_reference: Option<String>,
    pub security_review_reference: Option<String>,
    pub requester_snapshot_id: String,
    pub requester_actor_id: String,
    pub requester_token_hash: String,
    pub approver_snapshot_id: String,
    pub approver_actor_id: String,
    pub approver_role: String,
    pub approver_token_hash: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub request_digest: String,
    pub idempotency_key: String,
    pub inject_audit_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InternalChangeCommandOutcome {
    pub succeeded: bool,
    pub request_status: String,
    pub reason_code: Option<String>,
    pub target_version: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum InternalChangeCommandError {
    #[error("SQLite change command failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[cfg(feature = "postgres")]
    #[error("PostgreSQL change command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub fn execute_sqlite_change_command(
    conn: &mut Connection,
    command: &InternalChangeCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    if let Some(outcome) = validate_preflight(command, preflight) {
        return Ok(outcome);
    }
    let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let outcome = match command.mode {
        InternalChangeCommandMode::SubmitRequest => submit_sqlite_request(&transaction, command),
        InternalChangeCommandMode::ApplyProfileChange => {
            apply_sqlite_profile_change(&transaction, command, false)
        }
        InternalChangeCommandMode::ExecuteApprovedRequest => {
            apply_sqlite_profile_change(&transaction, command, true)
        }
    };
    match outcome {
        Ok(outcome) if outcome.succeeded => {
            transaction.commit()?;
            Ok(outcome)
        }
        Ok(outcome) => {
            transaction.rollback()?;
            Ok(outcome)
        }
        Err(error) if command.inject_audit_failure => {
            transaction.rollback()?;
            Ok(failed_outcome(
                REASON_AUDIT_WRITE_FAILED,
                command.expected_target_version,
            ))
        }
        Err(error) => {
            transaction.rollback()?;
            Err(error)
        }
    }
}

fn validate_preflight(
    command: &InternalChangeCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Option<InternalChangeCommandOutcome> {
    let actor_checks: &[(&str, &str)] = match command.mode {
        InternalChangeCommandMode::SubmitRequest => &[(
            command.requester_token_hash.as_str(),
            "ai_transparency_requester",
        )],
        InternalChangeCommandMode::ApplyProfileChange
        | InternalChangeCommandMode::ExecuteApprovedRequest => &[
            (
                command.requester_token_hash.as_str(),
                "ai_transparency_requester",
            ),
            (
                command.approver_token_hash.as_str(),
                command.approver_role.as_str(),
            ),
            (command.executor_token_hash.as_str(), "system_executor"),
        ],
    };
    for (token_hash, required_role) in actor_checks {
        let decision = preflight
            .iam
            .verify_actor_authorization(&ActorAuthorizationInput {
                token_hash,
                required_role,
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                operation: &command.operation,
            });
        if !decision.authorized {
            return Some(failed_outcome(
                decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
                command.expected_target_version,
            ));
        }
    }

    let required_reference = if matches!(
        command.operation.as_str(),
        "create_license" | "renew_license"
    ) {
        command
            .contract_reference
            .as_deref()
            .map(|reference_id| (ApprovalReferenceType::Contract, reference_id))
    } else if command.environment == "production"
        && matches!(
            command.operation.as_str(),
            "grant_profile_entitlement" | "renew_profile_entitlement"
        )
    {
        match command.profile_kind.as_str() {
            "regulatory" => command
                .legal_review_reference
                .as_deref()
                .map(|reference_id| (ApprovalReferenceType::LegalReview, reference_id)),
            "technical" => command
                .security_review_reference
                .as_deref()
                .map(|reference_id| (ApprovalReferenceType::SecurityReview, reference_id)),
            _ => None,
        }
    } else {
        Some((ApprovalReferenceType::Contract, "not-required"))
    };

    let Some((reference_type, reference_id)) = required_reference else {
        return Some(failed_outcome(
            match command.profile_kind.as_str() {
                "technical" => "reference_not_found",
                _ => "reference_not_found",
            },
            command.expected_target_version,
        ));
    };
    if reference_id != "not-required" {
        let decision = preflight
            .references
            .verify_approval_reference(&ApprovalReferenceInput {
                reference_type,
                reference_id,
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                operation: &command.operation,
            });
        if !decision.verified {
            return Some(failed_outcome(
                decision
                    .reason_code
                    .as_deref()
                    .unwrap_or("reference_unavailable"),
                command.expected_target_version,
            ));
        }
    }
    None
}

fn submit_sqlite_request(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    acquire_sqlite_target_lock(transaction, command)?;
    if sqlite_idempotency_exists(transaction, command)? {
        return Ok(failed_outcome(
            REASON_IDEMPOTENCY_REPLAY,
            sqlite_projection_version(transaction, command)?.unwrap_or(0),
        ));
    }
    if sqlite_inflight_request_exists(transaction, command)? {
        return Ok(failed_outcome(
            REASON_CONFLICTING_REQUEST_EXISTS,
            sqlite_projection_version(transaction, command)?.unwrap_or(0),
        ));
    }
    insert_sqlite_request(transaction, command, "pending_review")?;
    insert_sqlite_audit(
        transaction,
        command,
        1,
        "change_request_submitted",
        None,
        "pending_review",
        &command.requester_snapshot_id,
        "request_submitted",
        None,
        None,
    )?;
    Ok(InternalChangeCommandOutcome {
        succeeded: true,
        request_status: "pending_review".to_string(),
        reason_code: None,
        target_version: sqlite_projection_version(transaction, command)?.unwrap_or(0),
    })
}

fn apply_sqlite_profile_change(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
    execute_existing_request: bool,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    acquire_sqlite_target_lock(transaction, command)?;
    if execute_existing_request {
        if sqlite_execution_exists(transaction, command)? {
            return Ok(failed_outcome(
                REASON_TARGET_STATE_CONFLICT,
                sqlite_projection_version(transaction, command)?.unwrap_or(0),
            ));
        }
        let status: Option<String> = transaction
            .query_row(
                "SELECT status FROM ai_transparency_change_requests WHERE change_request_id = ?1",
                [&command.change_request_id],
                |row| row.get(0),
            )
            .optional()?;
        if status.as_deref() != Some("approved") {
            return Ok(failed_outcome(
                REASON_TARGET_STATE_CONFLICT,
                sqlite_projection_version(transaction, command)?.unwrap_or(0),
            ));
        }
    } else {
        if sqlite_idempotency_exists(transaction, command)? {
            return Ok(failed_outcome(
                REASON_IDEMPOTENCY_REPLAY,
                sqlite_projection_version(transaction, command)?.unwrap_or(0),
            ));
        }
        if sqlite_inflight_request_exists(transaction, command)? {
            return Ok(failed_outcome(
                REASON_CONFLICTING_REQUEST_EXISTS,
                sqlite_projection_version(transaction, command)?.unwrap_or(0),
            ));
        }
    }

    let current_version = sqlite_projection_version(transaction, command)?.unwrap_or(0);
    if current_version != command.expected_target_version {
        return Ok(failed_outcome(
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }

    if !execute_existing_request {
        insert_sqlite_request(transaction, command, "approved")?;
        insert_sqlite_audit(
            transaction,
            command,
            1,
            "change_request_submitted",
            None,
            "pending_review",
            &command.requester_snapshot_id,
            "request_submitted",
            None,
            None,
        )?;
        transaction.execute(
            "INSERT INTO ai_transparency_change_approvals (
                approval_id, change_request_id, decision, approver_snapshot_id,
                requester_actor_id, approver_actor_id, approver_role, decision_reason,
                policy_version, request_digest, decided_at
             ) VALUES (?1, ?2, 'approved', ?3, ?4, ?5, ?6, 'approved by internal maker-checker',
                'ai-transparency-approval-v1', ?7, ?8)",
            params![
                command.approval_id,
                command.change_request_id,
                command.approver_snapshot_id,
                command.requester_actor_id,
                command.approver_actor_id,
                command.approver_role,
                command.request_digest,
                now_text(),
            ],
        )?;
        insert_sqlite_audit(
            transaction,
            command,
            2,
            "approval_granted",
            Some("pending_review"),
            "approved",
            &command.approver_snapshot_id,
            "approval_granted",
            None,
            None,
        )?;
    }

    let audit_sequence_start = if execute_existing_request { 3 } else { 3 };
    transaction.execute(
        "INSERT INTO ai_transparency_change_executions (
            execution_id, change_request_id, executor_snapshot_id, status,
            target_version_before, target_version_after, started_at
         ) VALUES (?1, ?2, ?3, 'executing', ?4, ?5, ?6)",
        params![
            command.execution_id,
            command.change_request_id,
            command.executor_snapshot_id,
            command.expected_target_version,
            command.desired_next_version,
            now_text(),
        ],
    )?;
    transaction.execute(
        "UPDATE ai_transparency_change_requests
         SET status = 'executing', updated_at = ?2
         WHERE change_request_id = ?1",
        params![command.change_request_id, now_text()],
    )?;
    insert_sqlite_audit(
        transaction,
        command,
        audit_sequence_start,
        "execution_started",
        Some("approved"),
        "executing",
        &command.executor_snapshot_id,
        "execution_started",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )?;

    let previous_version_id: Option<String> = transaction
        .query_row(
            "SELECT current_version_id FROM ai_profile_entitlements
             WHERE license_id = ?1 AND profile_id = ?2",
            params![command.license_id, command.profile_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    transaction.execute(
        "UPDATE ai_profile_entitlement_versions
         SET status = 'superseded', superseded_at = ?3
         WHERE license_id = ?1 AND profile_id = ?2 AND status = 'active'",
        params![command.license_id, command.profile_id, now_text()],
    )?;
    transaction.execute(
        "INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version,
            previous_version_id, profile_kind, status, effective_at, expires_at,
            terms_version, legal_review_reference, security_review_reference,
            source_change_request_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            command.entitlement_version_id,
            command.license_id,
            command.profile_id,
            command.desired_next_version,
            previous_version_id,
            command.profile_kind,
            command.desired_status,
            now_text(),
            future_text(),
            command.terms_version,
            command.legal_review_reference,
            command.security_review_reference,
            command.change_request_id,
            now_text(),
        ],
    )?;
    transaction.execute(
        "UPDATE ai_profile_entitlements
         SET profile_kind = ?3, status = ?4, effective_at = ?5, expires_at = ?6,
             terms_version = ?7, approved_by = ?8, updated_at = ?5,
             current_version_id = ?9, current_version = ?10, projection_updated_at = ?5
         WHERE license_id = ?1 AND profile_id = ?2 AND current_version = ?11",
        params![
            command.license_id,
            command.profile_id,
            command.profile_kind,
            command.desired_status,
            now_text(),
            future_text(),
            command.terms_version,
            command.approver_actor_id,
            command.entitlement_version_id,
            command.desired_next_version,
            command.expected_target_version,
        ],
    )?;
    if transaction.changes() != 1 {
        return Ok(failed_outcome(
            REASON_TARGET_VERSION_CONFLICT,
            sqlite_projection_version(transaction, command)?.unwrap_or(0),
        ));
    }
    insert_sqlite_audit(
        transaction,
        command,
        audit_sequence_start + 1,
        "target_state_changed",
        Some("executing"),
        "executing",
        &command.executor_snapshot_id,
        "profile_projection_updated",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )?;
    transaction.execute(
        "UPDATE ai_transparency_change_executions
         SET status = 'succeeded', resulting_entitlement_version_id = ?2, finished_at = ?3
         WHERE execution_id = ?1",
        params![
            command.execution_id,
            command.entitlement_version_id,
            now_text()
        ],
    )?;
    transaction.execute(
        "UPDATE ai_transparency_change_requests
         SET status = 'succeeded', updated_at = ?2
         WHERE change_request_id = ?1",
        params![command.change_request_id, now_text()],
    )?;
    insert_sqlite_audit(
        transaction,
        command,
        audit_sequence_start + 2,
        "execution_succeeded",
        Some("executing"),
        "succeeded",
        &command.executor_snapshot_id,
        "execution_succeeded",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )?;
    if command.inject_audit_failure {
        insert_sqlite_audit(
            transaction,
            command,
            audit_sequence_start + 2,
            "execution_succeeded",
            Some("executing"),
            "succeeded",
            &command.executor_snapshot_id,
            "forced_duplicate_audit",
            Some(command.expected_target_version),
            Some(command.desired_next_version),
        )?;
    }
    Ok(InternalChangeCommandOutcome {
        succeeded: true,
        request_status: "succeeded".to_string(),
        reason_code: None,
        target_version: command.desired_next_version,
    })
}

fn acquire_sqlite_target_lock(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO ai_transparency_change_target_locks (target_scope_key, updated_at)
         VALUES (?1, ?2)
         ON CONFLICT(target_scope_key) DO UPDATE SET updated_at = excluded.updated_at",
        params![command.target_scope_key, now_text()],
    )?;
    Ok(())
}

fn sqlite_idempotency_exists(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<bool, rusqlite::Error> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE requester_snapshot_id = ?1 AND idempotency_key = ?2
         )",
        params![command.requester_snapshot_id, command.idempotency_key],
        |row| row.get(0),
    )
}

fn sqlite_inflight_request_exists(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<bool, rusqlite::Error> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE target_scope_key = ?1 AND status IN ('pending_review', 'approved', 'executing')
         )",
        [&command.target_scope_key],
        |row| row.get(0),
    )
}

fn sqlite_execution_exists(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<bool, rusqlite::Error> {
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_executions
            WHERE execution_id = ?1 OR change_request_id = ?2
         )",
        params![command.execution_id, command.change_request_id],
        |row| row.get(0),
    )
}

fn sqlite_projection_version(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
) -> Result<Option<i32>, rusqlite::Error> {
    transaction
        .query_row(
            "SELECT current_version FROM ai_profile_entitlements
             WHERE license_id = ?1 AND profile_id = ?2",
            params![command.license_id, command.profile_id],
            |row| row.get(0),
        )
        .optional()
}

fn insert_sqlite_request(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
    status: &str,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version,
            desired_next_version, desired_state_json, request_reason,
            contract_reference, legal_review_reference, security_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            evidence_quality, production_eligibility, created_at, updated_at
         ) VALUES (?1, ?2, 'profile_entitlement', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, 'internal change command', ?11, ?12, ?13, ?14,
            'hs-ai-change-request-digest-v1', ?15, ?16, ?17, ?18,
            'native_four_eyes', 0, ?19, ?19)",
        params![
            command.change_request_id,
            command.operation,
            command.profile_id,
            command.target_scope_key,
            command.tenant_id,
            command.workspace_id,
            command.environment,
            command.expected_target_version,
            command.desired_next_version,
            desired_state_json(command),
            command.contract_reference,
            command.legal_review_reference,
            command.security_review_reference,
            command.requester_snapshot_id,
            command.request_digest,
            command.idempotency_key,
            status,
            future_text(),
            now_text(),
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_sqlite_audit(
    transaction: &Transaction<'_>,
    command: &InternalChangeCommand,
    sequence: i32,
    event_type: &str,
    from_state: Option<&str>,
    to_state: &str,
    actor_snapshot_id: &str,
    reason_code: &str,
    target_version_before: Option<i32>,
    target_version_after: Option<i32>,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO ai_transparency_change_audit_events (
            audit_event_id, change_request_id, sequence, event_type, from_state, to_state,
            actor_snapshot_id, target_type, target_id, target_version_before,
            target_version_after, reason_code, request_digest, details_json, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'profile_entitlement', ?8, ?9, ?10,
            ?11, ?12, '{}', ?13)",
        params![
            format!("{}-audit-{sequence}", command.change_request_id),
            command.change_request_id,
            sequence,
            event_type,
            from_state,
            to_state,
            actor_snapshot_id,
            command.profile_id,
            target_version_before,
            target_version_after,
            reason_code,
            command.request_digest,
            now_text(),
        ],
    )?;
    Ok(())
}

fn desired_state_json(command: &InternalChangeCommand) -> String {
    serde_json::json!({
        "status": command.desired_status,
        "termsVersion": command.terms_version,
        "expiresAt": future_text(),
    })
    .to_string()
}

fn failed_outcome(reason_code: &str, target_version: i32) -> InternalChangeCommandOutcome {
    InternalChangeCommandOutcome {
        succeeded: false,
        request_status: "conflict".to_string(),
        reason_code: Some(reason_code.to_string()),
        target_version,
    }
}

fn now_text() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn future_text() -> String {
    (Utc::now() + chrono::Duration::days(365)).to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(feature = "postgres")]
pub async fn execute_postgres_change_command(
    connection: &mut sqlx::PgConnection,
    command: &InternalChangeCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    use sqlx::Connection as _;

    if let Some(outcome) = validate_preflight(command, preflight) {
        return Ok(outcome);
    }
    let mut transaction = connection.begin().await?;
    let outcome = match command.mode {
        InternalChangeCommandMode::SubmitRequest => {
            submit_postgres_request(&mut transaction, command).await
        }
        InternalChangeCommandMode::ApplyProfileChange => {
            apply_postgres_profile_change(&mut transaction, command, false).await
        }
        InternalChangeCommandMode::ExecuteApprovedRequest => {
            apply_postgres_profile_change(&mut transaction, command, true).await
        }
    };
    match outcome {
        Ok(outcome) if outcome.succeeded => {
            transaction.commit().await?;
            Ok(outcome)
        }
        Ok(outcome) => {
            transaction.rollback().await?;
            Ok(outcome)
        }
        Err(_) if command.inject_audit_failure => {
            transaction.rollback().await?;
            Ok(failed_outcome(
                REASON_AUDIT_WRITE_FAILED,
                command.expected_target_version,
            ))
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

#[cfg(feature = "postgres")]
async fn submit_postgres_request(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    acquire_postgres_target_lock(transaction, command).await?;
    if postgres_idempotency_exists(transaction, command).await? {
        return Ok(failed_outcome(
            REASON_IDEMPOTENCY_REPLAY,
            postgres_projection_version(transaction, command)
                .await?
                .unwrap_or(0),
        ));
    }
    if postgres_inflight_request_exists(transaction, command).await? {
        return Ok(failed_outcome(
            REASON_CONFLICTING_REQUEST_EXISTS,
            postgres_projection_version(transaction, command)
                .await?
                .unwrap_or(0),
        ));
    }
    insert_postgres_request(transaction, command, "pending_review").await?;
    insert_postgres_audit(
        transaction,
        command,
        1,
        "change_request_submitted",
        None,
        "pending_review",
        &command.requester_snapshot_id,
        "request_submitted",
        None,
        None,
    )
    .await?;
    Ok(InternalChangeCommandOutcome {
        succeeded: true,
        request_status: "pending_review".to_string(),
        reason_code: None,
        target_version: postgres_projection_version(transaction, command)
            .await?
            .unwrap_or(0),
    })
}

#[cfg(feature = "postgres")]
async fn apply_postgres_profile_change(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
    execute_existing_request: bool,
) -> Result<InternalChangeCommandOutcome, InternalChangeCommandError> {
    acquire_postgres_target_lock(transaction, command).await?;
    if execute_existing_request {
        if postgres_execution_exists(transaction, command).await? {
            return Ok(failed_outcome(
                REASON_TARGET_STATE_CONFLICT,
                postgres_projection_version(transaction, command)
                    .await?
                    .unwrap_or(0),
            ));
        }
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM ai_transparency_change_requests WHERE change_request_id = $1",
        )
        .bind(&command.change_request_id)
        .fetch_optional(&mut **transaction)
        .await?;
        if status.as_deref() != Some("approved") {
            return Ok(failed_outcome(
                REASON_TARGET_STATE_CONFLICT,
                postgres_projection_version(transaction, command)
                    .await?
                    .unwrap_or(0),
            ));
        }
    } else {
        if postgres_idempotency_exists(transaction, command).await? {
            return Ok(failed_outcome(
                REASON_IDEMPOTENCY_REPLAY,
                postgres_projection_version(transaction, command)
                    .await?
                    .unwrap_or(0),
            ));
        }
        if postgres_inflight_request_exists(transaction, command).await? {
            return Ok(failed_outcome(
                REASON_CONFLICTING_REQUEST_EXISTS,
                postgres_projection_version(transaction, command)
                    .await?
                    .unwrap_or(0),
            ));
        }
    }
    let current_version = postgres_projection_version(transaction, command)
        .await?
        .unwrap_or(0);
    if current_version != command.expected_target_version {
        return Ok(failed_outcome(
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }

    if !execute_existing_request {
        insert_postgres_request(transaction, command, "approved").await?;
        insert_postgres_audit(
            transaction,
            command,
            1,
            "change_request_submitted",
            None,
            "pending_review",
            &command.requester_snapshot_id,
            "request_submitted",
            None,
            None,
        )
        .await?;
        sqlx::query(
            "INSERT INTO ai_transparency_change_approvals (
                approval_id, change_request_id, decision, approver_snapshot_id,
                requester_actor_id, approver_actor_id, approver_role, decision_reason,
                policy_version, request_digest, decided_at
             ) VALUES ($1, $2, 'approved', $3, $4, $5, $6,
                'approved by internal maker-checker', 'ai-transparency-approval-v1', $7, NOW())",
        )
        .bind(&command.approval_id)
        .bind(&command.change_request_id)
        .bind(&command.approver_snapshot_id)
        .bind(&command.requester_actor_id)
        .bind(&command.approver_actor_id)
        .bind(&command.approver_role)
        .bind(&command.request_digest)
        .execute(&mut **transaction)
        .await?;
        insert_postgres_audit(
            transaction,
            command,
            2,
            "approval_granted",
            Some("pending_review"),
            "approved",
            &command.approver_snapshot_id,
            "approval_granted",
            None,
            None,
        )
        .await?;
    }

    sqlx::query(
        "INSERT INTO ai_transparency_change_executions (
            execution_id, change_request_id, executor_snapshot_id, status,
            target_version_before, target_version_after, started_at
         ) VALUES ($1, $2, $3, 'executing', $4, $5, NOW())",
    )
    .bind(&command.execution_id)
    .bind(&command.change_request_id)
    .bind(&command.executor_snapshot_id)
    .bind(command.expected_target_version)
    .bind(command.desired_next_version)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_requests
         SET status = 'executing', updated_at = NOW()
         WHERE change_request_id = $1",
    )
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    insert_postgres_audit(
        transaction,
        command,
        3,
        "execution_started",
        Some("approved"),
        "executing",
        &command.executor_snapshot_id,
        "execution_started",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )
    .await?;

    let previous_version_id: Option<String> = sqlx::query_scalar(
        "SELECT current_version_id FROM ai_profile_entitlements
         WHERE license_id = $1 AND profile_id = $2",
    )
    .bind(&command.license_id)
    .bind(&command.profile_id)
    .fetch_optional(&mut **transaction)
    .await?
    .flatten();
    sqlx::query(
        "UPDATE ai_profile_entitlement_versions
         SET status = 'superseded', superseded_at = NOW()
         WHERE license_id = $1 AND profile_id = $2 AND status = 'active'",
    )
    .bind(&command.license_id)
    .bind(&command.profile_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version,
            previous_version_id, profile_kind, status, effective_at, expires_at,
            terms_version, legal_review_reference, security_review_reference,
            source_change_request_id, created_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW() + INTERVAL '365 days',
            $8, $9, $10, $11, NOW())",
    )
    .bind(&command.entitlement_version_id)
    .bind(&command.license_id)
    .bind(&command.profile_id)
    .bind(command.desired_next_version)
    .bind(previous_version_id)
    .bind(&command.profile_kind)
    .bind(&command.desired_status)
    .bind(&command.terms_version)
    .bind(&command.legal_review_reference)
    .bind(&command.security_review_reference)
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    let projection = sqlx::query(
        "UPDATE ai_profile_entitlements
         SET profile_kind = $3, status = $4, effective_at = NOW(),
             expires_at = NOW() + INTERVAL '365 days', terms_version = $5,
             approved_by = $6, updated_at = NOW(), current_version_id = $7,
             current_version = $8, projection_updated_at = NOW()
         WHERE license_id = $1 AND profile_id = $2 AND current_version = $9",
    )
    .bind(&command.license_id)
    .bind(&command.profile_id)
    .bind(&command.profile_kind)
    .bind(&command.desired_status)
    .bind(&command.terms_version)
    .bind(&command.approver_actor_id)
    .bind(&command.entitlement_version_id)
    .bind(command.desired_next_version)
    .bind(command.expected_target_version)
    .execute(&mut **transaction)
    .await?;
    if projection.rows_affected() != 1 {
        return Ok(failed_outcome(
            REASON_TARGET_VERSION_CONFLICT,
            postgres_projection_version(transaction, command)
                .await?
                .unwrap_or(0),
        ));
    }
    insert_postgres_audit(
        transaction,
        command,
        4,
        "target_state_changed",
        Some("executing"),
        "executing",
        &command.executor_snapshot_id,
        "profile_projection_updated",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_executions
         SET status = 'succeeded', resulting_entitlement_version_id = $2, finished_at = NOW()
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .bind(&command.entitlement_version_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_requests
         SET status = 'succeeded', updated_at = NOW()
         WHERE change_request_id = $1",
    )
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    insert_postgres_audit(
        transaction,
        command,
        5,
        "execution_succeeded",
        Some("executing"),
        "succeeded",
        &command.executor_snapshot_id,
        "execution_succeeded",
        Some(command.expected_target_version),
        Some(command.desired_next_version),
    )
    .await?;
    if command.inject_audit_failure {
        insert_postgres_audit(
            transaction,
            command,
            5,
            "execution_succeeded",
            Some("executing"),
            "succeeded",
            &command.executor_snapshot_id,
            "forced_duplicate_audit",
            Some(command.expected_target_version),
            Some(command.desired_next_version),
        )
        .await?;
    }
    Ok(InternalChangeCommandOutcome {
        succeeded: true,
        request_status: "succeeded".to_string(),
        reason_code: None,
        target_version: command.desired_next_version,
    })
}

#[cfg(feature = "postgres")]
async fn acquire_postgres_target_lock(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_change_target_locks (target_scope_key, updated_at)
         VALUES ($1, NOW()) ON CONFLICT(target_scope_key) DO NOTHING",
    )
    .bind(&command.target_scope_key)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "SELECT target_scope_key FROM ai_transparency_change_target_locks
         WHERE target_scope_key = $1 FOR UPDATE",
    )
    .bind(&command.target_scope_key)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn postgres_idempotency_exists(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE requester_snapshot_id = $1 AND idempotency_key = $2
         )",
    )
    .bind(&command.requester_snapshot_id)
    .bind(&command.idempotency_key)
    .fetch_one(&mut **transaction)
    .await
}

#[cfg(feature = "postgres")]
async fn postgres_inflight_request_exists(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE target_scope_key = $1 AND status IN ('pending_review', 'approved', 'executing')
         )",
    )
    .bind(&command.target_scope_key)
    .fetch_one(&mut **transaction)
    .await
}

#[cfg(feature = "postgres")]
async fn postgres_execution_exists(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_executions
            WHERE execution_id = $1 OR change_request_id = $2
         )",
    )
    .bind(&command.execution_id)
    .bind(&command.change_request_id)
    .fetch_one(&mut **transaction)
    .await
}

#[cfg(feature = "postgres")]
async fn postgres_projection_version(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT current_version FROM ai_profile_entitlements
         WHERE license_id = $1 AND profile_id = $2",
    )
    .bind(&command.license_id)
    .bind(&command.profile_id)
    .fetch_optional(&mut **transaction)
    .await
}

#[cfg(feature = "postgres")]
async fn insert_postgres_request(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version,
            desired_next_version, desired_state_json, request_reason,
            contract_reference, legal_review_reference, security_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            evidence_quality, production_eligibility, created_at, updated_at
         ) VALUES ($1, $2, 'profile_entitlement', $3, $4, $5, $6, $7, $8, $9,
            $10::jsonb, 'internal change command', $11, $12, $13, $14,
            'hs-ai-change-request-digest-v1', $15, $16, $17, NOW() + INTERVAL '1 day',
            'native_four_eyes', FALSE, NOW(), NOW())",
    )
    .bind(&command.change_request_id)
    .bind(&command.operation)
    .bind(&command.profile_id)
    .bind(&command.target_scope_key)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.expected_target_version)
    .bind(command.desired_next_version)
    .bind(desired_state_json(command))
    .bind(&command.contract_reference)
    .bind(&command.legal_review_reference)
    .bind(&command.security_review_reference)
    .bind(&command.requester_snapshot_id)
    .bind(&command.request_digest)
    .bind(&command.idempotency_key)
    .bind(status)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
async fn insert_postgres_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalChangeCommand,
    sequence: i32,
    event_type: &str,
    from_state: Option<&str>,
    to_state: &str,
    actor_snapshot_id: &str,
    reason_code: &str,
    target_version_before: Option<i32>,
    target_version_after: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_change_audit_events (
            audit_event_id, change_request_id, sequence, event_type, from_state, to_state,
            actor_snapshot_id, target_type, target_id, target_version_before,
            target_version_after, reason_code, request_digest, details_json, occurred_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'profile_entitlement', $8, $9, $10,
            $11, $12, '{}'::jsonb, NOW())",
    )
    .bind(format!("{}-audit-{sequence}", command.change_request_id))
    .bind(&command.change_request_id)
    .bind(sequence)
    .bind(event_type)
    .bind(from_state)
    .bind(to_state)
    .bind(actor_snapshot_id)
    .bind(&command.profile_id)
    .bind(target_version_before)
    .bind(target_version_after)
    .bind(reason_code)
    .bind(&command.request_digest)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
