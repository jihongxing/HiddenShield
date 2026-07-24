# L3 视频画面盲水印 release gate QA 记录

更新时间：2026-07-01

## 1. 当前结论

L3 视频画面盲水印已经通过独立 release candidate gate：完整 24 个 2K 样本池已跑完并过线，H.264-HD summary 为 `release_thresholds_met`。该结论证明 `watermark-core` 算法候选与 Tauri release gate 已达到本轮北极星样本池阈值；桌面 / 移动 succeeded task 领取、版权库 L3 收据记录、正式报告字段和真实后端双向同步运行态 QA 已落地。但 L3 仍不能单独定义为用户可承诺的正式能力；正式创建 / 上传向导、失败文案和隐私边界仍需落地。

本轮新增 gate：

- `npm run watermark:l3-video-visual-release-gate`
- 内部命令：`cargo test --release --manifest-path src-tauri/Cargo.toml l3_2k_high_bitrate_release_sample_pool_records_thresholds --lib -- --nocapture --test-threads=1`
- 强制环境变量：`HIDDENSHIELD_L3_FULL_RELEASE_POOL=1`
- 证据目录：`tmp-ui-qa/l3-video-visual-release-gate/<runId>/`

## 2. 门禁要求

完整 gate 必须覆盖 24 个 2K 样本：

- `H264-HD`: 6 个样本，每个 confidence >= 0.950，分组均值 >= 0.970。
- `H264-LT`: 4 个样本，每个 confidence >= 0.950，分组均值 >= 0.980。
- `H264-MT`: 4 个样本，每个 confidence >= 0.950，分组均值 >= 0.980。
- `H264-RISK`: 2 个样本，只允许记录为 `risk_boundary_expected`，不得计入通过率。
- `HEVC-HD`: 4 个样本，每个 confidence >= 0.970，分组均值 >= 0.990。
- `HEVC-MIX`: 4 个样本，每个 confidence >= 0.970，分组均值 >= 0.990。

阻断条件：

- 完整 24 样本未跑完。
- HEVC 样本因 `libx265` 不可用而跳过。
- 非风险样本出现 `confidence_below_threshold`、`self_check_failed`、`visual_extract_failed` 或 payload mismatch。
- H.264-HD summary 不是 `release_thresholds_met`。

## 3. 本轮运行记录

### 2026-07-01 完整池尝试

- 命令：`npm run watermark:l3-video-visual-release-gate`
- 结果：未完成。
- 观察：命令运行约 40 分钟后仍未返回，随后被外层工具超时中断。
- 影响：当时脚本只在 cargo 退出后写最终 evidence，因此这次完整池尝试没有形成完整 JSON / Markdown 证据。
- 处理：脚本已改为先创建 evidence 目录并实时写 `raw-output.log`；后续即使超时也会写 `l3-video-visual-release-gate.json` 和 `l3-video-visual-release-gate.md`。

### 2026-07-01 evidence 机制验证

- 命令：`HIDDENSHIELD_L3_RELEASE_GATE_TIMEOUT_MS=60000 npm run watermark:l3-video-visual-release-gate`
- 结果：失败，符合预期。
- 证据：`tmp-ui-qa/l3-video-visual-release-gate/1782879574862/l3-video-visual-release-gate.md`
- 失败归因：60 秒内完整 24 样本池未完成，所有分组样本数为 0，缺少 H.264-HD summary。

该短超时运行只验证 evidence 机制，不代表完整 24 样本池的算法结论。

### 2026-07-01 完整 24 样本池通过

- 命令：`npm run watermark:l3-video-visual-release-gate`
- 结果：通过。
- 证据：`tmp-ui-qa/l3-video-visual-release-gate/1782888912515/l3-video-visual-release-gate.md`
- 原始日志：`tmp-ui-qa/l3-video-visual-release-gate/1782888912515/raw-output.log`
- 总耗时：约 23 分钟（cargo test reported `finished in 1379.95s`）。
- H.264-HD：6/6 通过，min confidence `1.000`，avg confidence `1.000`，summary `release_thresholds_met`。
- H.264-LT：4/4 通过，min confidence `1.000`，avg confidence `1.000`。
- H.264-MT：4/4 通过，min confidence `1.000`，avg confidence `1.000`。
- H.264-RISK：2/2 正确记录为 `risk_boundary_expected`，不计入通过率。
- HEVC-HD：4/4 通过，min confidence `1.000`，avg confidence `1.000`。
- HEVC-MIX：4/4 通过，min confidence `1.000`，avg confidence `1.000`。

本轮算法变更：

- `watermark-core` 将 DCT mid-band embed delta 从 `72.0` 提升到 `96.0`，修复完整池中的低纹理与高频非风险样本回读失败。
- DCT 自检 confidence 语义收紧为：只有所有策略帧均被检查且跨帧融合成功时，才把融合结果记为整段 `1.000`；缺帧场景仍按逐帧匹配比例计算。
- release 样本池将 H.264-HD 从历史预期阻断改为正式通过组，必须满足 per-sample `0.950` 和 group mean `0.970`。
- H.264-RISK 样本改为 1px 棋盘 / 逐帧翻转棋盘，并显式使用 `DistributedGrid` 区域选择，保证风险边界以 `self_check_failed` 归因，而不是策略容量错误。

## 4. 后端收据门状态

`feedback-backend` 的 L3 completion 已从用户 bearer `PATCH /v1/video-tasks/:task_id/status` 拆到可信 worker/admin 专用 API：

- 内部路由：`POST /internal/video-tasks/:task_id/completion`
- 任务领取路由：`POST /internal/video-tasks/claim`
- 失败归因路由：`POST /internal/video-tasks/:task_id/failure`
- 鉴权：复用 admin bearer token。
- 收据签名：`hmac-sha256:l3-completion-v1:<digest>`
- HMAC 绑定字段：`taskId`、`strategyDigest`、`selfCheckThreshold`、`selfCheckConfidence`、`checkedFrames`、`watermarkedMediaHash`、`outputMediaStorageRef`、`outputMediaBytes`、`outputMediaContentType`、`workerReceiptHash`、`workerId`、`attemptId`、`leaseToken`。
- 用户 bearer 对 `succeeded` status update 会被拒绝：`cloud_video_task_completion_requires_trusted_worker`。

trusted completion 成功态必须携带：

- `strategyDigest`
- `selfCheckThreshold`
- `selfCheckConfidence`
- `checkedFrames`
- `watermarkedMediaHash`
- `outputMediaStorageRef`
- `outputMediaBytes`
- `outputMediaContentType`
- `workerReceiptHash`
- `workerReceipt`
- `serverReceiptSignature`
- `workerId`
- `attemptId`
- `leaseToken`

并且必须满足：

- `selfCheckConfidence >= selfCheckThreshold`
- `checkedFrames > 0`
- `outputMediaStorageRef` 必须是 `object://l3-output/...`，`outputMediaContentType = video/mp4`，`outputMediaBytes > 0`
- `workerReceiptHash` 必须等于后端对 `workerReceipt` 重算的 `sha256:<digest>`
- task 当前处于 `running`，且 `workerId`、`attemptId`、`leaseToken` 必须匹配当前有效 lease；过期 / 旧 attempt / 错 token 返回 `cloud_video_task_completion_stale_attempt`
- 已成功任务再次 completion 返回 `cloud_video_task_already_succeeded`，不会重复写入 usage ledger
- 只有 trusted completion 成功态通过后才写入 `video_minutes`
- 失败 / 取消 / 过期不写入 `video_minutes`
- retryable worker failure 回到 `queued`，保留 `lastFailureCode` / `lastFailureStage` 和 `attemptCount`；non-retryable worker failure 进入 `failed`
- 当前固定失败归因码：`manifest_invalid`、`sandbox_transcode_failed`、`core_strategy_failed`、`core_embed_failed`、`self_check_failed`、`registry_confirm_failed`、`worker_receipt_invalid`

验证：

- `cargo check --manifest-path feedback-backend/Cargo.toml`
- `cargo test --manifest-path feedback-backend/Cargo.toml cloud_video_task --lib -- --test-threads=1`
- `npm run cloud-video:contract`
- `npm run cloud-video:ci`

## 5. 受控 worker 最小闭环

2026-07-01 已新增受控 L3 worker 最小闭环：

- worker binary：`watermark-core/src/bin/l3_controlled_worker_fixture.rs`
- QA 脚本：`scripts/verify-l3-controlled-worker-e2e.mjs`
- npm 入口：`npm run cloud-video:l3-worker-qa`
- CI 集成：`npm run cloud-video:ci`

该 worker 只处理内部 fixture / 受控上传清单，不读取用户原始视频、不输出用户可下载水印视频、不接桌面或移动正式入口。闭环步骤：

1. 创建 Studio 权益下的 `cloud_video_task_v1`，`uploadManifest.items[].kind = l3_controlled_worker_fixture`，并确认 manifest 不含原始视频、水印视频或本地路径。
2. 调用 `watermark-core` 生成受控 luma 帧、`VideoFeatureBundle`、正式 payload、`VideoVisualStrategy`。
3. 通过 `embed_video_visual_dct_frames` 写入，再通过 `self_check_video_visual_dct_frames` 在成品帧上自检。
4. 输出 `strategyDigest`、`selfCheckThreshold`、`selfCheckConfidence`、`checkedFrames`、`watermarkedMediaHash`。当前 worker 同时回显任务 `watermarkUid` 与 core 派生的 `payloadWatermarkUid`，两者不得在 QA 证据里混用；正式 worker 还需补 registry-reserved UID 与 core payload 的绑定接口。
5. 普通用户 bearer 伪造 `succeeded` 被拒绝为 `cloud_video_task_completion_requires_trusted_worker`。
6. trusted worker/admin 先通过 `POST /internal/video-tasks/claim` 领取任务，获得一次性 `leaseToken`、`attemptId` 和 `leaseExpiresAt`。
7. 使用 `POST /internal/video-tasks/:task_id/completion` 固化 HMAC 收据，任务进入 `succeeded` 并写入 `video_minutes`；错误 lease 和重复 completion 均被 QA 阻断。

本机验证：

- `cargo run --manifest-path watermark-core/Cargo.toml --bin l3_controlled_worker_fixture -- --task-id l3task_local --watermark-uid wm-local-l3-worker --source-hash sha256:1111111111111111111111111111111111111111111111111111111111111111`
- 输出：`selfCheckConfidence = 1.0`、`checkedFrames = 4`、`algorithmSource = watermark-core`、`fixtureOnly = true`
- `npm run cloud-video:contract`
- `npm run cloud-video:ci`

## 6. 真实 worker first-pass 闭环

2026-07-01 已新增真实 L3 worker 执行链路第一段：

- worker binary：`watermark-core/src/bin/l3_real_worker_first_pass.rs`
- QA 脚本：`scripts/verify-l3-real-worker-first-pass-e2e.mjs`
- npm 入口：`npm run cloud-video:l3-real-worker-first-pass-qa`
- CI 集成：`npm run cloud-video:ci`

该 worker 仍只处理受控上传清单，不开放普通用户上传原始视频。已完成的执行链路能力：

1. 通过 `/v1/watermark-ids/reserve` 预留 `mediaType = video_visual`、`payloadProtocolVersion = 2`、`payloadBytesLength = 119` 的 `HS-...` registry UID。
2. 创建 `cloud_video_task_v1`，`uploadManifest.items[].kind = l3_controlled_upload_proxy`，并保留 `storageRef = controlled://l3-upload-proxy/...`、`sandboxProfile = l3_ffmpeg_transcode_sandbox_v1`、`transcodeProfile = h264_controlled_proxy_v1`。
3. worker 解析后端返回的 task JSON 和受控上传清单，拒绝原始视频、水印视频、本地路径、非 controlled storage ref、非 sandbox profile 和非 transcode profile。
4. E2E 先在受控对象目录生成真实 H.264 proxy 文件，计算真实 `sha256` / `bytes` 写入 manifest；worker 只能通过 `--controlled-object-dir` 把 `controlled://l3-upload-proxy/...` 映射到受控对象根目录，读取后再次校验 `sha256` / `bytes`，拒绝本地路径或越界路径。
5. worker 在临时 sandbox 内解码受控 H.264 proxy 为 luma frame，执行后清理 sandbox；输出只记录 hash、尺寸和帧数，不输出本地路径。
6. worker 调用 `watermark-core::build_video_visual_payload_from_reserved_uid`，把后端预留 `watermarkUid` 绑定进 core payload；QA 强制 `payloadWatermarkUid === reserved.watermarkUid`。
7. worker 调用 `watermark-core` 生成策略、写入 DCT、水印后自检；随后把写入后的 luma 帧重新封装成 MP4，写入 `--output-object-dir` 对应的 `controlled://l3-output/<taskId>/<taskId>.l3-watermarked.mp4`，并再次解码最终 MP4 做 packaged self-check。
8. worker 输出 `outputMediaStorageRef`、`outputMediaBytes`、`outputMediaContentType = video/mp4`、`workerReceipt`、`workerReceiptHash`；E2E 校验输出文件真实存在，文件大小匹配，`watermarkedMediaHash` 等于最终 MP4 文件 SHA-256。
9. E2E 调用 `/v1/watermark-ids/confirm` 把 registry 标记为 `server_confirmed`，再通过当前 claim 的 `attemptId` / `leaseToken` 调用 `/internal/video-tasks/:task_id/completion` 固化输出产物、worker receipt 和服务端收据，并写入 `video_minutes`。
10. E2E 已证明运行中 lease 不会被第二个 worker 重复领取，旧 attempt / 错 lease completion 被拒绝为 `cloud_video_task_completion_stale_attempt`，成功后重复 completion 被拒绝为 `cloud_video_task_already_succeeded` 且 usage ledger 不变。
11. E2E 已证明 retryable failure `sandbox_transcode_failed` 会回到 `queued` 并保留 `lastFailureStage = transcode_sandbox`，重新领取后 `attemptCount = 2`；旧 attempt failure 不能覆盖新 claim；non-retryable failure `manifest_invalid` 进入 `failed` 且不扣 `video_minutes`。

本机验证：

- `npm run cloud-video:contract`
- `npm run cloud-video:ci`
- 本次 CI 记录：`L3 real worker reserved uid: HS-D30609AC-D658CAF7-C7AF1A82-27E69544`，`HiddenShield L3 real worker first-pass E2E OK`
- 2026-07-01 复跑 CI 记录：task `l3task_84e0206b_313d3e8f`，reserved uid `HS-A426F285-E4ECEF9B-97F92202-4C20FCA7`，strategy `sha256:0aa4564730ad40fe0beb898af21b56c9cb69421e1d1c4825dbe4a10764742e53`，`HiddenShield L3 real worker first-pass E2E OK`
- 2026-07-01 签名下载授权复跑记录：task `l3task_eb99e92c_5d3813e6`，reserved uid `HS-1CD58E2C-A24E2172-1CECD9BB-A5F9AAD7`，strategy `sha256:21c1092a6e9ba67cea20dc4d09b5548f3f437b5158ed766137cb5f2f2cc147c6`，`cloud-video:ci` 通过，覆盖成功 task 的 `output-download-authorizations`、pending task 的 `cloud_video_task_output_not_ready` 和 tampered token 拒绝。

## 7. 签名下载授权与双端受控入口

2026-07-01 已新增 L3 输出下载授权 API 与双端 Studio / Enterprise 受控入口：

- 用户侧授权路由：`POST /v1/video-tasks/:task_id/output-download-authorizations`
- 签名解析路由：`GET /v1/video-tasks/:task_id/output-download?token=...`
- 授权 schema：`l3_output_download_authorization_v1`
- 签名 token：`hs_l3dl_v1...`，HMAC 域为 `hidden-shield:l3-output-download:v1`
- 授权只对当前登录账号名下、状态为 `succeeded`、且已持久化 `object://l3-output/...`、`video/mp4`、`watermarkedMediaHash`、`workerReceiptHash` 的 task 生效。
- 签名 token 绑定 `taskId`、`accountId`、`workspaceId`、`outputMediaStorageRef`、`outputMediaBytes`、`outputMediaContentType`、`watermarkedMediaHash`、`workerReceiptHash` 和 `expiresAt`；后续 task 输出或 receipt 被替换时旧 token 会失效。
- `GET /v1/video-tasks/:task_id/output-download?token=...` 现在返回真实 `video/mp4` 字节，并在返回前重新校验对象文件大小与 `watermarkedMediaHash`。
- `cloud-video:ci` 已证明 pending task 不能创建下载授权，tampered token 不能解析成功，签名下载字节 SHA-256 必须匹配完成态 `watermarkedMediaHash`。
- 桌面工作台视频区新增 `L3 对象上传入口 / 视频画面盲水印 release gate`，只对 Studio / Enterprise 表述为对象上传队列。
- 移动工作台新增 `视频指纹存证与 L3 对象上传入口`，同步说明 Studio / Enterprise release gate、trusted worker receipt 和签名 URL 字节下载边界。

## 8. 对象存储上传与真实字节分发

2026-07-01 已把 L3 first-pass 从受控输出推进到普通用户对象存储上传与真实字节分发：

- 新增用户侧上传授权：`POST /v1/video-tasks/object-upload-authorizations`
- 新增签名上传解析：`PUT /v1/video-object-store/upload?token=...`
- 上传授权 schema：`l3_object_upload_authorization_v1`
- 上传 token：`hs_l3up_v1...`，HMAC 域为 `hidden-shield:l3-object-upload:v1`
- 上传授权只允许具备 `cloud_video_processing` 权益的账号为当前 workspace / creator profile 创建，内容类型固定 `video/mp4`，对象类型固定 `l3_user_object_upload_proxy`，token 绑定账号、workspace、creator、对象引用、SHA-256、字节数、content type 和过期时间。
- 后端对象存储适配使用 `HIDDENSHIELD_L3_OBJECT_STORE_DIR` 作为根目录，API 只暴露 `object://l3-upload/...` / `object://l3-output/...`，不暴露本地路径。
- `l3_real_worker_first_pass` 新增 `--object-store-dir`，正式 QA 路径读取 `object://l3-upload/...`，输出 `object://l3-output/<taskId>/<taskId>.l3-watermarked.mp4`，并继续调用 `watermark-core` 完成策略、写入、自检和 packaged self-check。
- `cloud-video:ci` 复跑记录：task `l3task_622c32eb_f047e2d1`，reserved uid `HS-673AB720-861F0EC5-CA2E214E-6AF8C421`，strategy `sha256:6a66278b6f19fa3bae0bb0f02f4ed43305fe13da13b99258e6e2d3b6d342a60b`。该次 E2E 覆盖签名上传、对象存储落盘、worker 读取 object 输入、object 输出、trusted completion、pending 下载拒绝、tampered token 拒绝和签名 MP4 字节下载哈希校验。
- 2026-07-01 字段链路复跑记录：task `l3task_fdbf9509_ea7663eb`，reserved uid `HS-DA352B53-1437B497-6F1F4E87-2D53CFF1`，strategy `sha256:fdee6ad062fabce1bda570966d0c12e74aafa3c5b08f1af805452730ea457262`；同次 `cloud-video:ci` 通过新版 `cloud-video:contract`，确认对象上传 / 真实下载 E2E 与 L3 版权记录收据字段门禁同跑。

## 9. 版权库 / 报告收据字段链路

2026-07-01 已把 L3 收据元数据接入桌面 / 移动版权记录字段链路：

- 桌面版权库 `VaultRecord`、SQLite schema、查询、云同步发送 / 接收和正式报告新增 `video_visual_*` 字段，覆盖 task、完成时间、策略摘要、自检 confidence / threshold / checkedFrames、成品媒体摘要、worker receipt hash、输出字节数和 content type。
- 移动端 `VaultRecord`、SQLite schema、同步 payload、远端同步解析、版权库详情和正式报告草稿接入同一组 `video_visual_*` 字段。
- 双端字段契约 `docs/双端版权记录字段一致性契约.md` 已明确 L3 字段只保存收据元数据，不保存对象存储引用、签名下载 URL、本地路径或媒体字节。
- 桌面报告测试 `formal_report_includes_l3_video_visual_receipt_without_paths_or_urls` 固定 JSON / Markdown 均包含 L3 收据字段，并拒绝 `object://`、`output-download` 和本地路径泄露。
- `dual:contract` 与 `cloud-video:contract` 已新增 L3 字段链路和隐私边界检查，避免后续把 L2 `video_notary_*` 字段复用成 L3 画面水印能力。
- 本轮验证：`npm run dual:contract`、`npm run cloud-video:contract`、`npm run watermark:video-phase-contract`、`cargo test --manifest-path src-tauri/Cargo.toml formal_report --lib`、`npm run cloud-video:ci` 均通过。

该步骤先把完成态收据元数据落到版权库 / 报告 / 同步字段模型；后续 product-flow gate 已继续补上 succeeded task 下载、哈希复核和写入版权库入口。正式创建 / 上传向导、跨端真实运行态验证、失败文案和隐私边界仍未完成。

## 10. Succeeded Task 下载入库产品流

2026-07-01 已把 L3 succeeded task 接入桌面 / 移动产品流第一段：

- 桌面新增 Tauri 命令 `save_l3_video_visual_task_to_vault`，工作台 L3 卡片提供 taskId 输入和“下载并保存版权库”按钮。
- 移动端新增 `MobileAppState.saveL3VideoVisualTaskToVault`，工作台 L3 卡片提供同样的 taskId 输入和“下载并保存版权库”按钮。
- 两端都只接受后端已 `succeeded` 的 `video_visual` task，硬性检查 `confidence >= threshold`、`checkedFrames > 0`、`object://l3-output/...`、`video/mp4`、`watermarkedMediaHash`、worker receipt hash 和 server receipt。
- 两端都通过后端下载授权领取真实 MP4 字节，并在写入版权库前复核下载字节 SHA-256 与 `watermarkedMediaHash` 完全一致、字节数与 `outputMediaBytes` 一致。
- 写入版权库时只保存 `video_visual_*` 收据元数据和保护副本名称 / 哈希；同步和正式报告不保存对象 ref、签名 URL、本地路径或媒体字节。
- 新增独立 gate：`npm run cloud-video:l3-product-flow-gate`，并接入 `cloud-video:ci`。
- 新增真实后端双向同步运行态 QA：`npm run cloud-video:l3-cross-end-runtime-qa`，并接入 `cloud-video:ci`。
- 2026-07-01 复跑 `cloud-video:ci` 已包含该 gate，并再次跑通真实 worker first-pass：task `l3task_4d9031a6_92d923c9`，reserved uid `HS-C2F71BDB-A716E8F7-4357B6A2-97CFA30A`，strategy `sha256:98b289bf0c6e91f88b7337287afe56c18c1c651777509fcb63730677c6905a73`。

该步骤证明“已成功 L3 task -> 签名下载 -> 哈希复核 -> 版权库记录 -> 报告字段 -> 同步字段”的代码路径已落地；它仍不代表 L3 已可正式销售。

## 11. 双端运行态同步 QA

2026-07-01 已补真实后端下 desktop->mobile 与 mobile->desktop 的 L3 记录同步运行态 QA：

- QA 入口：`npm run cloud-video:l3-cross-end-runtime-qa`
- 脚本：`scripts/verify-l3-video-visual-cross-end-runtime-qa.mjs`
- CI 集成：`npm run cloud-video:ci`
- 首次单跑证据：`tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1782920564265.md`
- `cloud-video:ci` 复跑证据：`tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1782921329784.md`
- 首次单跑 Run ID：`1782920564265`
- `cloud-video:ci` Run ID：`1782921329784`
- 真实 backend：脚本启动临时 `feedback-backend`，创建同一账号下 desktop / mobile 两个 device，并通过 Studio fixture 权益启用 `cloud_sync` 与 `cloud_video_processing`。
- desktop->mobile：desktop 通过后端 `watermark-ids reserve -> confirm` 生成 `video_visual` UID `HS-3E08275C-D803EC58-5347C336-C48F845B`，推送 `upsertVaultRecord` 后，mobile 通过 `/v1/sync/changes` 拉取并验证 `video_visual_*` 全字段进入版权库详情和正式报告投影。
- mobile->desktop：mobile 通过后端 `watermark-ids reserve -> confirm` 生成 `video_visual` UID `HS-5B7F2FAE-9A863E51-06313608-38EDF252`，推送 `upsertVaultRecord` 后，desktop 通过 `/v1/sync/changes` 拉取并验证 `video_visual_*` 全字段进入版权库详情和正式报告投影。
- 双向 QA 均强制校验 `confidence >= threshold`、`checkedFrames > 0`、`video/mp4`、media hash、worker receipt hash、task id、completedAt、strategy digest、output bytes 和 content type。
- 双向 QA 均强制拒绝同步 payload、详情和报告中出现 object ref、签名上传 / 下载 URL、本地路径或媒体字节。
- 桌面版权库详情页已补齐 L3 完成时间、策略摘要、自检置信度、自检阈值、检查帧数、成品字节数和内容类型，避免 mobile->desktop 只同步字段但详情页读取不完整。

该步骤证明 L3 `video_visual_*` 收据元数据可在真实后端云同步链路中双向流转，并能被另一端版权库详情与正式报告完整读取；它仍不代表 L3 已可正式销售。

## 12. 当前阻塞项

- 桌面 / 移动已具备 L3 succeeded task 领取与版权库写入入口，但正式创建 / 上传向导仍未落地为完整可售用户操作流。
- L3 收据字段已进入版权库 / 报告 / 同步模型，且 desktop->mobile / mobile->desktop 真实后端运行态同步读取验证已完成；失败文案和隐私边界仍未完成。
- 当前 first-pass worker 与 succeeded task 领取入口只证明对象上传、FFmpeg sandbox、registry-reserved UID 与 core payload 绑定、claim / lease / replay protection、失败归因、MP4 输出封装、worker receipt 持久审计、短期签名下载授权、自检、trusted completion 和收据入库可以闭环，不代表 L3 已可正式销售。

## 13. 下一步

下一步补桌面 / 移动正式创建 / 上传向导、失败文案和隐私边界验收，再判断是否满足正式可售定义。

## 14. 正式创建 / 上传向导 + 失败文案 + 隐私边界

2026-07-01 已把桌面 / 移动 L3 正式创建与上传向导接入同一条 product-flow release gate：

- 桌面新增 `create_l3_video_visual_upload_task` Tauri 命令，工作台 L3 卡片提供“创建并上传 L3 任务”，流程为权益校验、MP4 类型校验、时长校验、签名对象上传、上传哈希 / 字节回读、registry `video_visual` UID 预留和 `cloud_video_task_v1` 创建。
- 移动端新增 `MobileAppState.createL3VideoVisualUploadTaskFromBytes`，工作台 L3 卡片提供“选择 MP4”“视频时长（秒）”“创建并上传 L3 任务”，选择文件后只保留内存字节和文件名，不把本地路径写入任务、版权库、同步或报告。
- 两端创建出的任务均为 `hybrid_visual_watermark` 队列任务，manifest 只包含 `l3_user_object_upload_proxy`、`object://l3-upload/...`、sandbox / transcode profile 和哈希 / 字节元数据；创建成功不写版权库、不标记 succeeded、不触发 `video_minutes`。
- 两端继续通过已有 succeeded task 下载入库按钮完成第二段：只有 trusted worker 自检 succeeded 并固化 receipt 后，才允许签名下载 MP4、复核 `watermarkedMediaHash` / `outputMediaBytes`，并写入 `video_visual_*` 收据元数据。
- 失败文案已覆盖权益、登录、MP4 类型、时长、上传授权、哈希回读、任务创建和 worker `failureCode`。
- 隐私边界固定为 `signed_object_upload_only_no_local_path_no_raw_video_sync`；同步和正式报告仍禁止对象 ref、签名 URL、本地路径和媒体字节。
- 当前正式创建入口仍限制 MP4 输入；MOV / WebM / MKV 等源容器必须等后端上传授权和 worker 转码入口放开后再承诺。

验证记录：

- `cargo check --manifest-path src-tauri/Cargo.toml` 通过。
- `flutter analyze` 通过。
- `npm run build` 通过。

下一步应复跑 `npm run cloud-video:l3-product-flow-gate` 与 `npm run cloud-video:ci`，把该向导门禁与真实 worker / 跨端同步 QA 组合起来作为 L3 可售前阻断项。

## 15. 真实用户 MP4 样本池可售运行态 QA

2026-07-02 已新增 L3 可售验收最小运行态 QA：

- QA 入口：`npm run cloud-video:l3-sellable-runtime-qa`
- 脚本：`scripts/verify-l3-sellable-runtime-qa.mjs`
- CI 集成：`npm run cloud-video:ci`
- 证据 JSON：`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782925001403.json`
- 证据 Markdown：`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782925001403.md`
- 可售验收清单：`docs/L3视频画面盲水印可售验收清单.md`

覆盖链路：

- desktop 与 mobile 作为同一 Studio 账号下的两个正式 device 登录。
- 两端分别上传真实 H.264 MP4 字节到 `object://l3-upload/...`。
- 后端为每个样本预留 `video_visual` UID 并创建 `hybrid_visual_watermark` task。
- trusted worker claim 后运行 `watermark-core`，输出 `object://l3-output/...` MP4，完成自检并固化 receipt。
- completion 成功后生成 `usageLedgerId`，证明 `video_minutes` 只在 trusted completion succeeded 后扣费。
- 创建端通过签名下载领取 MP4，复核下载字节 SHA-256 与 `watermarkedMediaHash` 一致，再构造 `video_visual_*` 版权记录并推送云同步。
- 另一端通过真实 `/v1/sync/changes` 拉取记录，并验证版权库详情和正式报告投影完整读取 task、strategy、confidence、threshold、checkedFrames、media hash、receipt hash、bytes 和 content type。
- QA 强制拒绝 vault / report / sync 中出现 object ref、签名上传 / 下载 URL、本地路径或媒体字节。

当前最小样本池：

- `desktop_square_motion_mp4`：desktop 创建 / 领取 / 入库，mobile 拉取读取，confidence 1.0 / threshold 0.8999999761581421，checkedFrames 4。
- `mobile_square_detail_mp4`：mobile 创建 / 领取 / 入库，desktop 拉取读取，confidence 1.0 / threshold 0.8999999761581421，checkedFrames 4。

首版运行暴露的可售阻塞项：

- 首版外延样本曾观察到 `DCT mid-band frame bitstream exceeds block capacity`；后续第 16 节已把 16:9 成功样本提升为 1280x720，并把 512x512@2fps / 8 帧固定为稳定容量输入限制。
- 移动端创建向导仍需要用户手填时长，尚未接入可信视频时长探测。
- worker 的 `strategy_invalid` 需要继续从 QA wrapper 映射推进到正式 worker 结构化失败输出和生产客服文案。

该步骤证明 L3 具备“创建上传 -> worker succeeded -> 双端领取入库 -> 跨端读取报告”的最小真实证据链；它仍不是正式可售 SLA。

下一步扩展真实用户 MP4 尺寸 / 帧率样本池，优先补 9:16 / 1080p / 真实拍摄运动与字幕样本，并把 `strategy_invalid` 结构化输出推进到正式 worker。

## 16. MP4 尺寸 / 帧率扩展样本池与 strategy_invalid 归因

2026-07-02 已把 L3 真实用户 MP4 运行态 QA 扩展到 9:16、1080p、真实运动 fixture、字幕密集和尺寸 / 帧率容量拦截边界：

- QA 入口：`npm run cloud-video:l3-sellable-runtime-qa`
- 脚本：`scripts/verify-l3-sellable-runtime-qa.mjs`
- 证据 JSON：`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782931358998.json`
- 证据 Markdown：`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782931358998.md`

本轮样本池结果：

- `desktop_square_motion_mp4`：1024x1024 / 1fps / 4 帧，desktop 创建，mobile 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `mobile_square_detail_mp4`：1024x1024 / 1fps / 4 帧，mobile 创建，desktop 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `desktop_landscape_motion_mp4`：1280x720 / 1fps / 4 帧，desktop 创建，mobile 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `mobile_square_small_high_fps_strategy_invalid`：512x512 / 2fps / 8 帧，mobile 创建请求在任务创建阶段返回 `l3_strategy_capacity_insufficient`，没有 taskId，没有 `usageLedgerId`。
- `desktop_vertical_9x16_motion_mp4`：608x1080 / 1fps / 4 帧，desktop 创建，mobile 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `mobile_landscape_1080p_motion_mp4`：1920x1080 / 1fps / 4 帧，mobile 创建，desktop 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `desktop_real_motion_fixture_mp4`：1280x720 / 1fps / 4 帧，优先使用 `tmp-ui-qa/manual-test/original-video-input.mp4` 真实运动 fixture，desktop 创建，mobile 读取，`succeeded`，confidence 1.0，checkedFrames 4。
- `mobile_subtitle_dense_mp4`：1280x720 / 1fps / 4 帧，字幕密集样本，mobile 创建，desktop 读取，`succeeded`，confidence 1.0，checkedFrames 4。

本轮代码门禁变化：

- `feedback-backend` 的 L3 任务创建校验新增容量预检：manifest 已声明 width / height / frameCount 且估算 DCT bitstream 容量不足时，返回 `l3_strategy_capacity_insufficient`。
- 桌面创建入口在上传前执行同样容量预检；移动状态层在调用方传入 width / height / frameCount 时执行同样预检。当前移动向导仍缺可信尺寸 / 帧率探测，因此后端仍是最终硬门。
- `cloud-video:l3-product-flow-gate` 现在检查容量预检、`l3_strategy_capacity_insufficient`、桌面 / 移动创建向导文案、扩展样本池和非扣费断言。
- 桌面 / 移动创建向导失败文案均包含 `strategy_invalid 容量不足`，并展示“容量预检”步骤。

本轮决策：512x512@2fps / 8 帧不在当前 L3 可售主战场内，先作为产品输入限制明确拦截，而不是为了低容量短视频立即修改 `watermark-core` payload / 策略容量。后续只有当产品决定支持低分辨率高帧率短视频时，才进入 core 策略容量改造。

当前结论不放宽 L3 可售边界：9:16、1080p、真实运动 fixture 和字幕密集已进入 release gate，但 L3 仍是 release gate 路径，不是 Studio / Enterprise 已可售 SLA。后续第 17 / 18 节已继续补入生产队列监控、移动端可信视频尺寸 / 帧率探测、对象存储清理策略和 on-call runbook；剩余可售前阻断项收敛为生产 observability 面板 / 告警平台接入、客户开通验收和更大真实用户素材目录样本池。

## 17. 生产队列运行态监控 + worker attempt SLA / 回滚演练 + 客服失败文案矩阵

2026-07-02 已把 L3 生产运营面接入独立运行态 QA 和 `cloud-video:ci`：

- QA 入口：`npm run cloud-video:l3-production-ops-runtime-qa`
- 脚本：`scripts/verify-l3-production-ops-runtime-qa.mjs`
- CI 集成：`npm run cloud-video:ci`
- 证据 JSON：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782931405830.json`
- 证据 Markdown：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782931405830.md`

本轮门禁覆盖：

- 生产队列运行态监控快照：通过真实后端 `GET /v1/video-tasks?status=queued|running|failed` 固定 `l3_production_queue_monitor_snapshot_v1`，至少输出 queued / running / failed 计数、running lease、attemptId、workerId、leaseExpiresAt 和 billing guard。
- worker attempt SLA：固定 `l3_production_worker_attempt_sla_v1`，retryable failure 在 `retryableMaxAttemptsBeforeHumanReview = 3` 内自动回到 queued，下一次 claim 后 attemptCount 增长；running lease 的运营动作固定为 `watch_lease_until_expiry_then_reclaim`。
- 回滚 / 重试演练：`sandbox_transcode_failed` 作为 retryable failure 回队列，旧 attempt 再写 failure 被 `cloud_video_task_completion_stale_attempt` 拒绝，随后 `manifest_invalid` fatal failure 进入 failed 并保持 `usageLedgerId = null`；运营动作固定为 `rollback_requeue_retryable_then_hold_failed_no_charge`。
- 下载与扣费边界：pending / failed task 的下载授权返回 `cloud_video_task_output_not_ready`；非 succeeded ops task 全部保持 `usageLedgerId = null`，成功扣费仍由 `cloud-video:l3-sellable-runtime-qa` 覆盖。
- 客服失败文案矩阵：覆盖 `l3_strategy_capacity_insufficient`、`sandbox_transcode_failed`、`core_strategy_failed`、`strategy_invalid`、`self_check_failed`、`self_check_confidence_below_threshold`、`worker_receipt_invalid`、`manifest_invalid`、`cloud_video_task_output_not_ready`，每个错误码都绑定 retryable 属性、用户标题、用户说明和客服处理动作。
- `cloud-video:l3-product-flow-gate` 现在静态检查 ops QA 脚本、package 入口、CI 集成、客服矩阵令牌和本文档记录，避免后续把生产运营门禁从 L3 可售链路中移除。

当前结论仍不放宽 L3 可售边界：该 QA 证明生产队列、attempt SLA、回滚演练和客服文案矩阵已经进入自动化 release gate；后续第 18 节已补入移动端可信视频尺寸 / 帧率探测、对象存储清理策略和 on-call runbook。真实生产 observability 面板 / 告警平台接入、客户开通验收和更大真实素材目录样本池仍未完成，不能把 L3 表述为 Studio / Enterprise 已可售 SLA。

## 18. 移动端可信视频尺寸 / 帧率探测 + 对象存储清理策略 + 生产 on-call 告警 runbook

2026-07-02 已把移动端可信视频尺寸 / 帧率探测、对象存储清理策略和生产 on-call 告警 runbook 接入同一条 `cloud-video:l3-product-flow-gate` / `cloud-video:ci`：

- 最新 CI 证据：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782933203162.md`
- 移动端新增 `mobile_app/lib/features/workspace/video_metadata.dart`，在选择 MP4 后通过 ISO BMFF box 读取 `mvhd` / `mdhd` / `tkhd` / `stts` / `stsz`，产出 `trustedVideoMetadataProbe`、时长、宽高、帧数和帧率。
- 移动端创建向导优先使用可信视频探测结果填充时长，并把 `width` / `height` / `frameCount` 传入 `createL3VideoVisualUploadTaskFromBytes`，继续复用同一条容量预检和后端 manifest 路径；探测失败时仍要求人工确认时长，后端仍是最终硬门。
- 该探测只读取容器元数据，不保存本地路径，不把对象 ref、签名 URL 或媒体字节写入版权库、同步或报告；移动端仍不实现本地 L3 画面盲水印算法。
- `cloud-video:l3-production-ops-runtime-qa` 新增 `l3_object_storage_cleanup_policy_v1`：上传 / 下载 token TTL 均限制在 900 秒内；失败、取消、过期的 `object://l3-upload/` 对象需带 no-charge guard 和审计清理；`object://l3-output/` succeeded 产物在 receipt-backed vault / report 下载窗口内保留；hash mismatch / receipt invalid 输出进入隔离，不允许客户下载或扣费。
- 同一 QA 新增 `l3_production_on_call_alert_runbook_v1`：覆盖队列 backlog、running lease 过期 / 卡住、retry exhaustion / failure spike、receipt validation failure、object storage cleanup failure 和 billing guard violation，owner 固定为 `cloud-video-on-call`，并固化 15 / 30 / 60 分钟升级动作。
- `cloud-video:l3-product-flow-gate` 已静态检查移动端可信探测源码、ops QA schema token、清理策略、on-call 告警 runbook 和本文档记录。

当前结论仍不放宽 L3 可售边界：本轮把移动端可信探测、对象清理策略和 on-call runbook 固化为 release gate 证据；但真实生产 observability 面板 / 告警平台接入、客户开通验收和更大真实用户 MP4 目录样本池仍未完成，不能把 L3 表述为 Studio / Enterprise 已可售 SLA。下一步应把生产监控面板和客户开通验收清单接入同一条可售验收链。

## 19. 生产 observability 面板 / 告警平台接入 + 客户开通验收清单

2026-07-02 已把生产 observability 面板 / 告警平台接入 + 客户开通验收清单接入同一条 `cloud-video:l3-product-flow-gate` / `cloud-video:ci`：

- 最新运行态证据：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782934265216.md`
- 最新 `cloud-video:ci` 证据：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782934387004.md`
- `cloud-video:l3-production-ops-runtime-qa` 新增 `l3_production_observability_dashboard_v1`，固定 `cloudVideoL3ProductionObservabilityDashboard` 面板定义，覆盖 `l3_queue_health`、`l3_attempt_sla`、`l3_receipt_integrity`、`l3_object_store_hygiene`、`l3_billing_guard` 和 `l3_customer_impact`。
- 每个面板都绑定现有运行态来源与告警 ID：队列快照、attempt SLA、trusted completion receipt、对象清理策略、usage ledger billing guard 和客服失败码矩阵。
- 同一 QA 新增 `l3_alert_platform_integration_v1` 和 `l3_alert_platform_delivery_dry_run_v1`，固定告警平台路由：`cloud-video-on-call-primary`、`customer-support-l3-failures`、`finance-video-minutes-guard`。
- 告警 payload 合同要求 `schemaVersion`、`alertId`、`severity`、`dedupeKey`、`dashboardId`、`runbookId`、`taskId`、`workspaceId`、`firstAction` 和隐私边界；dry-run 事件强制不包含媒体、对象 ref、签名 URL 或本地路径。
- 同一 QA 新增 `l3_customer_opening_acceptance_checklist_v1` 和 `l3_customer_opening_acceptance_dry_run_v1`，固定客户开通验收步骤：权益与 `video_minutes`、客户 MP4 dry-run、桌面 / 移动版权库与正式报告回读、隐私边界、客服矩阵与 on-call 联系人、billing guard / rollback window、客户确认 L3 仍是 MP4-only release candidate。
- `cloud-video:l3-product-flow-gate` 已静态检查 observability dashboard、alert platform integration、delivery dry-run、customer opening checklist、本文档、可售验收清单、能力边界和商业化 Roadmap。

当前结论仍不放宽 L3 可售边界：本轮把生产观测面板定义、告警平台 dry-run 路由和客户开通验收清单固化为 release gate 证据；但还需要用真实生产告警平台 / dashboard backend 承载这些 schema，并用首个试点客户真实样本完成签字验收，才能讨论把 L3 从 release candidate 推进到用户可见可售 SLA。下一步应接入真实客户试点样本池与生产告警平台配置验证。

## 20. 移动端 L2 消费补齐与本轮 cloud-video CI 复跑

2026-07-02 本轮把移动端视频能力从“L1 可验证 + L2 只读同步”推进到“L1 可验证 + L2 可提交轻量不可逆 notary + L3 release gate 继续受控”：

- 移动端 L1：`mobile_app/lib/bridge/rust_watermark_bridge.dart` 对 `WatermarkAssetKind.video` 的 read / readonly / detect 路径复用移动端音频抽取读水印能力；写入仍明确抛出 `Mobile local video watermarking is disabled.`，避免伪造移动端视频保护副本生成能力。
- 移动端 L2：`mobile_app/lib/app/mobile_app_state.dart` 新增 `createL2VideoFingerprintNotaryFromBytes`，支持 MP4 / MOV / MKV / WebM，在 Creator `cloud_sync=true` 下生成 `mobile_video_fingerprint_metadata`、`mobile_metadata_fingerprint_v1`、`mobile_metadata_probe_v1` 和 `metadata_hash_only_no_raw_video_no_local_path` 边界后调用 `/v1/video-fingerprints/notaries`，回执校验通过后写入版权库 `video_notary_*` 字段与同步队列。
- 移动端工作台：`mobile_app/lib/features/workspace/workspace_page.dart` 新增“选择 L2 视频 / 提交 L2 指纹存证 / 查看 L2 记录”，并保留 L3 MP4 对象上传与 succeeded task 下载入库入口。
- 本轮移动端单测：`flutter test test/mobile_app_state_test.dart` 通过，覆盖 mock notary 请求、manifest 隐私边界、回执入库和同步 payload 不含 local path / originalVideoPath / storageRef。
- 本轮移动端组件测试：`flutter test test/widget_test.dart` 通过，覆盖 L1 / L2 / L3 工作台文案、L2 提交按钮和 L3 权益门禁。
- 本轮静态门禁：`node scripts/verify-cloud-video-ui-contract.mjs`、`node scripts/verify-dual-consistency-contract.mjs`、`node scripts/verify-watermark-video-phase-contract.mjs` 均通过。
- 本轮云视频总门禁：`npm run cloud-video:ci` 通过。
- 最新 L3 sellable runtime 证据：`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782938523428.md`。
- 最新 L3 production ops 证据：`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782938569583.md`。

当前结论：L1 / L2 的双端消费链路已经推进到当前可承诺边界；L3 的非外部依赖代码门禁、真实 worker、对象上传 / 下载、扣费守卫、双端入库 / 报告 / 同步和 ops dry-run 已通过，但 L3 仍不能定义为正式可售 SLA。剩余阻断项必须以 release blocker 形式固化：真实告警平台配置验证、首个试点客户签字验收、更大真实用户 MP4 目录样本池 manifest。

## 21. Production readiness blocker 机器化

2026-07-02 已新增 `cloud-video:l3-production-readiness-contract` 并纳入 `cloud-video:ci`：

- 脚本：`scripts/verify-l3-production-readiness-contract.mjs`
- 默认结论：`blocked`，但退出码为 0，用于确认阻断项被显式记录，避免 L3 release candidate 被误升为 ready。
- 最新 `cloud-video:ci` readiness 证据：`tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1782938570941.md`。
- 强制可售模式：设置 `HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY=1` 后，任一真实 artifact 缺失都会失败。
- 必需 artifact：
  - `HIDDENSHIELD_L3_ALERT_PLATFORM_WEBHOOK`：真实生产 HTTPS webhook。
  - `HIDDENSHIELD_L3_ALERT_PLATFORM_VALIDATION_JSON`：`l3_alert_platform_real_delivery_validation_v1` / `status=passed`。
  - `HIDDENSHIELD_L3_PILOT_SIGNOFF_MD`：`l3_pilot_customer_signoff_v1`，并包含客户接受 MP4-only 边界与 support / rollback owner 签字。
  - `HIDDENSHIELD_L3_REAL_USER_SAMPLE_MANIFEST`：`l3_real_user_mp4_sample_manifest_v1` / `status=passed` / 至少 24 个真实用户 MP4 样本结果。

当前结论不变：L3 仍是 release candidate；只有强制可售模式通过后，才能把“当前 Studio / Enterprise 已包含 L3 视频画面盲水印”从禁止承诺中移出。

