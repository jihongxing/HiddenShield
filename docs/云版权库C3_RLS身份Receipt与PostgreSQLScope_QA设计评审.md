# 云版权库 C3 RLS、身份 Receipt 与 PostgreSQL Scope QA 设计评审

更新时间：2026-07-29

状态：`c3_contract_frozen_no_migration_or_runtime_created`

能力分类：`只能内部测试`

## 1. 评审范围与结论

本评审只冻结：

1. 未来 `0024_cloud_copyright_rls_request_scope` additive migration 的数据库安全模型；
2. verified internal identity receipt adapter 的接口、验真和失败语义；
3. 真实 PostgreSQL scope QA 的双连接场景与验收证据。

评审结论：`approved_for_controlled_c3_implementation`。

本评审不创建 migration、adapter、HTTP handler、service account、router、Desktop/Android/iOS bridge 或公开 SDK。生产云版权库、团队协作、公开 SDK 和 production credential 继续关闭。

无外部依赖合同已冻结：

- `cloud-copyright-c3-contract-v1.schema.json` 与 identity/transport fail-closed fixtures；
- `c3-rls-policy-v1.sql` 静态安全模板；
- PostgreSQL role/Secret reference 与恢复演练交接模板；
- 旧 `/v1/sync` 与未来 internal API 隔离审计。
- external-readiness dry-run 与 C3 artifact secret/placeholder scanner；两者均不连接 provider、PostgreSQL 或启动 API。
- identity receipt key-order、最大生命周期、时钟偏差和 replay boundary fixture，以及五类 RLS 危险 SQL mutation Gate。
- PostgreSQL scope QA artifact Schema 与 12 场景结果 fixture；它固定证据保留 90 天、仅内部审计导出、排除 raw identity receipt/provider response。

dry-run 只接受 Secret/evidence/IAM reference，不接受字面密码、private key、JWT 或 connection string：

```text
npm run cloud:copyright-c3-external-readiness
```

缺少外部配置时输出 `blocked` 并成功结束，明确保持 `0024`、adapter、RLS、internal API、双连接 runtime QA 与生产 role 发放关闭；无效 provider kind、无效引用或字面秘密输出 `rejected`。

## 2. 现有可复用边界

- `0023_cloud_copyright_multitenant_core` 已提供 workspace、membership、record/change/event/audit/cursor 的独立 PostgreSQL 核心；repository guard 是当前唯一运行时租户防线。
- AI Transparency 的 `InternalIamAuthorizationAdapter` 证明 receipt 缺失、过期、scope mismatch 或 provider unavailable 必须 fail-closed；但它是同步接口并绑定 AI Transparency approval command，不能直接作为云版权库身份模型或共享 role 字符串。
- C2 已冻结 `app.account_id`、`app.workspace_id`、`app.device_id`、`app.membership_id`、`app.request_id` 的 transaction-local request scope 与 internal-only operation admission。

因此 C3 新增独立 `CloudCopyrightIdentityReceiptAdapter`，只复用 fail-closed 原则，不复用 AI Transparency 的请求对象、role、数据库表或 credential。

## 3. `0024` Additive Migration 设计

未来名称固定为：

```text
0024_cloud_copyright_rls_request_scope
```

创建前必须与 `0023` 同时在一次性 PostgreSQL 库执行 up/down smoke；不得改写 `0001`、`0023`、旧 `/v1/sync` 表或 SQLite schema。

### 3.1 数据库角色

| 角色 | 权限 | 禁止 |
| --- | --- | --- |
| `hiddenshield_cloud_copyright_owner` | migration owner，`NOLOGIN` | runtime connection、业务调用 |
| `hiddenshield_cloud_copyright_app` | runtime `LOGIN NOSUPERUSER NOBYPASSRLS`，仅最小 DML | `BYPASSRLS`、`SET ROLE`、DDL、直接 table owner |
| `hiddenshield_cloud_copyright_internal_service` | 受控后台 operation，`LOGIN NOSUPERUSER NOBYPASSRLS` | 默认跨 workspace 读取、无 reason/receipt 的写入 |

每个云版权库表必须由 owner 持有，`FORCE ROW LEVEL SECURITY`；runtime role 不能成为 table owner，避免 owner bypass。

### 3.2 Scope 函数与 policy

迁移新增 private SQL helper，只读取：

```text
current_setting('app.account_id', true)
current_setting('app.workspace_id', true)
current_setting('app.device_id', true)
current_setting('app.membership_id', true)
current_setting('app.request_id', true)
current_setting('app.actor_kind', true)
current_setting('app.system_reason', true)
current_setting('app.receipt_digest', true)
```

不得把客户端 body、HTTP header 或旧 session 的值直接写进 `set_config`。repository/adapter 必须在同一已 checkout 的 `sqlx::Transaction` 中，以参数化 `SELECT set_config($1, $2, true)` 设置全部 scope；commit/rollback 后 scope 自动释放，连接返回 pool 前不得保留跨请求 GUC。

RLS 至少覆盖：

| 表 | SELECT | INSERT/UPDATE | DELETE |
| --- | --- | --- | --- |
| `cloud_copyright_records` | active membership + workspace match | active `owner/admin/editor` + device match + workspace match | 禁止 |
| `cloud_copyright_changes` | active membership + workspace match + device match | 同左；只能本 device | 禁止 |
| `cloud_copyright_events` | active membership + workspace match | repository/system actor only | 禁止 |
| `cloud_copyright_audit_events` | active `owner/admin` + workspace match | repository/system actor only + receipt/reason/request | 禁止 |
| `cloud_copyright_workspace_cursors` | active membership + workspace/device match | repository only；仅本 device cursor | 禁止 |

`cloud_copyright_workspace_memberships` 和 `cloud_copyright_workspaces` 的 policy 必须保证撤销命令仍能锁定目标 membership，同时用户请求不能读取或枚举非本 workspace 成员。

system actor 仅在 `actor_kind=system`、allowlisted operation、非空 `system_reason`、`request_id` 与 `receipt_digest` 同时满足时存在；不允许 `BYPASSRLS` 作为实现捷径。

## 4. Verified Internal Identity Receipt Adapter

未来 Rust interface 固定为异步、provider-neutral：

```text
CloudCopyrightIdentityReceiptAdapter::verify(receipt, expected_operation)
  -> VerifiedCloudCopyrightIdentity
```

`VerifiedCloudCopyrightIdentity` 只在 adapter 成功验真后生成，包含：

```text
actor_id, actor_kind, account_id, workspace_id, device_id, membership_id,
roles, request_id, operation, issued_at, expires_at, provider_id, receipt_digest
```

验真必须同时检查：

- schema version、provider allowlist、签名/JWKS 或受控 mTLS proof；
- `issued_at <= now < expires_at`，并限制最大 receipt 生命周期；
- audience、environment、operation、account/workspace/device/membership 与请求目标完全匹配；
- request digest 与 canonical request bytes 匹配；
- replay identity 在 receipt 有效期内按 `provider_id + receipt_id + request_id` 去重；
- role 是 server-side allowlist，不能信任 caller 传入 role；
- provider unavailable、JWKS stale、signature invalid、过期、scope mismatch、digest mismatch、replay ambiguity 时返回固定 fail-closed error，且在数据库 transaction 前零业务写入。

adapter 只返回最小 verified identity，不返回 access token、raw JWT、private key、完整 provider response 或可被端侧重放的 receipt。

## 5. 真实 PostgreSQL Scope QA

QA 必须连接名称含 `hiddenshield_migrate_smoke` 的一次性 PostgreSQL 库，使用两个独立 `PgPool`，不得以 SQLite、mock database、单连接或 SQL 字符串检查替代。

| 场景 | 验收 |
| --- | --- |
| valid scoped read/write | 正确 receipt + scope 只读取/写入本 workspace。 |
| missing scope | 未设置任何 required GUC 时零行、零业务写入。 |
| account/workspace mismatch | receipt target 与 `set_config` 或 record workspace 不一致时零行、零写入。 |
| device/membership mismatch | device 或 membership 与 active row 不一致时拒绝。 |
| role denied | viewer write、editor membership revoke 均由 policy 与 repository 双重拒绝。 |
| revoked after receipt | receipt 未过期但 membership 已撤销时拒绝，零 event/audit 业务链写入。 |
| expired/invalid/unavailable receipt | adapter 失败在 transaction 前，数据库 projection 和 audit count 不变。 |
| service actor reason gate | 缺 reason/request/receipt digest 的 system write 必须拒绝。 |
| pool scope bleed | connection A commit 后，connection B 无 scope 不得继承 A 的可见性。 |
| two-workspace concurrency | 两个连接各自 scope 并发写入，不泄露行、不互相覆盖。 |
| audit failure rollback | RLS 已启用时 audit insert 失败，record/change/event/cursor 全事务回滚。 |
| direct SQL denial | app role 对无 policy 或错误 scope 的 direct query 不能绕过 RLS。 |

每个场景输出 receipt class、scope digest、连接数、可见行数、record/change/event/audit/cursor 差异和 rollback 结果；不得输出 raw identity receipt。

## 6. 实施 Gate

创建 `0024` 前必须：

1. C0/C1/C2 contract 均通过，且 C2 字段、错误和禁止暴露面未变；
2. 真实 Internal IAM/JWKS 或 mTLS provider 的环境配置接口已冻结；未配置时 adapter 必须拒绝启动或拒绝 operation；
3. PostgreSQL role bootstrap、migration owner、app/service role 的 Secret reference 交接包已审批；
4. 评审 scope helper、policy SQL、service actor 原因字段与所有 12 个 QA 场景；
5. 明确旧 `/v1/sync` 不进入 C3 route，也不得被 RLS policy 静默改变。

宣布 C3 内部实现完成前必须：

- `0024` up/down smoke 与 policy/role rollback；
- verified receipt adapter invalid/expired/scope mismatch/unavailable 的零写入测试；
- 12 场景真实 PostgreSQL 双连接 scope QA；
- RLS enabled/forced 与 app role `NOBYPASSRLS` 断言；
- internal API router isolation、zero-public-route 和 append-only audit QA；
- `cargo fmt --check`、相关测试、`git diff --check`。

## 7. 明确禁止项

- 不得在 C3 实现前创建或假装配置真实 IAM endpoint、JWKS、mTLS identity、database role password 或 production Secret；
- 不得允许 Desktop/Mobile 使用 raw receipt、直接数据库连接、公开 endpoint 或 public SDK；
- 不得以 `BYPASSRLS`、table owner runtime、全局 `SET`、连接池复用 scope、默认 workspace 或客户端 role 作为降级；
- 不得将 C3 design approval 解释为生产多租户隔离、合规、团队协作、公开 API、SDK、SLA 或客户发放已可用。

下一步：在真实内部身份 provider 与 PostgreSQL role/Secret reference 外部配置可获得后，先评审 `0024` SQL 与 adapter contract test，再创建任何 migration。
