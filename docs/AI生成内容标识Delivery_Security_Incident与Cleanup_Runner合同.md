# AI 生成内容标识 Delivery Security Incident 与 Cleanup Runner 合同

状态：**已冻结，internal-only，只能内部测试**

冻结日期：**2026-07-28**

## 1. 目标

本合同把 Delivery Security Observability 的 15 分钟监控摘要推进为可治理的内部安全事件，并提供不依赖外部通知渠道的 PostgreSQL 定时清理执行器。

本能力属于后端安全编排，不是 `watermark-core` 的水印写入、读取或验证结论。

## 2. Incident Projection

- 仅 `monitoring_15m` 且结果为 `warning` 或 `critical` 的摘要创建或合并 incident。
- active incident key 绑定 `tenantId + workspaceId + environment + 排序后的 alertCodes`。
- 相同 active key 并发投影最多保留一个 active incident；新证据增加 occurrence count 并更新 latest summary 绑定。
- severity 只能从 `warning` 升级到 `critical`，不得自动降级。
- resolved incident 不再复用 active key；同类风险再次出现时创建新 incident。
- incident 与 incident audit 不保存媒体 bytes、下载 token、authorizationId、delivery envelope ID、Secret 或完整 provider receipt。
- projection 与 observability snapshot、operations audit 在同一 PostgreSQL 事务内提交。

## 3. Ack / Resolve 四眼命令

操作：

- `ack_delivery_security_incident`
- `resolve_delivery_security_incident`

两项操作必须复用现有：

- `ai_transparency_change_requests`
- `ai_transparency_change_approvals`
- `ai_transparency_change_executions`
- `ai_transparency_change_audit_events`

角色与约束：

- requester：`ai_transparency_requester`
- approver：`ai_transparency_security_approver`
- executor：`system_executor`
- requester 与 approver 必须为不同 actor。
- approval 必须通过 fail-closed Internal IAM 和 security review reference 验真。
- request digest version 固定为 `hs-ai-delivery-security-incident-change-digest-v1`。
- desired state schema 固定为 `hs-ai-delivery-security-incident-desired-state-v1`。
- target scope key 固定为 `delivery_security_incident:{incidentId}`。
- stale control version、状态不匹配、scope 不匹配或重复冲突请求全部 fail-closed。
- execution、incident projection、incident audit 与 change audit 必须位于同一 PostgreSQL 事务。
- execution replay 返回既有成功结果，不重复写 incident audit 或 change audit。

允许状态迁移：

- `open -> acknowledged`
- `open -> resolved`
- `acknowledged -> resolved`

## 4. Cleanup Runner

- 数据库语义固定为 PostgreSQL-only。
- schedule interval 固定为 15 分钟。
- lease 固定为 5 分钟。
- claim 使用 `FOR UPDATE SKIP LOCKED`。
- 同一 scope 只有一个 schedule；并发 runner 最多一个取得 lease。
- 成功后下一次执行时间为 15 分钟后，连续失败计数归零。
- 失败后使用 1、2、4、8、16、32、60 分钟封顶退避。
- runner 只调用既有 delivery security cleanup command，不复制清理规则。
- schedule 配置、claim、成功和失败均写 append-only runner audit。
- runner 不依赖 PagerDuty、邮件或短信 adapter 才能执行。

## 5. 外部依赖挂起

以下 adapter 继续作为外部配置依赖挂起：

- PagerDuty
- 邮件
- 短信

当前实现不得伪造发送成功 receipt，也不得因缺少这些 adapter 阻塞 incident projection、四眼处置或 cleanup runner。

## 6. 发布边界

- SDK：关闭
- 公共 Resolver：关闭
- 生产 credential 发放：关闭
- 客户 UI：关闭
- 当前能力分类：`只能内部测试`

## 7. Gate

- migration up/down 与空 schema rollback 通过。
- 并发 monitoring summary 最多形成一个 active incident。
- ack 与 resolve 完整复用 change request、approval、execution、append-only audit。
- stale version、错误状态、错误 scope 与 IAM/reference 拒绝保持零业务写入。
- resolved 后复发创建新 incident。
- 两个 PostgreSQL runner 并发执行同一 schedule 时最多一个 claim。
- runner audit 与 incident audit 拒绝 UPDATE/DELETE。
- PagerDuty、邮件和短信未配置时，上述内部 Gate 仍可通过。
