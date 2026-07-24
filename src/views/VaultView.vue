<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import {
  buildCopyrightSummary,
  checkFilesExist,
  createReportPurchaseSession,
  exportPublicRightsEmbeddedImage,
  exportVaultBatchSummaryReport,
  exportVaultFormalReport,
  fetchPublicRightsMetadata,
  formatCopyrightDateTime,
  formatEvidenceTime,
  formatLocalDateTime,
  formatOutputStrategy,
  formatPayloadAuthStatus,
  formatRegistryStatus,
  formatTimeProofService,
  formatTrainingPermissionDeclaration,
  formatWatermarkIssueMode,
  formatWorkSourceDeclaration,
  flushDesktopCloudSyncQueue,
  getReportPurchaseSessionStatus,
  getDesktopCloudQueueStatus,
  getDesktopCloudSyncProfile,
  listVaultRecords,
  openOutputDir,
  pushSavedDesktopVaultRecordToCloud,
  pullSavedCloudChangesIntoDesktop,
  reconcileReportPurchaseSession,
  repairWatermarkRecordReissue,
  signOutDesktopCloud,
  supplementVaultTrustedTime,
  verifyFormalReportBundle,
  type CloudQueueStatus,
  type DesktopCloudSyncProfile,
  type EntitlementState,
  type FormalReportBundleVerificationResult,
  type FormalReportExportResult,
  type PublicRightsQueryResponse,
  type ReportPurchaseSession,
  type VaultRecord,
} from "../lib/tauri-api";
import { createPublicRightsScanner, type PublicRightsSdkResult } from "../lib/public-rights-sdk";
import {
  loadRecentReportExports,
  saveRecentReportExport,
} from "../lib/report-export-history";
import { userFacingErrorMessage } from "../lib/user-facing-errors";

const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const isMockPaymentMode =
  import.meta.env.DEV && import.meta.env.VITE_HIDDENSHIELD_ENABLE_MOCK_PAYMENT === "true";

const emit = defineEmits<{
  openSettings: [];
  openSubscription: [];
}>();

const records = ref<VaultRecord[]>([]);
const missingPaths = ref<Set<string>>(new Set());
const selectedRecord = ref<VaultRecord | null>(null);
const selectedLineageRecord = ref<VaultRecord | null>(null);
const cloudProfile = ref<DesktopCloudSyncProfile | null>(null);
const cloudQueueStatus = ref<CloudQueueStatus>({
  pending: 0,
  syncing: 0,
  failed: 0,
  blocked: 0,
  synced: 0,
  retryExhausted: 0,
  staleRecovered: 0,
  lastAttemptAt: null,
  lastSuccessAt: null,
  lastFailureAt: null,
  nextRetryAt: null,
  lastError: null,
  lastErrorCode: null,
  lastHttpStatus: null,
  blockedReason: null,
});
const syncingRecordId = ref<number | null>(null);
const exportingReportRecordId = ref<number | null>(null);
const exportingBatchReport = ref(false);
const flushingCloud = ref(false);
const pullingCloud = ref(false);
const purchasingReportRecordId = ref<number | null>(null);
const repairingRecordId = ref<number | null>(null);
const supplementingTrustedTimeRecordId = ref<number | null>(null);
const syncMessage = ref("");
const publicRightsResult = ref<PublicRightsSdkResult | null>(null);
const publicRightsLoading = ref(false);
const publicRightsError = ref("");
const exportingPublicRightsMetadata = ref(false);
const exportingEmbeddedPublicRightsImage = ref(false);
const recentReportExports = ref<FormalReportExportResult[]>(loadRecentReportExports());
const reportVerificationResults = ref<Record<string, FormalReportBundleVerificationResult>>({});
const verifyingReportId = ref<string | null>(null);
const canExportFormalReports = computed(() => props.entitlementState?.features?.report_export === true);

const rewrittenRecords = computed(() =>
  records.value.filter(record => record.revision > 1 || record.parentWatermarkUid),
);

const selectedRecordTimeline = computed(() => {
  const record = selectedRecord.value;
  if (!record) return [];
  const timeline = [
    { label: "创建记录", value: formatCopyrightDateTime(record.createdAt) },
    {
      label: "保护副本验证",
      value: `${verificationStatus(record.writeVerificationStatus)}${
        verificationMessage(record) ? ` · ${verificationMessage(record)}` : ""
      }`,
    },
  ];
  if (record.tsaTokenPath) {
    timeline.push(
      { label: "第三方时间证明", value: "已获取第三方时间戳回执" },
      { label: "可信时间", value: formatCopyrightDateTime(record.networkTime || record.createdAt) },
      { label: "时间证明服务", value: formatTimeProofService(record.tsaSource) },
    );
  } else if (record.networkTime) {
    timeline.push(
      { label: "时间依据", value: networkTimeSource(record) },
      { label: "网络授时时间", value: formatCopyrightDateTime(record.networkTime) },
      { label: "第三方时间证明", value: "未获取" },
    );
  } else {
    timeline.push(
      { label: "时间依据", value: "本机系统时间" },
      { label: "第三方时间证明", value: "未获取" },
    );
  }
  return timeline;
});

const cloudQueueSummary = computed(() => {
  const pending = cloudQueueStatus.value.pending;
  const failed = cloudQueueStatus.value.failed;
  const blocked = cloudQueueStatus.value.blocked;
  if (blocked > 0) return `云同步已阻断 ${blocked} 条`;
  if (pending === 0 && failed === 0) return "已全部同步";
  return `待同步 ${pending} 条 · 失败 ${failed} 条`;
});

const cloudQueueRetrySummary = computed(() => {
  const queue = cloudQueueStatus.value;
  if (queue.blocked > 0 || queue.blockedReason === "blocked_by_entitlement") {
    return "云同步已被后端权益快照阻断";
  }
  if (queue.failed === 0 && queue.pending > 0) return "等待上传，尚未完成云同步";
  if (queue.failed === 0) return "同步状态正常";
  if (queue.retryExhausted > 0 && queue.retryExhausted === queue.failed) {
    return "部分记录需要手动重试";
  }
  if (queue.nextRetryAt) {
    return `下次自动重试：${formatSyncTime(queue.nextRetryAt)}`;
  }
  return "失败记录可立即重试";
});

const canUseCloudSync = computed(() => cloudProfile.value?.entitlementFeatures?.cloud_sync === true);
const publicRightsStatusText = computed(() => {
  if (!cloudProfile.value) return "未连接公开 registry";
  if (publicRightsLoading.value) return "正在查询";
  if (publicRightsError.value) return "查询失败";
  if (!publicRightsResult.value?.scan) return "尚未查询";
  return formatPublicRightsScanStatus(publicRightsResult.value.scan.scanStatus);
});

const publicRightsSnapshot = computed<PublicRightsQueryResponse | null>(() => publicRightsResult.value?.scan ?? null);

const publicRightsMessage = computed(() =>
  publicRightsResult.value?.message ?? "公开查询只展示创作者声明与 registry 快照，不直接判断是否可训练。",
);

const registryAttentionRecords = computed(() =>
  records.value.filter(record =>
    ["pending_registry_reconcile", "conflict", "reissue_required"].includes(record.watermarkIdRegistryStatus),
  ),
);

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
  return formatLocalDateTime(value, "无");
}

function buildCloudDiagnosticsText(): string {
  const profile = cloudProfile.value;
  const queue = cloudQueueStatus.value;
  return [
    "HiddenShield 同步状态信息",
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
    `待同步记录: ${queue.pending}`,
    `同步中记录: ${queue.syncing}`,
    `同步失败记录: ${queue.failed}`,
    `权益阻断记录: ${queue.blocked}`,
    `已同步记录: ${queue.synced}`,
    `中断恢复记录: ${queue.staleRecovered}`,
    `重试状态: ${queue.retryExhausted > 0 ? `${queue.retryExhausted} 条已达上限` : "正常"}`,
    `下次自动重试: ${queue.nextRetryAt ? formatSyncTime(queue.nextRetryAt) : "无"}`,
    `最近尝试: ${formatSyncTime(queue.lastAttemptAt)}`,
    `最近成功: ${formatSyncTime(queue.lastSuccessAt)}`,
    `最近失败: ${formatSyncTime(queue.lastFailureAt)}`,
    `最近错误码: ${queue.lastErrorCode ?? "无"}`,
    `最近 HTTP 状态: ${queue.lastHttpStatus ?? "无"}`,
    `阻断原因: ${queue.blockedReason ?? "无"}`,
    `最近错误: ${queue.lastError ?? "无"}`,
  ].join("\n");
}

async function copyCloudDiagnostics() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中登录 HiddenShield 账户";
    return;
  }
  await navigator.clipboard.writeText(buildCloudDiagnosticsText());
  syncMessage.value = "同步状态信息已复制到剪贴板";
}

async function copyLineageSummary(record: VaultRecord) {
  await navigator.clipboard.writeText(buildCopyrightSummary(record));
  syncMessage.value = "版本摘要已复制到剪贴板";
}

async function handleExportVaultReport(record: VaultRecord) {
  exportingReportRecordId.value = record.id;
  try {
    const result = await exportVaultFormalReport(record.id);
    rememberReportExport(result);
    syncMessage.value = `已导出 ${record.fileName} 的正式报告`;
  } catch (error: unknown) {
    console.warn("vault formal report export failed", error);
    syncMessage.value = userFacingErrorMessage(error, "导出正式报告");
  } finally {
    exportingReportRecordId.value = null;
  }
}

function reportProductLabel(productCode: string) {
  return productCode === "rights_evidence_pack_single" ? "维权证据包" : "版权详细报告";
}

function formatReportPrice(priceCents: number) {
  return `${(priceCents / 100).toFixed(1)} 元`;
}

async function startSingleReportPurchase(
  record: VaultRecord,
  productCode: "copyright_report_single" | "rights_evidence_pack_single",
) {
  purchasingReportRecordId.value = record.id;
  try {
    const session = await createReportPurchaseSession(
      record.id,
      productCode,
      isMockPaymentMode ? "fixture" : undefined,
    );
    await confirmSingleReportPurchase(record, session);
  } catch (error: unknown) {
    console.warn("create report purchase session failed", error);
    syncMessage.value = userFacingErrorMessage(error, "创建报告购买会话");
  } finally {
    purchasingReportRecordId.value = null;
  }
}

async function confirmSingleReportPurchase(record: VaultRecord, session: ReportPurchaseSession) {
  try {
    const status = await getReportPurchaseSessionStatus(session.paymentSessionId);
    if (status.grant) {
      const result = await exportVaultFormalReport(record.id);
      rememberReportExport(result);
      syncMessage.value = `已导出 ${record.fileName} 的${reportProductLabel(session.productCode)}`;
      return;
    }
    const reconciled = await reconcileReportPurchaseSession(session.paymentSessionId);
    if (!reconciled.grant) {
      syncMessage.value = "暂未确认支付完成，可稍后再次确认购买状态";
      return;
    }
    const result = await exportVaultFormalReport(record.id);
    rememberReportExport(result);
    syncMessage.value = `已购买并导出 ${record.fileName} 的${reportProductLabel(session.productCode)}`;
  } catch (error: unknown) {
    console.warn("reconcile report purchase failed", error);
    syncMessage.value = userFacingErrorMessage(error, "确认报告购买状态");
  }
}

async function handleExportBatchSummary() {
  if (!canExportFormalReports.value) {
    syncMessage.value = "当前发布基线不提供批量正式报告；正式报告必须按记录单独购买";
    return;
  }
  exportingBatchReport.value = true;
  try {
    const result = await exportVaultBatchSummaryReport();
    rememberReportExport(result);
    syncMessage.value = `已导出 ${result.recordCount} 条记录的批量摘要`;
  } catch (error: unknown) {
    console.warn("vault batch report export failed", error);
    syncMessage.value = userFacingErrorMessage(error, "导出批量摘要");
  } finally {
    exportingBatchReport.value = false;
  }
}

function rememberReportExport(result: FormalReportExportResult) {
  recentReportExports.value = saveRecentReportExport(result);
}

async function openReportDir(result: FormalReportExportResult) {
  await openOutputDir(result.reportDir);
}

async function copyReportPath(path: string, label: string) {
  await navigator.clipboard.writeText(path);
  syncMessage.value = `${label}路径已复制`;
}

async function verifyReportBundle(item: FormalReportExportResult) {
  verifyingReportId.value = item.reportId;
  syncMessage.value = "";
  try {
    const result = await verifyFormalReportBundle(item.reportDir);
    reportVerificationResults.value = {
      ...reportVerificationResults.value,
      [item.reportId]: result,
    };
    syncMessage.value = result.message;
  } catch (error: unknown) {
    console.warn("verify formal report bundle failed", error);
    syncMessage.value = userFacingErrorMessage(error, "校验报告包");
  } finally {
    verifyingReportId.value = null;
  }
}

function reportTypeLabel(type: string) {
  return type === "batch_summary" ? "批量摘要" : "正式报告";
}

async function resetCloudAccountForRecovery() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中登录 HiddenShield 账户";
    return;
  }
  await signOutDesktopCloud();
  cloudProfile.value = null;
  syncMessage.value = "已退出当前云同步账户，请在设置中重新登录账户";
  emit("openSettings");
  await loadCloudState();
}

onMounted(async () => {
  await loadVault();
  await loadCloudState();

  // Lazily check which output files still exist
  const allPaths: string[] = [];
  for (const r of records.value) {
    const path = r.protectedCopyPath || r.outputDouyin || r.outputBilibili || r.outputXhs;
    if (path) allPaths.push(path);
  }
  if (allPaths.length > 0) {
    const missing = await checkFilesExist(allPaths);
    missingPaths.value = new Set(missing);
  }
});

async function loadVault() {
  const list = await listVaultRecords();
  const visibleRecords = list.filter((record) =>
    record.outputStrategy !== "video_audio_track" &&
    !record.videoNotaryId &&
    !record.videoVisualTaskId &&
    !record.videoVisualMediaHash
  );
  records.value = visibleRecords;
  selectedRecord.value = selectedRecord.value
    ? visibleRecords.find((record) => record.id === selectedRecord.value?.id) ?? visibleRecords[0] ?? null
    : visibleRecords[0] ?? null;
}

async function loadCloudState() {
  const [profile, queueStatus] = await Promise.all([
    getDesktopCloudSyncProfile(),
    getDesktopCloudQueueStatus(),
  ]);
  cloudProfile.value = profile;
  cloudQueueStatus.value = queueStatus;
  await loadSelectedPublicRights();
}

function isRecordOffline(record: VaultRecord): boolean {
  if (record.videoVisualTaskId || record.videoVisualMediaHash) return false;
  if (record.videoNotaryId) return false;
  const outputs = [record.protectedCopyPath || record.outputDouyin || record.outputBilibili || record.outputXhs].filter(Boolean) as string[];
  if (outputs.length === 0) return false;
  return outputs.every(p => missingPaths.value.has(p));
}

function recordKindLabel(record: VaultRecord) {
  if (record.videoVisualTaskId || record.videoVisualMediaHash) return "L3 视频画面盲水印";
  if (record.videoNotaryId) return "视频指纹存证";
  const lower = record.fileName.toLowerCase();
  if (/\.(mp3|wav|flac|aac|ogg|m4a)$/.test(lower)) return "音频";
  if (/\.(mp4|mov|mkv|webm)$/.test(lower)) return "视频";
  return "图片";
}

function displayValue(value?: string | null): string {
  return value?.trim() || "未记录";
}

function displayCreatorName(value?: string | null): string {
  return value?.trim() || "未声明";
}

function verificationStatus(status?: string | null): string {
  if (status === "verified") return "已通过";
  if (status === "failed") return "未通过";
  return "未完成";
}

function verificationMessage(record: VaultRecord): string {
  if (record.writeVerificationStatus === "verified") {
    return "已从保护副本回读并验证版权编号，可由 HiddenShield 再次读取验证";
  }
  return record.writeVerificationStatus
    ? record.writeVerificationMessage?.trim() || ""
    : "";
}

function registryReceipt(record: VaultRecord): string {
  const confirmed = ["server_confirmed", "offline_confirmed"].includes(
    record.watermarkIdRegistryStatus,
  );
  return confirmed ? record.watermarkIdRegistryReceipt?.trim() || "" : "";
}

function networkTimeSource(record: VaultRecord): string {
  const source = record.tsaSource?.trim();
  return source ? `网络授时服务（${source}）` : "网络授时服务";
}

async function supplementTrustedTime(record: VaultRecord) {
  supplementingTrustedTimeRecordId.value = record.id;
  syncMessage.value = "";
  try {
    const updated = await supplementVaultTrustedTime(record.id);
    records.value = records.value.map((item) => item.id === updated.id ? updated : item);
    selectedRecord.value = updated;
    if (selectedLineageRecord.value?.id === updated.id) {
      selectedLineageRecord.value = updated;
    }
    syncMessage.value = "可信时间材料已补充。";
  } catch (error: unknown) {
    syncMessage.value = userFacingErrorMessage(error, "补充可信时间");
  } finally {
    supplementingTrustedTimeRecordId.value = null;
  }
}

function formatPublicRightsScanStatus(value?: string | null): string {
  switch (value) {
    case "registry_active":
      return "registry 已生效";
    case "watermark_only":
      return "仅识别到水印锚点";
    case "registry_revoked":
      return "registry 已撤销";
    case "registry_superseded":
      return "registry 已被替代";
    case "backfill_disputed":
      return "需要人工复核";
    default:
      return value?.trim() || "未记录";
  }
}

function formatAnchorProtocol(value?: string | null): string {
  switch (value) {
    case "v3_minimal_anchor":
      return "V3 最小媒体锚点";
    case "v2_migration_anchor":
      return "V2 迁移桥接锚点";
    default:
      return value?.trim() || "未记录";
  }
}

function formatMediaPayloadRole(value?: string | null): string {
  switch (value) {
    case "minimal_media_anchor":
      return "最小媒体锚点";
    case "legacy_bridge_anchor":
      return "旧记录桥接锚点";
    default:
      return value?.trim() || "未记录";
  }
}

async function loadSelectedPublicRights() {
  const record = selectedRecord.value;
  const profile = cloudProfile.value;
  publicRightsResult.value = null;
  publicRightsError.value = "";
  if (!record || !profile?.cloudBaseUrl) return;
  publicRightsLoading.value = true;
  try {
    publicRightsResult.value = await createPublicRightsScanner(profile.cloudBaseUrl).scanOne(record.watermarkUid);
  } catch (error: unknown) {
    console.warn("public rights query failed", error);
    publicRightsError.value = userFacingErrorMessage(error, "查询公开权利信号");
  } finally {
    publicRightsLoading.value = false;
  }
}

async function exportSelectedPublicRightsMetadata() {
  const record = selectedRecord.value;
  const profile = cloudProfile.value;
  if (!record || !profile?.cloudBaseUrl) return;
  exportingPublicRightsMetadata.value = true;
  try {
    const metadata = await fetchPublicRightsMetadata(profile.cloudBaseUrl, record.watermarkUid);
    downloadJson(
      `hiddenshield-public-rights-${safeFilePart(record.watermarkUid)}.json`,
      metadata,
    );
    syncMessage.value = "公开元数据 JSON 已导出";
  } catch (error) {
    syncMessage.value = userFacingErrorMessage(error, "导出公开元数据 JSON");
  } finally {
    exportingPublicRightsMetadata.value = false;
  }
}

async function exportSelectedEmbeddedPublicRightsImage() {
  const record = selectedRecord.value;
  const profile = cloudProfile.value;
  if (!record || !profile?.cloudBaseUrl) return;
  exportingEmbeddedPublicRightsImage.value = true;
  try {
    const metadata = await fetchPublicRightsMetadata(profile.cloudBaseUrl, record.watermarkUid);
    const result = await exportPublicRightsEmbeddedImage(record.id, metadata);
    syncMessage.value = `已导出嵌入公开元数据的图片副本：${result.outputPath}`;
    await openOutputDir(result.outputDir);
  } catch (error) {
    syncMessage.value = userFacingErrorMessage(error, "导出嵌入公开元数据图片副本");
  } finally {
    exportingEmbeddedPublicRightsImage.value = false;
  }
}

function downloadJson(fileName: string, value: unknown) {
  const blob = new Blob([`${JSON.stringify(value, null, 2)}\n`], {
    type: "application/json;charset=utf-8",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function safeFilePart(value: string) {
  return value.trim().replace(/[^a-zA-Z0-9._-]+/g, "_") || "unknown";
}

watch(
  () => [selectedRecord.value?.watermarkUid, cloudProfile.value?.cloudBaseUrl],
  () => {
    void loadSelectedPublicRights();
  },
);

async function uploadRecord(record: VaultRecord) {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中登录 HiddenShield 账户";
    return;
  }
  if (!canUseCloudSync.value) {
    syncMessage.value = "当前云端账户未开放同步，仍可继续本地使用";
    return;
  }
  syncingRecordId.value = record.id;
  try {
    const result = await pushSavedDesktopVaultRecordToCloud(record.id);
    syncMessage.value = result.accepted > 0
      ? `已同步 ${record.fileName} 的版权记录`
      : "已加入待同步列表，稍后可重试";
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
    syncMessage.value = "请先在设置中登录 HiddenShield 账户";
    return;
  }
  if (!canUseCloudSync.value) {
    syncMessage.value = "当前云端账户未开放同步，仍可继续本地使用";
    return;
  }
  flushingCloud.value = true;
  try {
    const result = await flushDesktopCloudSyncQueue(50);
    syncMessage.value = `${result.message}（尝试 ${result.attempted} 条）`;
    await loadCloudState();
  } catch (e: unknown) {
    syncMessage.value = userFacingErrorMessage(e, "更新云端记录");
    await loadCloudState();
  } finally {
    flushingCloud.value = false;
  }
}

async function pullCloudChanges() {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中登录 HiddenShield 账户";
    return;
  }
  if (!canUseCloudSync.value) {
    syncMessage.value = "当前云端账户未开放同步，仍可继续本地使用";
    return;
  }
  pullingCloud.value = true;
  try {
    const result = await pullSavedCloudChangesIntoDesktop();
    syncMessage.value = `已更新 ${result.totalChanges} 条云端记录，保存 ${result.applied} 条，跳过 ${result.skipped} 条`;
    await loadVault();
    await loadCloudState();
  } catch (e: unknown) {
    syncMessage.value = userFacingErrorMessage(e, "拉取云端记录");
    await loadCloudState();
  } finally {
    pullingCloud.value = false;
  }
}

async function repairRegistryRecord(record: VaultRecord) {
  if (!cloudProfile.value) {
    syncMessage.value = "请先在设置中登录 HiddenShield 账户，再执行编号重新签发";
    return;
  }
  repairingRecordId.value = record.id;
  try {
    const result = await repairWatermarkRecordReissue(record.id);
    syncMessage.value = result.message;
    await loadVault();
    await loadCloudState();
    selectedRecord.value = records.value.find(item => item.id === record.id) ?? selectedRecord.value;
  } catch (error: unknown) {
    console.warn("watermark reissue repair failed", error);
    syncMessage.value = userFacingErrorMessage(error, "修复版权编号");
    await loadVault();
  } finally {
    repairingRecordId.value = null;
  }
}
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">版权库</p>
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
          <p v-if="cloudProfile && !canUseCloudSync" class="vault-sync-meta">
            当前账户已登录，云同步是否开放由服务端账户权益决定。
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
            <p>当前账户、设备或工作区与云端不一致。请重新登录账户以刷新授权和工作区绑定。</p>
            <div class="vault-sync-recovery__actions">
              <button class="ghost-button" type="button" @click="emit('openSettings')">
                打开设置
              </button>
              <button class="ghost-button" type="button" @click="resetCloudAccountForRecovery">
                退出并重新登录
              </button>
            </div>
          </div>
          <p v-if="!cloudProfile" class="vault-sync-hint">
            云同步未连接，设置中登录账户后可同步版权记录。
          </p>
        </div>
        <div class="vault-sync-actions">
          <button
            class="ghost-button"
            type="button"
            :disabled="flushingCloud || !cloudProfile || !canUseCloudSync || (cloudQueueStatus.pending === 0 && cloudQueueStatus.failed === 0)"
            @click="flushCloudQueue"
          >
            {{ flushingCloud ? "同步中" : "同步待处理记录" }}
          </button>
          <button
            class="ghost-button"
            type="button"
            :disabled="pullingCloud || !cloudProfile || !canUseCloudSync"
            @click="pullCloudChanges"
          >
            {{ pullingCloud ? "更新中" : "更新云端记录" }}
          </button>
          <button
            class="ghost-button"
            type="button"
            :disabled="!cloudProfile"
            @click="copyCloudDiagnostics"
          >
            复制同步信息
          </button>
          <span class="pill">{{ records.length }} 条</span>
        </div>
      </div>

      <!-- Subscription features -->
      <div class="vault-pro-actions">
        <button
          class="ghost-button"
          type="button"
          :disabled="exportingBatchReport"
          @click="handleExportBatchSummary"
        >
          {{ exportingBatchReport ? "导出中" : canExportFormalReports ? "导出批量摘要" : "当前不提供批量正式报告" }}
        </button>
        <ProBadge label="批量处理" :disabled="true" />
      </div>

      <div v-if="registryAttentionRecords.length" class="registry-arbitration-panel">
        <div class="registry-arbitration-panel__header">
          <div>
            <span class="registry-arbitration-panel__label">登记仲裁</span>
            <strong>需要处理的版权编号</strong>
            <p>
              同一版权编号出现不同作品指纹、后端返回冲突，或历史记录需要重新签发时，必须修复保护副本 payload 后再作为已登记记录使用。
            </p>
          </div>
          <span class="pill">{{ registryAttentionRecords.length }} 条</span>
        </div>
        <article
          v-for="record in registryAttentionRecords"
          :key="`registry-${record.id}`"
          class="registry-arbitration-item"
        >
          <div>
            <b>{{ record.fileName }}</b>
            <span>{{ record.watermarkUid }}</span>
            <small>
              {{ formatRegistryStatus(record.watermarkIdRegistryStatus) }} · 作品指纹 {{ record.originalHash.slice(0, 16) }}...
            </small>
          </div>
          <div class="registry-arbitration-item__actions">
            <button class="ghost-button" type="button" @click="selectedRecord = record">
              查看详情
            </button>
            <button
              class="primary-button"
              type="button"
              :disabled="repairingRecordId === record.id || !cloudProfile"
              @click="repairRegistryRecord(record)"
            >
              {{ repairingRecordId === record.id ? "修复中" : "重新签发并修复保护副本" }}
            </button>
          </div>
        </article>
      </div>

      <div class="vault-record-workspace">
        <div class="vault-record-table-wrap">
          <table class="vault-record-table">
            <thead>
              <tr>
                <th>作品对象</th>
                <th>类型</th>
                <th>权益状态</th>
                <th>版本</th>
                <th>同步</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="record in records"
                :key="record.id"
                :class="{ 'vault-record-table__row--active': selectedRecord?.id === record.id }"
                @click="selectedRecord = record"
              >
                <td>
                  <strong>{{ record.fileName }}</strong>
                  <span class="hash-text">{{ record.watermarkUid }}</span>
                  <small v-if="isRecordOffline(record)">本机文件缺失</small>
                </td>
                <td>{{ recordKindLabel(record) }}</td>
                <td>
                  <span
                    class="vault-verification-badge"
                    :class="{
                      'vault-verification-badge--ok': record.writeVerificationStatus === 'verified',
                      'vault-verification-badge--warn': record.writeVerificationStatus === 'failed',
                    }"
                  >
                    {{ record.writeVerificationStatus === 'verified' ? '已验证' : record.writeVerificationStatus === 'failed' ? '需复核' : '未记录' }}
                  </span>
                  <span v-if="record.videoVisualTaskId || record.videoVisualMediaHash" class="vault-video-notary-badge">L3 画面</span>
                  <span v-if="record.videoNotaryId" class="vault-video-notary-badge">L2 存证</span>
                  <span
                    v-if="['pending_registry_reconcile', 'conflict', 'reissue_required'].includes(record.watermarkIdRegistryStatus)"
                    class="vault-registry-badge"
                  >
                    {{ formatRegistryStatus(record.watermarkIdRegistryStatus) }}
                  </span>
                </td>
                <td>第 {{ record.revision }} 次</td>
                <td>{{ cloudProfile ? (canUseCloudSync ? "可同步" : "云端未授权") : "本地" }}</td>
              </tr>
            </tbody>
          </table>
          <div v-if="records.length === 0" class="hs-empty-state">版权库暂无记录。完成一次写入后会自动入库。</div>
        </div>

        <aside class="vault-detail-panel" aria-label="版权记录详情">
          <template v-if="selectedRecord">
            <div class="vault-detail-panel__header">
              <span>{{ recordKindLabel(selectedRecord) }}</span>
              <h3>{{ selectedRecord.fileName }}</h3>
              <p class="hash-text">{{ selectedRecord.watermarkUid }}</p>
            </div>

            <CopyrightCard :record="selectedRecord" :highlight="true" />

            <div
              v-if="selectedRecord.videoVisualTaskId || selectedRecord.videoVisualMediaHash"
              class="vault-detail-panel__grid"
            >
              <template>
                <span>L3 任务</span>
                <strong>{{ displayValue(selectedRecord.videoVisualTaskId) }}</strong>
                <span>L3 完成时间</span>
                <strong>{{ formatEvidenceTime(selectedRecord.videoVisualCompletedAt) }}</strong>
                <span>L3 策略摘要</span>
                <strong class="hash-text">{{ displayValue(selectedRecord.videoVisualStrategyDigest) }}</strong>
                <span>L3 成品摘要</span>
                <strong class="hash-text">{{ displayValue(selectedRecord.videoVisualMediaHash) }}</strong>
                <span>L3 Worker 收据</span>
                <strong class="hash-text">{{ displayValue(selectedRecord.videoVisualReceiptHash) }}</strong>
                <span>L3 自检置信度</span>
                <strong>{{ selectedRecord.videoVisualSelfCheckConfidence ?? "未记录" }}</strong>
                <span>L3 自检阈值</span>
                <strong>{{ selectedRecord.videoVisualSelfCheckThreshold ?? "未记录" }}</strong>
                <span>L3 检查帧数</span>
                <strong>{{ selectedRecord.videoVisualCheckedFrames ?? "未记录" }}</strong>
                <span>L3 成品字节数</span>
                <strong>{{ selectedRecord.videoVisualOutputBytes ?? "未记录" }}</strong>
                <span>L3 成品内容类型</span>
                <strong>{{ displayValue(selectedRecord.videoVisualOutputContentType) }}</strong>
              </template>
            </div>

            <div class="public-rights-card">
              <div class="public-rights-card__header">
                <div>
                  <span>公开权利信号</span>
                  <strong>{{ publicRightsStatusText }}</strong>
                </div>
                <button
                  class="ghost-button"
                  type="button"
                  :disabled="publicRightsLoading || !cloudProfile"
                  @click="loadSelectedPublicRights"
                >
                  {{ publicRightsLoading ? "查询中" : "刷新" }}
                </button>
                <button
                  class="ghost-button"
                  type="button"
                  :disabled="exportingPublicRightsMetadata || !cloudProfile?.cloudBaseUrl"
                  @click="exportSelectedPublicRightsMetadata"
                >
                  {{ exportingPublicRightsMetadata ? "导出中" : "导出公开元数据 JSON" }}
                </button>
                <button
                  class="ghost-button"
                  type="button"
                  :disabled="exportingEmbeddedPublicRightsImage || !cloudProfile?.cloudBaseUrl"
                  @click="exportSelectedEmbeddedPublicRightsImage"
                >
                  {{ exportingEmbeddedPublicRightsImage ? "导出中" : "导出嵌入元数据图片副本" }}
                </button>
              </div>
              <p v-if="!cloudProfile">
                当前未连接云端 registry，只显示本机版权库声明；连接后可公开查询训练许可快照。
              </p>
              <p v-else-if="publicRightsError" class="public-rights-card__error">
                {{ publicRightsError }}
              </p>
              <div v-else-if="publicRightsSnapshot" class="public-rights-card__grid">
                <span>训练许可</span>
                <strong>{{ publicRightsSnapshot.trainingPermission.label }}</strong>
                <span>声明来源</span>
                <strong>{{ publicRightsSnapshot.trainingPermission.effectiveSource }}</strong>
                <span>锚点协议</span>
                <strong>{{ formatAnchorProtocol(publicRightsSnapshot.registry.anchorProtocol) }}</strong>
                <span>媒体角色</span>
                <strong>{{ formatMediaPayloadRole(publicRightsSnapshot.registry.mediaPayloadRole) }}</strong>
                <span>Manifest</span>
                <strong>{{ publicRightsSnapshot.rightsManifest ? `v${publicRightsSnapshot.rightsManifest.manifestVersion}` : "待回填" }}</strong>
                <span>元数据一致性</span>
                <strong>{{ publicRightsSnapshot.publicMetadata.consistency }}</strong>
              </div>
              <p v-if="publicRightsSnapshot?.warnings.length" class="public-rights-card__warning">
                {{ publicRightsSnapshot.warnings.join(" / ") }}
              </p>
              <p class="public-rights-card__note">
                {{ publicRightsMessage }}
              </p>
            </div>

            <div class="vault-detail-panel__actions">
              <button
                v-if="['pending_registry_reconcile', 'conflict', 'reissue_required'].includes(selectedRecord.watermarkIdRegistryStatus)"
                class="primary-button"
                type="button"
                :disabled="repairingRecordId === selectedRecord.id || !cloudProfile"
                @click="repairRegistryRecord(selectedRecord)"
              >
                {{ repairingRecordId === selectedRecord.id ? "修复中" : "重新签发并修复保护副本" }}
              </button>
              <button
                class="ghost-button"
                type="button"
                :disabled="syncingRecordId === selectedRecord.id || !cloudProfile"
                @click="uploadRecord(selectedRecord)"
              >
                {{ syncingRecordId === selectedRecord.id ? "同步中" : "同步此记录" }}
              </button>
              <button
                v-if="!selectedRecord.tsaTokenPath && !selectedRecord.networkTime"
                class="ghost-button"
                type="button"
                :disabled="supplementingTrustedTimeRecordId === selectedRecord.id"
                @click="supplementTrustedTime(selectedRecord)"
              >
                {{ supplementingTrustedTimeRecordId === selectedRecord.id ? "正在获取可信时间" : "补充可信时间" }}
              </button>
            </div>

            <section class="report-commerce-card">
              <div class="report-commerce-card__header">
                <div>
                  <span>{{ isMockPaymentMode ? "开发测试 · Mock 支付" : "报告与维权服务" }}</span>
                  <strong>{{ isMockPaymentMode ? "模拟授权，仅用于本机测试" : "真实支付暂未接入" }}</strong>
                  <p>
                    {{
                      isMockPaymentMode
                        ? "本次操作不会创建订单或扣款，只会为当前记录写入本地测试授权。"
                        : "年度注册码不包含正式报告；真实支付完成前暂不可购买。"
                    }}
                  </p>
                </div>
                <button
                class="primary-button"
                type="button"
                :disabled="exportingReportRecordId === selectedRecord.id"
                @click="handleExportVaultReport(selectedRecord)"
              >
                {{ exportingReportRecordId === selectedRecord.id ? "导出中" : "导出已购买报告" }}
              </button>
              </div>
              <div class="report-commerce-card__products">
                <article>
                  <span>版权详细报告</span>
                  <strong>{{ isMockPaymentMode ? "模拟授权" : "¥19.9" }} <small>{{ isMockPaymentMode ? "不扣款" : "/ 份" }}</small></strong>
                  <p>包含版权信息、验证结果、可信时间与完整性摘要。</p>
                  <button
                class="primary-button report-commerce-card__buy"
                type="button"
                :disabled="purchasingReportRecordId === selectedRecord.id || !isMockPaymentMode"
                @click="startSingleReportPurchase(selectedRecord, 'copyright_report_single')"
              >
                    {{ purchasingReportRecordId === selectedRecord.id ? "处理中" : isMockPaymentMode ? "模拟购买版权详细报告并导出" : "购买版权详细报告（真实支付暂未接入）" }}
              </button>
                </article>
                <article class="report-commerce-card__product--evidence">
                  <span>维权证据包</span>
                  <strong>{{ isMockPaymentMode ? "模拟授权" : "¥49.9" }} <small>{{ isMockPaymentMode ? "不扣款" : "/ 份" }}</small></strong>
                  <p>包含案件材料目录、附件清单、完整性校验与完整导出包。</p>
                  <button
                class="primary-button report-commerce-card__buy"
                type="button"
                :disabled="purchasingReportRecordId === selectedRecord.id || !isMockPaymentMode"
                @click="startSingleReportPurchase(selectedRecord, 'rights_evidence_pack_single')"
              >
                    {{ purchasingReportRecordId === selectedRecord.id ? "处理中" : isMockPaymentMode ? "模拟购买维权证据包并导出" : "购买维权证据包（真实支付暂未接入）" }}
              </button>
                </article>
              </div>
            </section>

            <div class="vault-detail-panel__timeline">
              <h4>记录时间线</h4>
              <article v-for="item in selectedRecordTimeline" :key="item.label">
                <span>{{ item.label }}</span>
                <strong>{{ item.value }}</strong>
              </article>
            </div>
          </template>
          <div v-else class="hs-empty-state">选择一条版权记录查看详情、报告状态和时间线。</div>
        </aside>
      </div>
      <p v-if="syncMessage" class="vault-sync-message">{{ syncMessage }}</p>
      <div v-if="recentReportExports.length" class="report-export-list">
        <strong>最近导出</strong>
        <article
          v-for="item in recentReportExports"
          :key="item.reportId"
          class="report-export-item"
        >
          <div>
            <b>{{ reportTypeLabel(item.reportType) }} · {{ item.recordCount }} 条</b>
            <p>{{ item.pdfPath }}</p>
            <small>
              第 {{ item.bundleVersion }} 版 · {{ item.pdfPageCount }} 页 ·
              {{ Math.round(item.pdfGenerationMs) }} ms
            </small>
            <small v-if="reportVerificationResults[item.reportId]">
              完整性：{{ reportVerificationResults[item.reportId].integrityStatus === "matched" ? "文件匹配" : "校验失败" }}
              · 文档合同：{{ reportVerificationResults[item.reportId].documentContractStatus === "matched" ? "匹配" : "不匹配" }}
              · 签名：{{ reportVerificationResults[item.reportId].signatureStatus === "not_signed" ? "未签名" : "存在但未验证" }}
              · 可信时间：{{ reportVerificationResults[item.reportId].trustedTimeStatus === "not_timestamped" ? "未加盖" : "存在但未验证" }}
            </small>
          </div>
          <div class="report-export-item__actions">
            <button class="ghost-button" type="button" @click="openReportDir(item)">
              打开目录
            </button>
            <button class="ghost-button" type="button" @click="copyReportPath(item.pdfPath, 'PDF')">
              复制 PDF
            </button>
            <button class="ghost-button" type="button" @click="copyReportPath(item.jsonPath, 'JSON')">
              复制 JSON
            </button>
            <button class="ghost-button" type="button" @click="copyReportPath(item.manifestPath, 'Manifest')">
              复制 Manifest
            </button>
            <button
              class="ghost-button"
              type="button"
              :disabled="verifyingReportId === item.reportId"
              @click="verifyReportBundle(item)"
            >
              {{ verifyingReportId === item.reportId ? "校验中…" : "校验报告包" }}
            </button>
          </div>
        </article>
      </div>
    </section>

    <section v-if="rewrittenRecords.length" class="panel vault-lineage">
      <div class="panel__header">
        <div>
          <h3>版本记录</h3>
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
            <span>{{ record.parentWatermarkUid ?? "上一版未记录" }}</span>
            <span aria-hidden="true">→</span>
            <span>{{ record.watermarkUid }}</span>
          </div>
          <p v-if="record.rewriteReason" class="vault-lineage__reason">
            {{ record.rewriteReason }}
          </p>
        </article>
      </div>

      <aside v-if="selectedLineageRecord" class="vault-lineage-drawer" aria-label="版本记录详情">
        <div class="vault-lineage-drawer__header">
          <div>
            <strong>版本详情</strong>
            <p>{{ selectedLineageRecord.fileName }}</p>
          </div>
          <button class="ghost-button" type="button" @click="closeLineage">
            关闭
          </button>
          <button class="ghost-button" type="button" @click="copyLineageSummary(selectedLineageRecord)">
            复制摘要
          </button>
        </div>

        <div class="vault-lineage-drawer__grid">
          <span>版权编号</span>
          <b>{{ selectedLineageRecord.watermarkUid }}</b>
          <span>创作者显示名称</span>
          <b>{{ displayCreatorName(selectedLineageRecord.creatorDisplayName) }}</b>
          <span>身份信息来源</span>
          <b>用户本地声明</b>
          <span>身份核验状态</span>
          <b>未进行实名认证</b>
          <span>记录类型</span>
          <b>{{ recordKindLabel(selectedLineageRecord) }}</b>
          <template v-if="selectedLineageRecord.videoNotaryId">
            <span>存证编号</span>
            <b>{{ selectedLineageRecord.videoNotaryId }}</b>
            <span>存证时间</span>
            <b>{{ selectedLineageRecord.videoNotaryAt ?? "未记录" }}</b>
            <span>指纹根</span>
            <b>{{ selectedLineageRecord.videoFingerprintRoot ?? "未记录" }}</b>
            <span>指纹包摘要</span>
            <b>{{ selectedLineageRecord.videoBundleSha256 ?? "未记录" }}</b>
            <span>采样策略</span>
            <b>{{ selectedLineageRecord.videoFrameSamplePolicy ?? "未记录" }}</b>
          </template>
          <span>上一版编号</span>
          <b>{{ selectedLineageRecord.parentWatermarkUid ?? "上一版未记录" }}</b>
          <span>写入次数</span>
          <b>第 {{ selectedLineageRecord.revision }} 次</b>
          <span>更新说明</span>
          <b>{{ selectedLineageRecord.rewriteReason ?? "未记录" }}</b>
          <span>保护副本验证</span>
          <b>{{ verificationStatus(selectedLineageRecord.writeVerificationStatus) }}</b>
          <template v-if="verificationMessage(selectedLineageRecord)">
            <span>验证说明</span>
            <b>{{ verificationMessage(selectedLineageRecord) }}</b>
          </template>
          <span>版权编号生成方式</span>
          <b>{{ formatWatermarkIssueMode(selectedLineageRecord.watermarkIdIssueMode) }}</b>
          <span>联网登记状态</span>
          <b>{{ formatRegistryStatus(selectedLineageRecord.watermarkIdRegistryStatus) }}</b>
          <template v-if="registryReceipt(selectedLineageRecord)">
            <span>登记收据编号</span>
            <b>{{ registryReceipt(selectedLineageRecord) }}</b>
          </template>
          <span>时间依据</span>
          <b v-if="selectedLineageRecord.tsaTokenPath">第三方时间戳回执</b>
          <b v-else-if="selectedLineageRecord.networkTime">{{ networkTimeSource(selectedLineageRecord) }}</b>
          <b v-else>本机系统时间</b>
          <template v-if="selectedLineageRecord.networkTime && !selectedLineageRecord.tsaTokenPath">
            <span>网络授时时间</span>
            <b>{{ formatCopyrightDateTime(selectedLineageRecord.networkTime) }}</b>
          </template>
          <span>第三方时间证明</span>
          <b>{{ selectedLineageRecord.tsaTokenPath ? "已获取第三方时间戳回执" : "未获取" }}</b>
          <template v-if="selectedLineageRecord.tsaTokenPath">
            <span>可信时间</span>
            <b>{{ formatCopyrightDateTime(selectedLineageRecord.networkTime || selectedLineageRecord.createdAt) }}</b>
            <span>时间证明服务</span>
            <b>{{ formatTimeProofService(selectedLineageRecord.tsaSource) }}</b>
          </template>
          <span>记录创建时间</span>
          <b>{{ formatCopyrightDateTime(selectedLineageRecord.createdAt) }}</b>
          <span>作品指纹（SHA-256）</span>
          <b>{{ selectedLineageRecord.originalHash }}</b>
          <span>水印协议版本</span>
          <b>V{{ selectedLineageRecord.payloadProtocolVersion }}</b>
          <span>载荷完整性校验</span>
          <b>{{ formatPayloadAuthStatus(selectedLineageRecord.payloadAuthStatus) }}</b>
        </div>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.vault-sync-hint {
  margin: 0.25rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.85rem;
}

.vault-sync-hint__queue {
  display: inline-block;
  margin-left: 0.5rem;
  color: var(--hs-text-subtle);
}

.vault-sync-meta,
.vault-sync-error {
  margin: 0.2rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.8rem;
}

.vault-sync-error {
  color: var(--hs-warning);
}

.vault-sync-recovery {
  margin-top: 0.65rem;
  padding: 0.75rem 0.85rem;
  border: 1px solid rgba(255, 200, 87, 0.42);
  border-radius: 8px;
  background: rgba(255, 200, 87, 0.08);
  color: var(--hs-text);
  font-size: 0.84rem;
}

.vault-sync-recovery p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
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

.registry-arbitration-panel {
  margin: 1rem 0;
  padding: 0.9rem;
  border: 1px solid rgba(255, 200, 87, 0.42);
  border-radius: 8px;
  background: rgba(255, 200, 87, 0.06);
}

.registry-arbitration-panel__header,
.registry-arbitration-item {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
}

.registry-arbitration-panel__label {
  display: block;
  margin-bottom: 0.25rem;
  color: var(--hs-warning);
  font-size: 0.76rem;
  font-weight: 700;
}

.registry-arbitration-panel p,
.registry-arbitration-item small {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  line-height: 1.5;
}

.registry-arbitration-item {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.registry-arbitration-item span {
  display: block;
  margin-top: 0.25rem;
  color: var(--hs-text-subtle);
  font-family: var(--hs-font-mono);
  font-size: 0.78rem;
}

.registry-arbitration-item__actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.vault-registry-badge {
  display: inline-flex;
  margin-top: 0.35rem;
  padding: 0.18rem 0.45rem;
  border: 1px solid rgba(255, 200, 87, 0.38);
  border-radius: 999px;
  color: var(--hs-warning);
  font-size: 0.72rem;
  font-weight: 700;
}

.vault-card-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 0.6rem;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.public-rights-card {
  margin-top: 0.85rem;
  padding: 0.85rem;
  border: 1px solid rgba(114, 214, 202, 0.28);
  border-radius: 8px;
  background: rgba(114, 214, 202, 0.07);
}

.public-rights-card__header {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}

.public-rights-card__header span {
  display: block;
  margin-bottom: 0.2rem;
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.public-rights-card p {
  margin: 0.55rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.82rem;
  line-height: 1.5;
}

.public-rights-card__grid {
  display: grid;
  grid-template-columns: 6.5rem minmax(0, 1fr);
  gap: 0.55rem 0.7rem;
  margin-top: 0.75rem;
  font-size: 0.82rem;
}

.public-rights-card__grid span {
  color: var(--hs-text-subtle);
}

.public-rights-card__grid strong {
  color: var(--hs-text);
  overflow-wrap: anywhere;
}

.public-rights-card__warning,
.public-rights-card__error {
  color: var(--hs-warning) !important;
}

.report-commerce-card {
  margin-top: 1rem;
  padding: 1rem;
  border: 1px solid rgba(114, 214, 202, 0.34);
  border-radius: 10px;
  background:
    linear-gradient(135deg, rgba(114, 214, 202, 0.12), rgba(87, 143, 202, 0.08)),
    var(--hs-surface);
  box-shadow: 0 14px 36px rgba(11, 29, 46, 0.14);
}

.report-commerce-card__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.report-commerce-card__header span,
.report-commerce-card__products article > span {
  display: block;
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.report-commerce-card__header strong {
  display: block;
  margin-top: 0.25rem;
  font-size: 1rem;
}

.report-commerce-card p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.82rem;
  line-height: 1.5;
}

.report-commerce-card__products {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
  margin-top: 0.9rem;
}

.report-commerce-card__products article {
  display: flex;
  flex-direction: column;
  padding: 0.9rem;
  border: 1px solid rgba(87, 143, 202, 0.24);
  border-radius: 8px;
  background: rgba(7, 24, 39, 0.12);
}

.report-commerce-card__products article > strong {
  margin-top: 0.35rem;
  color: var(--hs-text);
  font-size: 1.35rem;
}

.report-commerce-card__products article > strong small {
  color: var(--hs-text-muted);
  font-size: 0.76rem;
  font-weight: 500;
}

.report-commerce-card__product--evidence {
  border-color: rgba(255, 200, 87, 0.42) !important;
  background: rgba(255, 200, 87, 0.08) !important;
}

.report-commerce-card__buy {
  width: 100%;
  margin-top: auto;
  padding-top: 0.72rem;
  padding-bottom: 0.72rem;
}

@media (max-width: 760px) {
  .report-commerce-card__header {
    display: grid;
  }

  .report-commerce-card__products {
    grid-template-columns: 1fr;
  }
}

.team-workspace-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 1rem;
  align-items: center;
  margin-top: 0.85rem;
  padding: 0.9rem 1rem;
  border: 1px solid rgba(114, 214, 202, 0.24);
  border-radius: 8px;
  background: rgba(114, 214, 202, 0.08);
}

.team-workspace-card strong {
  display: block;
  margin-top: 0.2rem;
}

.team-workspace-card p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.86rem;
  line-height: 1.5;
}

.team-workspace-card__label {
  display: inline-flex;
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.report-export-list {
  margin-top: 0.9rem;
  display: grid;
  gap: 0.7rem;
}

.report-export-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.75rem;
  align-items: center;
  padding: 0.8rem;
  border: 1px solid rgba(87, 143, 202, 0.22);
  border-radius: 8px;
  background: rgba(87, 143, 202, 0.07);
}

.report-export-item p {
  margin: 0.25rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.78rem;
  overflow-wrap: anywhere;
}

.report-export-item__actions {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.vault-verification-badge {
  display: inline-flex;
  margin: 0 0 0.55rem;
  padding: 0.25rem 0.55rem;
  border-radius: var(--hs-radius-pill);
  font-size: 0.76rem;
  font-weight: 600;
}

.vault-verification-badge--ok {
  background: rgba(114, 214, 202, 0.1);
  color: var(--hs-accent);
}

.vault-verification-badge--warn {
  background: rgba(255, 200, 87, 0.1);
  color: var(--hs-warning);
}

.vault-video-notary-badge {
  display: inline-flex;
  margin: 0 0 0.55rem 0.45rem;
  padding: 0.25rem 0.55rem;
  border-radius: var(--hs-radius-pill);
  background: var(--hs-chip);
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 600;
}

.vault-sync-message {
  margin: 0.85rem 0 0;
  padding: 0.65rem 0.8rem;
  border-radius: 8px;
  background: rgba(87, 143, 202, 0.1);
  border: 1px solid rgba(87, 143, 202, 0.22);
  color: var(--hs-text-muted);
  font-size: 0.86rem;
}

.vault-lineage__list {
  display: grid;
  gap: 0.75rem;
}

.vault-lineage__item {
  padding: 0.9rem;
  border-radius: var(--hs-radius-card);
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
  color: var(--hs-text-muted);
  font-size: 0.85rem;
}

.vault-lineage__chain {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-top: 0.7rem;
  font-family: monospace;
  font-size: 0.82rem;
  color: var(--hs-text);
  word-break: break-all;
}

.vault-lineage__reason {
  margin-top: 0.6rem !important;
}

.vault-lineage-drawer {
  margin-top: 1rem;
  padding: 1rem;
  border-radius: var(--hs-radius-card);
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
  color: var(--hs-text-muted);
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
  color: var(--hs-text-muted);
}

.vault-lineage-drawer__grid b {
  color: var(--hs-text);
  word-break: break-all;
}
</style>
