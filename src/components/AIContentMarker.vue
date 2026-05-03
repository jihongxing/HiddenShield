<script setup lang="ts">
import { ref } from 'vue';

const isAIGenerated = ref(false);
const generationMethod = ref('text_to_image');
const modificationLevel = ref('pure_ai');
const authenticityClaim = ref('unspecified');
const trainingPermission = ref('prohibited');

defineExpose({
  isAIGenerated,
  generationMethod,
  modificationLevel,
  authenticityClaim,
  trainingPermission,
  customMetadata: ref(''),
});
</script>

<template>
  <div class="ai-toggle">
    <label class="checkbox-label">
      <input type="checkbox" v-model="isAIGenerated" />
      <span>标记为AI生成内容</span>
    </label>
  </div>

  <div v-if="isAIGenerated" class="ai-marker-compact">
    <div class="ai-marker-grid">
      <label class="field-compact">
        <span>生成方式</span>
        <select v-model="generationMethod">
          <option value="text_to_image">文本生成图像</option>
          <option value="image_to_image">图像转换</option>
          <option value="text_to_video">文本生成视频</option>
          <option value="video_to_video">视频转换</option>
          <option value="audio_generation">音频生成</option>
          <option value="multimodal">多模态生成</option>
          <option value="other_ai">其他AI方式</option>
        </select>
      </label>

      <label class="field-compact">
        <span>人工修改</span>
        <select v-model="modificationLevel">
          <option value="pure_ai">纯AI</option>
          <option value="light">轻度</option>
          <option value="moderate">中度</option>
          <option value="heavy">重度</option>
        </select>
      </label>

      <label class="field-compact">
        <span>内容真实性</span>
        <select v-model="authenticityClaim">
          <option value="unspecified">未声明</option>
          <option value="synthetic">虚构/合成</option>
          <option value="based_on_reality">基于真实</option>
          <option value="authentic">完全真实</option>
        </select>
      </label>

      <label class="field-compact">
        <span>训练许可</span>
        <select v-model="trainingPermission">
          <option value="prohibited">🚫 禁止</option>
          <option value="non_commercial">🎓 非商业</option>
          <option value="commercial">💼 商业</option>
          <option value="public_domain">🌍 公共领域</option>
        </select>
      </label>
    </div>
  </div>
</template>

<style scoped>
.ai-toggle {
  margin-top: 12px;
  padding: 10px 12px;
  background: #f9fafb;
  border-radius: 6px;
  border: 1px solid #e5e7eb;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 14px;
  color: #374151;
}

.checkbox-label input[type="checkbox"] {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.ai-marker-compact {
  margin-top: 12px;
  padding: 12px;
  background: #eff6ff;
  border-radius: 6px;
  border: 1px solid #bfdbfe;
}

.ai-marker-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.field-compact {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-compact span {
  font-size: 12px;
  font-weight: 500;
  color: #6b7280;
}

.field-compact select {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  font-size: 13px;
  background: white;
}

.field-compact select:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.1);
}
</style>
