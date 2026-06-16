# 隐盾 HiddenShield

版权界的剪映 — 把多平台极致压制做成高频入口，把盲水印和版权资产沉到后端能力里。

## 核心能力

- **多平台视频压制**：一键输出抖音/B站/小红书最优规格（分辨率、帧率、码率）
- **DWT-DCT-SVD 图片盲水印**：抗 JPEG 压缩、缩放、裁剪、截图
- **QIM 频域音频盲水印**：抗重编码、格式转换
- **视频音轨盲水印**：无感嵌入版权基因
- **RFC 3161 可信时间戳**：第三方权威机构签发，防伪造时间
- **本地版权金库**：SQLite 存证，零上传
- **账户云同步契约**：本地 mock 服务验证登录注册合一、版权元数据 push / pull
- **维权取证报告**：一键生成结构化存证报告

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 + TypeScript + Vite |
| 桌面框架 | Tauri 2 (Rust) |
| 视频处理 | FFmpeg (自动检测/下载) |
| 图片水印 | DWT-DCT-SVD (纯 Rust, nalgebra) |
| 音频水印 | QIM 频域 (纯 Rust, realfft) |
| 数据库 | SQLite (rusqlite) |
| 时间戳 | RFC 3161 TSA (FreeTSA/DigiCert/Sectigo) |

## 快速开始

```bash
# 安装前端依赖
npm install

# 启动开发模式 (Vite + Tauri)
npx tauri dev
```

### 云同步后端

本地开发可先启动最小真实云后端，再让 Flutter / 桌面端对接同一套协议：

```bash
npm run cloud:backend
npm run cloud:contract
npm run cloud:e2e
```

默认地址来自系统配置 [config/hiddenshield.system.json](D:/codeSpace/HiddenShield/config/hiddenshield.system.json)，当前为 `http://127.0.0.1:43188`。Flutter 和桌面端都从各自的系统配置入口读取该地址，不在用户设置页暴露手动填写项。

桌面端 Tauri 已接入同协议云同步 client；设置页已提供“账户与云同步”主入口，并把原 LAN 配对码服务降级到高级调试区。

### 联调预览

同时启动云后端、桌面端和 Flutter 预览：

```bash
npm run dev:stack
```

这会默认拉起：

- 云后端 `http://127.0.0.1:43188`
- 桌面端 `npm run tauri:dev`
- Flutter Web 预览 `http://127.0.0.1:43189`

桌面端正式预览请始终走 Tauri，不要用 `vite preview` 代替。

详细流程见 [桌面 / 移动 / 云同步联调指南](docs/桌面移动云同步联调指南.md)。

### 前提条件

- Node.js 18+
- Rust toolchain (rustup)
- Windows: WebView2 运行时 (Win10/11 自带)

## 项目结构

```
src/                    # Vue 前端
src-tauri/src/          # Rust 后端
  commands/             # Tauri IPC 命令
  pipeline/             # 水印嵌入/提取/FFmpeg 调度
  encoder/              # 硬件编码检测/预设
  db/                   # SQLite schema/queries
  sync/                 # 移动端桌面同步 HTTP stub
  identity.rs           # 创作者身份管理
  tsa.rs                # RFC 3161 可信时间戳
docs/                   # 技术文档
AGENTS.md               # 项目级 Agent 工作约束
```

## 构建发布

```bash
npx tauri build
```

输出位于 `src-tauri/target/release/bundle/`。

## 致谢

- [guofei9987/blind_watermark](https://github.com/guofei9987/blind_watermark) — 图片盲水印 DWT-DCT-SVD 算法参考，本项目基于其核心思路用纯 Rust 重新实现
- [guofei9987/signal-transforms](https://github.com/guofei9987/signal-transforms) — Rust DCT 变换库参考

## 许可证

本项目采用 [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) 许可协议。

- ✅ 允许：学习、研究、个人非商业使用、二次开发（需注明出处）
- ❌ 禁止：任何直接或间接的商业用途（包括但不限于销售、SaaS 服务、嵌入商业产品）
- 📧 商用授权：如需商业使用，请联系作者获取商业许可
