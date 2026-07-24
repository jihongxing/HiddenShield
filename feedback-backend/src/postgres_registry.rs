use std::future::Future;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::repository::WatermarkRegistryRepository;
use crate::schema::{
    WatermarkIdConfirmRequest, WatermarkIdReconcileRequest, WatermarkIdRegistryResponse,
    WatermarkIdReissueRequest, WatermarkIdReissueResponse, WatermarkIdReserveRequest,
};
use crate::storage::StorageError;

#[derive(Clone)]
pub struct PostgresWatermarkRegistryRepository {
    pool: PgPool,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PostgresWatermarkRegistryRepository {
    pub fn new(pool: PgPool) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("postgres registry repository runtime must be available"),
        );
        Self { pool, runtime }
    }

    pub fn connect(database_url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| {
                    StorageError::BadRequest(format!("postgres_runtime_unavailable:{error}"))
                })?,
        );
        let pool = runtime.block_on(async move {
            PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(database_url)
                .await
        })?;
        Ok(Self { pool, runtime })
    }

    fn run<T, F>(&self, future: F) -> Result<T, StorageError>
    where
        F: Future<Output = Result<T, StorageError>>,
    {
        self.runtime.block_on(future)
    }
}

impl WatermarkRegistryRepository for PostgresWatermarkRegistryRepository {
    fn reserve_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReserveRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        self.run(reserve_watermark_id_pg(
            self.pool.clone(),
            access_token.to_string(),
            request.clone(),
        ))
    }

    fn confirm_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdConfirmRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        self.run(confirm_watermark_id_pg(
            self.pool.clone(),
            access_token.to_string(),
            request.clone(),
        ))
    }

    fn reconcile_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReconcileRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        self.run(reconcile_watermark_id_pg(
            self.pool.clone(),
            access_token.to_string(),
            request.clone(),
        ))
    }

    fn reissue_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReissueRequest,
    ) -> Result<WatermarkIdReissueResponse, StorageError> {
        self.run(reissue_watermark_id_pg(
            self.pool.clone(),
            access_token.to_string(),
            request.clone(),
        ))
    }
}

async fn reserve_watermark_id_pg(
    pool: PgPool,
    access_token: String,
    request: WatermarkIdReserveRequest,
) -> Result<WatermarkIdRegistryResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?.to_string();
    let creator_profile_id =
        require_non_empty(&request.creator_profile_id, "creatorProfileId")?.to_string();
    let request_id = require_non_empty(&request.request_id, "requestId")?.to_string();
    validate_payload_protocol(
        request.payload_protocol_version,
        request.payload_bytes_length,
    )?;
    validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
    let media_type = normalize_media_type(&request.media_type)?;
    let parent_watermark_uid = normalize_optional_string(request.parent_watermark_uid.as_deref());
    let original_hash = normalize_optional_string(request.original_hash.as_deref());

    if !session_workspace_matches_pg(&pool, &session.account_id, &workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    if !creator_profile_matches_pg(&pool, &session.account_id, &creator_profile_id).await? {
        return Err(StorageError::BadRequest(
            "creator_profile_required".to_string(),
        ));
    }
    if let Some(existing) =
        load_watermark_registry_by_request_pg(&pool, &session.account_id, &request_id).await?
    {
        return registry_response_from_row(existing);
    }

    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let watermark_uid = generate_watermark_uid();
    let registry_id = format!(
        "wmreg_{}",
        short_id(&format!("{request_id}{watermark_uid}"))
    );
    let registry_receipt = build_registry_receipt(&registry_id, &watermark_uid, "reserved");
    let registry_proof_hash = registry_proof_hash(&registry_receipt);
    sqlx::query(
        "INSERT INTO watermark_id_registry (
            registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
            watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
            registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
            parent_watermark_uid, revision, original_hash, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'server_reserved', 'reserved', $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17)",
    )
    .bind(&registry_id)
    .bind(&request_id)
    .bind(&session.account_id)
    .bind(&workspace_id)
    .bind(&creator_profile_id)
    .bind(&session.device_id)
    .bind(&watermark_uid)
    .bind(&registry_receipt)
    .bind(&registry_proof_hash)
    .bind(&media_type)
    .bind(request.payload_protocol_version as i32)
    .bind(request.payload_bytes_length as i32)
    .bind(parent_watermark_uid)
    .bind(request.revision as i32)
    .bind(original_hash)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let row = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
        .await?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    tx.commit().await?;
    registry_response_from_row(row)
}

async fn confirm_watermark_id_pg(
    pool: PgPool,
    access_token: String,
    request: WatermarkIdConfirmRequest,
) -> Result<WatermarkIdRegistryResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?.to_string();
    let creator_profile_id =
        require_non_empty(&request.creator_profile_id, "creatorProfileId")?.to_string();
    let watermark_uid = normalize_watermark_uid(&request.watermark_uid)?;
    validate_payload_protocol(
        request.payload_protocol_version,
        request.payload_bytes_length,
    )?;
    let write_status = require_non_empty(
        &request.write_verification_status,
        "writeVerificationStatus",
    )?
    .to_string();
    let original_hash = normalize_optional_string(request.original_hash.as_deref());
    let protected_copy_hash = normalize_optional_string(request.protected_copy_hash.as_deref());

    if !session_workspace_matches_pg(&pool, &session.account_id, &workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    if !creator_profile_matches_pg(&pool, &session.account_id, &creator_profile_id).await? {
        return Err(StorageError::BadRequest(
            "creator_profile_required".to_string(),
        ));
    }
    let mut tx = pool.begin().await?;
    let existing = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
        .await?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    if existing.registry_status == "conflict" || existing.registry_status == "reissue_required" {
        return Err(StorageError::BadRequest(
            "watermark_registry_conflict".to_string(),
        ));
    }
    let now = Utc::now();
    let registry_receipt =
        build_registry_receipt(&existing.registry_id, &watermark_uid, "server_confirmed");
    let registry_proof_hash = registry_proof_hash(&registry_receipt);
    sqlx::query(
        "UPDATE watermark_id_registry
         SET registry_status = 'server_confirmed',
             watermark_id_issue_mode = 'server_confirmed',
             registry_receipt = $2,
             registry_proof_hash = $3,
             payload_protocol_version = $4,
             payload_bytes_length = $5,
             original_hash = COALESCE($6, original_hash),
             protected_copy_hash = COALESCE($7, protected_copy_hash),
             write_verification_status = $8,
             confirmed_at = $9,
             updated_at = $9
         WHERE watermark_uid = $1 AND account_id = $10 AND workspace_id = $11",
    )
    .bind(&watermark_uid)
    .bind(&registry_receipt)
    .bind(&registry_proof_hash)
    .bind(request.payload_protocol_version as i32)
    .bind(request.payload_bytes_length as i32)
    .bind(original_hash)
    .bind(protected_copy_hash)
    .bind(write_status)
    .bind(now)
    .bind(&session.account_id)
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await?;
    let row = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
        .await?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    tx.commit().await?;
    registry_response_from_row(row)
}

async fn reconcile_watermark_id_pg(
    pool: PgPool,
    access_token: String,
    request: WatermarkIdReconcileRequest,
) -> Result<WatermarkIdRegistryResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?.to_string();
    let creator_profile_id =
        require_non_empty(&request.creator_profile_id, "creatorProfileId")?.to_string();
    let watermark_uid = normalize_watermark_uid(&request.watermark_uid)?;
    validate_payload_protocol(
        request.payload_protocol_version,
        request.payload_bytes_length,
    )?;
    validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
    let media_type = normalize_media_type(&request.media_type)?;
    let parent_watermark_uid = normalize_optional_string(request.parent_watermark_uid.as_deref());
    let original_hash = normalize_optional_string(request.original_hash.as_deref());
    let protected_copy_hash = normalize_optional_string(request.protected_copy_hash.as_deref());
    let write_status = normalize_optional_string(request.write_verification_status.as_deref());

    if !session_workspace_matches_pg(&pool, &session.account_id, &workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    if !creator_profile_matches_pg(&pool, &session.account_id, &creator_profile_id).await? {
        return Err(StorageError::BadRequest(
            "creator_profile_required".to_string(),
        ));
    }
    let mut tx = pool.begin().await?;
    if let Some(existing) = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid).await? {
        let existing_original_hash =
            load_watermark_original_hash_tx_pg(&mut tx, &watermark_uid).await?;
        let same_owner = watermark_registry_owner_matches_tx_pg(
            &mut tx,
            &watermark_uid,
            &session.account_id,
            &workspace_id,
        )
        .await?;
        let same_original_hash = original_hash.is_none()
            || existing_original_hash.is_none()
            || existing_original_hash == original_hash;
        if same_owner && same_original_hash {
            let now = Utc::now();
            let status = if existing.registry_status == "server_confirmed" {
                "server_confirmed"
            } else {
                "offline_confirmed"
            };
            let issue_mode = if existing.watermark_id_issue_mode == "server_confirmed" {
                "server_confirmed"
            } else {
                "offline_generated"
            };
            let registry_receipt =
                build_registry_receipt(&existing.registry_id, &watermark_uid, status);
            let registry_proof_hash = registry_proof_hash(&registry_receipt);
            sqlx::query(
                "UPDATE watermark_id_registry
                 SET registry_status = $2,
                     watermark_id_issue_mode = $3,
                     registry_receipt = $4,
                     registry_proof_hash = $5,
                     original_hash = COALESCE($6, original_hash),
                     protected_copy_hash = COALESCE($7, protected_copy_hash),
                     write_verification_status = COALESCE($8, write_verification_status),
                     confirmed_at = COALESCE(confirmed_at, $9),
                     updated_at = $9
                 WHERE watermark_uid = $1",
            )
            .bind(&watermark_uid)
            .bind(status)
            .bind(issue_mode)
            .bind(registry_receipt)
            .bind(registry_proof_hash)
            .bind(original_hash)
            .bind(protected_copy_hash)
            .bind(write_status)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        } else {
            let now = Utc::now();
            let registry_receipt =
                build_registry_receipt(&existing.registry_id, &watermark_uid, "conflict");
            let registry_proof_hash = registry_proof_hash(&registry_receipt);
            sqlx::query(
                "UPDATE watermark_id_registry
                 SET registry_status = 'conflict',
                     registry_receipt = $2,
                     registry_proof_hash = $3,
                     updated_at = $4
                 WHERE watermark_uid = $1",
            )
            .bind(&watermark_uid)
            .bind(registry_receipt)
            .bind(registry_proof_hash)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        let row = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
            .await?
            .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
        tx.commit().await?;
        return registry_response_from_row(row);
    }

    let now = Utc::now();
    let registry_id = format!(
        "wmreg_{}",
        short_id(&format!("{}{}{}", session.account_id, watermark_uid, now))
    );
    let registry_receipt =
        build_registry_receipt(&registry_id, &watermark_uid, "offline_confirmed");
    let registry_proof_hash = registry_proof_hash(&registry_receipt);
    sqlx::query(
        "INSERT INTO watermark_id_registry (
            registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
            watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
            registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
            parent_watermark_uid, revision, original_hash, protected_copy_hash,
            write_verification_status, confirmed_at, created_at, updated_at
        ) VALUES ($1, NULL, $2, $3, $4, $5, $6, 'offline_generated', 'offline_confirmed',
            $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17, $17)",
    )
    .bind(&registry_id)
    .bind(&session.account_id)
    .bind(&workspace_id)
    .bind(&creator_profile_id)
    .bind(&session.device_id)
    .bind(&watermark_uid)
    .bind(&registry_receipt)
    .bind(&registry_proof_hash)
    .bind(&media_type)
    .bind(request.payload_protocol_version as i32)
    .bind(request.payload_bytes_length as i32)
    .bind(parent_watermark_uid)
    .bind(request.revision as i32)
    .bind(original_hash)
    .bind(protected_copy_hash)
    .bind(write_status)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let row = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
        .await?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    tx.commit().await?;
    registry_response_from_row(row)
}

async fn reissue_watermark_id_pg(
    pool: PgPool,
    access_token: String,
    request: WatermarkIdReissueRequest,
) -> Result<WatermarkIdReissueResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?.to_string();
    let creator_profile_id =
        require_non_empty(&request.creator_profile_id, "creatorProfileId")?.to_string();
    let previous_watermark_uid = normalize_watermark_uid(&request.previous_watermark_uid)?;
    validate_payload_protocol(
        request.payload_protocol_version,
        request.payload_bytes_length,
    )?;
    validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
    let media_type = normalize_media_type(&request.media_type)?;
    let reason = require_non_empty(&request.reason, "reason")?.to_string();
    let parent_watermark_uid = normalize_optional_string(request.parent_watermark_uid.as_deref());
    let original_hash = normalize_optional_string(request.original_hash.as_deref());

    if !session_workspace_matches_pg(&pool, &session.account_id, &workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    if !creator_profile_matches_pg(&pool, &session.account_id, &creator_profile_id).await? {
        return Err(StorageError::BadRequest(
            "creator_profile_required".to_string(),
        ));
    }
    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let watermark_uid = generate_watermark_uid();
    let registry_id = format!(
        "wmreg_{}",
        short_id(&format!("{}{}{}", session.account_id, watermark_uid, now))
    );
    let registry_receipt = build_registry_receipt(&registry_id, &watermark_uid, "server_reissued");
    let registry_proof_hash = registry_proof_hash(&registry_receipt);
    sqlx::query(
        "INSERT INTO watermark_id_registry (
            registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
            watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
            registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
            parent_watermark_uid, revision, original_hash, created_at, updated_at
        ) VALUES ($1, NULL, $2, $3, $4, $5, $6, 'server_reissued', 'reserved', $7, $8, $9,
            $10, $11, $12, $13, $14, $15, $15)",
    )
    .bind(&registry_id)
    .bind(&session.account_id)
    .bind(&workspace_id)
    .bind(&creator_profile_id)
    .bind(&session.device_id)
    .bind(&watermark_uid)
    .bind(&registry_receipt)
    .bind(&registry_proof_hash)
    .bind(&media_type)
    .bind(request.payload_protocol_version as i32)
    .bind(request.payload_bytes_length as i32)
    .bind(parent_watermark_uid)
    .bind(request.revision as i32)
    .bind(original_hash)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let job_id = format!(
        "wmreissue_{}",
        short_id(&format!(
            "{}{}{}",
            previous_watermark_uid, watermark_uid, now
        ))
    );
    sqlx::query(
        "INSERT INTO watermark_id_reissue_jobs (
            job_id, account_id, workspace_id, creator_profile_id, previous_watermark_uid,
            replacement_watermark_uid, reason, status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'created', $8, $8)",
    )
    .bind(&job_id)
    .bind(&session.account_id)
    .bind(&workspace_id)
    .bind(&creator_profile_id)
    .bind(&previous_watermark_uid)
    .bind(&watermark_uid)
    .bind(reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let row = load_watermark_registry_by_uid_tx_pg(&mut tx, &watermark_uid)
        .await?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    tx.commit().await?;
    Ok(WatermarkIdReissueResponse {
        job_id,
        previous_watermark_uid,
        replacement: registry_response_from_row(row)?,
    })
}

async fn authenticate_pg(pool: &PgPool, access_token: &str) -> Result<SessionRecord, StorageError> {
    let access_token = access_token.trim();
    if access_token.is_empty() {
        return Err(StorageError::Unauthorized);
    }
    let row = sqlx::query(
        "SELECT account_id, device_id, revoked_at
         FROM cloud_sessions
         WHERE access_token = $1",
    )
    .bind(access_token)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    let session = SessionRecord {
        account_id: row.try_get("account_id")?,
        device_id: row.try_get("device_id")?,
        revoked_at: row.try_get("revoked_at")?,
    };
    if session.revoked_at.is_some() {
        return Err(StorageError::Unauthorized);
    }
    Ok(session)
}

async fn session_workspace_matches_pg(
    pool: &PgPool,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    let stored_workspace_id: Option<String> =
        sqlx::query_scalar("SELECT workspace_id FROM cloud_accounts WHERE id = $1")
            .bind(account_id.trim())
            .fetch_optional(pool)
            .await?;
    Ok(stored_workspace_id.as_deref() == Some(workspace_id))
}

async fn creator_profile_matches_pg(
    pool: &PgPool,
    account_id: &str,
    creator_profile_id: &str,
) -> Result<bool, StorageError> {
    if creator_profile_id.trim().is_empty() {
        return Ok(false);
    }
    let stored_creator_profile_id: Option<String> =
        sqlx::query_scalar("SELECT creator_profile_id FROM cloud_accounts WHERE id = $1")
            .bind(account_id.trim())
            .fetch_optional(pool)
            .await?;
    Ok(stored_creator_profile_id.as_deref() == Some(creator_profile_id))
}

async fn load_watermark_registry_by_request_pg(
    pool: &PgPool,
    account_id: &str,
    request_id: &str,
) -> Result<Option<WatermarkIdRegistryRow>, StorageError> {
    let row = sqlx::query(
        "SELECT registry_id, watermark_uid, watermark_id_issue_mode, registry_status,
                registry_receipt, registry_proof_hash, payload_protocol_version,
                payload_bytes_length, parent_watermark_uid, revision, created_at, updated_at
         FROM watermark_id_registry
         WHERE account_id = $1 AND request_id = $2",
    )
    .bind(account_id)
    .bind(request_id)
    .fetch_optional(pool)
    .await?;
    row.map(watermark_registry_row_from_sql).transpose()
}

async fn load_watermark_registry_by_uid_tx_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    watermark_uid: &str,
) -> Result<Option<WatermarkIdRegistryRow>, StorageError> {
    let row = sqlx::query(
        "SELECT registry_id, watermark_uid, watermark_id_issue_mode, registry_status,
                registry_receipt, registry_proof_hash, payload_protocol_version,
                payload_bytes_length, parent_watermark_uid, revision, created_at, updated_at
         FROM watermark_id_registry
         WHERE watermark_uid = $1",
    )
    .bind(watermark_uid)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(watermark_registry_row_from_sql).transpose()
}

async fn load_watermark_original_hash_tx_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    watermark_uid: &str,
) -> Result<Option<String>, StorageError> {
    sqlx::query_scalar("SELECT original_hash FROM watermark_id_registry WHERE watermark_uid = $1")
        .bind(watermark_uid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(StorageError::from)
}

async fn watermark_registry_owner_matches_tx_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    watermark_uid: &str,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    let owner = sqlx::query(
        "SELECT account_id, workspace_id FROM watermark_id_registry WHERE watermark_uid = $1",
    )
    .bind(watermark_uid)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(owner
        .map(|row| {
            let stored_account: Result<String, sqlx::Error> = row.try_get("account_id");
            let stored_workspace: Result<String, sqlx::Error> = row.try_get("workspace_id");
            stored_account.ok().as_deref() == Some(account_id)
                && stored_workspace.ok().as_deref() == Some(workspace_id)
        })
        .unwrap_or(false))
}

fn watermark_registry_row_from_sql(
    row: sqlx::postgres::PgRow,
) -> Result<WatermarkIdRegistryRow, StorageError> {
    Ok(WatermarkIdRegistryRow {
        registry_id: row.try_get("registry_id")?,
        watermark_uid: row.try_get("watermark_uid")?,
        watermark_id_issue_mode: row.try_get("watermark_id_issue_mode")?,
        registry_status: row.try_get("registry_status")?,
        registry_receipt: row.try_get("registry_receipt")?,
        registry_proof_hash: row.try_get("registry_proof_hash")?,
        payload_protocol_version: row.try_get::<i32, _>("payload_protocol_version")? as i64,
        payload_bytes_length: row.try_get::<i32, _>("payload_bytes_length")? as i64,
        parent_watermark_uid: row.try_get("parent_watermark_uid")?,
        revision: row.try_get::<i32, _>("revision")? as i64,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn registry_response_from_row(
    row: WatermarkIdRegistryRow,
) -> Result<WatermarkIdRegistryResponse, StorageError> {
    Ok(WatermarkIdRegistryResponse {
        registry_id: row.registry_id,
        watermark_uid: row.watermark_uid,
        watermark_id_issue_mode: row.watermark_id_issue_mode,
        registry_status: row.registry_status,
        registry_receipt: row.registry_receipt,
        registry_proof_hash: row.registry_proof_hash,
        payload_protocol_version: row.payload_protocol_version as u32,
        payload_bytes_length: row.payload_bytes_length as u32,
        parent_watermark_uid: row.parent_watermark_uid,
        revision: row.revision as u32,
        issued_at: row.created_at,
        updated_at: row.updated_at,
    })
}

#[derive(Debug, Clone)]
struct WatermarkIdRegistryRow {
    registry_id: String,
    watermark_uid: String,
    watermark_id_issue_mode: String,
    registry_status: String,
    registry_receipt: String,
    registry_proof_hash: String,
    payload_protocol_version: i64,
    payload_bytes_length: i64,
    parent_watermark_uid: Option<String>,
    revision: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    account_id: String,
    device_id: String,
    revoked_at: Option<DateTime<Utc>>,
}

fn require_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StorageError::BadRequest(format!("{field} is required")));
    }
    Ok(value)
}

fn validate_payload_protocol(version: u32, bytes: u32) -> Result<(), StorageError> {
    if version == 2 && bytes == 119 {
        return Ok(());
    }
    if version == 3 && (33..=64).contains(&bytes) {
        return Ok(());
    }
    if version == 3 {
        return Err(StorageError::BadRequest(
            "payload_protocol_v3_anchor_size_invalid".to_string(),
        ));
    }
    Err(StorageError::BadRequest(
        "payload_protocol_v2_or_v3_required".to_string(),
    ))
}

fn validate_revision(
    parent_watermark_uid: Option<&str>,
    revision: u32,
) -> Result<(), StorageError> {
    if revision == 0 {
        return Err(StorageError::BadRequest("revision_invalid".to_string()));
    }
    if normalize_optional_string(parent_watermark_uid).is_some() && revision == 1 {
        return Err(StorageError::BadRequest(
            "parent_watermark_requires_revision_gt_1".to_string(),
        ));
    }
    Ok(())
}

fn normalize_media_type(value: &str) -> Result<String, StorageError> {
    let value = value.trim().to_lowercase();
    match value.as_str() {
        "image" | "audio" | "video_audio_track" | "video_visual" => Ok(value),
        _ => Err(StorageError::BadRequest("media_type_invalid".to_string())),
    }
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_watermark_uid(value: &str) -> Result<String, StorageError> {
    let value = value.trim().to_uppercase();
    let parts = value.split('-').collect::<Vec<_>>();
    let valid = parts.len() == 5
        && parts[0] == "HS"
        && parts[1..]
            .iter()
            .all(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !valid {
        return Err(StorageError::BadRequest(
            "watermark_uid_invalid".to_string(),
        ));
    }
    Ok(value)
}

fn generate_watermark_uid() -> String {
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
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

fn build_registry_receipt(registry_id: &str, watermark_uid: &str, status: &str) -> String {
    let now = Utc::now().to_rfc3339();
    let proof = registry_proof_hash(&format!("{registry_id}:{watermark_uid}:{status}:{now}"));
    format!("hsreg:v1:{registry_id}:{watermark_uid}:{status}:{proof}")
}

fn registry_proof_hash(value: &str) -> String {
    let secret = std::env::var("HIDDENSHIELD_REGISTRY_PROOF_SECRET")
        .unwrap_or_else(|_| "hidden-shield-registry-proof-dev-secret".to_string());
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key");
    mac.update(value.as_bytes());
    hex_string(&mac.finalize().into_bytes())
}

fn short_id(input: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in input.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
