# HiddenShield AI 生成内容标识 Post-Embed Recovery Worker 合同

合同日期：2026-07-28。

## 目标

internal-only recovery worker 负责扫描并恢复：

- signing execution 为 `reserved` 且 signer lease 已过期；
- signing execution 为 `artifact_pending` 且超过配置的 finalize timeout；
- recovery worker 自身崩溃后遗留的过期 worker lease。

worker 必须复用 `ai_transparency_post_embed_signing` 单一 command module，不得实现第二套 signer、C2PA/V3 readback、confirm、ledger 或 artifact finalize 规则。

## Recovery 状态机

worker projection 与 signing execution 状态解耦：

```text
eligible
  -> leased
  -> completed
  -> retry_scheduled
  -> leased
  -> dead_letter
```

- `eligible`：满足业务状态后可被扫描。
- `leased`：已由唯一 worker claim；必须记录 owner、lease expiry 和 attempt。
- `retry_scheduled`：失败后等待 `nextRecoveryAt`。
- `dead_letter`：达到最大 attempt，或 command 已进入不可恢复业务状态。
- `completed`：execution 已 confirmed/finalized，或已进入正式 orphan terminal state。

## PostgreSQL Claim

- 使用 `FOR UPDATE SKIP LOCKED` claim。
- claim、attempt 加一、worker lease 和 `claimed` audit 必须在同一事务提交。
- 两个 worker 并发扫描同一 execution 时最多一个 claim 成功。
- worker lease 过期后允许其他 worker 重新 claim。
- `reserved` 仅在 signer lease 已过期后可 claim。
- `artifact_pending` 仅在 `updatedAt <= now - artifactPendingTimeout` 后可 claim。

## Retry 与 Dead-Letter

- retry backoff：`min(baseBackoff * 2^(attempt-1), maxBackoff)`。
- 每次失败必须清除 worker lease，写入稳定 reason code，并原子设置 `nextRecoveryAt`。
- `reserved` 失败必须释放 signer lease，避免 worker backoff 到期后仍被旧 signer lease 阻塞。
- 达到 `maxAttempts` 后进入 `dead_letter`，记录 `deadLetteredAt`，禁止自动扫描。
- execution 已离开 `reserved/artifact_pending` 且 recovery command 未成功时，必须直接进入 dead-letter，避免形成永不执行的 retry projection。

## Append-Only Audit

`ai_post_embed_recovery_audit_events` 只允许以下事件：

- `claimed`
- `succeeded`
- `retry_scheduled`
- `dead_letter`

每个事件必须绑定 execution、worker、attempt、reason、next attempt 和最小 details。禁止记录 credential、Secret reference、provider token、完整数据库错误或媒体 bytes。表必须由 PostgreSQL trigger 拒绝 UPDATE/DELETE。

## 当前 Gate

- 已实现 `0010_ai_transparency_post_embed_recovery_worker`。
- 已实现 internal-only batch worker module。
- PostgreSQL 16 已验证 expired reserved、artifact pending timeout、三次退避进入 dead-letter、双 worker 并发单 claim。
- append-only audit UPDATE/DELETE 拒绝已验证。
- 迁移 smoke：34 表、45 索引、0001–0010 up/down。
- 当前不是 production daemon、外部 API、SDK、公共 Resolver、客户 credential、生产 SLA 或真实 provider 恢复演练。

## 下一 Gate

冻结并实现 internal-only dead-letter inspect/requeue command；requeue 必须经过既有 change request、双人审批、execution 和 append-only audit 状态机，禁止直接修改 dead-letter 行。
