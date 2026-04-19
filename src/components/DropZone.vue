<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

defineProps<{
  selectedPath: string;
  sourceName: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  select: [path: string];
}>();

const isDragging = ref(false);

let unlistenDragDrop: (() => void) | null = null;

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const VIDEO_EXTS = ["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv"];
const IMAGE_EXTS = ["jpg", "jpeg", "png", "bmp", "gif", "webp", "tiff"];
const AUDIO_EXTS = ["wav", "mp3", "aac", "flac", "ogg", "m4a"];

function getFileTypeIcon(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  if (VIDEO_EXTS.includes(ext)) return "🎬";
  if (IMAGE_EXTS.includes(ext)) return "🖼️";
  if (AUDIO_EXTS.includes(ext)) return "🎵";
  return "📄";
}

function getFileTypeLabel(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  if (VIDEO_EXTS.includes(ext)) return "视频";
  if (IMAGE_EXTS.includes(ext)) return "图片";
  if (AUDIO_EXTS.includes(ext)) return "音频";
  return "文件";
}

async function handleTauriOpen() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: "Media",
        extensions: [
          "mp4", "mov", "avi", "mkv", "webm", "flv", "wmv",
          "wav", "mp3", "aac", "flac", "ogg", "m4a",
          "jpg", "jpeg", "png", "bmp", "gif", "webp", "tiff",
        ],
      },
    ],
  });
  if (selected) {
    emit("select", selected);
  }
}

const fileInputRef = ref<HTMLInputElement | null>(null);

async function handleClick() {
  if (isTauriRuntime()) {
    await handleTauriOpen();
  } else {
    fileInputRef.value?.click();
  }
}

function onBrowserFileChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  emit("select", file.name);
}

function onBrowserDrop(event: DragEvent) {
  isDragging.value = false;
  if (isTauriRuntime()) return; // Tauri drag-drop handled via event listener
  const file = event.dataTransfer?.files?.[0];
  if (!file) return;
  emit("select", file.name);
}

function onDragOver() {
  isDragging.value = true;
}

function onDragLeave() {
  isDragging.value = false;
}

onMounted(async () => {
  if (!isTauriRuntime()) return;

  const { listen } = await import("@tauri-apps/api/event");
  unlistenDragDrop = await listen<{ paths: string[] }>("tauri://drag-drop", (event) => {
    const paths = event.payload.paths;
    if (paths && paths.length > 0) {
      emit("select", paths[0]);
    }
  });
});

onUnmounted(() => {
  unlistenDragDrop?.();
});
</script>

<template>
  <div
    class="drop-zone"
    :class="{
      'drop-zone--disabled': disabled,
      'drop-zone--dragging': isDragging,
    }"
    role="button"
    tabindex="0"
    @click="handleClick"
    @dragover.prevent="onDragOver"
    @dragleave="onDragLeave"
    @drop.prevent="onBrowserDrop"
    @keydown.enter="handleClick"
  >
    <!-- Browser fallback: hidden file input (only functional in non-Tauri mode) -->
    <input
      v-if="!isTauriRuntime()"
      ref="fileInputRef"
      class="sr-only"
      type="file"
      :disabled="disabled"
      accept=".mp4,.mov,.avi,.mkv,.wav,.mp3,.jpg,.jpeg,.png"
      @click.stop
      @change="onBrowserFileChange"
    />

    <div class="drop-zone__title">拖入文件到这里</div>
    <div class="drop-zone__subtitle">或点击选择文件，支持视频 / 图片 / 音频</div>
    <div class="drop-zone__hint">
      <template v-if="sourceName">
        <span class="drop-zone__type-icon">{{ getFileTypeIcon(sourceName) }}</span>
        已选择：{{ sourceName }}（{{ getFileTypeLabel(sourceName) }}）
      </template>
      <template v-else>
        推荐先导入一条 iPhone HDR 视频做效果验证
      </template>
    </div>
    <div v-if="selectedPath" class="drop-zone__path">
      {{ selectedPath }}
    </div>
  </div>
</template>
