<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import DropZone from "../components/DropZone.vue";
import PlatformSelector from "../components/PlatformSelector.vue";
import ProgressPanel from "../components/ProgressPanel.vue";
import SystemStatus from "../components/SystemStatus.vue";
import SourceWarnings from "../components/SourceWarnings.vue";
import ResultPage from "../components/ResultPage.vue";
import ProBadge from "../components/ProBadge.vue";
import {
  cancelPipeline,
  checkActivePipelines,
  createEmptyPlatformPercents,
  generateWarnings,
  getHardwareInfo,
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

// For retry
const lastInputPath = ref("");
const lastPlatforms = ref<Platform[]>([]);
const lastOptions = reactive<TranscodeOptions>({ aspectStrategy: "letterbox", encodingMode: "fast_gpu" });

const progress = reactive<PipelineProgressPayload>({
  pipelineId: "",
  stage: "等待任务",
  percent: 0,
  platformPercents: createEmptyPlatformPercents(),
});

const options = reactive<TranscodeOptions>({
  aspectStrategy: "letterbox",
  encodingMode: "fast_gpu",
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
  if (isImage.value) return "🖼️ 图片";
  if (isAudio.value) return "🎵 音频";
  return "🎬 视频";
});

const showProMultiPlatform = computed(() => selectedPlatforms.value.length > 1);

async function refreshHardwareInfo() {
  hardwareInfo.value = await getHardwareInfo();
}

async function handleSourceSelect(path: string) {
  selectedPath.value = path;
  sourceMeta.value = await probeSource(path);

  const type = sourceMeta.value.fileType;
  if (type === "image") {
    statusMessage.value = "图片素材，将执行 DWT-DCT-SVD 盲水印嵌入";
  } else if (type === "audio") {
    statusMessage.value = "音频素材，将执行频域盲水印嵌入";
  } else if (sourceMeta.value.isHdr) {
    statusMessage.value = "HDR 视频，自动色彩映射 + 多平台压制";
  } else {
    statusMessage.value = "SDR 视频，标准压制链路";
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

async function handleStart() {
  if (!selectedPath.value) { statusMessage.value = "请先选择源文件"; return; }
  if (isVideo.value && selectedPlatforms.value.length === 0) {
    statusMessage.value = "请至少勾选一个目标平台"; return;
  }
  if (isVideo.value && selectedPlatforms.value.length > 1) {
    statusMessage.value = "免费版每次仅支持一个平台，升级 Pro 解锁多平台并行"; return;
  }

  busy.value = true;
  showResult.value = false;
  completePayload.value = null;
  const platforms = isVideo.value ? selectedPlatforms.value : [];

  // Save for retry
  lastInputPath.value = selectedPath.value;
  lastPlatforms.value = [...platforms];
  lastOptions.aspectStrategy = options.aspectStrategy;
  lastOptions.encodingMode = options.encodingMode;

  try {
    const result = await startPipeline(selectedPath.value, platforms, options);
    pipelineId.value = result.pipelineId;
    statusMessage.value = result.summary;
    setProgress({ pipelineId: result.pipelineId, stage: "任务已排队", percent: 1, platformPercents: createEmptyPlatformPercents() });
  } catch (err: any) {
    busy.value = false;
    statusMessage.value = `启动失败：${err?.message ?? err}`;
  }
}

async function handleRetry() {
  if (!lastInputPath.value) return;
  selectedPath.value = lastInputPath.value;
  selectedPlatforms.value = [...lastPlatforms.value];
  options.aspectStrategy = lastOptions.aspectStrategy;
  options.encodingMode = lastOptions.encodingMode;
  await handleStart();
}

async function handleCancel() {
  if (!pipelineId.value) return;
  await cancelPipeline(pipelineId.value);
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
    }
  });

  unlistenComplete = await listenPipelineComplete((payload) => {
    completePayload.value = payload;
    showResult.value = true;
    busy.value = false;
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
          <p class="eyebrow">Workbench</p>
          <h2>发布前的最后一站</h2>
          <p class="hero-card__copy">
            极致画质 + 无感版权保护
          </p>
        </div>
        <div class="hero-card__stats">
          <div>
            <span>硬件编码</span>
            <strong>{{ hardwareInfo?.preferredEncoder ?? "检测中" }}</strong>
          </div>
          <div>
            <span>FFmpeg 状态</span>
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
              <h3>导入与平台</h3>
              <p>{{ isVideo ? '选择目标平台' : isImage ? '图片隐写水印' : '音频频域水印' }}</p>
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
            <ProBadge label="多平台并行输出是 Pro 能力（免费版每次仅支持一个平台）" />
          </div>

          <!-- Recommend hint -->
          <div v-if="showRecommendHint" class="recommend-hint">
            ✨ 已为您智能推荐平台和策略
          </div>

          <div v-if="isVideo" class="options-grid">
            <label class="select-field">
              <span>横转竖策略</span>
              <select v-model="options.aspectStrategy" :disabled="busy" @change="userOverridden = true">
                <option value="letterbox">加黑边保画面</option>
                <option value="smart_crop">智能裁剪填满</option>
              </select>
            </label>
            <label class="select-field">
              <span>编码模式</span>
              <select v-model="options.encodingMode" :disabled="busy" @change="userOverridden = true">
                <option value="fast_gpu">极速 GPU</option>
                <option value="high_quality_cpu">高质量 CPU</option>
              </select>
            </label>
          </div>

          <div class="action-row">
            <button class="primary-button" type="button" :disabled="busy || !sourceMeta" @click="handleStart">
              {{ isVideo ? '开始压制' : isImage ? '嵌入图片水印' : '嵌入音频水印' }}
            </button>
            <button class="ghost-button" type="button" :disabled="!busy" @click="handleCancel">
              取消任务
            </button>
          </div>
        </div>

        <div class="panel">
          <div class="panel__header">
            <div>
              <h3>源文件画像</h3>
              <p>{{ outputSummary }}</p>
            </div>
            <span class="pill">{{ sourceMeta ? (sourceMeta.isHdr ? "HDR" : "SDR") : "待探测" }}</span>
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
              <span>SHA-256</span>
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
