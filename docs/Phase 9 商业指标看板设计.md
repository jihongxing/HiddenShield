# Phase 9 商业指标看板设计

本文档定义 HiddenShield 首期商业指标看板的上线口径。目标不是完整 BI 系统，而是为正式收费上线前提供可验证、可追踪、可审计的商业健康视图。

## 1. 设计原则

- 首期指标看板只做聚合统计，不采集原始媒体、加水印后的媒体、本地文件路径、文件名或完整媒体哈希。
- 云端看板负责全局指标：账户、权益分布、支付会话、云同步、L2 视频指纹存证和匿名失败分类。
- 桌面端和移动端负责本机摘要：当前权益、本地批量队列、正式报告最近状态、云同步队列、L2 视频指纹存证次数和匿名失败风险。
- 支付和订阅权益以云端 entitlement 为准，客户端不得根据本机摘要自行开通 Creator / Studio。
- L2 视频只统计指纹存证次数，不上传原始视频、加水印视频、本地路径或可还原画面的素材。

## 2. 云端指标接口

首期后端提供：

`GET /v1/commercial/metrics/overview`

返回字段：

| 字段 | 含义 | 数据源 |
| --- | --- | --- |
| `accounts.totalAccounts` | 总继续账户数 | `cloud_accounts` |
| `accounts.newAccountsToday` | 今日新增继续账户数 | `cloud_accounts.created_at` |
| `accounts.newAccounts7d` | 近 7 日新增继续账户数 | `cloud_accounts.created_at` |
| `entitlementDistribution[]` | Free / Creator / Studio / Enterprise 与状态分布 | `cloud_accounts.entitlement_plan_code/status` |
| `paymentSessions` | created / pending / succeeded / failed / expired / closed 计数 | `billing_payment_sessions` |
| `featureUsage.localBatchUnits` | 本地批量使用计数 | `cloud_usage_ledger.feature_name = local_batch_processing` |
| `featureUsage.reportExportUnits` | 正式报告导出计数 | `cloud_usage_ledger.feature_name = report_export` |
| `featureUsage.l2VideoNotaryCount` | L2 视频指纹存证次数 | `video_fingerprint_notaries` |
| `cloudSync.acceptedEvents` | 云同步成功接收事件数 | `cloud_sync_events` |
| `cloudSync.failureEvents` | 云同步相关匿名失败数 | `feedback_events` |
| `anonymousFailures[]` | 匿名失败按功能和错误码聚合 | `feedback_events` |

接口响应必须包含 `privacyBoundary`，明确：

- `excludesOriginalMedia = true`
- `excludesWatermarkedMedia = true`
- `excludesLocalPaths = true`
- `excludesFileNames = true`
- `excludesFullMediaHashes = true`

## 3. 双端本机摘要

桌面端和移动端设置页展示同一套产品口径：

- 当前权益。
- 本地批量队列数量、验证成功项和失败项。
- 正式报告导出最近状态。
- 云同步成功 / 失败队列状态。
- L2 视频指纹存证次数。
- 支付会话最近状态。
- 隐私边界提示：只展示计数、状态和错误分类，不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希。

桌面端可以读取最近报告导出历史，因此展示最近报告导出次数。移动端目前只展示最近是否发生报告导出，不虚构历史总数；如后续需要精确移动端报告累计，应先扩展本地 usage summary 字段并同步合同测试。

## 4. 上线边界

首期指标看板满足“收费上线前可追踪”的最低要求，并已接入管理员鉴权与访问审计。

`/v1/commercial/metrics/overview` 使用系统配置的管理员 token 鉴权：

- 后端读取 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN`。
- 未配置管理员 token 时，指标接口默认拒绝访问。
- 请求必须使用 `Authorization: Bearer <admin-token>`。
- 成功、未配置、token 缺失或 token 错误都会写入 `admin_audit_events`。
- 审计表只记录 endpoint、outcome、reason 和 occurredAt，不保存 token、不保存媒体、不保存本地路径、不保存文件名或完整媒体哈希。

当前接口不得用于公开用户端页面；建议部署在内部网络或后台管理入口。正式公网部署前，应由运维在系统配置文件或环境变量中注入管理员 token，并限制访问来源。

## 5. 验收标准

- 后端测试覆盖商业指标聚合和隐私边界字段。
- 后端测试覆盖指标接口管理员 token 鉴权、未配置拒绝、错误 token 拒绝和访问审计。
- `commercial:metrics` 合同检查指标文档、后端路由、后端 schema、桌面端设置页、移动端设置页和 Roadmap 回写。
- `commercial:ci` 纳入 `commercial:metrics`。
- Roadmap 中 Phase 9 “指标看板”从未完成变为首期完成，并保留真实微信商户联调、法务审阅和管理员 token 生产配置的上线阻断说明。
