# AI 生成内容标识 Regulatory Profile 法律控制矩阵

版本：`v1-draft`

状态：`owner_audit_approved_not_external_legal_opinion`

> 本矩阵是外部法务审查的技术与证据工作底稿，不是法律意见、合规认证或生产授权。授权产品负责人已于 `2026-07-27` 以内部审计状态批准继续工程，但该批准不是外部法律意见，也不替代有效外部法务签署 receipt。

## 1. Gate 结论

`法务 Profile Gate = PASSED_BY_AUTHORIZED_OWNER_NOT_EXTERNAL_LEGAL_OPINION`。

该 Gate 只允许继续内部工程、合同测试和非生产验证；不证明任一司法辖区合规，且不解锁 production entitlement、production credential、SDK、公共 Resolver 或法规合规宣传。当前仍未取得外部法务签署。CN Profile ID 仍存在命名不一致：MVP 文档使用 `cn_aigc_label_2025_image_v1`，数据库/API 与审批 fixture 使用 `cn_aigc_label_2025_image_export_v1`。在任何生产使用前必须指定唯一 canonical ID，并将旧名称作为 alias 或废弃项处理。

## 2. 共同审查规则

每个 regulatory Profile 都必须由外部法务确认：

1. 适用司法辖区、适用日期、适用主体和分发场景。
2. 绑定的官方法规 / 标准来源、来源快照、版本与 hash。
3. 每个必需技术控制、可选控制、例外和人工流程。
4. 技术证据如何映射到控制，不得把“控制已执行”表述为法律结论。
5. `effectiveFrom`、`reviewedAt`、`validUntil`、Profile owner、变更触发条件和复审频率。
6. 适用的显式标签、机器可读元数据、来源声明、检测/查询、日志与导出要求。

共同禁止：

- `compliant=true`、`globally_compliant=true`、`fully_legally_compliant`。
- 将 C2PA、数字水印、Manifest 或 provider receipt 单独描述为法律合规证明。
- 将本矩阵、内部 HMAC receipt 或未签署草案用于 production entitlement。

## 3. Profile 控制矩阵

| 字段 | CN 草案 | EU 草案 | US（加州）草案 |
| --- | --- | --- | --- |
| canonical `profileId` | `cn_aigc_label_2025_image_export_v1`（待统一） | `eu_ai_act_article_50_2026_image_v1` | `ca_ai_transparency_2026_image_v1` |
| Profile 类型 | `regulatory` | `regulatory` | `regulatory` |
| 媒体 / 生命周期 | 图片生成或编辑；平台 UI、下载导出、API 交付（均待法务确认） | 图片生成或编辑；平台 UI、下载导出、API 交付（均待法务确认） | 图片生成或编辑；平台 UI、下载导出、API 交付（均待法务确认） |
| 官方来源审查起点 | 中国国家互联网信息办公室等发布的《人工智能生成合成内容标识办法》及实施相关官方材料；由外部法务固定 URL、版本、发布日期与 source digest | [Regulation (EU) 2024/1689, Article 50](https://eur-lex.europa.eu/eli/reg/2024/1689/oj)；外部法务确认适用日、适用对象与相关官方实施材料 | [California Legislative Information](https://leginfo.legislature.ca.gov/) 中适用法案与最终法典条文；外部法务固定 bill / code section、版本、生效时间与 source digest |
| 适用主体 | 待确认：生成服务、传播服务、用户发布及平台角色的边界 | 待确认：AI system provider、deployer、内容平台及特定豁免 | 待确认：覆盖的 generative AI provider、部署者/平台及门槛/豁免 |
| 必需控制候选 | 显式标签、导出交付标识、机器可读元数据、生成/编辑事实证据、适用日志 | 机器可读标识、适用透明度说明、来源/元数据证据、适用日志 | latent disclosure、来源信息、适用检测/查询能力、平台/交付说明、适用日志 |
| 技术映射候选 | Explicit Label Adapter、Metadata Adapter、Manifest/Evidence、watermark-core 锚点、导出 receipt | Metadata Adapter、C2PA 兼容声明、Manifest/Evidence、watermark-core 锚点、Resolver 结果 | Metadata/Manifest、latent disclosure 适配层、detector signpost（若经法务确认）、Evidence |
| 不可单独满足的项目 | 传播/发布责任、用户场景、例外、执法解释 | 主体角色判断、风险/例外、实践框架是否适用 | 覆盖范围、门槛、检测义务、平台义务、州法变化 |
| 证据最小集 | source snapshot、control matrix、技术执行 receipt、外部法务签署 receipt | source snapshot、control matrix、技术执行 receipt、外部法务签署 receipt | source snapshot、control matrix、技术执行 receipt、外部法务签署 receipt |
| 签署前状态 | `configuration_required` | `configuration_required` | `configuration_required` |
| entitlement 状态 | `blocked` | `blocked` | `blocked` |

## 4. 控制项逐项填表

外部法务必须为每个 Profile 填写下表；空白字段一律视为 Gate 不通过。

| 控制 ID | 法律 / 标准来源 | 法律文本定位 | 适用条件 | 必需技术控制 | 人工 / 流程控制 | 例外与限制 | 技术证据字段 | 验证方法 | 结论 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `CN-01` | 待外部法务填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | `pending` |
| `EU-01` | 待外部法务填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | `pending` |
| `CA-01` | 待外部法务填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | `pending` |

## 5. Profile 变更与失效

以下任一变化自动使相应 legal review receipt 失效，并将 entitlement 保持 / 切换为 `blocked`：

- 法规、官方解释、适用日期、法案/条文版本或主体范围改变。
- 技术控制、数据字段、Metadata/C2PA 映射、显式标签、检测/查询或分发场景改变。
- provider receipt 协议、密钥、scope digest、证据存储或审计边界改变。
- 外部法务签署到期、撤销、范围不匹配或无法验证。

## 6. 签署前 Gate

每个 Profile 只有同时满足以下条件才能进入“可评审 production entitlement”的下一步，而不是自动获得 production entitlement：

```text
canonical profile ID resolved
official source snapshot + hash recorded
control matrix complete
external counsel review receipt signed and valid
receipt scope matches profile/version/jurisdiction/actor/distribution/media
technical evidence mapping tested
Profile owner and expiry recorded
change/revocation workflow tested
```

之后仍需通过 confirm 原子事务、production credential custody、平台试点及独立 release Gate。
