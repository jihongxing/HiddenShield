# Phase I-6 L3 视频画面盲水印同核与云端策略设计

状态：设计冻结，未进入实现

本文档定义 HiddenShield L3 端云协同视频画面盲水印的实现前边界。L3 是未来能力，不是当前 L2 视频指纹存证的别名，也不能由桌面端、移动端、后端或云任务各自实现一套画面盲水印算法。

## 1. 设计结论

- L3 的画面盲水印写入、读取、payload 编码、同步标记、鲁棒性参数和恢复逻辑必须位于 `watermark-core`。
- 云端只能提供策略生成、密钥托管、任务调度、权益校验、额度账本、策略签名和自检编排，不能成为第二套算法核心。
- 桌面端负责本地渲染、FFmpeg 编解码、策略包执行、完成后自检和本地版权库记录。
- 移动端首期只允许查看同步记录、报告和提交验证样本，不实现本地视频画面盲水印写入。
- L2 视频指纹存证只能作为不可逆相似性证据增强，不能包装成 L3 视频画面盲水印，也不能替代 L3 的水印命中。
- L3 只有在策略包生成成功、客户端本地渲染成功、完成后自检通过并固化云端收据后，才扣减 `video_minutes`。

## 2. 分层边界

| 层级 | 能力 | 是否盲水印 | 算法归属 | 是否扣 `video_minutes` |
| --- | --- | --- | --- | --- |
| L1 | 视频音轨水印 | 是，音频盲水印 | `watermark-core` 音频算法 | 否 |
| L2 | 视频画面指纹存证 | 否，不可逆指纹 | `VideoFingerprintBundle` 生成与 notary 合同 | 否 |
| L3 | 视频画面盲水印 | 是，画面盲水印 | `watermark-core` 视频画面算法 | 成功后扣 |

L3 不得复用 L2 的 `fingerprint_root` 作为水印命中结果。L3 报告可以同时展示 L2 相似性证据和 L3 水印命中，但两者必须分栏展示、分开解释。

## 3. `watermark-core` 算法契约

L3 进入实现前，`watermark-core` 必须先提供稳定的核心 API，而不是让平台层直接操作视频帧频域。

建议核心类型：

```rust
pub struct VideoVisualPayloadBuildInput {
    pub creator_identity: String,
    pub device_identity: String,
    pub media_sha256: [u8; 32],
    pub timestamp: u64,
    pub ai_flags: AIContentFlags,
}

pub struct VideoFeatureBundle {
    pub schema_version: String,
    pub source_sha256: String,
    pub duration_ms: u64,
    pub frame_sample_policy: String,
    pub scene_features: Vec<VideoSceneFeature>,
    pub codec_profile: VideoCodecProfile,
}

pub struct VideoVisualStrategy {
    pub schema_version: String,
    pub watermark_uid: String,
    pub target_profiles: Vec<String>,
    pub embed_regions: Vec<VideoVisualEmbedRegion>,
    pub self_check_threshold: f32,
}

pub struct VideoVisualSelfCheckResult {
    pub watermark_uid: String,
    pub confidence: f32,
    pub checked_frames: u32,
    pub passed: bool,
}
```

建议核心 API：

```rust
pub fn build_video_visual_payload(input: VideoVisualPayloadBuildInput) -> Result<WatermarkPayload, WatermarkError>;

pub fn derive_video_visual_strategy(
    payload: &WatermarkPayload,
    features: &VideoFeatureBundle,
    policy: &VideoVisualPolicy,
) -> Result<VideoVisualStrategy, WatermarkError>;

pub fn embed_video_visual_frame(
    frame: &mut VideoFramePlane,
    strategy: &VideoVisualStrategy,
    frame_context: &VideoFrameContext,
) -> Result<(), WatermarkError>;

pub fn extract_video_visual_watermark(
    frames: &[VideoFramePlane],
    strategy_hint: &VideoVisualStrategyHint,
) -> Result<WatermarkPayload, WatermarkError>;

pub fn self_check_video_visual_watermark(
    frames: &[VideoFramePlane],
    strategy: &VideoVisualStrategy,
) -> Result<VideoVisualSelfCheckResult, WatermarkError>;
```

API 约束：

- payload 构造继续复用当前身份源数据规则，不能重新引入平台层 seed 派生。
- 算法实现只能在 `watermark-core` 内部出现；Tauri、Flutter、后端和脚本只能调用核心 API。
- 错误必须返回结构化 `WatermarkErrorCode`，至少包含 `strategy_invalid`、`feature_bundle_invalid`、`self_check_failed`、`visual_extract_failed`、`unsupported_video_profile`。
- 核心测试必须覆盖 payload 稳定性、策略确定性、轻度二压/缩放后的提取、失败错误码和重复写入检测。

## 4. 云端策略包

云端返回的策略包只是一份由 `watermark-core` 生成或校验的执行计划，不包含服务端主密钥，也不包含可复用的完整算法。

```json
{
  "schema_version": "video_strategy_v1",
  "task_id": "vtask_...",
  "watermark_uid": "HS-....",
  "expires_at": "2026-06-20T12:00:00Z",
  "strategy_digest": "sha256:...",
  "target_profiles": ["douyin_1080x1920_h264"],
  "embed_regions": [
    {
      "scene_index": 3,
      "frame_range": [240, 360],
      "band": "mid",
      "strength": 0.018,
      "redundancy_group": "A"
    }
  ],
  "self_check_threshold": 0.82,
  "server_signature": "base64..."
}
```

策略包规则：

- 策略包必须是一次性的：绑定 `task_id`、`watermark_uid`、源文件摘要、目标 profile 和过期时间。
- 可验证：客户端必须校验 `server_signature`、`expires_at` 和 `strategy_digest`。
- 不泄密：策略包不得包含服务端主密钥、长期派生密钥或可跨任务复用的全局嵌入规律。
- 可审计：云端保存策略摘要、任务状态、签名、额度流水和自检结果，不保存原始视频或成品视频。

## 5. 密钥边界与防逆向

密钥分层：

- 服务端主密钥：仅云端 KMS / 环境密钥可访问，不能下发客户端。
- 任务派生密钥：由服务端主密钥、账户、工作区、`watermark_uid`、源摘要和 `task_id` 派生。
- 策略执行参数：只包含嵌入区域、强度、冗余组和自检阈值，不包含可恢复主密钥的信息。

防逆向要求：

- 客户端只能执行策略包，不得拥有生成长期策略的能力。
- 策略包过期后必须重新创建任务。
- 同一源文件再次创建 L3 任务必须产生新的 `task_id` 和策略摘要。
- 客户端日志、诊断日志和匿名反馈不得包含完整策略包、原始媒体路径、帧内容或密钥材料。

## 6. 客户端执行与自检

桌面端执行流程：

```text
用户明确发起 L3 高阶视频保护
  -> 桌面端生成 VideoFeatureBundle
  -> 云端校验权益和额度，生成策略包
  -> 桌面端校验策略包签名和有效期
  -> 桌面端本地 FFmpeg 解码 / 渲染 / 编码
  -> 每帧或分段调用 watermark-core 画面嵌入 API
  -> 桌面端抽样调用 watermark-core 自检 API
  -> 自检通过后提交完成回执
  -> 云端扣减 video_minutes 并返回收据
  -> 桌面端写入版权库和正式报告字段
```

自检要求：

- 必须在成品视频上执行，不得只检查中间帧。
- 必须记录 `checked_frames`、`confidence`、`self_check_threshold`、`strategy_digest`。
- 自检失败不能扣费，不能写入成功版权记录。
- 自检失败可以提示用户降低压缩强度、调整目标平台或重新生成策略。

## 7. 云端验证与取证

云端验证职责：

- 校验任务 ID、策略摘要、账户、工作区和收据。
- 使用服务端保存的策略摘要和密钥材料辅助验证疑似侵权样本。
- 默认接收不可逆特征包；如需上传低码率 proxy，必须有用户明确授权和自动删除策略。

报告字段：

- `l3_task_id`
- `watermark_uid`
- `strategy_digest`
- `self_check_confidence`
- `self_check_threshold`
- `video_minutes_charged`
- `server_receipt_signature`
- `verified_at`

报告必须同时声明：

- L3 是视频画面盲水印命中。
- L2 是相似性证据增强。
- 报告仍是技术辅助材料，不构成法律意见。

## 8. 额度与扣费

L3 使用 `cloud_video_processing` 权益和 `video_minutes` quota。

扣费前置条件：

- 策略包生成成功。
- 客户端本地渲染成功。
- 成品视频完成后自检通过。
- 云端收到客户端完成回执并固化收据。

不扣费场景：

- 用户取消。
- 权益不足。
- 额度不足。
- 格式不支持。
- 特征包无效。
- 策略生成失败。
- 客户端渲染失败。
- 自检失败。
- 服务异常或任务过期。

`quota_units = ceil(duration_ms / 60000)`，小于 1 分钟按 1 分钟计。多平台输出是否复用同一策略包和计费系数必须在实现前另行冻结。

## 9. 同步与隐私边界

允许同步：

- 版权记录元数据。
- L3 任务 ID、策略摘要、收据签名、自检摘要、可信时间。
- 输出文件哈希摘要。
- 报告所需的技术摘要。

禁止同步：

- 原始视频。
- 加水印视频。
- 低码率 proxy，除非用户明确选择云端验证任务且有自动删除策略。
- 本地文件路径。
- 完整策略包。
- 服务端密钥或任务派生密钥。

## 10. 实现前门禁

L3 进入编码前必须先完成：

- `watermark-core` 新增视频画面算法 API 与契约测试。
- `watermark:architecture-contract` 扫描并阻止核心外视频画面盲水印算法。
- `watermark:video-phase-contract` 检查本文档、策略包、密钥边界、自检、扣费和 L2/L3 区分。
- 云端任务 schema 固化 `task_id`、`strategy_digest`、`upload_manifest`、`quota_units`、`self_check_result`。
- 桌面端 UI 明确 Studio Beta / Enterprise 边界，Free / Creator 不承诺正式 L3。
- 移动端只读展示和提交验证样本边界明确，不新增本地 L3 写入算法。

## 11. 当前非目标

- 不在本阶段实现视频画面盲水印算法。
- 不在本阶段开放云端视频任务。
- 不上传原始视频做默认云端处理。
- 不把 L2 指纹匹配结果当作 L3 水印命中。
- 不给移动端单独实现视频画面盲水印写入。

## 12. 推荐下一步

下一步进入 L3 实现前技术 spike 设计：在 `watermark-core` 内定义最小视频帧平面模型、策略结构和自检结果结构，先用合成帧 fixture 验证策略确定性和错误码，不接 UI、不接云端任务、不做真实用户视频处理。
