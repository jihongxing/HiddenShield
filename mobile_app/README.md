# HiddenShield Mobile

Flutter 双端 App 的移动端壳。

当前状态：

- 只做图片和 WAV 音频的本地确权入口。
- 本地不做视频盲水印。
- 预留 Rust / FRB 桥接层，后续接入真正的底层能力。
- UI 与桌面端保持同一套工作台、取证、版权库、设置结构。

当前代码入口：

- `lib/main.dart`
- `lib/bridge/watermark_bridge.dart`
- `lib/bridge/local_preview_watermark_bridge.dart`

验证：

```bash
flutter analyze
flutter test
```
