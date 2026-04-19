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
  codec: string;
  note: string;
}> = [
  {
    key: "douyin",
    title: "抖音",
    ratio: "9:16 竖屏",
    codec: "H.264 / 30fps",
    note: "高兼容性，高码率喂饱二压",
  },
  {
    key: "bilibili",
    title: "B站",
    ratio: "16:9 横屏",
    codec: "HEVC / 60fps",
    note: "优先保住高运动场景与横屏清晰度",
  },
  {
    key: "xiaohongshu",
    title: "小红书",
    ratio: "3:4 竖屏",
    codec: "H.264 / 30fps",
    note: "提高信息流占屏面积，尽量对冲激进二压",
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
        <span>{{ item.codec }}</span>
        <p>{{ item.note }}</p>
      </div>
    </button>
  </div>
</template>
