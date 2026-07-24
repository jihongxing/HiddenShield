<script setup lang="ts">
import type { AppTab, EntitlementState } from "../../lib/tauri-api";
import { planLabel } from "../../lib/workspace-context";

defineProps<{
  activeTab: AppTab;
  title: string;
  subtitle: string;
  entitlementState: EntitlementState | null;
}>();

const emit = defineEmits<{
  navigate: [tab: AppTab];
}>();
</script>

<template>
  <header class="hs-top-bar">
    <div class="hs-top-bar__title">
      <span>{{ activeTab === "process" ? "处理" : title }}</span>
      <h1>{{ title }}</h1>
      <p>{{ subtitle }}</p>
    </div>

    <div class="hs-top-bar__tools">
      <label class="hs-top-bar__search">
        <span>⌕</span>
        <input type="search" placeholder="搜索版权编号、文件名、任务或能力边界" />
      </label>
      <button class="hs-top-bar__chip" type="button" @click="emit('navigate', 'settings')">
        同步状态
        <strong>{{ entitlementState?.features?.cloud_sync ? "正常" : "本地优先" }}</strong>
      </button>
      <button class="hs-top-bar__chip hs-top-bar__chip--accent" type="button" @click="emit('navigate', 'subscription')">
        {{ planLabel(entitlementState) }}
      </button>
    </div>
  </header>
</template>
