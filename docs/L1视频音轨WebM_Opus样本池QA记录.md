# L1 视频音轨 WebM/Opus 样本池 QA 记录

记录日期：2026-07-01

本记录用于固化 L1 视频音轨 WebM/Opus release gate 的真实样本池和回读证据。它不是 L3，也不是视频画面盲水印。它只说明当前 L1 视频音轨在 WebM 容器下的 Opus 回读链路，已经通过 release contract，且不再只靠一次脚本绿灯。

## 1. QA 结论

| 范围 | 结论 | 说明 |
| --- | --- | --- |
| WebM/Opus L1 成品回读 | PASS | WebM 输出使用 `libopus`，以 48kHz / 2 声道 / 160k / `application=audio` / `vbr=on` / `compression_level=10` 作为 release 参数。 |
| L1 release container matrix | PASS | `l1_video_audio_track_accepts_release_input_containers` 已进入 `watermark:cross-end-release`，并在 release contract 中稳定通过。 |
| 桌面回读验证 | PASS | WebM 成品视频抽出的音轨可被 `watermark-core` 重新识别并回读同一版权编号。 |
| 跨端口径 | PASS | 桌面 scheduler 与 release contract 使用同一回读策略，不再存在“脚本能过、生产不能过”的分叉。 |

## 2. 样本池

本次 WebM/Opus 样本池采用以下 release 组合：

| 样本 | 输入容器 | 音轨源 | 输出容器 | 关键参数 |
| --- | --- | --- | --- | --- |
| L1-WebM-1 | WebM | `sine_31s.m4a` | WebM | `libopus` / 48kHz / 2ch / 160k / `application=audio` |
| L1-WebM-2 | MP4 | `sine_31s.m4a` | MP4 | 回读 fallback 先试 44.1kHz mono，再试源轨参数 |
| L1-WebM-3 | MOV | `sine_31s.m4a` | MP4 | 回读 fallback 同上 |
| L1-WebM-4 | AVI | `sine_31s.m4a` | MP4 | 回读 fallback 同上 |
| L1-WebM-5 | MKV | `sine_31s.m4a` | MP4 | 回读 fallback 同上 |
| L1-WebM-6 | M4V | `sine_31s.m4a` | MP4 | 回读 fallback 同上 |

说明：

- 这里的“样本池”是 release gate 的可复现容器矩阵，不是一次性手工样例。
- WebM 分组是这次的重点，之前的差距口就在这里。
- L1 仍复用音频核心，样本池只证明容器回读策略可持续，不改变算法归属。

## 3. 验证命令

```bash
node scripts/verify-watermark-cross-end-contract.mjs --mode=release
```

## 4. 结果摘要

- `audio_container_fixtures_are_valid` 通过。
- `cross_end_image_bridge_contract_group` 通过。
- `cross_end_wav_core_algorithm_group` 通过。
- `cross_end_non_wav_mobile_normalize_group` 通过。
- `cross_end_non_wav_bridge_contract_group` 通过。
- `desktop_transcode_audio_fixtures_extract_to_core_wav` 通过。
- `l1_video_audio_track_roundtrip_extracts_core_watermark` 通过。
- `l1_video_audio_track_accepts_release_input_containers` 通过。
- L3 release 组按冻结策略跳过。

## 5. 相关改动

- `src-tauri/src/pipeline/scheduler.rs`
- `scripts/verify-watermark-cross-end-contract.mjs`
- `mobile_app/rust/src/api.rs`
- `docs/当前真实能力边界说明.md`
- `docs/双端能力一致性Roadmap.md`
- `docs/watermark-core能力说明.md`

## 6. 关联提交

| 提交 | 内容 |
| --- | --- |
| `8146cb1` | `fix: release AAC and WebM audio cross-end gates` |
| `636465e` | `refactor: isolate remaining watermark core changes` |

## 7. 后续注意

- 如果 WebM/Opus 的 ffmpeg 参数再变，必须先重跑这份 QA 记录对应的 release contract。
- 如果新增 WebM 以外的 L1 容器，样本池要继续补进同一份长期记录，不要另起一套口径。
- 这份记录只能说明当前 release 样本池通过，不能被写成 L3 已开放、也不能延伸到云任务或扣费。
