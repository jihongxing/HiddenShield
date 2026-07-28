# AI 生成内容标识 Production Credential Custody 与 Marking Session 创建合同

版本：`v1-frozen`

状态：`postgres_internal_implementation_authorized_no_sdk_no_public_endpoint`

## 1. Gate 结论

本合同只允许实现 PostgreSQL 内部 credential custody 和受控 `ready_to_confirm` session 创建命令。

它不开放 SDK、HTTP 公共端点、公共 Resolver、支付、自助开通或法规合规宣传。

## 2. Credential Custody

生产 credential：

- 明文格式为 `hsai_live_...`，只在内部签发结果中返回一次。
- 数据库不得保存明文、可逆密文或原始 bearer token。
- 数据库只保存 `keyPrefix`、HMAC-SHA256 hash、pepper/version、custody key ID、environment、scope、issuer mode、有效期和撤销状态。
- HMAC pepper 只能来自运行时 secret/KMS 注入，不得写入数据库、audit、fixture 或日志。
- 签发前必须通过 fail-closed Internal IAM custody authorization；provider unavailable、身份无效、role/scope 不匹配一律零写入。
- production credential 只能绑定 active、已生效、未过期的 production license。
- rotate/revoke 必须在单一 PostgreSQL 事务中：锁定旧 credential、创建 replacement（rotate 时）、撤销旧 credential、写 append-only lifecycle audit；任一步失败不得留下 replacement 或半撤销状态。
- credential rotation/revocation 后旧 credential 不得创建新 session。
- pepper 支持 `active` 与 retained versions。新 credential 必须使用 active KMS/HSM pepper；旧版本仅在保留窗口内用于校验尚未撤销的历史 credential，版本缺失或 KMS/HSM unavailable 必须 fail-closed。
- rotate/revoke 的 custody authorization 必须通过 `InternalIamAuthorizationAdapter` receipt，要求 `ai_transparency_credential_custodian` role；无 receipt、过期、scope mismatch 或 unavailable 一律拒绝。

## 3. Ready Session 创建 Gate

进入 `ready_to_confirm` 前必须同时满足：

```text
credential prefix/hash matches
-> credential active and unexpired
-> credential environment = production
-> credential contains mark:image
-> credential issuer mode permits license issuer mode
-> license active/effective/unexpired
-> tenant/workspace/environment match
-> every requested Profile entitlement active/effective/unexpired
-> idempotency key unused
-> create ready_to_confirm session and success audit atomically
```

任一失败必须：

- 不创建 marking session。
- 不更新 credential `last_used_at`。
- 不创建 success audit。
- 返回稳定拒绝码。

## 4. PostgreSQL 唯一生产语义

- PostgreSQL row lock、唯一约束和事务结果是唯一 production/release Gate。
- SQLite 不实现 production credential custody，也不形成 session 创建并发证据。
- 同 credential、同 idempotency key 的两个真实连接最多一个成功。

## 5. 稳定拒绝码

```text
ai_credential_unauthorized
ai_credential_inactive
ai_credential_expired
ai_credential_scope_denied
ai_credential_environment_mismatch
ai_credential_issuer_mode_denied
ai_license_inactive
ai_license_expired
ai_environment_mismatch
ai_scope_denied
ai_profile_not_entitled
ai_idempotency_conflict
ai_credential_rotation_conflict
```

## 6. 当前禁止项

- SDK 和公共 HTTP marking API。
- production customer credential 自助发放。
- 在未完成真实 provider/KMS、KMS/HSM endpoint、pepper rotation/revocation 和运行审计 Gate 前对外发放 credential。
- 将内部 owner audit 或技术测试描述为 CN/EU/US 法律意见或合规认证。
