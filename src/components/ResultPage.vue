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
      <p>版权保护已启用 ✓ — 水印 UID: {{ payload.watermarkUid }}</p>
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
      <h4>前后对比</h4>
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
          <tr>
            <td>色彩空间</td>
            <td>{{ sourceMeta.colorProfile }}</td>
            <td>BT.709 / SDR</td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- File size inflation notice -->
    <section v-if="sizeInflated" class="result-page__size-notice" role="note">
      <span class="result-page__size-notice-icon">ℹ️</span>
      <span>为了满足各平台最高画质标准，引擎为您重构了更高规格的码率，文件变大属于提升画质的正常现象。如需更小体积，可尝试"高质量 CPU"模式。</span>
    </section>

    <section class="result-page__info">
      <span>耗时 {{ processTimeFormatted }}</span>
      <span>编码器 {{ payload.encoderUsed }}</span>
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
</style>
