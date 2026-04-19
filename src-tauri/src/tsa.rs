//! Trusted Timestamp Authority (TSA) client with multi-source fallback.
//!
//! Provides two layers of time attestation:
//! 1. RFC 3161 TSA tokens (.tsr) — cryptographically signed by third-party CAs
//! 2. NTP network time snapshot — lightweight proof that system clock wasn't tampered
//!
//! Both are best-effort and non-blocking. Pipeline never fails due to network issues.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// TSA endpoints (fallback chain)
// ---------------------------------------------------------------------------

/// Multiple TSA endpoints for redundancy. Tried in order until one succeeds.
const TSA_ENDPOINTS: &[&str] = &[
    "https://freetsa.org/tsr",
    "https://timestamp.digicert.com",
    "http://timestamp.sectigo.com",
];

// ---------------------------------------------------------------------------
// NTP-like network time sources (HTTP-based, no UDP needed)
// ---------------------------------------------------------------------------

/// HTTP endpoints that return server time in response headers.
/// We extract the `Date` header as a lightweight network time proof.
const TIME_ENDPOINTS: &[&str] = &[
    "https://www.aliyun.com",
    "https://cloud.tencent.com",
    "https://www.baidu.com",
];

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Combined timestamp attestation result.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampAttestation {
    /// Path to RFC 3161 .tsr token file (if obtained).
    pub tsa_token_path: Option<String>,
    /// Which TSA endpoint succeeded.
    pub tsa_source: Option<String>,
    /// Network time from HTTP Date header (RFC 2822 format).
    pub network_time: Option<String>,
    /// Which time endpoint responded.
    pub network_time_source: Option<String>,
    /// Local system time at the moment of attestation (for comparison).
    pub local_time: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Request timestamp attestation from multiple sources.
/// Tries all TSA endpoints and all NTP-like endpoints concurrently.
/// Returns whatever succeeds — never blocks the pipeline.
pub async fn request_attestation(
    file_hash_hex: &str,
    watermark_uid: &str,
    tsa_dir: &Path,
) -> TimestampAttestation {
    let local_time = chrono::Utc::now().to_rfc3339();

    // Run TSA and network time requests concurrently
    let tsa_future = request_tsa_with_fallback(file_hash_hex, watermark_uid, tsa_dir);
    let ntp_future = fetch_network_time();

    let (tsa_result, ntp_result) = tokio::join!(tsa_future, ntp_future);

    TimestampAttestation {
        tsa_token_path: tsa_result.0.map(|p| p.to_string_lossy().to_string()),
        tsa_source: tsa_result.1,
        network_time: ntp_result.0,
        network_time_source: ntp_result.1,
        local_time,
    }
}

/// Legacy API — returns just the token path for backward compatibility.
pub async fn request_timestamp(
    file_hash_hex: &str,
    watermark_uid: &str,
    tsa_dir: &Path,
) -> Option<PathBuf> {
    let (path, _source) = request_tsa_with_fallback(file_hash_hex, watermark_uid, tsa_dir).await;
    path
}

// ---------------------------------------------------------------------------
// TSA with fallback
// ---------------------------------------------------------------------------

/// Try multiple TSA endpoints in sequence until one succeeds.
async fn request_tsa_with_fallback(
    file_hash_hex: &str,
    watermark_uid: &str,
    tsa_dir: &Path,
) -> (Option<PathBuf>, Option<String>) {
    let hash_bytes = match hex::decode(file_hash_hex) {
        Ok(b) => b,
        Err(_) => return (None, None),
    };
    let tsq = build_timestamp_request(&hash_bytes);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .unwrap_or_default();

    for &endpoint in TSA_ENDPOINTS {
        let result = client
            .post(endpoint)
            .header("Content-Type", "application/timestamp-query")
            .body(tsq.clone())
            .send()
            .await;

        let response = match result {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let tsr_bytes = match response.bytes().await {
            Ok(b) if b.len() > 100 => b,
            _ => continue,
        };

        // Save .tsr token
        if std::fs::create_dir_all(tsa_dir).is_err() {
            continue;
        }
        let safe_uid = watermark_uid.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let token_path = tsa_dir.join(format!("{safe_uid}.tsr"));
        if std::fs::write(&token_path, &tsr_bytes).is_ok() {
            log::info!("TSA token from {endpoint} saved: {}", token_path.display());
            return (Some(token_path), Some(endpoint.to_string()));
        }
    }

    log::warn!("All TSA endpoints failed");
    (None, None)
}

// ---------------------------------------------------------------------------
// Network time (HTTP Date header)
// ---------------------------------------------------------------------------

/// Fetch network time from HTTP Date headers as a lightweight time proof.
/// Tries multiple endpoints concurrently, returns the first success.
async fn fetch_network_time() -> (Option<String>, Option<String>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    for &endpoint in TIME_ENDPOINTS {
        let result = client
            .head(endpoint)
            .send()
            .await;

        if let Ok(response) = result {
            if let Some(date) = response.headers().get("date") {
                if let Ok(date_str) = date.to_str() {
                    log::info!("Network time from {endpoint}: {date_str}");
                    return (Some(date_str.to_string()), Some(endpoint.to_string()));
                }
            }
        }
    }

    log::warn!("All network time endpoints unreachable");
    (None, None)
}

// ---------------------------------------------------------------------------
// RFC 3161 ASN.1 builder
// ---------------------------------------------------------------------------

/// Build a minimal RFC 3161 TimeStampReq (DER-encoded ASN.1).
fn build_timestamp_request(hash: &[u8]) -> Vec<u8> {
    // SHA-256 OID: 2.16.840.1.101.3.4.2.1
    let sha256_oid: &[u8] = &[
        0x30, 0x0d, // SEQUENCE (AlgorithmIdentifier)
        0x06, 0x09, // OID
        0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, // SHA-256 OID
        0x05, 0x00, // NULL (parameters)
    ];

    let hash_octet = asn1_octet_string(hash);
    let msg_imprint_content = [sha256_oid, &hash_octet].concat();
    let msg_imprint = asn1_sequence(&msg_imprint_content);

    let version = &[0x02, 0x01, 0x01]; // INTEGER 1
    let cert_req = &[0x01, 0x01, 0xff]; // BOOLEAN TRUE

    let req_content = [version.as_slice(), &msg_imprint, cert_req].concat();
    asn1_sequence(&req_content)
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Verify that a .tsr file exists and is non-empty.
#[allow(dead_code)]
pub fn verify_token_exists(token_path: &Path) -> bool {
    token_path.exists() && std::fs::metadata(token_path).map(|m| m.len() > 100).unwrap_or(false)
}

/// Compute SHA-256 of a file and return as hex string.
#[allow(dead_code)]
pub fn hash_file(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}

fn asn1_sequence(content: &[u8]) -> Vec<u8> {
    let mut result = vec![0x30];
    result.extend(asn1_length(content.len()));
    result.extend_from_slice(content);
    result
}

fn asn1_octet_string(content: &[u8]) -> Vec<u8> {
    let mut result = vec![0x04];
    result.extend(asn1_length(content.len()));
    result.extend_from_slice(content);
    result
}

fn asn1_length(len: usize) -> Vec<u8> {
    if len < 128 {
        vec![len as u8]
    } else if len < 256 {
        vec![0x81, len as u8]
    } else {
        vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
    }
}
