<script setup lang="ts">
import { computed } from "vue";
import CopyrightCard from "./CopyrightCard.vue";
import {
  formatPayloadAuthStatus,
  formatRegistryStatus,
  formatWatermarkIssueMode,
  openOutputDir,
  type PipelineCompletePayload,
  type SourceMeta,
} from "../lib/tauri-api";

const props = defineProps<{
  payload: PipelineCompletePayload;
  sourceMeta: SourceMeta;
}>();

const emit = defineEmits<{ back: []; openVault: [] }>();

const isAudio = computed(() => props.sourceMeta.fileType === "audio");

const processTimeFormatted = computed(() => {
  const ms = props.payload.processTimeMs;
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
});

function formatAudioDuration(durationSecs: number): string {
  if (!Number.isFinite(durationSecs) || durationSecs <= 0) return "未确认";
  const totalSeconds = Math.round(durationSecs);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}

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

const outputLocation = computed(() =>
  props.payload.outputs.map((output) => output.path).join("\n"),
);

const verificationMessage = computed(() => {
  const verification = props.payload.writeVerification;
  if (!verification) return "";
  if (verification.verified) {
    return "已从保护副本回读并验证版权编号，可由 HiddenShield 再次读取验证";
  }
  return verification.message.trim();
});

const registryReceipt = computed(() => {
  const record = props.payload.vaultRecord;
  const confirmed = ["server_confirmed", "offline_confirmed"].includes(
    record.watermarkIdRegistryStatus,
  );
  return confirmed ? record.watermarkIdRegistryReceipt?.trim() || "" : "";
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
      <h3>保护副本已生成</h3>
      <p>版权编号：{{ payload.watermarkUid }}</p>
    </section>

    <section class="result-page__outputs">
      <div v-for="output in payload.outputs" :key="output.platform" class="result-page__output-card">
        <div class="result-page__output-header">
          <strong>保护副本</strong>
        </div>
        <div class="result-page__output-meta">
          <span v-if="isAudio">音频时长 {{ formatAudioDuration(sourceMeta.durationSecs) }}</span>
          <span v-if="!isAudio">{{ output.resolution }}</span>
          <span v-if="!isAudio && output.fps">{{ output.fps }}fps</span>
          <span>{{ output.sizeMb }} MB</span>
        </div>
        <div class="result-page__output-path">{{ output.path }}</div>
      </div>
    </section>

    <section class="result-page__compare">
      <h4>处理结果</h4>
      <table class="result-page__table">
        <thead>
          <tr><th></th><th>源文件</th><th>保护副本</th></tr>
        </thead>
        <tbody>
          <tr v-if="isAudio">
            <td>音频时长</td>
            <td>{{ formatAudioDuration(sourceMeta.durationSecs) }}</td>
            <td>{{ formatAudioDuration(sourceMeta.durationSecs) }}</td>
          </tr>
          <tr v-if="!isAudio">
            <td>分辨率</td>
            <td>{{ sourceMeta.width }}x{{ sourceMeta.height }}</td>
            <td>{{ payload.outputs.map(o => o.resolution).join(' / ') }}</td>
          </tr>
          <tr>
            <td>大小</td>
            <td>{{ sourceMeta.fileSizeMb }} MB</td>
            <td>{{ payload.outputs.map(o => `${o.sizeMb} MB`).join(' / ') }}</td>
          </tr>
          <tr v-if="sourceMeta.fileType === 'video'">
            <td>帧率</td>
            <td>{{ sourceMeta.fps }}fps</td>
            <td>{{ payload.outputs.map(o => `${o.fps}fps`).join(' / ') }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- File size inflation notice -->
    <section v-if="sizeInflated" class="result-page__size-notice" role="note">
      <span class="result-page__size-notice-icon" aria-hidden="true">i</span>
      <span v-if="isAudio">保护副本采用可稳定验证的音频格式，因此文件可能明显增大。</span>
      <span v-else>文件偏大，当前更偏向画质。</span>
    </section>

    <section class="result-page__info">
      <span>耗时 {{ processTimeFormatted }}</span>
    </section>

    <section class="result-page__evidence">
      <h4>存证摘要</h4>
      <div class="result-page__lineage-grid">
        <span>处理耗时</span>
        <b>{{ processTimeFormatted }}</b>
        <span>保护副本位置</span>
        <b>{{ outputLocation || '未记录' }}</b>
        <span>水印协议版本</span>
        <b>V{{ payload.vaultRecord.payloadProtocolVersion }}</b>
        <span>版权编号生成方式</span>
        <b>{{ formatWatermarkIssueMode(payload.vaultRecord.watermarkIdIssueMode) }}</b>
        <span>联网登记状态</span>
        <b>{{ formatRegistryStatus(payload.vaultRecord.watermarkIdRegistryStatus) }}</b>
        <template v-if="registryReceipt">
          <span>登记收据编号</span>
          <b>{{ registryReceipt }}</b>
        </template>
        <span>载荷完整性校验</span>
        <b>{{ formatPayloadAuthStatus(payload.vaultRecord.payloadAuthStatus) }}</b>
      </div>
    </section>

    <section v-if="payload.writeVerification" class="result-page__verification">
      <strong>保护副本验证</strong>
      <div
        class="result-page__verification-status"
        :class="{ 'result-page__verification-status--warn': !payload.writeVerification.verified }"
      >
        <span aria-hidden="true">{{ payload.writeVerification.verified ? '✓' : '!' }}</span>
        <b>{{ payload.writeVerification.verified ? '已通过' : '未通过' }}</b>
      </div>
      <div class="result-page__lineage-grid">
        <span>版权编号</span>
        <b>{{ payload.writeVerification.watermarkUid }}</b>
        <span>版本次数</span>
        <b>第 {{ payload.writeVerification.revision }} 次</b>
        <span>作品指纹</span>
        <b>{{ payload.vaultRecord.originalHash.slice(0, 16) }}...</b>
      </div>
      <p v-if="verificationMessage">{{ verificationMessage }}</p>
      <div v-if="!payload.writeVerification.verified" class="result-page__verification-recovery">
        <strong>建议处理</strong>
        <p>当前保护副本已经生成，但完成后验证没有通过。建议优先重新生成；如需排查，可打开保护副本位置或复制文件路径。</p>
        <div class="result-page__verification-actions">
          <button class="primary-button" type="button" @click="emit('back')">重新生成保护副本</button>
          <button class="ghost-button" type="button" @click="handleOpenDir">打开保护副本位置</button>
          <button class="ghost-button" type="button" @click="handleCopyPath">复制文件路径</button>
        </div>
        <details class="result-page__failure-reason">
          <summary>查看失败原因</summary>
          <p>{{ payload.writeVerification.message }}</p>
        </details>
      </div>
    </section>

    <section v-if="hasRewriteLineage" class="result-page__lineage">
      <details open>
        <summary>
          <strong>版本记录</strong>
          <span>第 {{ payload.vaultRecord.revision }} 次</span>
        </summary>
        <div class="result-page__lineage-grid">
          <span>版本次数</span>
          <b>第 {{ payload.vaultRecord.revision }} 次</b>
          <span>版权编号</span>
          <b>{{ payload.vaultRecord.watermarkUid }}</b>
          <span v-if="payload.vaultRecord.parentWatermarkUid">上一版编号</span>
          <b v-if="payload.vaultRecord.parentWatermarkUid">{{ payload.vaultRecord.parentWatermarkUid }}</b>
          <span v-if="payload.vaultRecord.rewriteReason">更新说明</span>
          <b v-if="payload.vaultRecord.rewriteReason">{{ payload.vaultRecord.rewriteReason }}</b>
        </div>
      </details>
    </section>

    <CopyrightCard :record="payload.vaultRecord" highlight />

    <section class="result-page__actions">
      <button class="primary-button" type="button" @click="handleOpenDir">打开保护副本位置</button>
      <button class="ghost-button" type="button" @click="emit('openVault')">查看版权库</button>
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
  background: var(--hs-warning-surface);
  border: 1px solid rgba(255, 200, 87, 0.32);
  border-radius: 8px;
  font-size: 0.85rem;
  color: var(--hs-warning);
  line-height: 1.5;
}
.result-page__size-notice-icon {
  flex-shrink: 0;
  display: inline-grid;
  place-items: center;
  width: 1.1rem;
  height: 1.1rem;
  border: 1px solid rgba(255, 200, 87, 0.38);
  border-radius: var(--hs-radius-card);
  font-size: 0.75rem;
  font-weight: 700;
}

.result-page__lineage {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(87, 143, 202, 0.28);
  border-radius: var(--hs-radius-card);
  background: rgba(87, 143, 202, 0.08);
}

.result-page__verification {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(89, 210, 194, 0.32);
  border-radius: var(--hs-radius-card);
  background: rgba(89, 210, 194, 0.1);
}

.result-page__evidence {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(87, 143, 202, 0.28);
  border-radius: var(--hs-radius-card);
  background: rgba(87, 143, 202, 0.08);
}

.result-page__evidence h4 {
  margin: 0 0 0.7rem;
}

.result-page__verification strong {
  display: block;
  margin-bottom: 0.45rem;
}

.result-page__verification p {
  margin: 0.65rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.88rem;
  line-height: 1.5;
}

.result-page__verification-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.65rem;
  color: var(--hs-accent);
}

.result-page__verification-status span {
  display: inline-grid;
  place-items: center;
  width: 1.2rem;
  height: 1.2rem;
  border-radius: var(--hs-radius-pill);
  background: rgba(89, 210, 194, 0.16);
  border: 1px solid rgba(89, 210, 194, 0.45);
  font-weight: 700;
  line-height: 1;
}

.result-page__verification-status--warn {
  color: var(--hs-warning);
}

.result-page__verification-status--warn span {
  background: rgba(255, 200, 87, 0.14);
  border-color: rgba(255, 200, 87, 0.45);
}

.result-page__verification-recovery {
  margin-top: 0.8rem;
  padding: 0.8rem;
  border-radius: 8px;
  border: 1px solid rgba(255, 200, 87, 0.28);
  background: rgba(255, 200, 87, 0.08);
}

.result-page__verification-recovery strong {
  margin-bottom: 0.35rem;
}

.result-page__verification-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  margin-top: 0.75rem;
}

.result-page__failure-reason {
  margin-top: 0.75rem;
  color: var(--hs-text-muted);
  font-size: 0.86rem;
}

.result-page__failure-reason summary {
  cursor: pointer;
}

.result-page__lineage summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  cursor: pointer;
  list-style: none;
}

.result-page__lineage summary::-webkit-details-marker {
  display: none;
}

.result-page__lineage summary::after {
  content: "展开";
  flex: none;
  color: var(--hs-text-muted);
  font-size: 0.78rem;
}

.result-page__lineage details[open] summary::after {
  content: "收起";
}

.result-page__lineage summary strong {
  display: block;
}

.result-page__lineage summary span {
  color: var(--hs-text-muted);
  font-size: 0.85rem;
}

.result-page__lineage details[open] .result-page__lineage-grid {
  margin-top: 0.7rem;
}

.result-page__lineage-grid {
  display: grid;
  grid-template-columns: minmax(90px, 0.35fr) minmax(0, 1fr);
  gap: 0.45rem 0.75rem;
  font-size: 0.9rem;
}

.result-page__lineage-grid span {
  color: var(--hs-text-muted);
}

.result-page__lineage-grid b {
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
