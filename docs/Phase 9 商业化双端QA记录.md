# Phase 9 商业化双端 QA 记录

记录日期：2026-06-19

本记录用于承接 `docs/Phase 9 商业化上线验收Checklist.md` 中的桌面端、移动端、权益门禁、本地批量、云同步和支付入口验收。本轮 QA 是基于当前仓库代码、合同脚本和自动化测试的证据验收；未接入真实微信商户材料，因此不声明真实收费链路已经上线。

## 1. 验收结论

| 范围 | 结论 | 说明 |
| --- | --- | --- |
| 桌面端订阅页面 | PASS | Free / Creator / Studio / Enterprise 四档、Creator 权益、Studio 预留、Enterprise 咨询、支付会话和“确认支付”入口已实现。 |
| 移动端订阅页面 | PASS | 移动端设置页使用同一套套餐和权益文案，继续账户与正式云同步分离，支付会话和“确认支付”入口已实现。 |
| 权益门禁 | PASS | Free 无法使用正式云同步和本地批量；未购买时不能导出正式报告，购买单份版权详细报告 / 维权证据包后仅对应记录 / 案件可导出；Creator 可进入本地批量、云同步和订阅内正式报告路径；Studio 团队空间只展示预留状态。 |
| 本地批量订阅服务 | PASS | 桌面端和移动端均明确 Free 不进入文件选择、不创建批量队列；Creator 可创建图片 / 音频批量队列。 |
| 云同步订阅门禁 | PASS | Free 可继续账户但不能启用正式云同步；Creator 权益可放行正式云同步；同步边界不包含原始媒体和本地路径。 |
| 报告与视频指纹存证 | PASS | Creator 订阅内正式报告受 `report_export` 门禁；Free 单份购买后通过 `report_purchase_grants` 解锁当前记录 / 案件报告。报告和 L2 视频指纹存证记录不包含原始媒体、加水印媒体或本地路径。 |
| 商业指标看板 | PASS | 已提供 `/v1/commercial/metrics/overview` 聚合接口、双端商业健康摘要、管理员 token 鉴权和访问审计。 |
| 微信支付真实联调 | BLOCKED | 需要微信支付商户号、AppID、商户 API 证书 / 私钥、平台公钥、APIv3 key 和公网 HTTPS 回调域名。 |
| 法务审阅 | BLOCKED | 隐私政策、用户协议和支付订阅条款已补齐草案，但仍需法律顾问正式审阅。 |

## 2. 桌面端验收

| 项目 | 结果 | 证据 |
| --- | --- | --- |
| 四档套餐展示完整 | PASS | `src/components/SubscriptionPanel.vue` 展示 `Free / Creator / Studio / Enterprise`。 |
| Creator 权益文案完整 | PASS | Creator 包含本地批量处理、桌面端与移动端云同步、正式报告。 |
| Studio 不暴露未完成管理动作 | PASS | 桌面订阅页只展示团队能力规划；版权库团队空间卡说明团队空间入口、共享版权库模型、成员权限模型和团队审计模型均为预留 / 建设中。 |
| Enterprise 不伪装为直接购买 | PASS | Enterprise 以咨询 / 定制入口呈现，不进入自动支付购买链路。 |
| 未继续账户时不能开通订阅 | PASS | `SubscriptionPanel.vue` 在缺少云账户档案时提示先继续账户。 |
| 微信支付未配置提示明确 | PASS | `wechat_pay_not_configured` 映射为“支付通道尚未完成配置，当前可先联系开通。” |
| 支付会话完成态 | PASS | 支付会话展示订单号、状态、有效期，并提供“确认支付”。 |
| Free 本地批量门禁 | PASS | `src/views/LocalBatchView.vue` 明确 Free 不进入文件选择，也不会创建批量队列。 |
| Free 正式报告门禁 | PASS | 未购买时由报告门禁阻断；购买单份版权详细报告 / 维权证据包后，报告命令层允许当前记录有效 `report_purchase_grants` 导出，且 `report_export=false` 保持不变。 |
| L2 视频指纹存证展示 | PASS | 版权库可展示视频指纹存证编号、指纹根、bundle 摘要、采样帧、耗时和采样策略。 |

桌面端备注：

- `src/components/SettingsPanel.vue` 仍保留“高级：临时直连”调试区。该能力属于桌面端高级维护入口，不作为移动端同步模式或用户侧桥接依赖展示。
- 正式收费上线前仍需要一次运行态 Tauri 交互验收，重点覆盖支付入口、订阅弹窗、批量入口、正式报告和视频指纹存证入口的视觉状态。

## 3. 移动端验收

| 项目 | 结果 | 证据 |
| --- | --- | --- |
| 四档套餐与桌面端一致 | PASS | `mobile_app/lib/features/settings/settings_page.dart` 使用 Free / Creator / Studio / Enterprise。 |
| 继续账户与正式云同步分离 | PASS | Free 可继续账户，本地功能可用；正式云同步从 Creator 开放。 |
| 支付会话完成态 | PASS | 移动端订阅 Sheet 展示“支付会话已创建”、订单号、状态、有效期和“确认支付”。 |
| 不显示桥接层或临时直连 | PASS | `mobile_app/test/widget_test.dart` 明确断言不显示“桥接层已接入”和“临时直连”。 |
| Free 本地批量门禁 | PASS | `mobile_app/lib/features/workspace/local_batch_page.dart` 明确 Free 不进入文件选择，也不会创建批量队列。 |
| Creator 本地批量队列 | PASS | 移动端 widget 测试覆盖 Creator 可进入本地批量队列预览。 |
| Free 正式报告门禁 | PASS | 验证页和版权库记录详情对未购买 Free 保持门禁；购买单份报告后当前记录可生成正式报告草稿，授权写入 `reportPurchaseGrantsJson`。 |
| Studio 团队空间预留 | PASS | 设置页和版权库仅展示 Studio 团队空间边界，不开放真实成员管理。 |
| L2 视频存证同步查看 | PASS | 移动端版权库详情可展示云同步接收的视频指纹存证记录，测试覆盖路径不泄漏。 |

移动端备注：

- 本轮未执行真机安装包手测。当前结论来自 Flutter widget / state 测试、合同脚本和代码审计。
- 移动端真实支付跳转体验需要在微信商户配置齐备后补测。

## 4. 自动化证据

当前商业化验收依赖以下自动化门禁：

- `npm run commercial:contract`
- `npm run commercial:metrics`
- `npm run commercial:ci`
- `npm run billing:contract`
- `npm run usage:contract`
- `npm run report:contract`
- `npm run team:contract`
- `npm run cloud:ci`
- `npm run cloud-video:ci`
- `flutter analyze`
- `flutter test`

最近一次已知状态：

- `commercial:ci` 已通过，覆盖商业化合同、支付合同、用量账本、正式报告、团队空间、桌面构建、后端测试、Tauri 测试、Flutter analyze / test、云同步 CI 和云视频 CI。
- 2026-06-25 之后 `commercial:ci` 还必须串行覆盖 `dual:contract`、`watermark:architecture-contract`、`watermark:video-phase-contract` 和 `watermark:cross-end-contract`，避免封版入口漏掉双端互解或 L3 冻结门禁。
- 本记录纳入 `commercial:contract` 后，需要重新执行 `npm run commercial:contract` 固定 QA 记录存在性与关键结论。

2026-06-22 发布候选补充：

- `commercial:ci` 已重新通过；Tauri 桌面测试按当前发布范围执行 `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip l3`，避免把已冻结的 L3 长跑池混入现有能力发布阻断。
- `watermark:cross-end-release` 已通过，覆盖图片 / 音频跨端互验、非 WAV 移动归一化、桌面 FFmpeg 转码和 L1 视频音轨；L3 长跑池按 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1` 显式开关保留为内部验证。
- 后端运行态已通过 `GET http://127.0.0.1:43188/healthz`；桌面端 Tauri dev 已启动并返回 200。
- 原生移动端运行态仍未完成：当前验收机器未检测到 Android / iOS 设备或模拟器，项目也未配置 Windows desktop runner，不能用 Windows 形态替代真机 / 模拟器验收。

## 5. 上线阻断项

以下项目未完成前，不建议正式上线收费：

- 真实微信商户联调：需要你提供微信支付商户号、AppID、商户 API 证书 / 私钥、平台公钥、APIv3 key 和公网 HTTPS 回调域名。
- 真实退款 / 撤销测试订单：用于验证退款或撤销后 entitlement 降级。
- 法律顾问审阅：隐私政策、用户协议、支付订阅条款已补齐草案，但需要正式法律审阅。
- 生产环境配置：需要配置 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN`，并限制商业指标后台访问来源。

## 6. 本轮判断

双端商业化页面和门禁已经具备进入上线前联调的产品基础：套餐口径一致、Free / Creator / Studio 行为可解释、客户端不自行写正式权益、本地批量和正式报告不会被 Free 绕过，移动端也不再暴露桥接层或临时直连。

当前商业化落地已阶段性完成；不能直接声明正式收费上线的原因不是双端页面或指标能力，而是法务审阅、真实微信商户联调和生产环境配置仍未完成。

## 7. 2026-06-25 Free 单份报告付费补充

Free 单份报告付费已纳入本版封版范围，覆盖单份版权详细报告 19.9 元 / 份和维权证据包 49.9 元 / 份。

- Free 未购买时只能复制基础摘要，不能导出正式报告。
- Free 购买后只解锁对应记录 / 案件的正式报告，不升级订阅、不打开 Creator `report_export`。
- 退款 / 撤销后只撤销对应记录 / 案件授权。
- 未配置真实微信支付通道时，产品应展示“支付通道尚未完成配置”。
