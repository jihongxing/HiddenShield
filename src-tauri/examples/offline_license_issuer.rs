use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
#[cfg(test)]
use hidden_shield_lib::offline_license::{encode_offline_license_v1, encode_revocation_list_v1};
use hidden_shield_lib::offline_license::{
    encode_offline_license_v1_with_signature, encode_revocation_list_v1_with_signature,
    offline_license_v1_signing_message, parse_activation_request_v1, parse_offline_license_v1,
    parse_revocation_list_v1, revocation_list_v1_signing_message,
    verify_activation_request_v1_checksum, verify_offline_license_v1_signature,
    verify_revocation_list_v1_signature, OfflineLicensePayloadV1, RevocationListPayloadV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

const KEY_FILE_SCHEMA_VERSION: u8 = 1;
const KEY_FILE_KDF: &str = "argon2id-v19-m19456-t2-p1";
const KEY_FILE_CIPHER: &str = "xchacha20poly1305";
const KEY_FILE_AAD_DOMAIN: &str = "HiddenShield-Offline-Issuer-Key-v1";

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), String> {
    let (command, options) = parse_arguments(arguments)?;
    match command.as_str() {
        "keygen" => command_keygen(&options),
        "inspect-request" => command_inspect_request(&options),
        "issue" => command_issue(&options),
        "rekey" => command_rekey(&options),
        "verify-license" => command_verify_license(&options),
        "sign-revocations" => command_sign_revocations(&options),
        "verify-revocations" => command_verify_revocations(&options),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => Err(format!("offline_license_issuer_unknown_command:{command}")),
    }
}

fn command_keygen(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(options, &["output", "key-id", "password-env"])?;
    let output = PathBuf::from(required(options, "output")?);
    let key_id = required(options, "key-id")?;
    if !is_identifier(key_id) {
        return Err("offline_license_issuer_invalid_key_id".to_string());
    }
    let password = password_from_env(required(options, "password-env")?)?;
    let mut seed = random_array::<32>()?;
    let signing_key = SigningKey::from_bytes(&seed);
    let public_key = signing_key.verifying_key().to_bytes();
    let salt = random_array::<16>()?;
    let nonce = random_array::<24>()?;
    let envelope = encrypt_seed(&seed, &password, key_id, &public_key, &salt, &nonce)?;
    seed.zeroize();
    write_json_new(&output, &envelope, true)?;
    print_json(&serde_json::json!({
        "status": "created",
        "keyId": key_id,
        "publicKeyBase64Url": URL_SAFE_NO_PAD.encode(public_key),
        "keyFile": output,
    }))
}

fn command_inspect_request(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(options, &["request"])?;
    let token = read_token(required(options, "request")?)?;
    let parsed = parse_activation_request_v1(&token)?;
    if !verify_activation_request_v1_checksum(&parsed) {
        return Err("offline_license_request_checksum_mismatch".to_string());
    }
    print_json(&serde_json::json!({
        "status": "valid",
        "payload": parsed.payload,
    }))
}

fn command_issue(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(
        options,
        &[
            "key",
            "password-env",
            "hardware-signer-config",
            "isolated-signer-config",
            "request",
            "expires-at",
            "issued-at",
            "operator-id",
            "replaces-license-id",
            "reason",
            "output",
            "audit-output",
        ],
    )?;
    let request_token = read_token(required(options, "request")?)?;
    let request = parse_activation_request_v1(&request_token)?;
    if !verify_activation_request_v1_checksum(&request) {
        return Err("offline_license_request_checksum_mismatch".to_string());
    }
    let signer = load_issuer_signer(options)?;
    let operator_id = required(options, "operator-id")?.to_string();
    ensure_audit_identifier(&operator_id)?;
    let replaces_license_id = options.get("replaces-license-id").cloned();
    let reason = options.get("reason").cloned();
    if replaces_license_id.is_some() != reason.is_some() {
        return Err("offline_license_issuer_output_conflict".to_string());
    }
    let issued_at = options
        .get("issued-at")
        .cloned()
        .unwrap_or_else(current_timestamp);
    let expires_at = required(options, "expires-at")?.to_string();
    ensure_time_order(&issued_at, &expires_at)?;
    let license_id = random_identifier("lic")?;
    let serial_number = random_identifier("serial")?;
    let payload = OfflineLicensePayloadV1 {
        expires_at: expires_at.clone(),
        installation_id: request.payload.installation_id.clone(),
        issued_at: issued_at.clone(),
        key_id: signer.key_id().to_string(),
        license_id: license_id.clone(),
        not_before: issued_at.clone(),
        product_code: "creator_offline".to_string(),
        schema_version: 1,
    };
    let signing_message = offline_license_v1_signing_message(&payload)?;
    let signature = signer.sign("license", &signing_message)?;
    let token = encode_offline_license_v1_with_signature(&payload, &signature)?;
    let payload_sha256 = sha256_hex(&parse_offline_license_v1(&token)?.payload_bytes);
    let output = PathBuf::from(required(options, "output")?);
    let audit_output = PathBuf::from(required(options, "audit-output")?);
    ensure_outputs_available(&[&output, &audit_output])?;
    write_text_new(&output, &token, false)?;
    write_json_new(
        &audit_output,
        &serde_json::json!({
            "schemaVersion": 1,
            "eventType": "offline_license_issued",
            "result": "accepted",
            "licenseId": license_id,
            "serialNumber": serial_number,
            "requestId": request.payload.request_id,
            "operatorId": operator_id,
            "keyId": signer.key_id(),
            "signerType": signer.signer_type(),
            "productCode": "creator_offline",
            "installationId": request.payload.installation_id,
            "issuedAt": issued_at,
            "expiresAt": expires_at,
            "replacesLicenseId": replaces_license_id,
            "reason": reason,
            "payloadSha256": payload_sha256,
            "tokenSha256": sha256_hex(token.as_bytes()),
        }),
        false,
    )?;
    print_json(&serde_json::json!({
        "status": "issued",
        "licenseId": payload.license_id,
        "keyId": payload.key_id,
        "output": output,
        "auditOutput": audit_output,
    }))
}

fn command_verify_license(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(options, &["license", "public-key"])?;
    let token = read_token(required(options, "license")?)?;
    let public_key = decode_public_key(required(options, "public-key")?)?;
    let parsed = parse_offline_license_v1(&token)?;
    if !verify_offline_license_v1_signature(&parsed, &public_key)? {
        return Err("offline_license_signature_invalid".to_string());
    }
    print_json(&serde_json::json!({
        "status": "valid",
        "payload": parsed.payload,
    }))
}

fn command_rekey(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(options, &["key", "password-env", "new-password-env"])?;
    let key_path = PathBuf::from(required(options, "key")?);
    let current_password = password_from_env(required(options, "password-env")?)?;
    let new_password = password_from_env(required(options, "new-password-env")?)?;
    let envelope = read_key_envelope(&key_path)?;
    let signing_key = decrypt_signing_key(&envelope, &current_password)?;
    let public_key = signing_key.verifying_key().to_bytes();
    let mut seed = signing_key.to_bytes();
    let salt = random_array::<16>()?;
    let nonce = random_array::<24>()?;
    let rotated = encrypt_seed(
        &seed,
        &new_password,
        &envelope.key_id,
        &public_key,
        &salt,
        &nonce,
    )?;
    seed.zeroize();
    decrypt_signing_key(&rotated, &new_password)?;
    let backup_path = key_backup_path(&key_path)?;
    fs::copy(&key_path, &backup_path)
        .map_err(|error| format!("offline_license_issuer_key_backup_failed:{error}"))?;
    let replacement_path = key_path.with_extension("json.rekeying");
    write_json_new(&replacement_path, &rotated, true)?;
    fs::remove_file(&key_path)
        .map_err(|error| format!("offline_license_issuer_key_replace_failed:{error}"))?;
    fs::rename(&replacement_path, &key_path)
        .map_err(|error| format!("offline_license_issuer_key_replace_failed:{error}"))?;
    print_json(&serde_json::json!({
        "status": "rekeyed",
        "keyId": envelope.key_id,
        "keyFile": key_path,
        "backupFile": backup_path,
    }))
}

fn command_sign_revocations(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(
        options,
        &[
            "key",
            "password-env",
            "hardware-signer-config",
            "isolated-signer-config",
            "input",
            "operator-id",
            "output",
            "audit-output",
        ],
    )?;
    let signer = load_issuer_signer(options)?;
    let operator_id = required(options, "operator-id")?.to_string();
    ensure_audit_identifier(&operator_id)?;
    let input: RevocationDraft = serde_json::from_slice(
        &fs::read(required(options, "input")?)
            .map_err(|error| format!("offline_license_issuer_read_failed:{error}"))?,
    )
    .map_err(|error| format!("offline_license_revocation_list_invalid:{error}"))?;
    let mut revoked_license_ids = input.revoked_license_ids;
    revoked_license_ids.sort();
    if revoked_license_ids
        .windows(2)
        .any(|pair| pair[0] == pair[1])
    {
        return Err("offline_license_revocation_list_invalid".to_string());
    }
    let payload = RevocationListPayloadV1 {
        generated_at: input.generated_at,
        key_id: signer.key_id().to_string(),
        list_id: input.list_id,
        list_type: "offline_license_revocations".to_string(),
        revoked_license_ids,
        schema_version: 1,
        sequence: input.sequence,
    };
    let signing_message = revocation_list_v1_signing_message(&payload)?;
    let signature = signer.sign("revocation", &signing_message)?;
    let token = encode_revocation_list_v1_with_signature(&payload, &signature)?;
    let payload_sha256 = sha256_hex(&parse_revocation_list_v1(&token)?.payload_bytes);
    let output = PathBuf::from(required(options, "output")?);
    let audit_output = PathBuf::from(required(options, "audit-output")?);
    ensure_outputs_available(&[&output, &audit_output])?;
    write_text_new(&output, &token, false)?;
    write_json_new(
        &audit_output,
        &serde_json::json!({
            "schemaVersion": 1,
            "eventType": "offline_license_revocations_signed",
            "result": "accepted",
            "operatorId": operator_id,
            "listId": payload.list_id,
            "keyId": payload.key_id,
            "signerType": signer.signer_type(),
            "sequence": payload.sequence,
            "revokedLicenseCount": payload.revoked_license_ids.len(),
            "generatedAt": payload.generated_at,
            "payloadSha256": payload_sha256,
            "tokenSha256": sha256_hex(token.as_bytes()),
        }),
        false,
    )?;
    print_json(&serde_json::json!({
        "status": "signed",
        "listId": payload.list_id,
        "sequence": payload.sequence,
        "output": output,
        "auditOutput": audit_output,
    }))
}

fn command_verify_revocations(options: &BTreeMap<String, String>) -> Result<(), String> {
    ensure_allowed(options, &["revocations", "public-key"])?;
    let token = read_token(required(options, "revocations")?)?;
    let public_key = decode_public_key(required(options, "public-key")?)?;
    let parsed = parse_revocation_list_v1(&token)?;
    if !verify_revocation_list_v1_signature(&parsed, &public_key)? {
        return Err("offline_license_revocation_signature_invalid".to_string());
    }
    print_json(&serde_json::json!({
        "status": "valid",
        "payload": parsed.payload,
    }))
}

enum IssuerSigner {
    EncryptedSoftwareFile {
        key_id: String,
        signing_key: SigningKey,
    },
    ExternalIsolated(ExternalIsolatedSigner),
}

impl IssuerSigner {
    fn key_id(&self) -> &str {
        match self {
            Self::EncryptedSoftwareFile { key_id, .. } => key_id,
            Self::ExternalIsolated(signer) => &signer.config.key_id,
        }
    }

    fn signer_type(&self) -> &str {
        match self {
            Self::EncryptedSoftwareFile { .. } => "software_encrypted_file",
            Self::ExternalIsolated(signer) => &signer.config.signer_type,
        }
    }

    fn sign(&self, purpose: &str, message: &[u8]) -> Result<[u8; 64], String> {
        match self {
            Self::EncryptedSoftwareFile { signing_key, .. } => {
                Ok(signing_key.sign(message).to_bytes())
            }
            Self::ExternalIsolated(signer) => signer.sign(purpose, message),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExternalIsolatedSignerConfig {
    schema_version: u8,
    signer_type: String,
    key_id: String,
    public_key_base64_url: String,
    key_handle: String,
    command: String,
    #[serde(default)]
    arguments: Vec<String>,
}

struct ExternalIsolatedSigner {
    config: ExternalIsolatedSignerConfig,
    public_key: [u8; 32],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExternalSignerRequest<'a> {
    schema_version: u8,
    operation: &'static str,
    key_id: &'a str,
    key_handle: &'a str,
    purpose: &'a str,
    message_base64_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExternalSignerResponse {
    schema_version: u8,
    key_id: String,
    signature_base64_url: String,
}

impl ExternalIsolatedSigner {
    fn load(path: &Path) -> Result<Self, String> {
        let config: ExternalIsolatedSignerConfig = serde_json::from_slice(
            &fs::read(path)
                .map_err(|error| format!("offline_license_hardware_signer_config_read:{error}"))?,
        )
        .map_err(|error| format!("offline_license_hardware_signer_config_invalid:{error}"))?;
        if config.schema_version != 1
            || !matches!(
                config.signer_type.as_str(),
                "external_hardware" | "managed_kms"
            )
            || !is_identifier(&config.key_id)
            || config.key_handle.trim().is_empty()
            || config.command.trim().is_empty()
            || !Path::new(&config.command).is_absolute()
        {
            return Err("offline_license_hardware_signer_config_invalid".to_string());
        }
        let public_key: [u8; 32] = URL_SAFE_NO_PAD
            .decode(&config.public_key_base64_url)
            .map_err(|_| "offline_license_hardware_signer_config_invalid".to_string())?
            .try_into()
            .map_err(|_| "offline_license_hardware_signer_config_invalid".to_string())?;
        VerifyingKey::from_bytes(&public_key)
            .map_err(|_| "offline_license_hardware_signer_config_invalid".to_string())?;
        Ok(Self { config, public_key })
    }

    fn sign(&self, purpose: &str, message: &[u8]) -> Result<[u8; 64], String> {
        if purpose != "license" && purpose != "revocation" {
            return Err("offline_license_hardware_signer_purpose_invalid".to_string());
        }
        let request = ExternalSignerRequest {
            schema_version: 1,
            operation: "ed25519_sign",
            key_id: &self.config.key_id,
            key_handle: &self.config.key_handle,
            purpose,
            message_base64_url: URL_SAFE_NO_PAD.encode(message),
        };
        let request_bytes = serde_json::to_vec(&request)
            .map_err(|_| "offline_license_hardware_signer_protocol_invalid".to_string())?;
        let mut child = Command::new(&self.config.command)
            .args(&self.config.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("offline_license_hardware_signer_unavailable:{error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "offline_license_hardware_signer_unavailable".to_string())?
            .write_all(&request_bytes)
            .map_err(|error| format!("offline_license_hardware_signer_unavailable:{error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("offline_license_hardware_signer_unavailable:{error}"))?;
        if !output.status.success() {
            return Err("offline_license_hardware_signer_rejected".to_string());
        }
        let response: ExternalSignerResponse = serde_json::from_slice(&output.stdout)
            .map_err(|_| "offline_license_hardware_signer_protocol_invalid".to_string())?;
        if response.schema_version != 1 || response.key_id != self.config.key_id {
            return Err("offline_license_hardware_signer_protocol_invalid".to_string());
        }
        let signature_bytes: [u8; 64] = URL_SAFE_NO_PAD
            .decode(response.signature_base64_url)
            .map_err(|_| "offline_license_hardware_signer_protocol_invalid".to_string())?
            .try_into()
            .map_err(|_| "offline_license_hardware_signer_protocol_invalid".to_string())?;
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|_| "offline_license_hardware_signer_config_invalid".to_string())?;
        let signature = Signature::from_bytes(&signature_bytes);
        verifying_key
            .verify(message, &signature)
            .map_err(|_| "offline_license_hardware_signer_signature_invalid".to_string())?;
        Ok(signature_bytes)
    }
}

fn load_issuer_signer(options: &BTreeMap<String, String>) -> Result<IssuerSigner, String> {
    let hardware_config = options.get("hardware-signer-config");
    let isolated_config = options.get("isolated-signer-config");
    let key_path = options.get("key");
    let password_env = options.get("password-env");
    if hardware_config.is_some() && isolated_config.is_some() {
        return Err("offline_license_issuer_signer_conflict".to_string());
    }
    let external_config = isolated_config.or(hardware_config);
    if external_config.is_some() && (key_path.is_some() || password_env.is_some()) {
        return Err("offline_license_issuer_signer_conflict".to_string());
    }
    if let Some(config_path) = external_config {
        return ExternalIsolatedSigner::load(Path::new(config_path))
            .map(IssuerSigner::ExternalIsolated);
    }
    if key_path.is_none() && password_env.is_none() {
        return Err("offline_license_issuer_signer_required".to_string());
    }
    let key_path = PathBuf::from(
        key_path.ok_or_else(|| "offline_license_issuer_missing_option:key".to_string())?,
    );
    let password = password_from_env(
        password_env
            .ok_or_else(|| "offline_license_issuer_missing_option:password-env".to_string())?,
    )?;
    let envelope = read_key_envelope(&key_path)?;
    let signing_key = decrypt_signing_key(&envelope, &password)?;
    Ok(IssuerSigner::EncryptedSoftwareFile {
        key_id: envelope.key_id,
        signing_key,
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedIssuerKey {
    schema_version: u8,
    key_id: String,
    public_key_base64_url: String,
    kdf: String,
    salt_base64_url: String,
    cipher: String,
    nonce_base64_url: String,
    ciphertext_base64_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevocationDraft {
    list_id: String,
    generated_at: String,
    sequence: u64,
    revoked_license_ids: Vec<String>,
}

fn encrypt_seed(
    seed: &[u8; 32],
    password: &str,
    key_id: &str,
    public_key: &[u8; 32],
    salt: &[u8; 16],
    nonce: &[u8; 24],
) -> Result<EncryptedIssuerKey, String> {
    let mut encryption_key = derive_encryption_key(password, salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&encryption_key)
        .map_err(|_| "offline_license_issuer_encryption_failed".to_string())?;
    let public_key_base64_url = URL_SAFE_NO_PAD.encode(public_key);
    let aad = key_file_aad(key_id, &public_key_base64_url);
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: seed,
                aad: &aad,
            },
        )
        .map_err(|_| "offline_license_issuer_encryption_failed".to_string())?;
    encryption_key.zeroize();
    Ok(EncryptedIssuerKey {
        schema_version: KEY_FILE_SCHEMA_VERSION,
        key_id: key_id.to_string(),
        public_key_base64_url,
        kdf: KEY_FILE_KDF.to_string(),
        salt_base64_url: URL_SAFE_NO_PAD.encode(salt),
        cipher: KEY_FILE_CIPHER.to_string(),
        nonce_base64_url: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext_base64_url: URL_SAFE_NO_PAD.encode(ciphertext),
    })
}

fn decrypt_signing_key(
    envelope: &EncryptedIssuerKey,
    password: &str,
) -> Result<SigningKey, String> {
    if envelope.schema_version != KEY_FILE_SCHEMA_VERSION
        || envelope.kdf != KEY_FILE_KDF
        || envelope.cipher != KEY_FILE_CIPHER
        || !is_identifier(&envelope.key_id)
    {
        return Err("offline_license_issuer_key_file_invalid".to_string());
    }
    let salt: [u8; 16] = decode_fixed(&envelope.salt_base64_url)?;
    let nonce: [u8; 24] = decode_fixed(&envelope.nonce_base64_url)?;
    let ciphertext = URL_SAFE_NO_PAD
        .decode(&envelope.ciphertext_base64_url)
        .map_err(|_| "offline_license_issuer_key_file_invalid".to_string())?;
    let mut encryption_key = derive_encryption_key(password, &salt)?;
    let cipher = XChaCha20Poly1305::new_from_slice(&encryption_key)
        .map_err(|_| "offline_license_issuer_key_file_invalid".to_string())?;
    let aad = key_file_aad(&envelope.key_id, &envelope.public_key_base64_url);
    let mut plaintext = cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| "offline_license_issuer_wrong_password_or_corrupt_key".to_string())?;
    encryption_key.zeroize();
    let mut seed: [u8; 32] = plaintext
        .as_slice()
        .try_into()
        .map_err(|_| "offline_license_issuer_key_file_invalid".to_string())?;
    plaintext.zeroize();
    let signing_key = SigningKey::from_bytes(&seed);
    seed.zeroize();
    let public_key = URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes());
    if public_key != envelope.public_key_base64_url {
        return Err("offline_license_issuer_key_file_invalid".to_string());
    }
    Ok(signing_key)
}

fn derive_encryption_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], String> {
    let mut output = [0u8; 32];
    let params = Params::new(19_456, 2, 1, Some(32))
        .map_err(|_| "offline_license_issuer_kdf_failed".to_string())?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|_| "offline_license_issuer_kdf_failed".to_string())?;
    Ok(output)
}

fn key_file_aad(key_id: &str, public_key: &str) -> Vec<u8> {
    format!("{KEY_FILE_AAD_DOMAIN}\0{key_id}\0{public_key}").into_bytes()
}

fn read_key_envelope(path: &Path) -> Result<EncryptedIssuerKey, String> {
    serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("offline_license_issuer_read_failed:{error}"))?,
    )
    .map_err(|error| format!("offline_license_issuer_key_file_invalid:{error}"))
}

fn password_from_env(name: &str) -> Result<Zeroizing<String>, String> {
    let password =
        env::var(name).map_err(|_| "offline_license_issuer_password_missing".to_string())?;
    if password.chars().count() < 8 {
        return Err("offline_license_issuer_password_too_short".to_string());
    }
    Ok(Zeroizing::new(password))
}

fn key_backup_path(key_path: &Path) -> Result<PathBuf, String> {
    let filename = key_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "offline_license_issuer_key_file_invalid".to_string())?;
    let backup = key_path.with_file_name(format!(
        "{filename}.backup-{}",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    if backup.exists() {
        return Err("offline_license_issuer_output_exists".to_string());
    }
    Ok(backup)
}

fn decode_public_key(value: &str) -> Result<Vec<u8>, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "offline_license_unknown_key".to_string())?;
    if bytes.len() != 32 {
        return Err("offline_license_unknown_key".to_string());
    }
    Ok(bytes)
}

fn decode_fixed<const N: usize>(value: &str) -> Result<[u8; N], String> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "offline_license_issuer_key_file_invalid".to_string())?
        .try_into()
        .map_err(|_| "offline_license_issuer_key_file_invalid".to_string())
}

fn random_array<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| format!("offline_license_issuer_random_failed:{error}"))?;
    Ok(bytes)
}

fn random_identifier(prefix: &str) -> Result<String, String> {
    Ok(format!("{prefix}_{}", hex::encode(random_array::<12>()?)))
}

fn ensure_audit_identifier(value: &str) -> Result<(), String> {
    if !(3..=64).contains(&value.len())
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        || !value
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err("offline_license_issuer_unknown_option:operator-id".to_string());
    }
    Ok(())
}

fn current_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn ensure_time_order(issued_at: &str, expires_at: &str) -> Result<(), String> {
    let issued = DateTime::parse_from_rfc3339(issued_at)
        .map_err(|_| "offline_license_invalid_format".to_string())?;
    let expires = DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| "offline_license_invalid_format".to_string())?;
    if expires <= issued {
        return Err("offline_license_expired".to_string());
    }
    if expires.signed_duration_since(issued).num_seconds() > 365 * 24 * 60 * 60 {
        return Err("offline_license_issuer_validity_too_long".to_string());
    }
    Ok(())
}

fn read_token(value: &str) -> Result<String, String> {
    let path = Path::new(value);
    if path.is_file() {
        return fs::read_to_string(path)
            .map(|token| token.trim().to_string())
            .map_err(|error| format!("offline_license_issuer_read_failed:{error}"));
    }
    Ok(value.to_string())
}

fn write_text_new(path: &Path, value: &str, sensitive: bool) -> Result<(), String> {
    write_bytes_new(path, format!("{value}\n").as_bytes(), sensitive)
}

fn write_json_new<T: Serialize>(path: &Path, value: &T, sensitive: bool) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("offline_license_issuer_json_failed:{error}"))?;
    bytes.push(b'\n');
    write_bytes_new(path, &bytes, sensitive)
}

fn write_bytes_new(path: &Path, bytes: &[u8], sensitive: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("offline_license_issuer_write_failed:{error}"))?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        if sensitive {
            options.mode(0o600);
        }
    }
    #[cfg(not(unix))]
    let _ = sensitive;
    let mut file = options
        .open(path)
        .map_err(|error| format!("offline_license_issuer_write_failed:{error}"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("offline_license_issuer_write_failed:{error}"))
}

fn ensure_outputs_available(paths: &[&Path]) -> Result<(), String> {
    let unique = paths
        .iter()
        .map(|path| path.to_path_buf())
        .collect::<HashSet<_>>();
    if unique.len() != paths.len() {
        return Err("offline_license_issuer_output_conflict".to_string());
    }
    if paths.iter().any(|path| path.exists()) {
        return Err("offline_license_issuer_output_exists".to_string());
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn parse_arguments(arguments: Vec<String>) -> Result<(String, BTreeMap<String, String>), String> {
    let mut iterator = arguments.into_iter();
    let command = iterator
        .next()
        .ok_or_else(|| "offline_license_issuer_command_required".to_string())?;
    let mut options = BTreeMap::new();
    while let Some(option) = iterator.next() {
        if !option.starts_with("--") {
            return Err(format!("offline_license_issuer_invalid_argument:{option}"));
        }
        let key = option.trim_start_matches("--").to_string();
        let value = iterator
            .next()
            .ok_or_else(|| format!("offline_license_issuer_missing_value:{key}"))?;
        if options.insert(key.clone(), value).is_some() {
            return Err(format!("offline_license_issuer_duplicate_option:{key}"));
        }
    }
    Ok((command, options))
}

fn ensure_allowed(options: &BTreeMap<String, String>, allowed: &[&str]) -> Result<(), String> {
    let allowed = allowed.iter().copied().collect::<HashSet<_>>();
    if let Some(option) = options
        .keys()
        .find(|option| !allowed.contains(option.as_str()))
    {
        return Err(format!("offline_license_issuer_unknown_option:{option}"));
    }
    Ok(())
}

fn required<'a>(options: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("offline_license_issuer_missing_option:{key}"))
}

fn is_identifier(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("offline_license_issuer_json_failed:{error}"))?
    );
    Ok(())
}

fn print_help() {
    println!(
        "HiddenShield offline license issuer\n\
         Commands:\n\
         keygen --output FILE --key-id ID --password-env ENV (internal-qa only)\n\
         inspect-request --request FILE_OR_TOKEN\n\
         issue --isolated-signer-config FILE --request FILE_OR_TOKEN --expires-at RFC3339 --operator-id ID --output FILE --audit-output FILE [--issued-at RFC3339] [--replaces-license-id ID --reason TEXT]\n\
         issue --hardware-signer-config FILE ... (legacy compatibility)\n\
         issue --key FILE --password-env ENV ... (internal-qa only)\n\
         rekey --key FILE --password-env ENV --new-password-env ENV\n\
         verify-license --license FILE_OR_TOKEN --public-key BASE64URL\n\
         sign-revocations --isolated-signer-config FILE --input JSON --operator-id ID --output FILE --audit-output FILE\n\
         sign-revocations --hardware-signer-config FILE ... (legacy compatibility)\n\
         sign-revocations --key FILE --password-env ENV ... (internal-qa only)\n\
         verify-revocations --revocations FILE_OR_TOKEN --public-key BASE64URL"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn offline_license_issuer_encrypts_key_and_rejects_wrong_password() {
        let seed = [7u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = signing_key.verifying_key().to_bytes();
        let envelope = encrypt_seed(
            &seed,
            TEST_PASSWORD,
            "offline-test-k1",
            &public_key,
            &[3u8; 16],
            &[5u8; 24],
        )
        .expect("key encryption");
        let decrypted = decrypt_signing_key(&envelope, TEST_PASSWORD).expect("correct password");
        assert_eq!(
            decrypted.verifying_key().to_bytes(),
            signing_key.verifying_key().to_bytes()
        );
        assert_eq!(
            decrypt_signing_key(&envelope, "incorrect password 123")
                .expect_err("wrong password must fail"),
            "offline_license_issuer_wrong_password_or_corrupt_key"
        );
    }

    #[test]
    fn offline_license_issuer_issues_only_frozen_creator_profile() {
        let fixture = include_str!("../../docs/fixtures/offline-license-k0/hsreq1-v1-valid.json");
        let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let token = fixture["token"].as_str().unwrap();
        let request = parse_activation_request_v1(token).expect("request");
        assert!(verify_activation_request_v1_checksum(&request));
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        let payload = OfflineLicensePayloadV1 {
            expires_at: "2027-07-15T00:00:00Z".to_string(),
            installation_id: request.payload.installation_id,
            issued_at: "2026-07-15T00:00:00Z".to_string(),
            key_id: "offline-test-k1".to_string(),
            license_id: "lic_k1_0001".to_string(),
            not_before: "2026-07-15T00:00:00Z".to_string(),
            product_code: "creator_offline".to_string(),
            schema_version: 1,
        };
        let token = encode_offline_license_v1(&payload, &signing_key).expect("issue");
        let parsed = parse_offline_license_v1(&token).expect("parse issued license");
        assert!(verify_offline_license_v1_signature(
            &parsed,
            &signing_key.verifying_key().to_bytes()
        )
        .unwrap());

        let mut forbidden = payload;
        forbidden.product_code = "studio_offline".to_string();
        assert_eq!(
            encode_offline_license_v1(&forbidden, &signing_key)
                .expect_err("unknown template must fail"),
            "offline_license_feature_profile_invalid"
        );
    }

    #[test]
    fn offline_license_issuer_rejects_unknown_cli_options() {
        let options = BTreeMap::from([("template".to_string(), "studio".to_string())]);
        assert_eq!(
            ensure_allowed(&options, &["output"]).expect_err("unknown option"),
            "offline_license_issuer_unknown_option:template"
        );
    }

    #[test]
    fn offline_license_issuer_accepts_exactly_365_days_but_rejects_longer_terms() {
        assert!(ensure_time_order("2026-07-21T00:00:00Z", "2027-07-21T00:00:00Z").is_ok());
        assert_eq!(
            ensure_time_order("2026-07-21T00:00:00Z", "2027-07-21T00:00:01Z")
                .expect_err("one second beyond 365 days must be rejected"),
            "offline_license_issuer_validity_too_long"
        );
    }
}
