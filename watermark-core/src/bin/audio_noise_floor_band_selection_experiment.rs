use realfft::RealFftPlanner;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use watermark_core::{
    audio_v3_quality_diagnostics, AIContentFlags, AudioProtectionMode, AudioV3QualityDiagnostics,
    EmbedOptions, MediaInput, MediaOutput, PayloadV2BuildInput, WatermarkDecodedPayload,
    WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkPayloadV3MinimalAnchor,
    WatermarkService, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const AUDIO_SAMPLE_RATE: usize = 44_100;
const FRAME_SIZE: usize = 4096;
const BAND_LO_BIN: usize = 186;
const RELATIVE_PAIR_WIDTH: usize = 4;
const AUDIO_MARKER_BITS_PER_FRAME: usize = 12;
const AUDIO_MARKER_BIT_LANES: usize = 3;
const AUDIO_MARKER_PAIR_OFFSET: usize = AUDIO_MARKER_BITS_PER_FRAME * AUDIO_MARKER_BIT_LANES + 4;
const AUDIO_RECOVERY_BITS_PER_FRAME: usize = 18;
const AUDIO_RECOVERY_BIT_LANES: usize = 3;
const AUDIO_FULL_MIN_SNR: f64 = 44.0;
const AUDIO_FULL_MAX_PEAK_DELTA: f64 = 0.8;
const AUDIO_FULL_MAX_LUFS_DELTA: f64 = 0.5;
const AUDIO_MAX_NEW_CLIPPING: usize = 0;
const MIN_EXTRACTION_CONFIDENCE: f32 = 0.99;
const BASELINE_MODIFIED_PAIR_RATIO: f32 = 0.284046;
const AUDIO_DIAGNOSTIC_FFT_SIZE: usize = 4096;
const AUDIO_DIAGNOSTIC_SEGMENT_SECONDS: usize = 1;
const AUDIO_LOW_BAND_HZ: (f64, f64) = (20.0, 1_500.0);
const AUDIO_WATERMARK_BAND_HZ: (f64, f64) = (2_000.0, 8_000.0);
const AUDIO_HIGH_BAND_HZ: (f64, f64) = (8_000.0, 16_000.0);

fn main() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let run_id = optional_arg(&args, "--run-id").unwrap_or_else(|| unix_seconds().to_string());
    let out_dir = PathBuf::from(optional_arg(&args, "--out-dir").unwrap_or_else(|| {
        format!("watermark-core/target/audio-noise-floor-band-selection/run-{run_id}")
    }));
    fs::create_dir_all(out_dir.join("candidates"))
        .map_err(|error| format!("create candidates dir: {error}"))?;
    fs::create_dir_all(out_dir.join("abx")).map_err(|error| format!("create abx dir: {error}"))?;

    let source_samples = make_field_noise_samples(30);
    let source_wav = encode_wav(&source_samples)?;
    let payload = build_payload(&run_id, "field-noise", &source_wav)?;
    let baseline_wav = embed_baseline(&source_wav, &payload)?;
    let baseline_samples = wav_samples(&baseline_wav)?;
    let baseline_decoded = extract_decoded(&baseline_wav)?;
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = baseline_decoded else {
        return Err("baseline did not decode V3 minimal anchor".to_string());
    };
    if anchor.watermark_uid() != payload.watermark_uid() {
        return Err("baseline decoded UID does not match expected payload UID".to_string());
    }

    write_candidate_wavs(&out_dir, "baseline", &source_wav, &baseline_wav)?;
    let baseline = evaluate_candidate(
        "baseline",
        "current_noise_floor_sparse_recovery",
        &source_samples,
        &baseline_samples,
        &baseline_wav,
        &payload,
        &anchor,
        None,
    )?;

    let inner_subband = run_inner_subband_candidate(
        &source_samples,
        &baseline_samples,
        &source_wav,
        &payload,
        &anchor,
        &out_dir,
    )?;

    let frame_stability = run_frame_stability_candidate(
        &source_samples,
        &baseline_samples,
        &source_wav,
        &payload,
        &anchor,
        &out_dir,
    )?;

    let masked_pair_budget = run_masked_pair_budget_candidate(
        &source_samples,
        &baseline_samples,
        &source_wav,
        &payload,
        &anchor,
        &out_dir,
    )?;

    let planned_candidates = vec![PlannedCandidate {
        candidate_id: "cross_end_readable_frequency_strategy_migration",
        status: "planned_not_executed",
        reason: "Only if A/B/C fail to lower watermark-band noise share; requires a separate cross-end readable migration design",
    }];
    let constraints = Constraints {
        payload_format_unchanged: true,
        copyright_id_unchanged: true,
        cross_end_readability_unchanged: true,
        extraction_confidence_floor: MIN_EXTRACTION_CONFIDENCE,
        quality_threshold_unchanged: true,
        full_gate_snr_threshold_db: AUDIO_FULL_MIN_SNR,
        official_ui_or_mock_path_touched: false,
    };
    let experiment_pass = baseline.invariant_passed
        && inner_subband.invariant_passed
        && frame_stability.invariant_passed
        && masked_pair_budget.invariant_passed;
    let report = ExperimentReport {
        run_id: &run_id,
        experiment: "audio_noise_floor_band_selection_experiment",
        status: if experiment_pass {
            "experiment_artifacts_generated"
        } else {
            "blocked_by_experiment_invariant"
        },
        sample_id: "field-noise",
        source_profile: "field_recording_noise_floor",
        payload_protocol_version: 3,
        payload_bytes_length: PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        expected_watermark_uid: payload.watermark_uid(),
        constraints,
        candidates: vec![
            baseline,
            inner_subband,
            frame_stability,
            masked_pair_budget,
        ],
        planned_candidates,
        abx_template: "abx/audio_noise_floor_band_selection_abx_trials.csv",
        recommendation: "If C also fails to reduce bandEnergyShare.watermark.noise, stop micro-tuning inside the current 2-8 kHz extractor-readable lane layout and open a cross-end readable frequency-strategy migration design.",
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize experiment report: {error}"))?;
    fs::write(out_dir.join("experiment.json"), format!("{json}\n"))
        .map_err(|error| format!("write experiment json: {error}"))?;
    fs::write(out_dir.join("experiment.md"), render_markdown(&report))
        .map_err(|error| format!("write experiment markdown: {error}"))?;
    write_abx_template(&out_dir, &report)?;
    println!("{json}");

    if experiment_pass {
        Ok(())
    } else {
        Err("audio_noise_floor_band_selection_experiment invariant failed".to_string())
    }
}

fn run_inner_subband_candidate(
    source_samples: &[f32],
    baseline_samples: &[f32],
    source_wav: &[u8],
    payload: &WatermarkPayload,
    anchor: &WatermarkPayloadV3MinimalAnchor,
    out_dir: &Path,
) -> Result<CandidateResult, String> {
    let keep_lanes_options = [2usize, 3usize];
    let mut best: Option<CandidateResult> = None;
    let mut best_wav = Vec::new();

    for keep_lanes in keep_lanes_options {
        let candidate_samples = build_inner_watermark_subband_sparse_candidate(
            source_samples,
            baseline_samples,
            keep_lanes,
        )?;
        let candidate_wav = encode_wav(&candidate_samples)?;
        let result = evaluate_candidate(
            "inner_watermark_subband_sparse",
            "preserve_low_noise_high_energy_recovery_lanes_with_existing_extractor",
            source_samples,
            &candidate_samples,
            &candidate_wav,
            payload,
            anchor,
            Some(keep_lanes as f32),
        )?;
        if result.invariant_passed {
            let replace = best
                .as_ref()
                .map(|current| {
                    result
                        .metrics
                        .perceptual_diagnosis
                        .band_energy_share
                        .watermark
                        .noise
                        < current
                            .metrics
                            .perceptual_diagnosis
                            .band_energy_share
                            .watermark
                            .noise
                        || (result
                            .metrics
                            .perceptual_diagnosis
                            .band_energy_share
                            .watermark
                            .noise
                            == current
                                .metrics
                                .perceptual_diagnosis
                                .band_energy_share
                                .watermark
                                .noise
                            && result.metrics.snr > current.metrics.snr)
                })
                .unwrap_or(true);
            if replace {
                best = Some(result);
                best_wav = candidate_wav;
            }
        }
    }

    let Some(result) = best else {
        let fallback_samples =
            build_inner_watermark_subband_sparse_candidate(source_samples, baseline_samples, 3)?;
        let fallback_wav = encode_wav(&fallback_samples)?;
        write_candidate_wavs(
            out_dir,
            "inner_watermark_subband_sparse",
            source_wav,
            &fallback_wav,
        )?;
        return evaluate_candidate(
            "inner_watermark_subband_sparse",
            "preserve_low_noise_high_energy_recovery_lanes_with_existing_extractor",
            source_samples,
            &fallback_samples,
            &fallback_wav,
            payload,
            anchor,
            Some(3.0),
        );
    };

    write_candidate_wavs(
        out_dir,
        "inner_watermark_subband_sparse",
        source_wav,
        &best_wav,
    )?;
    Ok(result)
}

fn build_inner_watermark_subband_sparse_candidate(
    source: &[f32],
    baseline: &[f32],
    keep_lanes: usize,
) -> Result<Vec<f32>, String> {
    let len = source.len().min(baseline.len());
    let frame_count = len / FRAME_SIZE;
    let mut candidate = baseline.to_vec();
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let ifft = planner.plan_fft_inverse(FRAME_SIZE);

    for frame_idx in 0..frame_count {
        let start = frame_idx * FRAME_SIZE;
        let end = start + FRAME_SIZE;
        let mut source_input = source[start..end].to_vec();
        let mut baseline_input = baseline[start..end].to_vec();
        let mut source_spectrum = fft.make_output_vec();
        let mut baseline_spectrum = fft.make_output_vec();
        fft.process(&mut source_input, &mut source_spectrum)
            .map_err(|error| format!("FFT source failed: {error}"))?;
        fft.process(&mut baseline_input, &mut baseline_spectrum)
            .map_err(|error| format!("FFT baseline failed: {error}"))?;

        let mut candidate_spectrum = source_spectrum.clone();
        for bit_slot in 0..AUDIO_RECOVERY_BITS_PER_FRAME {
            let mut lanes = (0..AUDIO_RECOVERY_BIT_LANES)
                .map(|lane| {
                    let pair_idx =
                        AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                    let bin = BAND_LO_BIN + pair_idx * RELATIVE_PAIR_WIDTH;
                    let signal_energy = pair_energy(&source_spectrum, bin);
                    let diff_energy = pair_diff_energy(&source_spectrum, &baseline_spectrum, bin);
                    let center_distance = (lane as f64 - 1.0).abs();
                    LaneScore {
                        lane,
                        signal_energy,
                        diff_energy,
                        center_distance,
                    }
                })
                .collect::<Vec<_>>();
            lanes.sort_by(|left, right| {
                left.diff_energy
                    .partial_cmp(&right.diff_energy)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        right
                            .signal_energy
                            .partial_cmp(&left.signal_energy)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        left.center_distance
                            .partial_cmp(&right.center_distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            for lane in lanes.iter().take(keep_lanes.min(AUDIO_RECOVERY_BIT_LANES)) {
                let pair_idx =
                    AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane.lane;
                let bin = BAND_LO_BIN + pair_idx * RELATIVE_PAIR_WIDTH;
                copy_pair_bins(&baseline_spectrum, &mut candidate_spectrum, bin);
            }
        }

        let mut output = ifft.make_output_vec();
        ifft.process(&mut candidate_spectrum, &mut output)
            .map_err(|error| format!("IFFT candidate failed: {error}"))?;
        let scale = 1.0 / FRAME_SIZE as f32;
        for (offset, sample) in output.iter().enumerate().take(FRAME_SIZE) {
            candidate[start + offset] = sample * scale;
        }
    }

    Ok(candidate)
}

fn run_frame_stability_candidate(
    source_samples: &[f32],
    baseline_samples: &[f32],
    source_wav: &[u8],
    payload: &WatermarkPayload,
    anchor: &WatermarkPayloadV3MinimalAnchor,
    out_dir: &Path,
) -> Result<CandidateResult, String> {
    let ratios = [0.20_f32, 0.15, 0.10, 0.06, 0.03];
    let mut best: Option<CandidateResult> = None;
    let mut best_wav = Vec::new();

    for ratio in ratios {
        let candidate_samples =
            build_frame_stability_window_sparse_candidate(source_samples, baseline_samples, ratio);
        let candidate_wav = encode_wav(&candidate_samples)?;
        let result = evaluate_candidate(
            "frame_stability_window_sparse",
            "revert_lowest_segment_snr_frames_with_existing_extractor",
            source_samples,
            &candidate_samples,
            &candidate_wav,
            payload,
            anchor,
            Some(ratio),
        )?;
        if result.invariant_passed {
            let replace = best
                .as_ref()
                .map(|current| result.metrics.snr > current.metrics.snr)
                .unwrap_or(true);
            if replace {
                best = Some(result);
                best_wav = candidate_wav;
            }
        }
    }

    let Some(result) = best else {
        let fallback_samples =
            build_frame_stability_window_sparse_candidate(source_samples, baseline_samples, 0.03);
        let fallback_wav = encode_wav(&fallback_samples)?;
        write_candidate_wavs(
            out_dir,
            "frame_stability_window_sparse",
            source_wav,
            &fallback_wav,
        )?;
        return evaluate_candidate(
            "frame_stability_window_sparse",
            "revert_lowest_segment_snr_frames_with_existing_extractor",
            source_samples,
            &fallback_samples,
            &fallback_wav,
            payload,
            anchor,
            Some(0.03),
        );
    };

    write_candidate_wavs(
        out_dir,
        "frame_stability_window_sparse",
        source_wav,
        &best_wav,
    )?;
    Ok(result)
}

fn build_frame_stability_window_sparse_candidate(
    source: &[f32],
    baseline: &[f32],
    revert_ratio: f32,
) -> Vec<f32> {
    let len = source.len().min(baseline.len());
    let frame_count = len / FRAME_SIZE;
    let mut frames = (0..frame_count)
        .map(|frame_idx| {
            let start = frame_idx * FRAME_SIZE;
            let end = start + FRAME_SIZE;
            let segment = start / AUDIO_SAMPLE_RATE;
            let frame_snr = audio_snr(&source[start..end], &baseline[start..end]).unwrap_or(0.0);
            let diff_energy = source[start..end]
                .iter()
                .zip(baseline[start..end].iter())
                .map(|(left, right)| {
                    let delta = f64::from(*left) - f64::from(*right);
                    delta * delta
                })
                .sum::<f64>();
            FrameScore {
                frame_idx,
                segment,
                frame_snr,
                diff_energy,
            }
        })
        .collect::<Vec<_>>();
    frames.sort_by(|left, right| {
        left.frame_snr
            .partial_cmp(&right.frame_snr)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .diff_energy
                    .partial_cmp(&left.diff_energy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.segment.cmp(&right.segment))
    });

    let revert_count = ((frame_count as f32 * revert_ratio).round() as usize).max(1);
    let mut candidate = baseline.to_vec();
    for score in frames.iter().take(revert_count) {
        let start = score.frame_idx * FRAME_SIZE;
        let end = start + FRAME_SIZE;
        candidate[start..end].copy_from_slice(&source[start..end]);
    }
    candidate
}

fn run_masked_pair_budget_candidate(
    source_samples: &[f32],
    baseline_samples: &[f32],
    source_wav: &[u8],
    payload: &WatermarkPayload,
    anchor: &WatermarkPayloadV3MinimalAnchor,
    out_dir: &Path,
) -> Result<CandidateResult, String> {
    let budget_keep_ratios = [0.995_f32, 0.990];
    let mut best: Option<CandidateResult> = None;
    let mut best_wav = Vec::new();

    for keep_ratio in budget_keep_ratios {
        let candidate_samples =
            build_masked_pair_budget_cap_candidate(source_samples, baseline_samples, keep_ratio)?;
        let candidate_wav = encode_wav(&candidate_samples)?;
        let result = evaluate_candidate(
            "masked_pair_budget_cap",
            "cap_highest_diff_recovery_pairs_per_second_with_existing_extractor",
            source_samples,
            &candidate_samples,
            &candidate_wav,
            payload,
            anchor,
            Some(keep_ratio),
        )?;
        if result.invariant_passed {
            let replace = best
                .as_ref()
                .map(|current| {
                    result
                        .metrics
                        .perceptual_diagnosis
                        .band_energy_share
                        .watermark
                        .noise
                        < current
                            .metrics
                            .perceptual_diagnosis
                            .band_energy_share
                            .watermark
                            .noise
                        || (result
                            .metrics
                            .perceptual_diagnosis
                            .band_energy_share
                            .watermark
                            .noise
                            == current
                                .metrics
                                .perceptual_diagnosis
                                .band_energy_share
                                .watermark
                                .noise
                            && result.metrics.snr > current.metrics.snr)
                })
                .unwrap_or(true);
            if replace {
                best = Some(result);
                best_wav = candidate_wav;
            }
        }
    }

    let Some(result) = best else {
        let fallback_samples =
            build_masked_pair_budget_cap_candidate(source_samples, baseline_samples, 0.995)?;
        let fallback_wav = encode_wav(&fallback_samples)?;
        write_candidate_wavs(out_dir, "masked_pair_budget_cap", source_wav, &fallback_wav)?;
        return evaluate_candidate(
            "masked_pair_budget_cap",
            "cap_highest_diff_recovery_pairs_per_second_with_existing_extractor",
            source_samples,
            &fallback_samples,
            &fallback_wav,
            payload,
            anchor,
            Some(0.995),
        );
    };

    write_candidate_wavs(out_dir, "masked_pair_budget_cap", source_wav, &best_wav)?;
    Ok(result)
}

fn build_masked_pair_budget_cap_candidate(
    source: &[f32],
    baseline: &[f32],
    keep_ratio: f32,
) -> Result<Vec<f32>, String> {
    let len = source.len().min(baseline.len());
    let frame_count = len / FRAME_SIZE;
    let mut pair_scores = score_recovery_pairs_by_segment(source, baseline)?;
    let mut keep_pair =
        vec![true; frame_count * AUDIO_RECOVERY_BITS_PER_FRAME * AUDIO_RECOVERY_BIT_LANES];

    pair_scores.sort_by(|left, right| {
        left.segment
            .cmp(&right.segment)
            .then_with(|| {
                right
                    .diff_energy
                    .partial_cmp(&left.diff_energy)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.frame_idx.cmp(&right.frame_idx))
            .then_with(|| left.bit_slot.cmp(&right.bit_slot))
            .then_with(|| left.lane.cmp(&right.lane))
    });

    let mut segment_start = 0usize;
    while segment_start < pair_scores.len() {
        let segment = pair_scores[segment_start].segment;
        let mut segment_end = segment_start + 1;
        while segment_end < pair_scores.len() && pair_scores[segment_end].segment == segment {
            segment_end += 1;
        }

        let segment_len = segment_end - segment_start;
        let keep_count = ((segment_len as f32 * keep_ratio).ceil() as usize).min(segment_len);
        let revert_count = segment_len.saturating_sub(keep_count);
        for score in &pair_scores[segment_start..segment_start + revert_count] {
            keep_pair[score.keep_index] = false;
        }

        segment_start = segment_end;
    }

    let mut candidate = baseline.to_vec();
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let ifft = planner.plan_fft_inverse(FRAME_SIZE);

    for frame_idx in 0..frame_count {
        let start = frame_idx * FRAME_SIZE;
        let end = start + FRAME_SIZE;
        let mut source_input = source[start..end].to_vec();
        let mut baseline_input = baseline[start..end].to_vec();
        let mut source_spectrum = fft.make_output_vec();
        let mut candidate_spectrum = fft.make_output_vec();
        fft.process(&mut source_input, &mut source_spectrum)
            .map_err(|error| format!("FFT source failed: {error}"))?;
        fft.process(&mut baseline_input, &mut candidate_spectrum)
            .map_err(|error| format!("FFT baseline failed: {error}"))?;

        for bit_slot in 0..AUDIO_RECOVERY_BITS_PER_FRAME {
            for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                let keep_index = recovery_keep_index(frame_idx, bit_slot, lane);
                if keep_pair.get(keep_index).copied() == Some(false) {
                    let pair_idx =
                        AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                    let bin = BAND_LO_BIN + pair_idx * RELATIVE_PAIR_WIDTH;
                    copy_pair_bins(&source_spectrum, &mut candidate_spectrum, bin);
                }
            }
        }

        let mut output = ifft.make_output_vec();
        ifft.process(&mut candidate_spectrum, &mut output)
            .map_err(|error| format!("IFFT candidate failed: {error}"))?;
        let scale = 1.0 / FRAME_SIZE as f32;
        for (offset, sample) in output.iter().enumerate().take(FRAME_SIZE) {
            candidate[start + offset] = sample * scale;
        }
    }

    Ok(candidate)
}

fn score_recovery_pairs_by_segment(
    source: &[f32],
    baseline: &[f32],
) -> Result<Vec<PairBudgetScore>, String> {
    let len = source.len().min(baseline.len());
    let frame_count = len / FRAME_SIZE;
    let mut scores =
        Vec::with_capacity(frame_count * AUDIO_RECOVERY_BITS_PER_FRAME * AUDIO_RECOVERY_BIT_LANES);
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    for frame_idx in 0..frame_count {
        let start = frame_idx * FRAME_SIZE;
        let end = start + FRAME_SIZE;
        let mut source_input = source[start..end].to_vec();
        let mut baseline_input = baseline[start..end].to_vec();
        let mut source_spectrum = fft.make_output_vec();
        let mut baseline_spectrum = fft.make_output_vec();
        fft.process(&mut source_input, &mut source_spectrum)
            .map_err(|error| format!("FFT source failed: {error}"))?;
        fft.process(&mut baseline_input, &mut baseline_spectrum)
            .map_err(|error| format!("FFT baseline failed: {error}"))?;

        for bit_slot in 0..AUDIO_RECOVERY_BITS_PER_FRAME {
            for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                let pair_idx =
                    AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                let bin = BAND_LO_BIN + pair_idx * RELATIVE_PAIR_WIDTH;
                scores.push(PairBudgetScore {
                    frame_idx,
                    segment: start / AUDIO_SAMPLE_RATE,
                    bit_slot,
                    lane,
                    keep_index: recovery_keep_index(frame_idx, bit_slot, lane),
                    diff_energy: pair_diff_energy(&source_spectrum, &baseline_spectrum, bin),
                });
            }
        }
    }

    Ok(scores)
}

fn recovery_keep_index(frame_idx: usize, bit_slot: usize, lane: usize) -> usize {
    (frame_idx * AUDIO_RECOVERY_BITS_PER_FRAME + bit_slot) * AUDIO_RECOVERY_BIT_LANES + lane
}

fn evaluate_candidate(
    candidate_id: &'static str,
    strategy: &'static str,
    source_samples: &[f32],
    protected_samples: &[f32],
    protected_wav: &[u8],
    payload: &WatermarkPayload,
    anchor: &WatermarkPayloadV3MinimalAnchor,
    candidate_parameter: Option<f32>,
) -> Result<CandidateResult, String> {
    let decoded = extract_decoded(protected_wav).ok();
    let decoded_uid = decoded.as_ref().map(WatermarkDecodedPayload::watermark_uid);
    let extract_passed = decoded_uid.as_deref() == Some(payload.watermark_uid().as_str());
    let diagnostics = audio_v3_quality_diagnostics(source_samples, protected_samples, anchor)
        .map_err(|error| format!("audio diagnostics {candidate_id}: {error}"))?;
    let perceptual = audio_perceptual_diagnosis(source_samples, protected_samples)?;
    let snr = audio_snr(source_samples, protected_samples)?;
    let peak_delta = (peak_abs(source_samples) - peak_abs(protected_samples)).abs();
    let lufs_delta = (approx_lufs(source_samples) - approx_lufs(protected_samples)).abs();
    let new_clipping = new_clipping_samples(source_samples, protected_samples);
    let official_quality_passed = snr >= AUDIO_FULL_MIN_SNR
        && peak_delta <= AUDIO_FULL_MAX_PEAK_DELTA
        && lufs_delta <= AUDIO_FULL_MAX_LUFS_DELTA
        && new_clipping <= AUDIO_MAX_NEW_CLIPPING;
    let release_blocking_reason = if !extract_passed {
        "extract_failed"
    } else if snr < AUDIO_FULL_MIN_SNR {
        "snr_below_threshold"
    } else if lufs_delta > AUDIO_FULL_MAX_LUFS_DELTA {
        "lufs_delta_above_threshold"
    } else if peak_delta > AUDIO_FULL_MAX_PEAK_DELTA {
        "peak_delta_above_threshold"
    } else if new_clipping > AUDIO_MAX_NEW_CLIPPING {
        "new_clipping"
    } else {
        "none"
    };
    let copyright_id_format_valid = decoded_uid.as_deref().map(is_long_hs_uid).unwrap_or(false);
    let invariant_passed = extract_passed
        && diagnostics.extraction_confidence >= MIN_EXTRACTION_CONFIDENCE
        && diagnostics.noise_floor_sparse_recovery
        && diagnostics.modified_pair_ratio <= BASELINE_MODIFIED_PAIR_RATIO + 0.01
        && copyright_id_format_valid;

    Ok(CandidateResult {
        candidate_id,
        strategy,
        status: if invariant_passed {
            "invariant_passed"
        } else {
            "blocked_by_invariant"
        },
        profile_matched: diagnostics.noise_floor_sparse_recovery,
        candidate_parameter,
        payload_protocol_version: decoded
            .as_ref()
            .map(WatermarkDecodedPayload::protocol_version)
            .unwrap_or(0),
        payload_bytes_length: decoded
            .as_ref()
            .map(WatermarkDecodedPayload::payload_bytes_length)
            .unwrap_or(0),
        decoded_watermark_uid: decoded_uid.unwrap_or_else(|| "unreadable".to_string()),
        copyright_id_format_valid,
        extract_passed,
        invariant_passed,
        official_quality_passed,
        release_blocking_reason,
        metrics: CandidateMetrics {
            snr,
            min_snr: AUDIO_FULL_MIN_SNR,
            peak_delta,
            max_peak_delta: AUDIO_FULL_MAX_PEAK_DELTA,
            lufs_delta,
            max_lufs_delta: AUDIO_FULL_MAX_LUFS_DELTA,
            new_clipping,
            max_new_clipping: AUDIO_MAX_NEW_CLIPPING,
            debug: DebugMetrics::from(diagnostics),
            perceptual_diagnosis: perceptual,
        },
    })
}

fn embed_baseline(source_wav: &[u8], payload: &WatermarkPayload) -> Result<Vec<u8>, String> {
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_wav.to_vec(),
        },
        payload,
        EmbedOptions {
            allow_rewrite: true,
            audio_protection_mode: AudioProtectionMode::StandaloneAudio,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed baseline audio: {error}"))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("baseline embed returned non-audio output".to_string());
    };
    Ok(bytes)
}

fn extract_decoded(wav: &[u8]) -> Result<WatermarkDecodedPayload, String> {
    WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: wav.to_vec(),
    })
    .map_err(|error| format!("extract audio: {error}"))
}

fn write_candidate_wavs(
    out_dir: &Path,
    candidate_id: &str,
    control_wav: &[u8],
    protected_wav: &[u8],
) -> Result<(), String> {
    let dir = out_dir.join("candidates").join(candidate_id);
    fs::create_dir_all(&dir).map_err(|error| format!("create candidate dir: {error}"))?;
    fs::write(dir.join("control.wav"), control_wav)
        .map_err(|error| format!("write control wav: {error}"))?;
    fs::write(dir.join("protected.wav"), protected_wav)
        .map_err(|error| format!("write protected wav: {error}"))
}

fn write_abx_template(out_dir: &Path, report: &ExperimentReport<'_>) -> Result<(), String> {
    let mut csv = String::from("runId,candidateId,sampleId,profile,device,environment,trial,A,B,X,answer,correct,confidence_1_5,perceivedDifference,notes\n");
    for candidate in &report.candidates {
        for device in ["headphones", "speaker"] {
            for environment in ["quiet-room", "office"] {
                for trial in 1..=3 {
                    let (a, b, x) = if trial % 2 == 0 {
                        ("protected", "original", "B")
                    } else {
                        ("original", "protected", "A")
                    };
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{},{},,,,,\n",
                        report.run_id,
                        candidate.candidate_id,
                        report.sample_id,
                        report.source_profile,
                        device,
                        environment,
                        trial,
                        a,
                        b,
                        x
                    ));
                }
            }
        }
    }
    fs::write(
        out_dir
            .join("abx")
            .join("audio_noise_floor_band_selection_abx_trials.csv"),
        csv,
    )
    .map_err(|error| format!("write ABX template: {error}"))
}

fn render_markdown(report: &ExperimentReport<'_>) -> String {
    let mut markdown = format!(
        "# Audio Noise Floor Band Selection Experiment\n\n- runId: `{}`\n- experiment: `{}`\n- status: `{}`\n- sample: `{}` / `{}`\n- payload: `V3 / {} bytes`\n- expectedWatermarkUid: `{}`\n- fullGateSnrThreshold: `{:.1} dB`\n- ABX template: `{}`\n\n",
        report.run_id,
        report.experiment,
        report.status,
        report.sample_id,
        report.source_profile,
        report.payload_bytes_length,
        report.expected_watermark_uid,
        report.constraints.full_gate_snr_threshold_db,
        report.abx_template
    );
    markdown.push_str("## Constraints\n\n");
    markdown.push_str("| constraint | value |\n| --- | --- |\n");
    markdown.push_str(&format!(
        "| payload format unchanged | {} |\n| copyright ID unchanged | {} |\n| cross-end readability unchanged | {} |\n| extraction confidence floor | {:.2} |\n| quality threshold unchanged | {} |\n| formal UI / mock touched | {} |\n\n",
        report.constraints.payload_format_unchanged,
        report.constraints.copyright_id_unchanged,
        report.constraints.cross_end_readability_unchanged,
        report.constraints.extraction_confidence_floor,
        report.constraints.quality_threshold_unchanged,
        report.constraints.official_ui_or_mock_path_touched
    ));
    markdown.push_str("## Candidate Results\n\n");
    markdown.push_str("| candidate | status | profile | extract | confidence | SNR | blocking | watermark noise share | modified pair ratio | parameter |\n");
    markdown.push_str("| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: |\n");
    for candidate in &report.candidates {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {:.6} | {:.4} | {} | {:.6} | {:.6} | {} |\n",
            candidate.candidate_id,
            candidate.status,
            candidate.profile_matched,
            candidate.extract_passed,
            candidate.metrics.debug.extraction_confidence,
            candidate.metrics.snr,
            candidate.release_blocking_reason,
            candidate
                .metrics
                .perceptual_diagnosis
                .band_energy_share
                .watermark
                .noise,
            candidate
                .metrics
                .debug
                .embedding_strength
                .modified_pair_ratio,
            candidate
                .candidate_parameter
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "n/a".to_string())
        ));
    }
    markdown.push_str("\n## Planned Candidates\n\n");
    markdown.push_str("| candidate | status | reason |\n| --- | --- | --- |\n");
    for candidate in &report.planned_candidates {
        markdown.push_str(&format!(
            "| {} | {} | {} |\n",
            candidate.candidate_id, candidate.status, candidate.reason
        ));
    }
    markdown.push_str("\n## Recommendation\n\n");
    markdown.push_str(report.recommendation);
    markdown.push('\n');
    markdown
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExperimentReport<'a> {
    run_id: &'a str,
    experiment: &'a str,
    status: &'a str,
    sample_id: &'a str,
    source_profile: &'a str,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    expected_watermark_uid: String,
    constraints: Constraints,
    candidates: Vec<CandidateResult>,
    planned_candidates: Vec<PlannedCandidate>,
    abx_template: &'a str,
    recommendation: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Constraints {
    payload_format_unchanged: bool,
    copyright_id_unchanged: bool,
    cross_end_readability_unchanged: bool,
    extraction_confidence_floor: f32,
    quality_threshold_unchanged: bool,
    full_gate_snr_threshold_db: f64,
    official_ui_or_mock_path_touched: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlannedCandidate {
    candidate_id: &'static str,
    status: &'static str,
    reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateResult {
    candidate_id: &'static str,
    strategy: &'static str,
    status: &'static str,
    profile_matched: bool,
    candidate_parameter: Option<f32>,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    decoded_watermark_uid: String,
    copyright_id_format_valid: bool,
    extract_passed: bool,
    invariant_passed: bool,
    official_quality_passed: bool,
    release_blocking_reason: &'static str,
    metrics: CandidateMetrics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateMetrics {
    snr: f64,
    min_snr: f64,
    peak_delta: f64,
    max_peak_delta: f64,
    lufs_delta: f64,
    max_lufs_delta: f64,
    new_clipping: usize,
    max_new_clipping: usize,
    debug: DebugMetrics,
    perceptual_diagnosis: AudioPerceptualDiagnosis,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugMetrics {
    frame_count: usize,
    short_time_rms: ShortTimeRms,
    low_energy_frame_ratio: f32,
    transient_frame_ratio: f32,
    noise_like_frame_ratio: f32,
    embedding_strength: EmbeddingStrength,
    noise_floor_sparse_recovery: bool,
    extraction_confidence: f32,
}

impl From<AudioV3QualityDiagnostics> for DebugMetrics {
    fn from(value: AudioV3QualityDiagnostics) -> Self {
        Self {
            frame_count: value.frame_count,
            short_time_rms: ShortTimeRms {
                min: value.short_time_rms_min,
                mean: value.short_time_rms_mean,
                max: value.short_time_rms_max,
            },
            low_energy_frame_ratio: value.low_energy_frame_ratio,
            transient_frame_ratio: value.transient_frame_ratio,
            noise_like_frame_ratio: value.noise_like_frame_ratio,
            embedding_strength: EmbeddingStrength {
                min_contrast: value.embedding_strength_min,
                mean_contrast: value.embedding_strength_mean,
                max_contrast: value.embedding_strength_max,
                modified_pair_ratio: value.modified_pair_ratio,
            },
            noise_floor_sparse_recovery: value.noise_floor_sparse_recovery,
            extraction_confidence: value.extraction_confidence,
        }
    }
}

#[derive(Serialize)]
struct ShortTimeRms {
    min: f32,
    mean: f32,
    max: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddingStrength {
    min_contrast: f32,
    mean_contrast: f32,
    max_contrast: f32,
    modified_pair_ratio: f32,
}

#[derive(Clone)]
struct FrameScore {
    frame_idx: usize,
    segment: usize,
    frame_snr: f64,
    diff_energy: f64,
}

#[derive(Clone)]
struct PairBudgetScore {
    frame_idx: usize,
    segment: usize,
    bit_slot: usize,
    lane: usize,
    keep_index: usize,
    diff_energy: f64,
}

#[derive(Clone)]
struct LaneScore {
    lane: usize,
    signal_energy: f64,
    diff_energy: f64,
    center_distance: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioPerceptualDiagnosis {
    segmented_snr: SegmentedSnr,
    band_energy_share: BandEnergyShare,
    dominant_noise_band: &'static str,
    diagnosis: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SegmentedSnr {
    segment_seconds: usize,
    segment_count: usize,
    min: f64,
    mean: f64,
    max: f64,
    first: f64,
    middle: f64,
    last: f64,
    spread: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BandEnergyShare {
    low: BandShare,
    watermark: BandShare,
    high: BandShare,
}

#[derive(Serialize)]
struct BandShare {
    signal: f64,
    noise: f64,
}

struct AudioBandEnergy {
    name: &'static str,
    lo_hz: f64,
    hi_hz: f64,
    signal_energy: f64,
    noise_energy: f64,
}

fn audio_perceptual_diagnosis(
    source: &[f32],
    protected: &[f32],
) -> Result<AudioPerceptualDiagnosis, String> {
    let segment_samples = AUDIO_SAMPLE_RATE * AUDIO_DIAGNOSTIC_SEGMENT_SECONDS;
    let len = source.len().min(protected.len());
    if len < segment_samples {
        return Err("audio too short for segmented SNR diagnosis".to_string());
    }
    let mut segment_snrs = Vec::new();
    for start in (0..len).step_by(segment_samples) {
        let end = (start + segment_samples).min(len);
        if end - start < segment_samples / 2 {
            continue;
        }
        segment_snrs.push(audio_snr(&source[start..end], &protected[start..end])?);
    }
    let min = segment_snrs
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max = segment_snrs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let mean = segment_snrs.iter().sum::<f64>() / segment_snrs.len() as f64;
    let first = segment_snrs[0];
    let middle = segment_snrs[segment_snrs.len() / 2];
    let last = *segment_snrs.last().unwrap_or(&middle);
    let spread = max - min;

    let bands = audio_band_energy_diagnosis(source, protected)?;
    let signal_total = bands
        .iter()
        .map(|band| band.signal_energy)
        .sum::<f64>()
        .max(1e-18);
    let noise_total = bands
        .iter()
        .map(|band| band.noise_energy)
        .sum::<f64>()
        .max(1e-18);
    let share = |energy: f64, total: f64| energy / total;
    let low = &bands[0];
    let watermark = &bands[1];
    let high = &bands[2];
    let dominant = bands
        .iter()
        .max_by(|left, right| {
            left.noise_energy
                .partial_cmp(&right.noise_energy)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| "no audio bands available for diagnosis".to_string())?;
    let watermark_noise_share = share(watermark.noise_energy, noise_total);
    let watermark_signal_share = share(watermark.signal_energy, signal_total);
    let diagnosis =
        if watermark_noise_share >= 0.55 && watermark_noise_share > watermark_signal_share * 1.4 {
            "specific_watermark_band_energy_redistribution"
        } else if spread <= 3.0 && (watermark_noise_share - watermark_signal_share).abs() <= 0.20 {
            "full_band_noise_floor_statistical_amplification"
        } else {
            "audible_distortion_not_indicated_by_objective_diagnostic"
        };

    Ok(AudioPerceptualDiagnosis {
        segmented_snr: SegmentedSnr {
            segment_seconds: AUDIO_DIAGNOSTIC_SEGMENT_SECONDS,
            segment_count: segment_snrs.len(),
            min,
            mean,
            max,
            first,
            middle,
            last,
            spread,
        },
        band_energy_share: BandEnergyShare {
            low: BandShare {
                signal: share(low.signal_energy, signal_total),
                noise: share(low.noise_energy, noise_total),
            },
            watermark: BandShare {
                signal: watermark_signal_share,
                noise: watermark_noise_share,
            },
            high: BandShare {
                signal: share(high.signal_energy, signal_total),
                noise: share(high.noise_energy, noise_total),
            },
        },
        dominant_noise_band: dominant.name,
        diagnosis,
    })
}

fn audio_band_energy_diagnosis(
    source: &[f32],
    protected: &[f32],
) -> Result<Vec<AudioBandEnergy>, String> {
    let len = source.len().min(protected.len());
    if len < AUDIO_DIAGNOSTIC_FFT_SIZE {
        return Err("audio too short for band-energy diagnosis".to_string());
    }
    let mut bands = vec![
        AudioBandEnergy {
            name: "low",
            lo_hz: AUDIO_LOW_BAND_HZ.0,
            hi_hz: AUDIO_LOW_BAND_HZ.1,
            signal_energy: 0.0,
            noise_energy: 0.0,
        },
        AudioBandEnergy {
            name: "watermark",
            lo_hz: AUDIO_WATERMARK_BAND_HZ.0,
            hi_hz: AUDIO_WATERMARK_BAND_HZ.1,
            signal_energy: 0.0,
            noise_energy: 0.0,
        },
        AudioBandEnergy {
            name: "high",
            lo_hz: AUDIO_HIGH_BAND_HZ.0,
            hi_hz: AUDIO_HIGH_BAND_HZ.1,
            signal_energy: 0.0,
            noise_energy: 0.0,
        },
    ];
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(AUDIO_DIAGNOSTIC_FFT_SIZE);
    for start in (0..=len - AUDIO_DIAGNOSTIC_FFT_SIZE).step_by(AUDIO_DIAGNOSTIC_FFT_SIZE) {
        let end = start + AUDIO_DIAGNOSTIC_FFT_SIZE;
        let mut signal_input = source[start..end]
            .iter()
            .map(|sample| f64::from(*sample))
            .collect::<Vec<_>>();
        let mut noise_input = source[start..end]
            .iter()
            .zip(protected[start..end].iter())
            .map(|(left, right)| f64::from(*right) - f64::from(*left))
            .collect::<Vec<_>>();
        let mut signal_spectrum = fft.make_output_vec();
        let mut noise_spectrum = fft.make_output_vec();
        fft.process(&mut signal_input, &mut signal_spectrum)
            .map_err(|error| format!("FFT failed: {error}"))?;
        fft.process(&mut noise_input, &mut noise_spectrum)
            .map_err(|error| format!("FFT failed: {error}"))?;

        for bin in 1..signal_spectrum.len() {
            let hz = bin as f64 * AUDIO_SAMPLE_RATE as f64 / AUDIO_DIAGNOSTIC_FFT_SIZE as f64;
            for band in &mut bands {
                if hz >= band.lo_hz && hz < band.hi_hz {
                    band.signal_energy += signal_spectrum[bin].norm_sqr();
                    band.noise_energy += noise_spectrum[bin].norm_sqr();
                }
            }
        }
    }
    Ok(bands)
}

fn pair_energy(spectrum: &[realfft::num_complex::Complex<f32>], bin: usize) -> f64 {
    (0..RELATIVE_PAIR_WIDTH)
        .filter_map(|offset| spectrum.get(bin + offset))
        .map(|value| f64::from(value.norm_sqr()))
        .sum()
}

fn pair_diff_energy(
    source: &[realfft::num_complex::Complex<f32>],
    baseline: &[realfft::num_complex::Complex<f32>],
    bin: usize,
) -> f64 {
    (0..RELATIVE_PAIR_WIDTH)
        .filter_map(|offset| source.get(bin + offset).zip(baseline.get(bin + offset)))
        .map(|(left, right)| {
            let delta = *right - *left;
            f64::from(delta.norm_sqr())
        })
        .sum()
}

fn copy_pair_bins(
    source: &[realfft::num_complex::Complex<f32>],
    target: &mut [realfft::num_complex::Complex<f32>],
    bin: usize,
) {
    for offset in 0..RELATIVE_PAIR_WIDTH {
        if let Some(value) = source.get(bin + offset).copied() {
            if let Some(slot) = target.get_mut(bin + offset) {
                *slot = value;
            }
        }
    }
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

fn samples_for_seconds(seconds: u32, sample_fn: fn(f32) -> f32) -> Vec<f32> {
    let total = AUDIO_SAMPLE_RATE * seconds as usize;
    (0..total)
        .map(|index| sample_fn(index as f32 / AUDIO_SAMPLE_RATE as f32).clamp(-0.95, 0.95))
        .collect()
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: AUDIO_SAMPLE_RATE as u32,
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

fn new_clipping_samples(source: &[f32], embedded: &[f32]) -> usize {
    let len = source.len().min(embedded.len());
    (0..len)
        .filter(|index| source[*index].abs() < 0.999 && embedded[*index].abs() >= 0.999)
        .count()
}

fn build_payload(
    run_id: &str,
    sample_id: &str,
    media_bytes: &[u8],
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{sample_id}:noise-band").as_bytes());
    let original_sha256: [u8; 32] = Sha256::digest(media_bytes).into();
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_783_036_800,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type: WatermarkMediaType::Audio,
        registry_proof_hash: None,
        creator_binding: Some("hidden-shield-audio-noise-band-experiment"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut output = [0u8; 16];
    output.copy_from_slice(&digest[..16]);
    output
}

fn is_long_hs_uid(uid: &str) -> bool {
    let parts = uid.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts[0] == "HS"
        && parts[1..].iter().all(|part| {
            part.len() == 8
                && part.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_lowercase()
                })
        })
}

fn optional_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
