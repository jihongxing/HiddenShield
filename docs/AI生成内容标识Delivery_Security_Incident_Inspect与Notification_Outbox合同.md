# AI 生成内容标识 Delivery Security Incident Inspect 与 Notification Outbox 合同

状态：**已冻结，internal-only，只能内部测试**

冻结日期：**2026-07-28**

## 1. Incident Inspect / List

- inspect 与 list 仅允许 `ai_transparency_readonly_auditor`。
- actor snapshot、tenant、workspace 与 environment 必须一致且未过期。
- inspect 只返回指定 scope 内单个 incident；跨 scope 与不存在统一不暴露对象信息。
- list 最大 100 条，可按 `open`、`acknowledged`、`resolved` 过滤，按更新时间倒序。
- 返回字段仅限 incident 状态、severity、alert codes、occurrence、summary digest 绑定、control version 与治理时间。
- 不返回 raw audit、媒体 ID、authorization ID、delivery envelope ID、token、bytes、Secret 或完整 provider receipt。
- inspect/list 的 succeeded、denied、not_found 均写 append-only inspection audit。

## 2. Provider-neutral Durable Outbox

- outbox 与 incident projection/ack/resolve 位于同一 PostgreSQL 事务。
- 支持事件：opened、became critical、acknowledged、resolved。
- dedupe key 固定绑定 `incidentId + eventType + controlVersion`。
- payload 使用稳定 JSON array canonicalization 计算 SHA-256 digest。
- payload 不包含渠道、收件人、endpoint、Secret、媒体或下载标识。
- 状态仅允许 `pending`、`leased`、`retry_scheduled`。
- 当前不存在 `sent`、`delivered` 或 provider success 状态，禁止伪造发送成功。

## 3. Lease / Replay

- claim 仅允许 `system_executor`，最大 100 条，lease 固定 5 分钟。
- claim 使用 `FOR UPDATE SKIP LOCKED`；并发 runner 对同一 notification 最多一个成功 claim。
- expired lease 可由后续 runner 回收，并写 `expired_lease_reclaimed` audit。
- replay 仅允许当前 lease owner，以 idempotency key 把 item 原子转为 `retry_scheduled` 并立即可重新 claim。
- 同一 replay idempotency key 重复执行返回既有结果，不重复增加 replay count。
- replay 不接收 provider receipt，也不得把 item 标记为已发送。
- enqueue、dedupe replay、claim、expired lease reclaim、replay scheduled 与 replay idempotency replay 均写 append-only outbox audit。

## 4. 外部依赖挂起

- PagerDuty endpoint、routing key 与 Secret：挂起。
- 邮件 provider、域名认证、收件策略与 Secret：挂起。
- 短信 provider、模板审批、号码策略与 Secret：挂起。

缺少上述配置不得阻塞 inspect/list、事务内 enqueue、dedupe、lease 或 replay Gate。

## 5. 发布边界

- SDK：关闭。
- 公共 Resolver：关闭。
- 客户 incident UI/API：关闭。
- 生产 credential 发放：关闭。
- 当前能力分类：`只能内部测试`。

## 6. Gate

- migration up/down 与空 schema rollback 通过。
- 同一事件并发或重复 enqueue 只保留一个 outbox item。
- 两个 PostgreSQL runner 并发 claim 同一 item 时最多一个成功。
- expired lease 可回收。
- replay 后同一 item 可再次 claim，且没有 provider success 状态或 receipt。
- replay idempotency 不重复增加 replay count。
- inspect/list scope mismatch 与错误角色 fail-closed。
- inspection/outbox audit 拒绝 UPDATE/DELETE。
