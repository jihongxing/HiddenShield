# Phase I-6 L3 2K 高码率 release 样本池与阈值策略

状态：测试门禁已落地，未进入商业实现。

本文把 2K 高码率候选从“单组 staged 结果”推进为可执行的 release 样本池和阈值策略。本文不新增算法、不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`，也不把 L3 视频画面盲水印包装成当前可售能力。

## 1. 设计结论

- 2K 高码率 release 样本池只覆盖 2560x1440 主战场，不覆盖 4K / 8K。
- H.264 候选预算固定为首压 10Mbps / CRF21，二压 8Mbps / CRF23。
- HEVC 候选预算固定为首压 8Mbps / CRF20，二压 6.5Mbps / CRF24。
- H.264 10Mbps -> 8Mbps 是当前 2K 高码率 H.264 release 候选预算。
- HEVC 8Mbps -> 6.5Mbps 是当前 2K 高码率 HEVC release 候选预算。
- 默认策略为 `watermark-core` core default，也就是 2K 进入 `TranscodeStable`。
- release 口径固定为 30 秒、30fps、16 个采样帧、96 个策略区域。
- H.264 非风险样本最低 confidence 门槛为 0.950。
- HEVC 非风险样本最低 confidence 门槛为 0.970。
- H.264 高细节当前只有 confidence 0.875，因此只能进入 release 样本池，不得进入商业 SLA。
- HEVC 高细节当前 confidence 1.000，可以作为高码率优先候选，但仍需样本池通过后才能进入商业包装讨论。

## 2. 样本池

首版 release 样本池至少 24 个样本，按编码和内容类型分层。

| 分组 | 编码 | 内容画像 | 样本数 | 预算 | 当前门槛 |
| --- | --- | --- | ---: | --- | --- |
| H264-HD | H.264 | 高细节横屏、细纹理、复杂背景 | 6 | 10Mbps -> 8Mbps | 每个样本 >= 0.950，分组均值 >= 0.970 |
| H264-LT | H.264 | 低纹理、色块、室内稳定镜头 | 4 | 10Mbps -> 8Mbps | 每个样本 >= 0.950，分组均值 >= 0.980 |
| H264-MT | H.264 | 运动纹理、人群、快速平移 | 4 | 10Mbps -> 8Mbps | 每个样本 >= 0.950，分组均值 >= 0.980 |
| H264-RISK | H.264 | 极端高频、逐帧噪声、字幕密集 | 2 | 10Mbps -> 8Mbps | 允许 `self_check_failed`，必须归因为风险边界 |
| HEVC-HD | HEVC | 高细节横屏、细纹理、复杂背景 | 4 | 8Mbps -> 6.5Mbps | 每个样本 >= 0.970，分组均值 >= 0.990 |
| HEVC-MIX | HEVC | 低纹理、运动纹理、常规平台内容 | 4 | 8Mbps -> 6.5Mbps | 每个样本 >= 0.970，分组均值 >= 0.990 |

样本要求：

- 样本必须是可重复生成或可追溯的固定 fixture，不能依赖临时用户文件。
- 每个样本记录内容画像、分辨率、时长、帧率、首压预算、二压预算、采样帧数、策略区域数、区域策略、confidence、失败码和分段耗时。
- 每个样本必须经过真实 FFmpeg 编码 / 解码后，再交回 `watermark-core` DCT staged API 自检。
- `libx265` 不可用时，HEVC 分组必须记录为环境跳过，不能把跳过当作通过。

## 3. Release 阈值

### 3.1 H.264 门槛

H.264 进入 release-blocking 门禁需要同时满足：

- `H264-HD` 6 个样本全部 confidence >= 0.950。
- `H264-HD` 分组均值 confidence >= 0.970。
- `H264-LT` 和 `H264-MT` 每个样本 confidence >= 0.950。
- `H264-LT` 和 `H264-MT` 分组均值 confidence >= 0.980。
- 非风险样本不得出现 `self_check_failed`、`visual_extract_failed` 或 payload 不一致。
- `H264-RISK` 只用于风险边界记录，不计入通过率，不得被包装成可承诺内容。

商业包装讨论门槛更高：

- H.264 非风险样本连续两轮 release 运行无失败。
- `H264-HD` 最低 confidence 连续两轮 >= 0.950。
- 2K H.264 处理倍率有稳定预算记录，并能进入成本模型。
- 桌面端 / 云端包装、策略包、账本、隐私边界和跨端验证另行通过，不得只凭算法门槛进入销售话术。

当前状态：

- H.264 高细节 10Mbps -> 8Mbps 当前为 confidence 0.875。
- 因此 H.264 高细节仍是 release-blocking 风险，不能进入默认商业承诺。

### 3.2 HEVC 门槛

HEVC 进入 release-blocking 门禁需要同时满足：

- `HEVC-HD` 4 个样本全部 confidence >= 0.970。
- `HEVC-HD` 分组均值 confidence >= 0.990。
- `HEVC-MIX` 4 个样本全部 confidence >= 0.970。
- `HEVC-MIX` 分组均值 confidence >= 0.990。
- 非风险样本不得出现 `self_check_failed`、`visual_extract_failed` 或 payload 不一致。
- HEVC 环境必须显式记录 `libx265` 可用性；缺少编码器时只能跳过 HEVC release 分组。

商业包装讨论门槛：

- HEVC 样本池连续两轮 release 运行无失败。
- HEVC 2K 处理倍率不超过当前 2K 平台权重可接受区间。
- H.264 高细节风险仍需在对外材料中单独披露，不能用 HEVC 满置信覆盖 H.264 风险。

当前状态：

- HEVC 高细节 8Mbps -> 6.5Mbps 当前为 confidence 1.000。
- 该结果只能证明 HEVC 高码率候选更稳，不能单独推出 L3 商业能力。

## 4. 失败归因

release 样本池必须记录结构化失败归因，不允许只写“失败”。

| 失败归因 | 判定条件 | 处理 |
| --- | --- | --- |
| `sample_fixture_invalid` | 样本不可重复、时长 / 分辨率 / 帧率不符合池定义 | 修复样本，不计入算法结论 |
| `encoder_unavailable` | `libx264` 或 `libx265` 不可用 | 记录环境跳过，不计入通过 |
| `decode_or_transcode_failed` | FFmpeg 生成、首压、二压或 Y plane 解码失败 | 修复测试链路，不计入算法结论 |
| `payload_mismatch` | 提取出的 payload 与写入 payload 不一致 | release-blocking 失败 |
| `confidence_below_threshold` | 可提取但低于分组最低 confidence | release-blocking 失败 |
| `self_check_failed` | core 自检失败 | 非风险样本为 release-blocking；风险样本记录边界 |
| `visual_extract_failed` | 无法恢复 payload 或 ECC 失败 | release-blocking 失败 |
| `risk_boundary_expected` | 极端高频、逐帧噪声、字幕密集等明确风险样本失败 | 不计入通过率，不得商业包装 |

## 5. 商业包装门槛

满足 release 样本池通过，不等于立刻可售。进入 Studio / Enterprise 包装讨论必须再满足：

- L3 写入、提取、自检、payload、策略、错误码继续只位于 `watermark-core`。
- 桌面端、移动端、后端和云端只包装或调用 `watermark-core`，不出现第二套算法。
- 云端任务、策略包签名、密钥边界、隐私边界、正式报告、用量账本和失败不扣费规则全部通过合同。
- 至少完成桌面写入 / 云端验证、云端写入 / 桌面验证的正式跨端互验。
- 用户可见文案必须区分 H.264 与 HEVC 支持边界，不得写成“任意 2K 视频平台二压后都稳定识别”。

禁止商业包装：

- H.264 高细节最低 confidence 仍低于 0.950。
- 任一非风险样本出现 `payload_mismatch`。
- 任一非风险样本出现未归因的 `self_check_failed`。
- HEVC 通过但 H.264 高细节未过，却对外宣称“2K 视频画面盲水印已生产可用”。
- 只完成测试层 staged API，没有 UI、云任务、账本、正式报告和跨端验证。

## 6. 下一步实现

已新增 release 样本池门禁，不改商业入口：

- Tauri 测试 `l3_2k_high_bitrate_release_sample_pool_records_thresholds` 已落地。
- 测试定义固定 24 个可重复生成的 2K fixture，并检查 `H264-HD` 6 个、`H264-LT` 4 个、`H264-MT` 4 个、`H264-RISK` 2 个、`HEVC-HD` 4 个、`HEVC-MIX` 4 个的样本定义数量。
- 默认本地 / CI 门禁运行每个分组 1 个代表样本，记录 confidence、失败归因、FFmpeg 耗时、core 写入耗时和 core 自检耗时。
- 完整 24 样本池属于长跑 release evidence gate，需显式设置 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1` 后运行；缺少 `libx265` 时 HEVC 分组必须记录为 `encoder_unavailable` 环境跳过，不能算通过。
- 2026-06-22 本机默认 smoke 结果：`H264-HD` 为 `confidence_below_threshold`，confidence 0.875；`H264-LT`、`H264-MT`、`HEVC-HD`、`HEVC-MIX` 均为 passed:1.000；`H264-RISK` 记录为 `risk_boundary_expected`。总运行约 8.4 分钟。
- 因 `H264-HD` 仍低于最低 0.950 / 分组均值 0.970，当前 release 状态继续阻断，不能进入商业包装。

## 7. 当前冻结策略

短期发布主线已切换到 `docs/双端现有能力发布计划.md`。完整 24 样本池长跑保留为未来恢复 L3 商业化评估时的 release evidence gate，当前不作为发布前任务，也不阻塞现有双端能力版本发布。冻结期间继续保持不接 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。
