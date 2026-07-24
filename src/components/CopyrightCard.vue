<script setup lang="ts">
import {
  buildCopyrightSummary,
  formatCopyrightDateTime,
  formatOutputStrategy,
  formatPayloadAuthStatus,
  formatRegistryStatus,
  formatTimeProofService,
  formatTrainingPermissionDeclaration,
  formatWatermarkIssueMode,
  formatWorkSourceDeclaration,
  formatAuthenticityClaimDeclaration,
  type VaultRecord,
} from "../lib/tauri-api";

const props = defineProps<{
  record: VaultRecord;
  highlight?: boolean;
}>();

function formatVerificationStatus(status?: string | null): string {
  if (status === "verified") return "已通过";
  if (status === "failed") return "未通过";
  return "未记录";
}

function displayValue(value?: string | null): string {
  return value?.trim() || "未记录";
}

function displayCreatorName(value?: string | null): string {
  return value?.trim() || "未声明";
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

async function handleCopy() {
  const text = buildCopyrightSummary(props.record);
  await navigator.clipboard.writeText(text);
}
</script>

<template>
  <div class="copyright-card" :class="{ 'copyright-card--highlight': highlight }">
    <div class="copyright-card__badge">
      <span>本地版权记录</span>
      <span v-if="highlight" class="copyright-card__new">新增</span>
      <span class="copyright-card__ai-badge">{{ formatWorkSourceDeclaration(record.workSourceDeclaration) }}</span>
    </div>
    <div class="copyright-card__body">
      <div class="copyright-card__row">
        <span>版权编号</span>
        <strong>{{ record.watermarkUid }}</strong>
      </div>
      <div v-if="record.revision > 1 || record.parentWatermarkUid" class="copyright-card__row">
        <span>版本次数</span>
        <strong>第 {{ record.revision }} 次</strong>
      </div>
      <div v-if="record.parentWatermarkUid" class="copyright-card__row">
        <span>上一版编号</span>
        <strong>{{ record.parentWatermarkUid }}</strong>
      </div>
      <div v-if="record.rewriteReason" class="copyright-card__row">
        <span>更新说明</span>
        <strong>{{ record.rewriteReason }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>处理完成时间</span>
        <strong>{{ formatCopyrightDateTime(record.createdAt) }}</strong>
      </div>
      <div v-if="record.writeVerificationAt" class="copyright-card__row">
        <span>验证完成时间</span>
        <strong>{{ formatCopyrightDateTime(record.writeVerificationAt) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>创作者显示名称</span>
        <strong>{{ displayCreatorName(record.creatorDisplayName) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>身份信息来源</span>
        <strong>用户本地声明</strong>
      </div>
      <div class="copyright-card__row">
        <span>身份核验状态</span>
        <strong>未进行实名认证</strong>
      </div>
      <div class="copyright-card__row">
        <span>保护副本验证</span>
        <strong>{{ formatVerificationStatus(record.writeVerificationStatus) }}</strong>
      </div>
      <div v-if="verificationMessage(record)" class="copyright-card__row">
        <span>验证说明</span>
        <strong>{{ verificationMessage(record) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>版权编号生成方式</span>
        <strong>{{ formatWatermarkIssueMode(record.watermarkIdIssueMode) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>联网登记状态</span>
        <strong>{{ formatRegistryStatus(record.watermarkIdRegistryStatus) }}</strong>
      </div>
      <div v-if="registryReceipt(record)" class="copyright-card__row">
        <span>登记收据编号</span>
        <strong>{{ registryReceipt(record) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>作品指纹（SHA-256）</span>
        <strong class="hash-text">{{ record.originalHash.slice(0, 16) }}...</strong>
      </div>
      <div v-if="record.tsaTokenPath" class="copyright-card__row">
        <span>时间依据</span>
        <strong>第三方时间戳回执</strong>
      </div>
      <div v-else-if="record.networkTime" class="copyright-card__row">
        <span>时间依据</span>
        <strong>{{ networkTimeSource(record) }}</strong>
      </div>
      <div v-else class="copyright-card__row">
        <span>时间依据</span>
        <strong>本机系统时间</strong>
      </div>
      <div v-if="record.networkTime && !record.tsaTokenPath" class="copyright-card__row">
        <span>网络授时时间</span>
        <strong>{{ formatCopyrightDateTime(record.networkTime) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>第三方时间证明</span>
        <strong>{{ record.tsaTokenPath ? "已获取第三方时间戳回执" : "未获取" }}</strong>
      </div>
      <div v-if="record.tsaTokenPath" class="copyright-card__row">
        <span>可信时间</span>
        <strong>{{ formatCopyrightDateTime(record.networkTime || record.createdAt) }}</strong>
      </div>
      <div v-if="record.tsaTokenPath" class="copyright-card__row">
        <span>时间证明服务</span>
        <strong>{{ formatTimeProofService(record.tsaSource) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>保护副本名称</span>
        <strong>{{ record.protectedCopyName?.trim() || "未生成保护副本" }}</strong>
      </div>
      <div v-if="record.protectedCopyHash" class="copyright-card__row">
        <span>保护副本摘要（SHA-256）</span>
        <strong class="hash-text">{{ record.protectedCopyHash.slice(0, 16) }}...</strong>
      </div>
      <div class="copyright-card__row">
        <span>输出策略</span>
        <strong>{{ formatOutputStrategy(record.outputStrategy) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>文件名</span>
        <strong>{{ record.fileName }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>训练许可声明</span>
        <strong>{{ formatTrainingPermissionDeclaration(record.trainingPermissionDeclaration) }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>真实性声明</span>
        <strong>{{ formatAuthenticityClaimDeclaration(record.authenticityClaimDeclaration) }}</strong>
      </div>
      <div v-if="record.customRightsStatement" class="copyright-card__row">
        <span>自定义声明</span>
        <strong>{{ record.customRightsStatement }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>水印协议版本</span>
        <strong>V{{ record.payloadProtocolVersion }}</strong>
      </div>
      <div class="copyright-card__row">
        <span>载荷完整性校验</span>
        <strong>{{ formatPayloadAuthStatus(record.payloadAuthStatus) }}</strong>
      </div>
    </div>
    <button class="ghost-button copyright-card__copy" type="button" @click="handleCopy">
      复制存证摘要
    </button>
  </div>
</template>

<style scoped>
.copyright-card__ai-badge {
  padding: 4px 8px;
  background: var(--hs-chip);
  color: var(--hs-accent);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-pill);
  font-size: 12px;
  font-weight: 600;
}

.copyright-card__verification {
  padding: 2px 8px;
  border-radius: var(--hs-radius-pill);
  font-size: 12px;
  font-weight: 600;
}

.copyright-card__verification--ok {
  background: rgba(114, 214, 202, 0.1);
  color: var(--hs-accent);
}

.copyright-card__verification--warn {
  background: rgba(255, 200, 87, 0.1);
  color: var(--hs-warning);
}
</style>
