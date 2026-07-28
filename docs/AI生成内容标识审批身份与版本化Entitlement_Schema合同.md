# AI 生成内容标识审批身份与版本化 Entitlement Schema 合同

版本：`v1`

冻结日期：`2026-07-27`

历史冻结状态：`schema_and_fixture_contract_test_frozen_no_database_implementation`

当前实现状态：`versioned_entitlement_postgresql_0003_and_change_command_implemented_internal_only`

versioned Profile entitlement、actor/role snapshot、change request、双人审批、execution 与 append-only audit 已在 PostgreSQL 控制面实现并通过真实双连接并发 Gate。真实 IAM 身份来源、法务/合同/security reference provider 与 production 发放仍为外部依赖。

## 1. 当前边界

本合同冻结双人审批所需的 actor / role 身份来源和 versioned Profile entitlement 逻辑模型。

本轮不创建数据库迁移、身份服务、角色管理接口、写入接口、production license、production credential 或 SDK。

## 2. Actor 身份来源

唯一允许的身份来源：

```text
HiddenShield Internal IAM
```

现有单一 admin token 只用于当前只读内部 API，不能作为以下事实的来源：

- 自然人 actor 身份。
- requester / approver 分离。
- role binding。
- employment / account status。
- production environment 授权。
- approval 时点权限快照。

未来认证上下文必须由 Internal IAM 签发短期 token，并提供：

```text
actorId
actorType
subjectId
displayName
status
authenticationLevel
sessionId
issuedAt
expiresAt
```

`actorType`：

```text
human
service
system
```

maker-checker 的 requester 和 approver 必须为 `human` 且 `status = active`。

## 3. Role Binding

逻辑对象：`ai_transparency_actor_role_bindings`

```text
roleBindingId
actorId
role
tenantScope[]
workspaceScope[]
environmentScope[]
status
effectiveAt
expiresAt
grantedByActorId
approvalReference
createdAt
revokedAt?
```

允许角色：

```text
ai_transparency_requester
ai_transparency_commercial_approver
ai_transparency_compliance_approver
ai_transparency_security_approver
ai_transparency_readonly_auditor
system_executor
```

不变量：

- `system_executor` 只能绑定 `actorType = system`。
- 其他审批角色不得绑定 service / system actor。
- binding 必须同时匹配 tenant、workspace 和 environment。
- approval 执行前必须重新检查 binding 仍有效。
- revoked / expired binding 不得用于新审批或执行。
- role binding 变更本身不属于本轮 license / Profile 写入状态机，未来必须有独立治理流程。

## 4. Actor Snapshot

每个 change request 和 approval 必须保存不可变身份快照：

```text
actorId
actorType
role
tenantScope
workspaceScope
environmentScope
roleBindingId
roleBindingVersion
capturedAt
```

执行时使用当前 IAM 状态重新授权，但历史审计保留提交或审批时 snapshot。

## 5. Versioned Profile Entitlement

逻辑对象：`ai_profile_entitlement_versions`

```text
profileEntitlementVersionId
licenseId
profileId
version
previousVersionId?
profileKind
status
effectiveAt
expiresAt
termsVersion
legalReviewReference?
securityReviewReference?
sourceChangeRequestId
createdAt
supersededAt?
```

唯一约束：

```text
UNIQUE(licenseId, profileId, version)
UNIQUE(sourceChangeRequestId)
```

active partial unique：

```text
每个 (licenseId, profileId) 最多一个 status = active 的版本
```

版本链不变量：

- `version` 从 `1` 开始且严格递增。
- `version > 1` 必须引用同一 license / profile 的前一版本。
- 前一版本必须在新版本生效事务中变为 `superseded`。
- revoked / expired / superseded 版本不可修改、复活或删除。
- renew 创建新版本，不覆盖旧版本。
- grant 在不存在历史版本时创建 version `1`。
- suspend / revoke 创建状态事件并关闭当前版本，不创建伪造的新 active 版本。
- regulatory Profile 的 production grant / renew 必须有 `legalReviewReference`。
- technical Profile 的 production grant / renew 必须有 `securityReviewReference`。

## 6. 查询投影

当前 `ai_profile_entitlements` 在未来迁移后只能作为 active projection 或兼容视图：

```text
ai_profile_entitlements
= latest effective entitlement projection
```

真相源必须是 `ai_profile_entitlement_versions`，禁止继续用 `(licenseId, profileId)` 覆盖历史记录。

## 7. Change Request 引用

涉及 Profile 的 change request 必须记录：

```text
targetProfileId
expectedCurrentVersion
expectedCurrentVersionId?
desiredNextVersion
desiredState
```

执行时：

```text
expectedCurrentVersion == current version
desiredNextVersion == current version + 1
```

否则进入 `target_version_conflict`，不得修改 projection 或历史版本。

## 8. Fixture 与测试

固定向量目录：

```text
docs/contracts/ai-transparency-approval/
```

必须覆盖：

- human requester、human approver 和 system executor。
- role binding 的 tenant / workspace / environment scope。
- requester 与 approver 分离。
- entitlement version 链和单 active version。
- production regulatory Profile 的 legal review reference。
- request digest 与 approval digest 一致。
- version conflict 不写目标状态。
- append-only audit 顺序。
- 所有审批与执行路径不创建 credential、marking session 或 ledger。

## 9. 下一 Gate

身份、版本化 Schema、fixture 和状态机 contract test 已通过；`0003` 迁移、真实 PostgreSQL 并发测试与 confirm 原子事务也已通过内部 Gate。当前仍禁止 production 发放，直至真实 IAM/reference provider、法务 Profile 签署与其他 production provider Gate 完成。
