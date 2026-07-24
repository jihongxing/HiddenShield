# Phase 7 L2 视频指纹技术 Spike

## 1. 目标

验证 L2 画面指纹存证是否具备进入云端存证 API 设计的基础条件。

本 spike 只做本地评估：

- 从公开视频样本本地抽取关键帧。
- 生成不可逆 `VideoFingerprintBundle`。
- 对同一视频生成缩放、二压、中心裁剪攻击样本。
- 计算攻击后指纹召回率和平均距离。
- 输出 JSON / Markdown 报告。

不做：

- 不上传原始视频。
- 不上传加水印视频。
- 不创建云端任务。
- 不接入产品 UI。

## 2. 工具入口

命令：

```powershell
npm run video:fingerprint-spike -- --video-dir "D:\path\to\public-videos" --max-videos 10
```

可选参数：

- `--video-dir`：公开视频样本目录，支持 `mp4 / mov / mkv / webm`。
- `--max-videos`：最多读取多少个样本，默认 10。
- `--max-frames`：每个视频最多抽取多少帧，默认 8。
- `--output-dir`：报告输出目录，默认 `src-tauri/target/video-fingerprint-spike`。
- `--ffmpeg`：FFmpeg 命令，默认 `ffmpeg`。
- `--ffprobe`：FFprobe 命令，默认 `ffprobe`。

## 3. 输出结构

每次运行会生成：

```text
src-tauri/target/video-fingerprint-spike/run-<timestamp>/
  report.md
  report.json
  <sample>/
    bundle.json
    scale_540p.mp4
    transcode_crf32.mp4
    center_crop_80.mp4
    frames-original/
    frames-scale_540p/
    frames-transcode_crf32/
    frames-center_crop_80/
```

`bundle.json` 使用 `video_fingerprint_v1`：

- `source_hash`
- `duration_ms`
- `frame_sample_policy`
- `fingerprints[].phash`
- `fingerprints[].color_hash`
- `fingerprints[].edge_hash`
- `fingerprints[].local_blocks[]`
- `fingerprints[].crop_windows[]`
- `client_signature`

## 4. 当前指纹策略

当前 spike 采用三层不可逆摘要：

- `phash`：8x8 灰度均值感知哈希。
- `color_hash`：4x4 颜色桶摘要。
- `edge_hash`：9x8 灰度差分边缘哈希。
- `local_blocks`：固定网格 + 多尺度滑窗局部块摘要，用于缩放、二压和局部相似性辅助。
- `crop_windows`：登记端预生成中心 / 边角裁剪候选窗口摘要，用于实际裁剪后的直接匹配。
- `motion_summary`：当前先固定为 `static-frame-v1`，后续可补帧间运动摘要。

`crop_windows` 不是把被攻击视频恢复到原始画布后再提取，也不要求知道攻击参数。它是在登记端预先保存多个不可逆候选区域摘要，验证端直接拿被攻击视频的帧摘要与这些候选摘要比对。

## 5. 攻击矩阵

默认攻击：

| 攻击 | 含义 |
| --- | --- |
| `scale_540p` | 缩放到 540p 并二压 |
| `transcode_crf32` | H.264 高压缩二压 |
| `center_crop_80` | 中心裁剪 80% 后重编码 |

通过标准：

- 单个攻击召回率 `recall >= 0.70` 视为通过。
- 全部攻击通过时，才建议进入云端存证 API 字段草案。

## 6. 决策规则

如果 10 个公开视频样本中：

- 缩放和二压召回率稳定通过，说明 `phash + edge_hash` 方向可作为 L2 API v1。
- 裁剪召回率明显低，说明 API 必须保留更多局部块指纹字段，不能只存整帧 hash。
- 多数样本失败，L2 应继续停留在本地技术验证，不进入云端存证接口。
- 若整帧和局部块不足以覆盖裁剪，L2 API 必须包含 `crop_windows` 或等价的不可逆裁剪候选摘要字段。

## 7. 下一步 API 候选字段

只有当 spike 通过后，才建议把以下字段写入云端存证 API：

- `schema_version`
- `watermark_uid`
- `source_hash`
- `duration_ms`
- `frame_sample_policy`
- `fingerprint_root`
- `fingerprint_count`
- `global_frame_fingerprints`
- `local_block_fingerprint_root`
- `crop_window_fingerprint_root`
- `crop_window_count`
- `client_signature`
- `upload_manifest`

逐帧明细是否上传，需要根据报告判断：

- 整帧指纹作为快速粗筛。
- 局部块指纹作为缩放、二压和局部相似性辅助。
- 裁剪候选窗口摘要是进入 L2 云端 API 的必要字段，不能只上传整帧 root。
- 云端可优先保存 Merkle root + 抽样明细；正式取证时按用户授权上传对应不可逆明细，不上传原始视频、加水印视频或本地路径。

## 8. 当前 Smoke 结果

已用 FFmpeg `testsrc2` 合成 1 个 12 秒视频做工具链 smoke：

```powershell
npm run video:fingerprint-spike -- --video-dir src-tauri\target\video-fingerprint-smoke-samples --max-videos 1 --max-frames 6 --output-dir src-tauri\target\video-fingerprint-spike-smoke
```

结果：

- `scale_540p`：recall 1.00。
- `transcode_crf32`：recall 1.00。
- `center_crop_80`：recall 1.00。
- 报告输出：`src-tauri/target/video-fingerprint-spike-smoke/run-1781830222/report.md`。

边界：

- 该结果只证明本地工具链能生成 bundle、攻击样本和报告。
- 合成视频内容过于规则，不能代表公开视频召回率。
- 进入云端 API 字段定稿前，仍必须用 10 个公开视频样本验证。

## 9. 真实视频样本结果

样本来源：

- `E:\Users\jihx\Pictures\*.mp4`
- `--max-videos 10`
- `--max-frames 8`

第一版整帧 + 固定局部块结果：

- 总攻击：30。
- 通过：25/30。
- 平均召回率：0.85。
- `scale_540p`：10/10。
- `transcode_crf32`：10/10。
- `center_crop_80`：5/10。
- 结论：只依赖整帧 hash 或固定网格局部块，不足以支撑生产级 L2 裁剪取证。

加入 `crop_windows` 后的结果：

- 报告：`E:\Users\jihx\AppData\Local\Temp\hidden-shield-video-fingerprint-spike-crop\run-1781835877\report.md`
- 总攻击：30。
- 通过：30/30。
- 平均召回率：1.00。
- `scale_540p`：10/10。
- `transcode_crf32`：10/10。
- `center_crop_80`：10/10。

产品决策：

- L2 可以进入第一版云端指纹存证 API 草案。
- API 不得只保存 `fingerprint_root` 或整帧 hash。
- API 必须包含整帧摘要、局部块摘要、裁剪候选窗口摘要三层不可逆字段。
- L2 仍应命名为“画面指纹存证 / 相似性取证增强”，不能宣传为视频画面盲水印。
