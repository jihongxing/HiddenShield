# AI 生成内容标识 Delivery Authorization Revoke 与 Resource Budget 合同

状态：`frozen_internal_only_v1`
冻结日期：2026-07-28
前置合同：`AI生成内容标识Delivery_Authorization_Retrieval合同.md`

## 1. 固定资源预算

所有预算由服务端 Profile 固定，调用方不得提交、更改或放大：

- 最大下载 bytes：`67,108,864`（64 MiB）
- 允许 content type：仅 `image/png`
- object-store 读取超时：`5,000 ms`
- 每 License claim 限速：每自然分钟最多 `30` 次

预算在创建 authorization 时固化到数据库和 grant。数据库 CHECK 约束拒绝非冻结值，避免应用层配置漂移。

## 2. Artifact Retriever 合同

内部接口：`DeliveryArtifactRetriever::load_finalized_for_delivery`

adapter 必须接收完整 `DeliveryDownloadBudget`，并在对象存储层：

1. 读取前传递 5 秒 provider timeout。
2. 使用 object metadata 验证 content length，超过 64 MiB 时停止流式读取。
3. 返回可信 object metadata content type。
4. 区分 `Unavailable` 与 `TimedOut`。

命令层必须再次验证：

- metadata content length 不超过预算；
- 实际 bytes 长度不超过预算；
- metadata 长度与实际 bytes 长度一致；
- content type 严格等于 `image/png`。

adapter 或命令层任一检查失败均不得返回 package 或 bytes。

## 3. Rate Limit

表：`ai_delivery_download_rate_limit_windows`

- key：`licenseId + minute window`
- claim 计数在授权行仍持有 `FOR UPDATE` 锁时原子递增。
- 达到 30 次后，新请求返回 `ai_delivery_retrieval_rate_limited`。
- rate limited 不消费 authorization、不读取 artifact，authorization 保持 `active`，可在下一窗口重试。
- 已进入 claim 的 artifact unavailable、MIME、size、timeout 或 bridge 失败仍消费 authorization。

## 4. Authorization Revoke

内部命令：`execute_postgres_revoke_delivery_authorization`

输入：

- `authorizationId`
- tenant/workspace/environment
- `revokerSnapshotId`
- Internal IAM token hash
- 非空 revoke reason，最大 512 字符

权限：

- Internal IAM 必须验证 `ai_transparency_security_approver`。
- snapshot 必须属于同 tenant/workspace/environment、未过期且角色相同。

状态机：

- `active -> revoked`：成功，写入 revoker snapshot、时间和原因。
- `revoked -> revoked`：幂等 replay，不重复写审计。
- `consumed | expired -> revoked`：拒绝。
- revoke 与 retrieve 竞争同一 PostgreSQL 行锁，最多一方成功；最终状态只能是 `revoked` 或 `consumed`。

撤销后的检索返回稳定原因码 `ai_delivery_authorization_revoked`，且在 artifact load 前终止。

## 5. 审计

新增 append-only event：`authorization_revoked`。

审计允许记录：

- authorization、delivery envelope、execution 标识；
- revoker snapshot ID；
- revoke reason SHA-256。

审计禁止记录：

- revoke reason 原文；
- retrieval token；
- bytes；
- Secret 引用；
- provider credential 或完整 provider receipt。

## 6. 失败码

- `ai_delivery_authorization_revoked`
- `ai_delivery_retrieval_rate_limited`
- `ai_delivery_retrieval_size_limit_exceeded`
- `ai_delivery_retrieval_content_type_invalid`
- `ai_delivery_retrieval_read_timeout`

上述失败全部返回零 package。

## 7. 数据库

迁移：`0014_ai_transparency_delivery_revoke_resource_budget`

变更：

- authorization 增加固定预算与 revocation 字段；
- 下载审计允许 `authorization_revoked`；
- 新增 License/minute rate-limit projection；
- 新增 rate-limit 清理索引。

## 8. Gate 证据

- 合同 fixture：`docs/contracts/ai-transparency-delivery-retrieval/resource-budget-v1.fixture.json`
- PostgreSQL 16 smoke：39 tables、50 indexes、0001–0014 up/down 和空 schema rollback。
- PostgreSQL QA：
  - revoke/replay；
  - revoked 后零 artifact load；
  - revoke/retrieve 双连接竞争最多一方成功；
  - 64 MiB metadata 超限；
  - 非 PNG MIME；
  - provider timeout；
  - License/minute 限速；
  - 所有失败零 package。

## 9. 外部边界

当前仍为 `只能内部测试`。SDK、公共 Resolver、客户下载/import UI、生产 credential、生产 object-store/IAM/KMS/HSM/signer 与生产发放继续关闭。

## 10. 下一 Gate

实现 internal rate-limit window cleanup 与 delivery security observability summary，冻结指标保留期、告警阈值和审计导出边界。
