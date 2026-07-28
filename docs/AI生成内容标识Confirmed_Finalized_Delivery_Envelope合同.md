# HiddenShield AI 生成内容标识 Confirmed / Finalized Delivery Envelope 合同

合同日期：2026-07-28。

## 目标

本合同冻结 internal-only AI 图片最终产物交付边界。只有同时满足下列条件的产物才允许生成 delivery envelope：

- signing execution 为 `confirmed`
- artifact 为 `finalized`
- recovery 为 `completed`
- final file hash、signer receipt、artifact finalize receipt、Profile identity 全部完成摘要绑定

`reserved`、`signed_staged`、`artifact_pending`、`orphaned`、`eligible`、`leased`、`retry_scheduled` 和 `dead_letter` 均不得生成或返回成功 envelope。

## Schema

Schema version：

```text
hs-ai-confirmed-artifact-delivery-envelope-v1
```

核心字段：

- identity：delivery envelope、execution、marking session、transparency manifest、license、watermark UID
- media：`image/png`、claim type、final file SHA-256
- state：signing status、artifact status、recovery state、worker attempts、recovery control version
- artifact：artifact reference、object version、finalized timestamp
- signer receipt：receipt id、canonical receipt SHA-256
- finalize receipt：receipt id、canonical receipt SHA-256
- Profile identity：entitlement version、entitlement digest、technical Profile ids、regional Profile id
- integrity：Profile identity digest、envelope digest

## Canonicalization

- JSON receipt 摘要必须先解析为 JSON，再递归按 object key 升序排序，最后以紧凑 UTF-8 JSON 计算 lowercase SHA-256。
- 禁止依赖 `serde_json` 的 `preserve_order`、编译 feature 或输入文本字段顺序。
- Profile identity digest 使用固定 JSON array：
  1. entitlement version
  2. entitlement digest
  3. technical Profile ids
  4. regional Profile id
- envelope digest 使用固定 JSON array，包含所有 identity、状态、artifact、receipt digest、拆平的 Profile identity 和 finalized timestamp；`envelopeDigest` 本身不进入摘要。

## Backend Gate

- migration `0012_ai_transparency_confirmed_delivery_envelope` 在 signing execution 持久化不可变 Profile identity。
- `ai_post_embed_delivery_envelopes` 每个 execution 最多一行。
- envelope projection 由 PostgreSQL trigger 拒绝 UPDATE/DELETE。
- command 在 transaction 内锁定 execution，检查 confirmed/finalized/completed，创建 envelope；重复调用只允许返回同一 envelope replay。
- receipt 缺失、Profile identity 缺失、状态不完整或 replay digest 冲突时 fail-closed，不写 envelope。
- envelope 生成不新增客户计量；计量仍只来自既有 committed `confirmed_marked_image` ledger。

## Desktop / Mobile Bridge Gate

Desktop 和 mobile Rust bridge 必须调用 `watermark-core::validate_ai_delivery_envelope`，不得各自实现摘要或状态规则。

bridge 输入：

- envelope JSON
- final media bytes
- signer receipt JSON
- artifact finalize receipt JSON

bridge 必须按以下顺序 fail-closed：

1. Schema 与必填字段
2. `confirmed`
3. `finalized`
4. recovery `completed`
5. final media hash
6. signer receipt canonical hash 与字段绑定
7. finalize receipt canonical hash 与字段绑定
8. Profile identity digest
9. envelope digest

拒绝结果不得返回可入库的 envelope digest、final hash、watermark UID 或 Profile digest。

## Fixture 与验证

- 共享 fixture：`docs/contracts/ai-transparency-delivery-envelope/success-v1.fixture.json`
- Desktop/mobile 对同一 fixture 均验证成功。
- 已覆盖 artifact pending、recovery leased、media hash mismatch、signer receipt mismatch、finalize receipt mismatch 和 Profile identity mismatch。
- PostgreSQL QA 已覆盖创建、幂等 replay、append-only projection 和 recovery 非 completed 零交付。
- migration smoke：36 表、47 索引、0001–0012 up/down。

## 当前边界

- 分类：`只能内部测试`。
- 不是下载 API、公共 Resolver response、SDK response、客户 vault record、法规结论或生产 SLA。
- envelope 不替代 C2PA/V3 媒体 readback；它只约束已经完成双回读和 confirm/finalize 的最终产物交付。
- iOS/macOS runtime 仍按环境依赖挂起；mobile Rust bridge 合同测试不等于 iOS runtime 验收。

## 下一 Gate

冻结 internal delivery authorization / retrieval command：必须验证调用方 entitlement、一次性或短期下载授权、artifact object-store receipt 和 envelope digest，并保证只有通过 bridge 校验的 bytes 才能进入端侧 vault/import 流程。
