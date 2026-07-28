# 基础设施与安全交接

将以下信息以引用形式提供给 HiddenShield，不要把值复制到仓库或本地 manifest。

| 责任方 | 交付项 | 允许格式 | 验收人 |
| --- | --- | --- | --- |
| IAM | receipt URL、issuer、audience、JWKS URL、scope 定义 | 非占位 HTTPS URL / 受控配置值 | Security |
| Workload Identity | 运行身份引用与最小权限策略 | `secret://...` 或云身份 URI | Security |
| KMS/HSM | provider、active/retained pepper 引用、health URL、key policy 引用 | KMS/HSM URI / HTTPS | Security |
| Signer | signing endpoint、issuer/key reference、receipt 验真策略 | 非占位 HTTPS / `secret://...` | Security + Legal |
| Object store | artifact bucket/prefix、finalize receipt 策略、保留策略 | `secret://...` / policy reference | Platform |
| Notification | destination policy、provider endpoint、routing Secret、receipt 策略 | 非占位 HTTPS / `secret://...` | Security + SRE |
| SRE | provider outage、pepper retirement、signer/object-store 故障恢复 runbook | `runbook://...` | SRE |

## 最小权限要求

- IAM scope 仅授予 `ai_transparency_credential_custodian` 所需操作。
- KMS/HSM 身份仅可使用指定 pepper/key 版本，不可列举或导出密钥材料。
- signer、object-store 和 notification adapter 必须各自使用独立身份与最小写权限。
- 每个 provider 必须提供健康检查、失效处理、轮换/撤销责任人和不可变演练证据引用。

## 演练出口

仅在所有真实引用已通过 preflight 后，才允许在隔离 PostgreSQL 环境执行：

1. IAM receipt expired、scope mismatch 与 unavailable 的零写入测试。
2. active/retained pepper、KMS/HSM unavailable 与 retirement/recovery 测试。
3. signer、object-store finalize、notification delivery 的 receipt mismatch 与崩溃恢复测试。
4. 双人复核的恢复演练完成记录。

