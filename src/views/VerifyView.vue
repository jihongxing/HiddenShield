<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import DropZone from "../components/DropZone.vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import { trackClick, trackFeatureEvent } from "../lib/analytics";
import { userFacingErrorMessage } from "../lib/user-facing-errors";
import {
  buildVerificationSummary,
  IMAGE_ORIENTATION_DETECTION_SCOPE,
  exportVaultFormalReport,
  openOutputDir,
  VAULT_DEEP_DETECTION_SCOPE,
  getTsaVerificationLabel,
  getDesktopCloudSyncProfile,
  flushAnonymousFeedbackQueue,
  importMobileReportHandoff,
  verifySuspect,
  verifyFormalReportBundle,
  verifyRightsEvidencePack,
  type EntitlementState,
  type FormalReportBundleVerificationResult,
  type FormalReportExportResult,
  type PublicRightsQueryResponse,
  type RightsEvidencePackVerificationResult,
  type VerificationResult,
} from "../lib/tauri-api";
import { saveRecentReportExport } from "../lib/report-export-history";
import { createPublicRightsScanner, type PublicRightsSdkResult } from "../lib/public-rights-sdk";

const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const emit = defineEmits<{
  switchTab: [tab: "vault"];
  openSubscription: [];
}>();

const suspectPath = ref("");
const suspectName = ref("");
const result = ref<VerificationResult | null>(null);
const loading = ref(false);
const errorMsg = ref("");
const diagnosticMsg = ref("");
const imageOrientationSupportText = IMAGE_ORIENTATION_DETECTION_SCOPE;
const vaultDeepDetectionText = VAULT_DEEP_DETECTION_SCOPE;
const canExportFormalReports = computed(() => props.entitlementState?.features?.report_export === true);
const canExportMatchedFormalReport = computed(() => Boolean(result.value?.matchedRecord));
const exportingFormalReport = ref(false);
const latestReportExport = ref<FormalReportExportResult | null>(null);
const latestReportVerification = ref<FormalReportBundleVerificationResult | null>(null);
const importedReportVerification = ref<FormalReportBundleVerificationResult | null>(null);
const importedReportDir = ref("");
const verifyingReportBundle = ref(false);
const importingMobileHandoff = ref(false);
const rightsEvidencePackDir = ref("");
const rightsEvidencePackVerification = ref<RightsEvidencePackVerificationResult | null>(null);
const verifyingRightsEvidencePack = ref(false);
const rightsEvidencePackError = ref("");
const publicRightsResult = ref<PublicRightsSdkResult | null>(null);
const publicRightsLoading = ref(false);
const publicRightsError = ref("");
const verifyProgress = ref(0);
const verifyStage = ref("等待样本");
let verifyProgressTimer: number | null = null;
const verifyTroubleshootingItems = [
  "确认样本是否由隐盾生成过保护副本。",
  "图片支持轴对齐、宽高各为原图 1/4 的裁切区域，以及 90/180/270 度旋转、85% 缩放、JPEG/WebP quality 75/60 的独立恢复。",
  "任意角度、多个扰动叠加、更低质量重编码或更大比例缩小时，请优先换更接近原发布内容的样本。",
  "音频样本请确认音轨仍完整，且没有被大幅重采样或二次转码。",
  "仍无法识别时，保留样本和本机版权库记录，再发送反馈排查。",
];

const verifyProgressPlan = [
  { percent: 12, stage: "正在读取样本信息" },
  { percent: 28, stage: "正在提取验证特征" },
  { percent: 44, stage: "正在进行默认验证检测" },
  { percent: 60, stage: "正在匹配本机版权库" },
  { percent: 74, stage: "正在复核可信时间与版本记录" },
  { percent: 86, stage: "正在生成验证结果" },
];

const VIDEO_EXTENSIONS = new Set(["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv", "m4v"]);

function isVideoPath(path: string): boolean {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  return VIDEO_EXTENSIONS.has(extension);
}

function startVerifyProgress() {
  stopVerifyProgress();
  verifyProgress.value = 6;
  verifyStage.value = "正在准备验证任务";
  let index = 0;
  verifyProgressTimer = window.setInterval(() => {
    if (index < verifyProgressPlan.length) {
      const next = verifyProgressPlan[index];
      verifyProgress.value = next.percent;
      verifyStage.value = next.stage;
      index += 1;
      return;
    }
    verifyProgress.value = Math.min(92, verifyProgress.value + 1);
    verifyStage.value = "验证仍在进行，请稍候";
  }, 1200);
}

function finishVerifyProgress(stage = "验证完成") {
  stopVerifyProgress();
  verifyProgress.value = 100;
  verifyStage.value = stage;
}

function failVerifyProgress(stage = "验证失败") {
  stopVerifyProgress();
  verifyStage.value = stage;
}

function stopVerifyProgress() {
  if (verifyProgressTimer !== null) {
    window.clearInterval(verifyProgressTimer);
    verifyProgressTimer = null;
  }
}

onUnmounted(stopVerifyProgress);

const publicRightsSnapshot = computed<PublicRightsQueryResponse | null>(() => publicRightsResult.value?.scan ?? null);

const publicRightsMessage = computed(() =>
  publicRightsResult.value?.message ?? "公开查询只展示创作者声明与 registry 快照，不直接判断是否可训练。",
);

async function handleFileSelect(path: string) {
  if (isVideoPath(path)) {
    suspectPath.value = "";
    suspectName.value = "";
    result.value = null;
    errorMsg.value = "当前版本仅支持图片和音频验证，暂不提供视频文件验证。";
    return;
  }
  suspectPath.value = path;
  suspectName.value = path.split(/[\\/]/).pop() ?? path;

  // 选择文件后自动执行验证
  await handleVerify();
}

async function handleVerify() {
  if (!suspectPath.value) return;
  if (isVideoPath(suspectPath.value)) {
    result.value = null;
    errorMsg.value = "当前版本仅支持图片和音频验证，暂不提供视频文件验证。";
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  diagnosticMsg.value = "";
  result.value = null;
  publicRightsResult.value = null;
  publicRightsError.value = "";
  startVerifyProgress();
  trackFeatureEvent("verify_suspect", "start", { mediaType: "unknown", source: "dropzone" });

  try {
    result.value = await verifySuspect(suspectPath.value);
    await loadPublicRightsForResult(result.value.watermarkUid);
    finishVerifyProgress(result.value.matched ? "已匹配版权记录" : "验证完成，未匹配本机记录");
    trackFeatureEvent("verify_suspect", "success", {
      mediaType: "unknown",
      source: result.value.matched ? "matched" : "unmatched",
    });
  } catch (err: any) {
    console.warn("verify suspect failed", err);
    errorMsg.value = userFacingErrorMessage(err, "验证");
    failVerifyProgress("验证失败");
    trackFeatureEvent("verify_suspect", "failure", {
      mediaType: "unknown",
      errorCode: "verify_failed",
      source: "command_error",
    });
  } finally {
    loading.value = false;
  }
}

async function loadPublicRightsForResult(watermarkUid: string | null | undefined) {
  const uid = watermarkUid?.trim();
  publicRightsResult.value = null;
  publicRightsError.value = "";
  if (!uid) return;
  const profile = await getDesktopCloudSyncProfile();
  if (!profile?.cloudBaseUrl) {
    publicRightsError.value = "未连接公开 registry";
    return;
  }
  publicRightsLoading.value = true;
  try {
    publicRightsResult.value = await createPublicRightsScanner(profile.cloudBaseUrl).scanOne(uid);
  } catch (error: unknown) {
    console.warn("verify public rights query failed", error);
    publicRightsError.value = userFacingErrorMessage(error, "查询公开权利信号");
  } finally {
    publicRightsLoading.value = false;
  }
}

async function handleSendDiagnostic() {
  trackClick("verify_send_diagnostic_click");
  diagnosticMsg.value = "";
  const response = await flushAnonymousFeedbackQueue();
  diagnosticMsg.value = response.message;
}

async function handleCopySummary() {
  if (!result.value) return;
  const text = buildVerificationSummary(result.value, suspectPath.value);
  await navigator.clipboard.writeText(text);
}

function handleFormalReportExport() {
  if (!result.value?.matchedRecord) {
    diagnosticMsg.value = "正式报告需要先匹配本机版权库记录。";
    return;
  }
  void exportFormalReport();
}

async function exportFormalReport() {
  if (!result.value) return;
  exportingFormalReport.value = true;
  diagnosticMsg.value = "";
  try {
    const recordId = result.value.matchedRecord?.id;
    if (!recordId) {
      diagnosticMsg.value = "正式报告需要先匹配本机版权库记录。";
      return;
    }
    const exported = await exportVaultFormalReport(recordId);
    latestReportExport.value = exported;
    latestReportVerification.value = null;
    saveRecentReportExport(exported);
    diagnosticMsg.value = "已导出正式报告";
  } catch (error: unknown) {
    console.warn("formal report export failed", error);
    diagnosticMsg.value = userFacingErrorMessage(error, "导出正式报告");
  } finally {
    exportingFormalReport.value = false;
  }
}

async function openReportDir() {
  if (!latestReportExport.value) return;
  await openOutputDir(latestReportExport.value.reportDir);
}

async function copyReportPath(path: string, label: string) {
  await navigator.clipboard.writeText(path);
  diagnosticMsg.value = `${label}路径已复制`;
}

function handleFileSelectError(message: string) {
  errorMsg.value = message;
  diagnosticMsg.value = "";
  result.value = null;
}

async function verifyLatestReportBundle() {
  if (!latestReportExport.value) return;
  verifyingReportBundle.value = true;
  diagnosticMsg.value = "";
  try {
    latestReportVerification.value = await verifyFormalReportBundle(
      latestReportExport.value.reportDir,
    );
    diagnosticMsg.value = latestReportVerification.value.message;
  } catch (error: unknown) {
    console.warn("verify formal report bundle failed", error);
    diagnosticMsg.value = userFacingErrorMessage(error, "校验报告包");
  } finally {
    verifyingReportBundle.value = false;
  }
}

async function selectAndVerifyReportBundle() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择 HiddenShield 报告包或移动签发交接包",
  });
  if (!selected || Array.isArray(selected)) return;
  verifyingReportBundle.value = true;
  diagnosticMsg.value = "";
  try {
    importedReportDir.value = selected;
    importedReportVerification.value = await verifyFormalReportBundle(selected);
    diagnosticMsg.value = importedReportVerification.value.message;
  } catch (error: unknown) {
    console.warn("verify imported report bundle failed", error);
    importedReportVerification.value = null;
    diagnosticMsg.value = userFacingErrorMessage(error, "校验导入报告包");
  } finally {
    verifyingReportBundle.value = false;
  }
}

async function renderImportedMobileHandoff() {
  if (
    !importedReportDir.value ||
    importedReportVerification.value?.reportType !== "formal_report_handoff"
  ) {
    return;
  }
  if (!canExportFormalReports.value) {
    emit("openSubscription");
    return;
  }
  importingMobileHandoff.value = true;
  diagnosticMsg.value = "";
  try {
    const exported = await importMobileReportHandoff(importedReportDir.value);
    latestReportExport.value = exported;
    latestReportVerification.value = null;
    saveRecentReportExport(exported);
    diagnosticMsg.value = "移动签发交接包已生成最终 PDF 三件套";
  } catch (error: unknown) {
    console.warn("import mobile report handoff failed", error);
    diagnosticMsg.value = userFacingErrorMessage(error, "生成移动交接最终报告");
  } finally {
    importingMobileHandoff.value = false;
  }
}

async function selectAndVerifyRightsEvidencePack() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择 HiddenShield 维权证据包",
  });
  if (!selected || Array.isArray(selected)) return;
  verifyingRightsEvidencePack.value = true;
  rightsEvidencePackError.value = "";
  try {
    rightsEvidencePackDir.value = selected;
    rightsEvidencePackVerification.value = await verifyRightsEvidencePack(selected);
  } catch (error: unknown) {
    console.warn("verify rights evidence pack failed", error);
    rightsEvidencePackVerification.value = null;
    rightsEvidencePackError.value = userFacingErrorMessage(error, "校验维权证据包");
  } finally {
    verifyingRightsEvidencePack.value = false;
  }
}

function formatEvidencePackStatus(status: string): string {
  switch (status) {
    case "matched":
      return "匹配";
    case "mismatch":
      return "不匹配";
    case "not_signed":
      return "未签名";
    case "not_timestamped":
      return "未加盖";
    case "present_unverified":
      return "存在但未验证";
    default:
      return status || "未记录";
  }
}

function evidencePackStatusClass(status: string): string {
  if (status === "matched") return "verify-case-pack__status--matched";
  if (status === "not_signed" || status === "not_timestamped") {
    return "verify-case-pack__status--boundary";
  }
  return "verify-case-pack__status--mismatch";
}

function formatEvidenceAttachmentRole(role: string): string {
  switch (role) {
    case "original":
      return "原件角色";
    case "working_copy":
      return "工作副本";
    case "capture":
      return "外部采集件";
    case "external_receipt":
      return "外部回执";
    default:
      return role || "未记录";
  }
}

function shortDigest(value: string | null): string {
  if (!value) return "未记录";
  if (value.length <= 30) return value;
  return `${value.slice(0, 14)}…${value.slice(-12)}`;
}

function handleReset() {
  suspectPath.value = "";
  suspectName.value = "";
  result.value = null;
  errorMsg.value = "";
  diagnosticMsg.value = "";
}

function getConfidenceClass(confidence: number) {
  if (confidence >= 0.95) return "verify-result--match";
  if (confidence >= 0.5) return "verify-result--warn";
  return "verify-result--miss";
}

function getUnmatchedReason(confidence: number, filePath: string): string {
  if (confidence < 0.1) return "该文件可能非本机处理的作品";
  if (confidence < 0.5) return "该文件可能经过强压缩、强裁剪、任意角度旋转、音轨替换等处理";
  return "";
}

function formatDurationMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
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
  if (value === "v3_minimal_anchor") return "V3 最小媒体锚点";
  if (value === "v2_migration_anchor") return "V2 迁移桥接锚点";
  return value?.trim() || "未记录";
}

function formatMediaPayloadRole(value?: string | null): string {
  if (value === "v3_minimal_anchor") return "V3 最小锚点";
  if (value === "v2_full_record") return "V2 完整载荷";
  return value?.trim() || "未记录";
}
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">验证</p>
        <h2>验证记录</h2>
      </div>
    </section>

    <section class="panel verify-panel">
      <div class="panel__header">
        <div>
          <h3>选择样本</h3>
        </div>
        <button
          v-if="result || errorMsg"
          class="ghost-button"
          type="button"
          @click="handleReset"
        >
          重新选择
        </button>
      </div>

      <div class="verify-workspace">
        <div class="verify-input-column">
          <DropZone
            :selected-path="suspectPath"
            :source-name="suspectName"
            :disabled="loading"
            @select="handleFileSelect"
            @error="handleFileSelectError"
          />

          <div class="verify-scope-grid">
            <div class="verify-scope-card">
              <span>默认检测</span>
              <p>{{ imageOrientationSupportText }}</p>
            </div>
            <div class="verify-scope-card">
              <span>版权库深度检测</span>
              <p>{{ vaultDeepDetectionText }}</p>
            </div>
          </div>
        </div>

        <div class="verify-result-column">
          <div v-if="!loading && !errorMsg && !result" class="verify-empty-result">
            <strong>等待验证样本</strong>
            <p>结果会先显示结论，再展示置信度、匹配记录、证据摘要和报告动作。</p>
          </div>

      <!-- Loading state -->
      <div v-if="loading" class="verify-loading">
        <div class="verify-loading__header">
          <span class="verify-loading__spinner" aria-hidden="true"></span>
          <div>
            <strong>{{ verifyStage }}</strong>
            <span>{{ verifyProgress }}%</span>
          </div>
        </div>
        <div class="verify-loading__track">
          <div class="verify-loading__fill" :style="{ width: `${verifyProgress}%` }"></div>
        </div>
        <p>大图、裁剪样本或音频样本可能需要更长时间，验证完成前请保持页面打开。</p>
      </div>

      <div v-if="errorMsg" class="verify-result verify-result--error">
        <strong>识别失败</strong>
        <p>{{ errorMsg }}</p>
        <div class="verify-checklist">
          <span>排查清单</span>
          <ul>
            <li v-for="item in verifyTroubleshootingItems" :key="item">{{ item }}</li>
          </ul>
        </div>
        <button class="ghost-button" type="button" @click="handleSendDiagnostic">
          发送反馈
        </button>
        <p v-if="diagnosticMsg" class="verify-result__confidence">{{ diagnosticMsg }}</p>
      </div>

      <!-- Result display with confidence-based styling -->
      <div v-if="result" class="verify-result" :class="getConfidenceClass(result.confidence)">
        <template v-if="result.matched">
          <strong>已匹配版权记录</strong>
          <p>{{ result.summary }}</p>
        </template>
        <template v-else-if="result.confidence >= 0.95">
          <strong>已识别，但未完成作品绑定</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__confidence">置信度 {{ Math.round(result.confidence * 100) }}%</p>
          <p class="verify-result__confidence">验证耗时 {{ formatDurationMs(result.durationMs) }}</p>
        </template>
        <template v-else-if="result.confidence >= 0.5">
          <strong>疑似匹配</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__confidence">置信度 {{ Math.round(result.confidence * 100) }}%</p>
          <p class="verify-result__confidence">验证耗时 {{ formatDurationMs(result.durationMs) }}</p>
        </template>
        <template v-else>
          <strong>未找到对应记录</strong>
          <p>{{ result.summary }}</p>
          <p class="verify-result__reason">{{ result.reasonDetail || getUnmatchedReason(result.confidence, suspectPath) }}</p>
          <p class="verify-result__confidence">验证耗时 {{ formatDurationMs(result.durationMs) }}</p>
        </template>

        <div v-if="result.reasonDetail" class="verify-reason">
          <span>参考说明</span>
          <p>{{ result.reasonDetail }}</p>
        </div>

        <div v-if="!result.matched" class="verify-checklist">
          <span>排查清单</span>
          <ul>
            <li v-for="item in verifyTroubleshootingItems" :key="item">{{ item }}</li>
          </ul>
        </div>

        <div class="verify-result__meta">
          <span>置信度 {{ Math.round(result.confidence * 100) }}%</span>
          <span>验证耗时 {{ formatDurationMs(result.durationMs) }}</span>
          <span v-if="result.watermarkUid">版权编号 {{ result.watermarkUid }}</span>
          <span v-if="result.payloadProtocolVersion && result.payloadBytesLength">
            Payload V{{ result.payloadProtocolVersion }} / {{ result.payloadBytesLength }} bytes
          </span>
          <span v-if="result.mediaPayloadRole">
            {{ formatMediaPayloadRole(result.mediaPayloadRole) }}
          </span>
          <span v-if="result.payloadAuthStatus">
            认证 {{ result.payloadAuthStatus === "verified" ? "已通过" : result.payloadAuthStatus }}
          </span>
          <span v-if="result.matchedRecord">第 {{ result.matchedRecord.revision }} 次版本</span>
        </div>
      </div>

      <!-- Matched record card -->
      <div v-if="result && result.matchedRecord && result.confidence >= 0.95" class="verify-matched">
        <CopyrightCard :record="result.matchedRecord" />

        <div
          v-if="result.matchedRecord.revision > 1 || result.matchedRecord.parentWatermarkUid || result.matchedRecord.rewriteReason"
          class="verify-lineage"
        >
          <strong>版本记录</strong>
          <div class="verify-lineage__row">
            <span>版本次数</span>
            <b>第 {{ result.matchedRecord.revision }} 次</b>
          </div>
          <div v-if="result.matchedRecord.parentWatermarkUid" class="verify-lineage__row">
            <span>上一版编号</span>
            <b>{{ result.matchedRecord.parentWatermarkUid }}</b>
          </div>
          <div v-if="result.matchedRecord.rewriteReason" class="verify-lineage__row">
            <span>更新说明</span>
            <b>{{ result.matchedRecord.rewriteReason }}</b>
          </div>
        </div>

        <div v-if="result.tsaTokenPresent || result.networkTime" class="verify-tsa">
          <strong>可信时间</strong>
          <p v-if="result.tsaTokenPresent && result.tsaTokenVerified">
            {{ getTsaVerificationLabel(result.tsaVerificationPath) ?? "时间回执已复验" }}
          </p>
          <p v-else-if="result.tsaTokenPresent">
            时间回执已获取
          </p>
          <p v-if="result.networkTime">网络时间: {{ new Date(result.networkTime).toLocaleString() }}</p>
          <p v-if="result.createdAt">存证时间: {{ new Date(result.createdAt).toLocaleString() }}</p>
        </div>

        <button class="ghost-button" type="button" @click="emit('switchTab', 'vault')">
          查看版权库
        </button>
      </div>

      <div v-if="result && result.watermarkUid" class="verify-public-rights">
        <div class="verify-public-rights__header">
          <div>
            <span>公开权利信号</span>
            <strong>
              {{ publicRightsLoading ? "正在查询" : publicRightsSnapshot ? formatPublicRightsScanStatus(publicRightsSnapshot.scanStatus) : publicRightsError || "未查询" }}
            </strong>
          </div>
          <button
            class="ghost-button"
            type="button"
            :disabled="publicRightsLoading"
            @click="loadPublicRightsForResult(result.watermarkUid)"
          >
            {{ publicRightsLoading ? "查询中" : "刷新" }}
          </button>
        </div>
        <div v-if="publicRightsSnapshot" class="verify-public-rights__grid">
          <span>训练许可</span>
          <strong>{{ publicRightsSnapshot.trainingPermission.label }}</strong>
          <span>锚点协议</span>
          <strong>{{ formatAnchorProtocol(publicRightsSnapshot.registry.anchorProtocol) }}</strong>
          <span>Manifest</span>
          <strong>{{ publicRightsSnapshot.rightsManifest ? `v${publicRightsSnapshot.rightsManifest.manifestVersion}` : "待回填" }}</strong>
          <span>法律结论</span>
          <strong>{{ publicRightsSnapshot.trainingPermission.legalConclusion ? "是" : "否" }}</strong>
        </div>
        <p v-if="publicRightsSnapshot?.warnings.length" class="verify-public-rights__warning">
          {{ publicRightsSnapshot.warnings.join(" / ") }}
        </p>
        <p class="verify-public-rights__note">{{ publicRightsMessage }}</p>
      </div>

      <!-- Actions -->
      <div v-if="result" class="verify-actions">
        <button class="primary-button" type="button" @click="handleCopySummary">
          复制验证摘要
        </button>
        <button
          v-if="!result.matched"
          class="ghost-button"
          type="button"
          @click="handleSendDiagnostic"
        >
          发送反馈
        </button>
        <button
          class="ghost-button"
          type="button"
          :disabled="exportingFormalReport || !result.matchedRecord"
          @click="handleFormalReportExport"
        >
          {{ exportingFormalReport ? "导出中" : canExportMatchedFormalReport ? "导出已购买报告" : "需匹配版权库" }}
        </button>
      </div>

      <p v-if="diagnosticMsg && !errorMsg" class="verify-diagnostic">{{ diagnosticMsg }}</p>

      <div class="verify-report-export">
        <div>
          <strong>跨端报告包校验</strong>
          <p>{{ importedReportDir || "可选择桌面完整报告包，或移动端生成的桌面签发交接包。" }}</p>
          <small v-if="importedReportVerification">
            类型：{{ importedReportVerification.reportType === "formal_report_handoff" ? "移动签发交接包" : "完整报告包" }}
            · 完整性：{{ importedReportVerification.integrityStatus === "matched" ? "文件匹配" : "校验失败" }}
            · 文档合同：{{ importedReportVerification.documentContractStatus === "matched" ? "匹配" : "不匹配" }}
            · 签名：{{ importedReportVerification.signatureStatus === "not_signed" ? "未签名" : "存在但未验证" }}
          </small>
        </div>
        <div class="verify-report-export__actions">
          <button
            class="ghost-button"
            type="button"
            :disabled="verifyingReportBundle"
            @click="selectAndVerifyReportBundle"
          >
            {{ verifyingReportBundle ? "校验中…" : "选择目录并校验" }}
          </button>
          <button
            v-if="importedReportVerification?.reportType === 'formal_report_handoff'"
            class="primary-button"
            type="button"
            :disabled="importingMobileHandoff"
            @click="renderImportedMobileHandoff"
          >
            {{
              importingMobileHandoff
                ? "生成中…"
                : canExportFormalReports
                  ? "生成最终 PDF"
                  : "需单独购买后生成"
            }}
          </button>
        </div>
      </div>

      <div class="verify-case-pack" data-testid="rights-evidence-pack-verifier">
        <div class="verify-case-pack__header">
          <div>
            <span class="verify-case-pack__eyebrow">报告与维权服务</span>
            <strong>校验维权证据包</strong>
            <p>
              {{
                rightsEvidencePackDir ||
                  "检查证据包目录、附件和完整性信息，校验过程不会修改文件。"
              }}
            </p>
          </div>
          <button
            class="ghost-button"
            type="button"
            :disabled="verifyingRightsEvidencePack"
            @click="selectAndVerifyRightsEvidencePack"
          >
            {{ verifyingRightsEvidencePack ? "校验中…" : "选择证据包并校验" }}
          </button>
        </div>

        <p v-if="rightsEvidencePackError" class="verify-case-pack__error">
          {{ rightsEvidencePackError }}
        </p>

        <template v-if="rightsEvidencePackVerification">
          <div
            class="verify-case-pack__status-grid"
            data-testid="rights-evidence-pack-status-grid"
          >
            <div
              v-for="item in [
                ['目录合同', rightsEvidencePackVerification.directoryContractStatus],
                ['附件完整性', rightsEvidencePackVerification.attachmentIntegrityStatus],
                ['采集事件链', rightsEvidencePackVerification.eventChainStatus],
                ['附件追加链', rightsEvidencePackVerification.attachmentChainStatus],
                ['数字签名', rightsEvidencePackVerification.signatureStatus],
                ['可信时间', rightsEvidencePackVerification.trustedTimeStatus],
              ]"
              :key="item[0]"
              class="verify-case-pack__status"
              :class="evidencePackStatusClass(item[1])"
            >
              <span>{{ item[0] }}</span>
              <strong>{{ formatEvidencePackStatus(item[1]) }}</strong>
            </div>
          </div>

          <div class="verify-case-pack__lineage">
            <div>
              <span>案件 / 证据包</span>
              <strong>
                {{ rightsEvidencePackVerification.caseId || "未记录" }} ·
                {{ rightsEvidencePackVerification.packId || "未记录" }}
              </strong>
            </div>
            <div>
              <span>Manifest 声明 root digest</span>
              <strong>{{ shortDigest(rightsEvidencePackVerification.declaredRootDigest) }}</strong>
            </div>
            <div>
              <span>本机复算 root digest</span>
              <strong>{{ shortDigest(rightsEvidencePackVerification.computedRootDigest) }}</strong>
            </div>
          </div>

          <details class="verify-case-pack__attachments">
            <summary>
              附件逐项结果（{{ rightsEvidencePackVerification.attachments.length }}）
            </summary>
            <div
              v-for="attachment in rightsEvidencePackVerification.attachments"
              :key="attachment.attachmentId"
              class="verify-case-pack__attachment"
            >
              <div>
                <span>{{ formatEvidenceAttachmentRole(attachment.role) }}</span>
                <strong>{{ attachment.path }}</strong>
              </div>
              <div>
                <span>{{ formatEvidencePackStatus(attachment.status) }}</span>
                <small>
                  {{ attachment.actualBytes ?? "缺失" }} / {{ attachment.expectedBytes }} bytes
                  · {{ shortDigest(attachment.actualSha256) }}
                </small>
              </div>
            </div>
          </details>

          <p class="verify-case-pack__message">
            {{ rightsEvidencePackVerification.message }}
          </p>
        </template>

        <p v-else class="verify-case-pack__boundary">
          校验只复算目录、文件与摘要链，不读取媒体水印，不判断侵权成立、签发主体可信或采集时间可信。
        </p>
      </div>

      <div v-if="latestReportExport" class="verify-report-export">
        <div>
          <strong>最近导出</strong>
          <p>{{ latestReportExport.pdfPath }}</p>
          <small>
            第 {{ latestReportExport.bundleVersion }} 版 ·
            {{ latestReportExport.pdfPageCount }} 页 ·
            {{ Math.round(latestReportExport.pdfGenerationMs) }} ms
          </small>
          <small v-if="latestReportVerification">
            完整性：{{ latestReportVerification.integrityStatus === "matched" ? "文件匹配" : "校验失败" }}
            · 文档合同：{{ latestReportVerification.documentContractStatus === "matched" ? "匹配" : "不匹配" }}
            · 签名：{{ latestReportVerification.signatureStatus === "not_signed" ? "未签名" : "存在但未验证" }}
            · 可信时间：{{ latestReportVerification.trustedTimeStatus === "not_timestamped" ? "未加盖" : "存在但未验证" }}
          </small>
        </div>
        <div class="verify-report-export__actions">
          <button class="ghost-button" type="button" @click="openReportDir">
            打开目录
          </button>
          <button
            class="ghost-button"
            type="button"
            @click="copyReportPath(latestReportExport.pdfPath, 'PDF')"
          >
            复制 PDF
          </button>
          <button
            class="ghost-button"
            type="button"
            @click="copyReportPath(latestReportExport.jsonPath, 'JSON')"
          >
            复制 JSON
          </button>
          <button
            class="ghost-button"
            type="button"
            @click="copyReportPath(latestReportExport.manifestPath, 'Manifest')"
          >
            复制 Manifest
          </button>
          <button
            class="ghost-button"
            type="button"
            :disabled="verifyingReportBundle"
            @click="verifyLatestReportBundle"
          >
            {{ verifyingReportBundle ? "校验中…" : "校验报告包" }}
          </button>
        </div>
      </div>

          <div v-if="result" class="verify-disclaimer">
            <details>
              <summary>免责声明</summary>
              <p>{{ result.disclaimer }}</p>
            </details>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.verify-workspace {
  display: grid;
  grid-template-columns: minmax(280px, 0.38fr) minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.verify-input-column,
.verify-result-column {
  display: grid;
  gap: 14px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

.verify-result-column > *,
.verify-result,
.verify-matched {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  overflow-wrap: anywhere;
}

.verify-input-column {
  position: sticky;
  top: 0;
}

.verify-empty-result {
  display: grid;
  gap: 6px;
  min-height: 180px;
  place-content: center;
  padding: 24px;
  border: 1px dashed var(--hs-border);
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-muted);
  text-align: center;
}

.verify-empty-result p {
  margin: 0;
  color: var(--hs-text-muted);
}

.verify-diagnostic {
  margin-top: 0.75rem;
  font-size: 0.85rem;
  color: var(--hs-text-muted);
}

.verify-loading {
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
  border: 1px solid rgba(114, 214, 202, 0.24);
  border-radius: var(--hs-radius-card);
  background: rgba(114, 214, 202, 0.08);
}

.verify-loading__header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.verify-loading__header div {
  display: flex;
  min-width: 0;
  flex: 1;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.verify-loading__header strong {
  color: var(--hs-text);
}

.verify-loading__header span {
  color: var(--hs-text-muted);
  font-variant-numeric: tabular-nums;
}

.verify-loading__spinner {
  width: 18px;
  height: 18px;
  border: 2px solid rgba(114, 214, 202, 0.18);
  border-top-color: var(--hs-accent);
  border-radius: var(--hs-radius-pill);
  animation: verify-spin 0.8s linear infinite;
}

.verify-loading__track {
  overflow: hidden;
  height: 10px;
  border-radius: var(--hs-radius-pill);
  background: var(--hs-surface-muted);
}

.verify-loading__fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--hs-accent), var(--hs-copper));
  transition: width 0.35s ease;
}

.verify-loading p {
  margin: 0;
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.55;
}

@keyframes verify-spin {
  to {
    transform: rotate(360deg);
  }
}

.verify-public-rights {
  padding: 0.9rem;
  border: 1px solid rgba(114, 214, 202, 0.28);
  border-radius: 8px;
  background: rgba(114, 214, 202, 0.07);
}

.verify-public-rights__header {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}

.verify-public-rights__header span {
  display: block;
  margin-bottom: 0.2rem;
  color: var(--hs-accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.verify-public-rights__grid {
  display: grid;
  grid-template-columns: 6rem minmax(0, 1fr);
  gap: 0.55rem 0.7rem;
  margin-top: 0.75rem;
  font-size: 0.82rem;
}

.verify-public-rights__grid span {
  color: var(--hs-text-subtle);
}

.verify-public-rights__grid strong {
  overflow-wrap: anywhere;
}

.verify-public-rights p {
  margin: 0.55rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.82rem;
  line-height: 1.5;
}

.verify-public-rights__warning {
  color: var(--hs-warning) !important;
}

.verify-report-export {
  margin-top: 0.9rem;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.75rem;
  align-items: center;
  padding: 0.8rem;
  border: 1px solid rgba(114, 214, 202, 0.24);
  border-radius: 8px;
  background: rgba(114, 214, 202, 0.08);
}

.verify-report-export p {
  margin: 0.25rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.78rem;
  overflow-wrap: anywhere;
}

.verify-report-export__actions {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.verify-case-pack {
  display: grid;
  gap: 0.85rem;
  margin-top: 0.9rem;
  padding: 1rem;
  border: 1px solid rgba(181, 133, 76, 0.36);
  border-radius: var(--hs-radius-card);
  background:
    linear-gradient(135deg, rgba(181, 133, 76, 0.11), rgba(114, 214, 202, 0.05)),
    var(--hs-surface);
}

.verify-case-pack__header {
  display: grid;
  gap: 1rem;
}

.verify-case-pack__header .ghost-button {
  justify-self: start;
}

.verify-case-pack__header strong {
  display: block;
  color: var(--hs-text);
  font-size: 1rem;
}

.verify-case-pack__header p,
.verify-case-pack__message,
.verify-case-pack__boundary,
.verify-case-pack__error {
  margin: 0.3rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.8rem;
  line-height: 1.55;
  overflow-wrap: anywhere;
}

.verify-case-pack__eyebrow {
  display: block;
  margin-bottom: 0.22rem;
  color: var(--hs-copper);
  font-size: 0.72rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.verify-case-pack__error {
  color: var(--hs-danger);
}

.verify-case-pack__status-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.verify-case-pack__status {
  min-width: 0;
  padding: 0.72rem;
  border: 1px solid var(--hs-border);
  border-radius: 7px;
  background: var(--hs-surface-muted);
}

.verify-case-pack__status span,
.verify-case-pack__lineage span,
.verify-case-pack__attachment span,
.verify-case-pack__attachment small {
  display: block;
  color: var(--hs-text-subtle);
  font-size: 0.72rem;
}

.verify-case-pack__status strong {
  display: block;
  margin-top: 0.3rem;
  font-size: 0.92rem;
}

.verify-case-pack__status--matched {
  border-color: rgba(114, 214, 202, 0.34);
  background: rgba(114, 214, 202, 0.09);
}

.verify-case-pack__status--matched strong {
  color: var(--hs-accent);
}

.verify-case-pack__status--boundary {
  border-color: rgba(255, 200, 87, 0.28);
  background: rgba(255, 200, 87, 0.08);
}

.verify-case-pack__status--boundary strong {
  color: var(--hs-warning);
}

.verify-case-pack__status--mismatch {
  border-color: rgba(255, 112, 112, 0.3);
  background: rgba(255, 112, 112, 0.08);
}

.verify-case-pack__status--mismatch strong {
  color: var(--hs-danger);
}

.verify-case-pack__lineage {
  display: grid;
  gap: 0.55rem;
  padding: 0.8rem;
  border: 1px solid var(--hs-border);
  border-radius: 7px;
  background: rgba(10, 20, 28, 0.18);
}

.verify-case-pack__lineage strong {
  display: block;
  margin-top: 0.16rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.76rem;
  overflow-wrap: anywhere;
}

.verify-case-pack__attachments {
  padding-top: 0.75rem;
  border-top: 1px solid var(--hs-border);
}

.verify-case-pack__attachments summary {
  color: var(--hs-text);
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
}

.verify-case-pack__attachment {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(170px, 0.55fr);
  gap: 0.75rem;
  padding: 0.65rem 0;
  border-bottom: 1px solid var(--hs-border);
}

.verify-case-pack__attachment strong {
  display: block;
  margin-top: 0.18rem;
  font-size: 0.76rem;
  overflow-wrap: anywhere;
}

.verify-case-pack__attachment > div:last-child {
  text-align: right;
}

.verify-case-pack__attachment small {
  margin-top: 0.2rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.verify-capability {
  margin: 0.85rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.86rem;
  line-height: 1.55;
}

.verify-scope-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
  margin-top: 0.9rem;
}

.verify-scope-card {
  padding: 0.85rem;
  border: 1px solid rgba(114, 214, 202, 0.24);
  border-radius: var(--hs-radius-card);
  background: rgba(114, 214, 202, 0.08);
}

.verify-scope-card span {
  display: block;
  color: var(--hs-text);
  font-size: 0.82rem;
  font-weight: 700;
}

.verify-scope-card p {
  margin: 0.4rem 0 0;
  color: var(--hs-text-muted);
  font-size: 0.84rem;
  line-height: 1.55;
}

@media (max-width: 1420px) {
  .verify-workspace {
    grid-template-columns: 1fr;
  }

  .verify-input-column {
    position: static;
  }
}

@media (max-width: 860px) {

  .verify-scope-grid {
    grid-template-columns: 1fr;
  }

  .verify-case-pack__status-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .verify-case-pack__attachment {
    grid-template-columns: 1fr;
  }

  .verify-case-pack__attachment > div:last-child {
    text-align: left;
  }
}

.verify-lineage {
  margin-top: 0.9rem;
  padding: 0.85rem;
  border: 1px solid rgba(114, 214, 202, 0.28);
  border-radius: var(--hs-radius-card);
  background: rgba(114, 214, 202, 0.08);
}

.verify-reason {
  margin-top: 0.85rem;
  padding: 0.8rem;
  border: 1px solid rgba(255, 200, 87, 0.22);
  border-radius: var(--hs-radius-card);
  background: rgba(255, 200, 87, 0.08);
}

.verify-reason span {
  display: block;
  color: var(--hs-text-muted);
  font-size: 0.78rem;
}

.verify-reason strong {
  display: block;
  margin-top: 0.2rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  overflow-wrap: anywhere;
}

.verify-reason p {
  margin: 0.45rem 0 0;
}

.verify-checklist {
  margin-top: 0.85rem;
  padding: 0.8rem;
  border: 1px solid rgba(114, 214, 202, 0.24);
  border-radius: var(--hs-radius-card);
  background: rgba(114, 214, 202, 0.08);
}

.verify-checklist span {
  display: block;
  color: var(--hs-text-muted);
  font-size: 0.78rem;
}

.verify-checklist ul {
  margin: 0.45rem 0 0;
  padding-left: 1.1rem;
}

.verify-checklist li {
  margin: 0.25rem 0;
  line-height: 1.5;
}

.verify-lineage strong {
  display: block;
  margin-bottom: 0.55rem;
}

.verify-lineage__row {
  display: grid;
  grid-template-columns: 6rem 1fr;
  gap: 0.75rem;
  font-size: 0.86rem;
  line-height: 1.5;
}

.verify-lineage__row span {
  color: var(--hs-text-muted);
}

.verify-lineage__row b {
  color: var(--hs-text);
  word-break: break-all;
}
</style>
