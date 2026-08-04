# 云版权库 C2 Transport、RLS 与内部 API 合同

更新时间：2026-07-29

状态：`c2_contract_frozen_no_runtime_or_migration`

能力分类：`只能内部测试`

## 1. 冻结结论

C2 只冻结三个后续实现边界：

1. Desktop Rust 与 Mobile Rust/Dart 的 `cloud-copyright-record-v1` transport mapping；
2. PostgreSQL transaction-local request scope 与 RLS policy 目标；
3. 仅服务间可调用的 cloud copyright internal API admission。

本阶段不创建 `0024` migration、不启用 RLS、不注册 HTTP router、不改写 Desktop/Android/iOS bridge、不接入生产 PostgreSQL，不开放团队协作、公开 SDK 或 production credential。

## 2. 统一术语

- **Transport Admission**：端侧 outbox 在本地白名单和 workspace context 校验后，向受控内部服务提交 change 的资格；不是端侧直接访问数据库或公开 API 的资格。
- **Request Scope**：服务端从已验证 internal identity receipt 注入的 `account/workspace/device/membership/request` 五元组；客户端请求体不能声明或覆盖该 scope。
- **Internal API Admission**：受控服务调用每个 operation 前的身份 receipt、角色、membership、workspace 与 request digest 校验；它不是公开 API key 或 SDK 授权。

## 3. Rust/Dart Transport Mapping

两端必须传递同一逻辑 envelope：

```text
workspaceId + recordId + baseRecordVersion + idempotencyKey + requestDigest
```

规则：

- 记录字段只允许 C0 `cloud-copyright-record-v1` allowlist；禁止媒体 bytes、路径、seed、token、private key、signed URL 和 object reference。
- `workspaceId` 来自已选定的本地 workspace context；端侧不得根据 account 默认 workspace 静默替换。
- `accepted` 才能将本地 outbox 标记为 `synced`；其 `serverRecordVersion` 与 `cursorSequence` 必须持久化为同一次 server receipt。
- `duplicate` 只在 request digest 匹配既有 receipt 时可标记为 `synced`。
- `conflict_version_changed`、`blocked_by_membership_revoked`、`forbidden`、`role_denied`、`scope_mismatch` 与 `internal_identity_unavailable` 一律保留本地草稿、停止自动重试，并显示同一 fail-closed 文案。
- Desktop 和 Mobile 不得直接调用 internal API；未来由各端 bridge 调用同一受控 CloudCopyright transport adapter。

正式跨端 fixture 必须覆盖 desktop-write/mobile-read、mobile-write/desktop-read、stale version、membership revoked 和 scope mismatch；C2 仅冻结 fixture，不实现端侧行为。

## 4. PostgreSQL Request Scope 与 RLS

每个 internal command transaction 必须：

```text
verify internal identity receipt
→ begin
→ set_config('app.account_id', ..., true)
→ set_config('app.workspace_id', ..., true)
→ set_config('app.device_id', ..., true)
→ set_config('app.membership_id', ..., true)
→ set_config('app.request_id', ..., true)
→ verify active membership / role
→ execute repository command
→ commit
```

- RLS 目标覆盖 `cloud_copyright_records`、`cloud_copyright_changes`、`cloud_copyright_events`、`cloud_copyright_audit_events`。
- 用户 scope 必须同时匹配 active membership、workspace 和必要 device；scope 缺失、失效、撤销或不匹配时零行可见、零业务写入。
- 审计读取仅 `owner/admin`，写入只允许 repository 或显式 system actor。
- system/service role 不得绕过审计：必须记录 `reason`、`request_digest`、receipt digest 和 request id。
- repository guard 继续作为 defense in depth；RLS 迁移和真实 PostgreSQL 负向 QA 通过前，不允许任何 direct SQL 或生产权限。

## 5. Internal API Admission

后续 API 仅可包含以下 operation：

| Operation | 最低角色 | 允许行为 |
| --- | --- | --- |
| `cloud_copyright_records_read` | `cloud_copyright_reader` | workspace scoped record projection read |
| `cloud_copyright_changes_execute` | `cloud_copyright_writer` | 调用 `CloudCopyrightRepository::execute_change` |
| `cloud_copyright_memberships_revoke` | `cloud_copyright_workspace_admin` | 调用受控 revoke command |

每次调用必须具备经过验真的 `actor_id`、`account_id`、`workspace_id`、`device_id`、`membership_id`、`role`、`request_id` 与 `receipt_digest`。API handler 不得接受这些字段作为可被客户端信任的 body 参数。

禁止项：

- public router、anonymous access、browser CORS；
- Desktop 或 Mobile direct call；
- public SDK export、production credential issuance；
- 以现有 `/v1/sync` 兼容性为由旁路 C2 admission；
- 任何返回媒体、路径、seed、token、private key 或内部 receipt 的响应。

## 6. 冻结证据与后续 Gate

合同文件：

- `docs/contracts/cloud-copyright/cloud-copyright-c2-contract-v1.schema.json`
- `docs/contracts/cloud-copyright/c2-transport-mapping-v1.fixture.json`
- `docs/contracts/cloud-copyright/c2-postgres-request-scope-v1.fixture.json`
- `docs/contracts/cloud-copyright/c2-internal-api-admission-v1.fixture.json`

验证命令：

```text
npm run cloud:copyright-c2-contract
```

进入 C3 前必须完成：

1. 单独评审 RLS additive migration 设计及 service-role/transaction scope adapter；
2. 实现 Rust/Dart adapter，但不改公开产品入口；
3. 真实 PostgreSQL 双连接 RLS scope-mismatch、revoked membership 与 service-role audit QA；
4. internal API receipt provider、router isolation 和 zero-public-route contract；
5. Desktop/Android 跨端 fixture；iOS runtime 仍按环境 Gate 单独挂起。

生产云版权库、团队协作、公开 SDK 和客户凭据发放继续关闭。
