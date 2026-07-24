# Phase 7 L2 云端指纹存证 API 草案

状态：设计草案

本文档把 L2 画面指纹存证从技术 spike 收口到后端 API 契约。L2 不是视频画面盲水印，而是“不可逆画面指纹 + 云端时间戳 + 相似性取证”的证据增强能力。

## 1. 设计结论

L2 云端存证 API 必须保存三层不可逆摘要：

- `global_frame_fingerprints`：整帧摘要，用于快速粗筛缩放、二压、轻微亮度变化。
- `local_block_fingerprint_root`：局部块摘要 Merkle root，用于局部相似性和后续细粒度比对。
- `crop_window_fingerprint_root`：裁剪候选窗口摘要 Merkle root，用于裁剪后的直接匹配。

不得只保存 `fingerprint_root` 或整帧 hash。真实样本 spike 已证明：整帧 + 固定局部块为 `25/30`，中心裁剪只有 `5/10`；加入不可逆 `crop_windows` 后为 `30/30`。

## 2. 产品边界

L2 允许：

- 本地生成不可逆 `VideoFingerprintBundle`。
- 云端保存不可逆摘要、Merkle root、客户端签名、云端时间戳和审计记录。
- Creator / Studio 在取证报告中使用相似性证据增强。

L2 不允许：

- 宣传为“视频画面盲水印”。
- 默认上传原始视频。
- 默认上传加水印后的视频。
- 默认上传本地文件路径。
- 上传可还原原始画面的关键帧、帧序列或特征矩阵。

## 3. 权益与计费

能力位：

- `cloud_sync`：控制存证收据和版权记录同步。
- `report_export`：控制正式取证报告导出。
- `team_workspace`：控制 Studio 团队共享指纹库。
- `cloud_video_processing`：仅控制 L3 端云协同画面盲水印，不控制 L2。

套餐建议：

| 套餐 | L2 本地生成 | L2 云端存证 | L2 报告增强 |
| --- | --- | --- | --- |
| Free | 可本地预留 | 不开放 | 不开放 |
| Creator | 支持 | 支持 | 支持 |
| Studio | 支持 | 团队共享 | 团队报告 |
| Enterprise | 支持 | 私有化 / API | 定制报告 |

计费规则：

- L2 不扣 `video_minutes`。
- L2 写入 `usage_ledger`，类型建议为 `video_fingerprint_notary`。
- 大规模批量检索可单独进入云端批量权益，不与 L3 视频分钟混用。

## 4. 数据模型

### 4.1 video_fingerprint_notaries

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `notary_id` | string | 云端存证 ID |
| `account_id` | string | 账户 ID |
| `workspace_id` | string | 工作区 ID |
| `creator_profile_id` | string | 创作者身份 ID |
| `watermark_uid` | string | 水印 UID |
| `source_hash` | string | 源视频哈希摘要 |
| `duration_ms` | number | 视频时长 |
| `frame_sample_policy` | string | 抽帧策略 |
| `scene_count` | number | 场景 / 抽样帧数量 |
| `fingerprint_schema_version` | string | 指纹 schema |
| `global_frame_fingerprints` | array | 抽样整帧不可逆摘要 |
| `local_block_fingerprint_root` | string | 局部块摘要 Merkle root |
| `local_block_count` | number | 局部块摘要数量 |
| `crop_window_fingerprint_root` | string | 裁剪候选窗口摘要 Merkle root |
| `crop_window_count` | number | 裁剪候选窗口摘要数量 |
| `fingerprint_root` | string | 三层摘要聚合 root |
| `client_signature` | string | 客户端签名 |
| `server_receipt_signature` | string | 云端收据签名 |
| `upload_manifest` | object | 上传内容清单 |
| `created_at` | string | 创建时间 |
| `notarized_at` | string | 云端存证时间 |

### 4.2 global_frame_fingerprints

整帧摘要可以随存证请求上传少量抽样明细，用于云端快速粗筛。

```json
{
  "scene_index": 0,
  "timestamp_ms": 1200,
  "phash": "hex64",
  "color_hash": "hex64",
  "edge_hash": "hex64",
  "motion_summary": "static-frame-v1"
}
```

### 4.3 local_block_fingerprints

局部块摘要默认只上传 Merkle root 和数量。正式取证或团队检索需要明细时，客户端再按授权上传不可逆明细。

```json
{
  "scene_index": 0,
  "grid": "dense_64x36",
  "row": 2,
  "col": 5,
  "phash": "hex64",
  "edge_hash": "hex64"
}
```

### 4.4 crop_window_fingerprints

裁剪候选窗口摘要是 L2 生产 API 的必要字段。

```json
{
  "scene_index": 0,
  "region": "center_80",
  "phash": "hex64",
  "edge_hash": "hex64"
}
```

## 5. API

### 5.1 创建 L2 存证

`POST /v1/video-fingerprints/notaries`

请求：

```json
{
  "schema_version": "video_fingerprint_notary_request_v1",
  "workspace_id": "ws_...",
  "creator_profile_id": "creator_...",
  "watermark_uid": "wm_...",
  "source_hash": "sha256:...",
  "duration_ms": 125000,
  "frame_sample_policy": "uniform_8_frames_v1",
  "scene_count": 8,
  "fingerprint_schema_version": "video_fingerprint_v1",
  "global_frame_fingerprints": [],
  "local_block_fingerprint_root": "sha256:...",
  "local_block_count": 912,
  "crop_window_fingerprint_root": "sha256:...",
  "crop_window_count": 56,
  "fingerprint_root": "sha256:...",
  "client_signature": "ed25519:...",
  "upload_manifest": {
    "schema_version": "video_upload_manifest_v1",
    "contains_original_video": false,
    "contains_watermarked_video": false,
    "contains_local_paths": false,
    "contains_proxy": false,
    "items": [
      {
        "kind": "video_fingerprint_bundle",
        "sha256": "sha256:...",
        "bytes": 48212
      }
    ]
  }
}
```

响应：

```json
{
  "schema_version": "video_fingerprint_notary_receipt_v1",
  "notary_id": "vfn_...",
  "watermark_uid": "wm_...",
  "source_hash": "sha256:...",
  "fingerprint_root": "sha256:...",
  "notarized_at": "2026-06-19T10:00:00Z",
  "server_receipt_signature": "ed25519:...",
  "usage_ledger_id": "usage_..."
}
```

### 5.2 查询 L2 存证

`GET /v1/video-fingerprints/notaries/{notary_id}`

返回存证摘要、收据签名、权益归属和审计状态。默认不返回局部块明细和裁剪窗口明细。

### 5.3 上传取证明细

`POST /v1/video-fingerprints/notaries/{notary_id}/evidence`

用于 Creator / Studio 正式取证。请求可包含疑似侵权视频在本地生成的不可逆指纹明细，仍不得包含原视频、加水印视频或本地路径。

### 5.4 相似性比对

`POST /v1/video-fingerprints/search`

请求：

```json
{
  "schema_version": "video_fingerprint_search_request_v1",
  "workspace_id": "ws_...",
  "query_fingerprint_root": "sha256:...",
  "global_frame_fingerprints": [],
  "local_block_fingerprints": [],
  "crop_window_fingerprints": [],
  "upload_manifest": {
    "schema_version": "video_upload_manifest_v1",
    "contains_original_video": false,
    "contains_watermarked_video": false,
    "contains_local_paths": false,
    "contains_proxy": false,
    "items": [
      {
        "kind": "query_fingerprint_bundle",
        "sha256": "sha256:...",
        "bytes": 50120
      }
    ]
  }
}
```

响应：

```json
{
  "schema_version": "video_fingerprint_search_response_v1",
  "matches": [
    {
      "notary_id": "vfn_...",
      "watermark_uid": "wm_...",
      "similarity_score": 0.93,
      "global_frame_recall": 0.88,
      "local_block_recall": 0.74,
      "crop_window_recall": 0.91,
      "notarized_at": "2026-06-19T10:00:00Z"
    }
  ]
}
```

## 6. 服务端校验

服务端必须校验：

- 账户已登录。
- `creator_profile_id` 属于当前账户或工作区。
- 套餐权益允许 L2 云端存证。
- `upload_manifest.contains_original_video=false`。
- `upload_manifest.contains_watermarked_video=false`。
- `upload_manifest.contains_local_paths=false`。
- `source_hash`、`fingerprint_root`、三层摘要 root 格式合法。
- `crop_window_fingerprint_root` 和 `crop_window_count` 存在且非空。
- `client_signature` 可验证。

服务端必须拒绝：

- 只提交整帧 root、缺少 `crop_window_fingerprint_root` 的请求。
- manifest 声明包含原始视频、加水印视频或本地路径的请求。
- 将 L2 请求计入 `video_minutes` 的账本行为。

## 7. 错误码

| 错误码 | 含义 |
| --- | --- |
| `entitlement_required` | 当前套餐不支持 L2 云端存证 |
| `creator_profile_required` | 缺少创作者身份 |
| `invalid_upload_manifest` | 上传清单不符合隐私边界 |
| `original_video_forbidden` | 请求包含原始视频 |
| `watermarked_video_forbidden` | 请求包含加水印视频 |
| `local_path_forbidden` | 请求包含本地路径 |
| `crop_windows_required` | 缺少裁剪候选窗口摘要 |
| `fingerprint_root_invalid` | 指纹 root 无效 |
| `client_signature_invalid` | 客户端签名无效 |

## 8. 验收标准

- L2 API 文档明确 L2 不是画面盲水印。
- API 请求包含整帧、局部块、裁剪候选窗口三层不可逆摘要。
- 服务端拒绝缺少 `crop_window_fingerprint_root` 的请求。
- 服务端拒绝默认上传原始视频、加水印视频和本地路径。
- L2 不扣 `video_minutes`。
- L2 可进入 `usage_ledger`。
- 取证响应能表达 `global_frame_recall`、`local_block_recall`、`crop_window_recall`。

## 9. 推荐下一步

下一步实现后端 L2 存证契约测试：在 `feedback-backend` 增加请求 schema、manifest 隐私拒绝用例、缺少 `crop_window_fingerprint_root` 的拒绝用例，以及成功请求不扣 `video_minutes` 的账本断言。
