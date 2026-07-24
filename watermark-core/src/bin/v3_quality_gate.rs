use image::{ImageBuffer, ImageFormat, Rgb};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use watermark_core::{
    AIContentFlags, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput,
    WatermarkDecodedPayload, WatermarkIssueMode, WatermarkMediaType, WatermarkPayload,
    WatermarkService, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const IMAGE_MIN_PSNR: f64 = 33.0;
const IMAGE_MIN_SSIM: f64 = 0.985;
const IMAGE_MAX_ROUNDTRIP_MS: u128 = 20_000;
const AUDIO_MIN_SNR: f64 = 35.0;
const AUDIO_MAX_PEAK_DELTA: f64 = 0.08;
const AUDIO_MAX_LUFS_DELTA: f64 = 1.5;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let run_id = optional_arg(&args, "--run-id").unwrap_or_else(|| unix_seconds().to_string());
    let out_dir =
        PathBuf::from(optional_arg(&args, "--out-dir").unwrap_or_else(|| {
            format!("watermark-core/target/perceptual-quality-gate/run-{run_id}")
        }));
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image = make_png_image()?;
    let audio = make_wav_audio()?;
    let image_row = run_image_gate(&run_id, &out_dir, &image)?;
    let audio_row = run_audio_gate(&run_id, &out_dir, &audio)?;
    let pass = image_row.passed && audio_row.passed;
    let json = format!(
        "{{\n  \"runId\": \"{}\",\n  \"payloadProtocolVersion\": 3,\n  \"payloadBytesLength\": {},\n  \"pass\": {},\n  \"image\": {},\n  \"audio\": {},\n  \"video\": {{\"status\":\"skipped\",\"reason\":\"VMAF requires ffmpeg/libvmaf sample pool and remains internal staged only\"}}\n}}\n",
        json_escape(&run_id),
        PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        pass,
        image_row.json,
        audio_row.json,
    );
    fs::write(out_dir.join("quality-gate.json"), &json)
        .map_err(|error| format!("write quality json: {error}"))?;
    fs::write(
        out_dir.join("quality-gate.md"),
        render_markdown(&run_id, &image_row, &audio_row),
    )
    .map_err(|error| format!("write quality markdown: {error}"))?;
    print!("{json}");
    if pass {
        Ok(())
    } else {
        Err("V3 quality/performance gate failed".to_string())
    }
}

struct GateRow {
    json: String,
    passed: bool,
}

fn run_image_gate(run_id: &str, out_dir: &Path, source: &[u8]) -> Result<GateRow, String> {
    let payload = build_payload(run_id, "image", source, WatermarkMediaType::Image)?;
    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source.to_vec(),
        },
        &payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed V3 image: {error}"))?;
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err("V3 image gate returned non-image output".to_string());
    };
    let embed_ms = started.elapsed().as_millis();
    let extract_started = Instant::now();
    let extracted = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract V3 image: {error}"))?;
    let extract_ms = extract_started.elapsed().as_millis();
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = extracted else {
        return Err("expected V3 minimal anchor from image gate".to_string());
    };
    fs::write(out_dir.join("image-source.png"), source)
        .map_err(|error| format!("write image source: {error}"))?;
    fs::write(out_dir.join("image-v3.png"), &bytes)
        .map_err(|error| format!("write image v3: {error}"))?;
    let source_img = image::load_from_memory(source)
        .map_err(|error| format!("open source image: {error}"))?
        .to_rgb8();
    let embedded_img = image::load_from_memory(&bytes)
        .map_err(|error| format!("open embedded image: {error}"))?
        .to_rgb8();
    let psnr = image_psnr(&source_img, &embedded_img)?;
    let ssim = image_ssim(&source_img, &embedded_img)?;
    let roundtrip_ms = embed_ms + extract_ms;
    let passed = anchor.watermark_uid() == payload.watermark_uid()
        && psnr >= IMAGE_MIN_PSNR
        && ssim >= IMAGE_MIN_SSIM
        && roundtrip_ms <= IMAGE_MAX_ROUNDTRIP_MS;
    Ok(GateRow {
        passed,
        json: format!(
            "{{\"passed\":{},\"watermarkUid\":\"{}\",\"extractPassed\":{},\"embedMs\":{},\"extractMs\":{},\"roundtripMs\":{},\"maxRoundtripMs\":{},\"psnr\":{:.4},\"minPsnr\":{:.4},\"ssim\":{:.6},\"minSsim\":{:.6}}}",
            passed,
            json_escape(&payload.watermark_uid()),
            anchor.watermark_uid() == payload.watermark_uid(),
            embed_ms,
            extract_ms,
            roundtrip_ms,
            IMAGE_MAX_ROUNDTRIP_MS,
            psnr,
            IMAGE_MIN_PSNR,
            ssim,
            IMAGE_MIN_SSIM,
        ),
    })
}

fn run_audio_gate(run_id: &str, out_dir: &Path, source: &[u8]) -> Result<GateRow, String> {
    let payload = build_payload(run_id, "audio", source, WatermarkMediaType::Audio)?;
    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source.to_vec(),
        },
        &payload,
        EmbedOptions {
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed V3 audio: {error}"))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("V3 audio gate returned non-audio output".to_string());
    };
    let embed_ms = started.elapsed().as_millis();
    let extract_started = Instant::now();
    let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract V3 audio: {error}"))?;
    let extract_ms = extract_started.elapsed().as_millis();
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = extracted else {
        return Err("expected V3 minimal anchor from audio gate".to_string());
    };
    fs::write(out_dir.join("audio-source.wav"), source)
        .map_err(|error| format!("write audio source: {error}"))?;
    fs::write(out_dir.join("audio-v3.wav"), &bytes)
        .map_err(|error| format!("write audio v3: {error}"))?;
    let source_samples = wav_samples(source)?;
    let embedded_samples = wav_samples(&bytes)?;
    let snr = audio_snr(&source_samples, &embedded_samples)?;
    let peak_delta = (peak_abs(&source_samples) - peak_abs(&embedded_samples)).abs();
    let lufs_delta = (approx_lufs(&source_samples) - approx_lufs(&embedded_samples)).abs();
    let passed = anchor.watermark_uid() == payload.watermark_uid()
        && snr >= AUDIO_MIN_SNR
        && peak_delta <= AUDIO_MAX_PEAK_DELTA
        && lufs_delta <= AUDIO_MAX_LUFS_DELTA;
    Ok(GateRow {
        passed,
        json: format!(
            "{{\"passed\":{},\"watermarkUid\":\"{}\",\"extractPassed\":{},\"embedMs\":{},\"extractMs\":{},\"snr\":{:.4},\"minSnr\":{:.4},\"peakDelta\":{:.6},\"maxPeakDelta\":{:.6},\"lufsDelta\":{:.6},\"maxLufsDelta\":{:.6}}}",
            passed,
            json_escape(&payload.watermark_uid()),
            anchor.watermark_uid() == payload.watermark_uid(),
            embed_ms,
            extract_ms,
            snr,
            AUDIO_MIN_SNR,
            peak_delta,
            AUDIO_MAX_PEAK_DELTA,
            lufs_delta,
            AUDIO_MAX_LUFS_DELTA,
        ),
    })
}

fn image_psnr(
    source: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    embedded: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<f64, String> {
    if source.dimensions() != embedded.dimensions() {
        return Err("image dimensions differ".to_string());
    }
    let mut mse = 0.0;
    let mut count = 0.0;
    for (left, right) in source.pixels().zip(embedded.pixels()) {
        for channel in 0..3 {
            let delta = f64::from(left[channel]) - f64::from(right[channel]);
            mse += delta * delta;
            count += 1.0;
        }
    }
    mse /= count;
    if mse == 0.0 {
        return Ok(99.0);
    }
    Ok(10.0 * ((255.0 * 255.0) / mse).log10())
}

fn image_ssim(
    source: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    embedded: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<f64, String> {
    if source.dimensions() != embedded.dimensions() {
        return Err("image dimensions differ".to_string());
    }
    let mut n = 0.0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for (left, right) in source.pixels().zip(embedded.pixels()) {
        let x = luma(left);
        let y = luma(right);
        sum_x += x;
        sum_y += y;
        n += 1.0;
    }
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    let mut cov = 0.0;
    for (left, right) in source.pixels().zip(embedded.pixels()) {
        let dx = luma(left) - mean_x;
        let dy = luma(right) - mean_y;
        var_x += dx * dx;
        var_y += dy * dy;
        cov += dx * dy;
    }
    var_x /= n - 1.0;
    var_y /= n - 1.0;
    cov /= n - 1.0;
    let c1 = (0.01_f64 * 255.0).powi(2);
    let c2 = (0.03_f64 * 255.0).powi(2);
    Ok(((2.0 * mean_x * mean_y + c1) * (2.0 * cov + c2))
        / ((mean_x.powi(2) + mean_y.powi(2) + c1) * (var_x + var_y + c2)))
}

fn luma(pixel: &Rgb<u8>) -> f64 {
    0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2])
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

fn audio_snr(source: &[f32], embedded: &[f32]) -> Result<f64, String> {
    let len = source.len().min(embedded.len());
    if len == 0 {
        return Err("empty audio samples".to_string());
    }
    let mut signal = 0.0;
    let mut noise = 0.0;
    for index in 0..len {
        let s = f64::from(source[index]);
        let e = f64::from(embedded[index]);
        signal += s * s;
        noise += (s - e) * (s - e);
    }
    if noise == 0.0 {
        return Ok(99.0);
    }
    Ok(10.0 * (signal / noise).log10())
}

fn peak_abs(samples: &[f32]) -> f64 {
    samples
        .iter()
        .fold(0.0_f64, |acc, value| acc.max(f64::from(value.abs())))
}

fn approx_lufs(samples: &[f32]) -> f64 {
    let mean_square = samples
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        / samples.len().max(1) as f64;
    -0.691 + 10.0 * mean_square.max(1.0e-12).log10()
}

fn build_payload(
    run_id: &str,
    kind: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{kind}:v3-quality").as_bytes());
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
            format!("{run_id}:{kind}:registry-proof").as_bytes(),
        )),
        creator_binding: Some("HiddenShield V3 perceptual quality gate"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn make_png_image() -> Result<Vec<u8>, String> {
    let width = 1024;
    let height = 1024;
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([
            (x * 255 / width) as u8,
            (y * 255 / height) as u8,
            (((x ^ y) & 0xff) as u8).saturating_add(16),
        ])
    });
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("encode png: {error}"))?;
    Ok(cursor.into_inner())
}

fn make_wav_audio() -> Result<Vec<u8>, String> {
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
        for sample_index in 0..(44_100 * 30) {
            let t = sample_index as f32 / 44_100.0;
            let value = (0.32 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
                + 0.16 * (2.0 * std::f32::consts::PI * 880.0 * t).sin())
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

fn render_markdown(run_id: &str, image: &GateRow, audio: &GateRow) -> String {
    format!(
        "# HiddenShield V3 感知质量与性能门禁\n\n- runId: `{}`\n- imagePassed: {}\n- audioPassed: {}\n- videoVmaf: skipped, 当前 L3 视频视觉仍是内部 staged 能力\n\n```json\n{{\n  \"image\": {},\n  \"audio\": {}\n}}\n```\n",
        run_id, image.passed, audio.passed, image.json, audio.json
    )
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
