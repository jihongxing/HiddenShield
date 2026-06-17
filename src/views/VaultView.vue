<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import {
  checkFilesExist,
  flushDesktopCloudSyncQueue,
  getDesktopCloudQueueStatus,
  getDesktopCloudSyncProfile,
  listVaultRecords,
  pushSavedDesktopVaultRecordToCloud,
  pullSavedCloudChangesIntoDesktop,
  signOutDesktopCloud,
  type CloudQueueStatus,
  type DesktopCloudSyncProfile,
  type VaultRecord,
} from "../lib/tauri-api";

const emit = defineEmits<{ openSettings: [] }>();

const records = ref<VaultRecord[]>([]);
const missingPaths = ref<Set<string>>(new Set());
const selectedLineageRecord = ref<VaultRecord | null>(null);
const cloudProfile = ref<DesktopCloudSyncProfile | null>(null);
const cloudQueueStatus = ref<CloudQueueStatus>({
  pending: 0,
  failed: 0,
  synced: 0,
  retryExhausted: 0,
  lastAttemptAt: null,
  lastSuccessAt: null,
  lastFailureAt: null,
  nextRetryAt: null,
  lastError: null,
});
const syncingRecordId = ref<number | null>(null);
const flushingCloud = ref(false);
const pullingCloud = ref(false);
const syncMessage = ref("");

const rewrittenRecords = computed(() =>
  records.value.filter(record => record.revision > 1 || record.parentWatermarkUid),
);

const cloudQueueSummary = computed(() => {
  const pending = cloudQueueStatus.value.pending;
  const failed = cloudQueueStatus.value.failed;
  if (pending === 0 && failed === 0) return "队列已清空";
  return `待同步 ${pending} 条 · 失败 ${failed} 条`;
});

const cloudQueueRetrySummary = computed(() => {
  const queue = cloudQueueStatus.value;
  if (queue.failed === 0) return "暂无失败重试压力";
  if (queue.retryExhausted > 0 && queue.retryExhausted === queue.failed) {
    return "已达自动重试上限，需手动同步";
  }
  if (queue.nextRetryAt) {
    return `下次自动重试：${formatSyncTime(queue.nextRetryAt)}`;
  }
  return "失败项可立即重试";
});

const recoverableCloudError = computed(() => {
  const error = cloudQueueStatus.value.lastError ?? "";
  return error.includes("HTTP 401") ||
    error.includes("HTTP 403") ||
    error.includes("登录状态已失效") ||
    error.includes("设备未被当前账户授权") ||
    error.includes("工作区或设备与云端账户不匹配");
});

function openLineage(record: VaultRecord) {
  selectedLineageRecord.value = record;
}

function closeLineage() {
  selectedLineageRecord.value = null;
}

function formatSyncTime(value: string | null): string {
  if (!value) return "无";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function buildCloudDiagnosticsText(): string {
  const profile = cloudProfile.value;
  const queue = cloudQueueStatus.value;
  return [
    "HiddenShield 桌面端同步诊断",
    `生成时间: ${new Date().toLocaleString()}`,
    `账户: ${profile?.accountLabel ?? "未登录"}`,
    `账户 ID: ${profile?.accountId ?? "无"}`,
    `工作区: ${profile?.workspaceName ?? "无"}`,
    `工作区 ID: ${profile?.workspaceId ?? "无"}`,
    `设备: ${profile?.deviceName ?? "无"}`,
    `设备 ID: ${profile?.deviceId ?? "无"}`,
    `设备平台: ${profile?.devicePlatform ?? "无"}`,
    `创作者: ${profile?.creatorDisplayName ?? "无"}`,
    `创作者档案 ID: ${profile?.creatorProfileId ?? "无"}`,
    `权益: ${profile ? `${profile.entitlementLabel} / ${profile.entitlementStatus}` : "无"}`,
    `权益代码: ${profile?.entitlementPlanCode ?? "无"}`,
    `云服务: ${profile?.cloudBaseUrl ?? "无"}`,
    `上次游标: ${profile?.lastRemoteCursor ?? "尚未拉取"}`,
    `队列待同步: ${queue.pending}`,
    `队列失败: ${queue.failed}`,
    `队列已同步: ${queue.synced}`,
    `重试状态: ${queue.retryExhausted > 0 ? `${queue.retryExhausted} 条已达上限` : "正常"}`,
    `下次自动重试: ${queue.nextRetryAt ? formatSyncTime(queue.nextRetryAt) : "无"}`,
    `最近尝试: ${formatSyncTime(queue.lastAttemptAt)}`,
    `最近成功: ${formatSyncTime(queue.lastSuccessAt)}`,
    `最近失败: ${formatSyncTime(queue.lastFailureAt)}`,
    `最近错误: ${queue.lastError ?? "无"}`,
  ].join("\n");
}

async function copyCloudDiagnostics() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中继续使用 HiddenShield 账户";
    return;
  }
  await navigator.clipboard.writeText(buildCloudDiagnosticsText());
  syncMessage.value = "同步诊断已复制到剪贴板";
}

async function resetCloudAccountForRecovery() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中继续使用 HiddenShield 账户";
    return;
  }
  await signOutDesktopCloud();
  cloudProfile.value = null;
  syncMessage.value = "已退出当前云同步账户，请在设置中重新继续账户";
  emit("openSettings");
  await loadCloudState();
}

onMounted(async () => {
  await loadVault();
  await loadCloudState();

  // Lazily check which output files still exist
  const allPaths: string[] = [];
  for (const r of records.value) {
    if (r.outputDouyin) allPaths.push(r.outputDouyin);
    if (r.outputBilibili) allPaths.push(r.outputBilibili);
    if (r.outputXhs) allPaths.push(r.outputXhs);
  }
  if (allPaths.length > 0) {
    const missing = await checkFilesExist(allPaths);
    missingPaths.value = new Set(missing);
  }
});

async function loadVault() {
  records.value = await listVaultRecords();
}

async function loadCloudState() {
  const [profile, queueStatus] = await Promise.all([
    getDesktopCloudSyncProfile(),
    getDesktopCloudQueueStatus(),
  ]);
  cloudProfile.value = profile;
  cloudQueueStatus.value = queueStatus;
}

function isRecordOffline(record: VaultRecord): boolean {
  const outputs = [record.outputDouyin, record.outputBilibili, record.outputXhs].filter(Boolean) as string[];
  if (outputs.length === 0) return false;
  return outputs.every(p => missingPaths.value.has(p));
}

async function uploadRecord(record: VaultRecord) {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中继续使用 HiddenShield 账户";
    return;
  }
  syncingRecordId.value = record.id;
  try {
    const result = await pushSavedDesktopVaultRecordToCloud(record.id);
    syncMessage.value = result.accepted > 0
      ? `已同步 ${record.fileName} 的版权元数据`
      : "已加入云同步队列，稍后可重试";
    await loadCloudState();
  } catch (e: unknown) {
    syncMessage.value = String(e);
    await loadCloudState();
  } finally {
    syncingRecordId.value = null;
  }
}

async function flushCloudQueue() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中继续使用 HiddenShield 账户";
    return;
  }
  flushingCloud.value = true;
  try {
    const result = await flushDesktopCloudSyncQueue(50);
    syncMessage.value = `${result.message}（尝试 ${result.attempted} 条）`;
    await loadCloudState();
  } catch (e: unknown) {
    syncMessage.value = String(e);
    await loadCloudState();
  } finally {
    flushingCloud.value = false;
  }
}

async function pullCloudChanges() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中继续使用 HiddenShield 账户";
    return;
  }
  pullingCloud.value = true;
  try {
    const result = await pullSavedCloudChangesIntoDesktop();
    syncMessage.value = `已拉取 ${result.totalChanges} 条云端变更，落库 ${result.applied} 条，跳过 ${result.skipped} 条`;
    await loadVault();
    await loadCloudState();
  } catch (e: unknown) {
    syncMessage.value = String(e);
    await loadCloudState();
  } finally {
    pullingCloud.value = false;
  }
}
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">Vault</p>
        <h2>版权库</h2>
      </div>
    </section>

    <section class="panel">
      <div class="panel__header">
        <div>
          <h3>存证记录</h3>
          <p v-if="cloudProfile" class="vault-sync-hint">
            云同步：{{ cloudProfile.accountLabel }} · {{ cloudProfile.workspaceName }}
            <span class="vault-sync-hint__queue">{{ cloudQueueSummary }}</span>
          </p>
          <p v-if="cloudProfile" class="vault-sync-meta">
            最近成功：{{ formatSyncTime(cloudQueueStatus.lastSuccessAt) }}
            <span v-if="cloudQueueStatus.lastFailureAt">
              · 最近失败：{{ formatSyncTime(cloudQueueStatus.lastFailureAt) }}
            </span>
          </p>
          <p v-if="cloudProfile" class="vault-sync-meta">
            {{ cloudQueueRetrySummary }}
          </p>
          <p v-if="cloudProfile && cloudQueueStatus.lastError" class="vault-sync-error">
            最近错误：{{ cloudQueueStatus.lastError }}
          </p>
          <div v-if="cloudProfile && recoverableCloudError" class="vault-sync-recovery">
            <strong>账户状态需要恢复</strong>
            <p>当前账户、设备或工作区与云端不一致。请重新继续账户以刷新授权和工作区绑定。</p>
            <div class="vault-sync-recovery__actions">
              <button class="ghost-button" type="button" @click="emit('openSettings')">
                打开设置
              </button>
              <button class="ghost-button" type="button" @click="resetCloudAccountForRecovery">
                退出并重新继续
              </button>
            </div>
          </div>
          <p v-if="!cloudProfile" class="vault-sync-hint">
            云同步未连接，设置中继续账户后可上传版权元数据。
          </p>
        </div>
        <div class="vault-sync-actions">
          <button
            class="ghost-button"
            type="button"
            :disabled="flushingCloud || !cloudProfile || (cloudQueueStatus.pending === 0 && cloudQueueStatus.failed === 0)"
            @click="flushCloudQueue"
          >
            {{ flushingCloud ? "同步中" : "同步队列" }}
          </button>
          <button
            class="ghost-button"
            type="button"
            :disabled="pullingCloud || !cloudProfile"
            @click="pullCloudChanges"
          >
            {{ pullingCloud ? "拉取中" : "拉取云变更" }}
          </button>
          <button
            class="ghost-button"
            type="button"
            :disabled="!cloudProfile"
            @click="copyCloudDiagnostics"
          >
            复制诊断
          </button>
          <span class="pill">{{ records.length }} 条</span>
        </div>
      </div>

      <!-- Pro features -->
      <div class="vault-pro-actions">
        <ProBadge label="导出版权库" :disabled="true" />
        <ProBadge label="批量处理" :disabled="true" />
      </div>

      <div class="vault-cards">
        <div v-for="(record, idx) in records" :key="record.id" class="vault-card-wrapper">
          <div v-if="isRecordOffline(record)" class="vault-offline-badge">
            已离线
          </div>
          <CopyrightCard
            :record="record"
            :highlight="idx === 0"
          />
          <div class="vault-card-actions">
            <button
              class="ghost-button"
              type="button"
              :disabled="syncingRecordId === record.id || !cloudProfile"
              @click="uploadRecord(record)"
            >
              {{ syncingRecordId === record.id ? "同步中" : "同步此记录" }}
            </button>
          </div>
        </div>
      </div>
      <p v-if="syncMessage" class="vault-sync-message">{{ syncMessage }}</p>
    </section>

    <section v-if="rewrittenRecords.length" class="panel vault-lineage">
      <div class="panel__header">
        <div>
          <h3>重写链路</h3>
        </div>
        <span class="pill">{{ rewrittenRecords.length }} 条</span>
      </div>

      <div class="vault-lineage__list">
        <article
          v-for="record in rewrittenRecords"
          :key="`lineage-${record.id}`"
          class="vault-lineage__item"
          :class="{ 'vault-lineage__item--selected': selectedLineageRecord?.id === record.id }"
          role="button"
          tabindex="0"
          @click="openLineage(record)"
          @keydown.enter.prevent="openLineage(record)"
        >
          <div>
            <strong>{{ record.fileName }}</strong>
            <p>第 {{ record.revision }} 次写入</p>
          </div>
          <div class="vault-lineage__chain">
            <span>{{ record.parentWatermarkUid ?? "未知父级" }}</span>
            <span aria-hidden="true">→</span>
            <span>{{ record.watermarkUid }}</span>
          </div>
          <p v-if="record.rewriteReason" class="vault-lineage__reason">
            {{ record.rewriteReason }}
          </p>
        </article>
      </div>

      <aside v-if="selectedLineageRecord" class="vault-lineage-drawer" aria-label="重写链路详情">
        <div class="vault-lineage-drawer__header">
          <div>
            <strong>链路详情</strong>
            <p>{{ selectedLineageRecord.fileName }}</p>
          </div>
          <button class="ghost-button" type="button" @click="closeLineage">
            关闭
          </button>
        </div>

        <div class="vault-lineage-drawer__grid">
          <span>当前 UID</span>
          <b>{{ selectedLineageRecord.watermarkUid }}</b>
          <span>父级 UID</span>
          <b>{{ selectedLineageRecord.parentWatermarkUid ?? "未知父级" }}</b>
          <span>写入版本</span>
          <b>第 {{ selectedLineageRecord.revision }} 次</b>
          <span>重写原因</span>
          <b>{{ selectedLineageRecord.rewriteReason ?? "未记录" }}</b>
          <span>入库时间</span>
          <b>{{ selectedLineageRecord.createdAt }}</b>
          <span>原文件哈希</span>
          <b>{{ selectedLineageRecord.originalHash }}</b>
        </div>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.vault-sync-hint {
  margin: 0.25rem 0 0;
  color: var(--text-muted, #8b95a7);
  font-size: 0.85rem;
}

.vault-sync-hint__queue {
  display: inline-block;
  margin-left: 0.5rem;
  color: var(--text-secondary, #aaa);
}

.vault-sync-meta,
.vault-sync-error {
  margin: 0.2rem 0 0;
  color: var(--text-muted, #8b95a7);
  font-size: 0.8rem;
}

.vault-sync-error {
  color: #ffc857;
}

.vault-sync-recovery {
  margin-top: 0.65rem;
  padding: 0.75rem 0.85rem;
  border: 1px solid rgba(255, 200, 87, 0.42);
  border-radius: 8px;
  background: rgba(255, 200, 87, 0.08);
  color: var(--text-primary, #e0e0e0);
  font-size: 0.84rem;
}

.vault-sync-recovery p {
  margin: 0.35rem 0 0;
  color: var(--text-secondary, #aaa);
  line-height: 1.5;
}

.vault-sync-recovery__actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-top: 0.65rem;
}

.vault-sync-actions {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.vault-card-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 0.6rem;
}

.vault-sync-message {
  margin: 0.85rem 0 0;
  padding: 0.65rem 0.8rem;
  border-radius: 8px;
  background: rgba(87, 143, 202, 0.1);
  border: 1px solid rgba(87, 143, 202, 0.22);
  color: var(--text-secondary, #aaa);
  font-size: 0.86rem;
}

.vault-lineage__list {
  display: grid;
  gap: 0.75rem;
}

.vault-lineage__item {
  padding: 0.9rem;
  border-radius: 10px;
  border: 1px solid rgba(87, 143, 202, 0.26);
  background: rgba(87, 143, 202, 0.07);
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.vault-lineage__item:hover,
.vault-lineage__item--selected {
  border-color: rgba(87, 143, 202, 0.55);
  background: rgba(87, 143, 202, 0.13);
}

.vault-lineage__item strong {
  display: block;
  margin-bottom: 0.25rem;
}

.vault-lineage__item p {
  margin: 0;
  color: var(--text-muted, #8b95a7);
  font-size: 0.85rem;
}

.vault-lineage__chain {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-top: 0.7rem;
  font-family: monospace;
  font-size: 0.82rem;
  color: var(--text-primary, #e0e0e0);
  word-break: break-all;
}

.vault-lineage__reason {
  margin-top: 0.6rem !important;
}

.vault-lineage-drawer {
  margin-top: 1rem;
  padding: 1rem;
  border-radius: 10px;
  border: 1px solid rgba(198, 91, 32, 0.28);
  background: rgba(198, 91, 32, 0.08);
}

.vault-lineage-drawer__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 0.85rem;
}

.vault-lineage-drawer__header p {
  margin: 0.25rem 0 0;
  color: var(--text-muted, #8b95a7);
  font-size: 0.86rem;
}

.vault-lineage-drawer__grid {
  display: grid;
  grid-template-columns: 7rem 1fr;
  gap: 0.55rem 0.9rem;
  font-size: 0.86rem;
  line-height: 1.5;
}

.vault-lineage-drawer__grid span {
  color: var(--text-muted, #8b95a7);
}

.vault-lineage-drawer__grid b {
  color: var(--text-primary, #e0e0e0);
  word-break: break-all;
}
</style>
