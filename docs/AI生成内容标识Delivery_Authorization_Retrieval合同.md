# AI 生成内容标识 Delivery Authorization / Retrieval 合同

状态：`frozen_internal_only_v1`
冻结日期：2026-07-28
适用范围：HiddenShield AI 生成图片标识基础设施的内部成品授权下载、检索与端侧导入准入。

## 1. 能力边界

- 本合同只允许内部服务调用，不开放 SDK、公共 Resolver、客户下载 API 或生产 credential。
- 只有已经生成 `confirmed + finalized + recovery completed` delivery envelope 的产物可以申请下载授权。
- 后端成功检索只返回“待端侧准入”的 package；Desktop/mobile 必须再次调用 `watermark-core::validate_ai_delivery_import`，成功后 bytes 才可进入 vault/import。
- 单独通过 delivery envelope 校验不等于获得下载权，也不等于获得 vault/import 权。
- 任一授权、entitlement、receipt、摘要、状态或 bridge 校验失败时，不返回媒体 bytes、receipt package、watermark UID 或可导入摘要。

## 2. 创建授权命令

内部命令：`execute_postgres_create_delivery_authorization`

输入必须绑定 `deliveryEnvelopeId`、tenant、workspace、environment、requester snapshot、Internal IAM token hash 与 `ttlSeconds`。

前置条件：

1. Internal IAM 必须验证角色 `ai_transparency_delivery_operator`，provider unavailable 时 fail-closed。
2. `requesterSnapshotId` 必须是同 tenant/workspace/environment、未过期且角色为 `ai_transparency_delivery_operator` 的 Internal IAM snapshot。
3. License 必须处于有效期内且状态为 `active`，tenant/workspace/environment 必须完全匹配。
4. Envelope 中全部 technical/regulatory Profile entitlement 必须是当前 active version，且 version 与 Profile identity 一致。
5. TTL 固定允许 `60–900` 秒。
6. 授权必须绑定 envelope digest 与 object-store finalize receipt SHA-256。

成功输出包含 `authorizationId`、`deliveryEnvelopeId`、一次性明文 `retrievalToken`、`expiresAt` 与 `envelopeDigest`。

明文 token 只返回一次；数据库只保存 SHA-256，不得写入 audit、日志、receipt JSON 或错误消息。

## 3. 检索命令

内部命令：`execute_postgres_retrieve_delivery`

输入仅包含 `authorizationId` 与 `retrievalToken`。

事务与顺序：

1. PostgreSQL 使用 `FOR UPDATE` 锁定授权行。
2. 重新验证 token hash、授权状态/有效期、License、Envelope digest、finalize receipt digest 和 Profile entitlement。
3. 成功 claim 后在同一事务将授权置为 `consumed`，写入 `retrieval_claimed` append-only audit。
4. 事务提交后通过 `PostEmbedArtifactStore::load_finalized` 读取 durable finalized bytes。
5. 后端调用共享 `validate_ai_delivery_envelope` 完成 bytes、signer receipt、finalize receipt、Profile identity 和 envelope digest 的统一校验。
6. 成功时生成并封装 `AiDeliveryRetrievalReceipt`，写入 `retrieval_succeeded` audit 后返回 package。

授权是单次使用。claim 后对象不可用或 bridge 拒绝时，授权仍保持 `consumed`，写入 `retrieval_failed`，调用方必须重新申请授权；禁止以同一 token 重试取得 bytes。

## 4. Retrieval Receipt

Schema version：`hs-ai-delivery-retrieval-receipt-v1`

必须包含：

- `retrievalReceiptId`
- `authorizationId`
- `deliveryEnvelopeId`
- `executionId`
- `envelopeDigest`
- `finalFileSha256`
- `artifactFinalizeReceiptSha256`
- `retrievedAt`
- `receiptDigest`

`receiptDigest` 对以上字段按固定数组顺序进行 SHA-256，作为跨端稳定合同。Receipt 不包含 token、Secret 引用、provider credential、媒体 bytes 或完整 IAM/KMS receipt。

## 5. 端侧 Import Admission

共享核心入口：`watermark_core::validate_ai_delivery_import`

- Desktop 内部函数：`admit_ai_delivery_for_desktop_vault_import`
- Desktop Tauri command：`admit_ai_delivery_vault_import_command`
- Mobile Rust bridge：`admit_ai_delivery_for_mobile_vault_import`

两端必须提交 envelope JSON、final media bytes、signer receipt JSON、object-store finalize receipt JSON 与 retrieval receipt JSON。

只有共享核心返回 `AiDeliveryImportAdmission { admitted: true }` 时，调用层才可继续 vault/import。拒绝响应中的 authorization ID、receipt ID、envelope digest、final hash 和 watermark UID 必须全部为空。

## 6. PostgreSQL 状态与审计

迁移：`0013_ai_transparency_delivery_authorization_retrieval`

授权表 `ai_delivery_retrieval_authorizations`：

- 状态：`active | consumed | expired | revoked`
- token hash 唯一
- envelope digest 与 finalize receipt digest 固定绑定
- `consumed` 必须存在 `consumedAt`

审计表 `ai_delivery_download_audit_events`：

- `authorization_granted`
- `retrieval_claimed`
- `retrieval_succeeded`
- `retrieval_failed`

审计为 append-only，数据库触发器拒绝 UPDATE/DELETE。审计不得包含 token、bytes、完整 receipt JSON、Secret 引用或 provider credential。

## 7. 并发与失败语义

- 同一授权的两个 PostgreSQL 连接并发检索时，最多一个进入 artifact load，最多一个返回 package。
- replay、错误 token、过期授权、License/Profile 失效、摘要不匹配、object-store unavailable 和 bridge 拒绝均 fail-closed。
- 无效 token 不消耗仍有效的授权；过期授权投影为 `expired`。
- 已 claim 后的对象读取或 bridge 失败不回滚为 `active`，避免同一授权被重复利用。
- `retrieval_succeeded` audit 写入失败时，调用失败且不返回 package；已完成的 claim 保持 `consumed`，不回滚为 `active`，成功审计行不得残留，避免同一 token 重复取得 bytes。
- 下载成功不产生 `confirmed_marked_image` 计量；下载审计不是客户计费 ledger。

## 8. Gate 证据

- 共享 fixture：`docs/contracts/ai-transparency-delivery-retrieval/success-v1.fixture.json`
- Desktop/mobile 使用同一 fixture 验证成功准入及 receipt mismatch 拒绝。
- PostgreSQL 16 migration smoke：38 tables、49 indexes、0001–0013 up/down 与空 schema rollback。
- PostgreSQL signing QA：签发、finalize、delivery envelope、授权、双连接并发检索、单次 artifact load、replay/invalid/expired、Profile revoke-after-grant、artifact unavailable、下载超时、tampered bytes 拒绝及 append-only audit 均通过；`retrieval_succeeded` audit 故障注入后不返回 package、无成功审计残留且授权保持 `consumed`。

## 9. 仍关闭的能力

- 外部 SDK 与客户 credential 发放
- 公共 Resolver
- 客户自助下载/import UI
- 生产 object-store、IAM、KMS/HSM、signer Secret 注入
- iOS/macOS runtime 互验
- 法规合规结论或生产 SLA

## 10. 下一 Gate

冻结并实现 internal delivery revocation / authorization revoke command，以及对象存储下载流的限速、最大 bytes、content-type 与超时预算；继续保持所有外部发放关闭。
