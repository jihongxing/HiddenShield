//! Trusted Timestamp Authority (TSA) client with multi-source fallback.
//!
//! Provides two layers of time attestation:
//! 1. RFC 3161 TSA tokens (.tsr) — cryptographically signed by third-party CAs
//! 2. NTP network time snapshot — lightweight proof that system clock wasn't tampered
//!
//! Both are best-effort and non-blocking. Pipeline never fails due to network issues.

use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier};
use const_oid::ObjectIdentifier;
use der::asn1::OctetString;
use der::referenced::OwnedToRef;
use der::{Decode, Encode};
use rustls_native_certs::load_native_certs;
use rustls_pki_types::{CertificateDer, UnixTime};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use webpki::{anchor_from_trusted_cert, EndEntityCert, KeyUsage, ALL_VERIFICATION_ALGS};
use x509_cert::attr::Attribute;
use x509_cert::ext::pkix::{
    ExtendedKeyUsage, KeyUsage as CertificateKeyUsage, SubjectKeyIdentifier,
};
use x509_cert::Certificate;
use x509_verify::{Message, Signature, VerifyInfo, VerifyingKey};

#[cfg(test)]
const CMS_SIGNED_DATA_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
#[cfg(test)]
const TST_INFO_CONTENT_TYPE_OID: &[u8] = &[
    0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x01, 0x04,
];
const SHA256_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const SHA1_OID: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
const SHA224_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x04];
const SHA384_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
const SHA512_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03];
const CONTENT_TYPE_ATTR_OID: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
const MESSAGE_DIGEST_ATTR_OID: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const CMS_SIGNED_DATA_OID_OBJ: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const TST_INFO_CONTENT_TYPE_OID_OBJ: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.1.4");
const TSA_EKU_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.8");
const TSA_EKU_OID_DER_VALUE: &[u8] = &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x08];

// ---------------------------------------------------------------------------
// TSA endpoints (fallback chain)
// ---------------------------------------------------------------------------

/// Multiple TSA endpoints for redundancy. Tried in order until one succeeds.
const TSA_ENDPOINTS: &[&str] = &["https://freetsa.org/tsr", "https://timestamp.digicert.com"];

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
    /// Original nonce used in the TimeStampReq, hex-encoded for later revalidation.
    pub tsa_request_nonce: Option<String>,
    /// Network time from HTTP Date header (RFC 2822 format).
    pub network_time: Option<String>,
    /// Which time endpoint responded.
    pub network_time_source: Option<String>,
    /// Local system time at the moment of attestation (for comparison).
    pub local_time: String,
}

#[derive(Debug, Clone)]
pub struct VerifiedTimestampToken {
    pub gen_time: String,
}

#[derive(Debug, Clone)]
struct ParsedTstInfo {
    der: Vec<u8>,
    gen_time: String,
    gen_time_unix_secs: u64,
}

impl TimestampAttestation {
    pub fn offline() -> Self {
        Self {
            tsa_token_path: None,
            tsa_source: None,
            tsa_request_nonce: None,
            network_time: None,
            network_time_source: None,
            local_time: chrono::Utc::now().to_rfc3339(),
        }
    }
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
        tsa_request_nonce: tsa_result.2,
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
    let (path, _source, _nonce) =
        request_tsa_with_fallback(file_hash_hex, watermark_uid, tsa_dir).await;
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
) -> (Option<PathBuf>, Option<String>, Option<String>) {
    let hash_bytes = match hex::decode(file_hash_hex) {
        Ok(b) => b,
        Err(_) => return (None, None, None),
    };
    let nonce = generate_request_nonce(&hash_bytes);
    let tsq = build_timestamp_request(&hash_bytes, nonce);
    let nonce_hex = format!("{nonce:016x}");

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

        if !is_timestamp_reply_content_type(
            response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
        ) {
            log::warn!("Ignoring TSA response from {endpoint}: unexpected content-type");
            continue;
        }

        let tsr_bytes = match response.bytes().await {
            Ok(b) if b.len() > 100 => b,
            _ => continue,
        };

        if let Err(reason) = verify_timestamp_response(&tsr_bytes, &hash_bytes, Some(nonce)) {
            log::warn!("Ignoring TSA response from {endpoint}: {reason}");
            continue;
        }

        // Save .tsr token
        if std::fs::create_dir_all(tsa_dir).is_err() {
            continue;
        }
        let token_path = tsa_dir.join(build_tsa_token_file_name(
            watermark_uid,
            file_hash_hex,
            &nonce_hex,
        ));
        if std::fs::write(&token_path, &tsr_bytes).is_ok() {
            log::info!("TSA token from {endpoint} saved: {}", token_path.display());
            return (
                Some(token_path),
                Some(endpoint.to_string()),
                Some(nonce_hex),
            );
        }
    }

    log::warn!("All TSA endpoints failed");
    (None, None, None)
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
        let result = client.head(endpoint).send().await;

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
fn build_timestamp_request(hash: &[u8], nonce: u64) -> Vec<u8> {
    let hash_algorithm = build_sha256_algorithm_identifier();
    let hash_octet = asn1_octet_string(hash);
    let msg_imprint_content = [hash_algorithm.as_slice(), hash_octet.as_slice()].concat();
    let msg_imprint = asn1_sequence(&msg_imprint_content);

    let version = asn1_integer_u64(1);
    let nonce = asn1_integer_u64(nonce);
    let cert_req = asn1_boolean(true);

    let req_content = [
        version.as_slice(),
        msg_imprint.as_slice(),
        nonce.as_slice(),
        cert_req.as_slice(),
    ]
    .concat();
    asn1_sequence(&req_content)
}

fn generate_request_nonce(hash: &[u8]) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut nonce = (now as u64) ^ ((now >> 64) as u64) ^ ((std::process::id() as u64) << 32);

    for (index, byte) in hash.iter().take(8).enumerate() {
        nonce ^= (*byte as u64) << (index * 8);
    }

    if nonce == 0 {
        1
    } else {
        nonce
    }
}

fn build_tsa_token_file_name(watermark_uid: &str, file_hash_hex: &str, nonce_hex: &str) -> String {
    let safe_uid = sanitize_file_name_segment(watermark_uid);
    let safe_hash = sanitize_hex_segment(file_hash_hex, 64);
    let safe_nonce = sanitize_hex_segment(nonce_hex, 16);
    format!("{safe_uid}-{safe_hash}-{safe_nonce}.tsr")
}

fn sanitize_file_name_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other if other.is_control() => '_',
            other => other,
        })
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string();

    if sanitized.is_empty() {
        "tsa".to_string()
    } else {
        sanitized
    }
}

fn sanitize_hex_segment(value: &str, max_len: usize) -> String {
    let sanitized: String = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .take(max_len)
        .map(|ch| ch.to_ascii_lowercase())
        .collect();

    if sanitized.is_empty() {
        "0".to_string()
    } else {
        sanitized
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Verify that a .tsr file exists and is non-empty.
#[allow(dead_code)]
pub fn verify_token_exists(token_path: &Path) -> bool {
    token_path.exists()
        && std::fs::metadata(token_path)
            .map(|m| m.len() > 100)
            .unwrap_or(false)
}

/// Compute SHA-256 of a file and return as hex string.
#[allow(dead_code)]
pub fn hash_file(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(format!("{:x}", hasher.finalize()))
}

pub fn verify_saved_token(
    token_path: &Path,
    expected_hash_hex: &str,
    expected_nonce_hex: Option<&str>,
) -> Result<VerifiedTimestampToken, String> {
    let expected_hash = hex::decode(expected_hash_hex)
        .map_err(|e| format!("invalid stored file hash for TSA verification: {e}"))?;
    let expected_nonce = expected_nonce_hex
        .ok_or_else(|| {
            "missing original TSA request nonce; cannot fully revalidate token".to_string()
        })
        .and_then(parse_nonce_hex)?;
    let bytes = std::fs::read(token_path)
        .map_err(|e| format!("failed to read TSA token {}: {e}", token_path.display()))?;

    verify_timestamp_response(&bytes, &expected_hash, Some(expected_nonce))
}

fn asn1_sequence(content: &[u8]) -> Vec<u8> {
    let mut result = vec![0x30];
    result.extend(asn1_length(content.len()));
    result.extend_from_slice(content);
    result
}

#[cfg(test)]
fn asn1_set(content: &[u8]) -> Vec<u8> {
    let mut result = vec![0x31];
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

fn asn1_oid(content: &[u8]) -> Vec<u8> {
    let mut result = vec![0x06];
    result.extend(asn1_length(content.len()));
    result.extend_from_slice(content);
    result
}

fn asn1_integer_u64(value: u64) -> Vec<u8> {
    let mut bytes = value.to_be_bytes().to_vec();
    while bytes.len() > 1 && bytes[0] == 0 {
        bytes.remove(0);
    }
    if bytes.is_empty() {
        bytes.push(0);
    }
    if bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0);
    }

    let mut result = vec![0x02];
    result.extend(asn1_length(bytes.len()));
    result.extend(bytes);
    result
}

fn asn1_boolean(value: bool) -> Vec<u8> {
    vec![0x01, 0x01, if value { 0xff } else { 0x00 }]
}

#[cfg(test)]
fn asn1_generalized_time(value: &str) -> Vec<u8> {
    let mut result = vec![0x18];
    result.extend(asn1_length(value.len()));
    result.extend_from_slice(value.as_bytes());
    result
}

#[cfg(test)]
fn asn1_explicit(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
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

fn build_sha256_algorithm_identifier() -> Vec<u8> {
    let mut algorithm = asn1_oid(SHA256_OID);
    algorithm.extend_from_slice(&[0x05, 0x00]);
    asn1_sequence(&algorithm)
}

fn is_timestamp_reply_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return true;
    };

    let lowered = content_type.to_ascii_lowercase();
    lowered.contains("application/timestamp-reply") || lowered.contains("application/octet-stream")
}

fn verify_timestamp_response(
    bytes: &[u8],
    expected_hash: &[u8],
    expected_nonce: Option<u64>,
) -> Result<VerifiedTimestampToken, String> {
    verify_timestamp_response_with_roots(bytes, expected_hash, expected_nonce, None)
}

fn verify_timestamp_response_with_roots(
    bytes: &[u8],
    expected_hash: &[u8],
    expected_nonce: Option<u64>,
    trust_roots: Option<&[CertificateDer<'static>]>,
) -> Result<VerifiedTimestampToken, String> {
    let token_der = extract_timestamp_token_der(bytes)?;
    let content_info =
        ContentInfo::from_der(token_der).map_err(|e| format!("invalid CMS ContentInfo: {e}"))?;

    if content_info.content_type != CMS_SIGNED_DATA_OID_OBJ {
        return Err("timeStampToken does not carry CMS signedData".to_string());
    }

    let signed_data: SignedData = content_info
        .content
        .decode_as()
        .map_err(|e| format!("invalid CMS signedData payload: {e}"))?;

    let tst_info_der = extract_tst_info_from_signed_data(&signed_data)?;
    let parsed_tst = validate_tst_info(&tst_info_der, expected_hash, expected_nonce)?;
    verify_time_stamp_token_cms(&signed_data, &parsed_tst, trust_roots)?;

    Ok(VerifiedTimestampToken {
        gen_time: parsed_tst.gen_time,
    })
}

fn extract_timestamp_token_der(bytes: &[u8]) -> Result<&[u8], String> {
    let (response_seq, rest) = parse_expected_tlv(bytes, 0x30).ok_or("invalid DER envelope")?;
    if !rest.is_empty() {
        return Err("unexpected trailing bytes after TimeStampResp".to_string());
    }

    let (status_info, remaining) =
        parse_expected_tlv(response_seq, 0x30).ok_or("missing PKIStatusInfo")?;
    let (status, _) = parse_der_u64_integer(status_info).ok_or("missing PKIStatus integer")?;

    if status != 0 && status != 1 {
        return Err("TSA did not grant the timestamp request".to_string());
    }

    let (content_info, trailing) =
        parse_expected_tlv_full(remaining, 0x30).ok_or("missing timeStampToken")?;
    if !trailing.is_empty() {
        return Err("unexpected trailing fields after timeStampToken".to_string());
    }

    Ok(content_info)
}

fn extract_tst_info_from_signed_data(signed_data: &SignedData) -> Result<Vec<u8>, String> {
    if signed_data.encap_content_info.econtent_type != TST_INFO_CONTENT_TYPE_OID_OBJ {
        return Err("CMS payload is not TSTInfo".to_string());
    }

    let econtent = signed_data
        .encap_content_info
        .econtent
        .as_ref()
        .ok_or_else(|| "missing TSTInfo eContent".to_string())?;
    let tst_info: OctetString = econtent
        .decode_as()
        .map_err(|e| format!("invalid TSTInfo eContent: {e}"))?;
    Ok(tst_info.as_bytes().to_vec())
}

fn validate_tst_info(
    tst_info: &[u8],
    expected_hash: &[u8],
    expected_nonce: Option<u64>,
) -> Result<ParsedTstInfo, String> {
    let full_tst_info_der = tst_info.to_vec();
    let (tst_info, rest) = parse_expected_tlv(tst_info, 0x30).ok_or("invalid TSTInfo envelope")?;
    if !rest.is_empty() {
        return Err("unexpected trailing bytes after TSTInfo".to_string());
    }

    let (version, remaining) = parse_der_u64_integer(tst_info).ok_or("missing TSTInfo version")?;
    if version != 1 {
        return Err("unsupported TSTInfo version".to_string());
    }

    let (_policy, remaining) =
        parse_expected_tlv(remaining, 0x06).ok_or("missing TSTInfo policy")?;
    let (message_imprint, remaining) =
        parse_expected_tlv(remaining, 0x30).ok_or("missing TSTInfo messageImprint")?;
    validate_message_imprint(message_imprint, expected_hash)?;

    let (_serial, remaining) =
        parse_expected_tlv(remaining, 0x02).ok_or("missing TSTInfo serialNumber")?;
    let (gen_time, mut remaining) =
        parse_expected_tlv(remaining, 0x18).ok_or("missing TSTInfo genTime")?;
    if gen_time.is_empty() {
        return Err("invalid TSTInfo genTime".to_string());
    }
    let gen_time = std::str::from_utf8(gen_time)
        .map_err(|_| "invalid UTF-8 in TSTInfo genTime".to_string())?;
    let gen_time_unix_secs = parse_generalized_time_to_unix_secs(gen_time)?;

    let mut nonce = None;
    while !remaining.is_empty() {
        let (tag, value, rest) =
            parse_der_tlv(remaining).ok_or("invalid optional TSTInfo field")?;
        match tag {
            0x02 => {
                if nonce.is_some() {
                    return Err("duplicate TSTInfo nonce".to_string());
                }
                nonce = Some(parse_der_u64_integer_value(value).ok_or("invalid TSTInfo nonce")?);
            }
            0x01 | 0x30 | 0xa0 | 0xa1 => {}
            _ => return Err("unsupported optional TSTInfo field".to_string()),
        }
        remaining = rest;
    }

    if let Some(expected_nonce) = expected_nonce {
        match nonce {
            Some(actual_nonce) if actual_nonce == expected_nonce => {}
            Some(_) => return Err("timestamp response nonce does not match request".to_string()),
            None => return Err("timestamp response is missing request nonce".to_string()),
        }
    }

    Ok(ParsedTstInfo {
        der: full_tst_info_der,
        gen_time: gen_time.to_string(),
        gen_time_unix_secs,
    })
}

fn validate_message_imprint(message_imprint: &[u8], expected_hash: &[u8]) -> Result<(), String> {
    let (hash_algorithm, remaining) =
        parse_expected_tlv(message_imprint, 0x30).ok_or("missing messageImprint hashAlgorithm")?;
    let (hashed_message, trailing) =
        parse_expected_tlv(remaining, 0x04).ok_or("missing messageImprint hashedMessage")?;
    if !trailing.is_empty() {
        return Err("unexpected trailing fields in messageImprint".to_string());
    }

    let (oid, parameters) =
        parse_expected_tlv(hash_algorithm, 0x06).ok_or("missing hashAlgorithm OID")?;
    if oid != SHA256_OID {
        return Err("unexpected messageImprint hash algorithm".to_string());
    }

    if !parameters.is_empty() {
        let (tag, _, rest) = parse_der_tlv(parameters).ok_or("invalid hashAlgorithm parameters")?;
        if tag != 0x05 || !rest.is_empty() {
            return Err("unexpected hashAlgorithm parameters".to_string());
        }
    }

    if hashed_message != expected_hash {
        return Err("timestamp response hash does not match request".to_string());
    }

    Ok(())
}

fn verify_time_stamp_token_cms(
    signed_data: &SignedData,
    parsed_tst: &ParsedTstInfo,
    trust_roots: Option<&[CertificateDer<'static>]>,
) -> Result<(), String> {
    let signer_infos: Vec<_> = signed_data.signer_infos.0.iter().collect();
    if signer_infos.is_empty() {
        return Err("CMS signedData is missing SignerInfo".to_string());
    }

    let certificates = signed_data
        .certificates
        .as_ref()
        .ok_or_else(|| "CMS signedData is missing certificates".to_string())?;
    let certificates: Vec<Certificate> = certificates
        .0
        .iter()
        .filter_map(|choice| match choice {
            CertificateChoices::Certificate(cert) => Some(cert.clone()),
            CertificateChoices::Other(_) => None,
        })
        .collect();
    if certificates.is_empty() {
        return Err("CMS signedData does not include X.509 certificates".to_string());
    }

    let mut last_error = None;
    for signer in signer_infos {
        match verify_single_signer(signed_data, signer, &certificates, parsed_tst, trust_roots) {
            Ok(()) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| "no CMS signer could be validated".to_string()))
}

fn verify_single_signer(
    signed_data: &SignedData,
    signer: &cms::signed_data::SignerInfo,
    certificates: &[Certificate],
    parsed_tst: &ParsedTstInfo,
    trust_roots: Option<&[CertificateDer<'static>]>,
) -> Result<(), String> {
    if !signed_data
        .digest_algorithms
        .iter()
        .any(|alg| alg.oid == signer.digest_alg.oid)
    {
        return Err("SignerInfo digest algorithm is not declared in SignedData".to_string());
    }

    let signer_certificate = select_signer_certificate(signer, certificates)?;
    ensure_signer_certificate_constraints(signer_certificate)?;

    let expected_message_digest = compute_digest_bytes(&signer.digest_alg.oid, &parsed_tst.der)?;
    let signed_attrs_der = validate_signed_attributes(signer, &expected_message_digest)?;
    verify_cms_signature(signer, signer_certificate, &signed_attrs_der)?;
    verify_certificate_chain(
        signer_certificate,
        certificates,
        parsed_tst.gen_time_unix_secs,
        trust_roots,
    )?;
    Ok(())
}

fn validate_signed_attributes(
    signer: &cms::signed_data::SignerInfo,
    expected_message_digest: &[u8],
) -> Result<Vec<u8>, String> {
    let signed_attrs = signer
        .signed_attrs
        .as_ref()
        .ok_or_else(|| "CMS SignerInfo is missing signedAttrs".to_string())?;
    let signed_attrs_der = signed_attrs
        .to_der()
        .map_err(|e| format!("failed to encode signedAttrs: {e}"))?;

    let mut saw_content_type = false;
    let mut saw_message_digest = false;

    for attr in signed_attrs.iter() {
        if attr.oid == CONTENT_TYPE_ATTR_OID {
            let value = only_attribute_value(attr, "contentType")?;
            let content_type: ObjectIdentifier = value
                .decode_as()
                .map_err(|e| format!("invalid signedAttrs contentType: {e}"))?;
            if content_type != TST_INFO_CONTENT_TYPE_OID_OBJ {
                return Err("signedAttrs contentType is not id-ct-TSTInfo".to_string());
            }
            saw_content_type = true;
        } else if attr.oid == MESSAGE_DIGEST_ATTR_OID {
            let value = only_attribute_value(attr, "messageDigest")?;
            let message_digest: OctetString = value
                .decode_as()
                .map_err(|e| format!("invalid signedAttrs messageDigest: {e}"))?;
            if message_digest.as_bytes() != expected_message_digest {
                return Err("signedAttrs messageDigest does not match TSTInfo".to_string());
            }
            saw_message_digest = true;
        }
    }

    if !saw_content_type {
        return Err("signedAttrs is missing contentType".to_string());
    }
    if !saw_message_digest {
        return Err("signedAttrs is missing messageDigest".to_string());
    }

    Ok(signed_attrs_der)
}

fn only_attribute_value<'a>(attr: &'a Attribute, label: &str) -> Result<&'a der::Any, String> {
    let mut values = attr.values.iter();
    let first = values
        .next()
        .ok_or_else(|| format!("signedAttrs {label} is empty"))?;
    if values.next().is_some() {
        return Err(format!("signedAttrs {label} contains multiple values"));
    }
    Ok(first)
}

fn verify_cms_signature(
    signer: &cms::signed_data::SignerInfo,
    signer_certificate: &Certificate,
    signed_attrs_der: &[u8],
) -> Result<(), String> {
    let key = VerifyingKey::new(
        signer_certificate
            .tbs_certificate
            .subject_public_key_info
            .owned_to_ref(),
    )
    .map_err(|e| format!("failed to load signer public key: {e}"))?;
    let signature = Signature::new(&signer.signature_algorithm, signer.signature.as_bytes());
    let verify_info = VerifyInfo::new(Message::new(signed_attrs_der), signature);
    key.verify(verify_info)
        .map_err(|e| format!("CMS signature verification failed: {e}"))
}

fn select_signer_certificate<'a>(
    signer: &cms::signed_data::SignerInfo,
    certificates: &'a [Certificate],
) -> Result<&'a Certificate, String> {
    certificates
        .iter()
        .find(|cert| matches_signer_identifier(&signer.sid, cert).unwrap_or(false))
        .ok_or_else(|| "could not find signer certificate in CMS certificate set".to_string())
}

fn matches_signer_identifier(
    signer_id: &SignerIdentifier,
    cert: &Certificate,
) -> Result<bool, String> {
    match signer_id {
        SignerIdentifier::IssuerAndSerialNumber(issuer_and_serial) => {
            Ok(cert.tbs_certificate.issuer == issuer_and_serial.issuer
                && cert.tbs_certificate.serial_number == issuer_and_serial.serial_number)
        }
        SignerIdentifier::SubjectKeyIdentifier(expected_ski) => {
            let actual_ski = cert
                .tbs_certificate
                .get::<SubjectKeyIdentifier>()
                .map_err(|e| format!("failed to decode SubjectKeyIdentifier: {e}"))?
                .map(|(_, ski)| ski);
            Ok(actual_ski
                .as_ref()
                .map(|ski| ski.0.as_bytes() == expected_ski.0.as_bytes())
                .unwrap_or(false))
        }
    }
}

fn ensure_signer_certificate_constraints(cert: &Certificate) -> Result<(), String> {
    let Some((eku_critical, eku)) = cert
        .tbs_certificate
        .get::<ExtendedKeyUsage>()
        .map_err(|e| format!("failed to decode signer EKU: {e}"))?
    else {
        return Err("signer certificate is missing ExtendedKeyUsage".to_string());
    };

    if !eku_critical {
        return Err("signer certificate timeStamping EKU must be critical".to_string());
    }
    if eku.0.len() != 1 || eku.0[0] != TSA_EKU_OID {
        return Err("signer certificate EKU must contain only id-kp-timeStamping".to_string());
    }

    if let Some((_, key_usage)) = cert
        .tbs_certificate
        .get::<CertificateKeyUsage>()
        .map_err(|e| format!("failed to decode signer KeyUsage: {e}"))?
    {
        if !(key_usage.digital_signature() || key_usage.non_repudiation()) {
            return Err(
                "signer certificate KeyUsage must allow digital signature or non repudiation"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn verify_certificate_chain(
    signer_certificate: &Certificate,
    certificates: &[Certificate],
    verification_time_secs: u64,
    trust_roots: Option<&[CertificateDer<'static>]>,
) -> Result<(), String> {
    let owned_native_roots;
    let trust_roots = if let Some(trust_roots) = trust_roots {
        trust_roots
    } else {
        let native = load_native_certs();
        if !native.errors.is_empty() {
            log::warn!(
                "Some native trust anchors could not be loaded for TSA verification: {}",
                native.errors.len()
            );
        }
        owned_native_roots = native.certs;
        &owned_native_roots
    };

    let trust_anchors: Vec<_> = trust_roots
        .iter()
        .filter_map(|cert| anchor_from_trusted_cert(cert).ok())
        .collect();
    if trust_anchors.is_empty() {
        return Err("no trusted root certificates available for TSA verification".to_string());
    }

    let signer_der = CertificateDer::from(
        signer_certificate
            .to_der()
            .map_err(|e| format!("failed to encode signer certificate: {e}"))?,
    );
    let end_entity = EndEntityCert::try_from(&signer_der)
        .map_err(|e| format!("failed to parse signer certificate: {e}"))?;

    let intermediates: Result<Vec<CertificateDer<'static>>, String> = certificates
        .iter()
        .filter(|cert| *cert != signer_certificate)
        .map(|cert| {
            cert.to_der()
                .map(CertificateDer::from)
                .map_err(|e| format!("failed to encode intermediate certificate: {e}"))
        })
        .collect();
    let intermediates = intermediates?;

    let verification_time = UnixTime::since_unix_epoch(Duration::from_secs(verification_time_secs));
    end_entity
        .verify_for_usage(
            ALL_VERIFICATION_ALGS,
            &trust_anchors,
            &intermediates,
            verification_time,
            KeyUsage::required(TSA_EKU_OID_DER_VALUE),
            None,
            None,
        )
        .map_err(|e| format!("signer certificate chain validation failed: {e:?}"))?;

    Ok(())
}

fn compute_digest_bytes(oid: &ObjectIdentifier, content: &[u8]) -> Result<Vec<u8>, String> {
    if oid.as_bytes() == SHA1_OID {
        Ok(Sha1::digest(content).to_vec())
    } else if oid.as_bytes() == SHA224_OID {
        Ok(sha2::Sha224::digest(content).to_vec())
    } else if oid.as_bytes() == SHA256_OID {
        Ok(Sha256::digest(content).to_vec())
    } else if oid.as_bytes() == SHA384_OID {
        Ok(sha2::Sha384::digest(content).to_vec())
    } else if oid.as_bytes() == SHA512_OID {
        Ok(sha2::Sha512::digest(content).to_vec())
    } else {
        Err(format!("unsupported CMS digest algorithm: {oid}"))
    }
}

fn parse_nonce_hex(nonce_hex: &str) -> Result<u64, String> {
    let trimmed = nonce_hex.trim();
    u64::from_str_radix(trimmed, 16).map_err(|e| format!("invalid TSA nonce '{trimmed}': {e}"))
}

fn parse_generalized_time_to_unix_secs(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    let value = trimmed
        .strip_suffix('Z')
        .ok_or_else(|| "TSTInfo genTime must be UTC (suffix Z)".to_string())?;

    let (base, fractional) = match value.split_once('.') {
        Some(parts) => parts,
        None => match value.split_once(',') {
            Some(parts) => parts,
            None => (value, ""),
        },
    };

    if base.len() != 14 || !base.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("unsupported TSTInfo genTime format: {trimmed}"));
    }
    if !fractional.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("unsupported fractional TSTInfo genTime: {trimmed}"));
    }

    let naive = chrono::NaiveDateTime::parse_from_str(base, "%Y%m%d%H%M%S")
        .map_err(|e| format!("invalid TSTInfo genTime: {e}"))?;
    let timestamp = naive.and_utc().timestamp();
    if timestamp < 0 {
        return Err("TSTInfo genTime is before UNIX epoch".to_string());
    }

    Ok(timestamp as u64)
}

fn parse_expected_tlv(input: &[u8], expected_tag: u8) -> Option<(&[u8], &[u8])> {
    let (tag, value, rest) = parse_der_tlv(input)?;
    if tag == expected_tag {
        Some((value, rest))
    } else {
        None
    }
}

fn parse_expected_tlv_full(input: &[u8], expected_tag: u8) -> Option<(&[u8], &[u8])> {
    let (element, rest) = split_der_element(input)?;
    if element.first().copied() == Some(expected_tag) {
        Some((element, rest))
    } else {
        None
    }
}

fn split_der_element(input: &[u8]) -> Option<(&[u8], &[u8])> {
    if input.len() < 2 {
        return None;
    }

    let (len, len_len) = parse_der_length(&input[1..])?;
    let header_len = 1 + len_len;
    let value_end = header_len.checked_add(len)?;
    if input.len() < value_end {
        return None;
    }

    Some((&input[..value_end], &input[value_end..]))
}

fn parse_der_tlv(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    if input.len() < 2 {
        return None;
    }

    let tag = input[0];
    let (len, len_len) = parse_der_length(&input[1..])?;
    let header_len = 1 + len_len;
    let value_end = header_len.checked_add(len)?;
    if input.len() < value_end {
        return None;
    }

    Some((tag, &input[header_len..value_end], &input[value_end..]))
}

fn parse_der_length(input: &[u8]) -> Option<(usize, usize)> {
    let first = *input.first()?;
    if first & 0x80 == 0 {
        return Some((first as usize, 1));
    }

    let byte_count = (first & 0x7f) as usize;
    if byte_count == 0 || byte_count > 4 || input.len() < byte_count + 1 {
        return None;
    }

    let mut len = 0usize;
    for &byte in &input[1..=byte_count] {
        len = (len << 8) | byte as usize;
    }
    Some((len, byte_count + 1))
}

fn parse_der_u64_integer(input: &[u8]) -> Option<(u64, &[u8])> {
    let (tag, value, rest) = parse_der_tlv(input)?;
    if tag != 0x02 {
        return None;
    }

    Some((parse_der_u64_integer_value(value)?, rest))
}

fn parse_der_u64_integer_value(value: &[u8]) -> Option<u64> {
    if value.is_empty() || value[0] & 0x80 != 0 {
        return None;
    }

    let mut bytes = value;
    while bytes.len() > 1 && bytes[0] == 0x00 {
        bytes = &bytes[1..];
    }
    if bytes.len() > 8 {
        return None;
    }

    let mut result = 0u64;
    for &byte in bytes {
        result = (result << 8) | byte as u64;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cms::content_info::ContentInfo;
    use cms::signed_data::SignedData;
    use rustls_pki_types::CertificateDer;

    #[test]
    fn timestamp_reply_accepts_granted_response() {
        let hash = [0x11; 32];
        let nonce = 42;
        let response = build_test_timestamp_response(&hash, Some(nonce));

        assert!(validate_test_binding(&response, &hash, nonce).is_ok());
    }

    #[test]
    fn timestamp_reply_rejects_failed_status() {
        let hash = [0x22; 32];
        let nonce = 7;
        let response = asn1_sequence(
            &[
                asn1_sequence(&asn1_integer_u64(2)),
                build_test_token(&hash, Some(nonce)),
            ]
            .concat(),
        );

        assert!(extract_timestamp_token_der(&response).is_err());
    }

    #[test]
    fn timestamp_reply_rejects_missing_token() {
        let response = asn1_sequence(&asn1_sequence(&asn1_integer_u64(0)));

        assert!(extract_timestamp_token_der(&response).is_err());
    }

    #[test]
    fn timestamp_reply_rejects_hash_mismatch() {
        let expected_hash = [0x44; 32];
        let actual_hash = [0x55; 32];
        let nonce = 128;
        let response = build_test_timestamp_response(&actual_hash, Some(nonce));

        assert!(validate_test_binding(&response, &expected_hash, nonce).is_err());
    }

    #[test]
    fn timestamp_reply_rejects_nonce_mismatch() {
        let hash = [0x66; 32];
        let response = build_test_timestamp_response(&hash, Some(512));

        assert!(validate_test_binding(&response, &hash, 1024).is_err());
    }

    #[test]
    fn timestamp_reply_rejects_missing_nonce() {
        let hash = [0x77; 32];
        let nonce = 2048;
        let response = build_test_timestamp_response(&hash, None);

        assert!(validate_test_binding(&response, &hash, nonce).is_err());
    }

    #[test]
    fn real_tsa_reply_verifies_cms_signature_chain_and_binding() {
        let response = load_real_response_fixture();
        let hash = [0x11; 32];
        let nonce = 0xbb7bdce02bd39bae_u64;
        let roots = extract_self_signed_roots(&response);

        let verified =
            verify_timestamp_response_with_roots(&response, &hash, Some(nonce), Some(&roots))
                .expect("fixture should verify");

        assert_eq!(verified.gen_time, "20260423034008Z");
    }

    #[test]
    fn timestamp_reply_content_type_accepts_expected_values() {
        assert!(is_timestamp_reply_content_type(Some(
            "application/timestamp-reply"
        )));
        assert!(is_timestamp_reply_content_type(Some(
            "application/octet-stream"
        )));
        assert!(!is_timestamp_reply_content_type(Some("text/html")));
    }

    #[test]
    fn tsa_token_file_name_binds_uid_hash_and_nonce() {
        let file_name = build_tsa_token_file_name(
            "HS-0123-4567-DEAD",
            "B69956820610C86F72E051AE0C32A54E9AF8BFCA69361BA3093A38D24DBDAEAA",
            "BB7BDCE02BD39BAE",
        );

        assert_eq!(
            file_name,
            "HS-0123-4567-DEAD-b69956820610c86f72e051ae0c32a54e9af8bfca69361ba3093a38d24dbdaeaa-bb7bdce02bd39bae.tsr"
        );
    }

    #[test]
    fn tsa_token_file_name_sanitizes_unsafe_segments() {
        let file_name = build_tsa_token_file_name("bad:/uid*", "12zz34", "xx56");

        assert_eq!(file_name, "bad__uid_-1234-56.tsr");
    }

    fn build_test_timestamp_response(hash: &[u8], nonce: Option<u64>) -> Vec<u8> {
        asn1_sequence(
            &[
                asn1_sequence(&asn1_integer_u64(0)),
                build_test_token(hash, nonce),
            ]
            .concat(),
        )
    }

    fn build_test_token(hash: &[u8], nonce: Option<u64>) -> Vec<u8> {
        let hash_algorithm = build_sha256_algorithm_identifier();
        let message_imprint = asn1_sequence(&[hash_algorithm, asn1_octet_string(hash)].concat());

        let mut tst_info_content = Vec::new();
        tst_info_content.extend(asn1_integer_u64(1));
        tst_info_content.extend(asn1_oid(SHA256_OID));
        tst_info_content.extend(message_imprint);
        tst_info_content.extend(asn1_integer_u64(1));
        tst_info_content.extend(asn1_generalized_time("20260423000000Z"));
        if let Some(nonce) = nonce {
            tst_info_content.extend(asn1_integer_u64(nonce));
        }
        let tst_info = asn1_sequence(&tst_info_content);

        let encap_content_info = asn1_sequence(
            &[
                asn1_oid(TST_INFO_CONTENT_TYPE_OID),
                asn1_explicit(0xa0, &asn1_octet_string(&tst_info)),
            ]
            .concat(),
        );
        let signed_data = asn1_sequence(
            &[
                asn1_integer_u64(1),
                asn1_set(&[]),
                encap_content_info,
                asn1_set(&[]),
            ]
            .concat(),
        );

        asn1_sequence(
            &[
                asn1_oid(CMS_SIGNED_DATA_OID),
                asn1_explicit(0xa0, &signed_data),
            ]
            .concat(),
        )
    }

    fn validate_test_binding(
        response: &[u8],
        expected_hash: &[u8],
        expected_nonce: u64,
    ) -> Result<(), String> {
        let token_der = extract_timestamp_token_der(response)?;
        let content_info =
            ContentInfo::from_der(token_der).map_err(|e| format!("invalid test CMS: {e}"))?;
        let signed_data: SignedData = content_info
            .content
            .decode_as()
            .map_err(|e| format!("invalid test SignedData: {e}"))?;
        let tst_info = extract_tst_info_from_signed_data(&signed_data)?;
        validate_tst_info(&tst_info, expected_hash, Some(expected_nonce)).map(|_| ())
    }

    fn load_real_response_fixture() -> Vec<u8> {
        let hex = include_str!("../testdata/tsa/freetsa-response-20260423.hex");
        let compact: String = hex.lines().collect();
        hex::decode(compact).expect("fixture hex should decode")
    }

    fn extract_self_signed_roots(response: &[u8]) -> Vec<CertificateDer<'static>> {
        let token_der = extract_timestamp_token_der(response).expect("fixture has token");
        let content_info = ContentInfo::from_der(token_der).expect("fixture content info");
        let signed_data: SignedData = content_info
            .content
            .decode_as()
            .expect("fixture signedData");
        let certificates = signed_data.certificates.expect("fixture certificates");

        certificates
            .0
            .iter()
            .filter_map(|choice| match choice {
                CertificateChoices::Certificate(cert)
                    if cert.tbs_certificate.subject == cert.tbs_certificate.issuer =>
                {
                    Some(CertificateDer::from(
                        cert.to_der().expect("root certificate should encode"),
                    ))
                }
                _ => None,
            })
            .collect()
    }
}
