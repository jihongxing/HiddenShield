use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgPool, Row};
use uuid::Uuid;

use crate::ai_transparency_confirm_command::{
    execute_in_transaction, ConfirmMarkingCommand, ConfirmMarkingOutcome,
};
use crate::ai_transparency_credential_custody::{
    constant_time_equal, create_postgres_upload_marking_session, credential_hash,
    credential_prefix, json_array_contains, pepper_for_version, CreateReadyMarkingSessionCommand,
    CredentialCustodyConfig, CredentialCustodyError, REASON_CREDENTIAL_EXPIRED,
    REASON_CREDENTIAL_INACTIVE, REASON_CREDENTIAL_ISSUER_MODE_DENIED,
    REASON_CREDENTIAL_SCOPE_DENIED, REASON_CREDENTIAL_UNAUTHORIZED, REASON_ENVIRONMENT_MISMATCH,
    REASON_LICENSE_EXPIRED, REASON_LICENSE_INACTIVE, REASON_PROFILE_NOT_ENTITLED,
};
use crate::ai_transparency_image_marking_executor::{
    prepare_internal_image_marking, InternalImageMarkingCommand,
};

const MAX_PNG_BYTES: usize = 64 * 1024 * 1024;
const PLATFORM_API_BODY_LIMIT: usize = 90 * 1024 * 1024;

#[derive(Clone)]
pub struct AiTransparencyPlatformApiState {
    pub pool: PgPool,
    pub custody: CredentialCustodyConfig,
    pub confirmation_token_secret: Arc<str>,
    pub internal_verification_base_url: Arc<str>,
}

pub fn build_ai_transparency_platform_router(state: AiTransparencyPlatformApiState) -> Router {
    Router::new()
        .route(
            "/v1/ai-transparency/admissions",
            post(admit_production_profile),
        )
        .route(
            "/v1/ai-transparency/sessions",
            post(create_generation_session),
        )
        .route(
            "/v1/ai-transparency/images/mark",
            post(mark_generated_image),
        )
        .route(
            "/v1/ai-transparency/images/confirm",
            post(confirm_generated_image),
        )
        .layer(DefaultBodyLimit::max(PLATFORM_API_BODY_LIMIT))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionRequest {
    license_id: String,
    tenant_id: String,
    workspace_id: String,
    issuer_mode: String,
    regulatory_profile_id: String,
    technical_profile_ids: Vec<String>,
    environment: String,
    media_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionResponse {
    admission_id: String,
    status: &'static str,
    environment: &'static str,
    license_id: String,
    tenant_id: String,
    workspace_id: String,
    issuer_mode: String,
    regulatory_profile_id: String,
    technical_profile_ids: Vec<String>,
    entitlement_version_id: String,
    entitlement_digest: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionRequest {
    admission_id: String,
    idempotency_key: String,
    generation_event_id: String,
    subject_reference: String,
    content_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    marking_session_id: String,
    admission_id: String,
    license_id: String,
    entitlement_digest: String,
    status: &'static str,
    watermark_uid: String,
    content_type: &'static str,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkRequest {
    marking_session_id: String,
    content_type: String,
    original_file_sha256: String,
    image_base64: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MarkResponse {
    marking_session_id: String,
    license_id: String,
    entitlement_digest: String,
    status: &'static str,
    watermark_uid: String,
    content_type: &'static str,
    original_file_sha256: String,
    marked_file_sha256: String,
    marked_image_base64: String,
    confirmation_token: String,
    marker_evidence_digest: String,
    explicit_label_receipt_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    marking_session_id: String,
    confirmation_token: String,
    marked_file_sha256: String,
    idempotency_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmResponse {
    status: &'static str,
    manifest_id: String,
    marking_session_id: String,
    watermark_uid: String,
    verification_url: String,
    profile_status: &'static str,
    explicit_label: ExplicitLabelResponse,
    metering_receipt: MeteringReceiptResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplicitLabelResponse {
    text: String,
    required_surface: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeteringReceiptResponse {
    receipt_id: String,
    ledger_entry_id: String,
    license_id: String,
    marking_session_id: String,
    metering_unit: &'static str,
    quantity: i32,
    ledger_status: &'static str,
    committed_at: DateTime<Utc>,
    replayed: bool,
}

#[derive(Debug)]
struct PlatformApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
}

impl PlatformApiError {
    fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            retryable: false,
        }
    }

    fn unavailable(message: &'static str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "service_unavailable",
            message,
            retryable: true,
        }
    }
}

impl IntoResponse for PlatformApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({
                "errorCode": self.code,
                "message": self.message,
                "retryable": self.retryable,
            })),
        )
            .into_response()
    }
}

struct CredentialAdmission {
    credential_id: String,
    license_id: String,
    expires_at: DateTime<Utc>,
}

struct EntitlementSet {
    version_id: String,
    digest: String,
    expires_at: DateTime<Utc>,
}

async fn admit_production_profile(
    State(state): State<AiTransparencyPlatformApiState>,
    headers: HeaderMap,
    Json(request): Json<AdmissionRequest>,
) -> Result<(StatusCode, Json<AdmissionResponse>), PlatformApiError> {
    require_non_empty(&request.license_id, "admission_invalid")?;
    require_non_empty(&request.tenant_id, "admission_invalid")?;
    require_non_empty(&request.workspace_id, "admission_invalid")?;
    require_non_empty(&request.regulatory_profile_id, "admission_invalid")?;
    if request.environment != "production"
        || request.media_type != "image"
        || request.technical_profile_ids.is_empty()
    {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "admission_invalid",
            "production image admission is required",
        ));
    }
    let mut unique_technical_profiles = request.technical_profile_ids.clone();
    unique_technical_profiles.sort();
    unique_technical_profiles.dedup();
    if unique_technical_profiles.len() != request.technical_profile_ids.len()
        || unique_technical_profiles
            .iter()
            .any(|profile_id| profile_id == &request.regulatory_profile_id)
    {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "admission_invalid",
            "regulatory and technical Profile identities must be distinct and unique",
        ));
    }
    let database_issuer_mode = map_issuer_mode(&request.issuer_mode)?;
    let cleartext_api_key = bearer_credential(&headers)?;
    let mut connection = state
        .pool
        .acquire()
        .await
        .map_err(|_| PlatformApiError::unavailable("PostgreSQL is unavailable"))?;
    let credential = validate_credential_admission(
        &mut connection,
        &state.custody,
        &cleartext_api_key,
        &request,
        database_issuer_mode,
    )
    .await?;
    let mut profile_ids = request.technical_profile_ids.clone();
    profile_ids.push(request.regulatory_profile_id.clone());
    profile_ids.sort();
    profile_ids.dedup();
    let entitlement_set = load_versioned_entitlement_set(
        &mut connection,
        &credential.license_id,
        &request.regulatory_profile_id,
        &request.technical_profile_ids,
        &profile_ids,
    )
    .await?;
    let expires_at = credential.expires_at.min(entitlement_set.expires_at);
    let admission_id = format!("adm_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO ai_platform_profile_admissions (
            admission_id, credential_id, license_id, tenant_id, workspace_id, environment,
            issuer_mode, regulatory_profile_id, technical_profile_ids_json,
            requested_profile_ids_json, entitlement_version_id, entitlement_digest,
            status, expires_at, created_at
         ) VALUES ($1,$2,$3,$4,$5,'production',$6,$7,$8,$9,$10,$11,'admitted',$12,$13)",
    )
    .bind(&admission_id)
    .bind(&credential.credential_id)
    .bind(&credential.license_id)
    .bind(&request.tenant_id)
    .bind(&request.workspace_id)
    .bind(&request.issuer_mode)
    .bind(&request.regulatory_profile_id)
    .bind(json!(request.technical_profile_ids))
    .bind(json!(profile_ids))
    .bind(&entitlement_set.version_id)
    .bind(&entitlement_set.digest)
    .bind(expires_at)
    .bind(now)
    .execute(&mut *connection)
    .await
    .map_err(internal_database_error)?;
    insert_api_audit(
        &mut connection,
        "admit_profile",
        "succeeded",
        Some(&admission_id),
        None,
        Some(&credential.license_id),
        None,
        json!({"entitlementDigest": entitlement_set.digest}),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(AdmissionResponse {
            admission_id,
            status: "admitted",
            environment: "production",
            license_id: credential.license_id,
            tenant_id: request.tenant_id,
            workspace_id: request.workspace_id,
            issuer_mode: request.issuer_mode,
            regulatory_profile_id: request.regulatory_profile_id,
            technical_profile_ids: request.technical_profile_ids,
            entitlement_version_id: entitlement_set.version_id,
            entitlement_digest: entitlement_set.digest,
            expires_at,
        }),
    ))
}

async fn create_generation_session(
    State(state): State<AiTransparencyPlatformApiState>,
    headers: HeaderMap,
    Json(request): Json<SessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), PlatformApiError> {
    require_non_empty(&request.admission_id, "admission_invalid")?;
    require_non_empty(&request.idempotency_key, "session_invalid")?;
    require_non_empty(&request.generation_event_id, "session_invalid")?;
    require_non_empty(&request.subject_reference, "session_invalid")?;
    if request.content_type != "image/png" {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "image_invalid",
            "only image/png is accepted",
        ));
    }
    let cleartext_api_key = bearer_credential(&headers)?;
    let mut connection = state
        .pool
        .acquire()
        .await
        .map_err(|_| PlatformApiError::unavailable("PostgreSQL is unavailable"))?;
    let admission = sqlx::query(
        "SELECT credential_id, license_id, tenant_id, workspace_id, requested_profile_ids_json,
                entitlement_digest, expires_at, status
         FROM ai_platform_profile_admissions WHERE admission_id = $1",
    )
    .bind(&request.admission_id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| {
        PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "admission_invalid",
            "admission is invalid",
        )
    })?;
    let admission_expires_at: DateTime<Utc> = admission.get("expires_at");
    if admission.get::<String, _>("status") != "admitted" || admission_expires_at <= Utc::now() {
        return Err(PlatformApiError::new(
            StatusCode::FORBIDDEN,
            "admission_expired",
            "admission has expired",
        ));
    }
    validate_presented_credential_id(
        &mut connection,
        &state.custody,
        &cleartext_api_key,
        admission.get::<String, _>("credential_id").as_str(),
    )
    .await?;
    let license_id: String = admission.get("license_id");
    if let Some(existing) = load_existing_session(
        &mut connection,
        &license_id,
        &request.idempotency_key,
        &request.admission_id,
    )
    .await?
    {
        return Ok((StatusCode::CREATED, Json(existing)));
    }
    let marking_session_id = format!("ms_{}", Uuid::new_v4().simple());
    let watermark_uid = generate_watermark_uid();
    let expires_at = admission_expires_at.min(Utc::now() + Duration::minutes(30));
    let requested_profile_ids: Vec<String> =
        serde_json::from_value(admission.get("requested_profile_ids_json")).map_err(|_| {
            PlatformApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "stored Profile admission is invalid",
            )
        })?;
    let custody_outcome = create_postgres_upload_marking_session(
        &mut connection,
        &state.custody,
        &CreateReadyMarkingSessionCommand {
            marking_session_id: marking_session_id.clone(),
            cleartext_api_key,
            tenant_id: admission.get("tenant_id"),
            workspace_id: admission.get("workspace_id"),
            environment: "production".to_string(),
            idempotency_key: request.idempotency_key,
            requested_profile_ids,
            claim_type: "ai_generated".to_string(),
            provider_content_id: Some(request.generation_event_id.clone()),
            expires_at,
            audit_event_id: format!("audit-session-{}", Uuid::new_v4().simple()),
        },
    )
    .await
    .map_err(custody_error)?;
    if !custody_outcome.succeeded {
        return Err(reason_error(
            custody_outcome
                .reason_code
                .as_deref()
                .unwrap_or("session_invalid"),
        ));
    }
    let entitlement_digest: String = admission.get("entitlement_digest");
    sqlx::query(
        "INSERT INTO ai_platform_marking_sessions (
            marking_session_id, admission_id, watermark_uid, generation_event_id,
            subject_reference, content_type, entitlement_digest, created_at
         ) VALUES ($1,$2,$3,$4,$5,'image/png',$6,$7)",
    )
    .bind(&marking_session_id)
    .bind(&request.admission_id)
    .bind(&watermark_uid)
    .bind(&request.generation_event_id)
    .bind(&request.subject_reference)
    .bind(&entitlement_digest)
    .bind(Utc::now())
    .execute(&mut *connection)
    .await
    .map_err(internal_database_error)?;
    insert_api_audit(
        &mut connection,
        "create_session",
        "succeeded",
        Some(&request.admission_id),
        Some(&marking_session_id),
        Some(&license_id),
        None,
        json!({"watermarkUid": watermark_uid}),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            marking_session_id,
            admission_id: request.admission_id,
            license_id,
            entitlement_digest,
            status: "ready_to_upload",
            watermark_uid,
            content_type: "image/png",
            expires_at,
        }),
    ))
}

async fn mark_generated_image(
    State(state): State<AiTransparencyPlatformApiState>,
    headers: HeaderMap,
    Json(request): Json<MarkRequest>,
) -> Result<Json<MarkResponse>, PlatformApiError> {
    let cleartext_api_key = bearer_credential(&headers)?;
    if request.content_type != "image/png" || !is_sha256(&request.original_file_sha256) {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "image_invalid",
            "PNG content type and SHA-256 are required",
        ));
    }
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(request.image_base64.as_bytes())
        .map_err(|_| {
            PlatformApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "image_invalid",
                "imageBase64 is invalid",
            )
        })?;
    if image_bytes.len() > MAX_PNG_BYTES
        || !is_png(&image_bytes)
        || sha256_hex(&image_bytes) != request.original_file_sha256
    {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "image_invalid",
            "PNG bytes or original digest are invalid",
        ));
    }
    let mut connection = state
        .pool
        .acquire()
        .await
        .map_err(|_| PlatformApiError::unavailable("PostgreSQL is unavailable"))?;
    let row = sqlx::query(
        "SELECT session.license_id, session.requested_profile_ids_json, session.status,
                session.expires_at, platform_session.admission_id,
                platform_session.watermark_uid, platform_session.entitlement_digest,
                platform_session.generation_event_id, admission.credential_id
         FROM ai_marking_sessions session
         JOIN ai_platform_marking_sessions platform_session
           ON platform_session.marking_session_id = session.marking_session_id
         JOIN ai_platform_profile_admissions admission
           ON admission.admission_id = platform_session.admission_id
         WHERE session.marking_session_id = $1",
    )
    .bind(&request.marking_session_id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| {
        PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "session is not ready_to_upload",
        )
    })?;
    validate_presented_credential_id(
        &mut connection,
        &state.custody,
        &cleartext_api_key,
        row.get::<String, _>("credential_id").as_str(),
    )
    .await?;
    if row.get::<String, _>("status") != "ready_to_upload"
        || row.get::<DateTime<Utc>, _>("expires_at") <= Utc::now()
    {
        return Err(PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "session is not ready_to_upload",
        ));
    }
    let claimed = sqlx::query(
        "UPDATE ai_marking_sessions
         SET status = 'processing', updated_at = NOW()
         WHERE marking_session_id = $1 AND status = 'ready_to_upload' AND expires_at > NOW()",
    )
    .bind(&request.marking_session_id)
    .execute(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .rows_affected();
    if claimed != 1 {
        return Err(PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "session is already being marked",
        ));
    }
    let requested_profile_ids: Vec<String> =
        serde_json::from_value(row.get("requested_profile_ids_json")).map_err(|_| {
            PlatformApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "stored Profile set is invalid",
            )
        })?;
    let execution_id = format!("exec_{}", Uuid::new_v4().simple());
    let watermark_uid: String = row.get("watermark_uid");
    let prepared = match prepare_internal_image_marking(
        &InternalImageMarkingCommand {
            marking_session_id: request.marking_session_id.clone(),
            execution_id: execution_id.clone(),
            watermark_uid: watermark_uid.clone(),
            source_image_bytes: image_bytes,
            provider_id: "internal-platform-api".to_string(),
            system_name: "HiddenShield AI Transparency Platform".to_string(),
            system_version: env!("CARGO_PKG_VERSION").to_string(),
            model_id: None,
            model_version: None,
            generation_mode: "text_to_image".to_string(),
            generated_at: Utc::now(),
            operations: json!([{"type": "generated", "source": "platform_api"}]),
            parent_subjects: json!([]),
        },
        &requested_profile_ids,
    ) {
        Ok(prepared) => prepared,
        Err(_) => {
            sqlx::query(
                "UPDATE ai_marking_sessions
                 SET status = 'failed', updated_at = NOW()
                 WHERE marking_session_id = $1 AND status = 'processing'",
            )
            .bind(&request.marking_session_id)
            .execute(&mut *connection)
            .await
            .map_err(internal_database_error)?;
            return Err(PlatformApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "image_invalid",
                "watermark-core marking or readback failed",
            ));
        }
    };
    let confirmation_token = generate_confirmation_token();
    let confirmation_token_hash =
        token_hash(&state.confirmation_token_secret, &confirmation_token)?;
    let admission_id: String = row.get("admission_id");
    let license_id: String = row.get("license_id");
    let entitlement_digest: String = row.get("entitlement_digest");
    let mut transaction = connection.begin().await.map_err(internal_database_error)?;
    let updated = sqlx::query(
        "UPDATE ai_marking_sessions
         SET status = 'ready_to_confirm', updated_at = NOW()
         WHERE marking_session_id = $1 AND status = 'processing'",
    )
    .bind(&request.marking_session_id)
    .execute(&mut *transaction)
    .await
    .map_err(internal_database_error)?
    .rows_affected();
    if updated != 1 {
        transaction
            .rollback()
            .await
            .map_err(internal_database_error)?;
        return Err(PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "marking session state changed",
        ));
    }
    sqlx::query(
        "INSERT INTO ai_platform_marking_submissions (
            submission_id, marking_session_id, admission_id, license_id, watermark_uid,
            original_file_sha256, marked_file_sha256, confirmation_token_hash,
            marker_evidence_digest, explicit_label_receipt_digest, confirm_command_json,
            status, created_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'ready_to_confirm',$12)",
    )
    .bind(format!("sub_{}", Uuid::new_v4().simple()))
    .bind(&request.marking_session_id)
    .bind(&admission_id)
    .bind(&license_id)
    .bind(&watermark_uid)
    .bind(&prepared.source_image_sha256)
    .bind(&prepared.protected_image_sha256)
    .bind(&confirmation_token_hash)
    .bind(&prepared.marker_evidence_digest)
    .bind(&prepared.explicit_label_receipt_digest)
    .bind(
        serde_json::to_value(&prepared.confirm_command).map_err(|_| {
            PlatformApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "confirm command serialization failed",
            )
        })?,
    )
    .bind(Utc::now())
    .execute(&mut *transaction)
    .await
    .map_err(internal_database_error)?;
    insert_api_audit_tx(
        &mut transaction,
        "mark_image",
        "succeeded",
        Some(&admission_id),
        Some(&request.marking_session_id),
        Some(&license_id),
        None,
        json!({"markedFileSha256": prepared.protected_image_sha256}),
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(internal_database_error)?;
    Ok(Json(MarkResponse {
        marking_session_id: request.marking_session_id,
        license_id,
        entitlement_digest,
        status: "ready_to_confirm",
        watermark_uid,
        content_type: "image/png",
        original_file_sha256: prepared.source_image_sha256,
        marked_file_sha256: prepared.protected_image_sha256,
        marked_image_base64: base64::engine::general_purpose::STANDARD
            .encode(prepared.protected_image_bytes),
        confirmation_token,
        marker_evidence_digest: prepared.marker_evidence_digest,
        explicit_label_receipt_digest: prepared.explicit_label_receipt_digest,
    }))
}

async fn confirm_generated_image(
    State(state): State<AiTransparencyPlatformApiState>,
    headers: HeaderMap,
    Json(request): Json<ConfirmRequest>,
) -> Result<Json<ConfirmResponse>, PlatformApiError> {
    require_non_empty(&request.idempotency_key, "confirm_conflict")?;
    if !is_sha256(&request.marked_file_sha256) {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "marked_image_digest_mismatch",
            "markedFileSha256 is invalid",
        ));
    }
    let cleartext_api_key = bearer_credential(&headers)?;
    let mut connection = state
        .pool
        .acquire()
        .await
        .map_err(|_| PlatformApiError::unavailable("PostgreSQL is unavailable"))?;
    let mut transaction = connection.begin().await.map_err(internal_database_error)?;
    let submission = sqlx::query(
        "SELECT submission.admission_id, submission.license_id, submission.watermark_uid,
                submission.marked_file_sha256, submission.confirmation_token_hash,
                submission.confirm_command_json, submission.confirm_idempotency_key,
                submission.status, admission.credential_id
         FROM ai_platform_marking_submissions submission
         JOIN ai_platform_profile_admissions admission
           ON admission.admission_id = submission.admission_id
         WHERE submission.marking_session_id = $1
         FOR UPDATE",
    )
    .bind(&request.marking_session_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| {
        PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "session_invalid",
            "marking submission is missing",
        )
    })?;
    validate_presented_credential_id_tx(
        &mut transaction,
        &state.custody,
        &cleartext_api_key,
        submission.get::<String, _>("credential_id").as_str(),
    )
    .await?;
    let expected_token_hash = token_hash(
        &state.confirmation_token_secret,
        &request.confirmation_token,
    )?;
    if submission.get::<String, _>("marked_file_sha256") != request.marked_file_sha256
        || !constant_time_equal(
            submission
                .get::<String, _>("confirmation_token_hash")
                .as_bytes(),
            expected_token_hash.as_bytes(),
        )
    {
        transaction
            .rollback()
            .await
            .map_err(internal_database_error)?;
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "marked_image_digest_mismatch",
            "confirmation token or marked digest mismatch",
        ));
    }
    let replayed = submission.get::<String, _>("status") == "confirmed";
    let stored_idempotency_key: Option<String> = submission.get("confirm_idempotency_key");
    if replayed && stored_idempotency_key.as_deref() != Some(request.idempotency_key.as_str()) {
        transaction
            .rollback()
            .await
            .map_err(internal_database_error)?;
        return Err(PlatformApiError::new(
            StatusCode::CONFLICT,
            "confirm_conflict",
            "confirm idempotency key is already bound",
        ));
    }
    let confirm_command: ConfirmMarkingCommand =
        serde_json::from_value(submission.get("confirm_command_json")).map_err(|_| {
            PlatformApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "stored confirm command is invalid",
            )
        })?;
    if !replayed {
        let outcome: ConfirmMarkingOutcome =
            execute_in_transaction(&mut transaction, &confirm_command)
                .await
                .map_err(internal_database_error)?;
        if !outcome.succeeded {
            transaction
                .rollback()
                .await
                .map_err(internal_database_error)?;
            return Err(reason_error(
                outcome.reason_code.as_deref().unwrap_or("confirm_conflict"),
            ));
        }
        sqlx::query(
            "UPDATE ai_platform_marking_submissions
             SET status = 'confirmed', confirmed_at = NOW(), confirm_idempotency_key = $2
             WHERE marking_session_id = $1 AND status = 'ready_to_confirm'",
        )
        .bind(&request.marking_session_id)
        .bind(&request.idempotency_key)
        .execute(&mut *transaction)
        .await
        .map_err(internal_database_error)?;
    }
    let ledger = sqlx::query(
        "SELECT ledger_entry_id, license_id, committed_at
         FROM ai_marking_ledger
         WHERE marking_session_id = $1 AND ledger_status = 'committed'",
    )
    .bind(&request.marking_session_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(internal_database_error)?;
    insert_api_audit_tx(
        &mut transaction,
        "confirm_image",
        if replayed { "replayed" } else { "succeeded" },
        Some(submission.get::<String, _>("admission_id").as_str()),
        Some(&request.marking_session_id),
        Some(submission.get::<String, _>("license_id").as_str()),
        None,
        json!({"manifestId": confirm_command.transparency_manifest_id}),
    )
    .await?;
    transaction
        .commit()
        .await
        .map_err(internal_database_error)?;
    let committed_at: DateTime<Utc> = ledger.get("committed_at");
    let explicit_label = confirm_command
        .explicit_label_receipts
        .first()
        .map(|receipt| ExplicitLabelResponse {
            text: receipt.label_text.clone(),
            required_surface: receipt.required_surface.clone(),
        })
        .unwrap_or(ExplicitLabelResponse {
            text: "AI generated".to_string(),
            required_surface: "platform_ui".to_string(),
        });
    Ok(Json(ConfirmResponse {
        status: "confirmed",
        manifest_id: confirm_command.transparency_manifest_id.clone(),
        marking_session_id: request.marking_session_id.clone(),
        watermark_uid: submission.get("watermark_uid"),
        verification_url: format!(
            "{}/{}",
            state.internal_verification_base_url.trim_end_matches('/'),
            confirm_command.transparency_manifest_id
        ),
        profile_status: "applied",
        explicit_label,
        metering_receipt: MeteringReceiptResponse {
            receipt_id: format!(
                "metering-receipt-{}",
                ledger.get::<String, _>("ledger_entry_id")
            ),
            ledger_entry_id: ledger.get("ledger_entry_id"),
            license_id: ledger.get("license_id"),
            marking_session_id: request.marking_session_id,
            metering_unit: "confirmed_marked_image",
            quantity: 1,
            ledger_status: "committed",
            committed_at,
            replayed,
        },
    }))
}

async fn validate_credential_admission(
    connection: &mut sqlx::PgConnection,
    config: &CredentialCustodyConfig,
    cleartext_api_key: &str,
    request: &AdmissionRequest,
    database_issuer_mode: &str,
) -> Result<CredentialAdmission, PlatformApiError> {
    let key_prefix = credential_prefix(cleartext_api_key)
        .map_err(|_| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    let now = Utc::now();
    let credential = sqlx::query(
        "SELECT credential_id, license_id, key_hash, hash_secret_version, environment,
                scopes_json, issuer_modes_json, status, expires_at, custody_key_id
         FROM ai_sdk_credential_bindings WHERE key_prefix = $1",
    )
    .bind(&key_prefix)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    verify_credential_row(config, cleartext_api_key, &credential, now)?;
    let license_id: String = credential.get("license_id");
    if license_id != request.license_id {
        return Err(reason_error(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    let license = sqlx::query(
        "SELECT tenant_id, workspace_id, environment, status, issuer_mode, effective_at, expires_at
         FROM ai_transparency_licenses WHERE license_id = $1",
    )
    .bind(&license_id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| reason_error(REASON_LICENSE_INACTIVE))?;
    if license.get::<String, _>("environment") != "production"
        || license.get::<String, _>("tenant_id") != request.tenant_id
        || license.get::<String, _>("workspace_id") != request.workspace_id
    {
        return Err(reason_error(REASON_ENVIRONMENT_MISMATCH));
    }
    if license.get::<String, _>("status") != "active" {
        return Err(reason_error(REASON_LICENSE_INACTIVE));
    }
    let license_expires_at: DateTime<Utc> = license.get("expires_at");
    if license.get::<DateTime<Utc>, _>("effective_at") > now || license_expires_at <= now {
        return Err(reason_error(REASON_LICENSE_EXPIRED));
    }
    if license.get::<String, _>("issuer_mode") != database_issuer_mode
        || !json_array_contains(
            &credential.get::<Value, _>("issuer_modes_json"),
            database_issuer_mode,
        )
    {
        return Err(reason_error(REASON_CREDENTIAL_ISSUER_MODE_DENIED));
    }
    let credential_expires_at = credential
        .get::<Option<DateTime<Utc>>, _>("expires_at")
        .unwrap_or(license_expires_at);
    Ok(CredentialAdmission {
        credential_id: credential.get("credential_id"),
        license_id,
        expires_at: credential_expires_at.min(license_expires_at),
    })
}

fn verify_credential_row(
    config: &CredentialCustodyConfig,
    cleartext_api_key: &str,
    credential: &sqlx::postgres::PgRow,
    now: DateTime<Utc>,
) -> Result<(), PlatformApiError> {
    let version: Option<String> = credential.get("hash_secret_version");
    let version = version.ok_or_else(|| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    let pepper = pepper_for_version(config, &version).map_err(custody_error)?;
    let presented_hash = credential_hash(cleartext_api_key, &pepper).map_err(custody_error)?;
    let stored_hash: Option<String> = credential.get("key_hash");
    let custody_key_id: Option<String> = credential.get("custody_key_id");
    if stored_hash
        .as_deref()
        .is_none_or(|stored| !constant_time_equal(stored.as_bytes(), presented_hash.as_bytes()))
        || custody_key_id.as_deref() != Some(pepper.key_id.as_str())
    {
        return Err(reason_error(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    if credential.get::<String, _>("status") != "active" {
        return Err(reason_error(REASON_CREDENTIAL_INACTIVE));
    }
    if credential
        .get::<Option<String>, _>("environment")
        .as_deref()
        != Some("production")
    {
        return Err(reason_error(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    if credential
        .get::<Option<DateTime<Utc>>, _>("expires_at")
        .is_some_and(|expires_at| expires_at <= now)
    {
        return Err(reason_error(REASON_CREDENTIAL_EXPIRED));
    }
    if !json_array_contains(&credential.get::<Value, _>("scopes_json"), "mark:image") {
        return Err(reason_error(REASON_CREDENTIAL_SCOPE_DENIED));
    }
    Ok(())
}

async fn load_versioned_entitlement_set(
    connection: &mut sqlx::PgConnection,
    license_id: &str,
    regulatory_profile_id: &str,
    technical_profile_ids: &[String],
    requested_profile_ids: &[String],
) -> Result<EntitlementSet, PlatformApiError> {
    let rows = sqlx::query(
        "SELECT entitlement.profile_id, entitlement.profile_kind,
                versioned.profile_entitlement_version_id, versioned.version,
                versioned.expires_at
         FROM ai_profile_entitlements entitlement
         JOIN ai_profile_entitlement_versions versioned
           ON versioned.profile_entitlement_version_id = entitlement.current_version_id
         WHERE entitlement.license_id = $1
           AND entitlement.profile_id = ANY($2)
           AND entitlement.status = 'active'
           AND entitlement.effective_at <= NOW() AND entitlement.expires_at > NOW()
           AND versioned.status = 'active'
           AND versioned.effective_at <= NOW() AND versioned.expires_at > NOW()
         ORDER BY entitlement.profile_id",
    )
    .bind(license_id)
    .bind(requested_profile_ids)
    .fetch_all(&mut *connection)
    .await
    .map_err(internal_database_error)?;
    if rows.len() != requested_profile_ids.len() {
        return Err(reason_error(REASON_PROFILE_NOT_ENTITLED));
    }
    for row in &rows {
        let profile_id: String = row.get("profile_id");
        let profile_kind: String = row.get("profile_kind");
        let kind_matches = (profile_id == regulatory_profile_id && profile_kind == "regulatory")
            || (technical_profile_ids
                .iter()
                .any(|value| value == &profile_id)
                && profile_kind == "technical");
        if !kind_matches {
            return Err(reason_error(REASON_PROFILE_NOT_ENTITLED));
        }
    }
    let canonical = rows
        .iter()
        .map(|row| {
            format!(
                "{}:{}:{}:{}",
                row.get::<String, _>("profile_id"),
                row.get::<String, _>("profile_kind"),
                row.get::<String, _>("profile_entitlement_version_id"),
                row.get::<i32, _>("version")
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let version_id = format!("entset_{}", sha256_hex(canonical.as_bytes()));
    let expires_at = rows
        .iter()
        .map(|row| row.get::<DateTime<Utc>, _>("expires_at"))
        .min()
        .ok_or_else(|| reason_error(REASON_PROFILE_NOT_ENTITLED))?;
    Ok(EntitlementSet {
        version_id,
        digest: sha256_hex(canonical.as_bytes()),
        expires_at,
    })
}

async fn load_existing_session(
    connection: &mut sqlx::PgConnection,
    license_id: &str,
    idempotency_key: &str,
    admission_id: &str,
) -> Result<Option<SessionResponse>, PlatformApiError> {
    let row = sqlx::query(
        "SELECT session.marking_session_id, session.status, session.expires_at,
                platform.admission_id, platform.watermark_uid, platform.entitlement_digest
         FROM ai_marking_sessions session
         JOIN ai_platform_marking_sessions platform
           ON platform.marking_session_id = session.marking_session_id
         WHERE session.license_id = $1 AND session.idempotency_key = $2",
    )
    .bind(license_id)
    .bind(idempotency_key)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<String, _>("admission_id") != admission_id
        || row.get::<String, _>("status") != "ready_to_upload"
    {
        return Err(PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "idempotency key is already bound",
        ));
    }
    Ok(Some(SessionResponse {
        marking_session_id: row.get("marking_session_id"),
        admission_id: admission_id.to_string(),
        license_id: license_id.to_string(),
        entitlement_digest: row.get("entitlement_digest"),
        status: "ready_to_upload",
        watermark_uid: row.get("watermark_uid"),
        content_type: "image/png",
        expires_at: row.get("expires_at"),
    }))
}

async fn validate_presented_credential_id(
    connection: &mut sqlx::PgConnection,
    config: &CredentialCustodyConfig,
    cleartext_api_key: &str,
    expected_credential_id: &str,
) -> Result<(), PlatformApiError> {
    let key_prefix = credential_prefix(cleartext_api_key)
        .map_err(|_| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    let credential = sqlx::query(
        "SELECT credential_id, key_hash, hash_secret_version, environment, scopes_json,
                status, expires_at, custody_key_id
         FROM ai_sdk_credential_bindings WHERE key_prefix = $1",
    )
    .bind(key_prefix)
    .fetch_optional(&mut *connection)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    if credential.get::<String, _>("credential_id") != expected_credential_id {
        return Err(reason_error(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    verify_credential_row(config, cleartext_api_key, &credential, Utc::now())
}

async fn validate_presented_credential_id_tx(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    config: &CredentialCustodyConfig,
    cleartext_api_key: &str,
    expected_credential_id: &str,
) -> Result<(), PlatformApiError> {
    let key_prefix = credential_prefix(cleartext_api_key)
        .map_err(|_| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    let credential = sqlx::query(
        "SELECT credential_id, key_hash, hash_secret_version, environment, scopes_json,
                status, expires_at, custody_key_id
         FROM ai_sdk_credential_bindings WHERE key_prefix = $1",
    )
    .bind(key_prefix)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(internal_database_error)?
    .ok_or_else(|| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    if credential.get::<String, _>("credential_id") != expected_credential_id {
        return Err(reason_error(REASON_CREDENTIAL_UNAUTHORIZED));
    }
    verify_credential_row(config, cleartext_api_key, &credential, Utc::now())
}

async fn insert_api_audit(
    connection: &mut sqlx::PgConnection,
    operation: &str,
    outcome: &str,
    admission_id: Option<&str>,
    marking_session_id: Option<&str>,
    license_id: Option<&str>,
    reason_code: Option<&str>,
    details: Value,
) -> Result<(), PlatformApiError> {
    sqlx::query(
        "INSERT INTO ai_platform_api_audit_events (
            audit_event_id, operation, outcome, admission_id, marking_session_id,
            license_id, reason_code, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(format!("paudit_{}", Uuid::new_v4().simple()))
    .bind(operation)
    .bind(outcome)
    .bind(admission_id)
    .bind(marking_session_id)
    .bind(license_id)
    .bind(reason_code)
    .bind(details)
    .bind(Utc::now())
    .execute(&mut *connection)
    .await
    .map_err(internal_database_error)?;
    Ok(())
}

async fn insert_api_audit_tx(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: &str,
    outcome: &str,
    admission_id: Option<&str>,
    marking_session_id: Option<&str>,
    license_id: Option<&str>,
    reason_code: Option<&str>,
    details: Value,
) -> Result<(), PlatformApiError> {
    sqlx::query(
        "INSERT INTO ai_platform_api_audit_events (
            audit_event_id, operation, outcome, admission_id, marking_session_id,
            license_id, reason_code, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(format!("paudit_{}", Uuid::new_v4().simple()))
    .bind(operation)
    .bind(outcome)
    .bind(admission_id)
    .bind(marking_session_id)
    .bind(license_id)
    .bind(reason_code)
    .bind(details)
    .bind(Utc::now())
    .execute(&mut **transaction)
    .await
    .map_err(internal_database_error)?;
    Ok(())
}

fn bearer_credential(headers: &HeaderMap) -> Result<String, PlatformApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| reason_error(REASON_CREDENTIAL_UNAUTHORIZED))?;
    Ok(value.to_string())
}

fn map_issuer_mode(mode: &str) -> Result<&'static str, PlatformApiError> {
    match mode {
        "hiddenshield_managed" => Ok("hiddenshield_managed"),
        "customer_managed" => Ok("customer_byok"),
        "platform_signed" => Ok("platform_managed"),
        _ => Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "admission_invalid",
            "issuerMode is invalid",
        )),
    }
}

fn require_non_empty(value: &str, code: &'static str) -> Result<(), PlatformApiError> {
    if value.trim().is_empty() {
        return Err(PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            code,
            "required field is empty",
        ));
    }
    Ok(())
}

fn custody_error(error: CredentialCustodyError) -> PlatformApiError {
    match error {
        CredentialCustodyError::Authorization(reason) => reason_error(&reason),
        CredentialCustodyError::ProviderUnavailable(_) => {
            PlatformApiError::unavailable("credential custody provider is unavailable")
        }
        CredentialCustodyError::InvalidConfiguration => {
            PlatformApiError::unavailable("credential custody is not configured")
        }
        CredentialCustodyError::Postgres(_) => {
            PlatformApiError::unavailable("PostgreSQL is unavailable")
        }
    }
}

fn reason_error(reason: &str) -> PlatformApiError {
    match reason {
        REASON_CREDENTIAL_UNAUTHORIZED => PlatformApiError::new(
            StatusCode::UNAUTHORIZED,
            "credential_invalid",
            "credential is invalid",
        ),
        REASON_CREDENTIAL_INACTIVE => PlatformApiError::new(
            StatusCode::UNAUTHORIZED,
            "credential_inactive",
            "credential is inactive",
        ),
        REASON_CREDENTIAL_EXPIRED => PlatformApiError::new(
            StatusCode::UNAUTHORIZED,
            "credential_expired",
            "credential has expired",
        ),
        REASON_CREDENTIAL_SCOPE_DENIED => PlatformApiError::new(
            StatusCode::FORBIDDEN,
            "credential_scope_denied",
            "credential scope is denied",
        ),
        REASON_LICENSE_INACTIVE => PlatformApiError::new(
            StatusCode::FORBIDDEN,
            "license_inactive",
            "production license is inactive",
        ),
        REASON_LICENSE_EXPIRED => PlatformApiError::new(
            StatusCode::FORBIDDEN,
            "license_expired",
            "production license has expired",
        ),
        REASON_PROFILE_NOT_ENTITLED | REASON_CREDENTIAL_ISSUER_MODE_DENIED => {
            PlatformApiError::new(
                StatusCode::FORBIDDEN,
                "profile_not_entitled",
                "requested Profile is not entitled",
            )
        }
        REASON_ENVIRONMENT_MISMATCH => PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "admission_invalid",
            "production admission scope mismatch",
        ),
        "ai_idempotency_conflict" => PlatformApiError::new(
            StatusCode::CONFLICT,
            "session_conflict",
            "idempotency key conflict",
        ),
        "ai_session_state_invalid" => PlatformApiError::new(
            StatusCode::CONFLICT,
            "confirm_conflict",
            "session is not ready to confirm",
        ),
        "ai_subject_digest_invalid" => PlatformApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "marked_image_digest_mismatch",
            "marked image digest mismatch",
        ),
        _ => PlatformApiError::new(
            StatusCode::CONFLICT,
            "confirm_conflict",
            "operation failed closed",
        ),
    }
}

fn internal_database_error(_error: sqlx::Error) -> PlatformApiError {
    PlatformApiError::unavailable("PostgreSQL operation failed")
}

fn generate_watermark_uid() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    if bytes == [0_u8; 16] {
        bytes[15] = 1;
    }
    format!(
        "HS-{}-{}-{}-{}",
        hex_upper(&bytes[0..4]),
        hex_upper(&bytes[4..8]),
        hex_upper(&bytes[8..12]),
        hex_upper(&bytes[12..16])
    )
}

fn generate_confirmation_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!(
        "hsai_confirm_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn token_hash(secret: &str, token: &str) -> Result<String, PlatformApiError> {
    if secret.len() < 32 {
        return Err(PlatformApiError::unavailable(
            "confirmation token secret is not configured",
        ));
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| {
        PlatformApiError::unavailable("confirmation token secret is not configured")
    })?;
    mac.update(token.as_bytes());
    Ok(hex_lower(&mac.finalize().into_bytes()))
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex_lower(&Sha256::digest(bytes.as_ref()))
}

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
