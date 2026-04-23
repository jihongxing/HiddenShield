# 实施计划：隐盾 (HiddenShield) MVP V1.0

## 概述

将当前项目中所有 stub/占位实现替换为真实功能。按依赖关系排序：基础设施（依赖补全、FFmpeg 环境探测、数据库初始化）→ 核心功能（编码器参数引擎、水印模块、图片水印、压制流水线）→ 系统防护（磁盘空间、休眠抑制）→ 前端集成（DropZone 真实路径、进度面板对接）→ 法律合规（取证置信度、免责声明）。后端使用 Rust，前端使用 TypeScript + Vue 3。

## Tasks

- [x] 1. 基础设施：Cargo 依赖补全与 AppState 扩展
  - [x] 1.1 更新 `src-tauri/Cargo.toml` 添加缺失依赖
    - 添加 `realfft = "3"` 用于音频 FFT 水印处理
    - 添加 `hound = "3.5"` 用于 WAV 文件读写
    - 添加 `rusqlite = { version = "0.31", features = ["bundled"] }` 用于 SQLite
    - 添加 `crc32fast = "1.4"` 用于 CRC32 校验
    - 添加 `thiserror = "1"` 用于错误类型定义
    - 添加 `image = "0.25"` 用于图片读写和 LSB 隐写水印
    - 添加 `reqwest = { version = "0.12", features = ["stream"] }` 用于联网取证、网络授时等受控网络请求
    - 将 tokio 特性改为 `full`
    - 添加 `[dev-dependencies]` 中的 `proptest = "1.4"` 和 `tempfile = "3"`
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 12.3, 13.3_

  - [x] 1.2 扩展 `src-tauri/src/lib.rs` 中的 AppState
    - 添加 `db: Mutex<Connection>` 字段（rusqlite 连接）
    - 添加 `ffmpeg_paths: OnceLock<FfmpegPaths>` 字段
    - 添加 `hw_info: OnceLock<DetectedHardware>` 字段
    - 在 Tauri Builder setup 中初始化 SQLite 连接并执行建表
    - _Requirements: 1.4, 4.5, 9.1, 9.4_

  - [x] 1.3 定义全局错误类型 `PipelineError`
    - 在 `src-tauri/src/pipeline/` 下新建错误类型文件
    - 使用 thiserror 派生 `PipelineError` 枚举，包含 FfmpegNotFound、FfmpegDownloadFailed、ProbeFailed、FileNotFound、InsufficientDiskSpace、FfmpegFailed、WatermarkEmbedFailed、WatermarkExtractFailed、DatabaseError、SleepInhibitFailed、Cancelled 变体
    - _Requirements: 8.6, 8.7, 13.6, 14.3_

- [x] 2. FFmpeg/ffprobe 进程管理模块
  - [x] 2.1 创建 `src-tauri/src/pipeline/ffmpeg.rs`
    - 实现 `detect_ffmpeg()` 函数：仅检查系统 PATH 中的 `ffmpeg` / `ffprobe`，并缓存路径到 `FfmpegPaths` 结构体
    - 对 PATH 命中的二进制执行健康检查，拒绝损坏、不可执行或异常包装脚本
    - 实现 `ffprobe_source()` 函数：调用 ffprobe -print_format json 获取视频元数据，解析 JSON 输出为 `FfprobeOutput` 结构体
    - 实现 `spawn_ffmpeg()` 函数：启动 FFmpeg 子进程并返回句柄
    - 实现 `parse_progress_line()` 函数：从 FFmpeg stderr 解析 `time=HH:MM:SS.ms` 字段计算进度百分比
    - 更新 `pipeline/mod.rs` 导出新模块
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 8.3, 13.1, 13.2, 13.3, 13.4, 13.5, 13.6_

  - [ ]* 2.2 编写 `parse_progress_line` 属性测试
    - **Property 8: FFmpeg 进度解析**
    - **Validates: Requirements 8.3**

- [x] 3. 检查点 — 确保 FFmpeg 模块编译通过
  - 确保所有测试通过，如有问题请向用户确认。

- [x] 4. 工具模块增强
  - [x] 4.1 增强 `src-tauri/src/utils/hash.rs`
    - 新增 `sha256_of_file(path: &str) -> Result<String, io::Error>` 函数，流式读取文件计算 SHA-256
    - _Requirements: 2.3_

  - [x] 4.2 增强 `src-tauri/src/utils/fs.rs`
    - 新增 `output_file_name(source_stem: &str, platform: Platform) -> String` 函数，生成 `{源文件名}_{平台中文名}优化版.mp4` 格式
    - 新增临时文件管理工具函数
    - _Requirements: 8.5_

  - [ ]* 4.3 编写输出文件命名属性测试
    - **Property 9: 输出文件命名格式**
    - **Validates: Requirements 8.5**

- [x] 5. 编码器参数引擎
  - [x] 5.1 重写 `src-tauri/src/encoder/hw_detect.rs`
    - 实现真实硬件编码器探测：通过执行 `ffmpeg -c:v <encoder> -f null -` 测试编码器可用性
    - macOS 测试 h264_videotoolbox / hevc_videotoolbox
    - Windows/Linux 按 NVENC → QSV → AMF 优先级测试
    - 所有硬件编码器不可用时回退到 libx264/libx265
    - 新增 `HwEncoderType` 枚举
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [x] 5.2 重写 `src-tauri/src/encoder/presets.rs`
    - 实现 `build_transcode_config()` 函数，根据平台 + 源视频 + 硬件编码器 + 用户选项生成完整 FFmpeg 参数
    - 抖音：1080×1920 / H.264 / CRF 18 / 12Mbps / 30fps
    - B站：1920×1080 / HEVC / CRF 20 / 16Mbps / 30 或 60fps
    - 小红书：1080×1440 / H.264 / CRF 17 / 15Mbps / 30fps
    - 所有平台：AAC 192kbps / 44.1kHz / faststart
    - 支持 letterbox（scale + pad）和 smart_crop 策略
    - 支持 fast_gpu 模式下替换为硬件编码器
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [ ]* 5.3 编写平台压制参数属性测试
    - **Property 2: 平台压制参数不变量**
    - **Validates: Requirements 5.1, 5.2, 5.3, 5.6**

  - [ ]* 5.4 编写 Letterbox 滤镜属性测试
    - **Property 3: Letterbox 滤镜生成**
    - **Validates: Requirements 5.4**

  - [ ]* 5.5 编写硬件编码器模式切换属性测试
    - **Property 4: 硬件编码器模式切换**
    - **Validates: Requirements 5.5**

  - [x] 5.6 重写 `src-tauri/src/encoder/tonemap.rs`
    - 实现基于 ffprobe 色彩空间数据的 HDR 判断函数（替代文件名推断）
    - 实现 `build_tonemap_filter()` 函数：HDR 时生成 zscale → tonemap=hable → format=yuv420p 完整滤镜链
    - SDR 时返回 None，不插入 tonemap 滤镜
    - _Requirements: 6.1, 6.2, 6.3_

  - [ ]* 5.7 编写 HDR 色彩空间判断属性测试
    - **Property 1: HDR 色彩空间判断正确性**
    - **Validates: Requirements 2.2**

  - [ ]* 5.8 编写 Tonemap 滤镜链属性测试
    - **Property 5: Tonemap 滤镜链条件生成**
    - **Validates: Requirements 6.1, 6.2, 6.3**

- [x] 6. 检查点 — 确保编码器模块编译通过且属性测试通过
  - 确保所有测试通过，如有问题请向用户确认。

- [x] 7. 音频盲水印模块
  - [x] 7.1 创建 `src-tauri/src/pipeline/watermark.rs`
    - 定义 `WatermarkPayload` 结构体（magic + timestamp + machine_fingerprint + crc32）
    - 实现 `encode_payload()` / `decode_payload()` 序列化/反序列化函数
    - 实现 `embed_watermark()` 函数：使用 realfft 对 PCM 按 4096 帧 FFT，在 2kHz-8kHz 频段以 0.02 强度编码水印比特，静音帧跳过
    - 实现 `extract_watermark()` 函数：逆向提取水印比特并验证魔数和 CRC32
    - 更新 `pipeline/mod.rs` 导出新模块
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

  - [ ]* 7.2 编写水印载荷序列化 Round-trip 属性测试
    - **Property 6: 水印载荷序列化 Round-trip**
    - **Validates: Requirements 7.5, 10.3**

  - [ ]* 7.3 编写水印嵌入/提取 Round-trip 属性测试
    - **Property 7: 水印嵌入/提取 Round-trip**
    - **Validates: Requirements 7.2, 7.3, 10.2**

- [x] 8. SQLite 数据层真实实现
  - [x] 8.1 重写 `src-tauri/src/db/queries.rs`
    - 实现 `init_db(conn: &Connection)` 函数：执行建表 SQL 和索引创建
    - 实现 `insert_record(conn: &Connection, record: &VaultRecord)` 函数
    - 实现 `list_records(conn: &Connection) -> Vec<VaultRecord>` 函数：按 created_at 倒序查询
    - 实现 `find_by_watermark_uid(conn: &Connection, uid: &str) -> Option<VaultRecord>` 函数
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

  - [x] 8.2 更新 `src-tauri/src/commands/vault.rs`
    - 修改 `VaultRecord` 结构体：将 `platforms: Vec<String>` 拆分为 `output_douyin`、`output_bilibili`、`output_xhs` 三个 `Option<String>` 字段，新增 `thumbnail_path`、`hw_encoder_used`、`process_time_ms` 字段
    - 修改 `list_vault_records` 命令从 AppState 中获取 SQLite 连接并调用真实查询
    - _Requirements: 9.2, 9.3_

  - [ ]* 8.3 编写版权金库存取 Round-trip 属性测试
    - **Property 10: 版权金库存取 Round-trip 与排序**
    - **Validates: Requirements 9.2, 9.3**

- [x] 9. 检查点 — 确保水印模块和数据层编译通过且测试通过
  - 确保所有测试通过，如有问题请向用户确认。

- [x] 10. 图片 LSB 隐写水印模块
  - [x] 10.1 创建 `src-tauri/src/pipeline/image_watermark.rs`
    - 实现 `embed_image_watermark()` 函数：使用 image crate 读取图片，在像素 LSB 中嵌入 WatermarkPayload
    - 实现 `extract_image_watermark()` 函数：从图片像素 LSB 提取水印数据
    - 更新 `pipeline/mod.rs` 导出新模块
    - _Requirements: 12.3_

- [x] 11. 系统防护模块：磁盘空间预检与休眠抑制
  - [x] 11.1 创建 `src-tauri/src/pipeline/system_guard.rs`
    - 实现 `check_disk_space()` 函数：根据源文件大小 × 平台数 × 1.5 + 500MB 估算所需空间，与可用空间比较
    - 实现 `SleepInhibitor` RAII guard：Windows 调用 SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)，macOS 调用 IOPMAssertionCreateWithName
    - Drop 时自动释放休眠抑制
    - 更新 `pipeline/mod.rs` 导出新模块
    - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5_

- [x] 12. 压制流水线调度器
  - [x] 12.1 创建 `src-tauri/src/pipeline/scheduler.rs`
    - 实现文件类型分发逻辑：根据扩展名判断视频/图片/音频，路由到对应处理链路
    - 视频链路：磁盘空间预检 → 休眠抑制 → 音频抽取 → 水印注入 → 多平台并行压制 → 释放休眠抑制
    - 图片链路：LSB 隐写水印嵌入 → 输出带水印图片
    - 音频链路：PCM 转换 → FFT 水印嵌入 → 输出带水印音频
    - 实现实时进度解析：从 FFmpeg stderr 解析 time= 字段，通过 Tauri Event 发送 PipelineProgressPayload
    - 实现 FFmpeg 失败降级重试：硬件编码失败时自动切换 CPU 软编码重试一次
    - 实现任务取消：kill FFmpeg 子进程，清理临时文件，释放休眠抑制
    - 更新 `pipeline/mod.rs` 导出新模块
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.6, 8.7, 12.1, 12.2, 12.4, 14.4, 14.5_

  - [x] 12.2 重写 `src-tauri/src/commands/transcode.rs`
    - 修改 `start_pipeline` 命令：调用 scheduler 启动真实流水线（替代模拟进度），支持视频/图片/音频三种文件类型
    - 修改 `cancel_pipeline` 命令：通过 scheduler 终止 FFmpeg 子进程
    - 修改 `get_hw_info` 命令：从 AppState 缓存读取真实硬件探测结果，包含 FFmpeg 下载状态
    - 压制完成后调用 db 层插入 VaultRecord（含 file_type 字段）
    - 输出文件命名使用 `utils/fs::output_file_name`
    - _Requirements: 4.6, 8.1, 8.2, 8.5, 9.2, 12.6_

  - [x] 12.3 重写 `src-tauri/src/commands/probe.rs`
    - 修改 `probe_source` 命令：根据文件扩展名分发——视频调用 ffprobe，图片使用 image crate 读取尺寸，音频调用 ffprobe
    - 使用 `utils/hash::sha256_of_file` 计算文件真实 SHA-256
    - 使用 tonemap 模块的 HDR 判断函数（基于 ffprobe 色彩空间数据）
    - 返回的 SourceMeta 新增 file_type 字段
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 12.1, 12.2, 12.5_

- [x] 13. 维权取证真实实现
  - [x] 13.1 重写 `src-tauri/src/commands/verify.rs`
    - 修改 `verify_suspect` 命令：根据文件类型分发——视频/音频调用 FFmpeg 抽取音频 → watermark::extract_watermark，图片调用 image_watermark::extract_image_watermark
    - 实现置信度阈值逻辑：>= 0.95 返回 matched=true，0.5~0.95 返回"置信度不足"，< 0.5 返回"未检测到"
    - 所有返回结果包含 disclaimer 法律免责声明字段
    - 在 SQLite 中按 watermark_uid 匹配
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 15.1, 15.2, 15.3, 15.4_

- [x] 14. 检查点 — 确保后端所有模块编译通过且测试通过
  - 确保所有测试通过，如有问题请向用户确认。

- [x] 15. 前端集成：DropZone 真实路径获取
  - [x] 15.1 改造 `src/components/DropZone.vue`
    - 使用 `@tauri-apps/plugin-dialog` 的 `open()` API 替代浏览器 `<input type="file">`
    - 使用 Tauri 拖拽事件获取完整磁盘路径（替代 `file.name`）
    - 将完整磁盘路径传递给 `probe_source` IPC 命令
    - 根据文件扩展名在 UI 上区分显示文件类型图标（视频/图片/音频）
    - 保留浏览器模式下的 fallback 逻辑
    - _Requirements: 3.1, 3.2, 3.3, 12.5_

- [x] 16. 前端集成：进度面板对接真实事件与数据模型同步
  - [x] 16.1 更新 `src/lib/tauri-api.ts` 中的接口
    - VaultRecord：将 `platforms: Platform[]` 替换为 `outputDouyin/outputBilibili/outputXhs`，新增 `fileType`、`hwEncoderUsed`、`processTimeMs` 字段
    - VerificationResult：新增 `disclaimer: string` 字段
    - 同步更新 mock 数据
    - _Requirements: 9.2, 9.3, 12.6, 15.4_

  - [x] 16.2 移除 `src/views/WorkbenchView.vue` 中的模拟进度逻辑
    - 删除 `simulateProgress()` 函数
    - 修改 `handleStart()` 函数：调用 `startPipeline` 后完全依赖后端 `pipeline-progress` 事件驱动进度
    - 确保 `listenPipelineProgress` 回调正确更新 progress reactive 对象
    - _Requirements: 16.1, 16.2, 16.3_

  - [x] 16.3 更新 `src/views/VaultView.vue` 适配新的 VaultRecord 结构
    - 将平台展示逻辑从 `platforms` 数组改为读取 `outputDouyin`/`outputBilibili`/`outputXhs` 字段
    - 显示文件类型标签（视频/图片/音频）
    - _Requirements: 9.3, 12.6_

  - [x] 16.4 更新 `src/views/VerifyView.vue` 添加免责声明展示
    - 在取证结果区域底部显示 disclaimer 免责声明文本
    - 根据置信度分级显示不同的结果提示
    - _Requirements: 15.4, 15.5_

- [x] 17. 最终检查点 — 全量编译与测试验证
  - 确保所有 Rust 测试通过（`cargo test`）
  - 确保前端 TypeScript 类型检查通过
  - 如有问题请向用户确认。

## Notes

- 标记 `*` 的子任务为可选属性测试任务，可跳过以加速 MVP 交付
- 每个任务引用了对应的需求编号以确保可追溯性
- 属性测试验证设计文档中定义的 10 个正确性属性
- 检查点任务确保增量验证，避免错误累积
- 后端使用 Rust，前端使用 TypeScript + Vue 3，与现有代码栈一致
- **发布收口**：FFmpeg 不打包进安装包，生产版仅接受系统 PATH 中预装的受控版本
- **杀软白名单**：需求 17（EV 代码签名）为发布流程任务，不涉及代码实现
- **法律免责**：取证结果强制附带免责声明，置信度阈值 0.95 防止误判
