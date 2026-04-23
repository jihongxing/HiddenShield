<script setup lang="ts">
import { ref } from "vue";
import DropZone from "../components/DropZone.vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import {
  buildVerificationSummary,
  getTsaVerificationLabel,
  verifySuspect,
  type VerificationResult,
} from "../lib/tauri-api";

const emit = defineEmits<{ switchTab: [tab: "vault"] }>();

const suspectPath = ref("");
const suspectName = ref("");
const result = ref<VerificationResult | null>(null);
const loading = ref(false);
const errorMsg = ref("");

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
  result.value = null;

  try {
    result.value = await verifySuspect(suspectPath.value);
  } catch (err: any) {
    errorMsg.value = err?.message ?? String(err);
  } finally {
    loading.value = false;
  }
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
        <p class="eyebrow">Verify</p>
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
          <p class="verify-result__reason">{{ getUnmatchedReason(result.confidence) }}</p>
        </template>

        <div class="verify-result__meta">
          <span>置信度 {{ Math.round(result.confidence * 100) }}%</span>
          <span v-if="result.watermarkUid">UID {{ result.watermarkUid }}</span>
        </div>
      </div>

      <!-- Matched record card -->
        <div v-if="result && result.matchedRecord && result.confidence >= 0.95" class="verify-matched">
          <CopyrightCard :record="result.matchedRecord" />

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
        <ProBadge label="PDF 报告" :disabled="true" />
      </div>

      <div v-if="result" class="verify-disclaimer">
        <details>
          <summary>免责声明</summary>
          <p>{{ result.disclaimer }}</p>
        </details>
      </div>
    </section>
  </div>
</template>
