use std::future::Future;
use std::sync::Arc;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::repository::AuthRepository;
use crate::schema::{
    AccountDevice, AccountDevicesResponse, AuthChallengeRequest, AuthChallengeResponse,
    AuthLogoutRequest, AuthLogoutResponse, AuthRefreshRequest, AuthSessionRequest, CloudAccount,
    CloudAccountSession, CloudAccountSnapshot, CloudCreatorProfile, CloudDevice, CloudEntitlement,
    CloudWorkspace, ContinueAccountRequest, RevokeDeviceResponse,
};
use crate::storage::StorageError;

#[derive(Clone)]
pub struct PostgresAuthRepository {
    pool: PgPool,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PostgresAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("postgres auth repository runtime must be available"),
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

impl AuthRepository for PostgresAuthRepository {
    fn continue_account(
        &self,
        request: &ContinueAccountRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        let password = if request.password.trim().is_empty() {
            request.verification_code.trim()
        } else {
            request.password.trim()
        };
        let request = AuthSessionRequest {
            identifier: request.identifier.clone(),
            challenge_id: None,
            verification_code: String::new(),
            password: password.to_string(),
            device: request.device.clone(),
            local_creator_profile: request.local_creator_profile.clone(),
        };
        self.create_auth_session(&request)
    }

    fn create_auth_challenge(
        &self,
        request: &AuthChallengeRequest,
    ) -> Result<AuthChallengeResponse, StorageError> {
        self.run(create_auth_challenge_pg(self.pool.clone(), request.clone()))
    }

    fn create_auth_session(
        &self,
        request: &AuthSessionRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        self.run(create_auth_session_pg(self.pool.clone(), request.clone()))
    }

    fn refresh_auth_session(
        &self,
        request: &AuthRefreshRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        self.run(refresh_auth_session_pg(self.pool.clone(), request.clone()))
    }

    fn logout_auth_session(
        &self,
        request: &AuthLogoutRequest,
    ) -> Result<AuthLogoutResponse, StorageError> {
        self.run(logout_auth_session_pg(self.pool.clone(), request.clone()))
    }

    fn list_devices(&self, access_token: &str) -> Result<AccountDevicesResponse, StorageError> {
        self.run(list_devices_pg(self.pool.clone(), access_token.to_string()))
    }

    fn revoke_device(
        &self,
        access_token: &str,
        device_id: &str,
    ) -> Result<RevokeDeviceResponse, StorageError> {
        self.run(revoke_device_pg(
            self.pool.clone(),
            access_token.to_string(),
            device_id.to_string(),
        ))
    }
}

async fn create_auth_challenge_pg(
    pool: PgPool,
    request: AuthChallengeRequest,
) -> Result<AuthChallengeResponse, StorageError> {
    let identifier = normalize_identifier(&request.identifier)?;
    let purpose = request.purpose.trim();
    if !matches!(
        purpose,
        "register_or_login" | "login" | "bind_identifier" | "reset_password"
    ) {
        return Err(StorageError::BadRequest("purpose is invalid".to_string()));
    }
    let client_device_id = request.client_device_id.trim();
    if client_device_id.is_empty() {
        return Err(StorageError::BadRequest(
            "clientDeviceId is required".to_string(),
        ));
    }
    ensure_auth_challenge_rate_limit_pg(&pool, &identifier, client_device_id).await?;
    let now = Utc::now();
    let challenge_id = format!(
        "chal_{}",
        short_id(&format!("{}:{}:{}", identifier, client_device_id, now))
    );
    let delivery_channel = auth_delivery_channel();
    let code = if delivery_channel == "fixture" {
        "000000".to_string()
    } else {
        generate_otp_code()
    };
    let code_salt = new_password_salt();
    let code_hash = auth_code_hash(&code, &code_salt);
    let expires_at = now + Duration::minutes(10);
    sqlx::query(
        "INSERT INTO auth_challenges (
            challenge_id, identifier, purpose, client_device_id, code_hash,
            code_salt, delivery_channel, expires_at, consumed_at, created_at,
            plain_code_for_delivery
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, $10)",
    )
    .bind(&challenge_id)
    .bind(&identifier)
    .bind(purpose)
    .bind(client_device_id)
    .bind(&code_hash)
    .bind(&code_salt)
    .bind(&delivery_channel)
    .bind(expires_at)
    .bind(now)
    .bind(if delivery_channel == "fixture" {
        None::<String>
    } else {
        Some(code.clone())
    })
    .execute(&pool)
    .await?;
    Ok(AuthChallengeResponse {
        challenge_id,
        delivery_channel: delivery_channel.clone(),
        expires_at,
        message: if delivery_channel == "fixture" {
            "本地研发环境未配置验证码投递服务，fixture 验证码为 000000。".to_string()
        } else {
            "如果账号可用，验证码会发送到对应联系方式。".to_string()
        },
        fixture_code: (delivery_channel == "fixture").then_some(code),
    })
}

async fn create_auth_session_pg(
    pool: PgPool,
    request: AuthSessionRequest,
) -> Result<CloudAccountSession, StorageError> {
    let identifier = normalize_identifier(&request.identifier)?;
    let display_name = request
        .local_creator_profile
        .display_name
        .trim()
        .to_string();
    let creator_display_name = if display_name.is_empty() {
        identifier.clone()
    } else {
        display_name
    };
    let creator_seed_ref = request.local_creator_profile.creator_seed_ref.trim();
    let creator_seed_ref = if creator_seed_ref.is_empty() {
        "local-seed-ref".to_string()
    } else {
        creator_seed_ref.to_string()
    };
    let password = request.password.trim();
    let password = if password.is_empty() {
        None
    } else {
        Some(password)
    };
    let challenge_id = request
        .challenge_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if password.is_none() && challenge_id.is_none() {
        return Err(StorageError::Unauthorized);
    }

    ensure_auth_login_rate_limit_pg(&pool, &identifier, &request.device.client_device_id).await?;
    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let result = if let Some(challenge_id) = challenge_id {
        match consume_auth_challenge_pg(
            &mut tx,
            challenge_id,
            &identifier,
            request.verification_code.trim(),
            now,
        )
        .await
        {
            Ok(()) => {
                ensure_account_pg(
                    &mut tx,
                    &identifier,
                    &creator_display_name,
                    &creator_seed_ref,
                    request.local_creator_profile.seed_envelope_version,
                    None,
                    now,
                )
                .await
            }
            Err(error) => Err(error),
        }
    } else {
        ensure_account_pg(
            &mut tx,
            &identifier,
            &creator_display_name,
            &creator_seed_ref,
            request.local_creator_profile.seed_envelope_version,
            password,
            now,
        )
        .await
    };
    let account = match result {
        Ok(account) => account,
        Err(error) => {
            let reason = match &error {
                StorageError::RateLimited(_) => "rate_limited",
                StorageError::Unauthorized => "invalid_credential",
                StorageError::BadRequest(_) => "bad_request",
                _ => "storage_error",
            };
            let _ = record_auth_attempt_pg(
                &mut tx,
                &identifier,
                Some(request.device.client_device_id.trim()),
                "login",
                "failure",
                reason,
                now,
            )
            .await;
            tx.commit().await?;
            return Err(error);
        }
    };
    let device = ensure_device_pg(&mut tx, &account.id, &request, now).await?;
    let session = create_session_pg(&mut tx, &account.id, &device.id, now).await?;
    record_auth_attempt_pg(
        &mut tx,
        &identifier,
        Some(request.device.client_device_id.trim()),
        "login",
        "success",
        if challenge_id.is_some() {
            "challenge"
        } else {
            "password"
        },
        now,
    )
    .await?;
    tx.commit().await?;
    let snapshot = account_snapshot_pg(&pool, account, device).await?;
    Ok(session_response(session, snapshot))
}

async fn refresh_auth_session_pg(
    pool: PgPool,
    request: AuthRefreshRequest,
) -> Result<CloudAccountSession, StorageError> {
    let refresh_token = request.refresh_token.trim();
    let device_id = request.device_id.trim();
    if refresh_token.is_empty() || device_id.is_empty() {
        return Err(StorageError::Unauthorized);
    }
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT account_id, device_id, revoked_at
         FROM cloud_sessions
         WHERE refresh_token = $1 AND device_id = $2
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(refresh_token)
    .bind(device_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    let existing = SessionRecord {
        account_id: row.try_get("account_id")?,
        device_id: row.try_get("device_id")?,
        revoked_at: row.try_get("revoked_at")?,
    };
    if existing.revoked_at.is_some() {
        return Err(StorageError::Unauthorized);
    }
    let now = Utc::now();
    sqlx::query(
        "UPDATE cloud_sessions
         SET revoked_at = $3, last_used_at = $3
         WHERE refresh_token = $1 AND device_id = $2 AND revoked_at IS NULL",
    )
    .bind(refresh_token)
    .bind(device_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let session =
        create_session_pg(&mut tx, &existing.account_id, &existing.device_id, now).await?;
    tx.commit().await?;
    let account = load_account_by_id_pg(&pool, &existing.account_id).await?;
    let device = load_device_by_id_pg(&pool, &existing.account_id, &existing.device_id).await?;
    let snapshot = account_snapshot_pg(&pool, account, device).await?;
    Ok(session_response(session, snapshot))
}

async fn logout_auth_session_pg(
    pool: PgPool,
    request: AuthLogoutRequest,
) -> Result<AuthLogoutResponse, StorageError> {
    let refresh_token = request.refresh_token.trim();
    let device_id = request.device_id.trim();
    if refresh_token.is_empty() || device_id.is_empty() {
        return Err(StorageError::Unauthorized);
    }
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE cloud_sessions
         SET revoked_at = COALESCE(revoked_at, $3), last_used_at = $3
         WHERE refresh_token = $1 AND device_id = $2",
    )
    .bind(refresh_token)
    .bind(device_id)
    .bind(now)
    .execute(&pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(StorageError::Unauthorized);
    }
    Ok(AuthLogoutResponse { ok: true })
}

async fn list_devices_pg(
    pool: PgPool,
    access_token: String,
) -> Result<AccountDevicesResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let rows = sqlx::query(
        "SELECT
            d.id,
            d.client_device_id,
            d.name,
            d.platform,
            d.app_version,
            d.registered,
            d.auto_sync_enabled,
            d.created_at,
            d.updated_at,
            (
                SELECT COUNT(*)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
                  AND s.revoked_at IS NULL
            ) AS active_session_count,
            (
                SELECT MAX(s.last_used_at)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
            ) AS last_seen_at
         FROM cloud_devices d
         WHERE d.account_id = $1
         ORDER BY d.updated_at DESC, d.id ASC",
    )
    .bind(&session.account_id)
    .fetch_all(&pool)
    .await?;
    let devices = rows
        .into_iter()
        .map(|row| {
            Ok(AccountDevice {
                id: row.try_get("id")?,
                client_device_id: row.try_get("client_device_id")?,
                name: row.try_get("name")?,
                platform: row.try_get("platform")?,
                app_version: row.try_get("app_version")?,
                registered: row.try_get("registered")?,
                auto_sync_enabled: row.try_get("auto_sync_enabled")?,
                is_current: row.try_get::<String, _>("id")? == session.device_id,
                active_session_count: row.try_get::<i64, _>("active_session_count")? as u32,
                last_seen_at: row.try_get("last_seen_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(AccountDevicesResponse { devices })
}

async fn revoke_device_pg(
    pool: PgPool,
    access_token: String,
    device_id: String,
) -> Result<RevokeDeviceResponse, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let device_id = device_id.trim();
    if device_id.is_empty() {
        return Err(StorageError::BadRequest("device_id_required".to_string()));
    }
    if device_id == session.device_id {
        return Err(StorageError::BadRequest(
            "cannot_revoke_current_device".to_string(),
        ));
    }
    let exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM cloud_devices WHERE account_id = $1 AND id = $2")
            .bind(&session.account_id)
            .bind(device_id)
            .fetch_optional(&pool)
            .await?;
    if exists.is_none() {
        return Err(StorageError::Unauthorized);
    }
    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let revoked = sqlx::query(
        "UPDATE cloud_sessions
         SET revoked_at = COALESCE(revoked_at, $3), last_used_at = $3
         WHERE account_id = $1 AND device_id = $2 AND revoked_at IS NULL",
    )
    .bind(&session.account_id)
    .bind(device_id)
    .bind(now)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query(
        "UPDATE cloud_devices
         SET registered = false, updated_at = $3
         WHERE account_id = $1 AND id = $2",
    )
    .bind(&session.account_id)
    .bind(device_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(RevokeDeviceResponse {
        ok: true,
        device_id: device_id.to_string(),
        revoked_session_count: revoked as u32,
    })
}

async fn ensure_auth_challenge_rate_limit_pg(
    pool: &PgPool,
    identifier: &str,
    client_device_id: &str,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let minute_ago = now - Duration::minutes(1);
    let hour_ago = now - Duration::hours(1);
    let recent_for_device: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_challenges
         WHERE identifier = $1 AND client_device_id = $2 AND created_at >= $3",
    )
    .bind(identifier)
    .bind(client_device_id)
    .bind(minute_ago)
    .fetch_one(pool)
    .await?;
    if recent_for_device >= 1 {
        return Err(StorageError::RateLimited(
            "auth_challenge_too_frequent".to_string(),
        ));
    }
    let hourly_for_identifier: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_challenges
         WHERE identifier = $1 AND created_at >= $2",
    )
    .bind(identifier)
    .bind(hour_ago)
    .fetch_one(pool)
    .await?;
    if hourly_for_identifier >= 5 {
        return Err(StorageError::RateLimited(
            "auth_challenge_hourly_limit".to_string(),
        ));
    }
    Ok(())
}

async fn ensure_auth_login_rate_limit_pg(
    pool: &PgPool,
    identifier: &str,
    client_device_id: &str,
) -> Result<(), StorageError> {
    let since = Utc::now() - Duration::minutes(15);
    let failed_for_identifier: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_attempts
         WHERE identifier = $1 AND outcome = 'failure' AND created_at >= $2",
    )
    .bind(identifier)
    .bind(since)
    .fetch_one(pool)
    .await?;
    let failed_for_device: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_attempts
         WHERE identifier = $1 AND client_device_id = $2 AND outcome = 'failure' AND created_at >= $3",
    )
    .bind(identifier)
    .bind(client_device_id)
    .bind(since)
    .fetch_one(pool)
    .await?;
    if failed_for_identifier >= 10 || failed_for_device >= 5 {
        return Err(StorageError::RateLimited(
            "auth_login_temporarily_limited".to_string(),
        ));
    }
    Ok(())
}

async fn consume_auth_challenge_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    challenge_id: &str,
    identifier: &str,
    verification_code: &str,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let row = sqlx::query(
        "SELECT code_hash, code_salt, expires_at, consumed_at
         FROM auth_challenges
         WHERE challenge_id = $1 AND identifier = $2",
    )
    .bind(challenge_id)
    .bind(identifier)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    let code_hash: String = row.try_get("code_hash")?;
    let code_salt: String = row.try_get("code_salt")?;
    let expires_at: DateTime<Utc> = row.try_get("expires_at")?;
    let consumed_at: Option<DateTime<Utc>> = row.try_get("consumed_at")?;
    if consumed_at.is_some() || expires_at < Utc::now() {
        return Err(StorageError::Unauthorized);
    }
    if auth_code_hash(verification_code.trim(), &code_salt) != code_hash {
        return Err(StorageError::Unauthorized);
    }
    sqlx::query("UPDATE auth_challenges SET consumed_at = $2 WHERE challenge_id = $1")
        .bind(challenge_id)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn ensure_account_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    identifier: &str,
    creator_display_name: &str,
    creator_seed_ref: &str,
    seed_envelope_version: u32,
    password: Option<&str>,
    now: DateTime<Utc>,
) -> Result<CloudAccountRow, StorageError> {
    let existing = sqlx::query(
        "SELECT id, password_hash, password_salt, password_hash_algorithm
         FROM cloud_accounts WHERE identifier = $1",
    )
    .bind(identifier)
    .fetch_optional(&mut **tx)
    .await?;
    let (account_id, password_hash, password_salt, password_hash_algorithm) = match existing {
        Some(row) => {
            let account_id: String = row.try_get("id")?;
            let stored_hash: Option<String> = row.try_get("password_hash")?;
            let stored_salt: Option<String> = row.try_get("password_salt")?;
            let stored_algorithm: Option<String> = row.try_get("password_hash_algorithm")?;
            match (stored_hash, stored_salt) {
                (Some(stored_hash), Some(stored_salt)) => {
                    if let Some(password) = password {
                        let algorithm = stored_algorithm.as_deref().unwrap_or("sha256");
                        if !verify_password(password, &stored_hash, &stored_salt, algorithm) {
                            return Err(StorageError::Unauthorized);
                        }
                        if algorithm != "argon2id" || !stored_hash.starts_with("$argon2") {
                            let salt = new_password_salt();
                            let hash = password_hash(password, &salt);
                            (account_id, hash, salt, "argon2id".to_string())
                        } else {
                            (account_id, stored_hash, stored_salt, "argon2id".to_string())
                        }
                    } else {
                        (
                            account_id,
                            stored_hash,
                            stored_salt,
                            algorithm_normalized(stored_algorithm),
                        )
                    }
                }
                _ => {
                    if let Some(password) = password {
                        let salt = new_password_salt();
                        let hash = password_hash(password, &salt);
                        (account_id, hash, salt, "argon2id".to_string())
                    } else {
                        (
                            account_id,
                            String::new(),
                            String::new(),
                            "argon2id".to_string(),
                        )
                    }
                }
            }
        }
        None => {
            if let Some(password) = password {
                let salt = new_password_salt();
                let hash = password_hash(password, &salt);
                (
                    format!("acct_{}", short_id(identifier)),
                    hash,
                    salt,
                    "argon2id".to_string(),
                )
            } else {
                (
                    format!("acct_{}", short_id(identifier)),
                    String::new(),
                    String::new(),
                    "argon2id".to_string(),
                )
            }
        }
    };
    let workspace_id = format!("ws_{}", short_id(&account_id));
    let creator_profile_id = format!("creator_{}", short_id(&account_id));
    let entitlement_id = format!("ent_{}", short_id(&account_id));
    let entitlement_features = default_entitlement_features();

    sqlx::query(
        "INSERT INTO cloud_accounts (
            id, identifier, password_hash, password_salt, password_hash_algorithm,
            display_name, workspace_id, workspace_name,
            creator_profile_id, creator_display_name, creator_seed_ref, seed_envelope_version,
            entitlement_id, entitlement_plan_name, entitlement_plan_code, entitlement_status,
            entitlement_features_json, created_at, updated_at
        ) VALUES ($1, $2, NULLIF($3, ''), NULLIF($4, ''), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        ON CONFLICT(identifier) DO UPDATE SET
            password_hash = COALESCE(excluded.password_hash, cloud_accounts.password_hash),
            password_salt = COALESCE(excluded.password_salt, cloud_accounts.password_salt),
            password_hash_algorithm = CASE
                WHEN excluded.password_hash IS NULL THEN cloud_accounts.password_hash_algorithm
                ELSE excluded.password_hash_algorithm
            END,
            display_name = excluded.display_name,
            creator_display_name = excluded.creator_display_name,
            creator_seed_ref = excluded.creator_seed_ref,
            seed_envelope_version = excluded.seed_envelope_version,
            updated_at = excluded.updated_at",
    )
    .bind(&account_id)
    .bind(identifier)
    .bind(&password_hash)
    .bind(&password_salt)
    .bind(&password_hash_algorithm)
    .bind(identifier)
    .bind(&workspace_id)
    .bind("个人空间")
    .bind(&creator_profile_id)
    .bind(creator_display_name)
    .bind(creator_seed_ref)
    .bind(seed_envelope_version as i32)
    .bind(&entitlement_id)
    .bind("免费版")
    .bind("free")
    .bind("free")
    .bind(entitlement_features)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;

    let row = sqlx::query(
        "SELECT id, display_name, workspace_id, workspace_name, creator_profile_id,
                creator_display_name, entitlement_id, entitlement_plan_name,
                entitlement_plan_code, entitlement_status, entitlement_features_json
         FROM cloud_accounts WHERE identifier = $1",
    )
    .bind(identifier)
    .fetch_one(&mut **tx)
    .await?;
    CloudAccountRow::from_row(row)
}

async fn ensure_device_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account_id: &str,
    request: &AuthSessionRequest,
    now: DateTime<Utc>,
) -> Result<CloudDeviceRow, StorageError> {
    let device_id = request.device.client_device_id.trim();
    let name = request.device.name.trim();
    let platform = request.device.platform.trim();
    let app_version = request.device.app_version.trim();
    let device_id = if device_id.is_empty() {
        format!("device_{}", short_id(account_id))
    } else {
        device_id.to_string()
    };
    let device_name = if name.is_empty() {
        "当前设备".to_string()
    } else {
        name.to_string()
    };
    let platform = if platform.is_empty() {
        "unknown".to_string()
    } else {
        platform.to_string()
    };
    let app_version = if app_version.is_empty() {
        "0.1.0".to_string()
    } else {
        app_version.to_string()
    };
    sqlx::query(
        "INSERT INTO cloud_devices (
            id, account_id, client_device_id, name, platform, app_version,
            public_key, registered, auto_sync_enabled, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, true, true, $8, $9)
        ON CONFLICT(id) DO UPDATE SET
            account_id = excluded.account_id,
            client_device_id = excluded.client_device_id,
            name = excluded.name,
            platform = excluded.platform,
            app_version = excluded.app_version,
            public_key = excluded.public_key,
            registered = excluded.registered,
            updated_at = excluded.updated_at",
    )
    .bind(&device_id)
    .bind(account_id)
    .bind(request.device.client_device_id.trim())
    .bind(&device_name)
    .bind(&platform)
    .bind(&app_version)
    .bind(request.device.public_key.clone())
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    let row = sqlx::query(
        "SELECT id, name, platform, auto_sync_enabled FROM cloud_devices
         WHERE account_id = $1 AND client_device_id = $2",
    )
    .bind(account_id)
    .bind(request.device.client_device_id.trim())
    .fetch_one(&mut **tx)
    .await?;
    Ok(CloudDeviceRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        platform: row.try_get("platform")?,
        auto_sync_enabled: row.try_get("auto_sync_enabled")?,
    })
}

async fn record_auth_attempt_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    identifier: &str,
    client_device_id: Option<&str>,
    attempt_type: &str,
    outcome: &str,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let attempt_id = format!(
        "auth_attempt_{}",
        short_id(&format!(
            "{}:{}:{}:{}:{}",
            identifier,
            client_device_id.unwrap_or_default(),
            attempt_type,
            outcome,
            now
        ))
    );
    sqlx::query(
        "INSERT INTO auth_attempts (
            attempt_id, identifier, client_device_id, attempt_type, outcome, reason, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT(attempt_id) DO NOTHING",
    )
    .bind(attempt_id)
    .bind(identifier)
    .bind(client_device_id)
    .bind(attempt_type)
    .bind(outcome)
    .bind(reason)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create_session_pg(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account_id: &str,
    device_id: &str,
    now: DateTime<Utc>,
) -> Result<SessionTokenRow, StorageError> {
    let token_nonce = new_password_salt();
    let access_token = format!(
        "hsat_{}_{}_{}",
        short_id(account_id),
        short_id(device_id),
        short_id(&format!("{now}:{token_nonce}:access"))
    );
    let refresh_token = format!(
        "hsrt_{}_{}_{}",
        short_id(account_id),
        short_id(device_id),
        short_id(&format!("{now}:{token_nonce}:refresh"))
    );
    let expires_at = now + Duration::minutes(60);
    let refresh_expires_at = now + Duration::days(90);
    let token_family_id = format!(
        "family_{}",
        short_id(&format!("{account_id}:{device_id}:{token_nonce}"))
    );
    sqlx::query(
        "INSERT INTO cloud_sessions (
            access_token, refresh_token, account_id, device_id, created_at, revoked_at,
            expires_at, refresh_expires_at, last_used_at, token_family_id
        ) VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $5, $8)",
    )
    .bind(&access_token)
    .bind(&refresh_token)
    .bind(account_id)
    .bind(device_id)
    .bind(now)
    .bind(expires_at)
    .bind(refresh_expires_at)
    .bind(token_family_id)
    .execute(&mut **tx)
    .await?;
    Ok(SessionTokenRow {
        access_token,
        refresh_token,
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

async fn load_account_by_id_pg(
    pool: &PgPool,
    account_id: &str,
) -> Result<CloudAccountRow, StorageError> {
    let row = sqlx::query(
        "SELECT id, display_name, workspace_id, workspace_name,
                creator_profile_id, creator_display_name,
                entitlement_id, entitlement_plan_name, entitlement_plan_code,
                entitlement_status, entitlement_features_json
         FROM cloud_accounts
         WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    CloudAccountRow::from_row(row)
}

async fn load_device_by_id_pg(
    pool: &PgPool,
    account_id: &str,
    device_id: &str,
) -> Result<CloudDeviceRow, StorageError> {
    let row = sqlx::query(
        "SELECT id, name, platform, auto_sync_enabled FROM cloud_devices
         WHERE account_id = $1 AND id = $2",
    )
    .bind(account_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    Ok(CloudDeviceRow {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        platform: row.try_get("platform")?,
        auto_sync_enabled: row.try_get("auto_sync_enabled")?,
    })
}

async fn account_snapshot_pg(
    pool: &PgPool,
    account: CloudAccountRow,
    device: CloudDeviceRow,
) -> Result<CloudAccountSnapshot, StorageError> {
    let sync_policy = sync_policy_for_entitlement_and_preference(
        &account.entitlement_features,
        device.auto_sync_enabled,
    );
    let cloud_vault_cursor = device_cursor_pg(pool, &account.id, &device.id).await?;
    Ok(CloudAccountSnapshot {
        account: CloudAccount {
            id: account.id,
            display_name: account.display_name,
        },
        workspace: CloudWorkspace {
            id: account.workspace_id,
            name: account.workspace_name,
        },
        device: CloudDevice {
            id: device.id,
            name: Some(device.name),
            platform: Some(device.platform),
            registered: true,
        },
        creator_profile: CloudCreatorProfile {
            id: account.creator_profile_id,
            display_name: account.creator_display_name,
            is_default: true,
        },
        entitlement: CloudEntitlement {
            id: account.entitlement_id,
            plan_name: Some(account.entitlement_plan_name),
            plan_code: account.entitlement_plan_code,
            status: account.entitlement_status,
            features: account.entitlement_features,
        },
        sync_policy,
        cloud_vault_cursor,
    })
}

async fn device_cursor_pg(
    pool: &PgPool,
    account_id: &str,
    device_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor: Option<String> = sqlx::query_scalar(
        "SELECT cursor
         FROM cloud_device_cursors
         WHERE account_id = $1 AND device_id = $2",
    )
    .bind(account_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await?;
    Ok(cursor)
}

fn session_response(
    session: SessionTokenRow,
    snapshot: CloudAccountSnapshot,
) -> CloudAccountSession {
    CloudAccountSession {
        access_token: session.access_token,
        refresh_token: session.refresh_token,
        account: snapshot.account,
        workspace: snapshot.workspace,
        device: snapshot.device,
        creator_profile: snapshot.creator_profile,
        entitlement: snapshot.entitlement,
        sync_policy: snapshot.sync_policy,
        cloud_vault_cursor: snapshot.cloud_vault_cursor,
    }
}

#[derive(Debug, Clone)]
struct CloudAccountRow {
    id: String,
    display_name: String,
    workspace_id: String,
    workspace_name: String,
    creator_profile_id: String,
    creator_display_name: String,
    entitlement_id: String,
    entitlement_plan_name: String,
    entitlement_plan_code: String,
    entitlement_status: String,
    entitlement_features: serde_json::Value,
}

impl CloudAccountRow {
    fn from_row(row: sqlx::postgres::PgRow) -> Result<Self, StorageError> {
        Ok(Self {
            id: row.try_get("id")?,
            display_name: row.try_get("display_name")?,
            workspace_id: row.try_get("workspace_id")?,
            workspace_name: row.try_get("workspace_name")?,
            creator_profile_id: row.try_get("creator_profile_id")?,
            creator_display_name: row.try_get("creator_display_name")?,
            entitlement_id: row.try_get("entitlement_id")?,
            entitlement_plan_name: row.try_get("entitlement_plan_name")?,
            entitlement_plan_code: row.try_get("entitlement_plan_code")?,
            entitlement_status: row.try_get("entitlement_status")?,
            entitlement_features: row.try_get("entitlement_features_json")?,
        })
    }
}

#[derive(Debug, Clone)]
struct CloudDeviceRow {
    id: String,
    name: String,
    platform: String,
    auto_sync_enabled: bool,
}

#[derive(Debug, Clone)]
struct SessionTokenRow {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    account_id: String,
    device_id: String,
    revoked_at: Option<DateTime<Utc>>,
}

fn legacy_sha256_password_hash(password: &str, salt_hex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt_hex.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn auth_code_hash(code: &str, salt_hex: &str) -> String {
    legacy_sha256_password_hash(code, salt_hex)
}

fn password_hash(password: &str, salt_hex: &str) -> String {
    let salt = SaltString::from_b64(salt_hex).unwrap_or_else(|_| SaltString::generate(&mut OsRng));
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .unwrap_or_else(|_| legacy_sha256_password_hash(password, salt_hex))
}

fn verify_password(password: &str, stored_hash: &str, stored_salt: &str, algorithm: &str) -> bool {
    if stored_hash.starts_with("$argon2") || algorithm == "argon2id" {
        return PasswordHash::new(stored_hash)
            .ok()
            .and_then(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .ok()
            })
            .is_some();
    }
    legacy_sha256_password_hash(password, stored_salt) == stored_hash
}

fn new_password_salt() -> String {
    let mut salt = [0_u8; 16];
    OsRng.fill_bytes(&mut salt);
    hex_string(&salt)
}

fn generate_otp_code() -> String {
    let mut bytes = [0_u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let value = u32::from_le_bytes(bytes) % 1_000_000;
    format!("{value:06}")
}

fn auth_delivery_channel() -> String {
    if std::env::var("HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        std::env::var("HIDDENSHIELD_AUTH_OTP_DELIVERY_CHANNEL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "email_or_sms".to_string())
    } else {
        "fixture".to_string()
    }
}

fn algorithm_normalized(value: Option<String>) -> String {
    match value.as_deref() {
        Some("argon2id") => "argon2id".to_string(),
        _ => "sha256".to_string(),
    }
}

fn short_id(input: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in input.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn normalize_identifier(input: &str) -> Result<String, StorageError> {
    let identifier = input.trim().to_lowercase();
    if identifier.is_empty() {
        return Err(StorageError::BadRequest(
            "identifier is required".to_string(),
        ));
    }
    Ok(identifier)
}

fn default_entitlement_features() -> serde_json::Value {
    serde_json::json!({
        "cloud_sync": false,
        "batch_processing": false,
        "report_export": false,
        "cloud_batch_processing": false,
        "cloud_video_processing": false,
        "priority_queue": false,
        "team_workspace": false,
        "api_access": false
    })
}

fn sync_policy_for_entitlement_and_preference(
    entitlement_features: &serde_json::Value,
    auto_sync_enabled: bool,
) -> String {
    let cloud_sync_entitled = entitlement_features
        .get("cloud_sync")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if !cloud_sync_entitled {
        "local_only_plan_limit".to_string()
    } else if auto_sync_enabled {
        "auto_sync_enabled".to_string()
    } else {
        "manual_local_only".to_string()
    }
}
