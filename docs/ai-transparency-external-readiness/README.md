# AI Transparency External Readiness 配置包

状态：`configuration_required_external_only`

本包交付给基础设施、安全、法务、产品和首个设计伙伴，用于收集并验真外部配置。它不包含真实 endpoint、Secret、证书、客户身份、法律意见或验收证据。

## 使用顺序

1. 平台与安全团队基于 `infrastructure-handoff.md` 在各自 Secret/身份系统中创建引用。
2. 设计伙伴基于 `partner-handoff.md` 完成 Sandbox 信息、数据边界和签署引用。
3. 复制 `external-readiness.template.json` 到受控配置系统并替换所有 `replace-me` 值；只保留引用，绝不保存 Secret 值。
4. 使用 `node scripts/verify-ai-transparency-external-readiness.mjs <manifest>` 执行 preflight。
5. 在隔离环境完成 provider recovery 与 12 场景 Sandbox 验收，并将不可变 evidence 引用写回受控 manifest。

## 安全边界

- 生产 Secret、私钥、token、pepper material 和客户媒体不得进入本包、Git、日志或审计事件。
- 所有 Secret 使用 `secret://`；KMS/HSM 对象使用 provider URI；外部 endpoint 必须为非占位 HTTPS URL。
- preflight 成功仅表示引用形状可评审，不表示 provider 可用、Sandbox 已验收、法规适用、生产 credential 已发放或 SDK 可对外发布。
- 任何缺失、占位符、HTTP endpoint、明文 Secret、无效 evidence 或未签署引用均保持 `configuration_required` 或 `blocked_external`。

## 现有合同

- Provider custody：`docs/AI生成内容标识Production_Provider_Deployment_Package.md`
- 伙伴 Sandbox：`docs/AI生成内容标识设计伙伴Sandbox接入包合同.md`
- 伙伴 JSON Schema：`packages/ai-transparency-design-partner-kit/schemas/design-partner-sandbox-kit-v1.schema.json`
- 伙伴 preflight：`packages/ai-transparency-design-partner-kit/bin/preflight.mjs`
