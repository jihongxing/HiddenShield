<script setup lang="ts">
import { ref } from "vue";
import { setupIdentity } from "../lib/tauri-api";

const emit = defineEmits<{ complete: [] }>();

const creatorInput = ref("");
const loading = ref(false);
const errorMsg = ref("");

async function handleSubmit() {
  if (!creatorInput.value.trim()) {
    errorMsg.value = "请输入您的创作者标识";
    return;
  }
  loading.value = true;
  errorMsg.value = "";
  try {
    await setupIdentity(creatorInput.value.trim());
    emit("complete");
  } catch (err: any) {
    errorMsg.value = err?.message ?? String(err);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="id-setup">
    <div class="id-setup__backdrop" />
    <div class="id-setup__card">
      <div class="id-setup__shield">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M12 2L3 7v5c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-9-5z" fill="url(#shield-grad)" opacity="0.9"/>
          <path d="M10 15.5l-3.5-3.5 1.41-1.41L10 12.67l5.59-5.59L17 8.5l-7 7z" fill="#fff"/>
          <defs>
            <linearGradient id="shield-grad" x1="3" y1="2" x2="21" y2="19">
              <stop offset="0%" stop-color="#c65b20"/>
              <stop offset="100%" stop-color="#1b365f"/>
            </linearGradient>
          </defs>
        </svg>
      </div>

      <h1 class="id-setup__title">激活版权保护</h1>
      <p class="id-setup__subtitle">设定您的专属创作者身份</p>

      <div class="id-setup__info">
        <p>输入一个您能记住的标识（笔名、手机号后四位、工作室名等），系统将不可逆加密后作为版权基因打入每一个作品。</p>
      </div>

      <div class="id-setup__input-wrap">
        <input
          v-model="creatorInput"
          type="text"
          class="id-setup__input"
          placeholder="输入创作者标识..."
          :disabled="loading"
          @keydown.enter="handleSubmit"
        />
      </div>

      <p v-if="errorMsg" class="id-setup__error">{{ errorMsg }}</p>

      <button
        class="id-setup__btn"
        type="button"
        :disabled="loading || !creatorInput.trim()"
        @click="handleSubmit"
      >
        <span v-if="loading" class="id-setup__spinner" />
        {{ loading ? "生成中..." : "确认激活" }}
      </button>

      <div class="id-setup__footer">
        <span>🔒</span>
        <span>全本地加密存储 · 不上传云端 · 请妥善记忆</span>
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
  background: linear-gradient(135deg, #0f1822 0%, #1b365f 50%, #0f1822 100%);
  opacity: 0.97;
}

.id-setup__card {
  position: relative;
  width: 100%;
  max-width: 420px;
  padding: 48px 40px 36px;
  background: rgba(255, 255, 255, 0.06);
  backdrop-filter: blur(24px);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 20px;
  text-align: center;
  box-shadow: 0 32px 80px rgba(0, 0, 0, 0.4);
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
  color: #fff;
  letter-spacing: -0.3px;
}

.id-setup__subtitle {
  margin: 0 0 24px;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.6);
}

.id-setup__info {
  margin-bottom: 24px;
  padding: 12px 16px;
  background: rgba(198, 91, 32, 0.1);
  border: 1px solid rgba(198, 91, 32, 0.2);
  border-radius: 10px;
  text-align: left;
}

.id-setup__info p {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: rgba(255, 255, 255, 0.8);
}

.id-setup__input-wrap {
  margin-bottom: 16px;
}

.id-setup__input {
  width: 100%;
  padding: 14px 18px;
  font-size: 16px;
  color: #fff;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 12px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.id-setup__input::placeholder {
  color: rgba(255, 255, 255, 0.35);
}

.id-setup__input:focus {
  border-color: #c65b20;
  box-shadow: 0 0 0 3px rgba(198, 91, 32, 0.2);
}

.id-setup__error {
  margin: 0 0 12px;
  font-size: 13px;
  color: #ff6b6b;
}

.id-setup__btn {
  width: 100%;
  padding: 14px;
  font-size: 15px;
  font-weight: 600;
  color: #fff;
  background: linear-gradient(135deg, #c65b20, #e07030);
  border: none;
  border-radius: 12px;
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

.id-setup__spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
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
  color: rgba(255, 255, 255, 0.4);
}
</style>
