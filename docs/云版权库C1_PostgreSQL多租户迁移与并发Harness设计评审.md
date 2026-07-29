# 云版权库 C1 PostgreSQL 多租户迁移与并发 Harness 设计评审

更新时间：2026-07-29

状态：`c1_postgres_core_implemented_internal_only`

能力分类：`只能内部测试`

## 1. 评审范围与结论

本评审已落实 C1 的 PostgreSQL additive migration、workspace 隔离 repository 和真实双连接并发 harness。未接入 HTTP runtime、生产数据库、桌面/移动同步 payload 或公开 SDK。

评审结论：`approved_for_controlled_c1_implementation`。

完成仅表示内部 PostgreSQL 核心可受控验证；不表示云版权库、团队协作、生产 PostgreSQL、账户恢复或公开 SDK 已完成或可对外承诺。

## 2. 现状与差距

现有 PostgreSQL `0001_auth_sync_registry` 已有：

- `cloud_accounts`、`cloud_devices`、`cloud_sessions`；
- `cloud_sync_events`、`cloud_device_cursors`；
- watermark registry 与 rights manifest。

但其 account 行同时携带单一 workspace、creator profile 和 entitlement；同步事件按 account cursor 拉取。它不能原生表达 C0 已冻结的：

- 一个 account 加入多个 personal/team workspace；
- workspace membership、角色版本、撤销；
- `cloud-copyright-record-v1` 的版本化投影；
- record 级 idempotency、版本冲突和 append-only audit；
- workspace 级 cursor 和严格租户隔离。

因此 C1 不改写或复用现有 `cloud_sync_events` 作为云版权库 record projection；新控制面必须独立建模，并在后续客户端迁移完成前与旧同步路径并存。

## 3. Additive migration 设计

已创建 migration：

```text
0023_cloud_copyright_multitenant_core
```

已提供 PostgreSQL up/down、迁移 smoke 更新和真实 PostgreSQL QA；未创建 SQLite 生产等价物。

### 3.1 新表

| 表 | 主键与关键约束 | 目的 |
| --- | --- | --- |
| `cloud_copyright_workspaces` | `workspace_id`；`workspace_type in (personal,team)`；每个 active personal workspace 对 owner 唯一 | 独立租户与状态。 |
| `cloud_copyright_workspace_memberships` | `membership_id`；`unique(workspace_id,account_id)`；role/status/version | membership、RBAC、撤销和角色历史。 |
| `cloud_copyright_creator_profiles` | `creator_profile_id`；`account_id`；seed 仅存受控 envelope reference | 与 account/workspace 解耦的署名档案。 |
| `cloud_copyright_records` | `record_id`；`workspace_id`；`record_version`；`etag` | `cloud-copyright-record-v1` 的私有 metadata 投影。 |
| `cloud_copyright_changes` | `change_id`；`unique(workspace_id,device_id,idempotency_key)` | 幂等 change receipt、base version 与 disposition。 |
| `cloud_copyright_events` | 单调 sequence；record/workspace/change 引用 | workspace change feed；不可变领域事件。 |
| `cloud_copyright_audit_events` | 单调 sequence；actor/member/device/request digest/previous hash | append-only 安全审计。 |
| `cloud_copyright_workspace_cursors` | `workspace_id + device_id` | 新控制面的 workspace cursor，不改动旧 account cursor。 |

禁止新增 `media_bytes`、`original_path`、`protected_copy_path`、`object_ref`、`signed_url`、creator seed 明文、access/refresh token 或私钥列。

### 3.2 `cloud_copyright_records` 最小字段

该表只映射 C0 `copyright-record-v1.fixture.json` 的 allowlist：

```text
record_id, workspace_id, owner_account_id, creator_profile_id, origin_device_id,
record_kind, watermark_uid, watermark_revision, parent_watermark_uid,
original_hash, protected_copy_hash, evidence_digest, write_verification_status,
rights_declaration_json, classification, visibility,
record_version, etag, created_at, updated_at, deleted_at
```

约束：

- `(workspace_id, watermark_uid, watermark_revision)` 唯一，防止同 workspace 内 revision 重复。
- `record_version >= 1`，每次可变投影写入加一。
- `classification='private_metadata'` 与 `visibility='workspace_members'` 是 C1 允许值；公共发布继续由既有 rights projection 单独处理。
- `deleted_at` 仅实现 tombstone，不允许 C1 物理删除。

### 3.3 外键、索引与兼容性

- 所有 `workspace_id`、`account_id`、`device_id`、`creator_profile_id` 使用显式外键或同一事务中的 fail-closed existence check；不得通过客户端字符串信任跨租户引用。
- 读取索引至少为 `(workspace_id, updated_at desc, record_id)`、`(workspace_id, watermark_uid, watermark_revision)`、`(workspace_id, sequence)`。
- `cloud_copyright_changes` 的唯一 idempotency index 必须在并发写入中生效；不得只在应用层先查后写。
- 旧 `cloud_accounts.workspace_id`、`cloud_sync_events`、`cloud_device_cursors`、watermark registry 和 rights manifest 不修改、不删除、不回填；C2 完成后另行制定迁移/退役计划。

## 4. Workspace 隔离与授权设计

### 4.1 repository interface

未来 `CloudCopyrightRepository` 是独立深模块，外部 interface 只接收：

```text
actor(account_id, device_id)
+ workspace_id
+ typed change / read request
```

调用方不得传入任意 `owner_account_id`、角色、membership status 或 audit actor；这些均由 repository 在同一事务中读取并生成。

### 4.2 事务内授权顺序

每个读写事务按以下顺序执行：

```text
authenticate active session/device
→ lock/load active workspace membership
→ verify role, membership_version and entitlement
→ set PostgreSQL request scope
→ execute scoped record/change/event/audit statement
→ commit
```

`owner`/`admin` 可执行成员管理和审计导出；`editor` 可写记录但不能管理成员；`viewer` 仅读。成员状态不是 `active`、设备已撤销、workspace 非 active、entitlement 不满足或 scope 不匹配时必须零业务写入。

### 4.3 PostgreSQL 强制隔离

- C1 必须以 `workspace_id` 作为所有 record/change/event/audit 查询的第一个 scope 条件。
- 实现 RLS 时，repository 在事务内以 `set_config('app.account_id', ..., true)`、`set_config('app.workspace_id', ..., true)` 与 `set_config('app.device_id', ..., true)` 设置请求 scope；policy 同时校验 active membership。
- 在 RLS 完整接入前，repository guard 是最低要求；任何绕过 repository 的 SQL 被视为发布阻断。
- 后台 worker 使用单独 service role，并显式记录 `system_actor`、reason 和 request digest；不得继承用户 workspace scope。

## 5. Change、冲突与审计原子性

一个 change command 必须在单一 PostgreSQL transaction 内：

1. `SELECT ... FOR UPDATE` 读取 membership 和目标 record。
2. 以数据库 unique index 写入 idempotency receipt；已有相同 request digest 返回 `duplicate`，不同 digest 返回 `conflict_payload_changed`。
3. 比较 `base_record_version` 与当前 version；不一致返回 C0 固定 `conflict_version_changed`。
4. 更新或 tombstone record，写入 domain event、workspace cursor change 和 append-only audit。
5. 任意 audit/event/cursor 写入失败时完整回滚 record、change receipt 与版本增量。

成员撤销与 record push 必须竞争同一 membership row lock：撤销先提交时 push 返回 `blocked_by_membership_revoked`，push 先提交时撤销命令必须按明确审计顺序完成；不得让撤销后的第二次重放获得成功。

## 6. 真实 PostgreSQL 双连接并发 Harness

已实现 harness：

```text
feedback-backend/src/bin/cloud_copyright_postgres_concurrency_qa.rs
```

它只能连接名称含 `hiddenshield_migrate_smoke` 的一次性 PostgreSQL 测试库；必须在同一数据库上建立两个独立 `PgPool`/连接，并使用 barrier 控制竞争窗口。禁止使用 SQLite、单连接模拟锁、in-memory fake 或只验证 SQL 字符串。

### 必跑场景

| 场景 | 两个真实连接动作 | 预期 |
| --- | --- | --- |
| duplicate idempotency | 同一 workspace/device/idempotency key 同时 push | 一次 `accepted`、一次 `duplicate`；一个 record version、一个 audit。 |
| changed duplicate | 相同 idempotency key、不同 request digest 并发 push | 至少一方 `conflict_payload_changed`；无覆盖。 |
| stale version | 两个 editor 使用相同 `base_record_version` 更新同一 record | 一次成功、一方 `conflict_version_changed`；version 仅加一。 |
| revoke vs push | admin 撤销 editor membership 与 editor push 同时开始 | 撤销后所有后续 push `blocked_by_membership_revoked`；无越权 event/audit。 |
| workspace isolation | account A 读取/写入 workspace B record | `forbidden`/not found；零 record/change/event/audit 泄漏。 |
| role boundary | viewer push、editor 成员管理 | fail-closed，零业务写入。 |
| audit failure rollback | trigger 拒绝 audit insert 后执行 accepted path | record/change/event/cursor 全部回滚。 |
| delete vs update | tombstone 与 update 竞争 | 一条确定审计链；另一方 conflict，禁止 resurrect。 |

每个场景输出 JSON，包括 winner count、dispositions、record version、event/audit count、membership version、cursor 和零写入断言。

## 7. 迁移与 harness Gate

实施前已满足：

1. `npm run cloud:copyright-contract` 通过，且 C0 fixture 未改写。
2. 本文的 migration 字段、角色、错误模型与八个并发场景经评审确认。
3. 未出现任何媒体、路径、seed、token 或私钥的同步字段。
4. PostgreSQL-only disposable database 安全检查可复用，且 harness 设计使用真实双连接。
5. `CloudCopyrightRepository` interface 与 request digest canonicalization 已冻结；不得让 HTTP handler 自行拼装 SQL。

宣布 C1 内部实现完成前必须通过：

- up/down migration smoke；
- 八个真实 PostgreSQL 双连接场景；
- C0 contract fixture 与旧 cloud sync contract 均通过；
- `cargo fmt --check`、相关 Rust tests 和 `git diff --check`。

## C1 实现证据（2026-07-29）

- migration：`feedback-backend/migrations/postgres/0023_cloud_copyright_multitenant_core.up.sql` 与 down migration 已通过一次性 PostgreSQL `cloud:postgres-migrate-smoke` 上下行验证。
- repository：`CloudCopyrightRepository` 在同一 PostgreSQL transaction 内执行 membership/device 校验、record 锁定、幂等 receipt、version check、event/cursor/audit 写入与提交；record 锁定先于 change receipt 外键写入，避免并发锁升级死锁。
- QA：`npm run cloud:copyright-postgres-qa` 通过八个场景，使用两个独立 `PgPool` 与名称含 `hiddenshield_migrate_smoke` 的一次性 PostgreSQL 库；SQLite、mock 和单连接模拟均未作为证据。
- 仍关闭：production 云版权库、团队协作、HTTP/API 暴露、桌面/移动 transport、公开 SDK、生产 credential、RLS/service-role 运行态与备份恢复演练。
- 下一 Gate：冻结 C2 Rust/Dart transport mapping、RLS/request scope 与受控内部 API 合同；在该 Gate、生产 PostgreSQL 配置和恢复演练完成前不得开放任何云版权库生产写入。

## 8. 明确禁止项

- 本评审不授权创建 migration、改 `0001`、改现有 sync payload、修改 `watermark-core`、上线 RLS、连接生产数据库或发放公开 SDK。
- 不把现有 SQLite team API 或 PostgreSQL migration smoke 解释成 C1 多租户完成。
- 不以 account 级 cursor 替代 workspace cursor，不以 UI 隐藏替代数据库隔离，不以单元 mock 替代双连接并发。

## 9. 后续任务

下一工程任务：在用户明确授权后，创建 `0023_cloud_copyright_multitenant_core` PostgreSQL additive migration，并先实现 `CloudCopyrightRepository` 与八场景真实双连接 QA；完成前继续保持 production 云版权库、团队协作和公开 SDK 关闭。
