<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getAnalyticsOverview, getRiskSnapshot, trackClick, trackFeatureEvent } from "../lib/analytics";
import { loadRecentReportExports } from "../lib/report-export-history";
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
  type DesktopCloudSyncProfile,
  type CloudQueueStatus,
  type AccountDevice,
  createDesktopAuthChallenge,
  continueCloudAccount,
  setDesktopCloudAutoSyncEnabled,
  listDesktopCloudDevices,
  updateDesktopCloudDeviceName,
  revokeDesktopCloudDevice,
  getDesktopCloudSyncProfile,
  getDesktopCloudQueueStatus,
  getPreferences,
  savePreferences,
  checkForUpdate,
  signOutDesktopCloud,
  listLocalBatchJobs,
  listVaultRecords,
  type LocalBatchJob,
  type PreferencesStatus,
  type VaultRecord,
} from "../lib/tauri-api";
import { userFacingErrorMessage } from "../lib/user-facing-errors";

const emit = defineEmits<{
  openSubscription: [];
}>();

const telemetryEnabled = ref(true);
const networkEnabled = ref(true);
const dataUsage = ref<DataUsageInfo | null>(null);
const entitlementState = ref<EntitlementState | null>(null);
const usageSummary = ref<UsageLedgerSummary | null>(null);
const feedbackStatus = ref<AnonymousFeedbackStatus | null>(null);
const cloudSyncProfile = ref<DesktopCloudSyncProfile | null>(null);
const cloudQueueStatus = ref<CloudQueueStatus | null>(null);
const cloudDevices = ref<AccountDevice[]>([]);
const preferences = ref<PreferencesStatus | null>(null);
const vaultRecords = ref<VaultRecord[]>([]);
const localBatchJobs = ref<LocalBatchJob[]>([]);
const reportExports = ref(loadRecentReportExports());
const analyticsOverview = ref<ReturnType<typeof getAnalyticsOverview> | null>(null);
const riskSnapshot = ref<ReturnType<typeof getRiskSnapshot> | null>(null);
const clearing = ref(false);
const flushingFeedback = ref(false);
const continuingCloud = ref(false);
const sendingAuthCode = ref(false);
const signingOutCloud = ref(false);
const updatingCloudAutoSync = ref(false);
const loadingCloudDevices = ref(false);
const updatingCloudDeviceId = ref<string | null>(null);
const pendingRevokeDeviceId = ref<string | null>(null);
const feedbackNudgeVisible = ref(false);
const cloudIdentifier = ref("");
const cloudPassword = ref("");
const cloudVerificationCode = ref("");
const authChallengeId = ref<string | null>(null);
const authMode = ref<"code" | "password">("code");
const creatorDisplayName = ref("本机创作者");
const defaultOutputDir = ref("");
const autoUpdateEnabled = ref(true);
const checkingForUpdate = ref(false);
const updateMessage = ref("");
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
  cloudQueueStatus.value = await getDesktopCloudQueueStatus();
  cloudDevices.value = cloudSyncProfile.value ? await listDesktopCloudDevices().catch(() => []) : [];
  preferences.value = await getPreferences();
  defaultOutputDir.value = preferences.value.defaultOutputDir ?? "";
  autoUpdateEnabled.value = preferences.value.autoUpdateEnabled;
  vaultRecords.value = await listVaultRecords();
  localBatchJobs.value = await listLocalBatchJobs();
  reportExports.value = loadRecentReportExports();
  if (cloudSyncProfile.value) {
    cloudIdentifier.value = cloudSyncProfile.value.accountLabel;
    creatorDisplayName.value = cloudSyncProfile.value.creatorDisplayName;
  }
  analyticsOverview.value = getAnalyticsOverview();
  riskSnapshot.value = getRiskSnapshot();
  refreshFeedbackNudge();
}

async function chooseDefaultOutputDir() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择默认输出位置",
  });
  if (typeof selected === "string") {
    defaultOutputDir.value = selected;
    await saveDefaultOutputDir();
  }
}

async function saveDefaultOutputDir() {
  preferences.value = await savePreferences({
    defaultOutputDir: defaultOutputDir.value.trim() || null,
  });
  defaultOutputDir.value = preferences.value.defaultOutputDir ?? "";
  message.value = preferences.value.defaultOutputDirWritable
    ? "默认输出位置已保存"
    : "默认输出位置无法写入，请重新选择";
}

const commercialHealth = computed(() => {
  const batchItems = localBatchJobs.value.flatMap((job) => job.items);
  const verifiedBatchItems = batchItems.filter((item) => item.status === "verified").length;
  const failedBatchItems = batchItems.filter((item) => item.status === "failed").length;
  const reportCount = reportExports.value.length;
  const latestReport = reportExports.value[0]?.exportedAt ?? null;
  const queue = cloudQueueStatus.value;
  const analytics = analyticsOverview.value;
  return {
    accountScope: cloudSyncProfile.value ? "当前账户" : "本机",
    entitlementPlan: entitlementState.value?.features?.batch_processing === true ? "图片 / 音频年费" : "未付费",
    localBatchJobs: localBatchJobs.value.length,
    verifiedBatchItems,
    failedBatchItems,
    reportCount,
    latestReport,
    cloudAcceptedEvents: queue?.synced ?? 0,
    cloudFailureEvents: queue?.failed ?? 0,
    anonymousFailureEvents: analytics ? analytics.failureEvents + analytics.diagnosticEvents : 0,
    privacyNote: "仅展示计数、状态和错误分类；不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希。",
  };
});

function isRecoverableCloudError(value?: string | null) {
  const error = value ?? "";
  return error.includes("HTTP 401") ||
    error.includes("HTTP 403") ||
    error.includes("登录状态已失效") ||
    error.includes("设备未被当前账户授权") ||
    error.includes("工作区或设备与云端账户不匹配");
}

function isCloudAuthExpiredError(error: unknown) {
  const raw = String((error as { message?: unknown })?.message ?? error ?? "").toLowerCase();
  return raw.includes("http 401") ||
    raw.includes("unauthorized") ||
    raw.includes("登录状态已失效");
}

function canEnableCloudSync() {
  return cloudSyncProfile.value?.entitlementFeatures?.cloud_sync === true;
}

function canAutoCloudSync() {
  return canEnableCloudSync() && cloudSyncProfile.value?.syncPolicy === "auto_cloud_vault";
}

function cloudHealthState() {
  const queue = cloudQueueStatus.value;
  if (isRecoverableCloudError(queue?.lastError)) {
    return {
      label: "需恢复账户",
      detail: "账户、设备或工作区授权不一致，请退出后重新登录账户。",
      tone: "danger",
    };
  }
  if (!cloudSyncProfile.value) {
    return {
      label: "未连接",
      detail: "本地功能可直接使用，云同步需要登录账户。",
      tone: "muted",
    };
  }
  if (!canEnableCloudSync()) {
    return {
      label: "未开放",
      detail: "当前账户可继续本地使用；云同步是否开放由服务端账户权益决定。",
      tone: "muted",
    };
  }
  if (!canAutoCloudSync()) {
    return {
      label: "已暂停",
      detail: "当前设备不会自动拉取或上传版权库；本地队列保留，可随时恢复。",
      tone: "muted",
    };
  }
  if ((queue?.failed ?? 0) > 0) {
    return {
      label: "有失败",
      detail: cloudRetryDetail(),
      tone: "warning",
    };
  }
  if ((queue?.pending ?? 0) > 0) {
    return {
      label: "有待同步",
      detail: `还有 ${queue?.pending ?? 0} 条版权记录等待上传。`,
      tone: "pending",
    };
  }
  return {
    label: "正常",
    detail: "同步状态正常，最近没有需要处理的同步问题。",
    tone: "ok",
  };
}

async function handleSetCloudAutoSync(enabled: boolean) {
  updatingCloudAutoSync.value = true;
  try {
    cloudSyncProfile.value = await setDesktopCloudAutoSyncEnabled(enabled);
    message.value = enabled
      ? "已恢复自动云同步，正在按顺序拉取和补发版权库"
      : "已暂停当前设备自动云同步，本地队列和云端版权库均保留";
    trackFeatureEvent("desktop_cloud_auto_sync_preference", "success", {
      source: enabled ? "settings_resume" : "settings_pause",
    });
  } catch (e: unknown) {
    console.warn("desktop cloud auto sync preference failed", e);
    if (isCloudAuthExpiredError(e)) {
      try {
        await signOutDesktopCloud();
      } catch (signOutError) {
        console.warn("clear expired cloud profile failed", signOutError);
      }
      cloudSyncProfile.value = null;
      cloudDevices.value = [];
      authChallengeId.value = null;
      cloudVerificationCode.value = "";
      message.value = "登录状态已失效，请重新登录后再调整自动云同步。";
    } else {
      message.value = userFacingErrorMessage(e, "调整自动云同步偏好");
    }
    trackFeatureEvent("desktop_cloud_auto_sync_preference", "failure", {
      errorCode: "cloud_sync_preference_failed",
      source: enabled ? "settings_resume" : "settings_pause",
    });
  } finally {
    updatingCloudAutoSync.value = false;
  }
}

async function refreshCloudDevices() {
  if (!cloudSyncProfile.value) {
    cloudDevices.value = [];
    return;
  }
  loadingCloudDevices.value = true;
  try {
    cloudDevices.value = await listDesktopCloudDevices();
    pendingRevokeDeviceId.value = null;
  } catch (e: unknown) {
    console.warn("list cloud devices failed", e);
    message.value = userFacingErrorMessage(e, "读取设备列表");
  } finally {
    loadingCloudDevices.value = false;
  }
}

async function handleRenameDevice(device: AccountDevice) {
  const nextName = window.prompt("设备名称", device.name)?.trim();
  if (!nextName || nextName === device.name) return;
  updatingCloudDeviceId.value = device.id;
  try {
    const updated = await updateDesktopCloudDeviceName(device.id, nextName);
    cloudDevices.value = cloudDevices.value.map((item) => item.id === updated.id ? updated : item);
    if (updated.isCurrent && cloudSyncProfile.value) {
      cloudSyncProfile.value = { ...cloudSyncProfile.value, deviceName: updated.name, updatedAt: updated.updatedAt };
    }
    message.value = "设备名称已更新";
  } catch (e: unknown) {
    console.warn("rename cloud device failed", e);
    message.value = userFacingErrorMessage(e, "更新设备名称");
  } finally {
    updatingCloudDeviceId.value = null;
  }
}

async function handleRevokeDevice(device: AccountDevice) {
  if (device.isCurrent) {
    message.value = "当前设备请使用退出账户；不能从设备列表撤销当前会话";
    return;
  }
  if (pendingRevokeDeviceId.value !== device.id) {
    pendingRevokeDeviceId.value = device.id;
    message.value = `再次点击确认撤销“${device.name}”；该设备需要重新登录后才能继续同步。`;
    return;
  }
  updatingCloudDeviceId.value = device.id;
  try {
    const result = await revokeDesktopCloudDevice(device.id);
    message.value = `已撤销设备，关闭 ${result.revokedSessionCount} 个会话`;
    pendingRevokeDeviceId.value = null;
    await refreshCloudDevices();
  } catch (e: unknown) {
    console.warn("revoke cloud device failed", e);
    message.value = userFacingErrorMessage(e, "撤销设备");
  } finally {
    updatingCloudDeviceId.value = null;
  }
}

function cloudRetryDetail() {
  const queue = cloudQueueStatus.value;
  if (!queue) {
    return "队列暂时没有失败项。";
  }
  if ((queue.blocked ?? 0) > 0 || queue.blockedReason === "blocked_by_entitlement") {
    return `后端权益快照已阻断 ${queue.blocked ?? 0} 条正式云同步。`;
  }
  if (queue.failed === 0 && queue.pending > 0) {
    return `还有 ${queue.pending} 条版权记录等待上传，尚未完成云同步。`;
  }
  if (queue.failed === 0) return "队列暂时没有失败项。";
  if (queue.retryExhausted > 0 && queue.retryExhausted === queue.failed) {
    return "已达自动重试上限；可在版权库中手动重试。";
  }
  if (queue.nextRetryAt) {
    return `下次自动重试：${formatDateTime(queue.nextRetryAt)}；也可在版权库手动重试。`;
  }
  return "失败记录可立即重试，也可继续等待自动重试。";
}

async function handleContinueCloudAccount() {
  if (!cloudIdentifier.value.trim()) {
    message.value = "请输入账户";
    return;
  }
  if (authMode.value === "password" && !cloudPassword.value.trim()) {
    message.value = "请输入密码";
    return;
  }
  if (authMode.value === "code" && (!authChallengeId.value || !cloudVerificationCode.value.trim())) {
    message.value = "请先发送验证码并输入验证码";
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
      authMode.value === "password" ? cloudPassword.value : "",
      creatorDisplayName.value.trim(),
      authMode.value === "code" ? authChallengeId.value : null,
      authMode.value === "code" ? cloudVerificationCode.value.trim() : null,
    );
    message.value = canEnableCloudSync()
      ? "已登录 HiddenShield 账户，正式云同步已自动准备"
      : "已登录 HiddenShield 账户；当前云端账户未开放同步";
    cloudPassword.value = "";
    cloudVerificationCode.value = "";
    authChallengeId.value = null;
    await refreshCloudDevices();
    trackFeatureEvent("desktop_cloud_login", "success", { source: `settings_${authMode.value}` });
  } catch (e: unknown) {
    console.warn("desktop cloud login failed", e);
    message.value = userFacingErrorMessage(e, "继续使用 HiddenShield 账户");
    trackFeatureEvent("desktop_cloud_login", "failure", { errorCode: "cloud_login_failed", source: `settings_${authMode.value}` });
  } finally {
    continuingCloud.value = false;
  }
}

async function handleSendAuthCode() {
  if (!cloudIdentifier.value.trim()) {
    message.value = "请输入账户";
    return;
  }
  sendingAuthCode.value = true;
  try {
    const challenge = await createDesktopAuthChallenge(cloudIdentifier.value.trim());
    authChallengeId.value = challenge.challengeId;
    cloudVerificationCode.value = challenge.fixtureCode ?? "";
    message.value = challenge.fixtureCode
      ? `${challenge.message} 验证码已填入。`
      : challenge.message;
    trackFeatureEvent("desktop_auth_challenge", "success", { source: "settings" });
  } catch (e: unknown) {
    console.warn("desktop auth challenge failed", e);
    message.value = userFacingErrorMessage(e, "发送验证码");
    trackFeatureEvent("desktop_auth_challenge", "failure", { errorCode: "auth_challenge_failed", source: "settings" });
  } finally {
    sendingAuthCode.value = false;
  }
}

async function handleSignOutCloud() {
  signingOutCloud.value = true;
  try {
    await signOutDesktopCloud();
    cloudSyncProfile.value = null;
    cloudDevices.value = [];
    message.value = "已退出云同步账户，本地版权库仍保留";
    trackFeatureEvent("desktop_cloud_sign_out", "success", { source: "settings" });
  } catch (e: unknown) {
    console.warn("desktop cloud sign out failed", e);
    message.value = userFacingErrorMessage(e, "退出云同步账户");
  } finally {
    signingOutCloud.value = false;
  }
}

function entitlementFeatureSummary(features: Record<string, boolean> | null | undefined): string {
  if (!features) return "—";
  const enabled = Object.entries(features)
    .filter(([key, value]) => value && ["cloud_sync", "batch_processing"].includes(key))
    .map(([key]) => entitlementFeatureLabel(key));
  return enabled.length ? enabled.join(" / ") : "未开放";
}

function entitlementFeatureLabel(key: string): string {
  const map: Record<string, string> = {
    cloud_sync: "云同步",
    batch_processing: "批量处理",
  };
  return map[key] ?? key;
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

async function toggleAutoUpdate() {
  preferences.value = await savePreferences({
    autoUpdateEnabled: !autoUpdateEnabled.value,
  });
  autoUpdateEnabled.value = preferences.value.autoUpdateEnabled;
  updateMessage.value = autoUpdateEnabled.value
    ? "已开启后台更新检查"
    : "已关闭后台更新检查";
}

async function handleCheckForUpdate() {
  checkingForUpdate.value = true;
  updateMessage.value = "";
  try {
    const update = await checkForUpdate();
    updateMessage.value = update
      ? `发现新版本 v${update.version}，将在本次启动中提示安装。`
      : "当前已是最新版本";
    if (update) {
      window.dispatchEvent(new CustomEvent("hiddenshield:update-available", { detail: update }));
    }
  } catch {
    updateMessage.value = "暂时无法检查更新，请稍后重试";
  } finally {
    checkingForUpdate.value = false;
  }
}

async function handleClearCache() {
  if (!confirm("确定清除缓存和日志？版权库数据将保留。")) return;
  clearing.value = true;
  try {
    message.value = await clearCacheOnly();
    dataUsage.value = await getDataUsage();
    trackFeatureEvent("clear_cache", "success", { source: "settings" });
  } catch (e: unknown) {
    console.warn("clear cache failed", e);
    message.value = userFacingErrorMessage(e, "清理缓存");
    trackFeatureEvent("clear_cache", "failure", { errorCode: "clear_cache_failed", source: "settings" });
  } finally {
    clearing.value = false;
  }
}

async function handleClearAll() {
  if (!confirm("确定删除所有数据？包括版权库，此操作不可恢复！")) return;
  if (!confirm("再次确认：删除后版权存证记录将永久丢失，是否继续？")) return;
  clearing.value = true;
  try {
    message.value = await clearAllData();
    trackFeatureEvent("clear_all_data", "success", { source: "settings" });
    window.location.reload();
  } catch (e: unknown) {
    console.warn("clear all data failed", e);
    message.value = userFacingErrorMessage(e, "删除本机数据");
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
    free: "未付费",
    trial: "试用",
    active: "生效",
    grace: "宽限期",
    expired: "已过期",
  };
  return map[status];
}

function usageFeatureLabel(featureName: string | null): string {
  if (!featureName) return "暂无记录";
  const labels: Record<string, string> = {
    watermark_image: "图片嵌入",
    watermark_audio: "音频嵌入",
    watermark_video: "已暂停能力",
  };
  return labels[featureName] ?? featureName;
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
        <p>如果遇到问题，点一下“发送反馈”，我们只收必要的匿名信息，不会上传原始素材。</p>
      </div>
      <div class="feedback-nudge__actions">
        <button class="btn btn--secondary" type="button" @click="dismissFeedbackNudge">
          这次还好
        </button>
        <button class="btn btn--primary" type="button" @click="handleFeedbackNudgeDiagnostic">
          发送反馈
        </button>
      </div>
    </div>

    <!-- Telemetry toggle -->
    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>匿名统计</strong>
          <p class="settings-hint">仅记录功能结果、错误码、耗时和桶化信息，不上传原始媒体、加水印媒体或本地路径。</p>
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
          <strong>应用更新</strong>
          <p class="settings-hint">仅检查已签名的版本清单；下载完成后由你确认重启更新。</p>
        </div>
        <button
          class="toggle-btn"
          :class="{ 'toggle-btn--on': autoUpdateEnabled }"
          type="button"
          @click="toggleAutoUpdate"
        >
          {{ autoUpdateEnabled ? "自动检查" : "已关闭" }}
        </button>
      </div>
      <div class="feedback-log">
        <button class="btn btn--secondary" type="button" :disabled="checkingForUpdate" @click="handleCheckForUpdate">
          {{ checkingForUpdate ? "检查中" : "检查更新" }}
        </button>
        <span v-if="updateMessage" class="settings-hint">{{ updateMessage }}</span>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>联网验证</strong>
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

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>默认输出位置</strong>
          <p class="settings-hint">保护副本会优先保存到这里；留空则保存到源文件所在文件夹。</p>
        </div>
        <span class="cloud-pill" :class="{ 'cloud-pill--on': preferences?.defaultOutputDirWritable !== false }">
          {{ preferences?.defaultOutputDirWritable === false ? "不可写" : "可用" }}
        </span>
      </div>
      <div class="cloud-form">
        <label>
          <span>保存位置</span>
          <input v-model="defaultOutputDir" type="text" placeholder="不设置时使用源文件所在文件夹" />
        </label>
      </div>
      <div class="feedback-log">
        <button class="btn btn--secondary" type="button" @click="chooseDefaultOutputDir">选择文件夹</button>
        <button class="btn btn--primary" type="button" @click="saveDefaultOutputDir">保存</button>
      </div>
    </div>

    <div class="settings-section" v-if="entitlementState && usageSummary">
      <div class="settings-row">
        <strong>权益与处理统计</strong>
        <button class="btn btn--secondary" type="button" @click="handleOpenSubscription">
          查看年度授权
        </button>
      </div>
      <div class="usage-grid">
        <span>权益状态</span><span>{{ formatEntitlementStatus(entitlementState.status) }}</span>
        <span>年度授权</span><span>{{ entitlementState.features?.batch_processing ? "图片 / 音频年费" : "未付费" }}</span>
        <span>当前周期</span><span>{{ entitlementState.currentPeriodEndsAt ? formatDateTime(entitlementState.currentPeriodEndsAt) : "—" }}</span>
        <span>试用结束</span><span>{{ entitlementState.trialEndsAt ? formatDateTime(entitlementState.trialEndsAt) : "—" }}</span>
        <span>宽限结束</span><span>{{ entitlementState.graceEndsAt ? formatDateTime(entitlementState.graceEndsAt) : "—" }}</span>
        <span>累计完成</span><span>{{ usageSummary.totalUnits }} 次</span>
        <span>类型分布</span><span>图片 {{ usageSummary.imageUnits }} / 音频 {{ usageSummary.audioUnits }} / 视频 {{ usageSummary.videoUnits }}</span>
        <span>最近完成</span><span>{{ usageSummary.lastUsedAt ? formatDateTime(usageSummary.lastUsedAt) : "暂无记录" }}</span>
        <span>最近功能</span><span>{{ usageFeatureLabel(usageSummary.lastFeatureName) }}</span>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>商业健康摘要</strong>
          <p class="settings-hint">云端看板负责全局账户、支付会话和权益分布；这里展示当前设备可确认的权益、同步和处理使用情况。</p>
        </div>
        <span class="cloud-pill" :class="{ 'cloud-pill--on': cloudSyncProfile }">
          {{ commercialHealth.accountScope }}
        </span>
      </div>
      <div class="commercial-metrics-grid">
        <div class="commercial-metric">
          <span>当前权益</span>
          <strong>{{ commercialHealth.entitlementPlan }}</strong>
          <small>{{ entitlementState ? formatEntitlementStatus(entitlementState.status) : "本机状态" }}</small>
        </div>
        <div class="commercial-metric">
          <span>本地批量</span>
          <strong>{{ commercialHealth.localBatchJobs }} 个队列</strong>
          <small>验证 {{ commercialHealth.verifiedBatchItems }} / 失败 {{ commercialHealth.failedBatchItems }}</small>
        </div>
        <div class="commercial-metric">
          <span>正式报告</span>
          <strong>{{ commercialHealth.reportCount }} 次</strong>
          <small>{{ commercialHealth.latestReport ? formatDateTime(commercialHealth.latestReport) : "暂无导出" }}</small>
        </div>
        <div class="commercial-metric">
          <span>云同步</span>
          <strong>成功 {{ commercialHealth.cloudAcceptedEvents }}</strong>
          <small>失败 {{ commercialHealth.cloudFailureEvents }}</small>
        </div>
        <div class="commercial-metric">
          <span>反馈状态</span>
          <strong>{{ commercialHealth.anonymousFailureEvents }} 条</strong>
          <small>仅统计需要关注的问题</small>
        </div>
      </div>
      <p class="settings-hint">{{ commercialHealth.privacyNote }}</p>
    </div>

    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>账户与云同步</strong>
          <p class="settings-hint">同一账户下同步版权库、验证记录、创作者身份和权益状态；默认不上传原始媒体、加水印媒体或本地路径。</p>
        </div>
        <span class="cloud-pill" :class="{ 'cloud-pill--on': cloudSyncProfile }">
          {{ cloudSyncProfile ? "已连接" : "未连接" }}
        </span>
      </div>

      <div class="cloud-health" :class="`cloud-health--${cloudHealthState().tone}`">
        <strong>{{ cloudHealthState().label }}</strong>
        <span>{{ cloudHealthState().detail }}</span>
      </div>

      <div v-if="cloudSyncProfile" class="usage-grid">
        <span>账户</span><span>{{ cloudSyncProfile.accountLabel }}</span>
        <span>工作区</span><span>{{ cloudSyncProfile.workspaceName }}</span>
        <span>设备</span><span>{{ cloudSyncProfile.deviceName ?? cloudSyncProfile.deviceId }}</span>
        <span>创作者</span><span>{{ cloudSyncProfile.creatorDisplayName }}</span>
        <span>权益</span><span>{{ cloudSyncProfile.entitlementFeatures?.batch_processing ? "图片 / 音频年费" : "未付费" }} · {{ cloudSyncProfile.entitlementStatus }}</span>
        <span>权益模块</span><span>{{ entitlementFeatureSummary(cloudSyncProfile.entitlementFeatures) }}</span>
        <span>自动同步</span><span>{{ canAutoCloudSync() ? "已开启" : canEnableCloudSync() ? "已暂停" : "未开放" }}</span>
        <span>更新时间</span><span>{{ formatDateTime(cloudSyncProfile.updatedAt) }}</span>
        <span>队列</span><span>待同步 {{ cloudQueueStatus?.pending ?? 0 }} / 失败 {{ cloudQueueStatus?.failed ?? 0 }} / 阻断 {{ cloudQueueStatus?.blocked ?? 0 }} / 已同步 {{ cloudQueueStatus?.synced ?? 0 }}</span>
        <span>重试状态</span><span>{{ cloudRetryDetail() }}</span>
      </div>
      <p v-if="cloudSyncProfile && !canEnableCloudSync()" class="settings-hint">
        当前账户已登录，但云同步尚未由服务端账户权益开放。本地写入、验证和版权库仍可继续使用。
      </p>

      <div v-else class="cloud-form">
        <div class="auth-mode-tabs" role="tablist" aria-label="登录方式">
          <button
            type="button"
            :class="{ active: authMode === 'code' }"
            @click="authMode = 'code'"
          >
            验证码登录
          </button>
          <button
            type="button"
            :class="{ active: authMode === 'password' }"
            @click="authMode = 'password'"
          >
            密码登录
          </button>
        </div>
        <label>
          <span>账户</span>
          <input v-model="cloudIdentifier" type="text" placeholder="name@example.com" />
        </label>
        <label v-if="authMode === 'password'">
          <span>密码</span>
          <input v-model="cloudPassword" type="password" placeholder="账户密码" />
        </label>
        <label v-else>
          <span>验证码</span>
          <div class="inline-auth-row">
            <input v-model="cloudVerificationCode" type="text" inputmode="numeric" placeholder="6 位验证码" />
            <button
              class="btn btn--secondary"
              type="button"
              :disabled="sendingAuthCode"
              @click="handleSendAuthCode"
            >
              {{ sendingAuthCode ? "发送中" : "发送验证码" }}
            </button>
          </div>
        </label>
        <label>
          <span>创作者身份</span>
          <input v-model="creatorDisplayName" type="text" placeholder="本机创作者" />
        </label>
        <p class="settings-hint">验证码或密码只用于账户登录；创作者身份用于版权记录和水印写入。未登录仍可本地写入、验证和查看本地版权库。</p>
      </div>

      <div class="feedback-log">
        <button
          v-if="cloudSyncProfile"
          class="btn btn--secondary"
          type="button"
          :disabled="!canEnableCloudSync() || updatingCloudAutoSync"
          @click="handleSetCloudAutoSync(!canAutoCloudSync())"
        >
          {{ updatingCloudAutoSync ? "处理中" : canAutoCloudSync() ? "暂停自动同步" : "恢复自动同步" }}
        </button>
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
          {{ continuingCloud ? "登录中" : "登录" }}
        </button>
      </div>
    </div>

    <div class="settings-section" v-if="cloudSyncProfile">
      <div class="settings-row">
        <div>
          <strong>设备与会话</strong>
          <p class="settings-hint">查看当前账户登录过的设备。撤销其他设备会关闭其会话，设备需重新登录后才能继续同步。</p>
        </div>
        <button
          class="btn btn--secondary"
          type="button"
          :disabled="loadingCloudDevices"
          @click="refreshCloudDevices"
        >
          {{ loadingCloudDevices ? "刷新中" : "刷新设备" }}
        </button>
      </div>
      <div class="device-list">
        <div v-for="device in cloudDevices" :key="device.id" class="device-row">
          <div>
            <strong>
              {{ device.name }}
              <span v-if="device.isCurrent" class="cloud-pill cloud-pill--on">当前设备</span>
              <span v-else-if="!device.registered" class="cloud-pill">已撤销</span>
            </strong>
            <p class="settings-hint">
              {{ device.platform }} · {{ device.appVersion }} · 活跃会话 {{ device.activeSessionCount }}
            </p>
            <p class="settings-hint">
              最近使用：{{ formatDateTime(device.lastSeenAt) }} · 自动同步：{{ device.autoSyncEnabled && device.registered ? "允许" : "关闭" }}
            </p>
          </div>
          <div class="device-row__actions">
            <button
              class="btn btn--secondary"
              type="button"
              :disabled="updatingCloudDeviceId === device.id || !device.registered"
              @click="handleRenameDevice(device)"
            >
              重命名
            </button>
            <button
              class="btn btn--danger"
              type="button"
              :disabled="device.isCurrent || updatingCloudDeviceId === device.id || !device.registered"
              @click="handleRevokeDevice(device)"
            >
              {{ pendingRevokeDeviceId === device.id ? "确认撤销" : "撤销" }}
            </button>
          </div>
        </div>
        <p v-if="!cloudDevices.length" class="settings-hint">暂无设备记录。</p>
      </div>
    </div>

    <div class="settings-section">
      <strong>条款与边界</strong>
      <div class="usage-grid terms-grid">
        <span>隐私政策</span><span>默认不同步原始图片、加水印图片、原始音频、加水印音频、原始视频、加水印视频和本地文件路径。</span>
        <span>用户协议</span><span>报告、时间戳和指纹存证是技术辅助材料，不构成法律意见、司法鉴定或诉讼结果承诺。</span>
        <span>年度授权支付</span><span>权益以云端状态为准；确认支付只触发查单或刷新，不会绕过后端直接开通年度权益。</span>
        <span>当前发布范围</span><span>当前版本只开放桌面端图片 / 音频能力及后端云服务；移动端开发与全部视频能力均已暂停。</span>
      </div>
    </div>

    <div class="settings-section" v-if="feedbackStatus">
      <strong>匿名反馈</strong>
      <div class="usage-grid">
        <span>待发送</span><span>{{ feedbackStatus.queuedEvents }} 条</span>
        <span>队列大小</span><span>{{ feedbackStatus.queuedBytes }} B</span>
        <span>失败次数</span><span>{{ feedbackStatus.consecutiveFailures }} 次</span>
        <span>下次重试</span><span>{{ formatDateTime(feedbackStatus.nextRetryAt) }}</span>
      </div>
      <div class="feedback-log">
        <button class="btn btn--primary" :disabled="flushingFeedback" @click="handleFlushFeedback">
          发送反馈
        </button>
      </div>
      <p class="settings-hint">
        发送的是当前匿名队列中的反馈事件，不包含文件名、路径、作品指纹或原始媒体内容。
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

    <div class="settings-section" v-if="analyticsOverview && riskSnapshot">
      <div class="settings-row">
        <strong>体验改进</strong>
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
        需要关注：{{ riskSnapshot.reasons.join("；") }}
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
        <span>处理组件缓存</span><span>{{ dataUsage.ffmpegSizeMb }} MB</span>
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
  background: var(--hs-surface);
  border-radius: var(--hs-radius-card);
  border: 1px solid var(--hs-border);
  box-shadow: none;
}
.settings-panel__title {
  margin: 0 0 1rem;
  font-size: 1.1rem;
  color: var(--hs-text);
}
.settings-section {
  margin-bottom: 1.25rem;
  padding-bottom: 1.25rem;
  border-bottom: 1px solid var(--hs-border);
}
.settings-section:last-of-type {
  border-bottom: none;
}
.feedback-nudge {
  margin-bottom: 1rem;
  padding: 1rem 1rem 0.95rem;
  border-radius: var(--hs-radius-card);
  background: rgba(114, 214, 202, 0.08);
  border: 1px solid rgba(114, 214, 202, 0.24);
}
.feedback-nudge strong {
  color: var(--hs-text);
}
.feedback-nudge p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
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
  color: var(--hs-text-muted);
  margin: 0.25rem 0 0;
}
.mac-hint {
  margin-top: 0.75rem;
}
.toggle-btn {
  padding: 0.4rem 0.8rem;
  border-radius: var(--hs-radius-card);
  border: 1px solid var(--hs-border);
  background: var(--hs-surface-muted);
  color: var(--hs-text-muted);
  cursor: pointer;
  font-size: 0.8rem;
  transition: all 0.2s;
}
.toggle-btn--on {
  background: rgba(114, 214, 202, 0.1);
  color: var(--hs-accent);
  border-color: rgba(114, 214, 202, 0.3);
}
.usage-grid {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.4rem 1rem;
  margin-top: 0.5rem;
  font-size: 0.85rem;
  color: var(--hs-text-muted);
}
.terms-grid {
  grid-template-columns: minmax(5.5rem, 0.35fr) minmax(0, 1fr);
}
.terms-grid span {
  min-width: 0;
}
.usage-total {
  font-weight: 600;
  color: var(--hs-text);
}
.commercial-metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 0.65rem;
  margin-top: 0.85rem;
}
.commercial-metric {
  min-height: 92px;
  padding: 0.75rem;
  border-radius: 8px;
  border: 1px solid var(--hs-border);
  background: var(--hs-surface-raised);
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.commercial-metric span {
  color: var(--hs-text-muted);
  font-size: 0.76rem;
}
.commercial-metric strong {
  color: var(--hs-text);
  font-size: 1.05rem;
  line-height: 1.25;
}
.commercial-metric small {
  color: var(--hs-text-muted);
  line-height: 1.35;
}
.cloud-pill {
  padding: 0.26rem 0.65rem;
  border-radius: var(--hs-radius-pill);
  font-size: 0.76rem;
  color: var(--hs-text-muted);
  background: var(--hs-chip);
  border: 1px solid var(--hs-border);
  white-space: nowrap;
}
.device-list {
  display: grid;
  gap: 0.65rem;
  margin-top: 0.85rem;
}
.device-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem;
  border: 1px solid var(--hs-border);
  border-radius: 8px;
  background: var(--hs-surface-raised);
}
.device-row strong {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--hs-text);
}
.device-row__actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.cloud-pill--on {
  color: var(--hs-accent);
  background: rgba(114, 214, 202, 0.1);
  border-color: rgba(114, 214, 202, 0.24);
}
.cloud-health {
  display: flex;
  align-items: flex-start;
  gap: 0.7rem;
  margin-top: 0.85rem;
  padding: 0.75rem 0.85rem;
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-raised);
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.45;
}
.cloud-health strong {
  min-width: 4.5rem;
  color: var(--hs-text);
}
.cloud-health--ok {
  background: rgba(114, 214, 202, 0.08);
  border-color: rgba(114, 214, 202, 0.28);
}
.cloud-health--pending {
  background: var(--hs-surface-raised);
  border-color: var(--hs-border);
}
.cloud-health--warning {
  background: var(--hs-warning-surface);
  border-color: rgba(255, 200, 87, 0.32);
}
.cloud-health--danger {
  background: var(--hs-danger-surface);
  border-color: rgba(255, 180, 171, 0.28);
}
.cloud-form {
  display: grid;
  gap: 0.75rem;
  margin-top: 0.85rem;
}
.cloud-form label {
  display: grid;
  gap: 0.35rem;
  color: var(--hs-text-muted);
  font-size: 0.82rem;
}
.cloud-form input {
  width: 100%;
  padding: 0.58rem 0.7rem;
  color: var(--hs-text);
  background: var(--hs-surface-muted);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  outline: none;
}
.cloud-form input:focus {
  border-color: var(--hs-accent);
  box-shadow: 0 0 0 3px rgba(114, 214, 202, 0.14);
}
.auth-mode-tabs {
  display: inline-flex;
  width: fit-content;
  gap: 0.25rem;
  padding: 0.25rem;
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-muted);
}
.auth-mode-tabs button {
  border: 0;
  border-radius: calc(var(--hs-radius-card) - 2px);
  padding: 0.45rem 0.7rem;
  color: var(--hs-text-muted);
  background: transparent;
  cursor: pointer;
}
.auth-mode-tabs button.active {
  color: var(--hs-text);
  background: var(--hs-surface);
}
.inline-auth-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.5rem;
  align-items: center;
}
.risk-pill {
  padding: 0.24rem 0.6rem;
  border-radius: var(--hs-radius-pill);
  font-size: 0.72rem;
  border: 1px solid transparent;
}
.risk-pill--low {
  background: rgba(114, 214, 202, 0.1);
  border-color: rgba(114, 214, 202, 0.28);
  color: var(--hs-accent);
}
.risk-pill--medium {
  background: rgba(255, 200, 87, 0.1);
  border-color: rgba(255, 200, 87, 0.32);
  color: var(--hs-warning);
}
.risk-pill--high {
  background: var(--hs-danger-surface);
  border-color: rgba(255, 180, 171, 0.28);
  color: var(--hs-danger);
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
  border-radius: var(--hs-radius-card);
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
  background: var(--hs-surface-muted);
  color: var(--hs-text);
  border: 1px solid var(--hs-border);
}
.btn--primary {
  background: var(--hs-accent);
  color: #061312;
}
.btn--danger {
  background: var(--hs-danger-surface);
  color: var(--hs-danger);
  border: 1px solid rgba(255, 180, 171, 0.28);
}
.settings-message {
  margin-top: 0.75rem;
  padding: 0.5rem 0.75rem;
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  font-size: 0.85rem;
  color: var(--hs-text-muted);
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
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  font-size: 0.85rem;
  color: var(--hs-text);
}
.feedback-icon {
  font-size: 1rem;
}
.feedback-label {
  color: var(--hs-text-muted);
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
  color: #061312;
  background: var(--hs-accent);
  border: 1px solid transparent;
  border-radius: var(--hs-radius-card);
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
  color: var(--hs-accent);
}
</style>
