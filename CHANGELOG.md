# Changelog

## [封版候选] - 2026-06-25

### 当前可发布范围

- 固定发布边界：图片 / 音频同核盲水印写入与验证、移动端保护副本出口、版权库、正式报告、正式云同步、本地批量、L1 视频音轨水印和 L2 视频指纹存证。
- 修复安装版本地优先阻断：云后端未配置 / 未启动导致云同步 503 时，用户仍可跳过云同步完成本地创作者身份和输出目录设置；本地图片 / 音频写入、回读验证和版权库入库不依赖云账号。
- 修复图片写入前版权记录检查阻塞主流程：大图预检超过 5 秒会返回可继续写入状态；用户点击“生成保护副本”时若预检仍在进行，会立即启动写入，已有水印仍由核心写入层自动阻断。
- Free 单份报告付费纳入本版封版范围：单份版权详细报告 19.9 元 / 份，维权证据包 49.9 元 / 份；授权只绑定对应记录 / 案件，不升级订阅，不打开 Creator `report_export`。
- 双端一致性升级为发布门禁：图片和音频保护副本必须支持 desktop->mobile 与 mobile->desktop 双向读取 / 验证 / 解密，得到同一版权编号和 payload。
- Studio 团队空间仅保留入口、成员权限模型、共享版权库模型和团队审计模型预留，不开放真实团队成员管理或共享操作。
- L3 视频画面盲水印、4K / 8K 产品线、云端视频扣费和 `video_minutes` 正式消费继续冻结为本版之后的内部储备。

### 验证

- 通过 `npm run commercial:ci`，覆盖商业合同、双端一致性、共享水印架构、视频分层、跨端 release 门禁、桌面构建、后端测试、Tauri release-scope 测试、Flutter analyze / test、云同步 CI 和云视频 CI。
- 通过后端临时服务 smoke：`GET /v1/health`、`cloud:contract`、`cloud:e2e`。
- 桌面 release exe：`src-tauri/target/release/hidden_shield.exe`，SHA256 `64D14F5195EA7E027FB4CDFF8F557C9231EF7143FC04056101086C89A6B8B8F1`，短启进程 smoke 通过。
- 后端 release exe：`feedback-backend/target/release/hiddenshield-feedback-backend.exe`，SHA256 `FE54082D64A1F721660EC3AEF5E4168C355167396F9E70A4373AFCC7E9F1415A`，`/v1/health` smoke 通过。
- Windows NSIS 安装器：`src-tauri/target/release/bundle/nsis/HiddenShield_0.1.0_x64-setup.exe`，SHA256 `4DF0C95596927AFE7C29DC994B836066BC45DE7F72173750098C7D6FBE31C3B6`，silent install smoke 通过，安装目录只包含正式主程序和卸载器。
- 发布包边界修复：L2 `video_fingerprint_spike` 研发工具迁出 Tauri 正式应用 crate，改为 `tools/video-fingerprint-spike` 独立工具包，避免内部 spike 二进制进入用户安装包。

### 发布前外部阻断

- 需要 Android / iOS 真机或模拟器完成原生移动端运行态 QA 和真实保护副本双端文件流转。
- 需要真实微信商户参数、公网 HTTPS 回调、真实订单 / 退款撤销验收和法务审阅后，才能开启真实收费上线。
- Windows MSI 安装器尚未生成：WiX 下载超时仍需后续补齐；当前 Windows 候选交付产物为未签名 NSIS 安装器和 release exe。

## [0.1.0] - 2026-04-19

> 历史记录：以下是早期桌面 MVP 能力记录，不代表当前封版发布口径。当前边界以 `docs/当前真实能力边界说明.md` 和 `docs/封版收口计划.md` 为准。

### 核心功能

- **图片盲水印**：从 LSB 空域隐写升级为 DWT-DCT-SVD 混合算法，抗 JPEG 压缩/缩放/裁剪
- **音频盲水印**：QIM 频域量化索引调制，2kHz-8kHz 中高频段嵌入，抗重编码
- **视频多平台压制**：抖音 1080×1920 / B站 1920×1080 HEVC / 小红书 1080×1440
- **HDR 自动色彩映射**：检测 PQ/HLG 信号自动 tonemap 到 SDR
- **维权取证**：拖入文件自动提取水印并匹配本地金库
- **存证报告**：一键生成结构化版权存证报告（含时间戳 / 网络授时辅助材料）

### 安全与防伪

- **32 字节融合载荷**：Magic + User Seed + 纳秒时间戳 + Device ID + File Hash + HMAC
- **创作者身份系统**：首次启动设定创作者标识，跨设备可追溯
- **防移植攻击**：原文件 SHA-256 前缀绑定，水印无法被提取后注入其他文件
- **RFC 3161 / 网络授时证据**：历史 MVP 曾记录 TSA fallback 与 HTTP 授时兜底；不代表当前生产可信证书链 / TSA 已上线
- **HTTP 网络授时**：阿里云/腾讯云/百度三源并发兜底

### 工程优化

- **双段式看门狗**：冷启动 90s + 失速 30s，容忍大文件初始化
- **HW/SW 信号量分离**：GPU 编码串行排队，防止显卡 session 超限
- **stdout 管道死锁修复**：FFmpeg stdout 设为 null，防止缓冲区满挂起
- **FFmpeg 心跳**：`-stats_period 1` 强制每秒输出状态
- **硬件编码自动降级**：GPU 失败自动切换 CPU 软编码并通知用户

### 前端

- **免费版单平台限制**：选择 >1 个平台时前端拦截并提示 Pro
- **动态进度面板**：只显示有实际进度的平台，图片/音频不显示空平台条
- **首次启动引导**：创作者身份设定 onboarding 页面
- **维权取证拖拽**：复用 DropZone 组件，拖入即自动取证
