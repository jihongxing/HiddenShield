<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import VerifyView from "./views/VerifyView.vue";
import VaultView from "./views/VaultView.vue";
import WorkbenchView from "./views/WorkbenchView.vue";
import ProBadge from "./components/ProBadge.vue";
import UpdateBanner from "./components/UpdateBanner.vue";
import TelemetryBanner from "./components/TelemetryBanner.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import IdentitySetup from "./components/IdentitySetup.vue";
import { trackClick, getClickSummary } from "./lib/analytics";
import {
  checkForUpdate,
  getIdentityStatus,
  getTelemetryAcknowledged,
  type UpdateInfo,
  type AppTab,
} from "./lib/tauri-api";

const activeTab = ref<AppTab>("workbench");
const showUpgradeStats = ref(false);
const showSettings = ref(false);
const needsIdentitySetup = ref(false);

// Update state
const pendingUpdate = ref<UpdateInfo | null>(null);
const updateDismissed = ref(false);

// Telemetry banner state
const showTelemetryBanner = ref(false);

const upgradeStats = computed(() => getClickSummary("upgrade_pro_click"));

function handleUpgradeClick() {
  trackClick("upgrade_pro_click");
  showUpgradeStats.value = false;
  alert("Pro 版即将上线，敬请期待！感谢你的支持 🙌");
}

function toggleStats() {
  showUpgradeStats.value = !showUpgradeStats.value;
}

function switchTab(tab: AppTab) {
  activeTab.value = tab;
}

function dismissUpdate() {
  updateDismissed.value = true;
}

function dismissTelemetry() {
  showTelemetryBanner.value = false;
  // User has now acknowledged — safe to check for updates (PIPL compliant).
  setTimeout(async () => {
    try {
      const update = await checkForUpdate();
      if (update) {
        pendingUpdate.value = update;
      }
    } catch {
      // Silently ignore — air-gapped or offline environments
    }
  }, 2000);
}

const tabs: Array<{ key: AppTab; label: string; description: string }> = [
  { key: "workbench", label: "工作台", description: "拖入文件，输出全平台优化版本" },
  { key: "vault", label: "版权库", description: "沉淀本地版权资产账本" },
  { key: "verify", label: "维权取证", description: "从疑似侵权文件里回溯水印" },
];

const activeDescription = computed(
  () => tabs.find((tab) => tab.key === activeTab.value)?.description ?? "",
);

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

  // Only check for updates if user has already consented to privacy policy.
  // Non-blocking: wrapped in setTimeout + catch to avoid blocking on air-gapped machines.
  setTimeout(async () => {
    try {
      const update = await checkForUpdate();
      if (update) {
        pendingUpdate.value = update;
      }
    } catch {
      // Silently ignore — air-gapped or offline environments
    }
  }, 5000);
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

    <div class="app-body">
      <aside class="sidebar">
        <div class="brand-block">
          <p class="eyebrow">HiddenShield</p>
          <h1>版权界的剪映</h1>
          <p class="brand-block__copy">
            把多平台极致压制做成高频入口，把盲水印和版权资产沉到后端能力里。
          </p>
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
            <span>{{ tab.description }}</span>
          </button>
        </nav>

        <!-- Batch processing Pro entry -->
        <div class="sidebar-pro-entry">
          <ProBadge label="批量处理" :disabled="true" />
        </div>

        <button class="upgrade-button" type="button" @click="handleUpgradeClick">
          ⚡ 升级 Pro 版
          <span class="upgrade-button__sub">批量压制 · 更多平台 · 优先支持</span>
        </button>

        <!-- Settings toggle -->
        <button class="settings-button" type="button" @click="showSettings = !showSettings">
          ⚙️ {{ showSettings ? '收起设置' : '设置' }}
        </button>

        <!-- Trust badge -->
        <div class="sidebar-trust">
          🔒 全本地处理 · 零上传
        </div>

        <div class="sidebar-card">
          <button class="stats-toggle" type="button" @click="toggleStats">
            {{ showUpgradeStats ? '收起' : '📊 查看点击数据' }}
          </button>
          <template v-if="showUpgradeStats">
            <div class="stats-content">
              <p><strong>升级按钮点击次数：</strong>{{ upgradeStats.totalClicks }}</p>
              <p v-if="upgradeStats.firstClick"><strong>首次点击：</strong>{{ upgradeStats.firstClick }}</p>
              <p v-if="upgradeStats.lastClick"><strong>最近点击：</strong>{{ upgradeStats.lastClick }}</p>
              <div v-if="Object.keys(upgradeStats.clicksByDay).length" class="stats-days">
                <p><strong>按日统计：</strong></p>
                <p v-for="(count, day) in upgradeStats.clicksByDay" :key="day">
                  {{ day }}：{{ count }} 次
                </p>
              </div>
            </div>
          </template>
        </div>
      </aside>

      <main class="content-area">
        <header class="content-header">
          <div>
            <p class="eyebrow">MVP Desktop Scaffold</p>
            <h2>{{ tabs.find((tab) => tab.key === activeTab)?.label }}</h2>
            <p>{{ activeDescription }}</p>
          </div>
          <div class="header-badges">
            <span class="pill">Vue 3 + Vite</span>
            <span class="pill">Tauri 2 + Rust</span>
            <span class="pill">FFmpeg-ready</span>
          </div>
        </header>

        <!-- Settings panel (overlay) -->
        <SettingsPanel v-if="showSettings" />

        <WorkbenchView v-else-if="activeTab === 'workbench'" />
        <VaultView v-else-if="activeTab === 'vault'" />
        <VerifyView v-else @switch-tab="switchTab" />
      </main>
    </div>
    </template>
  </div>
</template>
