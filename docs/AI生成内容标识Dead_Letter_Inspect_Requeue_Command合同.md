# HiddenShield AI 生成内容标识 Dead-Letter Inspect / Requeue Command 合同

合同日期：2026-07-28。

## 目标与边界

本合同冻结 internal-only post-embed dead-letter 的只读检查和受控重新入队能力。

- `inspect` 只读取最小恢复状态，不返回媒体 bytes、credential、Secret reference、provider token、完整 receipt JSON 或数据库错误。
- `requeue` 禁止直接运维 UPDATE，必须复用现有 change request、双人审批、execution 和 append-only audit 状态机。
- 仅 PostgreSQL 定义正式并发语义；SQLite 不属于生产并发 Gate。
- 本合同不开放 SDK、公共 Resolver、production credential 发放、外部生产 API 或客户自助运维入口。

## Inspect Command

必填字段：

- `executionId`
- `tenantId`
- `workspaceId`
- `environment`
- `actorSnapshotId`
- `actorTokenHash`

授权要求：

- Internal IAM 必须确认角色 `ai_transparency_readonly_auditor`。
- IAM unavailable、invalid、expired 或 scope mismatch 时 fail-closed。
- 查询必须同时匹配 execution、tenant、workspace、environment 和 `recoveryState=dead_letter`。

允许返回：

- signing status
- recovery state
- worker recovery attempts
- recovery control version
- stable reason code
- dead-letter timestamp
- last requeue change request id
- requeue timestamp

每次成功、拒绝或未找到均写入 `ai_post_embed_dead_letter_inspection_audit_events`；该表由 PostgreSQL trigger 拒绝 UPDATE/DELETE。

## Requeue Command

固定 operation：`requeue_post_embed_dead_letter`。

固定 target type：`post_embed_recovery`。

固定 target scope：`post_embed_recovery:{executionId}`。

三阶段：

1. `SubmitRequest`
   - 角色：`ai_transparency_requester`
   - 创建 `pending_review` change request。
   - 不修改 dead-letter projection。
2. `ApproveRequest`
   - 角色：`ai_transparency_security_approver`
   - approver actor 必须与 requester actor 不同。
   - 必须验证 security review reference。
   - 创建唯一 approval 并将 request 转为 `approved`。
3. `ExecuteApprovedRequest`
   - 角色：`system_executor`
   - 仅允许执行已批准且摘要、目标、版本完全匹配的 request。
   - execution、projection mutation 与 audit 必须处于同一 PostgreSQL 事务。

## Desired State Schema

Schema version：`hs-ai-post-embed-dead-letter-requeue-desired-state-v1`。

```json
{
  "schemaVersion": "hs-ai-post-embed-dead-letter-requeue-desired-state-v1",
  "executionId": "execution-id",
  "recoveryState": "retry_scheduled",
  "resetWorkerRecoveryAttempts": true,
  "nextRecoveryAt": "immediate",
  "expectedRecoveryControlVersion": 1,
  "desiredRecoveryControlVersion": 2
}
```

执行成功时必须原子完成：

- `dead_letter -> retry_scheduled`
- `worker_recovery_attempts -> 0`
- 清除 worker lease 和 dead-letter timestamp
- `next_recovery_at -> NOW()`
- `recovery_control_version -> desiredRecoveryControlVersion`
- 绑定 `last_requeue_change_request_id`
- 写入 `requeued_at`

## Request Digest Canonicalization

digest version：`hs-ai-post-embed-dead-letter-requeue-digest-v1`。

摘要输入顺序冻结为：

1. digest version
2. operation
3. target type
4. target execution id
5. target scope key
6. tenant id
7. workspace id
8. environment
9. expected control version
10. desired control version
11. desired state JSON
12. security review reference
13. requester actor id
14. requester snapshot id

编码规则为 canonical JSON array 的 UTF-8 bytes，再计算 lowercase SHA-256 hex。mode、approval id、change execution id 和 token hash 不进入摘要。

## 乐观并发与 Worker 冲突

- `recovery_control_version` 从 1 开始，desired version 必须严格等于 expected version 加 1。
- submit 通过 target lock 串行化同一 target scope 的在途 request。
- execute 使用 `FOR UPDATE OF execution` 锁定 dead-letter row。
- recovery worker 必须继续使用 `FOR UPDATE SKIP LOCKED`。
- 当 approved requeue 持有 execution row lock 时，worker 必须返回零 claim，不得等待并抢占。
- requeue 提交后，worker 才可 claim `retry_scheduled`；同一轮恢复最多一次 signer billable invocation、一次唯一 artifact stage、一次 confirm 和一次 committed ledger。

## Append-Only Audit

change audit 固定序列：

1. `change_request_submitted`
2. `approval_granted`
3. `execution_started`
4. `target_state_changed`
5. `execution_succeeded`

所有事件继续写入 `ai_transparency_change_audit_events`，并受既有 append-only trigger 保护。任何 audit 写入失败必须回滚 execution、request 状态和 dead-letter projection。

## PostgreSQL Gate 证据

- migration：`0011_ai_transparency_dead_letter_requeue_command`
- migration smoke：35 表、46 索引、0001–0011 up/down
- inspect 成功与 inspection audit append-only 已验证
- invalid digest、同人审批和未审批 execute 均零 projection 写入
- duplicate submit 返回 `idempotency_replay`
- approved requeue 与 worker claim 的真实双连接冲突中，持锁期间 worker claim 为 0
- requeue 提交后 worker 单次恢复至 `confirmed/finalized/committed`
- audit failure injection 保持 request 为 `approved`、target 为 `dead_letter`、execution 写入为 0、audit 序列为 `[1,2]`

## 当前 Gate

- 分类：`只能内部测试`。
- 允许：内部审计员 inspect；经过双人审批的内部 requeue；受控 PostgreSQL QA。
- 禁止：直接 UPDATE dead-letter、客户自助 requeue、SDK 暴露、公共 Resolver、production credential 发放、外部生产 API、生产 SLA 与法规合规承诺。

## 下一 Gate

冻结并实现 `confirmed/finalized` delivery envelope，绑定 final file hash、signer receipt reference、artifact finalize receipt reference、recovery completion 和 Profile identity；所有端侧 bridge 必须 fail-closed 拒绝非 finalized、hash mismatch 或 receipt mismatch 的产物。
