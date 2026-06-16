# HiddenShield Mobile

Flutter 双端 App 的移动端壳。

当前状态：

- 只做图片和 WAV 音频的本地确权入口。
- 本地不做视频盲水印。
- 预留 Rust / FRB 桥接层，后续接入真正的底层能力。
- UI 与桌面端保持同一套工作台、取证、版权库、设置结构。
- 图片嵌入测试页已接入文件选择、预览、重写开关和 preview 结果摘要。

当前代码入口：

- `lib/main.dart`
- `lib/app/app.dart`
- `lib/app/mobile_shell.dart`
- `lib/features/workspace/workspace_page.dart`
- `lib/bridge/watermark_bridge.dart`
- `lib/bridge/local_preview_watermark_bridge.dart`

桥接说明：

- `PreviewWatermarkBridge` 是 Flutter 侧占位实现，用于保持页面和测试稳定。
- `flutter_rust_bridge_codegen` 可用后，用真实 Rust 绑定替换 bridge 实现即可。

验证：

```bash
flutter analyze
flutter test
```
