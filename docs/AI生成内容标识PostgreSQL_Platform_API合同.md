# AI 生成内容标识 PostgreSQL Platform API 合同

状态：`internal_postgresql_platform_api_gate_passed`

日期：2026-07-28

能力分类：`只能内部测试`

## 1. 固定端点

- `POST /v1/ai-transparency/admissions`
- `POST /v1/ai-transparency/sessions`
- `POST /v1/ai-transparency/images/mark`
- `POST /v1/ai-transparency/images/confirm`

四个端点由独立 PostgreSQL-only Axum router 提供，不接入旧 SQLite `Storage`，不提供 SQLite fallback。

## 2. Admission

- Bearer credential 必须为 active、production、未过期并具备 `mark:image` scope。
- credential、license、tenant、workspace 与 issuer mode 必须一致。
- SDK issuer mode 映射固定为：
  - `hiddenshield_managed` → `hiddenshield_managed`
  - `customer_managed` → `customer_byok`
  - `platform_signed` → `platform_managed`
- regulatory Profile 必须绑定 active regulatory version。
- technical Profile 必须绑定 active technical version。
- entitlement version set 使用排序后的 Profile/version canonical form 计算 SHA-256，并持久化 admission snapshot。

## 3. Session

- Session 创建强制复用 production credential custody。
- presented credential 必须与 admission 中冻结的 credential identity 一致。
- 只有 active admission、有效 production license、有效 versioned Profile entitlement 和可用 custody provider 才能创建。
- 新平台 session 初始状态为 `ready_to_upload`。
- 旧 internal executor 使用的 `ready_to_confirm` 创建函数保持兼容。
- session 绑定 admission、watermark UID、generation event、subject reference、content type 与 entitlement digest。

## 4. Mark

- 只接受不超过 64 MiB 的 `image/png`。
- 服务端重新计算原图 SHA-256，摘要或 PNG signature 不匹配时零 marking 写入。
- credential 在 session state claim 前验证；无效 credential 不得把 session 改为 `processing`。
- 标识写入与写后回读强制调用现有 image marking executor 和 `watermark-core`。
- mark 成功后持久化 confirm command、marked hash、marker evidence digest、explicit label receipt digest 与 confirmation token HMAC。
- mark 不创建 Manifest、不写计量 ledger；状态只推进到 `ready_to_confirm`。

## 5. Confirm

- confirmation token、marked file hash 与持久化 submission 必须一致。
- confirm 在同一 PostgreSQL transaction 中复用现有 confirm command，原子写入 Manifest、evidence、marker、label receipt、`confirmed_marked_image` ledger、confirm audit 和 platform projection。
- 成功计量固定为 `confirmed_marked_image + quantity=1 + committed`。
- duplicate confirm replay 必须复用首次 confirm idempotency key、原 Manifest 和 ledger，不产生第二次计量；不同 key 拒绝。
- token/hash mismatch、session conflict 或 confirm 校验失败均不得产生 Manifest 或 committed ledger。

## 6. Migration

Migration：`0019_ai_transparency_platform_api`

新增：

- `ai_platform_profile_admissions`
- `ai_platform_marking_sessions`
- `ai_platform_marking_submissions`
- `ai_platform_api_audit_events`
- `ready_to_upload` session state

平台 API audit 为 append-only，不记录 credential 明文或图片 bytes。

## 7. QA Gate

一次性 PostgreSQL 16 数据库：

`hiddenshield_migrate_smoke_platform_api`

真实链路：

`@hiddenshield/ai-transparency-sdk → platform facade → HTTP transport → Axum router → PostgreSQL → credential custody/Profile entitlement → watermark-core executor → confirm command`

已验证：

- production admission 成功。
- Profile 未授权拒绝。
- session 创建为 `ready_to_upload`。
- 无效 credential 的 mark 零状态推进。
- PNG mark 与 V3 写后回读成功。
- hash mismatch confirm 零计量。
- confirm 只产生一个 Manifest 和一个 committed ledger。
- duplicate replay 复用 ledger。
- 0001–0019 PostgreSQL up/down migration smoke。
- backend 92 tests 与 SDK 9 tests 通过。

## 8. 外部边界

继续关闭：

- SDK npm 发布。
- 公网 API gateway。
- production credential 发放。
- 真实 IAM/JWKS、workload identity 与 KMS/HSM pepper 注入。
- 免费公共 Resolver。
- 客户 SLA、法规结论和设计伙伴生产验收。

QA provider 仅存在于一次性本地 PostgreSQL harness，不得进入生产 composition。

## 9. 下一 Gate

免费公共 Resolver 最小只读 Gate 已由 `0020_ai_transparency_public_resolver` 完成，详细合同见 `docs/AI生成内容标识免费公共Resolver合同.md`。

真实设计伙伴 sandbox 接入包已冻结并通过内部 contract test，详细合同见 `docs/AI生成内容标识设计伙伴Sandbox接入包合同.md`。

下一 Gate：用首个真实伙伴外部配置执行 12 场景 Sandbox 验收；公网 gateway、production credential 和真实 provider 继续关闭。
