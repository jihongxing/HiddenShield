<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import DropZone from "../components/DropZone.vue";
import ProgressPanel from "../components/ProgressPanel.vue";
import SystemStatus from "../components/SystemStatus.vue";
import ResultPage from "../components/ResultPage.vue";
import AIContentMarker from "../components/AIContentMarker.vue";
import { trackFeatureEvent } from "../lib/analytics";
import { userFacingErrorMessage } from "../lib/user-facing-errors";
import {
  cancelPipeline,
  checkActivePipelines,
  createL3VideoVisualUploadTask,
  createVideoFingerprintNotaryFromBundleFile,
  createEmptyPlatformPercents,
  generateVideoFingerprintBundle,
  getHardwareInfo,
  inspectRewriteTarget,
  listenHwDegradation,
  listenPipelineComplete,
  listenPipelineProgress,
  openOutputDir,
  probeSource,
  saveL3VideoVisualTaskToVault,
  startPipeline,
  systemCheck,
  isStandaloneAudioDurationUnknown as checkStandaloneAudioDurationUnknown,
  isStandaloneAudioTooShort as checkStandaloneAudioTooShort,
  standaloneAudioProtectionPreflight,
  type HardwareInfo,
  type PipelineCompletePayload,
  type PipelineProgressPayload,
  type Platform,
  type SourceMeta,
  type SystemCheckResult,
  type TranscodeOptions,
  type RewriteTargetInspectionResult,
  type EntitlementState,
  type VideoFingerprintBundleGeneration,
  type VideoFingerprintNotaryReceipt,
  type VaultRecord,
  type CreateL3VideoVisualUploadTaskResult,
  type SaveL3VideoVisualTaskResult,
} from "../lib/tauri-api";

const emit = defineEmits<{
  switchTab: [tab: "vault"];
  openSubscription: [];
}>();
const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const selectedPath = ref("");
const sourceMeta = ref<SourceMeta | null>(null);
const busy = ref(false);
const statusMessage = ref("就绪");
const hardwareInfo = ref<HardwareInfo | null>(null);
const systemStatus = ref<SystemCheckResult | null>(null);
const pipelineId = ref("");
const completePayload = ref<PipelineCompletePayload | null>(null);
const showResult = ref(false);
const degradationWarning = ref("");
const rewriteInspection = ref<RewriteTargetInspectionResult | null>(null);
const rewriteInspectionLoading = ref(false);
const rewriteInspectionError = ref("");
const filePickerError = ref("");
const videoNotaryBusy = ref(false);
const videoNotaryStatus = ref("");
const videoNotaryError = ref("");
const videoNotaryReceipt = ref<VideoFingerprintNotaryReceipt | null>(null);
const videoNotaryVaultRecord = ref<VaultRecord | null>(null);
const videoBundleBusy = ref(false);
const videoBundleStatus = ref("");
const videoBundleError = ref("");
const videoBundleResult = ref<VideoFingerprintBundleGeneration | null>(null);
const l3TaskIdInput = ref("");
const l3CreateBusy = ref(false);
const l3CreateStatus = ref("");
const l3CreateError = ref("");
const l3CreateResult = ref<CreateL3VideoVisualUploadTaskResult | null>(null);
const l3DownloadBusy = ref(false);
const l3DownloadStatus = ref("");
const l3DownloadError = ref("");
const l3DownloadResult = ref<SaveL3VideoVisualTaskResult | null>(null);
let rewriteInspectionRequestId = 0;

// AI Content Marker ref
const aiMarkerRef = ref<InstanceType<typeof AIContentMarker> | null>(null);

// For retry
const lastInputPath = ref("");
const lastOptions = reactive<TranscodeOptions>({
  aspectStrategy: "letterbox",
  encodingMode: "fast_gpu",
  allowRewrite: false,
  rewriteReason: "",
});

const progress = reactive<PipelineProgressPayload>({
  pipelineId: "",
  stage: "等待任务",
  percent: 0,
  platformPercents: createEmptyPlatformPercents(),
});

const options = reactive<TranscodeOptions>({
  aspectStrategy: "letterbox",
  encodingMode: "fast_gpu",
  allowRewrite: false,
  rewriteReason: "",
});

const outputSummary = computed(() => {
  if (!sourceMeta.value) return "等待导入";
  return `${sourceMeta.value.width}x${sourceMeta.value.height} / ${sourceMeta.value.fps}fps / ${sourceMeta.value.colorProfile}`;
});

const fileType = computed(() => sourceMeta.value?.fileType ?? "");
const isVideo = computed(() => fileType.value === "video");
const isImage = computed(() => fileType.value === "image");
const isAudio = computed(() => fileType.value === "audio");
const isStandaloneAudioTooShort = computed(() => checkStandaloneAudioTooShort(sourceMeta.value));
const isStandaloneAudioDurationUnknown = computed(() => checkStandaloneAudioDurationUnknown(sourceMeta.value));
const imagePixelCount = computed(
  () => (sourceMeta.value?.width ?? 0) * (sourceMeta.value?.height ?? 0),
);
const isImagePixelLimitExceeded = computed(
  () => isImage.value && imagePixelCount.value > 100_000_000,
);
const isImageFileSizeLimitExceeded = computed(
  () => isImage.value && (sourceMeta.value?.fileSizeMb ?? 0) > 512,
);
const isImageWatermarkCapacityInsufficient = computed(
  () => isImage.value &&
    sourceMeta.value?.watermarkEligible === false &&
    !isImagePixelLimitExceeded.value &&
    !isImageFileSizeLimitExceeded.value,
);
const isLargeImageResourceMode = computed(
  () => isImage.value &&
    imagePixelCount.value >= 50_000_000 &&
    imagePixelCount.value <= 100_000_000 &&
    !isImageFileSizeLimitExceeded.value,
);
const audioProtectionPreflight = computed(() => standaloneAudioProtectionPreflight(sourceMeta.value));
const isStandaloneAudioTooLong = computed(() => audioProtectionPreflight.value === "audio_too_long");
const isStandaloneAudioFileTooLarge = computed(
  () => audioProtectionPreflight.value === "audio_file_too_large",
);
const isStandaloneAudioSpecUnsupported = computed(
  () => ["audio_spec_unknown", "audio_sample_rate_too_low", "audio_sample_rate_too_high", "audio_channels_unsupported"]
    .includes(audioProtectionPreflight.value),
);
const canUseCloudVideo = computed(() => props.entitlementState?.features?.cloud_video_processing === true);
const canCreateVideoNotary = computed(() => props.entitlementState?.features?.cloud_sync === true);
const canUseControlledL3 = computed(() => false);
const canGenerateVideoBundle = computed(() => isVideo.value && !!sourceMeta.value && canCreateVideoNotary.value);
const currentPlanLabel = computed(() =>
  props.entitlementState?.features?.batch_processing === true ? "图片 / 音频年费" : "未付费",
);
const rewritePreflightBlocksStart = computed(
  () => (isImage.value || isAudio.value) && rewriteInspection.value?.hasWatermark === true && !options.allowRewrite,
);
const canStartCurrentTask = computed(
  () => !!sourceMeta.value &&
    (isImage.value || isAudio.value) &&
    !busy.value &&
    !isStandaloneAudioTooShort.value &&
    !isStandaloneAudioTooLong.value &&
    !isStandaloneAudioFileTooLarge.value &&
    !isStandaloneAudioDurationUnknown.value &&
    !isStandaloneAudioSpecUnsupported.value &&
    !isImagePixelLimitExceeded.value &&
    !isImageFileSizeLimitExceeded.value &&
    !isImageWatermarkCapacityInsufficient.value &&
    !rewriteInspectionLoading.value &&
    !rewritePreflightBlocksStart.value,
);
const fileTypeLabel = computed(() => {
  if (isImage.value) return "图片";
  if (isAudio.value) return "音频";
  return "图片 / 音频";
});
const processingModeLabel = computed(() => {
  if (!hardwareInfo.value) return "检测中";
  const encoder = hardwareInfo.value.preferredEncoder.toLowerCase();
  return encoder.includes("264") || encoder.includes("265") || encoder.includes("software")
    ? "标准处理"
    : "加速处理";
});
const processingReadyLabel = computed(() => {
  if (!hardwareInfo.value) return "检测中";
  return hardwareInfo.value.ffmpegStatus.toLowerCase().includes("missing")
    ? "需安装处理组件"
    : "可处理";
});

const rewriteInspectionTone = computed(() => {
  const result = rewriteInspection.value;
  if (!result) return "neutral";
  if (result.hasWatermark) return "warning";
  if (result.reasonCode === "preflight_extract_failed") return "danger";
  return "ok";
});

const rewriteInspectionSummaryLabel = computed(() => {
  const result = rewriteInspection.value;
  if (!result) return "";
  if (result.hasWatermark) return "已检测到已有版权记录";
  if (result.reasonCode === "no_valid_watermark") return "未检测到已有隐盾水印";
  return "版权记录检查完成";
});

const rewriteInspectionActionLabel = computed(() => {
  const result = rewriteInspection.value;
  if (!result) return "";
  if (result.hasWatermark) {
    return `继续写入将记录为第 ${result.nextRevision} 次写入`;
  }
  if (result.reasonCode === "no_valid_watermark") {
    return "将按首次写入处理";
  }
  return "写前预检已完成";
});

const rewriteInspectionEvidence = computed(() => {
  const result = rewriteInspection.value;
  if (!result) return [] as string[];
  const evidence: string[] = [];
  if (result.reasonDetail) evidence.push(result.reasonDetail);
  if (result.watermarkUid) evidence.push(`上一版编号：${result.watermarkUid}`);
  if (result.detectedRevision) evidence.push(`当前识别为第 ${result.detectedRevision} 次版本`);
  return evidence;
});

function currentFeatureName() {
  if (isImage.value) return "watermark_image";
  return "watermark_audio";
}

function currentMediaType() {
  if (isImage.value) return "image";
  return "audio";
}

async function refreshHardwareInfo() {
  hardwareInfo.value = await getHardwareInfo();
}

async function handleSourceSelect(path: string) {
  rewriteInspectionRequestId += 1;
  filePickerError.value = "";
  selectedPath.value = path;
  rewriteInspection.value = null;
  rewriteInspectionError.value = "";
  options.allowRewrite = false;
  options.rewriteReason = "";
  sourceMeta.value = await probeSource(path);
  if (sourceMeta.value.fileType === "video") {
    selectedPath.value = "";
    sourceMeta.value = null;
    systemStatus.value = null;
    filePickerError.value = "当前发布版本仅开放图片和音频，视频能力已暂停。";
    statusMessage.value = "视频能力已暂停";
    trackFeatureEvent("source_probe", "diagnostic", { mediaType: "video", source: "release_baseline" });
    return;
  }
  systemStatus.value = await systemCheck(path);
  trackFeatureEvent("source_probe", "success", { mediaType: currentMediaType(), source: "dropzone" });

  const type = sourceMeta.value.fileType;
  if (type === "image") {
    statusMessage.value = "图片已就绪";
  } else if (type === "audio") {
    if (isStandaloneAudioDurationUnknown.value) {
      statusMessage.value = "无法确认音频时长，请更换可识别时长的完整音频文件";
    } else if (isStandaloneAudioTooLong.value) {
      statusMessage.value = "当前桌面端支持最长 20 分钟音频，未生成保护副本";
    } else if (isStandaloneAudioFileTooLarge.value) {
      statusMessage.value = "当前桌面端支持不超过 512 MiB 的音频文件";
    } else if (audioProtectionPreflight.value === "audio_spec_unknown") {
      statusMessage.value = "无法确认音频采样率或声道，请更换可识别规格的完整音频文件";
    } else if (audioProtectionPreflight.value === "audio_sample_rate_too_low" || audioProtectionPreflight.value === "audio_sample_rate_too_high") {
      statusMessage.value = "当前仅支持 8–48 kHz 音频采样率，保护副本会保持原始规格不变";
    } else if (audioProtectionPreflight.value === "audio_channels_unsupported") {
      statusMessage.value = "当前仅支持 mono 或 stereo 音频，保护副本会保持原始规格不变";
    } else {
      statusMessage.value = isStandaloneAudioTooShort.value
        ? "音频时长不足 30 秒，请选择完整作品或更长片段"
        : "音频已就绪";
    }
  }

}

function handleSourceSelectError(message: string) {
  filePickerError.value = message;
  statusMessage.value = message;
}

async function refreshRewriteInspection(path: string, requestId = ++rewriteInspectionRequestId) {
  rewriteInspectionLoading.value = true;
  rewriteInspectionError.value = "";
  try {
    const result = await inspectRewriteTarget(path);
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspection.value = result;
      if (result.hasWatermark && !options.allowRewrite) {
        statusMessage.value = `检测到已有版权记录；如需生成新版，请开启“这是已有作品的新版”，本次将记录为第 ${result.nextRevision} 次写入`;
      }
    }
  } catch (err: any) {
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspection.value = null;
      console.warn("rewrite inspection failed", err);
      rewriteInspectionError.value = userFacingErrorMessage(err, "版权记录检查");
    }
  } finally {
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspectionLoading.value = false;
    }
  }
}

function setProgress(payload: Partial<PipelineProgressPayload>) {
  progress.pipelineId = payload.pipelineId ?? progress.pipelineId;
  progress.stage = payload.stage ?? progress.stage;
  progress.percent = payload.percent ?? progress.percent;
  progress.platformPercents = payload.platformPercents ?? progress.platformPercents;
}

function userFacingPipelineStage(stage: string) {
  const lower = stage.toLowerCase();
  if (stage.includes("audio_protection_duration_unknown")) {
    return "失败：无法确认音频时长，未生成保护副本。请更换可识别时长的完整音频文件后重试";
  }
  if (stage.includes("audio_protection_min_duration")) {
    return "失败：音频时长不足 30 秒，未生成保护副本。请选择 30 秒以上的完整音频作品后重试";
  }
  if (stage.includes("audio_protection_max_duration")) {
    return "失败：音频超过 20 分钟上限，未生成保护副本。请选择不超过 20 分钟的音频后重试";
  }
  if (stage.includes("audio_protection_file_size_limit_exceeded")) {
    return "失败：音频超过 512 MiB 上限，未生成保护副本。请选择不超过 512 MiB 的音频后重试";
  }
  if (lower.includes("[missing_creator_identity]")) {
    return "失败：请先完成创作者身份设置，再生成保护副本。";
  }
  if (lower.includes("[already_watermarked]") || lower.includes("watermark already exists in source media")) {
    return existingWatermarkBlockedMessage(stage);
  }
  if (lower.includes("[embed_failed]") || stage.includes("Watermark embedding failed")) {
    return "失败：保护副本未生成。请确认文件可读取后重试；如果持续失败，请复制诊断信息反馈。";
  }
  return stage;
}

function existingWatermarkBlockedMessage(source = "") {
  const uid =
    source.match(/HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}/)?.[0] ??
    rewriteInspection.value?.watermarkUid;
  return uid
    ? `失败：检测到已有版权记录 ${uid}。如需生成新版，请开启“这是已有作品的新版”。`
    : "失败：检测到已有版权记录。如需生成新版，请开启“这是已有作品的新版”。";
}

watch(
  () => options.allowRewrite,
  async (enabled) => {
    if (!enabled) {
      rewriteInspectionRequestId += 1;
      rewriteInspection.value = null;
      rewriteInspectionError.value = "";
      rewriteInspectionLoading.value = false;
      return;
    }
    if (!selectedPath.value || (!isImage.value && !isAudio.value)) return;
    await refreshRewriteInspection(selectedPath.value);
  },
);

async function ensureRewritePreflightBeforeStart(): Promise<boolean> {
  if (!options.allowRewrite) {
    return true;
  }
  if (rewriteInspectionLoading.value) {
    statusMessage.value = "正在识别上一版版权信息，请稍后再试";
    return false;
  }

  if (!selectedPath.value || isVideo.value || (!isImage.value && !isAudio.value)) {
    return true;
  }

  if (!rewriteInspection.value) {
    await refreshRewriteInspection(selectedPath.value);
  }

  if (rewriteInspectionError.value) {
    statusMessage.value = "无法识别上一版版权信息，请关闭新版写入或重新选择文件";
    return false;
  }

  return true;
}

async function confirmRewriteRisk() {
  if (!options.allowRewrite || isVideo.value) return true;

  const reason = options.rewriteReason?.trim() || "未填写，将使用默认新版原因";
  const detected = rewriteInspection.value?.hasWatermark ? rewriteInspection.value : null;
  const message = [
    detected
      ? `你正在为已有版权记录生成新版，本次会记录为第 ${detected.nextRevision} 次写入。`
      : "你正在允许为已有版权记录生成新版。",
    "",
    "这会生成一个新的保护副本，并在版权库中保留上一版编号、写入次数和新版原因。",
    "",
    detected?.watermarkUid ? `上一版编号：${detected.watermarkUid}` : "上一版编号：写入时再次检测，若存在则自动记录",
    `新版原因：${reason}`,
    "",
    "确认继续？",
  ].join("\n");

  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    return confirm(message, { title: "确认生成新版" });
  }

  return window.confirm(message);
}

async function handleStart() {
  if (!selectedPath.value) { statusMessage.value = "请先选择源文件"; return; }
  if (isAudio.value && systemStatus.value && !systemStatus.value.ffmpegAvailable) {
    statusMessage.value = "音频处理组件未就绪，请先安装后再处理音频";
    return;
  }
  if (isStandaloneAudioDurationUnknown.value) {
    statusMessage.value = "无法确认音频时长，未生成保护副本。请更换可识别时长的完整音频文件后重试";
    trackFeatureEvent("watermark_audio", "failure", {
      mediaType: "audio",
      errorCode: "audio_duration_unknown",
      source: "preflight",
    });
    return;
  }
  if (isStandaloneAudioTooShort.value) {
    statusMessage.value = "音频时长不足 30 秒，未生成保护副本。请选择 30 秒以上的完整音频作品后重试";
    trackFeatureEvent("watermark_audio", "failure", {
      mediaType: "audio",
      errorCode: "audio_duration_too_short",
      source: "preflight",
    });
    return;
  }
  if (isStandaloneAudioTooLong.value) {
    statusMessage.value = "音频超过 20 分钟上限，未生成保护副本。请选择不超过 20 分钟的音频后重试";
    trackFeatureEvent("watermark_audio", "failure", {
      mediaType: "audio",
      errorCode: "audio_duration_too_long",
      source: "preflight",
    });
    return;
  }
  if (isStandaloneAudioFileTooLarge.value) {
    statusMessage.value = "音频超过 512 MiB 上限，未生成保护副本。请选择不超过 512 MiB 的音频后重试";
    trackFeatureEvent("watermark_audio", "failure", {
      mediaType: "audio",
      errorCode: "audio_file_size_limit_exceeded",
      source: "preflight",
    });
    return;
  }
  if (isStandaloneAudioSpecUnsupported.value) {
    statusMessage.value = "当前音频规格不在支持范围内，未生成保护副本。请确认 8–48 kHz、mono 或 stereo，并保持原始规格后重试";
    trackFeatureEvent("watermark_audio", "failure", {
      mediaType: "audio",
      errorCode: audioProtectionPreflight.value,
      source: "preflight",
    });
    return;
  }
  if (isImagePixelLimitExceeded.value) {
    statusMessage.value = "图片超过 100 MP 上限，未生成保护副本。请选择像素不超过 100 MP 的静态图片后重试";
    trackFeatureEvent("watermark_image", "failure", {
      mediaType: "image",
      errorCode: "image_pixel_limit_exceeded",
      source: "preflight",
    });
    return;
  }
  if (isImageFileSizeLimitExceeded.value) {
    statusMessage.value = "图片超过 512 MiB 上限，未生成保护副本。请选择文件不超过 512 MiB 的静态图片后重试";
    trackFeatureEvent("watermark_image", "failure", {
      mediaType: "image",
      errorCode: "image_file_size_limit_exceeded",
      source: "preflight",
    });
    return;
  }
  if (isImageWatermarkCapacityInsufficient.value) {
    statusMessage.value = "当前图片可用水印容量不足，未生成保护副本。请选择像素更多或裁剪更少的原图后重试";
    trackFeatureEvent("watermark_image", "failure", {
      mediaType: "image",
      errorCode: "image_capacity_insufficient",
      source: "preflight",
    });
    return;
  }
  if (rewriteInspectionLoading.value) {
    statusMessage.value = "正在识别上一版版权信息，请稍后再试";
    return;
  }
  if (systemStatus.value && !systemStatus.value.outputDirWritable) {
    statusMessage.value = `目标输出目录不可写：${systemStatus.value.outputDir}`;
    return;
  }
  if (!(await ensureRewritePreflightBeforeStart())) {
    return;
  }
  if (!(await confirmRewriteRisk())) {
    statusMessage.value = "已取消重写";
    return;
  }

  busy.value = true;
  showResult.value = false;
  completePayload.value = null;
  const platforms = [] as Platform[];
  const featureName = currentFeatureName();
  trackFeatureEvent(featureName, "start", {
    mediaType: currentMediaType(),
    source: "single_media",
  });

  // Save for retry
  lastInputPath.value = selectedPath.value;
  lastOptions.aspectStrategy = options.aspectStrategy;
  lastOptions.encodingMode = options.encodingMode;
  lastOptions.allowRewrite = options.allowRewrite;
  lastOptions.rewriteReason = options.rewriteReason;

  // Collect AI content options from AIContentMarker
  const aiContent = aiMarkerRef.value ? {
    workSourceDeclaration: aiMarkerRef.value.workSourceDeclaration,
    trainingPermissionDeclaration: aiMarkerRef.value.trainingPermissionDeclaration,
    creationMethodDeclaration: aiMarkerRef.value.creationMethodDeclaration,
    humanEditLevelDeclaration: aiMarkerRef.value.humanEditLevelDeclaration,
    authenticityClaimDeclaration: aiMarkerRef.value.authenticityClaimDeclaration,
    customRightsStatement: aiMarkerRef.value.customRightsStatement.trim() || undefined,
  } : undefined;

  const rewriteReason = options.rewriteReason?.trim();
  const optionsWithAI = {
    ...options,
    rewriteReason: options.allowRewrite ? (rewriteReason || "用户确认重写已有水印") : undefined,
    aiContent,
  };

  try {
    const result = await startPipeline(selectedPath.value, platforms, optionsWithAI);
    pipelineId.value = result.pipelineId;
    statusMessage.value = result.summary;
    setProgress({ pipelineId: result.pipelineId, stage: "任务已排队", percent: 1, platformPercents: createEmptyPlatformPercents() });
  } catch (err: any) {
    busy.value = false;
    console.warn("pipeline start failed", err);
    statusMessage.value = userFacingErrorMessage(err, "启动写入任务");
    trackFeatureEvent(featureName, "failure", {
      mediaType: currentMediaType(),
      errorCode: "pipeline_start_failed",
    });
  }
}

async function handleRetry() {
  if (!lastInputPath.value) return;
  selectedPath.value = lastInputPath.value;
  options.aspectStrategy = lastOptions.aspectStrategy;
  options.encodingMode = lastOptions.encodingMode;
  options.allowRewrite = lastOptions.allowRewrite;
  options.rewriteReason = lastOptions.rewriteReason;
  await handleStart();
}

async function handleCancel() {
  if (!pipelineId.value) return;
  await cancelPipeline(pipelineId.value);
  trackFeatureEvent(currentFeatureName(), "cancel", { mediaType: currentMediaType(), source: "cancel_button" });
  busy.value = false;
  statusMessage.value = "已取消";
  setProgress({ stage: "已取消", percent: 0, platformPercents: createEmptyPlatformPercents() });
}

async function handleCreateVideoNotaryFromBundle() {
  videoNotaryError.value = "";
  videoNotaryReceipt.value = null;
  videoNotaryVaultRecord.value = null;
  if (!canCreateVideoNotary.value) {
    videoNotaryStatus.value = "当前发布版本不开放视频存证";
    return;
  }
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    videoNotaryStatus.value = "Web 预览模式已模拟提交";
    const result = await createVideoFingerprintNotaryFromBundleFile("mock/bundle.json", {
      title: sourceMeta.value?.fileName,
    });
    videoNotaryReceipt.value = result.receipt;
    videoNotaryVaultRecord.value = result.vaultRecord;
    return;
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    title: "选择视频指纹包",
    multiple: false,
    filters: [{ name: "VideoFingerprintBundle", extensions: ["json"] }],
  });
  if (!selected || Array.isArray(selected)) {
    videoNotaryStatus.value = "未选择指纹包";
    return;
  }
  if (!selected.endsWith("bundle.json")) {
    videoNotaryError.value = "请选择 video_fingerprint_spike 生成的 bundle.json";
    return;
  }

  await submitVideoFingerprintBundle(selected, "bundle_json");
}

async function handleGenerateVideoBundle() {
  videoBundleError.value = "";
  videoNotaryError.value = "";
  videoNotaryReceipt.value = null;
  videoNotaryVaultRecord.value = null;
  if (!selectedPath.value || !sourceMeta.value || !isVideo.value) {
    videoBundleStatus.value = "请先选择视频文件";
    return;
  }
  if (!canCreateVideoNotary.value) {
    videoBundleStatus.value = "当前发布版本不开放视频存证";
    return;
  }

  videoBundleBusy.value = true;
  videoBundleStatus.value = "正在生成本地不可逆指纹包";
  try {
    const result = await generateVideoFingerprintBundle(selectedPath.value);
    videoBundleResult.value = result;
    videoBundleStatus.value = "指纹包已生成，可确认提交云端存证";
    trackFeatureEvent("video_fingerprint_bundle", "success", {
      mediaType: "video",
      durationMs: result.elapsedMs,
      source: "desktop_local",
    });
  } catch (err: any) {
    console.warn("video fingerprint bundle failed", err);
    videoBundleError.value = userFacingErrorMessage(err, "生成指纹包");
    videoBundleStatus.value = "指纹包生成失败";
    trackFeatureEvent("video_fingerprint_bundle", "failure", {
      mediaType: "video",
      errorCode: "video_bundle_failed",
    });
  } finally {
    videoBundleBusy.value = false;
  }
}

async function handleSubmitGeneratedVideoBundle() {
  if (!videoBundleResult.value) {
    videoNotaryStatus.value = "请先生成指纹包";
    return;
  }
  await submitVideoFingerprintBundle(videoBundleResult.value.bundlePath, "generated_bundle");
}

async function submitVideoFingerprintBundle(bundlePath: string, source: string) {
  videoNotaryError.value = "";
  videoNotaryReceipt.value = null;
  if (!canCreateVideoNotary.value) {
    videoNotaryStatus.value = "当前发布版本不开放视频存证";
    return;
  }
  videoNotaryBusy.value = true;
  videoNotaryStatus.value = "正在提交视频指纹存证，最多等待约 10 秒返回结果";
  try {
    const result = await createVideoFingerprintNotaryFromBundleFile(bundlePath, {
      title: sourceMeta.value?.fileName,
      bundleElapsedMs: source === "generated_bundle" ? videoBundleResult.value?.elapsedMs : undefined,
    });
    videoNotaryReceipt.value = result.receipt;
    videoNotaryVaultRecord.value = result.vaultRecord;
    videoNotaryStatus.value = `云端存证已完成，存证编号 ${result.receipt.notaryId}，已保存到版权库`;
    trackFeatureEvent("video_fingerprint_notary", "success", {
      mediaType: "video",
      source,
    });
  } catch (err: any) {
    console.warn("video fingerprint notary failed", err);
    videoNotaryError.value = userFacingErrorMessage(err, "提交视频指纹存证");
    videoNotaryStatus.value = "视频指纹存证失败";
    trackFeatureEvent("video_fingerprint_notary", "failure", {
      mediaType: "video",
      errorCode: "video_notary_failed",
      source,
    });
  } finally {
    videoNotaryBusy.value = false;
  }
}

async function handleCreateL3UploadTask() {
  l3CreateError.value = "";
  l3CreateResult.value = null;
  if (!canUseControlledL3.value || !canUseCloudVideo.value) {
    l3CreateStatus.value = "当前发布版本不开放视频处理";
    return;
  }
  if (!sourceMeta.value || !selectedPath.value || !isVideo.value) {
    l3CreateStatus.value = "请先选择要上传的 MP4 视频";
    return;
  }
  if (!sourceMeta.value.fileName.toLowerCase().endsWith(".mp4")) {
    l3CreateStatus.value = "L3 正式创建上传入口当前只接收 MP4；其他容器待 worker 转码入口放开后再承诺";
    return;
  }
  if (sourceMeta.value.durationConfirmed === false || sourceMeta.value.durationSecs <= 0) {
    l3CreateStatus.value = "L3 创建任务需要可确认的视频时长，请先换用可探测的 MP4";
    return;
  }

  l3CreateBusy.value = true;
  l3CreateStatus.value = "步骤 1/4 准备上传并校验权益";
  try {
    const frameCount = Math.max(1, Math.round(sourceMeta.value.durationSecs * Math.max(1, sourceMeta.value.fps || 1)));
    l3CreateStatus.value = "步骤 2/4 上传受控对象并回读哈希";
    const result = await createL3VideoVisualUploadTask(selectedPath.value, {
      title: sourceMeta.value.fileName,
      durationSecs: sourceMeta.value.durationSecs,
      width: sourceMeta.value.width,
      height: sourceMeta.value.height,
      frameCount,
    });
    l3CreateResult.value = result;
    l3TaskIdInput.value = result.task.taskId;
    l3CreateStatus.value = `步骤 4/4 已创建 L3 任务 ${result.task.taskId}，等待 trusted worker 完成后再领取入库`;
    trackFeatureEvent("l3_video_visual_upload_wizard", "success", {
      mediaType: "video",
      source: "desktop_l3_formal_upload",
    });
  } catch (err: any) {
    console.warn("L3 video visual upload task creation failed", err);
    l3CreateError.value = userFacingErrorMessage(err, "创建 L3 上传任务");
    l3CreateStatus.value = "L3 创建上传任务失败";
    trackFeatureEvent("l3_video_visual_upload_wizard", "failure", {
      mediaType: "video",
      errorCode: "l3_upload_task_create_failed",
    });
  } finally {
    l3CreateBusy.value = false;
  }
}

async function handleSaveL3TaskToVault() {
  l3DownloadError.value = "";
  l3DownloadResult.value = null;
  const taskId = l3TaskIdInput.value.trim();
  if (!canUseControlledL3.value || !canUseCloudVideo.value) {
    l3DownloadStatus.value = "当前发布版本不开放视频处理";
    return;
  }
  if (!taskId) {
    l3DownloadStatus.value = "请输入已 succeeded 的 L3 taskId";
    return;
  }

  l3DownloadBusy.value = true;
  l3DownloadStatus.value = "正在领取 L3 MP4 成品并复核哈希";
  try {
    const result = await saveL3VideoVisualTaskToVault(taskId, {
      title: sourceMeta.value?.fileName || "L3 视频画面盲水印成品",
    });
    l3DownloadResult.value = result;
    l3DownloadStatus.value = `L3 成品已下载并保存到版权库 #${result.vaultRecord.id}`;
    trackFeatureEvent("l3_video_visual_product_flow", "success", {
      mediaType: "video",
      source: "desktop_succeeded_task",
    });
  } catch (err: any) {
    console.warn("L3 video visual task save failed", err);
    l3DownloadError.value = userFacingErrorMessage(err, "领取 L3 视频成品");
    l3DownloadStatus.value = "L3 成品领取失败";
    trackFeatureEvent("l3_video_visual_product_flow", "failure", {
      mediaType: "video",
      errorCode: "l3_task_download_or_vault_failed",
    });
  } finally {
    l3DownloadBusy.value = false;
  }
}

async function handleOpenVideoBundleDir() {
  if (!videoBundleResult.value) return;
  const dirPath = videoBundleResult.value.bundlePath.replace(/[\\/][^\\/]*$/, "");
  await openOutputDir(dirPath);
}

function handleBackFromResult() {
  showResult.value = false;
  completePayload.value = null;
  selectedPath.value = "";
  sourceMeta.value = null;
  statusMessage.value = "就绪";
  void systemCheck().then((result) => { systemStatus.value = result; });
}

let unlistenProgress: (() => void) | null = null;
let unlistenComplete: (() => void) | null = null;
let unlistenDegradation: (() => void) | null = null;

// Focus-based state reconciliation: when user returns to the window,
// check if pipelines completed while the WebView was suspended.
async function handleWindowFocus() {
  if (!busy.value || !pipelineId.value) return;
  const activePipelines = await checkActivePipelines();
  if (!activePipelines.includes(pipelineId.value)) {
    // Pipeline finished while we were away — sync state
    busy.value = false;
    if (progress.percent < 100 && !progress.stage.startsWith("失败")) {
      statusMessage.value = "已完成";
      setProgress({ stage: "完成", percent: 100, platformPercents: createEmptyPlatformPercents() });
    }
  }
}

// Close protection
function handleBeforeUnload(e: BeforeUnloadEvent) {
  if (busy.value) {
    e.preventDefault();
    e.returnValue = "当前有任务正在处理，关闭将中断任务";
  }
}

onMounted(async () => {
  await refreshHardwareInfo();
  systemStatus.value = await systemCheck();

  unlistenProgress = await listenPipelineProgress((payload) => {
    const nextPayload = {
      ...payload,
      stage: userFacingPipelineStage(payload.stage),
    };
    setProgress(nextPayload);
    if (payload.percent >= 100) {
      busy.value = false;
      statusMessage.value = "完成";
    } else if (nextPayload.stage.startsWith("失败")) {
      busy.value = false;
      statusMessage.value = nextPayload.stage;
      trackFeatureEvent(currentFeatureName(), "failure", {
        mediaType: currentMediaType(),
        errorCode: "pipeline_runtime_failed",
        source: nextPayload.stage,
      });
    }
  });

  unlistenComplete = await listenPipelineComplete((payload) => {
    completePayload.value = payload;
    showResult.value = true;
    busy.value = false;
    trackFeatureEvent(currentFeatureName(), "success", {
      mediaType: currentMediaType(),
      durationMs: payload.processTimeMs,
      source: "pipeline_complete",
    });
  });

  unlistenDegradation = await listenHwDegradation((payload) => {
    degradationWarning.value = payload.message;
    // Auto-dismiss after 10 seconds
    setTimeout(() => { degradationWarning.value = ""; }, 10000);
  });

  // Focus-based state sync
  window.addEventListener("focus", handleWindowFocus);

  // Close protection (browser mode)
  window.addEventListener("beforeunload", handleBeforeUnload);

  // Tauri close protection
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      getCurrentWindow().onCloseRequested(async (event) => {
        if (busy.value) {
          event.preventDefault();
          const { confirm } = await import("@tauri-apps/plugin-dialog");
          const confirmed = await confirm("当前有任务正在处理，关闭将中断任务。确定关闭？", { title: "关闭确认" });
          if (confirmed) {
            const { getCurrentWindow: getWin } = await import("@tauri-apps/api/window");
            await getWin().destroy();
          }
        }
      });
    });
  }
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenComplete?.();
  unlistenDegradation?.();
  window.removeEventListener("focus", handleWindowFocus);
  window.removeEventListener("beforeunload", handleBeforeUnload);
});
</script>

<template>
  <div class="view-shell">
    <!-- Result Page (shown after pipeline-complete) -->
    <ResultPage
      v-if="showResult && completePayload && sourceMeta"
      :payload="completePayload"
      :source-meta="sourceMeta"
      @back="handleBackFromResult"
      @open-vault="emit('switchTab', 'vault')"
    />

    <!-- Normal Workbench -->
    <template v-else>
      <section class="hero-card">
        <div>
          <p class="eyebrow">工作台</p>
          <h2>处理作品</h2>
          <p class="hero-card__copy">选择图片或音频，生成可离线读取和验证的保护副本与版权记录。</p>
        </div>
        <div class="hero-card__stats">
          <div>
            <span>当前权益</span>
            <strong>{{ currentPlanLabel }}</strong>
          </div>
          <div>
            <span>处理方式</span>
            <strong>{{ processingModeLabel }}</strong>
          </div>
          <div>
            <span>状态</span>
            <strong>{{ processingReadyLabel }}</strong>
          </div>
        </div>
      </section>

      <!-- System Status -->
      <SystemStatus v-if="systemStatus" :result="systemStatus" />

      <section class="workbench-grid">
        <div class="panel">
          <div class="panel__header">
            <div>
              <h3>导入</h3>
            </div>
            <span class="pill">{{ fileTypeLabel }}</span>
          </div>

          <DropZone
            :selected-path="selectedPath"
            :source-name="sourceMeta?.fileName ?? ''"
            :disabled="busy"
            @select="handleSourceSelect"
            @error="handleSourceSelectError"
          />
          <p v-if="filePickerError" class="workbench-inline-error">{{ filePickerError }}</p>

          <div v-if="false && isVideo" class="rewrite-panel">
            <div class="rewrite-panel__status rewrite-panel__status--ok">
              <strong>L1 视频音轨水印</strong>
              <span>处理页不做平台画幅适配、裁剪或多平台分发；仅生成可验证的保护副本，并采用最小必要变更策略。</span>
            </div>
          </div>

          <div v-if="isImage || isAudio" class="rewrite-panel">
            <div class="rewrite-panel__status rewrite-panel__status--ok">
              <strong>{{ isImage ? '图片写入' : '音频写入' }}</strong>
              <span>
                {{ isImage
                  ? '将生成 PNG 保护副本，完成前会验证版权编号。'
                  : '支持 30 秒–20 分钟、文件不超过 512 MiB 的音频作品。将生成 WAV 保护副本，完成前会验证版权编号。' }}
              </span>
            </div>
            <div v-if="isStandaloneAudioTooShort" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>音频时长不足</strong>
              <span>当前音频短于 30 秒，暂不生成保护副本。请选择完整作品。</span>
            </div>
            <div v-if="isStandaloneAudioDurationUnknown" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>无法确认音频时长</strong>
              <span>暂不生成保护副本。请更换可识别时长的完整音频文件。</span>
            </div>
            <div v-if="isStandaloneAudioTooLong" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>音频时长超过上限</strong>
              <span>当前桌面端支持最长 20 分钟音频，暂不生成保护副本。</span>
            </div>
            <div v-if="isStandaloneAudioFileTooLarge" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>音频文件超过上限</strong>
              <span>当前桌面端支持不超过 512 MiB 的音频文件。</span>
            </div>
            <div v-if="audioProtectionPreflight === 'audio_spec_unknown'" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>无法确认音频规格</strong>
              <span>暂不生成保护副本。请选择可识别采样率和声道的完整音频文件。</span>
            </div>
            <div v-if="audioProtectionPreflight === 'audio_sample_rate_too_low' || audioProtectionPreflight === 'audio_sample_rate_too_high'" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>音频采样率暂不支持</strong>
              <span>当前支持 8–48 kHz，保护副本会保持原始采样率不变。</span>
            </div>
            <div v-if="audioProtectionPreflight === 'audio_channels_unsupported'" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>音频声道暂不支持</strong>
              <span>当前支持 mono 或 stereo，保护副本会保持原始声道不变。</span>
            </div>
            <div v-if="isImageWatermarkCapacityInsufficient" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>图片可用水印容量不足</strong>
              <span>当前图片无法容纳完整水印。请选择像素更多或裁剪更少的原图。</span>
            </div>
            <div v-if="isImagePixelLimitExceeded" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>图片像素超过上限</strong>
              <span>当前桌面端支持不超过 100 MP 的静态图片，不会通过缩小尺寸规避边界。</span>
            </div>
            <div v-if="isImageFileSizeLimitExceeded" class="rewrite-panel__status rewrite-panel__status--danger">
              <strong>图片文件超过上限</strong>
              <span>当前桌面端支持不超过 512 MiB 的静态图片。</span>
            </div>
            <div v-if="isLargeImageResourceMode" class="rewrite-panel__status">
              <strong>高资源大图模式</strong>
              <span>50–100 MP 图片可能需要十几分钟并占用约 7 GiB 内存。建议使用至少 16 GiB 内存的电脑，并保持应用前台运行。</span>
            </div>
            <label class="rewrite-panel__toggle">
              <input v-model="options.allowRewrite" type="checkbox" :disabled="busy" />
              <span>这是已有作品的新版</span>
            </label>
            <p v-if="!options.allowRewrite" class="rewrite-panel__hint">
              普通作品无需提前检查；正式写入时仍会阻止覆盖已有水印。
            </p>
            <div v-if="options.allowRewrite && rewriteInspectionLoading" class="rewrite-panel__status">
              正在识别上一版版权信息...
            </div>
            <details
              v-else-if="options.allowRewrite && rewriteInspection"
              open
              class="rewrite-panel__details"
              :class="`rewrite-panel__details--${rewriteInspectionTone}`"
            >
              <summary class="rewrite-panel__status rewrite-panel__status--compact">
                <strong>{{ rewriteInspectionSummaryLabel }}</strong>
                <span>{{ rewriteInspectionActionLabel }}</span>
              </summary>
              <div class="rewrite-panel__details-body">
                <span v-for="line in rewriteInspectionEvidence" :key="line">{{ line }}</span>
              </div>
            </details>
            <div
              v-else-if="options.allowRewrite && rewriteInspectionError"
              class="rewrite-panel__status rewrite-panel__status--danger"
            >
              上一版版权信息识别失败：{{ rewriteInspectionError }}
            </div>
            <input
              v-if="options.allowRewrite"
              v-model="options.rewriteReason"
              class="rewrite-panel__input"
              type="text"
              :disabled="busy"
              placeholder="新版原因，例如：修正版、授权派生、重新导出"
            />
          </div>

          <!-- AI Content Marker -->
          <AIContentMarker ref="aiMarkerRef" />

          <div v-if="false && isVideo" class="video-track-card">
            <span class="video-track-card__label">L1 本地写入</span>
            <strong>视频音轨水印</strong>
            <p>
              当前发布版本不开放视频处理；此内部兼容区块不会进入正式桌面入口。
            </p>
          </div>

          <div v-if="false && isVideo" class="cloud-video-card">
            <div>
              <span class="cloud-video-card__label">L2 存证</span>
              <strong>视频指纹存证</strong>
              <p>
                {{ canCreateVideoNotary
                  ? "先在本机生成不可逆指纹包，再提交摘要和存证收据。不会上传原始视频或本地路径。"
                  : "当前发布版本不开放视频指纹包或存证提交。"
                }}
              </p>
              <p v-if="!canUseCloudVideo" class="cloud-video-card__note">全部视频能力均已退出当前发布范围。</p>
              <p v-if="!canCreateVideoNotary" class="cloud-video-card__status">L1 视频音轨写入仍可直接使用，当前只是不开放 L2 提交。</p>
              <div v-if="videoBundleResult" class="cloud-video-card__bundle">
                <span>指纹包</span>
                <strong class="hash-text">{{ videoBundleResult?.bundlePath }}</strong>
                <span>包摘要</span>
                <strong class="hash-text">{{ videoBundleResult?.bundleSha256 }}</strong>
                <span>采样帧</span>
                <strong>{{ videoBundleResult?.sceneCount }} 帧 / {{ Math.round((videoBundleResult?.elapsedMs ?? 0) / 100) / 10 }}s</strong>
              </div>
              <p v-else-if="videoBundleStatus" class="cloud-video-card__status">{{ videoBundleStatus }}</p>
              <p v-if="videoBundleError" class="cloud-video-card__error">{{ videoBundleError }}</p>
              <div v-if="videoNotaryReceipt" class="cloud-video-card__receipt">
                <span>存证编号</span>
                <strong>{{ videoNotaryReceipt?.notaryId }}</strong>
                <span>指纹根</span>
                <strong class="hash-text">{{ videoNotaryReceipt?.fingerprintRoot }}</strong>
                <span>版权库</span>
                <strong>{{ videoNotaryVaultRecord ? `已保存 #${videoNotaryVaultRecord?.id}` : "已保存" }}</strong>
              </div>
              <p v-else-if="videoNotaryStatus" class="cloud-video-card__status">{{ videoNotaryStatus }}</p>
              <p v-if="videoNotaryError" class="cloud-video-card__error">{{ videoNotaryError }}</p>
            </div>
            <div class="cloud-video-card__actions">
              <span class="pill">{{ canCreateVideoNotary ? "可存证" : "未开放" }}</span>
              <button
                v-if="!canCreateVideoNotary"
                class="ghost-button"
                type="button"
                @click="emit('openSubscription')"
              >
                当前不可用
              </button>
              <button
                class="primary-button"
                type="button"
                :disabled="videoBundleBusy || videoNotaryBusy || !canGenerateVideoBundle"
                @click="handleGenerateVideoBundle"
              >
                {{ videoBundleBusy ? "生成中" : "生成指纹包" }}
              </button>
              <button
                class="ghost-button"
                type="button"
                :disabled="videoNotaryBusy || !canCreateVideoNotary || !videoBundleResult"
                @click="handleSubmitGeneratedVideoBundle"
              >
                {{ videoNotaryBusy ? "提交中" : "提交存证" }}
              </button>
              <button
                v-if="videoBundleResult"
                class="ghost-button"
                type="button"
                :disabled="videoBundleBusy"
                @click="handleOpenVideoBundleDir"
              >
                打开位置
              </button>
              <button
                class="ghost-button"
                type="button"
                :disabled="videoNotaryBusy || !canCreateVideoNotary"
                @click="handleCreateVideoNotaryFromBundle"
              >
                {{ videoNotaryBusy ? "提交中" : "选择指纹包" }}
              </button>
            </div>
          </div>

          <div v-if="false && isVideo" class="cloud-video-card cloud-video-card--l3">
            <div>
              <span class="cloud-video-card__label">L3 对象上传入口</span>
              <strong>视频画面盲水印 release gate</strong>
              <p>
                {{ canUseControlledL3
                  ? "该内部候选流程已冻结，不属于当前桌面发布能力。"
                  : "该内部候选流程已冻结，不开放创建上传与领取入口。"
                }}
              </p>
              <p class="cloud-video-card__note">创建向导：容量预检、准备上传、上传受控对象、创建云端 L3 任务、等待 trusted worker；创建成功不代表写入成功，已 succeeded 的 L3 对象任务才可领取入库，video_minutes 只能在真实自检 succeeded 并固化收据后扣费，同步和报告只保存收据元数据。</p>
              <div class="cloud-video-card__receipt">
                <span>隐私边界</span>
                <strong>signed_object_upload_only_no_local_path_no_raw_video_sync</strong>
                <span>失败归因</span>
                <strong>权益 / 登录 / MP4 类型 / 时长 / 上传授权 / 哈希回读 / 任务创建 / strategy_invalid 容量不足 / self_check_failed / worker_receipt_invalid</strong>
              </div>
              <div v-if="l3CreateResult" class="cloud-video-card__receipt">
                <span>任务</span>
                <strong>{{ `${l3CreateResult?.task.taskId} (${l3CreateResult?.task.status})` }}</strong>
                <span>预留 UID</span>
                <strong>{{ l3CreateResult?.watermarkUid }}</strong>
                <span>上传摘要</span>
                <strong class="hash-text">{{ `${l3CreateResult?.sourceSha256} / ${l3CreateResult?.uploadedBytes} bytes` }}</strong>
              </div>
              <p v-if="l3CreateStatus" class="cloud-video-card__status">{{ l3CreateStatus }}</p>
              <p v-if="l3CreateError" class="cloud-video-card__error">{{ l3CreateError }}</p>
              <input
                v-model="l3TaskIdInput"
                class="rewrite-panel__input"
                type="text"
                :disabled="l3DownloadBusy || !canUseControlledL3"
                placeholder="trusted worker succeeded 后输入或使用上方 taskId"
              />
              <div v-if="l3DownloadResult" class="cloud-video-card__receipt">
                <span>版权库</span>
                <strong>{{ `已保存 #${l3DownloadResult?.vaultRecord.id}` }}</strong>
                <span>成品摘要</span>
                <strong class="hash-text">{{ l3DownloadResult?.outputSha256 }}</strong>
                <span>自检</span>
                <strong>{{ l3DownloadResult?.task.selfCheckConfidence ?? "未记录" }} / {{ l3DownloadResult?.task.selfCheckThreshold ?? "未记录" }}</strong>
              </div>
              <p v-else-if="l3DownloadStatus" class="cloud-video-card__status">{{ l3DownloadStatus }}</p>
              <p v-if="l3DownloadError" class="cloud-video-card__error">{{ l3DownloadError }}</p>
            </div>
            <div class="cloud-video-card__actions">
              <span class="pill">{{ canUseControlledL3 ? "可创建 / 可领取" : "内部冻结" }}</span>
              <button
                class="primary-button"
                type="button"
                :disabled="l3CreateBusy || !canUseControlledL3 || !canUseCloudVideo || !sourceMeta || !isVideo"
                @click="handleCreateL3UploadTask"
              >
                {{ l3CreateBusy ? "创建中" : "创建并上传 L3 任务" }}
              </button>
              <button
                class="primary-button"
                type="button"
                :disabled="l3DownloadBusy || !canUseControlledL3 || !canUseCloudVideo || !l3TaskIdInput.trim()"
                @click="handleSaveL3TaskToVault"
              >
                {{ l3DownloadBusy ? "领取中" : "下载并保存版权库" }}
              </button>
              <button
                v-if="l3DownloadResult"
                class="ghost-button"
                type="button"
                @click="emit('switchTab', 'vault')"
              >
                查看版权库
              </button>
            </div>
          </div>

          <div class="action-row">
            <button
              class="primary-button"
              type="button"
              :disabled="!canStartCurrentTask"
              @click="handleStart"
            >
              生成保护副本
            </button>
            <button class="ghost-button" type="button" :disabled="!busy" @click="handleCancel">
              取消任务
            </button>
          </div>
        </div>

        <div class="panel">
          <div class="panel__header">
            <div>
              <h3>素材</h3>
              <p>{{ outputSummary }}</p>
            </div>
            <span class="pill">{{ sourceMeta ? (sourceMeta.isHdr ? "HDR" : "SDR") : "待检查" }}</span>
          </div>

          <div class="meta-grid">
            <div class="meta-card">
              <span>文件名</span>
              <strong>{{ sourceMeta?.fileName ?? "未选择文件" }}</strong>
            </div>
            <div class="meta-card">
              <span>类型</span>
              <strong>{{ sourceMeta ? fileTypeLabel : "--" }}</strong>
            </div>
            <div v-if="isAudio" class="meta-card">
              <span>时长</span>
              <strong>{{ sourceMeta ? (sourceMeta.durationConfirmed === false ? "无法确认" : `${sourceMeta.durationSecs}s`) : "--" }}</strong>
            </div>
            <div v-if="false && isVideo" class="meta-card">
              <span>分辨率</span>
              <strong>{{ sourceMeta ? `${sourceMeta?.width}x${sourceMeta?.height}` : "--" }}</strong>
            </div>
            <div class="meta-card">
              <span>大小</span>
              <strong>{{ sourceMeta ? `${sourceMeta.fileSizeMb} MB` : "--" }}</strong>
            </div>
            <div class="meta-card">
              <span>作品指纹</span>
              <strong class="hash-text">{{ sourceMeta?.sha256 ?? "待计算" }}</strong>
            </div>
          </div>
        </div>
      </section>

      <!-- Hardware degradation warning toast -->
      <div v-if="degradationWarning" class="degradation-toast" role="alert">
        {{ degradationWarning }}
      </div>

      <ProgressPanel
        :busy="busy"
        :summary="statusMessage"
        :progress="progress"
        @retry="handleRetry"
      />
    </template>
  </div>
</template>

<style scoped>
.rewrite-panel {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border: 1px solid rgba(255, 200, 87, 0.28);
  border-radius: var(--hs-radius-card);
  background: var(--hs-warning-surface);
}

.rewrite-panel__toggle {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
  color: var(--text-primary);
}

.rewrite-panel__toggle input {
  accent-color: var(--hs-accent);
}

.rewrite-panel__input {
  width: 100%;
  margin-top: 0.75rem;
  padding: 0.65rem 0.8rem;
  border-radius: 8px;
  border: 1px solid var(--hs-border);
  background: var(--hs-surface-muted);
  color: var(--hs-text);
}

.rewrite-panel__status {
  display: grid;
  gap: 0.25rem;
  margin-top: 0.75rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid var(--hs-border);
  border-radius: 8px;
  background: var(--hs-surface-raised);
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.45;
}

.rewrite-panel__status strong {
  color: var(--hs-text);
  font-weight: 700;
}

.rewrite-panel__status--compact {
  margin-top: 0;
  list-style: none;
  cursor: pointer;
}

.rewrite-panel__status--compact::-webkit-details-marker {
  display: none;
}

.rewrite-panel__details {
  margin-top: 0.75rem;
  border: 1px solid var(--hs-border);
  border-radius: 8px;
  background: var(--hs-surface-raised);
}

.rewrite-panel__details--ok {
  border-color: rgba(89, 210, 194, 0.28);
  background: rgba(89, 210, 194, 0.08);
}

.rewrite-panel__details--warning {
  border-color: rgba(245, 177, 66, 0.36);
  background: rgba(245, 177, 66, 0.1);
}

.rewrite-panel__details--danger {
  border-color: rgba(232, 93, 93, 0.34);
  background: rgba(232, 93, 93, 0.1);
}

.rewrite-panel__details .rewrite-panel__status {
  border: 0;
  background: transparent;
}

.rewrite-panel__details[open] .rewrite-panel__status {
  border-bottom: 1px solid var(--hs-border);
  border-radius: 8px 8px 0 0;
}

.rewrite-panel__details-body {
  display: grid;
  gap: 0.28rem;
  padding: 0.55rem 0.8rem 0.75rem;
  color: var(--hs-text-muted);
  font-size: 0.82rem;
  line-height: 1.45;
}

.rewrite-panel__status--ok {
  border-color: rgba(89, 210, 194, 0.28);
  background: rgba(89, 210, 194, 0.08);
}

.rewrite-panel__status--warning {
  border-color: rgba(245, 177, 66, 0.36);
  background: rgba(245, 177, 66, 0.1);
}

.rewrite-panel__status--danger {
  border-color: rgba(232, 93, 93, 0.34);
  background: rgba(232, 93, 93, 0.1);
}

.video-track-card {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border: 1px solid rgba(87, 143, 202, 0.22);
  border-radius: var(--hs-radius-card);
  background: rgba(87, 143, 202, 0.06);
}

.video-track-card strong {
  display: block;
  margin-top: 0.2rem;
  color: var(--hs-text);
}

.video-track-card p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.5;
}

.video-track-card__label {
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.cloud-video-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.8rem;
  align-items: start;
  margin-top: 0.9rem;
  padding: 0.85rem;
  border: 1px solid rgba(87, 143, 202, 0.24);
  border-radius: var(--hs-radius-card);
  background: rgba(87, 143, 202, 0.08);
}

.cloud-video-card strong {
  display: block;
  margin-top: 0.2rem;
  color: var(--hs-text);
}

.cloud-video-card p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.5;
}

.cloud-video-card__label {
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.cloud-video-card__actions {
  display: grid;
  gap: 0.55rem;
  justify-items: end;
}

.cloud-video-card__actions .ghost-button {
  min-width: 7.5rem;
  padding: 0.58rem 0.75rem;
}

.cloud-video-card__actions .primary-button {
  min-width: 7.5rem;
  padding: 0.58rem 0.75rem;
}

.cloud-video-card__note,
.cloud-video-card__status {
  color: var(--hs-text-muted);
}

.cloud-video-card__error {
  color: var(--hs-danger);
  font-weight: 650;
}

.cloud-video-card__bundle,
.cloud-video-card__receipt {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: 0.35rem 0.65rem;
  margin-top: 0.7rem;
  padding: 0.65rem 0.7rem;
  border-radius: 8px;
  background: var(--hs-surface-muted);
  border: 1px solid var(--hs-border);
  font-size: 0.8rem;
}

.cloud-video-card__bundle span,
.cloud-video-card__receipt span {
  color: var(--hs-text-muted);
}

.cloud-video-card__bundle strong,
.cloud-video-card__receipt strong {
  min-width: 0;
  margin: 0;
}

@media (max-width: 720px) {
  .cloud-video-card {
    grid-template-columns: 1fr;
  }

  .cloud-video-card__actions {
    justify-items: stretch;
  }
}
</style>
