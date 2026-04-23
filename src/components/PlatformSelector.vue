<script setup lang="ts">
import type { Platform } from "../lib/tauri-api";

const props = defineProps<{
  selected: Platform[];
  disabled?: boolean;
}>();

const emit = defineEmits<{
  toggle: [platform: Platform];
}>();

const platformCards: Array<{
  key: Platform;
  title: string;
  ratio: string;
}> = [
  {
    key: "douyin",
    title: "抖音",
    ratio: "9:16 竖屏",
  },
  {
    key: "bilibili",
    title: "B站",
    ratio: "16:9 横屏",
  },
  {
    key: "xiaohongshu",
    title: "小红书",
    ratio: "3:4 竖屏",
  },
];

function isActive(platform: Platform) {
  return props.selected.includes(platform);
}
</script>

<template>
  <div class="platform-grid">
    <button
      v-for="item in platformCards"
      :key="item.key"
      class="platform-card"
      :class="{ 'platform-card--active': isActive(item.key) }"
      :disabled="disabled"
      type="button"
      @click="emit('toggle', item.key)"
    >
      <div class="platform-card__header">
        <span>{{ item.title }}</span>
        <span>{{ isActive(item.key) ? "已选中" : "待选择" }}</span>
      </div>
      <div class="platform-card__body">
        <strong>{{ item.ratio }}</strong>
      </div>
    </button>
  </div>
</template>
