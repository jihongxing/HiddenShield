# 设计伙伴 Sandbox 交接

设计伙伴必须使用现有 private package `@hiddenshield/ai-transparency-design-partner-kit` 创建 Sandbox bundle。本文件只说明外部材料，不取代该 package 的 Schema 或 preflight。

## 伙伴需提供的引用

| 类别 | 必填材料 | 允许格式 |
| --- | --- | --- |
| 身份 | legal name、技术/安全联系人、数据处理与采购签署引用 | `partner://...` / 受控文档引用 |
| 环境 | Sandbox API 与 Resolver 非占位 HTTPS endpoint | `https://...` |
| 访问 | Sandbox credential 引用 | `secret://...` |
| Profile | CN / EU / US-CA 使用场景、显式标签面、C2PA/anchor 选择与法务审查引用 | 伙伴 bundle 字段 |
| 验收 | 12 个强制场景的不可变 evidence | `evidence://sha256/<64-hex>` |
| 性能 | 约定 p50、p95、失败率和样本窗口 | 伙伴 bundle / evidence |

## 禁止事项

- 不得提交生产 credential、客户媒体、原始 token、私钥或可用于访问生产环境的 URL。
- `blocked_external`、`not_run`、synthetic evidence 或占位符不属于 `sandbox_accepted`。
- Sandbox 通过不代表法规合规、生产 entitlement、SDK 公布、收入确认或 SLA。

## 验收出口

仅当 private package preflight 返回 `sandbox_accepted`、12 个场景均为 `passed`，且每项均绑定不可变 evidence 与双方签署引用时，才能记录“该伙伴 Sandbox 验收已通过”。

