<script setup lang="ts">
import { onMounted, ref } from "vue";
import VerifyView from "./views/VerifyView.vue";
import VaultView from "./views/VaultView.vue";
import WorkbenchView from "./views/WorkbenchView.vue";
import ProBadge from "./components/ProBadge.vue";
import TelemetryBanner from "./components/TelemetryBanner.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import HelpCenter from "./components/HelpCenter.vue";
import IdentitySetup from "./components/IdentitySetup.vue";
import {
  getIdentityStatus,
  getTelemetryAcknowledged,
  type AppTab,
} from "./lib/tauri-api";

const activeTab = ref<AppTab>("workbench");
const showSettings = ref(false);
const showHelp = ref(false);
const needsIdentitySetup = ref(false);

// Telemetry banner state
const showTelemetryBanner = ref(false);

function handleUpgradeClick() {
  alert("Pro 版即将上线，敬请期待！感谢你的支持");
}

function switchTab(tab: AppTab) {
  activeTab.value = tab;
}

function dismissTelemetry() {
  showTelemetryBanner.value = false;
}

const tabs: Array<{ key: AppTab; label: string }> = [
  { key: "workbench", label: "工作台" },
  { key: "vault", label: "版权库" },
  { key: "verify", label: "取证" },
];

onMounted(async () => {
  // Check if creator identity is set up
  const identityStatus = await getIdentityStatus();
  if (!identityStatus.initialized) {
    needsIdentitySetup.value = true;
  }

  // Check telemetry/privacy acknowledgement
  const acknowledged = await getTelemetryAcknowledged();
  if (!acknowledged) {
    showTelemetryBanner.value = true;
    // PIPL compliance: Do NOT initiate any network requests until user consents.
    // Update check and telemetry are gated behind acknowledgement.
    return;
  }

});
</script>

<template>
  <div class="app-shell">
    <!-- Identity setup (first-time onboarding) -->
    <IdentitySetup v-if="needsIdentitySetup" @complete="needsIdentitySetup = false" />

    <!-- Main app (shown after identity is set) -->
    <template v-else>
    <!-- Global banners -->
    <TelemetryBanner
      v-if="showTelemetryBanner"
      @dismiss="dismissTelemetry"
    />

    <div class="app-body">
      <aside class="sidebar">
        <div class="brand-block">
          <p class="eyebrow">HiddenShield</p>
          <h1>处理 · 存证 · 取证</h1>
        </div>

        <nav class="tab-list">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="tab-list__item"
            :class="{ 'tab-list__item--active': activeTab === tab.key }"
            type="button"
            @click="activeTab = tab.key"
          >
            <strong>{{ tab.label }}</strong>
          </button>
        </nav>

        <!-- Batch processing Pro entry -->
        <div class="sidebar-pro-entry">
          <ProBadge label="批量处理" :disabled="true" />
        </div>

        <button class="upgrade-button" type="button" @click="handleUpgradeClick">Pro 版</button>

        <!-- Settings toggle -->
        <button class="settings-button" type="button" @click="showSettings = !showSettings; showHelp = false">
          {{ showSettings ? '关闭设置' : '设置' }}
        </button>

        <!-- Help toggle -->
        <button class="settings-button" type="button" @click="showHelp = !showHelp; showSettings = false">
          {{ showHelp ? '关闭帮助' : '帮助' }}
        </button>

        <!-- Trust badge -->
        <div class="sidebar-trust">全本地</div>
      </aside>

      <main class="content-area">
        <header class="content-header">
          <div>
            <h2>{{ tabs.find((tab) => tab.key === activeTab)?.label }}</h2>
          </div>
        </header>

        <!-- Settings panel (overlay) -->
        <SettingsPanel v-if="showSettings" />

        <!-- Help center -->
        <HelpCenter v-else-if="showHelp" />

        <WorkbenchView v-else-if="activeTab === 'workbench'" />
        <VaultView v-else-if="activeTab === 'vault'" />
        <VerifyView v-else @switch-tab="switchTab" />
      </main>
    </div>
    </template>
  </div>
</template>
