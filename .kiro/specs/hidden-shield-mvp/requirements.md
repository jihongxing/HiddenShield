# 需求文档：隐盾 (HiddenShield) MVP V1.0

## 简介

隐盾是一款基于 Tauri 2 + Rust + Vue 3 + FFmpeg 的桌面端多媒体（视频/图片/音频）压制与版权保护工具。产品定位为"创作者发布前的最后一站"，将多平台极致画质压制作为高频入口，将盲水印和版权资产管理作为隐藏价值无感沉淀。

当前项目已有完整的 UI 骨架和 Rust 后端占位实现，所有核心功能均为假数据和模拟逻辑。本需求文档定义 MVP V1.0 的完整功能范围：将所有 stub 替换为真实实现，同时覆盖 GPL 合规性、系统稳定性和法律风险防控等关键工程约束。

参考文档：
- `docs/隐盾（HiddenShield）版权保护SaaS平台.md` — 产品需求文档
- `docs/FFmpeg多平台压制参数方案.md` — FFmpeg 压制参数与硬件加速方案
- `docs/技术架构方案.md` — 技术架构与数据流设计

## 术语表

- **Pipeline（压制流水线）**: 从源文件输入到多平台输出文件生成的完整异步处理链路，包含探测、水印注入、编码压制等阶段
- **FFmpeg**: 开源音视频处理工具，本项目通过 Rust 调用其命令行二进制进行视频编解码
- **ffprobe**: FFmpeg 附带的媒体信息探测工具，用于获取视频元数据（分辨率、帧率、色彩空间等）
- **SourceMeta（源文件元数据）**: 通过 ffprobe 获取的视频属性集合，包括分辨率、帧率、时长、色彩空间、文件大小等
- **HDR（高动态范围）**: 指 BT.2020 / PQ / HLG 色彩空间的视频，常见于 iPhone 12+ 录制的 Dolby Vision 视频
- **Tone-mapping（色调映射）**: 将 HDR 视频的宽色域/高亮度信息映射到 SDR 色彩空间的过程，解决 HDR 视频上传平台后发灰的问题
- **盲水印（Blind Watermark）**: 嵌入音频频域的不可感知水印，不影响听觉体验，可通过 FFT 逆向提取
- **FFT（快速傅里叶变换）**: 将时域音频信号转换为频域表示的算法，用于水印嵌入和提取
- **扩频水印（Spread Spectrum Watermarking）**: 在音频中高频段（2kHz-8kHz）微调幅度编码水印比特的技术
- **版权金库（Vault）**: 本地 SQLite 数据库中存储的版权记录集合，包含文件哈希、水印 UID、时间戳等
- **VaultRecord（版权记录）**: 版权金库中的单条记录，关联源文件指纹与水印标识
- **硬件编码器（HW Encoder）**: GPU 提供的视频编码加速能力，包括 NVENC（NVIDIA）、VideoToolbox（macOS）、QSV（Intel）、AMF（AMD）
- **Platform（目标平台）**: 压制输出的目标社交媒体平台，MVP 支持抖音、B站、小红书三个平台
- **IPC（进程间通信）**: Tauri 框架中前端 Vue 与后端 Rust 之间的命令调用机制
- **PCM（脉冲编码调制）**: 未压缩的数字音频格式，用于 FFT 水印处理的中间格式
- **CRF（恒定质量因子）**: FFmpeg 中控制视频编码质量的参数，数值越低质量越高
- **SHA-256**: 密码学哈希算法，用于生成源文件的唯一指纹

## 需求

### 需求 1：FFmpeg 与 ffprobe 可用性保障

**用户故事：** 作为创作者，我希望应用能准确识别当前机器是否已具备可用的 FFmpeg/ffprobe 运行环境，以便所有音视频处理功能正常工作。

#### 验收标准

1. WHEN 应用启动时, THE Pipeline SHALL 检查系统 PATH 中是否存在 `ffmpeg` 与 `ffprobe`
2. WHEN 系统 PATH 中存在 ffmpeg 和 ffprobe 且探测命令执行成功时, THE Pipeline SHALL 缓存二进制路径并标记 FFmpeg 状态为"可用"
3. IF ffmpeg 或 ffprobe 不存在或探测命令执行失败, THEN THE Pipeline SHALL 阻止依赖 FFmpeg 的任务启动并提示用户按发布说明完成预装
4. THE Pipeline SHALL 在应用生命周期内仅执行一次 FFmpeg 可用性检测，并将结果缓存供后续使用

### 需求 2：源视频元数据真实探测

**用户故事：** 作为创作者，我希望拖入视频后能看到真实的视频属性（分辨率、帧率、时长、色彩空间），以便了解源素材的质量。

#### 验收标准

1. WHEN 用户选择或拖入一个视频文件时, THE Pipeline SHALL 调用 ffprobe 获取视频流的分辨率（宽×高）、帧率、时长（秒）、像素格式和色彩空间信息
2. WHEN ffprobe 返回视频流的 color_transfer 为 "smpte2084" 或 "arib-std-b67"，或 color_primaries 为 "bt2020" 时, THE Pipeline SHALL 将 SourceMeta 的 is_hdr 字段标记为 true
3. WHEN 用户选择或拖入一个视频文件时, THE Pipeline SHALL 读取文件的完整二进制内容并计算 SHA-256 哈希值作为文件指纹
4. WHEN ffprobe 执行成功时, THE Pipeline SHALL 将探测结果封装为 SourceMeta 结构体并通过 IPC 返回给前端
5. IF ffprobe 执行失败或返回无法解析的数据, THEN THE Pipeline SHALL 返回包含错误描述的 Err 结果

### 需求 3：文件拖拽获取真实路径

**用户故事：** 作为创作者，我希望拖入文件后系统能获取到文件的真实磁盘路径，以便后端能直接读取和处理该文件。

#### 验收标准

1. WHEN 用户通过拖拽方式导入文件时, THE DropZone 组件 SHALL 通过 Tauri 的拖拽 API 获取文件的完整磁盘路径（而非仅文件名）
2. WHEN 用户通过文件选择对话框导入文件时, THE DropZone 组件 SHALL 通过 Tauri 的文件对话框 API 获取文件的完整磁盘路径
3. THE DropZone 组件 SHALL 将获取到的完整磁盘路径传递给 probe_source IPC 命令

### 需求 4：硬件编码器真实探测

**用户故事：** 作为创作者，我希望应用能自动检测我的电脑是否支持 GPU 硬件加速编码，以便获得最快的压制速度。

#### 验收标准

1. WHEN 应用启动时, THE Pipeline SHALL 通过执行 FFmpeg 编码器初始化测试来探测可用的硬件编码器
2. WHEN 运行在 macOS 系统上时, THE Pipeline SHALL 测试 h264_videotoolbox 和 hevc_videotoolbox 编码器的可用性
3. WHEN 运行在 Windows 或 Linux 系统上时, THE Pipeline SHALL 按 NVENC → QSV → AMF 的优先级顺序测试硬件编码器
4. IF 所有硬件编码器测试均失败, THEN THE Pipeline SHALL 回退到软件编码器（libx264 / libx265）
5. THE Pipeline SHALL 在应用生命周期内仅执行一次硬件探测，并将结果缓存供后续使用
6. THE Pipeline SHALL 通过 get_hw_info IPC 命令将探测结果返回给前端展示

### 需求 5：多平台压制参数引擎

**用户故事：** 作为创作者，我希望系统能根据目标平台自动生成最优的 FFmpeg 压制参数，以便上传后获得最高画质。

#### 验收标准

1. WHEN 目标平台为抖音时, THE Presets_Engine SHALL 生成 1080×1920（9:16 竖屏）、H.264 High Profile、CRF 18、最大码率 12Mbps、30fps 的 FFmpeg 参数
2. WHEN 目标平台为 B 站时, THE Presets_Engine SHALL 生成 1920×1080（16:9 横屏）、HEVC Main Profile、CRF 20、最大码率 16Mbps 的 FFmpeg 参数，并根据源视频帧率动态选择 30fps 或 60fps
3. WHEN 目标平台为小红书时, THE Presets_Engine SHALL 生成 1080×1440（3:4 竖屏）、H.264 High Profile、CRF 17、最大码率 15Mbps、30fps 的 FFmpeg 参数
4. WHEN 源视频宽高比与目标平台不匹配且用户选择 letterbox 策略时, THE Presets_Engine SHALL 在 video_filter 中添加 scale + pad 滤镜以黑边填充保持完整画面
5. WHEN 检测到可用的硬件编码器且用户选择 fast_gpu 模式时, THE Presets_Engine SHALL 将软件编码器替换为对应的硬件编码器，并将 CRF 模式替换为 VBR 码率控制模式
6. THE Presets_Engine SHALL 为所有平台统一生成 AAC 192kbps / 44.1kHz / 立体声的音频参数和 -movflags +faststart 容器参数

### 需求 6：iPhone HDR 自动色调映射

**用户故事：** 作为 iPhone 用户，我希望 HDR 视频上传到各平台后不会发灰，系统能自动修复色彩。

#### 验收标准

1. WHEN ffprobe 探测到源视频为 HDR 色彩空间时, THE Tonemap_Module SHALL 在 FFmpeg 滤镜链前端插入 zscale + tonemap=hable 色调映射滤镜
2. WHEN 源视频为 HDR 且需要色调映射时, THE Tonemap_Module SHALL 生成的滤镜链包含 zscale=t=linear:npl=100 → format=gbrpf32le → zscale=p=bt709 → tonemap=hable:desat=0 → zscale=t=bt709:m=bt709:r=tv → format=yuv420p 的完整转换序列
3. WHEN 源视频为 SDR 色彩空间时, THE Tonemap_Module SHALL 跳过色调映射滤镜，直接使用缩放/填充滤镜
4. WHEN HDR 色调映射成功应用时, THE Pipeline SHALL 通过进度事件向前端发送"检测到 iPhone HDR 视频，正在优化色彩..."的阶段提示

### 需求 7：音频盲水印嵌入

**用户故事：** 作为创作者，我希望每次压制时系统自动在音频中嵌入不可感知的版权水印，以便日后维权取证。

#### 验收标准

1. WHEN 压制流水线启动时, THE Pipeline SHALL 调用 FFmpeg 将源视频的音频流抽取为 PCM 16-bit / 44.1kHz / 立体声的 WAV 文件
2. WHEN PCM 音频抽取完成后, THE Watermark_Module SHALL 使用 realfft 库对音频按 4096 采样点分帧执行 FFT 变换
3. WHEN 对每个音频帧执行 FFT 后, THE Watermark_Module SHALL 在 2kHz-8kHz 频段（FFT bin 索引约 180-740）以 0.02 的嵌入强度编码水印比特
4. WHEN 音频帧的均方能量低于 0.001 阈值时, THE Watermark_Module SHALL 跳过该静音帧不嵌入水印
5. THE Watermark_Module SHALL 嵌入固定 256 bit（32 字节）的水印数据，包含 4 字节魔数、8 字节 Unix 时间戳、16 字节机器指纹 SHA-256 前缀和 4 字节 CRC32 校验
6. WHEN 水印嵌入完成后, THE Watermark_Module SHALL 执行 IFFT 还原时域信号并输出为 WAV 文件

### 需求 8：压制流水线真实执行

**用户故事：** 作为创作者，我希望点击"开始压制"后系统能真正调用 FFmpeg 完成视频压制，而非模拟进度。

#### 验收标准

1. WHEN 用户点击"开始压制"时, THE Pipeline SHALL 按照"音频抽取 → 水印注入 → 多平台并行压制"的顺序执行真实处理流水线
2. WHEN 水印音频准备就绪后, THE Pipeline SHALL 为每个选中的目标平台启动独立的 FFmpeg 子进程，使用 -map 0:v:0 -map 1:a:0 双输入源模式（原视频画面 + 水印音频）
3. WHILE FFmpeg 子进程运行中, THE Pipeline SHALL 实时解析 stderr 输出中的 time= 字段，结合已知总时长计算压制百分比
4. WHILE FFmpeg 子进程运行中, THE Pipeline SHALL 通过 Tauri Event 机制向前端发送 PipelineProgressPayload 事件，包含当前阶段描述、总体百分比和各平台独立百分比
5. WHEN 所有平台的 FFmpeg 子进程执行完成时, THE Pipeline SHALL 将输出文件命名为 "{源文件名}_{平台名}优化版.mp4" 并存放在源文件所在目录
6. IF 任一 FFmpeg 子进程异常退出, THEN THE Pipeline SHALL 记录错误信息并尝试降级为 CPU 软编码重试一次
7. WHEN 用户点击"取消任务"时, THE Pipeline SHALL 终止所有正在运行的 FFmpeg 子进程并清理临时文件

### 需求 9：本地版权金库（SQLite）

**用户故事：** 作为创作者，我希望每次压制完成后系统自动记录版权信息到本地数据库，形成我的版权资产账本。

#### 验收标准

1. WHEN 应用首次启动时, THE Database SHALL 在应用数据目录下创建 SQLite 数据库文件并执行 vault_records 表的建表语句
2. WHEN 压制流水线成功完成时, THE Database SHALL 向 vault_records 表插入一条记录，包含源文件 SHA-256 哈希、文件名、创建时间戳、水印 UID、视频时长、分辨率、各平台输出路径、HDR 标记、使用的编码器和处理耗时
3. WHEN 前端调用 list_vault_records IPC 命令时, THE Database SHALL 从 SQLite 查询所有版权记录并按创建时间倒序返回
4. THE Database SHALL 使用 rusqlite 库（bundled 特性）管理 SQLite 连接，并在 Tauri 的 AppState 中维护连接池

### 需求 10：维权取证水印提取与比对

**用户故事：** 作为创作者，我希望能将疑似侵权视频拖入系统，提取其中的盲水印并与本地版权金库比对，确认是否为我的作品。

#### 验收标准

1. WHEN 用户在维权取证页面提交疑似侵权文件时, THE Verify_Module SHALL 调用 FFmpeg 抽取该文件的音频为 PCM WAV 格式
2. WHEN PCM 音频抽取完成后, THE Verify_Module SHALL 使用与嵌入相同的 FFT 参数（4096 帧长、2kHz-8kHz 频段）执行逆向水印提取
3. WHEN 水印比特序列提取完成后, THE Verify_Module SHALL 验证 4 字节魔数是否匹配，并使用 CRC32 校验数据完整性
4. WHEN 水印数据通过校验后, THE Verify_Module SHALL 解码时间戳和机器指纹，并在本地 SQLite 版权金库中按 watermark_uid 进行精确匹配
5. WHEN 匹配成功时, THE Verify_Module SHALL 返回 VerificationResult，包含 matched=true、水印 UID、置信度、匹配到的 VaultRecord 和摘要描述
6. IF 水印提取失败或魔数/CRC 校验不通过, THEN THE Verify_Module SHALL 返回 matched=false 并在 summary 中说明可能原因（深度篡改/非本机加密的文件）

### 需求 11：Cargo 依赖补全

**用户故事：** 作为开发者，我需要项目的 Cargo.toml 包含所有必要的依赖库，以便所有核心功能模块能正常编译和运行。

#### 验收标准

1. THE Cargo_Config SHALL 包含 realfft 依赖用于音频 FFT 水印处理
2. THE Cargo_Config SHALL 包含 hound 依赖用于 WAV 文件读写
3. THE Cargo_Config SHALL 包含 rusqlite 依赖（启用 bundled 特性）用于本地 SQLite 数据库操作
4. THE Cargo_Config SHALL 包含 crc32fast 依赖用于水印数据的 CRC32 校验
5. THE Cargo_Config SHALL 为 tokio 依赖启用 full 特性以支持异步进程管理和文件 I/O

### 需求 12：图片与音频文件处理支持

**用户故事：** 作为创作者，我不仅需要处理视频，还希望能对图片和音频文件进行处理和加水印保护。

#### 验收标准

1. WHEN 用户拖入图片文件（.jpg, .png）时, THE Pipeline SHALL 识别文件类型为图片，跳过视频压制流程，仅执行图片元数据读取和版权记录
2. WHEN 用户拖入音频文件（.wav, .mp3）时, THE Pipeline SHALL 识别文件类型为音频，直接执行音频盲水印嵌入流程（无需视频压制）
3. WHEN 处理图片文件时, THE Pipeline SHALL 使用 LSB（最低有效位）隐写术在图片像素中嵌入水印数据，输出带水印的图片文件
4. WHEN 处理音频文件时, THE Pipeline SHALL 复用视频流水线中的音频盲水印模块（FFT 频域嵌入），输出带水印的音频文件
5. THE DropZone 组件 SHALL 根据文件扩展名在 UI 上区分显示文件类型（视频/图片/音频），并相应调整可用的处理选项
6. WHEN 图片或音频处理完成时, THE Database SHALL 向版权金库插入记录，file_type 字段标记为 "image" 或 "audio"

### 需求 13：FFmpeg 发布合规与供应链控制

**用户故事：** 作为产品方，我需要确保 FFmpeg 的 GPL 传染性协议不会污染隐盾的闭源商业代码。

#### 验收标准

1. THE Installer SHALL 不包含任何 FFmpeg/ffprobe 二进制文件，安装包体积保持在 ~15MB
2. THE Production_Runtime SHALL 禁用运行时下载 FFmpeg/ffprobe 的能力
3. THE Pipeline SHALL 仅信任系统 PATH 中由运维或用户预装的 FFmpeg/ffprobe 可执行文件
4. WHEN FFmpeg 运行环境不可用时, THE Pipeline SHALL 给出明确的修复提示，而不是尝试后台补齐未知二进制
5. 发布文档和运维脚本 SHALL 明确要求目标机器预装受控版本的 FFmpeg/ffprobe
6. IF 企业环境需要进一步收口, THEN 运维流程 SHOULD 通过软件分发、镜像基线或白名单机制统一 FFmpeg 版本

### 需求 14：磁盘空间预检与系统休眠抑制

**用户故事：** 作为创作者，我希望压制大文件时不会因为磁盘空间不足而崩溃，也不会因为电脑休眠而中断任务。

#### 验收标准

1. WHEN 用户点击"开始压制"时, THE Pipeline SHALL 在启动 FFmpeg 之前检查输出目录所在磁盘的可用空间
2. THE Pipeline SHALL 根据源文件大小和选中平台数量估算所需空间（公式：源文件大小 × 选中平台数 × 1.5 + 500MB 临时文件余量）
3. IF 可用磁盘空间小于估算所需空间, THEN THE Pipeline SHALL 阻止压制启动并向前端返回错误提示"磁盘空间不足，预计需要 {X}GB，当前可用 {Y}GB，请清理磁盘后重试"
4. WHEN 压制流水线启动时, THE Pipeline SHALL 调用系统 API 阻止系统进入休眠/睡眠模式（Windows: SetThreadExecutionState, macOS: IOPMAssertionCreateWithName）
5. WHEN 压制流水线完成或被取消时, THE Pipeline SHALL 释放休眠抑制，恢复系统正常电源管理

### 需求 15：维权取证置信度阈值与法律免责声明

**用户故事：** 作为产品方，我需要确保维权取证功能不会因误判而给用户和公司带来法律风险。

#### 验收标准

1. THE Verify_Module SHALL 设置最低置信度阈值为 0.95（95%），仅当提取的水印数据通过魔数校验、CRC32 校验且置信度 >= 0.95 时才返回 matched=true
2. WHEN 置信度介于 0.5 和 0.95 之间时, THE Verify_Module SHALL 返回 matched=false 并在 summary 中说明"检测到疑似水印特征但置信度不足，无法确认匹配"
3. WHEN 置信度低于 0.5 时, THE Verify_Module SHALL 返回 matched=false 并在 summary 中说明"未检测到有效水印"
4. THE VerificationResult 结构体 SHALL 包含 disclaimer 字段，内容为固定的法律免责声明文本："本报告仅基于既定算法进行特征码技术提取，仅供参考，不代表任何司法鉴定意见。平台不对因本报告引发的连带法律责任负责。"
5. WHEN 前端展示取证结果时, THE VerifyView SHALL 在结果区域底部显示免责声明文本

### 需求 16：前端进度面板对接真实事件

**用户故事：** 作为创作者，我希望压制过程中看到的进度信息是真实的处理状态，而非模拟动画。

#### 验收标准

1. WHEN 后端通过 Tauri Event 发送 pipeline-progress 事件时, THE ProgressPanel SHALL 实时更新总体进度百分比和阶段描述文案
2. WHEN 后端发送包含各平台独立百分比的进度事件时, THE ProgressPanel SHALL 分别更新抖音、B站、小红书三个平台的独立进度条
3. WHEN 压制流水线完成（percent=100）时, THE WorkbenchView SHALL 移除前端模拟进度逻辑（simulateProgress 函数），完全依赖后端真实事件驱动进度展示

### 需求 17：EV 代码签名与杀软白名单（发布流程）

**用户故事：** 作为产品方，我需要确保 Windows 用户下载安装隐盾时不会被杀毒软件拦截或弹出"未知发布者"警告。

#### 验收标准

1. THE Build_Pipeline SHALL 使用 EV 代码签名证书对 Windows 安装包（.exe / .msi）进行数字签名
2. BEFORE 正式发布, THE Team SHALL 将签名后的安装包提交至 360 安全卫士、腾讯电脑管家、Windows Defender 的开发者误报申诉平台进行白名单认证
3. THE Tauri 配置 SHALL 在 tauri.conf.json 中配置 Windows 签名相关字段（certificateThumbprint、digestAlgorithm 等）
