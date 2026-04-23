<script setup lang="ts">
import { computed, ref } from "vue";
import { consentHighlights, legalDocuments, type LegalDocKey } from "../content/legal";
import { acknowledgeTelemetry, setNetworkEnabled, setTelemetryEnabled } from "../lib/tauri-api";

const emit = defineEmits<{
  dismiss: [];
}>();

const activeDoc = ref<LegalDocKey | null>(null);
const activeDocument = computed(() => (activeDoc.value ? legalDocuments[activeDoc.value] : null));

async function handleAccept() {
  await setNetworkEnabled(true);
  await setTelemetryEnabled(true);
  await acknowledgeTelemetry();
  emit("dismiss");
}

async function handleDecline() {
  await setNetworkEnabled(false);
  await setTelemetryEnabled(false);
  await acknowledgeTelemetry();
  emit("dismiss");
}

function openDoc(doc: LegalDocKey) {
  activeDoc.value = doc;
}

function closeDoc() {
  activeDoc.value = null;
}
</script>

<template>
  <div class="privacy-overlay" role="dialog" aria-modal="true" aria-labelledby="privacy-title">
    <div class="privacy-dialog">
      <h2 id="privacy-title">使用前确认</h2>
      <div class="privacy-dialog__body">
        <ul>
          <li v-for="item in consentHighlights" :key="item">{{ item }}</li>
        </ul>
        <p class="privacy-dialog__legal">
          点击“同意并继续”即表示您已阅读并同意
          <button class="legal-link" type="button" @click="openDoc('terms')">《用户协议》</button>
          和
          <button class="legal-link" type="button" @click="openDoc('privacy')">《隐私政策》</button>。
        </p>
      </div>
      <div class="privacy-dialog__actions">
        <button class="primary-button" type="button" @click="handleAccept">
          同意并继续
        </button>
        <button class="ghost-button" type="button" @click="handleDecline">
          仅离线使用
        </button>
      </div>

      <div v-if="activeDoc" class="legal-sheet" role="document" aria-live="polite">
        <div class="legal-sheet__header">
          <h3>{{ activeDocument?.title }}</h3>
          <button class="legal-close" type="button" @click="closeDoc">关闭</button>
        </div>

        <div v-if="activeDocument" class="legal-sheet__content">
          <p>生效日期：{{ activeDocument.effectiveDate }}</p>
          <p>{{ activeDocument.intro }}</p>
          <section
            v-for="section in activeDocument.sections"
            :key="section.heading"
            class="legal-section"
          >
            <h4>{{ section.heading }}</h4>
            <p v-for="paragraph in section.paragraphs" :key="paragraph">{{ paragraph }}</p>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.privacy-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(4px);
}
.privacy-dialog {
  max-width: 520px;
  width: 90%;
  background: #1a1f2e;
  border: 1px solid #2a3a4a;
  border-radius: 12px;
  padding: 2rem;
  color: #e0e8f0;
}
.privacy-dialog h2 {
  margin: 0 0 1rem;
  font-size: 1.3rem;
  color: #fff;
}
.privacy-dialog__body {
  font-size: 0.9rem;
  line-height: 1.6;
  color: #b0c0d0;
}
.privacy-dialog__body ul {
  padding-left: 1.2rem;
  margin: 0.8rem 0;
}
.privacy-dialog__body li {
  margin-bottom: 0.4rem;
}
.privacy-dialog__legal {
  margin-top: 1rem;
  font-size: 0.8rem;
  color: #8090a0;
}
.legal-link {
  padding: 0;
  border: none;
  background: transparent;
  color: #6aafff;
  text-decoration: underline;
  cursor: pointer;
}
.privacy-dialog__actions {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin-top: 1.5rem;
}
.privacy-dialog__actions .primary-button {
  width: 100%;
  padding: 0.7rem;
  border-radius: 8px;
  border: none;
  background: #3a7af0;
  color: #fff;
  font-size: 0.95rem;
  cursor: pointer;
}
.privacy-dialog__actions .ghost-button {
  width: 100%;
  padding: 0.6rem;
  border-radius: 8px;
  border: 1px solid #3a5a7c;
  background: transparent;
  color: #8ab4e0;
  font-size: 0.85rem;
  cursor: pointer;
}
.legal-sheet {
  margin-top: 1rem;
  border-top: 1px solid #2a3a4a;
  padding-top: 1rem;
}
.legal-sheet__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
.legal-sheet__header h3 {
  margin: 0;
  font-size: 1rem;
  color: #fff;
}
.legal-close {
  border: 1px solid #3a5a7c;
  background: transparent;
  color: #8ab4e0;
  border-radius: 6px;
  padding: 0.3rem 0.75rem;
  cursor: pointer;
}
.legal-sheet__content {
  margin-top: 0.75rem;
  max-height: 220px;
  overflow: auto;
  font-size: 0.82rem;
  line-height: 1.65;
  color: #b0c0d0;
}
.legal-sheet__content p {
  margin: 0 0 0.75rem;
}
.legal-section + .legal-section {
  margin-top: 1rem;
}
.legal-section h4 {
  margin: 0 0 0.5rem;
  font-size: 0.88rem;
  color: #e7eef7;
}
</style>
