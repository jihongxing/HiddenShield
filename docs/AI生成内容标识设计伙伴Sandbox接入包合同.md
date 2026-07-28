# AI 生成内容标识设计伙伴 Sandbox 接入包合同

状态：`frozen_internal_sandbox_kit_implemented_external_configuration_required`

能力分类：`只能内部测试`

冻结日期：2026-07-28

## 1. 目标

本合同冻结 HiddenShield AI 生成内容标识基础设施面向真实设计伙伴的受控 Sandbox 接入包，使平台方可以在不获得生产 credential、不接入公网生产环境、不形成法律意见或 SLA 的前提下，完成：

- onboarding 信息收集；
- CN / EU / US-CA Profile mapping questionnaire；
- server-only SDK / internal platform API 示例联调；
- 免费公共 Resolver link 构造与匿名查询验收；
- 标识、confirm、计量、失败关闭和最小公共字段验收；
- 形成可审计、不可将外部阻塞误记为通过的证据矩阵。

实现载体：

```text
packages/ai-transparency-design-partner-kit
```

冻结 Schema：

```text
hs-ai-design-partner-sandbox-kit-v1
```

该 package 必须保持 `private: true`，不得作为公开 npm 包、生产 credential 发放渠道或客户端 SDK 分发。

## 2. Onboarding 合同

伙伴 bundle 必须声明：

- `partnerId`；
- partner legal name、technical contact、security contact 的外部引用；
- `environment=sandbox`；
- Sandbox API 与 Resolver 的 HTTPS endpoint；
- `secret://...` credential 引用；
- AI 生成 / 编辑图片 use case；
- `issuerMode`；
- `deploymentMode`；
- `outputContentType=image/png`；
- 预期月 confirmed 图片量、峰值 RPS、mark-confirm 延迟预算；
- 非生产、无 SLA、非法律意见、不得提升 Sandbox credential 至生产的确认。

包内禁止写入真实 credential、Secret、私钥、token、个人联系方式和生产 provider 配置。真实伙伴身份、endpoint 与 Secret 注入属于外部配置依赖。

## 3. Profile Mapping Questionnaire

问卷只收集事实，不自动判断法律适用性。CN、EU、US-CA 分别允许：

- `applicable`；
- `not_applicable`；
- `unknown`。

`unknown` 不得被解释为 Profile 已通过。问卷必须覆盖：

- 新生成、编辑、混合来源内容；
- 原始资产是否保留；
- PNG 输出边界；
- 平台 UI、导出文件、两者或下游 API metadata 的显式声明 surface；
- `hiddenshield_v3_image_anchor_v1` 技术 Profile；
- C2PA-compatible、额外 metadata、customer-managed issuer 等可选技术 Profile；
- hosted/private deployment；
- CN/EU/US 数据驻留、媒体保留和 digest/object-store 权限；
- requested regulatory Profile、未决问题和 legal review 外部引用。

Sandbox Profile admission 仍复用 production-equivalent license/Profile entitlement 合同，但部署、标识符和 credential 必须与生产隔离，禁止直接晋级或复制到生产。

## 4. SDK / API 示例合同

示例必须：

- 在 trusted server runtime 使用 `@hiddenshield/ai-transparency-sdk`；
- 通过 `createAiTransparencyPlatformFacade` 调用 admission、session、mark、confirm；
- credential 仅从运行时环境或 Secret provider 读取；
- 不输出、不记录、不回传 credential；
- 对 Profile denial、credential invalid、hash mismatch、provider unavailable 和 receipt mismatch fail-closed；
- confirm 只接受 `confirmed_marked_image + quantity=1 + committed`；
- duplicate confirm replay 不重复计量；
- 不在伙伴包内实现或复制盲水印算法，mark 必须经 backend 调用 `watermark-core`。

示例文件：

```text
packages/ai-transparency-design-partner-kit/examples/server-mark-and-resolve.mjs
```

## 5. Resolver Link Contract

Resolver URL 只能使用以下一个公开标识：

- `watermarkUid`；
- `manifestId`。

两者必须且只能提供一个。URL 固定映射为：

```text
GET {resolverBaseUrl}/v1/ai-transparency/public/resolve/watermarks/{watermarkUid}
GET {resolverBaseUrl}/v1/ai-transparency/public/resolve/manifests/{manifestId}
```

Resolver 合同继续保持：

- 匿名；
- 无授权 header；
- 无媒体上传；
- 无 `confirmed_marked_image` 计量；
- 仅 confirmed 记录可见；
- 最小公共字段；
- `legalConclusion=false`；
- 不返回 license、tenant、workspace、credential、内部 evidence、原始媒体或内部审计字段。

伙伴自建链接页面不得扩展为 HiddenShield 法律结论、平台背书或内容真实性保证。

## 6. 验收矩阵

以下 12 个场景全部为强制项：

1. `admission_success`
2. `profile_denied_fail_closed`
3. `session_ready_to_upload`
4. `invalid_credential_zero_state_change`
5. `png_mark_write_after_read`
6. `confirm_single_metering_unit`
7. `confirm_replay_no_duplicate_metering`
8. `resolver_preconfirm_not_found`
9. `resolver_postconfirm_anonymous`
10. `resolver_minimum_public_fields`
11. `secret_redaction`
12. `latency_budget_recorded`

每个场景状态只能为：

- `not_run`；
- `passed`；
- `failed`；
- `blocked_external`。

`passed` 必须绑定不可变 `evidenceRef`。`blocked_external` 不是通过，不能用于 Sandbox acceptance、生产 entitlement、SLA、计费上线或客户宣传。

## 7. Readiness 状态机

```text
invalid
  -> configuration_required
  -> ready_for_internal_review
  -> sandbox_accepted
```

`sandbox_accepted` 必须同时满足：

- `packageStatus=approved_for_sandbox`；
- API 与 Resolver 为非 `.invalid` 的真实 HTTPS Sandbox endpoint；
- 12 个强制场景全部 `passed`；
- 每个场景均有不可变 evidence reference；
- bundle 无 raw Secret；
- 伙伴 technical/security 与 HiddenShield engineering/commercial owner 完成书面确认，并分别写入外部 approval reference；
- 每个通过证据使用 `evidence://sha256/{digest}` 内容寻址引用。

任一条件缺失必须 fail-closed，不能生成 accepted 状态。

## 8. 外部阻塞项

以下项可挂起，但不得伪造：

- 真实伙伴身份与联系人外部引用；
- 真实 Sandbox API / Resolver endpoint；
- 伙伴专属 Sandbox credential Secret 注入；
- 伙伴运行环境、流量、延迟和失败率证据；
- 伙伴法务、数据驻留、安全和采购确认；
- 公网 gateway、生产 provider、生产 credential 与客户 SLA。

未配置模板的预期结果固定为：

```json
{
  "valid": true,
  "readiness": "configuration_required"
}
```

## 9. 验证命令

```text
npm run ai-transparency:design-partner-kit
```

该 Gate 验证 Schema、模板、Secret 边界、Resolver link、12 场景矩阵和 readiness 计算，但不替代真实伙伴环境验收。

CI 必跑入口：

```text
npm run ai-transparency:ci
```

该入口执行 SDK contract/test、设计伙伴 package contract/test、template preflight 和 synthetic Sandbox QA。它不需要 PostgreSQL、真实 endpoint 或 Secret，因而可以作为后续 SDK/API 改动的持续回归 Gate。

## 10. 发布边界

当前可以内部承诺：

- 接入包、问卷、示例、Resolver link builder、验收矩阵和 preflight 已实现；
- 模板在无外部配置时稳定返回 `configuration_required`；
- 满足全部合同条件时才计算 `sandbox_accepted`。

当前不能承诺：

- 已有真实设计伙伴完成接入；
- 已提供真实 Sandbox endpoint 或 credential；
- SDK 已公开发布；
- production credential 可发放；
- 公网 Resolver、SLA、法律合规结论或生产互操作已上线。

## 11. 下一 Gate

## 11. Synthetic Sandbox QA

在没有真实设计伙伴时，可运行：

```text
npm run ai-transparency:synthetic-sandbox-qa
```

该 harness 固定输出：

```text
executionMode=synthetic_non_acceptance
acceptanceStatus=not_real_partner_acceptance
readiness=configuration_required
```

它复用 SDK / facade 的 admission、session、mark、confirm、duplicate replay、Profile 拒绝和最小 Resolver 响应形状，并生成 12 个 content-addressed synthetic evidence reference。它不调用网络、PostgreSQL、`watermark-core`、真实伙伴 endpoint、Secret、审批或外部 provider。

synthetic QA 不得生成 `sandbox_accepted`、production credential、真实合作伙伴 acceptance、可计费用量、SLA 证据、法律结论或真实延迟证据。真实伙伴接入时必须重新执行 12 个场景，不得复用 synthetic evidence。

## 12. 下一 Gate

取得首个真实设计伙伴的外部身份引用、Sandbox endpoint 和 Secret 引用后，生成伙伴专属 bundle，运行 12 场景真实验收并归档不可变 evidence；在此之前保持 production credential、公开 SDK 发布和公网平台 API 关闭。
