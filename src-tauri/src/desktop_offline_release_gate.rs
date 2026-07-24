use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use keyring::Entry;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::offline_license::{InstallationSecretStore, OfflineSecurityAnchor};
use crate::{db, entitlements};

const SECRET_USER: &str = "offline-license-installation-secret-v1";
const SECURITY_ANCHOR_USER: &str = "offline-license-security-anchor-v1";

pub fn run(arguments: Vec<String>) -> Result<(), String> {
    let mode = required_arg(&arguments, "--mode")?;
    let db_path = required_arg(&arguments, "--db")?;
    let scope = required_arg(&arguments, "--scope")?;
    let service = format!("com.hiddenshield.desktop.runtime-qa.{scope}");
    let store = ScopedKeyringInstallationSecretStore { service };
    let conn = Connection::open(db_path)
        .map_err(|error| format!("offline_release_gate_db_open_failed:{error}"))?;
    db::schema::run_migrations(&conn)
        .map_err(|error| format!("offline_release_gate_db_migration_failed:{error}"))?;

    match mode {
        "request" => {
            let output = required_arg(&arguments, "--output")?;
            let request = entitlements::export_activation_request(&conn, &store)?;
            fs::write(output, request.token.as_bytes())
                .map_err(|error| format!("offline_release_gate_request_write_failed:{error}"))?;
            print_json(&serde_json::json!({
                "mode": mode,
                "tokenPrefix": "HSREQ1",
                "installationId": request.installation_id,
                "output": output,
            }))
        }
        "import" => {
            let token = read_token(required_arg(&arguments, "--license")?)?;
            let status = entitlements::import_offline_license(&conn, &store, &token)?;
            print_status(mode, status)
        }
        "status" => {
            let status = entitlements::get_offline_license_status(&conn, &store)?;
            print_status(mode, status)
        }
        "revoke" => {
            let token = read_token(required_arg(&arguments, "--revocations")?)?;
            let status = entitlements::import_revocation_list(&conn, &store, &token)?;
            print_status(mode, status)
        }
        "expect-expired" => {
            let token = read_token(required_arg(&arguments, "--license")?)?;
            match entitlements::import_offline_license(&conn, &store, &token) {
                Ok(status) => Err(format!(
                    "offline_release_gate_expired_license_unexpectedly_accepted:{}",
                    status.status
                )),
                Err(error) if error == "offline_license_expired" => {
                    print_json(&serde_json::json!({
                        "mode": mode,
                        "status": "rejected",
                        "errorCode": error,
                    }))
                }
                Err(error) => Err(error),
            }
        }
        _ => Err(format!("offline_release_gate_unknown_mode:{mode}")),
    }
}

fn print_status(mode: &str, status: entitlements::OfflineLicenseStatus) -> Result<(), String> {
    print_json(&serde_json::json!({
        "mode": mode,
        "status": status.status,
        "installationId": status.installation_id,
        "licenseId": status.license_id,
        "productCode": status.product_code,
        "keyId": status.key_id,
        "expiresAt": status.expires_at,
        "revocationListSequence": status.revocation_list_sequence,
        "errorCode": status.error_code,
        "features": status.features,
    }))
}

fn read_token(path: &str) -> Result<String, String> {
    fs::read_to_string(Path::new(path))
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("offline_release_gate_artifact_read_failed:{error}"))
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|error| format!("offline_release_gate_json_failed:{error}"))?;
    println!("{output}");
    Ok(())
}

fn required_arg<'a>(arguments: &'a [String], name: &str) -> Result<&'a str, String> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("offline_release_gate_missing_argument:{name}"))
}

struct ScopedKeyringInstallationSecretStore {
    service: String,
}

impl ScopedKeyringInstallationSecretStore {
    fn entry(&self, user: &str) -> Result<Entry, String> {
        Entry::new(&self.service, user)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }
}

impl InstallationSecretStore for ScopedKeyringInstallationSecretStore {
    fn load(&self) -> Result<Option<Vec<u8>>, String> {
        match self.entry(SECRET_USER)?.get_secret() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("offline_license_secure_storage_unavailable".to_string()),
        }
    }

    fn store(&self, secret: &[u8]) -> Result<(), String> {
        self.entry(SECRET_USER)?
            .set_secret(secret)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }

    fn load_security_anchor(&self) -> Result<OfflineSecurityAnchor, String> {
        match self.entry(SECURITY_ANCHOR_USER)?.get_password() {
            Ok(value) => serde_json::from_str(&value)
                .map_err(|_| "offline_license_secure_storage_unavailable".to_string()),
            Err(keyring::Error::NoEntry) => Ok(OfflineSecurityAnchor {
                highest_observed_utc: None,
                revocation_high_water: BTreeMap::new(),
            }),
            Err(_) => Err("offline_license_secure_storage_unavailable".to_string()),
        }
    }

    fn store_security_anchor(&self, anchor: &OfflineSecurityAnchor) -> Result<(), String> {
        let value = serde_json::to_string(anchor)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
        self.entry(SECURITY_ANCHOR_USER)?
            .set_password(&value)
            .map_err(|_| "offline_license_secure_storage_unavailable".to_string())
    }
}
