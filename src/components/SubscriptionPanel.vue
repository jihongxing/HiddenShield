<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import OfflineLicensePanel from "./OfflineLicensePanel.vue";
import { trackClick } from "../lib/analytics";
import { userFacingErrorMessage } from "../lib/user-facing-errors";
import {
  createBillingPaymentSession,
  getBillingPaymentSessionStatus,
  getDesktopCloudSyncProfile,
  reconcileBillingPaymentSession,
  type BillingPaymentSession,
  type EntitlementState,
} from "../lib/tauri-api";

type PlanCode = "free" | "creator";

const props = defineProps<{
  entitlementState: EntitlementState | null;
  embedded?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  entitlementRefreshed: [];
}>();

const plans: Array<{
  code: PlanCode;
  name: string;
  tag: string;
  audience: string;
  items: string[];
  note: string;
}> = [
  {
    code: "free",
    name: "未付费",
    tag: "基础状态",
    audience: "图片 / 音频单文件用户",
    items: ["单文件图片写入与验证", "单文件音频写入与验证", "本地版权库"],
    note: "未付费不能使用批量处理。正式报告无论是否年费激活都需要按记录单独购买。",
  },
  {
    code: "creator",
    name: "图片 / 音频年费已激活",
    tag: "年度基础权益",
    audience: "需要批量处理的用户",
    items: ["图片批量处理", "音频批量处理", "一年有效，按年续费激活"],
    note: "年费不包含正式报告，也不包含未来视频服务。登录后的云能力继续由服务端订单和账户权益决定。",
  },
];

const currentStatus = computed(() => props.entitlementState?.status ?? "free");
const currentPlanCode = computed<PlanCode>(() => {
  const planCode = props.entitlementState?.planCode;
  return planCode && planCode !== "free" ? "creator" : "free";
});
const currentPlan = computed(() => plans.find((plan) => plan.code === currentPlanCode.value) ?? plans[0]);
const enabledFeatures = computed(() => {
  const features = props.entitlementState?.features ?? {};
  return Object.entries(features)
    .filter(([key, enabled]) =>
      enabled && ["cloud_sync", "batch_processing"].includes(key))
    .map(([key]) => featureLabel(key));
});
const paymentSession = ref<BillingPaymentSession | null>(null);
const paymentSessionStatus = ref("");
const paymentMessage = ref("");
const paymentLoadingPlan = ref<PlanCode | null>(null);
const entitlementRefreshing = ref(false);
const pollingStartedAt = ref<number | null>(null);

let previousBodyOverflow = "";
let paymentPollTimer: number | null = null;

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") close();
}

onMounted(() => {
  if (!props.embedded) {
    previousBodyOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
  }
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  stopPaymentPolling();
  if (!props.embedded) {
    document.body.style.overflow = previousBodyOverflow;
  }
  window.removeEventListener("keydown", handleKeydown);
});

function formatDateTime(value: string | null): string {
  if (!value) return "—";
  return new Date(value).toLocaleString();
}

function statusLabel(status: EntitlementState["status"]): string {
  const labels: Record<EntitlementState["status"], string> = {
    free: "未付费",
    trial: "年费试用中",
    active: "年费已激活",
    grace: "年费宽限期",
    expired: "年费已到期",
  };
  return labels[status];
}

function featureLabel(key: string): string {
  const map: Record<string, string> = {
    cloud_sync: "云同步",
    batch_processing: "批量处理",
  };
  return map[key] ?? key;
}

async function startPayment(plan: PlanCode) {
  if (plan === "free" || currentPlanCode.value === plan) {
    return;
  }
  paymentLoadingPlan.value = plan;
  paymentMessage.value = "";
  paymentSession.value = null;
  paymentSessionStatus.value = "";
  stopPaymentPolling();
  try {
    const profile = await getDesktopCloudSyncProfile();
    if (!profile?.accessToken) {
      paymentMessage.value = "请先在设置中登录账户，再开通图片 / 音频年费。";
      return;
    }
    const session = await createBillingPaymentSession(plan, "yearly", "wechat_pay");
    paymentSession.value = session;
    paymentSessionStatus.value = "created";
    paymentMessage.value = session.paymentAction.type === "qr_code"
      ? "请使用微信扫码完成支付。我们会在短时间内自动确认支付状态。"
      : "请按页面提示完成支付。我们会在短时间内自动确认支付状态。";
    startPaymentPolling();
    trackClick(`subscription_payment_${plan}`);
  } catch (error: unknown) {
    console.warn("create billing payment session failed", error);
    paymentMessage.value = userFacingErrorMessage(error, "创建支付会话");
  } finally {
    paymentLoadingPlan.value = null;
  }
}

async function refreshEntitlement() {
  const sessionId = paymentSession.value?.paymentSessionId;
  if (!sessionId) return;
  entitlementRefreshing.value = true;
  try {
    const result = await reconcileBillingPaymentSession(sessionId);
    paymentSessionStatus.value = result.status;
    paymentMessage.value = result.message || paymentSessionStatusMessage(result.status);
    if (result.status === "succeeded") {
      stopPaymentPolling();
    }
    emit("entitlementRefreshed");
  } catch (error: unknown) {
    console.warn("refresh entitlement failed", error);
    paymentMessage.value = userFacingErrorMessage(error, "刷新年度授权状态");
  } finally {
    entitlementRefreshing.value = false;
  }
}

async function pollPaymentSession() {
  const sessionId = paymentSession.value?.paymentSessionId;
  if (!sessionId) {
    stopPaymentPolling();
    return;
  }
  const startedAt = pollingStartedAt.value;
  if (startedAt && Date.now() - startedAt > 120_000) {
    stopPaymentPolling();
    paymentMessage.value = "暂未确认支付完成，可稍后手动确认支付状态。";
    return;
  }
  try {
    const status = await getBillingPaymentSessionStatus(sessionId);
    paymentSessionStatus.value = status.status;
    if (status.status === "succeeded") {
      stopPaymentPolling();
      paymentMessage.value = "支付已确认，权益已生效。";
      emit("entitlementRefreshed");
      return;
    }
    if (status.status === "failed" || status.status === "expired") {
      stopPaymentPolling();
      paymentMessage.value = paymentSessionStatusMessage(status.status);
      return;
    }
    const result = await reconcileBillingPaymentSession(sessionId);
    paymentSessionStatus.value = result.status;
    paymentMessage.value = result.message || paymentSessionStatusMessage(result.status);
    if (result.status === "succeeded") {
      stopPaymentPolling();
      emit("entitlementRefreshed");
    }
  } catch {
    paymentMessage.value = "暂未确认支付完成，可稍后手动确认支付状态。";
  }
}

function startPaymentPolling() {
  stopPaymentPolling();
  pollingStartedAt.value = Date.now();
  paymentPollTimer = window.setInterval(() => {
    void pollPaymentSession();
  }, 10_000);
  window.setTimeout(() => {
    void pollPaymentSession();
  }, 1_000);
}

function stopPaymentPolling() {
  if (paymentPollTimer !== null) {
    window.clearInterval(paymentPollTimer);
    paymentPollTimer = null;
  }
}

function paymentSessionStatusMessage(status: string): string {
  if (status === "succeeded") return "支付已确认，权益已生效。";
  if (status === "pending" || status === "created") return "尚未确认支付完成，请完成支付或稍后确认。";
  if (status === "expired") return "支付会话已过期，请重新创建支付。";
  if (status === "failed" || status === "closed") return "支付未完成，请重新创建支付。";
  return "暂未检测到支付完成，请稍后再试。";
}

function openPaymentLink() {
  const action = paymentSession.value?.paymentAction;
  const url = action?.h5Url ?? action?.qrCodeUrl;
  if (!url) return;
  window.open(url, "_blank");
}

function close() {
  emit("close");
}
</script>

<template>
  <div
    class="subscription-overlay"
    :class="{ 'subscription-overlay--embedded': embedded }"
    role="dialog"
    :aria-modal="embedded ? 'false' : 'true'"
    aria-labelledby="subscription-title"
    @click.self="close"
  >
    <div class="subscription-panel">
      <header class="subscription-header">
        <div>
          <p class="subscription-kicker">年度授权</p>
          <h2 id="subscription-title">未付费 / 图片音频年费</h2>
        </div>
        <button class="icon-button" type="button" aria-label="关闭" @click="close">×</button>
      </header>

      <section class="current-card" :class="{ 'current-card--expired': currentStatus === 'expired' }">
        <div>
          <span class="plan-pill">{{ statusLabel(currentStatus) }}</span>
          <strong>{{ currentPlan.name }}</strong>
          <p>
            当前可用：{{ enabledFeatures.length ? enabledFeatures.join(" / ") : "本地单文件处理与基础版权库" }}
          </p>
        </div>
        <div class="current-card__meta">
          <span>更新时间</span>
          <strong>{{ formatDateTime(entitlementState?.updatedAt ?? null) }}</strong>
        </div>
      </section>

      <OfflineLicensePanel @entitlement-refreshed="emit('entitlementRefreshed')" />

      <section v-if="paymentMessage || paymentSession" class="payment-card">
        <div>
          <strong>{{ paymentSession ? "支付会话已创建" : "开通提示" }}</strong>
          <p>{{ paymentMessage }}</p>
          <p v-if="paymentSession" class="payment-card__meta">
            订单号：{{ paymentSession.providerOrderId }} · 状态 {{ paymentSessionStatus || "created" }} · 有效期至 {{ formatDateTime(paymentSession.expiresAt) }}
          </p>
        </div>
        <button
          v-if="paymentSession?.paymentAction.qrCodeUrl || paymentSession?.paymentAction.h5Url"
          class="payment-card__button"
          type="button"
          @click="openPaymentLink"
        >
          打开支付动作
        </button>
        <button
          v-if="paymentSession"
          class="payment-card__button payment-card__button--primary"
          type="button"
          :disabled="entitlementRefreshing"
          @click="refreshEntitlement"
        >
          {{ entitlementRefreshing ? "确认中" : "确认支付" }}
        </button>
      </section>

      <section class="plans-grid" aria-label="年度权益对比">
        <article
          v-for="plan in plans"
          :key="plan.code"
          class="plan-card"
          :class="{
            'plan-card--current': plan.code === currentPlanCode,
            'plan-card--primary': plan.code === 'creator',
          }"
        >
          <div class="plan-card__head">
            <span>{{ plan.tag }}</span>
            <strong>{{ plan.name }}</strong>
          </div>
          <p class="plan-card__audience">{{ plan.audience }}</p>
          <ul>
            <li v-for="item in plan.items" :key="item">{{ item }}</li>
          </ul>
          <p class="plan-card__note">{{ plan.note }}</p>
          <button
            v-if="plan.code !== 'free'"
            class="plan-card__button"
            type="button"
            :disabled="paymentLoadingPlan === plan.code || currentPlanCode === plan.code"
            @click="startPayment(plan.code)"
          >
            {{
              paymentLoadingPlan === plan.code
                ? "创建中"
                : currentPlanCode === plan.code
                  ? "当前状态"
                  : "开通图片 / 音频年费"
            }}
          </button>
          <span v-else class="plan-card__button plan-card__button--static">当前入口</span>
        </article>
      </section>

      <section class="subscription-footnotes">
        <div>
          <strong>批量处理</strong>
          <p>未付费不能使用批量处理；导入有效年度注册码或完成年度付费后开放图片 / 音频批量处理。</p>
        </div>
        <div>
          <strong>当前发布范围</strong>
          <p>当前只发布桌面端图片 / 音频与后端云服务；移动端和全部视频能力均已暂停。未来视频将独立收费。</p>
        </div>
        <div>
          <strong>报告单独购买</strong>
          <p>未付费和年费已激活用户都必须按记录单独购买正式报告；报告不包含在年度注册码中。</p>
        </div>
        <div>
          <strong>年度激活</strong>
          <p>注册码和在线基础权益均按年生效；到期后回到未付费状态，但不会删除本地版权记录。</p>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.subscription-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  background: rgba(3, 8, 17, 0.76);
  backdrop-filter: none;
}

.subscription-overlay--embedded {
  position: static;
  inset: auto;
  z-index: auto;
  display: block;
  padding: 0;
  background: transparent;
}

.subscription-overlay--embedded .subscription-panel {
  width: 100% !important;
  max-width: 100%;
  max-height: none;
}

.subscription-panel {
  width: min(1040px, 100%);
  max-height: min(90vh, 860px);
  overflow: auto;
  border-radius: var(--hs-radius-card);
  border: 1px solid var(--hs-border);
  background: var(--hs-surface);
  color: var(--hs-text);
  box-shadow: none;
  padding: 1.5rem;
}

.subscription-header,
.current-card,
.subscription-footnotes {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.subscription-kicker {
  margin: 0;
  color: var(--hs-accent);
  font-size: 0.78rem;
}

.subscription-header h2 {
  margin: 0.3rem 0 0;
  font-size: 1.45rem;
}

.icon-button {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--hs-border);
  background: var(--hs-surface-muted);
  color: var(--hs-text);
  cursor: pointer;
  font-size: 1.3rem;
}

.current-card {
  margin-top: 1rem;
  padding: 1rem;
  border-radius: var(--hs-radius-card);
  background: rgba(89, 210, 194, 0.08);
  border: 1px solid rgba(89, 210, 194, 0.2);
}

.current-card--expired {
  background: rgba(255, 200, 87, 0.08);
  border-color: rgba(255, 200, 87, 0.28);
}

.current-card strong {
  display: block;
  margin: 0.35rem 0;
  font-size: 1.08rem;
}

.current-card p,
.plan-card p,
.subscription-footnotes p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  line-height: 1.55;
  font-size: 0.88rem;
}

.current-card__meta {
  text-align: right;
}

.current-card__meta span {
  display: block;
  color: var(--hs-text-subtle);
  font-size: 0.74rem;
}

.plan-pill {
  display: inline-flex;
  padding: 0.22rem 0.52rem;
  border-radius: var(--hs-radius-pill);
  background: var(--hs-chip);
  border: 1px solid var(--hs-border);
  color: var(--hs-accent);
  font-size: 0.72rem;
}

.payment-card {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  margin-top: 1rem;
  padding: 1rem;
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
}

.payment-card p {
  margin: 0.35rem 0 0;
  color: var(--hs-text-muted);
  line-height: 1.5;
  font-size: 0.88rem;
}

.payment-card__meta {
  color: var(--hs-text-subtle) !important;
}

.payment-card__button {
  align-self: center;
  min-width: 120px;
  min-height: 38px;
  border-radius: 8px;
  border: 1px solid rgba(89, 210, 194, 0.28);
  background: rgba(89, 210, 194, 0.12);
  color: var(--hs-accent);
  cursor: pointer;
}

.payment-card__button:disabled {
  opacity: 0.62;
  cursor: wait;
}

.payment-card__button--primary {
  background: var(--hs-accent);
  color: #061312;
  font-weight: 700;
}

.plans-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.8rem;
  margin-top: 1rem;
}

.plan-card {
  display: flex;
  flex-direction: column;
  min-height: 330px;
  padding: 1rem;
  border-radius: var(--hs-radius-card);
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
}

.plan-card--primary {
  border-color: rgba(89, 210, 194, 0.34);
  background: rgba(89, 210, 194, 0.08);
}

.plan-card--current {
  box-shadow: inset 0 0 0 1px rgba(255, 200, 87, 0.42);
}

.plan-card__head span {
  display: block;
  color: var(--hs-text-muted);
  font-size: 0.75rem;
}

.plan-card__head strong {
  display: block;
  margin-top: 0.35rem;
  font-size: 1.16rem;
}

.plan-card__audience {
  min-height: 2.6rem;
}

.plan-card ul {
  padding-left: 1.1rem;
  margin: 0.85rem 0;
  color: var(--hs-text);
  line-height: 1.65;
  font-size: 0.88rem;
}

.plan-card__note {
  flex: 1;
}

.plan-card__button {
  margin-top: 1rem;
  width: 100%;
  min-height: 38px;
  border-radius: 8px;
  border: 1px solid rgba(89, 210, 194, 0.28);
  background: rgba(89, 210, 194, 0.12);
  color: var(--hs-accent);
  cursor: pointer;
}

.plan-card__button:disabled {
  opacity: 0.62;
  cursor: wait;
}

.plan-card__button--static {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: default;
  color: var(--hs-text-muted);
  background: var(--hs-surface-muted);
  border-color: var(--hs-border);
}

.subscription-footnotes {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--hs-border);
}

.subscription-footnotes > div {
  flex: 1;
}

@media (max-width: 980px) {
  .plans-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .plans-grid,
  .subscription-footnotes {
    grid-template-columns: 1fr;
    display: grid;
  }
  .subscription-header,
  .current-card,
  .payment-card {
    flex-direction: column;
  }
  .current-card__meta {
    text-align: left;
  }
}
</style>
