<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  analyzeAudioPair,
  analyzeImagePair,
  assetUrl,
  clearLabSession,
  inspectMediaPair,
  prepareAbxAssets,
} from "./api";
import { createAbxTrials, summarizeAbx } from "./abx";
import type {
  AbxAssets,
  AbxChoice,
  AbxIdentity,
  AbxSummary,
  AbxTrial,
  AudioAnalysisResult,
  ImageAnalysisResult,
  MediaInfo,
  PairInspection,
} from "./types";

type LabMode = "analysis" | "abx";
type ImageViewMode = "side" | "split" | "blink" | "heatmap";

const sourcePath = ref("");
const candidatePath = ref("");
const inspection = ref<PairInspection | null>(null);
const imageAnalysis = ref<ImageAnalysisResult | null>(null);
const audioAnalysis = ref<AudioAnalysisResult | null>(null);
const labMode = ref<LabMode>("analysis");
const imageViewMode = ref<ImageViewMode>("side");
const heatmapStrength = ref<1 | 4 | 16>(4);
const splitPosition = ref(50);
const zoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const dragging = ref(false);
const dragOrigin = ref({ x: 0, y: 0, panX: 0, panY: 0 });
const blinkSourceVisible = ref(true);
const blinkTimer = ref<number | null>(null);
const busy = ref(false);
const statusMessage = ref("请选择原始素材和注入后素材");
const errorMessage = ref("");
const clipStartSeconds = ref(0);
const activeAudioIdentity = ref<AbxIdentity>("source");
const sourceAudio = ref<HTMLAudioElement | null>(null);
const candidateAudio = ref<HTMLAudioElement | null>(null);
const sharedVolume = ref(0.8);

const abxAssets = ref<AbxAssets | null>(null);
const abxMode = ref<"quick" | "formal">("quick");
const abxEnvironment = ref("normal-distance");
const abxDevice = ref("monitor");
const abxTrials = ref<AbxTrial[]>([]);
const abxTrialIndex = ref(0);
const abxSummary = ref<AbxSummary | null>(null);
const stableDifferenceObserved = ref(false);
const abxPlayer = ref<HTMLAudioElement | null>(null);
const abxImageLoadError = ref("");
const abxObjectUrls = ref<string[]>([]);
const activeAbxAudioLabel = ref<"a" | "b" | "x" | null>(null);
const abxAudioStatus = ref("");

const mediaKind = computed(() => inspection.value?.source.mediaKind);
const canAnalyze = computed(
  () => !!inspection.value?.sameMediaKind && !busy.value,
);
const canStartAbx = computed(
  () =>
    !!inspection.value?.sameMediaKind &&
    ((mediaKind.value === "image" && !!imageAnalysis.value) ||
      (mediaKind.value === "audio" && !!audioAnalysis.value)) &&
    !busy.value,
);
const currentAbxTrial = computed(() => abxTrials.value[abxTrialIndex.value] ?? null);
const abxComplete = computed(
  () => abxTrials.value.length > 0 && abxTrialIndex.value >= abxTrials.value.length,
);
const imageSourceUrl = computed(() =>
  imageAnalysis.value ? imageAnalysis.value.sourcePreviewDataUrl : "",
);
const imageCandidateUrl = computed(() =>
  imageAnalysis.value ? imageAnalysis.value.candidatePreviewDataUrl : "",
);
const heatmapUrl = computed(() => {
  if (!imageAnalysis.value) return "";
  return (
    heatmapStrength.value === 1
      ? imageAnalysis.value.heatmaps.x1DataUrl
      : heatmapStrength.value === 4
        ? imageAnalysis.value.heatmaps.x4DataUrl
        : imageAnalysis.value.heatmaps.x16DataUrl
  );
});
const sourceClipUrl = computed(() =>
  audioAnalysis.value ? assetUrl(audioAnalysis.value.sourceClipPath) : "",
);
const candidateClipUrl = computed(() =>
  audioAnalysis.value ? assetUrl(audioAnalysis.value.candidateClipPath) : "",
);
const maxClipStart = computed(() =>
  Math.max(0, (audioAnalysis.value?.alignment.commonDurationSeconds ?? 0) - 0.1),
);
const imageTransform = computed(
  () => `translate(${panX.value}px, ${panY.value}px) scale(${zoom.value})`,
);
const sourceWaveformPoints = computed(() =>
  waveformPoints(audioAnalysis.value?.waveform.source ?? [], 120, 38),
);
const candidateWaveformPoints = computed(() =>
  waveformPoints(audioAnalysis.value?.waveform.candidate ?? [], 120, 38),
);
const differenceWaveformPoints = computed(() =>
  waveformPoints(audioAnalysis.value?.waveform.difference ?? [], 120, 70),
);

watch(imageViewMode, (mode) => {
  stopBlink();
  if (mode === "blink") {
    blinkTimer.value = window.setInterval(() => {
      blinkSourceVisible.value = !blinkSourceVisible.value;
    }, 450);
  }
});

watch(sharedVolume, (value) => {
  if (sourceAudio.value) sourceAudio.value.volume = value;
  if (candidateAudio.value) candidateAudio.value.volume = value;
  if (abxPlayer.value) abxPlayer.value.volume = value;
});

onBeforeUnmount(() => {
  stopBlink();
  revokeAbxObjectUrls();
  void clearLabSession();
});

async function chooseFile(side: "source" | "candidate") {
  errorMessage.value = "";
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "图片或音频",
        extensions: ["png", "jpg", "jpeg", "webp", "bmp", "wav", "mp3", "flac", "m4a", "aac"],
      },
    ],
  });
  if (typeof selected !== "string") return;
  if (side === "source") sourcePath.value = selected;
  else candidatePath.value = selected;
  resetResults();
  if (sourcePath.value && candidatePath.value) {
    await refreshInspection();
  }
}

async function refreshInspection() {
  busy.value = true;
  statusMessage.value = "正在执行素材配对预检";
  try {
    inspection.value = await inspectMediaPair(sourcePath.value, candidatePath.value);
    statusMessage.value = inspection.value.formallyComparable
      ? "素材配对有效，可以开始质量分析"
      : "素材可读取，但存在正式比较阻断项";
  } catch (error) {
    inspection.value = null;
    errorMessage.value = String(error);
    statusMessage.value = "素材配对预检失败";
  } finally {
    busy.value = false;
  }
}

async function runAnalysis() {
  if (!inspection.value) return;
  busy.value = true;
  errorMessage.value = "";
  statusMessage.value = "正在计算质量指标和诊断素材";
  try {
    if (inspection.value.source.mediaKind === "image") {
      imageAnalysis.value = await analyzeImagePair(sourcePath.value, candidatePath.value);
      statusMessage.value = "图片客观指标和差异热力图已生成";
    } else {
      audioAnalysis.value = await analyzeAudioPair(
        sourcePath.value,
        candidatePath.value,
        clipStartSeconds.value,
      );
      clipStartSeconds.value = audioAnalysis.value.clipStartSeconds;
      statusMessage.value = audioAnalysis.value.formallyComparable
        ? "音频对齐、客观指标和试听片段已生成"
        : "音频诊断已生成，但当前配对不形成正式阈值结论";
      await nextTick();
      if (sourceAudio.value) sourceAudio.value.volume = sharedVolume.value;
      if (candidateAudio.value) candidateAudio.value.volume = sharedVolume.value;
    }
  } catch (error) {
    errorMessage.value = String(error);
    statusMessage.value = "质量分析失败";
  } finally {
    busy.value = false;
  }
}

async function updateAudioClip() {
  if (mediaKind.value !== "audio") return;
  await runAnalysis();
}

async function switchAudio(identity: AbxIdentity) {
  const current = identity === "source" ? candidateAudio.value : sourceAudio.value;
  const next = identity === "source" ? sourceAudio.value : candidateAudio.value;
  if (!next) return;
  const time = current?.currentTime ?? next.currentTime;
  current?.pause();
  next.currentTime = Math.min(time, next.duration || time);
  next.volume = sharedVolume.value;
  activeAudioIdentity.value = identity;
  await next.play();
}

function onPanStart(event: PointerEvent) {
  dragging.value = true;
  dragOrigin.value = {
    x: event.clientX,
    y: event.clientY,
    panX: panX.value,
    panY: panY.value,
  };
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onPanMove(event: PointerEvent) {
  if (!dragging.value) return;
  panX.value = dragOrigin.value.panX + event.clientX - dragOrigin.value.x;
  panY.value = dragOrigin.value.panY + event.clientY - dragOrigin.value.y;
}

function onPanEnd() {
  dragging.value = false;
}

function resetView() {
  zoom.value = 1;
  panX.value = 0;
  panY.value = 0;
  splitPosition.value = 50;
}

async function startAbx() {
  if (!canStartAbx.value) return;
  busy.value = true;
  errorMessage.value = "";
  abxImageLoadError.value = "";
  abxAudioStatus.value = "";
  statusMessage.value = "正在准备盲测素材";
  try {
    revokeAbxObjectUrls();
    if (mediaKind.value === "image") {
      abxDevice.value = "monitor";
      abxEnvironment.value = "normal-distance";
      if (!imageAnalysis.value) {
        throw new Error("图片分析预览不可用，请先重新运行质量分析");
      }
      const sourceAsset = dataUrlToObjectUrl(
        imageAnalysis.value.sourcePreviewDataUrl,
        "image/png",
      );
      const candidateAsset = dataUrlToObjectUrl(
        imageAnalysis.value.candidatePreviewDataUrl,
        "image/png",
      );
      abxObjectUrls.value = [sourceAsset, candidateAsset];
      abxAssets.value = {
        mediaKind: "image",
        sourceAsset,
        candidateAsset,
        startSeconds: 0,
        durationSeconds: 0,
      };
    } else {
      abxDevice.value = "headphones";
      abxEnvironment.value = "quiet-room";
      const preparedAssets = await prepareAbxAssets(
        sourcePath.value,
        candidatePath.value,
        clipStartSeconds.value,
      );
      const sourceAsset = dataUrlToObjectUrl(preparedAssets.sourceAsset, "audio/wav");
      const candidateAsset = dataUrlToObjectUrl(
        preparedAssets.candidateAsset,
        "audio/wav",
      );
      abxObjectUrls.value = [sourceAsset, candidateAsset];
      abxAssets.value = {
        ...preparedAssets,
        sourceAsset,
        candidateAsset,
      };
    }
    abxTrials.value = createAbxTrials(abxMode.value === "quick" ? 10 : 20);
    abxTrialIndex.value = 0;
    abxSummary.value = null;
    stableDifferenceObserved.value = false;
    labMode.value = "abx";
    statusMessage.value =
      mediaKind.value === "image"
        ? "ABX 已开始（图片内存预览 v3）；每轮结果将在全部完成后统一揭晓"
        : "ABX 已开始（音频内存播放 v3）；每轮结果将在全部完成后统一揭晓";
  } catch (error) {
    errorMessage.value = String(error);
    statusMessage.value = "ABX 素材准备失败";
  } finally {
    busy.value = false;
  }
}

function submitAbxAnswer(choice: AbxChoice) {
  const trial = currentAbxTrial.value;
  if (!trial) return;
  trial.answer = choice;
  abxTrialIndex.value += 1;
  if (abxComplete.value) {
    finishAbx();
  } else if (abxPlayer.value) {
    abxPlayer.value.pause();
    abxPlayer.value.currentTime = 0;
    activeAbxAudioLabel.value = null;
    abxAudioStatus.value = "";
  }
}

function finishAbx() {
  abxSummary.value = summarizeAbx(abxTrials.value, stableDifferenceObserved.value);
}

async function playAbx(label: "a" | "b" | "x") {
  const trial = currentAbxTrial.value;
  const assets = abxAssets.value;
  if (!trial || !assets || !abxPlayer.value) return;
  const identity =
    label === "a" ? trial.a : label === "b" ? trial.b : trial[trial.x];
  const selectedAsset =
    identity === "source" ? assets.sourceAsset : assets.candidateAsset;
  try {
    abxAudioStatus.value = "";
    abxPlayer.value.pause();
    abxPlayer.value.src = selectedAsset;
    abxPlayer.value.currentTime = 0;
    abxPlayer.value.volume = sharedVolume.value;
    abxPlayer.value.load();
    await abxPlayer.value.play();
    activeAbxAudioLabel.value = label;
    abxAudioStatus.value = `正在播放 ${label.toUpperCase()}`;
  } catch (error) {
    activeAbxAudioLabel.value = null;
    abxAudioStatus.value = `音频播放失败：${String(error)}`;
  }
}

function abxImageUrl(label: "a" | "b" | "x") {
  const trial = currentAbxTrial.value;
  const assets = abxAssets.value;
  if (!trial || !assets) return "";
  const identity =
    label === "a" ? trial.a : label === "b" ? trial.b : trial[trial.x];
  return identity === "source" ? assets.sourceAsset : assets.candidateAsset;
}

function dataUrlToObjectUrl(dataUrl: string, expectedMimeType: string) {
  const separator = dataUrl.indexOf(",");
  const prefix = `data:${expectedMimeType};base64,`;
  if (!dataUrl.startsWith(prefix) || separator < 0) {
    throw new Error(`ABX ${expectedMimeType} 资源格式无效，请重新运行质量分析`);
  }
  const binary = atob(dataUrl.slice(separator + 1));
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return URL.createObjectURL(new Blob([bytes], { type: expectedMimeType }));
}

function revokeAbxObjectUrls() {
  for (const url of abxObjectUrls.value) {
    URL.revokeObjectURL(url);
  }
  abxObjectUrls.value = [];
}

function onAbxImageError(label: "A" | "B" | "X") {
  abxImageLoadError.value = `${label} 图片预览加载失败，请退出盲测并重新运行图片质量分析`;
}

async function clearSession() {
  await clearLabSession();
  sourcePath.value = "";
  candidatePath.value = "";
  inspection.value = null;
  resetResults();
  statusMessage.value = "会话已清空，临时分析素材已删除";
}

function resetResults() {
  revokeAbxObjectUrls();
  activeAbxAudioLabel.value = null;
  abxAudioStatus.value = "";
  imageAnalysis.value = null;
  audioAnalysis.value = null;
  abxAssets.value = null;
  abxTrials.value = [];
  abxTrialIndex.value = 0;
  abxSummary.value = null;
  labMode.value = "analysis";
  resetView();
}

function waveformPoints(values: number[], width: number, height: number) {
  if (!values.length) return "";
  const max = Math.max(...values, 1e-9);
  return values
    .map((value, index) => {
      const x = (index / Math.max(1, values.length - 1)) * width;
      const y = height - (value / max) * height;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

function formatBytes(value: number) {
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 / 1024).toFixed(2)} MiB`;
}

function formatMediaMeta(info: MediaInfo) {
  if (info.mediaKind === "image") {
    return `${info.width} × ${info.height} · ${info.extension.toUpperCase()} · ${formatBytes(info.fileBytes)}`;
  }
  return `${info.durationSeconds?.toFixed(2)}s · ${info.sampleRate} Hz · ${info.channels} ch · ${info.codec ?? info.extension}`;
}

function thresholdClass(passed: boolean) {
  return passed ? "status-pass" : "status-block";
}

function diagnosisLabel(value: string) {
  const labels: Record<string, string> = {
    specific_watermark_band_energy_redistribution: "差异能量主要集中在水印频带",
    full_band_noise_floor_statistical_amplification: "差异呈全频噪声底统计放大",
    audible_distortion_not_indicated_by_objective_diagnostic: "客观诊断未定位到单一主导模式",
  };
  return labels[value] ?? value;
}

function abxConclusionLabel(summary: AbxSummary) {
  if (summary.conclusion === "review_required") return "进入质量复核";
  if (summary.conclusion === "inconclusive") return "结果不确定";
  return "未观察到稳定可区分证据";
}

function stopBlink() {
  if (blinkTimer.value !== null) {
    window.clearInterval(blinkTimer.value);
    blinkTimer.value = null;
  }
}
</script>

<template>
  <main class="lab-shell">
    <header class="hero">
      <div>
        <p class="eyebrow">LOCAL · OFFLINE · INTERNAL QA</p>
        <h1>HiddenShield 感知质量实验室</h1>
        <p class="hero-copy">
          对比水印注入前后的图片与音频，并通过客观指标和单人 ABX 判断是否存在稳定可感知差异。
        </p>
      </div>
      <div class="hero-actions">
        <span class="privacy-pill">不联网 · 不保存历史</span>
        <button class="button button-ghost" type="button" @click="clearSession">清空会话</button>
      </div>
    </header>

    <section class="boundary-banner">
      本工具只提供内部质量诊断。任何结果都不证明“绝对无感”“零影响”或所有真实素材均不可分辨。
    </section>

    <section class="mode-tabs" aria-label="实验室模式">
      <button
        :class="{ active: labMode === 'analysis' }"
        type="button"
        @click="labMode = 'analysis'"
      >
        质量分析
      </button>
      <button
        :class="{ active: labMode === 'abx' }"
        type="button"
        :disabled="!canStartAbx && labMode !== 'abx'"
        @click="abxAssets ? labMode = 'abx' : startAbx()"
      >
        ABX 盲测
      </button>
      <span>{{ statusMessage }}</span>
    </section>

    <template v-if="labMode === 'analysis'">
      <section class="pair-grid">
        <article class="file-card">
          <div class="file-card-heading">
            <span class="step-index">01</span>
            <div>
              <p>CONTROL</p>
              <h2>原始素材</h2>
            </div>
          </div>
          <button class="file-picker" type="button" @click="chooseFile('source')">
            <strong>{{ inspection?.source.fileName || "选择原始文件" }}</strong>
            <span>{{ inspection ? formatMediaMeta(inspection.source) : "PNG/JPEG/WebP/BMP 或 WAV/MP3/FLAC/M4A/AAC" }}</span>
          </button>
        </article>

        <article class="file-card">
          <div class="file-card-heading">
            <span class="step-index">02</span>
            <div>
              <p>CANDIDATE</p>
              <h2>注入后素材</h2>
            </div>
          </div>
          <button class="file-picker" type="button" @click="chooseFile('candidate')">
            <strong>{{ inspection?.candidate.fileName || "选择注入后文件" }}</strong>
            <span>{{ inspection ? formatMediaMeta(inspection.candidate) : "与原始素材保持同一作品和媒体类型" }}</span>
          </button>
        </article>
      </section>

      <section v-if="inspection" class="inspection-panel">
        <div>
          <span :class="inspection.formallyComparable ? 'status-pass' : 'status-block'">
            {{ inspection.formallyComparable ? "配对可正式比较" : "存在正式比较阻断" }}
          </span>
          <p v-for="warning in inspection.warnings" :key="warning" class="warning-line">{{ warning }}</p>
          <p v-for="blocker in inspection.blockers" :key="blocker" class="blocker-line">{{ blocker }}</p>
        </div>
        <button class="button button-primary" type="button" :disabled="!canAnalyze" @click="runAnalysis">
          {{ busy ? "分析中…" : "开始质量分析" }}
        </button>
      </section>

      <p v-if="errorMessage" class="error-panel">{{ errorMessage }}</p>

      <section v-if="imageAnalysis" class="analysis-panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">VISUAL DIAGNOSTIC</p>
            <h2>图片同步观察</h2>
          </div>
          <div class="segmented-control">
            <button v-for="mode in ['side', 'split', 'blink', 'heatmap']" :key="mode"
              :class="{ active: imageViewMode === mode }" type="button"
              @click="imageViewMode = mode as ImageViewMode">
              {{ { side: "并排", split: "分割", blink: "闪烁", heatmap: "热力图" }[mode] }}
            </button>
          </div>
        </div>

        <div class="viewer-toolbar">
          <label>缩放 <input v-model.number="zoom" type="range" min="0.5" max="6" step="0.1" /></label>
          <span>{{ zoom.toFixed(1) }}×</span>
          <button class="text-button" type="button" @click="resetView">重置视图</button>
          <template v-if="imageViewMode === 'split'">
            <label>分割 <input v-model.number="splitPosition" type="range" min="0" max="100" /></label>
          </template>
          <template v-if="imageViewMode === 'heatmap'">
            <button v-for="strength in [1, 4, 16]" :key="strength"
              :class="{ active: heatmapStrength === strength }" class="mini-toggle"
              type="button" @click="heatmapStrength = strength as 1 | 4 | 16">
              {{ strength }}×
            </button>
          </template>
        </div>

        <div
          class="image-viewer"
          @pointerdown="onPanStart"
          @pointermove="onPanMove"
          @pointerup="onPanEnd"
          @pointercancel="onPanEnd"
        >
          <div v-if="imageViewMode === 'side'" class="side-view">
            <figure><img :src="imageSourceUrl" :style="{ transform: imageTransform }" /><figcaption>原始素材</figcaption></figure>
            <figure><img :src="imageCandidateUrl" :style="{ transform: imageTransform }" /><figcaption>注入后素材</figcaption></figure>
          </div>
          <div v-else-if="imageViewMode === 'split'" class="overlay-view">
            <img :src="imageSourceUrl" :style="{ transform: imageTransform }" />
            <div class="split-layer" :style="{ clipPath: `inset(0 ${100 - splitPosition}% 0 0)` }">
              <img :src="imageCandidateUrl" :style="{ transform: imageTransform }" />
            </div>
            <div class="split-line" :style="{ left: `${splitPosition}%` }"></div>
          </div>
          <div v-else-if="imageViewMode === 'blink'" class="overlay-view">
            <img :src="blinkSourceVisible ? imageSourceUrl : imageCandidateUrl" :style="{ transform: imageTransform }" />
            <span class="viewer-badge">{{ blinkSourceVisible ? "A" : "B" }}</span>
          </div>
          <div v-else class="overlay-view heatmap-view">
            <img :src="heatmapUrl" :style="{ transform: imageTransform }" />
            <span class="viewer-badge">差异放大 {{ heatmapStrength }}×</span>
          </div>
        </div>
        <p v-if="imageViewMode === 'heatmap'" class="diagnostic-note">热力图仅用于定位差异，不代表正常观看效果。</p>

        <div class="metric-guide">
          <strong>如何理解图片指标</strong>
          <p>先看 PSNR 与 SSIM 是否达到内部阈值，再用 MAE、P95、最大差异和变化像素率定位差异形态。参考区间用于帮助阅读，不等同于人眼“无感”证明。</p>
        </div>
        <div class="metric-grid">
          <article>
            <span>PSNR</span><strong>{{ imageAnalysis.report.psnr.toFixed(2) }} dB</strong>
            <small class="metric-meaning">衡量整体像素误差；数值越高，解码后的两张图越接近。</small>
            <div class="metric-direction">越高越好</div>
            <div class="metric-ranges"><b>&lt;30 明显风险</b><b>30–38 复核</b><b>38–42 良好</b><b>≥42 优秀</b></div>
          </article>
          <article>
            <span>SSIM</span><strong>{{ imageAnalysis.report.ssim.toFixed(6) }}</strong>
            <small class="metric-meaning">衡量亮度、对比度和结构相似度，值域通常为 0–1，越接近 1 越相似。</small>
            <div class="metric-direction">越接近 1 越好 · 当前 quality gate 口径</div>
            <div class="metric-ranges"><b>&lt;0.980 风险</b><b>0.980–0.990 复核</b><b>0.990–0.995 良好</b><b>≥0.995 优秀</b></div>
          </article>
          <article>
            <span>MAE</span><strong>{{ imageAnalysis.report.mae.toFixed(4) }}</strong>
            <small class="metric-meaning">每个 RGB 通道的平均绝对差，值域 0–255；0 代表解码像素完全一致。</small>
            <div class="metric-direction">越低越好 · 诊断项</div>
            <div class="metric-ranges"><b>0 一致</b><b>&lt;0.5 很小</b><b>0.5–2 轻微</b><b>&gt;2 建议复核</b></div>
          </article>
          <article>
            <span>P95 差异</span><strong>{{ imageAnalysis.report.p95AbsoluteDifference.toFixed(1) }}</strong>
            <small class="metric-meaning">95% 的 RGB 通道差异不超过该值，值域 0–255，可避免只看平均值遗漏广泛变化。</small>
            <div class="metric-direction">越低越好 · 诊断项</div>
            <div class="metric-ranges"><b>0–1 优秀</b><b>2–4 轻微</b><b>&gt;4 建议复核</b></div>
          </article>
          <article>
            <span>最大通道差</span><strong>{{ imageAnalysis.report.maxChannelDifference }}</strong>
            <small class="metric-meaning">全图最严重的单个 RGB 通道差异，值域 0–255，对单个异常点非常敏感。</small>
            <div class="metric-direction">越低越好 · 不可单独判定</div>
            <div class="metric-ranges"><b>0 一致</b><b>1–8 小范围差异</b><b>&gt;8 查看热力图</b></div>
          </article>
          <article>
            <span>变化像素率</span><strong>{{ (imageAnalysis.report.changedPixelRatio * 100).toFixed(2) }}%</strong>
            <small class="metric-meaning">任一 RGB 通道发生变化的像素比例。大量像素各变化 1 级，也可能保持较高视觉质量。</small>
            <div class="metric-direction">无统一好坏方向 · 需结合 PSNR / SSIM / MAE</div>
            <div class="metric-ranges"><b>0% 完全一致</b><b>其余数值仅描述变化覆盖面</b></div>
          </article>
        </div>

        <div class="threshold-grid">
          <article :class="thresholdClass(imageAnalysis.report.forensic.passed)">
            <span>Forensic 默认</span>
            <strong>{{ imageAnalysis.report.forensic.passed ? "自动指标符合内部阈值" : "自动指标阻断" }}</strong>
            <small>PSNR ≥ 38 dB · SSIM ≥ 0.990</small>
          </article>
          <article :class="thresholdClass(imageAnalysis.report.balanced.passed)">
            <span>Balanced 候选</span>
            <strong>{{ imageAnalysis.report.balanced.passed ? "候选阈值满足" : "候选阈值未满足" }}</strong>
            <small>内部候选标准，不是当前产品策略</small>
          </article>
        </div>
      </section>

      <section v-if="audioAnalysis" class="analysis-panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">AUDITORY DIAGNOSTIC</p>
            <h2>音频同步试听</h2>
          </div>
          <span :class="audioAnalysis.formallyComparable ? 'status-pass' : 'status-block'">
            {{ audioAnalysis.formallyComparable ? "已完成可靠对齐" : "仅诊断，不形成正式结论" }}
          </span>
        </div>

        <div class="alignment-strip">
          <span>检测偏移 <strong>{{ (audioAnalysis.alignment.detectedOffsetSeconds * 1000).toFixed(1) }} ms</strong></span>
          <span>相关性 <strong>{{ audioAnalysis.alignment.correlationScore.toFixed(4) }}</strong></span>
          <span>共同区间 <strong>{{ audioAnalysis.alignment.commonDurationSeconds.toFixed(2) }}s</strong></span>
        </div>
        <p class="alignment-help">偏移越接近 0 越好，正式对齐容差为 ±250 ms；相关性越接近 1 越可靠，低于 0.10 会阻断正式分析；共同区间表示实际参与比较的时长，不代表质量好坏。</p>

        <div class="waveform-card">
          <svg viewBox="0 0 120 100" preserveAspectRatio="none" aria-label="同步波形">
            <polyline :points="sourceWaveformPoints" class="wave-source" />
            <polyline :points="candidateWaveformPoints" class="wave-candidate" />
            <polyline :points="differenceWaveformPoints" class="wave-difference" />
          </svg>
          <div class="wave-legend"><span class="source">原始</span><span class="candidate">注入后</span><span class="difference">差异放大</span></div>
        </div>

        <div class="audio-controls">
          <button :class="{ active: activeAudioIdentity === 'source' }" type="button" @click="switchAudio('source')">试听原始</button>
          <button :class="{ active: activeAudioIdentity === 'candidate' }" type="button" @click="switchAudio('candidate')">试听注入后</button>
          <label>共同音量 <input v-model.number="sharedVolume" type="range" min="0" max="1" step="0.05" /></label>
          <audio ref="sourceAudio" :src="sourceClipUrl" preload="auto" />
          <audio ref="candidateAudio" :src="candidateClipUrl" preload="auto" />
        </div>

        <div class="clip-selector">
          <label>10 秒试听窗口起点
            <input v-model.number="clipStartSeconds" type="range" min="0" :max="maxClipStart" step="0.1" />
          </label>
          <strong>{{ clipStartSeconds.toFixed(1) }}s</strong>
          <button class="button button-ghost" type="button" @click="updateAudioClip">更新试听片段</button>
        </div>

        <div class="metric-guide">
          <strong>如何理解音频指标</strong>
          <p>先看整体/分段 SNR、LUFS 差值和新增 clipping，再用峰值、静音噪声底与频带能量定位问题。所有区间均基于当前内部口径，不能替代耳听或 ABX。</p>
        </div>
        <div class="metric-grid">
          <article>
            <span>整体 SNR</span><strong>{{ audioAnalysis.report.snr.toFixed(2) }} dB</strong>
            <small class="metric-meaning">原始信号能量与差异噪声能量之比；数值越高，整体改动越不突出。</small>
            <div class="metric-direction">越高越好</div>
            <div class="metric-ranges"><b>&lt;30 明显风险</b><b>30–44 复核</b><b>44–50 良好</b><b>≥50 优秀</b></div>
          </article>
          <article>
            <span>分段 SNR</span><strong>{{ audioAnalysis.report.perceptualDiagnosis.segmentedSnr.min.toFixed(1) }} / {{ audioAnalysis.report.perceptualDiagnosis.segmentedSnr.mean.toFixed(1) }} / {{ audioAnalysis.report.perceptualDiagnosis.segmentedSnr.max.toFixed(1) }}</strong>
            <small class="metric-meaning">按 1 秒窗口统计最小 / 均值 / 最大 SNR；最小值用于发现短暂、局部的异常。</small>
            <div class="metric-direction">三者都越高越好，优先关注最小值</div>
            <div class="metric-ranges"><b>最小值 &lt;30 高风险</b><b>30–44 复核</b><b>≥44 良好</b></div>
          </article>
          <article>
            <span>LUFS 差值</span><strong>{{ audioAnalysis.report.lufsDelta.toFixed(4) }} LU</strong>
            <small class="metric-meaning">描述整体响度变化；当前使用 quality gate 的近似 LUFS 计算口径。</small>
            <div class="metric-direction">越接近 0 越好</div>
            <div class="metric-ranges"><b>≤0.3 优秀</b><b>0.3–0.5 良好</b><b>&gt;0.5 Forensic 阻断</b></div>
          </article>
          <article>
            <span>峰值差异</span><strong>{{ audioAnalysis.report.peakDelta.toFixed(6) }}</strong>
            <small class="metric-meaning">两份音频最大绝对采样幅度的差值，当前 gate 使用 0–1 采样幅度口径，并非 dB。</small>
            <div class="metric-direction">越接近 0 越好</div>
            <div class="metric-ranges"><b>Release ≤0.08</b><b>Balanced ≤0.5</b><b>Forensic ≤0.8</b></div>
          </article>
          <article>
            <span>新增 clipping</span><strong>{{ audioAnalysis.report.newClipping }}</strong>
            <small class="metric-meaning">注入后新出现的削波采样点数量；削波可能带来明显失真或爆音。</small>
            <div class="metric-direction">必须为 0</div>
            <div class="metric-ranges"><b>0 通过</b><b>&gt;0 阻断并复核</b></div>
          </article>
          <article>
            <span>静音噪声底变化</span><strong>{{ audioAnalysis.report.silenceNoiseFloorDelta.toExponential(3) }}</strong>
            <small class="metric-meaning">安静片段中“候选 RMS − 原始 RMS”；正值表示噪声底升高，负值表示降低。</small>
            <div class="metric-direction">越接近 0 越好 · 暂无正式阈值</div>
            <div class="metric-ranges"><b>诊断项：结合静音段试听与差异波形判断</b></div>
          </article>
        </div>

        <div class="band-grid">
          <article><span>低频差异能量</span><strong>{{ (audioAnalysis.report.perceptualDiagnosis.bandEnergy.lowNoiseShare * 100).toFixed(3) }}%</strong><small>差异能量落在 20–1500 Hz 的比例。没有统一好坏方向；低频异常通常更易被察觉。</small></article>
          <article><span>水印频带差异能量</span><strong>{{ (audioAnalysis.report.perceptualDiagnosis.bandEnergy.watermarkNoiseShare * 100).toFixed(3) }}%</strong><small>差异能量落在 2–8 kHz 的比例。比例高表示改动集中于当前诊断频带，需结合 SNR 与 ABX。</small></article>
          <article><span>高频差异能量</span><strong>{{ (audioAnalysis.report.perceptualDiagnosis.bandEnergy.highNoiseShare * 100).toFixed(3) }}%</strong><small>差异能量落在 8–16 kHz 的比例。三个频带比例约合计 100%，仅用于定位。</small></article>
          <article><span>诊断</span><strong>{{ diagnosisLabel(audioAnalysis.report.perceptualDiagnosis.diagnosis) }}</strong><small>根据分段 SNR 与主导差异频带生成的提示，不是独立通过结论。</small></article>
        </div>

        <div class="threshold-grid">
          <article :class="thresholdClass(audioAnalysis.report.forensic.passed && audioAnalysis.formallyComparable)">
            <span>Forensic 默认</span>
            <strong>{{ audioAnalysis.report.forensic.passed && audioAnalysis.formallyComparable ? "自动指标符合内部阈值" : "自动指标或配对条件阻断" }}</strong>
            <small>SNR ≥ 44 dB · LUFS ≤ 0.5 · 无新增 clipping</small>
          </article>
          <article :class="thresholdClass(audioAnalysis.report.balanced.passed && audioAnalysis.formallyComparable)">
            <span>Balanced 候选</span>
            <strong>{{ audioAnalysis.report.balanced.passed && audioAnalysis.formallyComparable ? "候选阈值满足" : "候选阈值未满足" }}</strong>
            <small>内部候选标准，不是当前产品策略</small>
          </article>
        </div>
      </section>

      <section v-if="canStartAbx" class="abx-launch">
        <div>
          <p class="eyebrow">NEXT: HUMAN PERCEPTION</p>
          <h2>客观指标不能替代实际辨别测试</h2>
          <p>开始后会隐藏文件身份，并且直到全部轮次结束才揭晓正确率。</p>
        </div>
        <div class="abx-launch-controls">
          <label><input v-model="abxMode" type="radio" value="quick" /> 10 轮快速筛查</label>
          <label><input v-model="abxMode" type="radio" value="formal" /> 20 轮正式单人测试</label>
          <button class="button button-primary" type="button" @click="startAbx">开始 ABX</button>
        </div>
      </section>
    </template>

    <section v-else class="abx-panel">
      <template v-if="!abxSummary && currentAbxTrial && abxAssets">
        <div class="section-heading">
          <div>
            <p class="eyebrow">DOUBLE-BLIND SESSION</p>
            <h2>第 {{ abxTrialIndex + 1 }} / {{ abxTrials.length }} 轮</h2>
          </div>
          <span class="privacy-pill">本轮不反馈答案</span>
        </div>

        <div class="abx-context">
          <label>设备
            <select v-model="abxDevice">
              <option v-if="abxAssets.mediaKind === 'image'" value="monitor">显示器</option>
              <option v-if="abxAssets.mediaKind === 'image'" value="phone">手机屏幕</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="headphones">耳机</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="speaker">外放</option>
            </select>
          </label>
          <label>环境
            <select v-model="abxEnvironment">
              <option v-if="abxAssets.mediaKind === 'image'" value="normal-distance">正常观看距离</option>
              <option v-if="abxAssets.mediaKind === 'image'" value="zoom-diagnostic">放大诊断</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="quiet-room">安静房间</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="office">办公室环境</option>
            </select>
          </label>
        </div>

        <template v-if="abxAssets.mediaKind === 'image'">
          <div class="abx-image-grid">
            <figure><span>A</span><img :src="abxImageUrl('a')" :style="{ transform: imageTransform }" @error="onAbxImageError('A')" /></figure>
            <figure><span>B</span><img :src="abxImageUrl('b')" :style="{ transform: imageTransform }" @error="onAbxImageError('B')" /></figure>
            <figure class="x"><span>X</span><img :src="abxImageUrl('x')" :style="{ transform: imageTransform }" @error="onAbxImageError('X')" /></figure>
          </div>
          <p v-if="abxImageLoadError" class="abx-image-error">{{ abxImageLoadError }}</p>
        </template>

        <template v-else>
          <div class="abx-audio-grid">
            <button :class="{ active: activeAbxAudioLabel === 'a' }" type="button" @click="playAbx('a')"><span>A</span>播放参考 A</button>
            <button :class="{ active: activeAbxAudioLabel === 'b' }" type="button" @click="playAbx('b')"><span>B</span>播放参考 B</button>
            <button :class="{ active: activeAbxAudioLabel === 'x' }" class="x" type="button" @click="playAbx('x')"><span>X</span>播放未知 X</button>
            <audio ref="abxPlayer" preload="auto" />
          </div>
          <p class="abx-audio-status">{{ abxAudioStatus || "点击 A、B 或 X 开始播放；每次都从同一片段起点播放。" }}</p>
        </template>

        <div class="trial-form">
          <label>判断置信度
            <input v-model.number="currentAbxTrial.confidence" type="range" min="1" max="5" step="1" />
            <strong>{{ currentAbxTrial.confidence }}/5</strong>
          </label>
          <label>感知差异
            <select v-model="currentAbxTrial.perceivedDifference">
              <option value="none">没有明确差异</option>
              <option v-if="abxAssets.mediaKind === 'image'" value="noise">噪点</option>
              <option v-if="abxAssets.mediaKind === 'image'" value="edge">边缘</option>
              <option v-if="abxAssets.mediaKind === 'image'" value="banding">色带/渐变</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="hiss">底噪</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="sibilance">齿音</option>
              <option v-if="abxAssets.mediaKind === 'audio'" value="loudness">响度</option>
              <option value="other">其他</option>
            </select>
          </label>
          <label class="notes">备注
            <input v-model="currentAbxTrial.notes" type="text" placeholder="可选：记录差异出现的位置和特征" />
          </label>
          <label class="stable-check">
            <input v-model="stableDifferenceObserved" type="checkbox" />
            我已经观察到稳定且可重复的差异
          </label>
        </div>

        <div class="answer-row">
          <p>X 与哪一个参考相同？</p>
          <button class="button button-primary" type="button" @click="submitAbxAnswer('a')">X = A</button>
          <button class="button button-primary" type="button" @click="submitAbxAnswer('b')">X = B</button>
        </div>
      </template>

      <template v-else-if="abxSummary">
        <div class="abx-result">
          <p class="eyebrow">SESSION COMPLETE</p>
          <h2>{{ abxConclusionLabel(abxSummary) }}</h2>
          <div class="result-number">{{ abxSummary.correct }} / {{ abxSummary.total }}</div>
          <p>正确率 {{ (abxSummary.correctRate * 100).toFixed(1) }}% · 单侧二项分布 p={{ abxSummary.pValue.toFixed(4) }}</p>
          <div :class="['result-verdict', `result-${abxSummary.conclusion}`]">
            <template v-if="abxSummary.conclusion === 'no_stable_evidence'">
              本次单人 ABX 未观察到稳定区分证据。该结论仅适用于当前素材、设备和测试环境。
            </template>
            <template v-else-if="abxSummary.conclusion === 'inconclusive'">
              当前结果位于不确定区间，建议使用 20 轮模式、更换设备或扩大测试人数。
            </template>
            <template v-else>
              正确率、统计显著性或稳定差异描述触发复核；不能对该素材使用“未观察到差异”的结论。
            </template>
          </div>
          <p class="diagnostic-note">单人 20 轮测试仍不能替代至少 5 人的正式发布验收。</p>
          <button class="button button-primary" type="button" @click="labMode = 'analysis'">返回质量分析</button>
        </div>
      </template>
    </section>
  </main>
</template>
