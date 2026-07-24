<script setup lang="ts">
import type { AppTab, EntitlementState } from "../../lib/tauri-api";
import SideNav from "./SideNav.vue";
import TopStatusBar from "./TopStatusBar.vue";

defineProps<{
  activeTab: AppTab;
  title: string;
  subtitle: string;
  entitlementState: EntitlementState | null;
  navItems: Array<{ key: AppTab; label: string; icon: string; group?: "primary" | "secondary" }>;
}>();

const emit = defineEmits<{
  navigate: [tab: AppTab];
}>();
</script>

<template>
  <div class="hs-product-shell">
    <SideNav
      :active-tab="activeTab"
      :entitlement-state="entitlementState"
      :items="navItems"
      @navigate="emit('navigate', $event)"
    />

    <section class="hs-workspace">
      <TopStatusBar
        :active-tab="activeTab"
        :title="title"
        :subtitle="subtitle"
        :entitlement-state="entitlementState"
        @navigate="emit('navigate', $event)"
      />

      <div class="hs-workspace__grid">
        <main class="hs-main-stage">
          <slot />
        </main>
      </div>
    </section>
  </div>
</template>
