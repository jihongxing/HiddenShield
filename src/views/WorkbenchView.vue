<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import DropZone from "../components/DropZone.vue";
import PlatformSelector from "../components/PlatformSelector.vue";
import ProgressPanel from "../components/ProgressPanel.vue";
import SystemStatus from "../components/SystemStatus.vue";
import SourceWarnings from "../components/SourceWarnings.vue";
import ResultPage from "../components/ResultPage.vue";
import ProBadge from "../components/ProBadge.vue";
import AIContentMarker from "../components/AIContentMarker.vue";
import { trackFeatureEvent } from "../lib/analytics";
import {
  cancelPipeline,
  checkActivePipelines,
  createEmptyPlatformPercents,
  generateWarnings,
  getHardwareInfo,
  inspectRewriteTarget,
  listenHwDegradation,
  listenPipelineComplete,
  listenPipelineProgress,
  probeSource,
  recommendPlatforms,
  recommendStrategy,
  startPipeline,
  systemCheck,
  type HardwareInfo,
  type PipelineCompletePayload,
  type PipelineProgressPayload,
  type Platform,
  type SourceMeta,
  type SourceWarning,
  type SystemCheckResult,
  type TranscodeOptions,
  type RewriteTargetInspectionResult,
} from "../lib/tauri-api";

const selectedPath = ref("");
const sourceMeta = ref<SourceMeta | null>(null);
const busy = ref(false);
const statusMessage = ref("就绪");
const hardwareInfo = ref<HardwareInfo | null>(null);
const systemStatus = ref<SystemCheckResult | null>(null);
const pipelineId = ref("");
const warnings = ref<SourceWarning[]>([]);
const userOverridden = ref(false);
const showRecommendHint = ref(false);
const completePayload = ref<PipelineCompletePayload | null>(null);
const showResult = ref(false);
const degradationWarning = ref("");
const rewriteInspection = ref<RewriteTargetInspectionResult | null>(null);
const rewriteInspectionLoading = ref(false);
const rewriteInspectionError = ref("");
let rewriteInspectionRequestId = 0;

// AI Content Marker ref
const aiMarkerRef = ref<InstanceType<typeof AIContentMarker> | null>(null);

// For retry
const lastInputPath = ref("");
const lastPlatforms = ref<Platform[]>([]);
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

const selectedPlatforms = ref<Platform[]>(["douyin"]);

const outputSummary = computed(() => {
  if (!sourceMeta.value) return "等待导入";
  return `${sourceMeta.value.width}x${sourceMeta.value.height} / ${sourceMeta.value.fps}fps / ${sourceMeta.value.colorProfile}`;
});

const fileType = computed(() => sourceMeta.value?.fileType ?? "video");
const isVideo = computed(() => fileType.value === "video");
const isImage = computed(() => fileType.value === "image");
const isAudio = computed(() => fileType.value === "audio");
const fileTypeLabel = computed(() => {
  if (isImage.value) return "图片";
  if (isAudio.value) return "音频";
  return "视频";
});

const showProMultiPlatform = computed(() => selectedPlatforms.value.length > 1);
const rewriteInspectionTone = computed(() => {
  const result = rewriteInspection.value;
  if (!result) return "neutral";
  if (result.hasWatermark) return "warning";
  if (result.reasonCode === "preflight_extract_failed") return "danger";
  return "ok";
});

function currentFeatureName() {
  if (isImage.value) return "watermark_image";
  if (isAudio.value) return "watermark_audio";
  return "watermark_video";
}

function currentMediaType() {
  if (isImage.value) return "image";
  if (isAudio.value) return "audio";
  return "video";
}

async function refreshHardwareInfo() {
  hardwareInfo.value = await getHardwareInfo();
}

async function handleSourceSelect(path: string) {
  const requestId = ++rewriteInspectionRequestId;
  selectedPath.value = path;
  rewriteInspection.value = null;
  rewriteInspectionError.value = "";
  sourceMeta.value = await probeSource(path);
  systemStatus.value = await systemCheck(path);
  trackFeatureEvent("source_probe", "success", { mediaType: currentMediaType(), source: "dropzone" });

  const type = sourceMeta.value.fileType;
  if (type === "image") {
    statusMessage.value = "图片已就绪";
  } else if (type === "audio") {
    statusMessage.value = "音频已就绪";
  } else if (sourceMeta.value.isHdr) {
    statusMessage.value = "HDR 已识别";
  } else {
    statusMessage.value = "视频已就绪";
  }

  // Auto-recommend platforms and strategy
  if (!userOverridden.value && isVideo.value) {
    const recommended = recommendPlatforms(sourceMeta.value);
    selectedPlatforms.value = recommended;
    if (hardwareInfo.value) {
      const strategy = recommendStrategy(sourceMeta.value, recommended, hardwareInfo.value);
      options.aspectStrategy = strategy.aspectStrategy;
      options.encodingMode = strategy.encodingMode;
    }
    showRecommendHint.value = true;
    setTimeout(() => { showRecommendHint.value = false; }, 3000);
  }

  // Generate warnings
  warnings.value = generateWarnings(sourceMeta.value, selectedPlatforms.value);

  if (isImage.value || isAudio.value) {
    await refreshRewriteInspection(path, requestId);
  }
}

async function refreshRewriteInspection(path: string, requestId = ++rewriteInspectionRequestId) {
  rewriteInspectionLoading.value = true;
  rewriteInspectionError.value = "";
  try {
    const result = await inspectRewriteTarget(path);
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspection.value = result;
      if (result.hasWatermark && !options.allowRewrite) {
        statusMessage.value = `检测到已有版权记录；如需生成新版，请开启重写，本次将记录为第 ${result.nextRevision} 次写入`;
      }
    }
  } catch (err: any) {
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspection.value = null;
      rewriteInspectionError.value = err?.message ?? String(err);
    }
  } finally {
    if (requestId === rewriteInspectionRequestId) {
      rewriteInspectionLoading.value = false;
    }
  }
}

function togglePlatform(platform: Platform) {
  userOverridden.value = true;
  if (selectedPlatforms.value.includes(platform)) {
    selectedPlatforms.value = selectedPlatforms.value.filter((item) => item !== platform);
  } else {
    selectedPlatforms.value = [...selectedPlatforms.value, platform];
  }
  // Regenerate warnings on platform change
  if (sourceMeta.value) {
    warnings.value = generateWarnings(sourceMeta.value, selectedPlatforms.value);
  }
}

function setProgress(payload: Partial<PipelineProgressPayload>) {
  progress.pipelineId = payload.pipelineId ?? progress.pipelineId;
  progress.stage = payload.stage ?? progress.stage;
  progress.percent = payload.percent ?? progress.percent;
  progress.platformPercents = payload.platformPercents ?? progress.platformPercents;
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
    "这会让当前文件优先识别到新的版权编号。HiddenShield 会在版权库中记录上一版编号、写入次数和新版原因。",
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
  if (isVideo.value && selectedPlatforms.value.length === 0) {
    statusMessage.value = "请至少勾选一个目标平台"; return;
  }
  if (isVideo.value && selectedPlatforms.value.length > 1) {
    statusMessage.value = "多平台需订阅"; return;
  }
  if (!isImage.value && systemStatus.value && !systemStatus.value.ffmpegAvailable) {
    statusMessage.value = "未找到 FFmpeg，请先安装";
    return;
  }
  if (systemStatus.value && !systemStatus.value.outputDirWritable) {
    statusMessage.value = `目标输出目录不可写：${systemStatus.value.outputDir}`;
    return;
  }
  if (!(await confirmRewriteRisk())) {
    statusMessage.value = "已取消重写";
    return;
  }

  busy.value = true;
  showResult.value = false;
  completePayload.value = null;
  const platforms = isVideo.value ? selectedPlatforms.value : [];
  const featureName = currentFeatureName();
  trackFeatureEvent(featureName, "start", {
    mediaType: currentMediaType(),
    source: isVideo.value ? `platforms:${platforms.join(",") || "none"}` : "single_media",
  });

  // Save for retry
  lastInputPath.value = selectedPath.value;
  lastPlatforms.value = [...platforms];
  lastOptions.aspectStrategy = options.aspectStrategy;
  lastOptions.encodingMode = options.encodingMode;
  lastOptions.allowRewrite = options.allowRewrite;
  lastOptions.rewriteReason = options.rewriteReason;

  // Collect AI content options from AIContentMarker
  const aiContent = aiMarkerRef.value ? {
    isAiGenerated: aiMarkerRef.value.isAIGenerated,
    trainingPermission: aiMarkerRef.value.trainingPermission,
    generationMethod: aiMarkerRef.value.generationMethod,
    modificationLevel: aiMarkerRef.value.modificationLevel,
    authenticityClaim: aiMarkerRef.value.authenticityClaim,
    customMetadata: aiMarkerRef.value.customMetadata.trim() || undefined,
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
    statusMessage.value = `启动失败：${err?.message ?? err}`;
    trackFeatureEvent(featureName, "failure", {
      mediaType: currentMediaType(),
      errorCode: "pipeline_start_failed",
    });
  }
}

async function handleRetry() {
  if (!lastInputPath.value) return;
  selectedPath.value = lastInputPath.value;
  selectedPlatforms.value = [...lastPlatforms.value];
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

function handleBackFromResult() {
  showResult.value = false;
  completePayload.value = null;
  selectedPath.value = "";
  sourceMeta.value = null;
  userOverridden.value = false;
  warnings.value = [];
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
    setProgress(payload);
    if (payload.percent >= 100) {
      busy.value = false;
      statusMessage.value = "完成";
    } else if (payload.stage.startsWith("失败")) {
      busy.value = false;
      statusMessage.value = payload.stage;
      trackFeatureEvent(currentFeatureName(), "failure", {
        mediaType: currentMediaType(),
        errorCode: "pipeline_runtime_failed",
        source: payload.stage,
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
    />

    <!-- Normal Workbench -->
    <template v-else>
      <section class="hero-card">
        <div>
          <p class="eyebrow">工作台</p>
          <h2>处理作品</h2>
        </div>
        <div class="hero-card__stats">
          <div>
            <span>处理方式</span>
            <strong>{{ hardwareInfo?.preferredEncoder ?? "检测中" }}</strong>
          </div>
          <div>
            <span>状态</span>
            <strong>{{ hardwareInfo?.ffmpegStatus ?? "检测中" }}</strong>
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
          />

          <PlatformSelector
            v-if="isVideo"
            :selected="selectedPlatforms"
            :disabled="busy"
            @toggle="togglePlatform"
          />

          <!-- Pro multi-platform hint -->
          <div v-if="showProMultiPlatform" class="pro-hint">
            <ProBadge label="多平台需订阅" />
          </div>

          <!-- Recommend hint -->
          <div v-if="showRecommendHint" class="recommend-hint">
            已推荐
          </div>

          <div v-if="isVideo" class="options-grid">
            <label class="select-field">
              <span>画面</span>
              <select v-model="options.aspectStrategy" :disabled="busy" @change="userOverridden = true">
                <option value="letterbox">加黑边保画面</option>
                <option value="smart_crop">智能裁剪填满</option>
              </select>
            </label>
            <label class="select-field">
              <span>模式</span>
              <select v-model="options.encodingMode" :disabled="busy" @change="userOverridden = true">
                <option value="fast_gpu">极速 GPU</option>
                <option value="high_quality_cpu">高质量 CPU</option>
              </select>
            </label>
          </div>

          <div v-if="isImage || isAudio" class="rewrite-panel">
            <div class="rewrite-panel__status rewrite-panel__status--ok">
              <strong>{{ isImage ? '图片取证优先' : '音频取证优先' }}</strong>
              <span>
                {{ isImage
                  ? '将生成 PNG 保护副本，并在完成前回读验证版权编号。'
                  : '将生成 WAV 保护副本，并在完成前回读验证版权编号。' }}
              </span>
            </div>
            <label class="rewrite-panel__toggle">
              <input v-model="options.allowRewrite" type="checkbox" :disabled="busy" />
              <span>作为新版写入</span>
            </label>
            <div v-if="rewriteInspectionLoading" class="rewrite-panel__status">
              正在检查已有版权记录...
            </div>
            <div
              v-else-if="rewriteInspection"
              class="rewrite-panel__status"
              :class="`rewrite-panel__status--${rewriteInspectionTone}`"
            >
              <strong>{{ rewriteInspection.summary }}</strong>
              <span>{{ rewriteInspection.reasonDetail }}</span>
              <span v-if="rewriteInspection.watermarkUid">上一版编号：{{ rewriteInspection.watermarkUid }}</span>
              <span v-if="rewriteInspection.detectedRevision">当前识别为第 {{ rewriteInspection.detectedRevision }} 次写入</span>
            </div>
            <div v-else-if="rewriteInspectionError" class="rewrite-panel__status rewrite-panel__status--danger">
              写入检查失败：{{ rewriteInspectionError }}
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

          <div class="action-row">
            <button class="primary-button" type="button" :disabled="busy || !sourceMeta" @click="handleStart">
              {{ isVideo ? '开始处理' : '生成保护副本' }}
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

          <!-- Source Warnings -->
          <SourceWarnings :warnings="warnings" />

          <div class="meta-grid">
            <div class="meta-card">
              <span>文件名</span>
              <strong>{{ sourceMeta?.fileName ?? "未选择文件" }}</strong>
            </div>
            <div class="meta-card">
              <span>类型</span>
              <strong>{{ sourceMeta ? fileTypeLabel : "--" }}</strong>
            </div>
            <div v-if="isVideo || isAudio" class="meta-card">
              <span>时长</span>
              <strong>{{ sourceMeta ? `${sourceMeta.durationSecs}s` : "--" }}</strong>
            </div>
            <div v-if="isVideo" class="meta-card">
              <span>分辨率</span>
              <strong>{{ sourceMeta ? `${sourceMeta.width}x${sourceMeta.height}` : "--" }}</strong>
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
        ⚠️ {{ degradationWarning }}
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
  border: 1px solid rgba(198, 91, 32, 0.22);
  border-radius: 10px;
  background: rgba(198, 91, 32, 0.08);
}

.rewrite-panel__toggle {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.9rem;
  color: var(--text-primary);
}

.rewrite-panel__toggle input {
  accent-color: #c65b20;
}

.rewrite-panel__input {
  width: 100%;
  margin-top: 0.75rem;
  padding: 0.65rem 0.8rem;
  border-radius: 8px;
  border: 1px solid var(--border, #2a2a4a);
  background: var(--surface-alt, #252545);
  color: var(--text-primary, #e0e0e0);
}

.rewrite-panel__status {
  display: grid;
  gap: 0.25rem;
  margin-top: 0.75rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-secondary, #b8bac7);
  font-size: 0.84rem;
  line-height: 1.45;
}

.rewrite-panel__status strong {
  color: var(--text-primary, #f5f6fb);
  font-weight: 700;
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
</style>
