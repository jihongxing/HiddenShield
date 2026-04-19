<script setup lang="ts">
import { acknowledgeTelemetry, setTelemetryEnabled } from "../lib/tauri-api";

const emit = defineEmits<{
  dismiss: [];
}>();

async function handleAccept() {
  await setTelemetryEnabled(true);
  await acknowledgeTelemetry();
  emit("dismiss");
}

async function handleDecline() {
  await setTelemetryEnabled(false);
  await acknowledgeTelemetry();
  emit("dismiss");
}
</script>

<template>
  <div class="privacy-overlay" role="dialog" aria-modal="true" aria-labelledby="privacy-title">
    <div class="privacy-dialog">
      <h2 id="privacy-title">用户协议与隐私政策</h2>
      <div class="privacy-dialog__body">
        <p>欢迎使用隐盾（HiddenShield）。在您开始使用前，请阅读以下内容：</p>
        <ul>
          <li>隐盾的所有文件处理均在本地完成，不会上传您的任何文件内容。</li>
          <li>为改善产品稳定性，隐盾会收集匿名崩溃信息（不含个人文件信息）。</li>
          <li>您可以随时在"设置"中关闭遥测数据上报。</li>
          <li>在您点击"同意"之前，隐盾不会向任何服务器发送网络请求。</li>
        </ul>
        <p class="privacy-dialog__legal">
          点击"同意并继续"即表示您已阅读并同意
          <a href="#" @click.prevent>《用户协议》</a>和<a href="#" @click.prevent>《隐私政策》</a>。
        </p>
      </div>
      <div class="privacy-dialog__actions">
        <button class="primary-button" type="button" @click="handleAccept">
          同意并继续
        </button>
        <button class="ghost-button" type="button" @click="handleDecline">
          仅使用离线功能（拒绝遥测）
        </button>
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
.privacy-dialog__legal a {
  color: #6aafff;
  text-decoration: underline;
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
</style>
