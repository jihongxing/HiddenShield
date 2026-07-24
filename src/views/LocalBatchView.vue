<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  cancelPipeline,
  listLocalBatchJobs,
  listenPipelineComplete,
  listenPipelineProgress,
  saveLocalBatchJob,
  startPipeline,
  type EntitlementState,
  type LocalBatchItem,
  type LocalBatchItemStatus,
  type LocalBatchJob,
  type LocalBatchJobStatus,
  type LocalBatchMediaKind,
  type PipelineCompletePayload,
  type PipelineProgressPayload,
} from "../lib/tauri-api";
import { trackFeatureEvent } from "../lib/analytics";

const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const emit = defineEmits<{
  openSubscription: [];
}>();

const canUseBatch = computed(() => props.entitlementState?.features?.batch_processing === true);
const currentPlan = computed(() => canUseBatch.value ? "图片 / 音频年费" : "未付费");
const fileInputRef = ref<HTMLInputElement | null>(null);

const batchJob = ref<LocalBatchJob | null>(null);
const loadingJobs = ref(false);
const saveError = ref("");
const processingMedia = ref(false);
const activePipelineId = ref<string | null>(null);
let unlistenProgress: (() => void) | undefined;
let unlistenComplete: (() => void) | undefined;
const pipelineWaiters = new Map<
  string,
  {
    resolve: (payload: PipelineCompletePayload) => void;
    reject: (error: Error) => void;
  }
>();

const queueStages = [
  { label: "等待导入", detail: "选择多张图片或多首音频后创建本地队列。" },
  { label: "写入中", detail: "逐个生成保护副本，失败项不影响已完成项。" },
  { label: "完成验证", detail: "每个文件完成后立即验证版权编号并写入版权库。" },
  { label: "可重试", detail: "失败项可单独重试，也可统一重试失败列表。" },
];

const totalItems = computed(() => batchJob.value?.items.length ?? 0);
const queuedItems = computed(() => countItems("queued"));
const runningItems = computed(() => countItems("running"));
const verifiedItems = computed(() => countItems("verified"));
const failedItems = computed(() => countItems("failed"));
const cancelledItems = computed(() => countItems("cancelled"));
const canPause = computed(() => batchJob.value?.status === "queued");
const canRetryFailed = computed(() => failedItems.value > 0);
const canProcessMedia = computed(() =>
  batchJob.value?.status === "queued" &&
  batchJob.value.items.some((item) => canRunMediaItem(item)),
);

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function countItems(status: LocalBatchItemStatus) {
  return batchJob.value?.items.filter((item) => item.status === status).length ?? 0;
}

function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

function mediaKindFromName(name: string): LocalBatchMediaKind {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["jpg", "jpeg", "png", "bmp", "webp", "tiff"].includes(ext)) return "image";
  if (["wav", "mp3", "aac", "flac", "ogg", "m4a"].includes(ext)) return "audio";
  return "unsupported";
}

function statusLabel(status: LocalBatchItemStatus) {
  const labels: Record<LocalBatchItemStatus, string> = {
    queued: "待处理",
    running: "写入中",
    verified: "已验证",
    failed: "需处理",
    cancelled: "已取消",
  };
  return labels[status];
}

function jobStatusLabel(status: LocalBatchJobStatus) {
  const labels: Record<LocalBatchJobStatus, string> = {
    draft: "草稿",
    queued: "已建队列",
    paused: "已暂停",
    cancelled: "已取消",
  };
  return labels[status];
}

function currentIsoTime() {
  return new Date().toISOString();
}

function buildBatchItems(paths: string[], jobId: string, now: string) {
  return paths.map((path, index) => {
    const fileName = fileNameFromPath(path);
    const mediaKind = mediaKindFromName(fileName);
    const supported = mediaKind !== "unsupported";
    return {
      id: `batch-item-${Date.now()}-${index}`,
      jobId,
      inputRef: path,
      fileName,
      mediaKind,
      status: supported ? "queued" : "failed",
      attempts: 0,
      lastError: supported ? null : "仅支持图片和音频文件",
      outputRef: null,
      vaultRecordId: null,
      writeVerificationStatus: null,
      writeVerificationMessage: null,
      createdAt: now,
      updatedAt: now,
    } satisfies LocalBatchItem;
  });
}

async function persistJob(job: LocalBatchJob) {
  try {
    saveError.value = "";
    await saveLocalBatchJob(job);
  } catch (error: any) {
    saveError.value = error?.message ?? String(error);
  }
}

async function setAndPersistJob(job: LocalBatchJob) {
  batchJob.value = job;
  await persistJob(job);
}

async function createBatchJob(paths: string[]) {
  if (!paths.length) return;
  const now = currentIsoTime();
  const jobId = `batch-${Date.now()}`;
  await setAndPersistJob({
    id: jobId,
    status: "queued",
    createdAt: now,
    updatedAt: now,
    entitlementPlanCode: props.entitlementState?.planCode ?? "free",
    entitlementStatus: props.entitlementState?.status ?? "free",
    items: buildBatchItems(paths, jobId, now),
  });
}

async function pickBatchFiles() {
  if (!canUseBatch.value) return;
  if (isTauriRuntime()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Image / Audio",
          extensions: ["jpg", "jpeg", "png", "bmp", "webp", "tiff", "wav", "mp3", "aac", "flac", "ogg", "m4a"],
        },
      ],
    });
    if (Array.isArray(selected)) {
      createBatchJob(selected);
    } else if (typeof selected === "string") {
      createBatchJob([selected]);
    }
    return;
  }
  fileInputRef.value?.click();
}

function handleBrowserFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  createBatchJob(files.map((file) => file.name));
  input.value = "";
}

async function pauseJob() {
  if (!batchJob.value || batchJob.value.status !== "queued") return;
  await setAndPersistJob({ ...batchJob.value, status: "paused", updatedAt: currentIsoTime() });
}

async function resumeJob() {
  if (!batchJob.value || batchJob.value.status !== "paused") return;
  await setAndPersistJob({ ...batchJob.value, status: "queued", updatedAt: currentIsoTime() });
}

async function cancelJob() {
  const job = batchJob.value;
  if (!job || job.status === "cancelled") return;
  const now = currentIsoTime();
  if (activePipelineId.value) {
    const pipelineId = activePipelineId.value;
    await cancelPipeline(pipelineId).catch(() => undefined);
    rejectPipelineWaiter(pipelineId, new Error("队列已取消"));
  }
  await setAndPersistJob({
    ...job,
    status: "cancelled",
    updatedAt: now,
    items: job.items.map((item) =>
      item.status === "queued" || item.status === "running"
        ? { ...item, status: "cancelled", updatedAt: now }
        : item,
    ),
  });
}

async function retryFailedItems() {
  const job = batchJob.value;
  if (!job) return;
  const now = currentIsoTime();
  await setAndPersistJob({
    ...job,
    status: "queued",
    updatedAt: now,
    items: job.items.map((item) =>
      item.status === "failed"
        ? {
            ...item,
            status: "queued",
            attempts: item.attempts + 1,
            lastError: null,
            outputRef: null,
            vaultRecordId: null,
            writeVerificationStatus: null,
            writeVerificationMessage: null,
            updatedAt: now,
          }
        : item,
    ),
  });
}

function canRunMediaItem(item: LocalBatchItem) {
  return item.status === "queued";
}

function replaceBatchItem(job: LocalBatchJob, nextItem: LocalBatchItem): LocalBatchJob {
  return {
    ...job,
    updatedAt: currentIsoTime(),
    items: job.items.map((item) => (item.id === nextItem.id ? nextItem : item)),
  };
}

function nextQueuedMedia(job: LocalBatchJob) {
  return job.items.find((item) => item.mediaKind !== "unsupported" && canRunMediaItem(item)) ?? null;
}

function itemDetail(item: LocalBatchItem) {
  const hint = batchFriendlyHint(item);
  if (hint) return `${hint.title}。${hint.action}`;
  if (item.lastError) return item.lastError;
  if (item.writeVerificationMessage) return item.writeVerificationMessage;
  const details: Record<LocalBatchItemStatus, string> = {
    queued: "等待开始处理",
    running: "正在生成保护副本并验证",
    verified: "完成后验证已通过，版权记录已保存",
    failed: "处理失败，可重试",
    cancelled: "已取消",
  };
  return details[item.status];
}

function batchFriendlyHint(item: LocalBatchItem): { title: string; action: string } | null {
  const error = item.lastError ?? "";
  if (error.includes("短于 30 秒") || error.includes("audio_protection_min_duration")) {
    return {
      title: "音频时长不足 30 秒，未生成保护副本",
      action: "请选择 30 秒以上的完整音频作品后重试",
    };
  }
  if (error.includes("无法确认音频时长")) {
    return {
      title: "无法确认音频时长，未生成保护副本",
      action: "请更换可识别时长的完整音频文件后重试",
    };
  }
  if (error.includes("audio_protection_max_duration") || error.includes("超过 20 分钟")) {
    return {
      title: "音频超过 20 分钟上限，未生成保护副本",
      action: "请选择不超过 20 分钟的音频后重试",
    };
  }
  if (
    error.includes("audio_protection_file_size_limit_exceeded") ||
    error.includes("超过 512 MiB")
  ) {
    return {
      title: "音频超过 512 MiB 上限，未生成保护副本",
      action: "请选择不超过 512 MiB 的音频后重试",
    };
  }
  return null;
}

async function processQueuedMedia() {
  if (processingMedia.value || !canProcessMedia.value) return;
  processingMedia.value = true;
  try {
    await recoverInterruptedRunningItems();
    while (batchJob.value?.status === "queued") {
      const item = nextQueuedMedia(batchJob.value);
      if (!item) break;
      await processMediaItem(item);
    }
  } finally {
    processingMedia.value = false;
    activePipelineId.value = null;
  }
}

async function recoverInterruptedRunningItems() {
  const job = batchJob.value;
  if (!job) return;
  const interrupted = job.items.some((item) => item.status === "running");
  if (!interrupted) return;
  const now = currentIsoTime();
  await setAndPersistJob({
    ...job,
    updatedAt: now,
    items: job.items.map((item) =>
      item.status === "running"
        ? {
            ...item,
            status: "failed",
            lastError: "上次处理未完成，已转为可重试状态",
            writeVerificationStatus: null,
            writeVerificationMessage: null,
            updatedAt: now,
          }
        : item,
    ),
  });
}

async function processMediaItem(item: LocalBatchItem) {
  const job = batchJob.value;
  if (!job) return;
  const now = currentIsoTime();
  await setAndPersistJob(
    replaceBatchItem(job, {
      ...item,
      status: "running",
      lastError: null,
      outputRef: null,
      vaultRecordId: null,
      writeVerificationStatus: null,
      writeVerificationMessage: null,
      updatedAt: now,
    }),
  );

  try {
    trackFeatureEvent("local_batch_item", "start", {
      mediaType: item.mediaKind === "image" || item.mediaKind === "audio" ? item.mediaKind : "unknown",
      entitlementStatus: props.entitlementState?.status,
      source: "local_batch",
    });
    const started = await startPipeline(item.inputRef, [], {
      aspectStrategy: "letterbox",
      encodingMode: "high_quality_cpu",
      allowRewrite: false,
    });
    activePipelineId.value = started.pipelineId;
    const payload = await waitForPipeline(started.pipelineId);
    const verification = payload.writeVerification;
    const verified = verification?.verified === true;
    const latestJob = batchJob.value;
    if (!latestJob || latestJob.status === "cancelled") return;
    await setAndPersistJob(
      replaceBatchItem(latestJob, {
        ...item,
        status: verified ? "verified" : "failed",
        attempts: item.attempts + 1,
        lastError: verified ? null : verification?.message ?? "完成后验证未通过",
        outputRef: payload.outputs[0]?.path ?? null,
        vaultRecordId: payload.vaultRecord.id,
        writeVerificationStatus: verified ? "verified" : "failed",
        writeVerificationMessage: verification?.message ?? null,
        updatedAt: currentIsoTime(),
      }),
    );
    trackFeatureEvent("local_batch_item", verified ? "success" : "failure", {
      mediaType: item.mediaKind === "image" || item.mediaKind === "audio" ? item.mediaKind : "unknown",
      durationMs: payload.processTimeMs,
      entitlementStatus: props.entitlementState?.status,
      source: verified ? "verified" : "verification_failed",
      errorCode: verified ? undefined : "write_verification_failed",
    });
  } catch (error: any) {
    const latestJob = batchJob.value;
    if (!latestJob || latestJob.status === "cancelled") return;
    await setAndPersistJob(
      replaceBatchItem(latestJob, {
        ...item,
        status: "failed",
        attempts: item.attempts + 1,
        lastError: friendlyBatchError(error),
        outputRef: null,
        vaultRecordId: null,
        writeVerificationStatus: null,
        writeVerificationMessage: null,
        updatedAt: currentIsoTime(),
      }),
    );
    trackFeatureEvent("local_batch_item", "failure", {
      mediaType: item.mediaKind === "image" || item.mediaKind === "audio" ? item.mediaKind : "unknown",
      entitlementStatus: props.entitlementState?.status,
      source: "local_batch",
      errorCode: "batch_item_failed",
    });
  } finally {
    activePipelineId.value = null;
  }
}

function waitForPipeline(pipelineId: string) {
  return new Promise<PipelineCompletePayload>((resolve, reject) => {
    pipelineWaiters.set(pipelineId, { resolve, reject });
  }).finally(() => {
    pipelineWaiters.delete(pipelineId);
  });
}

function handlePipelineComplete(payload: PipelineCompletePayload) {
  const waiter = pipelineWaiters.get(payload.pipelineId);
  if (waiter) {
    waiter.resolve(payload);
  }
}

function handlePipelineProgress(payload: PipelineProgressPayload) {
  if (!payload.stage.startsWith("失败：")) return;
  rejectPipelineWaiter(payload.pipelineId, new Error(payload.stage.replace(/^失败：/, "")));
}

function rejectPipelineWaiter(pipelineId: string, error: Error) {
  const waiter = pipelineWaiters.get(pipelineId);
  if (waiter) {
    waiter.reject(error);
  }
}

function friendlyBatchError(error: any) {
  const message = error?.message ?? String(error);
  if (message.includes("文件不存在") || message.includes("No such file")) {
    return "文件不存在或已被移动";
  }
  return message;
}

onMounted(async () => {
  loadingJobs.value = true;
  try {
    unlistenProgress = await listenPipelineProgress(handlePipelineProgress);
    unlistenComplete = await listenPipelineComplete(handlePipelineComplete);
    const jobs = await listLocalBatchJobs();
    batchJob.value = jobs[0] ?? null;
  } finally {
    loadingJobs.value = false;
  }
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenComplete?.();
  for (const waiter of pipelineWaiters.values()) {
    waiter.reject(new Error("批量页面已关闭"));
  }
  pipelineWaiters.clear();
});
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">图片 / 音频年度权益</p>
        <h2>批量队列</h2>
        <p class="hero-card__copy">适合连续处理图片和 30 秒以上音频作品。本地执行，不按次数扣点。</p>
      </div>
      <div class="hero-card__stats">
        <div>
          <span>当前方案</span>
          <strong>{{ currentPlan }}</strong>
        </div>
        <div>
          <span>批量权限</span>
          <strong>{{ canUseBatch ? "年费期内已开放" : "图片 / 音频年费激活后开放" }}</strong>
        </div>
      </div>
    </section>

    <section v-if="!canUseBatch" class="panel batch-gate">
      <div>
        <p class="eyebrow">年度授权门槛</p>
        <h3>未付费可使用单文件写入，批量队列需激活图片 / 音频年费</h3>
        <p>
          批量队列会创建本地任务队列、持续写入保护副本，并在完成后逐个验证版权编号。
          未付费状态不进入批量文件选择，也不会创建批量队列。
        </p>
      </div>
      <button class="primary-button" type="button" @click="emit('openSubscription')">查看年度基础权益</button>
    </section>

    <section v-else class="batch-command-layout">
      <div class="panel batch-command-panel">
        <div class="panel__header">
          <div>
            <h3>创建批量队列</h3>
            <p>选择图片或音频，统一生成保护副本。</p>
          </div>
          <span class="pill">年费已激活</span>
        </div>

        <input
          v-if="!isTauriRuntime()"
          ref="fileInputRef"
          class="sr-only"
          type="file"
          multiple
          accept=".jpg,.jpeg,.png,.bmp,.webp,.tiff,.wav,.mp3,.aac,.flac,.ogg,.m4a"
          @change="handleBrowserFiles"
        />

        <div class="batch-dropzone" role="button" tabindex="0" @click="pickBatchFiles" @keydown.enter="pickBatchFiles">
          <strong>{{ batchJob ? "重新选择文件" : "选择批量文件" }}</strong>
          <span>支持图片和音频。音频需满足 30 秒以上规则。</span>
        </div>

        <div v-if="saveError" class="batch-save-error">
          队列保存失败：{{ saveError }}
        </div>

        <div class="action-row">
          <button class="primary-button" type="button" @click="pickBatchFiles">创建队列</button>
          <button class="primary-button" type="button" :disabled="!canProcessMedia || processingMedia" @click="processQueuedMedia">
            {{ processingMedia ? "正在处理队列" : "开始处理队列" }}
          </button>
          <button v-if="batchJob?.status === 'paused'" class="ghost-button" type="button" @click="resumeJob">继续队列</button>
          <button v-else class="ghost-button" type="button" :disabled="!canPause" @click="pauseJob">暂停全部</button>
          <button class="ghost-button" type="button" :disabled="!batchJob || batchJob.status === 'cancelled'" @click="cancelJob">取消队列</button>
        </div>
      </div>

      <div class="panel batch-queue-panel">
        <div class="panel__header">
          <div>
            <h3>{{ batchJob ? "当前队列" : "队列规则" }}</h3>
            <p>{{ batchJob ? "按文件展示阶段、结果和恢复动作。" : "选择文件后会创建本地队列。" }}</p>
          </div>
        </div>

        <div v-if="loadingJobs" class="batch-stage-list">
          <article class="batch-stage-card">
            <strong>正在读取队列</strong>
            <span>从本地版权库恢复未完成的批量队列任务。</span>
          </article>
        </div>

        <div v-else-if="!batchJob" class="batch-stage-list">
          <article v-for="stage in queueStages" :key="stage.label" class="batch-stage-card">
            <strong>{{ stage.label }}</strong>
            <span>{{ stage.detail }}</span>
          </article>
        </div>

        <div v-else class="batch-queue">
          <div class="batch-summary">
            <div>
              <span>队列状态</span>
              <strong>{{ jobStatusLabel(batchJob.status) }}</strong>
            </div>
            <div>
              <span>总数</span>
              <strong>{{ totalItems }}</strong>
            </div>
            <div>
              <span>待处理</span>
              <strong>{{ queuedItems }}</strong>
            </div>
            <div>
              <span>写入中</span>
              <strong>{{ runningItems }}</strong>
            </div>
            <div>
              <span>已验证</span>
              <strong>{{ verifiedItems }}</strong>
            </div>
            <div>
              <span>需处理</span>
              <strong>{{ failedItems }}</strong>
            </div>
            <div>
              <span>已取消</span>
              <strong>{{ cancelledItems }}</strong>
            </div>
          </div>

          <div v-if="canRetryFailed" class="batch-retry-row">
            <span>有文件需要处理，可重新放回队列。</span>
            <button class="ghost-button" type="button" @click="retryFailedItems">重试失败项</button>
          </div>

          <article v-for="item in batchJob.items" :key="item.id" class="batch-item-card" :class="`batch-item-card--${item.status}`">
            <div>
              <strong>{{ item.fileName }}</strong>
              <span>{{ item.mediaKind === "image" ? "图片" : item.mediaKind === "audio" ? "音频" : "不支持" }}</span>
            </div>
            <div>
              <span>{{ statusLabel(item.status) }}</span>
              <template v-if="batchFriendlyHint(item)">
                <small class="batch-friendly-hint">
                  <b>{{ batchFriendlyHint(item)?.title }}</b>
                  {{ batchFriendlyHint(item)?.action }}
                </small>
              </template>
              <small v-else>{{ itemDetail(item) }}</small>
            </div>
          </article>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.batch-gate {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 20px;
  align-items: center;
}

.batch-command-layout {
  display: grid;
  grid-template-columns: minmax(280px, 0.38fr) minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.batch-command-panel {
  position: sticky;
  top: 0;
}

.batch-queue-panel {
  min-width: 0;
}

.batch-gate h3 {
  margin: 0 0 8px;
}

.batch-gate p {
  margin: 0;
  max-width: 760px;
  color: var(--hs-text-muted);
}

.batch-dropzone {
  display: grid;
  gap: 8px;
  padding: 28px;
  border-radius: var(--hs-radius-card);
  border: 1.5px dashed rgba(114, 214, 202, 0.28);
  background: var(--hs-surface-muted);
  color: var(--hs-text-muted);
}

.batch-dropzone strong {
  color: var(--hs-text);
  font-size: 1.05rem;
}

.batch-stage-list {
  display: grid;
  gap: 12px;
}

.batch-stage-card {
  display: grid;
  gap: 4px;
  padding: 16px;
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
}

.batch-stage-card span {
  color: var(--hs-text-muted);
}

.batch-queue {
  display: grid;
  gap: 12px;
}

.batch-summary {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 10px;
}

.batch-summary div,
.batch-item-card,
.batch-retry-row {
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-raised);
}

.batch-summary div {
  padding: 12px;
}

.batch-summary span,
.batch-item-card span,
.batch-item-card small,
.batch-retry-row {
  color: var(--hs-text-muted);
}

.batch-summary strong {
  display: block;
  margin-top: 4px;
}

.batch-retry-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
}

.batch-save-error {
  margin-top: 12px;
  padding: 12px;
  border-radius: var(--hs-radius-card);
  border: 1px solid rgba(255, 200, 87, 0.32);
  background: var(--hs-warning-surface);
  color: var(--hs-warning);
}

.batch-item-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.42fr);
  gap: 12px;
  padding: 14px;
}

.batch-item-card strong,
.batch-item-card span,
.batch-item-card small {
  display: block;
}

.batch-item-card--failed {
  border-color: rgba(198, 91, 32, 0.28);
  background: rgba(198, 91, 32, 0.08);
}

.batch-friendly-hint {
  display: grid;
  gap: 4px;
}

.batch-friendly-hint b {
  color: var(--hs-warning);
}

.batch-item-card--cancelled {
  opacity: 0.72;
}

@media (max-width: 720px) {
  .batch-command-layout {
    grid-template-columns: 1fr;
  }

  .batch-command-panel {
    position: static;
  }

  .batch-gate {
    grid-template-columns: 1fr;
  }

  .batch-summary,
  .batch-item-card {
    grid-template-columns: 1fr;
  }
}
</style>
