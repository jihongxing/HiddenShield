use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde_json::{json, Value};
use sha2::Sha256;
use sqlx::{Connection, PgConnection, Row};
use std::{fmt, sync::Arc};

use crate::ai_transparency_change_command::{
    ActorAuthorizationInput, InternalIamAuthorizationAdapter,
};
use crate::ai_transparency_production_provider::{
    ProductionCustodyOperation, ProductionProviderReadiness,
};
pub const REASON_CREDENTIAL_UNAUTHORIZED: &str = "ai_credential_unauthorized";
pub const REASON_CREDENTIAL_INACTIVE: &str = "ai_credential_inactive";
pub const REASON_CREDENTIAL_EXPIRED: &str = "ai_credential_expired";
pub const REASON_CREDENTIAL_SCOPE_DENIED: &str = "ai_credential_scope_denied";
pub const REASON_CREDENTIAL_ENVIRONMENT_MISMATCH: &str = "ai_credential_environment_mismatch";
pub const REASON_CREDENTIAL_ISSUER_MODE_DENIED: &str = "ai_credential_issuer_mode_denied";
pub const REASON_LICENSE_INACTIVE: &str = "ai_license_inactive";
pub const REASON_LICENSE_EXPIRED: &str = "ai_license_expired";
pub const REASON_ENVIRONMENT_MISMATCH: &str = "ai_environment_mismatch";
pub const REASON_PROFILE_NOT_ENTITLED: &str = "ai_profile_not_entitled";
pub const REASON_IDEMPOTENCY_CONFLICT: &str = "ai_idempotency_conflict";

#[derive(Debug, Clone)]
pub struct CustodyAuthorizationInput<'a> {
    pub actor_token_hash: &'a str,
    pub license_id: &'a str,
    pub operation: &'a str,
    pub custody_key_id: &'a str,
}

#[derive(Debug, Clone)]
pub struct CustodyAuthorizationDecision {
    pub authorized: bool,
    pub reason_code: Option<String>,
    pub receipt_id: Option<String>,
}

pub trait ProductionCredentialCustodyAuthorization: Send + Sync {
    fn authorize(&self, input: &CustodyAuthorizationInput<'_>) -> CustodyAuthorizationDecision;
}

pub struct InternalIamReceiptCustodyAuthorization<A> {
    iam: A,
}

impl<A> InternalIamReceiptCustodyAuthorization<A> {
    pub fn new(iam: A) -> Self {
        Self { iam }
    }
}

impl<A> ProductionCredentialCustodyAuthorization for InternalIamReceiptCustodyAuthorization<A>
where
    A: InternalIamAuthorizationAdapter + Send + Sync,
{
    fn authorize(&self, input: &CustodyAuthorizationInput<'_>) -> CustodyAuthorizationDecision {
        let decision = self
            .iam
            .verify_actor_authorization(&ActorAuthorizationInput {
                token_hash: input.actor_token_hash,
                required_role: "ai_transparency_credential_custodian",
                tenant_id: "internal-custody",
                workspace_id: input.license_id,
                environment: "production",
                operation: input.operation,
            });
        CustodyAuthorizationDecision {
            authorized: decision.authorized,
            reason_code: decision.reason_code,
            receipt_id: decision.verification_receipt_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PepperMaterial {
    pub key_id: String,
    pub version: String,
    pub secret: String,
}

pub trait KmsHsmPepperProvider: Send + Sync {
    fn active_pepper(&self) -> Result<PepperMaterial, CredentialCustodyError>;
    fn pepper_for_version(&self, version: &str) -> Result<PepperMaterial, CredentialCustodyError>;
}

#[derive(Clone)]
pub struct CredentialCustodyConfig {
    pub custody_key_id: String,
    pub active_pepper: PepperMaterial,
    pub retained_peppers: Vec<PepperMaterial>,
    pub provider_readiness: Arc<dyn ProductionProviderReadiness>,
}

impl fmt::Debug for CredentialCustodyConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialCustodyConfig")
            .field("custody_key_id", &self.custody_key_id)
            .field("active_pepper", &self.active_pepper)
            .field("retained_peppers", &self.retained_peppers)
            .field("provider_readiness", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct IssueProductionCredentialCommand {
    pub credential_id: String,
    pub api_key_id: String,
    pub license_id: String,
    pub scopes: Vec<String>,
    pub issuer_modes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub actor_token_hash: String,
    pub audit_event_id: String,
}

#[derive(Debug, Clone)]
pub struct IssuedProductionCredential {
    pub credential_id: String,
    pub cleartext_api_key: String,
    pub key_prefix: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateReadyMarkingSessionCommand {
    pub marking_session_id: String,
    pub cleartext_api_key: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub idempotency_key: String,
    pub requested_profile_ids: Vec<String>,
    pub claim_type: String,
    pub provider_content_id: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub audit_event_id: String,
}

#[derive(Debug, Clone)]
pub struct RotateProductionCredentialCommand {
    pub new_credential_id: String,
    pub new_api_key_id: String,
    pub previous_credential_id: String,
    pub actor_token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub audit_event_id: String,
}

#[derive(Debug, Clone)]
pub struct RevokeProductionCredentialCommand {
    pub credential_id: String,
    pub actor_token_hash: String,
    pub revoked_reason: String,
    pub audit_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateReadyMarkingSessionOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub marking_session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkingSessionInitialState {
    ReadyToUpload,
    ReadyToConfirm,
}

impl MarkingSessionInitialState {
    fn status(self) -> &'static str {
        match self {
            Self::ReadyToUpload => "ready_to_upload",
            Self::ReadyToConfirm => "ready_to_confirm",
        }
    }

    fn audit_operation(self) -> &'static str {
        match self {
            Self::ReadyToUpload => "create_upload_marking_session",
            Self::ReadyToConfirm => "create_ready_marking_session",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialCustodyError {
    #[error("PostgreSQL credential custody failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("credential custody configuration is invalid")]
    InvalidConfiguration,
    #[error("credential custody authorization failed: {0}")]
    Authorization(String),
    #[error("credential custody provider is unavailable: {0}")]
    ProviderUnavailable(String),
}

pub async fn issue_postgres_production_credential(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    command: &IssueProductionCredentialCommand,
) -> Result<IssuedProductionCredential, CredentialCustodyError> {
    validate_config(config)?;
    ensure_provider_ready(config, ProductionCustodyOperation::IssueCredential)?;
    let decision = authorization.authorize(&CustodyAuthorizationInput {
        actor_token_hash: &command.actor_token_hash,
        license_id: &command.license_id,
        operation: "issue_production_credential",
        custody_key_id: &config.custody_key_id,
    });
    if !decision.authorized {
        return Err(CredentialCustodyError::Authorization(
            decision
                .reason_code
                .unwrap_or_else(|| REASON_CREDENTIAL_UNAUTHORIZED.to_string()),
        ));
    }
    let receipt_id = decision.receipt_id.ok_or_else(|| {
        CredentialCustodyError::Authorization(REASON_CREDENTIAL_UNAUTHORIZED.to_string())
    })?;
    let now = Utc::now();
    let mut transaction = connection.begin().await?;
    let license = sqlx::query(
        "SELECT environment, status, issuer_mode, effective_at, expires_at
         FROM ai_transparency_licenses WHERE license_id = $1 FOR UPDATE",
    )
    .bind(&command.license_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(license) = license else {
        transaction.rollback().await?;
        return Err(CredentialCustodyError::Authorization(
            REASON_LICENSE_INACTIVE.to_string(),
        ));
    };
    validate_license_row(&license, "production", now)
        .map_err(|reason| CredentialCustodyError::Authorization(reason.to_string()))?;
    let issuer_mode: String = license.get("issuer_mode");
    if !command
        .issuer_modes
        .iter()
        .any(|value| value == &issuer_mode)
        || !command.scopes.iter().any(|value| value == "mark:image")
        || command.expires_at <= now
    {
        transaction.rollback().await?;
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_SCOPE_DENIED.to_string(),
        ));
    }
    let active_pepper = active_pepper(config)?;
    let cleartext_api_key = generate_cleartext_key();
    let key_prefix = credential_prefix(&cleartext_api_key)?;
    let key_hash = credential_hash(&cleartext_api_key, &active_pepper)?;
    sqlx::query(
        "INSERT INTO ai_sdk_credential_bindings (
            credential_id, license_id, api_key_id, scopes_json, status, expires_at, created_at,
            key_prefix, key_hash, hash_secret_version, environment, issuer_modes_json,
            custody_key_id, issued_at
         ) VALUES ($1,$2,$3,$4,'active',$5,$6,$7,$8,$9,'production',$10,$11,$6)",
    )
    .bind(&command.credential_id)
    .bind(&command.license_id)
    .bind(&command.api_key_id)
    .bind(json!(command.scopes))
    .bind(command.expires_at)
    .bind(now)
    .bind(&key_prefix)
    .bind(&key_hash)
    .bind(&active_pepper.version)
    .bind(json!(command.issuer_modes))
    .bind(&active_pepper.key_id)
    .execute(&mut *transaction)
    .await?;
    insert_runtime_audit(
        &mut transaction,
        &command.audit_event_id,
        "issue_production_credential",
        &command.credential_id,
        &command.license_id,
        None,
        &receipt_id,
        &active_pepper.key_id,
        json!({"scopes": command.scopes, "issuerModes": command.issuer_modes}),
        now,
    )
    .await?;
    transaction.commit().await?;
    Ok(IssuedProductionCredential {
        credential_id: command.credential_id.clone(),
        cleartext_api_key,
        key_prefix,
        expires_at: command.expires_at,
    })
}

pub async fn create_postgres_ready_marking_session(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    command: &CreateReadyMarkingSessionCommand,
) -> Result<CreateReadyMarkingSessionOutcome, CredentialCustodyError> {
    create_postgres_marking_session(
        connection,
        config,
        command,
        MarkingSessionInitialState::ReadyToConfirm,
    )
    .await
}

pub async fn create_postgres_upload_marking_session(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    command: &CreateReadyMarkingSessionCommand,
) -> Result<CreateReadyMarkingSessionOutcome, CredentialCustodyError> {
    create_postgres_marking_session(
        connection,
        config,
        command,
        MarkingSessionInitialState::ReadyToUpload,
    )
    .await
}

pub async fn create_postgres_marking_session(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    command: &CreateReadyMarkingSessionCommand,
    initial_state: MarkingSessionInitialState,
) -> Result<CreateReadyMarkingSessionOutcome, CredentialCustodyError> {
    validate_config(config)?;
    ensure_provider_ready(config, ProductionCustodyOperation::CreateMarkingSession)?;
    if command.environment != "production" {
        return Ok(rejected(REASON_ENVIRONMENT_MISMATCH));
    }
    let key_prefix = match credential_prefix(&command.cleartext_api_key) {
        Ok(value) => value,
        Err(_) => return Ok(rejected(REASON_CREDENTIAL_UNAUTHORIZED)),
    };
    let now = Utc::now();
    let mut transaction = connection.begin().await?;
    let credential = sqlx::query(
        "SELECT credential_id, license_id, key_hash, hash_secret_version, environment,
                scopes_json, issuer_modes_json, status, expires_at, custody_key_id
         FROM ai_sdk_credential_bindings
         WHERE key_prefix = $1
         FOR UPDATE",
    )
    .bind(&key_prefix)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(credential) = credential else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_UNAUTHORIZED));
    };
    let stored_version: Option<String> = credential.get("hash_secret_version");
    let stored_key_id: Option<String> = credential.get("custody_key_id");
    let Some(stored_version) = stored_version else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_UNAUTHORIZED));
    };
    let pepper = pepper_for_version(config, &stored_version)?;
    let presented_hash = credential_hash(&command.cleartext_api_key, &pepper)?;
    let stored_hash: Option<String> = credential.get("key_hash");
    if stored_hash
        .as_deref()
        .is_none_or(|value| !constant_time_equal(value.as_bytes(), presented_hash.as_bytes()))
        || stored_key_id.as_deref() != Some(pepper.key_id.as_str())
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    if credential.get::<String, _>("status") != "active" {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_INACTIVE));
    }
    if credential
        .get::<Option<String>, _>("environment")
        .as_deref()
        != Some("production")
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_ENVIRONMENT_MISMATCH));
    }
    if credential
        .get::<Option<DateTime<Utc>>, _>("expires_at")
        .is_some_and(|expires_at| expires_at <= now)
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_EXPIRED));
    }
    let scopes: Value = credential.get("scopes_json");
    if !json_array_contains(&scopes, "mark:image") {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_SCOPE_DENIED));
    }
    let credential_id: String = credential.get("credential_id");
    let license_id: String = credential.get("license_id");
    let license = sqlx::query(
        "SELECT tenant_id, workspace_id, environment, status, issuer_mode, effective_at, expires_at
         FROM ai_transparency_licenses WHERE license_id = $1 FOR UPDATE",
    )
    .bind(&license_id)
    .fetch_one(&mut *transaction)
    .await?;
    if let Err(reason) = validate_license_row(&license, "production", now) {
        transaction.rollback().await?;
        return Ok(rejected(reason));
    }
    if license.get::<String, _>("tenant_id") != command.tenant_id
        || license.get::<String, _>("workspace_id") != command.workspace_id
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_ENVIRONMENT_MISMATCH));
    }
    let issuer_modes: Value = credential.get("issuer_modes_json");
    if !json_array_contains(&issuer_modes, &license.get::<String, _>("issuer_mode")) {
        transaction.rollback().await?;
        return Ok(rejected(REASON_CREDENTIAL_ISSUER_MODE_DENIED));
    }
    for profile_id in &command.requested_profile_ids {
        let entitled: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM ai_profile_entitlements
                WHERE license_id = $1 AND profile_id = $2 AND status = 'active'
                  AND effective_at <= $3 AND expires_at > $3
            )",
        )
        .bind(&license_id)
        .bind(profile_id)
        .bind(now)
        .fetch_one(&mut *transaction)
        .await?;
        if !entitled {
            transaction.rollback().await?;
            return Ok(rejected(REASON_PROFILE_NOT_ENTITLED));
        }
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM ai_marking_sessions WHERE license_id = $1 AND idempotency_key = $2
        )",
    )
    .bind(&license_id)
    .bind(&command.idempotency_key)
    .fetch_one(&mut *transaction)
    .await?;
    if exists {
        transaction.rollback().await?;
        return Ok(rejected(REASON_IDEMPOTENCY_CONFLICT));
    }
    sqlx::query(
        "INSERT INTO ai_marking_sessions (
            marking_session_id, license_id, tenant_id, workspace_id, environment,
            idempotency_key, requested_profile_ids_json, claim_type, provider_content_id,
            status, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,'production',$5,$6,$7,$8,$9,$10,$11,$11)",
    )
    .bind(&command.marking_session_id)
    .bind(&license_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.idempotency_key)
    .bind(json!(command.requested_profile_ids))
    .bind(&command.claim_type)
    .bind(&command.provider_content_id)
    .bind(initial_state.status())
    .bind(command.expires_at)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("UPDATE ai_sdk_credential_bindings SET last_used_at = $1 WHERE credential_id = $2")
        .bind(now)
        .bind(&credential_id)
        .execute(&mut *transaction)
        .await?;
    insert_runtime_audit(
        &mut transaction,
        &command.audit_event_id,
        initial_state.audit_operation(),
        &credential_id,
        &license_id,
        Some(&command.marking_session_id),
        "credential-authenticated",
        &pepper.key_id,
        json!({"requestedProfileIds": command.requested_profile_ids}),
        now,
    )
    .await?;
    transaction.commit().await?;
    Ok(CreateReadyMarkingSessionOutcome {
        succeeded: true,
        reason_code: None,
        marking_session_id: Some(command.marking_session_id.clone()),
    })
}

pub async fn rotate_postgres_production_credential(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    command: &RotateProductionCredentialCommand,
) -> Result<IssuedProductionCredential, CredentialCustodyError> {
    validate_config(config)?;
    ensure_provider_ready(config, ProductionCustodyOperation::RotateCredential)?;
    let license_id: Option<String> = sqlx::query_scalar(
        "SELECT license_id FROM ai_sdk_credential_bindings WHERE credential_id = $1",
    )
    .bind(&command.previous_credential_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(license_id) = license_id else {
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_UNAUTHORIZED.to_string(),
        ));
    };
    let decision = authorize_custody(
        authorization,
        &command.actor_token_hash,
        &license_id,
        "rotate_production_credential",
        &config.custody_key_id,
    )?;
    let now = Utc::now();
    let active_pepper = active_pepper(config)?;
    let mut transaction = connection.begin().await?;
    let previous = sqlx::query(
        "SELECT license_id, api_key_id, scopes_json, issuer_modes_json, status, environment
         FROM ai_sdk_credential_bindings WHERE credential_id = $1 FOR UPDATE",
    )
    .bind(&command.previous_credential_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(previous) = previous else {
        transaction.rollback().await?;
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_UNAUTHORIZED.to_string(),
        ));
    };
    if previous.get::<String, _>("status") != "active"
        || previous.get::<Option<String>, _>("environment").as_deref() != Some("production")
        || command.expires_at <= now
    {
        transaction.rollback().await?;
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_INACTIVE.to_string(),
        ));
    }
    let cleartext_api_key = generate_cleartext_key();
    let key_prefix = credential_prefix(&cleartext_api_key)?;
    let key_hash = credential_hash(&cleartext_api_key, &active_pepper)?;
    let scopes: Value = previous.get("scopes_json");
    let issuer_modes: Value = previous.get("issuer_modes_json");
    sqlx::query(
        "INSERT INTO ai_sdk_credential_bindings (
            credential_id, license_id, api_key_id, scopes_json, status, expires_at, created_at,
            key_prefix, key_hash, hash_secret_version, environment, issuer_modes_json,
            custody_key_id, issued_at, rotated_from_credential_id
         ) VALUES ($1,$2,$3,$4,'active',$5,$6,$7,$8,$9,'production',$10,$11,$6,$12)",
    )
    .bind(&command.new_credential_id)
    .bind(&license_id)
    .bind(&command.new_api_key_id)
    .bind(scopes)
    .bind(command.expires_at)
    .bind(now)
    .bind(&key_prefix)
    .bind(&key_hash)
    .bind(&active_pepper.version)
    .bind(issuer_modes)
    .bind(&active_pepper.key_id)
    .bind(&command.previous_credential_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_sdk_credential_bindings
         SET status = 'revoked', revoked_at = $1, revoked_reason = 'rotated',
             rotated_at = $1
         WHERE credential_id = $2 AND status = 'active'",
    )
    .bind(now)
    .bind(&command.previous_credential_id)
    .execute(&mut *transaction)
    .await?;
    insert_lifecycle_audit(
        &mut transaction,
        &command.audit_event_id,
        "rotate_production_credential",
        Some(&command.previous_credential_id),
        Some(&command.new_credential_id),
        &license_id,
        &decision,
        &active_pepper.key_id,
        Some("rotated"),
        now,
    )
    .await?;
    transaction.commit().await?;
    Ok(IssuedProductionCredential {
        credential_id: command.new_credential_id.clone(),
        cleartext_api_key,
        key_prefix,
        expires_at: command.expires_at,
    })
}

pub async fn revoke_postgres_production_credential(
    connection: &mut PgConnection,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    command: &RevokeProductionCredentialCommand,
) -> Result<(), CredentialCustodyError> {
    validate_config(config)?;
    ensure_provider_ready(config, ProductionCustodyOperation::RevokeCredential)?;
    let license_id: Option<String> = sqlx::query_scalar(
        "SELECT license_id FROM ai_sdk_credential_bindings WHERE credential_id = $1",
    )
    .bind(&command.credential_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(license_id) = license_id else {
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_UNAUTHORIZED.to_string(),
        ));
    };
    let decision = authorize_custody(
        authorization,
        &command.actor_token_hash,
        &license_id,
        "revoke_production_credential",
        &config.custody_key_id,
    )?;
    let now = Utc::now();
    let mut transaction = connection.begin().await?;
    let updated = sqlx::query(
        "UPDATE ai_sdk_credential_bindings
         SET status = 'revoked', revoked_at = $1, revoked_reason = $2
         WHERE credential_id = $3 AND status = 'active' AND environment = 'production'",
    )
    .bind(now)
    .bind(&command.revoked_reason)
    .bind(&command.credential_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        transaction.rollback().await?;
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_INACTIVE.to_string(),
        ));
    }
    insert_lifecycle_audit(
        &mut transaction,
        &command.audit_event_id,
        "revoke_production_credential",
        Some(&command.credential_id),
        None,
        &license_id,
        &decision,
        &config.custody_key_id,
        Some(&command.revoked_reason),
        now,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

fn authorize_custody(
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    actor_token_hash: &str,
    license_id: &str,
    operation: &str,
    custody_key_id: &str,
) -> Result<String, CredentialCustodyError> {
    let decision = authorization.authorize(&CustodyAuthorizationInput {
        actor_token_hash,
        license_id,
        operation,
        custody_key_id,
    });
    if !decision.authorized {
        return Err(CredentialCustodyError::Authorization(
            decision
                .reason_code
                .unwrap_or_else(|| REASON_CREDENTIAL_UNAUTHORIZED.to_string()),
        ));
    }
    decision.receipt_id.ok_or_else(|| {
        CredentialCustodyError::Authorization(REASON_CREDENTIAL_UNAUTHORIZED.to_string())
    })
}

fn validate_config(config: &CredentialCustodyConfig) -> Result<(), CredentialCustodyError> {
    if config.custody_key_id.trim().is_empty()
        || config.active_pepper.secret.trim().len() < 32
        || config.active_pepper.version.trim().is_empty()
        || config.active_pepper.key_id.trim().is_empty()
    {
        return Err(CredentialCustodyError::InvalidConfiguration);
    }
    Ok(())
}

fn ensure_provider_ready(
    config: &CredentialCustodyConfig,
    operation: ProductionCustodyOperation,
) -> Result<(), CredentialCustodyError> {
    config
        .provider_readiness
        .ensure_ready(operation)
        .map_err(|error| CredentialCustodyError::ProviderUnavailable(error.to_string()))
}

async fn insert_lifecycle_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    audit_event_id: &str,
    operation: &str,
    previous_credential_id: Option<&str>,
    resulting_credential_id: Option<&str>,
    license_id: &str,
    receipt_id: &str,
    custody_key_id: &str,
    reason: Option<&str>,
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_credential_lifecycle_audit_events (
            audit_event_id, operation, previous_credential_id, resulting_credential_id,
            license_id, custody_authorization_receipt_id, custody_key_id, reason, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(audit_event_id)
    .bind(operation)
    .bind(previous_credential_id)
    .bind(resulting_credential_id)
    .bind(license_id)
    .bind(receipt_id)
    .bind(custody_key_id)
    .bind(reason)
    .bind(occurred_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn validate_license_row(
    row: &sqlx::postgres::PgRow,
    environment: &str,
    now: DateTime<Utc>,
) -> Result<(), &'static str> {
    if row.get::<String, _>("environment") != environment {
        return Err(REASON_ENVIRONMENT_MISMATCH);
    }
    if row.get::<String, _>("status") != "active" {
        return Err(REASON_LICENSE_INACTIVE);
    }
    if row.get::<DateTime<Utc>, _>("effective_at") > now
        || row.get::<DateTime<Utc>, _>("expires_at") <= now
    {
        return Err(REASON_LICENSE_EXPIRED);
    }
    Ok(())
}

fn generate_cleartext_key() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!(
        "hsai_live_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

pub(crate) fn credential_prefix(cleartext: &str) -> Result<String, CredentialCustodyError> {
    if !cleartext.starts_with("hsai_live_") || cleartext.len() < 22 {
        return Err(CredentialCustodyError::Authorization(
            REASON_CREDENTIAL_UNAUTHORIZED.to_string(),
        ));
    }
    Ok(cleartext.chars().take(22).collect())
}

pub(crate) fn credential_hash(
    cleartext: &str,
    pepper: &PepperMaterial,
) -> Result<String, CredentialCustodyError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(pepper.secret.as_bytes())
        .map_err(|_| CredentialCustodyError::InvalidConfiguration)?;
    mac.update(cleartext.as_bytes());
    Ok(format!(
        "hmac-sha256:v1:{}:{}",
        pepper.version,
        hex_string(&mac.finalize().into_bytes())
    ))
}

fn active_pepper(
    config: &CredentialCustodyConfig,
) -> Result<PepperMaterial, CredentialCustodyError> {
    validate_config(config)?;
    Ok(config.active_pepper.clone())
}

pub(crate) fn pepper_for_version(
    config: &CredentialCustodyConfig,
    version: &str,
) -> Result<PepperMaterial, CredentialCustodyError> {
    validate_config(config)?;
    if config.active_pepper.version == version {
        return Ok(config.active_pepper.clone());
    }
    config
        .retained_peppers
        .iter()
        .find(|pepper| pepper.version == version)
        .cloned()
        .ok_or(CredentialCustodyError::InvalidConfiguration)
}

pub(crate) fn json_array_contains(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

pub(crate) fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn insert_runtime_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    audit_event_id: &str,
    operation: &str,
    credential_id: &str,
    license_id: &str,
    marking_session_id: Option<&str>,
    receipt_id: &str,
    custody_key_id: &str,
    details: Value,
    occurred_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_runtime_credential_audit_events (
            audit_event_id, operation, credential_id, license_id, marking_session_id,
            custody_authorization_receipt_id, custody_key_id, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(audit_event_id)
    .bind(operation)
    .bind(credential_id)
    .bind(license_id)
    .bind(marking_session_id)
    .bind(receipt_id)
    .bind(custody_key_id)
    .bind(details)
    .bind(occurred_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn rejected(reason_code: &'static str) -> CreateReadyMarkingSessionOutcome {
    CreateReadyMarkingSessionOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        marking_session_id: None,
    }
}

pub fn default_session_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::minutes(30)
}
