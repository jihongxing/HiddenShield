export type Platform = "douyin" | "bilibili" | "xiaohongshu";
export type AppTab = "workbench" | "vault" | "verify";

export interface SourceMeta {
  fileName: string;
  path: string;
  width: number;
  height: number;
  fps: number;
  durationSecs: number;
  fileSizeMb: number;
  isHdr: boolean;
  colorProfile: string;
  sha256: string;
  fileType: string;
}

export interface HardwareInfo {
  preferredEncoder: string;
  availableEncoders: string[];
  toneMappingSupported: boolean;
  ffmpegStatus: string;
}

export interface VaultRecord {
  id: number;
  fileName: string;
  createdAt: string;
  watermarkUid: string;
  originalHash: string;
  resolution: string;
  durationSecs: number;
  isHdrSource: boolean;
  outputDouyin: string | null;
  outputBilibili: string | null;
  outputXhs: string | null;
  hwEncoderUsed: string | null;
  processTimeMs: number | null;
  isAiGenerated: boolean;
  aiTrainingPermission: string | null;
  aiGenerationMethod: string | null;
  humanModificationLevel: string | null;
  authenticityClaim: string | null;
  customMetadata: string | null;
  outputDouyinHash: string | null;
  outputBilibiliHash: string | null;
  outputXhsHash: string | null;
  parentWatermarkUid: string | null;
  revision: number;
  rewriteReason: string | null;
}

export type EntitlementStatus = "free" | "trial" | "active" | "grace" | "expired";

export interface EntitlementState {
  status: EntitlementStatus;
  planName: string | null;
  billingSource: string | null;
  subscriptionId: string | null;
  trialStartedAt: string | null;
  trialEndsAt: string | null;
  currentPeriodStartedAt: string | null;
  currentPeriodEndsAt: string | null;
  graceEndsAt: string | null;
  lastCheckedAt: string | null;
  updatedAt: string;
}

export interface UsageLedgerSummary {
  totalUnits: number;
  totalEvents: number;
  imageUnits: number;
  videoUnits: number;
  audioUnits: number;
  lastUsedAt: string | null;
  lastFeatureName: string | null;
  entitlement: EntitlementState;
}

export type AnonymousEventOutcome = "success" | "failure" | "crash" | "diagnostic";

export interface AnonymousFeedbackStatus {
  installId: string;
  sessionId: string;
  queuedEvents: number;
  queuedBytes: number;
  lastEventAt: string | null;
  lastFlushAt: string | null;
  lastFlushError: string | null;
  consecutiveFailures: number;
  nextRetryAt: string | null;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  telemetryEnabled: boolean;
  acknowledged: boolean;
  networkEnabled: boolean;
  endpointConfigured: boolean;
}

export interface AnonymousFlushResult {
  attemptedEvents: number;
  sentEvents: number;
  remainingEvents: number;
  endpointConfigured: boolean;
  flushedAt: string | null;
  message: string;
}

export interface MobileSyncStatus {
  enabled: boolean;
  listenPort: number;
  listenAddress: string;
  pairingCode: string;
  receivedEvents: number;
  latestEventAt: string | null;
  resolutionCount: number;
  latestResolution: SyncResolutionSummary | null;
}

export interface SyncResolutionSummary {
  resolvedAt: string;
  resolutionType: string;
  reason: string;
  watermarkUid: string;
  desktopHash: string | null;
  mobileHash: string | null;
  desktopRevision: number | null;
  mobileRevision: number | null;
}

export interface DesktopCloudSyncProfile {
  cloudBaseUrl: string;
  accountId: string;
  accountLabel: string;
  accessToken: string;
  refreshToken: string;
  workspaceId: string;
  workspaceName: string;
  deviceId: string;
  deviceName: string | null;
  devicePlatform: string | null;
  creatorProfileId: string;
  creatorDisplayName: string;
  entitlementId: string;
  entitlementLabel: string;
  entitlementStatus: string;
  entitlementPlanCode: string;
  entitlementFeatures: Record<string, boolean>;
  lastRemoteCursor: string | null;
  updatedAt: string;
}

export interface CloudSyncBatchResult {
  accepted: number;
  acceptedEventIds: string[];
  nextCursor: string | null;
  resolutions: unknown;
}

export interface CloudQueueStatus {
  pending: number;
  failed: number;
  synced: number;
  retryExhausted: number;
  lastAttemptAt: string | null;
  lastSuccessAt: string | null;
  lastFailureAt: string | null;
  nextRetryAt: string | null;
  lastError: string | null;
}

export interface CloudQueueFlushResult {
  attempted: number;
  synced: number;
  failed: number;
  message: string;
}

export interface CloudSyncChange {
  cursor: string | null;
  entityType: string;
  operation: string;
  sourceDevice: string | null;
  entity: Record<string, unknown>;
}

export interface CloudSyncChangesResult {
  nextCursor: string;
  changes: CloudSyncChange[];
}

export interface CloudPullResult {
  nextCursor: string;
  totalChanges: number;
  applied: number;
  skipped: number;
  importedQueueIds: string[];
}

export interface VerificationResult {
  matched: boolean;
  watermarkUid: string | null;
  confidence: number;
  matchedRecord: VaultRecord | null;
  summary: string;
  reasonCode: string;
  reasonDetail: string;
  disclaimer: string;
  tsaTokenPresent: boolean;
  tsaTokenVerified: boolean;
  tsaVerificationPath: TsaVerificationPath | null;
  tsaSource: string | null;
  networkTime: string | null;
  createdAt: string | null;
  originalHash: string | null;
}

export interface RewriteTargetInspectionResult {
  supported: boolean;
  fileKind: string;
  hasWatermark: boolean;
  watermarkUid: string | null;
  detectedRevision: number | null;
  nextRevision: number;
  parentWatermarkUid: string | null;
  rewriteReason: string | null;
  summary: string;
  reasonCode: string;
  reasonDetail: string;
}

export type TsaVerificationPath = "systemRoots" | "embeddedRoots";

export interface AIContentOptions {
  isAiGenerated: boolean;
  trainingPermission: string;
  generationMethod: string;
  modificationLevel: string;
  authenticityClaim: string;
  customMetadata?: string;
}

export interface TranscodeOptions {
  aspectStrategy: "letterbox" | "smart_crop";
  encodingMode: "fast_gpu" | "high_quality_cpu";
  aiContent?: AIContentOptions;
  allowRewrite: boolean;
  rewriteReason?: string;
}

export interface PipelineStartResult {
  pipelineId: string;
  summary: string;
}

export interface PipelineProgressPayload {
  pipelineId: string;
  stage: string;
  percent: number;
  platformPercents: Record<Platform, number>;
}

export interface SystemCheckResult {
  ffmpegAvailable: boolean;
  ffmpegVersion: string;
  gpuEncoderAvailable: boolean;
  gpuEncoderName: string;
  diskFreeMb: number;
  diskSufficient: boolean;
  outputDirWritable: boolean;
  outputDir: string;
}

export interface OutputFileInfo {
  platform: string;
  path: string;
  sizeMb: number;
  resolution: string;
  fps: number;
}

export interface WriteVerificationInfo {
  verified: boolean;
  watermarkUid: string;
  revision: number;
  message: string;
}

export interface PipelineCompletePayload {
  pipelineId: string;
  watermarkUid: string;
  processTimeMs: number;
  encoderUsed: string;
  outputs: OutputFileInfo[];
  vaultRecord: VaultRecord;
  writeVerification?: WriteVerificationInfo | null;
}

export interface SourceWarning {
  type: "info" | "warning";
  message: string;
}

const platformOrder: Platform[] = ["douyin", "bilibili", "xiaohongshu"];

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function buildMockSource(path: string): SourceMeta {
  const baseName = path.split(/[\\/]/).pop() || "demo.mp4";
  const isHdr = /\.mov$/i.test(baseName) || /hdr/i.test(baseName);
  const lowerName = baseName.toLowerCase();

  const ext = baseName.split(".").pop()?.toLowerCase() ?? "";
  const imageExts = ["jpg", "jpeg", "png", "webp", "bmp", "tiff"];
  const audioExts = ["mp3", "wav", "flac", "aac", "ogg"];
  const fileType = imageExts.includes(ext) ? "image" : audioExts.includes(ext) ? "audio" : "video";
  const durationSecs =
    fileType === "audio" && lowerName.includes("short")
      ? 10
      : fileType === "audio" && lowerName.includes("long")
        ? 42
        : isHdr
          ? 74
          : 42;

  return {
    fileName: baseName,
    path,
    width: isHdr ? 3840 : 1920,
    height: isHdr ? 2160 : 1080,
    fps: isHdr ? 60 : 30,
    durationSecs,
    fileSizeMb: isHdr ? 824.6 : 186.2,
    isHdr,
    colorProfile: isHdr ? "BT.2020 / PQ" : "BT.709 / SDR",
    sha256: "9a6c64f07e0c13bbf501f0f61a68e18d64db4dce5f83f4cbe91e23ba9f92d0c5",
    fileType,
  };
}

const mockVault: VaultRecord[] = [
  {
    id: 1,
    fileName: "春日咖啡馆_VLOG.mov",
    createdAt: "2026-04-18 20:16:21",
    watermarkUid: "HS-26A4-7D91-CA8F",
    originalHash: "4ca9c53d98f5d88f5a5cbdb8b9107c14c3d3b3c9f8e2b4af",
    resolution: "3840x2160",
    durationSecs: 74,
    isHdrSource: true,
    outputDouyin: "/output/春日咖啡馆_VLOG_抖音优化版.mp4",
    outputBilibili: null,
    outputXhs: "/output/春日咖啡馆_VLOG_小红书优化版.mp4",
    hwEncoderUsed: "h264_videotoolbox",
    processTimeMs: 48200,
    isAiGenerated: false,
    aiTrainingPermission: null,
    aiGenerationMethod: null,
    humanModificationLevel: null,
    authenticityClaim: null,
    customMetadata: null,
    outputDouyinHash: "f8a8e4f22d7d5f0cdd7b8d7b5e8f8d7a4e0f5b3a2c1d0e9f1234567890abcdef",
    outputBilibiliHash: null,
    outputXhsHash: "9e8d7c6b5a49382716f0e1d2c3b4a5968776655443322110fedcba9876543210",
    parentWatermarkUid: null,
    revision: 1,
    rewriteReason: null,
  },
  {
    id: 2,
    fileName: "品牌开箱_B站长横屏.mp4",
    createdAt: "2026-04-17 23:42:09",
    watermarkUid: "HS-154B-2EF8-90D2",
    originalHash: "7e1958c0d0a2834328a28ee3dcc8e3806ad9faa2ce37ed09",
    resolution: "1920x1080",
    durationSecs: 311,
    isHdrSource: false,
    outputDouyin: null,
    outputBilibili: "/output/品牌开箱_B站长横屏_B站优化版.mp4",
    outputXhs: null,
    hwEncoderUsed: null,
    processTimeMs: 126800,
    isAiGenerated: true,
    aiTrainingPermission: "commercial",
    aiGenerationMethod: "text_to_video",
    humanModificationLevel: "moderate",
    authenticityClaim: "based_on_reality",
    customMetadata: "示例内容",
    outputDouyinHash: null,
    outputBilibiliHash: "2f3e4d5c6b7a8990a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0",
    outputXhsHash: null,
    parentWatermarkUid: null,
    revision: 1,
    rewriteReason: null,
  },
];

const mockEntitlement: EntitlementState = {
  status: "free",
  planName: null,
  billingSource: null,
  subscriptionId: null,
  trialStartedAt: null,
  trialEndsAt: null,
  currentPeriodStartedAt: null,
  currentPeriodEndsAt: null,
  graceEndsAt: null,
  lastCheckedAt: null,
  updatedAt: "2026-05-03T00:00:00Z",
};

const mockUsageSummary: UsageLedgerSummary = {
  totalUnits: 2,
  totalEvents: 2,
  imageUnits: 0,
  videoUnits: 2,
  audioUnits: 0,
  lastUsedAt: "2026-05-03T00:00:00Z",
  lastFeatureName: "watermark_video",
  entitlement: mockEntitlement,
};

// ---------------------------------------------------------------------------
// IPC Functions
// ---------------------------------------------------------------------------

export async function systemCheck(inputPath?: string): Promise<SystemCheckResult> {
  if (!isTauriRuntime()) {
    return {
      ffmpegAvailable: true,
      ffmpegVersion: "ffmpeg version 6.1 (mock)",
      gpuEncoderAvailable: false,
      gpuEncoderName: "libx264",
      diskFreeMb: 52480,
      diskSufficient: true,
      outputDirWritable: true,
      outputDir: "/mock/output",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SystemCheckResult>("system_check", { inputPath });
}

export async function openOutputDir(path: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_output_dir", { dirPath: path });
}

/** Check which file paths are missing/offline. Returns list of missing paths. */
export async function checkFilesExist(paths: string[]): Promise<string[]> {
  if (!isTauriRuntime()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("check_files_exist", { paths });
}

export async function probeSource(path: string): Promise<SourceMeta> {
  if (!isTauriRuntime()) {
    return buildMockSource(path);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SourceMeta>("probe_source", { path });
}

export async function getHardwareInfo(): Promise<HardwareInfo> {
  if (!isTauriRuntime()) {
    return {
      preferredEncoder: "software",
      availableEncoders: ["libx264", "libx265"],
      toneMappingSupported: true,
      ffmpegStatus: "skeleton-mode",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<HardwareInfo>("get_hw_info");
}

export async function listVaultRecords(): Promise<VaultRecord[]> {
  if (!isTauriRuntime()) {
    return mockVault;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VaultRecord[]>("list_vault_records");
}

export async function getEntitlementState(): Promise<EntitlementState> {
  if (!isTauriRuntime()) {
    return mockEntitlement;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<EntitlementState>("get_entitlement_state");
}

export async function setEntitlementState(state: EntitlementState): Promise<EntitlementState> {
  if (!isTauriRuntime()) {
    return state;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<EntitlementState>("set_entitlement_state", { entitlementState: state });
}

export async function getUsageLedgerSummary(): Promise<UsageLedgerSummary> {
  if (!isTauriRuntime()) {
    return mockUsageSummary;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<UsageLedgerSummary>("get_usage_ledger_summary");
}

export async function getAnonymousFeedbackStatus(): Promise<AnonymousFeedbackStatus> {
  if (!isTauriRuntime()) {
    return {
      installId: "inst-mock",
      sessionId: "sess-mock",
      queuedEvents: 0,
      queuedBytes: 0,
      lastEventAt: null,
      lastFlushAt: null,
      lastFlushError: null,
      consecutiveFailures: 0,
      nextRetryAt: null,
      lastAttemptAt: null,
      lastSuccessAt: null,
      telemetryEnabled: true,
      acknowledged: true,
      networkEnabled: true,
      endpointConfigured: false,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AnonymousFeedbackStatus>("get_anonymous_feedback_status");
}

export async function flushAnonymousFeedbackQueue(): Promise<AnonymousFlushResult> {
  if (!isTauriRuntime()) {
    return {
      attemptedEvents: 0,
      sentEvents: 0,
      remainingEvents: 0,
      endpointConfigured: false,
      flushedAt: null,
      message: "mock flush",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AnonymousFlushResult>("flush_anonymous_feedback_queue");
}

export async function getMobileSyncStatus(): Promise<MobileSyncStatus> {
  if (!isTauriRuntime()) {
    return {
      enabled: true,
      listenPort: 47219,
      listenAddress: "http://0.0.0.0:47219",
      pairingCode: "123456",
      receivedEvents: 0,
      latestEventAt: null,
      resolutionCount: 0,
      latestResolution: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<MobileSyncStatus>("get_mobile_sync_status");
}

export async function regenerateMobilePairingCode(): Promise<string> {
  if (!isTauriRuntime()) return "654321";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("regenerate_mobile_pairing_code");
}

export async function getDesktopCloudSyncProfile(): Promise<DesktopCloudSyncProfile | null> {
  if (!isTauriRuntime()) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile | null>("get_desktop_cloud_sync_profile");
}

export async function getDesktopCloudQueueStatus(): Promise<CloudQueueStatus> {
  if (!isTauriRuntime()) {
    return {
      pending: 0,
      failed: 0,
      synced: 0,
      retryExhausted: 0,
      lastAttemptAt: null,
      lastSuccessAt: null,
      lastFailureAt: null,
      nextRetryAt: null,
      lastError: null,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudQueueStatus>("get_desktop_cloud_queue_status");
}

export async function signOutDesktopCloud(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("sign_out_desktop_cloud");
}

export async function continueCloudAccount(
  identifier: string,
  creatorDisplayName: string,
  verificationCode = "",
): Promise<DesktopCloudSyncProfile> {
  if (!isTauriRuntime()) {
    return {
      cloudBaseUrl: "http://127.0.0.1:43188",
      accountId: "acct_mock",
      accountLabel: identifier,
      accessToken: "mock-access-token",
      refreshToken: "mock-refresh-token",
      workspaceId: "ws_mock",
      workspaceName: "个人空间",
      deviceId: "desktop-mock",
      deviceName: "Desktop Mock",
      devicePlatform: "web",
      creatorProfileId: "creator_mock",
      creatorDisplayName,
      entitlementId: "ent_mock",
      entitlementLabel: "免费版",
      entitlementStatus: "free",
      entitlementPlanCode: "free",
      entitlementFeatures: { cloud_sync: true, batch_processing: false, cloud_video_processing: false },
      lastRemoteCursor: null,
      updatedAt: new Date().toISOString(),
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopCloudSyncProfile>("continue_cloud_account", {
    input: { identifier, verificationCode, creatorDisplayName },
  });
}

export async function pushDesktopVaultRecordToCloud(
  baseUrl: string,
  accessToken: string,
  deviceId: string,
  workspaceId: string,
  recordId: number,
): Promise<CloudSyncBatchResult> {
  if (!isTauriRuntime()) {
    return {
      accepted: 1,
      acceptedEventIds: [`desktop-vault-${recordId}`],
      nextCursor: "cursor_mock",
      resolutions: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncBatchResult>("push_desktop_vault_record_to_cloud", {
    input: { baseUrl, accessToken, deviceId, workspaceId, recordId },
  });
}

export async function pushSavedDesktopVaultRecordToCloud(
  recordId: number,
): Promise<CloudSyncBatchResult> {
  if (!isTauriRuntime()) {
    return {
      accepted: 1,
      acceptedEventIds: [`desktop-vault-${recordId}`],
      nextCursor: "cursor_mock",
      resolutions: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncBatchResult>("push_saved_desktop_vault_record_to_cloud", {
    input: { recordId },
  });
}

export async function flushDesktopCloudSyncQueue(limit = 50): Promise<CloudQueueFlushResult> {
  if (!isTauriRuntime()) {
    return {
      attempted: 0,
      synced: 0,
      failed: 0,
      message: "mock flush",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudQueueFlushResult>("flush_desktop_cloud_sync_queue", {
    input: { limit },
  });
}

export async function pullSavedCloudChangesIntoDesktop(): Promise<CloudPullResult> {
  if (!isTauriRuntime()) {
    return {
      nextCursor: "cursor_mock",
      totalChanges: 0,
      applied: 0,
      skipped: 0,
      importedQueueIds: [],
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudPullResult>("pull_saved_cloud_changes_into_desktop");
}

export async function fetchCloudChanges(
  baseUrl: string,
  accessToken: string,
  workspaceId: string,
  cursor?: string,
): Promise<CloudSyncChangesResult> {
  if (!isTauriRuntime()) {
    return { nextCursor: "cursor_mock", changes: [] };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CloudSyncChangesResult>("fetch_cloud_changes", {
    input: { baseUrl, accessToken, workspaceId, cursor },
  });
}

export async function verifySuspect(path: string): Promise<VerificationResult> {
  if (!isTauriRuntime()) {
    const matchedRecord = mockVault[0];
    return {
      matched: true,
      watermarkUid: matchedRecord.watermarkUid,
      confidence: 0.88,
      matchedRecord,
      summary: "检测到有效水印样本，已命中本地版权金库中的作品记录。",
      reasonCode: "matched_original",
      reasonDetail: "水印有效，并且与本地版权库记录完成绑定。",
      disclaimer: "本报告仅基于既定算法进行特征码技术提取，仅供参考，不代表任何司法鉴定意见。平台不对因本报告引发的连带法律责任负责。",
      tsaTokenPresent: true,
      tsaTokenVerified: true,
      tsaVerificationPath: "systemRoots",
      tsaSource: "https://freetsa.org/tsr",
      networkTime: "Sat, 19 Apr 2026 10:30:00 GMT",
      createdAt: "2026-04-19T10:30:00Z",
      originalHash: "b69956820610c86f72e051ae0c32a54e9af8bfca69361ba3093a38d24dbdaeaa",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<VerificationResult>("verify_suspect", { path });
}

export async function inspectRewriteTarget(path: string): Promise<RewriteTargetInspectionResult> {
  if (!isTauriRuntime()) {
    return {
      supported: true,
      fileKind: "image",
      hasWatermark: false,
      watermarkUid: null,
      detectedRevision: null,
      nextRevision: 1,
      parentWatermarkUid: null,
      rewriteReason: null,
      summary: "未检测到已有隐盾水印，将按首次写入处理。",
      reasonCode: "first_write",
      reasonDetail: "写前预检没有提取到有效水印；如果继续写入，会创建新的版权存证。",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<RewriteTargetInspectionResult>("inspect_rewrite_target", { path });
}

export async function startPipeline(
  inputPath: string,
  platforms: Platform[],
  options: TranscodeOptions,
): Promise<PipelineStartResult> {
  if (!isTauriRuntime()) {
    return {
      pipelineId: `mock-${Date.now()}`,
      summary: `模拟启动 ${platforms.length} 个平台的压制任务。`,
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PipelineStartResult>("start_pipeline", { inputPath, platforms, options });
}

export async function cancelPipeline(pipelineId: string): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("cancel_pipeline", { pipelineId });
}

/** Check which pipelines are still active (for state reconciliation on focus). */
export async function checkActivePipelines(): Promise<string[]> {
  if (!isTauriRuntime()) return [];
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string[]>("check_active_pipelines");
}

export async function listenPipelineProgress(
  handler: (payload: PipelineProgressPayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<PipelineProgressPayload>("pipeline-progress", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export async function listenPipelineComplete(
  handler: (payload: PipelineCompletePayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<PipelineCompletePayload>("pipeline-complete", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export interface HwDegradationPayload {
  pipelineId: string;
  failedEncoder: string;
  fallbackEncoder: string;
  message: string;
}

export async function listenHwDegradation(
  handler: (payload: HwDegradationPayload) => void,
) {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<HwDegradationPayload>("hw-degradation", (event) => {
    handler(event.payload);
  });
  return unlisten;
}

export function createEmptyPlatformPercents() {
  return platformOrder.reduce(
    (acc, platform) => { acc[platform] = 0; return acc; },
    {} as Record<Platform, number>,
  );
}

// ---------------------------------------------------------------------------
// Pure Functions: Recommendations, Warnings, Summaries
// ---------------------------------------------------------------------------

export const MIN_AUDIO_PROTECTION_SECONDS = 30;

export function isStandaloneAudioTooShort(meta: SourceMeta | null): boolean {
  return !!meta &&
    meta.fileType === "audio" &&
    meta.durationSecs < MIN_AUDIO_PROTECTION_SECONDS;
}

/** Recommend platforms based on source meta (aspect ratio + duration). */
export function recommendPlatforms(meta: SourceMeta): Platform[] {
  const isVertical = meta.height > meta.width;
  const isSquare = Math.abs(meta.width - meta.height) / Math.max(meta.width, 1) < 0.1;
  const isShort = meta.durationSecs < 180;
  const isMedium = meta.durationSecs >= 180 && meta.durationSecs <= 1200;

  if (isVertical && isShort) return ["douyin", "xiaohongshu"];
  if (isVertical && isMedium) return ["douyin", "xiaohongshu", "bilibili"];
  if (!isVertical && !isSquare) return ["bilibili"];
  return ["douyin", "bilibili"];
}

/** Recommend transcode strategy based on source, platforms, and hardware. */
export function recommendStrategy(
  meta: SourceMeta,
  _platforms: Platform[],
  hwInfo: HardwareInfo,
): TranscodeOptions {
  const hasGpu = hwInfo.preferredEncoder !== "libx264" && hwInfo.preferredEncoder !== "libx265";
  const encodingMode: TranscodeOptions["encodingMode"] = hasGpu ? "fast_gpu" : "high_quality_cpu";

  // If source is landscape and targeting vertical platforms, default to letterbox
  const isLandscape = meta.width > meta.height;
  const aspectStrategy: TranscodeOptions["aspectStrategy"] = isLandscape ? "letterbox" : "letterbox";

  return { aspectStrategy, encodingMode, allowRewrite: false };
}

/** Generate warnings based on source meta and selected platforms. */
export function generateWarnings(meta: SourceMeta, platforms: Platform[]): SourceWarning[] {
  const warnings: SourceWarning[] = [];

  if (meta.isHdr) {
    warnings.push({ type: "info", message: "当前素材是 HDR，将自动进行色调映射" });
  }

  const isLandscape = meta.width > meta.height;
  const hasVerticalPlatform = platforms.some(p => p === "douyin" || p === "xiaohongshu");
  if (isLandscape && hasVerticalPlatform) {
    warnings.push({ type: "warning", message: "横屏素材发抖音/小红书将转为竖屏（加黑边或裁剪）" });
  }

  // No audio track detection (simplified: check if fileType is video and duration > 0 but we can't detect no-audio from meta alone)
  if (meta.fileType === "video" && meta.durationSecs > 1800) {
    warnings.push({ type: "warning", message: "当前素材时长超过 30 分钟，预计处理时间较长" });
  }

  if (meta.fps >= 60 && platforms.length > 0) {
    const hasNon60 = platforms.some(p => p !== "bilibili");
    if (hasNon60) {
      warnings.push({ type: "info", message: "当前素材帧率 60fps，B站将保留 60fps，其他平台降为 30fps" });
    }
  }

  return warnings;
}

/** Build a copyright summary text for clipboard copy. */
export function buildCopyrightSummary(record: VaultRecord): string {
  const platforms: string[] = [];
  if (record.outputDouyin) platforms.push("抖音");
  if (record.outputBilibili) platforms.push("B站");
  if (record.outputXhs) platforms.push("小红书");

  return [
    `【隐盾版权存证】`,
    `版权编号: ${record.watermarkUid}`,
    `写入次数: 第 ${record.revision} 次`,
    record.parentWatermarkUid ? `上一版编号: ${record.parentWatermarkUid}` : "",
    record.rewriteReason ? `新版原因: ${record.rewriteReason}` : "",
    `原文件: ${record.fileName}`,
    `作品指纹: ${record.originalHash}`,
    `处理时间: ${record.createdAt}`,
    `输出平台: ${platforms.join("、") || "无"}`,
    `分辨率: ${record.resolution}`,
    `---`,
    `本存证由 HiddenShield 本地生成，数据未上传至任何服务器。`,
  ].join("\n");
}

/** Build a verification summary text for clipboard copy. */
export function buildVerificationSummary(result: VerificationResult, filePath: string): string {
  const status = result.matched
    ? "✅ 已命中"
    : result.confidence >= 0.95
      ? "⚠️ 检测到有效水印但未通过作品绑定"
      : result.confidence >= 0.5
        ? "⚠️ 疑似命中"
        : "❌ 未命中";

  const lines = [
    `═══════════════════════════════════════════`,
    `       隐盾 HiddenShield 数字版权存证报告`,
    `═══════════════════════════════════════════`,
    ``,
    `【检测结果】${status}`,
    `置信度: ${Math.round(result.confidence * 100)}%`,
    result.watermarkUid ? `版权编号: ${result.watermarkUid}` : "",
    result.reasonCode ? `判断依据: ${result.reasonCode}` : "",
    result.reasonDetail ? `判断说明: ${result.reasonDetail}` : "",
    ``,
    `───────────── 文件信息 ─────────────`,
    `检测文件: ${filePath.split(/[\\/]/).pop() ?? filePath}`,
    `文件路径: ${filePath}`,
    result.originalHash ? `原文件作品指纹: ${result.originalHash}` : "",
    `检测时间: ${new Date().toLocaleString()}`,
    ``,
  ];

  if (result.matched && result.matchedRecord) {
    const r = result.matchedRecord;
    lines.push(`───────────── 版权记录 ─────────────`);
    lines.push(`原始文件: ${r.fileName}`);
    lines.push(`写入次数: 第 ${r.revision} 次`);
    if (r.parentWatermarkUid) {
      lines.push(`上一版编号: ${r.parentWatermarkUid}`);
    }
    if (r.rewriteReason) {
      lines.push(`新版原因: ${r.rewriteReason}`);
    }
    lines.push(`入库时间: ${new Date(r.createdAt).toLocaleString()}`);
    if (r.resolution) {
      lines.push(`分辨率: ${r.resolution}`);
    }
    lines.push(`作品指纹: ${r.originalHash}`);
    lines.push(``);
  }

  if (result.tsaTokenPresent || result.networkTime) {
    lines.push(`───────────── 时间取证材料 ─────────────`);
    if (result.tsaTokenPresent && result.tsaSource) {
      lines.push(`可信时间回执: 已获取`);
      lines.push(`回执来源: ${result.tsaSource}`);
      lines.push(
        `状态: ${
          result.tsaTokenVerified
            ? getTsaVerificationLabel(result.tsaVerificationPath) ?? "已复验"
            : "已获取未复验"
        }`,
      );
    }
    if (result.networkTime) {
      lines.push(`网络授时 (GMT): ${result.networkTime}`);
      // Convert GMT to local time for readability
      const localTime = new Date(result.networkTime).toLocaleString();
      lines.push(`网络授时 (本地): ${localTime}`);
    }
    if (result.createdAt) {
      const localCreated = new Date(result.createdAt).toLocaleString();
      lines.push(`本地记录时间: ${localCreated}`);
    }
    if (!result.tsaTokenVerified) {
      lines.push(
        ``,
        `⚠️ 上述回执与网络授时仅作为补充取证材料，`,
        `   可信时间回执仍需完成独立验签后方可作为正式证明使用。`,
        ``,
      );
    } else {
      lines.push(
        ``,
        `可信时间回执：${getTsaVerificationLabel(result.tsaVerificationPath) ?? "已完成本地复验"}。`,
        ``,
      );
    }
  }

  lines.push(
    `───────────── 免责声明 ─────────────`,
    result.disclaimer,
    ``,
    `═══════════════════════════════════════════`,
    `本报告由 HiddenShield v1.0 本地生成`,
    `数据未上传至任何服务器`,
    `═══════════════════════════════════════════`,
  );

  return lines.filter(l => l !== undefined).join("\n");
}

export function getTsaVerificationLabel(path: TsaVerificationPath | null): string | null {
  if (path === "systemRoots") return "系统根已验证";
  if (path === "embeddedRoots") return "嵌入根已验证";
  return null;
}

// ---------------------------------------------------------------------------
// Telemetry & Data Management
// ---------------------------------------------------------------------------

export interface DataUsageInfo {
  ffmpegSizeMb: number;
  dbSizeMb: number;
  logSizeMb: number;
  totalSizeMb: number;
}

export interface UpdateInfo {
  available: boolean;
  version: string;
  body: string;
}

export async function getTelemetryEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_telemetry_enabled");
}

export async function setTelemetryEnabled(enabled: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("set_telemetry_enabled", { enabled });
}

export async function getTelemetryAcknowledged(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_telemetry_acknowledged");
}

export async function acknowledgeTelemetry(): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("acknowledge_telemetry");
}

export async function getNetworkEnabled(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_network_enabled");
}

export async function setNetworkEnabled(enabled: boolean): Promise<void> {
  if (!isTauriRuntime()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("set_network_enabled", { enabled });
}

export async function exportCrashLog(): Promise<string> {
  if (!isTauriRuntime()) return "(mock) no crash logs";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("export_crash_log");
}

export async function getDataUsage(): Promise<DataUsageInfo> {
  if (!isTauriRuntime()) {
    return { ffmpegSizeMb: 85.2, dbSizeMb: 2.4, logSizeMb: 0.1, totalSizeMb: 87.7 };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DataUsageInfo>("get_data_usage");
}

export async function clearAllData(): Promise<string> {
  if (!isTauriRuntime()) return "所有数据已清除，可安全卸载";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("clear_all_data");
}

export async function clearCacheOnly(): Promise<string> {
  if (!isTauriRuntime()) return "缓存已清除，版权库数据已保留";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("clear_cache_only");
}

// ---------------------------------------------------------------------------
// Auto-Updater
// ---------------------------------------------------------------------------

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  return null;
}

export async function installUpdate(
  onProgress?: (downloaded: number, total: number | null) => void,
): Promise<void> {
  const _ = onProgress;
  throw new Error("当前版本已禁用应用内自动更新，请使用受控发布包升级。");
}

// ---------------------------------------------------------------------------
// Identity (Creator Seed)
// ---------------------------------------------------------------------------

export interface IdentityStatus {
  initialized: boolean;
  watermarkUidPreview: string | null;
  deviceIdHex: string | null;
}

export async function getIdentityStatus(): Promise<IdentityStatus> {
  if (!isTauriRuntime()) {
    return { initialized: false, watermarkUidPreview: null, deviceIdHex: null };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<IdentityStatus>("get_identity_status");
}

export async function setupIdentity(creatorInput: string): Promise<IdentityStatus> {
  if (!isTauriRuntime()) {
    return { initialized: true, watermarkUidPreview: "HS-MOCK-MOCK-MOCK", deviceIdHex: "deadbeef" };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<IdentityStatus>("setup_identity", { creatorInput });
}
