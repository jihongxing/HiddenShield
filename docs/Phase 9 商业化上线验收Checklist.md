# Phase 9 商业化上线验收 Checklist

本文档用于把 HiddenShield 当前商业化能力从“功能已实现”推进到“可上线验收”。验收范围覆盖桌面端、移动端、后端、支付与订阅状态、权益门禁、报告、云同步、视频指纹存证、法务文案和指标看板。

当前发布主线详见 `docs/双端现有能力发布计划.md`。短期冻结 L3 视频画面盲水印，本版只验收当前可承诺能力；L3 不进入 UI、订阅权益、云端任务、账本扣费或销售话术。

## 1. 验收原则

- 商业化口径统一使用 Free / Creator / Studio / Enterprise。
- 桌面端和移动端必须展示同一套套餐、权益、门禁和完成态文案。
- 客户端不得自行写入正式权益；正式 entitlement 只能来自后端 webhook、provider 查单补偿或云端权威权益接口。
- 本地批量是 Creator 订阅权益，不做 Free 小批量试用。
- 不默认同步原始图片、加水印图片、原始音频、加水印音频、原始视频、加水印视频和本地文件路径。
- 云端视频画面盲水印仍是未来能力；当前可验收的是 L2 视频指纹存证。
- L1 视频音轨水印可以按“视频音轨水印”表达，不能包装成视频画面盲水印。
- L3 视频画面盲水印不作为本版上线阻断项，也不能作为本版卖点。
- 真实微信商户沙箱 / 生产联调只有在提供商户配置、回调域名和证书材料后才进入验收。

## 2. 自动化验收

上线前必须通过以下命令。云同步与云视频 CI 都会占用 `127.0.0.1:43188`，必须串行执行。

| 验收项 | 命令 | 通过标准 |
| --- | --- | --- |
| 桌面端构建 | `npm run build` | TypeScript 与 Vite 构建通过 |
| 桌面端发布范围核心测试 | `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --skip l3` | Tauri 命令、版权库、报告、同步协议和 L1/L2 发布范围测试通过；L3 冻结项不纳入本版发布阻断 |
| 后端核心测试 | `cargo test --manifest-path feedback-backend/Cargo.toml --lib` | billing、云同步、视频存证、统计测试通过 |
| 移动端静态检查 | `flutter analyze` | 无 analyzer error / info gate 失败 |
| 移动端 widget / state 测试 | `flutter test` | 移动端主流程、设置页、版权库、同步展示测试通过 |
| 云同步 CI | `npm run cloud:ci` | 自动启动后端并通过 cloud contract / e2e |
| 支付订阅合同 | `npm run billing:contract` | provider 抽象、微信支付、补偿、双端入口、Roadmap 记录均通过 |
| 用量账本合同 | `npm run usage:contract` | 桌面端和移动端 usage ledger 字段与文案一致 |
| 报告导出合同 | `npm run report:contract` | Creator 订阅内正式报告受 `report_export` 门禁；Free 单份购买后仅对应记录 / 案件可导出，且不泄漏媒体路径 |
| 双端一致性合同 | `npm run dual:contract` | 双端术语、能力入口、同步边界和 Web preview 非正式边界一致 |
| 共享水印跨端互解 | `npm run watermark:cross-end-release` | 图片 / 音频 desktop->mobile 与 mobile->desktop 均可读取 / 验证 / 解密同一版权编号和 payload |
| 水印架构合同 | `npm run watermark:architecture-contract` | 正式水印能力只能调用 `watermark-core`，Web preview 不进入正式记录 |
| 视频分层合同 | `npm run watermark:video-phase-contract` | L1 / L2 / L3 分层正确，L3 不进入本版 UI、权益、云任务或扣费 |
| Studio 团队合同 | `npm run team:contract` | 团队空间入口受 `team_workspace` 门禁，不暴露未完成管理动作 |
| 云端视频 CI | `npm run cloud-video:ci` | L2 指纹存证 contract / UI contract / bundle / HTTP e2e 均通过 |
| 商业化总门禁 | `npm run commercial:ci` | 串行运行当前全部自动化商业验收命令，且 `cloud:ci` 与 `cloud-video:ci` 不并行 |

## 3. 桌面端人工验收

### 3.1 订阅页

- Free / Creator / Studio / Enterprise 四档展示完整。
- Creator 显示本地批量、云同步、报告导出。
- Studio 显示团队空间、成员权限、共享版权库、团队审计，但不开放未完成管理动作。
- Enterprise 显示定制咨询，不伪装为可直接购买。
- 未继续账户时点击 Creator / Studio 开通，提示先继续账户。
- 微信支付未配置时，显示“支付通道尚未完成配置”类明确提示。
- 支付会话创建后展示订单号、状态、有效期和“确认支付”。

### 3.2 权益门禁

- Free 不能开启正式云同步。
- Free 不能进入本地批量执行队列。
- Free 未购买时不能导出正式报告，只能复制基础摘要；购买单份版权详细报告 / 维权证据包后，仅对应记录 / 案件可导出，且不打开 `report_export`。
- Creator 可以进入本地批量、正式云同步和正式报告。
- Studio 团队空间只展示预留状态，不开放真实成员管理。
- 权益刷新后本地 `entitlement_state` 与云端快照一致。

### 3.3 本地批量

- Creator 可以创建图片批量任务。
- Creator 可以创建音频批量任务。
- 批量任务支持暂停、继续、取消和失败项重试。
- 成功项写入版权库，并产生 usage ledger。
- 失败项保留可解释错误，不绕过单文件完成后验证。

### 3.4 报告和视频指纹存证

- 单条版权记录可导出正式报告。
- Free 单份报告购买授权只绑定当前记录 / 案件；其他未购买记录仍被门禁阻断。
- 批量摘要报告受 Creator 门禁。
- 报告不包含原始媒体、加水印媒体或本地路径。
- L2 视频指纹存证记录可展示存证编号、指纹根、bundle 摘要、采样帧、耗时和采样策略。
- L2 视频存证成功后可进入版权库，并可纳入正式报告。

## 4. 移动端人工验收

### 4.1 订阅与设置页

- 套餐、权益和桌面端一致。
- 继续账户与正式云同步是两个动作。
- Free 可继续账户，但不能启用正式云同步。
- Creator / Studio 可创建支付会话，并展示“确认支付”。
- 不显示旧的桥接层、临时直连或桌面端依赖表述。

### 4.2 核心能力

- 图片写入、验证、入库路径可用。
- 音频写入、验证、入库路径可用。
- 移动端支持的图片和音频格式口径与桌面端一致。
- 版权库可搜索、筛选、查看记录详情。
- 从云同步接收的 L2 视频指纹存证记录可以展示，不暴露本地路径。

### 4.3 商业门禁

- Free 本地批量入口被 Creator 门禁阻断。
- Creator 可进入本地批量队列。
- Free 未购买时正式报告入口被门禁阻断；购买单份报告后当前记录可生成 / 复制正式报告草稿。
- Creator 可生成正式报告草稿。
- Studio 团队空间仅展示预留状态。

## 5. 后端与支付验收

### 5.1 可自动验收

- `payment-sessions` 创建仅允许 allowlist 套餐。
- 微信支付配置缺失时返回 `wechat_pay_not_configured`，不静默降级 fixture。
- fixture webhook 幂等。
- fixture 手动 reconcile 可恢复 webhook 缺失的支付成功。
- 后台补偿任务按 `next_check_after` 扫描 pending session。
- `wechat_pay` session 不再用 fixture session 占位。
- 微信 `out_trade_no` 查单请求、`trade_state` 映射、金额校验已由单元测试固定。
- 微信查单结果复用标准 `apply_billing_event`，不绕过 entitlement 状态机。
- Free 单份报告付费通过 `report_purchase_grants` 授权当前记录 / 案件，不升级订阅、不打开 Creator `report_export`。

### 5.2 需要外部材料后验收

以下项目不能在当前本地仓库内完成，必须由项目方提供材料：

- 微信支付商户号、AppID、商户 API 证书 / 私钥、平台公钥、APIv3 key。
- 可公网访问的 HTTPS 回调域名。
- 微信支付沙箱或生产测试订单。
- 退款 / 撤销测试订单。

拿到材料后验收：

- Native 下单真实返回二维码。
- 手机微信扫码支付后 webhook 正常到达。
- webhook 验签、AES-GCM 解密、金额校验通过。
- entitlement 自动升级为 Creator / Studio。
- report purchase 订单成功后只写当前记录 / 案件授权，不升级订阅。
- webhook 未到时，后台查单补偿可以恢复权益。
- 退款 / 撤销后 entitlement 降级为 Free 或 expired。
- report purchase 退款 / 撤销后只撤销对应授权；若用户没有 Creator `report_export`，该记录恢复报告门禁。

## 6. 法务与文案验收

### 6.1 草案状态

- 隐私政策草案：已补齐本地处理、云同步范围、不默认上传媒体 / 本地路径、匿名统计、诊断信息脱敏和支付数据边界。
- 用户协议草案：已补齐软件用途、版权保护边界、用户责任、禁止滥用、报告非法律意见和 L2 视频边界。
- 支付与订阅条款草案：已补齐 Creator / Studio 权益、续费、取消、退款、宽限期、过期处理和确认支付边界。
- 云端视频条款：已明确 L2 是指纹存证，不是画面盲水印；L3 是未来能力。

正式收费上线前，以上草案仍需法律顾问审阅确认，并把审阅意见同步回桌面端、移动端和后端对外文案。

### 6.2 文案一致性

- 产品内不得承诺“绝对防盗”。
- L2 视频能力不得称为“视频盲水印已支持”。
- 报告不得称为司法鉴定或法律意见。
- 本地批量不得出现 Free 小批量试用暗示。
- 云同步不得暗示会同步原始媒体。

## 7. 指标看板验收

首期指标看板可以先做后端 / 本地统计口径，不要求完整 BI 系统。

必须能回答：

- 每日新增继续账户数。
- Free / Creator / Studio / Enterprise 权益分布。
- 支付会话创建数、成功数、失败数、过期数。
- 本地批量使用次数。
- 正式报告导出次数。
- 云同步成功 / 失败次数。
- L2 视频指纹存证次数。
- 匿名失败事件按功能和错误码聚合。

不得采集：

- 原始媒体文件。
- 加水印媒体文件。
- 本地文件路径。
- 原始媒体 hash 明文作为产品分析字段。

## 8. 上线阻断项

以下任一项未完成，不建议正式上线收费：

- 自动化验收命令失败。
- 双端套餐或权益文案不一致。
- Free 可绕过 Creator 门禁使用本地批量、正式报告或正式云同步。
- 客户端可自行修改正式 entitlement。
- 支付成功只依赖用户点击“确认支付”。
- 报告、云同步或视频存证泄漏本地路径或媒体文件。
- 未补齐隐私政策、用户协议、支付条款。
- 未完成真实微信商户联调却在产品中声明“支付已上线”。

## 9. 当前结论

当前项目已经具备进入 Phase 9 验收清单执行的基础：

- Phase 3-7 的核心商业能力已由代码和合同脚本固定。
- Phase 8 已完成支付 provider 抽象、微信 Native 下单核心、webhook、查单映射、补偿任务和双端订阅入口。
- 真实微信商户联调、法务条款和指标看板仍是上线前阻断项。

下一步应优先执行桌面端和移动端的人工商业化 QA，并补齐隐私政策、用户协议和支付条款。
