<script setup lang="ts">
import { ref } from "vue";
import DropZone from "../components/DropZone.vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import { trackClick, trackFeatureEvent } from "../lib/analytics";
import {
  buildVerificationSummary,
  getTsaVerificationLabel,
  flushAnonymousFeedbackQueue,
  verifySuspect,
  type VerificationResult,
} from "../lib/tauri-api";

const emit = defineEmits<{ switchTab: [tab: "vault"] }>();

const suspectPath = ref("");
const suspectName = ref("");
const result = ref<VerificationResult | null>(null);
const loading = ref(false);
const errorMsg = ref("");
const diagnosticMsg = ref("");

async function handleFileSelect(path: string) {
  suspectPath.value = path;
  suspectName.value = path.split(/[\\/]/).pop() ?? path;

  // 选择文件后自动执行取证
  await handleVerify();
}

async function handleVerify() {
  if (!suspectPath.value) return;
  loading.value = true;
  errorMsg.value = "";
  diagnosticMsg.value = "";
  result.value = null;
  trackFeatureEvent("verify_suspect", "start", { mediaType: "unknown", source: "dropzone" });

  try {
    result.value = await verifySuspect(suspectPath.value);
    trackFeatureEvent("verify_suspect", "success", {
      mediaType: "unknown",
      source: result.value.matched ? "matched" : "unmatched",
    });
  } catch (err: any) {
    errorMsg.value = err?.message ?? String(err);
    trackFeatureEvent("verify_suspect", "failure", {
      mediaType: "unknown",
      errorCode: "verify_failed",
      source: "command_error",
    });
  } finally {
    loading.value = false;
  }
}

async function handleSendDiagnostic() {
  trackClick("verify_send_diagnostic_click");
  diagnosticMsg.value = "";
  const response = await flushAnonymousFeedbackQueue();
  diagnosticMsg.value = response.message;
}

async function handleCopySummary() {
  if (!result.value) return;
  const text = buildVerificationSummary(result.value, suspectPath.value);
  await navigator.clipboard.writeText(text);
}

function handleReset() {
  suspectPath.value = "";
  suspectName.value = "";
  result.value = null;
  errorMsg.value = "";
  diagnosticMsg.value = "";
}

function getConfidenceClass(confidence: number) {
  if (confidence >= 0.95) return "verify-result--match";
  if (confidence >= 0.5) return "verify-result--warn";
  return "verify-result--miss";
}

function getUnmatchedReason(confidence: number): string {
  if (confidence < 0.1) return "该文件可能非本机处理的作品";
  if (confidence < 0.5) return "该文件可能经过深度篡改（重编码、裁剪、音轨替换等）";
  return "";
}
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">取证</p>
        <h2>取证</h2>
      </div>
    </section>

    <section class="panel verify-panel">
      <div class="panel__header">
        <div>
          <h3>导入文件</h3>
        </div>
        <button
          v-if="result || errorMsg"
          class="ghost-button"
          type="button"
          @click="handleReset"
        >
          重新选择
        </button>
      </div>

      <DropZone
        :selected-path="suspectPath"
        :source-name="suspectName"
        :disabled="loading"
        @select="handleFileSelect"
      />

      <!-- Loading state -->
      <div v-if="loading" class="verify-loading">
        <span class="verify-loading__spinner" aria-hidden="true"></span>
        <span>识别中...</span>
      </div>

      <div v-if="errorMsg" class="verify-result verify-result--error">
        <strong>识别失败</strong>
        <p>{{ errorMsg }}</p>
        <button class="ghost-button" type="button" @click="handleSendDiagnostic">
          发送反馈
        </button>
        <p v-if="diagnosticMsg" class="verify-result__confidence">{{ diagnosticMsg }}</p>
      </div>

      <!-- Result display with confidence-based styling -->
      <div v-if="result" class="verify-result" :class="getConfidenceClass(result.confidence)">
        <!-- Hit (>= 0.95) -->
        <template v-if="result.matched">
          <strong>✅ 已命中</strong>
          <p>{{ result.summary }}</p>
        </template>
        <!-- Valid watermark but asset binding mismatch -->
        <template v-else-if="result.confidence >= 0.95">
          <strong>⚠️ 已识别，但未完成绑定</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__confidence">置信度 {{ Math.round(result.confidence * 100) }}%</p>
        </template>
        <!-- Suspect (0.5 ~ 0.95) -->
        <template v-else-if="result.confidence >= 0.5">
          <strong>⚠️ 疑似命中</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__confidence">置信度 {{ Math.round(result.confidence * 100) }}%</p>
        </template>
        <!-- Miss (< 0.5) -->
        <template v-else>
          <strong>❌ 未命中</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__reason">{{ result.reasonDetail || getUnmatchedReason(result.confidence) }}</p>
        </template>

        <div v-if="result.reasonDetail" class="verify-reason">
          <span>判断依据</span>
          <p>{{ result.reasonDetail }}</p>
        </div>

        <div class="verify-result__meta">
          <span>置信度 {{ Math.round(result.confidence * 100) }}%</span>
          <span v-if="result.watermarkUid">版权编号 {{ result.watermarkUid }}</span>
          <span v-if="result.matchedRecord">第 {{ result.matchedRecord.revision }} 次写入</span>
        </div>
      </div>

      <!-- Matched record card -->
      <div v-if="result && result.matchedRecord && result.confidence >= 0.95" class="verify-matched">
        <CopyrightCard :record="result.matchedRecord" />

        <div
          v-if="result.matchedRecord.revision > 1 || result.matchedRecord.parentWatermarkUid || result.matchedRecord.rewriteReason"
          class="verify-lineage"
        >
          <strong>写入记录</strong>
          <div class="verify-lineage__row">
            <span>写入次数</span>
            <b>第 {{ result.matchedRecord.revision }} 次写入</b>
          </div>
          <div v-if="result.matchedRecord.parentWatermarkUid" class="verify-lineage__row">
            <span>上一版编号</span>
            <b>{{ result.matchedRecord.parentWatermarkUid }}</b>
          </div>
          <div v-if="result.matchedRecord.rewriteReason" class="verify-lineage__row">
            <span>新版原因</span>
            <b>{{ result.matchedRecord.rewriteReason }}</b>
          </div>
        </div>

        <!-- TSA attestation badge -->
        <div v-if="result.tsaTokenPresent || result.networkTime" class="verify-tsa">
          <strong>时间信息</strong>
          <p v-if="result.tsaTokenPresent && result.tsaTokenVerified">
            {{ getTsaVerificationLabel(result.tsaVerificationPath) ?? "时间回执已复验" }}
          </p>
          <p v-else-if="result.tsaTokenPresent">
            时间回执已获取
          </p>
          <p v-if="result.networkTime">网络时间: {{ new Date(result.networkTime).toLocaleString() }}</p>
          <p v-if="result.createdAt">存证时间: {{ new Date(result.createdAt).toLocaleString() }}</p>
        </div>

        <button class="ghost-button" type="button" @click="emit('switchTab', 'vault')">
          查看版权库
        </button>
      </div>

      <!-- Actions -->
      <div v-if="result" class="verify-actions">
        <button class="primary-button" type="button" @click="handleCopySummary">
          复制报告
        </button>
        <button
          v-if="!result.matched"
          class="ghost-button"
          type="button"
          @click="handleSendDiagnostic"
        >
          发送反馈
        </button>
        <ProBadge label="PDF 报告" :disabled="true" />
      </div>

      <p v-if="diagnosticMsg && !errorMsg" class="verify-diagnostic">{{ diagnosticMsg }}</p>

      <div v-if="result" class="verify-disclaimer">
        <details>
          <summary>免责声明</summary>
          <p>{{ result.disclaimer }}</p>
        </details>
      </div>
    </section>
  </div>
</template>

<style scoped>
.verify-diagnostic {
  margin-top: 0.75rem;
  font-size: 0.85rem;
  color: var(--text-muted, #8fb8ff);
}

.verify-lineage {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border: 1px solid rgba(87, 143, 202, 0.28);
  border-radius: 10px;
  background: rgba(87, 143, 202, 0.08);
}

.verify-reason {
  margin-top: 0.85rem;
  padding: 0.8rem;
  border: 1px solid rgba(255, 200, 87, 0.22);
  border-radius: 10px;
  background: rgba(255, 200, 87, 0.08);
}

.verify-reason span {
  display: block;
  color: var(--text-muted, #8b95a7);
  font-size: 0.78rem;
}

.verify-reason strong {
  display: block;
  margin-top: 0.2rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow-wrap: anywhere;
}

.verify-reason p {
  margin: 0.45rem 0 0;
}

.verify-lineage strong {
  display: block;
  margin-bottom: 0.55rem;
}

.verify-lineage__row {
  display: grid;
  grid-template-columns: 6rem 1fr;
  gap: 0.75rem;
  font-size: 0.86rem;
  line-height: 1.5;
}

.verify-lineage__row span {
  color: var(--text-muted, #8b95a7);
}

.verify-lineage__row b {
  color: var(--text-primary, #e0e0e0);
  word-break: break-all;
}
</style>
