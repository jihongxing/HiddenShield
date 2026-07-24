# Phase 8 Free 单份报告付费设计

状态：纳入本版封版范围；后端 fixture 闭环、双端购买入口、真实微信一次性商品后端核心和退款撤销授权链路已落地；真实商户参数、公网 HTTPS 回调、真实支付回调验收和退款撤销运行态验收待配置 / 待验收。

本文用于把 Free 用户的“单份版权详细报告”和“维权证据包”从 Creator 订阅权益中拆出，作为独立一次性付费商品。该能力是本版封版功能：双端入口、一次性 purchase grant、记录级核销和退款撤销链路进入发布范围；真实支付通道未配置或未通过验收时，产品应展示“支付通道尚未完成配置”，不能把该能力降级为未来功能，也不能让用户误以为已完成真实扣款。

## 1. 产品判断

Free 用户可能不是长期订阅用户，但在作品被盗用、平台申诉、举报侵权或准备维权材料时，会突然需要一份结构化、可信、可提交的证据材料。单份购买比强制订阅更符合该场景。

商业目标：

- 用 Free 基础记录建立信任。
- 在真实维权需求出现时提供低摩擦付费转化。
- 让单份购买成为 Creator 订阅的前置体验，而不是替代 Creator。
- 为后续 Studio / Enterprise 的案件证据包和团队审计报告预留模型。

## 2. 商品与价格

| 商品 | 价格 | 面向用户 | 核心价值 |
| --- | ---: | --- | --- |
| 单份版权详细报告 | 19.9 元 / 份 | Free 用户、偶发维权用户 | 将单条版权记录整理成可提交的平台申诉 / 版权管理辅助报告 |
| 维权证据包 | 49.9 元 / 份 | 已发生盗用或准备投诉的用户 | 在版权详细报告基础上组织侵权链接、截图、发现时间、平台信息和证据清单 |

价格单位为人民币，首期只支持按份购买，不做报告包、余额或积分。

## 3. 权益边界

### Free

- 可查看版权库记录和版权信息。
- 可复制基础存证摘要。
- 可按单条版权记录购买“单份版权详细报告”。
- 可按案件 / 单条版权记录购买“维权证据包”。
- 未购买时不能导出正式报告或证据包。

### Creator

- 订阅内包含单条版权详细报告导出能力，仍由 `report_export` 控制。
- 本地批量摘要和常规正式报告继续作为 Creator 权益。
- 维权证据包可作为 Creator 增购项或阶段性权益，正式策略需在实现前确认。

### Studio / Enterprise

- Studio 面向团队报告、共享版权库、团队审计和批量案件材料。
- Enterprise 面向 API、私有化、白标报告、合同 SLA 和定制证据格式。

## 4. 报告层级

| 层级 | 是否免费 | 说明 |
| --- | --- | --- |
| 基础存证摘要 | 免费 | 可复制，适合日常留档，不视为正式报告 |
| 版权详细报告 | Free 单份付费 / Creator 起订阅内 | 包含版权编号、创作者身份、作品指纹、可信时间、第三方验证、写入后验证、版本链、隐私边界和免责声明 |
| 维权证据包 | 单份付费 / 后续 Studio 可扩展 | 在版权详细报告基础上加入侵权链接、侵权截图、平台、发现时间、对比说明和证据清单 |

## 5. 支付与核销模型

一次性报告购买不能复用“订阅权益生效”语义，必须独立建模。

建议新增商品代码：

- `copyright_report_single`
- `rights_evidence_pack_single`

建议新增 purchase 类型：

- `one_time_report`
- `one_time_evidence_pack`

建议新增授权记录：

- `report_purchase_grants`

建议字段：

- `grant_id`
- `account_id`
- `workspace_id`
- `creator_profile_id`
- `vault_record_id`
- `product_code`
- `price_cents`
- `currency`
- `payment_session_id`
- `provider`
- `provider_order_id`
- `status`
- `granted_at`
- `revoked_at`
- `created_at`
- `updated_at`

核销规则：

- 支付成功后只给对应 `vault_record_id` 授权，不改变用户订阅等级。
- 授权可以重复打开 / 复制 / 下载同一份报告。
- 退款、撤销或风控失败后撤销授权。
- 客户端不能自行写入授权；必须由后端 provider webhook 或查单补偿写入。
- 授权记录不得包含原始媒体、加水印媒体、本地路径或保护副本路径。

## 6. API 草案

- `POST /v1/billing/report-purchase-sessions`
- `GET /v1/billing/report-purchase-sessions/{paymentSessionId}`
- `POST /v1/billing/report-purchase-sessions/{paymentSessionId}:reconcile`
- `GET /v1/reports/grants?vaultRecordId=...`
- `POST /v1/reports/grants/{grantId}:export`

设计原则：

- 支付 session 继续复用 provider adapter、签名校验、查单补偿和 webhook 幂等框架。
- 商品 allowlist 必须在后端，客户端只传 `productCode` 和目标记录。
- 价格校验必须由后端完成，微信回调 / 查单金额必须与商品价格一致。
- 导出报告时再次校验记录归属、创作者档案和工作区。

## 7. 用户表达

推荐文案：

- “基础存证摘要可免费复制。”
- “版权详细报告适合平台申诉、版权管理和维权材料整理。”
- “维权证据包用于整理侵权链接、截图、发现时间和证据清单。”

禁止文案：

- “保证追回盗用收益。”
- “法院必然认可。”
- “司法鉴定报告。”
- “购买后自动完成维权。”

免责声明：

本报告和证据包由 HiddenShield 根据本机版权库记录与用户补充材料生成，仅作为技术验证、版权管理和平台申诉辅助材料，不构成法律意见、司法鉴定意见或诉讼结果承诺。

## 8. 实施顺序

1. 更新商业化契约和合同脚本，固定商品、价格和支付配置边界。
2. 后端新增一次性 report purchase session、商品 allowlist、授权表和 fixture 支付测试。
3. [已完成] 桌面端版权库为 Free 用户展示“购买版权详细报告 / 购买维权证据包”入口。
4. [已完成] 移动端版权库记录详情展示同口径购买入口。
5. [已完成] 导出报告时支持 Creator `report_export` 或有效单份授权二选一通过。
6. [已完成] 增加退款 / 撤销授权测试。
7. [已完成] 运行商业化 CI、报告合同、billing 合同、桌面端构建和移动端状态测试验证；真实微信商户参数、公网 HTTPS 回调、真实支付回调验收和退款撤销运行态验收仍待补。

## 9. 当前边界

当前已完成后端 fixture 和双端入口闭环：

- `report_purchase_sessions` 用于一次性报告购买会话。
- `report_purchase_grants` 用于单条记录 / 案件授权核销。
- `/v1/billing/report-purchase-sessions` 支持创建、查询和 reconcile。
- fixture 支付成功后只写对应 `vault_record_id` 授权，不改变 Free 订阅等级，也不打开 `report_export`。
- 桌面端和移动端版权库都已展示“购买版权详细报告 / 购买维权证据包”入口。
- 桌面端和移动端正式报告导出都支持 Creator `report_export` 或当前记录有效单份授权二选一通过。
- `preferredProvider=wechat_pay` 时，后端可创建真实微信 Native 一次性商品订单，attach 固定 `purchaseType=report_purchase`，查单 / webhook 成功后只写 `report_purchase_grants`。
- report purchase 退款 / 撤销会把对应授权置为 `revoked`，不改变 Free entitlement，也不打开或关闭 Creator `report_export`。

封版产品逻辑：

- Free 可复制基础摘要。
- Creator 起可导出正式报告。
- Free 单份付费报告和维权证据包已进入双端版权库 UI，并完成双端记录级导出核销。
- 配置真实微信支付通道并完成联调后，Free 用户可按记录购买单份版权详细报告或维权证据包。
- 未配置真实微信支付通道时，入口可以保留，但必须展示支付通道未完成配置，不得伪造真实扣款或授权成功。
