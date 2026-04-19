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
    </div>
    <div class="copyright-card__body">
      <div class="copyright-card__row">
        <span>水印 UID</span>
        <strong>{{ record.watermarkUid }}</strong>
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
    </div>
    <button class="ghost-button copyright-card__copy" type="button" @click="handleCopy">
      复制存证摘要
    </button>
  </div>
</template>
