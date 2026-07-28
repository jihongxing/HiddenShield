# AI 生成内容标识第三方视觉水印公开 Fixture 合同

## 来源与许可

- Fixture：`docs/fixtures/ai-transparency-third-party-visual-watermark-v1/watermarkreco-synthetic.jpg`。
- 来源：`IMAGINE-Paris/WatermarkReco` 公开仓库的 `figure/synthetic.jpg`。
- 许可证：MIT；来源、revision、SHA-256 与预期固定在同目录 `manifest.json`。

## 可验证语义

- 经人工检查，该研究图像含可见嵌入水印/纹样，并来自第三方水印检索研究语料。
- 该文件不是平台生产 AIGC 样本，不携带经验证的第三方隐式水印、C2PA manifest 或平台验收授权。
- 已验证：写入前未发现 HiddenShield V3 anchor；写入后由 `watermark-core` 读取到 `HS-89ABCDEF-01234567-89ABCDEF-01234567` V3 anchor。

## 三层 Benchmark 边界

- 当前公开语料分别覆盖 C2PA metadata 层与外部视觉水印层；它们不是同一资产，不能宣称已完成同资产三层互操作。
- `npm run ai-transparency:third-party-layered-benchmark` 固定运行 C2PA Reader 非混淆检查，以及外部视觉水印样本的写前拒绝、V3 写入与写后回读。
- 同资产三层 Gate 必须取得带明确许可证的“C2PA metadata + 外部视觉/隐式水印”单一媒体样本，或取得提供方的允许生成该组合的工具链与验收标准。
- iOS runtime 继续挂起；SDK、公共 Resolver、production credential 与生产发放继续关闭。
