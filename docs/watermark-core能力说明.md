# watermark-core 能力说明

更新时间：2026-07-21

## 2026-07-31 宣传性能专项基准 v1

- 新增 `promo_performance_bench` Release runner 和确定性素材生成脚本，固定测试 `5` 张 `10.79–10.80 MiB / 4000×3000 / 12 MP JPEG` 与 `5` 首 `18.83–19.69 MiB / 180 秒 / 44.1 kHz / stereo / 16-bit FLAC`。
- 每份素材预热 `1` 次、正式测量 `5` 次，共 `25` 次/操作；所有写入均完成立即回读并恢复同一 UID。
- 图片写入从原始 JPEG bytes 调用 `WatermarkService::embed`，包含 JPEG 解码与 PNG 保护副本编码，平均 `985.73 ms`、p95 `1191.40 ms`；保护副本读取平均 `49.00 ms`、p95 `56.65 ms`。
- 音频写入总计包含 FFmpeg FLAC→WAV 准备和 `WatermarkService::embed`，平均 `2284.72 ms`、p95 `2387.16 ms`；其中准备平均 `134.18 ms`、核心写入平均 `2150.54 ms`。WAV 保护副本读取平均 `221.83 ms`、p95 `252.12 ms`。
- 外部暴露边界：宣传图可写“约 10 MB JPEG 写入约 0.99 秒 / 读取约 0.05 秒”和“约 20 MB、3 分钟 FLAC 写入约 2.28 秒 / 读取约 0.22 秒”，但必须同时显示固定 Windows Release 标准化素材平均值和非 SLA 提示。
- 素材由确定性滤镜生成，只用于固定性能桶，不能表述为真实用户照片或真实歌曲；不能外推到任意相同文件大小、其他像素量、时长、设备或完整桌面 UI 等待时间。
- 当前性能快照设备：AMD Ryzen 5 4500U，6 核 / 6 线程，15.37 GiB RAM，Windows 11，Samsung SATA SSD。
- 回滚路径：如果后续 core、解码策略或输出容器变化导致均值或 p95 漂移，宣传图必须回退到不展示具体耗时，直至使用同 schema 重跑并更新 `docs/宣传性能专项基准v1.md`。

下一核心性能任务：

- 为 runner 增加可选真实用户素材模式，在不提交原始媒体的前提下输出匿名规格和聚合统计；标准化素材桶继续作为可复现回归基线。

## 2026-07-21 性能与音频采样率复核

- 先前长时间矩阵的主要性能误差来自执行配置：默认 `watermark:matrix` 与临时基线程序都使用 Rust `dev` 构建。相同 1920×1080 JPEG 在 `dev` 中写入 / 读取为 `147383 ms / 286 ms`，在 `release` 中为 `9659 ms / 13 ms`；相同 30 秒 mono WAV 的写入 / 读取从 `28948 ms / 51921 ms` 降为 `631 ms / 1126 ms`。
- `package.json` 的 `watermark:matrix` 已改为 `cargo run --release`；新增 `watermark:baseline:real-file` 也固定使用 `--release`。这只修正测试执行配置，不改变核心算法。
- 桌面端确实调用同一 `watermark-core`，但单独音频管线当前保留源采样率；核心音频频带使用 `44.1 kHz` canonical rate。对同一类 31 秒控制音频，`44.1 kHz` WAV / MP3 的 mono、stereo 共 `6 / 6` 写后回读通过，而 `48 kHz` 对照组 `0 / 6` 通过。
- 桌面正式 `sine_31s` 夹具的 FLAC / MP3 / OGG / M4A 均为 `44.1 kHz` mono，release 基线 `4 / 4` 通过；这解释了“桌面端能解析 MP3”与 48 kHz 真实矩阵失败并不矛盾：容器解析成功不等于当前采样率下水印可回读。
- 根因已修复：默认 V3 WAV 写入此前在实际采样率以外错误使用 `44.1 kHz` 频带；现已将源 WAV 的实际采样率传入 V3 频带和质量规划，写回仍沿用原 `WavSpec`，不改变采样率或声道。
- 修复后，30 秒、48 kHz 的 WAV / MP3 / FLAC / OGG / M4A，mono / stereo 共 `10 / 10` 立即写读通过，输出均保持 `48 kHz` 和原声道；31 秒 48 kHz 的粉噪 / 纯音 WAV / MP3 控制组 `6 / 6` 通过。
- 新增 48 kHz mono / stereo 核心回归，直接断言 V3 写后回读成功且输出 WAV 规格不变。48 kHz 仍需完成扰动和跨端 Gate，当前不能据此承诺任意变换后的可读性。
- `robustness_bench` 已移除固定 `44.1 kHz / mono` 转换，所有音频扰动均显式保持源采样率和声道；每个输出在提取前读取 WAV 头并断言规格不变，报告记录 source/output SR、channel 和 `specPreserved`。
- `robustness_bench` 音频读取已从 rollback-only `extract_v2` 切换到当前默认 V3 `WatermarkService::extract`。
- 30 秒、48 kHz、五种容器 × mono/stereo 的完整 release 扰动矩阵共 `240` 条：`106` 通过、`134` 失败、规格变化 `0` 条。基线、WAV 重编码、音量 80%、音量 120%、MP3 192k 往返共 `50 / 50` 通过。
- 剩余失败全部位于 5–15 秒短裁剪及其位置 / MP3 往返组合；短裁剪低于 standalone audio 的 30 秒保护输入门槛，当前不能承诺任意短片段均可取回水印。
- 证据：`tmp-ui-qa/watermark-real-file-matrix/20260721/perf-differential-release.json`、`baseline-desktop-fixture-audio-release.json`、`baseline-30s-48k-format-channel-after-rate-fix.json`、`baseline-31s-48k-after-rate-fix.json`、`perturbation-medium-audio-48k-preserve-spec-summary.json`。

## 2026-07-21 广泛采样率 / 声道基线边界

- 在 31 秒、原规格保持的 WAV 控制矩阵中测试了 `4 / 8 / 11.025 / 16 / 22.05 / 32 / 44.1 / 48 / 88.2 / 96 / 192 kHz`，并覆盖 mono 及 2 / 3 / 4 / 5 / 6 / 7 / 8 声道的代表组合。
- 共 `34` 组：`22` 组写后回读通过，`12` 组在嵌入阶段因核心容量不足而拒绝；已生成输出的规格变化为 `0`，另有 `12` 组因未生成输出而不适用规格断言。
- 通过范围：当前控制矩阵中的 `8–48 kHz` 及 `1–8` 声道组合均通过，包括 48 kHz 的 1–8 声道；这不是对所有声道布局和所有采样率的数学证明。
- `4 kHz` 失败原因是 31 秒样本数不足以容纳 V3 recovery packet；`88.2 / 96 / 192 kHz` 失败原因是固定 4096 FFT 与当前 2–8 kHz 频带组合产生的可用频对不足。
- 外部暴露边界：当前对用户承诺为常见的 `8–48 kHz` 与 `mono / stereo`，且输出保持原始采样率和声道不变；`4–8` 声道及 `48 kHz` 以上采样率属于后续兼容性扩展范围。
- 当前不承诺低于 `8 kHz`、高于 `48 kHz`、超过 `2` 声道或短于单文件保护门槛的组合；后续扩展必须在 `watermark-core` 内设计独立容量策略，不能由端侧重采样或降声道规避。
- 输入预检：`validate_audio_protection_input` 在核心写入前执行 `8–48 kHz`、`mono / stereo`、最短 `30` 秒校验；`audio-support-v1` fixture 固化桌面端允许 / 拒绝边界及错误码，移动端当前冻结。
- 图片输入预检：`image_embed_capacity_sufficient` 按分块容量判断是否能容纳完整载荷，不使用固定 `1920×1080` 门槛；`320×240` 是已知失败参考，`1920×1080` 是成功参考，二者均不构成用户可见的最小尺寸承诺。
- 桌面音频 wrapper 会按 FFprobe 选择 `pcm_s16le`、`pcm_s24le`、`pcm_s32le` 或 `pcm_f32le` WAV；`watermark-core` 按输入 `WavSpec` 写回，保持采样率、声道、支持的 sample format 和 bit depth。`64-bit float` 明确拒绝，避免静默降精度；高位深质量与运行时 Gate 完成前仍不作为当前用户承诺。
- 证据：`tmp-ui-qa/watermark-real-file-matrix/20260721/arbitrary-spec-probe/summary.json`。

## 2026-07-21 图片 / 音频全量自检结果

- 真实文件矩阵已生成并由 FFprobe 验证可解码：图片 PNG / JPEG / WebP 各小 `320×240`、中 `1920×1080`、大 `4096×2160` 共 9 个；音频 WAV / MP3 / FLAC / OGG / M4A 各 8 / 30 / 90 秒及 mono / stereo 共 30 个。
- 已执行的小图矩阵显示 `320×240` 的 PNG / JPEG / WebP 在写后基线读取均失败，不能作为正式可保护的最小尺寸。
- 已执行的 8 秒 WAV 被核心按 `audio_protection_min_duration` 拒绝；30 秒、48 kHz、mono 粉红噪声 WAV 虽完成写入，但基线读取在所有 frame phase 失败。
- 4096×2160 完整图片扰动矩阵单样本耗时过长，已中止；MP3 / FLAC / OGG / M4A 水印矩阵尚未执行，不能宣称这些真实容器已通过。
- 部分矩阵证据：`tmp-ui-qa/watermark-real-file-matrix/20260721/partial-matrix-summary.json`。
- `watermark:quality-gate:full`：图片 8 / 8 写入、读取和质量通过；音频 6 / 6 可读回，但只有 5 / 6 通过质量阈值。
- 失败样本为 `field_recording_noise_floor`：提取成功，但 SNR `13.2013 dB`，低于发布阈值 `44 dB`；峰值、响度和新增削波未超阈值。
- `watermark:cross-end-release` 通过，说明冻结的跨端合同继续成立。
- `watermark:matrix` 因没有外部图片或音频源而失败，未形成真实文件格式、尺寸、时长或声道矩阵。
- 当前性能快照：本次图片 roundtrip 全部低于 25 秒上限；音频噪声底失败属于感知质量问题，不是水印读取失败。
- 外部暴露边界：不能承诺所有图片 / 音频格式、大小和声道均成功；PNG / JPEG / WebP 与非 WAV 路径目前主要由合同和既有 fixture 支撑。
- 新限制与回滚：噪声底音频在质量问题解决前不得进入“全部音频均可稳定保护”的用户承诺；算法仍保持当前 V3，不降低 SNR 发布阈值。
- 证据：`artifacts/watermark-self-check/20260721/summary.json`。

## 2026-07-16 发布暴露边界与 RC / GA Gate

本次不修改 `watermark-core` 的算法、payload、公有 API、fixture、benchmark 或性能参数，只调整产品暴露边界：

- 当前桌面发布只暴露图片 / 音频正式写入、读取和验证。
- 桌面必须补齐无网络环境下的图片 / 音频读取与验证 gate。
- L1 视频音轨、L2 视频指纹存证和 L3 视频画面候选全部退出当前桌面 UI、商业权益、正式报告和发布 Gate。
- 视频相关 core API 与候选算法保留为内部资产，不等于当前产品能力；后端 wrapper 也不得成为绕过桌面屏蔽的用户入口。
- 移动端开发冻结。已有移动端 wrapper、fixture 和跨端证据保留，但不再要求为当前桌面发布继续扩展。
- `RC Gate 待新映射复验`：旧 RC 安装版已在物理断网环境完成图片 / 音频 V3/39 功能验证及离线注册码生命周期验证；新商业映射不改算法，但新候选包仍需复验。
- `GA Gate 进行中`：仍需正式企业分发证书、干净 Windows 页面级证据和生产密钥托管。全新本地用户复跑只补强环境隔离证据，不改变算法能力或 GA 判定。
- 未付费 / 年费、批量、报告逐份付费和未来视频独立收费都属于 core 外商业包装；HSLIC1 不得通过 core 或桌面 wrapper 获得 `report_export`。

当前性能快照：

- 图片 / 音频性能沿用本文后续记录，本次没有重新调参或改变阈值。
- 视频历史 gate 结果继续作为内部研究证据，不进入当前 release decision。
- 2026-07-16 安装版真实 WLAN 断网 Gate：图片 / 音频 internal QA 与默认 V3/39 写读共 4 / 4 通过，全部读回同一 `watermarkUid` 且 `payloadAuthStatus=verified`；本轮只证明离线功能正确性，不替代感知质量与大样本性能门禁。

外部暴露边界：

- 可对用户承诺：桌面图片 / 音频；完成发布 gate 后的离线验证。
- 只能内部测试：所有视频路径、移动端路径和历史跨端路径。
- 明确不能承诺：当前桌面支持 L1 / L2 / L3 或任何形式的视频盲水印。

限制与回滚：

- 保留视频 API 和历史字段会继续产生维护成本，但避免删除历史数据或破坏未来研究。
- 如未来恢复视频能力，必须先更新能力边界与 Roadmap，再恢复 UI，并重新执行真正视频盲水印的鲁棒性、误报 / 漏报、性能和运行态 Gate；不能直接恢复 L1 / L2 为正式视频盲水印。

下一核心任务：

- 保持 V3/39 算法与 API 不变，重建桌面候选包并验证新 HSLIC1 映射只开放图片 / 音频批量；页面级证据由人工补齐，之后再在干净 Windows 环境完成 GA 复跑。

本文是 HiddenShield 全项目级的 `watermark-core` 能力口径。它面向产品、研发、测试、销售和支持，回答四个问题：

1. `watermark-core` 现在能做什么。
2. 现在对外实际暴露哪些能力。
3. 当前性能大概到什么位置。
4. 还需要继续记录什么，才能不把口径写飘。

`watermark-core` 是 HiddenShield 的共享盲水印算法核心，不是独立业务服务。它只负责正式写入、读取、验证、payload 编码、版权编号锚点和重写判断，版权库、云版权库、公开权利信号、C2PA 传播层、企业额度、支付和报告都在 core 外面。

## 1. 一句话定位

`watermark-core` 是 HiddenShield 图片、音频以及内部视频候选能力的单一算法事实源；当前产品只暴露图片和音频。

- 桌面端、原生移动端、后端云任务都只能调用它，不能各自再写一套盲水印算法。
- 默认正式路径已经切到 V3/39 最小锚点。
- V2/119 不再属于正式图片能力；图片写入、读取和验证只支持 V3/39。音频旧版回滚仅保留在隔离测试套件。
- 视频画面盲水印已有独立 release candidate gate，但不是当前用户可售主线。

## 2. 当前正式能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 图片盲水印写入与读取 | 正式可用 | 支持正式保护副本写入、回读和写入后验证。默认路径已切到 V3/39 最小锚点；非外部依赖 V3 payload release QA 已覆盖 PNG / JPEG / WebP / BMP 四类北极星正式图片格式。 |
| 音频盲水印写入与读取 | 正式可用 | 支持 30 秒及以上音频作品的正式保护副本写入、回读和验证。视频音轨走同一音频核心。 |
| 版权编号锚点 | 正式可用 | 读回同一 `watermarkUid`，并通过版本链和 registry 解释补全语义。 |
| V3 最小锚点 codec | 正式默认 | 只保留 `watermark_id + payloadProtocolVersion + auth_tag`，39 bytes。 |
| V2/119 回滚 | 图片已退役 / 音频隔离测试 | 图片调用 `force_v2_rollback`、`embed_v2` 或 `extract_v2` 必须返回 `v2_image_rollback_retired`；音频旧版回滚不进入默认 release suite。 |
| 只读候选 reader | 受控可用 | 用于迁移期、报告桥接和内部 QA，不是默认写入路径。 |
| 内部 QA 写入 gate | 受控可用 | 仅供内部 QA 写 V3/39，用于回滚矩阵和受控运行态验证。 |
| 视频音轨盲水印 | 内部保留 / 当前隐藏 | 复用音频核心的历史实现，当前不进入桌面 UI、报告、权益或发布 Gate。 |
| 视频画面盲水印 | release candidate / internal | 仍不开放为用户主功能；`watermark:l3-video-visual-release-gate` 已完成完整 24 个 2K FFmpeg 样本池，H.264 / HEVC 非风险样本均过线，H.264-RISK 正确记录风险边界。`l3_controlled_worker_fixture` 已作为受控 worker 最小闭环调用 core 生成策略、写入、自检并交给后端 trusted completion 固化收据；`l3_real_worker_first_pass` 已完成 `object://l3-upload/...` 对象上传读取、FFmpeg sandbox、registry-reserved UID 绑定到 core payload、claim / lease / replay protection、失败归因、最终 MP4 封装、packaged self-check、`object://l3-output/...` 输出引用、签名 MP4 字节下载和 worker receipt 持久审计闭环。运行记录见 `docs/L3视频画面盲水印release_gate_QA记录.md`。 |

## 3. 对外暴露的能力

这里说的“对外”，不是 `watermark-core` 自己开 HTTP 接口，而是它被上层产品、SDK 和脚本调用后，用户和第三方最终能看到什么。

### 3.1 直接暴露给产品包装层

- 图片正式写入和读取。
- 音频正式写入和读取。
- 视频音轨保护。
- 写入后验证。
- 保护副本互读。
- 版本链解释。
- 旧记录回滚和迁移桥接。

### 3.2 直接暴露给研发和 QA

- `encode_payload()` / `decode_payload()`
- `encode_payload_v3_minimal_anchor()` / `decode_payload_v3_minimal_anchor()`
- `decode_watermark_payload_readonly()`
- `WatermarkService::embed()` / `WatermarkService::extract()`
- `WatermarkService::embed_v2()` / `WatermarkService::extract_v2()`
- 图片 / 音频 / 视频音轨的固定 fixture、release gate、contract gate
- L3 视频画面 24 样本 release candidate gate：`watermark:l3-video-visual-release-gate`
- L3 受控 worker 最小闭环：`cloud-video:l3-worker-qa`
- L3 真实 worker first-pass + 队列执行模型 + 受控 MP4 输出封装：`cloud-video:l3-real-worker-first-pass-qa`
- 内部 QA 专用 V3 写入门禁

### 3.3 明确不属于 core 的能力

- 公开权利信号扫描结论。
- C2PA / CAWG / IPTC / XMP / JSON-LD 传播层。
- rights registry。
- 企业 API key、quota、限流、审计。
- 支付、订阅、团队空间、云同步账本。
- L2 视频指纹存证。

这些能力可以和 `watermark-core` 一起形成完整产品，但不是 core 本身。

## 4. 当前性能与质量

下面是当前仓库里最新可复现的性能快照。它们是 benchmark / gate 结果，不是 SLA。

### 4.1 图片

- `watermark:quality-gate:release` 已作为正式发布阻断门禁启用。
- 固定样本池已通过，覆盖自然渐变、低纹理、海报色块、UI / 文字截图。
- `watermark:quality-gate:full` 已新增为感知质量 full gate 入口，会扩展人像肤色、暗光噪声、高细节和小尺寸边界等确定性风险样本，并生成 ABX 人工盲测模板；它是无感测试证据入口，不是“绝对无感”承诺。
- `rights:v3-media-payload-release-qa` 已把图片 V3/39 写读从单 PNG 扩展到 PNG / JPEG / WebP / BMP 四类正式图片格式；TIFF 仍是候选输入，不进入当前正式矩阵。
- `src-tauri` 侧已把 L2 视频指纹本地生成容器白名单扩到 MP4 / MOV / WebM / AVI / MKV / M4V；L1 视频音轨 release gate 已覆盖 MP4 / MOV / AVI / MKV / M4V / WebM，WebM/Opus 成品音轨回读已接入 release QA。
- 过去的 17 张真实图片 release baseline 中，写入均值约 `258.2 ms/张`，提取均值约 `332.5 ms`。
- 当前 V3 fast gate 只看开发回归级指标，阈值是 PSNR `>= 33 dB`、SSIM `>= 0.985`，外加 roundtrip 时限。

### 4.2 音频

- 9 首真实音频 release baseline 已通过，常规场景写入均值约 `306.8 ms/首`，提取均值约 `30.5 ms`。
- 当前 release gate 只统计 30 秒以上作品的常规场景，短裁剪仍只是观察项。
- `watermark:quality-gate:full` 额外覆盖现场噪声、瞬态打击和人声 / 音乐混合等确定性风险样本，并生成耳机 / 外放 ABX 模板；正式“无感”表述仍需要人工 ABX 原始记录补证。
- 2026-07-02 第一轮音频 V3 强度收敛实验已完成：V3 recovery lane 先按 frame RMS、噪声型频谱和瞬态判定选择目标对比度，再对每个相对频带 pair 只做最小必要改写；readonly candidate 提取顺序已改为先尝试 V3 recovery，再回退 V2 legacy，避免 V3 样本误入 V2 tolerant 纠错长跑。
- `watermark:quality-gate:release -- --run-id codex-v3-audio-v3-first-release` 通过；`watermark:quality-gate:full -- --run-id codex-v3-audio-v3-first-full` 仍按预期 blocked，阻断项只剩 `field-noise` 的 `snr_below_threshold`，SNR `12.5383 dB`，但提取置信度 `1.000000`。`transient` 已从 baseline 的 `43.52 dB` 提升到 `54.9592 dB` 并通过 full gate。
- 2026-07-02 噪声底稀疏写入实验已完成：稳定噪声底 profile 触发 `noiseFloorSparseRecovery=true` 时，V3 recovery bit-slot 只写到 3-lane majority 所需的最少 pair，不改变 payload、版权编号或提取格式。`watermark:quality-gate:release -- --run-id codex-noise-sparse-final-release` 通过；`watermark:quality-gate:full -- --run-id codex-noise-sparse-final-full` 仍 blocked。`field-noise` 的 modified pair ratio 从约 `0.568` 降到 `0.282321`，extraction confidence 保持 `1.000000`，但 SNR 仅 `13.5115 dB`，仍不能解除 full gate 阻断。
- 2026-07-02 已新增只读感知诊断：release/full gate 音频与 L1 视频音轨报告现在输出 `metrics.perceptualDiagnosis`，包含 1 秒分段 SNR、低频 / watermark 频带 / 高频信号与差异能量占比、dominant noise band 和诊断枚举；该诊断不改变 payload、版权编号、跨端读取、提取阈值或质量阈值。
- `watermark:quality-gate:release -- --run-id codex-field-noise-diagnosis-release` 通过；`watermark:quality-gate:full -- --run-id codex-field-noise-diagnosis-full` 仍按预期 blocked。`field-noise` SNR `13.4966 dB`，modified pair ratio `0.284046`，extraction confidence `1.000000`；分段 SNR min/mean/max 为 `10.8940 / 13.7845 / 16.2193 dB`，频带差异噪声 low/watermark/high 占比为 `0.000001 / 0.999998 / 0.000001`，诊断为 `specific_watermark_band_energy_redistribution`。
- 2026-07-02 已新增隔离实验入口 `watermark:audio-noise-floor-band-selection-experiment` 和 bin `audio_noise_floor_band_selection_experiment`，只生成 `watermark-core/target/audio-noise-floor-band-selection/` 下的实验 artifact，不接入正式 UI / mock / release gate 默认路径。B 组 `frame_stability_window_sparse` 运行 `codex-noise-band-selection-b1` 通过实验硬约束：payload 仍为 V3/39，同一 `HS-5FF2FFE9-8601B35B-6D9EE4D2-D4F84DB4` 可读，extraction confidence `1.000000`；但 SNR 只从 baseline `13.1354 dB` 到 `13.6935 dB`，`bandEnergyShare.watermark.noise` 仍为 `0.999998`，不能解除 full gate 阻断，也不能晋级为正式算法候选。
- 同一隔离实验入口的 A 组 `inner_watermark_subband_sparse` 运行 `codex-noise-band-selection-a1` 通过硬约束：payload 仍为 V3/39，同一 `HS-62C2B433-3816FA34-A347F16A-7747A74D` 可读，extraction confidence `1.000000`；但最终必须保留 3 条 recovery lane，SNR `13.1427 dB`，`bandEnergyShare.watermark.noise=1.000000`。A 组说明当前 2-8 kHz extractor 可读布局内的 lane 优先级选择不能解决 `field-noise` 的频带差异集中问题，也不能晋级为正式算法候选。
- 同一隔离实验入口的 C 组 `masked_pair_budget_cap` 运行 `codex-noise-band-selection-c3` 通过硬约束：payload 仍为 V3/39，同一 `HS-D9B167D1-9723143D-22D7FD34-C442D72B` 可读，extraction confidence `1.000000`；预算保留比例 `0.990` 把 SNR 从同轮 baseline `13.1997 dB` 提升到 `21.1644 dB`，但仍低于 full gate `44 dB`，且 `bandEnergyShare.watermark.noise` 仅从 `0.999998` 降到 `0.999989`。C 组不能晋级正式算法候选；当前 2-8 kHz extractor 可读 lane 内微调应停止。`docs/音频噪声底跨端可读频带策略迁移设计.md` 已建立下一阶段设计闸门，并已进入 read-only candidate 扫描阶段，但尚未进入写入迁移实现。
- 2026-07-03 已新增音频噪声底迁移 fixture manifest、读取兼容门禁和 read-only candidate 扫描：`watermark-core/fixtures/audio-noise-floor-migration/manifest.schema.json`、最小 `manifest.example.json` 和 `watermark:audio-noise-floor-migration-read-compat`。该门禁使用共享核心生成 `watermark_core_legacy`、`desktop_legacy`、`mobile_legacy` 三类旧 V3/39 field-noise 保护副本占位，并验证当前默认 extractor 与 legacy readonly candidate 可读回同一长格式 UID、V3/39 和 `extractionConfidence >= 0.99`。随后补齐桌面端 `audio_noise_floor_migration_desktop_fixture` 与 Android 原生 Rust bridge `audio_noise_floor_migration_android_fixture` 生成的 file-backed 旧产物，manifest 记录 `protectedPath`、`sha256`、`bytes` 和 `generatedBy`；同时固化读取顺序和报告字段 `audioStrategyVersion / extractorPath / extractorFallbackPath / candidateFailureCode / candidateFailureMessage / candidateFailureMatrix / candidateScanAttempted / candidateScanProfiles / readCompatibilityMode`。当前 read compat 已接入真实 read-only `v3_noise_floor_migrated_band_v1_candidate` scanner：`watermark-core` 导出 `extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes` / samples 入口和 `AudioNoiseFloorMigrationCandidateFailureCode`，只读枚举 `noise_floor_low_mid_0_9_4_8k`、`noise_floor_mid_shift_1_2_6_2k`、`noise_floor_high_spread_3_8_9_6k` 三组候选频带；报告输出 `candidateScanAttempted=true`、`candidateScanProfiles`、`extractorPath=v3_recovery_2_8k_legacy`、`extractorFallbackPath=v3_noise_floor_migrated_band_v1_candidate -> v3_recovery_2_8k_legacy`、`candidateFailureCode=candidate_payload_not_found`、`readCompatibilityMode=legacy_v3_read_compat_candidate_interface_fallback`，实际读取继续 fallback 命中 legacy V3。`candidateFailureMatrix` 固定 5 个候选失败码的处理策略：当前旧样本预期为 payload not found 后 fallback；not implemented / 输入无效 / 过短阻断；未来 payload invalid 对旧样本可 fallback、对新候选样本阻断。它不实现新频带写入策略，不改变 payload / 版权编号 / 正式阈值，也不接入正式 UI / mock / release gate 默认路径。
- 同日已新增 `watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate/manifest.draft.json`，只作为新候选 fixture 草案和 read-compat 阻断矩阵，不包含真实 WAV。该 draft 固定 `draftOnly=true`、`mediaMutationAllowedInThisTask=false`、`writingImplementationAllowed=false`、`formalUiMockReleaseDefaultPathAllowed=false`；旧 V3/39 fixture 的通过态是 `candidate_payload_not_found -> v3_recovery_2_8k_legacy` fallback，新候选 fixture 的通过态必须是 `v3_noise_floor_migrated_band_v1_candidate` 直接命中同一 V3/39 UID。fallback-only、`candidate_payload_invalid`、`candidate_not_implemented_no_frequency_strategy` 和输入 / 时长错误都必须阻断。RC1 已将 `field-noise` 标记为 release blocker / known limitation，`plannedNewCandidateFixtures` 当前为 `paused_rc1_no_bytes_until_field_noise_blocker_is_resolved`，新频带 writer 实验暂停；该 draft 不改变 `watermark-core` 对外正式能力，也不代表新频带写入策略已实现。
- full gate 音频报告现在输出调试指标：短时 RMS min/mean/max、低能量帧比例、瞬态帧比例、noise-like 帧比例、embedding strength min/mean/max、modified pair ratio、extraction confidence，以及 `perceptualDiagnosis` 分段 SNR / 频带能量差异。
- 当前 V3 fast gate 的音频质量阈值为 SNR `>= 35 dB`、Integrated LUFS 差异 `<= 1.5 LU`、峰值差异 `<= 0.08`。

### 4.3 视频音轨

- L1 视频音轨水印跟随音频质量门禁。
- 当前 release smoke 已通过音频与 L1 视频音轨固定样本。
- `watermark:quality-gate:full` 对 L1 视频音轨增加 voice / mixed track 风险样本并输出 ABX 模板；L2 视频指纹存证不注入盲水印，不属于无感测试对象。
- 这不等于视频画面能力可售，更不等于 L3 已上线。

### 4.4 视频画面

- L3 独立 `watermark:l3-video-visual-release-gate` 已在 2026-07-01 完成完整 24 个 2K 样本池：H.264-HD / H264-LT / H264-MT / HEVC-HD / HEVC-MIX 非风险样本 confidence 均为 `1.000`，H.264-RISK 两个样本均记录为 `risk_boundary_expected`，H.264-HD summary 为 `release_thresholds_met`。
- L3 受控 worker fixture 已在 2026-07-01 接入 `cloud-video:ci`：worker 只处理内部 fixture / 受控上传清单，调用 `watermark-core` 构造 `VideoFeatureBundle`、正式 payload、`VideoVisualStrategy`、DCT 写入和成品帧自检，并通过后端 trusted completion 固化 `strategyDigest`、`selfCheckConfidence`、`checkedFrames` 和 `watermarkedMediaHash`。当前 QA 明确区分任务 `watermarkUid` 与 core 派生 `payloadWatermarkUid`；不能把 fixture 派生 UID 当作用户记录 UID。
- L3 真实 worker first-pass 已在 2026-07-01 接入 `cloud-video:ci`：`watermark-core` 新增 `build_video_visual_payload_from_reserved_uid` 和 `VideoVisualReservedPayloadBuildInput`，可把后端 registry-reserved `HS-...` UID 构造成 `WatermarkIssueMode::ServerReserved`、`WatermarkMediaType::VideoVisual` 的正式 V2/119 payload；`l3_real_worker_first_pass` 解析对象上传 manifest，使用 FFmpeg sandbox 解码 H.264 proxy，并强制 `payloadWatermarkUid === reserved.watermarkUid`。同一 CI 现在要求后端先通过 `POST /internal/video-tasks/claim` 发放 `attemptId` / `leaseToken`，completion HMAC 绑定当前 attempt / lease，旧 attempt / 错 lease / 重复 completion 均被拒绝；retryable failure 会回到 `queued` 并保留失败归因，non-retryable failure 不扣 `video_minutes`。本轮 worker 已从 `object://l3-upload/...` 读取真实 proxy，校验 manifest `sha256` / `bytes`，把写入后的 luma 帧重新封装为 `video/mp4`，输出到 `object://l3-output/<taskId>/<taskId>.l3-watermarked.mp4`，再解码最终 MP4 做 packaged self-check；completion 持久化 output ref / bytes / content type、worker receipt JSON 和 `workerReceiptHash`，签名下载 URL 返回真实 MP4 字节并复核 SHA-256。对象存储、队列和封装语义仍是 core 执行包装，不成为第二套视频画面水印算法。
- 本轮核心算法把 DCT mid-band embed delta 从 `72.0` 提升到 `96.0`，并收紧跨帧融合 confidence 语义：只有所有策略帧都被检查且融合成功时才把整段记为 `1.000`。
- gate 需要真实 FFmpeg / libx265 环境；HEVC 样本跳过、非风险样本 `confidence_below_threshold`、`self_check_failed`、`visual_extract_failed` 都会阻断正式化。
- 即使 24 样本 gate 与对象上传 / 签名 MP4 字节下载闭环已通过，也只能证明算法候选进入正式化下一阶段；商业可售还需要桌面 / 移动可操作下载入口、报告、版权库、跨端验证、失败文案和隐私边界通过。

## 5. 当前使用边界

`watermark-core` 可以做的，和不能做的，要分开写。

### 可以承诺

- 图片和音频可以写入、读取、验证。
- 桌面端和原生移动端可以互读正式保护副本。
- V3/39 是当前默认算法。
- V2/119 只作为显式回滚和迁移工具链。
- 保护副本一定会改变媒体字节，不能宣传成零扰动。

### 只能内部测试

- L3 视频画面盲水印的用户可见正式入口。
- 内部 QA 专用 V3 写入门禁。
- 只读候选 reader。
- 迁移桥接报告字段。

### 明确不能承诺

- “完全无损”。
- “零影响”。
- “已证明不可感知”。
- “生产级 C2PA / TSA 已上线”。
- “训练许可等同法律授权结论”。
- “L3 视频画面已经是正式可售能力”。

## 6. 建议长期记录项

后续每次 core 有变化，建议至少记录下面这些字段：

- 版本号和日期。
- 默认算法是 V2 还是 V3。
- 是否改动了 payload 长度。
- 是否改动了图片、音频、视频路径。
- 是否影响桌面端、移动端、后端和 QA 脚本。
- release gate 是否通过。
- 图片、音频、视频的样本池名称。
- 当前性能快照。
- 当前回滚门禁。
- 已知限制和环境挂起项。

这样做的目的很简单，避免文档只写“能力升级”，却没人知道它是在哪个样本池、哪个门禁、哪台机器上成立的。

### 2026-07-02 门禁口径记录

本次未改变 `watermark-core` 算法、payload、公有 API 或样本池。为配合 RC1 无外部依赖验收，`watermark:architecture-contract` 的后端扫描口径做了精确化：

- 允许后端保存 L3 execution wrapper 必需的 worker receipt 元数据，例如 `algorithmSource: watermark-core`，用于证明真实写入 / 自检来自共享核心。
- 允许后端在任务创建前做 L3 容量预检所需的保守位数估算，作为产品输入限制和失败文案依据。
- 继续禁止后端实现或调用正式盲水印 `embed / extract`、payload 编解码、媒体 IO 类型、DCT / QIM / LSB 等算法主体。
- 如果真实样本暴露 L3 写入、自检或 payload 问题，修复仍必须回到 `watermark-core`，不能在后端 wrapper 中绕过。

## 7. 推荐读法

如果你只想快速看结论，按这个顺序读：

1. 本文第 1 节，看 core 是什么。
2. 本文第 2 节，看现在能做什么。
3. 本文第 4 节，看当前性能基线。
4. 本文第 5 节，看能承诺什么，不能承诺什么。
5. 再去看：
   - `docs/当前真实能力边界说明.md`
   - `docs/共享水印核心与跨端互验推进计划.md`
   - `docs/感知质量发布门禁设计.md`

## 8. 结论

`watermark-core` 现在已经不是“一个图片水印库”，而是 HiddenShield 正式媒体锚点的共享核心。

它能稳定覆盖图片、音频和 L1 视频音轨的正式写入和读取，默认算法已经切到 V3/39。它也已经具备迁移桥、只读候选和内部 QA 的工具链。

但它还不是一个可以单独对外讲完整授权、生产 C2PA / TSA、L3 视频画面或企业商业 API 的总开关。那些能力要么在 core 外，要么仍在受控阶段。
## 2026-07-22 桌面高位深 WAV 保真闭环

- `watermark-core` 的 WAV 读写继续以输入 `hound::WavSpec` 为输出规格来源，已通过 `24-bit integer` 与 `32-bit float` 的写入、回读和水印提取测试。
- 桌面 FFmpeg 包装层修复了 `24-bit + sample_fmt=s32` 的判定顺序：明确的 `bits_per_raw_sample / bits_per_sample=24` 现在选择 `pcm_s24le`，不再错误升格为 `pcm_s32le`。
- 新增 `desktop_audio_read_qa` 独立读取器，供安装版 Gate 在不依赖写入任务状态的情况下复核保护副本。
- 当前性能快照：最终六个 48 kHz、31 秒 mono / stereo 安装版样本总 Gate 约 49 秒，六个单样本端到端累计 `42.5 秒`，单样本约 `5.6–10.6 秒`；该耗时包含 FFprobe、两次验证和量化统计，不等同于桌面 UI 单次写入耗时。
- 外部暴露边界：正式桌面承诺覆盖 `24-bit PCM WAV`、`24-bit FLAC` 和 `float32 WAV` 输入到 WAV 保护副本；`float64` 继续显式拒绝，其他 32-bit integer 容器组合仍需独立 Gate。
- 回滚路径：若任一安装版样本出现规格变化或读取失败，立即把高位深条目退回内部测试，并恢复只接受已验证位深的预检提示，不允许静默降精度。
- 证据：`artifacts/desktop-high-bit-depth-audio-gate/20260722-final/summary.json`。

## 2026-07-22 桌面图片资源边界闭环

- 新增公开边界 API：`validate_image_protection_input`、`validate_image_protection_file_size`、`MAX_IMAGE_PROTECTION_PIXELS` 与 `MAX_IMAGE_PROTECTION_BYTES`。
- `watermark-core` 在图片解码 / 写入路径执行不可绕过的 `100 MP`、`512 MiB` 与分块容量校验；桌面探测和运行时使用同一核心边界。
- 新增 `desktop_image_read_qa`，由独立核心进程复核安装版保护副本，避免只依赖写入任务自身状态。
- 性能快照：常规 2–4 MP 产品管线约 `8–24 秒`、峰值 `0.12–0.27 GiB`；约 99.92 MP 产品管线约 `17–19 分钟`、峰值 `6.25–6.57 GiB`；精确 512 MiB、1920×1080 PNG 产品管线约 `13 秒`、峰值约 `653 MiB`。
- 外部暴露边界：桌面仅开放静态 PNG / JPEG / WebP，统一输出 PNG 并保持尺寸；动画图片和其他容器不进入正式桌面承诺。
- 回滚路径：若大图资源回归出现 OOM，受影响区间立即退回内部测试，禁止通过缩图规避尺寸保持承诺。
- 证据：`artifacts/desktop-image-resource-gate/20260722-final/summary.json`。

## 2026-07-22 V3 spatial-recovery-v1 图片承载布局与桌面正式链路接入

- 共享核心模块正式命名为 `image_spatial_recovery_v1`；载荷协议继续使用 V3/39，图片承载布局单独标记为 `spatial-recovery-v1`，不复用或兼容 V2 载荷。
- 保持既有 `32×35` 恢复包 footprint、位置推导、`HSR1` layout ID 和 V3 UID 不变，使用 `2×3` 局部块的水平 Haar 系数符号承载；恢复包保存 `HSR1 + layout ID + 16-byte watermark ID + checksum`，读取后由共享 V3 构造器重建完全相同的 V3/39 最小锚点。
- `WatermarkService::embed_v3` 的正式图片写入在现有 V3 PNG 低影响通道之后叠加空间恢复包；`WatermarkService::extract` 的正式只读验证优先读取空间恢复包，再回退现有 V3 同步通道。
- Haar 目标亮度差由初始 96 经质量 Gate 收敛为 16；没有降低既有 PSNR / SSIM 阈值。release 质量 Gate 图片结果为 PSNR `35.80–46.06 dB`、SSIM `0.9890–0.9997`，全部通过。
- 当前性能快照：1920×1080 的 `4×4` 十六宫格真实裁切回读 `16 / 16` 通过并得到同一版权编号；36 个关键边界滑动 `1/16` 真实裁切全部通过；未裁切精确读取低于毫秒计时分辨率。
- 覆盖模拟：320×600 使用 30 个恢复包；1920×1080、2048×2048、5000×5000、9992×10000 均使用 25 个恢复包。所有尺寸的四分之一宽 × 四分之一高轴对齐滑动窗口在水平和垂直穷举中均为零缺口，十六宫格固定区域也均为零缺口。
- 重编码与误报快照：保护 PNG 经 JPEG `q=2 / yuv444p` 和 WebP `quality=90` 重编码后均恢复相同 V3 UID；纹理、纯色、中灰渐变和棋盘格四类干净图的精确读取与全图扫描均正确拒绝。
- 安装版验证：三张 Windows 内置摄影照片均通过正式写入、写后回读、独立核心读取和安装版只读验证；PSNR `44.19–51.59 dB`、SSIM `0.9952–0.9982`，每张十六宫格 `16 / 16` 和关键滑动裁切 `36 / 36` 均恢复同一 UID。
- 近 100 MP 资源快照：`9992×10000` PNG 通过，产品处理约 `20.61 分钟`，峰值工作集约 `6.58 GiB`，独立核心与安装版只读验证均通过。
- 外部暴露边界：桌面正式写入器和只读验证器已经调用同一共享核心；移动端继续冻结。当前仍缺缩放、旋转、严重重压缩和统计规模误报 Gate，因此不能对用户承诺“裁到 1/16 仍可恢复”。
- 证据：`artifacts/desktop-image-spatial-recovery-gate/20260722-local-transform/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-visual/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-100mp/summary.json`。
- 回滚路径：从 `embed_image_v3_internal_qa_bytes` 和正式只读候选提取器移除空间恢复调用，并恢复 `WatermarkService` 现有 V3 同步通道即可；V3 payload 与 UID 不需要迁移。

## 2026-07-22 V3 图片空间恢复正式闭环

- 正式写入 API 已从 `embed_image_v3_internal_qa_bytes` 收敛为 `embed_image_v3_bytes`；正式读取 API 为 `extract_image_v3_bytes`，`WatermarkService` 与桌面只读验证统一使用该入口。
- `spatial-recovery-v1` 保持 `HSR1` layout ID、V3 UID、`32×35` footprint 和既有位置推导；25 个恢复包的魔数固定，其余 144 位使用按包序号确定的分散排列，读取端逆排列后做共识，降低 JPEG 系统性误差。
- 全图失败扫描由逐候选读取 32 位魔数改为 O(width) 内存的流式 Haar 行签名；11 MP 干净真实照片 release 拒绝从约 `32 秒` 降至约 `1.2 秒`。最终 102 样本误报 Gate 平均 `184 ms`、最慢 `373 ms`。
- 读取支持原图、90/180/270 度旋转、80/85/90/95% 缩放候选中的已验证 85% 路径，以及 JPEG/WebP quality 75/60。缩放先使用廉价魔数近似检测，仅命中后执行精确回放。
- Haar 目标亮度差最终为 20；三张真实照片 PSNR `44.11–51.29 dB`、SSIM `0.9951–0.9981`，未降低既有 release 阈值。
- 安装版每张真实照片完成 `16/16` 十六宫格、`36/36` 关键滑动裁切和 `8/8` 独立变换恢复；近 100 MP 产品处理约 `6.36 秒`、峰值约 `0.70 GiB`。
- 外部暴露边界：桌面端可承诺轴对齐宽高各 `1/4` 的裁切区域，以及已列明的独立旋转、缩放和重编码案例；组合扰动、任意角度、低于 quality 60、低于 80% 缩放和移动端同等能力不在当前承诺内。
- 回滚路径：保留前一提交作为代码回滚点；若 RC 复验出现 UID 错读、误报或质量阈值下降，整体撤回空间恢复用户承诺，不允许桌面消费方自行增加算法兜底。
- 证据：`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final-installed/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final/false-positive-summary.json`。

## 2026-07-22 桌面音频 20 分钟 / 512 MiB 资源边界闭合

- 图片 `spatial-recovery-v1` 算法与既有桌面产品口径保持冻结；本次没有修改图片 API、承载布局、恢复阈值或用户承诺。
- 音频不使用图片 `spatial-recovery-v1`。音频继续由 `watermark-core` 的独立时频 / QIM 写读链路承载，只与图片共享 V3 UID、`WatermarkService` 和发布治理。
- 新增公开边界常量 `MAX_AUDIO_PROTECTION_SECONDS = 1200`、`MAX_AUDIO_PROTECTION_BYTES = 512 MiB`，以及 `validate_audio_protection_file_size`；`validate_audio_protection_input` 同时执行最短时长、最长时长、采样率和声道校验。
- 桌面执行入口在 FFprobe、解码和写入前再次执行精确源文件字节校验，不能通过绕过前端预检处理超限音频；桌面探测结果新增精确 `fileSizeBytes`，不再依赖四舍五入后的 MiB 判定。
- 当前安装版性能快照：`8 kHz / mono / 20:00` WAV 完成写入、写后回读、独立核心读取和只读验证约 `21.9 秒`；精确 `512 MiB`、`48 kHz / stereo / 31 秒` WAV 完成同链路约 `9.6 秒`。
- 拒绝性能：`20:01` 约 `0.8 秒`结束且不创建版权记录；`512 MiB + 1 byte` 约 `2.4 秒`结束且不创建版权记录。
- 外部暴露边界：桌面端可承诺 `30 秒–20 分钟`、源文件不超过 `512 MiB`、`8–48 kHz`、mono / stereo，并保持原采样率与声道；移动端没有因本次 Gate 自动获得相同资源承诺。
- 回滚路径：若后续安装版不能稳定复现允许边界，先把 `20 分钟 / 512 MiB` 退回内部测试并收紧桌面预检，不允许通过重采样、降声道或格式转换规避边界。
- 证据：`artifacts/desktop-audio-resource-gate/20260722-final-v2/summary.json`、`artifacts/desktop-installer-self-contained/20260722-audio-resource-v2/desktop-installer-self-contained-gate.json`。

## 2026-07-22 桌面媒体内部 RC 阻断

- 最终组合候选复跑冻结图片完整 Gate 时，第三张真实照片的 WebP quality 60 变体被独立核心读取为错误 UID。
- 原 UID 为 `HS-9214D504-63C9EFDF-5376CA9B-9A81A854`，读取 UID 为 `HS-9214D504-63C9EFDF-5336CA9A-9A81A854`；其余七个独立变换恢复正确。
- 该结果说明当前 WebP q60 恢复存在 UID / 内容组合相关的不稳定性。它是共享核心问题，桌面消费方不得增加自己的纠错或 UID 替换兜底。
- 当前默认完整测试为 `110 passed / 7 failed`：六个旧图片 API / 迁移测试失败，一个暂停范围内 L3 视频性能测试超过阈值；正式 V3 图片服务测试 `5 / 5` 通过。
- 外部暴露边界：WebP q60 恢复暂时降级为内部测试；整个桌面媒体 RC 阻断，直到错误 UID 问题修复或产品口径正式收窄。
- 回滚路径：保持当前算法冻结，不使用重复重跑覆盖失败证据；优先增加同一真实照片、多独立 UID 的固定回归，再决定核心修复或移除 WebP q60 承诺。
- 证据：`artifacts/desktop-media-internal-rc/20260722/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-media-rc-final-candidate/summary.json`。

## 2026-07-22 RC-MEDIA-001 WebP q60 错误 UID 修复

- 根因：`spatial-recovery-v1` 精确读取器此前在计算 25 包共识前，先返回第一个通过 8-bit 校验的独立包。固定照片 `windows-theme-c-img29.jpg` 的 WebP q60 变体中，packet variant 0 的 UID 位 `73`、`95` 同时翻转但校验碰撞，导致返回错误 UID；同一图像的 25 包共识恢复正确 UID。
- 核心变更：`extract_spatial_recovery_v1_exact` 改为直接共识、受限软纠错共识、独立包依次降级；扫描裁切读取路径与 V3 UID、布局、写入位置均不改变。诊断 API 记录每包 UID、差异位、共识票强度和最终选择阶段，外部消费方仍只能调用共享核心。
- 固定回归：同一真实照片的三个独立 UID 在 WebP q60 下由 `2/3` 提升为 `3/3`，原失败 UID 不再出现第 `73`、`95` 位错误；聚焦 V3 图片服务测试 `5/5`、干净图误报 Gate `102/102` 拒绝、架构契约均通过。
- 安装版快照：候选 SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40` 的综合图片 Gate 通过，三张真实照片各 `8/8` 变换恢复、强裁切恢复与近 100 MP 资源检查保持通过。
- 外部暴露边界：桌面 WebP quality 60 恢复承诺保持不变，不收窄为 quality 75；低于 quality 60、组合扰动和移动端同等能力仍不承诺。
- 证据：`artifacts/image-webp-q60-uid-regression/20260722-diagnostic/summary.json`、`artifacts/image-webp-q60-uid-regression/20260722-green/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-webp-q60-core-fix-installed/summary.json`。
- 回滚路径：如后续共识优先引发裁切或误报回归，回滚本次读取顺序变更并临时将 WebP q60 降为内部测试，不允许在桌面消费方添加 UID 替换逻辑。

## 2026-07-22 RC-MEDIA-001 正式关闭

- 使用同一 installed exe（SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`）完成三轮安装版综合图片 Gate。
- 三张真实照片分别获得三个互不重复的独立 UID；每个 UID 均通过旋转 90/180/270、85% 缩放、JPEG q75/q60、WebP q75/q60，共 `72/72` 个变换单元 UID 精确一致。
- 每个变换单元同时要求独立 `watermark-core` 读取和 installed desktop 只读验证通过；两类读取均为 `72/72`。
- `RC-MEDIA-001` 的三个解除条件全部满足，正式关闭。该结论不改变低于 q60、组合扰动或移动端边界。
- 证据：`artifacts/desktop-media-internal-rc/20260722/rc-media-001-closure.json`。
- 当前性能快照：三轮综合 Gate 分别约 `229`、`186`、`186` 秒；大图临时文件在每轮后清理，D 盘剩余空间稳定。

## 2026-07-22 RC-MEDIA-002 V3-only 默认套件收口

- 六项失败测试的根因是仍引用已退役的 V2 图片 API，而不是 V3 图片算法回归；这些测试不应继续存在于默认 release suite。
- 正式图片写入、读取、验证、重写预检和消费方统一使用 V3/39；V2 图片写读与 `force_v2_rollback` 统一稳定拒绝，错误码为 `v2_image_rollback_retired`。
- 默认 `watermark-core` release suite 为 `108 passed / 0 failed`，正式 V3 图片服务测试保持 `5/5`。
- legacy / rollback-only 验证迁移到 `npm run watermark:legacy-rollback-suite`；该套件验证 V2 图片拒绝合同，并隔离保留音频旧版回滚。
- `RC-MEDIA-002` 正式关闭；整体桌面媒体 RC 仍被 `RC-MEDIA-003`、`RC-MEDIA-004`、`RC-RELEASE-001`、`RC-RELEASE-002` 阻断。
- 下一步：执行 `RC-MEDIA-003`，使用最终安装候选完成五种音频格式 × mono/stereo 基线并归档正式证据。

## 2026-07-22 RC-MEDIA-003 安装版音频格式 / 声道基线

- 新增 `release:desktop-audio-format-channel`，固定使用真实 30 秒、48 kHz 的 WAV / MP3 / FLAC / OGG / M4A × mono / stereo 十单元 fixture。
- 每个单元均通过安装版 `probe_source`、正式 `start_pipeline`、写后回读、独立 `watermark-core` 读取和安装版只读验证；V3 UID 精确一致，结果为 `10/10`。
- 输出均为 WAV，所有单元保持源采样率和声道；16-bit WAV 与 24-bit FLAC 同时保持有效位深。有损 MP3 / OGG / M4A 输出为 float32 WAV，不承诺保留原有损编码格式。
- 同一 installed exe SHA-256 为 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`；整轮约 `53.3 秒`，单单元约 `4.0–9.8 秒`。
- 外部暴露边界：该 Gate 关闭格式与 mono / stereo 安装版覆盖缺口，不替代既有 `8–48 kHz` 采样率边界测试，也不关闭 20 分钟高位深上包络组合。
- 证据：`artifacts/desktop-audio-format-channel-gate/20260722-final/summary.json`。
- 下一步：执行 `RC-MEDIA-004`，验证 `20:00 + 48 kHz + stereo + 高位深` 最终安装候选组合。

## 2026-07-22 RC-MEDIA-004 音频合法上包络组合

- 新增 `release:desktop-audio-upper-envelope`，生成真实 `20:00 / 48 kHz / stereo / 24-bit FLAC`，并通过最终安装候选输出 `24-bit PCM WAV`。
- 输入约 `108.04 MiB`，输出约 `329.59 MiB`；时长、采样率、声道和有效位深全部保持，写后回读、独立 V3 核心读取和安装版只读验证读回同一 UID。
- 完成场景约 `57.5 秒`；应用主进程峰值工作集约 `1.215 GiB`，完整进程树工作集求和峰值约 `2.151 GiB`。
- 取消场景约 `14 ms` 完成 UI / 命令确认，不创建版权记录；当前 FFmpeg / 核心阶段不能被瞬时抢占，约 `45.8 秒`后达到连续 CPU 静默。取消场景主进程峰值约 `1.252 GiB`，进程树求和峰值约 `2.192 GiB`。
- 外部暴露边界：允许承诺取消后不落库，但不能承诺上包络任务立即释放 CPU / 内存；该延迟属于当前明确资源特征。
- 证据：`artifacts/desktop-audio-upper-envelope-gate/20260722-final/summary.json`。
- 下一步：执行 `RC-RELEASE-001`，对最终候选完成 Authenticode 签名且不得重新构建。

## 2026-07-26 独立感知质量实验室共享指标 API

- 新增公开模块 `watermark_core::quality`，提供 `compare_image_quality`、`compare_audio_quality`、`ImageQualityInput / Report`、`AudioQualityInput / Report`、`QualityThresholdProfile / Result`。
- `watermark:quality-gate:release` 与 `watermark:quality-gate:full` 已改为调用同一共享实现；PSNR、SSIM、SNR、当前 gate 口径 LUFS、峰值差和 clipping 阈值语义不变。
- 图片报告新增 MAE、P95 通道差、最大通道差和变化像素率；P95 使用固定 256 桶直方图，不再按像素量额外保存并排序完整差异数组。
- 音频报告新增显式声道输入、单声道诊断下混、分段 SNR、静音噪声底变化及低频 / 水印频带 / 高频差异能量占比；正式 gate 继续使用原单声道 fixture，因此既有结果口径不变。
- 外部暴露边界：该 API 只负责媒体质量比较，不写入、读取或验证水印，不改变 payload、版权编号、算法强度或跨端互读契约。
- 当前性能边界：独立实验室对音频使用 FFmpeg 解码后的内存样本进行比较，最长限制 20 分钟、文件不超过 512 MiB；长音频仍可能占用数百 MiB 内存。图片热力图会生成三份会话级 PNG 临时文件。
- 回滚路径：独立程序可直接移除而不影响正式媒体能力；共享 quality 模块若回滚，必须同时恢复 gate 内原指标实现并通过固定 fixture 数值回归，禁止保留两套漂移口径。
- 验证：`cargo test --manifest-path watermark-core/Cargo.toml quality::tests --lib` 通过 `3/3`；独立实验室前端 ABX 测试通过 `3/3`，Rust 媒体辅助测试通过 `2/2`，前端生产构建和 Windows Tauri release EXE 构建通过。
- 运行态 gate 结果：release gate 当前仍由 `low-texture` 的 SSIM `0.982920 < 0.985` 阻断；full gate 仍由 6 个图片样本和 `field-noise` 音频阻断，其中 `field-noise` SNR 为 `12.9879 dB < 44 dB`。共享 API 保留了原公式和阈值，当前失败属于仓库现有算法 / fixture 基线，不能记录为本次工具已使 release/full gate 通过。
- 下一核心任务：为 release gate 固定不随 runId 漂移的基准 artifact，并单独修复 `low-texture` SSIM 阻断；之后再对共享 API 重构前后 JSON 做逐字段基线比对。
# 2026-07-27 Post-Embed C2PA Resign Gate

- `watermark-core` 仍只负责正式 V3 图片 anchor 写入与读取；未增加 C2PA 签发、证书、信任链或 metadata 实现。
- internal QA 已验证：共享核心输出的 verified V3 PNG 经外层 ephemeral C2PA signer 重新签发后，同一最终 PNG 仍可回读相同 verified V3。
- C2PA active manifest 可读，但分类为 `manifest_present_with_validation_findings`，只表示本地自签测试链。
- 外部暴露边界：生产 signer/KMS/HSM、C2PA 信任链、SDK 与公共 Resolver 均未开放；失败回滚和最终 hash 绑定尚未进入 production command。
- 回滚：移除 post-embed wrapper 即恢复为原 V3 PNG 输出，不影响共享核心 API、payload 或算法。

## 2026-07-27 Production Post-Embed Wrapper 合同边界

- 正式顺序冻结为：`watermark-core` V3 写入/回读 → 外部 C2PA signer → C2PA/V3 双回读 → final hash → confirm。
- `watermark-core` 不保存 signer credential、不生成 production C2PA receipt、不负责 Profile entitlement 或 PostgreSQL confirm。
- 任一外层失败均不得把未签名或已签名 bytes 作为成功产物返回；共享核心输出只能作为未确认中间件。
- 当前性能快照未变化；尚未测量 production signer latency、最终文件增长和双回读成本。
- 回滚路径：关闭 signer wrapper 后恢复 internal V3-only 路径，但不得对外承诺 C2PA + V3 双层能力。

## 2026-07-27 Post-Embed Wrapper Schema Contract

- 新增的 command/receipt/Profile Schema 与七类 fixture 只约束外层 signer wrapper；`watermark-core` 代码、API、V3/39 payload、算法和性能均未变化。
- fixture 要求 signer 后再次由共享核心读取相同 UID、V3/39 和 verified auth。
- 当前性能快照未变化；internal command 实现后需单独记录 V3 写入、C2PA 签发、C2PA readback、V3 readback 和 confirm 耗时。
- 回滚：移除外层 Schema/command 不影响现有共享核心媒体兼容性。

## 2026-07-27 Internal Post-Embed Signing Command 边界

- `watermark-core` 本次无代码、公开 API、payload、算法、fixture 或性能变化；正式 V3 图片写入/读取仍由共享核心唯一负责。
- backend 新增的 internal-only command 负责 production license/credential/Profile 校验、外部 C2PA signer receipt、最终 hash、双回读结果编排、artifact 隔离和 PostgreSQL confirm，不得实现第二套 V3 embed/extract/auth 逻辑。
- PostgreSQL 七场景 QA 使用受控 readback interface 验证事务状态机；真实媒体层“最终 PNG 同时可读 C2PA manifest 与 verified V3”的证据仍来自既有 post-embed prototype，两类证据不得混写。
- 当前性能快照未变化，尚未获得 production signer、C2PA readback、V3 readback、artifact finalize 与 confirm 的分阶段延迟数据。
- 外部暴露边界：仍不得承诺 production C2PA chain、SDK、公共 Resolver、客户 credential、跨平台验收或法规合规。
- 回滚：移除 backend post-embed command 与 0007 projection 可恢复 internal V3-only 路径，不影响 `watermark-core` API 或既有保护副本读取。

## 2026-07-27 Signing Reservation / Artifact Recovery 核心边界

- `watermark-core` 本次继续无代码、API、payload、算法、fixture 或性能变化。
- PostgreSQL reservation、lease、advisory lock、signer invocation key、artifact staging/finalize、recovery audit 和 ledger 延迟提交全部位于 backend 外层。
- 恢复 `signed_staged` 或 `artifact_pending` 时禁止再次调用共享核心写入；最终 V3 readback 的生产 adapter 仍必须调用 `watermark-core`。
- 九场景 PostgreSQL QA 证明事务和调用次数，不构成新的媒体鲁棒性、跨端读取、production C2PA trust chain 或性能证据。
- 回滚：可移除 0008 与 reservation/recovery orchestration 并回到 0007 internal command；`watermark-core` 保护副本与读取兼容性不受影响。

## 2026-07-28 Adapter Receipt / Crash Recovery 核心边界

- `watermark-core` 本次继续无代码、公共 API、payload、算法、fixture、benchmark 或性能变化。
- `0009`、production signer/object-store receipt、四个 crash injection point 和 PostgreSQL recovery harness 均属于 backend 外层；不得据此在 `watermark-core` 中引入 C2PA signer、对象存储或账单语义。
- 恢复过程使用同一 final hash 与 signer invocation key，禁止重复 V3 embed；正式 V3 回读 adapter 仍只能调用共享核心。
- 十三场景 QA 证明单一 external-cost projection、单一 artifact write 和单一 confirm/ledger，不构成媒体鲁棒性、跨端读取、production trust chain 或性能提升证据。
- 回滚：可移除 0009 receipt columns/indexes 与 crash harness，保留 0008 reservation/recovery；共享核心输出与读取兼容性不受影响。

## 2026-07-28 Internal Recovery Worker 核心边界

- `watermark-core` 本次仍无代码、公共 API、payload、算法、fixture、benchmark 或性能变化。
- worker scanning、claim lease、attempt、backoff、dead-letter 和 recovery audit 全部属于 backend PostgreSQL orchestration。
- reserved recovery 不得重新调用 V3 embed；artifact pending recovery 不得调用 signer 或共享核心写入。
- worker QA 证明 orchestration 与数据库并发，不构成新的媒体鲁棒性、跨端读取或 production trust chain 证据。
- 回滚：可移除 0010 与 recovery worker module，保留现有手工 idempotent replay；共享核心兼容性不受影响。

## 2026-07-28 Dead-Letter Inspect / Requeue 核心边界

- `watermark-core` 本次仍无代码、公共 API、payload、算法、fixture、benchmark 或性能变化。
- inspect/requeue command、双人审批、recovery control version、PostgreSQL 行锁与 append-only audit 全部位于 backend 外层。
- requeue 不执行 V3 embed/extract；后续 reserved recovery 仍必须复用既有 V3 中间产物，正式 V3 readback adapter 仍只能调用共享核心。
- PostgreSQL 并发 QA 只证明持锁期间 worker 零 claim、提交后单次恢复与成本幂等，不增加跨端媒体兼容性证据。
- 外部暴露边界不变：无 SDK、公共 Resolver、production credential、客户自助 requeue 或 production C2PA trust chain 承诺。
- 回滚：可移除 0011 与 dead-letter command module，保留 0010 worker dead-letter 终态；共享核心输出和读取兼容性不受影响。

## 2026-07-28 Confirmed Delivery Envelope 共享合同

- 新增公共类型 `AiConfirmedArtifactDeliveryEnvelope`、Profile identity、验证结果和稳定失败码。
- 新增 `seal_ai_delivery_envelope`、`validate_ai_delivery_envelope`、Profile/envelope digest 与 canonical receipt JSON SHA-256。
- 摘要实现显式递归排序 JSON object key，不受 `serde_json preserve_order` 或依赖 feature unification 影响。
- Desktop/mobile 已共同调用该 API；未在任一端复制 envelope digest、receipt binding 或状态规则。
- 当前性能快照：新增操作仅为 JSON parse/canonicalize 与 SHA-256；未改变图像 V3 写入/读取性能，尚未建立 production 大 receipt 延迟基线。
- 外部暴露边界：只能内部测试；不是 SDK、公共 Resolver、法规结论、production trust chain 或客户交付承诺。
- 回滚：移除 delivery envelope module、0012 projection 与双端 wrapper，不影响现有 V3 payload、媒体 fixture 或跨端读取兼容性。

## 2026-07-28 Delivery Retrieval Receipt 与 Import Admission

- 新增公共类型 `AiDeliveryRetrievalReceipt`、`AiDeliveryImportAdmission` 和 schema version `hs-ai-delivery-retrieval-receipt-v1`。
- 新增 `seal_ai_delivery_retrieval_receipt`、`ai_delivery_retrieval_receipt_digest` 与 `validate_ai_delivery_import`。
- 导入准入复用现有 delivery envelope 校验，并额外绑定 authorization ID、retrieval receipt ID、delivery envelope ID、execution ID、envelope digest、final file hash 与 finalize receipt digest。
- Desktop/mobile 使用同一共享 fixture 和同一核心 API；拒绝结果不得携带任何可继续 vault/import 的 ID 或摘要。
- 当前性能快照：仅增加固定字段 SHA-256、JSON decode 与既有 envelope 校验；不改变图像 V3 embed/extract 性能，尚无大对象 production 下载延迟基线。
- 外部暴露边界：`只能内部测试`；receipt digest 不是外部公钥签名，不得解释为公共可验证凭证、SDK token 或法规证明。
- 回滚：可移除 retrieval receipt/import admission API 和 0013 backend orchestration；现有 V3 payload、delivery envelope 与跨端媒体读取兼容性不受影响。

## 2026-07-28 PostgreSQL Platform API Executor 分层

- `watermark-core` 本次无代码、payload、算法、fixture 或性能变化。
- backend executor 新增 prepare-only 编排：继续调用共享核心完成 PNG V3 embed、extract、UID 与认证状态回读，再由独立 confirm endpoint 提交 Manifest 和计量。
- 原 `execute_postgres_internal_image_marking` 保持兼容，内部改为 prepare 后调用既有 PostgreSQL confirm command。
- Platform API 不实现第二套水印写入、读取、payload、copyright ID 或 rewrite 规则。
- 性能快照沿用现有 512×512 PostgreSQL E2E fixture；本次只证明真实 HTTP/数据库闭环，不构成新鲁棒性或吞吐结论。
- 外部暴露边界：`只能内部测试`；SDK、公共 Resolver、production credential、真实 provider 与客户 SLA 继续关闭。
- 回滚：可移除 0019、Platform API router 和 prepare-only 外层，保留原 executor wrapper；共享核心输出格式与跨端读取兼容性不受影响。
## 2026-07-28 免费公共 Resolver 核心边界

- `watermark-core` 本次无代码、公共 API、payload、算法、fixture 或性能变化。
- Resolver 不上传媒体、不调用 embed/extract，只按 watermark UID 或 Manifest ID 读取 confirmed PostgreSQL 公共 view。
- public `watermarkDetectionStatus` 复用 confirm 时共享核心 write-after-read 结果，不构成现场媒体检测。
- 当前性能证据仅覆盖小型 JSON/SQL 读取，不构成 V3 检测吞吐、CDN SLA 或公网容量结论。
- 外部暴露边界：`只能内部测试`；公网域名、WAF/CDN、生产 SLA 与法律义务满足性均未开放。
- 回滚：可移除 0020 views 与 Resolver router，不影响 V3 payload、正式写入/读取和跨端兼容性。
## 2026-07-28 设计伙伴 Sandbox 接入包影响

- 本次新增设计伙伴 onboarding、Profile questionnaire、SDK/API 示例、Resolver link 与验收矩阵。
- 未修改 `watermark-core` 公共 API、V3 payload、图片嵌入/读取算法、rewrite 规则、fixture 或性能快照。
- 伙伴示例只能通过 backend image marking executor 调用 `watermark-core`，不得形成第二套标识算法。
- 外部暴露边界不变：当前设计伙伴包为 `只能内部测试`，不代表 core SDK 已公开分发或生产 SLA 已建立。
- 回滚路径：移除 private partner kit 不影响现有 core 写入、读取、跨端 bridge 或 PostgreSQL 标识记录。

下一核心任务：待真实伙伴生成 accepted Sandbox PNG 后，将其作为外部来源 fixture 纳入跨端读取回归，不因伙伴接入修改现有 V3 算法。

## 2026-07-28 Synthetic Sandbox QA 影响

- 新增 synthetic Sandbox QA 仅模拟 SDK/facade transport；未调用、修改或包装 `watermark-core`。
- synthetic marked PNG 不含可对外承诺的 HiddenShield V3 盲水印，不进入 core fixture、性能快照或跨端互验。
- 回滚路径：移除 synthetic harness 不影响 core API、现有图片写读算法或正式 executor。
