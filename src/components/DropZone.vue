<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const props = defineProps<{
  selectedPath: string;
  sourceName: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  select: [path: string];
  error: [message: string];
}>();

const isDragging = ref(false);

let unlistenDragDrop: (() => void) | null = null;

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const IMAGE_EXTS = ["jpg", "jpeg", "png", "webp"];
const AUDIO_EXTS = ["wav", "mp3", "aac", "flac", "ogg", "m4a"];
const HIDDEN_VIDEO_EXTS = ["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv", "m4v"];

function fileExtension(filePath: string): string {
  return filePath.split(".").pop()?.toLowerCase() ?? "";
}

function selectSupportedFile(filePath: string) {
  const extension = fileExtension(filePath);
  if (HIDDEN_VIDEO_EXTS.includes(extension)) {
    emit("error", "当前发布版本仅开放图片和音频，视频能力已暂停。");
    return;
  }
  if (!IMAGE_EXTS.includes(extension) && !AUDIO_EXTS.includes(extension)) {
    emit("error", "当前图片仅支持 PNG、JPEG、WebP；音频仅支持 WAV、MP3、AAC、FLAC、OGG、M4A。");
    return;
  }
  emit("select", filePath);
}

function getFileTypeMark(filePath: string): string {
  const ext = fileExtension(filePath);
  if (IMAGE_EXTS.includes(ext)) return "IMG";
  if (AUDIO_EXTS.includes(ext)) return "AUD";
  return "FILE";
}

function getFileTypeLabel(filePath: string): string {
  const ext = fileExtension(filePath);
  if (IMAGE_EXTS.includes(ext)) return "图片";
  if (AUDIO_EXTS.includes(ext)) return "音频";
  return "文件";
}

async function handleTauriOpen() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Media",
          extensions: [
            "wav", "mp3", "aac", "flac", "ogg", "m4a",
            "jpg", "jpeg", "png", "webp",
          ],
        },
      ],
    });
    if (typeof selected === "string") {
      selectSupportedFile(selected);
    }
  } catch (error) {
    console.warn("file picker failed", error);
    emit("error", "文件选择器没有打开，请确认桌面端窗口仍在运行后重试。");
  }
}

const fileInputRef = ref<HTMLInputElement | null>(null);

async function handleClick() {
  if (props.disabled) return;
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
  selectSupportedFile(file.name);
}

function onBrowserDrop(event: DragEvent) {
  isDragging.value = false;
  if (props.disabled) return;
  if (isTauriRuntime()) return; // Tauri drag-drop handled via event listener
  const file = event.dataTransfer?.files?.[0];
  if (!file) return;
  selectSupportedFile(file.name);
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
      selectSupportedFile(paths[0]);
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
      accept=".wav,.mp3,.aac,.flac,.ogg,.m4a,.jpg,.jpeg,.png,.webp"
      @click.stop
      @change="onBrowserFileChange"
    />

    <div class="drop-zone__title">拖入或选择文件</div>
    <div class="drop-zone__subtitle">图片 / 音频</div>
    <div class="drop-zone__hint">
      <template v-if="sourceName">
        <span class="drop-zone__type-icon">{{ getFileTypeMark(sourceName) }}</span>
        {{ sourceName }}（{{ getFileTypeLabel(sourceName) }}）
      </template>
      <template v-else>
        未选择文件
      </template>
    </div>
    <div v-if="selectedPath" class="drop-zone__path">
      {{ selectedPath }}
    </div>
  </div>
</template>
