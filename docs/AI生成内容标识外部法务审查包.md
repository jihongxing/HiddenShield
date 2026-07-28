# AI 生成内容标识外部法务审查包

版本：`v1-draft`

状态：`owner_audit_approved_not_external_legal_opinion`

关联矩阵：`docs/AI生成内容标识Regulatory_Profile法律控制矩阵.md`

## 1. 审查委托范围

授权产品负责人已于 `2026-07-27` 以内部审计状态允许项目继续内部工程。该决定不是外部法律意见、法律签署或合规认证；本审查包仍需由具备相应资质的外部法务完成。

请外部法务按 Profile 分别审查：

- `cn_aigc_label_2025_image_export_v1`（先解决与 MVP 文档中的 CN ID 命名不一致）。
- `eu_ai_act_article_50_2026_image_v1`。
- `ca_ai_transparency_2026_image_v1`。

审查对象仅限 HiddenShield 的 AI 图片生成 / 编辑时标识基础设施。请求法务确认技术控制映射与适用边界，不请求不受条件限制的“全球合规”结论。

## 2. 交付物清单

外部法务必须提交或确认：

1. 每个 Profile 的官方来源 URL、获取日期、版本、适用日期和 source digest。
2. 适用主体、媒体、地域、分发场景、业务角色与例外。
3. 控制矩阵的逐条结论：`required`、`optional`、`not_applicable`、`insufficient_evidence`。
4. 技术证据映射、无法由技术单独证明的人工/流程义务。
5. Profile 有效期、复审触发条件、owner 和变更流程。
6. 已签署的 legal review receipt；未签署或 scope 不完整的 receipt 一律拒绝。

## 3. 给外部法务的问题

每个 Profile 必须回答：

1. 当前法律文本是否已生效、何时生效、是否存在过渡期？
2. HiddenShield 的平台客户在该场景下属于何种法律角色？
3. 平台 UI、导出文件、API 交付、二次编辑与传播分别需要哪些控制？
4. 显式标签、机器可读 metadata、C2PA/Manifest、水印、检测/查询和日志分别是必需、可选还是不足？
5. 哪些义务依赖平台操作、用户行为、合同、通知、治理或人工流程，不能由 HiddenShield 技术控制代替？
6. 已知例外、地域门槛、主体豁免、内容类型限制和执法不确定性是什么？
7. Profile 的审查有效期、复审频率和失效触发条件是什么？

## 4. 签署 Receipt 模板

此模板只用于外部法务签署；不得由 HiddenShield 工程、产品、商业人员代签。

```json
{
  "schemaVersion": "hs-ai-regulatory-legal-review-receipt-v1",
  "receiptId": "external-counsel-generated-id",
  "profileId": "cn_aigc_label_2025_image_export_v1",
  "profileVersion": "v1",
  "jurisdiction": "CN",
  "actorRole": "generator_provider",
  "distributionModes": ["in_product_view", "download_export", "api_delivery"],
  "mediaTypes": ["image"],
  "sourceSnapshotDigest": "sha256:external-counsel-generated",
  "controlMatrixDigest": "sha256:external-counsel-generated",
  "reviewDisposition": "approved_for_technical_control_mapping",
  "limitations": [
    "Technical controls do not replace platform-specific legal and operational obligations."
  ],
  "effectiveFrom": "external-counsel-confirmed",
  "validUntil": "external-counsel-confirmed",
  "reviewedAt": "external-counsel-confirmed",
  "counsel": {
    "firm": "external-counsel-confirmed",
    "reviewerId": "external-counsel-confirmed",
    "jurisdictionQualifications": ["external-counsel-confirmed"]
  },
  "signatureFormat": "external-counsel-confirmed",
  "signature": "external-counsel-generated",
  "signedAt": "external-counsel-confirmed"
}
```

## 5. Receipt 验收规则

内部 provider / reference adapter 在未来接收外部 legal review receipt 前，必须验证：

- receipt schema/version、Profile ID/version、jurisdiction、actor/distribution/media scope。
- source snapshot 与 control matrix digest。
- 签署者身份、授权范围、签名、签署时间、有效期、撤销状态。
- receipt 对应的 Profile owner、变更记录和复审条件。

当前受控 HMAC provider receipt 仅用于内部测试，**不得**验证或替代外部法务签名。

## 6. Gate 决策记录

| Profile | 外部法务 receipt | scope / signature 验证 | 控制矩阵完成 | Gate |
| --- | --- | --- | --- | --- |
| CN | 未提供 | 未开始 | 草案 | `OWNER_AUDIT_PASSED_NOT_EXTERNAL_LEGAL_OPINION` |
| EU | 未提供 | 未开始 | 草案 | `OWNER_AUDIT_PASSED_NOT_EXTERNAL_LEGAL_OPINION` |
| US（加州） | 未提供 | 未开始 | 草案 | `OWNER_AUDIT_PASSED_NOT_EXTERNAL_LEGAL_OPINION` |

上述内部审计状态仅允许继续内部工程。在三项 receipt 均有效、逐条控制矩阵完成且所有其他生产 Gate 通过前，仍禁止创建 production entitlement、production credential、SDK 发行、公共 Resolver 或法规合规宣传。
