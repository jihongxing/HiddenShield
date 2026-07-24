# Phase I-6 L3 平台成本模型

状态：设计冻结，未进入实现。

本文只把 `watermark-core` / Tauri 测试层的 L3 30 秒平台矩阵转成成本模型输入，不开放 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`，也不宣称 L3 视频画面盲水印已达到可售 SLA。

## 1. 适用范围

当前主战场只覆盖：

- 720p
- 1080p
- 2K

当前平台预算只覆盖：

- 抖音 1080x1920 竖屏 H.264
- 小红书 1080x1440 3:4 H.264
- B站 1920x1080 横屏 H.264
- B站 2560x1440 2K 横屏 H.264
- B站 1920x1080 横屏 HEVC
- B站 2560x1440 2K 横屏 HEVC

4K / 8K 暂不进入当前默认 release 门禁，后续作为大型商业片、院线产品或高阶商业产品线单独设计成本模型。

## 2. 输入证据

证据来源：`l3_platform_timing_budget_records_16frame_seeded_costs`、`l3_bilibili_hevc_texture_aware_records_cost_budget`、`l3_platform_second_pass_transcode_risk_records_outcomes`、`l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight`、`l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes` 和 `l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes`。

统一口径：

- 源视频时长：30 秒
- 采样帧：16
- 策略区域：96
- 区域选择：`SeededRandom`
- 算法核心：`watermark-core` staged DCT API
- 验证方式：真实 FFmpeg 编码 / 解码后交回 `watermark-core` 自检

| 平台画像 | 分辨率 | 编码预算 | 总耗时 | 处理倍率 | confidence |
| --- | ---: | --- | ---: | ---: | ---: |
| 抖音 1080p 竖屏 | 1080x1920 | H.264 4.5Mbps / CRF23 | 36.6s | 1.22x | 0.812 |
| 小红书 1080p 3:4 | 1080x1440 | H.264 6Mbps / CRF20 | 26.4s | 0.88x | 0.875 |
| B站 1080p 横屏 | 1920x1080 | H.264 6Mbps / CRF20 | 36.5s | 1.22x | 1.000 |
| B站 2K 横屏 | 2560x1440 | H.264 8Mbps / CRF23 | 55.9s | 1.86x | 0.938 |

TextureAware 复测结果：

| 平台画像 | 分辨率 | 编码预算 | 总耗时 | 处理倍率 | confidence |
| --- | ---: | --- | ---: | ---: | ---: |
| 抖音 1080p 竖屏 | 1080x1920 | H.264 4.5Mbps / CRF23 | 33.0s | 1.10x | 1.000 |
| 小红书 1080p 3:4 | 1080x1440 | H.264 6Mbps / CRF20 | 26.5s | 0.88x | 1.000 |
| B站 1080p 横屏 | 1920x1080 | H.264 6Mbps / CRF20 | 33.9s | 1.13x | 1.000 |
| B站 2K 横屏 | 2560x1440 | H.264 8Mbps / CRF23 | 55.8s | 1.86x | 1.000 |

B站 HEVC TextureAware 复测结果：

| 平台画像 | 分辨率 | 编码预算 | 总耗时 | 处理倍率 | confidence |
| --- | ---: | --- | ---: | ---: | ---: |
| B站 1080p 横屏 | 1920x1080 | HEVC 4Mbps / CRF20 | 35.1s | 1.17x | 1.000 |
| B站 2K 横屏 | 2560x1440 | HEVC 6.5Mbps / CRF20 | 57.7s | 1.92x | 1.000 |

默认策略切换回归结果：

| 平台画像 | 分辨率 | 编码预算 | 默认策略 | 总耗时 | 处理倍率 | confidence |
| --- | ---: | --- | --- | ---: | ---: | ---: |
| 720p 横屏 | 1280x720 | H.264 2.5Mbps / CRF23 | core default，保守预算 | 17.7s | 0.59x | 1.000 |
| 1080p 横屏 | 1920x1080 | H.264 6Mbps / CRF20 | core default，TranscodeStable | 35.1s | 1.17x | 1.000 |
| 2K 横屏 | 2560x1440 | H.264 8Mbps / CRF23 | core default，TranscodeStable | 56.1s | 1.87x | 1.000 |
| B站 1080p 横屏 | 1920x1080 | HEVC 4Mbps / CRF20 | core default，TranscodeStable | 36.6s | 1.22x | 1.000 |
| B站 2K 横屏 | 2560x1440 | HEVC 6.5Mbps / CRF20 | core default，TranscodeStable | 58.4s | 1.95x | 1.000 |

默认策略真实素材多样性回归结果：

| 内容画像 | 分辨率 | 编码预算 | 默认策略 | 总耗时 | 处理倍率 | confidence |
| --- | ---: | --- | --- | ---: | ---: | ---: |
| 低纹理网格横屏 | 1920x1080 | H.264 6Mbps / CRF20 | core default，TranscodeStable | 50.2s | 1.67x | 1.000 |
| 高细节横屏 | 1920x1080 | H.264 6Mbps / CRF20 | core default，TranscodeStable | 39.7s | 1.32x | 0.938 |
| 高细节竖屏 | 1080x1920 | H.264 6Mbps / CRF20 | core default，TranscodeStable | 40.3s | 1.34x | 1.000 |
| 低纹理网格 2K 横屏 | 2560x1440 | H.264 8Mbps / CRF23 | core default，TranscodeStable | 79.2s | 2.64x | 1.000 |

真实素材风险边界矩阵 / 真实素材风险边界结果：

| 风险画像 | 分辨率 | 编码预算 | 预期结果 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | ---: | ---: | --- |
| 低码率竖屏高细节 | 1080x1920 | H.264 4.5Mbps / CRF23 | 通过但非满置信 | 38.6s | 1.29x | passed:0.875 |
| 极端程序化高频纹理 | 1920x1080 | H.264 6Mbps / CRF20 | 风险边界失败 | 67.0s | 2.23x | failed:self_check_failed |
| 逐帧随机噪声 | 1920x1080 | H.264 6Mbps / CRF20 | 风险边界失败 | 84.3s | 2.81x | failed:self_check_failed |

平台二压风险矩阵结果：

| 二压画像 | 分辨率 | 首次平台预算 | 二次平台预算 | 预期结果 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | --- | ---: | ---: | --- |
| 1080p 竖屏高细节 | 1080x1920 | H.264 6Mbps / CRF20 | H.264 4.5Mbps / CRF23 | 风险边界失败 | 42.4s | 1.41x | failed:self_check_failed |
| 2K 横屏常规纹理 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 压线通过 | 57.1s | 1.90x | passed:0.750 |

平台二压稳定性诊断结果：

| 诊断画像 | 分辨率 | 采样帧 / 区域 | 二次平台预算 | 总耗时 | self-check |
| --- | ---: | --- | --- | ---: | --- |
| 1080p 竖屏高细节加帧 | 1080x1920 | 20 帧 / 96 区域 | H.264 4.5Mbps / CRF23 | 53.7s | failed:self_check_failed |
| 1080p 竖屏高细节加区域 | 1080x1920 | 16 帧 / 128 区域 | H.264 4.5Mbps / CRF23 | 43.4s | failed:self_check_failed |
| 1080p 竖屏高细节 TranscodeStable | 1080x1920 | 16 帧 / 96 区域 | H.264 4.5Mbps / CRF23 | 41.8s | passed:0.812 |
| 2K 横屏常规纹理加帧 | 2560x1440 | 20 帧 / 96 区域 | H.264 6.5Mbps / CRF24 | 77.3s | passed:0.950 |

TranscodeStable 平台泛化结果：

| 平台画像 | 分辨率 | 编码预算 | 二压预算 | 总耗时 | self-check |
| --- | ---: | --- | --- | ---: | --- |
| 720p H.264 二压风险边界 | 1280x720 | H.264 4Mbps / CRF21 | H.264 3Mbps / CRF23 | 16.1s | failed:self_check_failed |
| B站 1080p 横屏 H.264 | 1920x1080 | H.264 6Mbps / CRF20 | H.264 4.5Mbps / CRF23 | 47.6s | passed:1.000 |
| B站 2K 横屏 H.264 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 62.8s | passed:0.875 |
| B站 1080p 横屏 HEVC | 1920x1080 | HEVC 4Mbps / CRF20 | HEVC 3.2Mbps / CRF24 | 47.0s | passed:1.000 |
| B站 2K 横屏 HEVC | 2560x1440 | HEVC 6.5Mbps / CRF20 | HEVC 5.2Mbps / CRF24 | 63.5s | passed:1.000 |

默认 TranscodeStable 平台二压成本权重复核结果：

| 平台画像 | 分辨率 | 编码预算 | 二压预算 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | ---: | ---: | --- |
| 720p H.264 默认二压风险边界 | 1280x720 | H.264 4Mbps / CRF21 | H.264 3Mbps / CRF23 | 16.7s | 0.56x | failed:self_check_failed |
| B站 1080p 横屏 H.264 | 1920x1080 | H.264 6Mbps / CRF20 | H.264 4.5Mbps / CRF23 | 47.1s | 1.57x | passed:1.000 |
| B站 2K 横屏 H.264 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 65.1s | 2.17x | passed:0.875 |
| B站 1080p 横屏 HEVC | 1920x1080 | HEVC 4Mbps / CRF20 | HEVC 3.2Mbps / CRF24 | 55.2s | 1.84x | passed:1.000 |
| B站 2K 横屏 HEVC | 2560x1440 | HEVC 6.5Mbps / CRF20 | HEVC 5.2Mbps / CRF24 | 64.9s | 2.16x | passed:1.000 |

默认 TranscodeStable 真实内容二压结果：

| 内容画像 | 分辨率 | 编码预算 | 二压预算 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | ---: | ---: | --- |
| 1080p 高细节横屏 H.264 | 1920x1080 | H.264 6Mbps / CRF20 | H.264 4.5Mbps / CRF23 | 45.2s | 1.51x | passed:1.000 |
| 1080p 高细节竖屏 H.264 | 1080x1920 | H.264 6Mbps / CRF20 | H.264 4.5Mbps / CRF23 | 43.0s | 1.43x | passed:1.000 |
| 2K 常规纹理 H.264 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 58.7s | 1.96x | passed:0.875 |
| 2K 高细节 H.264 风险边界 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 75.1s | 2.50x | failed:self_check_failed |

2K 高细节 H.264 二压预算策略结果：

| 候选策略 | 分辨率 | 编码预算 | 二压预算 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | ---: | ---: | --- |
| 加帧：20 帧 / 96 区域 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 77.7s | 2.59x | failed:self_check_failed |
| 加区域：16 帧 / 128 区域 | 2560x1440 | H.264 8Mbps / CRF23 | H.264 6.5Mbps / CRF24 | 64.8s | 2.16x | failed:self_check_failed |
| 提高码率：16 帧 / 96 区域 | 2560x1440 | H.264 10Mbps / CRF21 | H.264 8Mbps / CRF23 | 66.2s | 2.21x | passed:0.875 |

2K 高码率内容候选结果：

| 内容候选 | 分辨率 | 编码预算 | 二压预算 | 总耗时 | 处理倍率 | self-check |
| --- | ---: | --- | --- | ---: | ---: | --- |
| 高细节 H.264 | 2560x1440 | H.264 10Mbps / CRF21 | H.264 8Mbps / CRF23 | 69.1s | 2.30x | passed:0.875 |
| 低纹理 H.264 | 2560x1440 | H.264 10Mbps / CRF21 | H.264 8Mbps / CRF23 | 101.3s | 3.38x | passed:1.000 |
| 运动纹理 H.264 | 2560x1440 | H.264 10Mbps / CRF21 | H.264 8Mbps / CRF23 | 72.5s | 2.42x | passed:1.000 |
| 高细节 HEVC | 2560x1440 | HEVC 8Mbps / CRF20 | HEVC 6.5Mbps / CRF24 | 73.8s | 2.46x | passed:1.000 |

说明：

- 处理倍率 = 本机测试总耗时 / 30 秒源视频时长。
- 4.5Mbps 仍是 1080p 主流地板，但平台矩阵已暴露小红书 3:4 与 B站 1080p 在 4.5Mbps / CRF23 下不稳。
- 因此 6Mbps 是当前 1080p 平台候选预算，不是商业 SLA。
- HEVC TextureAware 在 B站 1080p / 2K 两档保持 confidence 1.000，耗时接近对应 H.264 TextureAware 档位；当前只作为 staged 成本模型证据，不进入用户可见视频能力。
- 默认策略切换不对所有尺寸一刀切：720p 仍走 core default 的保守预算，1080p / 2K 默认 TranscodeStable。这样保留最低主战场档稳定性，同时把商业价值更高且二压更敏感的 1080p / 2K 作为默认策略主线。
- 多样性矩阵里的 1080p 高细节横屏样本 confidence 为 0.938，已通过当前 0.75 阈值但不是满置信，后续真实素材回归应优先扩充高细节横屏和竖屏样本。极端逐帧噪声和程序化高频纹理已暴露为压缩不友好的风险边界，不进入当前主流硬门禁。
- 风险边界矩阵固定了两类失败不应被包装成可售能力：极端程序化高频纹理和逐帧随机噪声。低码率竖屏高细节虽然通过，但 confidence 0.875 说明 4.5Mbps 只适合作为低档风险口径，不适合作为 1080p 竖屏商业默认预算。
- 平台二压风险矩阵进一步证明：1080p 竖屏高细节从 6Mbps 再二压到 4.5Mbps 会稳定 `self_check_failed`，不能作为当前默认商业承诺；2K 二压当前只是阈值线上的通过证据，不能按满置信 SLA 包装。
- 平台二压稳定性诊断显示：1080p 竖屏高细节不是简单加帧或加区域可以解决的问题；2K 20 帧诊断可把二压 confidence 提升到 0.950，但总耗时从约 57.1s 上升到约 77.3s，需要进入成本权重评估后才能作为候选预算。
- `TranscodeStable` 是首个面向二压稳态的核心区域候选：它在 `watermark-core` 内选择纹理足够但不过度高频的候选块。1080p TranscodeStable 可在不加帧、不加区域的情况下恢复到 passed:0.812，总耗时约 41.8s；当前已成为 1080p / 2K staged 默认路径，但仍不是商业 SLA。
- TranscodeStable 平台泛化矩阵显示：720p 真实二压仍是当前失败边界，不能包装成已解决能力；1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压全部通过。其中 1080p H.264 和 HEVC 两档 confidence 1.000，2K H.264 confidence 0.875，2K HEVC confidence 1.000，支撑 1080p / 2K 默认策略从 TextureAware 切到 TranscodeStable，并提示 2K H.264 仍需要真实内容样本扩展。
- 默认 TranscodeStable 二压矩阵首次运行暴露 1080p H.264 会因 task_id / seed 抽样漂移而 `self_check_failed`；`watermark-core` 已将 TranscodeStable 收紧为按核心派生的稳定候选确定性取点。重跑后默认非二压矩阵五档 confidence 均为 1.000；默认二压矩阵中 1080p H.264 为 1.000，2K H.264 为 0.875，HEVC 两档为 1.000。当前不需要为 TranscodeStable 单独提高 `strategy_weight`，但二压路径的总体处理倍率应进入平台成本权重复核。
- 默认 TranscodeStable 真实内容二压矩阵显示：1080p 高细节横屏 / 竖屏都可满置信通过，2K 常规纹理 H.264 通过但只有 0.875，2K 高细节 H.264 在 8Mbps -> 6.5Mbps 二压下稳定 `self_check_failed`。因此 2K 高细节不能进入当前默认商业承诺，下一步应只在 `watermark-core` / 测试层评估 2K 高细节的预算策略，不接 UI、不开放云任务。
- 2K 高细节预算策略矩阵显示：单纯加帧或加区域都不能救回 8Mbps -> 6.5Mbps 二压，反而提高耗时；把候选预算提高到 10Mbps -> 8Mbps 后可以通过，但 confidence 仍只有 0.875。因此 2K 高细节的当前方向应是码率预算分档和真实内容样本扩展，而不是盲目增加采样密度。
- 2K 高码率内容候选矩阵显示：H.264 低纹理和运动纹理已到 confidence 1.000，HEVC 高细节在 8Mbps -> 6.5Mbps 下达到 confidence 1.000；但 H.264 高细节在 10Mbps -> 8Mbps 仍只有 confidence 0.875。因此 2K 高码率候选可以进入 release 样本池设计，但 H.264 高细节仍不能按满置信 SLA 包装。
- 2K 高码率 release 样本池与阈值策略已冻结在 `docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md`：首版至少 24 个 2K 样本；H.264 使用 10Mbps -> 8Mbps，非风险样本最低 confidence 0.950，高细节分组均值 >= 0.970，低纹理 / 运动纹理分组均值 >= 0.980；HEVC 使用 8Mbps -> 6.5Mbps，非风险样本最低 confidence 0.970，分组均值 >= 0.990。该门槛仍只用于 release 样本池，不进入 UI、账本或销售 SLA。
- 2K 高码率 release 样本池门禁已落地为 `l3_2k_high_bitrate_release_sample_pool_records_thresholds`：默认 smoke 运行每个分组 1 个代表样本，完整 24 样本池需设置 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1`。2026-06-22 本机默认 smoke 约 8.4 分钟，继续由 `H264-HD` confidence 0.875 触发 `confidence_below_threshold` 阻断；该结果不进入 UI、账本或销售 SLA。

## 3. 内部成本单位

内部建模使用 `l3_cost_units`，不得直接等同于用户账单的 `video_minutes`。

建议首版计算：

```text
platform_seconds = measured_total_ms / 1000
processing_ratio = platform_seconds / source_duration_seconds
video_minutes = ceil(source_duration_seconds / 60)
l3_cost_units = ceil(video_minutes * platform_weight * strategy_weight)
```

首版 `platform_weight`：

| 平台画像 | platform_weight | 依据 |
| --- | ---: | --- |
| 1080p 竖屏 / 横屏 H.264 | 1.25 | 30 秒处理倍率约 0.83x 到 1.12x，保留平台差异与队列余量 |
| 2K 横屏 H.264 | 2.00 | 30 秒处理倍率约 1.72x，计算资源明显高于 1080p |
| 1080p 横屏 HEVC | 1.25 | 30 秒 TextureAware 处理倍率约 1.17x，暂不高于 1080p H.264 权重 |
| 2K 横屏 HEVC | 2.00 | 30 秒 TextureAware 处理倍率约 1.92x，接近 2K H.264 权重上沿 |

首版 `strategy_weight`：

| 策略 | strategy_weight | 依据 |
| --- | ---: | --- |
| 16 帧 / 96 区域 / seeded random | 1.00 | 当前默认 staged baseline，四个平台画像均通过，但部分 confidence 低于 TextureAware |
| 16 帧 / 96 区域 / TextureAware | 1.00 | 四个平台画像均通过且 confidence 1.000；耗时未高于现有 1080p / 2K 平台权重，因此 `strategy_weight` 暂定为 1.00 |
| 16 帧 / 96 区域 / TranscodeStable | 1.00 | 默认二压矩阵通过 1080p / 2K H.264 / HEVC，稳定性来自 core 候选质量而不是增加帧数或区域数；当前成本差异计入平台二压倍率，不额外提高策略权重 |

## 4. 扣费边界

未来如果 L3 进入 Studio / Enterprise 任务包装，用户可见扣费仍以 `video_minutes` 为单位，并必须满足：

- 策略包生成成功。
- 客户端或云端执行成功。
- 成品视频完成后自检通过。
- 云端收据固化成功。
- 只有成功完成后才扣额度。

不得扣费：

- 用户取消。
- 策略生成失败。
- 解码 / 编码失败。
- `self_check_failed`。
- 客户端渲染失败。
- 云端服务异常。

`l3_cost_units` 只能用于内部容量规划、定价测算和套餐边界设计，不能在当前阶段进入 UI、后端账本或用户报告。

## 5. 不进入当前实现

当前阶段禁止：

- 新增 `cloud_video_processing` 真实任务。
- 在后端新增 L3 扣费 ledger。
- 把 staged 测试结果写成用户可见 SLA。
- 把 L2 视频指纹存证包装成 L3 视频画面盲水印。
- 在 `watermark-core` 之外实现 DCT / bitstream / ECC / 自检算法。
- 把 4K / 8K 纳入默认发布门禁。

## 6. 当前冻结策略

短期发布主线已切换到 `docs/双端现有能力发布计划.md`。本成本模型继续作为内部容量和未来定价储备，不进入 UI、后端账本、用户报告或 `video_minutes` 扣费；继续保持不接 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。完整 24 个 2K 样本长跑只在未来恢复 L3 商业化评估时执行，当前不作为发布前任务。
