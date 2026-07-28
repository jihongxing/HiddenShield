# AI 生成内容标识 0003 迁移前置 Gate 合同

版本：`v1`

冻结日期：`2026-07-27`

实现状态：`six_pre_migration_gates_frozen_0003_atomic_command_and_fail_closed_adapter_boundary_implemented`

## 1. Gate 结论

本合同冻结创建 `0003_ai_transparency_approval_state_machine` 前的六项阻断：

1. request digest canonicalization。
2. 八类 operation 的 `desiredState` Schema。
3. Internal IAM 验真 interface。
4. contract / legal / security reference 验真 interface。
5. synthetic backfill 固定证据措辞。
6. PostgreSQL 真实双连接并发测试 harness；SQLite 只允许用于本地迁移和单元合同测试，不构成并发或 release Gate。

六项合同已冻结；`0003` additive schema、migration regression test、单一内部 change-command 原子事务 module、fail-closed IAM/reference adapter boundary 与 PostgreSQL 真实双连接 harness 已于 `2026-07-27` 创建并通过。SQLite 结果只保留为本地兼容性证据，不参与生产 Gate。后续 PostgreSQL-only confirm 原子事务与 `0004` confirm audit 已实现并通过本地真实并发 Gate；生产发放仍未开放。

## 2. Request Digest Canonicalization

算法版本：

```text
hs-ai-change-request-digest-v1
```

摘要：

```text
SHA-256(UTF-8(canonicalArray))
```

`canonicalArray` 固定顺序：

```text
[
  digestVersion,
  operation,
  targetType,
  targetIdOrNull,
  targetScopeKey,
  tenantId,
  workspaceId,
  environment,
  expectedTargetVersionOrNull,
  desiredNextVersionOrNull,
  canonicalDesiredState,
  contractReferenceOrNull,
  legalReviewReferenceOrNull,
  securityReviewReferenceOrNull,
  requesterActorId,
  requesterRoleBindingId,
  requesterRoleBindingVersion
]
```

规则：

- 所有输入先通过 Schema 验证，再 canonicalize；禁止在摘要阶段修复非法值。
- 字符串必须为 Unicode NFC、无首尾空白；ID 大小写保持原值。
- 时间必须为 UTC RFC3339 秒精度：`YYYY-MM-DDTHH:mm:ssZ`。
- 数字只允许安全整数；禁止浮点、指数形式、`NaN` 和负零。
- `null` 必须显式编码，禁止省略数组位置。
- `canonicalDesiredState` 使用对应 operation 的固定字段顺序。
- JSON 使用 UTF-8、无 BOM、无多余空白。
- 数组顺序不可改变。
- digest 输出为 64 字符小写十六进制。
- `changeRequestId`、idempotency key、status 和时间戳不进入摘要。

固定测试向量见 `pre-migration-gates-v1.fixture.json`。

## 3. Desired State Schema

Schema：

```text
docs/contracts/ai-transparency-approval/ai-transparency-desired-state-v1.schema.json
```

八类 operation：

```text
create_license
renew_license
suspend_license
revoke_license
grant_profile_entitlement
renew_profile_entitlement
suspend_profile_entitlement
revoke_profile_entitlement
```

所有对象：

- `additionalProperties = false`。
- 不允许写 credential、API key、issuer private key、price、payment、ledger、Manifest 或媒体字段。
- 引用字段继续位于 change request 顶层，不放入 `desiredState`。

## 4. Internal IAM 验真 Interface

module interface：

```text
verify_actor_authorization(input) -> ActorAuthorizationDecision
```

输入：

```text
tokenHash
requiredRole
tenantId
workspaceId
environment
operation
verifiedAt
```

禁止向审批数据库或审计写入原始 token。

成功输出：

```text
authorized = true
actor snapshot
role binding snapshot
sourceIdentitySystem = hiddenshield_internal_iam
authenticationLevel
sourceExpiresAt
verificationReceiptId
```

稳定拒绝码：

```text
iam_token_invalid
iam_token_expired
iam_actor_inactive
iam_actor_type_denied
iam_role_missing
iam_scope_denied
iam_authentication_level_insufficient
iam_unavailable
```

规则：

- requester / approver 必须为 human。
- executor 必须为 system 且 role 为 `system_executor`。
- tenant、workspace、environment 和 operation 必须同时匹配。
- `iam_unavailable` 必须 fail-closed。

## 5. 外部引用验真 Interface

module interface：

```text
verify_approval_reference(input) -> ApprovalReferenceDecision
```

输入：

```text
referenceType
referenceId
tenantId
workspaceId
environment
operation
verifiedAt
```

`referenceType`：

```text
contract
legal_review
security_review
```

成功输出：

```text
verified = true
referenceStatus = active
authority
scopeDigest
validFrom
validUntil?
verificationReceiptId
```

稳定拒绝码：

```text
reference_not_found
reference_inactive
reference_expired
reference_scope_mismatch
reference_operation_denied
reference_authority_untrusted
reference_unavailable
```

规则：

- 不复制完整合同或法务文件，只保存 opaque reference、scope digest 和 receipt。
- `reference_unavailable` 必须 fail-closed。
- production regulatory Profile grant / renew 必须验证 legal review。
- production technical Profile grant / renew 必须验证 security review。
- production license create / renew 必须验证 contract。

## 6. Synthetic Backfill 证据措辞

固定机器字段：

```text
evidenceQuality = migrated_legacy_without_four_eyes
productionEligibility = false
historicalHumanApprovalAsserted = false
```

固定中文措辞：

```text
此授权记录由旧版数据结构迁移生成。HiddenShield 不声明该历史记录曾完成双人审批；该记录不能单独作为生产 License、生产 Credential 或 SDK 发放依据。
```

固定英文措辞：

```text
This entitlement record was migrated from a legacy schema. HiddenShield does not assert that historical maker-checker approval occurred. This record alone is not sufficient for production license, production credential, or SDK issuance.
```

禁止：

- 使用 `approved`、`compliant`、`production_verified` 描述 synthetic approval。
- 将 migration system actor 伪装为 human approver。
- 用 backfill 记录满足未来 production four-eyes Gate。

## 7. PostgreSQL 双连接并发 Harness

统一 harness interface：

```text
createAdapter(databaseKind)
applyMigrationsThrough0003()
seedScenario(fixture)
openConnection(label)
runBarrier(name, participants)
executeCommand(connection, command)
snapshotState()
assertNoProductionSideEffects()
dispose()
```

- 两个独立 pool connection。
- row lock 和 target lock。
- 必须运行在一次性本地 PostgreSQL 数据库，数据库名包含 `hiddenshield_migrate_smoke`。
- SQLite 仅可用于 migration regression、SQL 约束和本地单元测试；不得输出 production concurrency 结论。

必测场景：

```text
duplicate_idempotency_request
concurrent_profile_renew
duplicate_execution
grant_vs_revoke_same_target
audit_failure_rollback
projection_version_conflict
```

每个场景必须由 PostgreSQL 双连接输出：

```text
winnerCount
requestStatuses
targetVersion
activeVersionCount
auditSequence
reasonCodes
credentialCount = 0
markingSessionCount = 0
manifestCount = 0
ledgerCount = 0
```

## 8. 可执行 Gate

fixture：

```text
docs/contracts/ai-transparency-approval/pre-migration-gates-v1.fixture.json
docs/contracts/ai-transparency-approval/concurrency-harness-v1.fixture.json
```

验证命令：

```text
npm run ai-transparency:approval-contract
```

## 9. 允许创建 0003 的条件

只有同时满足：

- 两个新增 fixture 通过 contract test。
- desiredState Schema 覆盖八类 operation。
- digest 固定向量通过。
- IAM 和 reference 验真 reason code 冻结。
- backfill 中英文措辞逐字冻结。
- concurrency harness 明确使用 PostgreSQL 双连接。

才允许开始创建 `0003`。

即使允许创建 migration，在 migration 与真实 PostgreSQL 双连接并发测试通过前，仍禁止 license / Profile 写接口和所有生产发放。
