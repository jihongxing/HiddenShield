use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const HSLIC1_PREFIX: &str = "HSLIC1";
const HSREQ1_PREFIX: &str = "HSREQ1";
const HSRVL1_PREFIX: &str = "HSRVL1";
pub const HSLIC1_SIGNATURE_DOMAIN: &[u8] = b"HiddenShield-Offline-License-v1\0";
pub const HSREQ1_CHECKSUM_DOMAIN: &[u8] = b"HiddenShield-Offline-Activation-Request-v1\0";
pub const HSRVL1_SIGNATURE_DOMAIN: &[u8] = b"HiddenShield-Offline-Revocation-List-v1\0";
pub const INSTALLATION_ID_DOMAIN: &[u8] = b"HiddenShield-Installation-v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfflineLicensePayloadV1 {
    pub expires_at: String,
    pub installation_id: String,
    pub issued_at: String,
    pub key_id: String,
    pub license_id: String,
    pub not_before: String,
    pub product_code: String,
    pub schema_version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ActivationRequestPayloadV1 {
    pub app_version: String,
    pub created_at: String,
    pub installation_id: String,
    pub nonce: String,
    pub platform: String,
    pub request_id: String,
    pub requested_product_code: String,
    pub schema_version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RevocationListPayloadV1 {
    pub generated_at: String,
    pub key_id: String,
    pub list_id: String,
    pub list_type: String,
    pub revoked_license_ids: Vec<String>,
    pub schema_version: u8,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOfflineLicenseV1 {
    pub payload: OfflineLicensePayloadV1,
    pub payload_bytes: Vec<u8>,
    pub signature_bytes: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedActivationRequestV1 {
    pub payload: ActivationRequestPayloadV1,
    pub payload_bytes: Vec<u8>,
    pub checksum_bytes: [u8; 12],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRevocationListV1 {
    pub payload: RevocationListPayloadV1,
    pub payload_bytes: Vec<u8>,
    pub signature_bytes: [u8; 64],
}

pub fn parse_offline_license_v1(token: &str) -> Result<ParsedOfflineLicenseV1, String> {
    let (payload_bytes, signature_bytes) =
        decode_three_segment_token::<64>(token, HSLIC1_PREFIX, "offline_license_invalid_format")?;
    let payload: OfflineLicensePayloadV1 = parse_canonical_payload(
        &payload_bytes,
        "offline_license_invalid_format",
        "offline_license_non_canonical_payload",
    )?;
    if payload.schema_version != 1 {
        return Err("offline_license_unknown_schema".to_string());
    }
    if payload.product_code != "creator_offline" {
        return Err("offline_license_feature_profile_invalid".to_string());
    }
    if !is_timestamp(&payload.expires_at)
        || !is_installation_id(&payload.installation_id)
        || !is_timestamp(&payload.issued_at)
        || !is_identifier(&payload.key_id)
        || !is_identifier(&payload.license_id)
        || !is_timestamp(&payload.not_before)
    {
        return Err("offline_license_invalid_format".to_string());
    }
    Ok(ParsedOfflineLicenseV1 {
        payload,
        payload_bytes,
        signature_bytes,
    })
}

pub fn parse_activation_request_v1(token: &str) -> Result<ParsedActivationRequestV1, String> {
    let (payload_bytes, checksum_bytes) = decode_three_segment_token::<12>(
        token,
        HSREQ1_PREFIX,
        "offline_license_request_invalid_format",
    )?;
    let payload: ActivationRequestPayloadV1 = parse_canonical_payload(
        &payload_bytes,
        "offline_license_request_invalid_format",
        "offline_license_request_non_canonical_payload",
    )?;
    if payload.schema_version != 1 {
        return Err("offline_license_request_unknown_schema".to_string());
    }
    if payload.requested_product_code != "creator_offline" {
        return Err("offline_license_request_product_invalid".to_string());
    }
    if !is_app_version(&payload.app_version)
        || !is_timestamp(&payload.created_at)
        || !is_installation_id(&payload.installation_id)
        || !is_nonce(&payload.nonce)
        || !is_platform(&payload.platform)
        || !is_identifier(&payload.request_id)
    {
        return Err("offline_license_request_invalid_format".to_string());
    }
    Ok(ParsedActivationRequestV1 {
        payload,
        payload_bytes,
        checksum_bytes,
    })
}

pub fn parse_revocation_list_v1(token: &str) -> Result<ParsedRevocationListV1, String> {
    let (payload_bytes, signature_bytes) = decode_three_segment_token::<64>(
        token,
        HSRVL1_PREFIX,
        "offline_license_revocation_invalid_format",
    )?;
    let payload: RevocationListPayloadV1 = parse_canonical_payload(
        &payload_bytes,
        "offline_license_revocation_invalid_format",
        "offline_license_revocation_non_canonical_payload",
    )?;
    if payload.schema_version != 1 {
        return Err("offline_license_revocation_unknown_schema".to_string());
    }
    if payload.list_type != "offline_license_revocations" {
        return Err("offline_license_revocation_list_invalid".to_string());
    }
    if payload.sequence == 0 {
        return Err("offline_license_revocation_sequence_invalid".to_string());
    }
    if !is_timestamp(&payload.generated_at)
        || !is_identifier(&payload.key_id)
        || !is_identifier(&payload.list_id)
        || !is_sorted_unique_identifiers(&payload.revoked_license_ids)
    {
        return Err("offline_license_revocation_list_invalid".to_string());
    }
    Ok(ParsedRevocationListV1 {
        payload,
        payload_bytes,
        signature_bytes,
    })
}

pub fn verify_offline_license_v1_signature(
    parsed: &ParsedOfflineLicenseV1,
    public_key_bytes: &[u8],
) -> Result<bool, String> {
    verify_ed25519(
        HSLIC1_SIGNATURE_DOMAIN,
        &parsed.payload_bytes,
        &parsed.signature_bytes,
        public_key_bytes,
    )
}

pub fn verify_activation_request_v1_checksum(parsed: &ParsedActivationRequestV1) -> bool {
    let digest = digest_with_domain(HSREQ1_CHECKSUM_DOMAIN, &parsed.payload_bytes);
    constant_time_equal(&parsed.checksum_bytes, &digest[..12])
}

pub fn verify_revocation_list_v1_signature(
    parsed: &ParsedRevocationListV1,
    public_key_bytes: &[u8],
) -> Result<bool, String> {
    verify_ed25519(
        HSRVL1_SIGNATURE_DOMAIN,
        &parsed.payload_bytes,
        &parsed.signature_bytes,
        public_key_bytes,
    )
}

pub fn encode_offline_license_v1(
    payload: &OfflineLicensePayloadV1,
    signing_key: &SigningKey,
) -> Result<String, String> {
    let message = offline_license_v1_signing_message(payload)?;
    let signature = signing_key.sign(&message);
    encode_offline_license_v1_with_signature(payload, &signature.to_bytes())
}

pub fn offline_license_v1_signing_message(
    payload: &OfflineLicensePayloadV1,
) -> Result<Vec<u8>, String> {
    validate_license_payload(payload)?;
    signing_message_for_payload(HSLIC1_SIGNATURE_DOMAIN, payload)
}

pub fn encode_offline_license_v1_with_signature(
    payload: &OfflineLicensePayloadV1,
    signature_bytes: &[u8],
) -> Result<String, String> {
    validate_license_payload(payload)?;
    encode_signed_token_with_signature(HSLIC1_PREFIX, payload, signature_bytes)
}

pub fn encode_activation_request_v1(
    payload: &ActivationRequestPayloadV1,
) -> Result<String, String> {
    validate_request_payload(payload)?;
    let payload_bytes =
        serde_json::to_vec(payload).map_err(|_| "offline_license_request_invalid_format")?;
    let digest = digest_with_domain(HSREQ1_CHECKSUM_DOMAIN, &payload_bytes);
    Ok(format!(
        "{HSREQ1_PREFIX}.{}.{}",
        URL_SAFE_NO_PAD.encode(payload_bytes),
        URL_SAFE_NO_PAD.encode(&digest[..12])
    ))
}

pub fn encode_revocation_list_v1(
    payload: &RevocationListPayloadV1,
    signing_key: &SigningKey,
) -> Result<String, String> {
    let message = revocation_list_v1_signing_message(payload)?;
    let signature = signing_key.sign(&message);
    encode_revocation_list_v1_with_signature(payload, &signature.to_bytes())
}

pub fn revocation_list_v1_signing_message(
    payload: &RevocationListPayloadV1,
) -> Result<Vec<u8>, String> {
    validate_revocation_payload(payload)?;
    signing_message_for_payload(HSRVL1_SIGNATURE_DOMAIN, payload)
}

pub fn encode_revocation_list_v1_with_signature(
    payload: &RevocationListPayloadV1,
    signature_bytes: &[u8],
) -> Result<String, String> {
    validate_revocation_payload(payload)?;
    encode_signed_token_with_signature(HSRVL1_PREFIX, payload, signature_bytes)
}

pub fn validate_offline_artifact_v1(
    artifact_type: &str,
    token: &str,
    public_key_bytes: Option<&[u8]>,
) -> Result<(), String> {
    match artifact_type {
        "activation_request" => {
            let parsed = parse_activation_request_v1(token)?;
            if !verify_activation_request_v1_checksum(&parsed) {
                return Err("offline_license_request_checksum_mismatch".to_string());
            }
            Ok(())
        }
        "license" => {
            let parsed = parse_offline_license_v1(token)?;
            let public_key_bytes =
                public_key_bytes.ok_or_else(|| "offline_license_unknown_key".to_string())?;
            if !verify_offline_license_v1_signature(&parsed, public_key_bytes)? {
                return Err("offline_license_signature_invalid".to_string());
            }
            Ok(())
        }
        "revocation_list" => {
            let parsed = parse_revocation_list_v1(token)?;
            let public_key_bytes =
                public_key_bytes.ok_or_else(|| "offline_license_unknown_key".to_string())?;
            if !verify_revocation_list_v1_signature(&parsed, public_key_bytes)? {
                return Err("offline_license_revocation_signature_invalid".to_string());
            }
            Ok(())
        }
        _ => Err("offline_license_invalid_format".to_string()),
    }
}

pub fn derive_installation_id_v1(
    installation_secret: &[u8],
    salt: &[u8],
) -> Result<String, String> {
    if installation_secret.len() != 32 || salt.len() != 16 {
        return Err("offline_license_secure_storage_unavailable".to_string());
    }
    let mut input = Vec::with_capacity(INSTALLATION_ID_DOMAIN.len() + 48);
    input.extend_from_slice(INSTALLATION_ID_DOMAIN);
    input.extend_from_slice(installation_secret);
    input.extend_from_slice(salt);
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(input)))
}

fn decode_three_segment_token<const N: usize>(
    token: &str,
    prefix: &str,
    error_code: &str,
) -> Result<(Vec<u8>, [u8; N]), String> {
    if token.trim() != token || token.chars().any(char::is_whitespace) {
        return Err(error_code.to_string());
    }
    let segments = token.split('.').collect::<Vec<_>>();
    if segments.len() != 3 || segments[0] != prefix {
        return Err(error_code.to_string());
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(segments[1])
        .map_err(|_| error_code.to_string())?;
    let trailer_bytes = URL_SAFE_NO_PAD
        .decode(segments[2])
        .map_err(|_| error_code.to_string())?
        .try_into()
        .map_err(|_| error_code.to_string())?;
    Ok((payload_bytes, trailer_bytes))
}

fn parse_canonical_payload<T>(
    payload_bytes: &[u8],
    format_error: &str,
    canonical_error: &str,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let payload: T = serde_json::from_slice(payload_bytes).map_err(|_| format_error.to_string())?;
    let canonical_bytes = serde_json::to_vec(&payload).map_err(|_| format_error.to_string())?;
    if canonical_bytes != payload_bytes {
        return Err(canonical_error.to_string());
    }
    Ok(payload)
}

fn signing_message_for_payload<T>(domain: &[u8], payload: &T) -> Result<Vec<u8>, String>
where
    T: Serialize,
{
    let payload_bytes =
        serde_json::to_vec(payload).map_err(|_| "offline_license_invalid_format")?;
    Ok(message_with_domain(domain, &payload_bytes))
}

fn encode_signed_token_with_signature<T>(
    prefix: &str,
    payload: &T,
    signature_bytes: &[u8],
) -> Result<String, String>
where
    T: Serialize,
{
    let signature_bytes: &[u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "offline_license_signature_invalid".to_string())?;
    let payload_bytes =
        serde_json::to_vec(payload).map_err(|_| "offline_license_invalid_format")?;
    Ok(format!(
        "{prefix}.{}.{}",
        URL_SAFE_NO_PAD.encode(payload_bytes),
        URL_SAFE_NO_PAD.encode(signature_bytes)
    ))
}

fn verify_ed25519(
    domain: &[u8],
    payload_bytes: &[u8],
    signature_bytes: &[u8; 64],
    public_key_bytes: &[u8],
) -> Result<bool, String> {
    let public_key_bytes: &[u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "offline_license_unknown_key".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(public_key_bytes)
        .map_err(|_| "offline_license_unknown_key".to_string())?;
    let signature = Signature::from_bytes(signature_bytes);
    let message = message_with_domain(domain, payload_bytes);
    Ok(verifying_key.verify(&message, &signature).is_ok())
}

pub fn message_with_domain(domain: &[u8], payload_bytes: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(domain.len() + payload_bytes.len());
    message.extend_from_slice(domain);
    message.extend_from_slice(payload_bytes);
    message
}

pub fn digest_with_domain(domain: &[u8], payload_bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(message_with_domain(domain, payload_bytes)).into()
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn is_timestamp(value: &str) -> bool {
    value.len() == 20
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value.as_bytes()[10] == b'T'
        && value.as_bytes()[13] == b':'
        && value.as_bytes()[16] == b':'
        && value.as_bytes()[19] == b'Z'
        && value.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
}

fn is_installation_id(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_nonce(value: &str) -> bool {
    value.len() == 22
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
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

fn is_app_version(value: &str) -> bool {
    let core = value.split(['-', '+']).next().unwrap_or_default();
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn is_platform(value: &str) -> bool {
    matches!(value, "windows" | "macos" | "linux" | "android" | "ios")
}

fn is_sorted_unique_identifiers(values: &[String]) -> bool {
    values.iter().all(|value| is_identifier(value))
        && values
            .windows(2)
            .all(|pair| pair.first().is_some_and(|left| left < &pair[1]))
}

fn validate_license_payload(payload: &OfflineLicensePayloadV1) -> Result<(), String> {
    if payload.schema_version != 1 {
        return Err("offline_license_unknown_schema".to_string());
    }
    if payload.product_code != "creator_offline" {
        return Err("offline_license_feature_profile_invalid".to_string());
    }
    if !is_timestamp(&payload.expires_at)
        || !is_installation_id(&payload.installation_id)
        || !is_timestamp(&payload.issued_at)
        || !is_identifier(&payload.key_id)
        || !is_identifier(&payload.license_id)
        || !is_timestamp(&payload.not_before)
    {
        return Err("offline_license_invalid_format".to_string());
    }
    Ok(())
}

fn validate_request_payload(payload: &ActivationRequestPayloadV1) -> Result<(), String> {
    if payload.schema_version != 1 {
        return Err("offline_license_request_unknown_schema".to_string());
    }
    if payload.requested_product_code != "creator_offline" {
        return Err("offline_license_request_product_invalid".to_string());
    }
    if !is_app_version(&payload.app_version)
        || !is_timestamp(&payload.created_at)
        || !is_installation_id(&payload.installation_id)
        || !is_nonce(&payload.nonce)
        || !is_platform(&payload.platform)
        || !is_identifier(&payload.request_id)
    {
        return Err("offline_license_request_invalid_format".to_string());
    }
    Ok(())
}

fn validate_revocation_payload(payload: &RevocationListPayloadV1) -> Result<(), String> {
    if payload.schema_version != 1 {
        return Err("offline_license_revocation_unknown_schema".to_string());
    }
    if payload.list_type != "offline_license_revocations" {
        return Err("offline_license_revocation_list_invalid".to_string());
    }
    if payload.sequence == 0 {
        return Err("offline_license_revocation_sequence_invalid".to_string());
    }
    if !is_timestamp(&payload.generated_at)
        || !is_identifier(&payload.key_id)
        || !is_identifier(&payload.list_id)
        || !is_sorted_unique_identifiers(&payload.revoked_license_ids)
    {
        return Err("offline_license_revocation_list_invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture<T> {
        token: String,
        canonical_payload: String,
        public_key_base64_url: Option<String>,
        expected: T,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LicenseExpected {
        token_length: usize,
        schema_version: u8,
        license_id: String,
        product_code: String,
        installation_id: String,
        key_id: String,
        issued_at: String,
        not_before: String,
        expires_at: String,
        signature_valid: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RequestExpected {
        token_length: usize,
        schema_version: u8,
        request_id: String,
        requested_product_code: String,
        installation_id: String,
        platform: String,
        app_version: String,
        created_at: String,
        nonce: String,
        checksum_valid: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RevocationExpected {
        token_length: usize,
        schema_version: u8,
        list_id: String,
        list_type: String,
        key_id: String,
        generated_at: String,
        sequence: u64,
        revoked_license_ids: Vec<String>,
        signature_valid: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ErrorVectors {
        cases: Vec<ErrorVector>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ErrorVector {
        case_id: String,
        source: String,
        mutation: Mutation,
        expected_error: String,
    }

    #[derive(Deserialize)]
    struct Mutation {
        kind: String,
        value: Option<String>,
        from: Option<String>,
        to: Option<String>,
    }

    fn license_fixture() -> Fixture<LicenseExpected> {
        serde_json::from_str(include_str!(
            "../../docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json"
        ))
        .expect("license fixture must parse")
    }

    fn request_fixture() -> Fixture<RequestExpected> {
        serde_json::from_str(include_str!(
            "../../docs/fixtures/offline-license-k0/hsreq1-v1-valid.json"
        ))
        .expect("request fixture must parse")
    }

    fn revocation_fixture() -> Fixture<RevocationExpected> {
        serde_json::from_str(include_str!(
            "../../docs/fixtures/offline-license-k0/hsrvl1-ed25519-v1-valid.json"
        ))
        .expect("revocation fixture must parse")
    }

    #[test]
    fn offline_license_parses_all_shared_valid_vectors() {
        let license_fixture = license_fixture();
        let license = parse_offline_license_v1(&license_fixture.token).expect("license must parse");
        let public_key = decode_public_key(&license_fixture);
        assert_eq!(
            license_fixture.token.len(),
            license_fixture.expected.token_length
        );
        assert_eq!(
            license.payload_bytes,
            license_fixture.canonical_payload.as_bytes()
        );
        assert_eq!(
            license.payload.schema_version,
            license_fixture.expected.schema_version
        );
        assert_eq!(
            license.payload.license_id,
            license_fixture.expected.license_id
        );
        assert_eq!(
            license.payload.product_code,
            license_fixture.expected.product_code
        );
        assert_eq!(
            license.payload.installation_id,
            license_fixture.expected.installation_id
        );
        assert_eq!(license.payload.key_id, license_fixture.expected.key_id);
        assert_eq!(
            license.payload.issued_at,
            license_fixture.expected.issued_at
        );
        assert_eq!(
            license.payload.not_before,
            license_fixture.expected.not_before
        );
        assert_eq!(
            license.payload.expires_at,
            license_fixture.expected.expires_at
        );
        assert_eq!(
            verify_offline_license_v1_signature(&license, &public_key).unwrap(),
            license_fixture.expected.signature_valid
        );

        let request_fixture = request_fixture();
        let request =
            parse_activation_request_v1(&request_fixture.token).expect("request must parse");
        assert_eq!(
            request_fixture.token.len(),
            request_fixture.expected.token_length
        );
        assert_eq!(
            request.payload_bytes,
            request_fixture.canonical_payload.as_bytes()
        );
        assert_eq!(
            request.payload.schema_version,
            request_fixture.expected.schema_version
        );
        assert_eq!(
            request.payload.request_id,
            request_fixture.expected.request_id
        );
        assert_eq!(
            request.payload.requested_product_code,
            request_fixture.expected.requested_product_code
        );
        assert_eq!(
            request.payload.installation_id,
            request_fixture.expected.installation_id
        );
        assert_eq!(request.payload.platform, request_fixture.expected.platform);
        assert_eq!(
            request.payload.app_version,
            request_fixture.expected.app_version
        );
        assert_eq!(
            request.payload.created_at,
            request_fixture.expected.created_at
        );
        assert_eq!(request.payload.nonce, request_fixture.expected.nonce);
        assert_eq!(
            verify_activation_request_v1_checksum(&request),
            request_fixture.expected.checksum_valid
        );

        let revocation_fixture = revocation_fixture();
        let revocation =
            parse_revocation_list_v1(&revocation_fixture.token).expect("revocation must parse");
        let public_key = decode_public_key(&revocation_fixture);
        assert_eq!(
            revocation_fixture.token.len(),
            revocation_fixture.expected.token_length
        );
        assert_eq!(
            revocation.payload_bytes,
            revocation_fixture.canonical_payload.as_bytes()
        );
        assert_eq!(
            revocation.payload.schema_version,
            revocation_fixture.expected.schema_version
        );
        assert_eq!(
            revocation.payload.list_id,
            revocation_fixture.expected.list_id
        );
        assert_eq!(
            revocation.payload.list_type,
            revocation_fixture.expected.list_type
        );
        assert_eq!(
            revocation.payload.key_id,
            revocation_fixture.expected.key_id
        );
        assert_eq!(
            revocation.payload.generated_at,
            revocation_fixture.expected.generated_at
        );
        assert_eq!(
            revocation.payload.sequence,
            revocation_fixture.expected.sequence
        );
        assert_eq!(
            revocation.payload.revoked_license_ids,
            revocation_fixture.expected.revoked_license_ids
        );
        assert_eq!(
            verify_revocation_list_v1_signature(&revocation, &public_key).unwrap(),
            revocation_fixture.expected.signature_valid
        );
    }

    #[test]
    fn offline_license_matches_all_shared_error_vectors() {
        let license_fixture = license_fixture();
        let request_fixture = request_fixture();
        let revocation_fixture = revocation_fixture();
        let vectors: ErrorVectors = serde_json::from_str(include_str!(
            "../../docs/fixtures/offline-license-k0/offline-license-errors-v1.json"
        ))
        .expect("error vectors must parse");

        for vector in vectors.cases {
            let fixture = match vector.source.as_str() {
                "license" => (
                    &license_fixture.token,
                    license_fixture.public_key_base64_url.as_deref(),
                ),
                "activation_request" => (&request_fixture.token, None),
                "revocation_list" => (
                    &revocation_fixture.token,
                    revocation_fixture.public_key_base64_url.as_deref(),
                ),
                _ => panic!("unknown source"),
            };
            let (token, public_key) = mutate_vector(fixture.0, fixture.1, &vector.mutation);
            let public_key_bytes = public_key.as_deref().map(|value| {
                URL_SAFE_NO_PAD
                    .decode(value)
                    .expect("public key must decode")
            });
            let actual =
                validate_offline_artifact_v1(&vector.source, &token, public_key_bytes.as_deref())
                    .expect_err("error vector must fail");
            assert_eq!(actual, vector.expected_error, "{}", vector.case_id);
        }
    }

    #[test]
    fn offline_license_derives_shared_installation_identity() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/fixtures/offline-license-k0/installation-identity-v1.json"
        ))
        .unwrap();
        let secret = URL_SAFE_NO_PAD
            .decode(fixture["testOnlySecretBase64Url"].as_str().unwrap())
            .unwrap();
        let salt = URL_SAFE_NO_PAD
            .decode(fixture["saltBase64Url"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            derive_installation_id_v1(&secret, &salt).unwrap(),
            fixture["expectedInstallationId"].as_str().unwrap()
        );
    }

    fn decode_public_key<T>(fixture: &Fixture<T>) -> Vec<u8> {
        URL_SAFE_NO_PAD
            .decode(
                fixture
                    .public_key_base64_url
                    .as_deref()
                    .expect("fixture public key"),
            )
            .expect("public key must decode")
    }

    fn mutate_vector(
        token: &str,
        public_key: Option<&str>,
        mutation: &Mutation,
    ) -> (String, Option<String>) {
        let mut segments = token.split('.').map(str::to_string).collect::<Vec<_>>();
        let mut public_key = public_key.map(str::to_string);
        match mutation.kind.as_str() {
            "replace_prefix" => segments[0] = mutation.value.clone().expect("value"),
            "replace_payload" => {
                let payload = String::from_utf8(
                    URL_SAFE_NO_PAD
                        .decode(&segments[1])
                        .expect("payload decode"),
                )
                .expect("payload utf8");
                let from = mutation.from.as_deref().expect("from");
                assert_eq!(payload.matches(from).count(), 1);
                let mutated = payload.replacen(from, mutation.to.as_deref().expect("to"), 1);
                segments[1] = URL_SAFE_NO_PAD.encode(mutated);
            }
            "replace_trailer" => segments[2] = mutation.value.clone().expect("value"),
            "replace_public_key" => public_key = mutation.value.clone(),
            _ => panic!("unknown mutation"),
        }
        (segments.join("."), public_key)
    }
}
