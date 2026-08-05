# 云版权库 PostgreSQL 迁移设计

## 2026-07-31 P5 Podman staging-equivalent 技术演练

- 新增 `cloud:postgres-p5-podman-rehearsal`，对已迁移的云版权库核心 auth / sync / registry 执行正式 HTTP 并发负载、数据库观测和 PITR 恢复。
- 本次负载为 8 账号、16 设备、160 push、160 pull、并发 8，零失败；push / pull p95 分别为 `59.34 ms` / `64.34 ms`。
- Observability 覆盖 `pg_stat_database`、`pg_stat_statements`、`pg_stat_user_tables`、锁等待和连接池峰值；PITR 使用 base backup、WAL archive 与 `recovery_target_time`，恢复约 `2.51 s`。
- 本地 Podman artifact 可作为核心主链 staging-equivalent 技术证据，但不能自动批准 cutover；P5 强制 Gate 仍要求 release owner 对 runbook 显式 review 并生成具名人工 signoff。
- 当前剩余阻塞：release owner 审批与签字。Enterprise、支付、团队和云视频等未进入正式 PostgreSQL Router 的模块继续维持各自迁移与发布 Gate，不得由本次核心演练代替。

更新时间：2026-07-03

本文用于冻结 HiddenShield 云版权库从当前 SQLite 后端迁移到 PostgreSQL 的上线前决策。当前任务只做设计，不改运行时代码；代码迁移必须在本文评审通过后再进入实施。

## 1. 背景与决策

当前 `feedback-backend` 仍以 SQLite 单文件作为本地云后端存储：

- `package.json` 的 `cloud:backend` 使用 `--db-path feedback-backend/cloud.sqlite`。
- `feedback-backend/Cargo.toml` 依赖 `rusqlite`。
- `feedback-backend/src/lib.rs` 暴露 `--db-path` / `HIDDENSHIELD_FEEDBACK_DB_PATH` 风格的文件数据库配置。
- `feedback-backend/src/storage.rs` 的 schema、事务和查询均基于 `rusqlite::Connection`。

这个形态适合本地开发、合同测试、RC1 无外部依赖验收和单机演示，但不适合作为云版权库生产数据库。原因不是“SQLite 不好”，而是 HiddenShield 的生产云版权库已经包含同步事件、版权编号 registry、公开权利 manifest、Enterprise API key、quota ledger、API audit、支付授权、云视频任务和团队空间等多类高并发写入 / 审计数据；未来 Enterprise API 还会开放给外部客户调用，必须有连接池、行级锁、在线迁移、备份恢复、审计留存和压测容量边界。

决策：

- SQLite 继续保留在本地 / 开发 / 测试范围。
- PostgreSQL 是生产云版权库、Enterprise API、团队共享库、云端视频任务和支付 / quota / audit 的唯一生产数据库目标。
- 在 PostgreSQL 迁移合同与压测门槛通过前，不把云版权库生产 SLA、Enterprise API 大客户接入或 Creator 自动云同步稳定性写成已上线完成。

## 2. SQLite 保留范围

SQLite 保留用于：

| 范围 | 是否保留 SQLite | 说明 |
| --- | --- | --- |
| 桌面端本地版权库 | 是 | 本机 vault、队列、离线写入和本地报告缓存继续使用本地 SQLite。 |
| 移动端本地版权库 | 是 | 原生端本地库继续使用平台本地 SQLite / drift / Rust bridge 等既有方案。 |
| `feedback-backend` 本地开发 | 是 | `npm run cloud:backend` 可继续用 `feedback-backend/cloud.sqlite` 做无外部依赖开发。 |
| 自动化合同 smoke | 是 | 不依赖 Docker / 外部服务的合同测试可继续使用 SQLite fixture。 |
| RC1 无外部依赖验收 | 是 | 当前 RC1 可以验证协议、权益门禁和本地运行态，不代表生产数据库形态。 |
| 单机 demo / 离线演示 | 是 | 明确标注为 non-production，不接真实 Enterprise API 或真实支付回调。 |

SQLite 不再用于：

- 生产云版权库。
- Enterprise 公开扫描 API 生产路由。
- 企业 quota 余额、quota ledger、API audit 生产账本。
- 真实支付订阅 / 一次性报告购买生产 webhook。
- Studio / Enterprise 团队共享版权库生产数据。
- L3 云端视频任务生产队列和 worker attempt 状态。
- 任何承诺 SLA、外部客户接入、横向扩容或多实例部署的后端环境。

## 3. PostgreSQL 生产边界

PostgreSQL 是以下能力的生产边界：

- 云版权库 metadata 同步：`cloud_sync_events`、`cloud_device_cursors` 以及未来版权库投影表。
- 版权编号登记和重新签发：`watermark_id_registry`、`watermark_id_reissue_jobs`。
- 公开权利信号与训练许可 registry：`rights_manifests`。
- Enterprise 公开扫描 API：`enterprise_api_keys`、`enterprise_quota_balances`、`enterprise_quota_ledger`、`enterprise_api_audit_events`、`enterprise_rate_limit_windows`、`enterprise_admin_audit_events`。
- 订阅、支付和单份报告授权：`billing_payment_sessions`、`report_purchase_sessions`、`report_purchase_grants`、`billing_customers`、`subscriptions`、`subscription_events`、`entitlements`。
- 云端视频任务和用量账本：`cloud_video_tasks`、`cloud_usage_ledger`。
- 团队空间和共享版权库：`team_workspaces`、`team_members`、`team_shared_library_records`、`team_audit_logs`。
- 通用管理审计和反馈数据：`admin_audit_events`、`feedback_events`、`feedback_batches`。

生产后端必须支持：

- 多实例 API 服务共享同一数据库。
- 连接池和连接数上限。
- 行级锁和事务隔离。
- 在线 schema migration。
- PITR / 定期备份 / 恢复演练。
- 慢查询日志和关键表指标。
- 只读副本或后续分析库导出，不直接拖慢主库。

## 4. 迁移表清单

### 4.1 必须首批迁移

这些表直接决定云版权库、权益和企业 API 生产正确性：

| 模块 | 表 |
| --- | --- |
| 账户 / 会话 / 设备 | `cloud_accounts`、`cloud_devices`、`cloud_sessions`、`auth_challenges`、`auth_attempts` |
| 云同步 | `cloud_sync_events`、`cloud_device_cursors` |
| 版权编号 registry | `watermark_id_registry`、`watermark_id_reissue_jobs` |
| 公开权利 manifest | `rights_manifests` |
| Enterprise API | `enterprise_api_keys`、`enterprise_quota_balances`、`enterprise_quota_ledger`、`enterprise_api_audit_events`、`enterprise_rate_limit_windows`、`enterprise_admin_audit_events` |
| 支付 / 权益 | `billing_payment_sessions`、`report_purchase_sessions`、`report_purchase_grants`、`billing_customers`、`subscriptions`、`subscription_events`、`entitlements` |
| 管理审计 | `admin_audit_events` |

### 4.2 第二批迁移

这些表与高阶商业能力相关，可在首批库兼容层稳定后迁移：

| 模块 | 表 |
| --- | --- |
| 云视频 / 用量 | `cloud_video_tasks`、`cloud_usage_ledger` |
| L2 notary | `video_fingerprint_notaries` |
| 团队空间 | `team_workspaces`、`team_members`、`team_shared_library_records`、`team_audit_logs` |
| 反馈统计 | `feedback_events`、`feedback_batches` |

### 4.3 需要新增或调整的生产表

当前 SQLite schema 以同步事件为主，生产云版权库建议补充投影表，避免每次从事件流重放：

- `cloud_vault_records`：按 `account_id + workspace_id + watermark_uid + revision` 保存云版权库当前投影。
- `cloud_vault_record_revisions`：保存版权库 metadata 历史版本和 payload hash。
- `cloud_sync_event_ingestion_results`：可选，用于记录 batch per-event accepted / duplicate / conflict / rejected。
- `schema_migrations`：统一数据库迁移版本表。

这些新增表不在本设计阶段直接实现，但必须进入 PostgreSQL 迁移评审。

## 5. SQLite 风险清单

若继续用 SQLite 承载生产云版权库，主要风险如下：

- 单写者锁：Enterprise API 每次请求可能同时写 quota ledger、更新 quota balance、写 API audit、更新 API key last used；并发下容易写锁等待或超时。
- 多实例部署困难：云后端横向扩容时，单文件 SQLite 不适合作为共享写入数据库。
- quota 扣减竞态：生产扣费需要行级锁或等价串行化，SQLite 难以表达 `SELECT ... FOR UPDATE` 这类明确锁语义。
- 审计写入放大：外部 API 成功、失败、限流、quota 不足和鉴权失败都要写审计，写流量会与同步事件、支付 webhook、视频任务队列竞争同一个 writer。
- 在线迁移风险：生产 schema 变更、索引创建、大表回填和 JSON 字段演进需要可控迁移，不适合继续散落在 `init_schema` 中。
- 备份恢复边界弱：生产需要 PITR、恢复演练、加密备份和保留策略；单文件备份无法满足企业级审计预期。
- 查询能力不足：公开权利 manifest、审计查询、队列监控和客户对账需要 JSONB、部分索引、组合索引和查询计划可观测性。
- 运维隔离不足：生产只读分析、报表、客户审计导出不应直接压在写入主链路上。

## 6. PostgreSQL 目标能力

PostgreSQL 迁移后的目标不是简单替换驱动，而是建立生产数据边界：

- 事务隔离：quota 扣减、同步事件接收、registry confirm、支付授权和视频任务 completion 必须在明确事务中完成。
- 行级锁：quota balance 扣减、task claim、session refresh token rotation 必须有行级锁或等价乐观锁。
- 幂等 upsert：保留 `ON CONFLICT` 语义，所有外部重试路径必须可重复提交。
- JSONB：`*_json` 字段生产存储使用 JSONB，并为高频查询字段建立表达式索引或生成列。
- 时间类型：生产使用 `timestamptz`，端侧仍可传 RFC3339 字符串。
- bigint sequence：`cloud_sync_events.sequence` 使用 `bigserial` 或显式 sequence，避免长期同步事件溢出。
- partial index：`rights_manifests` 的 active 唯一约束继续使用部分唯一索引。
- pool + timeout：后端接 `sqlx` 或等价异步 Postgres pool，固定连接池大小、statement timeout、idle timeout。
- migration tool：引入 `sqlx migrate`、`refinery` 或等价工具，禁止生产依赖运行时 `CREATE TABLE IF NOT EXISTS` 漂移 schema。

## 7. SQL 差异注意点

迁移时必须逐项处理 SQLite 与 PostgreSQL 差异：

| 主题 | 当前 SQLite 形态 | PostgreSQL 迁移要求 |
| --- | --- | --- |
| boolean | `INTEGER 0/1` | 改为 `boolean`，兼容旧导入值。 |
| datetime | `TEXT` RFC3339 | 改为 `timestamptz`，输出仍保持 RFC3339。 |
| json | `TEXT` | 改为 `jsonb`，必要时加 JSON schema 合同。 |
| autoincrement | `INTEGER PRIMARY KEY AUTOINCREMENT` | 改为 `bigserial` 或 identity。 |
| upsert | `INSERT OR IGNORE` / 手写查询 | 改为 `INSERT ... ON CONFLICT ... DO UPDATE/NOTHING`，并明确 conflict target。 |
| partial unique index | SQLite 支持 | PostgreSQL 继续保留，迁移脚本必须显式创建。 |
| check constraint | 当前已使用部分 CHECK | PostgreSQL 保留并补齐 enum-like CHECK。 |
| transaction | 单连接 mutex + transaction | pool connection + explicit transaction，禁止跨 await 持锁。 |
| lock | 隐式写锁 | `SELECT ... FOR UPDATE` 或乐观版本字段。 |

## 8. 增量同步与幂等要求

PostgreSQL 迁移不能改变端侧同步语义：

- 已 `synced` 的本地记录不能重复上传。
- 新增 / 修改 metadata 必须产生新的 `eventRevision`、`payloadHash` 或新版 `clientEventId`。
- `cloud_sync_events` 必须保留 `account_id + device_id + client_event_id` 幂等约束。
- 后端 batch 必须能区分 accepted / duplicate / conflict / rejected。
- pull cursor 必须只在本地合并成功后推进。
- PostgreSQL 迁移后必须保留“不同设备同一账号最终一致”的双端验收。

建议生产投影写入流程：

1. 接收 batch，按 account / device / client event 去重。
2. 对每条 event 计算或校验 `payload_hash`。
3. 写 `cloud_sync_events` 原始事件。
4. 在同一事务内更新 `cloud_vault_records` 当前投影。
5. 返回 per-event disposition。
6. 只有 accepted / duplicate-safe 的事件允许端侧标记 synced。

## 9. Enterprise API 并发要求

Enterprise API 不是只读数据库负载。即使接口业务语义是只读公开扫描，每个请求也会产生写入：

- API key 鉴权后更新 `last_used_at`。
- DB rate-limit 更新 `enterprise_rate_limit_windows`。
- 成功请求写 `enterprise_quota_ledger` committed debit。
- 同一事务或受控事务内更新 `enterprise_quota_balances.used_units`。
- 成功 / 失败 / 限流 / quota 不足 / 鉴权失败写 `enterprise_api_audit_events`。

PostgreSQL 生产实现必须：

- 对 `enterprise_quota_balances` 使用行级锁或乐观版本，避免并发超扣。
- 对 `enterprise_quota_ledger` 使用 `UNIQUE(account_id, workspace_id, quota_type, idempotency_key)` 保证重试不重复扣费。
- 对 rate-limit window 使用 upsert + 原子递增。
- 对 API audit 使用 append-only 模式，不允许失败请求跳过审计。
- 对 quota 不足请求不扣费，但仍写审计。
- 对 5xx 未完成扣费的请求不得留下 committed debit。

## 10. 压测门槛

PostgreSQL 迁移进入生产前，至少通过以下压测门槛。数值先作为首版 release blocker，可在真实客户规模明确后上调，不允许下调到低于当前目标。

| 场景 | 最低门槛 | 阻断条件 |
| --- | ---: | --- |
| Creator 云同步 push | 50 并发设备、每设备 20 events batch，连续 10 分钟 | p95 > 800ms、错误率 > 0.5%、重复 accepted、synced 事件重传 |
| 云同步 pull | 100 并发设备按 cursor 拉取，连续 10 分钟 | p95 > 500ms、cursor 倒退、漏事件 |
| Enterprise batch scan | 100 RPS、每请求 50 watermarkUid，连续 10 分钟 | p95 > 1000ms、quota 超扣 / 少扣、audit 漏写 |
| quota 幂等重试 | 同一 idempotency key 并发重放 100 次 | 出现重复扣费或 ledger 冲突未被稳定处理 |
| 支付 webhook | 20 RPS provider event，含重复 webhook | 重复授权、漏授权、退款撤销不幂等 |
| L3 task claim | 20 worker 并发 claim 200 个 queued task | 同一 task 被多个有效 attempt 同时持有 |
| 管理审计查询 | 100 万 audit rows 下按 account / api key / time 查询 | p95 > 1500ms 或顺序扫主表 |
| 恢复演练 | 从最近备份恢复到 staging 并跑核心合同 | 无法在 30 分钟内恢复或恢复后合同失败 |

压测输出必须包含：

- 数据库版本、配置、连接池大小。
- 表规模、索引列表、关键查询 `EXPLAIN ANALYZE`。
- p50 / p95 / p99 latency、错误率、锁等待、deadlock 数量。
- quota ledger / audit / sync event 的一致性校验。
- 回滚或降级建议。

## 11. 实施阶段

### Phase P0：设计冻结

- 完成本文评审。
- 明确 SQLite 保留范围和 PostgreSQL 生产边界。
- 决定 Postgres runtime crate：优先评估 `sqlx`，并确认是否保留同步 repository trait。
- 新增迁移合同任务清单，不改正式代码。

验收：本文通过评审，Roadmap 和能力边界已回写。

Phase P0 评审结论：2026-07-03 评审通过，可以进入 Phase P1。

P0 评审通过。

评审确认：

- 迁移表清单覆盖当前 `feedback-backend/src/storage.rs` 中的生产相关表：账户 / 会话 / 设备、云同步、版权编号 registry、公开权利 manifest、Enterprise API、quota、audit、支付 / 权益、云视频、团队空间、反馈统计和管理审计。
- 首批迁移表包含云版权库和 Enterprise API 生产正确性所需的最小闭环；第二批迁移表覆盖 L2 notary、L3 云视频、团队共享库和反馈统计，不阻断 P1 抽象层。
- 压测门槛覆盖 Creator 云同步 push / pull、Enterprise batch scan、quota 幂等重试、支付 webhook、L3 task claim、管理审计查询和恢复演练；没有发现必须在 P1 前补充的新压测场景。
- P1 不直接重写业务 SQL；先建立数据库后端配置、SQLite adapter 保留、PostgreSQL adapter skeleton 和 `cloud:db-portability-contract`，用合同保证 SQLite 现有路径不退。

P0 决定：

- Postgres runtime crate 暂定优先评估 `sqlx`；本次 P1 只建立 skeleton，不引入真实 Postgres 依赖。
- 保留同步 `Storage` 外观，避免一次性迁移所有 handler 签名；后续 P2/P3 再按 auth / sync / registry / Enterprise / payment 切 repository。
- 生产环境禁止 SQLite 的规则从 P1 开始进入后端配置层，真实生产切换仍等 P5。

P1.2 评审结论：2026-07-03 进入 feature-gated Postgres 依赖和 repository trait 抽象。

- `sqlx` 以 `postgres` feature 可选引入，默认构建仍不拉起 Postgres runtime。
- 首批 repository trait 固定为 auth、cloud sync 和 watermark registry 三组，先由现有 SQLite `Storage` 实现，handler 仍可继续调用当前 `Storage` 外观。
- `cloud:db-portability-contract` 从结构合同升级为双路径合同：检查 SQLite adapter 保持可用、Postgres adapter 仍为 skeleton、Postgres schema smoke 覆盖 P1 表，并强制 `cargo check --features postgres`。
- Postgres schema smoke 当前只验证 schema 片段和 `sqlx` 类型编译，不连接真实 PostgreSQL；真实迁移、migrate up/down 和数据库运行态留到 P2。

P2 评审结论：2026-07-03 建立真实 Postgres migration 文件与非连接型 migration contract。

- 首批 migration 目录固定为 `feedback-backend/migrations/postgres/`。
- `0001_auth_sync_registry.up.sql` 覆盖 auth、cloud sync、watermark registry 和 `rights_manifests` 的 P2 表、索引、partial unique index、JSONB、TIMESTAMPTZ、BIGSERIAL 和 BOOLEAN 语义。
- `0001_auth_sync_registry.down.sql` 明确按索引优先、表逆序、`schema_migrations` 最后删除的回滚边界。
- `database.rs` 的 Postgres schema smoke 改为 `include_str!` 引用 migration 文件，不再保留内嵌 `CREATE TABLE` 字符串。
- 新增 `cloud:postgres-migration-contract`，只做本地文件 / schema / up-down / Rust feature smoke 检查，不连接生产库、不执行真实 migrate。

P2.1 评审结论：2026-07-03 准备 disposable Postgres migrate smoke。

- 新增 `cloud:postgres-migrate-smoke`，优先使用 `HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL` / `DATABASE_URL` 指向的临时库；未提供时尝试用 Podman 或 Docker 启动一次性 `postgres:16-alpine` 容器。
- `postgres_migrate_smoke` Rust bin 会真实执行 `0001_auth_sync_registry.up.sql` / `.down.sql`，校验表、索引、`idx_rights_manifests_one_active` partial unique index、关键列类型和回滚后空 schema。
- 为避免误连生产，smoke URL 必须包含 `localhost` 或 `127.0.0.1`，且数据库名 / URL 必须包含 `hiddenshield_migrate_smoke`。
- 本机无需 psql；有 Podman 或 Docker 时，`cloud:postgres-migrate-smoke` 会自动准备 disposable Postgres。两者都没有时，脚本会明确失败并提示提供 disposable database URL；默认 `commercial:ci` 仍只跑非连接型 `cloud:postgres-migration-contract`。
- 2026-07-03 本机 Podman 5.7.1 实跑通过：`upTablesChecked=11`、`indexesChecked=11`、`rollback=empty_schema_verified`。

P2.2 评审结论：2026-07-03 为 disposable migrate smoke 增加 RC / 上线审计 artifact。

- `cloud:postgres-migrate-smoke` 每次运行都会输出 `tmp-ui-qa/postgres-migration/postgres-migrate-smoke-<timestamp>.json`。
- Artifact schema 固定为 `postgres_migration_smoke_artifact_v1`，记录 runtime kind / version、镜像、容器名、端口、数据库名、表校验数、索引校验数、rollback 结果、安全约束和容器清理结果。
- 该 artifact 只记录 disposable 环境元数据和 schema 校验摘要，不记录数据库密码或完整连接 URL。
- 2026-07-03 最新 RC1 证据索引：`tmp-ui-qa/postgres-migration/postgres-migrate-smoke-1783021160601.json`，Podman 5.7.1，`postgres:16-alpine`，`upTablesChecked=11`、`indexesChecked=11`、`rollback=empty_schema_verified`、cleanup `removed`。该证据不表示生产云版权库已切换数据库。

P3 前置设计结论：2026-07-03 P3 先做 repository 的 Postgres 读写实现顺序评审，不直接切默认运行路径。

- 第一组先实现 `AuthRepository` 的 Postgres 读写。原因：auth 是云同步、registry、Enterprise API 和 billing webhook 的身份地基，业务范围最窄，先验证账号 / 设备 / session / challenge 的 SQL 差异、事务边界和错误码映射。最小运行态 QA：`auth:postgres-runtime-qa`，覆盖账号 fixture、登录 challenge、session 创建、refresh rotation、logout、device revoke、过期 session 拒绝、SQLite/Postgres 响应字段一致；失败不得影响现有 SQLite dev/test adapter。
- 第二组实现 `CloudSyncRepository` 的 Postgres 读写。原因：sync 依赖 auth 身份，并且是当前 Creator 云版权库可靠性 blocker 的主路径；先把增量同步、幂等、去重和限流压实，再让 registry 写入进入云库。最小运行态 QA：`cloud:sync-postgres-runtime-qa`，覆盖 desktop / mobile 双设备 push-pull-pull、已同步记录下次不重复上传、重复 `client_event_id` 幂等、stale `syncing` 恢复、断线续传、Free 403、Creator allowed、队列限流与 cursor 递增；payload 和版权编号格式不变。
- 第三组实现 `WatermarkRegistryRepository` 的 Postgres 读写。原因：registry 涉及版权编号签发、离线记录 reconcile、公开权利和未来企业 API 查询，是产品风险最高的一层，必须在 auth 与 sync 稳定后进入。最小运行态 QA：`watermark:registry-postgres-runtime-qa`，覆盖 reserve、同一 request id 幂等 reserve、confirm、离线 UID reconcile、长格式 `HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX` 保持、冲突返回稳定错误、reissue skeleton 不自动改 payload；rights manifest active partial unique index 继续由 P2 migration smoke 覆盖，未在 P3.3 默认写路径启用。
- P3 聚合门禁建议为 `cloud:postgres-runtime-qa`：依次跑 `auth:postgres-runtime-qa`、`cloud:sync-postgres-runtime-qa`、`watermark:registry-postgres-runtime-qa`，只连接 disposable / staging PostgreSQL，不连接生产库；通过前不得把 Postgres adapter 接入正式 UI / mock / release 默认路径。
- P3 不改变桌面 / 移动同步 payload，不改变版权编号，不改变 watermark payload，不降低正式阈值，不用 Postgres 新实现绕过现有权益、报告、公开权利或 Enterprise 合同。

P3.1 评审结论：2026-07-03 完成 `AuthRepository` Postgres adapter 与 `auth:postgres-runtime-qa`。

- 新增 feature-gated `PostgresAuthRepository`，只实现 `AuthRepository`：`create_auth_challenge`、`create_auth_session`、`refresh_auth_session`、`logout_auth_session`、`list_devices`、`revoke_device`。没有实现 `CloudSyncRepository` 或 `WatermarkRegistryRepository` 的 Postgres 写路径。
- `auth:postgres-runtime-qa` 使用 Podman / Docker 或外部 disposable URL 启动 `hiddenshield_auth_runtime_qa`，运行时先 migrate up，完成 QA 后 migrate down 并校验 auth 表为空；安全约束仍要求 localhost / 127.0.0.1，不允许生产库。
- 最新本机证据：`tmp-ui-qa/postgres-auth-runtime/auth-postgres-runtime-qa-1783025723013.json`，Podman 5.7.1，`postgres:16-alpine`，覆盖 fixture challenge、challenge session、password session、同账号双设备一致、refresh rotation、旧 refresh 拒绝、设备列表、device revoke、logout 后 refresh 拒绝；artifact 明确 `syncRepositoryWritePath=not_executed`、`registryRepositoryWritePath=not_executed`。
- `cloud:db-portability-contract` 已升级为结构合同，检查 `PostgresAuthRepository`、`auth_postgres_runtime_qa`、`auth:postgres-runtime-qa` artifact safety token，并禁止 P3.1 悄悄接入 sync / registry Postgres 写路径。

P3.2 评审结论：2026-07-03 完成 `CloudSyncRepository` Postgres adapter 与 `cloud:sync-postgres-runtime-qa`。

- P3.1 artifact `tmp-ui-qa/postgres-auth-runtime/auth-postgres-runtime-qa-1783025723013.json` 可接受：它证明 auth adapter 在 disposable PostgreSQL 下完成 challenge / session / refresh / logout / device revoke 主链，且 `syncRepositoryWritePath=not_executed`、`registryRepositoryWritePath=not_executed`，因此允许进入 P3.2。
- 新增 feature-gated `PostgresCloudSyncRepository`，只实现 `CloudSyncRepository`：`push_cloud_events_batch` 与 `get_cloud_changes`。没有实现 `WatermarkRegistryRepository` 的 Postgres 写路径。
- `cloud:sync-postgres-runtime-qa` 使用 Podman / Docker 或外部 disposable URL 启动 `hiddenshield_sync_runtime_qa`，运行时先 migrate up，完成 QA 后 migrate down 并校验 sync 相关表为空；安全约束仍要求 localhost / 127.0.0.1，不允许生产库。
- 最新本机证据：`tmp-ui-qa/postgres-sync-runtime/cloud-sync-postgres-runtime-qa-1783038415955.json`，Podman 5.7.1，`postgres:16-alpine`，覆盖 desktop push、重复 `client_event_id` 幂等、mobile 初次 pull、重复 pull 空变更、cursor resume、wrong device 拒绝、Free push 403；artifact 明确 `registryRepositoryWritePath=not_executed`。
- P3.2 为了贴近 SQLite 单线程语义，对重复 `client_event_id` 先显式查重再插入，避免 Postgres `BIGSERIAL` 在普通重复事件场景中产生可见 cursor 跳号；唯一约束仍保留作为并发兜底。

P3.3 评审结论：2026-07-03 完成 `WatermarkRegistryRepository` Postgres adapter 与 `watermark:registry-postgres-runtime-qa`。

- P3.2 artifact `tmp-ui-qa/postgres-sync-runtime/cloud-sync-postgres-runtime-qa-1783038415955.json` 可接受：它证明 sync adapter 在 disposable PostgreSQL 下完成 push / pull / cursor / 幂等 / 权益 / 设备绑定，且 `registryRepositoryWritePath=not_executed`，因此允许进入 P3.3。
- 新增 feature-gated `PostgresWatermarkRegistryRepository`，只实现 `WatermarkRegistryRepository`：`reserve_watermark_id`、`confirm_watermark_id`、`reconcile_watermark_id`、`reissue_watermark_id`。没有实现 `CloudSyncRepository` 的 Postgres 写路径，也没有把正式 UI / mock / release 默认路径切到 Postgres。
- `watermark:registry-postgres-runtime-qa` 使用 Podman / Docker 或外部 disposable URL 启动 `hiddenshield_registry_runtime_qa`，运行时先 migrate up，完成 QA 后 migrate down 并校验 registry 相关表为空；安全约束仍要求 localhost / 127.0.0.1，不允许生产库。
- 最新本机证据：`tmp-ui-qa/postgres-registry-runtime/watermark-registry-postgres-runtime-qa-1783051039045.json`，Podman 5.7.1，`postgres:16-alpine`，覆盖 server reserve、同一 request id 幂等 reserve、server confirm、offline reconcile、冲突检测、reissue job、长格式 `HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX` 保持；artifact 明确 `syncRepositoryWritePath=not_executed`、`formalUiMockReleaseDefaultPath=not_switched`。

P3.4 评审结论：2026-07-03 完成 `cloud:postgres-runtime-qa` 聚合门禁。

- 新增 `cloud:postgres-runtime-qa`，串行执行 `auth:postgres-runtime-qa`、`cloud:sync-postgres-runtime-qa` 和 `watermark:registry-postgres-runtime-qa`，每个子门禁都必须生成新的 disposable Postgres artifact。
- 聚合 artifact schema 固定为 `cloud_postgres_runtime_qa_aggregate_v1`，检查三组子 artifact 的 `ok=true`、`productionDatabaseAllowed=false`、`cleanup.status=removed`，并继续要求 sync / registry / formal UI 默认路径不被提前启用。
- 最新本机证据：`tmp-ui-qa/postgres-runtime-aggregate/cloud-postgres-runtime-qa-1783053449984.json`。本次聚合复跑生成的子证据为 `tmp-ui-qa/postgres-auth-runtime/auth-postgres-runtime-qa-1783053450477.json`、`tmp-ui-qa/postgres-sync-runtime/cloud-sync-postgres-runtime-qa-1783053459156.json`、`tmp-ui-qa/postgres-registry-runtime/watermark-registry-postgres-runtime-qa-1783053469951.json`。
- `cloud:db-portability-contract` 已纳入 P3.4 聚合脚本检查，防止后续跳过三组 runtime QA 直接宣称 Postgres 核心路径可用。

P4 本机可验证结论：2026-07-03 完成 SQLite -> PostgreSQL 一次性导入 smoke 的本机切片。

- 新增 `cloud:postgres-import-smoke`，使用 in-memory SQLite fixture 模拟当前首批 auth / sync / registry / rights manifest 数据，再导入 disposable PostgreSQL。
- 当前 smoke 覆盖 10 张首批数据表：`cloud_accounts`、`cloud_devices`、`cloud_sessions`、`auth_challenges`、`auth_attempts`、`cloud_sync_events`、`cloud_device_cursors`、`watermark_id_registry`、`watermark_id_reissue_jobs`、`rights_manifests`。
- 验收固定为 row count 匹配、primary-key hash aggregate 匹配、二次导入 row count / hash 不变、逻辑引用检查通过、`cloud_sync_events` 幂等唯一约束和 `rights_manifests` active partial unique index 生效、最后 migrate down 回到 empty schema。
- 最新本机证据：`tmp-ui-qa/postgres-import/postgres-import-smoke-1783053193204.json`，`tablesChecked=10`、`totalRowsImported=14`、`idempotentRerun=row_counts_unchanged`、`hashAggregate=primary_key_hash_match`、`rollback=empty_schema_verified`。
- 该 P4 smoke 不是 staging 数据迁移，不读取真实用户 SQLite 文件，不证明生产迁移耗时或真实数据质量；真实 staging shadow / replay / 双写仍保持 P5 前阻断。

P5 blocked gate 结论：2026-07-03 已把生产压测与切换前置项机器化为默认 BLOCKED。

- 新增 `cloud:postgres-production-readiness-gate`，默认输出 `cloud_postgres_production_readiness_gate_v1` blocked artifact；设置 `HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY=1` 后，缺少真实 artifact 会失败。
- 强制 ready 模式必须提供并通过：`cloud_postgres_load_gate_artifact_v1`、`cloud_postgres_restore_drill_artifact_v1`、`cloud_postgres_observability_artifact_v1`、`cloud_postgres_cutover_runbook_artifact_v1`、`cloud_postgres_release_owner_signoff_v1`。
- 最新 blocked 证据：`tmp-ui-qa/postgres-production-readiness/cloud-postgres-production-readiness-gate-1783053429272.json`。阻断原因是缺少真实 staging 压测、备份 / PITR / 恢复演练、慢查询 / 锁等待 / 连接池监控、生产切换 runbook 和 release owner 签字 artifact。
- 当前不得把 Enterprise API、云版权库生产 SLA 或生产数据库切换标记为完成。

P6 blocked gate 结论：2026-07-03 已把 SQLite 生产路径下线机器化为默认 BLOCKED。

- 新增 `cloud:postgres-sqlite-shutdown-gate`，检查代码层已有 `SqliteForbiddenInProduction`、Postgres URL 必填和 SQLite dev/test adapter 仍保留。
- P6 gate 必须引用一个已通过的 `cloud_postgres_production_readiness_gate_v1` artifact；在缺少 P5 通过证据时默认 blocked。设置 `HIDDENSHIELD_POSTGRES_REQUIRE_SQLITE_SHUTDOWN_READY=1` 后，缺少 P5 通过证据会失败。
- 最新 blocked 证据：`tmp-ui-qa/postgres-sqlite-shutdown/cloud-postgres-sqlite-shutdown-gate-1783053429239.json`。当前阻断原因是 P5 production readiness 尚未通过。
- SQLite 继续保留在本地 / dev / test / RC1 无外部依赖范围；不得移除或破坏 SQLite dev/test adapter。

### Phase P1：数据库抽象层

- 为 `feedback-backend` 增加 Storage trait / repository 分层。
- 保留 SQLite adapter 作为 dev / test adapter。
- 新增 Postgres adapter skeleton，不改变 API 行为。
- 增加 `cloud:db-portability-contract`，同一组合同样例在 SQLite / Postgres schema 上通过。

验收：SQLite 路径合同不退，Postgres adapter 可跑 schema 和最小读写 smoke。

### Phase P2：PostgreSQL schema migration

- 建立 `schema_migrations`。
- 把首批迁移表改写为显式 PostgreSQL migration。
- 建立约束、索引、partial unique index、JSONB 字段和 timestamptz。
- 加入本地 dev Postgres 启动说明或 CI service 配置。

验收：空库 migrate up 成功，重复 migrate 不漂移，schema diff 合同通过。

### Phase P3：核心读写切换

- 先按 P3 前置设计迁移 auth，再迁移 cloud sync，最后迁移 watermark registry / rights manifest。
- auth 通过 `auth:postgres-runtime-qa` 后，才能进入 sync；sync 通过 `cloud:sync-postgres-runtime-qa` 后，才能进入 registry；registry 通过 `watermark:registry-postgres-runtime-qa` 后，才能评审 Enterprise quota / audit / payment / report grants。
- 所有 API 保持响应契约不变。
- SQLite 继续支持本地 smoke，但标记 non-production。

验收：`cloud:ci`、`commercial:ci` 拆分步骤、Enterprise runtime QA、cloud sync reliability contract 在 Postgres 上通过。

### Phase P4：数据迁移与双写演练

- 编写 SQLite -> PostgreSQL 一次性导入工具。
- 对关键表做 row count、hash aggregate、唯一约束和外键引用校验。
- staging 环境做只读 shadow、再做受控双写或写入回放。
- 记录迁移耗时、失败批次、可重跑策略。

验收：staging 数据迁移可重跑，导入后合同和运行态 QA 通过。

### Phase P5：生产压测与切换

- 用 PostgreSQL staging 跑第 10 节压测。
- 配置备份、恢复演练、慢查询监控、锁等待告警。
- 设定生产切换窗口和回滚窗口。
- 切换 Enterprise API / 云版权库生产环境到 Postgres。

验收：压测门槛全部通过，恢复演练通过，release owner 签字。

### Phase P6：SQLite 生产路径下线

- 后端生产启动拒绝 `--db-path` SQLite 配置。
- 保留 dev / test 命令，但明确输出 `non_production_sqlite_backend`。
- 文档和 runbook 移除 SQLite 生产部署描述。

验收：生产环境无法误用 SQLite 启动。

## 12. 回滚策略

迁移前必须有两级回滚：

- 配置回滚：P1-P3 期间可以通过环境变量切回 SQLite adapter，仅限本地 / staging，不允许生产长期回退到 SQLite。
- 数据回滚：生产切换前冻结写入窗口或启用可重放事件日志，保留切换前 SQLite 快照和 Postgres PITR 点。

不允许的回滚：

- Enterprise API 已对外生产接入后，不允许把 quota ledger / audit 写回 SQLite。
- 支付 webhook 生产接入后，不允许用手工 SQLite 文件替代正式交易账本。
- 云版权库多实例部署后，不允许回到单文件共享 SQLite。

## 13. 验收门禁

建议新增门禁：

1. `cloud:postgres-migration-contract`
   - 检查本文存在并包含 SQLite 保留范围、PostgreSQL 生产边界、迁移表清单、风险、压测门槛和实施阶段。
   - 检查生产配置不得使用 SQLite。

2. `cloud:db-portability-contract`
   - 同一组 auth / entitlement / sync / registry / rights manifest / payment / Enterprise fixture 在 SQLite dev adapter 与 Postgres adapter 行为一致。

3. `auth:postgres-runtime-qa`
   - 用真实 disposable / staging Postgres 跑账号 fixture、登录 challenge、session 创建、refresh rotation、logout、device revoke、过期 session 拒绝和 SQLite/Postgres 响应字段一致性。

4. `cloud:sync-postgres-runtime-qa`
   - 用真实 Postgres 跑 desktop / mobile 双设备 pull / flush / pull、断线恢复、重复 flush、增量 cursor、Free 403、Creator allowed、同步队列限流和已同步记录不重复上传。

5. `watermark:registry-postgres-runtime-qa`
   - 用真实 Postgres 跑 UID reserve / confirm / idempotency / offline reconcile / conflict / long UID preservation / rights manifest active partial unique index。

6. `enterprise:postgres-runtime-qa`
   - 用真实 Postgres 跑 API key、trusted proxy、rate limit、quota ledger、audit 和 idempotency。

7. `billing:postgres-runtime-qa`
   - 用真实 Postgres 跑订阅 webhook、report purchase webhook、查单补偿和退款撤销。

8. `cloud:postgres-load-gate`
   - 固化第 10 节压测门槛，默认可 smoke，生产强制模式必须跑完整压测 artifact。

## 14. P3 前置设计边界

P3 前置设计阶段不做：

- 不把 PostgreSQL adapter 接入正式 UI / mock / release 默认路径。
- 不连接生产 PostgreSQL。
- 不一次性实现 auth / sync / registry 三组写路径。
- 不改桌面端或移动端同步 payload。
- 不改版权编号、watermark payload 或 `watermark-core`。
- 不绕过现有权益、报告、公开权利、Enterprise API 或 billing 合同。
- 不把 Enterprise API 或云版权库 SLA 状态改为已完成。

P3 前置设计阶段只做：

- 固化 repository Postgres 读写实现顺序。
- 固化每组最小运行态 QA。
- 固化 disposable / staging PostgreSQL 连接边界。
- 固化 SQLite dev/test adapter 不退要求。

## 15. 推荐下一步

当前真实状态记录：2026-07-03 PostgreSQL 迁移长任务暂停推进。本机可验证范围已推进到 P4：P3.4 聚合 runtime QA 通过，P4 SQLite fixture -> disposable PostgreSQL 导入 smoke 通过；P5 生产压测 / 备份恢复 / observability / 切换 runbook / release owner 签字与 P6 SQLite 生产路径下线均已机器化为 BLOCKED artifact。当前不再继续推进 PostgreSQL 迁移实现，不连接生产库，不切正式 UI / mock / release 默认路径，SQLite dev/test adapter 继续保留。

进入 P5 真实外部环境准备：由 release owner 提供 staging PostgreSQL 压测、备份 / PITR / 恢复演练、observability、切换 runbook 和签字 artifact，然后以 `HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY=1 npm run cloud:postgres-production-readiness-gate` 强制模式复跑。
