# HiddenShield 云版权库阶段性收尾与外部 Gate 交接

更新时间：2026-07-29

状态：`phase_checkpoint_external_gates_suspended`

能力分类：`只能内部测试`

## 1. 阶段结论

云版权库任务族已完成当前环境下所有高价值、非外部依赖工作，并在 C3 运行时实现前阶段性挂起。

本 checkpoint 覆盖：

- C0：`cloud-copyright-record-v1`、workspace membership/RBAC、change/conflict/error 与隐私白名单合同；
- C1：`0023_cloud_copyright_multitenant_core`、`CloudCopyrightRepository`、migration smoke 与八场景真实 PostgreSQL 双连接 QA；
- C2：Desktop Rust / Mobile Rust-Dart transport mapping、request scope/RLS 目标与 internal-only API admission；
- C3：identity receipt canonical digest/fail-closed fixture、RLS static lint/mutation Gate、external readiness、secret/placeholder scan、旧 sync 隔离审计与 12 场景 scope QA artifact 合同。

生产云版权库、团队协作、端侧正式 transport、internal/public API、公开 SDK、RLS runtime 和 production role/credential 继续关闭。

## 2. 已完成 Gate

| Gate | 状态 | 证据 |
| --- | --- | --- |
| C0 contract | `passed` | 六类跨端/RBAC/冲突/隐私 fixture。 |
| C1 migration smoke | `passed` | `0023` PostgreSQL up/down、表/索引/trigger/约束回归。 |
| C1 concurrency QA | `passed` | 八场景、两个真实 `PgPool`、一次性 PostgreSQL。 |
| C2 contract | `passed` | transport、transaction-local scope、internal API 零公开暴露。 |
| C3 identity contract | `passed` | canonical digest、invalid/expired/scope mismatch/unavailable、生命周期/时钟/replay。 |
| C3 RLS static/mutation | `passed` | 拒绝缺 `FORCE RLS`、`BYPASSRLS`、`SET ROLE`、`PUBLIC` grant、global scope。 |
| C3 external readiness | `blocked_expected` | 缺少真实外部引用时显式 `blocked`，无 provider/database/API 副作用。 |
| C3 secret scan | `passed` | C3 artifact 未发现 PEM、JWT、密码 URL、字面 secret 或合同 placeholder。 |
| C3 scope QA artifact | `passed_contract_only` | 12 场景、PostgreSQL-only、双连接、receipt 脱敏、失败零写入格式。 |

## 3. 外部阻塞 Gate

以下资料必须由基础设施/安全团队真实提供，不能伪造或以 fixture 替代：

1. Internal IAM/JWKS endpoint 或 mTLS provider metadata；
2. workload identity Secret reference；
3. PostgreSQL migration owner/app/internal-service role bootstrap IaC；
4. app/internal-service role Secret reference；
5. 两人审批 reference 与 staging recovery drill evidence。

完整填写模板：

```text
docs/云版权库C3_外部配置与恢复演练交接模板.md
```

任一项缺失时必须继续挂起：

- `0024_cloud_copyright_rls_request_scope`；
- `CloudCopyrightIdentityReceiptAdapter`；
- 真实 RLS 与 PostgreSQL role；
- internal cloud copyright API；
- 12 场景真实 PostgreSQL scope QA；
- production role/credential 发放；
- Desktop/Android/iOS 正式云版权库 transport；
- 公开 SDK、团队协作和 SLA。

## 4. 恢复流程

外部引用到位后按顺序恢复：

1. 运行 `npm run cloud:copyright-c3-secret-scan`；
2. 注入引用并运行 `npm run cloud:copyright-c3-external-readiness`；
3. 仅当结果为 `ready_for_review` 时，评审 `0024` SQL 与 identity adapter fixture；
4. 评审通过后才可创建 additive migration 与 fail-closed adapter；
5. 在名称含 `hiddenshield_migrate_smoke` 的一次性 PostgreSQL 库执行 migration smoke 与 12 场景双连接 scope QA；
6. 完成 router isolation、恢复演练与双端 transport QA 后，重新评审生产 Gate。

不得跳过任何步骤或把 `fixture_contract_only` artifact 升格为真实运行证据。

## 5. Checkpoint 验证命令

```text
npm run cloud:copyright-contract
npm run cloud:copyright-c2-contract
npm run cloud:copyright-c3-contract
npm run cloud:copyright-c3-rls-lint-mutations
npm run cloud:copyright-c3-external-readiness-contract
npm run cloud:copyright-c3-secret-scan
npm run cloud:copyright-c3-scope-qa-contract
npm run cloud:postgres-migrate-smoke
npm run cloud:copyright-postgres-qa
cargo test --manifest-path feedback-backend/Cargo.toml --features postgres
cargo fmt --manifest-path feedback-backend/Cargo.toml -- --check
git diff --check
```

## 6. Checkpoint 说明

- checkpoint commit：本文件所在 commit。
- 当前分支：`main`；该本地分支在 checkpoint 前已包含 AI Transparency 内部 QA checkpoint。
- 本 checkpoint 不推送、不创建 PR、不改变 `origin/main`。
- 后续切换任务族时不得删除本交接清单；恢复云版权库时以本文件与 C3 external-readiness Gate 为入口。

## 7. Checkpoint 验证记录

- C0-C3 无容器合同、C3 external-readiness/secret scan 与 RLS mutation Gate 在 checkpoint 前通过。
- `0023` migration smoke 与八场景 PostgreSQL QA 已在本任务族内通过并形成既有 evidence。
- 本次 checkpoint 重跑 disposable PostgreSQL smoke 时，Podman 在 readiness 窗口两次返回 Windows `ConnectionReset (10054)`；该失败发生在 migration command 进入断言前，不改变既有通过证据，也不解除 production Gate。
- 恢复云版权库或修复本机 Podman 网络后，必须重新执行第 5 节的 PostgreSQL smoke/QA 命令并保存新 artifact。

下一任务：切换到用户指定的新任务族；云版权库只在真实外部引用到位后恢复。
