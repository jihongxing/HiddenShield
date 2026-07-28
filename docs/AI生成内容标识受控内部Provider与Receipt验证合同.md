# AI 生成内容标识受控内部 Provider 与 Receipt 验证合同

版本：`v1`

状态：`internal_receipt_validation_implemented_real_provider_activation_external_only`

当前内部 receipt validation、签名/过期/scope/health fail-closed 路径与 PostgreSQL 零写入 QA 已实现。真实 Internal IAM/JWKS、工作负载身份、KMS/HSM、非对称签名、health SLA 与 replay registry 仍需要外部 provider 配置和演练。

## 1. 边界

本合同定义内部 change-command 使用的受控 Provider client。它只验证 Internal IAM 与 contract / legal / security reference 的 receipt，不提供 HTTP route、SDK、生产 credential、License/Profile 发放或公共验证能力。

## 2. Receipt

receipt 必须包含：

- `providerId`、`keyId`、`receiptId`、`kind`。
- `granted`、`status=active`。
- `scopeDigest`。
- `issuedAt`、`expiresAt`。
- `signature`。

当前内部测试协议为 `hs-internal-provider-receipt-v1`，使用配置的 HMAC-SHA256 key 对 canonical receipt payload 签名。该协议是受控内部测试实现，不表示生产密钥管理、跨组织信任或公开标准。

## 3. Scope Digest

IAM scope digest 绑定：

```text
tokenHash
requiredRole
tenantId
workspaceId
environment
operation
```

reference scope digest 绑定：

```text
referenceType
referenceId
tenantId
workspaceId
environment
operation
```

原始 token、合同全文、法务文件和安全文件不得进入 receipt persistence、审批数据库或 audit。

## 4. Fail-Closed

client 必须验证 provider/key identity、HMAC signature、receipt kind、active status、grant 状态、issued/expires 和 scope digest。

| 失败 | IAM 拒绝码 | Reference 拒绝码 |
| --- | --- | --- |
| 签名、key、status 或 grant 无效 | `iam_token_invalid` | `reference_authority_untrusted` |
| receipt 过期或未来签发 | `iam_token_expired` | `reference_expired` |
| scope digest 不匹配 | `iam_scope_denied` | `reference_scope_mismatch` |
| health 或 transport unavailable | `iam_unavailable` | `reference_unavailable` |

所有生产 Gate 拒绝必须发生在 PostgreSQL transaction 之前；不得写入 target lock、request、approval、execution、projection、audit、credential、session、Manifest 或 ledger。SQLite 可保留同类本地单元测试，但不形成生产证据。

## 5. 验证与限制

PostgreSQL harness 覆盖有效 receipt，以及 IAM/reference 的签名、过期、scope digest、health/transport unavailable 拒绝与零写入。SQLite 结果只作为本地快速回归。

仍属于外部依赖：

- 生产 IAM/reference provider client。
- HSM/KMS、密钥轮换、非对称签名或 JWKS。
- provider health SLA、failover、receipt replay registry。
- 法务 Profile 的司法辖区控制清单、法务签署和生产审查。

因此所有生产发放、SDK、公共 Resolver 和法规合规宣传继续禁止。
