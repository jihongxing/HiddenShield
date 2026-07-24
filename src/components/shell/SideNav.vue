<script setup lang="ts">
import type { AppTab, EntitlementState } from "../../lib/tauri-api";
import { planLabel } from "../../lib/workspace-context";

defineProps<{
  activeTab: AppTab;
  entitlementState: EntitlementState | null;
  items: Array<{ key: AppTab; label: string; icon: string; group?: "primary" | "secondary" }>;
}>();

const emit = defineEmits<{
  navigate: [tab: AppTab];
}>();
</script>

<template>
  <aside class="hs-side-nav">
    <div class="hs-side-nav__brand">
      <div class="hs-side-nav__mark">H</div>
      <div>
        <strong>HiddenShield</strong>
        <span>版权保护工作台</span>
      </div>
    </div>

    <div class="hs-side-nav__plan">
      <span>当前方案</span>
      <strong>{{ planLabel(entitlementState) }}</strong>
    </div>

    <nav class="hs-side-nav__items" aria-label="HiddenShield 主导航">
      <button
        v-for="item in items"
        :key="item.key"
        class="hs-side-nav__item"
        :class="{
          'hs-side-nav__item--active': activeTab === item.key,
          'hs-side-nav__item--secondary': item.group === 'secondary',
        }"
        :aria-label="item.label"
        type="button"
        @click="emit('navigate', item.key)"
      >
        <span class="hs-side-nav__icon">{{ item.icon }}</span>
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <button class="hs-side-nav__primary" type="button" @click="emit('navigate', 'process')">
      <span>+</span>
      开始处理
    </button>

    <div class="hs-side-nav__status">
      <span>同步边界</span>
      <strong>只同步元数据</strong>
      <small>不上传原始媒体、保护副本或本地路径</small>
    </div>
  </aside>
</template>
