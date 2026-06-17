# watermark-core

可复用的盲水印核心库，已从 `HiddenShield` 桌面端中拆出。

## 导出能力

- `WatermarkPayload` / `AIContentFlags`
- `encode_payload()` / `decode_payload()`
- `embed_watermark()` / `extract_watermark()`（PCM samples）
- `embed_watermark_wav_bytes()` / `extract_watermark_wav_bytes()`（WAV bytes，写入要求 30 秒及以上）
- `embed_image_watermark()` / `extract_image_watermark()`（文件路径封装）
- `embed_image_watermark_bytes()` / `extract_image_watermark_bytes()`（image bytes）
- `WatermarkService` / `MediaInput` / `MediaOutput` / `EmbedOptions`（服务层统一接口）

## 在新项目里怎么接

```toml
[dependencies]
watermark-core = { path = "../watermark-core" }
```

桌面端或云盘后端可以继续保留自己的文件上传、转码、存储层，只把媒体解码后的数据交给这个库处理。

音频写入的产品级入口以 30 秒为最小时长边界。`AudioWavBytes` 会拒绝短于 30 秒的 WAV；`AudioSamples` 仍保留为底层算法和单元测试入口，不代表对短音频版权保护的产品承诺。

推荐的后端接法是：

1. 接收上传文件 bytes
2. 识别媒体类型
3. 直接构造 `MediaInput`
4. 调用 `WatermarkService::embed()` 或 `WatermarkService::extract()`
5. 需要落盘时再由上层决定是否写文件
