use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use watermark_core::{
    audio_v3_quality_diagnostics, extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes,
    extract_watermark_wav_readonly_candidate_bytes, AIContentFlags,
    AudioNoiseFloorMigrationCandidateFailureCode, EmbedOptions, MediaInput, MediaOutput,
    PayloadV2BuildInput, WatermarkDecodedPayload, WatermarkIssueMode, WatermarkMediaType,
    WatermarkPayload, AUDIO_NOISE_FLOOR_CANDIDATE_FALLBACK_PATH,
    AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE, AUDIO_NOISE_FLOOR_LEGACY_V3_FALLBACK_PATH,
    AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const SAMPLE_RATE: usize = 44_100;
const SAMPLE_SECONDS: usize = 30;
const AUDIO_STRATEGY_VERSION: &str = "v3_recovery_2_8k_legacy";
const EXPECTED_EXTRACTOR_PATH: &str =
    "WatermarkService::extract -> audio::extract_watermark_wav_readonly_candidate_bytes_with_delta";

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let manifest_path = PathBuf::from(optional_arg(&args, "--manifest").unwrap_or_else(|| {
        "watermark-core/fixtures/audio-noise-floor-migration/manifest.example.json".to_string()
    }));
    let run_id = optional_arg(&args, "--run-id").unwrap_or_else(|| unix_seconds().to_string());
    let out_dir = PathBuf::from(optional_arg(&args, "--out-dir").unwrap_or_else(|| {
        format!("watermark-core/target/audio-noise-floor-migration-read-compat/run-{run_id}")
    }));
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("read manifest {}: {error}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&manifest_text).map_err(|error| format!("parse manifest: {error}"))?;
    validate_manifest(&manifest)?;

    let rows = manifest
        .fixtures
        .iter()
        .map(run_fixture)
        .collect::<Result<Vec<_>, _>>()?;
    let pass = rows.iter().all(|row| row.pass);

    let result = json!({
        "runId": run_id,
        "gate": "watermark:audio-noise-floor-migration-read-compat",
        "manifest": manifest_path.to_string_lossy(),
        "schemaVersion": manifest.schema_version,
        "migrationPhase": manifest.migration_phase,
        "payloadProtocolVersion": manifest.payload_protocol_version,
        "payloadBytesLength": manifest.payload_bytes_length,
        "audioStrategyVersion": manifest.audio_strategy_version,
        "expectedExtractorPath": manifest.expected_extractor_path,
        "extractorPath": AUDIO_NOISE_FLOOR_LEGACY_V3_FALLBACK_PATH,
        "extractorFallbackPath": AUDIO_NOISE_FLOOR_CANDIDATE_FALLBACK_PATH,
        "readCompatibilityMode": AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE,
        "candidateScanAttempted": true,
        "candidateScanProfiles": candidate_scan_profiles_json(),
        "candidateFailureCode": AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound.as_str(),
        "candidateFailureMessage": "migrated-band candidate scan did not find a V3/39 payload",
        "candidateFailureMatrix": candidate_failure_matrix_json(&rows),
        "pass": pass,
        "fixtures": rows.iter().map(|row| row.json.clone()).collect::<Vec<_>>()
    });
    let json_text =
        serde_json::to_string_pretty(&result).map_err(|error| format!("render json: {error}"))?;
    fs::write(out_dir.join("read-compat.json"), json_text.as_bytes())
        .map_err(|error| format!("write read-compat json: {error}"))?;
    fs::write(
        out_dir.join("read-compat.md"),
        render_markdown(&rows, pass).as_bytes(),
    )
    .map_err(|error| format!("write read-compat markdown: {error}"))?;
    println!("{json_text}");

    if pass {
        Ok(())
    } else {
        Err("audio noise-floor migration read compatibility failed".to_string())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    migration_phase: String,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    audio_strategy_version: String,
    expected_extractor_path: String,
    formal_thresholds: FormalThresholds,
    rollback_policy: RollbackPolicy,
    planned_extractor_read_order: Vec<String>,
    planned_report_fields: Vec<String>,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FormalThresholds {
    field_noise_min_snr_db: f64,
    extraction_confidence_min: f32,
    thresholds_must_not_drop: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RollbackPolicy {
    write_strategy_flag_required: bool,
    legacy_fallback_required: bool,
    platform_algorithm_drift_forbidden: bool,
    formal_thresholds_must_not_drop: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    sample_id: String,
    origin_endpoint: String,
    artifact_role: String,
    artifact_mode: String,
    audio_profile: String,
    media_type: String,
    watermark_id_hex32: String,
    expected_watermark_uid: String,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    audio_strategy_version: String,
    expected_read_paths: Vec<String>,
    min_extraction_confidence: f32,
    protected_path: Option<String>,
    sha256: Option<String>,
    bytes: Option<u64>,
    generated_by: Option<String>,
}

struct FixtureRow {
    pass: bool,
    markdown: String,
    json: serde_json::Value,
}

struct CandidateReadReport {
    candidate_path: &'static str,
    candidate_status: &'static str,
    candidate_scan_attempted: bool,
    candidate_failure_code: Option<String>,
    candidate_failure_message: Option<String>,
    extractor_path: &'static str,
    extractor_fallback_path: &'static str,
    read_compatibility_mode: &'static str,
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    require(
        manifest.schema_version == "audio_noise_floor_migration_manifest_v1",
        "schemaVersion must be audio_noise_floor_migration_manifest_v1",
    )?;
    require(
        manifest.migration_phase == "read_compat_legacy_v3",
        "migrationPhase must be read_compat_legacy_v3",
    )?;
    require(
        manifest.payload_protocol_version == 3
            && manifest.payload_bytes_length == PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        "manifest must stay on V3/39",
    )?;
    require(
        manifest.audio_strategy_version == AUDIO_STRATEGY_VERSION,
        "manifest must target the legacy V3 2-8 kHz recovery strategy",
    )?;
    require(
        manifest.expected_extractor_path == EXPECTED_EXTRACTOR_PATH,
        "manifest must document the current shared-core extractor path",
    )?;
    require(
        manifest.formal_thresholds.field_noise_min_snr_db == 44.0
            && manifest.formal_thresholds.extraction_confidence_min >= 0.99
            && manifest.formal_thresholds.thresholds_must_not_drop,
        "formal thresholds must not drop",
    )?;
    require(
        manifest.rollback_policy.write_strategy_flag_required
            && manifest.rollback_policy.legacy_fallback_required
            && manifest.rollback_policy.platform_algorithm_drift_forbidden
            && manifest.rollback_policy.formal_thresholds_must_not_drop,
        "rollback policy must preserve strategy flag, legacy fallback, no platform drift, and no threshold drop",
    )?;
    let expected_order = [
        "v3_noise_floor_migrated_band_v1_candidate",
        "v3_recovery_2_8k_legacy",
        "legacy_v3_readonly_candidate",
        "v2_rollback_legacy",
    ];
    require(
        manifest.planned_extractor_read_order.len() == expected_order.len()
            && manifest
                .planned_extractor_read_order
                .iter()
                .zip(expected_order)
                .all(|(actual, expected)| actual == expected),
        "planned extractor read order must keep candidate -> legacy V3 -> readonly candidate -> V2 rollback",
    )?;
    for field in [
        "watermarkUid",
        "payloadProtocolVersion",
        "payloadBytesLength",
        "audioStrategyVersion",
        "extractorPath",
        "extractorFallbackPath",
        "candidateFailureCode",
        "candidateFailureMessage",
        "extractionConfidence",
        "readCompatibilityMode",
    ] {
        require(
            manifest
                .planned_report_fields
                .iter()
                .any(|candidate| candidate == field),
            &format!("planned report fields must include {field}"),
        )?;
    }
    require(
        manifest.fixtures.len() >= 3,
        "read-compat manifest must include core, desktop, and mobile legacy fixtures",
    )?;
    for origin in ["watermark_core_legacy", "desktop_legacy", "mobile_legacy"] {
        require(
            manifest
                .fixtures
                .iter()
                .any(|fixture| fixture.origin_endpoint == origin),
            &format!("missing legacy fixture for {origin}"),
        )?;
    }
    for origin in ["desktop_legacy", "android_native_legacy"] {
        require(
            manifest.fixtures.iter().any(|fixture| {
                fixture.origin_endpoint == origin
                    && fixture.artifact_mode == "file_backed_legacy_v3_wav"
            }),
            &format!("missing file-backed legacy fixture for {origin}"),
        )?;
    }
    Ok(())
}

fn run_fixture(fixture: &Fixture) -> Result<FixtureRow, String> {
    validate_fixture(fixture)?;
    let control_samples = make_field_noise_samples(SAMPLE_SECONDS);
    let control_wav = encode_wav(&control_samples)?;
    let payload = build_payload(fixture, &control_wav)?;
    require(
        payload.watermark_uid() == fixture.expected_watermark_uid,
        &format!(
            "{} expected UID does not match watermarkIdHex32",
            fixture.sample_id
        ),
    )?;

    let protected_wav = match fixture.artifact_mode.as_str() {
        "generated_legacy_v3_wav" => {
            let output = watermark_core::WatermarkService::embed(
                MediaInput::AudioWavBytes {
                    bytes: control_wav.clone(),
                },
                &payload,
                EmbedOptions {
                    allow_rewrite: true,
                    ..EmbedOptions::default()
                },
            )
            .map_err(|error| format!("embed legacy V3 fixture {}: {error}", fixture.sample_id))?;
            let MediaOutput::AudioWavBytes { bytes } = output else {
                return Err(format!("{} returned non-audio output", fixture.sample_id));
            };
            bytes
        }
        "file_backed_legacy_v3_wav" => read_file_backed_fixture(fixture)?,
        _ => {
            return Err(format!(
                "{} has unsupported artifactMode {}",
                fixture.sample_id, fixture.artifact_mode
            ))
        }
    };

    let default_decoded = watermark_core::WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: protected_wav.clone(),
    })
    .map_err(|error| format!("default extract {}: {error}", fixture.sample_id))?;
    let readonly_decoded = extract_watermark_wav_readonly_candidate_bytes(&protected_wav)
        .map_err(|error| format!("readonly candidate extract {}: {error}", fixture.sample_id))?;
    let protected_samples = decode_wav_samples(&protected_wav)?;
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = default_decoded.clone() else {
        return Err(format!(
            "{} did not decode as V3 minimal anchor",
            fixture.sample_id
        ));
    };
    let diagnostics = audio_v3_quality_diagnostics(&control_samples, &protected_samples, &anchor)
        .map_err(|error| format!("diagnostics {}: {error}", fixture.sample_id))?;

    let default_ok = decoded_matches(&default_decoded, fixture);
    let readonly_ok = decoded_matches(&readonly_decoded, fixture);
    let confidence_ok = diagnostics.extraction_confidence >= fixture.min_extraction_confidence;
    let candidate_read = run_new_extractor_candidate_read(&protected_wav);
    let fallback_ok = candidate_read.extractor_path == AUDIO_STRATEGY_VERSION
        && candidate_read
            .extractor_fallback_path
            .contains(AUDIO_STRATEGY_VERSION)
        && candidate_read.read_compatibility_mode == AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE
        && candidate_read.candidate_failure_code.as_deref()
            == Some(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound.as_str());
    let pass = default_ok && readonly_ok && confidence_ok && fallback_ok;

    Ok(FixtureRow {
        pass,
        markdown: format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.6} | {} |",
            fixture.sample_id,
            fixture.origin_endpoint,
            fixture.artifact_role,
            candidate_read.extractor_path,
            candidate_read.extractor_fallback_path,
            candidate_read.read_compatibility_mode,
            if default_ok { "PASS" } else { "FAIL" },
            if readonly_ok { "PASS" } else { "FAIL" },
            diagnostics.extraction_confidence,
            if pass { "PASS" } else { "FAIL" },
        ),
        json: json!({
            "sampleId": fixture.sample_id,
            "originEndpoint": fixture.origin_endpoint,
            "artifactRole": fixture.artifact_role,
            "artifactMode": fixture.artifact_mode,
            "audioProfile": fixture.audio_profile,
            "protectedPath": fixture.protected_path,
            "sha256": fixture.sha256,
            "bytes": fixture.bytes,
            "generatedBy": fixture.generated_by,
            "expectedWatermarkUid": fixture.expected_watermark_uid,
            "payloadProtocolVersion": fixture.payload_protocol_version,
            "payloadBytesLength": fixture.payload_bytes_length,
            "audioStrategyVersion": fixture.audio_strategy_version,
            "extractorPath": candidate_read.extractor_path,
            "extractorFallbackPath": candidate_read.extractor_fallback_path,
            "candidateFailureCode": candidate_read.candidate_failure_code,
            "candidateFailureMessage": candidate_read.candidate_failure_message,
            "readCompatibilityMode": candidate_read.read_compatibility_mode,
            "newExtractorCandidate": {
                "path": candidate_read.candidate_path,
                "status": candidate_read.candidate_status,
                "readOnly": true,
                "scanAttempted": candidate_read.candidate_scan_attempted,
                "scanProfiles": candidate_scan_profiles_json(),
                "failureCode": candidate_read.candidate_failure_code,
                "failureMessage": candidate_read.candidate_failure_message
            },
            "defaultExtractor": {
                "pass": default_ok,
                "watermarkUid": default_decoded.watermark_uid(),
                "payloadProtocolVersion": default_decoded.protocol_version(),
                "payloadBytesLength": default_decoded.payload_bytes_length()
            },
            "legacyReadonlyCandidate": {
                "pass": readonly_ok,
                "watermarkUid": readonly_decoded.watermark_uid(),
                "payloadProtocolVersion": readonly_decoded.protocol_version(),
                "payloadBytesLength": readonly_decoded.payload_bytes_length()
            },
            "metrics": {
                "extractionConfidence": diagnostics.extraction_confidence,
                "minExtractionConfidence": fixture.min_extraction_confidence,
                "modifiedPairRatio": diagnostics.modified_pair_ratio,
                "noiseFloorSparseRecovery": diagnostics.noise_floor_sparse_recovery
            },
            "pass": pass
        }),
    })
}

fn run_new_extractor_candidate_read(protected_wav: &[u8]) -> CandidateReadReport {
    match extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes(protected_wav) {
        Ok(_) => CandidateReadReport {
            candidate_path: AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH,
            candidate_status: "candidate_read_succeeded_unexpected_for_legacy_fixture",
            candidate_scan_attempted: true,
            candidate_failure_code: None,
            candidate_failure_message: None,
            extractor_path: AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH,
            extractor_fallback_path: "none",
            read_compatibility_mode: "new_candidate_read",
        },
        Err(error) => CandidateReadReport {
            candidate_path: error.extractor_path,
            candidate_status: "candidate_failed_fallback_required",
            candidate_scan_attempted: true,
            candidate_failure_code: Some(error.code.as_str().to_string()),
            candidate_failure_message: Some(error.message),
            extractor_path: AUDIO_NOISE_FLOOR_LEGACY_V3_FALLBACK_PATH,
            extractor_fallback_path: AUDIO_NOISE_FLOOR_CANDIDATE_FALLBACK_PATH,
            read_compatibility_mode: AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE,
        },
    }
}

fn candidate_failure_matrix_json(rows: &[FixtureRow]) -> serde_json::Value {
    let codes = [
        (
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateNotImplementedNoFrequencyStrategy,
            "legacy_stub_regression",
            "fail_read_compat_gate",
            "block_if_candidate_scan_regresses_to_not_implemented_stub",
            true,
            true,
        ),
        (
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateInputInvalid,
            "input_or_candidate_parser_regression",
            "fail_read_compat_gate",
            "block_until_fixture_input_or_candidate_wav_parser_is_fixed",
            false,
            true,
        ),
        (
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort,
            "fixture_coverage_regression",
            "fail_read_compat_gate",
            "block_until_fixture_duration_meets_candidate_and_legacy_read_requirements",
            false,
            true,
        ),
        (
            AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound,
            "current_expected_for_legacy_v3_fixtures_after_read_only_scan",
            "legacy_fixture_may_fallback_new_candidate_fixture_must_block",
            "old_v3_fixtures_pass_only_if_legacy_v3_reads_confidently_new_strategy_fixtures_block",
            true,
            false,
        ),
        (
            AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadInvalid,
            "future_candidate_payload_decode_or_auth_failure",
            "legacy_fixture_may_fallback_new_candidate_fixture_must_block",
            "old_v3_fixtures_pass_only_if_legacy_v3_reads_confidently_new_strategy_fixtures_block",
            true,
            false,
        ),
    ];
    json!(codes
        .iter()
        .map(
            |(
                code,
                expectation,
                expected_handling,
                gate_disposition,
                legacy_fallback_allowed,
                current_observation_blocks_gate,
            )| {
                let code = code.as_str();
                json!({
                    "code": code,
                    "expectation": expectation,
                    "expectedHandling": expected_handling,
                    "gateDisposition": gate_disposition,
                    "legacyFallbackAllowed": legacy_fallback_allowed,
                    "currentObservedCount": count_candidate_failure_code(rows, code),
                    "currentObservationBlocksGate": current_observation_blocks_gate,
                })
            }
        )
        .collect::<Vec<_>>())
}

fn candidate_scan_profiles_json() -> serde_json::Value {
    json!([
        {
            "id": "noise_floor_low_mid_0_9_4_8k",
            "frequencyRangeHz": [900, 4800],
            "window": "v3_39_recovery_packet_frames",
            "readOnly": true
        },
        {
            "id": "noise_floor_mid_shift_1_2_6_2k",
            "frequencyRangeHz": [1200, 6200],
            "window": "v3_39_recovery_packet_frames",
            "readOnly": true
        },
        {
            "id": "noise_floor_high_spread_3_8_9_6k",
            "frequencyRangeHz": [3800, 9600],
            "window": "v3_39_recovery_packet_frames",
            "readOnly": true
        }
    ])
}

fn count_candidate_failure_code(rows: &[FixtureRow], code: &str) -> usize {
    rows.iter()
        .filter(|row| row.json["candidateFailureCode"].as_str() == Some(code))
        .count()
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    require(
        matches!(
            fixture.origin_endpoint.as_str(),
            "watermark_core_legacy" | "desktop_legacy" | "mobile_legacy" | "android_native_legacy"
        ),
        &format!("{} has unsupported originEndpoint", fixture.sample_id),
    )?;
    require(
        matches!(
            fixture.artifact_role.as_str(),
            "legacy_core_reference"
                | "desktop_old_write"
                | "mobile_old_write"
                | "android_native_old_write"
        ),
        &format!("{} has unsupported artifactRole", fixture.sample_id),
    )?;
    require(
        matches!(
            fixture.artifact_mode.as_str(),
            "generated_legacy_v3_wav" | "file_backed_legacy_v3_wav"
        ),
        &format!(
            "{} must be generated_legacy_v3_wav or file_backed_legacy_v3_wav",
            fixture.sample_id
        ),
    )?;
    require(
        fixture.audio_profile == "field_noise_noise_floor" && fixture.media_type == "audio",
        &format!("{} must be a field-noise audio fixture", fixture.sample_id),
    )?;
    require(
        fixture.payload_protocol_version == 3
            && fixture.payload_bytes_length == PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        &format!("{} must stay on V3/39", fixture.sample_id),
    )?;
    require(
        fixture.audio_strategy_version == AUDIO_STRATEGY_VERSION,
        &format!("{} must use the legacy strategy marker", fixture.sample_id),
    )?;
    require(
        fixture
            .expected_read_paths
            .iter()
            .any(|path| path == "current_default_extractor")
            && fixture
                .expected_read_paths
                .iter()
                .any(|path| path == "legacy_v3_readonly_candidate"),
        &format!("{} must cover both read paths", fixture.sample_id),
    )?;
    require(
        fixture.min_extraction_confidence >= 0.99,
        &format!(
            "{} must keep extraction confidence >= 0.99",
            fixture.sample_id
        ),
    )?;
    if fixture.artifact_mode == "file_backed_legacy_v3_wav" {
        require(
            fixture.protected_path.is_some()
                && fixture.sha256.is_some()
                && fixture.bytes.is_some()
                && fixture.generated_by.is_some(),
            &format!(
                "{} file-backed fixture must include protectedPath, sha256, bytes, and generatedBy",
                fixture.sample_id
            ),
        )?;
    }
    Ok(())
}

fn read_file_backed_fixture(fixture: &Fixture) -> Result<Vec<u8>, String> {
    let path = fixture
        .protected_path
        .as_deref()
        .ok_or_else(|| format!("{} missing protectedPath", fixture.sample_id))?;
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path))?;
    let expected_len = fixture
        .bytes
        .ok_or_else(|| format!("{} missing bytes", fixture.sample_id))?;
    require(
        bytes.len() as u64 == expected_len,
        &format!(
            "{} bytes mismatch: expected {}, got {}",
            fixture.sample_id,
            expected_len,
            bytes.len()
        ),
    )?;
    let expected_sha = fixture
        .sha256
        .as_deref()
        .ok_or_else(|| format!("{} missing sha256", fixture.sample_id))?;
    let actual_sha = sha256_hex(&bytes);
    require(
        actual_sha == expected_sha,
        &format!(
            "{} sha256 mismatch: expected {}, got {}",
            fixture.sample_id, expected_sha, actual_sha
        ),
    )?;
    Ok(bytes)
}

fn decoded_matches(decoded: &WatermarkDecodedPayload, fixture: &Fixture) -> bool {
    decoded.is_v3_minimal_anchor()
        && decoded.watermark_uid() == fixture.expected_watermark_uid
        && decoded.protocol_version() == fixture.payload_protocol_version
        && decoded.payload_bytes_length() == fixture.payload_bytes_length
}

fn build_payload(fixture: &Fixture, source_wav: &[u8]) -> Result<WatermarkPayload, String> {
    let watermark_id = parse_hex_16(&fixture.watermark_id_hex32)?;
    let original_sha256: [u8; 32] = Sha256::digest(source_wav).into();
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_788_192_000,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type: WatermarkMediaType::Audio,
        registry_proof_hash: Some(sha256_prefix_16(
            format!("{}:legacy-registry-proof", fixture.sample_id).as_bytes(),
        )),
        creator_binding: Some("HiddenShield audio noise-floor migration read compat"),
    })
    .map_err(|error| format!("build payload {}: {error}", fixture.sample_id))
}

fn make_field_noise_samples(seconds: usize) -> Vec<f32> {
    (0..(SAMPLE_RATE * seconds))
        .map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            let deterministic_noise = (index as u32)
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let noise = ((deterministic_noise >> 8) & 0xffff) as f32 / 32768.0 - 1.0;
            0.11 * noise + 0.08 * (2.0 * std::f32::consts::PI * 130.0 * t).sin()
        })
        .collect()
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|error| format!("wav: {error}"))?;
        for sample in samples {
            writer
                .write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
                .map_err(|error| format!("write wav sample: {error}"))?;
        }
        writer
            .finalize()
            .map_err(|error| format!("finalize wav: {error}"))?;
    }
    Ok(cursor.into_inner())
}

fn decode_wav_samples(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let cursor = Cursor::new(bytes);
    let mut reader =
        hound::WavReader::new(cursor).map_err(|error| format!("read protected wav: {error}"))?;
    reader
        .samples::<i16>()
        .map(|sample| sample.map(|value| f32::from(value) / f32::from(i16::MAX)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("decode protected wav samples: {error}"))
}

fn parse_hex_16(value: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 {
        return Err("watermarkIdHex32 must contain 32 hex chars".to_string());
    }
    let mut out = [0_u8; 16];
    for index in 0..16 {
        out[index] = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("parse watermarkIdHex32: {error}"))?;
    }
    Ok(out)
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut out = [0_u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn render_markdown(rows: &[FixtureRow], pass: bool) -> String {
    let mut markdown = format!(
        "# Audio Noise-floor Migration Read Compatibility\n\n- gate: `watermark:audio-noise-floor-migration-read-compat`\n- pass: {}\n\n",
        pass
    );
    markdown.push_str("| sample | origin | artifact role | extractor path | fallback path | read mode | default extractor | readonly candidate | extraction confidence | result |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | ---: | --- |\n");
    for row in rows {
        markdown.push_str(&row.markdown);
        markdown.push('\n');
    }
    markdown.push_str("\n## Candidate Failure Matrix\n\n");
    markdown.push_str("| code | expected handling | gate disposition | observed |\n");
    markdown.push_str("| --- | --- | --- | ---: |\n");
    for entry in candidate_failure_matrix_json(rows)
        .as_array()
        .into_iter()
        .flatten()
    {
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            entry["code"].as_str().unwrap_or("unknown"),
            entry["expectedHandling"].as_str().unwrap_or("unknown"),
            entry["gateDisposition"].as_str().unwrap_or("unknown"),
            entry["currentObservedCount"].as_u64().unwrap_or(0),
        ));
    }
    markdown.push_str(
        "\nThis gate only verifies legacy V3/39 read compatibility. It does not implement a new frequency strategy and does not lower formal quality thresholds.\n",
    );
    markdown
}

fn optional_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
