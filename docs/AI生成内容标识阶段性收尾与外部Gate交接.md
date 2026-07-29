# AI 生成内容标识阶段性收尾与外部 Gate 交接

更新时间：2026-07-29

状态：`phase_suspended_external_dependencies_internal_control_plane_frozen_through_0022`

能力分类：`只能内部测试`

## 收尾结论

HiddenShield AI 生成内容标识基础设施的内部生产导向控制面已冻结至 PostgreSQL
`0022_ai_transparency_external_evidence_review`。当前阶段不再新增 SDK、公共
Resolver、production credential 或公网生产行为；任务族因外部环境与配置依赖阶段性挂起。

该暂停不降低既有回归要求，也不把内部测试结果升级为真实平台接入、法律合规、生产可用或 SLA。

## 已冻结的内部基线

- Profile entitlement、双人审批、append-only audit、confirm/ledger 原子事务和 PostgreSQL 并发语义。
- production credential custody、marking session、`watermark-core` PNG V3 写入/回读、post-embed signing、recovery/dead-letter 与交付安全控制面。
- `packages/ai-transparency-sdk`、internal platform API facade、confirmed-only 最小匿名 Resolver 与 synthetic Sandbox。
- external Evidence intake/review 的双人审核、reference/IAM fail-closed 校验与 PostgreSQL `0021/0022` QA。
- PostgreSQL QA 故障注入矩阵，以及仅允许名称含 `hiddenshield_migrate_smoke` 的一次性数据库执行的统一 QA suite。

## 挂起 Gate

| Gate | 恢复所需外部输入 | 不可替代的验收 |
| --- | --- | --- |
| 真实 provider recovery | Internal IAM endpoint/JWKS、工作负载身份引用、KMS/HSM pepper、signer/object-store/notification provider 配置 | 受控 provider recovery 演练与不可变 evidence |
| 法务 Profile | CN、EU、US-CA 的签署法务审查 receipt 与持续更新责任 | 按 Profile 的受控验真，不输出全球合规结论 |
| 设计伙伴 Sandbox | 伙伴身份、Sandbox endpoint、`secret://` 引用、处理链运行 evidence 与书面验收 | 12 场景真实 Sandbox 验收 |
| 三层处理链与 iOS | 可再分发第三方样本及授权、平台处理链说明、macOS/iOS runtime | 独立跨端与互操作 Benchmark |

## 暂停期约束

- 保持 SDK、公共 Resolver、公网 API、production credential、客户 SLA 与任何生产发放关闭。
- 继续把 `accepted_for_gate`、synthetic QA、migration smoke 和本地 PostgreSQL QA 表述为内部控制面证据，不能表述为外部验收或法规合规。
- 不伪造 provider、伙伴、Secret、法务签署、第三方样本授权或 iOS 运行证据。
- 任何恢复工作都必须先经 External Evidence Intake 和既有双人审核状态机，再执行对应 Gate。

## 保留验证

- `npm run ai-transparency:ci`
- `npm run ai-transparency:postgres-qa-contract`
- `npm run ai-transparency:postgres-qa`
- `npm run ai-transparency:postgres-failure-matrix`

`ai-transparency:postgres-qa` 只能使用一次性 PostgreSQL 测试库；不应在普通开发、共享或生产数据库上执行。

## 恢复顺序

1. 外部责任方提供不可变引用，并通过 Evidence Intake 与双人审核。
2. 使用受控隔离环境完成真实 provider recovery 或设计伙伴 12 场景 Sandbox Gate。
3. 对应 Gate 的签署 evidence 通过后，逐项评审 production entitlement、生产发放和公网能力；不得批量解除。
4. iOS 与第三方处理链 Gate 独立恢复，不以 Android、synthetic 或本地 PostgreSQL 结果替代。
