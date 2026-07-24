# Enterprise 公开扫描 API Key 与额度账本模型草案

本文档定义 Enterprise 公开权利扫描的数据模型、门禁和当前上线边界。当前已开放的客户路由仅限受 API key、scope、可信反向代理 hash-only 指纹限流、quota 和审计保护的只读批量扫描：`POST /v1/enterprise/public-rights/batch`。这不表示已经开放外部分发 SDK 计费版、客户自助 API key 控制台、外部 quota 管理、回填、撤销、替代、重签或媒体元数据写入能力；所有返回仍是 registry 与创作者声明解释，不是法律授权结论。

## 1. 目标

- 为未来 Enterprise API key、调用方身份、额度账本、调用审计和网关限流预留统一模型。
- 让公开权利扫描 API 能从当前匿名只读查询，平滑升级到可审计、可限流、可按合同额度管理的 Enterprise 能力。
- 继续保持边界：扫描结果是 registry 与创作者声明，不是法律授权结论。

## 2. 当前范围与不在当前范围

- 当前已开放真实企业只读批量 API 路由：`POST /v1/enterprise/public-rights/batch`。
- 当前路由必须通过 `Authorization: Bearer <key>` 或 `X-HiddenShield-Api-Key` 进行 API key 鉴权，通过 `public_rights:batch_read` scope 校验，执行 DB rate-limit、quota balance 查询、quota ledger committed debit、`used_units` 回写、API audit 和 `last_used_at` 更新。
- 不新增可售套餐或 `api_access=true` 自动开通逻辑。
- 不允许客户端自行创建 API key。
- 不开放外部客户 API key 管理或 quota 管理路由，包括 `/v1/enterprise/api-keys` 和 `/v1/enterprise/quotas`。
- 不开放回填、撤销、替代、重签、manifest 写入或媒体内嵌 C2PA / IPTC。
- 不上传原始媒体、保护副本、本地路径或可还原媒体内容。

## 3. 数据模型草案

### 3.1 `enterprise_api_keys`

用于保存 Enterprise 调用方的 key 元数据。只保存 key hash，不保存明文。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `api_key_id` | text pk | API key ID |
| `account_id` | text | 企业账户 |
| `workspace_id` | text | 绑定工作区 |
| `creator_profile_id` | text nullable | 可选创作者档案 |
| `key_prefix` | text | 明文前缀，用于 UI / 日志定位 |
| `key_hash` | text | Argon2id 或 HMAC-SHA256 hash |
| `name` | text | key 名称 |
| `status` | text | `active` / `paused` / `revoked` / `expired` |
| `scopes_json` | json text | 允许范围 |
| `rate_limit_policy_json` | json text | 限流策略 |
| `quota_policy_json` | json text | 合同额度策略 |
| `created_by_account_id` | text | 创建者 |
| `created_at` | datetime | 创建时间 |
| `last_used_at` | datetime nullable | 最近使用 |
| `expires_at` | datetime nullable | 过期时间 |
| `revoked_at` | datetime nullable | 撤销时间 |
| `revoked_reason` | text nullable | 撤销原因 |

索引建议：

- `idx_enterprise_api_keys_account`
- `idx_enterprise_api_keys_workspace_status`
- `idx_enterprise_api_keys_prefix`
- `idx_enterprise_api_keys_hash`

### 3.1.1 API key 明文签发 / key custody 草案

当前已在内部管理面实现受管理员 token 保护的明文签发入口和 CLI，但仍不开放 `/v1/enterprise/...` 客户路由，也不把内部管理页升级为客户自助控制台。

签发责任边界：

- 只有后端可信执行环境中的 key custody 服务或内部运维 CLI 可以生成明文 API key；桌面端、移动端、Web 管理页和客户浏览器不得生成明文 key。
- 生成动作必须在管理员 token、企业账户、workspace、合同额度和 `api_access` 人工授权均已确认后执行；首版仍不自动开通 `api_access=true`。
- 生产形态应使用 KMS / HSM 或等价密钥托管服务管理 hash pepper / HMAC secret；本地研发可使用环境变量模拟，但不得把 secret 写入仓库、数据库或审计日志。
- 明文 key 使用 CSPRNG 生成至少 256 bit 随机 secret，推荐格式为 `hsent_live_<publicPrefix>_<secret>`；`publicPrefix` 可进入 `keyPrefix`，`secret` 永不入库。
- 当前内部实现使用 `POST /internal/enterprise/api-key-issuances` 和 CLI `issue-api-key` 执行签发；必须配置 `HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET`，可用 `HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET_VERSION` 标记 secret version。未配置 secret 时签发入口拒绝执行并写入失败审计。

明文只显示一次：

- 明文 API key 只允许在签发成功响应或受控终端输出中显示一次；刷新、列表、查询、审计导出、桌面内部页重新打开后均只能看到 `apiKeyId`、`keyPrefix`、状态、scope 和时间字段。
- 签发响应不得被前端持久化到 localStorage、SQLite、日志文件、崩溃报告或截图证据；内部页面如未来接入签发，只能用临时内存弹窗承载一次性复制。
- 客户交付必须走合同约定的安全通道，并记录 `deliveryChannel`、`recipientRef`、`deliveryReceiptHash` 或等价收据摘要；不得在审计里保存完整明文 key。
- 如果管理员关闭一次性展示窗口后未保存明文，系统只能轮换生成新 key，不能找回旧明文。
- `POST /internal/enterprise/api-key-issuances` 返回 `cleartextApiKey`、`keyPrefix`、`hashAlgorithm`、`shownOnce=true` 和 key 元数据；后续 `GET /internal/enterprise/api-keys`、`GET /internal/enterprise/api-keys/{apiKeyId}`、审计查询和桌面内部页均不返回 `cleartextApiKey` 或 `keyHash`。

`keyHash` 入库：

- 入库前先对明文 key 做规范化：去除首尾空白，拒绝含换行、控制字符或长度不足的 key。
- `keyPrefix` 保存非敏感定位前缀，例如 `hsent_live_<publicPrefix>`；日志、错误响应和审计只能使用 `apiKeyId` / `keyPrefix` 定位。
- `keyHash` 推荐保存为带算法和 secret 版本的字符串，例如 `hmac-sha256:v1:<secretVersion>:<digest>`；digest 使用 KMS / HSM 托管 secret 计算。若选择 Argon2id，也必须在字段中携带算法版本和参数。
- 当前内部实现写入 `hmac-sha256:v1:<secretVersion>:<digest>`；生产替换为 KMS / HSM 后仍必须保持算法和 secret version 可审计。
- 网关鉴权时先用 `keyPrefix` 查候选 key，再用当前和仍处于宽限期的 secret version 校验 `keyHash`；成功后只把 `apiKeyId`、account、workspace、scope 和状态传入后续限流 / quota 决策。
- 任何 API 响应、内部列表、审计、QA 证据和错误对象都不得返回 `keyHash`。

轮换流程：

- 轮换不是修改原 key 明文，而是签发一个新的 active key，并把旧 key 标记为 `paused` 或在宽限期后 `revoked`。
- 当前内部实现提供 `POST /internal/enterprise/api-keys/{apiKeyId}/rotate` 和 CLI `rotate-api-key`：复用同一套后端明文签发 / `keyHash` 入库流程生成新 active key，旧 key 立即进入 `paused`；grace period 结束后的正式 `revoked` 仍走 `POST /internal/enterprise/api-keys/{apiKeyId}/revoke` 或后续自动任务。
- 当前内部实现提供 `POST /internal/enterprise/api-key-rotations/revoke-expired` 和 CLI `revoke-expired-rotations`：从 `rotate_api_key` 管理审计 details 的 `rotationDeadlineAt` 读取到期时间，只撤销已到期且仍为 `paused` 的旧 key；每条撤销继续写 `revoke_api_key` 审计，巡检本身写 `revoke_expired_rotations` 汇总审计。
- 未来 schema 可增加 `rotated_from_api_key_id`、`rotated_to_api_key_id`、`rotation_reason`、`rotation_deadline_at` 和 `hash_secret_version`；在字段落地前，这些信息只能进入内部管理审计 `details` 的去敏摘要。
- 轮换审计必须记录 `rotate_api_key` 或等价 operation、操作者、旧 `apiKeyId`、新 `apiKeyId`、reason、grace period、scope 是否变化和 quota policy 是否变化；不得记录明文 key 或 `keyHash`。
- 轮换期间旧 key 如仍处于 grace period，网关必须继续记录旧 `apiKeyId` 的调用审计；过期后返回稳定错误码 `api_key_revoked` 或 `api_key_expired`。

撤销流程：

- 撤销必须是不可恢复状态变更：`active` / `paused` -> `revoked`，写入 `revokedAt` 和 `revokedReason`；重复撤销保持幂等，不覆盖首次撤销原因。
- 撤销审计必须记录 `revoke_api_key`、操作者、目标 `apiKeyId`、account、workspace、reason 和触发来源；不得删除历史调用审计、quota ledger 或管理审计。
- 被撤销 key 的后续网关请求必须拒绝，返回稳定错误码 `api_key_revoked`，并写入去敏的外部 API audit；当前未开放外部路由时只通过 dry-run helper 和运行态 QA 固定该行为。
- 紧急泄露处置优先级高于 grace period：确认泄露后应直接 revoke，必要时人工 void 未完成 reservation，并通过客户安全联系人重新签发。

### 3.2 `enterprise_quota_balances`

用于保存 Enterprise 合同额度余额。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `quota_balance_id` | text pk | 余额 ID |
| `account_id` | text | 企业账户 |
| `workspace_id` | text | 工作区 |
| `quota_type` | text | 例如 `public_rights_scan_units` |
| `period_start` | datetime | 周期开始 |
| `period_end` | datetime | 周期结束 |
| `included_units` | integer | 合同包含额度 |
| `used_units` | integer | 已用额度 |
| `reserved_units` | integer | 预留额度 |
| `overage_allowed` | boolean | 是否允许超额 |
| `overage_unit_price_cents` | integer nullable | 超额单价 |
| `currency` | text | 币种 |
| `updated_at` | datetime | 更新时间 |

索引建议：

- `idx_enterprise_quota_balances_account_period`
- `idx_enterprise_quota_balances_workspace_type`

### 3.3 `enterprise_quota_ledger`

用于记录每次额度消耗、回滚和人工调整。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `quota_ledger_id` | text pk | 流水 ID |
| `account_id` | text | 企业账户 |
| `workspace_id` | text | 工作区 |
| `api_key_id` | text nullable | 来源 key |
| `quota_type` | text | `public_rights_scan_units` |
| `units` | integer | 正数扣减，负数回滚 |
| `direction` | text | `debit` / `credit` |
| `event_type` | text | `scan_batch` / `refund` / `manual_adjustment` |
| `reference_id` | text | 请求 ID 或任务 ID |
| `idempotency_key` | text | 幂等键 |
| `status` | text | `reserved` / `committed` / `voided` |
| `created_at` | datetime | 创建时间 |
| `committed_at` | datetime nullable | 提交时间 |

索引建议：

- `idx_enterprise_quota_ledger_account_type_time`
- `idx_enterprise_quota_ledger_reference`
- `idx_enterprise_quota_ledger_idempotency`

### 3.4 `enterprise_api_audit_events`

用于安全审计和客户对账。不得保存原始媒体、本地路径或完整素材哈希。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `audit_event_id` | text pk | 审计 ID |
| `account_id` | text | 企业账户 |
| `workspace_id` | text | 工作区 |
| `api_key_id` | text nullable | 来源 key |
| `endpoint` | text | API 路径模板 |
| `method` | text | HTTP 方法 |
| `request_count` | integer | 请求数量 |
| `item_count` | integer | 扫描条数 |
| `status_code` | integer | HTTP 状态 |
| `error_code` | text nullable | 稳定错误码 |
| `quota_units` | integer | 本次额度单位 |
| `client_label` | text nullable | 调用方标签 |
| `request_id` | text | 请求 ID |
| `occurred_at` | datetime | 发生时间 |

### 3.5 `enterprise_admin_audit_events`

用于内部后台管理操作审计，与未来外部 API 调用审计分离。不得保存原始媒体、本地路径、完整素材哈希、API key 明文或 `keyHash`。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `audit_event_id` | text pk | 审计事件 ID |
| `operation` | text | `create_api_key` / `list_api_keys` / `get_api_key` / `pause_api_key` / `revoke_api_key` / `init_quota_balance` / `dry_run_gateway` |
| `outcome` | text | `succeeded` / `failed` |
| `endpoint` | text | 内部管理接口路径模板 |
| `account_id` | text nullable | 企业账户 |
| `workspace_id` | text nullable | 工作区 |
| `api_key_id` | text nullable | 目标 API key |
| `target_id` | text nullable | 目标资源 |
| `reason` | text | 操作原因或失败摘要 |
| `details_json` | json text | 去敏后的操作细节 |
| `occurred_at` | datetime | 发生时间 |

## 4. Scope 草案

首批只允许只读 scope：

- `public_rights:read`
- `public_rights:batch_read`
- `public_rights:metadata_export`

禁止 scope：

- `rights_manifest:write`
- `rights_manifest:revoke`
- `rights_manifest:supersede`
- `watermark_payload:write`
- `media_metadata:embed`

## 5. 额度单位

建议新增 `quota_type = public_rights_scan_units`。

计费单位草案：

- 单条查询：1 unit。
- 批量查询：按去重后的 `watermarkUid` 数量计 unit。
- metadata sidecar 导出：1 unit，可在首版配置为不计费但入 audit。
- 失败请求：
  - `watermark_uid_invalid` 不扣。
  - `not_found` 是否扣费由合同决定，默认不扣但入 audit。
  - `rate_limited` 不扣。
  - 服务端 5xx 不扣。

## 6. 状态机

API key 状态：

- `active`：可调用。
- `paused`：企业或管理员暂停，不可调用。
- `revoked`：永久撤销，不可恢复。
- `expired`：到期不可调用。

额度流水状态：

- `reserved`：预扣，任务尚未完成。
- `committed`：任务完成，正式扣减。
- `voided`：任务失败或取消，释放额度。

首版如果只做同步查询，可直接写 `committed`，但仍要保留 `reserved` 模型以支持未来异步批量任务。

## 7. API 草案

当前只实现第一条客户只读批量扫描路由；其余仍作为未来设计：

```http
POST /v1/enterprise/public-rights/batch
GET /v1/enterprise/public-rights/{watermarkUid}
GET /v1/enterprise/quotas/current
GET /v1/enterprise/quotas/ledger
GET /v1/enterprise/api-keys
POST /v1/enterprise/api-keys
PATCH /v1/enterprise/api-keys/{apiKeyId}
DELETE /v1/enterprise/api-keys/{apiKeyId}
```

上线前置条件：

- `api_access=true` 只对 Enterprise 或后台手工授权账户生效；当前实现中外部批量路由只允许 active key 并沿用网关合同的 `api_access` 决策，不提供客户自助开通。
- API key 创建、暂停、撤销必须写 `enterprise_api_audit_events`。
- 当前后端已落地 DB rate-limit window，并支持通过 `HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET` + `HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY=true` 启用生产可信反向代理 / IP 指纹限流。只有代理共享密钥校验通过时，后端才接受 `x-hiddenshield-client-fingerprint`、`x-forwarded-for` 或 `x-real-ip`，并只保存 hash-only `clientFingerprintHash`。
- quota ledger 必须具备幂等键，避免重试重复扣费；当前 `POST /v1/enterprise/public-rights/batch` 已执行 committed debit 和 `used_units` 增量。
- 所有响应必须继续返回 `legalConclusion=false`。

## 7.1 外部网关合同草案

当前合同已用于已开放的 `POST /v1/enterprise/public-rights/batch` 客户路由。任何新的 Enterprise 公开权利 API 进入实现前，也必须通过同一条网关顺序：

1. `authenticate_api_key`：从 `Authorization: Bearer <enterprise key>` 读取 key，使用 `keyPrefix` 定位候选记录，再用 `keyHash` 校验；日志、审计和错误响应不得保存明文 key。
2. `authorize_scope`：单条查询需要 `public_rights:read`，批量查询需要 `public_rights:batch_read`，公开元数据导出需要 `public_rights:metadata_export`；写入、撤销、替代、重签、媒体内嵌元数据 scope 一律拒绝。
3. `check_entitlement_api_access`：即使 key 有效，也必须确认账户具备 Enterprise 合同或后台手工授权的 `api_access=true`；当前阶段固定为关闭。
4. `apply_rate_limit`：按 API key + `EnterpriseGatewayClientFingerprint` 分桶限流；真实客户端 IP / 指纹必须来自可信反向代理，后端不信任普通客户端自报的 forwarded header，也不保存原始 IP。
5. `resolve_readonly_public_rights`：只读取 registry / rights manifest / public metadata sidecar，不做 backfill、manifest 写入、payload 重签、媒体扫描或媒体文件内嵌 C2PA / IPTC。
6. `record_quota_ledger`：按去重后的 `watermarkUid` 数量或 metadata 导出策略写入 `enterprise_quota_ledger`，使用 `requestId + endpoint + normalized item set` 生成幂等键；同步查询首版可直接 `committed`，异步任务必须先 `reserved` 再 `committed` / `voided`。
7. `record_api_audit_event`：无论成功、鉴权失败、限流、quota 不足、not found 或 5xx，都写入去敏后的 `enterprise_api_audit_events`；不得保存原始媒体、本地路径、完整素材哈希、明文 key 或可还原媒体内容。

稳定错误码首版冻结为：

- `enterprise_api_closed`
- `api_key_missing`
- `api_key_invalid`
- `api_key_paused`
- `api_key_revoked`
- `api_key_expired`
- `scope_denied`
- `api_access_disabled`
- `rate_limited`
- `quota_exhausted`
- `quota_contract_missing`
- `watermark_uid_invalid`
- `not_found`
- `registry_unavailable`
- `internal_error`

只读扣费规则：

- 单条公开权利查询：命中且请求有效时 `chargeableUnits=1`。
- 批量公开权利查询：按去重后的有效 `watermarkUid` 数量计费，非法格式项不计费但进入 audit。
- metadata sidecar 导出：合同字段 `chargeMetadataExport` 控制，首版默认 `false`，但必须入 audit。
- `not_found`：合同字段 `chargeOnNotFound` 控制，首版默认 `false`，但必须入 audit。
- `rate_limited`、`api_access_disabled`、`quota_exhausted`、`5xx`：不扣费，但必须入 audit。
- 所有返回仍是训练许可声明和 registry 状态解释，不得把 `legalConclusion` 置为 `true`。

本合同已经在后端 schema 中以 `EnterpriseGatewayAuthContext`、`EnterpriseGatewayRateLimitPolicy`、`EnterpriseGatewayClientFingerprint`、`EnterpriseGatewayQuotaChargePlan`、`EnterpriseGatewayAuditContract`、`EnterpriseGatewayReadOnlyScanContract`、`EnterpriseGatewayDryRunRequest`、`EnterpriseGatewayDryRunDecision`、`ENTERPRISE_GATEWAY_REQUIRED_STEPS` 和 `ENTERPRISE_GATEWAY_STABLE_ERROR_CODES` 固定，并由 `enterprise_gateway_readonly_contract_freezes_auth_rate_limit_quota_and_audit`、`enterprise_gateway_dry_run_helper_outputs_auth_rate_limit_quota_and_audit_decisions` 和 `enterprise_gateway_dry_run_helper_denies_without_charging_or_legal_conclusion` 测试确保网关前置约束不会丢失。内部 helper `dry_run_enterprise_gateway_readonly_scan` 只接受模拟 API key 元数据、required scope、rate-limit 窗口、quota balance 快照、item 数和 hash-only 客户端指纹，输出鉴权、scope、权益、限流、quota 扣费计划和 API audit 决策；它不读取数据库、不写 quota ledger、不生成明文 API key，所有返回仍固定 `legalConclusion=false`。

已开放路由的生产约束：

- 只开放 `POST /v1/enterprise/public-rights/batch`，不开放外部 key 管理、quota 管理或 metadata 写入路由。
- API key lookup 使用 `keyPrefix + hmac-sha256:v1:<secretVersion>:<digest>` 校验，不返回明文或 `keyHash`。
- 成功请求按 item 数写入 quota ledger committed debit，并更新 quota balance `used_units`。
- 缺 key、无效 key、scope 拒绝、限流、quota 不足和 revoked key 必须返回稳定错误码或稳定 HTTP 拒绝结果，并写入去敏审计。
- 所有返回仍是训练许可声明和 registry 状态解释，不得表述为法律授权结论。

## 8. 内部管理入口

当前已实现的入口只面向内部运维和后台管理，不属于外部 Enterprise API：

```http
GET /internal/enterprise/api-keys
POST /internal/enterprise/api-keys
POST /internal/enterprise/api-key-issuances
GET /internal/enterprise/api-keys/{apiKeyId}
POST /internal/enterprise/api-keys/{apiKeyId}/pause
POST /internal/enterprise/api-keys/{apiKeyId}/rotate
POST /internal/enterprise/api-keys/{apiKeyId}/revoke
POST /internal/enterprise/api-key-rotations/revoke-expired
POST /internal/enterprise/quota-balances
GET /internal/enterprise/admin-audit-events
POST /internal/enterprise/gateway-dry-run
```

约束：

- 这些入口均复用管理员 token 保护和 `admin_audit_events` 审计。
- 进入内部入口的 token 校验结果仍记录在 `admin_audit_events`；具体业务操作另写 `enterprise_admin_audit_events`，按 `create_api_key`、`issue_api_key`、`rotate_api_key`、`revoke_expired_rotations`、`list_api_keys`、`get_api_key`、`pause_api_key`、`revoke_api_key`、`init_quota_balance`、`dry_run_gateway` 细分。
- `GET /internal/enterprise/api-keys` 支持按 `accountId`、`workspaceId`、`status` 和 `limit` 查询，只返回 key 元数据，不返回 `keyHash` 或明文 key。
- `GET /internal/enterprise/api-keys/{apiKeyId}` 返回单个 key 元数据。
- `POST /internal/enterprise/api-keys` 只创建 API key 元数据，要求调用方传入 `keyPrefix` 和 `keyHash`，后端不生成、不返回、不保存明文 key。
- `POST /internal/enterprise/api-key-issuances` 由后端生成一次性明文 key、计算 `keyHash` 并创建 active key；响应只在本次返回 `cleartextApiKey`，同时写入 `issue_api_key` 内部审计，审计 details 只允许记录 `keyPrefix`、scope、hash algorithm、交付通道和 recipient ref。
- `POST /internal/enterprise/api-keys/{apiKeyId}/pause` 将 active / paused key 置为 `paused`，用于合同复核、客户暂停或风控观察；已 revoked / expired 的 key 不能再暂停。
- `POST /internal/enterprise/api-keys/{apiKeyId}/rotate` 生成新 active key，旧 key 立即置为 `paused`，响应只在本次返回新 key 的 `cleartextApiKey`；`rotate_api_key` 审计必须用旧 `apiKeyId` 作为 `apiKeyId`，新 `apiKeyId` 作为 `targetId`，并记录 grace period / deadline / delivery 摘要，不能记录明文或 `keyHash`。
- `POST /internal/enterprise/api-keys/{apiKeyId}/revoke` 将 active / paused key 置为 `revoked`，写入 `revokedAt` 和 `revokedReason`；重复撤销保持幂等，不覆盖第一次撤销原因。
- `POST /internal/enterprise/api-key-rotations/revoke-expired` 是 internal-only 巡检入口，支持 `now`、`limit` 和 `reason`；它只处理 `rotationDeadlineAt <= now` 且旧 key 仍为 `paused` 的轮换记录，不修改新 key、不读取或返回 `keyHash`，也不接真实 quota 扣费。
- `POST /internal/enterprise/quota-balances` 用于初始化或调整合同周期余额，唯一键为 `accountId + workspaceId + quotaType + periodStart + periodEnd`。
- quota balance 初始化是幂等 upsert：允许调整 `includedUnits`、`overageAllowed`、`overageUnitPriceCents` 和 `currency`，但不会重置 `usedUnits` 或 `reservedUnits`。
- 当前只允许 `quotaType=public_rights_scan_units`。
- `GET /internal/enterprise/admin-audit-events` 是只读内部审计查询入口，支持按 `operation`、`outcome`、`accountId`、`apiKeyId`、`fromOccurredAt`、`toOccurredAt` 和 `limit` 过滤；查询本身不再写入新的 `enterprise_admin_audit_events`，避免审计日志自污染。管理员 token 校验仍由通用 `admin_audit_events` 记录。`POST /internal/enterprise/gateway-dry-run` 调用 `dry_run_enterprise_gateway_readonly_scan`，只根据请求体中的模拟 key / scope / quota / item 数返回决策并写入 `dry_run_gateway` 内部管理审计，不写 `enterprise_api_audit_events`、不写 quota ledger。
- 桌面端内部后台页 `EnterpriseAuditView` 已升级为 Enterprise 内部管理工作台：同一页面接入 create / list / get / pause / revoke API key 元数据、quota balance 初始化和 `GET /internal/enterprise/admin-audit-events` 审计筛选、按时间游标分页、当前页 JSON 导出；管理员 token 仅保存在页面内存，内部服务地址可本地保存用于调试。
- 内部管理页只展示和提交 key 元数据；当前不接明文签发弹窗，不持久化明文 API key。
- 维护脚本 `scripts/enterprise-internal-admin.mjs` 已覆盖 issue / rotate / revoke expired rotations / create / list / get / pause / revoke / init quota balance / list admin audit events / dry-run gateway，只是上述内部入口的 CLI 薄封装，默认拒绝调用 `/v1/enterprise/...`。

## 9. 当前决策

当前已完成 schema 草案、数据库迁移、内部后台入口、内部 CLI、quota balance 初始化、API key 内部列表 / 查询 / 暂停 / 撤销、内部操作审计细分、内部只读审计查询入口、桌面端 Enterprise 内部管理工作台、外部 Enterprise API 网关鉴权 / 可信代理指纹限流 / 只读扣费 / 审计合同、内部 dry-run 网关校验 helper、受管理员 token 保护的内部一次性明文签发入口、内部 key 轮换命令、过期轮换自动撤销内部巡检命令，以及 `POST /v1/enterprise/public-rights/batch` 外部只读批量扫描路由。内部 Storage 管理命令覆盖 API key 元数据、quota balance、quota ledger、外部 API 调用审计预留和内部管理操作审计。后端目前具备内部签发、轮换、过期轮换撤销、创建、查看、暂停、撤销 API key 元数据，初始化 quota balance，记录 quota ledger、记录外部 API audit event、记录内部 admin audit event，并按 operation / outcome / accountId / apiKeyId / occurredAt 过滤查询内部 admin audit event 的能力；桌面内部页可执行元数据管理工作流并导出审计当前页 JSON，但不接明文签发、轮换或自动撤销弹窗。下一步应补客户合同开通签字、外部 API 文档、生产 SLA / 对账和支持流程，同时继续禁止外部 `/v1/enterprise/api-keys`、`/v1/enterprise/quotas` 和任何写入型客户路由。
