# AI 生成内容标识免费公共 Resolver 合同

状态：`internal_anonymous_public_resolver_gate_passed`

日期：2026-07-28

能力分类：`只能内部测试`

## 1. 目标

提供无需 API key、无需 license ID、无需上传媒体的 confirmed AI transparency record 最小查询接口。

该接口是免费公共验证基础能力，不是批量验证、法律意见、AI 检测器、作者身份判断或付费审计导出。

## 2. 固定端点

- `GET /v1/ai-transparency/public/resolve/watermarks/{watermarkUid}`
- `GET /v1/ai-transparency/public/resolve/manifests/{manifestId}`

请求不得要求：

- `Authorization`
- production credential
- license ID
- tenant/workspace
- 图片或其他媒体上传

## 3. PostgreSQL 公共投影

Migration：`0020_ai_transparency_public_resolver`

Resolver 只能读取：

- `ai_public_confirmed_manifests`
- `ai_public_confirmed_markers`
- `ai_public_confirmed_evidence_summary`

三个 view 只包含 confirmed session/submission 对应的公开字段。未 confirm、marking 中、失败、取消或过期记录不可见。

同一 watermark UID 存在多个 Manifest version 时，Resolver 固定选择 active 优先、version 最高的记录。

Resolver runtime 不查询或写入：

- `ai_marking_ledger`
- license、credential、tenant/workspace
- admission/session 内部标识
- subject digest、confirmation token
- provider/system/model 内部字段
- evidence signature、key、issuer 私有字段
- internal audit projection

## 4. 最小响应

Schema：`hs-ai-public-resolver-v1`

confirmed 响应固定包含：

- `manifestId`
- `watermarkUid`
- `manifestStatus`
- `claimType`
- `markerStatus`
- `metadataSignatureStatus`
- `watermarkDetectionStatus`
- `issuerTrustStatus`
- `evidenceLevel`
- `evidenceVerificationStatus`
- `generatedAt`
- `profiles`
- `markers`
- `legalConclusion=false`
- `warnings`

禁止增加：

- license、tenant/workspace
- marking session、admission
- subject/media digest
- provider/system/model
- ledger/receipt
- credential/token
- 内部 object-store、signer 或 custody receipt

## 5. 结果语义

- `resolutionStatus=confirmed`：存在已 confirm 的 HiddenShield transparency record。
- `resolutionStatus=not_found`：未找到 confirmed record；不表示内容不是 AI 生成。
- `resolutionStatus=unavailable`：Resolver 暂时不可用；不得返回部分成功或推断。
- `issuerTrustStatus=not_evaluated`：当前最小 Resolver 不宣称公共 issuer trust。
- Profile 状态只输出 `applied`、`partially_applied`、`not_applicable`、`configuration_required` 或 `failed`；内部状态不得直接外泄。
- `legalConclusion=false`：所有响应固定不输出法律结论。

## 6. 免费与无计量

- Resolver 无 API key、无 license admission、无 quota 扣减。
- Resolver 不创建 `confirmed_marked_image`、batch verification 或其他 ledger。
- Resolver 不写 platform audit、download audit 或匿名用户追踪记录。
- 可使用 HTTP/CDN cache；当前成功响应 `max-age=60`，not-found `max-age=30`。
- 企业 batch、SLA、Webhook、长期审计和高并发仍属于未来付费接口，不得复用本匿名 endpoint 隐式计费。

## 7. 安全与隐私

- malformed watermark UID 与不存在 UID 均返回相同 not-found 语义。
- 响应设置 `X-Content-Type-Options: nosniff`。
- 允许任意 Origin 的匿名 `GET` CORS，不允许写方法。
- 不接收媒体，因此不存在上传媒体保留。
- 当前 endpoint 不实现法律辖区判断、真人/AI 二分类或版权归属判断。
- 网关级 DDoS、CDN、IP rate limit 和公网隐私日志策略属于部署 Gate，尚未开放。

## 8. QA Gate

一次性 PostgreSQL 16 数据库：

`hiddenshield_migrate_smoke_public_resolver`

已验证：

- 未 confirm 的 watermark UID 返回 `404/not_found`。
- confirm 后可匿名按 watermark UID 查询。
- 同一记录可匿名按 Manifest ID 查询，结果一致。
- 不发送 Authorization。
- response exact-key 校验通过，敏感字段不存在。
- 不存在 UID 返回最小 not-found 响应。
- Resolver 查询后仍只有一个 `confirmed_marked_image` ledger。
- Resolver 查询后 platform internal audit 数量不增加。
- `0001–0020` PostgreSQL up/down migration 与三个 view rollback 通过。

## 9. 发布边界

继续关闭：

- 公网域名和 API gateway。
- CDN、WAF、DDoS 和 IP rate-limit 配置。
- production credential 与 SDK 发布。
- 真实 IAM/KMS/HSM provider 注入。
- 客户 SLA、批量验证与法规合规承诺。

## 10. 下一 Gate

真实设计伙伴接入包已由 `packages/ai-transparency-design-partner-kit` 与 `docs/AI生成内容标识设计伙伴Sandbox接入包合同.md` 冻结并验证。

下一 Gate：取得首个真实伙伴的 Sandbox endpoint、Secret 引用与运行证据，完成 12 场景验收；公网 Resolver、production credential 和客户 SLA 继续关闭。
