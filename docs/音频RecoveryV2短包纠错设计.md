# 音频 Recovery V2 短包纠错设计

## 背景

当前音频水印已经有三层能力：

1. 主 payload 投票层：适合完整音频、音量变化、重采样和常规转码。
2. 时间片 marker 层：用于证明片段来自已加水印音频，并帮助定位。
3. Recovery V1 层：重复写入完整 payload recovery packet，已支持 10 秒中段裁剪。

当前稳定基线：

```powershell
cargo test --manifest-path watermark-core/Cargo.toml
npm run watermark:bench -- --image-dir "E:\Users\jihx\Pictures\人脸" --audio-glob "E:\Users\jihx\Pictures\*.mp3" --max-images 1 --max-audio 1
npm run watermark:matrix -- --image-dir "E:\Users\jihx\Pictures\人脸" --audio-glob "E:\Users\jihx\Pictures\*.mp3" --max-images 1 --max-audio 1
```

| 验证项 | 当前结果 |
| --- | ---: |
| watermark-core 全量单测 | 23/23 passed |
| 默认 bench | 13/13 passed |
| audio matrix | 23/31 passed |

## 产品边界更新

根据真实素材矩阵和 V2 纯工具模拟结果，HiddenShield 不再把 5 秒 / 10 秒短片段恢复完整 payload 作为当前产品目标。音频版权保护从 **30 秒及以上** 的原始音频开始；短于 30 秒的素材不生成版权保护副本。

短裁剪片段仍可作为取证输入：如果能提取到水印，则作为辅助证据展示；如果不能提取，不应被表述为产品失败。该文档后续作为研究归档和下一代 recovery 设计参考，不直接驱动生产写入改造。

矩阵失败集中在：

- 5 秒中段 / 结尾裁剪。
- 10 秒结尾裁剪。
- 15 秒中段裁剪。
- MP3 roundtrip 后同类短裁剪场景。

这说明 V1 已经能覆盖一部分截取场景，但还不能支撑“任意 5 秒片段可恢复完整 payload”的产品承诺。

## 已排除方案

### 1. 直接压缩 V1 包周期

尝试把 `AUDIO_RECOVERY_BITS_PER_FRAME` 从 18 提升到 34，并把 lane 从 3 降到 2。

结果：

- 默认 bench 退化。
- `clip_10s_middle` 从 PASS 变为 FAIL。

再尝试 24 bit/frame + 3 lane，仍导致默认 bench 退化。

结论：

- 当前频段容量下，直接压缩 V1 单包周期会降低每 bit 稳定性。
- 这条路不能作为产品级修复。

### 2. 同频段交错起点

尝试在奇偶 recovery 周期使用半包偏移，避免额外占用频段。

结果：

- 合成裁剪 recovery 单测失败。

根因：

- 裁剪后丢失原始 recovery 周期编号。
- 提取端无法可靠知道当前片段对应哪个交错起点。

结论：

- 交错写入必须携带局部同步信息。
- 不能只依赖原始全局 frame index。

## 设计目标

Recovery V2 不替换 V1，而是新增短包层。当前阶段先归档，不进入生产写入。

目标：

1. 保留 V1 的稳定路径，不影响默认 bench。
2. 在不改变 30 秒产品边界的前提下，研究短裁剪和结尾裁剪恢复率。
3. 支持 MP3 roundtrip 后再裁剪的局部恢复。
4. 让短片段不依赖原始全局 frame index。
5. 所有恢复结果必须最终通过 payload HMAC，避免误判。

非目标：

- 不改变 `WatermarkPayload` 的 32 字节业务格式。
- 不在 UI 暴露 recovery、ECC、FFT、marker 等技术词。
- 不承诺任意极短片段恢复完整 payload。
- 不删除 V1，V2 必须可回滚。

## 核心方案

Recovery V2 采用“短包分片 + 局部同步 + 强校验”的结构。

V1 写完整 payload：

- 包长：38 bytes。
- 用途：10 秒级片段恢复完整 payload。
- 当前保留。

V2 写 payload 分片：

- 每个短包只承载 payload 的一部分。
- 每个短包自带局部同步信息。
- 多个短包在 5 秒片段内可聚合还原完整 payload。
- 片段顺序不依赖原始全局 frame index，只依赖短包里的 `segment_id` 和 `epoch_id`。

## V2 短包结构

建议短包长度控制在 14 bytes：

| 字段 | 长度 | 说明 |
| --- | ---: | --- |
| preamble | 2 bytes | 固定同步头，例如 `0xA7 0x5C` 的 V2 变体 |
| version | 1 byte | 固定为 2 |
| epoch_id | 1 byte | 轮次编号，用于区分同一 payload 的不同重复周期 |
| segment_id | 1 byte | payload 分片编号 |
| payload_tag | 2 bytes | 完整 payload 短标签 |
| data | 4 bytes | payload 分片数据 |
| parity | 1 byte | 分片级奇偶或简单校验 |
| checksum | 2 bytes | 对前 12 字节的校验 |

总计：14 bytes，即 112 bit。

如果使用 3 倍 redundancy，总 raw bits 为 336 bit。按当前 18 bit/frame 计算，约 19 帧，约 1.76 秒。5 秒片段理论上可覆盖 2 到 3 个短包。

## 分片策略

`WatermarkPayload` 为 32 bytes。

建议拆成 8 个 segment：

| segment_id | data 范围 |
| ---: | --- |
| 0 | payload[0..4] |
| 1 | payload[4..8] |
| 2 | payload[8..12] |
| 3 | payload[12..16] |
| 4 | payload[16..20] |
| 5 | payload[20..24] |
| 6 | payload[24..28] |
| 7 | payload[28..32] |

为了让 5 秒片段有机会恢复完整 payload，需要加入轻量冗余。

建议每个 epoch 写：

- 8 个 data segment。
- 4 个 parity segment。

总计 12 个 segment。

初版 parity 可先使用 XOR 组：

| parity segment | 覆盖 |
| ---: | --- |
| 8 | 0 ^ 1 ^ 2 ^ 3 |
| 9 | 4 ^ 5 ^ 6 ^ 7 |
| 10 | 0 ^ 2 ^ 4 ^ 6 |
| 11 | 1 ^ 3 ^ 5 ^ 7 |

这不是最强 ECC，但实现简单、可解释、可先验证收益。若矩阵提升不足，再升级 Reed-Solomon 或 BCH。

## 写入布局

V2 不占用 V1 的频段。

建议新增 V2 专用频段区：

```text
marker lanes
V1 recovery lanes
V2 short packet lanes
main payload lanes
```

写入规则：

1. V1 保持当前行为。
2. V2 在长音频启用，短音频自动跳过。
3. V2 segment 按 frame 滚动写入。
4. 每个 epoch 的起点由 frame 顺序自然决定，但每个短包自带 `epoch_id` 和 `segment_id`，提取时不需要知道原始全局 frame index。
5. 静音帧不写入，提取时用短包 checksum 和 payload HMAC 过滤错误结果。

## 提取流程

现有顺序保持：

1. 主 payload 投票。
2. V1 recovery。
3. marker 定位。
4. legacy QIM。

新增 V2 建议插入在 V1 recovery 之后：

1. 扫描 V2 短包候选。
2. 对每个候选窗口做 majority vote。
3. 解出短包后按 `payload_tag + epoch_id` 聚合。
4. 收集到足够 data/parity segment 后重组 32-byte payload。
5. 调用 `decode_payload` 校验 HMAC。
6. HMAC 通过才返回水印结果。

## 误判控制

必须同时满足：

- preamble bit error 不超过阈值。
- 短包 checksum 通过。
- payload_tag 能与重组 payload 对上。
- 最终 `decode_payload` HMAC 通过。

不允许只凭短包 checksum 返回用户可见水印。

## 验收标准

第一阶段目标：

| 验证项 | 目标 |
| --- | ---: |
| watermark-core 全量单测 | 100% pass |
| 默认 bench | 13/13 pass |
| audio matrix | 作为研究指标记录，不作为发布门禁 |
| 10s middle clip | 保留观测，不作为产品承诺 |
| 写入耗时 | 不超过当前基线 1.5x |

第二阶段目标：

| 验证项 | 目标 |
| --- | ---: |
| 3 首歌 matrix | 每首至少 29/31 |
| MP3 192k roundtrip 后 10s middle | 100% pass |
| 5s start | 100% pass |
| 5s middle/end | 尽量提升，但不作为首版宣传承诺 |

## 实施计划

### P1：纯工具模拟

先新增或扩展诊断工具，不接生产提取：

- 生成 V2 短包序列。
- 在真实音频 frame 上估算短包可承载容量。
- 输出 5s/10s/15s 裁剪中理论可收集的 segment 数量。

目标：

- 验证 14-byte 短包是否能在 5 秒片段内收集足够 segment。
- 避免再次把未验证布局直接塞进生产算法。

当前已新增工具：

```powershell
cargo run --manifest-path watermark-core/Cargo.toml --bin audio_recovery_v2_probe -- --audio-glob "E:\Users\jihx\Pictures\*.mp3" --max-audio 1
```

首轮模拟结果：

| 布局 | 结果 |
| --- | ---: |
| 14-byte 短包 | 112 bit |
| 8 data segment + 4 parity segment | 12 segment / epoch |
| 18 bit/frame + 3x redundancy | 约 19 frame / packet |
| 真实 1 首歌 5s/10s/15s × start/middle/end | 2/9 recoverable |

结论：

- 该布局对 15 秒片段有一定覆盖，但对 5 秒和 10 秒裁剪明显不足。
- 不能进入 P2 生产写入实验。
- 当前阶段停止继续深挖短片段恢复，把工程资源转回 30 秒以上音频写入稳定性、可验证性和用户提示。

### P2：生产写入但实验提取

- 在写入端增加 V2 短包层。
- 提取端只在 `audio_recovery_diag` 中解析 V2。
- 不进入 `WatermarkService::extract` 返回路径。

目标：

- 跑 `watermark:matrix` 和诊断工具，确认 V2 能把失败用例拉起来。

### P3：接入生产提取

条件：

- 默认 bench 仍为 13/13。
- matrix 至少达到 29/31。
- 误判控制路径有单测。

接入顺序：

```text
main payload -> V1 recovery -> V2 recovery -> marker -> legacy
```

### P4：移动端同步

底层能力稳定后，移动端和桌面端只展示统一文案：

- “检测到版权水印”
- “片段可验证”
- “片段过短，建议使用更长文件”

不展示 V1/V2/recovery/FFT 等技术词。

## 当前结论

Recovery V2 应该是一个独立短包纠错层，而不是继续调 V1 参数。

但在当前产品阶段，Recovery V2 只保留为研究方向。生产系统采用 30 秒最小时长边界，优先保证完整作品和可独立追索音频的写入稳定性。

最重要的工程原则：

- 稳定路径不退化。
- 新层可开关、可诊断、可回滚。
- 先用矩阵证明收益，再进入产品提取路径。
