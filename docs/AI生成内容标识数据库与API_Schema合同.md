# HiddenShield AI 生成内容标识数据库与 API Schema 合同

更新时间：2026-07-27

合同版本：`ai_transparency_schema_v1`

状态：`schema_and_fixture_frozen_no_implementation`

实现状态：`Schema、JSON fixture、PostgreSQL / SQLite 存储迁移、数据库约束 smoke 与内部只读 license / Profile 管理已完成；未新增 SDK、未新增计费、未新增公共 Resolver`

关联设计：`docs/AI生成内容标识基础设施MVP设计.md`

## 1. 合同目的与边界

本合同冻结 AI 图片平台生成时标识 MVP 的逻辑数据库模型和 HTTP API 表面，用于后续迁移、fixture、contract test 和 SDK 实现。

本合同不：

- 修改 `watermark-core`、V3 / 39-byte payload、Watermark ID 或 auth tag。
- 实现媒体写入、C2PA 签名、显式标签渲染、Detector 或 SDK。
- 创建真实支付、价格、扣款、发票或客户合同。
- 声明任何地区法规已经满足。

所有实体都以 tenant / workspace 为边界。`watermarkUid` 仍是不透明媒体锚点；法规语义、商业授权、Evidence 和计量均不写入媒体 payload。

## 2. 规范约定

### 2.1 标识符

| 名称 | 格式 |
| --- | --- |
| `licenseId` | `atl_` 前缀的不透明 ID |
| `markingSessionId` | `ats_` 前缀的不透明 ID |
| `transparencyManifestId` | `atm_` 前缀的不透明 ID |
| `evidenceId` | `ate_` 前缀的不透明 ID |
| `markerBindingId` | `atb_` 前缀的不透明 ID |
| `ledgerEntryId` | `atlgr_` 前缀的不透明 ID |

不得从 ID 推导客户名称、地区、套餐、法规 Profile、模型或媒体语义。

### 2.2 时间与摘要

- 所有 API 时间字段使用 RFC 3339 UTC。
- 最终图片使用 `binary_sha256`，并记录 `subjectDigestScope=protected_output`。
- 后续如加入 canonical / perceptual digest，必须新增字段和 Profile 版本，不得改变本合同的精确字节语义。

### 2.3 枚举

```text
environment:
  sandbox | production

licenseStatus:
  active | suspended | expired | revoked

issuerMode:
  hiddenshield_managed | platform_managed | customer_byok

entitlementStatus:
  active | suspended | expired | revoked

markingSessionStatus:
  reserved | processing | ready_to_confirm | confirmed | failed | cancelled | expired

manifestStatus:
  active | superseded | revoked | disputed

claimType:
  ai_generated | ai_manipulated

evidenceLevel:
  self_declared | device_signed | registry_signed | platform_signed |
  externally_verified | unsupported_proof | invalid_proof

markerType:
  c2pa | xmp | iptc | json_ld | blind_watermark | explicit_label

profileStatus:
  applied | partially_applied | not_applicable |
  configuration_required | failed

ledgerStatus:
  pending | committed | reversed | no_charge
```

除上述枚举外的值必须被拒绝，不能静默降级为默认值。

## 3. 授权与 Profile 逻辑模型

### 3.1 `ai_transparency_licenses`

一条 license 是一个生产或 sandbox tenant / workspace 在指定期限内使用 AI Transparency SDK 的授权根。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `license_id` | text | 主键，`atl_` 前缀 |
| `tenant_id` | text | 非空 |
| `workspace_id` | text | 非空 |
| `environment` | enum | `sandbox` 或 `production` |
| `status` | enum | `licenseStatus` |
| `issuer_mode` | enum | `issuerMode` |
| `deployment_mode` | text | `hosted` / `private`，本期冻结枚举但不实现 |
| `public_verification_required` | boolean | production 必须为 `true` |
| `metering_plan_id` | text | 非空、不含价格 |
| `effective_at` | timestamptz | 非空 |
| `expires_at` | timestamptz | 非空 |
| `created_at` | timestamptz | 非空 |
| `updated_at` | timestamptz | 非空 |

唯一约束：

```text
tenant_id + workspace_id + environment + active status
```

同一 production workspace 不允许同时存在两条 active license。续约必须更新有效期或创建明确的替代关系，不能依赖“选择最新一条”的隐式行为。

### 3.2 `ai_profile_entitlements`

Profile 授权独立于 license，避免将法规 / 技术 Profile 直接硬编码进套餐名称。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `license_id` | text | 外键指向 license |
| `profile_id` | text | 非空、版本化 |
| `profile_kind` | enum | `regulatory` / `technical` |
| `status` | enum | `entitlementStatus` |
| `effective_at` | timestamptz | 非空 |
| `expires_at` | timestamptz | 非空 |
| `terms_version` | text | 非空 |
| `approved_by` | text | 非空，记录授权主体 |
| `created_at` | timestamptz | 非空 |
| `updated_at` | timestamptz | 非空 |

唯一约束：

```text
license_id + profile_id
```

最小 `profile_id`：

```text
cn_aigc_label_2025_image_export_v1
eu_ai_act_article_50_2026_image_v1
ca_ai_transparency_2026_image_v1
c2pa_ai_output_2_4_image_v1
```

本表只表示客户获准使用某一 Profile，不表示该输出或客户天然满足法规。

### 3.3 凭据绑定

现有 Enterprise API key 可以作为底层凭据存储基础，但 AI Transparency 不能复用公开权利查询的 scope、quota 或计量语义。

后续实现新增：

```text
ai_sdk_credential_bindings
```

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `credential_id` | text | 主键 |
| `license_id` | text | 非空 |
| `api_key_id` | text | 绑定现有或新 API key |
| `scopes` | json/text array | 仅 AI Transparency scope |
| `status` | text | active / suspended / revoked |
| `expires_at` | timestamptz | 可空 |
| `created_at` | timestamptz | 非空 |

冻结 scope：

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

## 4. 生成时标识逻辑模型

### 4.1 `ai_marking_sessions`

会话绑定授权、幂等键和生成时工作流；不保存原始图片或 prompt。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `marking_session_id` | text | 主键，`ats_` 前缀 |
| `license_id` | text | 非空 |
| `tenant_id` | text | 非空，必须匹配 license |
| `workspace_id` | text | 非空，必须匹配 license |
| `environment` | enum | 必须匹配 license |
| `idempotency_key` | text | 非空 |
| `requested_profile_ids` | json/text array | 非空，至少一个 Profile |
| `claim_type` | enum | `claimType` |
| `provider_content_id` | text | 可空，客户侧内容编号 |
| `status` | enum | `markingSessionStatus` |
| `expires_at` | timestamptz | 非空 |
| `confirmed_at` | timestamptz | 可空 |
| `created_at` | timestamptz | 非空 |
| `updated_at` | timestamptz | 非空 |

唯一约束：

```text
license_id + idempotency_key
```

相同幂等键且请求摘要不同，必须返回 `idempotency_conflict`，不得创建第二次标识或第二笔计量。

### 4.2 `ai_transparency_manifests`

该表与既有 `rights_manifests` 平行。Rights Manifest 继续负责训练许可和权利声明，不能承载 AI 来源 Evidence。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `transparency_manifest_id` | text | 主键，`atm_` 前缀 |
| `marking_session_id` | text | 唯一，已确认会话 |
| `watermark_uid` | text | 非空，不透明锚点 |
| `manifest_version` | integer | 从 1 起递增 |
| `status` | enum | `manifestStatus` |
| `claim_type` | enum | `claimType` |
| `modality` | text | V1 固定 `image` |
| `generation_mode` | text | 非空 |
| `provider_id` | text | 非空 |
| `system_name` | text | 非空 |
| `system_version` | text | 非空 |
| `model_id` | text | 可空 |
| `model_version` | text | 可空 |
| `operations_json` | json | 非空，允许空数组 |
| `generated_at` | timestamptz | 非空 |
| `provider_content_id` | text | 可空 |
| `subject_digest_algorithm` | text | V1 固定 `sha256` |
| `subject_digest_scope` | text | V1 固定 `protected_output` |
| `subject_digest` | text | 64 位小写 hex |
| `parent_subjects_json` | json | 非空，允许空数组 |
| `manifest_sha256` | text | 非空，64 位小写 hex |
| `created_at` | timestamptz | 非空 |
| `updated_at` | timestamptz | 非空 |

唯一约束：

```text
watermark_uid + manifest_version
```

每个 `watermark_uid` 同时最多一个 `active` Manifest。

### 4.3 `ai_claim_evidence`

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `evidence_id` | text | 主键，`ate_` 前缀 |
| `transparency_manifest_id` | text | 非空 |
| `evidence_level` | enum | `evidenceLevel` |
| `evidence_source` | text | 非空 |
| `issuer_id` | text | 可空；`self_declared` 可为空 |
| `key_id` | text | 可空；无签名时为空 |
| `proof_type` | text | 非空 |
| `subject_digest` | text | 必须与 Manifest 一致 |
| `signature_algorithm` | text | 可空 |
| `signature` | text | 可空 |
| `verification_status` | text | 非空 |
| `verified_at` | timestamptz | 可空 |
| `failure_code` | text | 可空 |
| `created_at` | timestamptz | 非空 |

约束：

- `platform_signed`、`registry_signed` 和 `externally_verified` 必须包含 `issuer_id`、`key_id`、`signature_algorithm` 和 `signature`。
- `unsupported_proof` 与 `invalid_proof` 必须包含 `failure_code`。
- 未验证的 Evidence 不能提升为 `platform_signed` 或 `externally_verified`。

### 4.4 `ai_marker_bindings`

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `marker_binding_id` | text | 主键，`atb_` 前缀 |
| `transparency_manifest_id` | text | 非空 |
| `marker_type` | enum | `markerType` |
| `marker_profile_id` | text | 非空 |
| `marker_version` | text | 非空 |
| `detector_scheme` | text | 可空 |
| `detector_endpoint` | text | 可空 |
| `signpost` | text | 可空 |
| `embed_status` | text | 非空 |
| `verify_status` | text | 非空 |
| `binding_digest` | text | 可空 |
| `created_at` | timestamptz | 非空 |

同一 Manifest 至少要有：

```text
blind_watermark
```

任何法规 Profile 要求的显式标识、数字签名元数据或其他标识，也必须有对应 binding 和可验证状态。

### 4.5 `ai_explicit_label_receipts`

显式标识不能只存在于 SDK 返回值或平台前端 state。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `receipt_id` | text | 主键 |
| `transparency_manifest_id` | text | 非空 |
| `profile_id` | text | 非空 |
| `required_surface` | enum | `platform_ui` / `exported_file` / `both` |
| `render_mode` | text | 非空 |
| `rendered_asset_digest` | text | `exported_file` / `both` 时非空 |
| `placement_json` | json | 非空 |
| `locale` | text | 非空 |
| `label_text` | text | 非空 |
| `applied_at` | timestamptz | 非空 |
| `applied_by` | text | 非空 |
| `verification_status` | text | 非空 |
| `created_at` | timestamptz | 非空 |

对 `cn_aigc_label_2025_image_export_v1`，`required_surface=both`，除非该 Profile 有经过法务审查的例外配置和可审计依据。

## 5. 计量与免费公共验证

### 5.1 `ai_marking_ledger`

一条 `confirmed_marked_image` 只在标识完成后提交一次，不等同于支付扣款。

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `ledger_entry_id` | text | 主键，`atlgr_` 前缀 |
| `license_id` | text | 非空 |
| `marking_session_id` | text | 唯一 |
| `transparency_manifest_id` | text | 唯一 |
| `metering_unit` | text | V1 固定 `confirmed_marked_image` |
| `quantity` | integer | V1 固定 `1` |
| `ledger_status` | enum | `ledgerStatus` |
| `committed_at` | timestamptz | 可空 |
| `reversal_reason` | text | 可空 |
| `created_at` | timestamptz | 非空 |

不得创建 ledger 的情况：

- 创建 session。
- failed、cancelled、expired session。
- 重试。
- 重复 confirm。
- 内部 write-after-read。
- 普通用户公共单文件验证。

### 5.2 免费公共验证边界

V1 只冻结“公开 Resolver”合同，不冻结服务器端任意文件上传检测服务。

公开 Resolver：

```text
GET /v1/public/ai-transparency/{watermarkUid}
```

规则：

- 不需要 API key、licenseId 或付费 entitlement。
- 不读取、保存或计量用户提交的媒体文件。
- 只根据可公开解析的 `watermarkUid`、公开元数据 locator 或已由正式客户端提取的标识查询。
- 永不创建 `ai_marking_ledger`。
- 返回 `legalConclusion=false`。
- 可使用滥用防护和匿名速率限制，但不得按次收费。

该 endpoint 本身不等于加州或其他地区要求的完整媒体检测工具。适用客户是否需要提供上传 / URL 检测、浏览器本地检测或平台内检测，必须由其已购 Profile、法务审查和后续 Detector 合同决定。

收费企业验证：

```text
POST /v1/enterprise/ai-transparency/batch-verify
```

需要授权 `verify:batch`，可使用独立的 `batch_verify_item` 计量类型；该计量类型不属于本合同的 `confirmed_marked_image`。

## 6. API 合同

### 6.1 认证

SDK 生产接口使用：

```text
Authorization: Bearer <server-to-server API key or short-lived token>
X-HiddenShield-Idempotency-Key: <opaque value>
```

禁止：

- 在浏览器、移动应用、桌面安装包或开源 SDK 中嵌入生产 issuer 私钥。
- 客户端声明任意 `licenseId` 后跳过 credential 绑定。
- 使用公开 Resolver credential 调用 `mark:image`。

### 6.2 创建标识会话

```text
POST /v1/ai-transparency/marking-sessions
```

请求：

```json
{
  "licenseId": "atl_example",
  "claimType": "ai_generated",
  "providerContentId": "provider-content-123",
  "requestedProfileIds": [
    "cn_aigc_label_2025_image_export_v1",
    "c2pa_ai_output_2_4_image_v1"
  ]
}
```

成功响应：

```json
{
  "markingSessionId": "ats_example",
  "watermarkUid": "HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX",
  "environment": "production",
  "authorizedProfiles": [
    {
      "profileId": "cn_aigc_label_2025_image_export_v1",
      "status": "configuration_required"
    },
    {
      "profileId": "c2pa_ai_output_2_4_image_v1",
      "status": "configuration_required"
    }
  ],
  "expiresAt": "2026-07-27T00:00:00Z"
}
```

服务端必须依序验证：

```text
credential
-> credential binding
-> license status / environment / expiry
-> scope mark:image
-> requested Profile entitlement
-> issuer mode scope
-> idempotency key
-> reserve watermark UID
```

Production credential custody 与受控 `ready_to_confirm` session 创建的冻结合同见：

```text
docs/AI生成内容标识Production_Credential_Custody与Marking_Session创建合同.md
```

内部实现只在 PostgreSQL 中保存 key prefix、HMAC hash、pepper version、custody key ID、scope、issuer mode 和有效/撤销状态；明文 credential 只在签发结果中返回一次。

### 6.3 确认已标识输出

```text
POST /v1/ai-transparency/marking-sessions/{markingSessionId}/confirm
```

请求：

```json
{
  "subjectDigest": {
    "algorithm": "sha256",
    "scope": "protected_output",
    "value": "64-character-lowercase-hex"
  },
  "generation": {
    "generationMode": "text_to_image",
    "providerId": "platform.example",
    "systemName": "Platform Image",
    "systemVersion": "2026.07",
    "modelId": "model-example",
    "modelVersion": "1.0",
    "generatedAt": "2026-07-27T00:00:00Z",
    "operations": []
  },
  "evidence": {
    "evidenceLevel": "platform_signed",
    "issuerId": "platform.example",
    "keyId": "key-2026-07",
    "proofType": "jws",
    "signatureAlgorithm": "EdDSA",
    "signature": "opaque-signature"
  },
  "markers": [
    {
      "markerType": "blind_watermark",
      "markerProfileId": "hiddenshield_v3_image_anchor_v1",
      "embedStatus": "verified",
      "verifyStatus": "verified"
    },
    {
      "markerType": "c2pa",
      "markerProfileId": "c2pa_ai_output_2_4_image_v1",
      "embedStatus": "verified",
      "verifyStatus": "verified"
    }
  ],
  "explicitLabelReceipts": [
    {
      "profileId": "cn_aigc_label_2025_image_export_v1",
      "requiredSurface": "both",
      "renderMode": "file_overlay_and_platform_ui",
      "renderedAssetDigest": "64-character-lowercase-hex",
      "placement": {
        "position": "bottom_right"
      },
      "locale": "zh-CN",
      "labelText": "AI 生成",
      "appliedAt": "2026-07-27T00:00:00Z",
      "appliedBy": "platform.example",
      "verificationStatus": "verified"
    }
  ]
}
```

确认前置条件：

- session 处于 `ready_to_confirm`。
- 盲水印 binding 为 `verified`。
- 每个 requested Profile 都有明确状态，不能缺省。
- 每个 Profile 的必需 marker 和 explicit label receipt 均满足。
- Evidence 与 `subjectDigest` 绑定。
- 最终输出字节已写后读验证。

确认成功时必须在同一事务中：

```text
create manifest
-> create evidence
-> create marker bindings
-> create explicit label receipts
-> create one pending ledger entry
-> commit ledger entry
-> append confirm audit event
-> mark session confirmed
```

任一步失败必须不产生 `committed` 的 `confirmed_marked_image`。

### 6.4 公共 Resolver

```text
GET /v1/public/ai-transparency/{watermarkUid}
```

成功响应最小字段：

```json
{
  "watermarkUid": "HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX",
  "markerStatus": "detected",
  "metadataSignatureStatus": "verified",
  "watermarkDetectionStatus": "verified",
  "issuerTrustStatus": "not_publicly_trusted",
  "claimType": "ai_generated",
  "evidenceLevel": "platform_signed",
  "manifestStatus": "active",
  "profileStatuses": [],
  "warnings": [],
  "legalConclusion": false,
  "resolvedAt": "2026-07-27T00:00:00Z"
}
```

缺失、撤销、冲突和不支持的 Evidence 必须显式返回状态和 warning，不能折叠为“未发现 AI”。

### 6.5 错误码

```text
ai_license_not_found
ai_license_inactive
ai_license_expired
ai_environment_mismatch
ai_scope_denied
ai_profile_not_entitled
ai_issuer_mode_denied
ai_quota_preflight_failed
ai_idempotency_conflict
ai_session_expired
ai_session_state_invalid
ai_subject_digest_invalid
ai_evidence_invalid
ai_marker_requirement_failed
ai_explicit_label_requirement_failed
ai_confirmation_conflict
ai_public_resolution_not_found
ai_public_resolution_rate_limited
```

## 7. 不变量与拒绝规则

- `licenseId` 只能从 credential binding 推导，不能信任请求体提供的任意值。
- production session 不得使用 sandbox license 或测试 issuer。
- 未获授权的法规 Profile 不得被确认成 `applied`。
- `confirmed_marked_image` 只能由 confirmed session 产生一次。
- 公共 Resolver 不得消耗客户标识配额或企业验证配额。
- 公共 Resolver 不得因未发现标识输出“人工创作”或“非 AI”。
- 一个 `watermarkUid` 的 active Manifest 被 supersede / revoke 后，历史事实仍可查询。
- 任何修改 V3 payload、在 SDK 中实现盲水印或把 AI 语义写进 Watermark ID 的实现，都违反本合同。

## 8. 迁移与测试顺序

后续工程必须按以下顺序进行：

```text
1. 为本合同建立 JSON fixture 和 schema contract test。已完成。
2. 创建数据库迁移和唯一约束。已完成。
3. 实现存储层迁移 / 约束回归测试。已完成。
4. 实现内部 license / Profile 管理接口。已完成只读查询、Profile 校验、production credential custody 和受控 `ready_to_confirm` session command；仍未开放公共接口。
5. 实现 marking session 与 confirm 原子事务。已完成 PostgreSQL-only 内部 command、专用 append-only audit migration 和真实双连接/故障回滚 harness；尚未开放 HTTP、SDK 或 production credential。
6. 实现公共 Resolver。
7. 建立 watermark-core + C2PA + 显式标签的图片 fixture。
8. 最后才创建 SDK 包。
```

第 7 步之前禁止发布、销售或分发生产 SDK。

## 9. 当前能力边界

本合同是设计资产；其中的存储结构已开始落地，但不是运行时能力。

- PostgreSQL `0002_ai_transparency_schema` 和本地 SQLite 初始化镜像已定义合同中的表、唯一约束和索引；已在一次性 PostgreSQL 16 环境执行真实 up/down smoke 与 7 项数据库约束回归断言。
- 当前没有运行时端点、真实凭据、可用计量 ledger 流程、公开 Resolver 或 SDK。
- 当前没有真实客户 license、价格、扣款或免费检测工具。
- 当前没有可对用户承诺的中、美、欧 AI 内容标识合规能力。
- 任何后续 Profile 都必须经过外部法务审查、互操作测试和生产信任链 Gate。

## 10. 已冻结 Fixture

固定向量目录：

```text
docs/contracts/ai-transparency/
```

已覆盖：

```text
production license
-> CN / EU / CA Profile entitlement
-> successful confirmed_marked_image
-> free public resolver
-> expired license rejection
-> profile entitlement rejection
-> duplicate confirm conflict rejection
```

Fixture 不包含真实密钥、真实签名、真实客户或真实媒体字节。

## 11. 下一任务

下一工程任务：

```text
评审内部 license / Profile 管理的最小内部接口，
并为后续 confirm 原子事务设计数据库事务边界；
不先实现 SDK、支付扣款、公共 Resolver 或公共媒体上传 Detector。
```

在内部授权管理、confirm 原子事务、法务 Profile 审查和平台试点完成前，不允许开始 SDK 或生产发放。

## 12. 内部管理合同

内部只读 license 查询、Profile entitlement 校验和独立审计事件合同见：

```text
docs/AI生成内容标识内部授权管理与审计合同.md
```

这些接口仅使用现有 admin token；它们不是 SDK 生产接口、公共 Resolver 或 production credential 发放能力。

license / Profile 的未来写入必须先遵守 `docs/AI生成内容标识授权写入双人审批与审计状态机合同.md`；当前 schema 不是已批准的版本化 entitlement 写入模型。

版本化 entitlement 与审批状态机的迁移设计评审见 `docs/AI生成内容标识审批状态机数据库迁移设计评审.md`；评审只允许继续冻结 `0003` Schema Contract 和并发测试 harness，不允许直接创建 migration。
