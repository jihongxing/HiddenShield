<script setup lang="ts">
import { computed, ref } from "vue";
import type { SystemCheckResult } from "../lib/tauri-api";

const props = defineProps<{ result: SystemCheckResult }>();
const expanded = ref(false);

const allGood = computed(() =>
  props.result.ffmpegAvailable &&
  props.result.diskSufficient &&
  props.result.outputDirWritable
);

const hasIssue = computed(() => !allGood.value);
</script>

<template>
  <div class="system-status" :class="{ 'system-status--ok': allGood, 'system-status--warn': hasIssue }">
    <div class="system-status__header" @click="expanded = !expanded">
      <span v-if="allGood" class="system-status__summary">环境就绪</span>
      <span v-else class="system-status__summary system-status__summary--warn">需处理</span>
      <span class="system-status__toggle">{{ expanded ? '收起' : '展开' }}</span>
    </div>

    <div v-if="expanded || hasIssue" class="system-status__details">
      <div class="system-status__item" :class="{ 'system-status__item--error': !result.ffmpegAvailable }">
        <span>FFmpeg</span>
        <strong v-if="result.ffmpegAvailable">已就绪</strong>
        <strong v-else>
          未安装
          <a href="https://ffmpeg.org/download.html" target="_blank" class="system-status__link">点击下载</a>
        </strong>
      </div>
      <div class="system-status__item" :class="{ 'system-status__item--warn': !result.gpuEncoderAvailable }">
        <span>加速</span>
        <strong>{{ result.gpuEncoderAvailable ? '可用' : 'CPU' }}</strong>
      </div>
      <div class="system-status__item" :class="{ 'system-status__item--error': !result.diskSufficient }">
        <span>磁盘空间</span>
        <strong>{{ result.diskFreeMb >= 1024 ? `${(result.diskFreeMb / 1024).toFixed(1)} GB` : `${result.diskFreeMb} MB` }}</strong>
      </div>
      <div class="system-status__item" :class="{ 'system-status__item--error': !result.outputDirWritable }">
        <span>输出目录</span>
        <strong>{{ result.outputDirWritable ? '可写' : '不可写' }}</strong>
        <span class="system-status__hint">{{ result.outputDir }}</span>
      </div>
    </div>
  </div>
</template>
