use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::offline_license::derive_installation_id_v1;

const KEYRING_SERVICE: &str = "com.hiddenshield.desktop";
const KEYRING_USER: &str = "offline-license-installation-secret-v1";
const KEYRING_SECURITY_ANCHOR_USER: &str = "offline-license-security-anchor-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallationIdentity {
    pub installation_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StoredInstallationIdentity {
    pub installation_id: String,
    pub salt_base64_url: String,
    pub secret_fingerprint_sha256: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StoredOfflineLicense {
    pub signed_token: String,
    pub token_sha256: String,
    pub license_id: String,
    pub installation_id: String,
    pub product_code: String,
    pub key_id: String,
    pub issued_at: String,
    pub not_before: String,
    pub expires_at: String,
    pub imported_at: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StoredRevocationList {
    pub key_id: String,
    pub signed_token: String,
    pub token_sha256: String,
    pub list_id: String,
    pub sequence: u64,
    pub generated_at: String,
    pub imported_at: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OfflineLicenseAuditEvent {
    pub occurred_at: String,
    pub event_type: String,
    pub outcome: String,
    pub installation_id: Option<String>,
    pub artifact_id: Option<String>,
    pub key_id: Option<String>,
    pub detail_code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSecurityAnchor {
    pub highest_observed_utc: Option<String>,
    pub revocation_high_water: BTreeMap<String, RevocationHighWater>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RevocationHighWater {
    pub sequence: u64,
    pub token_sha256: String,
}

pub trait InstallationSecretStore: Send + Sync {
    fn load(&self) -> Result<Option<Vec<u8>>, String>;
    fn store(&self, secret: &[u8]) -> Result<(), String>;
    fn load_security_anchor(&self) -> Result<OfflineSecurityAnchor, String>;
    fn store_security_anchor(&self, anchor: &OfflineSecurityAnchor) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct OsKeyringInstallationSecretStore;

impl InstallationSecretStore for OsKeyringInstallationSecretStore {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        match entry.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("offline_license_secure_storage_unavailable".to_string()),
        }
    }

    fn store(&self, secret: &[u8]) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        entry
            .set_secret(secret)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }

    fn load_security_anchor(&self) -> Result<OfflineSecurityAnchor, String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_SECURITY_ANCHOR_USER)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        match entry.get_password() {
            Ok(value) => serde_json::from_str(&value)
                .map_err(|_| "offline_license_secure_storage_unavailable".to_string()),
            Err(keyring::Error::NoEntry) => Ok(OfflineSecurityAnchor::default()),
            Err(_) => Err("offline_license_secure_storage_unavailable".to_string()),
        }
    }

    fn store_security_anchor(&self, anchor: &OfflineSecurityAnchor) -> Result<(), String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_SECURITY_ANCHOR_USER)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        let value = serde_json::to_string(anchor)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        entry
            .set_password(&value)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct MemoryInstallationSecretStore {
    secret: Mutex<Option<Vec<u8>>>,
    security_anchor: Mutex<OfflineSecurityAnchor>,
}

#[cfg(test)]
impl MemoryInstallationSecretStore {
    pub fn with_secret(secret: Vec<u8>) -> Arc<Self> {
        Arc::new(Self {
            secret: Mutex::new(Some(secret)),
            security_anchor: Mutex::new(OfflineSecurityAnchor::default()),
        })
    }

    pub fn with_state(
        secret: Option<Vec<u8>>,
        security_anchor: OfflineSecurityAnchor,
    ) -> Arc<Self> {
        Arc::new(Self {
            secret: Mutex::new(secret),
            security_anchor: Mutex::new(security_anchor),
        })
    }

    pub fn snapshot_state(&self) -> (Option<Vec<u8>>, OfflineSecurityAnchor) {
        (
            self.secret.lock().unwrap().clone(),
            self.security_anchor.lock().unwrap().clone(),
        )
    }
}

#[cfg(test)]
impl InstallationSecretStore for MemoryInstallationSecretStore {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        self.secret
            .lock()
            .map(|secret| secret.clone())
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }

    fn store(&self, secret: &[u8]) -> Result<(), String> {
        let mut stored = self
            .secret
            .lock()
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        *stored = Some(secret.to_vec());
        Ok(())
    }

    fn load_security_anchor(&self) -> Result<OfflineSecurityAnchor, String> {
        self.security_anchor
            .lock()
            .map(|anchor| anchor.clone())
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }

    fn store_security_anchor(&self, anchor: &OfflineSecurityAnchor) -> Result<(), String> {
        let mut stored = self
            .security_anchor
            .lock()
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        *stored = anchor.clone();
        Ok(())
    }
}

pub fn get_or_create_installation_identity(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
) -> Result<InstallationIdentity, String> {
    if let Some(stored) = load_installation_identity(conn)? {
        let secret = secret_store
            .load()?
            .ok_or_else(|| "offline_license_secure_storage_unavailable".to_string())?;
        verify_stored_identity(&stored, &secret)?;
        return Ok(InstallationIdentity {
            installation_id: stored.installation_id,
            created_at: stored.created_at,
        });
    }

    let secret = match secret_store.load()? {
        Some(secret) if secret.len() == 32 => secret,
        Some(_) => return Err("offline_license_secure_storage_unavailable".to_string()),
        None => {
            let mut secret = [0u8; 32];
            getrandom::getrandom(&mut secret)
                .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
            secret_store.store(&secret)?;
            secret.to_vec()
        }
    };
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt)
        .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
    let installation_id = derive_installation_id_v1(&secret, &salt)?;
    let now = Utc::now().to_rfc3339();
    let stored = StoredInstallationIdentity {
        installation_id: installation_id.clone(),
        salt_base64_url: URL_SAFE_NO_PAD.encode(salt),
        secret_fingerprint_sha256: sha256_hex(&secret),
        created_at: now.clone(),
        updated_at: now,
    };
    save_installation_identity(conn, &stored)?;
    append_audit_event(
        conn,
        &OfflineLicenseAuditEvent {
            occurred_at: Utc::now().to_rfc3339(),
            event_type: "installation_identity_created".to_string(),
            outcome: "accepted".to_string(),
            installation_id: Some(installation_id.clone()),
            artifact_id: None,
            key_id: None,
            detail_code: None,
        },
    )?;
    Ok(InstallationIdentity {
        installation_id,
        created_at: stored.created_at,
    })
}

pub fn load_installation_identity(
    conn: &Connection,
) -> Result<Option<StoredInstallationIdentity>, String> {
    conn.query_row(
        "SELECT installation_id, salt_base64_url, secret_fingerprint_sha256, created_at, updated_at
         FROM installation_identity WHERE id = 1",
        [],
        |row| {
            Ok(StoredInstallationIdentity {
                installation_id: row.get(0)?,
                salt_base64_url: row.get(1)?,
                secret_fingerprint_sha256: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|error| format!("offline_license_db_error:{error}"))
}

fn save_installation_identity(
    conn: &Connection,
    identity: &StoredInstallationIdentity,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO installation_identity (
            id, installation_id, salt_base64_url, secret_fingerprint_sha256, created_at, updated_at
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
        params![
            identity.installation_id,
            identity.salt_base64_url,
            identity.secret_fingerprint_sha256,
            identity.created_at,
            identity.updated_at,
        ],
    )
    .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

fn verify_stored_identity(
    identity: &StoredInstallationIdentity,
    secret: &[u8],
) -> Result<(), String> {
    if secret.len() != 32 || sha256_hex(secret) != identity.secret_fingerprint_sha256 {
        return Err("offline_license_installation_identity_mismatch".to_string());
    }
    let salt = URL_SAFE_NO_PAD
        .decode(&identity.salt_base64_url)
        .map_err(|_| "offline_license_installation_identity_mismatch".to_string())?;
    let derived = derive_installation_id_v1(secret, &salt)?;
    if derived != identity.installation_id {
        return Err("offline_license_installation_identity_mismatch".to_string());
    }
    Ok(())
}

pub fn load_offline_license(conn: &Connection) -> Result<Option<StoredOfflineLicense>, String> {
    conn.query_row(
        "SELECT signed_token, token_sha256, license_id, installation_id, product_code, key_id,
                issued_at, not_before, expires_at, imported_at
         FROM offline_license_state WHERE id = 1",
        [],
        |row| {
            Ok(StoredOfflineLicense {
                signed_token: row.get(0)?,
                token_sha256: row.get(1)?,
                license_id: row.get(2)?,
                installation_id: row.get(3)?,
                product_code: row.get(4)?,
                key_id: row.get(5)?,
                issued_at: row.get(6)?,
                not_before: row.get(7)?,
                expires_at: row.get(8)?,
                imported_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(|error| format!("offline_license_db_error:{error}"))
}

pub fn save_offline_license(
    conn: &Connection,
    license: &StoredOfflineLicense,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO offline_license_state (
            id, signed_token, token_sha256, license_id, installation_id, product_code, key_id,
            issued_at, not_before, expires_at, imported_at
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
            signed_token = excluded.signed_token,
            token_sha256 = excluded.token_sha256,
            license_id = excluded.license_id,
            installation_id = excluded.installation_id,
            product_code = excluded.product_code,
            key_id = excluded.key_id,
            issued_at = excluded.issued_at,
            not_before = excluded.not_before,
            expires_at = excluded.expires_at,
            imported_at = excluded.imported_at",
        params![
            license.signed_token,
            license.token_sha256,
            license.license_id,
            license.installation_id,
            license.product_code,
            license.key_id,
            license.issued_at,
            license.not_before,
            license.expires_at,
            license.imported_at,
        ],
    )
    .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

pub fn clear_offline_license(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM offline_license_state WHERE id = 1", [])
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

pub fn load_revocation_lists(conn: &Connection) -> Result<Vec<StoredRevocationList>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT key_id, signed_token, token_sha256, list_id, sequence, generated_at, imported_at
             FROM offline_revocation_lists ORDER BY key_id",
        )
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredRevocationList {
                key_id: row.get(0)?,
                signed_token: row.get(1)?,
                token_sha256: row.get(2)?,
                list_id: row.get(3)?,
                sequence: row.get::<_, i64>(4)? as u64,
                generated_at: row.get(5)?,
                imported_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("offline_license_db_error:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(rows)
}

pub fn save_revocation_list(conn: &Connection, list: &StoredRevocationList) -> Result<(), String> {
    conn.execute(
        "INSERT INTO offline_revocation_lists (
            key_id, signed_token, token_sha256, list_id, sequence, generated_at, imported_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(key_id) DO UPDATE SET
            signed_token = excluded.signed_token,
            token_sha256 = excluded.token_sha256,
            list_id = excluded.list_id,
            sequence = excluded.sequence,
            generated_at = excluded.generated_at,
            imported_at = excluded.imported_at",
        params![
            list.key_id,
            list.signed_token,
            list.token_sha256,
            list.list_id,
            list.sequence as i64,
            list.generated_at,
            list.imported_at,
        ],
    )
    .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

pub fn load_highest_observed_utc(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT highest_observed_utc
         FROM offline_license_security_state
         WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(|error| format!("offline_license_db_error:{error}"))
}

pub fn save_highest_observed_utc(
    conn: &Connection,
    highest_observed_utc: &str,
    updated_at: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO offline_license_security_state (
            id, highest_observed_utc, updated_at
         ) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET
            highest_observed_utc = excluded.highest_observed_utc,
            updated_at = excluded.updated_at",
        params![highest_observed_utc, updated_at],
    )
    .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

pub fn append_audit_event(
    conn: &Connection,
    event: &OfflineLicenseAuditEvent,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO offline_license_audit (
            occurred_at, event_type, outcome, installation_id, artifact_id, key_id, detail_code
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event.occurred_at,
            event.event_type,
            event.outcome,
            event.installation_id,
            event.artifact_id,
            event.key_id,
            event.detail_code,
        ],
    )
    .map_err(|error| format!("offline_license_db_error:{error}"))?;
    Ok(())
}

pub fn token_sha256(token: &str) -> String {
    sha256_hex(token.as_bytes())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    #[test]
    fn copied_identity_metadata_rejects_a_different_keyring_secret() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        let first_store = MemoryInstallationSecretStore::with_secret(vec![7u8; 32]);
        get_or_create_installation_identity(&conn, first_store.as_ref()).unwrap();

        let copied_machine_store = MemoryInstallationSecretStore::with_secret(vec![8u8; 32]);
        assert_eq!(
            get_or_create_installation_identity(&conn, copied_machine_store.as_ref()).unwrap_err(),
            "offline_license_installation_identity_mismatch"
        );
    }
}
