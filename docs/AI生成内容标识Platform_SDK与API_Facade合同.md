# HiddenShield AI 生成内容标识 Platform SDK 与 API Facade 合同

更新时间：2026-07-28

状态：`internal_sdk_gate`

能力分类：`只能内部测试`

## 1. 产品边界

`@hiddenshield/ai-transparency-sdk` 是授权使用的服务端付费组件，面向 AI 图片生成平台的可信后端。

SDK：

- 只负责编排、合同校验和调用 HiddenShield 平台 API。
- 不实现盲水印算法，不替代 `watermark-core`。
- 不保存生产长期 credential。
- 不允许生产 credential 进入浏览器、桌面安装包或移动端应用。
- 不提供 AI 检测或法律结论。

## 2. 最小流程

固定顺序：

1. `admitProductionProfile`
2. `createGenerationSession`
3. `submitGeneratedImage`
4. `confirmGeneratedAsset`

平台 facade 的 `markAndConfirmGeneratedImage` 必须按相同顺序调用 SDK。任一步失败后立即停止，后续步骤不得执行。

## 3. Production Admission

请求必须包含：

- `licenseId`
- `tenantId`
- `workspaceId`
- `issuerMode`
- `regulatoryProfileId`
- `technicalProfileIds`
- `mediaType=image`
- `environment=production`

响应必须绑定：

- `admissionId`
- `licenseId`
- `entitlementVersionId`
- `entitlementDigest`
- Profile identity
- `expiresAt`
- `status=admitted`

SDK 不允许调用方绕过 admission 直接创建 session。

## 4. Marking Session

请求必须包含：

- `admissionId`
- `idempotencyKey`
- `subjectReference`
- `generationEventId`
- `contentType=image/png`

响应：

- `markingSessionId`
- `watermarkUid`
- `status=ready_to_upload`
- `expiresAt`

SDK 必须验证 admission 未过期，并拒绝 session/admission identity 不一致。

## 5. 图片提交

SDK 接收最终 PNG bytes，计算 SHA-256，并提交：

- `markingSessionId`
- `image/png`
- `originalFileSha256`
- 原始 bytes

V1 SDK 输入上限固定为 64 MiB；超限在 transport 调用前 fail-closed。

响应必须包含：

- `status=ready_to_confirm`
- `markedImageBytes`
- `markedFileSha256`
- `confirmationToken`
- `markerEvidenceDigest`
- `explicitLabelReceiptDigest`

SDK 必须重新计算返回图片摘要；摘要不匹配时 fail-closed，不进入 confirm。

## 6. Confirm 与计量

confirm 请求绑定：

- `markingSessionId`
- `confirmationToken`
- `markedFileSha256`
- `idempotencyKey`

成功响应必须为：

- `status=confirmed`
- `manifestId`
- `watermarkUid`
- `verificationUrl`
- `profileStatus=applied`
- `meteringReceipt.meteringUnit=confirmed_marked_image`
- `meteringReceipt.quantity=1`
- `meteringReceipt.ledgerStatus=committed`

失败、图片提交失败、摘要不匹配和重复 confirm 不得生成新的 `confirmed_marked_image` receipt。

## 7. Fail-Closed 错误模型

错误返回 `AiTransparencySdkError`：

- `code`
- `category`
- `retryable`
- `httpStatus`
- `requestId`

固定类别：

- `authorization`
- `entitlement`
- `validation`
- `conflict`
- `integrity`
- `availability`
- `internal`

未知响应、非 JSON、超时、网络失败、字段缺失、状态不匹配和摘要不匹配均抛出错误，不返回部分成功结果。

## 8. Credential

- SDK 配置必须提供服务端 production credential。
- credential 只进入 `Authorization: Bearer`。
- SDK error、日志、receipt 和 facade response 不得回显 credential。
- production base URL 必须使用 HTTPS；仅测试 transport 可绕过网络。

## 9. API Facade

最小 facade 路径语义：

- `POST /v1/ai-transparency/admissions`
- `POST /v1/ai-transparency/sessions`
- `POST /v1/ai-transparency/images/mark`
- `POST /v1/ai-transparency/images/confirm`

本阶段 facade 为框架无关 handler，由平台服务注入 request/context 并调用 SDK。真实公网路由、认证网关和生产 credential 发放继续关闭。

## 10. 当前 Gate

当前 Gate 验证：

- production admission 后才能创建 session。
- 图片 SHA-256 本地计算与服务端返回值双绑定。
- mark 失败或摘要 mismatch 时 confirm 零调用。
- confirm 成功只接受 `confirmed_marked_image + quantity=1 + committed`。
- duplicate confirm replay 不重复计量。
- license/Profile/credential/availability/integrity 错误全部 fail-closed。

## 11. 下一 Gate

SDK、facade 与 PostgreSQL internal endpoint Gate 已于 2026-07-28 通过：

1. 四个固定端点由独立 PostgreSQL-only Axum router 提供。
2. session 复用 credential custody，admission 绑定 versioned Profile entitlement。
3. mark 复用 image marking executor / `watermark-core`，只推进到 `ready_to_confirm`。
4. confirm 复用 PostgreSQL confirm command，原子写 Manifest 与 `confirmed_marked_image` ledger。
5. SDK → platform facade → HTTP → Axum → PostgreSQL 真实 E2E 与 duplicate replay 已通过。

详细合同见 `docs/AI生成内容标识PostgreSQL_Platform_API合同.md`。

免费公共 Resolver 最小只读 Gate 已完成；SDK 发布、production credential 和公网平台 API 继续关闭。

真实设计伙伴 sandbox 接入包和验收矩阵已冻结，详细合同见 `docs/AI生成内容标识设计伙伴Sandbox接入包合同.md`。

下一 Gate：注入首个真实伙伴的 Sandbox endpoint 与 Secret 引用，完成 evidence-backed 验收；SDK 公开发布和 production credential 继续关闭。
