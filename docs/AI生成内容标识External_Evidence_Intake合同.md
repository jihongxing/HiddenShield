# AI 生成内容标识 External Evidence Intake 合同

状态：`postgresql_internal_only_append_only_intake_and_review_implemented`

能力分类：`只能内部测试`

本合同接收未来真实 provider recovery 与设计伙伴 Sandbox 的外部证据引用。它只验证结构、不可变摘要、授权与签署引用，并写入 PostgreSQL append-only intake/audit；不访问外部 URL、不验证事实真实性，也不将材料升级为 production activation 或伙伴验收。

## 提交前置

- submitter 必须经 Internal IAM 以 `ai_transparency_security_approver` 授权。
- contract 与 security review reference 必须通过既有 fail-closed reference adapter。
- 失败发生在 PostgreSQL 事务前，不得创建 intake 或 audit。

## 数据与审计边界

- `sourceKind` 仅允许 `provider_recovery` 或 `design_partner_sandbox`。
- `evidenceReference` 必须为 `evidence://sha256/<64-lowercase-hex>`，且与 `evidenceSha256` 完全一致。
- 来源、签署与审批只允许 `provider://` / `partner://`、`approval://` / `receipt://` 和 `approval://` 引用。
- 不允许 raw Secret、token、`replace-me`、placeholder 或可变 URL；`validUntil` 必须晚于 `validFrom`。
- migration `0021_ai_transparency_external_evidence_intake` 在同一 PostgreSQL 事务写入 intake 与 `evidence_received` audit；两张表均为 append-only。

## 不构成通过

`received_for_review` 只表示结构化材料已受控接收，不表示 provider 可用、证据真实、法务已签署、Sandbox 已验收、production credential 可发放、SDK 可发布或法规义务已满足。

真实 evidence、provider receipt、签署引用、伙伴身份与运行证据仍必须由外部责任方提供，并在独立 recovery / Sandbox Gate 中验真。

## 审核决策

- 审核人必须经 Internal IAM 在 intake 的真实 `tenant/workspace/environment` 作用域内取得 `ai_transparency_compliance_approver` 授权；禁止通配符作用域授权。
- `reviewReference` 必须由 fail-closed security-review reference adapter 验真；同一证据提交人与审核人不得相同，过期证据不得审核。
- migration `0022_ai_transparency_external_evidence_review` 对每个 intake 最多追加一个不可变决策和一个对应 audit；决策、审计必须在同一 PostgreSQL 事务写入。
- `accepted_for_gate` 仅记录内部材料审核结论，不改变 intake，不解锁真实 provider activation、Sandbox acceptance、production credential、SDK 生产发放或法律合规结论。
- `ai_transparency_external_evidence_review_qa` 已在 disposable PostgreSQL 16 数据库验证 accept、同人审核拒绝、过期拒绝、reference 拒绝、audit 写入故障全事务回滚、append-only trigger 和同一 intake 的双连接竞争；拒绝/故障场景零决策与审计残留，竞争场景最多一条决策与对应 audit。
