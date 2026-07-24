<script setup lang="ts">
import { computed, ref } from "vue";
import {
  createDesktopAuthChallenge,
  continueCloudAccount,
  getIdentityStatus,
  getPreferences,
  savePreferences,
  signOutDesktopCloud,
  setupIdentity,
} from "../lib/tauri-api";
import { userFacingErrorMessage } from "../lib/user-facing-errors";

const emit = defineEmits<{ complete: [] }>();

const creatorInput = ref("");
const accountInput = ref("");
const passwordInput = ref("");
const verificationCodeInput = ref("");
const authChallengeId = ref<string | null>(null);
const authMode = ref<"code" | "password">("code");
const outputDir = ref("");
const step = ref<"account" | "setup">("account");
const baseSetupCompleted = ref(false);
const signedInAccountLabel = ref("");
const useLocalOnly = ref(false);
const loading = ref(false);
const sendingCode = ref(false);
const errorMsg = ref("");
const cloudHint = ref("");

const canSubmit = computed(() => {
  if (loading.value) return false;
  if (step.value === "account") {
    if (!accountInput.value.trim()) return false;
    if (authMode.value === "password") return !!passwordInput.value.trim();
    return !!authChallengeId.value && !!verificationCodeInput.value.trim();
  }
  return !!creatorInput.value.trim() && !!outputDir.value.trim();
});

function continueLocalOnly() {
  if (loading.value) return;
  useLocalOnly.value = true;
  signedInAccountLabel.value = "";
  cloudHint.value = "已跳过云同步；本地图片、音频写入与验证不会依赖云服务。";
  errorMsg.value = "";
  step.value = "setup";
}

async function chooseOutputDir() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择默认输出位置",
  });
  if (typeof selected === "string") {
    outputDir.value = selected;
  }
}

Promise.all([getPreferences(), getIdentityStatus()]).then(([preferences, identityStatus]) => {
  outputDir.value = preferences.defaultOutputDir ?? "";
  baseSetupCompleted.value = preferences.onboardingCompleted;
  if (preferences.onboardingCompleted && !identityStatus.initialized) {
    useLocalOnly.value = true;
    cloudHint.value = "检测到本机创作者身份缺失，请先补完本地写入设置；云同步可稍后在设置中开启。";
    step.value = "setup";
  }
});

async function handleSubmit() {
  if (step.value === "account") {
    if (!accountInput.value.trim()) {
      errorMsg.value = "请输入账户";
      return;
    }
    if (authMode.value === "password" && !passwordInput.value.trim()) {
      errorMsg.value = "请输入密码";
      return;
    }
    if (authMode.value === "code" && (!authChallengeId.value || !verificationCodeInput.value.trim())) {
      errorMsg.value = "请先发送验证码并输入验证码";
      return;
    }
    loading.value = true;
    errorMsg.value = "";
    cloudHint.value = "";
    try {
      const profile = await continueCloudAccount(
        accountInput.value.trim(),
        authMode.value === "password" ? passwordInput.value : "",
        creatorInput.value.trim() || "本机创作者",
        authMode.value === "code" ? authChallengeId.value : null,
        authMode.value === "code" ? verificationCodeInput.value.trim() : null,
      );
      signedInAccountLabel.value = profile.accountLabel || accountInput.value.trim();
      useLocalOnly.value = false;
      if (baseSetupCompleted.value) {
        emit("complete");
      } else {
        step.value = "setup";
      }
    } catch (err: any) {
      errorMsg.value = `${userFacingErrorMessage(err, "账户登录")}；也可以先跳过云同步，完成本地写入设置。`;
    } finally {
      loading.value = false;
    }
    return;
  }
  if (!creatorInput.value.trim()) {
    errorMsg.value = "请输入要写入版权记录的创作者身份";
    return;
  }
  if (!outputDir.value.trim()) {
    errorMsg.value = "请选择保护副本的默认保存位置";
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    await setupIdentity(creatorInput.value.trim());
    await savePreferences({
      defaultOutputDir: outputDir.value.trim() || null,
      onboardingCompleted: true,
    });
    emit("complete");
  } catch (err: any) {
    errorMsg.value = err?.message ?? String(err);
  } finally {
    loading.value = false;
  }
}

async function sendAuthCode() {
  if (!accountInput.value.trim()) {
    errorMsg.value = "请输入账户";
    return;
  }
  sendingCode.value = true;
  errorMsg.value = "";
  try {
    const challenge = await createDesktopAuthChallenge(accountInput.value.trim());
    authChallengeId.value = challenge.challengeId;
    verificationCodeInput.value = challenge.fixtureCode ?? "";
    cloudHint.value = challenge.fixtureCode
      ? `${challenge.message} 验证码已填入。`
      : challenge.message;
  } catch (err: any) {
    errorMsg.value = userFacingErrorMessage(err, "发送验证码");
  } finally {
    sendingCode.value = false;
  }
}

async function changeAccount() {
  if (loading.value) return;
  await signOutDesktopCloud();
  signedInAccountLabel.value = "";
  useLocalOnly.value = false;
  cloudHint.value = "";
  step.value = "account";
}
</script>

<template>
  <div class="id-setup">
    <div class="id-setup__backdrop" />
    <div class="id-setup__card">
      <div class="id-setup__shield">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M12 2L3 7v5c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-9-5z" fill="url(#shield-grad)" opacity="0.95"/>
          <path d="M10 15.5l-3.5-3.5 1.41-1.41L10 12.67l5.59-5.59L17 8.5l-7 7z" fill="#061312"/>
          <defs>
            <linearGradient id="shield-grad" x1="3" y1="2" x2="21" y2="19">
              <stop offset="0%" stop-color="#72d6ca"/>
              <stop offset="100%" stop-color="#d1844d"/>
            </linearGradient>
          </defs>
        </svg>
      </div>

      <h1 class="id-setup__title">{{ step === "account" ? "登录或创建账户" : "完成使用前设置" }}</h1>
      <p class="id-setup__subtitle">{{ step === "account" ? "云同步可稍后开启，本地保护可直接使用。" : "设置本地创作者身份和保护副本保存位置。" }}</p>

      <div v-if="step === 'account'" class="id-setup__input-wrap">
        <label class="id-setup__label">账户</label>
        <input
          v-model="accountInput"
          type="text"
          class="id-setup__input"
          placeholder="输入邮箱或手机号"
          :disabled="loading"
          @keydown.enter="handleSubmit"
        />
      </div>

      <div v-if="step === 'account'" class="id-setup__auth-tabs" role="tablist" aria-label="登录方式">
        <button type="button" :class="{ active: authMode === 'code' }" @click="authMode = 'code'">
          验证码登录
        </button>
        <button type="button" :class="{ active: authMode === 'password' }" @click="authMode = 'password'">
          密码登录
        </button>
      </div>

      <div v-if="step === 'account' && authMode === 'password'" class="id-setup__input-wrap">
        <label class="id-setup__label">密码</label>
        <input
          v-model="passwordInput"
          type="password"
          class="id-setup__input"
          placeholder="输入账户密码"
          autocomplete="current-password"
          :disabled="loading"
          @keydown.enter="handleSubmit"
        />
      </div>

      <div v-if="step === 'account' && authMode === 'code'" class="id-setup__input-wrap">
        <label class="id-setup__label">验证码</label>
        <div class="id-setup__output-row">
          <input
            v-model="verificationCodeInput"
            type="text"
            inputmode="numeric"
            class="id-setup__input"
            placeholder="输入 6 位验证码"
            :disabled="loading"
            @keydown.enter="handleSubmit"
          />
          <button class="id-setup__secondary" type="button" :disabled="loading || sendingCode" @click="sendAuthCode">
            {{ sendingCode ? "发送中" : "发送" }}
          </button>
        </div>
      </div>

      <div v-if="step === 'setup'" class="id-setup__account-summary">
        <span>{{ useLocalOnly ? "使用方式" : "当前账户" }}</span>
        <strong>{{ useLocalOnly ? "本地使用" : signedInAccountLabel }}</strong>
        <button v-if="!useLocalOnly" type="button" :disabled="loading" @click="changeAccount">更换</button>
        <button v-else type="button" :disabled="loading" @click="changeAccount">登录</button>
      </div>

      <div v-if="step === 'setup'" class="id-setup__input-wrap">
        <label class="id-setup__label">创作者身份</label>
        <p class="id-setup__field-note">写入版权记录和盲水印。</p>
        <input
          v-model="creatorInput"
          type="text"
          class="id-setup__input"
          placeholder="例如：工作室名称、艺名或公司名称"
          :disabled="loading"
          @keydown.enter="handleSubmit"
        />
      </div>

      <div v-if="step === 'setup'" class="id-setup__input-wrap">
        <label class="id-setup__label">默认输出位置</label>
        <p class="id-setup__field-note">保护副本默认保存路径。</p>
        <div class="id-setup__output-row">
          <input
            v-model="outputDir"
            type="text"
            class="id-setup__input"
            placeholder="请选择保护副本保存文件夹"
            :disabled="loading"
          />
          <button class="id-setup__secondary" type="button" :disabled="loading" @click="chooseOutputDir">
            选择
          </button>
        </div>
      </div>

      <p v-if="cloudHint" class="id-setup__hint">{{ cloudHint }}</p>
      <p v-if="errorMsg" class="id-setup__error">{{ errorMsg }}</p>

      <button
        class="id-setup__btn"
        type="button"
        :disabled="!canSubmit"
        @click="handleSubmit"
      >
        <span v-if="loading" class="id-setup__spinner" />
        {{ loading ? "正在准备" : step === "account" ? "继续设置" : "进入工作台" }}
      </button>

      <button
        v-if="step === 'account'"
        class="id-setup__link-btn"
        type="button"
        :disabled="loading"
        @click="continueLocalOnly"
      >
        跳过云同步，本地使用
      </button>

      <div class="id-setup__footer">
        <span>原始媒体、保护副本和本地路径默认留在本机。</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.id-setup {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.id-setup__backdrop {
  position: absolute;
  inset: 0;
  background: rgba(3, 8, 17, 0.88);
}

.id-setup__card {
  position: relative;
  width: 100%;
  max-width: 420px;
  padding: 48px 40px 36px;
  background: var(--hs-surface);
  backdrop-filter: none;
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  text-align: center;
  box-shadow: none;
  animation: card-in 0.4s ease-out;
}

@keyframes card-in {
  from {
    opacity: 0;
    transform: translateY(20px) scale(0.96);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.id-setup__shield {
  margin-bottom: 20px;
}

.id-setup__title {
  margin: 0 0 4px;
  font-size: 22px;
  font-weight: 700;
  color: var(--hs-text);
  letter-spacing: -0.3px;
}

.id-setup__subtitle {
  margin: 0 0 24px;
  font-size: 14px;
  color: var(--hs-text-muted);
}

.id-setup__input-wrap {
  margin-bottom: 16px;
  text-align: left;
}

.id-setup__auth-tabs {
  display: inline-flex;
  gap: 4px;
  margin: 0 0 16px;
  padding: 4px;
  background: var(--hs-surface-muted);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
}

.id-setup__auth-tabs button {
  padding: 8px 12px;
  color: var(--hs-text-muted);
  background: transparent;
  border: 0;
  border-radius: calc(var(--hs-radius-card) - 2px);
  cursor: pointer;
}

.id-setup__auth-tabs button.active {
  color: var(--hs-text);
  background: var(--hs-surface-raised);
}

.id-setup__account-summary {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  padding: 10px 12px;
  text-align: left;
  color: var(--hs-text-muted);
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
}

.id-setup__account-summary strong {
  overflow: hidden;
  color: var(--hs-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.id-setup__account-summary button {
  color: var(--hs-text);
  background: transparent;
  border: none;
  cursor: pointer;
}

.id-setup__label {
  display: block;
  margin-bottom: 4px;
  font-size: 13px;
  font-weight: 700;
  color: var(--hs-text-muted);
}

.id-setup__field-note {
  margin: 0 0 8px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--hs-text-subtle);
}

.id-setup__output-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}

.id-setup__input {
  width: 100%;
  padding: 14px 18px;
  font-size: 16px;
  color: var(--hs-text);
  background: var(--hs-surface-muted);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.id-setup__secondary {
  padding: 0 14px;
  color: var(--hs-text);
  background: var(--hs-surface-muted);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
}

.id-setup__input::placeholder {
  color: var(--hs-text-subtle);
}

.id-setup__input:focus {
  border-color: var(--hs-accent);
  box-shadow: 0 0 0 3px rgba(114, 214, 202, 0.14);
}

.id-setup__error {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--hs-danger);
}

.id-setup__hint {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--hs-text-muted);
}

.id-setup__btn {
  width: 100%;
  padding: 14px;
  font-size: 15px;
  font-weight: 600;
  color: #061312;
  background: var(--hs-accent);
  border: none;
  border-radius: var(--hs-radius-card);
  cursor: pointer;
  transition: opacity 0.2s, transform 0.1s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.id-setup__btn:hover:not(:disabled) {
  opacity: 0.9;
  transform: translateY(-1px);
}

.id-setup__btn:active:not(:disabled) {
  transform: translateY(0);
}

.id-setup__btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.id-setup__link-btn {
  width: 100%;
  margin-top: 10px;
  padding: 10px;
  color: var(--hs-text-muted);
  background: transparent;
  border: none;
  cursor: pointer;
}

.id-setup__link-btn:hover:not(:disabled) {
  color: var(--hs-text);
}

.id-setup__link-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.id-setup__spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(6, 19, 18, 0.3);
  border-top-color: #061312;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.id-setup__footer {
  margin-top: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 12px;
  color: var(--hs-text-subtle);
}
</style>
