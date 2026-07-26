use image::{ImageBuffer, ImageFormat, Rgb};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use watermark_core::{
    audio_v3_quality_diagnostics, compare_audio_quality, compare_image_quality, AIContentFlags,
    AudioPerceptualDiagnosis, AudioProtectionMode, AudioQualityInput, AudioV3QualityDiagnostics,
    EmbedOptions, ImageOutputFormat, ImageQualityInput, MediaInput, MediaOutput,
    PayloadV2BuildInput, WatermarkDecodedPayload, WatermarkIssueMode, WatermarkMediaType,
    WatermarkPayload, AUDIO_FORENSIC_MAX_LUFS_DELTA, AUDIO_FORENSIC_MAX_PEAK_DELTA,
    AUDIO_FORENSIC_MIN_SNR, AUDIO_MAX_NEW_CLIPPING, AUDIO_RELEASE_MAX_LUFS_DELTA,
    AUDIO_RELEASE_MAX_PEAK_DELTA, AUDIO_RELEASE_MIN_SNR, IMAGE_FORENSIC_MIN_PSNR,
    IMAGE_FORENSIC_MIN_SSIM, IMAGE_RELEASE_MIN_PSNR, IMAGE_RELEASE_MIN_SSIM,
    PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const IMAGE_MAX_ROUNDTRIP_MS: u128 = 25_000;
const AUDIO_SAMPLE_RATE: usize = 44_100;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let run_id = optional_arg(&args, "--run-id").unwrap_or_else(|| unix_seconds().to_string());
    let scope = GateScope::from_arg(optional_arg(&args, "--scope").as_deref())?;
    let out_dir = PathBuf::from(
        optional_arg(&args, "--out-dir")
            .unwrap_or_else(|| format!("{}/run-{run_id}", scope.default_output_root())),
    );
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;
    fs::create_dir_all(out_dir.join("images"))
        .map_err(|error| format!("create images dir: {error}"))?;
    fs::create_dir_all(out_dir.join("audio"))
        .map_err(|error| format!("create audio dir: {error}"))?;
    fs::create_dir_all(out_dir.join("abx")).map_err(|error| format!("create abx dir: {error}"))?;

    let image_rows = image_samples()
        .into_iter()
        .chain(full_image_samples(scope))
        .map(|sample| run_image_sample(&run_id, &out_dir, sample, scope))
        .collect::<Result<Vec<_>, _>>()?;
    let audio_rows = audio_samples()
        .into_iter()
        .chain(full_audio_samples(scope))
        .map(|sample| run_audio_sample(&run_id, &out_dir, sample, false, scope))
        .collect::<Result<Vec<_>, _>>()?;
    let video_audio_rows = video_audio_track_samples()
        .into_iter()
        .chain(full_video_audio_track_samples(scope))
        .map(|sample| run_audio_sample(&run_id, &out_dir, sample, true, scope))
        .collect::<Result<Vec<_>, _>>()?;
    write_abx_templates(&out_dir, &image_rows, &audio_rows, &video_audio_rows)?;
    let pass = image_rows.iter().all(|row| row.passed)
        && audio_rows.iter().all(|row| row.passed)
        && video_audio_rows.iter().all(|row| row.passed);

    let json = format!(
        "{{\n  \"runId\": \"{}\",\n  \"gate\": \"{}\",\n  \"scope\": \"{}\",\n  \"payloadProtocolVersion\": 3,\n  \"payloadBytesLength\": {},\n  \"pass\": {},\n  \"thresholds\": {},\n  \"images\": [{}],\n  \"audio\": [{}],\n  \"videoAudioTrack\": [{}],\n  \"videoFingerprintNotary\": {},\n  \"videoVisualStaged\": {},\n  \"abxTemplates\": {}\n}}\n",
        json_escape(&run_id),
        scope.gate_name(),
        scope.scope_name(),
        PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        pass,
        thresholds_json(scope),
        image_rows
            .iter()
            .map(|row| row.json.clone())
            .collect::<Vec<_>>()
            .join(","),
        audio_rows
            .iter()
            .map(|row| row.json.clone())
            .collect::<Vec<_>>()
            .join(","),
        video_audio_rows
            .iter()
            .map(|row| row.json.clone())
            .collect::<Vec<_>>()
            .join(","),
        video_fingerprint_notary_json(),
        video_visual_json(scope),
        abx_templates_json(),
    );
    fs::write(out_dir.join(scope.json_file_name()), &json)
        .map_err(|error| format!("write release json: {error}"))?;
    fs::write(
        out_dir.join(scope.markdown_file_name()),
        render_markdown(
            &run_id,
            pass,
            &image_rows,
            &audio_rows,
            &video_audio_rows,
            scope,
        ),
    )
    .map_err(|error| format!("write release markdown: {error}"))?;
    print!("{json}");

    if pass {
        Ok(())
    } else {
        Err(format!("{} failed", scope.gate_name()))
    }
}

#[derive(Clone)]
struct ImageSample {
    id: &'static str,
    profile: &'static str,
    bytes: Vec<u8>,
}

#[derive(Clone)]
struct AudioSample {
    id: &'static str,
    profile: &'static str,
    samples: Vec<f32>,
}

struct GateRow {
    json: String,
    markdown: String,
    debug_markdown: Option<String>,
    audio_analysis_markdown: Option<String>,
    passed: bool,
    media_type: &'static str,
    sample_id: &'static str,
    profile: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GateScope {
    ReleaseSmoke,
    Full,
}

impl GateScope {
    fn from_arg(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("release") {
            "release" | "release-smoke" | "smoke" => Ok(Self::ReleaseSmoke),
            "full" | "perceptual-full" => Ok(Self::Full),
            other => Err(format!(
                "unsupported quality gate scope {other:?}; expected release or full"
            )),
        }
    }

    fn gate_name(self) -> &'static str {
        match self {
            Self::ReleaseSmoke => "watermark:quality-gate:release",
            Self::Full => "watermark:quality-gate:full",
        }
    }

    fn scope_name(self) -> &'static str {
        match self {
            Self::ReleaseSmoke => "non_external_dependency_release_smoke",
            Self::Full => "perceptual_full_gate",
        }
    }

    fn default_output_root(self) -> &'static str {
        match self {
            Self::ReleaseSmoke => "watermark-core/target/perceptual-quality-release-gate",
            Self::Full => "watermark-core/target/perceptual-quality-full-gate",
        }
    }

    fn json_file_name(self) -> &'static str {
        match self {
            Self::ReleaseSmoke => "quality-release-gate.json",
            Self::Full => "quality-full-gate.json",
        }
    }

    fn markdown_file_name(self) -> &'static str {
        match self {
            Self::ReleaseSmoke => "quality-release-gate.md",
            Self::Full => "quality-full-gate.md",
        }
    }

    fn image_min_psnr(self) -> f64 {
        match self {
            Self::ReleaseSmoke => IMAGE_RELEASE_MIN_PSNR,
            Self::Full => IMAGE_FORENSIC_MIN_PSNR,
        }
    }

    fn image_min_ssim(self) -> f64 {
        match self {
            Self::ReleaseSmoke => IMAGE_RELEASE_MIN_SSIM,
            Self::Full => IMAGE_FORENSIC_MIN_SSIM,
        }
    }

    fn audio_min_snr(self) -> f64 {
        match self {
            Self::ReleaseSmoke => AUDIO_RELEASE_MIN_SNR,
            Self::Full => AUDIO_FORENSIC_MIN_SNR,
        }
    }

    fn audio_max_peak_delta(self) -> f64 {
        match self {
            Self::ReleaseSmoke => AUDIO_RELEASE_MAX_PEAK_DELTA,
            Self::Full => AUDIO_FORENSIC_MAX_PEAK_DELTA,
        }
    }

    fn audio_max_lufs_delta(self) -> f64 {
        match self {
            Self::ReleaseSmoke => AUDIO_RELEASE_MAX_LUFS_DELTA,
            Self::Full => AUDIO_FORENSIC_MAX_LUFS_DELTA,
        }
    }
}

fn run_image_sample(
    run_id: &str,
    out_dir: &Path,
    sample: ImageSample,
    scope: GateScope,
) -> Result<GateRow, String> {
    let payload = build_payload(run_id, sample.id, &sample.bytes, WatermarkMediaType::Image)?;
    let started = Instant::now();
    let output = watermark_core::WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: sample.bytes.clone(),
        },
        &payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed image {}: {error}", sample.id))?;
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err(format!("image {} returned non-image output", sample.id));
    };
    let embed_ms = started.elapsed().as_millis();
    let extract_started = Instant::now();
    let extracted = watermark_core::WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract image {}: {error}", sample.id))?;
    let extract_ms = extract_started.elapsed().as_millis();
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = extracted else {
        return Err(format!(
            "image {} did not decode V3 minimal anchor",
            sample.id
        ));
    };
    let sample_dir = out_dir.join("images").join(sample.id);
    fs::create_dir_all(&sample_dir).map_err(|error| format!("create sample dir: {error}"))?;
    fs::write(sample_dir.join("control.png"), &sample.bytes)
        .map_err(|error| format!("write image control: {error}"))?;
    fs::write(sample_dir.join("protected.png"), &bytes)
        .map_err(|error| format!("write image protected: {error}"))?;
    let control = image::load_from_memory(&sample.bytes)
        .map_err(|error| format!("open image control: {error}"))?
        .to_rgb8();
    let protected = image::load_from_memory(&bytes)
        .map_err(|error| format!("open image protected: {error}"))?
        .to_rgb8();
    let quality = compare_image_quality(ImageQualityInput {
        source: &control,
        candidate: &protected,
    })?;
    let psnr = quality.psnr;
    let ssim = quality.ssim;
    let roundtrip_ms = embed_ms + extract_ms;
    let extract_passed = anchor.watermark_uid() == payload.watermark_uid();
    let passed = extract_passed
        && psnr >= scope.image_min_psnr()
        && ssim >= scope.image_min_ssim()
        && roundtrip_ms <= IMAGE_MAX_ROUNDTRIP_MS;
    let reason = if !extract_passed {
        "extract_failed"
    } else if psnr < scope.image_min_psnr() {
        "psnr_below_threshold"
    } else if ssim < scope.image_min_ssim() {
        "ssim_below_threshold"
    } else if roundtrip_ms > IMAGE_MAX_ROUNDTRIP_MS {
        "roundtrip_above_threshold"
    } else {
        "none"
    };
    Ok(GateRow {
        passed,
        json: format!(
            "{{\"sampleId\":\"{}\",\"mediaType\":\"image\",\"sourceProfile\":\"{}\",\"watermarkUid\":\"{}\",\"payloadProtocolVersion\":3,\"payloadBytesLength\":{},\"extractPassed\":{},\"qualityPassed\":{},\"releaseBlockingReason\":\"{}\",\"metrics\":{{\"psnr\":{:.4},\"minPsnr\":{:.4},\"ssim\":{:.6},\"minSsim\":{:.6},\"embedMs\":{},\"extractMs\":{},\"roundtripMs\":{},\"maxRoundtripMs\":{}}}}}",
            sample.id,
            sample.profile,
            json_escape(&payload.watermark_uid()),
            PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
            extract_passed,
            passed,
            reason,
            psnr,
            scope.image_min_psnr(),
            ssim,
            scope.image_min_ssim(),
            embed_ms,
            extract_ms,
            roundtrip_ms,
            IMAGE_MAX_ROUNDTRIP_MS,
        ),
        markdown: format!(
            "| image | {} | {} | {:.2} | {:.5} | {} | {} |",
            sample.id,
            sample.profile,
            psnr,
            ssim,
            roundtrip_ms,
            if passed { "PASS" } else { reason },
        ),
        debug_markdown: None,
        audio_analysis_markdown: None,
        media_type: "image",
        sample_id: sample.id,
        profile: sample.profile,
    })
}

fn run_audio_sample(
    run_id: &str,
    out_dir: &Path,
    sample: AudioSample,
    video_track: bool,
    scope: GateScope,
) -> Result<GateRow, String> {
    let source_wav = encode_wav(&sample.samples)?;
    let media_type = if video_track {
        WatermarkMediaType::VideoAudioTrack
    } else {
        WatermarkMediaType::Audio
    };
    let payload = build_payload(run_id, sample.id, &source_wav, media_type)?;
    let started = Instant::now();
    let output = watermark_core::WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_wav.clone(),
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
    .map_err(|error| format!("embed audio {}: {error}", sample.id))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err(format!("audio {} returned non-audio output", sample.id));
    };
    let embed_ms = started.elapsed().as_millis();
    let extract_started = Instant::now();
    let extracted = watermark_core::WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract audio {}: {error}", sample.id))?;
    let extract_ms = extract_started.elapsed().as_millis();
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = extracted else {
        return Err(format!(
            "audio {} did not decode V3 minimal anchor",
            sample.id
        ));
    };
    let media_dir = if video_track {
        "video_audio_track"
    } else {
        "audio"
    };
    let sample_dir = out_dir.join(media_dir).join(sample.id);
    fs::create_dir_all(&sample_dir).map_err(|error| format!("create audio sample dir: {error}"))?;
    fs::write(sample_dir.join("control.wav"), &source_wav)
        .map_err(|error| format!("write audio control: {error}"))?;
    fs::write(sample_dir.join("protected.wav"), &bytes)
        .map_err(|error| format!("write audio protected: {error}"))?;
    let protected_samples = wav_samples(&bytes)?;
    let diagnostics = audio_v3_quality_diagnostics(&sample.samples, &protected_samples, &anchor)
        .map_err(|error| format!("audio diagnostics {}: {error}", sample.id))?;
    let quality = compare_audio_quality(AudioQualityInput {
        source: &sample.samples,
        candidate: &protected_samples,
        sample_rate: AUDIO_SAMPLE_RATE,
        channels: 1,
    })?;
    let perceptual_analysis = &quality.perceptual_diagnosis;
    let snr = quality.snr;
    let peak_delta = quality.peak_delta;
    let lufs_delta = quality.lufs_delta;
    let new_clipping = quality.new_clipping;
    let extract_passed = anchor.watermark_uid() == payload.watermark_uid();
    let passed = extract_passed
        && snr >= scope.audio_min_snr()
        && peak_delta <= scope.audio_max_peak_delta()
        && lufs_delta <= scope.audio_max_lufs_delta()
        && new_clipping <= AUDIO_MAX_NEW_CLIPPING;
    let reason = if !extract_passed {
        "extract_failed"
    } else if snr < scope.audio_min_snr() {
        "snr_below_threshold"
    } else if lufs_delta > scope.audio_max_lufs_delta() {
        "lufs_delta_above_threshold"
    } else if peak_delta > scope.audio_max_peak_delta() {
        "peak_delta_above_threshold"
    } else if new_clipping > AUDIO_MAX_NEW_CLIPPING {
        "new_clipping"
    } else {
        "none"
    };
    let media_type_label = if video_track {
        "video_audio_track"
    } else {
        "audio"
    };
    Ok(GateRow {
        passed,
        json: format!(
            "{{\"sampleId\":\"{}\",\"mediaType\":\"{}\",\"sourceProfile\":\"{}\",\"watermarkUid\":\"{}\",\"payloadProtocolVersion\":3,\"payloadBytesLength\":{},\"extractPassed\":{},\"qualityPassed\":{},\"releaseBlockingReason\":\"{}\",\"metrics\":{{\"snr\":{:.4},\"minSnr\":{:.4},\"peakDelta\":{:.6},\"maxPeakDelta\":{:.6},\"lufsDelta\":{:.6},\"maxLufsDelta\":{:.6},\"newClipping\":{},\"maxNewClipping\":{},\"embedMs\":{},\"extractMs\":{},\"debug\":{},\"perceptualDiagnosis\":{}}}}}",
            sample.id,
            media_type_label,
            sample.profile,
            json_escape(&payload.watermark_uid()),
            PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
            extract_passed,
            passed,
            reason,
            snr,
            scope.audio_min_snr(),
            peak_delta,
            scope.audio_max_peak_delta(),
            lufs_delta,
            scope.audio_max_lufs_delta(),
            new_clipping,
            AUDIO_MAX_NEW_CLIPPING,
            embed_ms,
            extract_ms,
            audio_diagnostics_json(&diagnostics),
            audio_perceptual_diagnosis_json(perceptual_analysis),
        ),
        markdown: format!(
            "| {} | {} | {} | {:.2} | {:.4} | {:.4} | {} |",
            media_type_label,
            sample.id,
            sample.profile,
            snr,
            lufs_delta,
            peak_delta,
            if passed { "PASS" } else { reason },
        ),
        debug_markdown: Some(audio_diagnostics_markdown(
            media_type_label,
            sample.id,
            &diagnostics,
        )),
        audio_analysis_markdown: Some(audio_perceptual_diagnosis_markdown(
            media_type_label,
            sample.id,
            perceptual_analysis,
        )),
        media_type: media_type_label,
        sample_id: sample.id,
        profile: sample.profile,
    })
}

fn image_samples() -> Vec<ImageSample> {
    vec![
        ImageSample {
            id: "photo-gradient",
            profile: "photo_natural_gradient",
            bytes: make_png_image(1024, 1024, image_pixel_photo).expect("photo image"),
        },
        ImageSample {
            id: "low-texture",
            profile: "low_texture_gradient",
            bytes: make_png_image(1024, 1024, image_pixel_low_texture).expect("low texture image"),
        },
        ImageSample {
            id: "poster-lines",
            profile: "illustration_poster_edges",
            bytes: make_png_image(1024, 1024, image_pixel_poster).expect("poster image"),
        },
        ImageSample {
            id: "ui-text",
            profile: "ui_text_screenshot",
            bytes: make_png_image(1024, 1024, image_pixel_ui).expect("ui image"),
        },
    ]
}

fn full_image_samples(scope: GateScope) -> Vec<ImageSample> {
    if scope != GateScope::Full {
        return Vec::new();
    }
    vec![
        ImageSample {
            id: "portrait-skin",
            profile: "portrait_skin_gradient",
            bytes: make_png_image(1280, 960, image_pixel_portrait).expect("portrait image"),
        },
        ImageSample {
            id: "dark-high-iso",
            profile: "dark_high_iso_noise",
            bytes: make_png_image(1280, 960, image_pixel_dark_noise).expect("dark image"),
        },
        ImageSample {
            id: "fine-detail",
            profile: "fine_detail_texture",
            bytes: make_png_image(1280, 960, image_pixel_fine_detail).expect("detail image"),
        },
        ImageSample {
            id: "small-boundary",
            profile: "minimum_capacity_boundary",
            bytes: make_png_image(512, 512, image_pixel_low_texture).expect("small image"),
        },
    ]
}

fn audio_samples() -> Vec<AudioSample> {
    vec![
        AudioSample {
            id: "voice",
            profile: "voice_podcast",
            samples: make_voice_samples(30),
        },
        AudioSample {
            id: "music",
            profile: "music_wideband",
            samples: make_music_samples(30),
        },
        AudioSample {
            id: "quiet",
            profile: "quiet_sparse",
            samples: make_quiet_samples(30),
        },
    ]
}

fn full_audio_samples(scope: GateScope) -> Vec<AudioSample> {
    if scope != GateScope::Full {
        return Vec::new();
    }
    vec![
        AudioSample {
            id: "field-noise",
            profile: "field_recording_noise_floor",
            samples: make_field_noise_samples(30),
        },
        AudioSample {
            id: "transient",
            profile: "transient_percussion",
            samples: make_transient_samples(30),
        },
        AudioSample {
            id: "speech-music-mix",
            profile: "speech_music_mix",
            samples: make_speech_music_mix_samples(30),
        },
    ]
}

fn video_audio_track_samples() -> Vec<AudioSample> {
    vec![AudioSample {
        id: "l1-video-audio-track",
        profile: "l1_video_audio_track_short",
        samples: make_music_samples(12),
    }]
}

fn full_video_audio_track_samples(scope: GateScope) -> Vec<AudioSample> {
    if scope != GateScope::Full {
        return Vec::new();
    }
    vec![
        AudioSample {
            id: "l1-video-voice-track",
            profile: "l1_video_voice_track",
            samples: make_voice_samples(12),
        },
        AudioSample {
            id: "l1-video-mixed-track",
            profile: "l1_video_mixed_track",
            samples: make_speech_music_mix_samples(12),
        },
    ]
}

fn make_png_image(
    width: u32,
    height: u32,
    pixel_fn: fn(u32, u32, u32, u32) -> Rgb<u8>,
) -> Result<Vec<u8>, String> {
    let image = ImageBuffer::from_fn(width, height, |x, y| pixel_fn(x, y, width, height));
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("encode png: {error}"))?;
    Ok(cursor.into_inner())
}

fn image_pixel_photo(x: u32, y: u32, width: u32, height: u32) -> Rgb<u8> {
    Rgb([
        (x * 255 / width) as u8,
        (y * 255 / height) as u8,
        (((x * 3 + y * 5) & 0xff) as u8).saturating_add(12),
    ])
}

fn image_pixel_low_texture(x: u32, y: u32, width: u32, height: u32) -> Rgb<u8> {
    let base = 112 + ((x * 18 / width + y * 14 / height) as u8);
    Rgb([base, base.saturating_add(4), base.saturating_add(8)])
}

fn image_pixel_poster(x: u32, y: u32, _width: u32, _height: u32) -> Rgb<u8> {
    if x % 96 < 4 || y % 96 < 4 {
        Rgb([24, 28, 32])
    } else if (x / 96 + y / 96) % 2 == 0 {
        Rgb([224, 68, 54])
    } else {
        Rgb([248, 204, 80])
    }
}

fn image_pixel_ui(x: u32, y: u32, _width: u32, _height: u32) -> Rgb<u8> {
    let in_text_line = (y % 72 > 18 && y % 72 < 28) && (x % 240 > 24 && x % 240 < 190);
    let in_rule = y % 96 == 0 || x % 320 == 0;
    if in_text_line || in_rule {
        Rgb([36, 42, 48])
    } else {
        Rgb([244, 246, 248])
    }
}

fn image_pixel_portrait(x: u32, y: u32, width: u32, height: u32) -> Rgb<u8> {
    let nx = x as f32 / width as f32;
    let ny = y as f32 / height as f32;
    let cheek = ((1.0 - ((nx - 0.52).powi(2) + (ny - 0.48).powi(2)) * 3.0).max(0.0) * 32.0) as u8;
    Rgb([
        176_u8
            .saturating_add(cheek)
            .saturating_add((ny * 18.0) as u8),
        126_u8.saturating_add((nx * 22.0) as u8),
        104_u8.saturating_add(((1.0 - ny) * 16.0) as u8),
    ])
}

fn image_pixel_dark_noise(x: u32, y: u32, _width: u32, _height: u32) -> Rgb<u8> {
    let noise = (((x * 17) ^ (y * 31) ^ (x * y)) & 0x1f) as u8;
    let base = 22_u8.saturating_add(noise / 2);
    Rgb([base, base.saturating_add(4), base.saturating_add(10)])
}

fn image_pixel_fine_detail(x: u32, y: u32, _width: u32, _height: u32) -> Rgb<u8> {
    let weave = (((x / 3 + y / 5) % 2) * 34) as u8;
    let diagonal = (((x + y) % 64) * 2) as u8;
    Rgb([
        80_u8.saturating_add(weave),
        120_u8.saturating_add(diagonal / 3),
        150_u8.saturating_add(weave / 2),
    ])
}

fn make_voice_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        let envelope = 0.45 + 0.35 * (2.0 * std::f32::consts::PI * 3.0 * t).sin().abs();
        envelope
            * (0.28 * (2.0 * std::f32::consts::PI * 190.0 * t).sin()
                + 0.08 * (2.0 * std::f32::consts::PI * 760.0 * t).sin())
    })
}

fn make_music_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        0.24 * (2.0 * std::f32::consts::PI * 220.0 * t).sin()
            + 0.18 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            + 0.11 * (2.0 * std::f32::consts::PI * 880.0 * t).sin()
    })
}

fn make_quiet_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        0.055 * (2.0 * std::f32::consts::PI * 330.0 * t).sin()
            + 0.018 * (2.0 * std::f32::consts::PI * 37.0 * t).sin()
    })
}

fn make_field_noise_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        let deterministic_noise = ((t * 44_100.0) as u32)
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let noise = ((deterministic_noise >> 8) & 0xffff) as f32 / 32768.0 - 1.0;
        0.11 * noise + 0.08 * (2.0 * std::f32::consts::PI * 130.0 * t).sin()
    })
}

fn make_transient_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        let beat_phase = (t * 2.0).fract();
        let click = if beat_phase < 0.018 {
            (1.0 - beat_phase / 0.018) * 0.42
        } else {
            0.0
        };
        click + 0.16 * (2.0 * std::f32::consts::PI * 510.0 * t).sin()
    })
}

fn make_speech_music_mix_samples(seconds: u32) -> Vec<f32> {
    samples_for_seconds(seconds, |t| {
        0.18 * (2.0 * std::f32::consts::PI * 210.0 * t).sin()
            + 0.11 * (2.0 * std::f32::consts::PI * 420.0 * t).sin()
            + 0.08
                * (2.0 * std::f32::consts::PI * 720.0 * t).sin()
                * (0.5 + 0.5 * (2.0 * std::f32::consts::PI * 2.7 * t).sin().abs())
    })
}

fn samples_for_seconds(seconds: u32, sample_fn: fn(f32) -> f32) -> Vec<f32> {
    let sample_rate = 44_100usize;
    let total = sample_rate * seconds as usize;
    (0..total)
        .map(|index| sample_fn(index as f32 / sample_rate as f32).clamp(-0.95, 0.95))
        .collect()
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
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

fn wav_samples(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes))
        .map_err(|error| format!("open wav samples: {error}"))?;
    reader
        .samples::<i16>()
        .map(|sample| sample.map(|value| f32::from(value) / f32::from(i16::MAX)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read wav sample: {error}"))
}

fn audio_diagnostics_json(diagnostics: &AudioV3QualityDiagnostics) -> String {
    format!(
        "{{\"frameCount\":{},\"shortTimeRms\":{{\"min\":{:.6},\"mean\":{:.6},\"max\":{:.6}}},\"lowEnergyFrameRatio\":{:.6},\"transientFrameRatio\":{:.6},\"noiseLikeFrameRatio\":{:.6},\"embeddingStrength\":{{\"minContrast\":{:.6},\"meanContrast\":{:.6},\"maxContrast\":{:.6},\"modifiedPairRatio\":{:.6}}},\"noiseFloorSparseRecovery\":{},\"extractionConfidence\":{:.6}}}",
        diagnostics.frame_count,
        diagnostics.short_time_rms_min,
        diagnostics.short_time_rms_mean,
        diagnostics.short_time_rms_max,
        diagnostics.low_energy_frame_ratio,
        diagnostics.transient_frame_ratio,
        diagnostics.noise_like_frame_ratio,
        diagnostics.embedding_strength_min,
        diagnostics.embedding_strength_mean,
        diagnostics.embedding_strength_max,
        diagnostics.modified_pair_ratio,
        diagnostics.noise_floor_sparse_recovery,
        diagnostics.extraction_confidence,
    )
}

fn audio_diagnostics_markdown(
    media_type: &str,
    sample_id: &str,
    diagnostics: &AudioV3QualityDiagnostics,
) -> String {
    format!(
        "| {} | {} | {} | {:.6} / {:.6} / {:.6} | {:.3} | {:.3} | {:.3} | {:.6} / {:.6} / {:.6} | {:.3} | {} | {:.3} |",
        media_type,
        sample_id,
        diagnostics.frame_count,
        diagnostics.short_time_rms_min,
        diagnostics.short_time_rms_mean,
        diagnostics.short_time_rms_max,
        diagnostics.low_energy_frame_ratio,
        diagnostics.transient_frame_ratio,
        diagnostics.noise_like_frame_ratio,
        diagnostics.embedding_strength_min,
        diagnostics.embedding_strength_mean,
        diagnostics.embedding_strength_max,
        diagnostics.modified_pair_ratio,
        diagnostics.noise_floor_sparse_recovery,
        diagnostics.extraction_confidence,
    )
}

fn audio_perceptual_diagnosis_json(analysis: &AudioPerceptualDiagnosis) -> String {
    format!(
        "{{\"segmentedSnr\":{{\"segmentSeconds\":{},\"segmentCount\":{},\"min\":{:.4},\"mean\":{:.4},\"max\":{:.4},\"first\":{:.4},\"middle\":{:.4},\"last\":{:.4},\"spread\":{:.4}}},\"bandEnergyShare\":{{\"low\":{{\"signal\":{:.6},\"noise\":{:.6}}},\"watermark\":{{\"signal\":{:.6},\"noise\":{:.6}}},\"high\":{{\"signal\":{:.6},\"noise\":{:.6}}}}},\"dominantNoiseBand\":\"{}\",\"diagnosis\":\"{}\"}}",
        analysis.segmented_snr.segment_seconds,
        analysis.segmented_snr.segment_count,
        analysis.segmented_snr.min,
        analysis.segmented_snr.mean,
        analysis.segmented_snr.max,
        analysis.segmented_snr.first,
        analysis.segmented_snr.middle,
        analysis.segmented_snr.last,
        analysis.segmented_snr.spread,
        analysis.band_energy.low_signal_share,
        analysis.band_energy.low_noise_share,
        analysis.band_energy.watermark_signal_share,
        analysis.band_energy.watermark_noise_share,
        analysis.band_energy.high_signal_share,
        analysis.band_energy.high_noise_share,
        analysis.band_energy.dominant_noise_band,
        analysis.diagnosis,
    )
}

fn audio_perceptual_diagnosis_markdown(
    media_type: &str,
    sample_id: &str,
    analysis: &AudioPerceptualDiagnosis,
) -> String {
    format!(
        "| {} | {} | {} | {:.2} / {:.2} / {:.2} | {:.2} | {:.3} / {:.3} | {:.3} / {:.3} | {:.3} / {:.3} | {} | {} |",
        media_type,
        sample_id,
        analysis.segmented_snr.segment_count,
        analysis.segmented_snr.min,
        analysis.segmented_snr.mean,
        analysis.segmented_snr.max,
        analysis.segmented_snr.spread,
        analysis.band_energy.low_signal_share,
        analysis.band_energy.low_noise_share,
        analysis.band_energy.watermark_signal_share,
        analysis.band_energy.watermark_noise_share,
        analysis.band_energy.high_signal_share,
        analysis.band_energy.high_noise_share,
        analysis.band_energy.dominant_noise_band,
        analysis.diagnosis,
    )
}

fn build_payload(
    run_id: &str,
    sample_id: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{sample_id}:v3-release").as_bytes());
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
        creator_binding: Some("HiddenShield V3 release quality gate"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn thresholds_json(scope: GateScope) -> String {
    format!(
        "{{\"image\":{{\"minPsnr\":{},\"minSsim\":{},\"maxRoundtripMs\":{IMAGE_MAX_ROUNDTRIP_MS}}},\"audio\":{{\"minSnr\":{},\"maxPeakDelta\":{},\"maxLufsDelta\":{},\"maxNewClipping\":{AUDIO_MAX_NEW_CLIPPING}}}}}",
        scope.image_min_psnr(),
        scope.image_min_ssim(),
        scope.audio_min_snr(),
        scope.audio_max_peak_delta(),
        scope.audio_max_lufs_delta(),
    )
}

fn write_abx_templates(
    out_dir: &Path,
    images: &[GateRow],
    audio: &[GateRow],
    video_audio: &[GateRow],
) -> Result<(), String> {
    let abx_dir = out_dir.join("abx");
    write_abx_template(
        &abx_dir.join("image_abx_trials.csv"),
        images,
        "monitor / phone",
        "normal-distance / zoom-diagnostic",
    )?;
    write_abx_template(
        &abx_dir.join("audio_abx_trials.csv"),
        audio,
        "headphones / speaker",
        "quiet-room / office",
    )?;
    write_abx_template(
        &abx_dir.join("video_audio_track_abx_trials.csv"),
        video_audio,
        "headphones / speaker",
        "quiet-room / office",
    )?;
    fs::write(
        abx_dir.join("video_visual_staged_abx_trials.csv"),
        "runId,mediaType,sampleId,profile,device,environment,trial,A,B,X,answer,correct,confidence_1_5,perceivedDifference,notes\n",
    )
    .map_err(|error| format!("write video visual ABX template: {error}"))?;
    Ok(())
}

fn write_abx_template(
    path: &Path,
    rows: &[GateRow],
    device: &str,
    environment: &str,
) -> Result<(), String> {
    let mut csv = String::from(
        "runId,mediaType,sampleId,profile,device,environment,trial,A,B,X,answer,correct,confidence_1_5,perceivedDifference,notes\n",
    );
    for row in rows {
        for trial in 1..=3 {
            let (a, b, x) = if trial % 2 == 0 {
                ("protected", "original", "B")
            } else {
                ("original", "protected", "A")
            };
            csv.push_str(&format!(
                ",{},{},{},{},{},{},{},{},{},,,,,\n",
                row.media_type, row.sample_id, row.profile, device, environment, trial, a, b, x
            ));
        }
    }
    fs::write(path, csv).map_err(|error| format!("write ABX template {}: {error}", path.display()))
}

fn abx_templates_json() -> String {
    "{\"image\":\"abx/image_abx_trials.csv\",\"audio\":\"abx/audio_abx_trials.csv\",\"videoAudioTrack\":\"abx/video_audio_track_abx_trials.csv\",\"videoVisualStaged\":\"abx/video_visual_staged_abx_trials.csv\"}".to_string()
}

fn video_visual_json(scope: GateScope) -> String {
    match scope {
        GateScope::ReleaseSmoke => "{\"status\":\"delegated_internal_staged\",\"gate\":\"watermark:l3-video-visual-release-gate\",\"futureFullGateMetrics\":[\"vmaf_delta\",\"video_psnr\",\"video_ssim\",\"video_visual_abx\"],\"abxTemplate\":\"abx/video_visual_staged_abx_trials.csv\",\"reason\":\"L3 video visual has a separate 24-sample FFmpeg release gate and is not evaluated by this non-external-dependency smoke\"}".to_string(),
        GateScope::Full => "{\"status\":\"internal_staged_only\",\"gate\":\"watermark:l3-video-visual-release-gate\",\"futureFullGateMetrics\":[\"vmaf_delta\",\"video_psnr\",\"video_ssim\",\"video_visual_abx\"],\"recommendedMetrics\":[\"vmaf_delta\",\"video_psnr\",\"video_ssim\",\"payload_consistency\",\"self_check_confidence\"],\"abxTemplate\":\"abx/video_visual_staged_abx_trials.csv\",\"reason\":\"L3 video visual perceptual quality requires FFmpeg/libvmaf and remains internal staged evidence, not a user-facing no-sense commitment\"}".to_string(),
    }
}

fn video_fingerprint_notary_json() -> String {
    "{\"status\":\"not_applicable_no_media_mutation\",\"capability\":\"video_l2_fingerprint_notary\",\"reason\":\"L2 computes an irreversible fingerprint/notary manifest and does not inject or mutate picture/audio media, so perceptual no-sense metrics such as PSNR, SNR, VMAF, and ABX do not apply\",\"recommendedGate\":\"watermark:video-phase-contract\"}".to_string()
}

fn render_markdown(
    run_id: &str,
    pass: bool,
    images: &[GateRow],
    audio: &[GateRow],
    video_audio: &[GateRow],
    scope: GateScope,
) -> String {
    let mut markdown = format!(
        "# HiddenShield V3 Perceptual Quality Gate\n\n- runId: `{}`\n- gate: `{}`\n- scope: `{}`\n- payload: `V3 / {} bytes`\n- pass: {}\n- videoFingerprintNotary: `not_applicable_no_media_mutation`\n- videoVisualStaged: `{}` via `watermark:l3-video-visual-release-gate`, future VMAF / ABX template `abx/video_visual_staged_abx_trials.csv`\n- ABX templates: `abx/*.csv`\n\n",
        run_id,
        scope.gate_name(),
        scope.scope_name(),
        PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        pass,
        if scope == GateScope::Full {
            "internal staged template only; not user-facing"
        } else {
            "delegated_internal_staged"
        },
    );
    markdown.push_str("| media | sample | profile | metric A | metric B | metric C | result |\n");
    markdown.push_str("| --- | --- | --- | ---: | ---: | ---: | --- |\n");
    for row in images.iter().chain(audio.iter()).chain(video_audio.iter()) {
        markdown.push_str(&row.markdown);
        markdown.push('\n');
    }
    let debug_rows = audio
        .iter()
        .chain(video_audio.iter())
        .filter_map(|row| row.debug_markdown.as_deref())
        .collect::<Vec<_>>();
    if !debug_rows.is_empty() {
        markdown.push_str("\n## Audio V3 Debug Metrics\n\n");
        markdown.push_str("| media | sample | frames | short RMS min/mean/max | low energy ratio | transient ratio | noise-like ratio | strength min/mean/max | modified pair ratio | noise-floor sparse | extraction confidence |\n");
        markdown.push_str(
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
        );
        for row in debug_rows {
            markdown.push_str(row);
            markdown.push('\n');
        }
    }
    let analysis_rows = audio
        .iter()
        .chain(video_audio.iter())
        .filter_map(|row| row.audio_analysis_markdown.as_deref())
        .collect::<Vec<_>>();
    if !analysis_rows.is_empty() {
        markdown.push_str("\n## Audio Perceptual Diagnosis\n\n");
        markdown.push_str("| media | sample | segments | segment SNR min/mean/max | segment spread | low band signal/noise | watermark band signal/noise | high band signal/noise | dominant noise band | diagnosis |\n");
        markdown.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |\n");
        for row in analysis_rows {
            markdown.push_str(row);
            markdown.push('\n');
        }
    }
    markdown
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
