# AI 生成内容标识 Delivery Security Observability 合同

状态：`frozen_internal_only_v1`
冻结日期：2026-07-28
适用范围：AI 生成图片标识内部 delivery authorization、revoke、retrieval 与资源预算的安全观测。

## 1. 数据保留

- rate-limit minute window：保留 24 小时。
- security metric snapshot：保留 90 天。
- cleanup 单批最多处理 1,000 个 rate window 和 1,000 个到期 metric snapshot。
- metric snapshot 在 90 天保留期内禁止 UPDATE/DELETE；到期后只能由受控 cleanup command 删除。
- 原始 `ai_delivery_download_audit_events` 继续保持 append-only，本合同不开放其原始导出。

## 2. Monitoring Summary

内部命令：`execute_postgres_generate_delivery_security_summary`

实时监控模式固定为最近 15 分钟，调用方不得改变窗口。

权限：

- Internal IAM 角色：`ai_transparency_readonly_auditor`
- snapshot 必须属于同 tenant/workspace/environment、未过期且角色一致。

输出仅包含聚合计数、alert status/codes、时间窗口、summary ID、scope-bound digest 与 retention expiry。

禁止输出：

- authorization ID
- delivery envelope ID
- execution ID
- token
- bytes
- revoke reason 原文
- Secret 引用
- provider credential 或完整 receipt

## 3. 告警阈值

Critical：

- size-limit、content-type invalid 或 bridge rejected 合计至少 1 次：`delivery_integrity_failure`
- revoked authorization access 在 15 分钟内至少 3 次：`revoked_authorization_access_burst`

Warning：

- rate limited 在 15 分钟内至少 5 次：`delivery_rate_limit_pressure`
- read timeout 与 artifact unavailable 合计至少 3 次：`delivery_artifact_availability_degraded`
- retrieval attempts 至少 10 次，且失败比例至少 20%：`delivery_failure_ratio_elevated`

Critical 优先于 Warning；同一 summary 可以携带多个 alert code。

## 4. 聚合审计导出边界

模式：`audit_export`

- 最小窗口：15 分钟。
- 最大窗口：31 天（44,640 分钟）。
- 只允许 `ai_transparency_readonly_auditor`。
- 只返回与 monitoring 相同的聚合字段。
- `alertStatus` 固定为 `not_evaluated`，避免把长窗口累计值误解释为实时告警。
- 超过 31 天、错误角色或 scope mismatch 均零 snapshot、零导出。
- raw audit events 永远不通过该命令导出。

## 5. Summary Digest

SHA-256 固定绑定：

- tenant/workspace/environment
- mode
- window start/end
- 全部聚合计数
- alert status
- alert codes

相同计数但不同作用域不得产生相同语义摘要。

## 6. Cleanup Command

内部命令：`execute_postgres_cleanup_delivery_security_windows`

权限：

- Internal IAM 角色：`system_executor`
- system snapshot 必须属于同 tenant/workspace/environment 且未过期。

行为：

1. 使用 `FOR UPDATE SKIP LOCKED` claim 24 小时以前的 rate window。
2. 删除 retention expiry 已到期的 metric snapshot。
3. 每类最多处理 1,000 条。
4. 双连接并发 cleanup 的删除总数必须等于唯一到期记录数，不允许重复删除。
5. 每次执行写入 append-only operations audit，即使删除数为零。

## 7. Operations Audit

表：`ai_delivery_security_operations_audit_events`

事件：

- `delivery_security_summary_generated`
- `delivery_security_audit_summary_exported`
- `delivery_rate_limit_cleanup`

允许记录聚合数量、summary ID/digest、窗口参数、保留参数和 actor snapshot。

禁止记录 raw download audit、token、bytes、Secret、媒体标识或完整 provider receipt。

## 8. 数据库

迁移：`0015_ai_transparency_delivery_security_observability`

新增：

- `ai_delivery_security_observability_snapshots`
- `ai_delivery_security_operations_audit_events`
- scope/time、retention 和 operations audit 索引
- metric retention mutation guard
- operations audit append-only trigger

## 9. Gate 证据

- fixture：`docs/contracts/ai-transparency-delivery-retrieval/security-observability-v1.fixture.json`
- PostgreSQL 16 smoke：41 tables、53 indexes、0001–0015 up/down 与空 schema rollback。
- PostgreSQL QA：
  - 真实 delivery 失败事件汇总为 critical；
  - 90 天 retention；
  - 保留期内 UPDATE/DELETE 拒绝；
  - 31 天 aggregate-only export；
  - 超窗与错误角色零写入；
  - 双连接 cleanup 唯一删除；
  - operations audit UPDATE/DELETE 拒绝。

## 10. 能力边界

当前分类仍为 `只能内部测试`。没有外部告警渠道、客户仪表盘、SDK、公共 Resolver、客户下载 API、生产 credential 或生产发放。

## 11. 下一 Gate

实现 internal delivery security incident projection、ack/resolve command 与定时 cleanup runner；外部 PagerDuty/邮件/短信通知 adapter 继续作为外部配置依赖挂起。
