<script setup lang="ts">
import { buildCopyrightSummary, type VaultRecord } from "../lib/tauri-api";

const props = defineProps<{
  record: VaultRecord;
  highlight?: boolean;
}>();

const platforms: string[] = [];
if (props.record.outputDouyin) platforms.push("抖音");
if (props.record.outputBilibili) platforms.push("B站");
if (props.record.outputXhs) platforms.push("小红书");

function formatTrainingPermission(permission?: string | null): string {
  const map: Record<string, string> = {
    prohibited: "🚫 完全禁止",
    non_commercial: "🎓 仅非商业",
    commercial: "💼 允许商业",
    public_domain: "🌍 公共领域",
  };
  return permission ? map[permission] || permission : "—";
}

async function handleCopy() {
  const text = buildCopyrightSummary(props.record);
  await navigator.clipboard.writeText(text);
}
</script>

<template>
  <div class="copyright-card" :class="{ 'copyright-card--highlight': highlight }">
    <div class="copyright-card__badge">
      <span>🛡️ 版权存证</span>
      <span v-if="highlight" class="copyright-card__new">新增</span>
      <span v-if="record.isAiGenerated" class="copyright-card__ai-badge">🤖 AI生成</span>
    </div>
    <div class="copyright-card__body">
      <div class="copyright-card__row">
        <span>水印 UID</span>
        <strong>{{ record.watermarkUid }}</strong>
      </div>
      <div v-if="record.revision > 1 || record.parentWatermarkUid" class="copyright-card__row">
        <span>写入版本</span>
        <strong>第 {{ record.revision }} 次</strong>
      </div>
      <div v-if="record.parentWatermarkUid" class="copyright-card__row">
        <span>父级 UID</span>
        <strong>{{ record.parentWatermarkUid }}</strong>
      </div>
      <div v-if="record.rewriteReason" class="copyright-card__row">
        <span>重写原因</span>
        <strong>{{ record.rewriteReason }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>处理时间</span>
        <strong>{{ record.createdAt }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>SHA-256</span>
        <strong class="hash-text">{{ record.originalHash.slice(0, 16) }}...</strong>
      </div>
      <div class="copyright-card__row">
        <span>输出平台</span>
        <strong>{{ platforms.join('、') || '—' }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>文件名</span>
        <strong>{{ record.fileName }}</strong>
      </div>
      <div v-if="record.isAiGenerated" class="copyright-card__row">
        <span>AI训练许可</span>
        <strong>{{ formatTrainingPermission(record.aiTrainingPermission) }}</strong>
      </div>
    </div>
    <button class="ghost-button copyright-card__copy" type="button" @click="handleCopy">
      复制存证摘要
    </button>
  </div>
</template>

<style scoped>
.copyright-card__ai-badge {
  padding: 4px 8px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
}
</style>
