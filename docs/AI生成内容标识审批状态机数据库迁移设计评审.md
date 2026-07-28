# AI 生成内容标识审批状态机数据库迁移设计评审

评审日期：`2026-07-27`

评审结论：`conditional_design_pass`

实现状态：`design_review_only_no_migration`

## 1. 评审范围

本次只评审以下对象的数据库迁移设计：

```text
versioned Profile entitlement
change request
approval
execution
append-only audit
```

本次不创建 migration、不修改 SQLite 初始化、不实现写接口、不接入 Internal IAM、不发放 production license / credential、不发布 SDK。

## 2. 结论摘要

建议未来新增：

```text
0003_ai_transparency_approval_state_machine
```

迁移必须采用 additive 设计：

- 新增不可变历史表，不直接重写或删除 `0002` 表。
- `ai_profile_entitlement_versions` 成为未来 entitlement 真相源。
- 现有 `ai_profile_entitlements` 暂时保留为 current projection，确保当前内部只读查询不被破坏。
- 所有审批写入只能通过一个深事务 module 执行，调用方不得分别写 request、approval、execution、target 和 audit。
- PostgreSQL 和 SQLite 必须使用同一逻辑模型、状态与 reason code；平台差异只能存在于 adapter 内。

迁移设计可以进入 Schema Contract 阶段，但当前仍不允许创建 migration。

## 3. 为什么不能原地改表

现有 `ai_profile_entitlements`：

```text
PRIMARY KEY(license_id, profile_id)
```

它适合作为“当前 entitlement”投影，但不适合作为历史真相源：

- renew 会覆盖原有效期和审批依据。
- revoked / expired 后重新授权无法保留版本链。
- 并发 renew 无法表达 `expectedCurrentVersion`。
- approval 和 audit 无法可靠引用具体 entitlement 版本。
- rollback 或争议调查无法重建当时的授权事实。

因此禁止：

- 删除旧表后用同名新表替换。
- 向旧表添加 version 后继续原地 update 同一行。
- 将 revoked / expired 行改回 active。
- 在应用代码中维护隐藏的 JSON history。

## 4. 推荐模块与事务 Seam

未来写入实现应形成一个深 module：

```text
AiTransparencyApprovalRepository
```

外部 interface 只暴露：

```text
submit_change_request(command)
decide_change_request(command)
execute_approved_change(command)
get_change_request(id)
list_change_audit(id)
```

该 module 内部隐藏：

- request 状态转换。
- maker-checker 校验。
- role snapshot 与当前 role revalidation。
- request digest 校验。
- target lock。
- version conflict。
- entitlement version 创建与 projection 更新。
- execution 和 append-only audit。
- transaction rollback。

禁止 handler、CLI、审批 UI 或 SDK adapter 直接写底层表。

## 5. 推荐表

### 5.1 `ai_transparency_actor_role_snapshots`

用途：保存 request / approval 时从 Internal IAM 获得的不可变身份和 role snapshot，不成为身份真相源。

关键字段：

```text
actor_role_snapshot_id PK
actor_id
actor_type
role
tenant_id
workspace_id
environment
role_binding_id
role_binding_version
source_identity_system
captured_at
source_expires_at
snapshot_sha256
```

约束：

- `source_identity_system = 'hiddenshield_internal_iam'`。
- requester / approver snapshot 的 `actor_type = 'human'`。
- executor snapshot 的 `actor_type = 'system' AND role = 'system_executor'`。
- `snapshot_sha256` 为小写 SHA-256。
- snapshot append-only。

不建议在本迁移中创建可写 role binding 主表；Internal IAM 仍是唯一身份来源。

### 5.2 `ai_profile_entitlement_versions`

关键字段：

```text
profile_entitlement_version_id PK
license_id FK
profile_id
version
previous_version_id self FK
profile_kind
status
effective_at
expires_at
terms_version
legal_review_reference
security_review_reference
source_change_request_id UNIQUE
created_at
superseded_at
```

约束：

```text
UNIQUE(license_id, profile_id, version)
UNIQUE(source_change_request_id)
CHECK(version >= 1)
CHECK(expires_at > effective_at)
```

partial unique：

```text
UNIQUE(license_id, profile_id) WHERE status = 'active'
```

数据库不能只靠 FK 保证版本严格递增和 previous version 属于同一 license / profile；这些规则必须由事务 module 校验，并由真实并发测试证明。

### 5.3 `ai_transparency_change_requests`

关键字段：

```text
change_request_id PK
operation
target_type
target_id
target_scope_key
tenant_id
workspace_id
environment
expected_target_version
desired_next_version
desired_state_json
request_reason
contract_reference
legal_review_reference
security_review_reference
requester_snapshot_id FK
request_digest
idempotency_key
status
expires_at
supersedes_change_request_id self FK
created_at
updated_at
```

约束：

```text
UNIQUE(requester_snapshot_id, idempotency_key)
CHECK(request_digest is lowercase SHA-256)
CHECK(status in frozen state set)
CHECK(expires_at > created_at)
```

`target_scope_key` 必须是稳定、无秘密的 canonical key：

```text
license:{tenantId}:{workspaceId}:{environment}:{licenseId-or-new-key}
profile:{licenseId}:{profileId}
```

partial unique：

```text
UNIQUE(target_scope_key)
WHERE status IN ('pending_review', 'approved', 'executing')
```

这可阻止同一目标存在多个 in-flight request，但不能替代执行时 target row lock。

### 5.4 `ai_transparency_change_approvals`

关键字段：

```text
approval_id PK
change_request_id UNIQUE FK
decision
approver_snapshot_id FK
requester_actor_id
approver_actor_id
approver_role
decision_reason
policy_version
request_digest
decided_at
```

行内约束：

```text
CHECK(requester_actor_id <> approver_actor_id)
CHECK(decision IN ('approved', 'rejected'))
```

由于 requester 身份和 request digest 的真相位于 request 表，仍需事务 module 或数据库 trigger 检查冗余字段与 request 完全一致。只做行内 `CHECK` 不足以证明四眼分离。

### 5.5 `ai_transparency_change_executions`

关键字段：

```text
execution_id PK
change_request_id UNIQUE FK
executor_snapshot_id FK
status
target_version_before
target_version_after
resulting_entitlement_version_id
reason_code
started_at
finished_at
```

约束：

- 一个 request 最多一个 execution。
- executor 必须为 `system_executor` snapshot。
- succeeded 必须有 `finished_at` 和 target version after。
- failed / conflict 必须有稳定 `reason_code`。
- execution 行不是 target write 的替代；两者必须在同一事务。

### 5.6 `ai_transparency_change_audit_events`

关键字段：

```text
audit_event_id PK
change_request_id FK
sequence
event_type
from_state
to_state
actor_snapshot_id FK
target_type
target_id
target_version_before
target_version_after
reason_code
request_digest
details_json
occurred_at
```

约束：

```text
UNIQUE(change_request_id, sequence)
UNIQUE(change_request_id, event_type, to_state, target_version_after)
CHECK(sequence >= 1)
```

append-only：

- PostgreSQL：以权限和 `BEFORE UPDATE OR DELETE` trigger 双重阻断。
- SQLite：使用 `BEFORE UPDATE` / `BEFORE DELETE` trigger 执行 `RAISE(ABORT, ...)`。
- audit insert 必须和 request / target 状态变化处于同一事务。

### 5.7 `ai_transparency_change_target_locks`

建议新增显式目标锁表：

```text
target_scope_key PK
updated_at
```

理由：

- renew / suspend / revoke 可以锁现有 target row。
- create 操作的 target row 尚不存在，无法 `SELECT ... FOR UPDATE`。
- 单靠 partial unique index 不能覆盖请求提交前后的所有执行竞态。

执行时先 upsert lock row，再锁定该行，统一处理 create 与 existing target。

SQLite adapter 使用 immediate transaction 和同一 target key 语义；PostgreSQL adapter 使用 row-level lock。不得让调用方感知两种实现差异。

## 6. Projection 迁移策略

### Phase A：additive schema

只新增上述表、索引和 trigger，不修改现有读取。

### Phase B：backfill

将每条现有 `ai_profile_entitlements` 转为：

```text
ai_profile_entitlement_versions.version = 1
previous_version_id = null
source_change_request_id = synthetic migration request
```

必须同时生成：

- synthetic requester/system snapshot。
- synthetic succeeded change request。
- synthetic approval，明确标识 `migration_backfill`，不能伪装真实人工审批。
- synthetic execution。
-完整 audit stream。

注意：历史数据没有真实 maker-checker 证据，因此 backfill 记录必须标记：

```text
evidenceQuality = migrated_legacy_without_four_eyes
```

它只能保持现有内部测试数据可读，不能提升为 production 授权证据。

### Phase C：projection metadata

为 `ai_profile_entitlements` 增加：

```text
current_version_id
current_version
projection_updated_at
```

并通过 FK 指向版本表。未来 executor 在同一事务中写 version history 和 current projection。

### Phase D：read validation

内部只读查询同时读取 projection 和 version source，比较：

```text
status
effectiveAt
expiresAt
termsVersion
currentVersion
```

不一致时 fail-closed，并记录内部审计。

### Phase E：write enablement

只有迁移、backfill、projection 对账和真实并发测试全部通过后，才允许评审写接口实现。

## 7. 状态转换约束

数据库 `CHECK` 只负责状态集合，合法转换由事务 module 统一执行：

```text
draft -> pending_review | cancelled
pending_review -> approved | rejected | expired | cancelled
approved -> executing | expired | conflict
executing -> succeeded | failed | conflict
```

终态：

```text
succeeded
rejected
expired
cancelled
failed
conflict
```

终态记录不得 update。重试必须创建新 request 并引用 `supersedes_change_request_id`。

## 8. 事务与并发顺序

`execute_approved_change` 必须遵守：

```text
BEGIN
-> lock target_scope_key
-> lock change request
-> verify status = approved
-> verify approval digest and actor separation
-> revalidate current IAM role
-> compare expected target version
-> validate legal target transition
-> insert execution_started audit
-> set request executing
-> create entitlement version or update license
-> supersede previous entitlement version
-> update current projection
-> set execution succeeded
-> set request succeeded
-> insert execution_succeeded audit
-> insert target_state_changed audit
-> COMMIT
```

任何错误：

```text
ROLLBACK
-> 在新事务中记录 failed/conflict 终态和对应 audit
```

若失败发生在 audit insert，本次目标变化必须回滚；不能返回“目标已更新但审计失败”。

## 9. PostgreSQL / SQLite Adapter 对齐

共同 interface：

```text
submit_change_request
decide_change_request
execute_approved_change
```

PostgreSQL：

- row-level lock。
- partial unique index。
- JSONB。
- append-only trigger。

SQLite：

- `BEGIN IMMEDIATE`。
- partial index。
- canonical JSON text。
- append-only trigger。

release gate：

- 两个 adapter 对相同 fixture 输出相同状态、reason code、版本和 audit 序列。
- SQLite 只用于本地和测试，不因此获得 production runtime 资格。

## 10. 必需迁移测试

### 10.1 Up / down

- 空数据库执行 `0001 -> 0002 -> 0003`。
- 从真实 `0002` fixture 数据执行 `0003`。
- 未启用任何写入前验证 down 恢复 `0002`。
- 一旦产生真实 approval / audit 数据，禁止 destructive down，只允许 forward fix。

### 10.2 Backfill

- 每条旧 entitlement 恰好生成一个 version `1`。
- projection 与 version `1` 完全一致。
- backfill 重跑幂等。
- synthetic audit 明确 `migrated_legacy_without_four_eyes`。

### 10.3 约束

- requester = approver 被拒绝。
- 一个 request 的第二个 final approval 被拒绝。
- 同一 `(license, profile, version)` 被拒绝。
- 同一 license / profile 的第二个 active version 被拒绝。
- audit update / delete 被拒绝。
- 同一 target 的第二个 in-flight request 被拒绝。
- 非 system executor 被拒绝执行。

### 10.4 真实并发

必须使用至少两个独立数据库连接：

1. 两个相同 idempotency request：只创建一个 request。
2. 两个 renew 均期望 version `1`：一个成功生成 version `2`，另一个进入 `target_version_conflict`。
3. 两个 executor 执行同一 approved request：只允许一个成功。
4. grant 与 revoke 同时作用于同一 target：串行化后只允许一个合法结果。
5. audit insert 故障注入：target、projection、execution 和 request 全部回滚。
6. 成功或冲突均不得创建 credential、marking session、Manifest 或 ledger。

## 11. Rollback 设计

在 write enablement 前：

- 可以 down `0003`。
- down 前必须确认没有非 synthetic request / approval / execution / audit。

write enablement 后：

- 禁止删除历史审批或 audit。
- migration down 必须拒绝执行。
- 只允许 forward migration 修复 schema 或 projection。

## 12. 阻断项

创建 migration 前的六项合同已冻结：

1. request digest canonical JSON 算法与版本号。
2. 每个 operation 的 `desired_state_json` Schema。
3. Internal IAM token、actor 和 role binding 的真实性验证 interface。
4. contract / legal / security reference 的验证 interface。
5. synthetic backfill request / approval 的明确非生产证据措辞。
6. PostgreSQL 真实双连接并发测试 harness；SQLite 只承担本地 migration regression 与单元合同测试。

证据见：

```text
docs/AI生成内容标识0003迁移前置Gate合同.md
docs/contracts/ai-transparency-approval/
npm run ai-transparency:approval-contract
```

这允许开始创建 `0003`，但不代表 migration、IAM/reference adapter、并发 harness 或写入接口已实现。

## 13. 最终评审

通过项：

- additive 迁移方向。
- version history 与 current projection 分离。
- request / approval / execution / audit 独立建模。
- 显式 target lock。
- append-only audit。
- PostgreSQL 是唯一生产事务语义和并发 release Gate。
- SQLite 可保留相同命令模型的本地测试适配，但其锁、事务和并发结果不得作为生产证据。

未通过项：

- 立即创建 `0003`。
- 开放写接口。
- 使用现有 admin token 承载 maker-checker。
- 覆盖现有 entitlement 历史。
- 在 migration 和真实并发测试前发放任何 production license、credential 或 SDK。

六项合同 Gate 已完成。下一步可以创建 `0003`，并必须在真实 PostgreSQL 双连接测试通过前保持所有写接口和生产发放禁用。
