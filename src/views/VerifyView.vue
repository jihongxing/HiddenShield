<script setup lang="ts">
import { ref } from "vue";
import DropZone from "../components/DropZone.vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import {
  buildVerificationSummary,
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
        <h2>维权取证</h2>
        <p class="hero-card__copy">
          拖入疑似侵权文件，自动提取音频指纹并匹配本地金库。
        </p>
      </div>
    </section>

    <section class="panel verify-panel">
      <div class="panel__header">
        <div>
          <h3>疑似侵权文件</h3>
          <p>拖入或点击选择需要取证的文件</p>
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
        <span>正在提取水印指纹并匹配金库...</span>
      </div>

      <div v-if="errorMsg" class="verify-result verify-result--error">
        <strong>取证失败</strong>
        <p>{{ errorMsg }}</p>
      </div>

      <!-- Result display with confidence-based styling -->
      <div v-if="result" class="verify-result" :class="getConfidenceClass(result.confidence)">
        <!-- Hit (>= 0.95) -->
        <template v-if="result.confidence >= 0.95">
          <strong>✅ 已命中水印记录</strong>
          <p>{{ result.summary }}</p>
        </template>
        <!-- Suspect (0.5 ~ 0.95) -->
        <template v-else-if="result.confidence >= 0.5">
          <strong>⚠️ 疑似匹配</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__confidence">置信度 {{ Math.round(result.confidence * 100) }}%</p>
        </template>
        <!-- Miss (< 0.5) -->
        <template v-else>
          <strong>❌ 未检测到有效水印</strong>
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
        <div v-if="result.tsaVerified || result.networkTime" class="verify-tsa">
          <strong>🔐 可信时间证明</strong>
          <p v-if="result.tsaVerified">RFC 3161 时间戳已获取（{{ result.tsaSource }}）</p>
          <p v-if="result.networkTime">网络授时: {{ new Date(result.networkTime).toLocaleString() }}</p>
          <p v-if="result.createdAt">入库时间: {{ new Date(result.createdAt).toLocaleString() }}</p>
        </div>

        <button class="ghost-button" type="button" @click="emit('switchTab', 'vault')">
          跳转到版权库
        </button>
      </div>

      <!-- Actions -->
      <div v-if="result" class="verify-actions">
        <button class="primary-button" type="button" @click="handleCopySummary">
          📋 复制存证报告
        </button>
        <ProBadge label="导出 PDF 报告" :disabled="true" />
      </div>

      <div v-if="result" class="verify-disclaimer">
        <p>{{ result.disclaimer }}</p>
      </div>
    </section>
  </div>
</template>
