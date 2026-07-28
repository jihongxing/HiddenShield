# HiddenShield AI 生成内容标识基础设施 MVP 设计

更新时间：2026-07-28

状态：`production_oriented_mvp_control_plane_implemented_through_0018`

实施状态：`生产导向控制面已实现至 PostgreSQL 0020；内部图片标识、签发、交付、安全治理、通知 outbox、provider-neutral delivery、server-side SDK/API facade、真实 PostgreSQL 平台端点与匿名免费 Resolver Gate 已通过；公网部署、真实 provider 注入和生产发放仍关闭`

能力分类：`只能内部测试，不构成当前用户承诺或真实法律意见`

## 1. 决策摘要

HiddenShield 提前建设公共信任层中的一个生产导向最小子集：

```text
AI 图片平台生成时标识
```

该 MVP 面向能够在图片生成或编辑链路中主动接入 HiddenShield 的 AIGC 平台、企业生成服务和模型应用，不面向上传任意未知图片后概率判断其是否由 AI 生成。

冻结定位：

> HiddenShield 为 AI 图片平台提供生成时的多层 AI 来源标识、C2PA 兼容声明、鲁棒水印锚点、跨法规 Profile 和统一验证接口。

冻结技术组合：

```text
可感知标签数据
+ 数字签名元数据
+ watermark-core 鲁棒锚点
+ AI Transparency Manifest
+ Evidence
+ Registry / Resolver
+ 统一验证结果
```

当前 MVP 不再定义为一次性演示原型，而定义为能够支撑付费授权、生产凭据、可审计标识、故障恢复和后续平台嵌入的最小生产基础设施。

### 1.1 生产导向 MVP 定义

本项目中的 `MVP` 指：

> 在不对外开放 SDK、公共 Resolver 或生产 credential 的前提下，先完成最小可生产控制面和内部端到端图片标识闭环，再通过真实 provider 与设计伙伴 Gate 开放平台能力。

因此以下能力属于 MVP 核心，而不是偏离或后置运维附属项：

- License、issuer mode、Profile entitlement 与 production/sandbox 隔离。
- actor/role、change request、四眼审批、execution 和 append-only audit。
- production credential custody、rotate/revoke 与受控 marking session。
- `watermark-core` 图片写入、写后回读、confirm 原子事务和唯一计量 ledger。
- post-embed C2PA signing reservation、artifact finalize、崩溃恢复与 dead-letter。
- confirmed/finalized delivery envelope、短期 retrieval authorization、revoke 与资源预算。
- delivery observability、incident、cleanup runner、inspect/list、provider-neutral notification outbox、destination policy、receipt、completion/dead-letter 和 recovery。

生产导向不等于当前已经生产可用：

- 真实 Internal IAM、KMS/HSM、signer、object-store 和通知 provider 配置仍属于外部依赖 Gate。
- 平台 SDK、公共 Resolver、客户 UI/API、生产 credential 发放和客户 SLA 仍保持关闭。
- 当前实现证明内部合同、事务、并发和恢复语义，不代表真实 provider、真实平台或真实法律合规已验收。

## 2. 为什么提前

### 2.1 监管窗口

中国《人工智能生成合成内容标识办法》已于 2025-09-01 起施行，要求显式标识和文件元数据隐式标识协同，并鼓励使用数字水印。

欧盟 AI Act 第 50 条透明度义务自 2026-08-02 起适用。欧盟 2026 年最终透明度实践框架强调机器可读标识、可检测性、有效性、互操作性、鲁棒性和可靠性，并把数字签名元数据与不可感知水印作为重要组合。

美国当前没有覆盖全部 AI 内容的统一联邦标识制度，但加州 AI Transparency Act 自 2026-08-02 起进入适用阶段，并要求覆盖范围内的生成式 AI 提供商提供 latent disclosure、检测能力和符合广泛采用行业标准的来源信息。

三地共同方向不等于一套固定字段即可自动满足全部要求：

- 中国需要区分生成服务、传播服务和用户发布责任，并针对导出、复制等文件交付场景处理显式和隐式标识。
- 欧盟需要区分 AI 系统提供者和 deployer；Article 50 的法定义务是技术中立的，透明度实践框架是可用于证明措施充分性的自愿路径，不是自动合规证书。
- 美国需要按州和适用主体判断。加州覆盖范围、免费检测工具、manifest disclosure、latent disclosure 和大型在线平台义务的生效时间并不相同。

因此产品只允许输出“指定 Profile 的技术控制状态和证据”，不允许输出不附条件的 `compliant=true`、`global_compliance=true` 或“已满足中美欧全部法规”。

### 2.2 行业缺口

各平台已经分别采用：

- C2PA / Content Credentials。
- 平台自有不可感知水印。
- 文件元数据。
- 平台数据库和内容编号。
- Provider 专属检测接口。
- 用户可见 AI 标签。

当前缺口不是完全没有标准，而是：

- 标识层组合不一致。
- AI 生成与 AI 编辑分类不一致。
- 平台签名和自声明容易混淆。
- 检测接口不可互操作。
- 元数据被剥离后难以恢复来源。
- 法规字段、行业字段和平台字段缺少稳定适配层。

### 2.3 HiddenShield 的可用基础

当前项目已经具备：

- `watermark-core` 图片鲁棒锚点写入和读取能力。
- V3 / 39-byte 最小媒体锚点。
- Watermark ID 和 registry。
- Rights Manifest 和公开权利查询。
- 图片 C2PA / XMP / IPTC / JSON-LD 传播原型。
- 公开只读 SDK、批量 API 和公开元数据导出。
- 桌面与移动端图片跨端互验约束。

因此 MVP 不应重新发明媒体身份基础设施，而应在现有锚点、Registry 和公开传播层上增加独立的 AI 来源声明与 Evidence 对象。

## 3. 目标用户与最小切口

### 3.1 第一目标用户

- AI 图片生成平台。
- 企业内部文生图服务。
- AI 图片编辑和营销素材生成平台。
- 向中国、欧盟或加州用户提供图片生成能力的平台。
- 需要向下游内容平台提供来源证明的模型应用服务商。

### 3.2 MVP 使用场景

```text
平台创建生成任务
-> 生成或编辑图片
-> 调用 HiddenShield 标识 SDK
-> 写入标准元数据和鲁棒锚点
-> 创建 AI Transparency Manifest 与 Evidence
-> 返回显式标签数据和验证入口
-> 平台导出最终图片
```

### 3.3 本阶段不覆盖

- 对任意未知素材进行概率型 AI 检测。
- 对没有标识的图片断言“不是 AI 生成”。
- 文本水印。
- 音频、视频或虚拟场景正式平台接入。
- 通用深度伪造检测。
- 版权归属自动判断。
- 法律合规自动结论。
- 生产 C2PA 公共信任链已经上线的承诺。
- 要求其他平台采用 HiddenShield 专有元数据作为唯一标准。

## 4. 产品原则

### 4.1 生成时事实优先于事后猜测

MVP 只处理平台在生成或编辑链路中已经知道的事实：

- 哪个平台执行了生成。
- 使用了哪个系统或模型。
- 何时生成。
- 是完整生成还是局部编辑。
- 输出对应哪个内容编号和摘要。

HiddenShield 负责记录、签署、传播和恢复这些事实，不负责从像素概率推断未知来源。

### 4.2 Claim 与 Evidence 分离

Claim 表达：

- 图片被声明为 AI 生成或 AI 编辑。
- 生成方式和操作类型。
- 平台、系统和模型信息。

Evidence 表达：

- 谁提供了该声明。
- 使用哪把 key 签名。
- Evidence 等级。
- 签名和验证结果。
- Evidence 绑定的图片摘要。

平台签名只证明平台签署了该声明，不自动证明现实世界中的内容真实、权属成立或不存在其他编辑。

### 4.3 开放标准优先

公开传播优先使用：

- C2PA / Content Credentials。
- XMP。
- IPTC。
- JSON-LD。
- 适用地区要求的文件元数据。

HiddenShield 专有字段只作为公开标准尚未覆盖部分的命名空间扩展，不替代公开标准。

### 4.4 多层标识而非单点依赖

MVP 默认采用：

```text
Layer 1：数字签名元数据
Layer 2：watermark-core 鲁棒锚点
Layer 3：Registry 在线状态与 Evidence
Layer 4：平台可感知标签数据
```

元数据负责开放互操作，鲁棒锚点负责元数据丢失后的身份恢复，Registry 负责当前状态、撤销、替代和完整声明。

## 5. 系统架构

```text
AIGC Platform
  ├── Generation Session
  ├── Generated Image Bytes
  └── Platform Evidence
          |
          v
HiddenShield AI Transparency SDK
  ├── Compliance Profile Resolver
  ├── Explicit Label Data Builder
  ├── Standard Metadata Builder
  ├── watermark-core Adapter
  └── Manifest / Evidence Client
          |
          +--> C2PA / XMP / IPTC / JSON-LD
          +--> V3 / 39-byte Opaque Anchor
          +--> AI Transparency Manifest
          +--> Evidence
          +--> Registry / Resolver
```

职责边界：

| 层 | 负责 | 不负责 |
| --- | --- | --- |
| `watermark-core` | 图片盲水印写入、读取、认证和正式跨端互验 | AI 语义、法规判断、平台信任判断 |
| AI Transparency SDK | 生成会话、Profile 解析、标识编排、Manifest / Evidence 提交 | 自行实现第二套盲水印算法 |
| Registry | Watermark ID、Manifest、Evidence、状态、撤销和查询 | 从像素概率判断 AI 来源 |
| Metadata Adapter | C2PA / XMP / IPTC / JSON-LD 和地区字段映射 | 成为事实源 |
| Explicit Label Adapter | 返回平台 UI 或导出层需要的可感知标签数据 | 强制修改平台产品 UI |
| Resolver | 合并元数据、水印、Registry 和 Evidence 结果 | 输出法律结论 |

## 6. 媒体锚点设计

### 6.1 不修改 V3 / 39-byte payload

正式图片锚点继续只包含稳定、不透明、可认证的身份信息。

禁止加入：

- AI flag。
- provider。
- model ID。
- 法规 Profile。
- Evidence 等级。
- 权利或撤销状态。

原因：

- AI 声明可能被修正、替代或撤销。
- `AI=0` 无法区分人工创作、未声明、旧版本和证据缺失。
- Provider、模型和法规 Profile 会持续变化。
- payload 扩容会损害容量、性能、鲁棒性和跨端兼容。

### 6.2 Registry 外层用途

Watermark ID 继续保持无业务语义。Registry 记录增加外层用途：

```text
recordPurpose:
  - copyright_protection
  - ai_transparency
  - combined
```

`recordPurpose` 不进入 Watermark ID 或 V3 payload。

### 6.3 正式端点约束

任何 AI 图片平台正式写入端点都必须：

- 调用 `watermark-core`。
- 使用与桌面、Android、iOS 和后端正式读取端兼容的 V3 锚点。
- 通过 write-after-read。
- 通过平台写入、桌面读取和移动端读取 fixture。
- 不在 SDK、后端或平台插件中复制图片盲水印算法。

## 7. 领域模型

### 7.1 AI Transparency Manifest

建议独立对象：

```text
ai_transparency_manifests
```

最小字段：

| 字段 | 含义 |
| --- | --- |
| `transparencyManifestId` | AI 来源 Manifest ID |
| `watermarkUid` | 绑定的不透明锚点 |
| `manifestVersion` | Manifest 版本 |
| `status` | `active` / `superseded` / `revoked` / `disputed` |
| `claimType` | `ai_generated` / `ai_manipulated` |
| `modality` | MVP 固定为 `image` |
| `generationMode` | 文生图、图生图、局部编辑等 |
| `providerId` | 平台或企业 Provider ID |
| `systemName` | AI 系统名称 |
| `systemVersion` | AI 系统版本 |
| `modelId` | 可选模型标识 |
| `modelVersion` | 可选模型版本 |
| `operations` | AI 编辑操作列表 |
| `generatedAt` | 生成或编辑时间 |
| `contentIdentifier` | 平台内容编号 |
| `subjectDigest` | 最终图片精确字节摘要 |
| `parentSubjects` | 原始素材或 ingredient 引用 |
| `complianceProfiles` | 本次输出采用的 Profile |
| `manifestSha256` | 规范化 Manifest 摘要 |
| `createdAt` | 创建时间 |
| `updatedAt` | 更新时间 |

### 7.2 Evidence

建议独立对象：

```text
ai_claim_evidence
```

最小字段：

| 字段 | 含义 |
| --- | --- |
| `evidenceId` | Evidence ID |
| `transparencyManifestId` | 绑定的 Manifest |
| `evidenceLevel` | Evidence 等级 |
| `evidenceSource` | 用户、设备、Registry、平台或外部验证方 |
| `issuerId` | 签发者 |
| `keyId` | 签名 key |
| `proofType` | 数字签名、设备证明、平台回执等 |
| `subjectDigest` | Evidence 绑定的最终图片摘要 |
| `signature` | 签名值 |
| `signatureAlgorithm` | 签名算法 |
| `verificationStatus` | 验证状态 |
| `verifiedAt` | 验证时间 |
| `failureCode` | 不支持或无效时的错误码 |
| `createdAt` | 创建时间 |

冻结 Evidence 等级：

```text
self_declared
device_signed
registry_signed
platform_signed
externally_verified
unsupported_proof
invalid_proof
```

### 7.3 Marker Binding

建议独立对象：

```text
marker_bindings
```

最小字段：

| 字段 | 含义 |
| --- | --- |
| `markerBindingId` | 标识绑定 ID |
| `transparencyManifestId` | 绑定 Manifest |
| `watermarkUid` | 鲁棒锚点 |
| `markerType` | `c2pa` / `xmp` / `iptc` / `json_ld` / `blind_watermark` / `explicit_label` |
| `markerProfile` | 标准或法规 Profile |
| `markerVersion` | 标识版本 |
| `detectorScheme` | 检测器类型 |
| `detectorEndpoint` | 可选检测入口 |
| `signpost` | 可公开读取的 detector signpost |
| `embedStatus` | 嵌入状态 |
| `verifyStatus` | 写后验证状态 |
| `createdAt` | 创建时间 |

## 8. Compliance Profile

### 8.1 Profile 原则

法规 Profile 是版本化技术映射，不是法律意见。

每个 Profile 必须记录：

- `profileId`。
- 版本。
- 生效时间。
- 适用主体。
- 适用媒体。
- 显式标识要求。
- 隐式标识字段。
- 签名要求。
- 水印要求。
- 检测接口要求。
- 日志要求。
- 已知限制。
- 法务审查状态。

### 8.2 MVP Profile

第一阶段冻结以下 Profile 名称：

```text
cn_aigc_label_2025_image_v1
eu_ai_act_article_50_2026_image_v1
eu_transparency_code_2026_image_v1
ca_ai_transparency_2026_image_v1
c2pa_ai_output_2_4_image_v1
```

Profile 可以组合使用。SDK 应返回本次输出实际成功应用的 Profile 和未满足项，不得只返回一个笼统的 `compliant=true`。

建议结果：

```text
profileStatus:
  - applied
  - partially_applied
  - not_applicable
  - configuration_required
  - failed
```

禁止结果：

```text
fully_legally_compliant
globally_compliant
```

### 8.3 法规 Profile 与技术 Profile 分离

下列对象不能混为同一种 Profile：

| 类型 | 作用 | MVP 示例 |
| --- | --- | --- |
| `regulatory_profile` | 定义某地区、主体、媒体和流转环节的技术控制清单 | `cn_aigc_label_2025_image_export_v1` |
| `technical_profile` | 定义可互操作的元数据、签名、水印和检测实现 | `c2pa_ai_output_2_4_image_v1` |
| `commercial_entitlement` | 定义某租户获授权使用哪些能力 | `profile:cn_image_export` |

`c2pa_ai_output_2_4_image_v1` 是技术 Profile，不是法规合规 Profile。一个输出可以同时引用多个法规 Profile 和多个技术 Profile；Resolver 必须逐项返回 `applied`、`partially_applied`、`not_applicable`、`configuration_required` 或 `failed`。

每个法规 Profile 还必须绑定：

- `jurisdiction`、`effectiveFrom`、`reviewedAt` 和法规 / 标准来源。
- `actorRole`，例如 `generator_provider`、`content_platform`、`deployer`。
- `distributionMode`，例如 `in_product_view`、`download_export`、`api_delivery`。
- 必需控制、可选控制、例外条件和人工法务复核状态。
- Profile owner、变更日志和废弃策略。

## 9. 显式与隐式标识

### 9.1 显式标识

MVP 必须同时支持平台界面标签和文件导出标签，不能只返回展示建议。

SDK 返回的结构化标签数据：

```text
labelType
labelText
locale
placementHint
accessibilityText
profileId
```

中文默认建议：

```text
AI 生成
AI 辅助编辑
```

英文默认建议：

```text
AI-generated
AI-manipulated
```

对于要求显式标识的法规 Profile，SDK 还必须生成并验证 `explicitLabelReceipt`：

```text
requiredSurface
renderMode
renderedAssetDigest
placement
locale
appliedAt
appliedBy
verificationStatus
```

`requiredSurface` 至少区分：

- `platform_ui`：平台展示页或交互界面。
- `exported_file`：用户下载、复制或导出的图片文件。
- `both`：界面和文件都必须有显式标识。

中国图片导出 Profile 默认使用 `both`，并要求平台在导出副本中真正写入可感知标识，或记录法规允许的例外、用户协议和留痕；不能只在前端 UI 显示一个标签后仍导出无标识文件。

平台可自定义视觉样式，但不能绕过 Profile 的必需表面、文本语义、可访问性和导出验证。SDK 未取得必要回执时必须返回 `partially_applied` 或 `failed`，不得确认该 Profile 已应用。

### 9.2 隐式标识

默认组合：

- C2PA signed manifest。
- XMP / IPTC / JSON-LD 兼容映射。
- 适用地区要求的文件元数据。
- `watermark-core` V3 鲁棒锚点。
- Detector signpost。

### 9.3 元数据保留

SDK 和平台接入文档必须：

- 禁止无正当原因删除已有标准来源元数据。
- 避免覆盖第三方已有 C2PA Manifest。
- 对合法转换建立 ingredient / action 关系。
- 检测元数据丢失时通过鲁棒锚点和 Registry 恢复来源。

## 10. SDK 与 API 合同

### 10.1 SDK

建议新增独立包：

```text
packages/ai-transparency-sdk
```

MVP 接口：

```text
createGenerationSession()
markGeneratedImage()
buildExplicitLabel()
confirmGeneratedAsset()
verifyGeneratedImage()
resolveTransparencyClaim()
revokeTransparencyClaim()
supersedeTransparencyClaim()
```

SDK 只负责编排和调用，不实现自己的盲水印算法。

### 10.2 服务端 API

建议端点：

```text
POST /v1/ai-transparency/sessions
POST /v1/ai-transparency/images/mark
POST /v1/ai-transparency/images/confirm
POST /v1/ai-transparency/images/verify
GET  /v1/ai-transparency/{watermarkUid}
POST /v1/ai-transparency/batch-verify
POST /v1/ai-transparency/{manifestId}/supersede
POST /v1/ai-transparency/{manifestId}/revoke
```

### 10.3 生成时流程

```text
1. 平台创建 generation session。
2. HiddenShield reserve Watermark ID。
3. 平台生成最终图片字节。
4. SDK 构造 Claim、Evidence 和 Compliance Profile。
5. SDK 调用 watermark-core 写入 V3 锚点。
6. SDK 写入数字签名元数据。
7. SDK执行 write-after-read 和元数据回读。
8. 后端 confirm 最终摘要、Manifest、Evidence 和 Marker Binding。
9. 返回最终图片、显式标签数据、验证 URL 和 Profile 状态。
```

### 10.4 授权与计量合同

SDK 授权必须由服务端控制，不能把生产长期 secret、Profile 权限或签发私钥放入浏览器、桌面包或可分发的客户端 SDK。

每个 production tenant 必须有：

```text
licenseId
tenantId
workspaceId
issuerMode
allowedMedia
allowedRegulatoryProfiles
allowedTechnicalProfiles
allowedDeploymentModes
meteringPlan
supportTier
effectiveAt
expiresAt
```

冻结 `issuerMode`：

- `platform_managed`：平台使用自身已受信任的签发链。
- `hiddenshield_managed`：HiddenShield 代管受限签发能力。
- `customer_byok`：客户自带 key，HiddenShield 只完成编排、审计和验证。

SDK / API scope 最小集合：

```text
mark:image
profile:cn_image_export
profile:eu_image_provider
profile:ca_image_provider
verify:public
verify:batch
issuer:platform
deployment:private
```

计量单位固定为 `confirmed_marked_image`：只在最终图片完成 watermark、元数据写入、必要显式标识回执和后端 confirm 后记一次。失败、重试、重复 confirm、免费公共验证和内部 write-after-read 不重复收费。

授权失效、额度耗尽或 Profile 未获授权时，生产调用默认 `fail_closed`：不返回被标记为已应用的输出。客户可配置人工队列或明确降级流程，但降级结果必须记录为 `configuration_required` 或 `failed`，不得静默导出为“已满足标识要求”。

### 10.5 免费公共验证与付费 API 的边界

SDK 可以是授权付费组件，但不得把法规或平台承诺所需的普通用户验证能力变成按次付费墙。

冻结边界：

- `verify:public`：由已接入的平台向普通用户提供的基础单文件验证，必须可用、低摩擦、无按次收费；加州适用客户必须自行确保其免费检测工具义务得到满足。
- `verify:batch`：企业批量验证、审计导出、SLA、Webhook、长保留日志和高并发可收费。
- `mark:image`：平台生成时标识是主要可计费动作。
- 高级法规 Profile、私有部署、BYOK / HSM、专属 issuer、地域数据驻留和支持 SLA 是独立可计费能力。

HiddenShield 提供工具和证据，不替代平台对其所在地法律义务的最终责任。

## 11. 统一验证结果

验证结果必须分离：

```text
anchorIntegrity
metadataSignature
watermarkDetection
issuerTrust
claimStatus
evidenceLevel
registryStatus
profileStatus
legalConclusion
```

建议最小结果：

| 字段 | 含义 |
| --- | --- |
| `watermarkUid` | 检出的锚点 |
| `markerStatus` | 是否发现支持的标识 |
| `metadataSignatureStatus` | 元数据签名状态 |
| `watermarkDetectionStatus` | 鲁棒锚点检测状态 |
| `issuerTrustStatus` | issuer/key 信任状态 |
| `claimType` | AI 生成或 AI 编辑声明 |
| `evidenceLevel` | 声明证据等级 |
| `provider` | 声明平台 |
| `system` | AI 系统和版本 |
| `generatedAt` | 声明生成时间 |
| `manifestStatus` | active / superseded / revoked / disputed |
| `complianceProfiles` | Profile 结果 |
| `warnings` | 冲突和限制 |
| `legalConclusion` | 固定为 `false` |

用户可见文案：

- “发现平台签署的 AI 生成声明，签名验证通过。”
- “发现 HiddenShield 鲁棒标识，并恢复到有效的 AI 来源声明。”
- “仅发现用户自声明，没有平台签名证据。”
- “发现 AI 来源标识，但签名无效或签发者不受信任。”
- “未发现支持的 AI 来源标识；这不等于该内容不是 AI 生成。”

禁止文案：

- “系统已检测该图片一定由 AI 生成。”
- “未检测到标识，因此该图片由人工创作。”
- “该图片符合全球所有 AI 法规。”
- “该声明证明内容真实或版权归平台所有。”

## 12. 桌面与移动端边界

MVP 的主交付面是平台 SDK、后端 API 和验证接口，不是桌面或移动端生成工具。

桌面与移动端后续只允许增加同构的只读查看能力：

- 显示 AI 来源声明。
- 显示 Evidence 等级。
- 显示签名、锚点和 Registry 状态。
- 显示 Profile 结果和限制。
- 使用相同枚举和用户文案。

任何正式 AI 图片写入能力必须同时满足：

- 平台写入、桌面读取。
- 平台写入、Android 读取。
- 平台写入、iOS 读取。
- 桌面现有正式图片读取不受影响。

Android QA 不能替代 iOS QA。

## 13. 隐私、安全与运营

### 13.1 数据最小化

默认保存：

- 内容摘要。
- Watermark ID。
- 平台内容编号。
- Claim、Evidence 和签名。
- 必要审计事件。

默认不保存：

- 用户 prompt。
- 未经约定的原始生成图片。
- 可直接识别终端用户的个人信息。
- 与来源验证无关的模型输入。

### 13.2 检测上传

验证服务必须明确：

- 上传文件是否仅在内存处理。
- 临时文件保留时间。
- 是否记录摘要。
- 是否允许客户选择私有部署。
- 删除失败和异常日志策略。

### 13.3 密钥与签名

平台签名生产化必须具备：

- Issuer Document。
- Key Document。
- KMS / HSM 或等效密钥保护。
- key usage 分离。
- 密钥轮换。
- verify-only 历史 key。
- Evidence、Manifest 和 C2PA 的独立签名用途。
- 撤销和历史验证。

当前 mock signature 和 ephemeral development certificate 不能进入正式平台试点结论。

## 14. Benchmark 与发布门禁

### 14.1 功能 Gate

- Claim、Evidence、Marker Binding 可以独立版本化。
- Manifest 替代和撤销可查询。
- C2PA / XMP / IPTC / JSON-LD 回读通过。
- write-after-read 通过。
- Detector signpost 可被独立客户端解析。
- 所有验证结果保持 `legalConclusion=false`。

### 14.2 图片鲁棒性 Gate

至少覆盖：

- PNG。
- JPEG。
- WebP。
- JPEG 重压缩。
- resize。
- 常见裁剪。
- 轻度滤镜。
- 元数据完整。
- 元数据剥离。
- 元数据与 Registry 冲突。
- C2PA Manifest 被替换。
- 无标识对照样本。

每项必须记录：

- 提取成功率。
- 误报率。
- 置信度。
- 感知质量。
- 处理延迟。
- 输出文件增量。

### 14.3 跨端 Gate

- 平台 SDK 写入、桌面读取。
- 平台 SDK 写入、Android 读取。
- 平台 SDK 写入、iOS 读取。
- 后端读取结果与三端一致。
- C2PA 第三方工具验签。
- 公开 Resolver 不依赖 HiddenShield 私有数据库结构。

### 14.4 平台 Gate

至少一个真实 AIGC 图片平台完成：

- 真实生成链路接入。
- 真实吞吐和延迟测试。
- 二次编码测试。
- 错误和重试测试。
- 撤销和替代测试。
- 数据保留审查。
- 客户签字验收。

在该 Gate 通过前，不得宣称“已成为 AIGC 平台基础能力”。

## 15. 分阶段实施

### Phase A：法规、商业与合同冻结

状态：`completed`

范围：

- Schema。
- Profile。
- SDK / API 合同。
- 统一验证结果。
- 隐私边界。
- Benchmark。

该阶段已完成，并形成数据库/API Schema、Profile、授权、审批、provider receipt、签发、交付和安全治理合同。

### Phase B：生产控制面与内部图片闭环

状态：`implemented_through_postgresql_0018_internal_gate_passed`

已完成范围：

- PostgreSQL `0002–0018` additive migration、up/down smoke 和真实并发 QA。
- License、versioned Profile entitlement、生产 credential custody 和 marking session。
- `watermark-core` 图片 V3 写入、写后回读、confirm、计量和 append-only audit。
- post-embed C2PA 重新签发、reservation/lease、artifact finalize 和 recovery/dead-letter。
- delivery envelope、authorization/retrieval/revoke、资源预算和端侧 import admission。
- observability、incident、四眼 ack/resolve、cleanup runner、inspect/list、notification outbox 与 provider-neutral delivery Gate。
- `packages/ai-transparency-sdk` server-side package、framework-neutral API facade、严格摘要/计量 receipt 校验和 fail-closed 错误模型。
- 内部 fixture、contract、跨端读取和第三方分层样本 Gate。

已完成的 Phase B 发布出口：

- HiddenShield PostgreSQL 后端的 admission / session / mark / confirm 四个 internal platform endpoint，并通过 SDK → facade → HTTP → Axum → PostgreSQL 端到端 QA。
- 免费公共 Resolver 的最小只读接口；仅查询 confirmed 记录，保持匿名、无计量、最小公共字段和 `legalConclusion=false`。

仍未完成且作为外部依赖挂起的 Phase B 发布出口：

- 生产 provider Secret 注入和真实恢复演练。

在真实 provider Secret 注入和恢复演练完成前，平台能力保持 `只能内部测试`；已通过的 internal API 和 Resolver Gate 不构成公网部署、SDK 发布、production credential 发放或客户 SLA。

### Phase C：平台接入与设计伙伴试点

状态：`not_started`

范围：

- 接入一个真实 AI 图片平台并使用正式 SDK/API facade。
- 完成真实生成链路 Benchmark。
- 冻结平台回执和 Evidence。
- 验证 Profile 输出。
- 完成私有部署和数据边界评审。
- 取得设计伙伴对授权、计量、延迟、错误和支持边界的书面验收。

### Phase D：真实 Provider 激活、生产信任与公开验证

状态：`blocked_by_external_environment_and_configuration`

范围：

- 真实 Internal IAM、工作负载身份和审批 reference provider。
- 生产证书链。
- KMS / HSM。
- production signer、object-store 和通知 provider。
- Issuer / Key 文档。
- 公共验签。
- 撤销。
- Detector API。
- SLA、审计和安全评估。

只有完成 Phase D 的对应 Gate，相关能力才可以进入 `可对用户承诺` 评审。

## 16. 商业化边界

### 16.1 产品线边界

AI Transparency SDK 是独立的 B2B 授权产品线，不复用当前桌面端“未付费 / 图片音频年费”的用户权益，也不在桌面产品内显示企业 SKU。

销售对象是生成服务提供商、企业 AIGC 工作流和模型应用平台，而不是普通创作者。

### 16.2 商业模型

收入模型采用“年度平台授权 + 已确认标识量 + 高级合规与信任服务”，不按验证失败、SDK 重试或普通用户基础验证收费。

| 收费层 | 收费对象 | 计费依据 | 包含能力 |
| --- | --- | --- | --- |
| Evaluation License | 设计伙伴 / 集成测试客户 | 固定期限和非生产限额 | Sandbox、测试 Profile、测试 issuer、无生产承诺 |
| Production Platform License | 每个生产 tenant / workspace | 年度基础授权 | 一个生产环境、图片标识、基础 Registry、公共单文件验证、标准技术 Profile |
| Marking Volume Pack | 平台 | `confirmed_marked_image` 阶梯量 | 超出包含量的生产图片标识 |
| Regulatory Profile Pack | 平台 / 区域 | 每个生产 tenant 的年度 Profile 授权 | 中国导出、欧盟提供者、加州提供者等经法务冻结的 Profile 与更新 |
| Trust & Deployment Pack | 企业 | 年度附加项 | BYOK、HSM、私有部署、地域驻留、专属 issuer、审计保留、SLA |
| Enterprise Verification Pack | 企业使用方 | 批量量级和 SLA | 批量验证、Webhook、审计导出、高并发和支持 |

价格暂不冻结。价格发现必须通过 3 家设计伙伴的月输出量、延迟预算、合规风险、私有部署需求和采购流程确定，不能从当前桌面订阅价格外推。

### 16.3 付费能力反推的技术要求

为了使授权产品可销售，MVP 必须具备：

- production / sandbox 隔离。
- tenant、workspace、issuer、Profile 和 deployment mode 的授权绑定。
- API key、短期 token、scope、轮换、撤销和审计。
- 幂等会话和唯一计量 ledger。
- 配额预检、额度耗尽策略和账单导出。
- Profile 版本、变更通知和客户确认记录。
- 公共基础验证与收费企业验证的隔离。
- 客户自带 key、HiddenShield 代管 key 和平台签名三种 issuer 模式。
- 数据驻留、媒体不落盘默认策略和私有部署边界。
- 每个 `confirmed_marked_image` 的可审计收据。

当前后端已有 Enterprise API key、scope、quota、ledger 和 audit 的内部基础，但这些资产尚未构成可售 AI Transparency SDK；后续实现必须新增独立 scope、计量类型、账单归属和授权合同，不能借用当前公开权利查询的免费 / 内部语义。

### 16.4 当前商业承诺限制

当前不能宣称：

- 已获得平台客户或设计伙伴。
- 已符合全部中国、美国或欧盟法规。
- 已形成生产 C2PA 信任链或平台签名 Evidence。
- 已提供通用 AI 检测。
- 已提供付费 AI Transparency SDK。
- 已进入当前图片 / 音频年费权益。

## 17. 回滚与停止条件

满足以下任一条件时暂停实现：

- 没有真实平台愿意提供生成链路试点。
- 平台只需要普通 C2PA 元数据，不需要鲁棒锚点或 Registry。
- 图片写入延迟和成本无法满足平台要求。
- 鲁棒水印与其他平台标识发生不可接受冲突。
- 生产签名和密钥运营成本超过已验证客户价值。
- 为满足 MVP 被迫修改 V3 / 39-byte payload。
- 需要绕过 `watermark-core` 实现第二套图片水印算法。

回滚不删除历史锚点和签发事实，只停止新建 AI Transparency Manifest，并保留历史验证和撤销查询。

## 18. 当前能力边界

截至 2026-07-28：

- `可对用户承诺`：无新增。
- `只能内部测试`：生产导向 PostgreSQL 控制面、图片 marking executor、post-embed signing/recovery、delivery、observability、incident、provider-neutral outbox、notification delivery 与 server-side SDK/API facade Gate 已通过内部 fixture、contract、SDK tests 和真实 PostgreSQL QA。
- `明确不能承诺`：任意图片 AI 检测、全球法规合规、真实生产 provider 已接入、平台签名 Evidence 已上线、生产 C2PA 信任链已上线、付费 SDK/公共 Resolver 已开放或已嵌入大型 AIGC 平台。

## 19. 2026-07-27 法规与商业审计结论

审计结论：`conditional_design_pass`

本设计可以作为“中、美、欧三地可配置技术控制”的 MVP 基础，但不能作为不经地区、主体、流转环节和法务确认的统一合规保证。

本次关闭的 P0 缺口：

- 中国图片导出显式标识从“标签建议”升级为 Profile 强制的文件 / UI 回执和验证。
- 法规 Profile、技术 Profile 和商业授权拆分，避免把 C2PA 误称为法规合规结论。
- 加州适用客户的免费基础检测边界与 SDK 付费标识、收费企业批量验证分离。
- SDK 授权、issuer mode、scope、计量单位、额度耗尽和 fail-closed 行为进入 MVP 合同。

仍然阻塞生产承诺的 P1 项：

- 每个目标地区 Profile 的外部法务审查和持续更新责任。
- 生产 C2PA 签发链、issuer/key、KMS / HSM、撤销和独立验签。
- 与至少一个真实平台的格式、延迟、吞吐、数据驻留和导出标签联调。
- 与第三方来源元数据、水印和内容平台处理链的互操作 Benchmark。
- SDK 授权协议、计量口径、支持边界和数据处理协议。

## 20. 下一任务

当前生产导向控制面已实现至：

```text
0020_ai_transparency_public_resolver
```

已完成：

```text
packages/ai-transparency-sdk
+ framework-neutral platform API facade
+ production admission/session/mark/confirm 合同
+ confirmed_marked_image receipt 与 fail-closed 错误模型
+ PostgreSQL-backed admission/session/mark/confirm internal endpoint
+ SDK → HTTP → Axum → PostgreSQL 真实端到端 QA
+ anonymous public Resolver
+ confirmed Manifest/marker/Profile 最小公共 PostgreSQL views
+ zero-auth / zero-write / zero-metering QA
```

已新增并冻结：

```text
packages/ai-transparency-design-partner-kit
+ hs-ai-design-partner-sandbox-kit-v1
+ onboarding 与 Profile mapping questionnaire
+ server-only SDK/API sample
+ anonymous Resolver link contract
+ 12 场景验收矩阵与 preflight
```

当前状态为 `design_partner_sandbox_kit_implemented_external_partner_configuration_required`。未注入真实伙伴身份引用、Sandbox endpoint、`secret://` credential 引用和伙伴运行证据时，模板必须保持 `valid=true + readiness=configuration_required`。

已补齐内部 synthetic Sandbox QA：它复用 SDK/facade 与最小 Resolver contract 演练 12 个场景，输出固定为 `synthetic_non_acceptance + not_real_partner_acceptance + configuration_required`，不构成真实伙伴、生产、法律、SLA 或计费证据。

CI 已新增独立 `AI Transparency contract gate`，固定运行 `npm run ai-transparency:ci`，将 SDK contract/test、接入包 contract/test、template preflight 和 synthetic Sandbox QA 设为必跑回归；GitHub active ruleset 已对 `main` / `master` 将其设为 required check，待工作流合并后的首个 PR 报告该 check。

下一工程任务：

```text
取得首个真实设计伙伴的外部配置后生成专属 bundle，
完成 12 场景真实 Sandbox 验收并归档不可变 evidence；
SDK 发布、production credential 和公网平台 API 继续关闭。
```
