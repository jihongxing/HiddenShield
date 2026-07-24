# HiddenShield V3 feature gate 写入与回滚验证方案

更新时间：2026-06-30

本文档定义 V3 media payload 默认写读后的最小回滚方案。当前已实现 `watermark-core` 默认 V3/39 image/audio 写读、内部 QA 专用 V3 写入 API、`off -> internal_qa -> force_v2_rollback` 自动验证矩阵，以及桌面端 + Android 原生端默认 V3 运行态 QA。正式图片能力只支持 V3/39；V2 图片写读与回滚已退役并返回 `v2_image_rollback_retired`。V2/119 仅在隔离套件中保留音频旧版回滚与必要的历史协议解析。

## 1. 冻结结论

- V3 默认写入已开启；正式图片 / 音频用户路径默认写 V3/39。
- V3 写入已经进入默认图片 / 音频正式路径；内部 QA 入口只用于生成受控证据，不再代表默认开关。
- V3 内部 QA 写入必须显式传入 gate，不允许通过环境变量悄悄影响正式 UI。
- `off` 表示没有额外 QA gate，仍走默认 V3/39；图片即使显式请求 `force_v2_rollback` 也必须拒绝。音频 V2 回滚仅存在于隔离测试，不属于正式产品路径。
- registry / rights manifest 仍是权利事实源；V3 媒体 payload 只提供最小锚点，不提供训练许可或法律结论。

## 2. Gate 状态

| 状态 | 默认值 | 允许行为 | 禁止行为 | 验证要求 |
| --- | --- | --- | --- | --- |
| `off` | 是 | 旧阶段含义保留为历史名称；当前正式路径默认写 V3/39 | 静默退回 V2 | 自动验证默认写入图片 / 音频均为 V3/39 |
| `internal_qa` | 否 | 仅内部 QA 命令显式写 V3 样本；产物进入 `tmp-ui-qa` 或内部 fixture 目录 | 改变默认算法语义、开放外部 API、把 QA 产物当用户报告结论 | 必须生成 V3 样本并记录证据 |
| `force_v2_rollback` | 否 | 图片稳定拒绝并返回 `v2_image_rollback_retired`；音频仅在隔离套件回到 V2/119 | 图片继续产出 V2，或删除既有 V3 样本记录 | 自动验证图片拒绝合同、音频 legacy fallback 与 registry 查询不受影响 |

## 3. 最小实现边界

R2 内部 QA 写入只允许通过显式内部 API；默认 `WatermarkService::embed/extract` 已切为 V3/39，显式 rollback API 保留 V2：

| 层 | 允许新增 | 禁止新增 |
| --- | --- | --- |
| `watermark-core` | `V3InternalQaWriteGate`、`V3InternalQaWriteInput`、`embed_v3_internal_qa_media`、内部 `embed_*_v3_internal_qa` 受控入口、图片 V2 稳定拒绝合同、音频隔离 legacy 回滚入口 | 修改 `PAYLOAD_BYTES = 119`；让默认图片 `extract` 接受 V2 当成功 |
| 桌面端 | QA bin / 内部命令显式传 gate，输出到 QA 目录；正式 scheduler 默认 V3 | 默认验证页把 V2 当默认算法；开放外部 Enterprise 路由 |
| Android 原生端 | 内部 QA tool 显式传 gate；默认 write/read 走 V3 | 移动端默认 read 兼容接受 V2 当成功 |
| iOS 原生端 | macOS 环境恢复后补同场景 QA | 用 Android QA 替代 iOS |
| 后端 / registry | 接收 V3 protocol 描述和 rights manifest 对照 | 从 V3 payload 解释训练许可或输出法律结论 |

当前 gate 结构语义：

```text
V3WriteGate {
  state: off | internal_qa | force_v2_rollback
  requested_by: internal_qa_command
  reason: string
  expires_at: optional timestamp
}
```

实现要求：

- `off` 是编译和运行默认值，含义为“无额外 QA gate，使用正式默认 V3/39”。
- `internal_qa` 必须由内部 QA 命令显式传入，不能只靠全局环境变量影响正式 UI。
- `force_v2_rollback` 仅用于合同验证：图片必须稳定拒绝，只有隔离的音频 legacy 测试允许写入 V2/119。
- 所有 V3 内部 QA 产物必须带 `payloadProtocolVersion=3`、`payloadBytesLength=39`、`mediaPayloadRole=v3_minimal_anchor`。
- 所有 rollback 产物必须带 `payloadProtocolVersion=2`、`payloadBytesLength=119`、`mediaPayloadRole=v2_full_record` 或等价派生结果。

## 4. 回滚验证脚本

新增脚本：

- `npm run rights:v3-feature-gate-rollback-contract`
- 实现文件：`scripts/verify-v3-feature-gate-rollback-contract.mjs`

当前脚本执行以下检查：

1. 静态检查 `PAYLOAD_BYTES = 119` 和 `PAYLOAD_V3_MINIMAL_ANCHOR_BYTES = 39` 未漂移。
2. 静态检查默认 `WatermarkService` 没有接入 `encode_payload_v3_minimal_anchor`、`WatermarkPayloadV3MinimalAnchor`、内部 QA V3 写入 API 或 readonly candidate 写入 helper。
3. 静态检查 package script、本文档和 V3 迁移合同包含 feature gate 与 rollback 门禁。
4. 运行 `watermark-core` QA bin，生成 `off -> internal_qa -> force_v2_rollback` 图片 / 音频矩阵，其中图片 rollback 行必须是预期拒绝。
5. `off`：用默认 `WatermarkService::embed/extract` 生成图片 / 音频，断言 V3/39。
6. `internal_qa`：用 `embed_v3_internal_qa_media` 生成图片 / 音频，断言 V3/39，且只进入 QA 目录。
7. `force_v2_rollback`：图片断言返回 `v2_image_rollback_retired`；音频在隔离套件中用显式 `embed_v2` / `extract_v2` 断言回到 V2/119。
8. 用默认读取验证 V3 样本；V2 rollback 样本只能用显式 V2 读取入口验证。
9. 输出 JSON / Markdown 证据到 `tmp-ui-qa/v3-feature-gate-rollback/<runId>/`。

该脚本是 `watermark-core` 内部 QA 写入与回滚门禁，不等同于桌面 + Android + iOS 三端默认写入 release gate；iOS 无环境时仍记录挂起，不能用 Android 或 Web QA 替代。

## 5. 发布门禁

R2 feature gate 写入进入代码前必须同时满足：

- `rights:v3-migration-contract` 通过。
- `rights:v3-minimal-anchor-contract` 通过。
- `rights:v3-readonly-candidate-runtime-qa` 通过。
- `rights:v3-report-sync-migration-qa` 通过。
- `rights:v3-feature-gate-rollback-contract` 通过。
- `dual:contract` 通过。
- `watermark-core` 图片 / 音频 V2 旧样本读取回归通过。
- iOS 同场景 QA 已具备环境或明确记录为非 release 状态。

## 6. 当前状态

当前已完成 gate 方案、`watermark-core` 默认 V3/39 image/audio 写读、内部 QA 专用 V3 image/audio 写入 API、rollback 验证矩阵，以及桌面端 + Android 原生端默认 V3 运行态 QA。`internal_qa` 只能由内部 QA 命令显式调用，输出 V3/39 样本到 QA 目录；`off` 证明默认写读为 V3/39，`force_v2_rollback` 证明图片 V2 已稳定退役且音频 legacy 回滚保持隔离。默认写入、默认读取、正式报告、同步和公开权利查询均应把 V3 媒体 payload 视为最小锚点，完整声明继续来自版权库 / 云版权库 / registry。

下一步：等 macOS + Xcode + iOS Simulator 或真机环境恢复后补齐 iOS 默认 V3 写读同场景证据，并建立 V3 图片 / 音频感知质量与性能基准。
