<script setup lang="ts">
import { computed } from "vue";
import type { PipelineProgressPayload, Platform } from "../lib/tauri-api";

const props = defineProps<{
  busy: boolean;
  summary: string;
  progress: PipelineProgressPayload;
}>();

const emit = defineEmits<{ retry: [] }>();

const labels: Record<Platform, string> = {
  douyin: "抖音",
  bilibili: "B站",
  xiaohongshu: "小红书",
};

const isFailed = computed(() => props.progress.stage.startsWith("失败"));
const failureReason = computed(() => {
  if (!isFailed.value) return "";
  return props.progress.stage.replace(/^失败[：:]?\s*/, "");
});

// Only show platforms that have non-zero progress or are actively being processed
const activePlatforms = computed(() => {
  return Object.entries(props.progress.platformPercents)
    .filter(([, value]) => value > 0 || props.busy)
    .filter(([key]) => key in labels);
});

const hasPlatformProgress = computed(() => activePlatforms.value.some(([, v]) => v > 0));
</script>

<template>
  <section class="panel progress-panel">
    <div class="panel__header">
      <div>
        <h3>进度</h3>
        <p>{{ busy ? progress.stage : isFailed ? "失败" : "就绪" }}</p>
      </div>
      <span class="pill">{{ progress.percent }}%</span>
    </div>

    <div class="progress-panel__track">
      <div class="progress-panel__fill" :style="{ width: `${progress.percent}%` }" />
    </div>

    <!-- Platform-specific progress (only for video with active platforms) -->
    <div v-if="hasPlatformProgress" class="progress-panel__platforms">
      <div
        v-for="[key, value] in activePlatforms"
        :key="key"
        class="progress-panel__platform"
      >
        <div class="progress-panel__platform-header">
          <span>{{ labels[key as Platform] }}</span>
          <span>{{ value }}%</span>
        </div>
        <div class="progress-panel__mini-track">
          <div class="progress-panel__mini-fill" :style="{ width: `${value}%` }" />
        </div>
      </div>
    </div>

    <!-- Failure details and retry -->
    <div v-if="isFailed" class="progress-panel__failure">
      <p class="progress-panel__failure-reason">{{ failureReason || "未知错误" }}</p>
      <button class="primary-button" type="button" @click="emit('retry')">重试</button>
    </div>
  </section>
</template>
