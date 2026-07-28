# AI 生成内容标识内部授权管理与审计合同

版本：`v1`

冻结日期：`2026-07-27`

实现状态：`internal_read_only_v1`

## 1. 目的与硬边界

本合同只允许 HiddenShield 内部运维和法务配置流程查询 AI Transparency license，并校验某个 license 是否具有 requested Profile entitlement。

本阶段：

- 只允许通过现有内部 admin token 访问。
- 只读 license 和 Profile entitlement。
- 每次成功、拒绝或失败都写独立审计事件。
- 不创建、续期、暂停或撤销 license。
- 不创建或修改 Profile entitlement。
- 不创建 SDK credential、marking session、Manifest、ledger 或计费记录。
- 不开放公共 Resolver、Detector 或任何 `/v1/ai-transparency/*` 生产接口。
- 不输出生产密钥、签名、媒体字节或用户本地路径。

## 2. 内部认证

所有接口必须使用现有内部管理员认证：

```text
X-HiddenShield-Admin-Token: <configured admin token>
```

禁止接受 Enterprise API key、SDK credential、licenseId 本身或公共 Resolver 身份作为内部管理员认证。

## 3. License 查询

```text
GET /internal/ai-transparency/licenses/{licenseId}
```

成功响应：

```json
{
  "license": {
    "licenseId": "atl_example",
    "tenantId": "tenant_example",
    "workspaceId": "workspace_example",
    "environment": "production",
    "status": "active",
    "issuerMode": "hiddenshield_managed",
    "deploymentMode": "hosted",
    "publicVerificationRequired": true,
    "meteringPlanId": "metering_example",
    "effectiveAt": "2026-07-27T00:00:00Z",
    "expiresAt": "2027-07-27T00:00:00Z",
    "createdAt": "2026-07-27T00:00:00Z",
    "updatedAt": "2026-07-27T00:00:00Z"
  },
  "profileEntitlements": []
}
```

规则：

- `licenseId` 必须是非空 opaque ID。
- 不返回 credential binding、API key、密钥材料或计量余额。
- license 不存在时返回 `404 / ai_license_not_found`。
- 查询成功写 `get_license / succeeded` 审计。
- 不存在、非法请求或存储失败分别写 `denied` 或 `failed` 审计。

## 4. Profile Entitlement 校验

```text
POST /internal/ai-transparency/profile-entitlements/check
```

请求：

```json
{
  "licenseId": "atl_example",
  "environment": "production",
  "requestedProfileIds": [
    "cn_aigc_label_2025_image_export_v1",
    "c2pa_ai_output_2_4_image_v1"
  ]
}
```

成功响应：

```json
{
  "licenseId": "atl_example",
  "authorized": true,
  "evaluatedAt": "2026-07-27T00:00:00Z",
  "licenseDecision": {
    "authorized": true,
    "reasonCode": "authorized"
  },
  "profileDecisions": [
    {
      "profileId": "cn_aigc_label_2025_image_export_v1",
      "authorized": true,
      "reasonCode": "authorized",
      "profileKind": "regulatory",
      "termsVersion": "v1",
      "expiresAt": "2027-07-27T00:00:00Z"
    }
  ]
}
```

校验顺序：

```text
license exists
-> environment matches
-> license status is active
-> effectiveAt <= server time < expiresAt
-> every requested Profile exists for license
-> Profile status is active
-> Profile effectiveAt <= server time < expiresAt
```

稳定 `reasonCode`：

```text
authorized
ai_license_not_found
ai_license_environment_mismatch
ai_license_not_effective
ai_license_expired
ai_license_inactive
ai_profile_not_entitled
ai_profile_not_effective
ai_profile_expired
ai_profile_inactive
```

规则：

- `requestedProfileIds` 必须包含 `1..32` 个去重后的非空值。
- 任一 Profile 未授权时，顶层 `authorized = false`。
- 校验是只读判断，不保留 Watermark ID，不创建 marking session，不创建 ledger。
- 业务拒绝仍返回结构化校验结果；非法请求返回 `400`。
- 成功授权写 `check_profile_entitlements / succeeded` 审计。
- 业务拒绝写 `check_profile_entitlements / denied` 审计。
- 存储或内部错误写 `check_profile_entitlements / failed` 审计。

## 5. 审计事件

表：`ai_transparency_admin_audit_events`

字段：

```text
auditEventId
operation
outcome
endpoint
licenseId?
tenantId?
workspaceId?
requestedProfileIds
reasonCode
details
occurredAt
```

允许值：

```text
operation: get_license | check_profile_entitlements
outcome: succeeded | denied | failed
```

约束：

- `requestedProfileIds` 和 `details` 使用结构化 JSON。
- `details` 只能包含状态、环境、计数和稳定 reason code。
- 禁止写 API key、credential、密钥、签名、媒体内容、媒体摘要、用户本地路径或完整 HTTP header。
- 审计写入失败时，成功的内部管理请求必须 fail-closed，不得返回未审计的授权结论。
- 审计事件不产生 `confirmed_marked_image` 或任何其他计量。

## 6. 实现范围

V1 实现：

- 内部 license 查询。
- 内部 Profile entitlement 校验。
- 上述操作的独立审计写入。
- SQLite 运行时存储与 PostgreSQL schema / smoke 对齐。

V1 不实现：

- license / Profile 创建、修改、续期、暂停、撤销。
- 审计列表 HTTP 接口。
- SDK credential 发放。
- marking / confirm。
- 公共 Resolver 或 Detector。
- 支付、扣款或生产 SKU 开通。

## 7. 写入审批 Gate

license / Profile 的创建、续期、暂停和撤销必须遵循：

```text
docs/AI生成内容标识授权写入双人审批与审计状态机合同.md
```

该合同尚未实现；当前 `ai_profile_entitlements` 的覆盖式逻辑唯一键也不允许直接用于重新授权或历史版本覆盖。

## 8. 下一 Gate

下一任务只允许冻结 actor / role 身份来源、Profile entitlement 版本模型与 change request fixture / 状态机 contract test；在这些 Gate、confirm 原子事务与法务 Profile 审查通过前，不得开始 SDK、公共 Resolver 或 production credential 发放。
