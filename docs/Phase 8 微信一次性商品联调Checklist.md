# Phase 8 微信一次性商品联调 Checklist

状态：待执行真实商户联调

本文用于执行 Free 单份版权详细报告和维权证据包的微信一次性商品真实联调。当前代码已经具备后端可测试核心：`preferredProvider=wechat_pay` 可创建 report purchase 微信 Native 订单，微信 attach 使用 `purchaseType=report_purchase` 分流，查单 / webhook 成功只写 `report_purchase_grants`，退款 / 撤销只撤销对应授权，不改变 Free entitlement，也不打开 Creator `report_export`。

Free 单份报告付费已经纳入本版封版范围。本文用于验收真实支付配置：在商户参数、公网 HTTPS 回调、真实下单、回调、查单补偿、退款撤销和双端授权互认全部通过前，产品内不得让用户误以为真实扣款链路已配置完成；未配置时应展示“支付通道尚未完成配置”。

## 1. 联调范围

本次只验收 Free 一次性商品：

| 商品 | productCode | 金额 |
| --- | --- | ---: |
| 单份版权详细报告 | `copyright_report_single` | 19.9 元 |
| 维权证据包 | `rights_evidence_pack_single` | 49.9 元 |

不在本次范围内：

- Creator / Studio 订阅真实扣费上线。
- 订阅续费、取消、宽限期和过期回收。
- Studio 团队报告。
- L3 视频画面盲水印、云端视频扣费和 `video_minutes` 账本。

## 2. 商户与环境准备

需要准备：

- 微信支付商户号。
- 微信支付 AppID。
- 商户 API 证书序列号。
- 商户 API 私钥 PEM。
- 微信支付平台公钥 PEM。
- APIv3 key。
- 公网 HTTPS 回调域名。
- 可访问后端服务的公网 HTTPS 地址。
- 桌面端和移动端可访问同一后端环境。

后端环境变量：

```text
HIDDENSHIELD_WECHAT_PAY_APP_ID=
HIDDENSHIELD_WECHAT_PAY_MCH_ID=
HIDDENSHIELD_WECHAT_PAY_MERCHANT_SERIAL_NO=
HIDDENSHIELD_WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH=
HIDDENSHIELD_WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH=
HIDDENSHIELD_WECHAT_PAY_API_V3_KEY=
HIDDENSHIELD_WECHAT_PAY_NOTIFY_URL=https://<public-host>/v1/billing/webhooks/wechat-pay
```

也可以使用 PEM 文本变量：

```text
HIDDENSHIELD_WECHAT_PAY_MERCHANT_PRIVATE_KEY_PEM=
HIDDENSHIELD_WECHAT_PAY_PLATFORM_PUBLIC_KEY_PEM=
```

约束：

- 不得把商户私钥、平台公钥、APIv3 key 写入 Git、日志、报告或客户端配置。
- `HIDDENSHIELD_WECHAT_PAY_NOTIFY_URL` 必须是公网 HTTPS，不能是 `localhost`、`127.0.0.1` 或内网地址。
- 回调路径必须是 `/v1/billing/webhooks/wechat-pay`。
- 后端缺少微信配置时必须返回 `wechat_pay_not_configured`，不能静默降级到 fixture。

## 3. 后端启动前检查

- [ ] 后端能读取全部微信环境变量或文件路径。
- [ ] 商户私钥 PEM 可被后端解析。
- [ ] 平台公钥 PEM 可被后端解析。
- [ ] APIv3 key 长度和格式正确。
- [ ] `notifyUrl` 与微信商户平台配置一致。
- [ ] 公网 HTTPS 地址能访问后端健康接口或已知 API。
- [ ] 服务器时间同步正常，避免微信签名时间偏差。
- [ ] 日志级别不会打印支付密钥、完整回调 body、完整用户 token 或媒体路径。

## 4. 下单验收

### 4.1 单份版权详细报告

步骤：

1. 使用 Free 账户登录。
2. 进入版权库任一正式记录详情。
3. 点击“购买版权详细报告 · 19.9 元”。
4. 客户端请求 `POST /v1/billing/report-purchase-sessions`，传入 `preferredProvider=wechat_pay`。
5. 后端创建微信 Native 订单。
6. 客户端展示微信支付二维码 / 支付动作。

通过标准：

- [ ] 返回 `provider=wechat_pay`。
- [ ] 返回 `productCode=copyright_report_single`。
- [ ] 返回 `priceCents=1990`。
- [ ] 返回 `paymentAction.type=qr_code`。
- [ ] 微信订单金额为 19.9 元。
- [ ] 微信 attach 包含 `purchaseType=report_purchase`。
- [ ] 微信 attach 包含对应 `vaultRecordId`。
- [ ] 该支付会话写入 `report_purchase_sessions`。
- [ ] 未支付前不写入 `report_purchase_grants`。
- [ ] 未支付前 Free entitlement 仍为 Free，`report_export=false`。

### 4.2 维权证据包

步骤同上，入口改为“购买维权证据包 · 49.9 元”。

通过标准：

- [ ] 返回 `productCode=rights_evidence_pack_single`。
- [ ] 返回 `priceCents=4990`。
- [ ] 微信订单金额为 49.9 元。
- [ ] 成功后只授权当前记录 / 当前案件，不升级订阅。

## 5. 支付成功验收

步骤：

1. 使用微信扫码完成支付。
2. 等待微信回调到 `/v1/billing/webhooks/wechat-pay`。
3. 客户端点击“确认支付”或等待轻量轮询。
4. 客户端查询 report purchase session 状态。
5. 导出当前记录正式报告。

通过标准：

- [ ] webhook 验签通过。
- [ ] AES-256-GCM resource 解密通过。
- [ ] 金额校验通过。
- [ ] attach 中的 `purchaseType=report_purchase` 被识别。
- [ ] 成功事件只写 `report_purchase_grants`。
- [ ] 授权 `status=active`。
- [ ] 授权只绑定当前 `vaultRecordId`。
- [ ] Free entitlement 仍为 Free。
- [ ] `report_export` 仍为 `false`。
- [ ] 当前记录可导出正式报告。
- [ ] 其他未购买记录仍不可导出正式报告。
- [ ] 客户端展示成熟产品文案，不展示技术错误码。

## 6. 查单补偿验收

目标：验证 webhook 延迟或丢失时，后端查单能恢复授权。

步骤：

1. 创建 report purchase 微信订单。
2. 完成支付。
3. 模拟 webhook 未到或暂时不可用。
4. 触发客户端“确认支付”或后台查单。
5. 后端通过 `out_trade_no` 查单。

通过标准：

- [ ] 查单请求签名通过。
- [ ] 微信返回 `SUCCESS` 后金额校验通过。
- [ ] 查单结果被转为 report purchase 授权。
- [ ] 授权写入 `report_purchase_grants`。
- [ ] 不写入订阅 `entitlements`。
- [ ] 重复查单幂等，不重复授权、不重复升级。

## 7. 退款 / 撤销验收

步骤：

1. 对已支付的单份报告订单发起退款或撤销。
2. 等待微信退款 / 撤销回调，或通过查单得到退款状态。
3. 查询对应 report purchase session。
4. 再次尝试导出该记录正式报告。

通过标准：

- [ ] 退款 / 撤销事件能定位同一 `providerOrderId`。
- [ ] 对应 `report_purchase_grants.status` 更新为 `revoked`。
- [ ] `revokedAt` 有记录。
- [ ] 当前记录不再凭该授权导出正式报告。
- [ ] 若用户同时拥有 Creator `report_export=true`，仍可按 Creator 权益导出。
- [ ] 退款 / 撤销不改变 Free entitlement。
- [ ] 退款 / 撤销不删除版权库基础记录。

## 8. 双端运行态验收

桌面端：

- [ ] Free 用户可在版权库记录详情看到两个购买入口。
- [ ] 支付成功后当前记录可导出正式报告。
- [ ] 未购买记录仍提示需要 Creator 或单份购买。
- [ ] 退款撤销后当前记录恢复门禁。

移动端：

- [ ] Free 用户可在版权库记录详情看到两个购买入口。
- [ ] 支付成功后当前记录可复制 / 导出正式报告草稿。
- [ ] 授权写入 `reportPurchaseGrantsJson`。
- [ ] App 重启后授权仍生效。
- [ ] 退款撤销后本地授权刷新为不可用。

双端一致性：

- [ ] 桌面端购买的授权，移动端拉取后可识别。
- [ ] 移动端购买的授权，桌面端拉取后可识别。
- [ ] 两端都不把本地路径、媒体文件路径或保护副本路径上传到云同步。
- [ ] 两端都不展示“保证追回收益”“司法鉴定报告”等禁止文案。

## 9. 日志与证据留存

联调记录需要保留：

- 支付商品。
- 金额。
- `paymentSessionId`。
- `providerOrderId`。
- 微信交易号。
- webhook 到达时间。
- 查单时间。
- 授权 `grantId`。
- 授权状态变化。
- 客户端截图。
- 后端日志摘要。

不得保留：

- 商户私钥。
- APIv3 key。
- 用户完整 token。
- 原始媒体。
- 加水印媒体。
- 本地文件路径。
- 保护副本路径。

## 10. 上线阻断项

任一项未通过时，不得对外开启正式购买：

- 真实微信下单失败。
- 回调验签失败。
- resource 解密失败。
- 金额校验失败。
- attach 无法区分订阅和 report purchase。
- 支付成功后授权未写入。
- 支付成功后误升级 Free entitlement。
- 退款 / 撤销后授权未撤销。
- 双端对同一授权识别不一致。
- 客户端展示技术错误或法律承诺过度文案。
- 日志或云同步包含商户密钥、媒体文件、本地路径或保护副本路径。

## 11. 完成定义

全部完成后，Free 单份报告付费即可从“封版能力但支付通道待配置 / 待验收”进入“真实支付通道已启用”的上线状态：

- [ ] 两个商品真实微信下单通过。
- [ ] 两个商品真实支付成功通过。
- [ ] webhook 成功授权通过。
- [ ] 查单补偿授权通过。
- [ ] 退款 / 撤销授权通过。
- [ ] 桌面端运行态验收通过。
- [ ] 移动端运行态验收通过。
- [ ] 双端授权互认通过。
- [ ] 法务文案审阅通过。
- [ ] `docs/当前真实能力边界说明.md` 已回写能力边界。
- [ ] `docs/商业化落地Roadmap.md` 已回写验收结果。

下一步任务：准备真实微信商户参数和公网 HTTPS 回调环境，启动后端进入一次性商品真实下单联调。
