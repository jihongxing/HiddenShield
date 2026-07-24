# Phase 8 支付与订阅状态闭环设计

## 1. 目标

Phase 8 的目标不是再新增一个“订阅展示页”，而是把 Free / Creator / Studio / Enterprise 从静态展示推进到真实可更新、可恢复、可过期的权益闭环。

完成后必须满足：

- 桌面端、移动端、云端读取同一份 `entitlement` 快照。
- 支付状态只由云端 webhook 写入，客户端不能自行修改正式权益。
- 订阅过期、宽限期、恢复订阅、退款撤销会统一更新 `entitlement.features`。
- 本地批量、云同步、正式报告、团队空间、云端视频都继续只看 feature map 门禁。
- 不同步原始媒体、加水印媒体、本地文件路径。

## 2. Provider 抽象与首期选择

Phase 8 不能把支付闭环写死到某一个 provider。后端必须先抽象统一 `BillingProvider` 适配层，再接具体支付渠道。

首期 provider：微信支付 APIv3。

原因：

- 当前首要商业化场景面向国内用户，微信支付的支付完成率和用户熟悉度更高。
- 桌面端可以使用 Native 扫码支付，移动端可以使用 H5 / App 支付，双端都不需要保存支付凭证。
- 微信支付支持支付结果异步回调通知，适合写入 `subscription_events` 幂等事件账本，再驱动 `entitlements`。
- 微信支付官方文档要求回调验签、金额校验和重复通知幂等处理，正好可以作为 Phase 8 的首个 provider 契约。

扩展 provider：

- Stripe：海外用户和国际信用卡场景，后续作为 `billing_source = "stripe"` 扩展。
- Paddle / Lemon Squeezy：可作为海外税务或轻量出海备选。
- Apple / Google 内购：如果移动端上架应用商店，必须单独作为 `billing_source = "app_store"` / `billing_source = "play_billing"`，不能复用微信支付或 Stripe webhook 语义。
- 企业合同：作为 `billing_source = "manual_grant"`，由后台人工授权并写入审计日志。

首期决策：

- 支付抽象层字段使用 provider 中立命名：`provider`、`provider_customer_id`、`provider_subscription_id`、`provider_price_id`、`provider_order_id`、`provider_transaction_id`。
- 首期 `billing_source = "wechat_pay"`。
- 客户端不保存支付凭证，不保存商户密钥，不处理 provider secret。
- 后端保存 provider 映射、订单号、交易号、订阅周期和状态。
- 支付入口统一由后端返回 `paymentAction`，桌面端和移动端根据 action 类型展示扫码、打开外部支付页或调起系统能力。

### 2.1 `BillingProvider` 适配层

后端只允许业务层调用统一接口：

```text
BillingProvider {
  create_payment_session(account, workspace, plan, billing_cycle) -> PaymentSession
  verify_webhook(headers, body) -> ProviderEvent
  normalize_event(provider_event) -> BillingEvent
  query_order(provider_order_id) -> BillingEvent
  close_order(provider_order_id) -> CloseOrderResult
}
```

业务层只理解 `BillingEvent`：

```text
BillingEvent(
  provider,
  provider_event_id,
  provider_order_id,
  provider_transaction_id,
  account_id,
  workspace_id,
  plan_code,
  billing_cycle,
  amount_cents,
  currency,
  event_type,
  paid_at,
  raw_payload_json
)
```

规则：

- provider 事件必须先进入 `subscription_events`，再更新 `subscriptions` 和 `entitlements`。
- webhook、主动查单、后台人工补单都必须收敛成同一种 `BillingEvent`。
- `price_id -> plan_code` 或 `product_code -> plan_code` 只能来自服务端 allowlist。
- 业务代码不得出现微信支付、Stripe 等 provider 专属字段分支；分支只能存在于 provider adapter。

## 3. 套餐映射

| plan_code | plan_name | provider product / price | status 来源 | feature map |
| --- | --- | --- | --- | --- |
| `free` | 免费版 | 无 | 默认账户 | 全 false |
| `creator` | Creator | `WECHAT_PRODUCT_CREATOR_*` / `STRIPE_PRICE_CREATOR_*` | subscription | `cloud_sync`、`batch_processing`、`report_export` |
| `studio` | Studio | `WECHAT_PRODUCT_STUDIO_*` / `STRIPE_PRICE_STUDIO_*` | subscription | Creator + `cloud_batch_processing`、`priority_queue`、`team_workspace` |
| `enterprise` | Enterprise | 合同 / 手工开通 | admin grant | Studio + `cloud_video_processing`、`api_access` |

首期只需要自动化 Creator / Studio。Enterprise 可先保留后台手工授权。

## 4. 数据模型

### 4.1 `billing_customers`

```text
billing_customers(
  account_id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  provider_customer_id TEXT NOT NULL,
  email TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

约束：

- `provider + provider_customer_id` 唯一。
- 一个账户首期只绑定一个支付 provider。

### 4.2 `subscriptions`

```text
subscriptions(
  subscription_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider TEXT NOT NULL,
  provider_subscription_id TEXT NOT NULL,
  provider_price_id TEXT NOT NULL,
  provider_product_id TEXT,
  provider_order_id TEXT,
  provider_transaction_id TEXT,
  plan_code TEXT NOT NULL,
  status TEXT NOT NULL,
  current_period_started_at TEXT,
  current_period_ends_at TEXT,
  trial_started_at TEXT,
  trial_ends_at TEXT,
  grace_ends_at TEXT,
  cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
  canceled_at TEXT,
  latest_invoice_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

`status` 使用现有枚举：

- `trial`
- `active`
- `grace`
- `expired`

`free` 不是 subscription 状态，而是账户没有有效订阅时的 entitlement 状态。

### 4.3 `subscription_events`

```text
subscription_events(
  event_id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  provider_event_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  account_id TEXT,
  provider_customer_id TEXT,
  provider_subscription_id TEXT,
  provider_order_id TEXT,
  provider_transaction_id TEXT,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL,
  processed_at TEXT,
  processing_status TEXT NOT NULL,
  processing_error TEXT
)
```

约束：

- `provider + provider_event_id` 唯一，保证 webhook 幂等。
- `payload_json` 只保存 provider 事件，不保存银行卡信息。
- 所有 entitlement 更新必须能追溯到 `subscription_events.event_id`。

### 4.4 `entitlements`

当前后端 `cloud_accounts` 已有 entitlement 快照字段，Phase 8 应收敛成独立概念：

```text
entitlements(
  entitlement_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  plan_code TEXT NOT NULL,
  plan_name TEXT NOT NULL,
  status TEXT NOT NULL,
  features_json TEXT NOT NULL,
  billing_source TEXT,
  subscription_id TEXT,
  trial_started_at TEXT,
  trial_ends_at TEXT,
  current_period_started_at TEXT,
  current_period_ends_at TEXT,
  grace_ends_at TEXT,
  last_provider_event_id TEXT,
  updated_at TEXT NOT NULL
)
```

兼容策略：

- 首期可以继续把 entitlement 快照冗余写回 `cloud_accounts`，保证现有 `POST /v1/auth/continue` 不破坏。
- 但 webhook 的最终写入目标应是 `entitlements`，`cloud_accounts` 只作为兼容投影。

## 5. 后端 API

### 5.1 创建支付会话

`POST /v1/billing/payment-sessions`

请求：

```json
{
  "accountId": "acct_xxx",
  "workspaceId": "ws_xxx",
  "planCode": "creator",
  "billingCycle": "monthly",
  "successUrl": "hiddenshield://billing/success",
  "cancelUrl": "hiddenshield://billing/cancel",
  "preferredProvider": "wechat_pay"
}
```

响应：

```json
{
  "paymentSessionId": "pay_sess_xxx",
  "provider": "wechat_pay",
  "providerOrderId": "out_trade_no_xxx",
  "paymentAction": {
    "type": "qr_code",
    "qrCodeUrl": "weixin://wxpay/bizpayurl?...",
    "expiresAt": "2026-06-19T10:00:00Z"
  },
  "expiresAt": "2026-06-19T10:00:00Z"
}
```

规则：

- Free 用户可创建 Creator / Studio 支付会话。
- 已有 active subscription 的用户应优先返回订阅管理入口或当前订阅状态，不重复创建同套餐订单。
- `planCode` 必须来自服务端 allowlist，不能信任客户端传 provider product id / price id。
- 桌面端优先返回 `paymentAction.type = "qr_code"`，用于微信 Native 扫码支付。
- 移动端可返回 `h5_url` 或 `app_pay_payload`，由 provider adapter 根据平台能力决定。

### 5.2 打开订阅管理

`POST /v1/billing/portal-sessions`

响应：

```json
{
  "provider": "wechat_pay",
  "managementAction": {
    "type": "in_app_status",
    "message": "当前订阅由微信支付开通，可在本页续费、取消自动续费或联系客服处理退款。"
  }
}
```

规则：

- 只允许已登录账户创建。
- 微信支付首期没有 Stripe Customer Portal 这类现成托管页面，后端必须提供 provider 中立的订阅状态和操作入口。
- 如果后续 provider 支持托管管理页，可以返回 `managementAction.type = "external_url"`。

### 5.3 当前权益

`GET /v1/entitlements/current`

响应继续沿用：

```json
{
  "id": "ent_xxx",
  "planName": "Creator",
  "planCode": "creator",
  "status": "active",
  "features": {
    "cloud_sync": true,
    "batch_processing": true,
    "report_export": true,
    "cloud_batch_processing": false,
    "cloud_video_processing": false,
    "priority_queue": false,
    "team_workspace": false,
    "api_access": false
  },
  "billingSource": "wechat_pay",
  "subscriptionId": "sub_xxx",
  "currentPeriodStartedAt": "2026-06-01T00:00:00Z",
  "currentPeriodEndsAt": "2026-07-01T00:00:00Z",
  "graceEndsAt": null,
  "lastCheckedAt": "2026-06-19T10:00:00Z",
  "updatedAt": "2026-06-19T10:00:00Z"
}
```

### 5.4 Provider Webhook

`POST /v1/billing/webhooks/{provider}`

首期：

- `POST /v1/billing/webhooks/wechat-pay`

后续：

- `POST /v1/billing/webhooks/stripe`

必须验证：

- provider signature。
- event id 幂等。
- order/customer/subscription 能映射到 account。
- product id / price id 能映射到 plan_code。
- 微信支付回调必须校验商户号、应用号、订单号、金额、币种和支付状态。

必须拒绝：

- 未知 product id / price id。
- 账户映射缺失且无法从支付会话 metadata 找回。
- 重放但 payload 不一致的事件。

### 5.5 支付状态补偿

支付完成不能只依赖客户端手动刷新，也不能只依赖 provider webhook。生产环境必须增加后端支付状态补偿机制，详见 `docs/Phase 8 支付状态补偿机制设计.md`。

新增后端能力：

- 持久化 `billing_payment_sessions`，记录 payment session、provider order、套餐、金额、状态、检查次数和下次检查时间。
- `GET /v1/billing/payment-sessions/{paymentSessionId}` 查询支付会话状态。
- `POST /v1/billing/payment-sessions/{paymentSessionId}:reconcile` 手动触发查单补偿。
- 后台任务 `reconcile_pending_payment_sessions(now, limit)` 自动补偿 pending / created 支付会话。

补偿规则：

- webhook、主动查单、后台补偿、人工补单都必须生成 provider 中立 `BillingEvent`。
- 查单成功后必须复用 `apply_billing_event`，不能绕过状态机直接写 entitlement。
- 客户端只读取补偿状态和 entitlement，不自行判定支付成功。
- 真实微信查单只允许存在于 `WechatPayNativeAdapter`，业务层只消费 `BillingOrderStatus`。

## 6. Webhook 状态机

| 标准 BillingEvent | subscription.status | entitlement.status | feature map |
| --- | --- | --- | --- |
| `payment.succeeded` | `trial` 或 `active` | `trial` 或 `active` | 按 plan 开启 |
| `subscription.renewed` | `active` | `active` | 按 plan 开启 |
| `subscription.trialing` | `trial` | `trial` | 按 plan 开启 |
| `payment.failed` | `grace` | `grace` | Creator / Studio 暂保留，显示宽限期 |
| `subscription.canceled` / `subscription.expired` | `expired` | `expired` | 降级为 Free feature map |
| `refund.succeeded` / dispute lost | `expired` | `expired` | 降级为 Free feature map |

微信支付首期事件映射：

- 支付通知 `trade_state=SUCCESS` -> `payment.succeeded`。
- 续费订单支付成功 -> `subscription.renewed`。
- 用户取消自动续费或后台取消 -> `subscription.canceled`。
- 退款成功通知 -> `refund.succeeded`。

Stripe 后续事件映射：

- `checkout.session.completed` -> `payment.succeeded`。
- `invoice.paid` -> `subscription.renewed`。
- `invoice.payment_failed` -> `payment.failed`。
- `customer.subscription.deleted` -> `subscription.expired`。
- `charge.refunded` / dispute lost -> `refund.succeeded`。

宽限期策略：

- `grace` 保留已订阅功能 3 天。
- `grace_ends_at` 过后，如果没有恢复付款，定时任务将 entitlement 改为 `expired`。
- `expired` 后 feature map 必须降级为 Free。

取消策略：

- `cancel_at_period_end=true` 时，在 `current_period_ends_at` 前仍为 `active`。
- 到期收到 provider 取消 / 过期事件后转为 `expired`。

退款策略：

- 全额退款或争议失败：立即 `expired`。
- 部分退款：首期不自动变更 entitlement，只记录事件，后台人工确认。

## 7. 双端同步规则

### 7.1 桌面端

- 登录 / 继续账户后读取云端 entitlement 快照。
- App 启动、打开设置、点击同步、点击批量、点击正式报告、点击视频云能力前，应尽量刷新 entitlement。
- 离线时可使用本地缓存，但云端能力必须在请求服务端时再次校验。
- `expired` 后：
  - `cloud_sync=false`：停止上传 / 拉取，保留本地队列。
  - `batch_processing=false`：不允许创建新批量队列，已有队列暂停。
  - `report_export=false`：只允许基础摘要。
  - `team_workspace=false`：团队入口只展示订阅说明。

### 7.2 移动端

- 与桌面端相同，只消费 `CloudEntitlement`。
- 移动端不能自行把 Free 改成 Creator。
- 离线缓存可以显示“上次权益”，但执行 Creator / Studio 能力前必须检查 `features`。
- `continueWithAccount` 返回的 entitlement 是移动端唯一的正式权益来源。

### 7.3 云端

- 所有付费能力最终都由服务端校验 feature map。
- 客户端 UI 门禁只是体验优化，不作为安全边界。
- `cloud_sync`、`report_export`、`team_workspace`、`cloud_video_processing` 的服务端检查必须使用最新 entitlement 投影。

## 8. UI 行为

桌面端：

- 订阅页显示当前套餐、状态、周期结束时间、宽限期结束时间。
- Free 点击升级：请求 payment session，首期显示微信支付二维码。
- 支付会话创建后显示“刷新权益”，并预留轻量轮询 payment session 状态；轮询只调用后端 session / reconcile API，不自行开通权益。
- Active / Trial / Grace 点击管理：请求 provider 中立订阅管理入口，微信支付首期显示续费、取消自动续费、联系客服处理退款等状态操作。
- Expired 显示“恢复订阅”，重新创建 payment session。

移动端：

- 同样显示套餐、状态、周期和宽限期。
- 首期不做内购，优先走微信 H5 / App 支付；如果 provider adapter 只返回二维码，则移动端显示可保存或长按识别的二维码。
- 支付会话创建后与桌面端一致，只读取后端补偿状态和 entitlement，不自行判定支付成功。
- 如果后续上架应用商店，再新增 `billing_source=app_store` / `billing_source=play_billing`，不能复用微信支付或 Stripe webhook 语义。

## 9. 安全与合规

- Webhook endpoint 必须验证 provider signature。
- `product_id / price_id -> plan_code` 只能来自服务端配置。
- Payment session metadata 必须包含 `account_id`、`workspace_id`、`requested_plan_code`。
- 不保存银行卡、支付方式完整信息。
- 不在客户端保存 provider secret。
- 微信支付商户私钥、APIv3 key、平台证书只能存在服务端密钥配置。
- 微信支付回调必须校验签名、订单号、金额、币种、商户号和应用号。
- Webhook payload 保留最小必要字段，避免长期保存敏感支付明细。
- 后台人工授权 Enterprise 必须写入 audit log。

## 10. 验收标准

- `POST /v1/billing/payment-sessions` 只能创建 allowlist 套餐。
- provider adapter 是唯一允许出现微信支付 / Stripe 专属字段的层。
- 微信支付 webhook 重放不会重复更新 entitlement。
- 微信支付支付成功通知能把 Free 更新为 Creator / Studio。
- 续费失败或过期检查能进入 `grace`。
- 取消 / 过期事件能进入 `expired` 并关闭 feature map。
- 退款成功事件能立即降级为 Free feature map。
- 桌面端和移动端刷新后看到相同 entitlement。
- webhook 延迟或丢失时，后端查单补偿能把支付成功订单转为同一条 `BillingEvent` 状态机。
- `cloud_sync`、`batch_processing`、`report_export`、`team_workspace` 的门禁随 entitlement 变化。
- CI 增加 `billing:contract`，固定 provider 抽象、微信支付首期、API、状态机和双端字段。

## 11. 实施顺序

1. 新增 `billing:contract` 静态契约，固定 Phase 8 API、表、状态机和双端字段。
2. 后端新增 provider 中立的 `BillingProvider` trait / interface 和本地 provider fixture。
3. 后端新增 `billing_customers`、`subscriptions`、`subscription_events`、`entitlements` schema。
4. 后端新增 payment session / subscription management API。
5. 后端新增微信支付 adapter：下单、回调验签、幂等、查单和退款事件映射。
5. `POST /v1/auth/continue` 和 `GET /v1/entitlements/current` 返回 webhook 更新后的 entitlement。
6. 桌面端订阅页接入 payment session 和二维码支付完成轮询 / 刷新。
7. 移动端订阅页接入 payment session 和微信 H5 / App 支付入口。
8. 新增支付状态补偿机制：payment session 账本、查单补偿 API、后台补偿任务和 fixture e2e。
9. 增加过期、宽限期、恢复订阅的端到端测试。

## 12. 当前结论

Phase 8 可以开始实现，但必须先落 provider 抽象和 `billing:contract`，再写后端 schema 和微信支付 adapter。不要先从桌面端或移动端做“手动升级按钮”，否则会再次产生端侧权益与云端权益分叉。
