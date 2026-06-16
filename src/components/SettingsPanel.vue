<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getAnalyticsOverview, getRiskSnapshot, trackClick, trackFeatureEvent } from "../lib/analytics";
import {
  flushAnonymousFeedbackQueue,
  getAnonymousFeedbackStatus,
  getEntitlementState,
  getUsageLedgerSummary,
  getTelemetryEnabled,
  setTelemetryEnabled,
  getNetworkEnabled,
  setNetworkEnabled,
  getDataUsage,
  clearAllData,
  clearCacheOnly,
  exportCrashLog,
  type EntitlementState,
  type DataUsageInfo,
  type AnonymousFeedbackStatus,
  type UsageLedgerSummary,
  type MobileSyncStatus,
  type DesktopCloudSyncProfile,
  continueCloudAccount,
  getDesktopCloudSyncProfile,
  getMobileSyncStatus,
  regenerateMobilePairingCode,
  signOutDesktopCloud,
} from "../lib/tauri-api";

const emit = defineEmits<{
  openSubscription: [];
}>();

const telemetryEnabled = ref(true);
const networkEnabled = ref(true);
const dataUsage = ref<DataUsageInfo | null>(null);
const entitlementState = ref<EntitlementState | null>(null);
const usageSummary = ref<UsageLedgerSummary | null>(null);
const feedbackStatus = ref<AnonymousFeedbackStatus | null>(null);
const mobileSyncStatus = ref<MobileSyncStatus | null>(null);
const cloudSyncProfile = ref<DesktopCloudSyncProfile | null>(null);
const analyticsOverview = ref<ReturnType<typeof getAnalyticsOverview> | null>(null);
const riskSnapshot = ref<ReturnType<typeof getRiskSnapshot> | null>(null);
const clearing = ref(false);
const flushingFeedback = ref(false);
const continuingCloud = ref(false);
const signingOutCloud = ref(false);
const regeneratingPairingCode = ref(false);
const feedbackNudgeVisible = ref(false);
const cloudIdentifier = ref("");
const creatorDisplayName = ref("本机创作者");
const message = ref("");
const copyMsg = ref("");

type FeedbackNudgeState = {
  lastShownUnits: number;
  lastShownAt: number;
};

const FEEDBACK_NUDGE_KEY = "hiddenshield_feedback_nudge_v1";

function loadFeedbackNudgeState(): FeedbackNudgeState {
  try {
    const raw = localStorage.getItem(FEEDBACK_NUDGE_KEY);
    if (!raw) return { lastShownUnits: 0, lastShownAt: 0 };
    const parsed = JSON.parse(raw) as Partial<FeedbackNudgeState>;
    return {
      lastShownUnits: Number(parsed.lastShownUnits ?? 0),
      lastShownAt: Number(parsed.lastShownAt ?? 0),
    };
  } catch {
    return { lastShownUnits: 0, lastShownAt: 0 };
  }
}

function saveFeedbackNudgeState(state: FeedbackNudgeState) {
  localStorage.setItem(FEEDBACK_NUDGE_KEY, JSON.stringify(state));
}

function refreshFeedbackNudge() {
  const totalUnits = usageSummary.value?.totalUnits ?? 0;
  if (totalUnits < 5) {
    feedbackNudgeVisible.value = false;
    return;
  }

  const state = loadFeedbackNudgeState();
  const enoughUsageGap = totalUnits - state.lastShownUnits >= 5;
  const enoughTimeGap = state.lastShownAt === 0 || Date.now() - state.lastShownAt >= 7 * 24 * 60 * 60 * 1000;
  feedbackNudgeVisible.value = enoughUsageGap && enoughTimeGap;
}

async function copyWechat() {
  await navigator.clipboard.writeText("Zoro998877");
  copyMsg.value = "微信号已复制";
  setTimeout(() => { copyMsg.value = ""; }, 3000);
}

async function loadState() {
  telemetryEnabled.value = await getTelemetryEnabled();
  networkEnabled.value = await getNetworkEnabled();
  dataUsage.value = await getDataUsage();
  entitlementState.value = await getEntitlementState();
  usageSummary.value = await getUsageLedgerSummary();
  feedbackStatus.value = await getAnonymousFeedbackStatus();
  cloudSyncProfile.value = await getDesktopCloudSyncProfile();
  if (cloudSyncProfile.value) {
    cloudIdentifier.value = cloudSyncProfile.value.accountLabel;
    creatorDisplayName.value = cloudSyncProfile.value.creatorDisplayName;
  }
  mobileSyncStatus.value = await getMobileSyncStatus();
  analyticsOverview.value = getAnalyticsOverview();
  riskSnapshot.value = getRiskSnapshot();
  refreshFeedbackNudge();
}

async function refreshMobileSyncStatus() {
  mobileSyncStatus.value = await getMobileSyncStatus();
}

function syncResolutionLabel(type: string) {
  if (type === "variant_accepted") return "已接收为同 UID 素材变体";
  if (type === "revision_upgraded") return "已升级为更高写入版本";
  if (type === "stale_revision_ignored") return "已忽略过期写入版本";
  if (type === "record_inserted") return "已写入新版权记录";
  return type;
}

async function handleRegeneratePairingCode() {
  regeneratingPairingCode.value = true;
  try {
    await regenerateMobilePairingCode();
    mobileSyncStatus.value = await getMobileSyncStatus();
    message.value = "移动端配对码已更新";
  } catch (e: unknown) {
    message.value = String(e);
  } finally {
    regeneratingPairingCode.value = false;
  }
}

async function handleContinueCloudAccount() {
  if (!cloudIdentifier.value.trim()) {
    message.value = "请输入邮箱或手机号";
    return;
  }
  if (!creatorDisplayName.value.trim()) {
    message.value = "请输入创作者身份";
    return;
  }
  continuingCloud.value = true;
  try {
    cloudSyncProfile.value = await continueCloudAccount(
      cloudIdentifier.value.trim(),
      creatorDisplayName.value.trim(),
    );
    message.value = "已继续使用 HiddenShield 账户，云同步资料已保存";
    trackFeatureEvent("desktop_cloud_continue", "success", { source: "settings" });
  } catch (e: unknown) {
    message.value = String(e);
    trackFeatureEvent("desktop_cloud_continue", "failure", { errorCode: "cloud_continue_failed", source: "settings" });
  } finally {
    continuingCloud.value = false;
  }
}

async function handleSignOutCloud() {
  signingOutCloud.value = true;
  try {
    await signOutDesktopCloud();
    cloudSyncProfile.value = null;
    message.value = "已退出云同步账户，本地版权库仍保留";
    trackFeatureEvent("desktop_cloud_sign_out", "success", { source: "settings" });
  } catch (e: unknown) {
    message.value = String(e);
  } finally {
    signingOutCloud.value = false;
  }
}

function entitlementFeatureSummary(features: Record<string, boolean> | null | undefined): string {
  if (!features) return "—";
  const enabled = Object.entries(features)
    .filter(([, value]) => value)
    .map(([key]) => key);
  return enabled.length ? enabled.join(" / ") : "未开放";
}

async function toggleTelemetry() {
  telemetryEnabled.value = !telemetryEnabled.value;
  await setTelemetryEnabled(telemetryEnabled.value);
  trackFeatureEvent("toggle_telemetry", "success", { source: telemetryEnabled.value ? "on" : "off" });
  analyticsOverview.value = getAnalyticsOverview();
  riskSnapshot.value = getRiskSnapshot();
}

async function toggleNetwork() {
  networkEnabled.value = !networkEnabled.value;
  await setNetworkEnabled(networkEnabled.value);
  trackFeatureEvent("toggle_network", "success", { source: networkEnabled.value ? "on" : "off" });
  analyticsOverview.value = getAnalyticsOverview();
  riskSnapshot.value = getRiskSnapshot();
}

async function handleClearCache() {
  if (!confirm("确定清除 FFmpeg 缓存和日志？版权库数据将保留。")) return;
  clearing.value = true;
  try {
    message.value = await clearCacheOnly();
    dataUsage.value = await getDataUsage();
    trackFeatureEvent("clear_cache", "success", { source: "settings" });
  } catch (e: unknown) {
    message.value = String(e);
    trackFeatureEvent("clear_cache", "failure", { errorCode: "clear_cache_failed", source: "settings" });
  } finally {
    clearing.value = false;
  }
}

async function handleClearAll() {
  if (!confirm("⚠️ 确定删除所有数据？包括版权库，此操作不可恢复！")) return;
  if (!confirm("再次确认：删除后版权存证记录将永久丢失，是否继续？")) return;
  clearing.value = true;
  try {
    message.value = await clearAllData();
    trackFeatureEvent("clear_all_data", "success", { source: "settings" });
    window.location.reload();
  } catch (e: unknown) {
    message.value = String(e);
    trackFeatureEvent("clear_all_data", "failure", { errorCode: "clear_all_failed", source: "settings" });
  } finally {
    clearing.value = false;
  }
}

async function handleExportLog() {
  const log = await exportCrashLog();
  if (!log.trim()) {
    message.value = "暂无崩溃日志";
    return;
  }
  await navigator.clipboard.writeText(log);
  message.value = "日志已复制到剪贴板";
  trackFeatureEvent("export_crash_log", "success", { source: "settings" });
}

async function handleFlushFeedback() {
  flushingFeedback.value = true;
  try {
    trackClick("send_diagnostic_click");
    const result = await flushAnonymousFeedbackQueue();
    message.value = result.message;
    feedbackStatus.value = await getAnonymousFeedbackStatus();
    trackFeatureEvent("flush_anonymous_feedback", "success", {
      source: result.endpointConfigured ? "endpoint" : "local_only",
    });
  } catch (e: unknown) {
    message.value = String(e);
    trackFeatureEvent("flush_anonymous_feedback", "failure", { errorCode: "flush_failed", source: "settings" });
  } finally {
    flushingFeedback.value = false;
    analyticsOverview.value = getAnalyticsOverview();
    riskSnapshot.value = getRiskSnapshot();
  }
}

function handleOpenSubscription() {
  trackClick("subscription_settings_open");
  emit("openSubscription");
}

function dismissFeedbackNudge() {
  const totalUnits = usageSummary.value?.totalUnits ?? 0;
  saveFeedbackNudgeState({ lastShownUnits: totalUnits, lastShownAt: Date.now() });
  feedbackNudgeVisible.value = false;
  analyticsOverview.value = getAnalyticsOverview();
  riskSnapshot.value = getRiskSnapshot();
}

async function handleFeedbackNudgeDiagnostic() {
  dismissFeedbackNudge();
  await handleFlushFeedback();
}

function formatEntitlementStatus(status: EntitlementState["status"]): string {
  const map: Record<EntitlementState["status"], string> = {
    free: "免费",
    trial: "试用",
    active: "生效",
    grace: "宽限期",
    expired: "已过期",
  };
  return map[status];
}

function formatDateTime(value: string | null): string {
  if (!value) return "立即可试";
  return new Date(value).toLocaleString();
}

onMounted(loadState);
</script>

<template>
  <div class="settings-panel">
    <h3 class="settings-panel__title">设置</h3>

    <div v-if="feedbackNudgeVisible" class="feedback-nudge">
      <div>
        <strong>这次体验顺手吗？</strong>
        <p>如果遇到问题，点一下“发送诊断”，我们只收匿名队列，不会上传原始素材。</p>
      </div>
      <div class="feedback-nudge__actions">
        <button class="btn btn--secondary" type="button" @click="dismissFeedbackNudge">
          这次还好
        </button>
        <button class="btn btn--primary" type="button" @click="handleFeedbackNudgeDiagnostic">
          发送诊断
        </button>
      </div>
    </div>

    <!-- Telemetry toggle -->
    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>匿名统计</strong>
          <p class="settings-hint">仅上传必要诊断信息，可随时关闭</p>
        </div>
        <button
          class="toggle-btn"
          :class="{ 'toggle-btn--on': telemetryEnabled }"
          type="button"
          @click="toggleTelemetry"
        >
          {{ telemetryEnabled ? "已开启" : "已关闭" }}
        </button>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>联网取证</strong>
          <p class="settings-hint">时间回执与网络时间</p>
        </div>
        <button
          class="toggle-btn"
          :class="{ 'toggle-btn--on': networkEnabled }"
          type="button"
          @click="toggleNetwork"
        >
          {{ networkEnabled ? "已开启" : "已关闭" }}
        </button>
      </div>
    </div>

    <div class="settings-section" v-if="entitlementState && usageSummary">
      <div class="settings-row">
        <strong>权益与账本</strong>
        <button class="btn btn--secondary" type="button" @click="handleOpenSubscription">
          查看订阅方案
        </button>
      </div>
      <div class="usage-grid">
        <span>权益状态</span><span>{{ formatEntitlementStatus(entitlementState.status) }}</span>
        <span>订阅方案</span><span>{{ entitlementState.planName ?? "—" }}</span>
        <span>当前周期</span><span>{{ entitlementState.currentPeriodEndsAt ? formatDateTime(entitlementState.currentPeriodEndsAt) : "—" }}</span>
        <span>试用结束</span><span>{{ entitlementState.trialEndsAt ? formatDateTime(entitlementState.trialEndsAt) : "—" }}</span>
        <span>宽限结束</span><span>{{ entitlementState.graceEndsAt ? formatDateTime(entitlementState.graceEndsAt) : "—" }}</span>
        <span>累计用量</span><span>{{ usageSummary.totalUnits }} 次</span>
        <span>媒体分布</span><span>图 {{ usageSummary.imageUnits }} / 视 {{ usageSummary.videoUnits }} / 音 {{ usageSummary.audioUnits }}</span>
        <span>最近处理</span><span>{{ usageSummary.lastUsedAt ? formatDateTime(usageSummary.lastUsedAt) : "—" }}</span>
        <span>最近功能</span><span>{{ usageSummary.lastFeatureName ?? "—" }}</span>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>账户与云同步</strong>
          <p class="settings-hint">同一账户下同步版权库、取证记录、创作者身份和权益状态；不默认上传媒体文件。</p>
        </div>
        <span class="cloud-pill" :class="{ 'cloud-pill--on': cloudSyncProfile }">
          {{ cloudSyncProfile ? "已连接" : "未连接" }}
        </span>
      </div>

      <div v-if="cloudSyncProfile" class="usage-grid">
        <span>账户</span><span>{{ cloudSyncProfile.accountLabel }}</span>
        <span>工作区</span><span>{{ cloudSyncProfile.workspaceName }}</span>
        <span>设备</span><span>{{ cloudSyncProfile.deviceName ?? cloudSyncProfile.deviceId }}</span>
        <span>创作者</span><span>{{ cloudSyncProfile.creatorDisplayName }}</span>
        <span>权益</span><span>{{ cloudSyncProfile.entitlementLabel }} · {{ cloudSyncProfile.entitlementStatus }}</span>
        <span>权益模块</span><span>{{ entitlementFeatureSummary(cloudSyncProfile.entitlementFeatures) }}</span>
        <span>云服务</span><span class="mono">{{ cloudSyncProfile.cloudBaseUrl }}</span>
        <span>更新时间</span><span>{{ formatDateTime(cloudSyncProfile.updatedAt) }}</span>
      </div>

      <div v-else class="cloud-form">
        <label>
          <span>账户</span>
          <input v-model="cloudIdentifier" type="text" placeholder="name@example.com" />
        </label>
        <label>
          <span>创作者身份</span>
          <input v-model="creatorDisplayName" type="text" placeholder="本机创作者" />
        </label>
        <p class="settings-hint">输入邮箱或手机号后继续；新用户自动创建账户，老用户直接登录。云服务地址由系统配置提供。</p>
      </div>

      <div class="feedback-log">
        <button
          v-if="cloudSyncProfile"
          class="btn btn--secondary"
          type="button"
          :disabled="signingOutCloud"
          @click="handleSignOutCloud"
        >
          退出账户
        </button>
        <button
          v-else
          class="btn btn--primary"
          type="button"
          :disabled="continuingCloud"
          @click="handleContinueCloudAccount"
        >
          {{ continuingCloud ? "继续中" : "继续" }}
        </button>
      </div>
    </div>

    <div class="settings-section" v-if="feedbackStatus">
      <strong>匿名反馈</strong>
      <div class="usage-grid">
        <span>待发送</span><span>{{ feedbackStatus.queuedEvents }} 条</span>
        <span>队列大小</span><span>{{ feedbackStatus.queuedBytes }} B</span>
        <span>失败次数</span><span>{{ feedbackStatus.consecutiveFailures }} 次</span>
        <span>下次重试</span><span>{{ formatDateTime(feedbackStatus.nextRetryAt) }}</span>
        <span>Install ID</span><span class="mono">{{ feedbackStatus.installId }}</span>
        <span>Session ID</span><span class="mono">{{ feedbackStatus.sessionId }}</span>
      </div>
      <div class="feedback-log">
        <button class="btn btn--primary" :disabled="flushingFeedback" @click="handleFlushFeedback">
          发送诊断
        </button>
      </div>
      <p class="settings-hint">
        发送的是当前匿名队列中的诊断事件，不包含文件名、路径、hash 或原始媒体内容。
      </p>
      <p class="settings-hint">
        {{ feedbackStatus.endpointConfigured ? "已配置上报地址" : "未配置上报地址，队列仅本地保留" }}
      </p>
      <div class="usage-grid">
        <span>最近尝试</span><span>{{ feedbackStatus.lastAttemptAt ? formatDateTime(feedbackStatus.lastAttemptAt) : "—" }}</span>
        <span>最近成功</span><span>{{ feedbackStatus.lastSuccessAt ? formatDateTime(feedbackStatus.lastSuccessAt) : "—" }}</span>
        <span>最后错误</span><span>{{ feedbackStatus.lastFlushError ?? "—" }}</span>
      </div>
    </div>

    <details class="settings-section advanced-sync" v-if="mobileSyncStatus">
      <summary>
        <strong>高级：局域网调试同步</strong>
        <span>开发联调 / 临时迁移</span>
      </summary>
      <div class="settings-row advanced-sync__row">
        <p class="settings-hint">该通道不是正式跨端同步方案；正式同步请使用账户与云同步。</p>
        <button class="btn btn--secondary" type="button" @click="refreshMobileSyncStatus">
          刷新
        </button>
      </div>
      <div class="usage-grid">
        <span>状态</span><span>{{ mobileSyncStatus.enabled ? "监听中" : "未启用" }}</span>
        <span>监听地址</span><span class="mono">{{ mobileSyncStatus.listenAddress }}</span>
        <span>端口</span><span>{{ mobileSyncStatus.listenPort }}</span>
        <span>配对码</span><span class="mono">{{ mobileSyncStatus.pairingCode }}</span>
        <span>已收事件</span><span>{{ mobileSyncStatus.receivedEvents }} 条</span>
        <span>最近事件</span><span>{{ mobileSyncStatus.latestEventAt ? formatDateTime(mobileSyncStatus.latestEventAt) : "—" }}</span>
        <span>自动解决</span><span>{{ mobileSyncStatus.resolutionCount }} 次</span>
      </div>
      <div v-if="mobileSyncStatus.latestResolution" class="feedback-log">
        <p class="settings-hint">
          最近处理：{{ syncResolutionLabel(mobileSyncStatus.latestResolution.resolutionType) }}
        </p>
        <div class="usage-grid">
          <span>水印 UID</span><span class="mono">{{ mobileSyncStatus.latestResolution.watermarkUid }}</span>
          <span>处理时间</span><span>{{ formatDateTime(mobileSyncStatus.latestResolution.resolvedAt) }}</span>
          <span>桌面 hash</span><span class="mono">{{ mobileSyncStatus.latestResolution.desktopHash ?? "—" }}</span>
          <span>移动 hash</span><span class="mono">{{ mobileSyncStatus.latestResolution.mobileHash ?? "—" }}</span>
          <span>桌面版本</span><span>{{ mobileSyncStatus.latestResolution.desktopRevision ?? "—" }}</span>
          <span>移动版本</span><span>{{ mobileSyncStatus.latestResolution.mobileRevision ?? "—" }}</span>
        </div>
      </div>
      <div class="feedback-log">
        <button
          class="btn btn--primary"
          type="button"
          :disabled="regeneratingPairingCode"
          @click="handleRegeneratePairingCode"
        >
          生成新配对码
        </button>
      </div>
      <p class="settings-hint">
        手机端填写本机局域网地址和配对码后，可切换到桌面 HTTP 同步模式。
      </p>
    </details>

    <div class="settings-section" v-if="analyticsOverview && riskSnapshot">
      <div class="settings-row">
        <strong>埋点与风控</strong>
        <span class="risk-pill" :class="`risk-pill--${riskSnapshot.level}`">
          {{ riskSnapshot.level === "high" ? "高风险" : riskSnapshot.level === "medium" ? "中风险" : "低风险" }}
        </span>
      </div>
      <div class="usage-grid">
        <span>总事件</span><span>{{ analyticsOverview.totalEvents }} 条</span>
        <span>启动 / 成功</span><span>{{ analyticsOverview.startEvents }} / {{ analyticsOverview.successEvents }}</span>
        <span>失败 / 诊断</span><span>{{ analyticsOverview.failureEvents }} / {{ analyticsOverview.diagnosticEvents }}</span>
        <span>取消次数</span><span>{{ analyticsOverview.cancelEvents }} 次</span>
        <span>转化率</span><span>{{ Math.round(analyticsOverview.conversionRate * 100) }}%</span>
        <span>失败率</span><span>{{ Math.round(analyticsOverview.failureRate * 100) }}%</span>
        <span>重复错误</span><span>{{ riskSnapshot.repeatedErrorCount }} 次</span>
        <span>最后事件</span><span>{{ analyticsOverview.lastEventAt ?? "—" }}</span>
      </div>
      <p v-if="riskSnapshot.reasons.length" class="settings-hint">
        风控信号：{{ riskSnapshot.reasons.join("；") }}
      </p>
      <div class="usage-grid" v-if="analyticsOverview.topActions.length">
        <span>高频动作</span>
        <span>{{ analyticsOverview.topActions[0].action }} × {{ analyticsOverview.topActions[0].total }}</span>
      </div>
    </div>

    <!-- Data usage -->
    <div class="settings-section" v-if="dataUsage">
      <strong>占用</strong>
      <div class="usage-grid">
        <span>FFmpeg 缓存</span><span>{{ dataUsage.ffmpegSizeMb }} MB</span>
        <span>版权库</span><span>{{ dataUsage.dbSizeMb }} MB</span>
        <span>日志</span><span>{{ dataUsage.logSizeMb }} MB</span>
        <span class="usage-total">总计</span><span class="usage-total">{{ dataUsage.totalSizeMb }} MB</span>
      </div>
    </div>

    <!-- Actions -->
    <div class="settings-section">
      <div class="settings-actions">
        <button class="btn btn--secondary" :disabled="clearing" @click="handleClearCache">
          清除缓存
        </button>
        <button class="btn btn--danger" :disabled="clearing" @click="handleClearAll">
          清除所有数据
        </button>
      </div>
      <p class="settings-hint mac-hint">
        卸载前可先清空数据
      </p>
    </div>

    <!-- Message -->
    <p v-if="message" class="settings-message">{{ message }}</p>

    <!-- Feedback -->
    <div class="settings-section feedback-section">
      <strong>问题反馈</strong>
      <div class="feedback-items">
        <div class="feedback-item">
          <span class="feedback-icon">微</span>
          <span class="feedback-label">微信</span>
          <span class="feedback-value">Zoro998877</span>
          <button class="feedback-btn" type="button" @click="copyWechat">复制</button>
        </div>
        <div class="feedback-item">
          <span class="feedback-icon">@</span>
          <span class="feedback-label">邮箱</span>
          <span class="feedback-value">jhx800@163.com</span>
          <a class="feedback-btn" href="mailto:jhx800@163.com?subject=隐盾问题反馈">发送</a>
        </div>
      </div>
      <div class="feedback-log">
        <button class="btn btn--secondary" type="button" @click="handleExportLog">
          导出日志
        </button>
      </div>
      <p v-if="copyMsg" class="settings-message feedback-toast">{{ copyMsg }}</p>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: 1.5rem;
  background: var(--surface, #1a1a2e);
  border-radius: 12px;
  border: 1px solid var(--border, #2a2a4a);
}
.settings-panel__title {
  margin: 0 0 1rem;
  font-size: 1.1rem;
  color: var(--text-primary, #e0e0e0);
}
.settings-section {
  margin-bottom: 1.25rem;
  padding-bottom: 1.25rem;
  border-bottom: 1px solid var(--border, #2a2a4a);
}
.settings-section:last-of-type {
  border-bottom: none;
}
.feedback-nudge {
  margin-bottom: 1rem;
  padding: 1rem 1rem 0.95rem;
  border-radius: 16px;
  background: linear-gradient(135deg, rgba(198, 91, 32, 0.16), rgba(12, 18, 32, 0.82));
  border: 1px solid rgba(198, 91, 32, 0.25);
}
.feedback-nudge strong {
  color: var(--text-primary, #fff);
}
.feedback-nudge p {
  margin: 0.35rem 0 0;
  color: var(--text-secondary, #bbb);
  font-size: 0.85rem;
  line-height: 1.6;
}
.feedback-nudge__actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 0.85rem;
}
.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
.settings-hint {
  font-size: 0.8rem;
  color: var(--text-muted, #888);
  margin: 0.25rem 0 0;
}
.mac-hint {
  margin-top: 0.75rem;
}
.toggle-btn {
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  border: 1px solid var(--border, #2a2a4a);
  background: var(--surface-alt, #252545);
  color: var(--text-muted, #888);
  cursor: pointer;
  font-size: 0.8rem;
  transition: all 0.2s;
}
.toggle-btn--on {
  background: #2d5a2d;
  color: #8f8;
  border-color: #3a7a3a;
}
.usage-grid {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.4rem 1rem;
  margin-top: 0.5rem;
  font-size: 0.85rem;
  color: var(--text-secondary, #aaa);
}
.usage-total {
  font-weight: 600;
  color: var(--text-primary, #e0e0e0);
}
.cloud-pill {
  padding: 0.26rem 0.65rem;
  border-radius: 999px;
  font-size: 0.76rem;
  color: #aab3bf;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
}
.cloud-pill--on {
  color: #9ee7bf;
  background: rgba(35, 96, 58, 0.18);
  border-color: rgba(35, 96, 58, 0.32);
}
.cloud-form {
  display: grid;
  gap: 0.75rem;
  margin-top: 0.85rem;
}
.cloud-form label {
  display: grid;
  gap: 0.35rem;
  color: var(--text-secondary, #aaa);
  font-size: 0.82rem;
}
.cloud-form input {
  width: 100%;
  padding: 0.58rem 0.7rem;
  color: var(--text-primary, #e0e0e0);
  background: var(--surface-alt, #252545);
  border: 1px solid var(--border, #2a2a4a);
  border-radius: 6px;
  outline: none;
}
.cloud-form input:focus {
  border-color: rgba(198, 91, 32, 0.75);
  box-shadow: 0 0 0 3px rgba(198, 91, 32, 0.14);
}
.advanced-sync {
  padding-bottom: 1.25rem;
}
.advanced-sync summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  cursor: pointer;
  color: var(--text-primary, #e0e0e0);
}
.advanced-sync summary span {
  color: var(--text-muted, #888);
  font-size: 0.78rem;
}
.advanced-sync__row {
  margin-top: 0.8rem;
  align-items: flex-start;
}
.risk-pill {
  padding: 0.24rem 0.6rem;
  border-radius: 999px;
  font-size: 0.72rem;
  border: 1px solid transparent;
}
.risk-pill--low {
  background: rgba(35, 96, 58, 0.18);
  border-color: rgba(35, 96, 58, 0.32);
  color: #9ee7bf;
}
.risk-pill--medium {
  background: rgba(171, 116, 31, 0.16);
  border-color: rgba(171, 116, 31, 0.35);
  color: #ffd58a;
}
.risk-pill--high {
  background: rgba(160, 53, 53, 0.18);
  border-color: rgba(160, 53, 53, 0.38);
  color: #ffb0b0;
}
.mono {
  font-family: monospace;
  word-break: break-all;
}
.settings-actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.btn {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
  transition: opacity 0.2s;
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn--secondary {
  background: var(--surface-alt, #252545);
  color: var(--text-primary, #e0e0e0);
  border: 1px solid var(--border, #2a2a4a);
}
.btn--primary {
  background: linear-gradient(135deg, #c65b20, #eb8747);
  color: #fff;
}
.btn--danger {
  background: #5a2020;
  color: #f88;
  border: 1px solid #7a3030;
}
.settings-message {
  margin-top: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--surface-alt, #252545);
  border-radius: 6px;
  font-size: 0.85rem;
  color: var(--text-secondary, #aaa);
}
.feedback-section {
  border-bottom: none;
}
.feedback-items {
  margin-top: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.feedback-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 0.75rem;
  background: var(--surface-alt, #252545);
  border-radius: 8px;
  font-size: 0.85rem;
  color: var(--text-primary, #e0e0e0);
}
.feedback-icon {
  font-size: 1rem;
}
.feedback-label {
  color: var(--text-muted, #888);
  min-width: 60px;
}
.feedback-value {
  font-weight: 600;
  font-family: monospace;
}
.feedback-btn {
  margin-left: auto;
  padding: 0.3rem 0.6rem;
  font-size: 0.75rem;
  font-weight: 500;
  color: #fff;
  background: var(--brand, #c65b20);
  border: none;
  border-radius: 5px;
  cursor: pointer;
  text-decoration: none;
  transition: opacity 0.2s;
}
.feedback-btn:hover {
  opacity: 0.85;
}
.feedback-log {
  margin-top: 1rem;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.feedback-log .settings-hint {
  margin: 0;
}
.feedback-toast {
  margin-top: 0.5rem;
  color: #8f8;
}
</style>
