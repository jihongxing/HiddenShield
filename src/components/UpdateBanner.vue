<script setup lang="ts">
import { ref } from "vue";
import { installUpdate, type UpdateInfo } from "../lib/tauri-api";

const props = defineProps<{
  update: UpdateInfo;
}>();

const emit = defineEmits<{
  dismiss: [];
}>();

const installing = ref(false);
const progress = ref(0);
const failed = ref(false);

async function handleInstall() {
  installing.value = true;
  failed.value = false;
  try {
    await installUpdate((downloaded, total) => {
      if (total && total > 0) {
        progress.value = Math.round((downloaded / total) * 100);
      }
    });
  } catch (e) {
    console.error("Update install failed:", e);
    failed.value = true;
    installing.value = false;
  }
}
</script>

<template>
  <div class="update-banner">
    <template v-if="failed">
      <span class="update-banner__text">更新失败，当前版本仍可继续使用。</span>
      <button class="update-btn update-btn--ghost" @click="handleInstall">重试</button>
      <button class="update-btn update-btn--ghost" @click="emit('dismiss')">关闭</button>
    </template>
    <template v-else-if="!installing">
      <span class="update-banner__text">
        发现新版本 <strong>v{{ props.update.version }}</strong>，下载后重启完成更新
      </span>
      <div class="update-banner__actions">
        <button class="update-btn update-btn--primary" @click="handleInstall">
          立即更新
        </button>
        <button class="update-btn update-btn--ghost" @click="emit('dismiss')">
          稍后
        </button>
      </div>
    </template>
    <template v-else>
      <span class="update-banner__text">正在下载更新... {{ progress }}%</span>
      <div class="update-progress">
        <div class="update-progress__bar" :style="{ width: progress + '%' }"></div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.update-banner {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.6rem 1.2rem;
  background: var(--hs-appbar);
  border-bottom: 1px solid var(--hs-border);
  font-size: 0.85rem;
  color: var(--hs-text-muted);
}
.update-banner__text {
  flex: 1;
}
.update-banner__actions {
  display: flex;
  gap: 0.5rem;
}
.update-btn {
  padding: 0.35rem 0.75rem;
  border-radius: var(--hs-radius-card);
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.8rem;
  transition: opacity 0.2s;
}
.update-btn--primary {
  background: var(--hs-accent);
  color: #061312;
}
.update-btn--ghost {
  background: var(--hs-surface-muted);
  color: var(--hs-text);
  border: 1px solid var(--hs-border);
}
.update-progress {
  flex: 1;
  max-width: 200px;
  height: 6px;
  background: var(--hs-surface-muted);
  border-radius: var(--hs-radius-pill);
  overflow: hidden;
}
.update-progress__bar {
  height: 100%;
  background: var(--hs-accent);
  transition: width 0.3s;
}
</style>
