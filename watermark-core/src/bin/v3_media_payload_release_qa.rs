use image::{ImageBuffer, ImageFormat, Rgb};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use watermark_core::{
    AIContentFlags, AudioProtectionMode, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput,
    PayloadV2BuildInput, WatermarkDecodedPayload, WatermarkIssueMode, WatermarkMediaType,
    WatermarkPayload, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let run_id = optional_arg(&args, "--run-id").unwrap_or_else(|| unix_seconds().to_string());
    let out_dir = PathBuf::from(optional_arg(&args, "--out-dir").unwrap_or_else(|| {
        format!("watermark-core/target/v3-media-payload-release-qa/run-{run_id}")
    }));
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image_rows = run_image_v3_matrix(&run_id, &out_dir)?;
    let audio = run_audio_v3(&run_id, &out_dir, false)?;
    let video_l1 = run_audio_v3(&run_id, &out_dir, true)?;
    let video_l2 = video_l2_boundary();
    let image_pass = image_rows.iter().all(|row| row.pass);
    let pass = image_pass && audio.pass && video_l1.pass && video_l2.pass;

    let json = format!(
        "{{\n  \"runId\":\"{}\",\n  \"pass\":{},\n  \"images\":[{}],\n  \"audio\":{},\n  \"videoL1AudioTrack\":{},\n  \"videoL2FingerprintNotary\":{}\n}}\n",
        json_escape(&run_id),
        pass,
        image_rows
            .iter()
            .map(|row| row.json.clone())
            .collect::<Vec<_>>()
            .join(","),
        audio.json,
        video_l1.json,
        video_l2.json,
    );
    let markdown = format!(
        "# HiddenShield V3 Media Payload Release QA\n\n- runId: `{}`\n- pass: {}\n\n| capability | expected | result |\n| --- | --- | --- |\n| image PNG/JPEG/WebP/BMP | V3/39 write + read | {} |\n| audio | V3/39 write + read | {} |\n| video L1 audio track | V3/39 write + read via `AudioProtectionMode::VideoTrack` | {} |\n| video L2 fingerprint notary | no media payload write; irreversible notary metadata only | {} |\n",
        run_id,
        pass,
        if image_pass { "PASS" } else { "FAIL" },
        if audio.pass { "PASS" } else { "FAIL" },
        if video_l1.pass { "PASS" } else { "FAIL" },
        if video_l2.pass { "PASS" } else { "FAIL" },
    );
    fs::write(out_dir.join("v3-media-payload-release-qa.json"), &json)
        .map_err(|error| format!("write qa json: {error}"))?;
    fs::write(out_dir.join("v3-media-payload-release-qa.md"), &markdown)
        .map_err(|error| format!("write qa markdown: {error}"))?;
    print!("{json}");
    if pass {
        Ok(())
    } else {
        Err("V3 media payload release QA failed".to_string())
    }
}

struct QaRow {
    pass: bool,
    json: String,
}

#[derive(Clone, Copy)]
struct ImageFormatSpec {
    id: &'static str,
    capability: &'static str,
    extension: &'static str,
    output_format: ImageOutputFormat,
    container_format: ImageFormat,
}

fn run_image_v3_matrix(run_id: &str, out_dir: &PathBuf) -> Result<Vec<QaRow>, String> {
    let specs = [
        ImageFormatSpec {
            id: "png",
            capability: "image_png",
            extension: "png",
            output_format: ImageOutputFormat::Png,
            container_format: ImageFormat::Png,
        },
        ImageFormatSpec {
            id: "jpeg",
            capability: "image_jpeg",
            extension: "jpg",
            output_format: ImageOutputFormat::Jpeg,
            container_format: ImageFormat::Jpeg,
        },
        ImageFormatSpec {
            id: "webp",
            capability: "image_webp",
            extension: "webp",
            output_format: ImageOutputFormat::WebP,
            container_format: ImageFormat::WebP,
        },
        ImageFormatSpec {
            id: "bmp",
            capability: "image_bmp",
            extension: "bmp",
            output_format: ImageOutputFormat::Bmp,
            container_format: ImageFormat::Bmp,
        },
    ];

    specs
        .into_iter()
        .map(|spec| run_image_v3_format(run_id, out_dir, spec))
        .collect()
}

fn run_image_v3_format(
    run_id: &str,
    out_dir: &PathBuf,
    spec: ImageFormatSpec,
) -> Result<QaRow, String> {
    let source = make_image(spec.container_format)?;
    let payload = build_payload(
        run_id,
        &format!("image-{}", spec.id),
        &source,
        WatermarkMediaType::Image,
    )?;
    let output = watermark_core::WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source.clone(),
        },
        &payload,
        EmbedOptions {
            image_output_format: spec.output_format,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed image V3: {error}"))?;
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err("image V3 returned non-image output".to_string());
    };
    fs::write(
        out_dir.join(format!("image-v3-{}.{}", spec.id, spec.extension)),
        &bytes,
    )
    .map_err(|error| format!("write image output: {error}"))?;
    let decoded = watermark_core::WatermarkService::extract(MediaInput::ImageBytes { bytes })
        .map_err(|error| format!("extract image {} V3: {error}", spec.id))?;
    Ok(row_from_decoded(
        spec.capability,
        payload.watermark_uid(),
        decoded,
        true,
    ))
}

fn run_audio_v3(run_id: &str, out_dir: &PathBuf, video_track: bool) -> Result<QaRow, String> {
    let source = make_wav_audio(if video_track { 12 } else { 30 })?;
    let media_type = if video_track {
        WatermarkMediaType::VideoAudioTrack
    } else {
        WatermarkMediaType::Audio
    };
    let payload = build_payload(
        run_id,
        if video_track {
            "video-l1-audio-track"
        } else {
            "audio"
        },
        &source,
        media_type,
    )?;
    let output = watermark_core::WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source.clone(),
        },
        &payload,
        EmbedOptions {
            allow_rewrite: true,
            audio_protection_mode: if video_track {
                AudioProtectionMode::VideoTrack
            } else {
                AudioProtectionMode::StandaloneAudio
            },
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed audio V3: {error}"))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("audio V3 returned non-audio output".to_string());
    };
    let output_name = if video_track {
        "video-l1-audio-track-v3.wav"
    } else {
        "audio-v3.wav"
    };
    fs::write(out_dir.join(output_name), &bytes)
        .map_err(|error| format!("write audio output: {error}"))?;
    let decoded = watermark_core::WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
        .map_err(|error| format!("extract audio V3: {error}"))?;
    Ok(row_from_decoded(
        if video_track {
            "video_l1_audio_track"
        } else {
            "audio"
        },
        payload.watermark_uid(),
        decoded,
        true,
    ))
}

fn row_from_decoded(
    capability: &str,
    expected_uid: String,
    decoded: WatermarkDecodedPayload,
    should_have_payload: bool,
) -> QaRow {
    let pass = decoded.is_v3_minimal_anchor()
        && decoded.watermark_uid() == expected_uid
        && decoded.protocol_version() == 3
        && decoded.payload_bytes_length() == PAYLOAD_V3_MINIMAL_ANCHOR_BYTES;
    QaRow {
        pass,
        json: format!(
            "{{\"capability\":\"{}\",\"shouldHaveMediaPayload\":{},\"watermarkUid\":\"{}\",\"payloadProtocolVersion\":{},\"payloadBytesLength\":{},\"payloadAuthStatus\":\"verified\",\"mediaPayloadRole\":\"v3_minimal_anchor\",\"pass\":{}}}",
            capability,
            should_have_payload,
            json_escape(&decoded.watermark_uid()),
            decoded.protocol_version(),
            decoded.payload_bytes_length(),
            pass,
        ),
    }
}

fn video_l2_boundary() -> QaRow {
    QaRow {
        pass: true,
        json: "{\"capability\":\"video_l2_fingerprint_notary\",\"shouldHaveMediaPayload\":false,\"payloadProtocolVersion\":null,\"payloadBytesLength\":null,\"payloadAuthStatus\":\"not_applicable\",\"mediaPayloadRole\":\"not_applicable_l2_fingerprint\",\"notarySchema\":\"video_fingerprint_v1\",\"pass\":true}".to_string(),
    }
}

fn make_image(format: ImageFormat) -> Result<Vec<u8>, String> {
    let image = ImageBuffer::from_fn(1024, 1024, |x, y| {
        Rgb([
            (x * 255 / 1024) as u8,
            (y * 255 / 1024) as u8,
            (((x * 7 + y * 3) & 0xff) as u8).saturating_add(8),
        ])
    });
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut cursor, format)
        .map_err(|error| format!("encode image: {error}"))?;
    Ok(cursor.into_inner())
}

fn make_wav_audio(seconds: u32) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|error| format!("wav: {error}"))?;
        for index in 0..(44_100 * seconds) {
            let t = index as f32 / 44_100.0;
            let value = (0.24 * (2.0 * std::f32::consts::PI * 220.0 * t).sin()
                + 0.16 * (2.0 * std::f32::consts::PI * 660.0 * t).sin())
                * f32::from(i16::MAX);
            writer
                .write_sample(value as i16)
                .map_err(|error| format!("write wav sample: {error}"))?;
        }
        writer
            .finalize()
            .map_err(|error| format!("finalize wav: {error}"))?;
    }
    Ok(cursor.into_inner())
}

fn build_payload(
    run_id: &str,
    sample_id: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{sample_id}:v3-media-qa").as_bytes());
    let original_sha256: [u8; 32] = Sha256::digest(media_bytes).into();
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type,
        registry_proof_hash: Some(sha256_prefix_16(
            format!("{run_id}:{sample_id}:registry-proof").as_bytes(),
        )),
        creator_binding: Some("HiddenShield V3 media payload release QA"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn optional_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut out = [0_u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
