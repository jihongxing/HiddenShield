use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CloudCopyrightRepository {
    pool: PgPool,
}

impl CloudCopyrightRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn execute_change(
        &self,
        actor: &CloudCopyrightActor,
        command: &CloudCopyrightChangeCommand,
    ) -> Result<CloudCopyrightDisposition, CloudCopyrightError> {
        validate_change(command)?;
        let mut transaction = self.pool.begin().await?;
        let membership =
            load_membership_for_update(&mut transaction, actor, &command.workspace_id).await?;
        require_record_write(&membership)?;
        require_active_device(&mut transaction, actor).await?;

        let record = sqlx::query(
            "SELECT record_version, deleted_at
             FROM cloud_copyright_records
             WHERE workspace_id = $1 AND record_id = $2
             FOR UPDATE",
        )
        .bind(&command.workspace_id)
        .bind(&command.record_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(CloudCopyrightError::RecordNotFound)?;
        let inserted_change = sqlx::query(
            "INSERT INTO cloud_copyright_changes (
                change_id, workspace_id, device_id, record_id, idempotency_key,
                request_digest, operation, base_record_version, status,
                record_version, created_at, updated_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'accepted',NULL,NOW(),NOW())
             ON CONFLICT(workspace_id, device_id, idempotency_key) DO NOTHING
             RETURNING change_id",
        )
        .bind(&command.change_id)
        .bind(&command.workspace_id)
        .bind(&actor.device_id)
        .bind(&command.record_id)
        .bind(&command.idempotency_key)
        .bind(&command.request_digest)
        .bind(command.operation.as_str())
        .bind(command.base_record_version)
        .fetch_optional(&mut *transaction)
        .await?;

        if inserted_change.is_none() {
            let existing = sqlx::query(
                "SELECT request_digest, record_version
                 FROM cloud_copyright_changes
                 WHERE workspace_id = $1 AND device_id = $2 AND idempotency_key = $3",
            )
            .bind(&command.workspace_id)
            .bind(&actor.device_id)
            .bind(&command.idempotency_key)
            .fetch_one(&mut *transaction)
            .await?;
            let existing_digest: String = existing.try_get("request_digest")?;
            if existing_digest != command.request_digest {
                return Err(CloudCopyrightError::ConflictPayloadChanged);
            }
            let record_version: Option<i64> = existing.try_get("record_version")?;
            transaction.commit().await?;
            return Ok(CloudCopyrightDisposition::Duplicate {
                record_version: record_version.unwrap_or(command.base_record_version),
            });
        }
        let current_version: i64 = record.try_get("record_version")?;
        let deleted_at: Option<chrono::DateTime<Utc>> = record.try_get("deleted_at")?;
        if current_version != command.base_record_version || deleted_at.is_some() {
            return Err(CloudCopyrightError::ConflictVersionChanged);
        }

        let next_version = current_version + 1;
        let next_etag = record_etag(&command.record_id, next_version, &command.request_digest);
        match command.operation {
            CloudCopyrightOperation::UpsertRecord => {
                sqlx::query(
                    "UPDATE cloud_copyright_records
                     SET rights_declaration_json = $3,
                         record_version = $4,
                         etag = $5,
                         updated_at = NOW()
                     WHERE workspace_id = $1 AND record_id = $2",
                )
                .bind(&command.workspace_id)
                .bind(&command.record_id)
                .bind(&command.rights_declaration)
                .bind(next_version)
                .bind(&next_etag)
                .execute(&mut *transaction)
                .await?;
            }
            CloudCopyrightOperation::TombstoneRecord => {
                sqlx::query(
                    "UPDATE cloud_copyright_records
                     SET record_version = $3,
                         etag = $4,
                         deleted_at = NOW(),
                         updated_at = NOW()
                     WHERE workspace_id = $1 AND record_id = $2",
                )
                .bind(&command.workspace_id)
                .bind(&command.record_id)
                .bind(next_version)
                .bind(&next_etag)
                .execute(&mut *transaction)
                .await?;
            }
        }

        let event_id = format!("cce_{}", Uuid::new_v4().simple());
        let event_type = command.operation.event_type();
        let event_sequence: i64 = sqlx::query_scalar(
            "INSERT INTO cloud_copyright_events (
                event_id, workspace_id, record_id, change_id, event_type,
                record_version, payload_json, created_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,NOW())
             RETURNING sequence",
        )
        .bind(&event_id)
        .bind(&command.workspace_id)
        .bind(&command.record_id)
        .bind(&command.change_id)
        .bind(event_type)
        .bind(next_version)
        .bind(json!({
            "operation": command.operation.as_str(),
            "requestDigest": command.request_digest,
            "recordVersion": next_version
        }))
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO cloud_copyright_workspace_cursors (
                workspace_id, device_id, cursor_sequence, updated_at
             ) VALUES ($1,$2,$3,NOW())
             ON CONFLICT(workspace_id, device_id) DO UPDATE SET
                cursor_sequence = GREATEST(
                    cloud_copyright_workspace_cursors.cursor_sequence,
                    excluded.cursor_sequence
                ),
                updated_at = excluded.updated_at",
        )
        .bind(&command.workspace_id)
        .bind(&actor.device_id)
        .bind(event_sequence)
        .execute(&mut *transaction)
        .await?;

        append_audit(
            &mut transaction,
            actor,
            &membership,
            &command.workspace_id,
            event_type,
            "copyright_record",
            &command.record_id,
            &command.request_digest,
        )
        .await?;

        sqlx::query(
            "UPDATE cloud_copyright_changes
             SET record_version = $2, updated_at = NOW()
             WHERE change_id = $1",
        )
        .bind(&command.change_id)
        .bind(next_version)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(CloudCopyrightDisposition::Accepted {
            record_version: next_version,
            event_sequence,
        })
    }

    pub async fn revoke_membership(
        &self,
        actor: &CloudCopyrightActor,
        command: &RevokeWorkspaceMembershipCommand,
    ) -> Result<i64, CloudCopyrightError> {
        let mut transaction = self.pool.begin().await?;
        let actor_membership =
            load_membership_for_update(&mut transaction, actor, &command.workspace_id).await?;
        require_member_admin(&actor_membership)?;
        require_active_device(&mut transaction, actor).await?;

        let target = sqlx::query(
            "SELECT membership_id, status, membership_version
             FROM cloud_copyright_workspace_memberships
             WHERE workspace_id = $1 AND account_id = $2
             FOR UPDATE",
        )
        .bind(&command.workspace_id)
        .bind(&command.target_account_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(CloudCopyrightError::MembershipNotFound)?;
        let target_membership_id: String = target.try_get("membership_id")?;
        let target_status: String = target.try_get("status")?;
        let target_version: i64 = target.try_get("membership_version")?;
        if target_status == "removed" {
            transaction.commit().await?;
            return Ok(target_version);
        }

        let next_version = target_version + 1;
        sqlx::query(
            "UPDATE cloud_copyright_workspace_memberships
             SET status = 'removed',
                 membership_version = $3,
                 removed_at = NOW(),
                 updated_at = NOW()
             WHERE workspace_id = $1 AND account_id = $2",
        )
        .bind(&command.workspace_id)
        .bind(&command.target_account_id)
        .bind(next_version)
        .execute(&mut *transaction)
        .await?;

        append_audit(
            &mut transaction,
            actor,
            &actor_membership,
            &command.workspace_id,
            "membership_revoked",
            "workspace_membership",
            &target_membership_id,
            &command.request_digest,
        )
        .await?;
        transaction.commit().await?;
        Ok(next_version)
    }

    pub async fn get_record(
        &self,
        actor: &CloudCopyrightActor,
        workspace_id: &str,
        record_id: &str,
    ) -> Result<CloudCopyrightRecordProjection, CloudCopyrightError> {
        let mut transaction = self.pool.begin().await?;
        let membership = load_membership(&mut transaction, actor, workspace_id).await?;
        require_record_read(&membership)?;
        require_active_device(&mut transaction, actor).await?;
        let row = sqlx::query(
            "SELECT record_id, workspace_id, watermark_uid, watermark_revision,
                    record_version, etag, deleted_at
             FROM cloud_copyright_records
             WHERE workspace_id = $1 AND record_id = $2",
        )
        .bind(workspace_id)
        .bind(record_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(CloudCopyrightError::RecordNotFound)?;
        let projection = CloudCopyrightRecordProjection {
            record_id: row.try_get("record_id")?,
            workspace_id: row.try_get("workspace_id")?,
            watermark_uid: row.try_get("watermark_uid")?,
            watermark_revision: row.try_get("watermark_revision")?,
            record_version: row.try_get("record_version")?,
            etag: row.try_get("etag")?,
            deleted: row
                .try_get::<Option<chrono::DateTime<Utc>>, _>("deleted_at")?
                .is_some(),
        };
        transaction.commit().await?;
        Ok(projection)
    }
}

#[derive(Debug, Clone)]
pub struct CloudCopyrightActor {
    pub account_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone)]
pub struct CloudCopyrightChangeCommand {
    pub change_id: String,
    pub workspace_id: String,
    pub record_id: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub base_record_version: i64,
    pub operation: CloudCopyrightOperation,
    pub rights_declaration: Value,
}

#[derive(Debug, Clone, Copy)]
pub enum CloudCopyrightOperation {
    UpsertRecord,
    TombstoneRecord,
}

impl CloudCopyrightOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::UpsertRecord => "upsert_record",
            Self::TombstoneRecord => "tombstone_record",
        }
    }

    fn event_type(self) -> &'static str {
        match self {
            Self::UpsertRecord => "record_upserted",
            Self::TombstoneRecord => "record_tombstoned",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RevokeWorkspaceMembershipCommand {
    pub workspace_id: String,
    pub target_account_id: String,
    pub request_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudCopyrightDisposition {
    Accepted {
        record_version: i64,
        event_sequence: i64,
    },
    Duplicate {
        record_version: i64,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCopyrightRecordProjection {
    pub record_id: String,
    pub workspace_id: String,
    pub watermark_uid: String,
    pub watermark_revision: i64,
    pub record_version: i64,
    pub etag: String,
    pub deleted: bool,
}

#[derive(Debug, Error)]
pub enum CloudCopyrightError {
    #[error("workspace access forbidden")]
    Forbidden,
    #[error("workspace membership revoked")]
    MembershipRevoked,
    #[error("workspace role denied")]
    RoleDenied,
    #[error("workspace membership not found")]
    MembershipNotFound,
    #[error("device is not active")]
    DeviceNotActive,
    #[error("record not found")]
    RecordNotFound,
    #[error("conflict_version_changed")]
    ConflictVersionChanged,
    #[error("conflict_payload_changed")]
    ConflictPayloadChanged,
    #[error("invalid change: {0}")]
    InvalidChange(&'static str),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug)]
struct MembershipSnapshot {
    membership_id: String,
    role: String,
    status: String,
}

async fn load_membership_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &CloudCopyrightActor,
    workspace_id: &str,
) -> Result<MembershipSnapshot, CloudCopyrightError> {
    load_membership_query(transaction, actor, workspace_id, true).await
}

async fn load_membership(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &CloudCopyrightActor,
    workspace_id: &str,
) -> Result<MembershipSnapshot, CloudCopyrightError> {
    load_membership_query(transaction, actor, workspace_id, false).await
}

async fn load_membership_query(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &CloudCopyrightActor,
    workspace_id: &str,
    for_update: bool,
) -> Result<MembershipSnapshot, CloudCopyrightError> {
    let sql = if for_update {
        "SELECT membership_id, role, status
         FROM cloud_copyright_workspace_memberships
         WHERE workspace_id = $1 AND account_id = $2
         FOR UPDATE"
    } else {
        "SELECT membership_id, role, status
         FROM cloud_copyright_workspace_memberships
         WHERE workspace_id = $1 AND account_id = $2"
    };
    let row = sqlx::query(sql)
        .bind(workspace_id)
        .bind(&actor.account_id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(CloudCopyrightError::Forbidden)?;
    let snapshot = MembershipSnapshot {
        membership_id: row.try_get("membership_id")?,
        role: row.try_get("role")?,
        status: row.try_get("status")?,
    };
    if snapshot.status == "removed" {
        return Err(CloudCopyrightError::MembershipRevoked);
    }
    if snapshot.status != "active" {
        return Err(CloudCopyrightError::Forbidden);
    }
    Ok(snapshot)
}

async fn require_active_device(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &CloudCopyrightActor,
) -> Result<(), CloudCopyrightError> {
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM cloud_devices
            WHERE id = $1 AND account_id = $2 AND registered = TRUE
         )",
    )
    .bind(&actor.device_id)
    .bind(&actor.account_id)
    .fetch_one(&mut **transaction)
    .await?;
    if !active {
        return Err(CloudCopyrightError::DeviceNotActive);
    }
    Ok(())
}

fn require_record_read(membership: &MembershipSnapshot) -> Result<(), CloudCopyrightError> {
    if matches!(
        membership.role.as_str(),
        "owner" | "admin" | "editor" | "viewer"
    ) {
        Ok(())
    } else {
        Err(CloudCopyrightError::RoleDenied)
    }
}

fn require_record_write(membership: &MembershipSnapshot) -> Result<(), CloudCopyrightError> {
    if matches!(membership.role.as_str(), "owner" | "admin" | "editor") {
        Ok(())
    } else {
        Err(CloudCopyrightError::RoleDenied)
    }
}

fn require_member_admin(membership: &MembershipSnapshot) -> Result<(), CloudCopyrightError> {
    if matches!(membership.role.as_str(), "owner" | "admin") {
        Ok(())
    } else {
        Err(CloudCopyrightError::RoleDenied)
    }
}

async fn append_audit(
    transaction: &mut Transaction<'_, Postgres>,
    actor: &CloudCopyrightActor,
    membership: &MembershipSnapshot,
    workspace_id: &str,
    action: &str,
    target_type: &str,
    target_id: &str,
    request_digest: &str,
) -> Result<(), CloudCopyrightError> {
    sqlx::query(
        "SELECT workspace_id
         FROM cloud_copyright_workspaces
         WHERE workspace_id = $1
         FOR UPDATE",
    )
    .bind(workspace_id)
    .fetch_one(&mut **transaction)
    .await?;
    let previous_hash = sqlx::query_scalar::<_, String>(
        "SELECT event_hash
         FROM cloud_copyright_audit_events
         WHERE workspace_id = $1
         ORDER BY sequence DESC
         LIMIT 1",
    )
    .bind(workspace_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let event_hash = audit_hash(
        previous_hash.as_deref(),
        workspace_id,
        &actor.account_id,
        action,
        target_id,
        request_digest,
    );
    sqlx::query(
        "INSERT INTO cloud_copyright_audit_events (
            audit_event_id, workspace_id, actor_account_id, actor_membership_id,
            actor_device_id, action, target_type, target_id, request_digest,
            previous_event_hash, event_hash, created_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())",
    )
    .bind(format!("cca_{}", Uuid::new_v4().simple()))
    .bind(workspace_id)
    .bind(&actor.account_id)
    .bind(&membership.membership_id)
    .bind(&actor.device_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(request_digest)
    .bind(previous_hash)
    .bind(event_hash)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn validate_change(command: &CloudCopyrightChangeCommand) -> Result<(), CloudCopyrightError> {
    if command.change_id.trim().is_empty()
        || command.workspace_id.trim().is_empty()
        || command.record_id.trim().is_empty()
        || command.idempotency_key.trim().is_empty()
        || command.request_digest.trim().is_empty()
        || command.base_record_version < 1
    {
        return Err(CloudCopyrightError::InvalidChange("required_fields"));
    }
    Ok(())
}

fn record_etag(record_id: &str, version: i64, request_digest: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(record_id.as_bytes());
    hasher.update(version.to_le_bytes());
    hasher.update(request_digest.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn audit_hash(
    previous_hash: Option<&str>,
    workspace_id: &str,
    actor_account_id: &str,
    action: &str,
    target_id: &str,
    request_digest: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(previous_hash.unwrap_or_default().as_bytes());
    hasher.update(workspace_id.as_bytes());
    hasher.update(actor_account_id.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(target_id.as_bytes());
    hasher.update(request_digest.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
