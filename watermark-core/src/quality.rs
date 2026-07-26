use image::{ImageBuffer, Rgb};
use realfft::RealFftPlanner;
use serde::{Deserialize, Serialize};

const AUDIO_DIAGNOSTIC_FFT_SIZE: usize = 4096;
const AUDIO_DIAGNOSTIC_SEGMENT_SECONDS: usize = 1;
const AUDIO_LOW_BAND_HZ: (f64, f64) = (20.0, 1_500.0);
const AUDIO_WATERMARK_BAND_HZ: (f64, f64) = (2_000.0, 8_000.0);
const AUDIO_HIGH_BAND_HZ: (f64, f64) = (8_000.0, 16_000.0);
const SILENCE_RMS_THRESHOLD: f64 = 0.001;

pub const IMAGE_RELEASE_MIN_PSNR: f64 = 32.0;
pub const IMAGE_RELEASE_MIN_SSIM: f64 = 0.985;
pub const IMAGE_FORENSIC_MIN_PSNR: f64 = 38.0;
pub const IMAGE_FORENSIC_MIN_SSIM: f64 = 0.990;
pub const IMAGE_BALANCED_MIN_PSNR: f64 = 42.0;
pub const IMAGE_BALANCED_MIN_SSIM: f64 = 0.995;

pub const AUDIO_RELEASE_MIN_SNR: f64 = 45.0;
pub const AUDIO_RELEASE_MAX_PEAK_DELTA: f64 = 0.08;
pub const AUDIO_RELEASE_MAX_LUFS_DELTA: f64 = 0.8;
pub const AUDIO_FORENSIC_MIN_SNR: f64 = 44.0;
pub const AUDIO_FORENSIC_MAX_PEAK_DELTA: f64 = 0.8;
pub const AUDIO_FORENSIC_MAX_LUFS_DELTA: f64 = 0.5;
pub const AUDIO_BALANCED_MIN_SNR: f64 = 50.0;
pub const AUDIO_BALANCED_MAX_PEAK_DELTA: f64 = 0.5;
pub const AUDIO_BALANCED_MAX_LUFS_DELTA: f64 = 0.3;
pub const AUDIO_MAX_NEW_CLIPPING: usize = 0;

#[derive(Debug, Clone, Copy)]
pub struct ImageQualityInput<'a> {
    pub source: &'a ImageBuffer<Rgb<u8>, Vec<u8>>,
    pub candidate: &'a ImageBuffer<Rgb<u8>, Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioQualityInput<'a> {
    pub source: &'a [f32],
    pub candidate: &'a [f32],
    pub sample_rate: usize,
    pub channels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityThresholdProfile {
    ReleaseSmoke,
    ForensicDefault,
    BalancedCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityThresholdResult {
    pub profile: QualityThresholdProfile,
    pub passed: bool,
    pub blocking_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageQualityReport {
    pub width: u32,
    pub height: u32,
    pub psnr: f64,
    pub ssim: f64,
    pub mae: f64,
    pub p95_absolute_difference: f64,
    pub max_channel_difference: u8,
    pub changed_pixel_ratio: f64,
    pub release: QualityThresholdResult,
    pub forensic: QualityThresholdResult,
    pub balanced: QualityThresholdResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBandEnergyReport {
    pub low_signal_share: f64,
    pub low_noise_share: f64,
    pub watermark_signal_share: f64,
    pub watermark_noise_share: f64,
    pub high_signal_share: f64,
    pub high_noise_share: f64,
    pub dominant_noise_band: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSegmentSnrReport {
    pub segment_seconds: usize,
    pub segment_count: usize,
    pub min: f64,
    pub mean: f64,
    pub max: f64,
    pub first: f64,
    pub middle: f64,
    pub last: f64,
    pub spread: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPerceptualDiagnosis {
    pub segmented_snr: AudioSegmentSnrReport,
    pub band_energy: AudioBandEnergyReport,
    pub diagnosis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioQualityReport {
    pub sample_rate: usize,
    pub channels: usize,
    pub compared_samples: usize,
    pub snr: f64,
    pub peak_delta: f64,
    pub lufs_delta: f64,
    pub new_clipping: usize,
    pub silence_noise_floor_delta: f64,
    pub perceptual_diagnosis: AudioPerceptualDiagnosis,
    pub release: QualityThresholdResult,
    pub forensic: QualityThresholdResult,
    pub balanced: QualityThresholdResult,
}

pub fn compare_image_quality(input: ImageQualityInput<'_>) -> Result<ImageQualityReport, String> {
    if input.source.dimensions() != input.candidate.dimensions() {
        return Err("image dimensions differ".to_string());
    }

    let psnr = image_psnr(input.source, input.candidate)?;
    let ssim = image_ssim(input.source, input.candidate)?;
    let mut difference_histogram = [0_usize; 256];
    let mut sum = 0_u64;
    let mut max_channel_difference = 0_u8;
    let mut changed_pixels = 0_usize;
    let mut channel_count = 0_usize;

    for (source, candidate) in input.source.pixels().zip(input.candidate.pixels()) {
        let mut changed = false;
        for channel in 0..3 {
            let difference = source[channel].abs_diff(candidate[channel]);
            sum += u64::from(difference);
            max_channel_difference = max_channel_difference.max(difference);
            changed |= difference != 0;
            difference_histogram[difference as usize] += 1;
            channel_count += 1;
        }
        if changed {
            changed_pixels += 1;
        }
    }

    let p95_rank = ((channel_count.saturating_sub(1)) as f64 * 0.95).round() as usize;
    let mut cumulative = 0_usize;
    let mut p95_absolute_difference = 0.0;
    for (difference, count) in difference_histogram.into_iter().enumerate() {
        cumulative += count;
        if cumulative > p95_rank {
            p95_absolute_difference = difference as f64;
            break;
        }
    }
    let channel_count = channel_count.max(1);
    let pixel_count = (input.source.width() as usize * input.source.height() as usize).max(1);

    Ok(ImageQualityReport {
        width: input.source.width(),
        height: input.source.height(),
        psnr,
        ssim,
        mae: sum as f64 / channel_count as f64,
        p95_absolute_difference,
        max_channel_difference,
        changed_pixel_ratio: changed_pixels as f64 / pixel_count as f64,
        release: image_threshold_result(QualityThresholdProfile::ReleaseSmoke, psnr, ssim),
        forensic: image_threshold_result(QualityThresholdProfile::ForensicDefault, psnr, ssim),
        balanced: image_threshold_result(QualityThresholdProfile::BalancedCandidate, psnr, ssim),
    })
}

pub fn compare_audio_quality(input: AudioQualityInput<'_>) -> Result<AudioQualityReport, String> {
    let len = input.source.len().min(input.candidate.len());
    if len == 0 {
        return Err("empty audio samples".to_string());
    }
    if input.sample_rate == 0 {
        return Err("audio sample rate must be positive".to_string());
    }
    if input.channels == 0 {
        return Err("audio channels must be positive".to_string());
    }

    let source = &input.source[..len];
    let candidate = &input.candidate[..len];
    let snr = audio_snr(source, candidate)?;
    let peak_delta = (peak_abs(source) - peak_abs(candidate)).abs();
    let lufs_delta = (approx_lufs(source) - approx_lufs(candidate)).abs();
    let new_clipping = new_clipping_samples(source, candidate);
    let silence_noise_floor_delta = silence_noise_floor_delta(source, candidate);
    let source_mono = downmix_to_mono(source, input.channels);
    let candidate_mono = downmix_to_mono(candidate, input.channels);
    let perceptual_diagnosis =
        audio_perceptual_diagnosis(&source_mono, &candidate_mono, input.sample_rate)?;

    Ok(AudioQualityReport {
        sample_rate: input.sample_rate,
        channels: input.channels,
        compared_samples: len,
        snr,
        peak_delta,
        lufs_delta,
        new_clipping,
        silence_noise_floor_delta,
        perceptual_diagnosis,
        release: audio_threshold_result(
            QualityThresholdProfile::ReleaseSmoke,
            snr,
            lufs_delta,
            peak_delta,
            new_clipping,
        ),
        forensic: audio_threshold_result(
            QualityThresholdProfile::ForensicDefault,
            snr,
            lufs_delta,
            peak_delta,
            new_clipping,
        ),
        balanced: audio_threshold_result(
            QualityThresholdProfile::BalancedCandidate,
            snr,
            lufs_delta,
            peak_delta,
            new_clipping,
        ),
    })
}

fn image_threshold_result(
    profile: QualityThresholdProfile,
    psnr: f64,
    ssim: f64,
) -> QualityThresholdResult {
    let (min_psnr, min_ssim) = match profile {
        QualityThresholdProfile::ReleaseSmoke => (IMAGE_RELEASE_MIN_PSNR, IMAGE_RELEASE_MIN_SSIM),
        QualityThresholdProfile::ForensicDefault => {
            (IMAGE_FORENSIC_MIN_PSNR, IMAGE_FORENSIC_MIN_SSIM)
        }
        QualityThresholdProfile::BalancedCandidate => {
            (IMAGE_BALANCED_MIN_PSNR, IMAGE_BALANCED_MIN_SSIM)
        }
    };
    let blocking_reason = if psnr < min_psnr {
        "psnr_below_threshold"
    } else if ssim < min_ssim {
        "ssim_below_threshold"
    } else {
        "none"
    };
    QualityThresholdResult {
        profile,
        passed: blocking_reason == "none",
        blocking_reason: blocking_reason.to_string(),
    }
}

fn audio_threshold_result(
    profile: QualityThresholdProfile,
    snr: f64,
    lufs_delta: f64,
    peak_delta: f64,
    new_clipping: usize,
) -> QualityThresholdResult {
    let (min_snr, max_lufs_delta, max_peak_delta) = match profile {
        QualityThresholdProfile::ReleaseSmoke => (
            AUDIO_RELEASE_MIN_SNR,
            AUDIO_RELEASE_MAX_LUFS_DELTA,
            AUDIO_RELEASE_MAX_PEAK_DELTA,
        ),
        QualityThresholdProfile::ForensicDefault => (
            AUDIO_FORENSIC_MIN_SNR,
            AUDIO_FORENSIC_MAX_LUFS_DELTA,
            AUDIO_FORENSIC_MAX_PEAK_DELTA,
        ),
        QualityThresholdProfile::BalancedCandidate => (
            AUDIO_BALANCED_MIN_SNR,
            AUDIO_BALANCED_MAX_LUFS_DELTA,
            AUDIO_BALANCED_MAX_PEAK_DELTA,
        ),
    };
    let blocking_reason = if snr < min_snr {
        "snr_below_threshold"
    } else if lufs_delta > max_lufs_delta {
        "lufs_delta_above_threshold"
    } else if peak_delta > max_peak_delta {
        "peak_delta_above_threshold"
    } else if new_clipping > AUDIO_MAX_NEW_CLIPPING {
        "new_clipping"
    } else {
        "none"
    };
    QualityThresholdResult {
        profile,
        passed: blocking_reason == "none",
        blocking_reason: blocking_reason.to_string(),
    }
}

fn image_psnr(
    source: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    candidate: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<f64, String> {
    if source.dimensions() != candidate.dimensions() {
        return Err("image dimensions differ".to_string());
    }
    let mut mse = 0.0;
    let mut count = 0.0;
    for (left, right) in source.pixels().zip(candidate.pixels()) {
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
    candidate: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<f64, String> {
    if source.dimensions() != candidate.dimensions() {
        return Err("image dimensions differ".to_string());
    }
    let mut n = 0.0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for (left, right) in source.pixels().zip(candidate.pixels()) {
        let x = luma(left);
        let y = luma(right);
        sum_x += x;
        sum_y += y;
        n += 1.0;
    }
    if n <= 1.0 {
        return Err("image needs at least two pixels for SSIM".to_string());
    }
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    let mut cov = 0.0;
    for (left, right) in source.pixels().zip(candidate.pixels()) {
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

fn audio_snr(source: &[f32], candidate: &[f32]) -> Result<f64, String> {
    let len = source.len().min(candidate.len());
    if len == 0 {
        return Err("empty audio samples".to_string());
    }
    let mut signal = 0.0;
    let mut noise = 0.0;
    for index in 0..len {
        let source_sample = f64::from(source[index]);
        let candidate_sample = f64::from(candidate[index]);
        signal += source_sample * source_sample;
        noise += (source_sample - candidate_sample) * (source_sample - candidate_sample);
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

fn new_clipping_samples(source: &[f32], candidate: &[f32]) -> usize {
    let len = source.len().min(candidate.len());
    (0..len)
        .filter(|index| source[*index].abs() < 0.999 && candidate[*index].abs() >= 0.999)
        .count()
}

fn silence_noise_floor_delta(source: &[f32], candidate: &[f32]) -> f64 {
    let len = source.len().min(candidate.len());
    let mut source_energy = 0.0;
    let mut candidate_energy = 0.0;
    let mut count = 0_usize;
    for index in 0..len {
        let source_sample = f64::from(source[index]);
        if source_sample.abs() <= SILENCE_RMS_THRESHOLD {
            let candidate_sample = f64::from(candidate[index]);
            source_energy += source_sample * source_sample;
            candidate_energy += candidate_sample * candidate_sample;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let source_rms = (source_energy / count as f64).sqrt();
    let candidate_rms = (candidate_energy / count as f64).sqrt();
    candidate_rms - source_rms
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

fn audio_perceptual_diagnosis(
    source: &[f32],
    candidate: &[f32],
    sample_rate: usize,
) -> Result<AudioPerceptualDiagnosis, String> {
    let segment_samples = sample_rate * AUDIO_DIAGNOSTIC_SEGMENT_SECONDS;
    let len = source.len().min(candidate.len());
    if len < segment_samples {
        return Err("audio too short for segmented SNR diagnosis".to_string());
    }
    let mut segment_snrs = Vec::new();
    for start in (0..len).step_by(segment_samples) {
        let end = (start + segment_samples).min(len);
        if end - start < segment_samples / 2 {
            continue;
        }
        segment_snrs.push(audio_snr(&source[start..end], &candidate[start..end])?);
    }
    if segment_snrs.is_empty() {
        return Err("no audio segments available for SNR diagnosis".to_string());
    }

    let segment_snr_min = segment_snrs
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let segment_snr_max = segment_snrs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let segment_snr_mean = segment_snrs.iter().sum::<f64>() / segment_snrs.len() as f64;
    let segment_snr_first = segment_snrs[0];
    let segment_snr_middle = segment_snrs[segment_snrs.len() / 2];
    let segment_snr_last = *segment_snrs.last().unwrap_or(&segment_snr_middle);
    let segment_snr_spread = segment_snr_max - segment_snr_min;

    let bands = audio_band_energy_diagnosis(source, candidate, sample_rate)?;
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
    let high_noise_share = share(high.noise_energy, noise_total);
    let low_noise_share = share(low.noise_energy, noise_total);
    let watermark_signal_share = share(watermark.signal_energy, signal_total);
    let diagnosis =
        if watermark_noise_share >= 0.55 && watermark_noise_share > watermark_signal_share * 1.4 {
            "specific_watermark_band_energy_redistribution"
        } else if segment_snr_spread <= 3.0
            && (watermark_noise_share - watermark_signal_share).abs() <= 0.20
        {
            "full_band_noise_floor_statistical_amplification"
        } else {
            "audible_distortion_not_indicated_by_objective_diagnostic"
        };

    Ok(AudioPerceptualDiagnosis {
        segmented_snr: AudioSegmentSnrReport {
            segment_seconds: AUDIO_DIAGNOSTIC_SEGMENT_SECONDS,
            segment_count: segment_snrs.len(),
            min: segment_snr_min,
            mean: segment_snr_mean,
            max: segment_snr_max,
            first: segment_snr_first,
            middle: segment_snr_middle,
            last: segment_snr_last,
            spread: segment_snr_spread,
        },
        band_energy: AudioBandEnergyReport {
            low_signal_share: share(low.signal_energy, signal_total),
            low_noise_share,
            watermark_signal_share,
            watermark_noise_share,
            high_signal_share: share(high.signal_energy, signal_total),
            high_noise_share,
            dominant_noise_band: dominant.name.to_string(),
        },
        diagnosis: diagnosis.to_string(),
    })
}

struct AudioBandEnergy {
    name: &'static str,
    lo_hz: f64,
    hi_hz: f64,
    signal_energy: f64,
    noise_energy: f64,
}

fn audio_band_energy_diagnosis(
    source: &[f32],
    candidate: &[f32],
    sample_rate: usize,
) -> Result<Vec<AudioBandEnergy>, String> {
    let len = source.len().min(candidate.len());
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
    if len < AUDIO_DIAGNOSTIC_FFT_SIZE {
        return Err("audio too short for band-energy diagnosis".to_string());
    }

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
            .zip(candidate[start..end].iter())
            .map(|(left, right)| f64::from(*right) - f64::from(*left))
            .collect::<Vec<_>>();
        let mut signal_spectrum = fft.make_output_vec();
        let mut noise_spectrum = fft.make_output_vec();
        fft.process(&mut signal_input, &mut signal_spectrum)
            .map_err(|error| format!("FFT failed: {error}"))?;
        fft.process(&mut noise_input, &mut noise_spectrum)
            .map_err(|error| format!("FFT failed: {error}"))?;

        for bin in 1..signal_spectrum.len() {
            let hz = bin as f64 * sample_rate as f64 / AUDIO_DIAGNOSTIC_FFT_SIZE as f64;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_images_report_no_difference() {
        let image = ImageBuffer::from_pixel(4, 4, Rgb([12, 34, 56]));
        let report = compare_image_quality(ImageQualityInput {
            source: &image,
            candidate: &image,
        })
        .unwrap();

        assert_eq!(report.psnr, 99.0);
        assert!((report.ssim - 1.0).abs() < 1e-12);
        assert_eq!(report.mae, 0.0);
        assert_eq!(report.changed_pixel_ratio, 0.0);
        assert!(report.forensic.passed);
    }

    #[test]
    fn deterministic_image_difference_is_measured() {
        let source = ImageBuffer::from_pixel(4, 4, Rgb([10, 10, 10]));
        let mut candidate = source.clone();
        candidate.put_pixel(0, 0, Rgb([20, 10, 10]));
        let report = compare_image_quality(ImageQualityInput {
            source: &source,
            candidate: &candidate,
        })
        .unwrap();

        assert_eq!(report.max_channel_difference, 10);
        assert_eq!(report.changed_pixel_ratio, 1.0 / 16.0);
        assert!(report.mae > 0.0);
    }

    #[test]
    fn identical_audio_reports_no_difference() {
        let samples = (0..88_200)
            .map(|index| ((index as f32 / 44_100.0) * std::f32::consts::TAU * 440.0).sin() * 0.2)
            .collect::<Vec<_>>();
        let report = compare_audio_quality(AudioQualityInput {
            source: &samples,
            candidate: &samples,
            sample_rate: 44_100,
            channels: 1,
        })
        .unwrap();

        assert_eq!(report.snr, 99.0);
        assert_eq!(report.new_clipping, 0);
        assert!(report.forensic.passed);
    }
}
