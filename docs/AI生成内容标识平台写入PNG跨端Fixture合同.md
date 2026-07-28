# AI 生成内容标识平台写入 PNG 跨端 Fixture 合同

## 状态

- 状态：`internal_fixture_contract`。
- Fixture 仅证明 internal Executor 对 `watermark-core` V3 图片 anchor 的写入、读取和 metadata 剥离后读取；不证明 SDK、公共 Resolver、生产 C2PA/TSA、平台 UI 渲染、客户交付或法规合规。

## 固定文件

目录：`docs/fixtures/ai-transparency-platform-executor-v1/`

| 文件 | 语义 |
| --- | --- |
| `platform-executor-v3.png` | Executor 返回的原始 PNG 保护副本。 |
| `platform-executor-v3-with-metadata.png` | 对原始副本加入测试用 PNG `tEXt` 元数据后的平台交付模拟。 |
| `platform-executor-v3-metadata-stripped.png` | 对含元数据副本无损 PNG 重编码后的 metadata 剥离副本。 |
| `manifest.json` | UID、V3/39、auth status、`legalConclusion=false` 与三份 SHA-256。 |

## 读取矩阵

三份 PNG 都必须由以下正式读取路径读取为同一结果：

| 端点 | 正式路径 | 结果 |
| --- | --- | --- |
| backend Executor QA | `WatermarkService::extract(ImageBytes)` | 同一 V3 UID、`verified`。 |
| Desktop | `src-tauri` 的 `WatermarkService::extract(ImageBytes)` | 同一 V3 UID、`verified`。 |
| Android | `mobile_app/rust::extract_image_for_mobile` | 同一 V3 UID、39 bytes。 |
| iOS | 同一 mobile Rust bridge 的 `extract_image_for_mobile` | 同一 V3 UID、39 bytes；实际 iOS runtime 仍需在 macOS/iOS 环境单独复跑。 |

metadata 剥离后仍读到 anchor 仅说明现有鲁棒锚点能力，不表示元数据签名、Manifest、Evidence 或显式标签仍存在。

## Gate

- Fixture 由 `ai_transparency_image_marking_executor_qa` 仅在一次性 `hiddenshield_migrate_smoke_*` PostgreSQL 数据库中生成。
- 生成前必须完成 custody -> `ready_to_confirm` -> executor write-after-read -> confirm。
- 任一文件 hash、UID、V3/39 或 auth status 不一致即阻断矩阵。
- 在 desktop、Android 与实际 iOS runtime 均完成 Gate 前，继续禁止 SDK、公共 Resolver 与生产发放。
