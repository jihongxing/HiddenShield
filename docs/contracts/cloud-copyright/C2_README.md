# Cloud Copyright C2 Contract V1

状态：`c2_contract_frozen_no_runtime_or_migration`

本目录冻结 C2 的端侧 transport mapping、PostgreSQL request scope/RLS 和受控 internal API admission。它不创建 `0024` migration、不注册 HTTP route、不修改 Desktop/Android/iOS bridge，也不开放公开 SDK。

## 文件

- `cloud-copyright-c2-contract-v1.schema.json`：三类 C2 fixture 的 envelope。
- `c2-transport-mapping-v1.fixture.json`：Desktop Rust 与 Mobile Rust/Dart 必须使用同一 outbox/request/result 语义。
- `c2-postgres-request-scope-v1.fixture.json`：transaction-local request scope、RLS 和零写入 failure case。
- `c2-internal-api-admission-v1.fixture.json`：internal-only operation、verified claim 和禁止暴露面。

## 验证

```text
npm run cloud:copyright-c2-contract
```

验证不替代未来的 RLS migration、真实 PostgreSQL scope QA、Rust/Dart bridge implementation 或 internal API runtime QA。
