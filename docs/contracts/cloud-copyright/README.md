# Cloud Copyright Contract V1

状态：`c0_contract_frozen`

该目录冻结云版权库 C0 合同，不包含数据库迁移或生产实现。

## 文件

- `cloud-copyright-contract-v1.schema.json`：六类 fixture 的统一 envelope Schema。
- `copyright-record-v1.fixture.json`：`cloud-copyright-record-v1` 与 desktop↔mobile 双向读取摘要。
- `workspace-membership-rbac-v1.fixture.json`：workspace membership 与 owner/admin/editor/viewer 权限矩阵。
- `change-batch-v1.fixture.json`：幂等 change、accepted/duplicate disposition 与 audit 语义。
- `conflict-version-changed-v1.fixture.json`：版本冲突零远端写入、保留本地草稿。
- `membership-revoked-v1.fixture.json`：成员撤销后 fail-closed 与重新授权。
- `forbidden-sync-data-rejection-v1.fixture.json`：路径、媒体、seed、token 和私钥零同步。

## 验证

```text
npm run cloud:copyright-contract
```

该验证已加入 `npm run cloud:ci`。在 C0 合同测试通过前禁止创建云版权库生产迁移；进入 C1 后，迁移和 PostgreSQL QA 必须继续复用本合同，不得静默改变字段、RBAC、冲突或隐私边界。
