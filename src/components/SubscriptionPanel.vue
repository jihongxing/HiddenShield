<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { trackClick } from "../lib/analytics";
import { type EntitlementState } from "../lib/tauri-api";

const props = defineProps<{
  entitlementState: EntitlementState | null;
}>();

const emit = defineEmits<{
  close: [];
}>();

const currentStatus = computed(() => props.entitlementState?.status ?? "free");
let previousBodyOverflow = "";

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    close();
  }
}

onMounted(() => {
  previousBodyOverflow = document.body.style.overflow;
  document.body.style.overflow = "hidden";
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  document.body.style.overflow = previousBodyOverflow;
  window.removeEventListener("keydown", handleKeydown);
});

function formatDateTime(value: string | null): string {
  if (!value) return "—";
  return new Date(value).toLocaleString();
}

function statusLabel(status: EntitlementState["status"]): string {
  const labels: Record<EntitlementState["status"], string> = {
    free: "免费期",
    trial: "试用中",
    active: "已开通",
    grace: "宽限期",
    expired: "已过期",
  };
  return labels[status];
}

function openMail() {
  trackClick("subscription_contact_click");
  window.open(
    "mailto:jhx800@163.com?subject=隐盾订阅咨询&body=你好，我想了解 HiddenShield 的包月订阅方案。",
    "_blank",
  );
}

function close() {
  emit("close");
}
</script>

<template>
  <div
    class="subscription-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="subscription-title"
    @click.self="close"
  >
    <div class="subscription-card">
      <div class="subscription-card__hero">
        <div>
          <p class="eyebrow">Subscription</p>
          <h2 id="subscription-title">订阅方案</h2>
        </div>
        <button class="ghost-button" type="button" @click="close">关闭</button>
      </div>

      <div class="subscription-card__status" v-if="entitlementState">
        <div>
          <span class="subscription-pill">{{ statusLabel(currentStatus) }}</span>
          <strong>{{ entitlementState.planName ?? "默认免费" }}</strong>
          <p>当前权益状态：{{ statusLabel(entitlementState.status) }}</p>
        </div>
        <div class="subscription-card__meta">
          <span>更新时间</span>
          <strong>{{ formatDateTime(entitlementState.updatedAt) }}</strong>
        </div>
      </div>

      <div class="subscription-grid">
        <article class="subscription-tile subscription-tile--muted">
          <span class="subscription-tile__tag">当前</span>
          <h3>免费期</h3>
          <p>核心处理保持可用，先把真实使用量和问题摸清。</p>
        </article>
        <article class="subscription-tile subscription-tile--accent">
          <span class="subscription-tile__tag">主线</span>
          <h3>包月订阅</h3>
          <p>订阅期内无限次使用，适合高频创作者与工作室。</p>
        </article>
        <article class="subscription-tile">
          <span class="subscription-tile__tag">预留</span>
          <h3>点数包</h3>
          <p>后续作为补充方案开放，按 1 次处理消耗 1 点。</p>
        </article>
      </div>

      <div class="subscription-notes">
        <div class="subscription-note">
          <strong>现在会发生什么</strong>
          <p>当前版本继续免费开放，不会因为未付费而中断本地处理流程。</p>
        </div>
        <div class="subscription-note">
          <strong>以后会怎么走</strong>
          <p>先上线包月，再按需要开放点数包，不改变你现有的免费体验。</p>
        </div>
      </div>

      <div class="subscription-actions">
        <button class="primary-button" type="button" @click="openMail">
          联系开通
        </button>
        <button class="ghost-button" type="button" @click="close">
          稍后再说
        </button>
      </div>
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
  background: rgba(3, 8, 17, 0.78);
  backdrop-filter: blur(10px);
}

.subscription-card {
  width: min(860px, 100%);
  border-radius: 24px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background:
    radial-gradient(circle at top left, rgba(198, 91, 32, 0.18), transparent 32%),
    linear-gradient(180deg, rgba(16, 20, 31, 0.98), rgba(10, 14, 23, 0.98));
  color: #eef2f7;
  box-shadow: 0 30px 90px rgba(0, 0, 0, 0.45);
  padding: 1.5rem;
}

.subscription-card__hero,
.subscription-card__status,
.subscription-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.subscription-card__hero h2 {
  margin: 0.2rem 0 0;
  font-size: 1.5rem;
}

.subscription-card__status {
  margin-top: 1rem;
  padding: 1rem 1.1rem;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.04);
}

.subscription-card__status strong {
  display: block;
  margin: 0.2rem 0;
  font-size: 1.1rem;
}

.subscription-card__status p,
.subscription-note p,
.subscription-tile p {
  margin: 0.35rem 0 0;
  color: rgba(238, 242, 247, 0.72);
  line-height: 1.65;
  font-size: 0.88rem;
}

.subscription-card__meta {
  text-align: right;
  color: rgba(238, 242, 247, 0.8);
}

.subscription-card__meta span {
  display: block;
  font-size: 0.76rem;
  color: rgba(238, 242, 247, 0.56);
}

.subscription-pill {
  display: inline-flex;
  padding: 0.22rem 0.55rem;
  border-radius: 999px;
  background: rgba(198, 91, 32, 0.16);
  border: 1px solid rgba(198, 91, 32, 0.32);
  color: #ffcbaf;
  font-size: 0.72rem;
  margin-bottom: 0.45rem;
}

.subscription-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.85rem;
  margin-top: 1rem;
}

.subscription-tile {
  position: relative;
  padding: 1rem;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.07);
  min-height: 140px;
}

.subscription-tile--accent {
  background: linear-gradient(180deg, rgba(198, 91, 32, 0.22), rgba(198, 91, 32, 0.08));
  border-color: rgba(198, 91, 32, 0.28);
}

.subscription-tile--muted {
  background: linear-gradient(180deg, rgba(40, 46, 60, 0.92), rgba(22, 26, 37, 0.98));
}

.subscription-tile__tag {
  display: inline-flex;
  padding: 0.2rem 0.45rem;
  border-radius: 999px;
  font-size: 0.7rem;
  color: rgba(238, 242, 247, 0.7);
  background: rgba(255, 255, 255, 0.08);
}

.subscription-tile h3 {
  margin: 0.7rem 0 0;
  font-size: 1rem;
}

.subscription-notes {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.85rem;
  margin-top: 1rem;
}

.subscription-note {
  padding: 0.95rem 1rem;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.subscription-note strong {
  display: block;
  font-size: 0.88rem;
}

.subscription-actions {
  margin-top: 1.2rem;
  justify-content: flex-end;
}

.primary-button,
.ghost-button {
  padding: 0.75rem 1rem;
  border-radius: 999px;
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.9rem;
}

.primary-button {
  background: linear-gradient(135deg, #c65b20, #f29b47);
  color: #fff;
}

.ghost-button {
  background: transparent;
  border-color: rgba(255, 255, 255, 0.12);
  color: #eef2f7;
}

@media (max-width: 760px) {
  .subscription-grid,
  .subscription-notes {
    grid-template-columns: 1fr;
  }
  .subscription-card__hero,
  .subscription-card__status,
  .subscription-actions {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
