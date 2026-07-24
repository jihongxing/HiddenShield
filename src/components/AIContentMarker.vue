<script setup lang="ts">
import { ref } from 'vue';

const workSourceDeclaration = ref('unspecified');
const creationMethodDeclaration = ref('unspecified');
const humanEditLevelDeclaration = ref('unspecified');
const authenticityClaim = ref('unspecified');
const trainingPermission = ref('prohibited');
const customRightsStatement = ref('');

defineExpose({
  workSourceDeclaration,
  creationMethodDeclaration,
  humanEditLevelDeclaration,
  authenticityClaimDeclaration: authenticityClaim,
  trainingPermissionDeclaration: trainingPermission,
  customRightsStatement,
});
</script>

<template>
  <details class="ai-toggle" open>
    <summary>作品声明与授权策略</summary>
    <p>HiddenShield 不判断作品是否由 AI 生成，只记录创作者对作品来源、训练许可和授权边界的声明，并写入版权记录与报告摘要。</p>
    <div class="ai-marker-grid">
      <label class="field-compact">
        <span>作品来源声明</span>
        <select v-model="workSourceDeclaration">
          <option value="unspecified">未声明</option>
          <option value="human_created">人工创作</option>
          <option value="ai_assisted">AI 辅助</option>
          <option value="ai_generated">AI 生成</option>
        </select>
      </label>

      <label class="field-compact">
        <span>创作方式声明</span>
        <select v-model="creationMethodDeclaration">
          <option value="unspecified">未声明</option>
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
        <span>人工编辑声明</span>
        <select v-model="humanEditLevelDeclaration">
          <option value="unspecified">未声明</option>
          <option value="pure_ai">几乎无人工编辑</option>
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
          <option value="creator_claimed_authentic">创作者声明真实</option>
        </select>
      </label>

      <label class="field-compact">
        <span>训练许可</span>
        <select v-model="trainingPermission">
          <option value="prohibited">禁止模型训练</option>
          <option value="separate_authorization_required">需单独授权</option>
          <option value="non_commercial_allowed">允许非商业训练</option>
          <option value="commercial_allowed">允许商业训练</option>
        </select>
      </label>

      <label class="field-compact field-compact--wide">
        <span>自定义版权声明</span>
        <textarea v-model="customRightsStatement" rows="2" placeholder="可选：补充授权边界、署名要求或合同编号"></textarea>
      </label>
    </div>
  </details>
</template>

<style scoped>
.ai-toggle {
  margin-top: 12px;
  padding: 12px;
  background: var(--hs-surface-raised);
  border-radius: var(--hs-radius-card);
  border: 1px solid var(--hs-border);
}

.ai-toggle summary {
  cursor: pointer;
  font-size: 14px;
  font-weight: 700;
  color: var(--hs-text);
}

.ai-toggle p {
  margin-top: 12px;
  color: var(--hs-text-muted);
  font-size: 13px;
  line-height: 1.5;
}

.ai-marker-grid {
  margin-top: 12px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.field-compact--wide {
  grid-column: 1 / -1;
}

.field-compact {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-compact span {
  font-size: 12px;
  font-weight: 500;
  color: var(--hs-text-muted);
}

.field-compact select,
.field-compact textarea {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  font-size: 13px;
  background: var(--hs-surface-muted);
  color: var(--hs-text);
  resize: vertical;
}

.field-compact select:focus,
.field-compact textarea:focus {
  outline: none;
  border-color: var(--hs-accent);
  box-shadow: 0 0 0 3px rgba(114, 214, 202, 0.14);
}
</style>
