<script setup lang="ts">
import { computed } from "vue";
import CopyrightCard from "./CopyrightCard.vue";
import {
  openOutputDir,
  type PipelineCompletePayload,
  type SourceMeta,
} from "../lib/tauri-api";

const props = defineProps<{
  payload: PipelineCompletePayload;
  sourceMeta: SourceMeta;
}>();

const emit = defineEmits<{ back: [] }>();

const processTimeFormatted = computed(() => {
  const ms = props.payload.processTimeMs;
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
});

/** Detect if output is significantly larger than source (file size inflation). */
const sizeInflated = computed(() => {
  const maxOutputMb = Math.max(...props.payload.outputs.map(o => o.sizeMb));
  return maxOutputMb > props.sourceMeta.fileSizeMb * 1.5;
});

const hasRewriteLineage = computed(() =>
  props.payload.vaultRecord.revision > 1 ||
  Boolean(props.payload.vaultRecord.parentWatermarkUid) ||
  Boolean(props.payload.vaultRecord.rewriteReason),
);

async function handleOpenDir() {
  const firstOutput = props.payload.outputs[0];
  if (!firstOutput) return;
  const dir = firstOutput.path.replace(/[\\/][^\\/]+$/, "");
  await openOutputDir(dir);
}

async function handleCopyPath() {
  const paths = props.payload.outputs.map(o => o.path).join("\n");
  await navigator.clipboard.writeText(paths);
}
</script>

<template>
  <div class="result-page">
    <section class="result-page__header">
      <h3>✅ 处理完成</h3>
      <p>版权编号：{{ payload.watermarkUid }}</p>
    </section>

    <section class="result-page__outputs">
      <div v-for="output in payload.outputs" :key="output.platform" class="result-page__output-card">
        <div class="result-page__output-header">
          <strong>{{ output.platform === 'douyin' ? '抖音' : output.platform === 'bilibili' ? 'B站' : output.platform === 'xiaohongshu' ? '小红书' : output.platform }}</strong>
        </div>
        <div class="result-page__output-meta">
          <span>{{ output.resolution }}</span>
          <span v-if="output.fps">{{ output.fps }}fps</span>
          <span>{{ output.sizeMb }} MB</span>
        </div>
        <div class="result-page__output-path">{{ output.path }}</div>
      </div>
    </section>

    <section class="result-page__compare">
      <h4>处理结果</h4>
      <table class="result-page__table">
        <thead>
          <tr><th></th><th>源文件</th><th>输出</th></tr>
        </thead>
        <tbody>
          <tr>
            <td>分辨率</td>
            <td>{{ sourceMeta.width }}x{{ sourceMeta.height }}</td>
            <td>{{ payload.outputs.map(o => o.resolution).join(' / ') }}</td>
          </tr>
          <tr>
            <td>大小</td>
            <td>{{ sourceMeta.fileSizeMb }} MB</td>
            <td>{{ payload.outputs.map(o => `${o.sizeMb} MB`).join(' / ') }}</td>
          </tr>
          <tr>
            <td>帧率</td>
            <td>{{ sourceMeta.fps }}fps</td>
            <td>{{ payload.outputs.map(o => `${o.fps}fps`).join(' / ') }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- File size inflation notice -->
    <section v-if="sizeInflated" class="result-page__size-notice" role="note">
      <span class="result-page__size-notice-icon">ℹ️</span>
      <span>文件偏大，当前更偏向画质。</span>
    </section>

    <section class="result-page__info">
      <span>耗时 {{ processTimeFormatted }}</span>
    </section>

    <section v-if="payload.writeVerification" class="result-page__verification">
      <strong>写入验收</strong>
      <p>{{ payload.writeVerification.message }}</p>
      <div class="result-page__lineage-grid">
        <span>版权编号</span>
        <b>{{ payload.writeVerification.watermarkUid }}</b>
        <span>写入次数</span>
        <b>第 {{ payload.writeVerification.revision }} 次写入</b>
      </div>
    </section>

    <section v-if="hasRewriteLineage" class="result-page__lineage">
      <strong>写入记录</strong>
      <div class="result-page__lineage-grid">
        <span>写入次数</span>
        <b>第 {{ payload.vaultRecord.revision }} 次写入</b>
        <span>版权编号</span>
        <b>{{ payload.vaultRecord.watermarkUid }}</b>
        <span v-if="payload.vaultRecord.parentWatermarkUid">上一版编号</span>
        <b v-if="payload.vaultRecord.parentWatermarkUid">{{ payload.vaultRecord.parentWatermarkUid }}</b>
        <span v-if="payload.vaultRecord.rewriteReason">新版原因</span>
        <b v-if="payload.vaultRecord.rewriteReason">{{ payload.vaultRecord.rewriteReason }}</b>
      </div>
    </section>

    <CopyrightCard :record="payload.vaultRecord" highlight />

    <section class="result-page__actions">
      <button class="primary-button" type="button" @click="handleOpenDir">打开输出目录</button>
      <button class="ghost-button" type="button" @click="handleCopyPath">复制文件路径</button>
      <button class="ghost-button" type="button" @click="emit('back')">返回工作台</button>
    </section>
  </div>
</template>

<style scoped>
.result-page__size-notice {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: #2a2a1a;
  border: 1px solid #4a4a2a;
  border-radius: 8px;
  font-size: 0.85rem;
  color: #e0d8a0;
  line-height: 1.5;
}
.result-page__size-notice-icon {
  flex-shrink: 0;
}

.result-page__lineage {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(87, 143, 202, 0.28);
  border-radius: 10px;
  background: rgba(87, 143, 202, 0.08);
}

.result-page__verification {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(89, 210, 194, 0.32);
  border-radius: 10px;
  background: rgba(89, 210, 194, 0.1);
}

.result-page__verification strong {
  display: block;
  margin-bottom: 0.45rem;
}

.result-page__verification p {
  margin: 0 0 0.65rem;
}

.result-page__lineage strong {
  display: block;
  margin-bottom: 0.6rem;
}

.result-page__lineage-grid {
  display: grid;
  grid-template-columns: minmax(90px, 0.35fr) minmax(0, 1fr);
  gap: 0.45rem 0.75rem;
  font-size: 0.9rem;
}

.result-page__lineage-grid span {
  color: var(--text-muted, #8b95a7);
}

.result-page__lineage-grid b {
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
