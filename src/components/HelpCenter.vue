<script setup lang="ts">
import { ref } from "vue";

const copyMsg = ref("");

async function copyWechat() {
  await navigator.clipboard.writeText("Zoro998877");
  copyMsg.value = "微信号已复制，快去添加吧 👋";
  setTimeout(() => { copyMsg.value = ""; }, 3000);
}

const faqs = [
  {
    q: "FFmpeg 检测失败怎么办？",
    a: "隐盾会自动检测系统 PATH 中的 FFmpeg，如果未找到会尝试自动下载。如果下载失败，请手动安装 FFmpeg 并确保 ffmpeg 和 ffprobe 在系统 PATH 中。",
  },
  {
    q: "图片水印嵌入后格式变成了 PNG？",
    a: "这是设计如此。DWT-DCT-SVD 盲水印需要无损格式保存以确保最高提取成功率。PNG 输出把全部容错预算留给了后续传播中的压缩（微信、微博等平台会再压一次）。",
  },
  {
    q: "维权取证时提示「未检测到有效水印」？",
    a: "可能原因：1) 文件不是由隐盾处理过的；2) 文件经过了极端压缩或裁剪；3) 使用的是旧版本嵌入的水印（载荷结构不兼容）。建议用新版本重新处理原文件。",
  },
  {
    q: "视频压制时提示 FFmpeg 超时？",
    a: "大文件或复杂编码（如 HDR 4K）在初始化阶段可能需要较长时间。隐盾已设置 90 秒冷启动超时。如果仍然超时，尝试切换为「高质量 CPU」编码模式。",
  },
  {
    q: "可信时间戳获取失败？",
    a: "时间戳依赖网络连接到第三方 TSA 服务器。如果网络受限（公司防火墙/代理），时间戳会静默跳过，不影响水印嵌入。存证报告中会标注是否获取成功。",
  },
  {
    q: "换了电脑后水印 UID 变了？",
    a: "水印 UID 由「创作者标识 + 设备指纹」生成。换电脑后设备指纹变化，UID 会不同，但只要输入相同的创作者标识，仍可关联到同一创作者。",
  },
  {
    q: "免费版和 Pro 版有什么区别？",
    a: "免费版每次仅支持输出 1 个平台。Pro 版支持多平台并行、PDF 报告导出、批量处理、区块链存证等高级功能（即将上线）。",
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
      <p>隐盾 (HiddenShield) 是一款本地优先的版权保护工具。对视频、图片、音频无感嵌入盲水印，生成带有第三方可信时间戳的版权存证，在需要维权时一键提取水印并生成取证报告。</p>
      <div class="help__features">
        <span>🎬 多平台视频压制</span>
        <span>🖼️ DWT-DCT-SVD 图片盲水印</span>
        <span>🎵 QIM 频域音频盲水印</span>
        <span>🔐 RFC 3161 可信时间戳</span>
        <span>📋 一键生成存证报告</span>
        <span>🔒 全本地处理，零上传</span>
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
            <p>在工作台拖入或点击选择视频/图片/音频文件</p>
          </div>
        </div>
        <div class="help__step">
          <span class="help__step-num">2</span>
          <div>
            <strong>处理 & 存证</strong>
            <p>选择目标平台（视频）后点击开始，系统自动嵌入水印、压制输出、获取可信时间戳并存入版权库</p>
          </div>
        </div>
        <div class="help__step">
          <span class="help__step-num">3</span>
          <div>
            <strong>维权取证</strong>
            <p>在取证页面拖入疑似侵权文件，自动提取水印并匹配版权库，一键复制存证报告</p>
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
      <p>以上没有解决你的问题，请直接联系作者：</p>
      <div class="help__contact-items">
        <div class="help__contact-item">
          <span>💬 微信：Zoro998877</span>
          <button class="help__contact-btn" type="button" @click="copyWechat">复制</button>
        </div>
        <div class="help__contact-item">
          <span>📧 邮箱：jhx800@163.com</span>
          <a class="help__contact-btn" href="mailto:jhx800@163.com?subject=隐盾用户反馈">发邮件</a>
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
  background: var(--panel, rgba(255,255,255,0.74));
  border: 1px solid var(--line, rgba(15,24,34,0.1));
  border-radius: 16px;
}

.help__section h3 {
  margin: 0 0 12px;
  font-size: 15px;
  color: var(--brand, #c65b20);
}

.help__section p {
  margin: 0;
  font-size: 13px;
  line-height: 1.7;
  color: rgba(15, 24, 34, 0.72);
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
  background: rgba(198, 91, 32, 0.08);
  border: 1px solid rgba(198, 91, 32, 0.15);
  border-radius: 6px;
  color: var(--brand-ink, #672e11);
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
  color: #fff;
  background: var(--brand, #c65b20);
  border-radius: 50%;
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
  border: 1px solid var(--line, rgba(15,24,34,0.08));
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.2s;
}

.help__faq--open {
  border-color: rgba(198, 91, 32, 0.3);
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
  background: rgba(198, 91, 32, 0.04);
}

.help__faq-arrow {
  font-size: 16px;
  color: rgba(15, 24, 34, 0.4);
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
  background: rgba(15, 24, 34, 0.03);
  border-radius: 10px;
  font-size: 13px;
}

.help__contact-btn {
  padding: 4px 12px;
  font-size: 12px;
  font-weight: 500;
  color: #fff;
  background: var(--brand, #c65b20);
  border: none;
  border-radius: 6px;
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
  color: var(--teal, #0f6e66);
}
</style>
