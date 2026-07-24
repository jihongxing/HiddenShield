use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use watermark_core::{
    build_video_feature_bundle, build_video_visual_payload_from_reserved_uid,
    derive_video_visual_strategy, embed_video_visual_dct_frames,
    self_check_video_visual_dct_frames, AIContentFlags, VideoFeatureBundleBuildInput,
    VideoFramePlane, VideoVisualProfile, VideoVisualReservedPayloadBuildInput,
    VideoVisualSelfCheckFramesInput, VideoVisualStrategyBuildInput,
};

const SELF_CHECK_THRESHOLD: f32 = 0.90;
const MAX_REGIONS: u32 = 96;
const DEFAULT_WIDTH: u32 = 1024;
const DEFAULT_HEIGHT: u32 = 1024;
const DEFAULT_FRAME_COUNT: u32 = 4;
const MAX_FRAME_COUNT: u32 = 8;
const CONTROLLED_KIND: &str = "l3_controlled_upload_proxy";
const USER_OBJECT_KIND: &str = "l3_user_object_upload_proxy";
const SANDBOX_PROFILE: &str = "l3_ffmpeg_transcode_sandbox_v1";
const TRANSCODE_PROFILE: &str = "h264_controlled_proxy_v1";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudVideoTaskRecord {
    task_id: String,
    schema_version: String,
    workspace_id: String,
    creator_profile_id: String,
    capability_level: String,
    watermark_uid: String,
    source_hash: String,
    duration_ms: u64,
    target_profiles: Vec<String>,
    upload_manifest: VideoUploadManifest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoUploadManifest {
    schema_version: String,
    contains_original_video: bool,
    contains_watermarked_video: bool,
    contains_local_paths: bool,
    contains_proxy: bool,
    items: Vec<VideoUploadManifestItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoUploadManifestItem {
    kind: String,
    sha256: String,
    bytes: u64,
    storage_ref: Option<String>,
    sandbox_profile: Option<String>,
    transcode_profile: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    frame_count: Option<u32>,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task_json_path = required_arg(&args, "--task-json")?;
    let registry_proof_hash = required_arg(&args, "--registry-proof-hash")?;
    let creator_identity =
        optional_arg(&args, "--creator-identity").unwrap_or_else(|| "l3-real-worker".into());
    let ffmpeg = optional_arg(&args, "--ffmpeg").unwrap_or_else(|| "ffmpeg".into());
    let object_store_dir = optional_arg(&args, "--object-store-dir");
    let controlled_object_dir = optional_arg(&args, "--controlled-object-dir");
    let output_object_dir = optional_arg(&args, "--output-object-dir");

    let task_text = fs::read_to_string(&task_json_path)
        .map_err(|error| format!("read task json {task_json_path}: {error}"))?;
    let task: CloudVideoTaskRecord =
        serde_json::from_str(&task_text).map_err(|error| format!("parse task json: {error}"))?;
    let item = validate_task_and_manifest(&task)?;
    let width = item.width.unwrap_or(DEFAULT_WIDTH);
    let height = item.height.unwrap_or(DEFAULT_HEIGHT);
    let frame_count = item.frame_count.unwrap_or(DEFAULT_FRAME_COUNT);
    validate_sandbox_dimensions(width, height, frame_count)?;

    let sandbox = SandboxDir::new()?;
    let proxy_path = sandbox.path.join("controlled_proxy_h264.mp4");
    let decoded_path = sandbox.path.join("decoded_luma.gray");
    let source_path = input_storage_ref_to_path(
        item.storage_ref.as_deref().unwrap_or_default(),
        object_store_dir.as_deref(),
        controlled_object_dir.as_deref(),
    )?;
    let source_object_bytes =
        fs::read(&source_path).map_err(|error| format!("read upload object: {error}"))?;
    validate_upload_object_bytes(&source_object_bytes, item)?;
    fs::write(&proxy_path, &source_object_bytes)
        .map_err(|error| format!("copy upload object into sandbox: {error}"))?;
    run_ffmpeg_decode(&ffmpeg, frame_count, &proxy_path, &decoded_path)?;
    let decoded_bytes =
        fs::read(&decoded_path).map_err(|error| format!("read decoded sandbox luma: {error}"))?;
    let mut frames = decoded_frames(width, height, frame_count, &decoded_bytes)?;

    let source_video_sha256 = source_digest(&task.source_hash)?;
    let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
        frames: &frames,
        source_video_sha256,
        duration_ms: task.duration_ms,
    })
    .map_err(|error| format!("build video feature bundle: {error}"))?;
    let payload =
        build_video_visual_payload_from_reserved_uid(VideoVisualReservedPayloadBuildInput {
            watermark_uid: &task.watermark_uid,
            creator_identity: &creator_identity,
            source_video_sha256,
            timestamp: unix_seconds(),
            ai_flags: AIContentFlags::default(),
            registry_proof_hash: Some(&registry_proof_hash),
        })
        .map_err(|error| format!("build reserved video visual payload: {error}"))?;
    if payload.watermark_uid() != task.watermark_uid {
        return Err("reserved payload watermarkUid did not match task watermarkUid".to_string());
    }

    let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
        task_id: &task.task_id,
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
            "real L3 worker first-pass self-check failed: confidence={} threshold={}",
            self_check.confidence, self_check.self_check_threshold
        ));
    }

    let raw_watermarked_path = sandbox.path.join("watermarked_luma.gray");
    let output_storage_ref = if object_store_dir.is_some() {
        format!(
            "object://l3-output/{}/{}.l3-watermarked.mp4",
            task.task_id, task.task_id
        )
    } else {
        format!(
            "controlled://l3-output/{}/{}.l3-watermarked.mp4",
            task.task_id, task.task_id
        )
    };
    let output_media_path = output_storage_ref_to_path(
        &output_storage_ref,
        object_store_dir.as_deref(),
        output_object_dir.as_deref(),
    )?;
    if let Some(parent) = output_media_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create controlled output directory: {error}"))?;
    }
    write_luma_frames(&frames, &raw_watermarked_path)?;
    run_ffmpeg_encode_luma_mp4(
        &ffmpeg,
        width,
        height,
        frame_count,
        &raw_watermarked_path,
        &output_media_path,
    )?;
    let packaged_decoded_path = sandbox.path.join("packaged_output_luma.gray");
    run_ffmpeg_decode(
        &ffmpeg,
        frame_count,
        &output_media_path,
        &packaged_decoded_path,
    )?;
    let packaged_decoded_bytes = fs::read(&packaged_decoded_path)
        .map_err(|error| format!("read packaged output decoded luma: {error}"))?;
    let packaged_frames = decoded_frames(width, height, frame_count, &packaged_decoded_bytes)?;
    let packaged_self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
        strategy: &strategy,
        observed_strategy_digest: &strategy.strategy_digest,
        frames: &packaged_frames,
        expected_payload: &payload,
    })
    .map_err(|error| format!("self-check packaged video visual watermark: {error}"))?;
    if !packaged_self_check.passed {
        return Err(format!(
            "real L3 worker packaged output self-check failed: confidence={} threshold={}",
            packaged_self_check.confidence, packaged_self_check.self_check_threshold
        ));
    }

    let source_proxy_hash = format!("sha256:{}", hex_lower(&sha256_file(&proxy_path)?));
    let decoded_frame_hash = format!("sha256:{}", hex_lower(&Sha256::digest(&decoded_bytes)));
    let watermarked_frame_hash = format!("sha256:{}", hex_lower(&hash_frames(&packaged_frames)));
    let watermarked_media_hash = format!("sha256:{}", hex_lower(&sha256_file(&output_media_path)?));
    let sandbox_input_bytes = fs::metadata(&proxy_path)
        .map_err(|error| format!("stat proxy: {error}"))?
        .len();
    let output_media_bytes = fs::metadata(&output_media_path)
        .map_err(|error| format!("stat controlled output media: {error}"))?
        .len();
    let worker_receipt = json!({
        "algorithmSource": "watermark-core",
        "attemptScoped": true,
        "input": {
            "bytes": item.bytes,
            "objectStoreRead": item.storage_ref.as_deref().unwrap_or_default().starts_with("object://l3-upload/"),
            "controlledObjectRead": item.storage_ref.as_deref().unwrap_or_default().starts_with("controlled://l3-upload-proxy/"),
            "sha256": item.sha256,
            "storageRef": item.storage_ref
        },
        "output": {
            "bytes": output_media_bytes,
            "container": "mp4",
            "contentType": "video/mp4",
            "downloadableObjectStoreObject": output_storage_ref.starts_with("object://l3-output/"),
            "downloadableControlledObject": output_storage_ref.starts_with("controlled://l3-output/"),
            "sha256": watermarked_media_hash,
            "storageRef": output_storage_ref
        },
        "privacyBoundary": {
            "containsLocalPaths": false,
            "containsOriginalVideo": false,
            "containsWatermarkedVideo": false,
            "objectUploadOnly": item.storage_ref.as_deref().unwrap_or_default().starts_with("object://l3-upload/"),
            "controlledUploadOnly": item.storage_ref.as_deref().unwrap_or_default().starts_with("controlled://l3-upload-proxy/"),
            "noLocalPathInReceipt": true
        },
        "schemaVersion": "l3_worker_receipt_v1",
        "selfCheck": {
            "checkedFrames": packaged_self_check.checked_frames,
            "confidence": packaged_self_check.confidence,
            "strategyDigest": strategy.strategy_digest,
            "threshold": packaged_self_check.self_check_threshold,
            "watermarkedFrameHash": watermarked_frame_hash
        },
        "taskId": task.task_id,
        "watermarkUid": task.watermark_uid,
        "workerId": "watermark-core-l3-real-worker-first-pass"
    });
    let worker_receipt_text =
        serde_json::to_string(&worker_receipt).map_err(|error| error.to_string())?;
    let worker_receipt_hash = format!(
        "sha256:{}",
        hex_lower(&Sha256::digest(worker_receipt_text.as_bytes()))
    );
    let sandbox_cleanup = sandbox.cleanup();
    let output = json!({
        "schemaVersion": "l3_real_worker_first_pass_v1",
        "workerId": "watermark-core-l3-real-worker-first-pass",
        "taskId": task.task_id,
        "workspaceId": task.workspace_id,
        "creatorProfileId": task.creator_profile_id,
        "watermarkUid": task.watermark_uid,
        "payloadWatermarkUid": payload.watermark_uid(),
        "sourceHash": task.source_hash,
        "sourceVideoSha256": format!("sha256:{}", hex_lower(&source_video_sha256)),
        "strategyDigest": strategy.strategy_digest,
        "selfCheckThreshold": packaged_self_check.self_check_threshold,
        "selfCheckConfidence": packaged_self_check.confidence,
        "checkedFrames": packaged_self_check.checked_frames,
        "embeddedFrames": embedded_frames,
        "watermarkedMediaHash": watermarked_media_hash,
        "watermarkedFrameHash": watermarked_frame_hash,
        "outputMediaStorageRef": output_storage_ref,
        "outputMediaBytes": output_media_bytes,
        "outputMediaContentType": "video/mp4",
        "workerReceiptHash": worker_receipt_hash,
        "workerReceipt": worker_receipt,
        "featureDigest": format!("sha256:{}", hex_lower(&feature_bundle.feature_digest)),
        "algorithmSource": "watermark-core",
        "manifestBinding": {
            "kind": item.kind,
            "storageRef": item.storage_ref,
            "sha256": item.sha256,
            "bytes": item.bytes,
            "containsProxy": task.upload_manifest.contains_proxy,
            "sandboxProfile": item.sandbox_profile,
            "transcodeProfile": item.transcode_profile,
            "objectStoreRead": item.storage_ref.as_deref().unwrap_or_default().starts_with("object://l3-upload/"),
            "controlledObjectRead": item.storage_ref.as_deref().unwrap_or_default().starts_with("controlled://l3-upload-proxy/")
        },
        "transcodeSandbox": {
            "engine": "ffmpeg",
            "profile": SANDBOX_PROFILE,
            "sourceProxyHash": source_proxy_hash,
            "decodedFrameHash": decoded_frame_hash,
            "inputBytes": sandbox_input_bytes,
            "width": width,
            "height": height,
            "frameCount": frame_count,
            "cleanup": sandbox_cleanup
        },
        "outputPackaging": {
            "storageRef": output_storage_ref,
            "sha256": watermarked_media_hash,
            "bytes": output_media_bytes,
            "contentType": "video/mp4",
            "container": "mp4",
            "packagedSelfCheckConfidence": packaged_self_check.confidence,
            "downloadableObjectStoreObject": output_storage_ref.starts_with("object://l3-output/"),
            "downloadableControlledObject": output_storage_ref.starts_with("controlled://l3-output/")
        },
        "privacyBoundary": {
            "containsOriginalVideo": false,
            "containsWatermarkedVideo": false,
            "containsLocalPaths": false,
            "objectUploadOnly": item.storage_ref.as_deref().unwrap_or_default().starts_with("object://l3-upload/"),
            "controlledUploadOnly": item.storage_ref.as_deref().unwrap_or_default().starts_with("controlled://l3-upload-proxy/"),
            "noLocalPathInReceipt": true
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn validate_task_and_manifest(
    task: &CloudVideoTaskRecord,
) -> Result<&VideoUploadManifestItem, String> {
    if task.schema_version.trim() != "cloud_video_task_v1" {
        return Err("task schemaVersion must be cloud_video_task_v1".to_string());
    }
    if task.capability_level.trim() != "hybrid_visual_watermark" {
        return Err("task capabilityLevel must be hybrid_visual_watermark".to_string());
    }
    if task.target_profiles.is_empty() {
        return Err("task targetProfiles must not be empty".to_string());
    }
    if task.upload_manifest.schema_version.trim() != "video_upload_manifest_v1" {
        return Err("upload manifest schemaVersion must be video_upload_manifest_v1".to_string());
    }
    if task.upload_manifest.contains_original_video
        || task.upload_manifest.contains_watermarked_video
        || task.upload_manifest.contains_local_paths
    {
        return Err("upload manifest privacy flags forbid worker execution".to_string());
    }
    if !task.upload_manifest.contains_proxy {
        return Err("real worker first-pass requires an object proxy manifest".to_string());
    }
    if task.upload_manifest.items.len() != 1 {
        return Err("real worker first-pass requires exactly one manifest item".to_string());
    }
    let item = &task.upload_manifest.items[0];
    if item.kind.trim() != CONTROLLED_KIND && item.kind.trim() != USER_OBJECT_KIND {
        return Err(format!(
            "manifest item kind must be {CONTROLLED_KIND} or {USER_OBJECT_KIND}"
        ));
    }
    if item.sha256.trim() != task.source_hash.trim() || source_digest(&item.sha256).is_err() {
        return Err("manifest item sha256 must match task sourceHash".to_string());
    }
    if item.bytes == 0 {
        return Err("manifest item bytes must be greater than zero".to_string());
    }
    let storage_ref = item
        .storage_ref
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if !(storage_ref.starts_with("controlled://l3-upload-proxy/")
        || storage_ref.starts_with("object://l3-upload/"))
        || looks_like_local_path(storage_ref)
        || storage_ref.contains("..")
        || storage_ref.contains('\\')
    {
        return Err(
            "manifest item storageRef must be a controlled or object upload proxy ref".to_string(),
        );
    }
    if item.sandbox_profile.as_deref().map(str::trim) != Some(SANDBOX_PROFILE) {
        return Err(format!(
            "manifest item sandboxProfile must be {SANDBOX_PROFILE}"
        ));
    }
    if item.transcode_profile.as_deref().map(str::trim) != Some(TRANSCODE_PROFILE) {
        return Err(format!(
            "manifest item transcodeProfile must be {TRANSCODE_PROFILE}"
        ));
    }
    Ok(item)
}

fn validate_sandbox_dimensions(width: u32, height: u32, frame_count: u32) -> Result<(), String> {
    if width < 512 || height < 512 || width % 8 != 0 || height % 8 != 0 {
        return Err("sandbox dimensions must be at least 512x512 and divisible by 8".to_string());
    }
    if frame_count == 0 || frame_count > MAX_FRAME_COUNT {
        return Err(format!(
            "frameCount must be between 1 and {MAX_FRAME_COUNT}"
        ));
    }
    Ok(())
}

fn looks_like_local_path(value: &str) -> bool {
    value.starts_with("file:")
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.as_bytes().get(1) == Some(&b':')
}

fn run_ffmpeg_decode(
    ffmpeg: &str,
    frame_count: u32,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    run_command(
        Command::new(ffmpeg)
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-i")
            .arg(input)
            .arg("-frames:v")
            .arg(frame_count.to_string())
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("gray")
            .arg(output),
        "ffmpeg controlled proxy decode",
    )
}

fn run_ffmpeg_encode_luma_mp4(
    ffmpeg: &str,
    width: u32,
    height: u32,
    frame_count: u32,
    input: &Path,
    output: &Path,
) -> Result<(), String> {
    run_command(
        Command::new(ffmpeg)
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("gray")
            .arg("-s:v")
            .arg(format!("{width}x{height}"))
            .arg("-r")
            .arg("1")
            .arg("-i")
            .arg(input)
            .arg("-frames:v")
            .arg(frame_count.to_string())
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("ultrafast")
            .arg("-crf")
            .arg("16")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-movflags")
            .arg("+faststart")
            .arg(output),
        "ffmpeg controlled output encode",
    )
}

fn run_command(command: &mut Command, label: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{label} failed to start: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} failed with status {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn write_luma_frames(frames: &[VideoFramePlane], path: &Path) -> Result<(), String> {
    let total_bytes = frames
        .iter()
        .map(|frame| frame.luma_pixels().len())
        .sum::<usize>();
    let mut bytes = Vec::with_capacity(total_bytes);
    for frame in frames {
        bytes.extend_from_slice(&frame.luma_pixels());
    }
    fs::write(path, bytes).map_err(|error| format!("write watermarked luma frames: {error}"))
}

fn decoded_frames(
    width: u32,
    height: u32,
    frame_count: u32,
    bytes: &[u8],
) -> Result<Vec<VideoFramePlane>, String> {
    let frame_bytes = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "decoded frame dimensions overflow".to_string())?;
    let expected = frame_bytes
        .checked_mul(frame_count as usize)
        .ok_or_else(|| "decoded frame count overflow".to_string())?;
    if bytes.len() != expected {
        return Err(format!(
            "decoded luma size mismatch: expected {expected}, got {}",
            bytes.len()
        ));
    }
    let mut frames = Vec::with_capacity(frame_count as usize);
    for chunk in bytes.chunks_exact(frame_bytes) {
        frames.push(
            VideoFramePlane::new_luma_dct_mid_band(width, height, width as usize, chunk.to_vec())
                .map_err(|error| format!("build decoded frame: {error}"))?,
        );
    }
    Ok(frames)
}

fn validate_upload_object_bytes(
    bytes: &[u8],
    item: &VideoUploadManifestItem,
) -> Result<(), String> {
    if bytes.len() as u64 != item.bytes {
        return Err(format!(
            "upload object bytes mismatch: manifest={} actual={}",
            item.bytes,
            bytes.len()
        ));
    }
    let actual = format!("sha256:{}", hex_lower(&Sha256::digest(bytes)));
    if actual != item.sha256.trim() {
        return Err("upload object sha256 mismatch".to_string());
    }
    Ok(())
}

fn input_storage_ref_to_path(
    storage_ref: &str,
    object_store_dir: Option<&str>,
    controlled_object_dir: Option<&str>,
) -> Result<PathBuf, String> {
    let storage_ref = storage_ref.trim();
    if storage_ref.starts_with("object://l3-upload/") {
        let root = object_store_dir.ok_or_else(|| {
            "object:// input requires --object-store-dir for worker byte access".to_string()
        })?;
        return object_storage_ref_to_path(storage_ref, Path::new(root));
    }
    if storage_ref.starts_with("controlled://l3-upload-proxy/") {
        let root = controlled_object_dir.ok_or_else(|| {
            "controlled:// input requires --controlled-object-dir for worker byte access"
                .to_string()
        })?;
        return controlled_ref_to_path(
            storage_ref,
            Path::new(root),
            "controlled://l3-upload-proxy/",
        );
    }
    Err("input storageRef has unsupported prefix".to_string())
}

fn output_storage_ref_to_path(
    storage_ref: &str,
    object_store_dir: Option<&str>,
    output_object_dir: Option<&str>,
) -> Result<PathBuf, String> {
    let storage_ref = storage_ref.trim();
    if storage_ref.starts_with("object://l3-output/") {
        let root = object_store_dir.ok_or_else(|| {
            "object:// output requires --object-store-dir for worker byte access".to_string()
        })?;
        return object_storage_ref_to_path(storage_ref, Path::new(root));
    }
    if storage_ref.starts_with("controlled://l3-output/") {
        let root = output_object_dir.ok_or_else(|| {
            "controlled:// output requires --output-object-dir for worker byte access".to_string()
        })?;
        return controlled_ref_to_path(storage_ref, Path::new(root), "controlled://l3-output/");
    }
    Err("output storageRef has unsupported prefix".to_string())
}

fn object_storage_ref_to_path(storage_ref: &str, root: &Path) -> Result<PathBuf, String> {
    let relative = storage_ref
        .trim()
        .strip_prefix("object://")
        .ok_or_else(|| "object storageRef must start with object://".to_string())?;
    safe_relative_path(root, relative)
}

fn controlled_ref_to_path(storage_ref: &str, root: &Path, prefix: &str) -> Result<PathBuf, String> {
    let relative = storage_ref
        .trim()
        .strip_prefix(prefix)
        .ok_or_else(|| format!("storageRef must start with {prefix}"))?;
    safe_relative_path(root, relative)
}

fn safe_relative_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.trim().is_empty() || looks_like_local_path(relative) {
        return Err("controlled storage relative path is invalid".to_string());
    }
    let mut path = PathBuf::from(root);
    for segment in relative.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.contains(':')
        {
            return Err("storage relative path contains unsafe segment".to_string());
        }
        path.push(segment);
    }
    Ok(path)
}

fn hash_frames(frames: &[VideoFramePlane]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"hidden-shield:l3-real-worker-first-pass:watermarked-frames:v1");
    for frame in frames {
        hasher.update(frame.width.to_be_bytes());
        hasher.update(frame.height.to_be_bytes());
        hasher.update(frame.luma_pixels());
    }
    hasher.finalize().into()
}

fn sha256_file(path: &Path) -> Result<[u8; 32], String> {
    let bytes = fs::read(path).map_err(|error| format!("read file for hash: {error}"))?;
    Ok(Sha256::digest(&bytes).into())
}

fn source_digest(source_hash: &str) -> Result<[u8; 32], String> {
    let hex = source_hash
        .trim()
        .strip_prefix("sha256:")
        .ok_or_else(|| "source hash must start with sha256:".to_string())?;
    if hex.len() != 64 {
        return Err("source hash must contain 32 bytes".to_string());
    }
    let mut out = [0u8; 32];
    for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|error| format!("invalid hash: {error}"))?;
        out[index] = u8::from_str_radix(text, 16)
            .map_err(|error| format!("invalid hash byte '{text}': {error}"))?;
    }
    Ok(out)
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

struct SandboxDir {
    path: PathBuf,
}

impl SandboxDir {
    fn new() -> Result<Self, String> {
        let path = env::temp_dir().join(format!(
            "hiddenshield-l3-worker-sandbox-{}-{}",
            std::process::id(),
            unix_seconds()
        ));
        fs::create_dir_all(&path).map_err(|error| format!("create sandbox dir: {error}"))?;
        Ok(Self { path })
    }

    fn cleanup(self) -> bool {
        fs::remove_dir_all(&self.path).is_ok()
    }
}
