<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  getTelemetryEnabled,
  setTelemetryEnabled,
  getNetworkEnabled,
  setNetworkEnabled,
  getDataUsage,
  clearAllData,
  clearCacheOnly,
  exportCrashLog,
  type DataUsageInfo,
} from "../lib/tauri-api";

const telemetryEnabled = ref(true);
const networkEnabled = ref(true);
const dataUsage = ref<DataUsageInfo | null>(null);
const clearing = ref(false);
const message = ref("");
const copyMsg = ref("");

async function copyWechat() {
  await navigator.clipboard.writeText("Zoro998877");
  copyMsg.value = "微信号已复制";
  setTimeout(() => { copyMsg.value = ""; }, 3000);
}

async function loadState() {
  telemetryEnabled.value = await getTelemetryEnabled();
  networkEnabled.value = await getNetworkEnabled();
  dataUsage.value = await getDataUsage();
}

async function toggleTelemetry() {
  telemetryEnabled.value = !telemetryEnabled.value;
  await setTelemetryEnabled(telemetryEnabled.value);
}

async function toggleNetwork() {
  networkEnabled.value = !networkEnabled.value;
  await setNetworkEnabled(networkEnabled.value);
}

async function handleClearCache() {
  if (!confirm("确定清除 FFmpeg 缓存和日志？版权库数据将保留。")) return;
  clearing.value = true;
  try {
    message.value = await clearCacheOnly();
    dataUsage.value = await getDataUsage();
  } catch (e: unknown) {
    message.value = String(e);
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
    window.location.reload();
  } catch (e: unknown) {
    message.value = String(e);
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
}

onMounted(loadState);
</script>

<template>
  <div class="settings-panel">
    <h3 class="settings-panel__title">设置</h3>

    <!-- Telemetry toggle -->
    <div class="settings-section">
      <div class="settings-row">
        <div>
          <strong>异常上报</strong>
          <p class="settings-hint">仅上传必要诊断信息</p>
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
      <strong>反馈</strong>
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
          <a class="feedback-btn" href="mailto:jhx800@163.com?subject=隐盾 V1.0 用户反馈">发送</a>
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
