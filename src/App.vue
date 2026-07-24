<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import VerifyView from "./views/VerifyView.vue";
import VaultView from "./views/VaultView.vue";
import WorkbenchView from "./views/WorkbenchView.vue";
import WorkbenchOverviewView from "./views/WorkbenchOverviewView.vue";
import LocalBatchView from "./views/LocalBatchView.vue";
import TelemetryBanner from "./components/TelemetryBanner.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import HelpCenter from "./components/HelpCenter.vue";
import IdentitySetup from "./components/IdentitySetup.vue";
import SubscriptionPanel from "./components/SubscriptionPanel.vue";
import AppShell from "./components/shell/AppShell.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import { trackClick, trackEntitlementSnapshot, trackFeatureEvent } from "./lib/analytics";
import {
  getIdentityStatus,
  getEntitlementState,
  getPreferences,
  getTelemetryAcknowledged,
  checkForUpdate,
  type EntitlementState,
  type AppTab,
  type UpdateInfo,
} from "./lib/tauri-api";

const activeTab = ref<AppTab>("workbench");
const needsIdentitySetup = ref(false);
const entitlementState = ref<EntitlementState | null>(null);
const pendingUpdate = ref<UpdateInfo | null>(null);
const updateDismissed = ref(false);

// Telemetry banner state
const showTelemetryBanner = ref(false);

function handleUpgradeClick() {
  trackClick("subscription_sidebar_open");
  switchTab("subscription");
}

function switchTab(tab: AppTab) {
  trackClick(`tab_switch_${tab}`);
  activeTab.value = tab;
}

function dismissTelemetry() {
  showTelemetryBanner.value = false;
}

function dismissUpdate() {
  updateDismissed.value = true;
}

function handleAvailableUpdate(event: Event) {
  const update = (event as CustomEvent<UpdateInfo>).detail;
  if (update) {
    pendingUpdate.value = update;
    updateDismissed.value = false;
  }
}

const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
const UPDATE_CHECK_STORAGE_KEY = "hiddenshield_last_update_check_at_v1";

function shouldCheckForUpdate() {
  const lastCheckedAt = Number(localStorage.getItem(UPDATE_CHECK_STORAGE_KEY) ?? "0");
  return !Number.isFinite(lastCheckedAt) || Date.now() - lastCheckedAt >= UPDATE_CHECK_INTERVAL_MS;
}

async function checkForAvailableUpdate() {
  try {
    const update = await checkForUpdate();
    localStorage.setItem(UPDATE_CHECK_STORAGE_KEY, String(Date.now()));
    if (update) {
      pendingUpdate.value = update;
    }
  } catch (error) {
    console.warn("Update check failed:", error);
  }
}

async function reloadEntitlementState() {
  entitlementState.value = await getEntitlementState();
  trackEntitlementSnapshot(entitlementState.value.status, "billing_refresh");
}

function openSettingsPanel() {
  switchTab("settings");
}

const tabs: Array<{ key: AppTab; label: string; icon: string; subtitle: string; group?: "primary" | "secondary" }> = [
  { key: "workbench", label: "工作台", icon: "⌂", subtitle: "开始、恢复和检查当前工作区" },
  { key: "process", label: "处理", icon: "◈", subtitle: "选择对象，完成预检、写入、验证和入库" },
  { key: "verify", label: "验证", icon: "✓", subtitle: "验证样本并输出证据摘要" },
  { key: "vault", label: "版权库", icon: "▣", subtitle: "管理版权记录、报告和同步状态" },
  { key: "batch", label: "批量队列", icon: "☷", subtitle: "图片 / 音频年度授权的本地批量处理" },
  { key: "subscription", label: "年度授权", icon: "◇", subtitle: "查看未付费状态、年度基础权益和注册码", group: "secondary" },
  { key: "settings", label: "设置", icon: "⚙", subtitle: "账户、同步、隐私、反馈和日志", group: "secondary" },
  { key: "help", label: "帮助与能力边界", icon: "?", subtitle: "查看可承诺能力、内部测试和禁止承诺", group: "secondary" },
];

const activeTabMeta = computed(() => tabs.find((tab) => tab.key === activeTab.value) ?? tabs[0]);

onMounted(async () => {
  window.addEventListener("hiddenshield:update-available", handleAvailableUpdate);
  const [identityStatus, preferences] = await Promise.all([
    getIdentityStatus(),
    getPreferences(),
  ]);
  if (!identityStatus.initialized || !preferences.onboardingCompleted) {
    needsIdentitySetup.value = true;
  }

  entitlementState.value = await getEntitlementState();
  trackEntitlementSnapshot(entitlementState.value.status, "app_mount");
  trackFeatureEvent("app_open", "success", { source: "main_window" });

  // Check telemetry/privacy acknowledgement
  const acknowledged = await getTelemetryAcknowledged();
  if (!acknowledged) {
    showTelemetryBanner.value = true;
    // PIPL compliance: Do NOT initiate any network requests until user consents.
    // Update check and telemetry are gated behind acknowledgement.
    return;
  }

  if (preferences.autoUpdateEnabled && shouldCheckForUpdate()) {
    window.setTimeout(() => {
      void checkForAvailableUpdate();
    }, 5_000);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("hiddenshield:update-available", handleAvailableUpdate);
});
</script>

<template>
  <div class="app-shell">
    <!-- Identity setup (first-time onboarding) -->
    <IdentitySetup v-if="needsIdentitySetup" @complete="needsIdentitySetup = false" />

    <!-- Main app (shown after identity is set) -->
    <template v-else>
    <!-- Global banners -->
    <UpdateBanner
      v-if="pendingUpdate && !updateDismissed"
      :update="pendingUpdate"
      @dismiss="dismissUpdate"
    />
    <TelemetryBanner
      v-if="showTelemetryBanner"
      @dismiss="dismissTelemetry"
    />

    <AppShell
      :active-tab="activeTab"
      :title="activeTabMeta.label"
      :subtitle="activeTabMeta.subtitle"
      :entitlement-state="entitlementState"
      :nav-items="tabs"
      @navigate="switchTab"
    >
        <WorkbenchOverviewView
          v-if="activeTab === 'workbench'"
          :entitlement-state="entitlementState"
          @navigate="switchTab"
        />
        <WorkbenchView
          v-else-if="activeTab === 'process'"
          :entitlement-state="entitlementState"
          @switch-tab="switchTab"
          @open-subscription="handleUpgradeClick"
        />
        <LocalBatchView
          v-else-if="activeTab === 'batch'"
          :entitlement-state="entitlementState"
          @open-subscription="handleUpgradeClick"
        />
        <VaultView
          v-else-if="activeTab === 'vault'"
          :entitlement-state="entitlementState"
          @open-settings="openSettingsPanel"
          @open-subscription="handleUpgradeClick"
        />
        <VerifyView
          v-else-if="activeTab === 'verify'"
          :entitlement-state="entitlementState"
          @switch-tab="switchTab"
          @open-subscription="handleUpgradeClick"
        />
        <SubscriptionPanel
          v-else-if="activeTab === 'subscription'"
          :entitlement-state="entitlementState"
          embedded
          @close="switchTab('workbench')"
          @entitlement-refreshed="reloadEntitlementState"
        />
        <SettingsPanel
          v-else-if="activeTab === 'settings'"
          @open-subscription="handleUpgradeClick"
        />
        <HelpCenter v-else />
    </AppShell>
    </template>
  </div>
</template>
