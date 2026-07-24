<script setup lang="ts">
import { ref } from "vue";

const copyMsg = ref("");

async function copyWechat() {
  await navigator.clipboard.writeText("Zoro998877");
  copyMsg.value = "微信号已复制";
  setTimeout(() => { copyMsg.value = ""; }, 3000);
}

const faqs = [
  {
    q: "第一次使用需要做什么？",
    a: "登录账户、填写创作者身份，并确认默认输出位置。账户用于同步版权库和权益，创作者身份用于版权记录展示，默认输出位置用于保存保护副本。",
  },
  {
    q: "图片写入后的保护副本为什么默认是 PNG？",
    a: "PNG 更适合作为保护副本，能减少再次压缩前的细节损失，提高后续验证成功率。原图不会被覆盖。",
  },
  {
    q: "图片被裁切、旋转或压缩后还能验证吗？",
    a: "桌面版支持轴对齐、宽高各为原图 1/4 的裁切区域，以及 90/180/270 度旋转、85% 缩放、JPEG/WebP quality 75/60 的独立恢复。多个扰动叠加、任意角度旋转、更低质量压缩或更大比例缩小不在当前承诺内。",
  },
  {
    q: "音频为什么要求 30 秒以上？",
    a: "短片段可承载的信息太少，容易让版权保护和验证结果变得不稳定。隐盾只对 30 秒以上、可确认时长的音频生成保护副本。",
  },
  {
    q: "验证时提示未找到版权记录怎么办？",
    a: "可能是样本没有经过隐盾保护、文件被严重裁剪或压缩，或当前设备版权库没有对应记录。登录同一账户并开启云同步后，可以先更新云端记录再验证。",
  },
  {
    q: "云同步会上传我的原始文件吗？",
    a: "默认不会。云同步只同步账户、权益、创作者档案、版权记录元数据、验证记录和同步状态摘要，不同步原始媒体、保护副本或本地文件路径。",
  },
  {
    q: "当前版本支持哪些媒体？",
    a: "当前桌面版本只开放图片和音频；全部视频能力与移动端开发均已暂停，不属于当前发布范围。",
  },
  {
    q: "未付费和年度授权有什么区别？",
    a: "未付费用户可使用单文件图片、音频处理、验证和本地版权库；图片 / 音频年费用户可使用批量处理。正式报告始终按记录单独购买，未来视频服务独立收费。",
  },
  {
    q: "正式报告可以直接作为法律意见吗？",
    a: "不可以。正式报告、时间回执和指纹存证是技术辅助材料，不构成法律意见、司法鉴定意见或诉讼结果承诺。",
  },
  {
    q: "遇到问题怎么反馈？",
    a: "在设置中发送匿名反馈，或通过本页联系方式联系作者。匿名反馈不包含原始媒体、保护副本、文件名、本地路径或完整作品指纹。",
  },
];

const expandedIndex = ref<number | null>(null);

function toggleFaq(index: number) {
  expandedIndex.value = expandedIndex.value === index ? null : index;
}
</script>

<template>
  <div class="help">
    <!-- About -->
    <section class="help__section">
      <h3>关于隐盾</h3>
      <p>HiddenShield 是一款本地优先的版权保护工具。当前为桌面端图片和音频工作流生成保护副本、版权记录和验证摘要。</p>
      <div class="help__features">
        <span>图片写入</span>
        <span>音频写入</span>
        <span>版权库</span>
        <span>验证摘要</span>
        <span>本地优先</span>
      </div>
    </section>

    <!-- Quick Start -->
    <section class="help__section">
      <h3>快速上手</h3>
      <div class="help__steps">
        <div class="help__step">
          <span class="help__step-num">1</span>
          <div>
            <strong>导入文件</strong>
            <p>在工作台选择图片或音频文件。</p>
          </div>
        </div>
        <div class="help__step">
          <span class="help__step-num">2</span>
          <div>
            <strong>生成保护副本</strong>
            <p>确认设置后开始处理，完成前会自动验证保护副本。</p>
          </div>
        </div>
        <div class="help__step">
          <span class="help__step-num">3</span>
          <div>
            <strong>验证与留档</strong>
            <p>在验证页导入样本，匹配版权库并生成可复制的验证摘要。</p>
          </div>
        </div>
      </div>
    </section>

    <!-- FAQ -->
    <section class="help__section">
      <h3>常见问题</h3>
      <div class="help__faq-list">
        <div
          v-for="(faq, i) in faqs"
          :key="i"
          class="help__faq"
          :class="{ 'help__faq--open': expandedIndex === i }"
        >
          <button class="help__faq-q" type="button" @click="toggleFaq(i)">
            <span>{{ faq.q }}</span>
            <span class="help__faq-arrow">{{ expandedIndex === i ? '−' : '+' }}</span>
          </button>
          <div v-if="expandedIndex === i" class="help__faq-a">
            <p>{{ faq.a }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- Contact -->
    <section class="help__section help__contact">
      <h3>仍有问题？</h3>
      <p>以上没有解决你的问题，可以直接联系作者：</p>
      <div class="help__contact-items">
        <div class="help__contact-item">
          <span>微信：Zoro998877</span>
          <button class="help__contact-btn" type="button" @click="copyWechat">复制</button>
        </div>
        <div class="help__contact-item">
          <span>邮箱：jhx800@163.com</span>
          <a class="help__contact-btn" href="mailto:jhx800@163.com?subject=隐盾问题反馈">发邮件</a>
        </div>
      </div>
      <p v-if="copyMsg" class="help__toast">{{ copyMsg }}</p>
    </section>
  </div>
</template>

<style scoped>
.help {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.help__section {
  padding: 20px 24px;
  background: var(--hs-surface);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
}

.help__section h3 {
  margin: 0 0 12px;
  font-size: 15px;
  color: var(--hs-accent);
}

.help__section p {
  margin: 0;
  font-size: 13px;
  line-height: 1.7;
  color: var(--hs-text-muted);
}

.help__features {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.help__features span {
  padding: 4px 10px;
  font-size: 12px;
  background: var(--hs-chip);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-pill);
  color: var(--hs-text-muted);
}

/* Steps */
.help__steps {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.help__step {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}

.help__step-num {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 700;
  color: #061312;
  background: var(--hs-accent);
  border-radius: var(--hs-radius-card);
}

.help__step strong {
  display: block;
  font-size: 13px;
  margin-bottom: 2px;
}

.help__step p {
  font-size: 12px;
}

/* FAQ */
.help__faq-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.help__faq {
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  overflow: hidden;
  transition: border-color 0.2s;
}

.help__faq--open {
  border-color: rgba(114, 214, 202, 0.28);
}

.help__faq-q {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 14px;
  font-size: 13px;
  font-weight: 500;
  text-align: left;
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
}

.help__faq-q:hover {
  background: var(--hs-surface-muted);
}

.help__faq-arrow {
  font-size: 16px;
  color: var(--hs-text-subtle);
}

.help__faq-a {
  padding: 0 14px 12px;
}

.help__faq-a p {
  font-size: 12px;
  line-height: 1.7;
}

/* Contact */
.help__contact-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
}

.help__contact-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: var(--hs-surface-raised);
  border: 1px solid var(--hs-border);
  border-radius: var(--hs-radius-card);
  font-size: 13px;
}

.help__contact-btn {
  padding: 4px 12px;
  font-size: 12px;
  font-weight: 500;
  color: #061312;
  background: var(--hs-accent);
  border: none;
  border-radius: var(--hs-radius-card);
  cursor: pointer;
  text-decoration: none;
  transition: opacity 0.2s;
}

.help__contact-btn:hover {
  opacity: 0.85;
}

.help__toast {
  margin-top: 8px;
  font-size: 12px;
  color: var(--hs-accent);
}
</style>
