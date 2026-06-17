use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const FRAME_SIZE: usize = 4096;
const SAMPLE_RATE: u32 = 44_100;
const SILENCE_THRESHOLD: f32 = 0.001;
const V2_PACKET_BYTES: usize = 14;
const V2_PACKET_BITS: usize = V2_PACKET_BYTES * 8;
const V2_REDUNDANCY: usize = 3;
const V2_BITS_PER_FRAME: usize = 18;
const V2_DATA_SEGMENTS: usize = 8;
const V2_PARITY_SEGMENTS: usize = 4;
const V2_TOTAL_SEGMENTS: usize = V2_DATA_SEGMENTS + V2_PARITY_SEGMENTS;
const V2_EPOCH_STRIDE_FRAMES: usize = 2;

#[derive(Debug, Clone)]
struct Config {
    audio_glob: String,
    max_audio: usize,
    output_dir: PathBuf,
    ffmpeg: String,
    seconds: usize,
}

#[derive(Debug, Clone)]
struct ClipReport {
    source: String,
    clip: String,
    start_sec: usize,
    duration_sec: usize,
    frames: usize,
    active_frames: usize,
    ideal_bytes: usize,
    packets_seen: usize,
    data_segments_seen: usize,
    parity_segments_seen: usize,
    recoverable: bool,
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
        .join(format!("audio-recovery-v2-probe-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create probe dir: {error}"))?;

    let sources = collect_audio(&config.audio_glob, config.max_audio)?;
    if sources.is_empty() {
        return Err("no audio sources found".into());
    }

    let mut rows = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        rows.extend(probe_source(source, index, &run_dir, &config)?);
    }

    let report_path = run_dir.join("audio-recovery-v2-probe.md");
    write_report(&report_path, &rows)?;
    println!("Audio Recovery V2 probe finished: {} clips", rows.len());
    println!("Report: {}", display_path(&report_path));
    print_summary(&rows);
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut audio_glob = None;
        let mut max_audio = 1usize;
        let mut output_dir = PathBuf::from("watermark-core/target/audio-recovery-v2-probe");
        let mut ffmpeg = "ffmpeg".to_string();
        let mut seconds = 30usize;

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
                "--seconds" => {
                    index += 1;
                    seconds = parse_usize(required_value(&args, index, "--seconds")?)?;
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
            seconds,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin audio_recovery_v2_probe -- \
  --audio-glob <path/*.mp3> [--max-audio 1] [--seconds 30]"
    );
}

fn probe_source(
    source: &Path,
    index: usize,
    run_dir: &Path,
    config: &Config,
) -> Result<Vec<ClipReport>, String> {
    let audio_dir = run_dir.join(format!("{index:02}_{}", safe_stem(source)));
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio dir: {error}"))?;
    let source_wav = audio_dir.join("source.wav");
    run_ffmpeg(
        &config.ffmpeg,
        &[
            "-y",
            "-i",
            &source.display().to_string(),
            "-t",
            &config.seconds.to_string(),
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            &source_wav.display().to_string(),
        ],
    )?;

    let mut reader = hound::WavReader::open(&source_wav)
        .map_err(|error| format!("open wav '{}': {error}", source_wav.display()))?;
    let samples = read_wav_samples(&mut reader)?;
    let active_frame_map = active_frames(&samples);
    let packets = simulate_v2_packets(&active_frame_map);
    let source_name = display_name(source);

    let mut rows = Vec::new();
    for duration in [5usize, 10, 15] {
        for position in ["start", "middle", "end"] {
            let start_sec = clip_start_for_position(position, duration, config.seconds);
            rows.push(analyze_clip(
                &source_name,
                &active_frame_map,
                &packets,
                start_sec,
                duration,
                position,
            ));
        }
    }
    Ok(rows)
}

fn active_frames(samples: &[f32]) -> Vec<bool> {
    let num_frames = samples.len() / FRAME_SIZE;
    (0..num_frames)
        .map(|frame_idx| {
            let offset = frame_idx * FRAME_SIZE;
            rms_energy(&samples[offset..offset + FRAME_SIZE]) >= SILENCE_THRESHOLD
        })
        .collect()
}

fn simulate_v2_packets(active_frames: &[bool]) -> Vec<PacketPlacement> {
    let frames_per_packet = v2_frames_per_packet();
    let mut placements = Vec::new();
    let mut cursor = 0usize;
    let mut epoch_id = 0usize;
    while cursor + frames_per_packet <= active_frames.len() {
        for segment_id in 0..V2_TOTAL_SEGMENTS {
            let start_frame = cursor + segment_id * frames_per_packet;
            let end_frame = start_frame + frames_per_packet;
            if end_frame > active_frames.len() {
                break;
            }
            let active_count = active_frames[start_frame..end_frame]
                .iter()
                .filter(|&&active| active)
                .count();
            if active_count * 2 >= frames_per_packet {
                placements.push(PacketPlacement {
                    epoch_id,
                    segment_id,
                    start_frame,
                    end_frame,
                });
            }
        }
        cursor += frames_per_packet * V2_EPOCH_STRIDE_FRAMES;
        epoch_id += 1;
    }
    placements
}

fn analyze_clip(
    source: &str,
    active_frame_map: &[bool],
    packets: &[PacketPlacement],
    start_sec: usize,
    duration_sec: usize,
    position: &str,
) -> ClipReport {
    let start_frame = seconds_to_frame(start_sec);
    let end_frame = seconds_to_frame(start_sec + duration_sec);
    let frames = end_frame.saturating_sub(start_frame);
    let active_frames = active_frame_map
        .get(start_frame..end_frame.min(active_frame_map.len()))
        .unwrap_or(&[])
        .iter()
        .filter(|&&active| active)
        .count();
    let ideal_bytes = active_frames * V2_BITS_PER_FRAME / V2_REDUNDANCY / 8;
    let mut segments_by_epoch = Vec::<EpochSegments>::new();
    for packet in packets {
        if packet.start_frame >= start_frame && packet.end_frame <= end_frame {
            let entry = find_or_insert_epoch(&mut segments_by_epoch, packet.epoch_id);
            if packet.segment_id < V2_DATA_SEGMENTS {
                entry.data[packet.segment_id] = true;
            } else {
                entry.parity[packet.segment_id - V2_DATA_SEGMENTS] = true;
            }
        }
    }

    let data_segments_seen = segments_by_epoch
        .iter()
        .map(|epoch| epoch.data.iter().filter(|&&seen| seen).count())
        .max()
        .unwrap_or(0);
    let parity_segments_seen = segments_by_epoch
        .iter()
        .map(|epoch| epoch.parity.iter().filter(|&&seen| seen).count())
        .max()
        .unwrap_or(0);
    let recoverable = segments_by_epoch.iter().any(EpochSegments::recoverable);

    ClipReport {
        source: source.to_string(),
        clip: format!("{duration_sec}s_{position}"),
        start_sec,
        duration_sec,
        frames,
        active_frames,
        ideal_bytes,
        packets_seen: segments_by_epoch
            .iter()
            .map(EpochSegments::packet_count)
            .max()
            .unwrap_or(0),
        data_segments_seen,
        parity_segments_seen,
        recoverable,
    }
}

#[derive(Debug, Clone, Copy)]
struct PacketPlacement {
    epoch_id: usize,
    segment_id: usize,
    start_frame: usize,
    end_frame: usize,
}

#[derive(Debug, Clone)]
struct EpochSegments {
    epoch_id: usize,
    data: [bool; V2_DATA_SEGMENTS],
    parity: [bool; V2_PARITY_SEGMENTS],
}

impl EpochSegments {
    fn packet_count(&self) -> usize {
        self.data.iter().filter(|&&seen| seen).count()
            + self.parity.iter().filter(|&&seen| seen).count()
    }

    fn recoverable(&self) -> bool {
        if self.data.iter().all(|&seen| seen) {
            return true;
        }
        let missing = self.data.iter().filter(|&&seen| !seen).count();
        let parity_seen = self.parity.iter().filter(|&&seen| seen).count();
        missing <= parity_seen && missing <= 2
    }
}

fn find_or_insert_epoch(epochs: &mut Vec<EpochSegments>, epoch_id: usize) -> &mut EpochSegments {
    if let Some(index) = epochs.iter().position(|epoch| epoch.epoch_id == epoch_id) {
        return &mut epochs[index];
    }
    epochs.push(EpochSegments {
        epoch_id,
        data: [false; V2_DATA_SEGMENTS],
        parity: [false; V2_PARITY_SEGMENTS],
    });
    epochs.last_mut().expect("epoch was just inserted")
}

fn v2_frames_per_packet() -> usize {
    (V2_PACKET_BITS * V2_REDUNDANCY).div_ceil(V2_BITS_PER_FRAME)
}

fn seconds_to_frame(seconds: usize) -> usize {
    (seconds * SAMPLE_RATE as usize) / FRAME_SIZE
}

fn clip_start_for_position(position: &str, duration: usize, total: usize) -> usize {
    match position {
        "start" => 0,
        "middle" => total.saturating_sub(duration) / 2,
        "end" => total.saturating_sub(duration),
        _ => 0,
    }
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

fn write_report(path: &Path, rows: &[ClipReport]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Audio Recovery V2 Probe\n\n");
    out.push_str(&format!(
        "- V2 packet bytes: {}\n- V2 frames per packet: {}\n- Data segments: {}\n- Parity segments: {}\n- Epoch stride packets: {}\n- Ideal bytes formula: active_frames * bits_per_frame / redundancy / 8\n\n",
        V2_PACKET_BYTES,
        v2_frames_per_packet(),
        V2_DATA_SEGMENTS,
        V2_PARITY_SEGMENTS,
        V2_EPOCH_STRIDE_FRAMES
    ));
    out.push_str("| Source | Clip | Start sec | Duration sec | Frames | Active frames | Ideal bytes | Packets seen | Data segments | Parity segments | Recoverable |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            escape_md(&row.source),
            row.clip,
            row.start_sec,
            row.duration_sec,
            row.frames,
            row.active_frames,
            row.ideal_bytes,
            row.packets_seen,
            row.data_segments_seen,
            row.parity_segments_seen,
            row.recoverable
        ));
    }
    fs::write(path, out).map_err(|error| format!("write probe report: {error}"))
}

fn print_summary(rows: &[ClipReport]) {
    let recoverable = rows.iter().filter(|row| row.recoverable).count();
    println!("Recoverable clips: {}/{}", recoverable, rows.len());
    for row in rows {
        println!(
            "{} {}: recoverable={}, packets={}, data={}, parity={}",
            row.source,
            row.clip,
            row.recoverable,
            row.packets_seen,
            row.data_segments_seen,
            row.parity_segments_seen
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
