# HiddenShield AI 生成内容标识 Notification Provider Adapter 与 Delivery Gate 合同

更新时间：2026-07-28

状态：`internal_gate_verified`

能力分类：`只能内部测试`

## 1. 范围

本合同冻结 delivery security notification outbox 从 `leased` 到 `completed`、`retry_scheduled` 或 `dead_letter` 的内部生产控制面语义。

本合同不接入或伪造 PagerDuty、邮件、短信发送成功。当前唯一可执行 adapter 为 sandbox-only `zero_send`，其 receipt outcome 为 `simulated`，且 `deliveryClaimed=false`。

## 2. Destination Policy

Schema：`hs-ai-delivery-security-destination-policy-v1`

强制字段：

- `policyId`
- `version`
- `environment`
- `adapterKind`
- `deliveryMode`
- `destinationRef`
- `eventTypes`
- `minimumPriority`
- `maxDeliveryAttempts`
- `retryBaseSeconds`

约束：

- policy 必须在 adapter 调用前绑定到 outbox。
- outbox 固定保存 policy identity、version、canonical digest 和完整 JSON。
- `zero_send` 只能用于 `sandbox + simulation`。
- `pagerduty`、`email`、`sms` 只能声明 `external`，但真实 adapter 继续挂起。
- `maxDeliveryAttempts` 范围为 1–20。
- 同一 lease 已绑定不同 policy digest 时 fail-closed。

## 3. Provider Adapter Receipt

Schema：`hs-ai-delivery-security-provider-receipt-v1`

receipt 必须绑定：

- `notificationId`
- `payloadDigest`
- `destinationPolicyDigest`
- `adapterKind`
- `adapterInvocationKey`
- `issuedAt`
- `expiresAt`
- `outcome`
- `deliveryClaimed`

验收约束：

- receipt 最大有效期 900 秒。
- `adapterInvocationKey` 由 notification、delivery attempt、payload digest 和 policy digest 确定。
- `simulated` 只允许 `zero_send`，并强制 `deliveryClaimed=false`。
- `delivered` 禁止由 `zero_send` 产生，并要求非空外部 provider reference。
- receipt mismatch、过期、scope/lease mismatch 均零写入。
- 已接受 receipt 写入 append-only 表，禁止 UPDATE/DELETE。

## 4. Completion

- completion 必须持有有效 lease。
- receipt 插入、outbox `completed` 投影和 append-only audit 在同一 PostgreSQL 事务内提交。
- completion idempotency key 与 receipt digest 同时匹配时返回 replay，不重复 receipt。
- `completed` 是终态，必须清除 lease，并绑定 provider receipt id/digest。
- sandbox zero-send completion 只证明内部 delivery state machine 可完成，不证明外部通知已送达。

## 5. Failure 与 Dead Letter

- retryable failure 在 attempt budget 内进入 `retry_scheduled`。
- backoff 使用 policy 的 `retryBaseSeconds` 指数退避，最大 3600 秒。
- non-retryable failure 或达到 `maxDeliveryAttempts` 时进入 `dead_letter`。
- dead-letter 不要求 provider receipt，但必须保存 failure code、时间和 append-only audit。
- failure command 使用 idempotency key，重复执行不得重复状态转换。

## 6. Recovery

- expired lease 由 PostgreSQL `FOR UPDATE SKIP LOCKED` reclaim，且增加 recovery count。
- dead-letter 可通过 internal system executor recovery command 重排到 `retry_scheduled`。
- recovery 使用 idempotency key，重复 recovery 不重复增加 recovery count。
- recovery 不调用 provider，不生成 delivery receipt，不声称通知成功。
- recovery/replay 的 append-only audit 写入失败时，`retry_scheduled`、lease 释放及 recovery count 必须整体回滚；replay 不得因审计失败改变既有恢复状态。

## 7. PostgreSQL 持久化

Migration：`0018_ai_transparency_notification_delivery_gate`

新增：

- outbox destination policy、attempt budget、recovery、completion、receipt 和 dead-letter 字段。
- `ai_delivery_security_notification_provider_receipts` append-only 表。
- completion idempotency、dead-letter 查询和 receipt identity 索引。
- outbox audit 的 completion、failure、dead-letter 和 recovery 事件。

## 8. Gate 证据

一次性 PostgreSQL 16 数据库：

`hiddenshield_migrate_smoke_notification_gate`

已验证：

- 0001–0018 up/down migration smoke。
- sandbox zero-send receipt 完成。
- receipt/payload mismatch 零写入。
- completion audit 写入故障会使 provider receipt 插入与 outbox `completed` 投影一起回滚，通知保持原有效 `leased` 状态。
- completion replay 只保留一条 receipt。
- attempt budget 触发 dead-letter。
- dead-letter recovery 与 recovery replay。
- expired lease reclaim 与 recovery count。
- receipt append-only。
- completion audit 故障注入后 receipt 数保持 `0`、outbox 保持 `leased`，移除注入后正常 completion/replay 仍通过。
- recovery/replay audit 故障注入后分别保持 `dead_letter + 无 lease + recoveryCount=0` 与 `retry_scheduled + 无 lease + recoveryCount=1`，移除注入后正常 recovery/replay 仍通过。

## 9. 外部依赖

继续挂起：

- PagerDuty endpoint、credential 和 routing policy。
- 邮件 provider、Secret、域名认证和模板。
- 短信 provider、Secret 和模板审批。
- 真实 provider receipt authenticity validation。
- 生产 on-call 与 kill/restart 恢复演练。

## 10. 下一 Gate

内部 provider delivery Gate 已通过，工程主线转入：

1. `packages/ai-transparency-sdk`
2. 最小平台 API facade
3. 免费公共 Resolver 最小只读接口
4. 一个真实设计伙伴接入包
