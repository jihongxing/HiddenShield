# AI 生成内容标识合成三层处理链 Gate

- 状态：`internal_post_embed_resign_verified_nonproduction`。
- 输入：MIT 许可的外部视觉水印研究样本。
- 中间产物：使用 `EphemeralSigner` 生成本地自签 C2PA JPEG，`c2pa` Reader 读取到 active manifest。
- 输出：同一中间产物经 `watermark-core` 写入后生成 PNG，并回读 verified V3 anchor。
- 最终 PNG C2PA 分类：`manifest_absent_after_png_reencode`。
- post-embed 产物：先由 `watermark-core` 生成 verified V3 PNG，再由 `EphemeralSigner` 签发本地 C2PA manifest。
- post-embed C2PA 分类：`manifest_present_with_validation_findings`；active manifest 可读，validation findings 来自本地 ephemeral 自签链不受生产信任。
- post-embed V3：同一最终 PNG 回读相同 UID，`payloadAuthStatus=verified`。
- 输出容器 Gate：internal-only post-embed resign 原型通过；当前 `watermark-core` PNG 重编码路径本身仍不会保留输入 C2PA，必须在写入后重新签发。
- 分类合同：`manifest_present_and_readable`、`manifest_present_with_validation_findings`、`manifest_absent_after_png_reencode`、`reader_error:*`。
- 边界：自签 C2PA 仅用于本地 QA，不受生产信任；只能表述为“最终 PNG 同时可读本地自签 C2PA 与 verified V3”，不得表述为生产 C2PA、平台验收或法规合规。
- Gate：iOS runtime、第三方隐式水印、生产 C2PA/TSA、SDK、公共 Resolver 与生产发放继续关闭。
