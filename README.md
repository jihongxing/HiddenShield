# 隐盾 HiddenShield

面向创作者的本地优先版权保护工具，围绕图片、音频、视频音轨水印和视频指纹存证建立可验证版权记录。

## 核心能力

- **DWT-DCT-SVD 图片盲水印**：抗 JPEG 压缩、二次保存、轻度缩放与轻裁剪，支持写入后验证
- **QIM 频域音频盲水印**：面向 30 秒以上音频作品，抗重编码、格式转换
- **视频音轨盲水印**：复用音频盲水印保护视频音轨，不等同于视频画面盲水印
- **L2 视频指纹存证**：生成不可逆画面指纹摘要和存证记录，用于相似性证据增强
- **RFC 3161 / 网络授时证据**：配置可用 TSA 时记录第三方时间戳；未配置或不可用时仅保留辅助时间材料，不承诺生产可信时间戳链路
- **本地版权库**：默认不上传原始媒体、保护副本或本地路径
- **Creator 云同步**：启用后只同步账户、创作者身份、权益和版权记录白名单元数据
- **正式报告**：Creator 订阅内可导出；Free 可按记录购买单份版权详细报告或维权证据包，真实付款需配置微信支付通道

## 参考文档

- [watermark-core 能力说明](docs/watermark-core能力说明.md)
- [当前真实能力边界说明](docs/当前真实能力边界说明.md)
- [公开权利信号与训练许可扫描剩余任务清单](docs/公开权利信号与训练许可扫描剩余任务清单.md)
- `watermark-core` 改动硬约束：凡是涉及 `watermark-core` 的代码、公开 API、payload、fixture、benchmark、gate 或算法行为变更，必须同步更新 [watermark-core 能力说明](docs/watermark-core能力说明.md)。

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 + TypeScript + Vite |
| 桌面框架 | Tauri 2 (Rust) |
| 视频处理 | FFmpeg (自动检测/下载) |
| 图片水印 | DWT-DCT-SVD (纯 Rust, nalgebra) |
| 音频水印 | QIM 频域 (纯 Rust, realfft) |
| 数据库 | SQLite (rusqlite) |
| 时间戳 | RFC 3161 TSA / 网络授时 best-effort（生产 TSA 需单独配置和验收） |

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

### 音频版权保护边界

HiddenShield 的音频盲水印面向完整作品或可独立追索的音频内容。桌面端和移动端默认只为 **30 秒及以上** 的音频生成版权保护副本；短于 30 秒的片段暂不作为产品承诺范围。

取证时，较短片段仍可能命中已写入的水印，但这属于辅助检测结果，不代表任意 5 秒或 10 秒片段都可稳定恢复完整版权信息。

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
  tsa.rs                # TSA / 网络授时证据客户端
docs/                   # 技术文档
AGENTS.md               # 项目级 Agent 工作约束
```

## 构建发布

```bash
npx tauri build
```

输出位于 `src-tauri/target/release/bundle/`。

## 致谢

- [guofei9987/blind_watermark](https://github.com/guofei9987/blind_watermark) — MIT License，图片盲水印 DWT-DCT-SVD 算法参考。本项目参考其周期铺写、聚合投票、双奇异值冗余和裁剪恢复思路，并基于 HiddenShield 的版权取证场景用纯 Rust 独立实现；同时增强了重复写入防覆盖、写入后验证、sync packet 备份层、密集恢复层和版权库记录能力。
- [guofei9987/signal-transforms](https://github.com/guofei9987/signal-transforms) — MIT License，Rust DCT / 变换库参考。本项目未直接复用其产品逻辑，水印写入、提取、回归验证和跨端版权流程均为独立实现。

当前图片水印已经覆盖 JPEG、二次保存、椒盐噪点、局部遮盖、轻裁剪，以及 90/180/270 度旋转和水平/垂直镜像等常见方向变化；任意角度旋转仍不作为默认产品承诺。

## 许可证

本项目采用 [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) 许可协议。

- ✅ 允许：学习、研究、个人非商业使用、二次开发（需注明出处）
- ❌ 禁止：任何直接或间接的商业用途（包括但不限于销售、SaaS 服务、嵌入商业产品）
- 📧 商用授权：如需商业使用，请联系作者获取商业许可
