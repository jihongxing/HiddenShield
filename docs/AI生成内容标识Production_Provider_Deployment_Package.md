# AI 生成内容标识 Production Provider Deployment Package

## 状态

- 状态：`package_ready_internal_only`。
- 本包冻结生产 custody 的配置与 fail-closed 合同；不包含真实 Internal IAM、KMS/HSM endpoint、工作负载身份、pepper 或任何生产 Secret。
- 本包不是生产部署证明，不解锁 SDK、公共 Resolver、客户 credential、HTTP marking API 或合规宣传。

## 配置合同

- 模板：`config/ai-transparency-production-provider.env.example`。
- `HIDDENSHIELD_AI_TRANSPARENCY_CUSTODY_ENABLED=true` 时，服务启动必须同时验证 IAM receipt URL、issuer、audience、JWKS URL、KMS provider、active/retained pepper reference、workload identity reference、KMS health URL 和恢复 runbook reference。
- 缺失、非法 HTTPS URL、未知 KMS provider、明文 `secret=`、示例域名或模板占位符必须拒绝启动；仓库示例文件不能直接作为生产配置启动。
- 配置中只允许 Secret/KMS 引用；pepper material、API token、私钥和 provider credential 不得写入环境变量、配置文件、日志、数据库或审计事件。
- custody 未启用时服务可以启动，但所有 production custody 命令必须由 readiness Gate 显式放行；不存在默认放行路径。

## 操作 Gate

- `issue_production_credential`、`create_ready_marking_session`、`rotate_production_credential`、`revoke_production_credential` 必须在开启 PostgreSQL 事务前检查 provider readiness。
- readiness 同时要求 Internal IAM receipt、KMS health 和 active pepper 均可用；任一不可用返回 provider unavailable，且不得创建 credential/session、修改 credential、追加 runtime/lifecycle audit 或更新投影。
- 当前 `UnavailableProductionProviderProbe` 是部署前安全默认值；真实 provider adapter 只能在完成 endpoint、工作负载身份、KMS/HSM 授权和健康检查验收后替换它。
- 仓库不提供可供 production 调用的内置“永远就绪”实现；QA 的放行实现只定义在 PostgreSQL QA binary 内。

## 注入与演练顺序

1. 平台团队提供 Internal IAM receipt endpoint、issuer/audience、JWKS、工作负载身份 Secret 引用和 KMS/HSM pepper reference。
2. 安全团队确认 identity scope 仅限 `ai_transparency_credential_custodian`，并验证 KMS key policy、pepper retention 与 retirement owner。
3. 在隔离的 `hiddenshield_migrate_smoke_*` PostgreSQL 库运行 custody QA，验证 IAM/KMS/active pepper unavailable 的零写入、恢复后成功、轮换/撤销并发与审计完整性。
4. 将真实 adapter 接入非生产环境，执行 receipt 过期、scope mismatch、provider outage、pepper retirement 与恢复演练；保存审计证据。
5. 由安全、平台、法务和产品共同复核后，才可评审受控 production entitlement；该复核仍不自动发放 credential 或开放 SDK。

## 当前缺口

- 未提供真实 IAM/KMS/HSM 配置、外部 endpoint 验真或工作负载身份，因此不能执行真实 provider 演练。
- 已完成模拟 Gate：一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke_provider_20260727` 验证 provider unavailable 的四类 custody 命令零生产副作用，并验证 QA provider 恢复后的正常 session；该证据不等同于真实 provider 健康检查或 Secret 注入演练。
- 未实现生产 credential 发放入口、SDK、公共 Resolver 或任何对外 API。
- CN/EU/US（加州）Profile 的内部 Gate 通过不等同于法律意见、监管认证或跨平台合规承诺。
