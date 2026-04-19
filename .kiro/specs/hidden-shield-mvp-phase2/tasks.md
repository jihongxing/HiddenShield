# 实施计划：隐盾 MVP Phase 2 — 体验强化与商业闭环

## 概述

在 Phase 1 核心功能闭环基础上，实现 8 个高价值体验功能。按依赖关系排序：后端新增命令 → 前端接口层 → 前端组件 → 视图集成 → 商业化钩子 → 信任感。

## Tasks

- [x] 1. 后端：环境自检与文件管理器命令
  - [x] 1.1 在 `src-tauri/src/commands/probe.rs` 新增 `system_check` 命令
    - 检测 FFmpeg 可用性及版本号（调用 `ffmpeg -version` 解析首行）
    - 检测 GPU 编码器可用性（复用 `hw_detect` 模块缓存结果）
    - 检测输出目录磁盘可用空间（使用平台 API 获取分区剩余空间）
    - 检测输出目录写入权限（尝试创建临时文件后删除）
    - 返回 `SystemCheckResult` 结构体
    - _Requirements: FR-1.1_

  - [x] 1.2 在 `src-tauri/src/commands/transcode.rs` 新增 `open_output_dir` 命令
    - 接收目录路径参数
    - Windows: 调用 `explorer.exe`
    - macOS: 调用 `open`
    - Linux: 调用 `xdg-open`
    - 目录不存在时返回错误
    - _Requirements: FR-2.3_

  - [x] 1.3 注册新命令到 `src-tauri/src/lib.rs` 的 invoke_handler
    - 添加 `commands::probe::system_check`
    - 添加 `commands::transcode::open_output_dir`

  - [x] 1.4 增强 `pipeline-complete` 事件载荷
    - 在 scheduler.rs 压制完成时发送 `PipelineCompletePayload`
    - 包含每个平台的输出文件信息（路径、大小、分辨率、帧率）
    - 包含水印 UID、处理耗时、使用的编码器
    - _Requirements: FR-2.1, FR-4.1, FR-8.2_

- [x] 2. 前端接口层：新增 API 封装与纯函数
  - [x] 2.1 在 `src/lib/tauri-api.ts` 新增接口定义
    - `SystemCheckResult` 接口
    - `PipelineCompletePayload` 和 `OutputFileInfo` 接口
    - `SourceWarning` 接口
    - _Requirements: FR-1.1, FR-1.3, FR-2.1_

  - [x] 2.2 在 `src/lib/tauri-api.ts` 新增 IPC 函数
    - `systemCheck()` 函数（IPC 调用 + mock 分支）
    - `openOutputDir(path: string)` 函数
    - `listenPipelineComplete()` 事件监听函数
    - _Requirements: FR-1.1, FR-2.3_

  - [x] 2.3 在 `src/lib/tauri-api.ts` 新增纯函数
    - `recommendPlatforms(meta: SourceMeta): Platform[]` — 平台推荐
    - `recommendStrategy(meta, platforms, hwInfo): TranscodeOptions` — 策略推荐
    - `generateWarnings(meta, platforms): SourceWarning[]` — 素材警告生成
    - `buildCopyrightSummary(record: VaultRecord): string` — 存证摘要
    - `buildVerificationSummary(result, filePath): string` — 取证摘要
    - _Requirements: FR-3.1, FR-3.2, FR-3.3, FR-3.4, FR-4.2, FR-5.3_

- [x] 3. 前端组件：环境自检状态卡片
  - [x] 3.1 创建 `src/components/SystemStatus.vue`
    - 接收 `SystemCheckResult` 作为 prop
    - 所有项通过时：折叠为一行绿色摘要"环境就绪 ✓"
    - 有异常项时：展开显示各项状态，异常项红色高亮并提供修复建议
    - FFmpeg 不可用时显示"点击下载"按钮
    - 磁盘空间不足时显示具体数值
    - _Requirements: FR-1.2_

- [x] 4. 前端组件：素材异常前置提醒
  - [x] 4.1 创建 `src/components/SourceWarnings.vue`
    - 接收 `warnings: SourceWarning[]` 作为 prop
    - info 类型蓝色提示条、warning 类型橙色提示条
    - 无警告时不渲染
    - _Requirements: FR-1.3, FR-3.4_

- [x] 5. 前端组件：处理完成结果页
  - [x] 5.1 创建 `src/components/ResultPage.vue`
    - 接收 `PipelineCompletePayload` + `SourceMeta` 作为 props
    - 展示每个平台输出文件的路径、大小、分辨率
    - 展示处理耗时和使用的编码器
    - 展示"版权保护已启用 ✓"及水印 UID
    - 展示前后对比表格（源 vs 输出的分辨率、大小、帧率、色彩空间）
    - "打开输出目录"按钮
    - "复制文件路径"按钮
    - "返回工作台"按钮
    - _Requirements: FR-2.1, FR-2.2, FR-2.3, FR-8.2_

- [x] 6. 前端组件：版权存证卡片
  - [x] 6.1 创建 `src/components/CopyrightCard.vue`
    - 接收 `VaultRecord` 作为 prop
    - 展示：水印 UID、处理时间戳、原文件 SHA-256（截断）、输出平台列表、输出文件路径
    - "复制存证摘要"按钮（调用 `buildCopyrightSummary`）
    - 视觉上以卡片形式呈现，带"版权存证"标识
    - 可选 `highlight` prop 标记"新增"
    - _Requirements: FR-4.1, FR-4.2, FR-4.3_

- [x] 7. 前端组件：Pro 功能标识
  - [x] 7.1 创建 `src/components/ProBadge.vue`
    - 接收 `label: string` 和 `disabled: boolean` props
    - 展示 "Pro" 徽章样式
    - 点击时调用 `trackClick('upgrade_pro_click')` 并弹出升级提示
    - _Requirements: FR-7.1, FR-7.2, FR-7.3_

- [x] 8. 视图集成：WorkbenchView 全面改造
  - [x] 8.1 集成 SystemStatus 组件
    - 在 onMounted 中调用 `systemCheck()`
    - 将结果传递给 SystemStatus 组件，显示在工作台顶部（hero-card 下方）
    - _Requirements: FR-1.2_

  - [x] 8.2 集成 SourceWarnings 组件
    - 在 `handleSourceSelect` 后调用 `generateWarnings()` 生成警告
    - 在源文件画像区域下方展示
    - 平台选择变化时重新生成警告
    - _Requirements: FR-1.3, FR-3.4_

  - [x] 8.3 集成平台自动推荐
    - 在 `handleSourceSelect` 后调用 `recommendPlatforms()` 自动设置默认勾选
    - 调用 `recommendStrategy()` 自动设置默认策略
    - 用户手动修改后设置 `userOverridden` 标志，不再自动覆盖
    - 推荐变化时展示"已为您智能推荐"提示
    - _Requirements: FR-3.2, FR-3.3_

  - [x] 8.4 集成 ResultPage 组件
    - 监听 `pipeline-complete` 事件，收到后切换到结果页视图
    - 结果页中嵌入 CopyrightCard 展示本次存证信息
    - "返回工作台"按钮重置状态回到初始界面
    - _Requirements: FR-2.1, FR-4.1_

  - [x] 8.5 多平台 Pro 提示
    - 选择 2 个以上平台时，在平台选择器下方展示 Pro 提示条
    - "多平台并行输出是 Pro 能力"（MVP 不实际限制）
    - _Requirements: FR-7.1_

  - [x] 8.6 关闭保护
    - Tauri 模式：使用 `window.onCloseRequested` 拦截关闭
    - 浏览器模式：使用 `beforeunload` 事件
    - 仅在 `busy === true` 时拦截
    - _Requirements: FR-6.4_

- [x] 9. 视图集成：VaultView 改造
  - [x] 9.1 使用 CopyrightCard 组件展示记录
    - 替换或增强现有的记录列表展示
    - 最新记录高亮标识"新增"
    - _Requirements: FR-4.2, FR-4.3_

  - [x] 9.2 添加 Pro 功能入口
    - "导出版权库"按钮置灰 + ProBadge
    - "批量处理"入口 + ProBadge
    - _Requirements: FR-7.2_

  - [x] 9.3 添加信任标识
    - 页面顶部展示"📁 数据仅存储在本机"提示
    - _Requirements: FR-8.4_

- [x] 10. 视图集成：VerifyView 增强
  - [x] 10.1 增强取证结果分级展示
    - 已命中（confidence >= 0.95）：绿色卡片，展示匹配记录详情（使用 CopyrightCard）
    - 疑似命中（0.5 ~ 0.95）：黄色卡片，展示置信度和可能原因
    - 未命中（< 0.5）：灰色卡片，展示未命中原因解释
    - _Requirements: FR-5.1_

  - [x] 10.2 命中后关联原记录
    - 展示 `matchedRecord` 的完整信息
    - 提供"跳转到版权库"按钮（emit 事件切换 App.vue 的 activeTab）
    - _Requirements: FR-5.2_

  - [x] 10.3 取证摘要复制
    - 调用 `buildVerificationSummary()` 生成摘要文本
    - "复制取证摘要"按钮
    - _Requirements: FR-5.3_

  - [x] 10.4 未命中原因解释
    - confidence < 0.1：展示"该文件可能非本机处理的作品"
    - 0.1 ~ 0.5：展示"该文件可能经过深度篡改（重编码、裁剪、音轨替换等）"
    - _Requirements: FR-5.4_

  - [x] 10.5 添加 Pro 功能入口
    - "导出 PDF 取证报告"按钮置灰 + ProBadge
    - _Requirements: FR-7.2_

- [x] 11. App.vue 全局改造
  - [x] 11.1 侧边栏信任标识
    - 在侧边栏底部（升级按钮下方）添加"🔒 全本地处理 · 零上传"标识
    - _Requirements: FR-8.1, FR-8.4_

  - [x] 11.2 Tab 切换通信
    - 暴露 `switchTab(tab: AppTab)` 方法或使用 provide/inject
    - 供 VerifyView 的"跳转到版权库"按钮调用
    - _Requirements: FR-5.2_

  - [x] 11.3 批量处理入口（Pro 预览）
    - 在 tab-list 下方添加"批量处理"入口 + ProBadge
    - 点击触发升级提示
    - _Requirements: FR-7.2_

- [x] 12. 任务管理增强
  - [x] 12.1 增强 ProgressPanel 组件
    - 展示每个平台独立进度条（已有基础结构）
    - 失败时展示具体失败原因（从 pipeline-progress 事件的 stage 字段解析）
    - 失败后展示"重试"按钮
    - _Requirements: FR-6.1, FR-6.2, FR-6.3, FR-8.3_

  - [x] 12.2 在 WorkbenchView 中实现重试逻辑
    - "重试"按钮使用上次的参数重新调用 `startPipeline`
    - 保存上次的 inputPath、platforms、options 用于重试
    - _Requirements: FR-6.3_

- [x] 13. 后端：崩溃收集与遥测
  - [x] 13.1 创建 `src-tauri/src/telemetry/mod.rs` 模块
    - 定义 `CrashReport` 结构体（panic_message, backtrace, os_version, app_version, anonymous_device_id）
    - 定义 `FfmpegCrashReport` 结构体（exit_code, stderr_tail, encoder, input_format）
    - 实现匿名设备 ID 生成（基于机器特征的 SHA-256 哈希，不可逆）
    - 实现遥测开关状态读取（从 app_data_dir/config.json）
    - _Requirements: FR-9.1, FR-9.2, FR-9.3_

  - [x] 13.2 实现全局 panic hook
    - 在 `lib.rs` 的 `setup` 中注册 `std::panic::set_hook`
    - 捕获 panic 信息和 backtrace
    - 写入本地 `{app_data_dir}/logs/crash.log`
    - 遥测开启时异步 POST 到上报端点（fire-and-forget，不阻塞崩溃流程）
    - 脱敏处理：过滤堆栈中的本地路径前缀，仅保留 crate 内相对路径
    - _Requirements: FR-9.1, FR-9.4_

  - [x] 13.3 增强 FFmpeg 进程崩溃捕获
    - 在 `pipeline/ffmpeg.rs` 中，FFmpeg 非零退出时收集 stderr 最后 20 行
    - 脱敏：从 stderr 中移除所有本地文件路径（正则替换为 `[path]`）
    - 写入本地 crash.log
    - 遥测开启时上报 `FfmpegCrashReport`
    - _Requirements: FR-9.2, FR-9.4_

  - [x] 13.4 新增 Tauri 命令：遥测控制
    - `get_telemetry_enabled() -> bool`
    - `set_telemetry_enabled(enabled: bool)`
    - `export_crash_log() -> String`（返回 crash.log 内容）
    - `get_data_usage() -> DataUsageInfo`（FFmpeg 大小 + DB 大小 + 日志大小）
    - 注册到 invoke_handler
    - _Requirements: FR-9.3, FR-9.4, FR-11.2_

- [x] 14. 后端：卸载清理支持
  - [x] 14.1 新增 Tauri 命令：数据清理
    - `clear_all_data()` — 删除 FFmpeg 缓存、vault.db、日志文件
    - `clear_cache_only()` — 仅删除 FFmpeg 缓存和日志，保留 vault.db
    - 执行前验证无活跃任务（否则返回错误）
    - 注册到 invoke_handler
    - _Requirements: FR-11.2, FR-11.3_

  - [x] 14.2 配置 NSIS 卸载脚本
    - 在 `tauri.conf.json` 的 `bundle.windows.nsis` 中配置 `installerHooks`
    - 创建 `src-tauri/nsis-hooks.nsi` 自定义卸载钩子
    - 卸载时弹出 MessageBox 询问是否删除用户数据
    - 根据用户选择清理 `$APPDATA/com.hiddenshield.desktop/` 对应内容
    - _Requirements: FR-11.1_

- [x] 15. 配置：Tauri Updater 插件
  - [x] 15.1 启用 tauri-plugin-updater
    - 在 `Cargo.toml` 添加 `tauri-plugin-updater` 依赖
    - 在 `lib.rs` 注册 `.plugin(tauri_plugin_updater::init())`
    - 在 `tauri.conf.json` 的 `plugins.updater` 中配置端点和公钥占位
    - _Requirements: FR-10.1_

  - [x] 15.2 前端更新检查逻辑
    - 在 `src/lib/tauri-api.ts` 新增 `checkForUpdate()` 和 `installUpdate()` 函数
    - 在 `App.vue` 的 `onMounted` 中延迟 5 秒调用 `checkForUpdate()`
    - 有更新时设置响应式状态，展示更新横幅
    - _Requirements: FR-10.2, FR-10.3_

  - [x] 15.3 创建更新横幅 UI
    - 在 App.vue 顶部添加条件渲染的更新横幅
    - 展示版本号、"立即更新"按钮、"稍后"按钮
    - 点击"立即更新"后展示下载进度条
    - 下载完成后提示重启
    - _Requirements: FR-10.2, FR-10.3_

- [x] 16. CI/CD：GitHub Actions 发布流水线
  - [x] 16.1 创建 `.github/workflows/release.yml`
    - 触发条件：`push tags: v*`
    - Matrix 构建：windows-latest + macos-latest
    - 使用 `tauri-apps/tauri-action@v0` 构建
    - Windows：从 secrets 注入签名证书环境变量
    - macOS：从 secrets 注入 Apple 证书 + Notarization 凭据
    - 产物上传到 GitHub Releases
    - 同时生成 `latest.json` 供 Updater 消费
    - _Requirements: FR-12.1, FR-12.2, FR-12.3, FR-12.4_

  - [x] 16.2 文档：Secrets 配置说明
    - 在项目 README 或 docs 中记录需要配置的 GitHub Secrets：
      - `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
      - Windows: `WINDOWS_CERTIFICATE` (base64 PFX) / `WINDOWS_CERTIFICATE_PASSWORD`
      - macOS: `APPLE_CERTIFICATE` (base64 P12) / `APPLE_CERTIFICATE_PASSWORD` / `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID`
    - _Requirements: FR-12.2, FR-12.3_

- [x] 17. 前端：设置页面（遥测 + 清理）
  - [x] 17.1 在 WorkbenchView 或新建 SettingsView 中添加设置区域
    - 遥测开关（toggle）+ 说明文案
    - 数据占用空间展示（调用 `get_data_usage()`）
    - "清除缓存"按钮（仅清理 FFmpeg + 日志）
    - "清除所有数据"按钮（核弹按钮，二次确认）
    - macOS 提示："卸载前建议点击清除所有数据"
    - _Requirements: FR-9.3, FR-11.2, FR-11.3_

  - [x] 17.2 首次启动遥测告知横幅
    - 检测是否首次启动（config.json 中无 `telemetry_acknowledged` 标记）
    - 展示非阻塞横幅："隐盾会上报崩溃信息以改善稳定性，不含任何个人文件信息"
    - 用户点击"了解"后写入标记，不再展示
    - _Requirements: FR-9.3_

- [x] 18. 检查点 — 全量编译与类型验证
  - 运行 `npx vue-tsc --noEmit` 确保 TypeScript 无错误
  - 运行 `cargo check --manifest-path src-tauri/Cargo.toml` 确保 Rust 编译通过
  - 运行 `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` 确保无 lint 警告
  - 如有问题请向用户确认

## Notes

- Phase 2 以前端体验改造为主（约 70% 工作量），后端新增命令 + 增强事件载荷 + 遥测模块
- 所有推荐、警告、摘要生成逻辑为纯前端函数，不增加后端复杂度
- CopyrightCard 组件在结果页、版权库、取证结果中三处复用
- ProBadge 组件统一所有 Pro 功能入口的视觉和行为
- 保持 mock 分支兼容，确保浏览器开发模式可用
- 不涉及核心算法变更，不影响 Phase 1 已有功能
- Pro 功能钩子仅做展示和事件记录，不实际限制功能（验证付费意愿）
- 关闭保护仅在有活跃任务时触发，不影响正常使用

### 商业分发相关说明
- 遥测模块遵循最小数据原则：只上报 panic 堆栈和 FFmpeg exit code，绝不上报用户文件名或本地路径
- Updater 端点初期可使用 GitHub Releases + `latest.json` 静态文件，无需自建服务器
- NSIS 卸载钩子是 Windows 平台特有配置，macOS 通过应用内"清除数据"按钮引导
- CI/CD 流水线依赖 GitHub Secrets 中的证书配置，本地开发不受影响
- 签名证书（EV Code Signing / Apple Developer）需要创始人自行购买并配置到 Secrets
- Task 13-17 可与 Task 1-12 并行开发，无强依赖关系（除 Task 18 检查点需全部完成后执行）
