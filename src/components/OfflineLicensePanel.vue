<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { userFacingErrorMessage } from "../lib/user-facing-errors";
import {
  clearOfflineLicense,
  exportOfflineActivationRequest,
  getOfflineLicenseStatus,
  importOfflineLicense,
  importOfflineRevocationList,
  type OfflineLicenseStatus,
} from "../lib/tauri-api";

const emit = defineEmits<{
  entitlementRefreshed: [];
}>();

const status = ref<OfflineLicenseStatus | null>(null);
const tokenInput = ref("");
const loading = ref(false);
const message = ref("");

const statusLabel = computed(() => {
  const value = status.value?.status ?? "none";
  if (value === "active") return "已激活";
  if (value === "expired") return "已过期";
  if (value === "revoked") return "已撤销";
  if (value === "device_mismatch") return "设备不匹配";
  if (value === "invalid") return "许可证无效";
  return "未激活";
});

async function refresh() {
  status.value = await getOfflineLicenseStatus();
}

async function run(action: () => Promise<void>, fallback: string) {
  loading.value = true;
  message.value = "";
  try {
    await action();
  } catch (error: unknown) {
    message.value = userFacingErrorMessage(error, fallback);
  } finally {
    loading.value = false;
  }
}

async function exportRequest() {
  await run(async () => {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const outputPath = await save({
      title: "导出 HiddenShield 离线激活请求",
      defaultPath: "HiddenShield-activation-request.hsreq",
      filters: [{ name: "HiddenShield 激活请求", extensions: ["hsreq"] }],
    });
    if (!outputPath) return;
    const result = await exportOfflineActivationRequest(outputPath);
    message.value = `激活请求已导出：${result.outputPath ?? outputPath}`;
    await refresh();
  }, "导出离线激活请求");
}

async function importLicenseFile() {
  await run(async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      title: "导入 HiddenShield 离线许可证",
      multiple: false,
      directory: false,
      filters: [{ name: "HiddenShield 许可证", extensions: ["hslicense", "txt"] }],
    });
    if (typeof selected !== "string") return;
    status.value = await importOfflineLicense(selected);
    message.value = "许可证已验证并导入。";
    emit("entitlementRefreshed");
  }, "导入离线许可证");
}

async function importPastedToken() {
  const token = tokenInput.value.trim();
  if (!token) {
    message.value = "请先粘贴 HSLIC1 长激活码。";
    return;
  }
  await run(async () => {
    status.value = await importOfflineLicense(token);
    tokenInput.value = "";
    message.value = "长激活码已验证并导入。";
    emit("entitlementRefreshed");
  }, "激活离线许可证");
}

async function importRevocations() {
  await run(async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      title: "导入 HiddenShield 签名撤销列表",
      multiple: false,
      directory: false,
      filters: [{ name: "HiddenShield 撤销列表", extensions: ["hsrvl", "txt"] }],
    });
    if (typeof selected !== "string") return;
    status.value = await importOfflineRevocationList(selected);
    message.value = "撤销列表已验证并应用。";
    emit("entitlementRefreshed");
  }, "导入撤销列表");
}

async function clearLicense() {
  await run(async () => {
    const { confirm } = await import("@tauri-apps/plugin-dialog");
    const confirmed = await confirm(
      "清除后，本地批量处理和正式报告将恢复为云端权益决定。installation identity 不会被删除。",
      { title: "清除离线许可证", kind: "warning" },
    );
    if (!confirmed) return;
    status.value = await clearOfflineLicense();
    message.value = "离线许可证已清除。";
    emit("entitlementRefreshed");
  }, "清除离线许可证");
}

onMounted(() => {
  void run(refresh, "读取离线许可证状态");
});
</script>

<template>
  <section class="offline-license-card">
    <div class="offline-license-card__header">
      <div>
        <span class="offline-license-card__eyebrow">图片 / 音频年费</span>
        <strong>年度注册码授权</strong>
        <p>一年期注册码只开放本地批量处理；正式报告始终单独购买，云能力和未来视频也不随注册码开放。</p>
      </div>
      <span class="offline-license-card__status" :data-status="status?.status ?? 'none'">
        {{ statusLabel }}
      </span>
    </div>

    <dl class="offline-license-card__facts">
      <div>
        <dt>Installation ID</dt>
        <dd>{{ status?.installationId ?? "初始化中" }}</dd>
      </div>
      <div>
        <dt>许可证</dt>
        <dd>{{ status?.licenseId ?? "未导入" }}</dd>
      </div>
      <div>
        <dt>有效期至</dt>
        <dd>{{ status?.expiresAt ?? "—" }}</dd>
      </div>
      <div>
        <dt>签发密钥</dt>
        <dd>{{ status?.keyId ?? "—" }}</dd>
      </div>
    </dl>

    <div class="offline-license-card__actions">
      <button type="button" :disabled="loading" @click="exportRequest">导出激活请求</button>
      <button type="button" :disabled="loading" @click="importLicenseFile">导入许可证文件</button>
      <button type="button" :disabled="loading" @click="importRevocations">导入撤销列表</button>
      <button
        v-if="status?.licenseId"
        class="offline-license-card__danger"
        type="button"
        :disabled="loading"
        @click="clearLicense"
      >
        清除许可证
      </button>
    </div>

    <div class="offline-license-card__paste">
      <textarea
        v-model="tokenInput"
        rows="3"
        spellcheck="false"
        placeholder="粘贴 HSLIC1.<payload>.<signature> 长激活码"
      />
      <button type="button" :disabled="loading || !tokenInput.trim()" @click="importPastedToken">
        验证并激活
      </button>
    </div>

    <p v-if="status?.errorCode" class="offline-license-card__warning">
      最近验证结果：{{ status.errorCode }}
    </p>
    <p v-if="message" class="offline-license-card__message">{{ message }}</p>
  </section>
</template>

<style scoped>
.offline-license-card {
  display: grid;
  gap: 16px;
  padding: 20px;
  border: 1px solid rgba(118, 160, 255, 0.24);
  border-radius: 18px;
  background: linear-gradient(145deg, rgba(32, 43, 70, 0.96), rgba(17, 25, 45, 0.96));
}

.offline-license-card__header,
.offline-license-card__actions,
.offline-license-card__paste {
  display: flex;
  align-items: center;
  gap: 12px;
}

.offline-license-card__header {
  justify-content: space-between;
}

.offline-license-card__header strong {
  display: block;
  margin-top: 4px;
  font-size: 18px;
}

.offline-license-card__header p,
.offline-license-card__message,
.offline-license-card__warning {
  margin: 6px 0 0;
  color: rgba(224, 231, 248, 0.72);
}

.offline-license-card__eyebrow {
  color: #8cb2ff;
  font-size: 12px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.offline-license-card__status {
  padding: 7px 11px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.16);
  color: #d8e0ef;
}

.offline-license-card__status[data-status="active"] {
  background: rgba(61, 205, 151, 0.16);
  color: #76e5b6;
}

.offline-license-card__facts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  margin: 0;
}

.offline-license-card__facts div {
  min-width: 0;
  padding: 12px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.045);
}

.offline-license-card__facts dt {
  color: rgba(224, 231, 248, 0.56);
  font-size: 12px;
}

.offline-license-card__facts dd {
  overflow: hidden;
  margin: 5px 0 0;
  color: #f5f7fb;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  text-overflow: ellipsis;
}

.offline-license-card__actions {
  flex-wrap: wrap;
}

.offline-license-card button {
  padding: 9px 13px;
  border: 1px solid rgba(137, 173, 255, 0.3);
  border-radius: 10px;
  background: rgba(91, 132, 238, 0.14);
  color: #eef3ff;
  cursor: pointer;
}

.offline-license-card button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.offline-license-card__danger {
  border-color: rgba(255, 120, 120, 0.34) !important;
  background: rgba(179, 53, 68, 0.13) !important;
}

.offline-license-card__paste {
  align-items: stretch;
}

.offline-license-card__paste textarea {
  flex: 1;
  min-width: 0;
  resize: vertical;
  padding: 11px;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 10px;
  background: rgba(5, 10, 22, 0.46);
  color: #f5f7fb;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.offline-license-card__warning {
  color: #f8c97d;
}

@media (max-width: 720px) {
  .offline-license-card__facts {
    grid-template-columns: 1fr;
  }

  .offline-license-card__paste {
    flex-direction: column;
  }
}
</style>
