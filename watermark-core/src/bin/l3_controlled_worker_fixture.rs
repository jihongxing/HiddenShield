use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use watermark_core::{
    build_video_feature_bundle, build_video_visual_payload, derive_video_visual_strategy,
    embed_video_visual_dct_frames, self_check_video_visual_dct_frames, AIContentFlags,
    VideoFeatureBundleBuildInput, VideoFramePlane, VideoVisualPayloadBuildInput,
    VideoVisualProfile, VideoVisualSelfCheckFramesInput, VideoVisualStrategyBuildInput,
};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const FRAME_COUNT: u32 = 4;
const DURATION_MS: u64 = 125_000;
const SELF_CHECK_THRESHOLD: f32 = 0.90;
const MAX_REGIONS: u32 = 96;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task_id = required_arg(&args, "--task-id")?;
    let watermark_uid = required_arg(&args, "--watermark-uid")?;
    let source_hash = required_arg(&args, "--source-hash")?;
    let creator_identity =
        optional_arg(&args, "--creator-identity").unwrap_or_else(|| "l3-controlled-worker".into());
    let device_identity =
        optional_arg(&args, "--device-identity").unwrap_or_else(|| "l3-worker-fixture".into());
    let duration_ms = optional_arg(&args, "--duration-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DURATION_MS);

    let source_video_sha256 = source_digest(&source_hash);
    let mut frames = controlled_frames()?;
    let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
        frames: &frames,
        source_video_sha256,
        duration_ms,
    })
    .map_err(|error| format!("build video feature bundle: {error}"))?;
    let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
        creator_identity: &creator_identity,
        device_identity: &device_identity,
        source_video_sha256,
        timestamp: unix_seconds(),
        ai_flags: AIContentFlags::default(),
    })
    .map_err(|error| format!("build video visual payload: {error}"))?;
    let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
        task_id: &task_id,
        payload: &payload,
        feature_bundle: &feature_bundle,
        target_profile: VideoVisualProfile::LumaDctMidBandV1,
        expires_at: unix_seconds() + 3600,
        self_check_threshold: SELF_CHECK_THRESHOLD,
        max_regions: MAX_REGIONS,
    })
    .map_err(|error| format!("derive video visual strategy: {error}"))?;
    let embedded_frames = embed_video_visual_dct_frames(&mut frames, &strategy, &payload)
        .map_err(|error| format!("embed video visual watermark: {error}"))?;
    let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
        strategy: &strategy,
        observed_strategy_digest: &strategy.strategy_digest,
        frames: &frames,
        expected_payload: &payload,
    })
    .map_err(|error| format!("self-check video visual watermark: {error}"))?;
    if !self_check.passed {
        return Err(format!(
            "controlled L3 worker self-check failed: confidence={} threshold={}",
            self_check.confidence, self_check.self_check_threshold
        ));
    }

    let watermarked_media_hash = format!("sha256:{}", hex_lower(&hash_frames(&frames)));
    let output = json!({
        "schemaVersion": "l3_controlled_worker_fixture_v1",
        "workerId": "watermark-core-controlled-l3-fixture",
        "taskId": task_id,
        "watermarkUid": watermark_uid,
        "payloadWatermarkUid": payload.watermark_uid(),
        "sourceHash": source_hash,
        "sourceVideoSha256": format!("sha256:{}", hex_lower(&source_video_sha256)),
        "strategyDigest": strategy.strategy_digest,
        "selfCheckThreshold": self_check.self_check_threshold,
        "selfCheckConfidence": self_check.confidence,
        "checkedFrames": self_check.checked_frames,
        "embeddedFrames": embedded_frames,
        "watermarkedMediaHash": watermarked_media_hash,
        "featureDigest": format!("sha256:{}", hex_lower(&feature_bundle.feature_digest)),
        "frameCount": frames.len(),
        "durationMs": duration_ms,
        "targetProfile": "luma_dct_mid_band_v1",
        "algorithmSource": "watermark-core",
        "privacyBoundary": {
            "containsOriginalVideo": false,
            "containsWatermarkedVideo": false,
            "containsLocalPaths": false,
            "fixtureOnly": true
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn controlled_frames() -> Result<Vec<VideoFramePlane>, String> {
    (0..FRAME_COUNT)
        .map(|frame_index| {
            let mut pixels = Vec::with_capacity((WIDTH * HEIGHT) as usize);
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let gradient = ((x * 3 + y * 5 + frame_index * 17) % 256) as u8;
                    let checker = if ((x / 16) + (y / 16) + frame_index) % 2 == 0 {
                        42
                    } else {
                        196
                    };
                    let value = gradient.wrapping_add(checker / 3);
                    pixels.push(value);
                }
            }
            VideoFramePlane::new_luma_dct_mid_band(WIDTH, HEIGHT, WIDTH as usize, pixels)
                .map_err(|error| format!("build controlled frame {frame_index}: {error}"))
        })
        .collect()
}

fn hash_frames(frames: &[VideoFramePlane]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"hidden-shield:l3-controlled-worker:watermarked-frames:v1");
    for frame in frames {
        hasher.update(frame.width.to_be_bytes());
        hasher.update(frame.height.to_be_bytes());
        hasher.update(frame.luma_pixels());
    }
    hasher.finalize().into()
}

fn source_digest(source_hash: &str) -> [u8; 32] {
    if let Some(hex) = source_hash.trim().strip_prefix("sha256:") {
        if let Some(bytes) = parse_hex_32(hex) {
            return bytes;
        }
    }
    Sha256::digest(source_hash.as_bytes()).into()
}

fn parse_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        out[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

fn required_arg(args: &[String], name: &str) -> Result<String, String> {
    optional_arg(args, name).ok_or_else(|| format!("{name} is required"))
}

fn optional_arg(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
