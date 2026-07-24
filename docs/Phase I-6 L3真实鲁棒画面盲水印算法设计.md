# Phase I-6 L3 真实鲁棒画面盲水印算法设计

状态：设计冻结，未进入实现

本文档定义 L3 视频画面盲水印从当前 `watermark-core` 合成帧 LSB spike 进入真实鲁棒算法前的核心算法方案。本文不开放桌面端、移动端、后端或云端任务入口，也不改变 `cloud_video_processing` 的商业边界。

## 1. 设计结论

- L3 真实画面盲水印算法仍只能位于 `watermark-core`。
- 当前 `Luma8SyntheticV1` / LSB 合成帧实现只作为 API、错误码、自检和性能门禁，不得作为商业画面水印算法。
- 真实算法首版采用亮度平面块域频域方案：Y 平面 8x8 DCT 中频系数相对关系写入，策略包只选择区域、强度、冗余和目标 profile，不下发完整算法。
- 提取必须支持无原始视频参考的盲提取；可选策略提示只能缩小搜索范围，不能成为唯一可读条件。
- L2 指纹存证不能替代 L3 水印命中；L3 提取必须返回 `WatermarkPayload` 或结构化失败码。

## 2. 算法路线

首版真实算法命名建议：

```rust
VideoVisualProfile::LumaDctMidBandV1
```

写入路径：

```text
VideoFramePlane(Y)
  -> 按策略选择稳定区域
  -> 切分 8x8 luma block
  -> 对候选 block 执行 DCT
  -> 在中频系数 pair 上写入 payload bit
  -> 逆 DCT 回写 Y plane
  -> 多帧、多区域、多冗余组重复
```

提取路径：

```text
VideoFramePlane(Y)
  -> 按策略 hint 或盲扫候选区域
  -> 8x8 block DCT
  -> 读取中频系数 pair 相对关系
  -> 重组 bitstream
  -> 同步标记定位 payload
  -> ECC 恢复
  -> decode_payload
```

中频候选：

- 避免 DC 和低频，降低可见风险。
- 避免高频末端，提升二压、缩放和平台转码后存活率。
- 首版候选 pair 固定为 `[(2,3),(3,2)]`、`[(2,4),(4,2)]`、`[(3,4),(4,3)]`，实际 pair 由策略 seed 派生选择。

写入规则：

- bit=1 时保证 `coeff_a - coeff_b >= delta`。
- bit=0 时保证 `coeff_b - coeff_a >= delta`。
- `delta` 由目标 profile、局部纹理强度和策略强度共同决定。
- 写入后必须限制像素回写范围，避免明显亮度跳变。

## 3. Payload、同步与纠错

Payload 继续使用现有 `WatermarkPayload` 和 `encode_payload` / `decode_payload`，不得新增平台层 payload 编码。

真实 bitstream：

```text
sync_marker_v1
  + payload_version
  + encoded_payload
  + ecc_parity
  + redundancy_group_id
```

同步要求：

- `sync_marker_v1` 由 `watermark-core` 固定，不由桌面端、移动端或云端生成。
- 同步标记必须在每个冗余组重复出现。
- 提取时先定位同步标记，再恢复 payload；不得假设帧序号或本地路径存在。

纠错要求：

- 首版使用 `watermark-core` 内部轻量纠错，不引入平台层纠错逻辑。
- 至少支持局部 block 丢失、少量 bit flip 和部分帧缺失。
- ECC 失败返回 `visual_extract_failed`，自检未达阈值返回 `self_check_failed`。

## 4. 策略与特征选择

`VideoFeatureBundle` 需要从合成字段扩展为真实特征摘要，但仍不得保存原始帧：

- `frame_count`
- `duration_ms`
- `sampled_frame_indices`
- `scene_cut_digest`
- `luma_texture_score`
- `motion_stability_score`
- `safe_region_grid`
- `source_video_sha256`
- `feature_digest`

策略生成规则：

- 选择纹理适中、运动稳定、非纯色、非字幕密集区域。
- 每个 payload 至少写入 3 个冗余组。
- 每个冗余组跨越不同时间片，避免单段裁剪导致全部丢失。
- 竖屏 / 横屏 / 方形 profile 使用不同安全区域模板。
- 策略必须绑定 `task_id`、`watermark_uid`、源摘要、目标 profile、过期时间和 `strategy_digest`。

策略包仍只包含执行计划，不包含服务端主密钥、长期派生密钥或全局嵌入规律。

### 4.1 真实视频帧解码边界

解码器输出进入 `watermark-core` 的唯一边界为：

- `DecodedVideoLumaPlane`
- `VideoLumaBitDepth`
- `VideoLumaColorRange`
- `video_frame_plane_from_decoded_luma`

平台层只能提供解码后的 Y plane、宽高、`stride_samples`、bit depth 和 color range。`watermark-core` 负责：

- 校验 width / height 至少为 8x8。
- 校验 `stride_samples >= width`。
- 校验 `samples.len() >= stride_samples * height`。
- 将 8-bit / 10-bit / 12-bit 的 full 或 limited range luma 统一归一化为 8-bit Y。
- 丢弃 stride padding，只把可见区域写入 `VideoFramePlane`。
- 只允许真实解码 Y plane 进入 `LumaDctMidBandV1`，不得进入 `Luma8SyntheticV1`。

固定 fixture：

- `video_visual_decoded_y_plane_normalizes_limited_10_bit_with_stride`
- `video_visual_decoded_y_plane_rejects_short_buffer`
- `video_visual_decoded_y_plane_rejects_synthetic_profile`
- `video_visual_fixed_y_plane_fixture_roundtrips_dct_payload`

这些 fixture 只验证 core 的 Y-plane 边界和 DCT roundtrip，不代表已经接入真实视频文件、FFmpeg pipeline、桌面 UI、移动端或云任务。

## 5. 鲁棒性目标

首版进入平台包装前，`watermark-core` 必须用固定 fixture 覆盖：

- H.264 / H.265 二压模拟。
- 分辨率缩放到 720p / 1080p。
- 中心裁剪和边缘裁剪。
- 码率下降。
- 轻微亮度 / 对比度变化。
- 少量帧丢失。
- 画面局部遮挡。

验收目标：

- 自检阈值默认 `0.82`。
- 同一 payload 至少 3 个冗余组中 2 个可恢复时，判定通过。
- 只有 L3 payload 解码成功才算水印命中。
- L2 相似性证据只能作为报告辅助栏位。

## 6. 性能与复杂度预算

复杂度目标：

```text
O(sampled_frames * candidate_blocks_per_frame * selected_coeff_pairs)
```

当前 `watermark-core` staged API / fixture 预算：

| Tier | 采样帧 | 候选 block 上限 | 中频 pair 数 | 估算操作 | fixture roundtrip 上限 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Small | 4 | 512 / frame | 3 | 6,144 | < 1.5s |
| Standard | 8 | 768 / frame | 3 | 18,432 | < 3s |
| High | 12 | 1,024 / frame | 3 | 36,864 | < 6s |

这些数值已由 `VideoVisualComplexityTier`、`VideoVisualComplexityBudget`、`derive_video_visual_complexity_budget` 和 `sample_video_visual_frame_indices` 在 `watermark-core` 中固定。当前预算只约束 core synthetic Y-plane fixture，不能作为商业 SLA。

未来真实视频商业目标预算：

| 场景 | 采样帧 | 分辨率 | 候选 block 上限 | 目标 |
| --- | ---: | ---: | ---: | --- |
| 小视频 | 24 | 720p | 3,000 / frame | 本机写入 + 自检 < 8s |
| 标准视频 | 60 | 1080p | 5,000 / frame | 本机写入 + 自检 < 25s |
| 高阶视频 | 120 | 4K downsample strategy | 8,000 / frame | 需 Studio Beta / Enterprise 任务预算 |

实现约束：

- 桌面端可使用 FFmpeg 解码 / 编码，但 DCT、水印 bit 写入、提取和自检必须在 `watermark-core`。
- 移动端首期不做本地 L3 写入。
- 云端不能另写算法，只能部署或调用 `watermark-core` 产物。
- 性能基线必须在 `watermark-core` 单测或 bench 中固定，且按帧数、分辨率和区域数量分层。

## 7. 失败归因

真实算法必须继续使用结构化错误码：

- `feature_bundle_invalid`：特征包缺少必要采样、摘要或 profile。
- `strategy_invalid`：策略签名、摘要、区域、强度或过期时间不合法。
- `unsupported_video_profile`：Y 平面、bit depth、色彩空间或目标 profile 不支持。
- `visual_extract_failed`：无法恢复 payload 或 ECC 失败。
- `self_check_failed`：可提取帧比例低于阈值。

发布门禁和商业任务不得把这些失败混成“视频处理失败”。

## 8. 从 Synthetic Spike 到真实算法的替换边界

保留：

- `VideoFramePlane`
- `VideoFeatureBundle`
- `VideoVisualStrategy`
- `VideoVisualSelfCheckResult`
- `build_video_visual_payload`
- `derive_video_visual_strategy`
- `embed_video_visual_frame`
- `embed_video_visual_frames`
- `extract_video_visual_watermark`
- `extract_video_visual_watermark_from_frames`
- `self_check_video_visual_frames`
- L3 错误码

替换：

- `Luma8SyntheticV1` 内部 LSB 写入逻辑。
- 合成帧容量判断。
- 合成扰动 fixture。

新增：

- `LumaDctMidBandV1` profile。
- DCT block transform。
- sync marker / ECC。
- 真实 profile fixture。
- 真实视频解码后 Y plane fixture。
- 真实容器解码到 DCT staged roundtrip 测试桥。
- 受控编码回写后 DCT 自检门禁。
- 有损压缩边界：CRF 12 当前必须通过自检，CRF 38 当前必须返回 `self_check_failed`。
- 单帧失败后的多帧 bitstream 融合提取：同策略多帧按位多数投票后复用 sync / ECC / payload 解码，解决分散 bit 损坏，但不改变 payload 格式。
- 帧内 bitstream 重复副本：同一策略帧容量足够时最多写入 3 份 bitstream，提取时与跨帧副本共同投票。
- 目标平台二压矩阵：H.264 CRF 18 / CRF 23 和 384p 缩放再回 512p 后二压当前必须通过自检，CRF 38 当前必须返回 `self_check_failed`。
- 主战场分辨率矩阵：720p / 1080p / 2K 经 H.264 CRF 23 / CRF 28 二压后必须通过自检，中心裁切后补边再 CRF 23 二压也必须通过自检。
- 首版平台 profile 矩阵：抖音 9:16 H.264 High CRF18 覆盖 720p / 1080p，小红书 3:4 H.264 High CRF17 覆盖 720p / 1080p，B站 16:9 H.264 High CRF18 覆盖 720p / 1080p / 2K；平台 profile 矩阵必须真实经过 FFmpeg 编码 / 解码后再由 `watermark-core` 自检。
- 主流码率地板矩阵：720p H.264 2.5Mbps、1080p H.264 4.5Mbps、2K H.264 8Mbps 必须通过自检；低于主流地板的码率只记录风险边界，不作为当前算法优化目标。
- 30 秒商业采样性能矩阵：30 秒 30fps 源视频抽 12 帧进入 staged DCT 流程，必须分段记录 FFmpeg 源生成 / 抽样、core 写入、采样帧码率回写、core 自检和总耗时；720p H.264 2.5Mbps、1080p H.264 4.5Mbps 和 2K H.264 8Mbps 已在 12 个采样帧 / 96 个策略区域下通过。
- B站 HEVC 主流码率地板矩阵：测试必须先探测 `libx265`；可用时以同一 30 秒 / 12 采样帧 / 96 策略区域口径验证 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps。当前两档均通过，confidence 为 1.000。
- B站 H.264 / HEVC 成本对照矩阵：测试必须在同一 30 秒 / 12 采样帧 / 96 策略区域口径下同时记录 1080p H.264 4.5Mbps、1080p HEVC 4Mbps、2K H.264 8Mbps 与 2K HEVC 6.5Mbps 的分段耗时、总耗时和 confidence，用于后续商业成本模型，不作为可售 SLA。
- 2K H.264 策略密度预算矩阵：针对成本对照中 2K H.264 8Mbps / 96 策略区域 confidence 压线问题，测试必须在同一 30 秒 / 12 采样帧口径下对照 96 / 128 / 160 策略区域，记录 confidence 曲线和额外耗时。当前实测 96 区域 confidence 0.917，高于 128 / 160 区域的 0.833，说明不能靠单纯增加策略区域数解决 2K H.264 稳定性。
- 2K H.264 抽帧数量预算矩阵：针对策略密度预算中“增加区域无效”的结论，测试必须在同一 30 秒 / 2K H.264 8Mbps / 96 策略区域口径下对照 12 / 16 / 20 采样帧，记录 confidence 曲线和额外耗时。当前实测 16 帧 confidence 0.812，高于 12 帧 0.750 和 20 帧 0.800，16 帧暂作为 2K H.264 候选预算点。
- 2K H.264 区域质量预算矩阵：`watermark-core` 显式提供 `VideoVisualRegionSelectionMode`，默认 `SeededRandom` 保持现有行为；core 同时派生 `VideoVisualTextureHint`，让 `TextureAware` 策略在核心内选择高纹理 8x8 block，平台层不得自行实现选点算法。测试层在同一 30 秒 / 2K H.264 8Mbps / 16 采样帧 / 96 策略区域口径下对照 `SeededRandom`、`CenterSafeGrid`、`DistributedGrid`、`TextureAware`，记录 confidence 曲线和额外耗时。当前实测 `SeededRandom` 通过且 confidence 0.875，`CenterSafeGrid` 和 `DistributedGrid` 均 `self_check_failed`，`TextureAware` 通过且 confidence 1.000，总耗时约 55.6s。说明简单几何网格策略不优于现有 seeded random，纹理感知候选是下一阶段默认策略的优先候选，但仍不是可售 SLA。
- 平台矩阵耗时预算：测试层固定 30 秒 / 16 采样帧 / 96 策略区域 / seeded random 口径，覆盖抖音 1080x1920 H.264 4.5Mbps、小红书 1080x1440 H.264 6Mbps、B站 1920x1080 H.264 6Mbps 和 B站 2560x1440 H.264 8Mbps。当前 seeded random 复测总耗时约 36.6s、26.4s、36.5s、55.9s，confidence 分别为 0.812、0.875、1.000、0.938。4.5Mbps 作为 1080p 主流地板仍保留，但平台矩阵暴露小红书 3:4 与部分 1080p 画像在低预算下不稳，因此 6Mbps 成为 1080p 平台候选预算。
- TextureAware 平台矩阵耗时预算：同一 30 秒 / 16 采样帧 / 96 区域口径下，TextureAware 在抖音 1080p、小红书 1080p、B站 1080p、B站 2K 全部通过，confidence 均为 1.000；总耗时约 33.0s、26.5s、33.9s、55.8s。该结果支持 TextureAware 成为 staged 默认策略候选，且当前 `strategy_weight` 暂不高于 seeded random。
- B站 HEVC TextureAware 对照矩阵：新增 `l3_bilibili_hevc_texture_aware_records_cost_budget`，先探测 `libx265`，可用时以同一 30 秒 / 16 采样帧 / 96 策略区域 / TextureAware 口径验证 B站 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps；当前两档均通过，confidence 为 1.000，总耗时约 35.1s、57.7s。HEVC 结果支持 TextureAware 继续作为 staged 默认策略候选，但仍不是可售 SLA。
- 默认策略切换回归矩阵：`watermark-core` 默认策略新增主战场尺寸门槛，720p 仍保留 core default 的保守预算，1080p / 2K 默认进入 TranscodeStable；新增 `l3_default_transcode_stable_h264_hevc_regression_records_cost_budget`，同一 30 秒真实 FFmpeg 编码 / 解码后，720p H.264 2.5Mbps / 12 帧通过且 confidence 1.000，总耗时约 17.7s；1080p H.264 6Mbps / 16 帧通过且 confidence 1.000，总耗时约 35.1s；2K H.264 8Mbps / 16 帧、1080p HEVC 4Mbps / 16 帧、2K HEVC 6.5Mbps / 16 帧均通过且 confidence 1.000，总耗时约 56.1s、36.6s、58.4s。该结果说明默认 TranscodeStable 可进入下一轮平台二压回归，但仍不是可售 SLA。
- 默认策略真实素材多样性回归矩阵：新增 `l3_default_strategy_texture_diversity_records_cost_budget`，以受控 FFmpeg lavfi 源模拟低纹理网格、高细节横屏、高细节竖屏和 2K 低纹理网格，仍走 `watermark-core` core default。当前四档均通过；confidence 分别为 1.000、0.938、1.000、1.000，总耗时约 50.2s、39.7s、40.3s、79.2s。逐帧随机噪声和程序化高频纹理已暴露为压缩不友好风险边界，不作为当前主流硬门禁。
- 真实素材风险边界矩阵：新增 `l3_default_strategy_real_content_risk_boundary_records_outcomes`，固定边界分类而不是盲目要求全过。当前低码率竖屏高细节通过但 confidence 0.875；极端程序化高频纹理和逐帧随机噪声均稳定归因为 `self_check_failed`。这说明 4.5Mbps 可作为竖屏高细节低档风险口径，但不应作为 1080p 竖屏商业默认预算；极端高频和逐帧噪声不得被包装成当前可售能力。
- 平台二压风险矩阵：新增 `l3_platform_second_pass_transcode_risk_records_outcomes`，固定先生成保护采样帧视频、再按更低平台预算二次转码、最后交回 `watermark-core` 自检的路径。当前 1080p 竖屏高细节 6Mbps 再二压到 4.5Mbps 稳定归因为 `self_check_failed`，2K 8Mbps 再二压到 6.5Mbps 当前压线通过，confidence 0.750。该矩阵说明二压是当前 L3 staged 算法的最敏感商业边界，2K 通过也不能包装成满置信 SLA。
- 平台二压稳定性诊断矩阵：新增 `l3_platform_second_pass_stability_diagnostics_records_budget_curve`，在不接 UI 的前提下观察采样帧、策略区域和核心区域模式对二压边界的影响。1080p 竖屏高细节 20 帧 / 96 区域仍 `self_check_failed`，16 帧 / 128 区域仍 `self_check_failed`，说明它不是简单加帧或加区域能解决；新增 `TranscodeStable` 区域模式后，1080p 竖屏高细节 TranscodeStable 16 帧 / 96 区域二压通过，confidence 0.812；2K 20 帧 / 96 区域二压 confidence 提升到 0.950，但总耗时上升到约 77.3s。
- TranscodeStable 平台泛化矩阵：新增 `l3_transcode_stable_second_pass_platform_matrix_records_generalization`，以 16 帧 / 96 区域固定 720p 真实二压失败边界，并验证 1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压全部通过。当前 720p H.264 4Mbps -> 3Mbps 真实二压仍为 `self_check_failed`；在稳定候选确定性取点收紧后，1080p H.264、2K H.264、1080p HEVC、2K HEVC confidence 分别为 1.000、0.875、1.000、1.000。该结果说明 `TranscodeStable` 不只是救 1080p 竖屏高细节的特例，而是可进入 1080p / 2K 默认策略切换候选；720p 二压仍是当前失败边界，不得包装成已解决能力。
- 默认 TranscodeStable 平台二压成本权重复核矩阵：新增 `l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight`，用 core default 路径而不是显式模式选择来验证 1080p / 2K 二压。首次运行暴露 1080p H.264 因 seed 抽样漂移 `self_check_failed`；`watermark-core` 随后把 TranscodeStable 收紧为稳定候选确定性取点。重跑后 720p H.264 4Mbps -> 3Mbps 仍为 `self_check_failed`；1080p H.264、2K H.264、1080p HEVC、2K HEVC confidence 分别为 1.000、0.875、1.000、1.000，总耗时约 47.1s、65.1s、55.2s、64.9s。该结果说明 1080p / 2K 默认路径二压已经具备继续扩真实内容样本的资格，但仍不是可售 SLA。
- 默认 TranscodeStable 真实内容二压矩阵：新增 `l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes`，同一 30 秒 / 16 帧 / 96 区域 / core default 口径下覆盖 1080p 高细节横屏、1080p 高细节竖屏、2K 常规纹理和 2K 高细节 H.264 二压。当前 1080p 横屏、1080p 竖屏均通过且 confidence 1.000；2K 常规纹理通过但 confidence 0.875；2K 高细节在 8Mbps -> 6.5Mbps 二压下稳定返回 `self_check_failed`。该结果把 2K 高细节明确为当前主战场内的商业风险边界，不能包装成默认可售能力。
- 2K 高细节 H.264 二压预算策略矩阵：新增 `l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes`，同一高细节源下对照 20 帧 / 96 区域、16 帧 / 128 区域和 10Mbps -> 8Mbps。当前 20 帧 / 96 区域仍 `self_check_failed`，总耗时约 77.7s；16 帧 / 128 区域仍 `self_check_failed`，总耗时约 64.8s；提高到 10Mbps -> 8Mbps 后通过但 confidence 0.875，总耗时约 66.2s。该结果说明 2K 高细节的关键是码率预算分档，不是简单加帧或加区域。
- 2K 高码率内容候选矩阵：新增 `l3_2k_high_bitrate_content_candidate_matrix_records_outcomes`，同一 30 秒 / 16 帧 / 96 区域 / core default 口径下覆盖 H.264 高细节、H.264 低纹理、H.264 运动纹理和 HEVC 高细节。当前 H.264 高细节 10Mbps -> 8Mbps 通过但 confidence 0.875；低纹理和运动纹理 H.264 在 10Mbps -> 8Mbps 下均达到 confidence 1.000；HEVC 高细节 8Mbps -> 6.5Mbps 达到 confidence 1.000。该结果支持 2K 高码率候选进入 release 样本池设计，但 H.264 高细节仍不能按满置信商业 SLA 包装。
- 2K 高码率 release 样本池与阈值策略：新增 `docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md`，固定首版至少 24 个 2K 样本，H.264 使用 10Mbps -> 8Mbps，HEVC 使用 8Mbps -> 6.5Mbps；H.264 非风险样本最低 confidence 0.950，HEVC 非风险样本最低 confidence 0.970；`payload_mismatch`、非风险样本 `self_check_failed`、`visual_extract_failed` 和 `confidence_below_threshold` 均为 release-blocking 失败；极端高频、逐帧噪声、字幕密集等样本只能归因为 `risk_boundary_expected`，不得计入通过率或商业包装。
- 2K 高码率 release 样本池门禁：新增 `l3_2k_high_bitrate_release_sample_pool_records_thresholds`，固定 24 个 2K fixture 定义和分组数量；默认运行每个分组 1 个代表样本，完整池通过 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1` 显式长跑。2026-06-22 默认 smoke 中 `H264-HD` 仍为 confidence 0.875 并归因为 `confidence_below_threshold`，release 状态继续阻断；`H264-LT`、`H264-MT`、`HEVC-HD`、`HEVC-MIX` 代表样本通过，`H264-RISK` 正确归因为 `risk_boundary_expected`。
- L3 30 秒平台成本模型：新增 `docs/Phase I-6 L3平台成本模型.md`，将平台矩阵转成内部 `l3_cost_units`、`platform_weight` 和 `strategy_weight` 口径；该模型只用于容量规划、定价测算和套餐边界设计，不进入 UI、后端账本或用户报告，也不触发 `video_minutes` 扣费。
- 4K / 8K 暂不进入当前默认 release 门禁，后续作为大型商业片、院线产品或高阶商业产品线单独设计性能预算、成本模型和验收矩阵。

当前已导出的 `watermark-core` staged API：

- `embed_video_visual_dct_frames`
- `extract_video_visual_dct_from_frames`
- `self_check_video_visual_dct_frames`

这些 API 只允许平台层把已解码的 `VideoFramePlane`、`VideoVisualStrategy` 和正式 `WatermarkPayload` 交给核心执行；平台层仍不得实现 DCT、bitstream、ECC 或自检算法。

禁止：

- 在桌面端 Tauri、移动端 Flutter/Rust bridge、后端 handler 或脚本中实现 DCT / QIM / bitstream / ECC。
- 将 synthetic LSB spike 标记为 Studio Beta / Enterprise 能力。
- 在自检前扣减 `video_minutes`。

## 9. 实现顺序

1. 在 `watermark-core` 新增 `LumaDctMidBandV1` profile 和纯 Rust 8x8 DCT helper。
2. 在 core 内实现 bit 写入 / 读取的 DCT block 单元测试。
3. 加入 sync marker 和 ECC fixture。
4. 用 synthetic Y plane 生成压缩 / 缩放 / 裁剪扰动测试。
5. 再接入 FFmpeg 解码后的 Y plane fixture，但仍只在 core / Tauri 测试中验证，不开放 UI。
6. 使用真实容器解码出的多帧 Y plane 完成 `watermark-core` DCT staged 写入 / 提取 / 自检 roundtrip。
7. 在受控编码回写后验证水印存活率，再评估桌面端本地渲染包装和云端策略任务。
8. 增加受控有损压缩矩阵，把当前 CRF 12 通过、CRF 38 `self_check_failed` 的边界固定为 release 门禁。
9. 在 core 内增加多帧 bitstream 融合提取，先覆盖分散 bit 损坏恢复，同时保持缺帧 / 擦除按 confidence 失败。
10. 在 core 内加入同步头容错、DCT 写入强度常量和帧内重复副本后，扩展目标平台二压矩阵，避免先承诺后补算法。
11. 在 core 内将 DCT pair 下移到更低 AC 频段，处理 384p 缩放再回 512p 后二压失败。
12. 将 release 主战场切到 720p / 1080p / 2K，保留 512p 仅作为快速小 fixture，不再作为商业主线。
13. 增加首版平台 profile 矩阵：抖音 720p / 1080p 竖屏、小红书 720p / 1080p 3:4、B站 720p / 1080p / 2K 横屏。
14. 增加主流码率地板矩阵：720p H.264 2.5Mbps、1080p H.264 4.5Mbps、2K H.264 8Mbps。
15. 增加 30 秒商业采样性能矩阵，拆分 FFmpeg 源生成 / 抽样、core 写入、采样帧码率回写、core 自检和总耗时。
16. 增加 B站 HEVC 主流码率地板矩阵：1080p HEVC 4Mbps、2K HEVC 6.5Mbps，先探测 `libx265`，再记录与 H.264 相同口径的分段耗时和 confidence。
17. 增加 B站 H.264 / HEVC 成本对照矩阵：同一 30 秒 / 12 采样帧 / 96 策略区域口径下对照 1080p 和 2K 的 H.264 / HEVC 耗时、confidence 和码率档位。
18. 增加 2K H.264 策略密度预算矩阵：同一 30 秒 / 12 采样帧 / 8Mbps 口径下对照 96 / 128 / 160 策略区域的 confidence 和耗时。
19. 增加 2K H.264 抽帧数量预算矩阵：同一 30 秒 / 2K H.264 8Mbps / 96 策略区域口径下对照 12 / 16 / 20 采样帧的 confidence 和耗时。
20. 增加 2K H.264 区域质量预算矩阵：同一 30 秒 / 2K H.264 8Mbps / 16 采样帧 / 96 策略区域口径下对照 seeded random、center safe grid、distributed grid 和 texture aware。
21. 增加平台矩阵耗时预算：同一 30 秒 / 16 采样帧 / 96 策略区域口径下固定抖音、小红书和 B站 1080p / 2K 候选平台参数，并记录分段耗时、总耗时和 confidence。
22. 增加 L3 30 秒平台成本模型：把平台耗时矩阵转成内部成本单位、平台权重、策略权重和未来扣费前置条件，但不实现云端任务、不接 UI、不扣 `video_minutes`。
23. 增加 B站 HEVC TextureAware 对照矩阵：同一 30 秒 / 16 采样帧 / 96 策略区域口径下验证 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps。
24. 增加默认策略切换回归矩阵：720p 保留保守预算，1080p / 2K 默认 TranscodeStable，并覆盖 H.264 / HEVC。
25. 增加默认策略真实素材多样性回归矩阵：覆盖低纹理网格、高细节横屏、高细节竖屏和 2K 低纹理样本。
26. 增加真实素材风险边界矩阵：固定低码率竖屏高细节、极端高频纹理和逐帧随机噪声的通过 / 失败归因。
27. 增加平台二压风险矩阵：固定 1080p 竖屏高细节从 6Mbps 再二压到 4.5Mbps、2K 从 8Mbps 再二压到 6.5Mbps 的通过 / 失败归因和耗时。
28. 增加平台二压稳定性诊断矩阵：观察 1080p 竖屏高细节加帧 / 加区域是否改善，新增 `TranscodeStable` 二压稳态区域候选，并观察 2K 加帧是否把压线通过提升到稳定通过。
29. 增加 TranscodeStable 平台泛化矩阵：覆盖 720p 真实二压失败边界、1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压，验证该核心区域模式是否具备 1080p / 2K 默认策略候选资格。
30. 增加默认 TranscodeStable 平台二压成本权重复核矩阵：必须走 core default 路径，固定 720p 二压失败边界，并覆盖 1080p / 2K H.264 / HEVC 二压。
31. 增加默认 TranscodeStable 真实内容二压矩阵：覆盖 1080p 高细节横屏 / 竖屏、2K 常规纹理和 2K 高细节 H.264 二压，固定通过和风险边界。
32. 增加 2K 高细节 H.264 二压预算策略矩阵：对照 20 帧 / 96 区域、16 帧 / 128 区域和 10Mbps -> 8Mbps，固定“加帧 / 加区域无效，高码率候选可过但未达 SLA”的边界。
33. 增加 2K 高码率内容候选矩阵：以 H.264 10Mbps -> 8Mbps 和 HEVC 8Mbps -> 6.5Mbps 覆盖高细节、低纹理、运动纹理和 HEVC 对照，固定“H.264 高细节仍为 0.875，低纹理 / 运动纹理 / HEVC 可到 1.000”的边界。
34. 冻结 2K 高码率 release 样本池与阈值策略：首版至少 24 个样本，固定 H.264 / HEVC 最低 confidence、分组均值、失败归因和进入 / 禁止商业包装门槛。
35. 4K / 8K 作为未来高阶产品线另设设计文档和性能预算。

## 10. 当前冻结策略

短期不继续推进 L3 算法主线，发布工作转入 `docs/双端现有能力发布计划.md`。完整 24 个 2K 样本长跑、4K / 8K 高阶产品线设计、UI 接入、云端任务和 `video_minutes` 扣费全部后置；现有 L3 staged 证据只作为内部研发资产保留，不能宣称 L3 已达到商业可售鲁棒性。
