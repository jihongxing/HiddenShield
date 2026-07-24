import { readFileSync } from 'node:fs';

const sources = {
  design: readFileSync('docs/Phase 7 视频云端能力设计.md', 'utf8'),
  spikeDoc: readFileSync('docs/Phase 7 L2视频指纹技术Spike.md', 'utf8'),
  l2ApiDoc: readFileSync('docs/Phase 7 L2云端指纹存证API草案.md', 'utf8'),
  spikeBin: readFileSync('tools/video-fingerprint-spike/src/main.rs', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
  videoE2e: readFileSync('scripts/verify-cloud-video-e2e.mjs', 'utf8'),
  l3WorkerE2e: readFileSync('scripts/verify-l3-controlled-worker-e2e.mjs', 'utf8'),
  l3WorkerFixture: readFileSync('watermark-core/src/bin/l3_controlled_worker_fixture.rs', 'utf8'),
  l3RealWorkerE2e: readFileSync('scripts/verify-l3-real-worker-first-pass-e2e.mjs', 'utf8'),
  l3RealWorker: readFileSync('watermark-core/src/bin/l3_real_worker_first_pass.rs', 'utf8'),
  coreVideoVisual: readFileSync('watermark-core/src/video_visual.rs', 'utf8'),
  videoCi: readFileSync('scripts/run-cloud-video-ci.mjs', 'utf8'),
  videoBundles: readFileSync('scripts/verify-video-fingerprint-bundles.mjs', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  desktopApp: readFileSync('src/App.vue', 'utf8'),
  mobileWorkspace: readFileSync('mobile_app/lib/features/workspace/workspace_page.dart', 'utf8'),
  mobileBridge: readFileSync('mobile_app/lib/bridge/rust_watermark_bridge.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileStore: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  mobileSyncTransport: readFileSync('mobile_app/lib/sync/sync_transport.dart', 'utf8'),
  backendSchema: readFileSync('feedback-backend/src/schema.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendCargo: readFileSync('feedback-backend/Cargo.toml', 'utf8'),
  desktopCloudClient: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopSyncCommand: readFileSync('src-tauri/src/commands/sync.rs', 'utf8'),
  desktopLib: readFileSync('src-tauri/src/lib.rs', 'utf8'),
  desktopVault: readFileSync('src-tauri/src/commands/vault.rs', 'utf8'),
  desktopVaultView: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopReport: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  desktopDbSchema: readFileSync('src-tauri/src/db/schema.rs', 'utf8'),
  desktopDbQueries: readFileSync('src-tauri/src/db/queries.rs', 'utf8'),
  desktopVideoFingerprint: readFileSync('src-tauri/src/video_fingerprint.rs', 'utf8'),
};

assert(
  sources.design.includes('cloud_video_tasks') &&
    sources.design.includes('video_minutes') &&
    sources.design.includes('draft -> queued -> running') &&
    sources.design.includes('upload_manifest'),
  'Phase 7 design must define cloud video task model, minute quota, states, and upload manifest',
);
assert(
  sources.design.includes('contains_original_video') &&
    sources.design.includes('contains_watermarked_video') &&
    sources.design.includes('contains_local_paths') &&
    sources.design.includes('false'),
  'Phase 7 upload manifest must explicitly reject media files and local paths by default',
);
assert(
  sources.desktopApp.includes(':entitlement-state="entitlementState"') &&
    sources.desktopWorkbench.includes('cloud_video_processing') &&
    sources.desktopWorkbench.includes('L1 本地写入') &&
    sources.desktopWorkbench.includes('视频音轨水印') &&
    sources.desktopWorkbench.includes('视频指纹存证') &&
    sources.desktopWorkbench.includes('L3 对象上传入口') &&
    sources.desktopWorkbench.includes('已 succeeded 的 L3 对象任务') &&
    sources.desktopWorkbench.includes('video_visual_* 收据元数据') &&
    sources.desktopWorkbench.includes('下载并保存版权库') &&
    sources.desktopWorkbench.includes('生成指纹包') &&
    sources.desktopWorkbench.includes('提交存证') &&
    sources.desktopWorkbench.includes('选择指纹包') &&
    sources.desktopWorkbench.includes('同步和报告只保存收据元数据') &&
    sources.desktopWorkbench.includes('generateVideoFingerprintBundle') &&
    sources.desktopWorkbench.includes('createVideoFingerprintNotaryFromBundleFile'),
  'desktop workbench must expose L2 video fingerprint bundle generation, notary submission, and preserve privacy copy',
);
assert(
  sources.desktopVault.includes('video_notary_id') &&
    sources.desktopDbSchema.includes('video_notary_id') &&
    sources.desktopDbQueries.includes('video_notary_id') &&
    sources.desktopCloudClient.includes('video_notary_id') &&
    sources.desktopSyncCommand.includes('VideoFingerprintNotaryResult') &&
    sources.desktopSyncCommand.includes('persist_video_fingerprint_notary_record') &&
    sources.desktopSyncCommand.includes('video_bundle_sha256') &&
    sources.desktopWorkbench.includes('已保存到版权库') &&
    sources.desktopWorkbench.includes('videoNotaryVaultRecord') &&
    sources.desktopWorkbench.includes('bundleElapsedMs'),
  'desktop must persist L2 video notary receipt and bundle metadata into vault records without media paths',
);
assert(
  sources.mobileWorkspace.includes('cloud_video_processing') &&
    sources.mobileWorkspace.includes('L1 视频音轨水印') &&
    sources.mobileWorkspace.includes('视频指纹存证') &&
    sources.mobileWorkspace.includes('视频指纹存证与 L3 对象上传入口') &&
    sources.mobileWorkspace.includes('Studio / Enterprise release gate') &&
    sources.mobileWorkspace.includes('下载并保存版权库') &&
    sources.mobileWorkspace.includes('videoVisual* 收据元数据') &&
    sources.mobileWorkspace.includes('不保存本地路径、对象 ref 或签名 URL') &&
    sources.mobileWorkspace.includes('本机不做视频画面水印') &&
    sources.mobileWorkspace.includes('对象上传入口'),
  'mobile workspace must show L2 video fingerprint notary plus the Studio/Enterprise L3 object-upload release-gate entry',
);
assert(
    sources.mobileBridge.includes('Mobile local video watermarking is disabled.'),
  'mobile bridge must keep local video watermarking disabled',
);
assert(
  sources.mobileState.includes('videoNotaryId') &&
    sources.mobileState.includes('video_fingerprint_root') &&
    sources.mobileStore.includes('video_notary_id') &&
    sources.mobileStore.includes('video_bundle_sha256') &&
    sources.mobileSyncTransport.includes('video_notary_id') &&
    sources.mobileVault.includes('视频指纹存证') &&
    sources.mobileVault.includes('WatermarkAssetKind.video') &&
    sources.mobileVault.includes('videoFingerprintRoot') &&
    sources.mobileVault.includes('videoBundleSha256'),
  'mobile must preserve and display synced L2 video notary records with the same receipt and bundle metadata',
);
assert(
  sources.desktopVault.includes('video_visual_task_id') &&
    sources.desktopDbSchema.includes('video_visual_task_id') &&
    sources.desktopDbQueries.includes('video_visual_task_id') &&
    sources.desktopCloudClient.includes('video_visual_task_id') &&
    sources.desktopVaultView.includes('L3 视频画面盲水印') &&
    sources.desktopVaultView.includes('videoVisualMediaHash') &&
    sources.desktopReport.includes('video_visual_watermark: FormalReportVideoVisualWatermark') &&
    sources.desktopReport.includes('formal_report_includes_l3_video_visual_receipt_without_paths_or_urls') &&
    sources.desktopReport.includes('!json.contains("object://")') &&
    sources.mobileState.includes('videoVisualTaskId') &&
    sources.mobileState.includes('video_visual_task_id') &&
    sources.mobileState.includes('video_visual_watermark_receipt') &&
    sources.mobileStore.includes('video_visual_task_id') &&
    sources.mobileSyncTransport.includes("json['video_visual_task_id']") &&
    sources.mobileVault.includes('L3 视频画面盲水印') &&
    sources.mobileVault.includes('videoVisualMediaHash') &&
    !sources.desktopReport.includes('outputMediaStorageRef: record.video_visual') &&
    !sources.mobileState.includes('video_visual_output_storage_ref'),
  'desktop and mobile must persist, sync, display, and report L3 video visual receipt metadata without object refs, signed URLs, local paths, or media bytes',
);
assert(
    sources.packageJson.includes('video:fingerprint-spike') &&
    sources.packageJson.includes('cloud-video:bundles') &&
    sources.packageJson.includes('cloud-video:e2e') &&
    sources.packageJson.includes('cloud-video:l3-worker-qa') &&
    sources.packageJson.includes('cloud-video:l3-real-worker-first-pass-qa') &&
    sources.packageJson.includes('cloud-video:ci') &&
    sources.spikeBin.includes('VideoFingerprintBundle') &&
    sources.spikeBin.includes('#[serde(rename_all = "camelCase")]') &&
    sources.spikeBin.includes('local_blocks') &&
    sources.spikeBin.includes('crop_windows') &&
    sources.spikeBin.includes('CropWindowFingerprint') &&
    sources.spikeBin.includes('scale_540p') &&
    sources.spikeBin.includes('transcode_crf32') &&
    sources.spikeBin.includes('center_crop_80') &&
    sources.spikeBin.includes('recall >= 0.70'),
  'Phase 7 L2 spike tool must generate VideoFingerprintBundle and evaluate scale/transcode/crop recall',
);
assert(
  sources.videoE2e.includes('/v1/video-fingerprints/notaries') &&
    sources.videoE2e.includes('original_video_forbidden') &&
    sources.videoE2e.includes('local_path_forbidden') &&
    sources.videoE2e.includes('crop_windows_required') &&
    sources.videoE2e.includes('serverReceiptSignature') &&
    sources.videoE2e.includes('usageLedgerId') &&
    sources.videoE2e.includes('/v1/video-tasks') &&
    sources.videoE2e.includes('/v1/video-tasks/${created.body.taskId}') &&
    sources.videoE2e.includes('/internal/video-tasks/claim') &&
    sources.videoE2e.includes('/internal/video-tasks/${second.body.taskId}/completion') &&
    sources.videoE2e.includes('attemptId') &&
    sources.videoE2e.includes('leaseToken') &&
    sources.videoE2e.includes('outputMediaStorageRef') &&
    sources.videoE2e.includes('workerReceiptHash') &&
    sources.videoE2e.includes('hmac-sha256:l3-completion-v1') &&
    sources.videoE2e.includes('cloud_video_task_v1') &&
    sources.videoE2e.includes('strategyDigest') &&
    sources.videoE2e.includes('selfCheckConfidence') &&
    sources.videoE2e.includes('watermarkedMediaHash') &&
    sources.backendStorage.includes('server_receipt_signature_required') &&
    sources.backendStorage.includes('cloud_video_task_completion_requires_trusted_worker') &&
    sources.backendStorage.includes('cloud_video_task_failure_code_required') &&
    sources.backendStorage.includes('cloud_video_task_completion_stale_attempt'),
  'cloud video E2E must exercise L2 notary HTTP success and L3 task boundaries',
);
assert(
  sources.videoCi.includes('verify-cloud-video-contract.mjs') &&
    sources.videoCi.includes('verify-video-fingerprint-bundles.mjs') &&
    sources.videoCi.includes('verify-cloud-video-e2e.mjs') &&
    sources.videoCi.includes('verify-l3-controlled-worker-e2e.mjs') &&
    sources.videoCi.includes('verify-l3-real-worker-first-pass-e2e.mjs') &&
    sources.videoCi.includes('feedback-backend/Cargo.toml'),
  'cloud video CI must start feedback-backend and run contract, bundle, plus E2E checks',
);
assert(
  sources.backendCargo.includes('tower = "0.5"'),
  'feedback backend dev-dependencies must include tower for HTTP route QA',
);
assert(
  sources.videoBundles.includes('bundle.json') &&
    sources.videoBundles.includes('video_fingerprint_v1') &&
    sources.videoBundles.includes('video_fingerprint_bundle') &&
    sources.videoBundles.includes('containsOriginalVideo: false') &&
    sources.videoBundles.includes('containsWatermarkedVideo: false') &&
    sources.videoBundles.includes('containsLocalPaths: false') &&
    sources.videoBundles.includes('localBlockFingerprintRoot') &&
    sources.videoBundles.includes('cropWindowFingerprintRoot') &&
    sources.videoBundles.includes('assertNoForbiddenMediaFields') &&
    sources.videoBundles.includes('HIDDENSHIELD_VIDEO_FINGERPRINT_BUNDLE_DIR'),
  'bundle verifier must validate real spike bundle.json files, three-layer roots, manifest privacy, and optional bundle dir',
);
assert(
  sources.spikeDoc.includes('VideoFingerprintBundle') &&
    sources.spikeDoc.includes('不上传原始视频') &&
    sources.spikeDoc.includes('缩放') &&
    sources.spikeDoc.includes('二压') &&
    sources.spikeDoc.includes('裁剪') &&
    sources.spikeDoc.includes('crop_windows') &&
    sources.spikeDoc.includes('30/30') &&
    sources.spikeDoc.includes('不能只上传整帧 root'),
  'Phase 7 L2 spike doc must explain local-only sample evaluation, attack matrix, and crop-window API decision',
);
assert(
  sources.l2ApiDoc.includes('POST /v1/video-fingerprints/notaries') &&
    sources.l2ApiDoc.includes('POST /v1/video-fingerprints/search') &&
    sources.l2ApiDoc.includes('global_frame_fingerprints') &&
    sources.l2ApiDoc.includes('local_block_fingerprint_root') &&
    sources.l2ApiDoc.includes('crop_window_fingerprint_root') &&
    sources.l2ApiDoc.includes('crop_window_count') &&
    sources.l2ApiDoc.includes('client_signature') &&
    sources.l2ApiDoc.includes('upload_manifest'),
  'L2 API draft must define notary/search endpoints and the three-layer irreversible fingerprint fields',
);
assert(
  sources.l2ApiDoc.includes('contains_original_video": false') &&
    sources.l2ApiDoc.includes('contains_watermarked_video": false') &&
    sources.l2ApiDoc.includes('contains_local_paths": false') &&
    sources.l2ApiDoc.includes('original_video_forbidden') &&
    sources.l2ApiDoc.includes('watermarked_video_forbidden') &&
    sources.l2ApiDoc.includes('local_path_forbidden'),
  'L2 API draft must preserve manifest privacy rejection for original media and local paths',
);
assert(
  sources.l2ApiDoc.includes('L2 不扣 `video_minutes`') &&
    sources.l2ApiDoc.includes('usage_ledger') &&
    sources.l2ApiDoc.includes('crop_windows_required') &&
    sources.l2ApiDoc.includes('服务端拒绝缺少 `crop_window_fingerprint_root`'),
  'L2 API draft must keep usage-ledger accounting, no video-minute charge, and crop-window rejection semantics',
);
assert(
    sources.backendLib.includes('/v1/video-fingerprints/notaries') &&
    sources.backendLib.includes('/v1/video-tasks') &&
    sources.backendLib.includes('/v1/video-tasks/object-upload-authorizations') &&
    sources.backendLib.includes('/v1/video-object-store/upload') &&
    sources.backendLib.includes('/v1/video-tasks/:task_id') &&
    sources.backendLib.includes('/v1/video-tasks/:task_id/output-download-authorizations') &&
    sources.backendLib.includes('/v1/video-tasks/:task_id/output-download') &&
    sources.backendLib.includes('/v1/video-tasks/:task_id/status') &&
    sources.backendLib.includes('/internal/video-tasks/claim') &&
    sources.backendLib.includes('/internal/video-tasks/:task_id/completion') &&
    sources.backendLib.includes('/internal/video-tasks/:task_id/failure') &&
    sources.backendSchema.includes('VideoFingerprintNotaryRequest') &&
    sources.backendSchema.includes('CloudVideoTaskRequest') &&
    sources.backendSchema.includes('CloudVideoTaskCompletionRequest') &&
    sources.backendSchema.includes('CloudVideoTaskObjectUploadAuthorizationRequest') &&
    sources.backendSchema.includes('CloudVideoTaskObjectUploadAuthorizationResponse') &&
    sources.backendSchema.includes('CloudVideoTaskObjectUploadResponse') &&
    sources.backendSchema.includes('CloudVideoTaskDownloadAuthorizationRequest') &&
    sources.backendSchema.includes('CloudVideoTaskDownloadAuthorizationResponse') &&
    sources.backendSchema.includes('CloudVideoTaskDownloadAuthorizationQuery') &&
    sources.backendSchema.includes('CloudVideoTaskClaimRequest') &&
    sources.backendSchema.includes('CloudVideoTaskClaimResponse') &&
    sources.backendSchema.includes('CloudVideoTaskFailureRequest') &&
    sources.backendSchema.includes('CloudVideoTaskRecord') &&
    sources.backendSchema.includes('attempt_id') &&
    sources.backendSchema.includes('lease_token') &&
    sources.backendSchema.includes('attempt_count') &&
    sources.backendSchema.includes('output_media_storage_ref') &&
    sources.backendSchema.includes('worker_receipt_hash') &&
    sources.backendSchema.includes('worker_receipt') &&
    sources.backendSchema.includes('VideoUploadManifest') &&
    sources.backendSchema.includes('storage_ref') &&
    sources.backendSchema.includes('sandbox_profile') &&
    sources.backendSchema.includes('transcode_profile') &&
    sources.backendStorage.includes('create_video_fingerprint_notary') &&
    sources.backendStorage.includes('video_fingerprint_notaries') &&
    sources.backendStorage.includes('create_cloud_video_task') &&
    sources.backendStorage.includes('list_cloud_video_tasks') &&
    sources.backendStorage.includes('get_cloud_video_task') &&
    sources.backendStorage.includes('get_cloud_video_task_for_signed_download') &&
    sources.backendStorage.includes('update_cloud_video_task_status') &&
    sources.backendStorage.includes('claim_cloud_video_task_for_worker') &&
    sources.backendStorage.includes('fail_cloud_video_task_from_trusted_worker') &&
    sources.backendStorage.includes('cloud_video_tasks') &&
    sources.backendStorage.includes('cloud_usage_ledger'),
  'feedback backend must expose L2 notary and L3 task schema, routes, persistence, and usage ledger',
);
assert(
  sources.backendStorage.includes('crop_windows_required') &&
    sources.backendStorage.includes('original_video_forbidden') &&
    sources.backendStorage.includes('watermarked_video_forbidden') &&
    sources.backendStorage.includes('local_path_forbidden') &&
    sources.backendStorage.includes('cloud_video_task_schema_invalid') &&
    sources.backendStorage.includes('cloud_video_task_capability_invalid') &&
    sources.backendStorage.includes('cloud_video_task_failure_code_required') &&
    sources.backendStorage.includes('strategy_digest_required') &&
    sources.backendStorage.includes('self_check_threshold_required') &&
    sources.backendStorage.includes('self_check_confidence_required') &&
    sources.backendStorage.includes('self_check_confidence_below_threshold') &&
    sources.backendStorage.includes('checked_frames_required') &&
    sources.backendStorage.includes('watermarked_media_hash_required') &&
    sources.backendStorage.includes('server_receipt_signature_required') &&
    sources.backendStorage.includes('cloud_video_task_completion_requires_trusted_worker') &&
    sources.backendStorage.includes('cloud_video_task_queue_empty') &&
    sources.backendStorage.includes('cloud_video_task_completion_stale_attempt') &&
    sources.backendStorage.includes('cloud_video_task_already_succeeded') &&
    sources.backendStorage.includes('sandbox_transcode_failed') &&
    sources.backendStorage.includes('manifest_invalid') &&
    sources.backendStorage.includes('worker_receipt_invalid') &&
    sources.backendStorage.includes('lease_token_hash') &&
    sources.backendStorage.includes('output_media_storage_ref') &&
    sources.backendStorage.includes('worker_receipt_hash_mismatch') &&
    sources.backendStorage.includes('object://l3-output/') &&
    sources.backendLib.includes('create_cloud_video_task_object_upload_authorization') &&
    sources.backendLib.includes('resolve_cloud_video_task_object_upload_authorization') &&
    sources.backendLib.includes('l3_object_upload_authorization_v1') &&
    sources.backendLib.includes('hidden-shield:l3-object-upload:v1') &&
    sources.backendLib.includes('signed_object_upload_only_no_local_path_no_raw_video_sync') &&
    sources.backendLib.includes('l3_object_storage_ref_to_path') &&
    sources.backendLib.includes('HIDDENSHIELD_L3_OBJECT_STORE_DIR') &&
    sources.backendLib.includes('validate_l3_completion_receipt') &&
    sources.backendLib.includes('hmac-sha256:l3-completion-v1') &&
    sources.backendLib.includes('create_cloud_video_task_output_download_authorization') &&
    sources.backendLib.includes('resolve_cloud_video_task_output_download_authorization') &&
    sources.backendLib.includes('l3_output_download_authorization_v1') &&
    sources.backendLib.includes('hidden-shield:l3-output-download:v1') &&
    sources.backendLib.includes('cloud_video_task_output_not_ready') &&
    sources.backendLib.includes('signed_download_authorization_only_no_local_path_no_raw_upload') &&
    sources.backendStorage.includes('cloud_video_processing') &&
    sources.backendStorage.includes('video_minutes') &&
    sources.backendStorage.includes('quota_type, quota_units') &&
    sources.backendStorage.includes("'video_fingerprint_notary', 'usage_ledger', NULL, 0"),
  'feedback backend must reject missing crop windows/media uploads/local paths, keep L2 out of video_minutes quota, and charge L3 in video_minutes',
);
assert(
  sources.videoE2e.includes('strategyDigest') &&
    sources.videoE2e.includes('selfCheckThreshold') &&
    sources.videoE2e.includes('selfCheckConfidence') &&
    sources.videoE2e.includes('checkedFrames') &&
    sources.videoE2e.includes('watermarkedMediaHash') &&
    sources.videoE2e.includes('claim.body.leaseToken') &&
    sources.videoE2e.includes('user bearer L3 succeeded update must be rejected') &&
    sources.videoE2e.includes('usageLedgerId'),
  'cloud video E2E must prove L3 succeeded requires trusted worker/admin completion with a complete self-check receipt before charging video_minutes',
);
assert(
  sources.l3WorkerFixture.includes('build_video_feature_bundle') &&
    sources.l3WorkerFixture.includes('build_video_visual_payload') &&
    sources.l3WorkerFixture.includes('derive_video_visual_strategy') &&
    sources.l3WorkerFixture.includes('embed_video_visual_dct_frames') &&
    sources.l3WorkerFixture.includes('self_check_video_visual_dct_frames') &&
    sources.l3WorkerFixture.includes('"algorithmSource": "watermark-core"') &&
    sources.l3WorkerFixture.includes('"fixtureOnly": true') &&
    sources.l3WorkerFixture.includes('"payloadWatermarkUid": payload.watermark_uid()') &&
    sources.l3WorkerE2e.includes('l3_controlled_worker_fixture') &&
    sources.l3WorkerE2e.includes('l3_controlled_worker_fixture_v1') &&
    sources.l3WorkerE2e.includes('worker.watermarkUid === task.watermarkUid') &&
    sources.l3WorkerE2e.includes('worker.payloadWatermarkUid') &&
    sources.l3WorkerE2e.includes('/internal/video-tasks/${task.taskId}/completion') &&
    sources.l3WorkerE2e.includes('/internal/video-tasks/claim') &&
    sources.l3WorkerE2e.includes('cloud_video_task_completion_stale_attempt') &&
    sources.l3WorkerE2e.includes('cloud_video_task_already_succeeded') &&
    sources.l3WorkerE2e.includes('cloud_video_task_completion_requires_trusted_worker') &&
    sources.l3WorkerE2e.includes('worker.strategyDigest') &&
    sources.l3WorkerE2e.includes('worker.watermarkedMediaHash') &&
    sources.l3WorkerE2e.includes('usageLedgerId'),
  'cloud video CI must include a controlled L3 worker that calls watermark-core and completes through trusted worker/admin receipt API',
);
assert(
  sources.coreVideoVisual.includes('VideoVisualReservedPayloadBuildInput') &&
    sources.coreVideoVisual.includes('build_video_visual_payload_from_reserved_uid') &&
    sources.coreVideoVisual.includes('WatermarkIssueMode::ServerReserved') &&
    sources.coreVideoVisual.includes('WatermarkMediaType::VideoVisual') &&
    sources.l3RealWorker.includes('l3_real_worker_first_pass_v1') &&
    sources.l3RealWorker.includes('l3_controlled_upload_proxy') &&
    sources.l3RealWorker.includes('l3_user_object_upload_proxy') &&
    sources.l3RealWorker.includes('l3_ffmpeg_transcode_sandbox_v1') &&
    sources.l3RealWorker.includes('controlled://l3-upload-proxy/') &&
    sources.l3RealWorker.includes('object://l3-upload/') &&
    sources.l3RealWorker.includes('object://l3-output/') &&
    sources.l3RealWorker.includes('build_video_visual_payload_from_reserved_uid') &&
    sources.l3RealWorker.includes('payload.watermark_uid() != task.watermark_uid') &&
    sources.l3RealWorker.includes('input_storage_ref_to_path') &&
    sources.l3RealWorker.includes('validate_upload_object_bytes') &&
    sources.l3RealWorker.includes('run_ffmpeg_encode_luma_mp4') &&
    sources.l3RealWorker.includes('run_ffmpeg_decode') &&
    sources.l3RealWorker.includes('l3_worker_receipt_v1') &&
    sources.l3RealWorker.includes('"objectUploadOnly"') &&
    sources.l3RealWorker.includes('"downloadableObjectStoreObject"') &&
    sources.l3RealWorkerE2e.includes('/v1/watermark-ids/reserve') &&
    sources.l3RealWorkerE2e.includes('/v1/video-tasks/object-upload-authorizations') &&
    sources.l3RealWorkerE2e.includes('/v1/video-object-store/upload') &&
    sources.l3RealWorkerE2e.includes("mediaType: 'video_visual'") &&
    sources.l3RealWorkerE2e.includes('l3_real_worker_first_pass') &&
    sources.l3RealWorkerE2e.includes('--object-store-dir') &&
    sources.videoCi.includes('HIDDENSHIELD_L3_OBJECT_STORE_DIR') &&
    sources.l3RealWorkerE2e.includes('worker.payloadWatermarkUid === reserved.watermarkUid') &&
    sources.l3RealWorkerE2e.includes('worker.manifestBinding?.objectStoreRead === true') &&
    sources.l3RealWorkerE2e.includes('worker.outputPackaging?.downloadableObjectStoreObject === true') &&
    sources.l3RealWorkerE2e.includes('completed.body.outputMediaStorageRef === worker.outputMediaStorageRef') &&
    sources.l3RealWorkerE2e.includes('completed.body.workerReceiptHash === completion.workerReceiptHash') &&
    sources.l3RealWorkerE2e.includes('/v1/video-tasks/${taskId}/output-download-authorizations') &&
    sources.l3RealWorkerE2e.includes('/v1/video-tasks/${retryableTask.taskId}/output-download-authorizations') &&
    sources.l3RealWorkerE2e.includes('downloadToken?.startsWith') &&
    sources.l3RealWorkerE2e.includes('signedDownloadUrl') &&
    sources.l3RealWorkerE2e.includes('sha256Hex(resolved.bytes) === completedTask.watermarkedMediaHash') &&
    sources.l3RealWorkerE2e.includes('cloud_video_task_output_not_ready') &&
    sources.l3RealWorkerE2e.includes('tampered signed download token must be rejected') &&
    sources.l3RealWorkerE2e.includes('/v1/watermark-ids/confirm') &&
    sources.l3RealWorkerE2e.includes('/internal/video-tasks/${task.taskId}/completion') &&
    sources.l3RealWorkerE2e.includes('/internal/video-tasks/claim') &&
    sources.l3RealWorkerE2e.includes('/internal/video-tasks/${retryableTask.taskId}/failure') &&
    sources.l3RealWorkerE2e.includes('cloud_video_task_queue_empty') &&
    sources.l3RealWorkerE2e.includes('cloud_video_task_completion_stale_attempt') &&
    sources.l3RealWorkerE2e.includes('sandbox_transcode_failed') &&
    sources.l3RealWorkerE2e.includes('manifest_invalid') &&
    sources.l3RealWorkerE2e.includes('usageLedgerId == null'),
  'cloud video CI must include a real L3 worker first-pass that parses object upload manifest, uses a transcode sandbox, binds registry-reserved UID into watermark-core payload, confirms registry, completes through trusted API, proves real byte download, and proves claim/lease replay protection plus failure attribution',
);
assert(
  sources.desktopCloudClient.includes('VideoFingerprintNotaryRequest') &&
    sources.desktopCloudClient.includes('VideoFingerprintNotaryReceipt') &&
    sources.desktopCloudClient.includes('VideoFingerprintBundleForNotary') &&
    sources.desktopCloudClient.includes('video_fingerprint_bundle_to_notary_request') &&
    sources.desktopCloudClient.includes('video_fingerprint_spike_bundle_json_parses_into_notary_request') &&
    sources.desktopCloudClient.includes('video_fingerprint_bundle_file_smoke_binds_manifest_hash_and_size') &&
    sources.desktopCloudClient.includes('local_block_fingerprint_root') &&
    sources.desktopCloudClient.includes('crop_window_fingerprint_root') &&
    sources.desktopCloudClient.includes('video_upload_manifest_v1') &&
    sources.desktopCloudClient.includes('create_video_fingerprint_notary') &&
    sources.desktopCloudClient.includes('/v1/video-fingerprints/notaries') &&
    sources.desktopCloudClient.includes('contains_original_video') &&
    sources.desktopCloudClient.includes('contains_watermarked_video') &&
    sources.desktopCloudClient.includes('contains_local_paths'),
  'desktop cloud client must map L2 spike bundle into notary request with three-layer roots and preserve upload manifest privacy fields',
);
assert(
    sources.desktopSyncCommand.includes('CreateVideoFingerprintNotaryInput') &&
    sources.desktopSyncCommand.includes('CreateVideoFingerprintNotaryFromBundleFileInput') &&
    sources.desktopSyncCommand.includes('GenerateVideoFingerprintBundleInput') &&
    sources.desktopSyncCommand.includes('generate_video_fingerprint_bundle') &&
    sources.desktopSyncCommand.includes('video_fingerprint_bundles') &&
    sources.desktopSyncCommand.includes('create_video_fingerprint_notary_from_bundle_file') &&
    sources.desktopSyncCommand.includes('bundle.json') &&
    sources.desktopSyncCommand.includes('create_video_fingerprint_notary') &&
    sources.desktopSyncCommand.includes('profile.workspace_id') &&
    sources.desktopSyncCommand.includes('profile.creator_profile_id') &&
    sources.desktopLib.includes('commands::sync::create_video_fingerprint_notary') &&
    sources.desktopLib.includes('commands::sync::generate_video_fingerprint_bundle') &&
    sources.desktopLib.includes('commands::sync::create_video_fingerprint_notary_from_bundle_file'),
  'desktop Tauri command must expose L2 local bundle generation, notary, and bundle-file entry without UI media upload',
);
assert(
  sources.desktopVideoFingerprint.includes('VideoFingerprintBundleGeneration') &&
    sources.desktopVideoFingerprint.includes('generate_bundle') &&
    sources.desktopVideoFingerprint.includes('Some("mp4" | "mov" | "webm" | "avi" | "mkv" | "m4v")') &&
    sources.desktopVideoFingerprint.includes('l2_fingerprint_accepts_north_star_video_containers') &&
    sources.desktopVideoFingerprint.includes('frames-original') &&
    sources.desktopVideoFingerprint.includes('local_block_fingerprints') &&
    sources.desktopVideoFingerprint.includes('crop_window_fingerprints') &&
    sources.desktopVideoFingerprint.includes('bundle.json') &&
    !sources.desktopVideoFingerprint.includes('contains_original_video: true') &&
    !sources.desktopVideoFingerprint.includes('contains_watermarked_video: true'),
  'desktop local generator must create irreversible VideoFingerprintBundle with local blocks and crop windows only, and accept the North Star MP4/MOV/WEBM/AVI/MKV/M4V L2 container matrix',
);
assert(
  sources.roadmap.includes('定义云端视频任务模型') &&
    sources.roadmap.includes('桌面端视频云能力入口按 L1 可用、L2 锁定两层展示') &&
    sources.roadmap.includes('移动端视频云能力入口按 L1 可用、L2 锁定两层展示') &&
    sources.roadmap.includes('video_fingerprint_spike') &&
    sources.roadmap.includes('crop_window_fingerprint_root') &&
    sources.roadmap.includes('不能只保存整帧 root') &&
    sources.roadmap.includes('Phase 7 L2云端指纹存证API草案') &&
    sources.roadmap.includes('成功请求不扣 `video_minutes`') &&
    sources.roadmap.includes('create_video_fingerprint_notary') &&
    sources.roadmap.includes('桌面端 L2 存证 client 对接点'),
  'roadmap must track Phase 7 cloud video model and future capability entries',
);

console.log('Cloud video contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Cloud video contract failed: ${message}`);
    process.exit(1);
  }
}
