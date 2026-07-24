import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  videoCi: readFileSync('scripts/run-cloud-video-ci.mjs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  desktopCloudClient: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  desktopSyncCommand: readFileSync('src-tauri/src/commands/sync.rs', 'utf8'),
  desktopLib: readFileSync('src-tauri/src/lib.rs', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopReport: readFileSync('src-tauri/src/commands/report.rs', 'utf8'),
  desktopCloudSync: readFileSync('src-tauri/src/sync/cloud.rs', 'utf8'),
  mobileCloudClient: readFileSync('mobile_app/lib/sync/cloud_account_client.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileWorkspace: readFileSync('mobile_app/lib/features/workspace/workspace_page.dart', 'utf8'),
  mobileVideoMetadata: readFileSync('mobile_app/lib/features/workspace/video_metadata.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileStore: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  mobileSyncTransport: readFileSync('mobile_app/lib/sync/sync_transport.dart', 'utf8'),
  dualContract: readFileSync('scripts/verify-dual-consistency-contract.mjs', 'utf8'),
  sellableRuntimeQa: readFileSync('scripts/verify-l3-sellable-runtime-qa.mjs', 'utf8'),
  productionOpsRuntimeQa: readFileSync('scripts/verify-l3-production-ops-runtime-qa.mjs', 'utf8'),
  productionReadinessContract: readFileSync('scripts/verify-l3-production-readiness-contract.mjs', 'utf8'),
  qaRecord: readFileSync('docs/L3视频画面盲水印release_gate_QA记录.md', 'utf8'),
  sellableChecklist: readFileSync('docs/L3视频画面盲水印可售验收清单.md', 'utf8'),
  capabilityBoundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  commercialRoadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  dualRoadmap: readFileSync('docs/双端能力一致性Roadmap.md', 'utf8'),
  sharedCorePlan: readFileSync('docs/共享水印核心与跨端互验推进计划.md', 'utf8'),
};

assert(
  sources.packageJson.includes('"cloud-video:l3-product-flow-gate"') &&
    sources.videoCi.includes('verify-l3-video-visual-product-flow-gate.mjs') &&
    sources.packageJson.includes('"cloud-video:l3-cross-end-runtime-qa"') &&
    sources.videoCi.includes('verify-l3-video-visual-cross-end-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud-video:l3-sellable-runtime-qa"') &&
    sources.videoCi.includes('verify-l3-sellable-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud-video:l3-production-ops-runtime-qa"') &&
    sources.videoCi.includes('verify-l3-production-ops-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud-video:l3-production-readiness-contract"') &&
    sources.videoCi.includes('verify-l3-production-readiness-contract.mjs') &&
    sources.productionReadinessContract.includes('HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY'),
  'L3 product flow, cross-end runtime, sellable runtime, production ops runtime, and production readiness gates must be exposed and included in cloud-video:ci',
);

assert(
  sources.backendLib.includes('/v1/video-tasks/:task_id/output-download-authorizations') &&
    sources.backendLib.includes('/v1/video-tasks/:task_id/output-download') &&
    sources.backendLib.includes('validate_l3_output_download_ready') &&
    sources.backendStorage.includes('self_check_confidence_below_threshold') &&
    sources.backendStorage.includes('checked_frames_required') &&
    sources.backendStorage.includes('worker_receipt_hash_mismatch') &&
    sources.backendStorage.includes('cloud_video_task_completion_requires_trusted_worker') &&
    sources.backendStorage.includes('"strategy_invalid"') &&
    sources.backendStorage.includes('l3_strategy_capacity_insufficient') &&
    sources.backendStorage.includes('video_minutes'),
  'backend must keep L3 succeeded and download authorization bound to trusted self-check completion before charging video_minutes',
);

assert(
  sources.desktopCloudClient.includes('pub struct CloudVideoTaskRecord') &&
    sources.desktopCloudClient.includes('CloudVideoTaskDownloadAuthorizationResponse') &&
    sources.desktopCloudClient.includes('CloudVideoTaskObjectUploadAuthorizationRequest') &&
    sources.desktopCloudClient.includes('CloudVideoTaskObjectUploadResponse') &&
    sources.desktopCloudClient.includes('get_cloud_video_task') &&
    sources.desktopCloudClient.includes('create_cloud_video_task') &&
    sources.desktopCloudClient.includes('create_cloud_video_task_object_upload_authorization') &&
    sources.desktopCloudClient.includes('upload_cloud_video_task_object_bytes') &&
    sources.desktopCloudClient.includes('create_cloud_video_task_download_authorization') &&
    sources.desktopCloudClient.includes('download_cloud_video_task_output') &&
    sources.desktopCloudClient.includes('parse_response_bytes') &&
    sources.desktopSyncCommand.includes('create_l3_video_visual_upload_task') &&
    sources.desktopSyncCommand.includes('l3_user_object_upload_proxy') &&
    sources.desktopSyncCommand.includes('reserve_watermark_id') &&
    sources.desktopSyncCommand.includes('media_type: "video_visual"') &&
    sources.desktopSyncCommand.includes('object://l3-upload/') &&
    sources.desktopSyncCommand.includes('hybrid_visual_watermark') &&
    sources.desktopSyncCommand.includes('signed_object_upload_only_no_local_path_no_raw_video_sync') &&
    sources.desktopSyncCommand.includes('save_l3_video_visual_task_to_vault') &&
    sources.desktopSyncCommand.includes('validate_l3_video_visual_task_for_vault') &&
    sources.desktopSyncCommand.includes('"hybrid_visual_watermark"') &&
    sources.desktopSyncCommand.includes('confidence < threshold') &&
    sources.desktopSyncCommand.includes('object://l3-output/') &&
    sources.desktopSyncCommand.includes('output_sha256 != expected_hash') &&
    sources.desktopSyncCommand.includes('queries::insert_record_tx') &&
    sources.desktopSyncCommand.includes('enqueue_desktop_record_for_cloud') &&
    sources.desktopSyncCommand.includes('video_visual_task_id: Some(task.task_id.clone())') &&
    sources.desktopSyncCommand.includes('video_visual_media_hash: task.watermarked_media_hash.clone()') &&
    sources.desktopLib.includes('create_l3_video_visual_upload_task') &&
    sources.desktopLib.includes('save_l3_video_visual_task_to_vault'),
  'desktop must create a formal upload task, then download a real succeeded L3 task, verify bytes/hash, persist video_visual_* metadata, and enqueue cloud sync',
);

assert(
  sources.desktopApi.includes('createL3VideoVisualUploadTask') &&
    sources.desktopApi.includes('CreateL3VideoVisualUploadTaskResult') &&
    sources.desktopApi.includes('status: "queued"') &&
    sources.desktopApi.includes('signed_object_upload_only_no_local_path_no_raw_video_sync') &&
    sources.desktopApi.includes('saveL3VideoVisualTaskToVault') &&
    sources.desktopApi.includes('SaveL3VideoVisualTaskResult') &&
    sources.desktopWorkbench.includes('createL3VideoVisualUploadTask') &&
    sources.desktopWorkbench.includes('创建并上传 L3 任务') &&
    sources.desktopWorkbench.includes('等待 trusted worker') &&
    sources.desktopWorkbench.includes('失败归因') &&
    sources.desktopWorkbench.includes('strategy_invalid 容量不足') &&
    sources.desktopWorkbench.includes('容量预检') &&
    sources.desktopWorkbench.includes('隐私边界') &&
    sources.desktopWorkbench.includes('当前只接收 MP4') &&
    sources.desktopWorkbench.includes('saveL3VideoVisualTaskToVault') &&
    sources.desktopWorkbench.includes('下载并保存版权库') &&
    sources.desktopWorkbench.includes('trusted worker succeeded') &&
    sources.desktopWorkbench.includes('video_visual_* 收据元数据') &&
    sources.desktopWorkbench.includes('查看版权库') &&
    sources.desktopVault.includes('L3 视频画面盲水印') &&
    sources.desktopVault.includes('videoVisualMediaHash') &&
    sources.desktopReport.includes('videoVisualWatermark') &&
    sources.desktopReport.includes('formal_report_includes_l3_video_visual_receipt_without_paths_or_urls'),
  'desktop UI/report must expose the succeeded-task download -> vault -> formal report product flow',
);

assert(
  sources.mobileCloudClient.includes('createCloudVideoTaskObjectUploadAuthorization') &&
    sources.mobileCloudClient.includes('uploadCloudVideoTaskObjectBytes') &&
    sources.mobileCloudClient.includes('createCloudVideoTask') &&
    sources.mobileCloudClient.includes('CloudVideoTaskObjectUploadAuthorization') &&
    sources.mobileCloudClient.includes('CloudVideoTaskObjectUploadResponse') &&
    sources.mobileCloudClient.includes('getCloudVideoTask') &&
    sources.mobileCloudClient.includes('createCloudVideoTaskDownloadAuthorization') &&
    sources.mobileCloudClient.includes('downloadCloudVideoTaskOutput') &&
    sources.mobileCloudClient.includes('CloudVideoTaskRecord') &&
    sources.mobileCloudClient.includes('CloudVideoTaskDownloadAuthorization') &&
    sources.mobileState.includes('Future<L3VideoVisualUploadTaskResult> createL3VideoVisualUploadTaskFromBytes') &&
    sources.mobileState.includes("mediaType: 'video_visual'") &&
    sources.mobileState.includes("capabilityLevel': 'hybrid_visual_watermark'") &&
    sources.mobileState.includes("storageRef': uploadResult.storageRef") &&
    sources.mobileState.includes('signed_object_upload_only_no_local_path_no_raw_video_sync') &&
    sources.mobileState.includes('Future<VaultRecord> saveL3VideoVisualTaskToVault') &&
    sources.mobileState.includes('_validateL3VideoVisualTaskForVault') &&
    sources.mobileState.includes("normalized == 'hybrid_visual_watermark'") &&
    sources.mobileState.includes("outputMediaStorageRef?.startsWith('object://l3-output/')") &&
    sources.mobileState.includes('outputSha256 != task.watermarkedMediaHash') &&
    sources.mobileState.includes('SyncQueueOperation.upsertVaultRecord') &&
    sources.mobileState.includes('videoVisualTaskId: task.taskId') &&
    sources.mobileState.includes('videoVisualMediaHash: task.watermarkedMediaHash') &&
    sources.mobileVideoMetadata.includes('inspectVideoMetadata') &&
    sources.mobileVideoMetadata.includes('trustedVideoMetadataProbe') &&
    sources.mobileVideoMetadata.includes('tkhd') &&
    sources.mobileVideoMetadata.includes('stts') &&
    sources.mobileVideoMetadata.includes('stsz') &&
    sources.mobileVideoMetadata.includes('frameRate') &&
    sources.mobileWorkspace.includes('inspectVideoMetadata') &&
    sources.mobileWorkspace.includes('可信视频探测') &&
    sources.mobileWorkspace.includes('width: metadata?.width') &&
    sources.mobileWorkspace.includes('height: metadata?.height') &&
    sources.mobileWorkspace.includes('frameCount: metadata?.frameCount') &&
    sources.mobileWorkspace.includes('创建并上传 L3 任务') &&
    sources.mobileWorkspace.includes('等待 trusted worker') &&
    sources.mobileWorkspace.includes('失败归因') &&
    sources.mobileWorkspace.includes('strategy_invalid 容量不足') &&
    sources.mobileWorkspace.includes('容量预检') &&
    sources.mobileWorkspace.includes('隐私边界') &&
    sources.mobileWorkspace.includes('当前只接收 MP4') &&
    sources.mobileWorkspace.includes('下载并保存版权库') &&
    sources.mobileWorkspace.includes('trusted worker succeeded') &&
    sources.mobileWorkspace.includes('videoVisual* 收据元数据') &&
    sources.mobileVault.includes('L3 视频画面盲水印') &&
    sources.mobileVault.includes('videoVisualMediaHash'),
  'mobile must use the same formal upload wizard and succeeded-task download/hash/vault/sync/report path for L3 video visual receipts',
);

assert(
  sources.desktopCloudSync.includes('"video_visual_task_id".to_string()') &&
    sources.mobileState.includes("'video_visual_task_id': videoVisualTaskId") &&
    sources.mobileSyncTransport.includes("json['video_visual_task_id']") &&
    sources.mobileStore.includes("'video_visual_task_id'") &&
    sources.dualContract.includes('L3 vault/report fields must persist only receipt metadata') &&
    sources.dualContract.includes('video_visual_task_id'),
  'cross-end sync must carry video_visual_* receipt metadata through desktop payload, mobile parser/storage, and dual contract',
);

assert(
  !sources.desktopSyncCommand.includes('video_visual_output_storage_ref') &&
    !sources.desktopApi.includes('videoVisualOutputStorageRef') &&
    !sources.mobileState.includes('video_visual_output_storage_ref') &&
    !sources.mobileState.includes('videoVisualOutputStorageRef') &&
    sources.desktopReport.includes('!json.contains("object://")') &&
    sources.desktopReport.includes('!json.contains("output-download")') &&
    sources.mobileState.includes('video_visual_watermark_receipt'),
  'L3 product flow must not store object refs, signed URLs, local paths, or media bytes in shared vault/report/sync fields',
);

assert(
    sources.sellableRuntimeQa.includes('desktop_square_motion_mp4') &&
    sources.sellableRuntimeQa.includes('mobile_square_detail_mp4') &&
    sources.sellableRuntimeQa.includes('desktop_landscape_motion_mp4') &&
    sources.sellableRuntimeQa.includes('mobile_square_small_high_fps_strategy_invalid') &&
    sources.sellableRuntimeQa.includes('desktop_vertical_9x16_motion_mp4') &&
    sources.sellableRuntimeQa.includes('mobile_landscape_1080p_motion_mp4') &&
    sources.sellableRuntimeQa.includes('desktop_real_motion_fixture_mp4') &&
    sources.sellableRuntimeQa.includes('fixtureFallback') &&
    sources.sellableRuntimeQa.includes('mobile_subtitle_dense_mp4') &&
    sources.sellableRuntimeQa.includes("expectedOutcome: 'input_rejected'") &&
    sources.sellableRuntimeQa.includes('l3_strategy_capacity_insufficient') &&
    sources.sellableRuntimeQa.includes('DCT mid-band frame bitstream exceeds block capacity') &&
    sources.sellableRuntimeQa.includes('usageLedgerId == null'),
  'sellable runtime QA must include expanded MP4 size/frame-rate samples and prove insufficient-capacity inputs are rejected before billing',
);

assert(
    sources.qaRecord.includes('正式创建 / 上传向导 + 失败文案 + 隐私边界') &&
    sources.qaRecord.includes('真实用户 MP4 样本池可售运行态 QA') &&
    sources.sellableChecklist.includes('cloud-video:l3-sellable-runtime-qa') &&
    sources.sellableChecklist.includes('desktop_square_motion_mp4') &&
    sources.sellableChecklist.includes('mobile_square_detail_mp4') &&
    sources.sellableChecklist.includes('desktop_landscape_motion_mp4') &&
    sources.sellableChecklist.includes('mobile_square_small_high_fps_strategy_invalid') &&
    sources.sellableChecklist.includes('desktop_vertical_9x16_motion_mp4') &&
    sources.sellableChecklist.includes('mobile_landscape_1080p_motion_mp4') &&
    sources.sellableChecklist.includes('desktop_real_motion_fixture_mp4') &&
    sources.sellableChecklist.includes('mobile_subtitle_dense_mp4') &&
    sources.sellableChecklist.includes('l3_strategy_capacity_insufficient') &&
    sources.sellableChecklist.includes('DCT mid-band frame bitstream exceeds block capacity') &&
    sources.capabilityBoundary.includes('桌面 / 移动已接入 Studio / Enterprise 创建上传向导') &&
    sources.capabilityBoundary.includes('真实用户 MP4 最小样本池') &&
    sources.capabilityBoundary.includes('strategy_invalid 容量不足') &&
    sources.commercialRoadmap.includes('创建上传向导 + 失败文案 + 隐私边界') &&
    sources.commercialRoadmap.includes('真实用户 MP4 样本池运行态 QA') &&
    sources.commercialRoadmap.includes('512x512@2fps'),
  'docs must record that formal upload wizard and sellable runtime QA are gated and still not a sellable L3 SLA until remaining gates pass',
);

assert(
  sources.productionOpsRuntimeQa.includes('l3_production_queue_monitor_snapshot_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_production_worker_attempt_sla_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_object_storage_cleanup_policy_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_production_on_call_alert_runbook_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_production_observability_dashboard_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_alert_platform_integration_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_alert_platform_delivery_dry_run_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_customer_opening_acceptance_checklist_v1') &&
    sources.productionOpsRuntimeQa.includes('l3_customer_opening_acceptance_dry_run_v1') &&
    sources.productionOpsRuntimeQa.includes('objectStoreCleanupPolicy') &&
    sources.productionOpsRuntimeQa.includes('onCallAlertRunbook') &&
    sources.productionOpsRuntimeQa.includes('productionObservabilityDashboard') &&
    sources.productionOpsRuntimeQa.includes('alertPlatformIntegration') &&
    sources.productionOpsRuntimeQa.includes('customerL3OpeningChecklist') &&
    sources.productionOpsRuntimeQa.includes('cloudVideoL3ProductionObservabilityDashboard') &&
    sources.productionOpsRuntimeQa.includes('cloudVideoL3AlertPlatformIntegration') &&
    sources.productionOpsRuntimeQa.includes('customerL3OpeningAcceptanceChecklist') &&
    sources.productionOpsRuntimeQa.includes('l3_queue_health') &&
    sources.productionOpsRuntimeQa.includes('l3_receipt_integrity') &&
    sources.productionOpsRuntimeQa.includes('l3_object_store_hygiene') &&
    sources.productionOpsRuntimeQa.includes('l3_billing_guard') &&
    sources.productionOpsRuntimeQa.includes('cloud-video-on-call-primary') &&
    sources.productionOpsRuntimeQa.includes('customer-support-l3-failures') &&
    sources.productionOpsRuntimeQa.includes('finance-video-minutes-guard') &&
    sources.productionOpsRuntimeQa.includes('confirm_entitlement_and_video_minutes') &&
    sources.productionOpsRuntimeQa.includes('run_customer_fixture_mp4_dry_run') &&
    sources.productionOpsRuntimeQa.includes('verify_desktop_mobile_vault_report_readback') &&
    sources.productionOpsRuntimeQa.includes('confirm_no_media_path_or_signed_url_in_report_sync') &&
    sources.productionOpsRuntimeQa.includes('customer_signoff_l3_release_candidate_not_sla') &&
    sources.productionOpsRuntimeQa.includes('cleanup_failed_or_canceled_upload_objects') &&
    sources.productionOpsRuntimeQa.includes('retain_succeeded_outputs_for_vault_report_window') &&
    sources.productionOpsRuntimeQa.includes('l3_queued_backlog_sla_breach') &&
    sources.productionOpsRuntimeQa.includes('l3_running_lease_expired_or_stuck') &&
    sources.productionOpsRuntimeQa.includes('l3_retry_exhaustion_or_failure_spike') &&
    sources.productionOpsRuntimeQa.includes('l3_receipt_validation_failure') &&
    sources.productionOpsRuntimeQa.includes('l3_object_storage_cleanup_failure') &&
    sources.productionOpsRuntimeQa.includes('l3_billing_guard_violation') &&
    sources.productionOpsRuntimeQa.includes('customerFailureMessageMatrix') &&
    sources.productionOpsRuntimeQa.includes('rollback_requeue_retryable_then_hold_failed_no_charge') &&
    sources.productionOpsRuntimeQa.includes('watch_lease_until_expiry_then_reclaim') &&
    sources.productionOpsRuntimeQa.includes('cloud_video_task_output_not_ready') &&
    sources.productionOpsRuntimeQa.includes('l3_strategy_capacity_insufficient') &&
    sources.productionOpsRuntimeQa.includes('sandbox_transcode_failed') &&
    sources.productionOpsRuntimeQa.includes('core_strategy_failed') &&
    sources.productionOpsRuntimeQa.includes('strategy_invalid') &&
    sources.productionOpsRuntimeQa.includes('self_check_failed') &&
    sources.productionOpsRuntimeQa.includes('self_check_confidence_below_threshold') &&
    sources.productionOpsRuntimeQa.includes('worker_receipt_invalid') &&
    sources.productionOpsRuntimeQa.includes('manifest_invalid') &&
    sources.qaRecord.includes('生产队列运行态监控 + worker attempt SLA / 回滚演练 + 客服失败文案矩阵') &&
    sources.qaRecord.includes('移动端可信视频尺寸 / 帧率探测 + 对象存储清理策略 + 生产 on-call 告警 runbook') &&
    sources.qaRecord.includes('生产 observability 面板 / 告警平台接入 + 客户开通验收清单') &&
    sources.sellableChecklist.includes('cloud-video:l3-production-ops-runtime-qa') &&
    sources.sellableChecklist.includes('移动端可信视频尺寸 / 帧率探测') &&
    sources.sellableChecklist.includes('对象存储清理策略') &&
    sources.sellableChecklist.includes('on-call 告警 runbook') &&
    sources.sellableChecklist.includes('生产 observability 面板') &&
    sources.sellableChecklist.includes('告警平台接入') &&
    sources.sellableChecklist.includes('客户开通验收清单') &&
    sources.capabilityBoundary.includes('生产队列运行态监控') &&
    sources.capabilityBoundary.includes('移动端可信视频尺寸 / 帧率探测') &&
    sources.capabilityBoundary.includes('对象存储清理策略') &&
    sources.capabilityBoundary.includes('on-call 告警') &&
    sources.capabilityBoundary.includes('生产 observability 面板') &&
    sources.capabilityBoundary.includes('告警平台接入') &&
    sources.capabilityBoundary.includes('客户开通验收') &&
    sources.commercialRoadmap.includes('生产队列运行态监控') &&
    sources.commercialRoadmap.includes('移动端可信视频尺寸 / 帧率探测') &&
    sources.commercialRoadmap.includes('生产 observability 面板') &&
    sources.commercialRoadmap.includes('客户开通验收清单') &&
    sources.dualRoadmap.includes('移动端可信视频尺寸 / 帧率探测') &&
    sources.sharedCorePlan.includes('移动端可信视频尺寸 / 帧率探测'),
  'L3 production ops gate must cover mobile trusted probing, object cleanup, observability, alert platform routing, customer opening checklist, customer failure messaging, and docs',
);

console.log('L3 video visual product flow gate OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`L3 video visual product flow gate failed: ${message}`);
    process.exit(1);
  }
}
