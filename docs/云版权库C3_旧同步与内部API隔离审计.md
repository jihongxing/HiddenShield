# 云版权库 C3 旧同步与 Internal API 隔离审计

更新时间：2026-07-29

状态：`static_audit_passed_no_runtime_change`

## 审计范围

| 路径 | 当前用途 | C3 结论 |
| --- | --- | --- |
| `POST /v1/sync/events:batch` | 既有 account/device 云同步事件 batch | 不是 cloud copyright internal command，不得承载 C3 receipt 或 RLS scope。 |
| `GET /v1/sync/changes` | 既有 account cursor change feed | 不是 workspace-scoped cloud copyright event feed，不得返回 `cloud_copyright_*` 表。 |
| `feedback-backend/src/cloud_copyright.rs` | C1 PostgreSQL repository | 无 Axum router/HTTP handler；只能由未来受控 internal facade 调用。 |
| Desktop `src-tauri/src/sync/cloud.rs` | 旧 `/v1/sync` client | 不得构造、缓存或重放 C3 raw identity receipt。 |
| Android `mobile_app/lib/sync/sync_transport.dart` | 旧 `/v1/sync` client | 不得直接调用 future internal API 或数据库。 |

## 旁路禁止规则

1. 不增加 `/v1/cloud-copyright`、`/internal/cloud-copyright` 或任何 public/internal router，直到 C3 router isolation review 完成。
2. 不向旧 sync request/response 增加 `receiptDigest`、raw identity receipt、RLS scope、membership role 或 `cloud_copyright_*` projection。
3. 未来 internal facade 不得复用 Bearer client token 作为 identity receipt；必须调用 verified internal identity receipt adapter。
4. 旧 `/v1/sync` 可以继续服务既有本地 vault 同步，但不能被标记为生产云版权库、团队协作或 C3 transport。
5. 每次修改 `feedback-backend/src/lib.rs` 的 route、Desktop/mobile sync transport 或 `cloud_copyright.rs` 前，必须重跑 `npm run cloud:copyright-c3-contract`。

## 自动审计

`verify-cloud-copyright-c3-contract.mjs` 断言：

- 当前 router 仅存在旧 `/v1/sync/events:batch` 与 `/v1/sync/changes`；
- 不存在 C3 cloud copyright public/internal route；
- `CloudCopyrightRepository` 不依赖 Axum router；
- `0024` migration 尚未创建。

这不是生产 router isolation QA；真实 facade、receipt provider 与 RLS runtime 出现后仍需执行 C3 真实 PostgreSQL Gate。
