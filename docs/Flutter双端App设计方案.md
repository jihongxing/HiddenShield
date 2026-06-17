# HiddenShield Flutter 双端 App 设计方案

**文档状态：** 草案 V0.1  
**目标平台：** iOS + Android  
**推荐技术栈：** Flutter + Rust FFI + 本地 SQLite + 桌面端同步服务  
**产品定位：** 随身版权哨站。移动端不是桌面端缩小版，而是创作者在拍摄、发布、取证现场使用的轻量版权入口。

---

## 1. 背景与目标

HiddenShield 当前桌面端已经覆盖重型生产链路：视频探测、FFmpeg 压制、HDR 修复、音频盲水印、版权金库、维权取证。移动端要避免把这套桌面工作台硬搬到手机上。

移动端的核心机会是离创作者最近：

- 拍完照片或音频后，立刻本地确权。
- 在外发现疑似搬运内容，立刻做初步水印检测。
- 随身查看版权金库、证据链和维权材料。
- 和桌面端同步版权记录、证据链和派生关系。

**一句话目标：** 打开 App，选择或拍摄作品，几秒内完成版权保护，并把记录沉淀进可跨端漫游的版权金库。

---

## 2. 产品原则

### 2.1 手机端只做高频轻任务

移动端首版只做图片和音频的本地水印嵌入、提取、记录入库。视频处理不放进移动端。

原因很直接：

- iOS 不允许运行时唤起 FFmpeg 子进程。
- 手机长时间转码会触发发热、降频、后台中断和系统强杀。
- 用户在手机上愿意等待的是秒级保护，不是十几分钟压制。

### 2.2 桌面端是生产工具，移动端是版权随身层

| 场景 | 桌面端 | 移动端 |
|------|--------|--------|
| 多平台视频压制 | 主力能力 | 不做 |
| 图片水印 | 支持 | 主力能力 |
| 音频水印 | 支持 | 主力能力 |
| 本地版权金库 | 主库 | 查看、轻记录、同步 |
| 维权取证 | 深度报告 | 快速检测、分享报告 |
| 硬件编码 | 本地探测 | 不做 |
| 长任务后台 | 可控 | 尽量避免 |

### 2.3 默认本地、联网透明

图片和音频默认在本地处理，不上传原文件。桌面端同步、TSA 存证、匿名反馈必须有清晰状态和授权入口。

---

## 3. 首版范围

### 3.1 MVP 必须做

- 创作者身份初始化：复用桌面端 User Seed / device id 思路。
- 图片水印嵌入：从相册、文件、相机导入图片，本地生成带水印副本。
- 图片水印提取：从疑似侵权图片中提取水印并匹配本地金库。
- 音频水印嵌入：从文件导入 30 秒以上 WAV 音频，本地生成带水印副本。
- 音频水印提取：从疑似侵权音频中提取水印并匹配本地金库。
- 本地版权金库：按时间线查看记录，支持搜索、筛选、详情页。
- 与桌面端同步：版权记录、派生链、证据摘要双向同步。
- 存证摘要分享：调用系统分享面板，分享文本摘要或报告图片。
- 权限与隐私中心：相册、文件、网络、通知、遥测状态可见可控。
- Flutter 与 Rust 核心桥接：调用 `watermark-core` 的图片 / 音频 bytes 接口。

### 3.2 MVP 不做

- 本地视频压制。
- 本地 FFmpeg 调用。
- 长时间后台媒体处理。
- 多平台视频本地输出。
- 团队协作与多人权限。
- 自动全网侵权监控。

### 3.3 首版后增强

- PDF 或长图版维权报告导出。
- 音频格式适配与更清晰的兼容性提示。

### 3.4 中期增强

- 证据链检索增强。
- 桌面端与移动端更完整的多端一致性检查。
- 同步冲突处理与离线补发。
- Pro 权益联动。

---

## 4. 信息架构

移动端采用底部 Tab，保持单手操作和明确任务入口。为了和桌面端尽量同构，建议直接沿用同名能力区：工作台、取证、版权库、设置。

```text
HiddenShield App
├── 工作台
│   ├── 图片嵌入
│   ├── 音频嵌入
│   ├── 相机拍摄导入
│   ├── 选择文件导入
│   └── 最近任务
├── 取证
│   ├── 选择疑似侵权文件
│   ├── 图片/音频提取结果
│   ├── 维权材料分享
│   └── 链路详情
├── 版权库
│   ├── 时间线
│   ├── 搜索/筛选
│   ├── 作品详情
│   └── 证据链详情
└── 设置
    ├── 创作者身份
    ├── Pro 权益
    ├── 同步与备份
    ├── 隐私与权限
    └── 帮助与反馈
```

---

## 5. 核心用户流程

### 5.1 首次启动

1. 展示一句清楚的定位：`随拍随护，本地生成版权指纹。`
2. 请求用户设置创作者标识，可用昵称、邮箱或自定义短语。
3. 本地生成 User Seed 和设备标识，展示水印 UID 预览。
4. 进入主界面，不强制登录。

验收标准：

- 用户不登录也能完成图片本地保护。
- 创作者标识为空时不允许继续生成版权记录。
- 明确说明创作者标识用于生成水印，不会默认上传。

### 5.2 图片保护

```text
选择图片 → 预览与风险提示 → 点击保护 → Rust 本地嵌入水印
→ 保存副本 → 写入金库 → 展示结果 → 分享/保存/查看详情
```

页面状态：

- `idle`：展示三个入口，相机、相册、文件。
- `preview`：展示图片、文件名、尺寸、预计输出格式。
- `processing`：展示本地处理进度，通常 1-3 秒。
- `success`：展示水印 UID、保存位置、金库记录。
- `failed`：展示失败原因和重试入口。

输出策略：

- 默认输出 PNG，避免 JPEG 再压缩影响水印稳定性。
- 用户选择 JPEG 时提示“体积更小，但平台二压后水印强度可能下降”。
- 保留原图不覆盖，生成 `原文件名_HiddenShield.png`。

### 5.3 音频保护

```text
选择音频 → 预览与风险提示 → 点击保护 → Rust 本地嵌入水印
→ 保存副本 → 写入金库 → 展示结果 → 分享/保存/查看详情
```

页面状态和输出策略与图片一致，默认导出 WAV 或平台安全格式，不做移动端 FFmpeg 转码。音频保护要求素材至少 30 秒；短于 30 秒的片段只允许作为取证输入尝试检测，不生成版权保护副本。

### 5.4 快速取证

```text
选择疑似侵权图片 → 本地提取水印 → 匹配金库
→ 命中/疑似/未命中 → 生成摘要 → 分享给平台投诉入口
```

结果分级：

| 结果 | UI 表达 | 后续动作 |
|------|---------|----------|
| 命中 | 明确展示原作品记录和 UID | 生成维权摘要 |
| 疑似 | 展示置信度和可能原因 | 建议桌面端深度报告 |
| 未命中 | 说明未检测到有效水印 | 允许保存检测记录 |
| 读取失败 | 文件损坏或格式不支持 | 重新选择文件 |

### 5.5 金库浏览

金库是移动端的第二主入口，不只是历史记录。

列表项展示：

- 缩略图。
- 文件名。
- 保护时间。
- 媒体类型。
- 水印 UID。
- 同步状态：本地、已同步、待同步、冲突。

详情页展示：

- 原文件哈希。
- 输出文件哈希。
- 水印 Payload 摘要。
- TSA / 网络授时信息。
- AI 内容标识字段。
- 分享、导出、删除、重新检测入口。

---

## 6. 详细功能设计

这一章只定义移动端页面和能力，不另起一套产品语言。能和桌面端共用的名词全部共用，能复用的字段全部复用。

### 6.1 工作台

对应桌面端主工作区，承接最常用的创建动作。

页面组成：

- 顶部状态条：身份状态、同步状态、网络状态。
- 主操作区：图片嵌入、音频嵌入。
- 快捷入口：相机导入、相册导入、文件导入。
- 最近任务：最近 5 条处理记录。

核心行为：

- 点图片嵌入，直接进入图片选择与预览。
- 点音频嵌入，直接进入音频选择与预览。
- 最近任务可回看结果页，也可跳到版权库详情。
- 未初始化身份时，顶部固定显示引导条，不阻断浏览。

结果保持：

- 统一显示 `UID`、`原文件哈希`、`输出文件哈希`、`TSA`、`revision`。
- 成功后默认写入版权库，不要求用户再点一次保存。

### 6.2 取证

对应桌面端取证页。

页面组成：

- 文件导入区。
- 自动识别区。
- 结果摘要区。
- 链路详情区。

核心行为：

- 选完文件后自动开始提取。
- 命中后直接展示关联版权记录。
- 疑似命中给出置信度和原因。
- 未命中允许保留检测记录，不强迫删除。

结果字段：

- 命中状态。
- 置信度。
- 水印 UID。
- 关联记录。
- TSA / 网络授时。
- parent UID。
- revision。
- rewrite_reason。

### 6.3 版权库

对应桌面端版权库和时间线。

页面组成：

- 时间线。
- 搜索。
- 筛选。
- 详情页。
- 链路页。

列表项展示：

- 文件名。
- 类型。
- 时间。
- UID。
- 同步状态。
- 是否重写。

详情页展示：

- 原文件哈希。
- 输出文件哈希。
- 水印 Payload 摘要。
- TSA / 网络授时。
- AI 内容标识。
- parent UID。
- revision。
- rewrite_reason。

交互规则：

- 长按可复制 UID、哈希、时间。
- 离线文件要有明确的缺失提示，但记录仍保留。
- 版本链只读，移动端不直接改历史。

### 6.4 设置

对应桌面端设置、身份和帮助入口。

页面组成：

- 创作者身份。
- 同步与备份。
- 权限与隐私。
- Pro 权益。
- 帮助与反馈。

核心行为：

- 可查看和重置创作者标识。
- 可配对桌面端并查看上次同步状态。
- 可开关匿名反馈和同步。
- 可导出本地数据和清空缓存。

### 6.5 统一任务模型

移动端所有处理动作都应落入同一套任务状态机：

- `image_embed`
- `image_extract`
- `audio_embed`
- `audio_extract`
- `sync_push`
- `sync_pull`

统一状态：

- `idle`
- `running`
- `success`
- `failed`
- `canceled`

这样能直接复用桌面端的结果页、进度表达和错误提示，不用两套 UI 逻辑。

---

## 7. Flutter 技术架构

### 7.1 总体架构

```text
Flutter UI
  ↓
Feature Modules
  ↓
Application Services
  ├── WatermarkServiceClient
  ├── VaultRepository
  ├── IdentityRepository
  ├── SyncRepository
  └── PermissionService
  ↓
Infrastructure
  ├── Rust FFI / flutter_rust_bridge
  ├── SQLite
  ├── Secure Storage
  ├── File Picker / Photo Picker
  ├── HTTP Client
  └── Sync Engine
```

### 7.2 推荐 Flutter 依赖

| 能力 | 推荐库 | 说明 |
|------|--------|------|
| 状态管理 | `riverpod` | 适合 feature-first 架构和异步状态 |
| 路由 | `go_router` | 支持 Tab、详情页、深链 |
| Rust 桥接 | `flutter_rust_bridge` | 自动生成 Dart/Rust 类型绑定 |
| 本地数据库 | `drift` + `sqlite3_flutter_libs` | 类型安全、迁移清晰 |
| 安全存储 | `flutter_secure_storage` | 保存 User Seed 包装密钥、token |
| 文件选择 | `file_picker` | Android SAF / iOS Document Picker |
| 相册选择 | `photo_manager` 或 `image_picker` | 需要严格处理权限 |
| HTTP | `dio` | 同步请求、取消、重试方便 |
| 分享 | `share_plus` | 系统分享面板 |
| 联网状态 | `connectivity_plus` | 同步时判断在线状态 |

### 7.3 目录结构建议

```text
mobile_app/
├── lib/
│   ├── app/
│   │   ├── app.dart
│   │   ├── router.dart
│   │   └── theme.dart
│   ├── features/
│   │   ├── protect/
│   │   ├── verify/
│   │   ├── vault/
│   │   ├── identity/
│   │   ├── sync/
│   │   └── settings/
│   ├── core/
│   │   ├── ffi/
│   │   ├── db/
│   │   ├── storage/
│   │   ├── permissions/
│   │   └── network/
│   └── shared/
│       ├── widgets/
│       ├── models/
│       └── utils/
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── api.rs
│       └── lib.rs
├── ios/
└── android/
```

---

## 8. Rust 核心接入设计

现有 `watermark-core` 已导出关键能力：

- `WatermarkPayload`
- `embed_image_watermark_bytes()`
- `extract_image_watermark_bytes()`
- `embed_watermark_wav_bytes()`
- `extract_watermark_wav_bytes()`
- `WatermarkService`

移动端建议新建一个很薄的 Rust FFI wrapper，不让 Flutter 直接理解复杂 Rust 类型。

### 8.1 FFI API 草案

```rust
pub struct MobileMediaPayload {
    pub user_seed: Vec<u8>,
    pub timestamp: i64,
    pub device_id: Vec<u8>,
    pub file_hash: Vec<u8>,
    pub flags: MobileAiContentFlags,
}

pub struct MobileMediaResult {
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub sha256: String,
}

pub fn embed_image_for_mobile(
    image_bytes: Vec<u8>,
    payload: MobileMediaPayload,
    output_format: String,
) -> Result<MobileMediaResult, MobileWatermarkError>;

pub fn extract_image_for_mobile(
    image_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError>;

pub fn embed_audio_wav_for_mobile(
    audio_bytes: Vec<u8>,
    payload: MobileMediaPayload,
) -> Result<MobileMediaResult, MobileWatermarkError>;

pub fn extract_audio_wav_for_mobile(
    audio_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError>;
```

### 8.2 设计要求

- FFI 层只接受 bytes，不接受文件路径，避免 Android/iOS 文件沙盒差异污染核心层。
- Flutter 负责读取文件、申请权限、保存输出文件。
- Rust 负责水印算法、Payload 编解码、输出 bytes、哈希计算。
- 错误类型必须映射成稳定 code，例如 `unsupported_format`、`decode_failed`、`watermark_not_found`。
- 音频首版只承诺 WAV bytes；如果以后要支持 MP3/AAC，先放到平台解码层，不把 FFmpeg 塞进移动端。
- 音频写入首版只承诺 30 秒以上 WAV；短音频不进入写入流程。

---

## 9. 本地数据模型

移动端数据库需要与桌面端 `vault_records` 尽量兼容，但加入同步字段。下面列的是移动端必须字段，桌面端已有的视频导出相关字段可作为只读扩展继续保留。

### 9.1 `vault_records`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | text | UUID，跨端同步的逻辑主键 |
| `original_hash` | text | 原文件 SHA-256 |
| `file_name` | text | 原始文件名 |
| `file_type` | text | image/audio/video |
| `created_at` | text | ISO 8601 |
| `updated_at` | text | 最后修改时间 |
| `watermark_uid` | text | 水印 UID |
| `parent_watermark_uid` | text? | 上一次写入的父 UID |
| `revision` | int | 第几次写入 |
| `rewrite_reason` | text? | 重写原因 |
| `thumbnail_path` | text? | 本地缩略图 |
| `local_output_path` | text? | 本地输出副本 |
| `sync_status` | text | local_only/pending/synced/conflict |
| `last_synced_at` | text? | 最后同步时间 |
| `tsa_token_path` | text? | 本地 TSA 回执 |
| `network_time` | text? | 网络授时 |
| `is_ai_generated` | bool | AI 内容标识 |
| `ai_training_permission` | text? | 训练授权 |
| `ai_generation_method` | text? | 生成方式 |
| `human_modification_level` | text? | 人类修改程度 |
| `authenticity_claim` | text? | 真实性声明 |
| `custom_metadata` | text? | 自定义说明 |

### 9.2 `sync_state`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | int | 固定 1 |
| `enabled` | bool | 是否启用与桌面同步 |
| `paired_desktop_name` | text? | 已绑定桌面名称 |
| `paired_desktop_device_id` | text? | 已绑定桌面设备 ID |
| `sync_base_url` | text? | 桌面同步服务地址 |
| `push_cursor` | text? | 本地已推送游标 |
| `pull_cursor` | text? | 本地已拉取游标 |
| `last_sync_at` | text? | 最后同步时间 |
| `last_sync_status` | text? | success/failed/conflict |
| `last_error_code` | text? | 最近一次错误码 |

### 9.3 `identity_state`

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | int | 固定 1 |
| `initialized` | bool | 是否完成初始化 |
| `watermark_uid_preview` | text | UID 预览 |
| `device_id_hex` | text | 设备 ID |
| `created_at` | text | 创建时间 |

User Seed 原文不要直接放 SQLite，放安全存储或系统 Keychain/Keystore 包装后的密文。

---

## 10. 桌面同步接口预留

### 10.1 同步协议草案

```http
POST /v1/sync/push
```

请求：

```json
{
  "deviceId": "dev_...",
  "cursor": "sync_cursor_...",
  "changes": []
}
```

响应：

```json
{
  "nextCursor": "sync_cursor_...",
  "applied": 12
}
```

### 10.2 拉取增量

```http
GET /v1/sync/changes?cursor=sync_cursor_...
```

响应：

```json
{
  "nextCursor": "sync_cursor_...",
  "changes": []
}
```

当前实现状态：

- 移动端已实现 `CloudAccountClient`、`CloudSyncTransport`。
- 桌面端 Tauri 已实现同协议 `CloudSyncClient`，并提供 `continue_cloud_account`、`push_desktop_vault_record_to_cloud`、`fetch_cloud_changes` 命令。
- 仓库根目录提供 `npm run cloud:backend` 与 `npm run cloud:contract`，用于本地验证双端共同协议；`cloud:mock` 仅保留为轻量协议对照工具。

### 10.3 设计要求

- 同步对象优先是版权记录、证据摘要、派生链和状态，不默认同步原始媒体文件。
- MVP 先支持一台已配对桌面，别急着做多桌面、多账号树。
- 变更包必须可幂等重放。
- 同步中断后允许继续补发。
- 桌面同步服务不需要知道用户创作者明文标识，只接收派生后的 UID / 设备 ID。
- 同步冲突优先保留两端变更痕迹，移动端不自动覆盖原始媒体内容。

---

## 11. 视觉与交互方向

HiddenShield 是版权与证据工具，视觉上要可信、冷静、专业，但不能像政务系统一样沉重。

### 11.1 设计关键词

- 安全感。
- 证据链。
- 本地优先。
- 秒级完成。
- 创作者友好。

### 11.2 页面风格

- 主界面使用清晰的任务入口，不做营销式首页。
- 控件密度适中，手机上优先大触控区和明确状态。
- 版权 UID、哈希、时间戳等信息使用等宽数字或 tabular nums。
- 卡片只用于记录项、结果项、弹窗，不做层层嵌套。
- 重要状态使用明确图标和语义色：成功、警告、失败、同步中。

### 11.3 推荐设计系统方向

| 项 | 建议 |
|----|------|
| 美学方向 | Industrial / Utilitarian，克制的专业工具感 |
| 主色 | 深青绿或蓝绿，表达安全和技术可信 |
| 辅助色 | 琥珀色用于风险提示，蓝色用于同步状态 |
| 字体 | 系统字体优先，数字字段启用 tabular nums |
| 圆角 | 8px 以内，按钮和列表项保持利落 |
| 动效 | 只服务状态变化，例如处理完成、同步状态、Tab 切换 |

---

## 12. 权限与平台差异

| 能力 | Android | iOS | 设计策略 |
|------|---------|-----|----------|
| 相册读取 | Photo Picker / READ_MEDIA_IMAGES | Photos Picker | 优先使用系统选择器，减少常驻权限 |
| 文件读取 | SAF | Document Picker | Flutter 层统一成 bytes |
| 文件保存 | MediaStore / app dir | Photos / Files | 保存后给用户明确位置和分享入口 |
| 安全存储 | Android Keystore | Keychain | 存 User Seed 包装密钥 |
| 通知 | Runtime permission | 用户授权 | 只用于同步完成、冲突提醒 |
| 后台任务 | WorkManager | BGTaskScheduler 限制多 | 不依赖后台长计算 |

---

## 13. 错误处理与空状态

### 13.1 常见错误

| 错误 | 用户文案 | 技术处理 |
|------|----------|----------|
| 图片格式不支持 | 当前图片格式暂不支持，请换一张图片 | `unsupported_format` |
| 水印提取失败 | 没有检测到有效隐盾水印 | `watermark_not_found` |
| 权限被拒绝 | 需要访问你选择的文件才能继续 | 引导重新选择或去设置 |
| 保存失败 | 保存失败，请检查存储空间 | 检查可用空间、重试 |
| 桌面同步失败 | 桌面暂时不可达，记录已保留等待重试 | 保留变更队列、允许继续 |
| 同步冲突 | 这条记录在另一台设备上也被修改过 | 展示冲突详情 |

### 13.2 空状态

- 保护页空状态：`选择一张作品，生成第一条版权记录。`
- 金库空状态：`你的版权金库还没有记录。`
- 取证空状态：`选择疑似侵权文件，检测是否含有隐盾水印。`
- 同步空状态：`还没有绑定桌面设备，去我的页面完成配对。`

---

## 14. 埋点与隐私

移动端可以复用现有匿名反馈后端思路，但必须默认透明。

可采集：

- 功能使用结果：图片保护成功/失败、取证成功/失败。
- 错误码：不包含文件名、路径、哈希原文。
- 设备能力：系统版本、App 版本、是否低内存。
- 同步状态：推送失败、拉取失败、冲突、重试。

不可采集：

- 原文件。
- 原文件路径。
- 原文件哈希原文。
- 创作者明文标识。

设置页必须提供：

- 匿名反馈开关。
- 桌面同步总开关。
- 本地数据导出。
- 清除缓存。
- 删除全部数据。

---

## 15. 开发阶段计划

### Phase 0：移动工程骨架，3-5 天

- 创建 Flutter 工程。
- 接入路由、主题、Riverpod。
- 建立 Rust FFI wrapper。
- 跑通 Flutter 调 Rust 的 hello-world。
- 建立 CI 构建 Android debug 和 iOS simulator。

交付物：

- 可启动 App。
- 四个 Tab 空页面。
- FFI 示例调用通过。

### Phase 1：图片保护 MVP，2 周

- 创作者身份初始化。
- 图片选择与预览。
- 调用 `watermark-core` 完成本地嵌入。
- 保存输出文件。
- 写入本地金库。
- 金库列表与详情。

交付物：

- 用户可以在手机上完成第一条图片版权记录。

### Phase 2：图片取证与分享，1 周

- 图片水印提取。
- 本地金库匹配。
- 检测结果页。
- 版权摘要分享。
- 失败与疑似命中状态。

交付物：

- 用户可以现场检测疑似侵权图片并分享结果。

### Phase 3：音频轻处理，1-2 周

- WAV bytes 嵌入与提取。
- 音频文件选择。
- 音频记录入库。
- 格式限制说明。

交付物：

- 支持 30 秒以上 WAV 音频版权保护；短片段仅作为取证尝试，不作为稳定承诺。

### Phase 4：桌面同步，2-4 周

- 桌面配对与设备绑定。
- 记录差量推送与拉取。
- 同步状态、冲突状态与重试队列。
- 桌面端记录移动端可见。

交付物：

- 手机可以和一台已配对桌面完成金库同步。

### Phase 5：一致性收尾，1-2 周

- 同步冲突解决细节。
- 离线补发与断点续传。
- 同步数据加密与密钥轮换。
- 删除与恢复的同步语义。

交付物：

- 两端记录状态一致，断网后可自动补齐。

---

## 16. 关键风险

| 风险 | 影响 | 应对 |
|------|------|------|
| Flutter Rust FFI 打包复杂 | 延迟首版 | Phase 0 先验证 iOS 真机和 Android arm64 |
| 图片算法经平台二压后存活率不足 | 核心价值受损 | 建立测试集，覆盖微信、小红书、抖音图文压缩 |
| 音频格式兼容度不足 | 用户无法导入 | 只承诺支持少数稳定格式，先把 WAV 跑通 |
| 音频时长不足 | 用户误以为短片段也应被保护 | 写入前校验 30 秒门槛，并给出选择完整作品的提示 |
| iOS 文件保存体验绕 | 用户找不到输出 | 成功页提供保存到相册、保存到文件、分享三入口 |
| 跨端同步安全设计复杂 | 上线变慢 | MVP 先记录差量同步，不同步原始媒体 |

---

## 17. 验收清单

MVP 可上线前至少满足：

- iOS 真机和 Android 真机均能完成图片保护。
- iOS 真机和 Android 真机均能完成音频保护。
- 移动端不提供本地视频盲水印入口。
- App 断网时图片保护和本地金库可用。
- 删除 App 前用户能导出本地记录。
- 权限拒绝路径不崩溃。
- 10MB 图片处理不 OOM。
- 连续处理 20 张图片无明显内存增长。
- 30 秒以上音频嵌入与提取结果稳定。
- 水印提取错误能稳定返回错误码。
- 金库记录包含哈希、UID、创建时间、媒体类型。
- 金库记录包含 `parent_watermark_uid`、`revision`、`rewrite_reason`。
- 已配对桌面可同步新增/修改记录，默认不同步原始媒体文件。
- 所有联网行为都有明确开关或确认。

---

## 18. 建议的下一步

1. 先做 Phase 0 技术 spike，验证 `flutter_rust_bridge` 能稳定编译 `watermark-core` 到 iOS/Android。
2. 同时补一份移动端 UI 线框图，覆盖工作台、取证、版权库、设置四个 Tab。
3. 为图片水印建立移动平台压缩存活率测试集，用真实平台下载回来的图片验证。
4. 再决定是否把 Flutter 工程放在当前仓库 `mobile_app/`，还是新建独立仓库并通过 path/git dependency 引用 `watermark-core`。
