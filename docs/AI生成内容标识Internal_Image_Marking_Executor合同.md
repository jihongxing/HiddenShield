# AI 生成内容标识 Internal Image Marking Executor 合同

## 状态与边界

- 状态：`internal_only_postgres_executor`。
- 本合同只允许内部命令将已存在的 `ready_to_confirm` 图片标识会话推进为已确认记录；不提供 HTTP route、SDK、公共 Resolver、客户 credential 或生产发放。
- 图片盲水印写入、读取与写后回读必须只调用 `watermark-core` 的正式 V3 图片 API；backend 不得复制或实现另一套算法、payload 编码或提取规则。
- 本执行器产生内部显式标签计划与 receipt，不渲染平台 UI、不添加像素叠层、不写 C2PA production 签名或声称任何法律合规结果。

## 输入

内部命令必须包含：

- `markingSessionId`：已存在且状态为 `ready_to_confirm`。
- `executionId`：内部幂等/审计关联 ID，用于 Manifest、ledger、audit 与 receipt ID。
- `watermarkUid`：现有 HiddenShield V3 anchor UID。
- `sourceImageBytes`：平台生成的最终图片字节。
- 生成事实：provider、system、model、generation mode、生成时间和操作摘要。

调用前不写数据库；session 最终状态仍由既有 PostgreSQL confirm 原子事务决定。

## 执行顺序

1. 只读检查 session 为 `ready_to_confirm`，并读取 requested Profile。
2. 以 `watermarkUid` 构造 `watermark-core` V3 图片 anchor，写入保护副本。
3. 对保护副本调用同一 `watermark-core` 读取 API；仅当 UID、V3 协议和 auth status 全部匹配时继续。
4. 计算保护副本 SHA-256，构造内部 Manifest/Evidence、blind-watermark marker binding 与显式标签计划。
5. 调用 `execute_postgres_confirm_marking_command`；该命令在同一 PostgreSQL 事务写 Manifest、Evidence、Marker、label receipt、`confirmed_marked_image` ledger、confirm audit，并将 session 改为 `confirmed`。
6. 只有 confirm 成功才返回保护副本与标签计划；任何拒绝、核心写入失败、回读失败或 PostgreSQL 失败均不返回保护副本。

## 固定内部语义

- 仅接受图片、`claimType=ai_generated` 与 `hiddenshield_v3_image_anchor_v1` requested Profile。
- 输出统一为 PNG，`allow_rewrite=false`；已检测到既有 anchor 的输入由 `watermark-core` 拒绝。
- Evidence 固定为 `self_declared` / `watermark_core_write_after_read`，其 source 仅标识内部执行器；不携带平台签名、外部签名或法律结论。
- 除 anchor Profile 外，每个 requested Profile 生成一份 `platform_ui` 显式标签计划；其 receipt 证明计划已生成，不证明平台 UI 已实际渲染。

## PostgreSQL Gate

- 成功：保护副本可回读同一 V3 UID，session=`confirmed`，并且每张表仅产生一份 Manifest、Evidence、Marker、label receipt、committed ledger 与 confirm audit。
- 失败：session 保持 `ready_to_confirm`，且不产生上述 confirm 记录；不得返回保护副本。
- 端到端 QA 必须先通过 custody command 创建 `ready_to_confirm` session，再执行 executor，不允许直接伪造 confirmed 记录。

## 未解锁项

- 真实 IAM/KMS/HSM、production credential、生产 C2PA/TSA 签名、SDK、公共 Resolver、对外 API、真实客户发放和中美欧合规宣传继续关闭。

## 跨端 Fixture

- Executor 输出 PNG、metadata 剥离变体、固定 UID/auth 预期和平台写入到正式读取路径矩阵，以 `docs/AI生成内容标识平台写入PNG跨端Fixture合同.md` 为唯一合同。
- Desktop 与 Android/iOS 共用 mobile Rust bridge 的宿主测试只构成内部 fixture 证据；iOS 仍必须在实际 macOS/iOS runtime 复跑，宿主测试不得替代设备证据。
