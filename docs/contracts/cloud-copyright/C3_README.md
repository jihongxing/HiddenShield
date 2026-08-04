# Cloud Copyright C3 Contract V1

状态：`c3_contract_frozen_no_migration_or_runtime`

该合同冻结 verified internal identity receipt、canonical digest、fail-closed identity/transport 行为和 RLS SQL 静态安全目标。`c3-rls-policy-v1.sql` 是评审模板，不能作为生产 migration 执行。

## 验证

```text
npm run cloud:copyright-c3-contract
```

该验证拒绝已创建的 `0024` migration，并检查：

- receipt digest 仅绑定 canonical claims，不包含 signature；
- invalid、expired、scope mismatch、digest mismatch 与 provider unavailable 均为零业务写入；
- Desktop 与 Android 保留同一份本地草稿并停止自动重试；
- RLS 模板强制 `ENABLE/FORCE ROW LEVEL SECURITY`、`NOBYPASSRLS` 与 transaction-local scope，拒绝 public grant、`SET ROLE` 和全局 scope。

`npm run cloud:copyright-c3-rls-lint-mutations` 必须证明 lint 会拒绝缺失 `FORCE RLS`、`BYPASSRLS`、`SET ROLE`、`PUBLIC` grant 和全局 scope 五类危险 SQL mutation。

`npm run cloud:copyright-c3-scope-qa-contract` 冻结未来真实 PostgreSQL scope QA 的 artifact 格式：必须使用两个连接、名称含 `hiddenshield_migrate_smoke` 的一次性库，排除 SQLite/mock/raw identity receipt，并保留 12 个场景的零写入证据。
