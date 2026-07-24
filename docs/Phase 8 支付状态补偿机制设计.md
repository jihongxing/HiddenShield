# Phase 8 支付状态补偿机制设计

## 1. 背景

当前 Phase 8 已完成：

- provider 抽象层。
- 微信支付 Native 下单。
- 微信支付 webhook 验签、解密、金额校验与 `BillingEvent` 映射。
- `GET /v1/entitlements/current` 云端权益刷新。
- 桌面端和移动端支付后手动刷新权益。

但仍有一个生产级风险：

> 用户完成支付后，如果 provider webhook 延迟、丢失或后端短暂不可用，客户端刷新 entitlement 仍可能看到 Free，导致“已付款但权益未生效”。

因此需要支付状态补偿机制。补偿机制必须由后端执行，客户端只能触发刷新、显示状态或进行轻量轮询，不能自行判定支付成功，也不能自行修改正式权益。

## 2. 目标

支付状态补偿机制要解决三类问题：

- webhook 延迟：用户支付成功，但 webhook 尚未处理。
- webhook 丢失：provider 已成功支付，但后端没有收到事件。
- 客户端误解：用户刷新权益时，后端必须能区分“未支付”“支付处理中”“已支付待补偿”“已生效”。

完成后必须满足：

- 支付会话创建后，后端有可追踪的 payment session / provider order 记录。
- 补偿任务通过 provider adapter 的 `query_order` 查询订单，统一转为 `BillingEvent`。
- webhook、查单补偿、后台人工补单都走同一个 `apply_billing_event` 状态机。
- 客户端只展示补偿状态和刷新结果，不保存 provider secret。
- `billing:contract` 固定该补偿边界，避免后续把成功判断写回客户端。

## 3. 核心原则

### 3.1 云端权威

正式权益只来自云端 `entitlements`。

客户端行为：

- 可以创建支付会话。
- 可以展示二维码 / H5 支付动作。
- 可以点击“刷新权益”。
- 可以在支付会话有效期内轻量轮询补偿状态。

客户端禁止：

- 不能根据二维码打开、H5 返回、用户点击“已支付”直接开通 Creator / Studio。
- 不能保存商户密钥、APIv3 key、provider secret。
- 不能直接写正式 `entitlements`。

### 3.2 Provider Adapter 封装

所有 provider 专属查单逻辑只能存在于 provider adapter。

业务层只消费：

```text
BillingEvent
BillingOrderStatus
BillingPaymentSession
```

业务层不得出现微信支付 `trade_state`、Stripe `payment_intent` 这类 provider 专属字段分支。

### 3.3 一条状态机

无论事件来源是什么：

- webhook
- 主动查单
- 定时补偿
- 后台人工补单

最终都必须生成 provider 中立 `BillingEvent`，并复用 `apply_billing_event`。

## 4. 数据模型补充

### 4.1 `billing_payment_sessions`

新增支付会话账本：

```text
billing_payment_sessions(
  payment_session_id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  provider_order_id TEXT NOT NULL,
  account_id TEXT NOT NULL,
  workspace_id TEXT NOT NULL,
  plan_code TEXT NOT NULL,
  billing_cycle TEXT NOT NULL,
  amount_cents INTEGER NOT NULL,
  currency TEXT NOT NULL,
  status TEXT NOT NULL,
  payment_action_json TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  last_provider_event_id TEXT,
  last_provider_transaction_id TEXT,
  last_checked_at TEXT,
  next_check_after TEXT,
  check_attempts INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(provider, provider_order_id)
)
```

`status` 枚举：

- `created`：支付动作已创建，尚未确认支付结果。
- `pending`：provider 返回仍在支付中。
- `succeeded`：已确认支付成功，并已写入 entitlement。
- `failed`：provider 明确失败或关闭。
- `expired`：会话过期，未确认成功。
- `compensation_required`：需要后端查单补偿。

规则：

- payment session 必须绑定 `account_id`、`workspace_id`、`plan_code`、`billing_cycle`。
- 创建真实微信 Native 订单后必须保存 `provider_order_id`，不能只返回给客户端。
- `payment_action_json` 只保存二维码 / H5 URL 等动作信息，不保存密钥。
- `amount_cents`、`currency` 来自服务端套餐 allowlist，不信任客户端。

### 4.2 `subscription_events` 来源标记

当前 `subscription_events` 已能记录 provider event。补偿机制需要在 payload 中保留来源：

```json
{
  "source": "webhook | order_query | manual_grant",
  "providerOrderId": "hs_xxx",
  "providerTransactionId": "420000...",
  "raw": {}
}
```

首期可以不新增列，但必须保证 `provider_event_id` 可区分来源：

- webhook：使用 provider 原始 event id。
- 查单补偿：使用 `order_query:{provider}:{provider_order_id}:{provider_transaction_id}`。
- 人工补单：使用 `manual:{provider}:{provider_order_id}:{operator_id}:{timestamp}`。

## 5. 后端 API 补充

### 5.1 查询支付会话状态

`GET /v1/billing/payment-sessions/{paymentSessionId}`

返回：

```json
{
  "paymentSessionId": "pay_sess_xxx",
  "provider": "wechat_pay",
  "providerOrderId": "hs_xxx",
  "status": "pending",
  "planCode": "creator",
  "billingCycle": "monthly",
  "expiresAt": "2026-06-19T10:00:00Z",
  "lastCheckedAt": "2026-06-19T09:56:00Z",
  "nextCheckAfter": "2026-06-19T09:56:10Z",
  "entitlement": {
    "planCode": "free",
    "status": "free"
  }
}
```

用途：

- 客户端支付后轻量轮询。
- 客服排查支付会话。
- 测试 fixture 支付闭环。

### 5.2 手动触发补偿

`POST /v1/billing/payment-sessions/{paymentSessionId}:reconcile`

行为：

1. 校验 bearer token。
2. 校验 payment session 属于当前 account。
3. 检查 `next_check_after`，避免高频刷 provider。
4. 调用 provider adapter `query_order(provider_order_id)`。
5. 如果 provider 返回支付成功，转换为 `BillingEvent(payment.succeeded)`。
6. 调用 `apply_billing_event`。
7. 更新 `billing_payment_sessions.status = succeeded`。
8. 返回最新 payment session 状态和 entitlement。

返回：

```json
{
  "paymentSessionId": "pay_sess_xxx",
  "status": "succeeded",
  "message": "支付已确认，权益已生效。",
  "entitlement": {
    "planCode": "creator",
    "status": "active",
    "features": {
      "cloud_sync": true,
      "batch_processing": true,
      "report_export": true
    }
  }
}
```

### 5.3 后台补偿任务

内部任务，不对客户端开放：

```text
reconcile_pending_payment_sessions(now, limit)
```

扫描范围：

- `status IN ('created', 'pending', 'compensation_required')`
- `expires_at > now - 24h`
- `next_check_after IS NULL OR next_check_after <= now`
- `check_attempts < max_attempts`

退避策略：

- 支付会话创建后 0-2 分钟：10 秒一次。
- 2-15 分钟：30 秒一次。
- 15 分钟到 24 小时：5 分钟一次。
- 超过 24 小时仍未成功：标记 `expired`，保留人工排查入口。

## 6. Provider 查单语义

统一返回：

```text
BillingOrderStatus {
  provider,
  provider_order_id,
  provider_transaction_id,
  account_id,
  workspace_id,
  plan_code,
  billing_cycle,
  amount_cents,
  currency,
  status,
  paid_at,
  raw_payload_json
}
```

`status` 枚举：

- `not_found`
- `pending`
- `succeeded`
- `failed`
- `closed`
- `refunded`

映射规则：

- `succeeded` -> `BillingEvent(payment.succeeded)`
- `refunded` -> `BillingEvent(refund.succeeded)`
- `failed` / `closed` -> 更新 payment session，不直接改 entitlement
- `pending` -> 更新 payment session，继续退避检查
- `not_found` -> 继续退避，直到过期

## 7. 微信支付首期查单边界

首期设计使用微信支付商户订单号查单：

```text
GET /v3/pay/transactions/out-trade-no/{out_trade_no}?mchid={mchid}
```

必须校验：

- `appid` 与服务端配置一致。
- `mchid` 与服务端配置一致。
- `out_trade_no` 与 payment session 一致。
- `amount.total` 与服务端套餐金额一致。
- `amount.currency = CNY`。
- `attach` 中的 `accountId / workspaceId / planCode / billingCycle` 与 payment session 一致。

微信状态映射：

- `SUCCESS` -> `succeeded`
- `NOTPAY` / `USERPAYING` -> `pending`
- `CLOSED` / `REVOKED` / `PAYERROR` -> `failed`
- `REFUND` -> `refunded`

本阶段不做真实微信商户联调，但 adapter 设计必须为该 API 预留。

## 8. 客户端行为

### 8.1 桌面端

支付会话创建后：

- 显示支付动作。
- 显示“刷新权益”。
- 预留轻量轮询状态：前 2 分钟每 10 秒查询 payment session；超时后停止自动轮询，提示用户稍后手动刷新。
- 轮询只调用后端 session / reconcile API，不自行判断支付成功。

完成态文案：

- `succeeded`：支付已确认，权益已生效。
- `pending`：尚未确认支付完成，请完成支付或稍后刷新。
- `expired`：支付会话已过期，请重新创建支付。
- `failed`：支付未完成，请重新创建支付。

### 8.2 移动端

移动端与桌面端一致。

限制：

- 首期不做 App Store / Google Play 内购。
- 如果 provider 返回二维码，移动端只展示二维码或可打开链接，不自行识别支付结果。
- 如果后续进入应用商店，必须新增 `app_store` / `play_billing` provider，不复用微信支付状态机。

## 9. 监控与人工处理

需要记录指标：

- payment session 创建数。
- webhook 成功处理数。
- 查单补偿成功数。
- webhook 后仍需补偿的比例。
- 用户支付后 2 分钟内权益生效率。
- pending 超过 15 分钟的订单数。
- expired 但用户投诉的订单数。

人工处理入口：

- 通过 provider order id 查询 payment session。
- 查看 subscription_events。
- 手动触发 reconcile。
- 必要时创建 `manual_grant`，但必须写 audit log。

## 10. 验收标准

- 创建 payment session 后，后端必须持久化 provider order。
- webhook 与查单补偿生成的 `BillingEvent` 必须幂等。
- 查单成功后必须复用 `apply_billing_event`，不能绕过状态机直接写 entitlement。
- 客户端刷新或轮询只能读取后端状态，不能自行开通权益。
- `billing:contract` 必须检查 payment session 账本、reconcile API、provider `query_order` 设计、双端轻量轮询边界。

## 11. 推荐实施顺序

1. 扩展后端 schema，新增 `billing_payment_sessions`。
2. fixture provider 支持 `query_order`，先完成不接真实 provider 的补偿 e2e。
3. 新增 `GET /v1/billing/payment-sessions/{id}` 与 `POST /v1/billing/payment-sessions/{id}:reconcile`。
4. 桌面端和移动端支付会话卡片接入 session 状态与轻量轮询。
5. 微信支付 adapter 增加查单请求构造和响应映射测试。
6. 接入后台补偿任务，再进入真实商户联调。
