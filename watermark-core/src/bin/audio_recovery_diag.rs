use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use realfft::num_complex::Complex;
use realfft::RealFftPlanner;
use sha2::{Digest, Sha256};
use watermark_core::{
    encode_payload, EmbedOptions, MediaInput, MediaOutput, WatermarkPayload, WatermarkService,
};

const FRAME_SIZE: usize = 4096;
const CANONICAL_SAMPLE_RATE: u32 = 44_100;
const BAND_LO_HZ: f32 = 186.0 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
const BAND_HI_HZ: f32 = 743.0 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
const SILENCE_THRESHOLD: f32 = 0.001;
const RELATIVE_PAIR_WIDTH: usize = 4;
const AUDIO_SLICE_MARKERS: usize = 16;
const AUDIO_MARKER_PREAMBLE: [u8; 2] = [0xA7, 0x5C];
const AUDIO_MARKER_BYTES: usize = 8;
const AUDIO_MARKER_BITS: usize = AUDIO_MARKER_BYTES * 8;
const AUDIO_MARKER_REDUNDANCY: usize = 3;
const AUDIO_MARKER_BITS_PER_FRAME: usize = 12;
const AUDIO_MARKER_BIT_LANES: usize = 3;
const AUDIO_RECOVERY_PREAMBLE: [u8; 4] = [0xA7, 0x5C, 0x41, 0x52];
const AUDIO_RECOVERY_CHECKSUM_BYTES: usize = 2;
const AUDIO_RECOVERY_PACKET_BYTES: usize = 4 + 32 + AUDIO_RECOVERY_CHECKSUM_BYTES;
const AUDIO_RECOVERY_PACKET_BITS: usize = AUDIO_RECOVERY_PACKET_BYTES * 8;
const AUDIO_RECOVERY_REDUNDANCY: usize = 3;
const AUDIO_RECOVERY_BITS_PER_FRAME: usize = 18;
const AUDIO_RECOVERY_BIT_LANES: usize = 3;
const AUDIO_MARKER_PAIR_OFFSET: usize = AUDIO_MARKER_BITS_PER_FRAME * AUDIO_MARKER_BIT_LANES + 4;
const AUDIO_RECOVERY_PAIR_OFFSET: usize =
    AUDIO_MARKER_PAIR_OFFSET + AUDIO_RECOVERY_BITS_PER_FRAME * AUDIO_RECOVERY_BIT_LANES + 4;

#[derive(Debug, Clone)]
struct Config {
    audio_glob: String,
    max_audio: usize,
    output_dir: PathBuf,
    ffmpeg: String,
    phase_step: usize,
    phase_center: Option<usize>,
    phase_radius: usize,
    top: usize,
}

#[derive(Debug, Clone)]
struct AnalysisReport {
    source: String,
    target: String,
    duration_ms: u128,
    sample_rate: u32,
    samples: usize,
    full_extract_ok: bool,
    marker_hits: usize,
    marker_best_bit_errors: Option<usize>,
    best: Option<RecoveryCandidate>,
    top: Vec<RecoveryCandidate>,
}

#[derive(Debug, Clone)]
struct RecoveryCandidate {
    phase: usize,
    start_frame: usize,
    total_bit_errors: usize,
    preamble_bit_errors: usize,
    payload_bit_errors: usize,
    checksum_bit_errors: usize,
    preamble_ok: bool,
    checksum_ok: bool,
    payload_decode_ok: bool,
    active_frames: usize,
}

#[derive(Debug, Clone, Copy)]
struct MarkerCandidate {
    bit_errors: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let run_dir = config
        .output_dir
        .join(format!("audio-recovery-diag-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create diag dir: {error}"))?;

    let sources = collect_audio(&config.audio_glob, config.max_audio)?;
    if sources.is_empty() {
        return Err("no audio sources found".into());
    }

    let mut reports = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        reports.extend(run_source(source, index, &run_dir, &config)?);
    }

    let report_path = run_dir.join("audio-recovery-diag.md");
    write_report(&report_path, &reports)?;
    println!(
        "Audio recovery diagnostic finished: {} reports",
        reports.len()
    );
    println!("Report: {}", display_path(&report_path));
    for report in &reports {
        print_summary(report);
    }

    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut audio_glob = None;
        let mut max_audio = 1usize;
        let mut output_dir = PathBuf::from("watermark-core/target/audio-recovery-diag");
        let mut ffmpeg = "ffmpeg".to_string();
        let mut phase_step = 512usize;
        let mut phase_center = None;
        let mut phase_radius = 0usize;
        let mut top = 5usize;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--audio-glob" => {
                    index += 1;
                    audio_glob = Some(required_value(&args, index, "--audio-glob")?.to_string());
                }
                "--max-audio" => {
                    index += 1;
                    max_audio = parse_usize(required_value(&args, index, "--max-audio")?)?;
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(required_value(&args, index, "--output-dir")?);
                }
                "--ffmpeg" => {
                    index += 1;
                    ffmpeg = required_value(&args, index, "--ffmpeg")?.to_string();
                }
                "--phase-step" => {
                    index += 1;
                    phase_step = parse_usize(required_value(&args, index, "--phase-step")?)?.max(1);
                }
                "--phase-center" => {
                    index += 1;
                    phase_center = Some(parse_usize(required_value(
                        &args,
                        index,
                        "--phase-center",
                    )?)?);
                }
                "--phase-radius" => {
                    index += 1;
                    phase_radius = parse_usize(required_value(&args, index, "--phase-radius")?)?;
                }
                "--top" => {
                    index += 1;
                    top = parse_usize(required_value(&args, index, "--top")?)?.max(1);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
            index += 1;
        }

        Ok(Self {
            audio_glob: audio_glob
                .ok_or_else(|| "--audio-glob is required, for example path/*.mp3".to_string())?,
            max_audio,
            output_dir,
            ffmpeg,
            phase_step,
            phase_center,
            phase_radius,
            top,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin audio_recovery_diag -- \
  --audio-glob <path/*.mp3> [--max-audio 1] [--phase-step 512] [--phase-center 2728] [--phase-radius 128] [--top 5]"
    );
}

fn run_source(
    source: &Path,
    index: usize,
    run_dir: &Path,
    config: &Config,
) -> Result<Vec<AnalysisReport>, String> {
    let audio_dir = run_dir.join(format!("{index:02}_{}", safe_stem(source)));
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio dir: {error}"))?;

    let source_wav = audio_dir.join("source_30s.wav");
    run_ffmpeg(
        &config.ffmpeg,
        &[
            "-y",
            "-i",
            &source.display().to_string(),
            "-t",
            "30",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            &source_wav.display().to_string(),
        ],
    )?;

    let source_bytes =
        fs::read(&source_wav).map_err(|error| format!("read source wav: {error}"))?;
    let payload = payload_for_source(&source_bytes, 10_000 + index as u64);
    let expected_uid = payload.watermark_uid();
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed audio '{}': {error}", source.display()))?;
    let MediaOutput::AudioWavBytes { bytes: embedded } = output else {
        return Err("unexpected non-audio output".into());
    };

    let embedded_wav = audio_dir.join("embedded.wav");
    fs::write(&embedded_wav, &embedded).map_err(|error| format!("write embedded wav: {error}"))?;

    let clip_wav = audio_dir.join("clip_10s_middle.wav");
    run_ffmpeg(
        &config.ffmpeg,
        &[
            "-y",
            "-i",
            &embedded_wav.display().to_string(),
            "-ss",
            "10",
            "-t",
            "10",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            &clip_wav.display().to_string(),
        ],
    )?;

    let payload_bytes = encode_payload(&payload);
    let expected_packet = encode_audio_recovery_packet(&payload_bytes);
    let source_name = display_name(source);

    Ok(vec![
        analyze_wav(
            &source_name,
            "embedded_30s",
            &embedded_wav,
            &expected_uid,
            &expected_packet,
            config,
        )?,
        analyze_wav(
            &source_name,
            "clip_10s_middle",
            &clip_wav,
            &expected_uid,
            &expected_packet,
            config,
        )?,
    ])
}

fn analyze_wav(
    source: &str,
    target: &str,
    wav_path: &Path,
    expected_uid: &str,
    expected_packet: &[u8; AUDIO_RECOVERY_PACKET_BYTES],
    config: &Config,
) -> Result<AnalysisReport, String> {
    let bytes = fs::read(wav_path)
        .map_err(|error| format!("read wav '{}': {error}", wav_path.display()))?;
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|error| format!("open wav '{}': {error}", wav_path.display()))?;
    let spec = reader.spec();
    let samples = read_wav_samples(&mut reader)?;

    let started = Instant::now();
    let expected_bits = bytes_to_bits(expected_packet);
    let top = scan_recovery_candidates(&samples, spec.sample_rate, &expected_bits, config)?;
    let marker = scan_marker_candidates(&samples, spec.sample_rate, config)?;
    let duration_ms = started.elapsed().as_millis();

    let full_extract_ok = WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
        .map(|payload| payload.watermark_uid() == expected_uid)
        .unwrap_or(false);

    Ok(AnalysisReport {
        source: source.to_string(),
        target: target.to_string(),
        duration_ms,
        sample_rate: spec.sample_rate,
        samples: samples.len(),
        full_extract_ok,
        marker_hits: marker
            .iter()
            .filter(|candidate| candidate.bit_errors == 0)
            .count(),
        marker_best_bit_errors: marker.iter().map(|candidate| candidate.bit_errors).min(),
        best: top.first().cloned(),
        top,
    })
}

fn scan_recovery_candidates(
    samples: &[f32],
    sample_rate: u32,
    expected_bits: &[bool],
    config: &Config,
) -> Result<Vec<RecoveryCandidate>, String> {
    let recovery_frames = audio_recovery_frames_per_packet();
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if samples.len() < FRAME_SIZE * recovery_frames
        || usable_pairs <= AUDIO_RECOVERY_PAIR_OFFSET + 16
    {
        return Ok(Vec::new());
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let mut top = Vec::<RecoveryCandidate>::new();
    for phase in phase_candidates(config) {
        if phase >= FRAME_SIZE || phase >= samples.len() {
            continue;
        }
        let candidate_samples = &samples[phase..];
        let num_frames = candidate_samples.len() / FRAME_SIZE;
        let frame_recovery_bits = precompute_recovery_frame_bits(candidate_samples, &fft, bin_lo)?;
        let max_start = recovery_frames.min(num_frames.saturating_sub(recovery_frames) + 1);
        for start_frame in 0..max_start {
            let candidate =
                scan_recovery_at(&frame_recovery_bits, phase, start_frame, expected_bits)?;
            push_top(&mut top, candidate, config.top);
        }
    }

    top.sort_by_key(candidate_sort_key);
    Ok(top)
}

fn phase_candidates(config: &Config) -> Vec<usize> {
    if let Some(center) = config.phase_center {
        let start = center.saturating_sub(config.phase_radius);
        let end = (center + config.phase_radius).min(FRAME_SIZE - 1);
        return (start..=end).step_by(config.phase_step).collect();
    }
    (0..FRAME_SIZE).step_by(config.phase_step).collect()
}

fn scan_recovery_at(
    frame_recovery_bits: &[Option<Vec<bool>>],
    phase: usize,
    start_frame: usize,
    expected_bits: &[bool],
) -> Result<RecoveryCandidate, String> {
    let recovery_frames = audio_recovery_frames_per_packet();
    let raw_total = AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let mut raw_votes = vec![0i32; raw_total];
    let mut raw_seen = vec![false; raw_total];
    let mut active_frames = 0usize;
    for (frame_idx, frame_bits) in frame_recovery_bits.iter().enumerate().skip(start_frame) {
        let Some(frame_bits) = frame_bits else {
            continue;
        };
        active_frames += 1;

        let local_frame = (frame_idx - start_frame) % recovery_frames;
        let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
        if raw_start >= raw_total {
            continue;
        }
        for (slot, bit) in frame_bits.iter().copied().enumerate() {
            let raw_idx = raw_start + slot;
            if raw_idx >= raw_total {
                break;
            }
            raw_seen[raw_idx] = true;
            raw_votes[raw_idx] += if bit { 1 } else { -1 };
        }
    }

    let bits = if raw_seen.iter().all(|seen| *seen) {
        let raw_bits = raw_votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
        majority_bits_with_redundancy(
            &raw_bits,
            AUDIO_RECOVERY_PACKET_BITS,
            AUDIO_RECOVERY_REDUNDANCY,
        )
    } else {
        vec![false; AUDIO_RECOVERY_PACKET_BITS]
    };
    let bytes = bits_to_bytes(&bits);

    Ok(RecoveryCandidate {
        phase,
        start_frame,
        total_bit_errors: hamming(&bits, expected_bits),
        preamble_bit_errors: hamming(&bits[0..32], &expected_bits[0..32]),
        payload_bit_errors: hamming(&bits[32..288], &expected_bits[32..288]),
        checksum_bit_errors: hamming(&bits[288..304], &expected_bits[288..304]),
        preamble_ok: bytes.get(0..4) == Some(AUDIO_RECOVERY_PREAMBLE.as_slice()),
        checksum_ok: bytes
            .get(4..36)
            .and_then(|payload| payload.try_into().ok())
            .map(|payload: &[u8; 32]| {
                bytes.get(36..38) == Some(audio_recovery_checksum(payload).as_slice())
            })
            .unwrap_or(false),
        payload_decode_ok: bytes
            .get(4..36)
            .and_then(|payload| payload.try_into().ok())
            .map(|payload| watermark_core::decode_payload(payload).is_ok())
            .unwrap_or(false),
        active_frames,
    })
}

fn precompute_recovery_frame_bits(
    samples: &[f32],
    fft: &std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    bin_lo: usize,
) -> Result<Vec<Option<Vec<bool>>>, String> {
    let num_frames = samples.len() / FRAME_SIZE;
    (0..num_frames)
        .map(|frame_idx| {
            let offset = frame_idx * FRAME_SIZE;
            let frame = &samples[offset..offset + FRAME_SIZE];
            if rms_energy(frame) < SILENCE_THRESHOLD {
                return Ok(None);
            }

            let mut input = frame.to_vec();
            let mut spectrum = fft.make_output_vec();
            fft.process(&mut input, &mut spectrum)
                .map_err(|error| format!("FFT failed: {error}"))?;
            Ok(Some(extract_audio_recovery_frame_bits(&spectrum, bin_lo)))
        })
        .collect()
}

fn scan_marker_candidates(
    samples: &[f32],
    sample_rate: u32,
    config: &Config,
) -> Result<Vec<MarkerCandidate>, String> {
    let marker_frames =
        (AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY).div_ceil(AUDIO_MARKER_BITS_PER_FRAME);
    if samples.len() < FRAME_SIZE * marker_frames {
        return Ok(Vec::new());
    }

    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if usable_pairs <= AUDIO_MARKER_PAIR_OFFSET + 16 {
        return Ok(Vec::new());
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let mut candidates = Vec::<MarkerCandidate>::new();
    for phase in phase_candidates(config) {
        if phase >= FRAME_SIZE || phase >= samples.len() {
            continue;
        }
        let candidate_samples = &samples[phase..];
        let num_frames = candidate_samples.len() / FRAME_SIZE;
        for start_frame in 0..num_frames.saturating_sub(marker_frames) {
            let mut raw_bits = Vec::with_capacity(AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY);
            for frame_idx in start_frame..start_frame + marker_frames {
                let offset = frame_idx * FRAME_SIZE;
                let frame = &candidate_samples[offset..offset + FRAME_SIZE];
                if rms_energy(frame) < SILENCE_THRESHOLD {
                    continue;
                }

                let mut input = frame.to_vec();
                let mut spectrum = fft.make_output_vec();
                fft.process(&mut input, &mut spectrum)
                    .map_err(|error| format!("FFT failed: {error}"))?;
                let remaining = AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY - raw_bits.len();
                let mut frame_bits = extract_audio_marker_frame_bits(&spectrum, bin_lo);
                frame_bits.truncate(remaining);
                raw_bits.extend(frame_bits);
                if raw_bits.len() >= AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY {
                    break;
                }
            }
            if raw_bits.len() < AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY {
                continue;
            }
            let marker_bits = majority_bits_with_redundancy(
                &raw_bits,
                AUDIO_MARKER_BITS,
                AUDIO_MARKER_REDUNDANCY,
            );
            let marker_bytes = bits_to_bytes(&marker_bits);
            let bit_errors = best_marker_preamble_errors(&marker_bytes);
            candidates.push(MarkerCandidate { bit_errors });
        }
    }

    Ok(candidates)
}

fn push_top(top: &mut Vec<RecoveryCandidate>, candidate: RecoveryCandidate, limit: usize) {
    top.push(candidate);
    top.sort_by_key(candidate_sort_key);
    top.truncate(limit);
}

fn candidate_sort_key(candidate: &RecoveryCandidate) -> (usize, usize, usize, usize) {
    (
        candidate.total_bit_errors,
        candidate.preamble_bit_errors,
        candidate.checksum_bit_errors,
        candidate.start_frame,
    )
}

fn extract_audio_marker_frame_bits(spectrum: &[Complex<f32>], bin_lo: usize) -> Vec<bool> {
    (0..AUDIO_MARKER_BITS_PER_FRAME)
        .map(|bit_slot| {
            let ones = (0..AUDIO_MARKER_BIT_LANES)
                .filter(|&lane| {
                    let pair_idx = bit_slot * AUDIO_MARKER_BIT_LANES + lane;
                    extract_relative_pair(spectrum, bin_lo + pair_idx * RELATIVE_PAIR_WIDTH)
                })
                .count();
            ones > AUDIO_MARKER_BIT_LANES / 2
        })
        .collect()
}

fn extract_audio_recovery_frame_bits(spectrum: &[Complex<f32>], bin_lo: usize) -> Vec<bool> {
    (0..AUDIO_RECOVERY_BITS_PER_FRAME)
        .map(|bit_slot| {
            let ones = (0..AUDIO_RECOVERY_BIT_LANES)
                .filter(|&lane| {
                    let pair_idx =
                        AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                    extract_relative_pair(spectrum, bin_lo + pair_idx * RELATIVE_PAIR_WIDTH)
                })
                .count();
            ones > AUDIO_RECOVERY_BIT_LANES / 2
        })
        .collect()
}

fn extract_relative_pair(spectrum: &[Complex<f32>], group_start: usize) -> bool {
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left = (0..half_width)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let right = (half_width..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    left >= right
}

fn audio_band_bins(sample_rate: u32) -> (usize, usize) {
    let sample_rate = sample_rate.max(1) as f32;
    let nyquist = sample_rate / 2.0;
    let lo_hz = BAND_LO_HZ.min(nyquist * 0.95);
    let hi_hz = BAND_HI_HZ.min(nyquist * 0.98);
    let mut lo = ((lo_hz * FRAME_SIZE as f32) / sample_rate).round() as usize;
    let mut hi = ((hi_hz * FRAME_SIZE as f32) / sample_rate).round() as usize;
    let max_bin = FRAME_SIZE / 2;
    lo = lo.clamp(1, max_bin.saturating_sub(2));
    hi = hi.clamp(lo + 2, max_bin);
    (lo, hi)
}

fn audio_recovery_frames_per_packet() -> usize {
    (AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY).div_ceil(AUDIO_RECOVERY_BITS_PER_FRAME)
}

fn encode_audio_recovery_packet(payload_bytes: &[u8; 32]) -> [u8; AUDIO_RECOVERY_PACKET_BYTES] {
    let mut packet = [0u8; AUDIO_RECOVERY_PACKET_BYTES];
    packet[0..4].copy_from_slice(&AUDIO_RECOVERY_PREAMBLE);
    packet[4..36].copy_from_slice(payload_bytes);
    let checksum = audio_recovery_checksum(payload_bytes);
    packet[36..38].copy_from_slice(&checksum);
    packet
}

fn audio_recovery_checksum(payload_bytes: &[u8; 32]) -> [u8; AUDIO_RECOVERY_CHECKSUM_BYTES] {
    let mut state = 0xA6D3u16;
    for &byte in payload_bytes {
        state = state.rotate_left(5) ^ byte as u16;
        state = state.wrapping_mul(241);
    }
    state.to_be_bytes()
}

fn best_marker_preamble_errors(bytes: &[u8]) -> usize {
    if bytes.len() < AUDIO_MARKER_BYTES {
        return AUDIO_MARKER_BITS;
    }
    let bits = bytes_to_bits(bytes);
    let preamble_bits = bytes_to_bits(&AUDIO_MARKER_PREAMBLE);
    let preamble_errors = hamming(&bits[0..16], &preamble_bits);
    let version_errors = if bytes[2] == 1 { 0 } else { 1 };
    let slice_errors = if (bytes[3] as usize) < AUDIO_SLICE_MARKERS {
        0
    } else {
        1
    };
    preamble_errors + version_errors + slice_errors
}

fn read_wav_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
) -> Result<Vec<f32>, String> {
    let spec = reader.spec();
    if spec.sample_format == hound::SampleFormat::Float {
        reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| format!("read WAV sample: {error}")))
            .collect()
    } else {
        let max_val = (1i32 << (spec.bits_per_sample - 1)) as f32;
        reader
            .samples::<i32>()
            .map(|sample| {
                sample
                    .map_err(|error| format!("read WAV sample: {error}"))
                    .map(|value| value as f32 / max_val)
            })
            .collect()
    }
}

fn rms_energy(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum = frame.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / frame.len() as f32).sqrt()
}

fn majority_bits_with_redundancy(
    raw_bits: &[bool],
    bit_count: usize,
    redundancy: usize,
) -> Vec<bool> {
    (0..bit_count)
        .map(|bit_idx| {
            let start = bit_idx * redundancy;
            let chunk = &raw_bits[start..start + redundancy];
            let ones = chunk.iter().filter(|&&bit| bit).count();
            ones > chunk.len() / 2
        })
        .collect()
}

fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &byte in bytes {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1 == 1);
        }
    }
    bits
}

fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            byte
        })
        .collect()
}

fn hamming(left: &[bool], right: &[bool]) -> usize {
    left.iter()
        .zip(right.iter())
        .filter(|(left, right)| left != right)
        .count()
}

fn collect_audio(audio_glob: &str, limit: usize) -> Result<Vec<PathBuf>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let glob_path = PathBuf::from(audio_glob);
    let parent = glob_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_pattern = glob_path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("invalid audio glob: {audio_glob}"))?;
    let extension = file_pattern
        .strip_prefix("*.")
        .ok_or_else(|| "only simple audio globs like path/*.mp3 are supported".to_string())?
        .to_ascii_lowercase();

    let mut paths = Vec::new();
    for entry in fs::read_dir(parent).map_err(|error| format!("read audio dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read audio dir entry: {error}"))?
            .path();
        if path
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case(&extension))
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths.truncate(limit);
    Ok(paths)
}

fn payload_for_source(bytes: &[u8], salt: u64) -> WatermarkPayload {
    let digest = Sha256::digest(bytes);
    let mut user_seed = [0u8; 8];
    user_seed.copy_from_slice(&digest[0..8]);
    user_seed[0] ^= (salt & 0xFF) as u8;
    let mut device_id = [0u8; 4];
    device_id.copy_from_slice(&digest[8..12]);
    let mut file_hash = [0u8; 2];
    file_hash.copy_from_slice(&digest[12..14]);
    WatermarkPayload::new(
        user_seed,
        1_800_000_000 + salt,
        device_id,
        file_hash,
        Default::default(),
    )
}

fn run_ffmpeg(ffmpeg: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(ffmpeg)
        .args(args)
        .output()
        .map_err(|error| format!("start ffmpeg: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .take(6)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

fn write_report(path: &Path, reports: &[AnalysisReport]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Audio Recovery Bit Error Diagnostic\n\n");
    out.push_str("| Source | Target | Extract OK | Marker hits | Best marker errors | Best packet errors | Preamble errors | Payload errors | Checksum errors | Phase | Start frame | Time ms |\n");
    out.push_str(
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for report in reports {
        let best = report.best.as_ref();
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            escape_md(&report.source),
            report.target,
            if report.full_extract_ok {
                "PASS"
            } else {
                "FAIL"
            },
            report.marker_hits,
            format_option(report.marker_best_bit_errors),
            format_option(best.map(|candidate| candidate.total_bit_errors)),
            format_option(best.map(|candidate| candidate.preamble_bit_errors)),
            format_option(best.map(|candidate| candidate.payload_bit_errors)),
            format_option(best.map(|candidate| candidate.checksum_bit_errors)),
            format_option(best.map(|candidate| candidate.phase)),
            format_option(best.map(|candidate| candidate.start_frame)),
            report.duration_ms,
        ));
    }

    for report in reports {
        out.push_str(&format!(
            "\n## {} / {}\n\n- sample_rate: {}\n- samples: {}\n- full_extract_ok: {}\n- marker_hits: {}\n\n",
            escape_md(&report.source),
            report.target,
            report.sample_rate,
            report.samples,
            report.full_extract_ok,
            report.marker_hits
        ));
        out.push_str("| Rank | Packet errors | Preamble | Payload | Checksum | Preamble OK | Checksum OK | Payload decode OK | Phase | Start frame | Active frames |\n");
        out.push_str(
            "| ---: | ---: | ---: | ---: | ---: | --- | --- | --- | ---: | ---: | ---: |\n",
        );
        for (index, candidate) in report.top.iter().enumerate() {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                index + 1,
                candidate.total_bit_errors,
                candidate.preamble_bit_errors,
                candidate.payload_bit_errors,
                candidate.checksum_bit_errors,
                candidate.preamble_ok,
                candidate.checksum_ok,
                candidate.payload_decode_ok,
                candidate.phase,
                candidate.start_frame,
                candidate.active_frames
            ));
        }
    }

    fs::write(path, out).map_err(|error| format!("write diag report: {error}"))
}

fn print_summary(report: &AnalysisReport) {
    if let Some(best) = &report.best {
        println!(
            "{} {}: extract_ok={}, marker_hits={}, best_errors={} (preamble={}, payload={}, checksum={}, start_frame={})",
            report.source,
            report.target,
            report.full_extract_ok,
            report.marker_hits,
            best.total_bit_errors,
            best.preamble_bit_errors,
            best.payload_bit_errors,
            best.checksum_bit_errors,
            best.start_frame,
        );
    } else {
        println!(
            "{} {}: extract_ok={}, marker_hits={}, no recovery candidates",
            report.source, report.target, report.full_extract_ok, report.marker_hits
        );
    }
}

fn required_value<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid number '{value}': {error}"))
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn safe_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("source")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn format_option(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "".to_string())
}
