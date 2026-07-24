<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  inspectActivationRequest,
  issueLicense,
  issuerReadiness,
  type ActivationRequest,
  type IssueLicenseOutput,
} from "./tauri-api";

const requestPath = ref("");
const request = ref<ActivationRequest | null>(null);
const issued = ref<IssueLicenseOutput | null>(null);
const error = ref("");
const notice = ref("");
const loading = ref(false);
const engineReady = ref(false);

const canIssue = computed(
  () => !!request.value && engineReady.value && !loading.value,
);

onMounted(async () => {
  try {
    await issuerReadiness();
    engineReady.value = true;
  } catch (cause) {
    error.value = messageFrom(cause);
  }
});

async function chooseRequest() {
  const selected = await open({
    title: "导入客户激活请求",
    multiple: false,
    directory: false,
    filters: [{ name: "HiddenShield 激活请求", extensions: ["hsreq"] }],
  });
  if (typeof selected !== "string") return;
  requestPath.value = selected;
  request.value = null;
  issued.value = null;
  await inspectRequest();
}

async function inspectRequest() {
  if (!requestPath.value) return;
  await run(async () => {
    request.value = await inspectActivationRequest(requestPath.value);
    notice.value = "激活请求已验证，可签发绑定到该安装实例的年度授权。";
  });
}

async function issue() {
  await run(async () => {
    issued.value = await issueLicense(requestPath.value);
    notice.value = "授权已签发。请将许可证文件或长注册码交付给客户。";
  });
}

async function copyToken() {
  if (!issued.value) return;
  try {
    await navigator.clipboard.writeText(issued.value.token);
    notice.value = "HSLIC1 长注册码已复制。";
  } catch {
    error.value = "无法访问剪贴板，请从下方文本框手动复制。";
  }
}

async function run(action: () => Promise<void>) {
  loading.value = true;
  error.value = "";
  notice.value = "";
  try {
    await action();
  } catch (cause) {
    error.value = messageFrom(cause);
  } finally {
    loading.value = false;
  }
}

function messageFrom(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}
</script>

<template>
  <main class="issuer-shell">
    <header class="issuer-header">
      <div>
        <p class="eyebrow">INTERNAL OPERATIONS · OFFLINE LICENSE</p>
        <h1>授权签发台</h1>
        <p class="subtitle">导入客户激活请求，签发绑定设备的一年期离线授权。</p>
      </div>
      <span class="engine-status" :class="{ ready: engineReady }">
        {{ engineReady ? "签发引擎就绪" : "签发引擎未就绪" }}
      </span>
    </header>

    <section class="security-note">
      <strong>内部工具</strong>
      <span>签发密钥与口令由当前 Windows 用户的服务方 DPAPI 目录自动读取，不显示、不保存到界面，也不会进入客户安装包。</span>
    </section>

    <p v-if="error" class="message error">{{ error }}</p>
    <p v-if="notice" class="message success">{{ notice }}</p>

    <div class="issuer-grid">
      <section class="card request-card">
        <div class="section-heading">
          <span>01</span>
          <div>
            <h2>导入并校验请求</h2>
            <p>客户导出的 `.hsreq` 文件只能用于对应安装实例。</p>
          </div>
        </div>
        <button class="button secondary" type="button" :disabled="loading" @click="chooseRequest">
          选择激活请求
        </button>
        <p v-if="requestPath" class="file-path">{{ requestPath }}</p>
        <button
          v-if="requestPath"
          class="text-button"
          type="button"
          :disabled="loading"
          @click="inspectRequest"
        >
          重新校验
        </button>
        <dl v-if="request" class="request-details">
          <div><dt>请求编号</dt><dd>{{ request.payload.requestId }}</dd></div>
          <div><dt>安装实例</dt><dd>{{ request.payload.installationId }}</dd></div>
          <div><dt>请求时间</dt><dd>{{ request.payload.createdAt }}</dd></div>
        </dl>
      </section>

      <section class="card issue-card">
        <div class="section-heading">
          <span>02</span>
          <div>
            <h2>签发年度授权</h2>
            <p>自动分配当天序号，固定记录操作员 ops-jihx，并以当前时间起 364 天为有效期。</p>
          </div>
        </div>
        <dl class="defaults-list">
          <div><dt>客户参考号</dt><dd>UTC 日期 + 当天 5 位自增序列</dd></div>
          <div><dt>操作员</dt><dd>ops-jihx</dd></div>
          <div><dt>有效期</dt><dd>签发时刻起 364 天</dd></div>
          <div><dt>交付位置</dt><dd>Documents/HiddenShield-License-Delivery/客户参考号</dd></div>
        </dl>
        <button class="button primary issue-button" type="button" :disabled="!canIssue" @click="issue">
          {{ loading ? "正在签发…" : "签发授权" }}
        </button>
      </section>
    </div>

    <section v-if="issued" class="card delivery-card">
      <div class="section-heading">
        <span>03</span>
        <div>
          <h2>交付给客户</h2>
          <p>优先发送 `.hslicense` 文件；也可复制下方 HSLIC1 长注册码。</p>
        </div>
      </div>
      <dl class="delivery-details">
        <div><dt>客户参考号</dt><dd>{{ issued.customerReference }}</dd></div>
        <div><dt>许可证编号</dt><dd>{{ issued.licenseId }}</dd></div>
        <div><dt>到期时间</dt><dd>{{ issued.expiresAt }}</dd></div>
        <div><dt>交付文件</dt><dd>{{ issued.licensePath }}</dd></div>
        <div><dt>审计文件</dt><dd>{{ issued.auditPath }}</dd></div>
      </dl>
      <label>
        HSLIC1 长注册码
        <textarea readonly :value="issued.token" rows="4" />
      </label>
      <button class="button primary" type="button" @click="copyToken">复制注册码</button>
    </section>
  </main>
</template>
