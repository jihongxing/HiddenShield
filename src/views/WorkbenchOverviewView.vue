<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  formatLocalDateTime,
  listLocalBatchJobs,
  listVaultRecords,
  type AppTab,
  type EntitlementState,
  type LocalBatchJob,
  type VaultRecord,
} from "../lib/tauri-api";

const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const emit = defineEmits<{
  navigate: [tab: AppTab];
}>();

const records = ref<VaultRecord[]>([]);
const jobs = ref<LocalBatchJob[]>([]);
const loading = ref(true);

const planName = computed(() =>
  props.entitlementState?.features?.batch_processing === true ? "图片 / 音频年费" : "未付费",
);
const recentRecords = computed(() => records.value.slice(0, 5));
const latestJob = computed(() => jobs.value[0] ?? null);
const verifiedRecords = computed(() =>
  records.value.filter((record) => record.writeVerificationStatus === "verified").length,
);
const batchItems = computed(() => jobs.value.flatMap((job) => job.items));
const failedBatchItems = computed(() => batchItems.value.filter((item) => item.status === "failed").length);
const activeBatchItems = computed(() =>
  batchItems.value.filter((item) => item.status === "queued" || item.status === "running").length,
);

onMounted(async () => {
  loading.value = true;
  try {
    const [vaultRecords, batchJobs] = await Promise.all([
      listVaultRecords(),
      listLocalBatchJobs(),
    ]);
    records.value = vaultRecords.filter((record) =>
      record.outputStrategy !== "video_audio_track" &&
      !record.videoNotaryId &&
      !record.videoVisualTaskId &&
      !record.videoVisualMediaHash
    );
    jobs.value = batchJobs;
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="workbench-overview">
    <section class="hs-stage-hero">
      <div>
        <span class="hs-stage-hero__eyebrow">今日主动作</span>
        <h2>从作品对象开始，而不是从功能按钮开始</h2>
        <p>选择图片或音频后，HiddenShield 会按预检、写入、完成后验证、版权库入库和报告状态组织下一步。</p>
      </div>
      <div class="hs-stage-hero__actions">
        <button class="primary-button" type="button" @click="emit('navigate', 'process')">开始处理</button>
        <button class="ghost-button" type="button" @click="emit('navigate', 'verify')">验证文件</button>
        <button class="ghost-button" type="button" @click="emit('navigate', 'vault')">查看版权库</button>
      </div>
    </section>

    <section class="hs-metric-grid">
      <article class="hs-metric-tile">
        <span>当前方案</span>
        <strong>{{ planName }}</strong>
        <small>正式报告需按记录单独购买</small>
      </article>
      <article class="hs-metric-tile">
        <span>版权记录</span>
        <strong>{{ records.length }}</strong>
        <small>完成后验证 {{ verifiedRecords }} 条</small>
      </article>
      <article class="hs-metric-tile">
        <span>批量队列</span>
        <strong>{{ jobs.length }}</strong>
        <small>进行中 {{ activeBatchItems }} / 失败 {{ failedBatchItems }}</small>
      </article>
      <article class="hs-metric-tile">
        <span>离线能力</span>
        <strong>图片 / 音频可用</strong>
        <small>无需联网即可读取与验证</small>
      </article>
    </section>

    <section class="hs-stage-section">
      <div class="hs-stage-section__header">
        <div>
          <h3>最近任务与记录</h3>
          <p>按对象展示最近版权记录，帮助你恢复下一步。</p>
        </div>
        <button class="ghost-button" type="button" @click="emit('navigate', 'vault')">全部记录</button>
      </div>

      <div v-if="loading" class="hs-empty-state">正在读取本地版权库与批量队列...</div>
      <div v-else-if="!recentRecords.length" class="hs-empty-state">
        还没有版权记录。先处理一份图片或音频，完成后会出现在这里。
      </div>
      <table v-else class="hs-record-table">
        <thead>
          <tr>
            <th>作品对象</th>
            <th>状态</th>
            <th>版权编号</th>
            <th>时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="record in recentRecords" :key="record.id" @click="emit('navigate', 'vault')">
            <td>
              <strong>{{ record.fileName }}</strong>
              <span>{{ record.resolution || "未记录分辨率" }}</span>
            </td>
            <td>
              <span class="hs-status-chip" :class="record.writeVerificationStatus === 'verified' ? 'hs-status-chip--ok' : 'hs-status-chip--warning'">
                {{ record.writeVerificationStatus === "verified" ? "已验证" : "需复核" }}
              </span>
            </td>
            <td class="hs-mono">{{ record.watermarkUid }}</td>
            <td>{{ formatLocalDateTime(record.createdAt) }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <section class="hs-stage-section">
      <div class="hs-stage-section__header">
        <div>
          <h3>批量队列摘要</h3>
          <p>图片 / 音频年费批量队列按失败恢复和完成后验证组织。</p>
        </div>
        <button class="ghost-button" type="button" @click="emit('navigate', 'batch')">打开队列</button>
      </div>
      <div v-if="latestJob" class="hs-queue-row">
        <div>
          <strong>最近队列 {{ latestJob.id }}</strong>
          <span>{{ latestJob.items.length }} 个项目 · {{ latestJob.status }}</span>
        </div>
        <div class="hs-queue-row__bar">
          <span :style="{ width: `${latestJob.items.length ? Math.round((latestJob.items.filter((item) => item.status === 'verified').length / latestJob.items.length) * 100) : 0}%` }"></span>
        </div>
      </div>
      <div v-else class="hs-empty-state">暂无批量队列。未付费不创建队列，年费激活后可本地批量处理图片和音频。</div>
    </section>
  </div>
</template>
