# Phase 7 视频云端能力设计

状态：设计预留

本文档定义 HiddenShield 视频能力的分层路线、上传边界、成本模型、验证方式和套餐归属。Phase 7 的目标不是立即实现云端视频处理，而是先把未来能力拆清楚，避免把所有视频动作盲目搬到云端。

## 1. 设计结论

HiddenShield 视频能力采用三档分层：

| 档位 | 能力 | 核心价值 | 云端成本 | 推荐套餐 |
| --- | --- | --- | --- | --- |
| L1 | 本地音频盲水印 | 发布前无感保护 | 无云端算力 | Free 单文件 / Creator 批量 |
| L2 | 画面指纹存证 | 去音轨后仍有辅助证据链 | 低 CPU / 存储成本 | Creator / Studio |
| L3 | 端云协同画面盲水印 | 防去音轨盗搬的高阶保护 | 订阅 + 视频分钟额度 | Studio Beta / Enterprise |

优先顺序：

1. 先稳定 L1：继续复用本地 FFmpeg、音频 QIM 盲水印、完成后验证和版权金库。
2. 再落地 L2：本地抽取不可逆画面指纹，云端只保存指纹、哈希、UID、时间戳和签名。
3. 最后开放 L3：云端生成一次性画面水印策略包，本地完成高码率渲染和自检。

## 2. 总体原则

- 默认不上传原始视频、加水印后的视频、本地文件路径。
- 云端视频能力必须由用户明确发起，不能作为本地压制的隐式步骤。
- 本地视频编解码继续由 Tauri 2 + Rust + FFmpeg 承担，优先利用 NVENC / AMF / QSV / VideoToolbox。
- 云端只承担本地做不了或不适合暴露的能力：时间戳、证据固化、密钥派生、策略生成和高阶取证。
- 成功完成才入账；用户取消、格式不支持、服务异常、任务失败不扣额度。
- 产品文案不对普通用户暴露 DCT、DWT、QIM、频域系数等技术词，内部文档可以保留技术定义。

## 3. L1：本地音频盲水印

### 3.1 能力定义

L1 是当前视频保护的默认底座：用户在桌面端压制视频时，本地抽取音轨，写入音频频域盲水印，再由 FFmpeg 合成到目标平台成品中。

适用场景：

- 抖音、B站、小红书等发布前压制。
- 普通创作者的低成本版权标识。
- 大多数盗搬不会主动去除音轨的场景。

不承诺：

- 不承诺在音轨被删除后仍能从视频画面中提取水印。
- 不承诺移动端本地视频处理。
- 不承诺云端参与本地压制。

### 3.2 上传内容

默认上传：无。

可同步到云端的内容仅限：

- `watermark_uid`
- 源文件 SHA-256 或分片哈希摘要
- 输出文件哈希摘要
- 视频基础元数据摘要：时长、分辨率、目标平台、编码器类型
- 版权记录元数据、取证记录、审计记录

禁止默认上传：

- 原始视频
- 加水印后的视频
- 本地文件路径
- 原始音轨或加水印后音轨

### 3.3 成本模型

成本由用户本机承担：

- 视频编解码：本地 GPU / CPU
- 音频水印：本地 Rust 计算
- 存储：本地 SQLite
- 云端：仅在启用云同步时保存元数据

计费建议：

- Free：允许单文件本地视频音频水印和基础本地记录。
- Creator：开放本地批量、跨端同步、报告导出。
- Studio：继承 Creator，并预留团队归属和审计。

L1 不进入 quota ledger；可进入 usage ledger 作为成功使用统计。

### 3.4 验证方式

本地验证：

- 压制完成后立即抽取输出音频。
- 使用 `watermark-core` 提取 `watermark_uid`。
- 命中后写入版权金库。
- 提取失败时标记输出不可信，并允许重试或调整音频参数。

取证验证：

- 用户下载疑似侵权视频后，本地抽取音轨并提取水印。
- 与本地版权库或云同步版权库比对。
- Creator / Studio 可生成证据报告。

## 4. L2：画面指纹存证

### 4.1 能力定义

L2 不是画面盲水印，而是画面证据链增强。客户端在本地抽取关键帧和场景特征，生成不可逆的视频画面指纹，并把指纹与版权记录绑定。

它解决的问题是：当侵权方去掉音轨后，HiddenShield 仍能提供“该视频画面与原作品高度相似，并且原作品先完成存证”的辅助证据。

适用场景：

- 去音轨盗搬。
- 轻微裁剪、缩放、转码后的相似性证明。
- Creator / Studio 的报告增强。
- 企业客户的版权库检索和批量比对。

不承诺：

- 不把 L2 描述成“可从画面提取盲水印”。
- 不把相似性匹配等同于司法最终裁判。
- 不上传可逆还原原片的特征包。

### 4.2 本地生成内容

客户端本地生成 `VideoFingerprintBundle`：

```json
{
  "schema_version": "video_fingerprint_v1",
  "watermark_uid": "wm_...",
  "source_hash": "sha256:...",
  "duration_ms": 125000,
  "frame_sample_policy": "scene_keyframes_v1",
  "scene_count": 42,
  "fingerprints": [
    {
      "scene_index": 0,
      "timestamp_ms": 0,
      "phash": "...",
      "color_hash": "...",
      "edge_hash": "...",
      "motion_summary": "..."
    }
  ],
  "client_signature": "..."
}
```

字段原则：

- `phash` 用于抵抗缩放、轻微压缩和亮度变化。
- `color_hash` 用于辅助区分画面风格和场景。
- `edge_hash` 用于抵抗轻微调色后的结构匹配。
- `motion_summary` 用于视频级连续性判断。
- `client_signature` 防止本地记录被无声篡改。

### 4.3 上传内容

允许上传：

- 视频画面不可逆指纹包
- 源文件哈希摘要
- 水印 UID
- 创作者身份 ID
- 客户端签名
- 云端时间戳请求
- 任务状态和审计记录

默认禁止上传：

- 原始视频
- 加水印后的视频
- 原始关键帧图片
- 可还原画面的特征矩阵
- 本地文件路径

对于报告场景，疑似侵权视频也应优先在本地生成指纹，再上传不可逆指纹进行比对。

### 4.4 成本模型

云端成本较低：

- 指纹存储：KB 到数 MB 级，随视频长度和采样密度增长。
- 指纹匹配：CPU / 向量检索为主，可批量异步执行。
- 时间戳与签名：按记录或批量 Merkle Tree 聚合。

计费建议：

- Creator：包含个人视频画面指纹存证和报告增强。
- Studio：包含团队共享指纹库、团队审计和更高检索额度。
- Enterprise：开放 API、私有化指纹库、批量检索。

L2 通常不按视频分钟扣云端视频额度；它更适合纳入订阅权益和 usage ledger。大规模批量检索可进入云端批量额度。

### 4.5 验证方式

本地验证：

- 生成指纹后校验 bundle schema。
- 随机抽样重算关键帧指纹，确认本地记录可复现。
- 写入本地版权库。

云端验证：

- 校验客户端签名、账户身份和权益状态。
- 固化 `watermark_uid + source_hash + fingerprint_root + timestamp`。
- 返回云端签名收据。

取证验证：

- 对疑似侵权视频生成指纹。
- 与版权库指纹做相似度匹配。
- 报告输出匹配分数、命中片段、时间戳证明和原始版权记录。

## 5. L3：端云协同画面盲水印

### 5.1 能力定义

L3 是真正的视频画面盲水印高阶能力。云端根据本地生成的内容特征、目标平台参数和服务端密钥，生成一次性画面水印策略包；客户端使用该策略包在本地完成最终高码率视频压制，并进行完成后自检。

目标：

- 即使音轨被删除，也能从画面中恢复或验证水印。
- 抵抗常见二压、缩放、轻微裁剪、调色和平台转码。
- 控制云端 GPU 成本，避免云端处理完整大视频编解码。

### 5.2 推荐链路

```text
用户明确发起 L3 高阶视频保护
  -> 本地 ffprobe 探测元数据
  -> 本地抽取关键帧/场景/频域摘要
  -> 本地上传不可逆特征包
  -> 云端校验权益和额度
  -> 云端生成一次性画面水印策略包
  -> 客户端本地 FFmpeg 渲染成品
  -> 客户端本地自检水印强度
  -> 成功后云端扣减视频分钟额度并固化收据
```

可选增强链路：

- 在用户明确同意时上传极低码率 proxy，供云端做更稳的策略生成。
- proxy 必须有独立提示、任务级授权和自动过期删除策略。
- proxy 不应作为默认路径。

### 5.3 上传内容

默认上传：

- `VideoFeatureBundle`
- 源文件哈希摘要
- 目标平台参数：分辨率、帧率、GOP、编码器目标、码率档
- 场景切分摘要
- 关键帧不可逆频域摘要
- 客户端环境摘要：硬件编码器类型、FFmpeg 能力
- 任务 ID、账户 ID、创作者身份 ID

可选上传：

- 极低码率 proxy 音视频轨，仅在用户明确发起云端视频任务且单独授权时允许。

禁止默认上传：

- 原始 2GB / 4K 视频
- 最终成品视频
- 本地文件路径
- 可逆的完整帧序列

### 5.4 云端输出内容

云端返回 `VideoWatermarkStrategyPacket`：

```json
{
  "schema_version": "video_strategy_v1",
  "task_id": "vtask_...",
  "watermark_uid": "wm_...",
  "expires_at": "2026-06-18T12:00:00Z",
  "target_profiles": ["douyin_1080x1920_h264", "bilibili_1080p_hevc"],
  "embed_regions": [
    {
      "scene_index": 3,
      "frame_range": [240, 360],
      "frequency_band": "mid",
      "strength": 0.018,
      "redundancy_group": "A"
    }
  ],
  "self_check_threshold": 0.82,
  "server_signature": "..."
}
```

安全原则：

- 策略包必须是一次性的。
- 策略包不能泄露服务端主密钥。
- 过期策略包不可复用。
- 服务端保留任务签名、策略摘要和验证所需的密钥材料。

### 5.5 成本模型

L3 是明确有边际成本的云端能力，应进入 `cloud_video_processing` 和 quota ledger。

成本组成：

- 云端策略生成：GPU / CPU 混合，目标是秒级，不做完整高码率编解码。
- 存储：任务特征包、策略摘要、收据、审计记录。
- 队列：高峰期需要排队和优先级调度。
- 验证：云端取证提取或策略辅助验证。

计费建议：

- Free：不开放。
- Creator：默认不开放正式 L3，可在内测期给少量体验名额，但不作为公开承诺。
- Studio：开放 Beta，按团队视频分钟额度计费。
- Enterprise：合同额度、专属队列、API / 私有化部署。

扣费规则：

- 成功生成策略包且客户端完成自检后扣减视频分钟额度。
- 用户取消、权益不足、格式不支持、服务异常、策略生成失败、自检失败不扣额度。
- 多平台输出可按同一母片策略包 + 输出版本系数计费，具体规则上线前再固化。

### 5.6 验证方式

客户端自检：

- 本地渲染完成后抽样检测画面水印强度。
- 低于 `self_check_threshold` 时自动重试更高强度或提示用户调整码率。
- 自检成功后写入版权金库和 usage ledger。

云端验证：

- 云端根据任务 ID、策略摘要和服务端密钥执行辅助提取。
- 取证时可上传疑似侵权视频的不可逆特征包；必要时由用户明确上传片段或低码率 proxy。
- 输出报告包含水印命中、相似性匹配、时间戳、任务收据和审计记录。

失败处理：

- 自检失败不扣额度。
- 策略包过期后需重新创建任务。
- 用户本地 FFmpeg 或硬件编码失败不扣云端视频额度。

## 6. 套餐映射

| 能力 | Free | Creator | Studio | Enterprise |
| --- | --- | --- | --- | --- |
| L1 单文件本地音频盲水印 | 支持 | 支持 | 支持 | 支持 |
| L1 本地批量视频音频水印 | 不开放 | 可作为本地批量扩展 | 支持更高并发 | 可定制 |
| L2 画面指纹本地生成 | 可本地预留 | 支持 | 支持 | 支持 |
| L2 云端指纹存证 | 不开放或少量预留 | 支持 | 团队共享 | 私有化 / API |
| L2 指纹取证报告 | 不开放 | 支持 | 团队报告 | 定制报告 |
| L3 端云协同画面盲水印 | 不开放 | 内测名额，不公开承诺 | Beta + 分钟额度 | 合同额度 |
| L3 优先队列 | 不开放 | 不开放 | 可加购 | 专属 SLA |

当前权益字段沿用：

- `cloud_video_processing`：控制 L3 云端视频任务。
- `report_export`：控制 L2 / L3 报告导出。
- `cloud_sync`：控制版权记录和存证收据同步。
- `team_workspace`：控制 Studio 团队共享指纹库。
- `priority_queue`：控制 L3 高优先级队列。

如后续需要更细粒度控制，可新增服务端能力位，但客户端首期不应提前暴露未实现入口。

## 7. 任务、额度与状态契约

Phase 7 正式预留 `cloud_video_tasks`。该模型只描述云端视频能力的任务和额度，不代表当前已经开放 L3 处理。

| 字段 | 说明 |
| --- | --- |
| `task_id` | 云端视频任务 ID |
| `account_id` | 账户 ID |
| `workspace_id` | 工作区 ID |
| `creator_profile_id` | 创作者身份 ID |
| `capability_level` | `audio_local` / `fingerprint_notary` / `hybrid_visual_watermark` |
| `watermark_uid` | 水印 UID |
| `source_hash` | 源文件哈希摘要 |
| `duration_ms` | 视频时长 |
| `target_profiles` | 目标平台输出配置 |
| `upload_manifest` | 上传内容清单，不含本地路径 |
| `status` | queued / running / waiting_client_render / self_checking / succeeded / failed / canceled |
| `quota_units` | 扣减额度，L3 使用视频分钟 |
| `failure_code` | 失败原因 |
| `created_at` | 创建时间 |
| `completed_at` | 完成时间 |

### 7.1 capability_level

| 值 | 含义 | 是否扣视频分钟 |
| --- | --- | --- |
| `audio_local` | L1 本地音频盲水印，仅作为统一任务视图预留 | no |
| `fingerprint_notary` | L2 画面指纹存证 | no |
| `hybrid_visual_watermark` | L3 端云协同画面盲水印 | yes |

说明：

- `audio_local` 默认不创建云端任务，除非未来需要统一审计视图。
- `fingerprint_notary` 可进入 usage ledger，不进入视频分钟 quota。
- `hybrid_visual_watermark` 必须由 `cloud_video_processing` 权益控制。

### 7.2 视频分钟额度

L3 使用 `video_minutes` 作为 quota 类型。

建议字段：

| 字段 | 说明 |
| --- | --- |
| `quota_id` | 额度记录 ID |
| `account_id` | 账户 ID |
| `workspace_id` | 工作区 ID |
| `quota_type` | 固定为 `video_minutes` |
| `included_units` | 套餐内分钟数 |
| `purchased_units` | 额外购买分钟数 |
| `used_units` | 已成功扣减分钟数 |
| `reserved_units` | 已预约但未最终扣减分钟数 |
| `period_started_at` | 周期开始时间 |
| `period_ends_at` | 周期结束时间 |
| `updated_at` | 更新时间 |

扣减计算：

- `quota_units = ceil(duration_ms / 60000)`。
- 小于 1 分钟的 L3 成功任务按 1 分钟计。
- 多平台输出可使用同一母片任务加输出版本系数，系数在上线前固化。
- 用户取消、格式不支持、权益不足、策略生成失败、自检失败、客户端渲染失败、服务异常均不扣额度。

### 7.3 任务状态

任务状态固定为以下集合：

| 状态 | 含义 | 是否终态 | 是否扣额度 |
| --- | --- | --- | --- |
| `draft` | 客户端已准备任务草稿，尚未提交 | no | no |
| `queued` | 云端已接收，等待执行 | no | no |
| `running` | 云端正在生成策略或处理指纹 | no | no |
| `waiting_client_render` | 策略包已返回，等待客户端本地渲染 | no | no |
| `self_checking` | 客户端正在做完成后自检 | no | no |
| `succeeded` | 任务成功完成并固化收据 | yes | yes，仅 L3 |
| `failed` | 任务失败 | yes | no |
| `canceled` | 用户取消 | yes | no |
| `expired` | 策略包或任务超时 | yes | no |

状态流转：

```text
draft -> queued -> running -> waiting_client_render -> self_checking -> succeeded
draft -> canceled
queued -> canceled / failed / expired
running -> failed / expired
waiting_client_render -> self_checking / canceled / expired
self_checking -> succeeded / failed
```

失败原因建议：

- `entitlement_required`
- `quota_insufficient`
- `unsupported_format`
- `feature_bundle_invalid`
- `strategy_generation_failed`
- `client_render_failed`
- `self_check_failed`
- `service_unavailable`
- `user_canceled`

### 7.4 上传清单

所有云端视频任务必须包含 `upload_manifest`，用于证明本次任务上传了什么。

建议结构：

```json
{
  "schema_version": "video_upload_manifest_v1",
  "contains_original_video": false,
  "contains_watermarked_video": false,
  "contains_local_paths": false,
  "contains_proxy": false,
  "items": [
    {
      "kind": "feature_bundle",
      "sha256": "sha256:...",
      "bytes": 48212
    }
  ]
}
```

客户端和云端都必须拒绝默认包含原始视频、加水印视频或本地路径的 manifest。

## 8. 验收标准

Phase 7 设计验收：

- 视频能力被拆成 L1 / L2 / L3 三档。
- 每档明确上传内容、成本模型、验证方式和套餐归属。
- 文档明确 L2 不是画面盲水印，避免产品误承诺。
- 文档明确 L3 不默认上传原始视频。
- 文档明确 L3 成功后才扣视频分钟额度。
- 文档保持 Free / Creator / Studio / Enterprise 术语一致。

后续实现验收：

- 客户端未获得 `cloud_video_processing` 时不能创建 L3 云端视频任务。
- 服务端以权益快照为最终裁判。
- 所有云端视频任务都有 upload manifest。
- 所有云端视频任务失败、取消、崩溃均不扣额度。
- L2 / L3 报告不得包含原始媒体或本地文件路径。

## 9. 风险与开放问题

风险：

- L2 指纹相似性只能作为辅助证据，不能替代水印命中。
- L3 策略包如果设计过细，客户端逆向后可能暴露嵌入规律。
- 不同硬件编码器的量化行为会影响 L3 画面水印存活率。
- 平台二压策略会变化，需要持续维护鲁棒性测试矩阵。

开放问题：

- L2 指纹采样密度如何按视频时长自适应。
- L3 多平台输出是否共享同一策略包，还是按平台生成独立策略包。
- Creator 是否提供 L3 公开体验额度，需等成本基准测试后决定。
- Enterprise 私有化部署是否允许客户自管密钥，需单独做安全设计。

## 10. 推荐下一步

下一步实现 L2 云端指纹存证后端契约测试：按 `docs/Phase 7 L2云端指纹存证API草案.md` 增加请求 schema、manifest 隐私拒绝用例、缺少 `crop_window_fingerprint_root` 的拒绝用例，以及成功请求不扣 `video_minutes` 的账本断言。
