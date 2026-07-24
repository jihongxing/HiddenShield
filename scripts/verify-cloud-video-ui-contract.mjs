import { readFileSync } from 'node:fs';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  videoCi: readFileSync('scripts/run-cloud-video-ci.mjs', 'utf8'),
  desktopWorkbench: readFileSync('src/views/WorkbenchView.vue', 'utf8'),
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopApi: readFileSync('src/lib/tauri-api.ts', 'utf8'),
  mobileWorkspace: readFileSync('mobile_app/lib/features/workspace/workspace_page.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileStore: readFileSync('mobile_app/lib/storage/vault_store.dart', 'utf8'),
  mobileSyncTransport: readFileSync('mobile_app/lib/sync/sync_transport.dart', 'utf8'),
  mobileWidgetTest: readFileSync('mobile_app/test/widget_test.dart', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
};

assert(
  sources.packageJson.includes('"cloud-video:ui-contract"') &&
    sources.videoCi.includes('verify-cloud-video-ui-contract.mjs'),
  'cloud video UI contract must be runnable and included in cloud-video:ci',
);

assert(
  sources.desktopWorkbench.includes('视频指纹存证') &&
    sources.desktopWorkbench.includes('L1 本地写入') &&
    sources.desktopWorkbench.includes('generateVideoFingerprintBundle') &&
    sources.desktopWorkbench.includes('handleGenerateVideoBundle') &&
    sources.desktopWorkbench.includes('videoBundleResult') &&
    sources.desktopWorkbench.includes('指纹包已生成，可确认提交云端存证') &&
    sources.desktopWorkbench.includes('handleSubmitGeneratedVideoBundle') &&
    sources.desktopWorkbench.includes('createVideoFingerprintNotaryFromBundleFile') &&
    sources.desktopWorkbench.includes('云端存证已完成') &&
    sources.desktopWorkbench.includes('已保存到版权库') &&
    sources.desktopWorkbench.includes('videoNotaryVaultRecord') &&
    sources.desktopWorkbench.includes('生成指纹包') &&
    sources.desktopWorkbench.includes('提交存证') &&
    sources.desktopWorkbench.includes('选择指纹包'),
  'desktop workbench must preserve the generate bundle -> submit notary -> saved vault record UI flow',
);

assert(
  sources.desktopWorkbench.includes('同步和报告只保存收据元数据') &&
    sources.desktopWorkbench.includes('L3 对象上传入口') &&
    sources.desktopWorkbench.includes('已 succeeded 的 L3 对象任务') &&
    sources.desktopWorkbench.includes('video_visual_* 收据元数据') &&
    sources.desktopWorkbench.includes('下载并保存版权库') &&
    sources.desktopWorkbench.includes('查看版权库') &&
    sources.desktopWorkbench.includes('bundleElapsedMs') &&
    sources.desktopWorkbench.includes('打开位置'),
  'desktop workbench must explain privacy boundary, object-upload L3 release gate, elapsed metadata, and local bundle location action',
);

assert(
  sources.desktopVault.includes('recordKindLabel') &&
    sources.desktopVault.includes('if (record.videoNotaryId) return "视频指纹存证"') &&
    sources.desktopVault.includes('vault-video-notary-badge') &&
    sources.desktopVault.includes('selectedLineageRecord.videoNotaryId') &&
    sources.desktopVault.includes('selectedLineageRecord.videoFingerprintRoot') &&
    sources.desktopVault.includes('selectedLineageRecord.videoBundleSha256') &&
    sources.desktopVault.includes('同步此记录') &&
    sources.desktopVault.includes('导出正式报告'),
  'desktop vault must show video notary records, sync them, and keep formal report access',
);

assert(
  !sources.desktopVault.includes('bundlePath') &&
    !sources.desktopVault.includes('originalVideoPath') &&
    !sources.desktopApi.includes('originalVideoPath'),
  'desktop vault records must not expose local bundle paths or original video paths',
);

assert(
  sources.mobileWorkspace.includes('视频指纹存证') &&
    sources.mobileWorkspace.includes('视频指纹存证与 L3 对象上传入口') &&
    sources.mobileWorkspace.includes('L1 视频音轨水印') &&
    sources.mobileWorkspace.includes('提交 L2 指纹存证') &&
    sources.mobileWorkspace.includes('createL2VideoFingerprintNotaryFromBytes') &&
    sources.mobileWorkspace.includes('不可逆 metadata 指纹包') &&
    sources.mobileWorkspace.includes('查看同步来的视频指纹存证记录') &&
    sources.mobileWorkspace.includes('Studio / Enterprise release gate') &&
    sources.mobileWorkspace.includes('下载并保存版权库') &&
    sources.mobileWorkspace.includes('videoVisual* 收据元数据') &&
    sources.mobileWorkspace.includes('对象上传入口') &&
    sources.mobileWorkspace.includes('不保存本地路径、对象 ref 或签名 URL'),
  'mobile workspace must expose L2 notary submit/record flow and keep the same product boundary while exposing the Studio/Enterprise L3 object-upload release-gate entry',
);

assert(
  sources.mobileState.includes('videoNotaryId') &&
    sources.mobileState.includes('createL2VideoFingerprintNotaryFromBytes') &&
    sources.mobileState.includes('video_fingerprint_notary_request_v1') &&
    sources.mobileState.includes('mobile_video_fingerprint_metadata') &&
    sources.mobileState.includes('mobile_video_fingerprint_notary') &&
    sources.mobileState.includes('metadata_hash_only_no_raw_video_no_local_path') &&
    sources.mobileState.includes('videoFingerprintRoot') &&
    sources.mobileState.includes('videoBundleSha256') &&
    sources.mobileState.includes('videoFrameSamplePolicy') &&
    sources.mobileState.includes("'video_notary_id': videoNotaryId") &&
    sources.mobileState.includes("'video_bundle_sha256': videoBundleSha256"),
  'mobile state must submit L2 video notary metadata and preserve L2 fields for sync and vault rendering',
);

assert(
  sources.mobileStore.includes('video_notary_id') &&
    sources.mobileStore.includes('video_fingerprint_root') &&
    sources.mobileStore.includes('video_bundle_sha256') &&
    sources.mobileSyncTransport.includes('RemoteSyncChange.fromCloudJson') &&
    sources.mobileSyncTransport.includes('entityType') &&
    sources.mobileState.includes('SyncQueueOperation.upsertVaultRecord'),
  'mobile storage and sync transport must keep video notary records compatible with cloud sync',
);

assert(
  sources.mobileVault.includes("label: '视频'") &&
    sources.mobileVault.includes('WatermarkAssetKind.video') &&
    sources.mobileVault.includes('视频指纹存证: ${record.videoNotaryId}') &&
    sources.mobileVault.includes("title: '视频指纹存证'") &&
    sources.mobileVault.includes("label: '存证编号'") &&
    sources.mobileVault.includes("label: '指纹根'") &&
    sources.mobileVault.includes("label: '指纹包摘要'") &&
    sources.mobileVault.includes("label: '采样策略'") &&
    sources.mobileVault.includes('buildFormalReportDraft'),
  'mobile vault must filter, summarize, detail, and report L2 video notary records',
);

assert(
  sources.mobileWidgetTest.includes('opens synced video notary record details') &&
    sources.mobileWidgetTest.includes('提交 L2 指纹存证') &&
    sources.mobileWidgetTest.includes('videoNotaryId:') &&
    sources.mobileWidgetTest.includes('videoFingerprintRoot:') &&
    sources.mobileWidgetTest.includes('videoBundleSha256:') &&
    sources.mobileWidgetTest.includes('videoFrameSamplePolicy:'),
  'mobile widget tests must cover synced video notary record details',
);

assert(
  !sources.mobileVault.includes('bundlePath') &&
    !sources.mobileState.includes('bundlePath') &&
    !sources.mobileVault.includes('originalVideoPath') &&
    !sources.mobileState.includes('originalVideoPath'),
  'mobile UI/state must not expose local bundle paths or original video paths',
);

assert(
  sources.roadmap.includes('UI 闭环') &&
    sources.roadmap.includes('生成指纹包 -> 提交存证 -> 保存到版权库') &&
    sources.roadmap.includes('同步视频存证记录 -> 版权库查看 -> 正式报告草稿') &&
    sources.roadmap.includes('cloud-video:ui-contract'),
  'roadmap must record the Phase 7 UI contract status and next task',
);

console.log('Cloud video UI contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Cloud video UI contract failed: ${message}`);
    process.exit(1);
  }
}
