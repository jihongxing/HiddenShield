# AI 生成内容标识 PostgreSQL QA 故障注入矩阵

状态：`internal_postgresql_failure_matrix_frozen`

能力分类：`只能内部测试`

本矩阵登记 AI Transparency PostgreSQL 控制面的真实事务 QA。`已覆盖`必须指向可执行 runner；`委托`必须说明下层事务 owner；`不适用`必须说明原因。禁止以未登记的隐含覆盖替代 Gate。

CI verifier 会自动扫描 `feedback-backend/src/bin/ai_transparency_*_qa.rs`；任何新 runner 未出现在本矩阵都会使 Gate 失败。

统一本地执行入口为 `npm run ai-transparency:postgres-qa`：它在显式提供
`HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL`（或 `DATABASE_URL`）时只使用该一次性测试库；否则只在
Podman/Docker 可用时创建并清理名称含 `hiddenshield_migrate_smoke` 的临时 PostgreSQL 容器。该入口不进入
默认 CI，避免 CI 对本地容器运行时产生隐式依赖。

显式 URL 同样 fail-closed：协议必须为 PostgreSQL，且数据库名必须包含
`hiddenshield_migrate_smoke`；不满足时在启动容器、运行迁移或写入任何记录之前拒绝。普通开发库、
共享库和生产库都不得作为该 suite 的目标。

## 覆盖准则

- 并发：至少两个真实 PostgreSQL 连接竞争同一逻辑资源。
- 回放：相同 idempotency/replay 输入不重复收费、状态变更或外部成本。
- audit 故障：由数据库 trigger 注入 INSERT 失败，并断言事务投影、receipt、计数不会部分泄漏；单次下载 claim 的不可复用语义除外。
- 外部/读取故障：对象、signer、provider 或读取失败必须按合同 fail-closed。
- 恢复：lease、dead-letter 或 crash 后只能按既定幂等路径恢复。

## 控制面矩阵

| 控制面 | Runner | 并发 | 回放 | audit 故障 | 外部/读取故障 | 恢复 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Profile change request / approval / execution | `ai_transparency_approval_concurrency_qa` | 已覆盖：版本竞争 | 已覆盖：idempotency | 已覆盖：零业务写入 | 不适用：无外部调用 | 不适用：同步命令 | 已覆盖 |
| Marking confirm / ledger | `ai_transparency_confirm_concurrency_qa` | 已覆盖：一胜一败 | 已覆盖：重复 confirm | 已覆盖：audit/ledger 全回滚 | 不适用：无外部调用 | 不适用：同步命令 | 已覆盖 |
| Credential custody / session | `ai_transparency_credential_custody_qa` | 已覆盖：rotate/revoke | 已覆盖：credential lifecycle | 已覆盖：custody audit | 已覆盖：IAM/KMS unavailable | 不适用：无异步 lease | 已覆盖 |
| Image marking executor | `ai_transparency_image_marking_executor_qa` | 委托：session/confirm Gate | 委托：confirm Gate | 不适用：executor 不写独立审计投影 | 已覆盖：invalid session、写后回读 | 不适用：同步 `watermark-core` 调用 | 已覆盖/委托 |
| Post-embed signing / recovery / dead-letter | `ai_transparency_post_embed_signing_qa` | 已覆盖：reservation、worker claim、requeue 冲突 | 已覆盖：signing/recovery/requeue | 已覆盖：requeue audit 全回滚 | 已覆盖：signer、receipt、readback、artifact | 已覆盖：四 crash 点、lease、dead-letter | 已覆盖 |
| Delivery authorization / retrieval / revoke | `ai_transparency_post_embed_signing_qa` | 已覆盖：单消费、revoke-vs-claim | 已覆盖：revoke/retrieval | 已覆盖：grant/revoke/retrieval 成功 audit | 已覆盖：timeout、MIME、size、artifact | 不适用：单次消费后不恢复为 active | 已覆盖 |
| Delivery security notification outbox | `ai_transparency_post_embed_signing_qa` | 已覆盖：claim/reclaim | 已覆盖：completion/recovery replay | 已覆盖：completion/recovery/replay audit | 已覆盖：receipt、lease、attempt budget | 已覆盖：expired lease、dead-letter recovery | 已覆盖 |
| External evidence intake/review | `ai_transparency_external_evidence_review_qa` | 已覆盖：同一 intake 一胜 | 已覆盖：唯一 decision 拒绝重复 | 已覆盖：decision/audit 全回滚 | 已覆盖：IAM/reference 拒绝 | 不适用：同步内部审核 | 已覆盖 |
| Platform API facade / public resolver | `ai_transparency_platform_api_qa` | 委托：session/confirm/Resolver DB projection | 委托：confirm replay | 委托：下层事务 owner | 不适用：不直接调用 provider | 不适用：无本地恢复 worker | 已覆盖/委托 |

## 明确边界

- 本矩阵不替代真实 IAM、KMS/HSM、object-store、signer 或通知 provider 的外部恢复演练；这些仍是外部配置 Gate。
- 真实设计伙伴/provider evidence、真实平台处理链和 iOS runtime 仍为外部环境 Gate。
- `委托`不代表无需测试：任何将来新增本地事务写入、外部成本或 lease 的 facade/executor 必须升级为独立行并补齐适用的故障注入。

## 必跑验证

- `npm run ai-transparency:postgres-qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_approval_concurrency_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_confirm_concurrency_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_credential_custody_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_image_marking_executor_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_external_evidence_review_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_platform_api_qa`
- `cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin ai_transparency_post_embed_signing_qa`

下一 Gate：仅在真实 provider 或设计伙伴配置到位后，把相应外部恢复演练附加到本矩阵；不得把内部 QA
或统一 suite runner 提升为生产 SLA 或法律合规结论。
