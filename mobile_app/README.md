# HiddenShield Mobile

Flutter 双端 App 的移动端壳。

当前状态：

- 只做图片和 WAV 音频的本地确权入口。
- 本地不做视频盲水印。
- 预留 Rust / FRB 桥接层，后续接入真正的底层能力。
- UI 与桌面端保持同一套工作台、取证、版权库、设置结构。
- 图片嵌入测试页已接入文件选择、预览、重写开关和 preview 结果摘要。
- 音频嵌入页已接入 WAV 文件选择、重写开关和结果摘要。

当前代码入口：

- `lib/main.dart`
- `lib/app/app.dart`
- `lib/app/mobile_shell.dart`
- `lib/features/workspace/workspace_page.dart`
- `lib/bridge/watermark_bridge.dart`
- `lib/bridge/local_preview_watermark_bridge.dart`
- `lib/bridge/rust_watermark_bridge.dart`
- `lib/src/rust/`（FRB 生成文件）

桥接说明：

- `PreviewWatermarkBridge` 是 Flutter 侧占位实现，用于保持页面和测试稳定。
- `RustWatermarkBridge` 已包住 FRB 生成的图片与 WAV 音频写入 / 提取 API。
- 默认 App 启动时会尝试初始化 Rust bridge，失败时回落到 preview bridge。
- `rust_builder/` 是 FRB `integrate` 生成的 Cargokit FFI plugin，用于 Android/iOS 构建 Rust 库。

重新生成 FRB 绑定：

```bash
flutter_rust_bridge_codegen generate --rust-root rust --rust-input crate::api --dart-output lib/src/rust --c-output rust/include/hidden_shield_mobile_bridge.h --no-web
```

Android 构建前需要安装 Rust Android targets：

```bash
rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi i686-linux-android
```

当前 Windows 环境已完成 Cargokit 工程接线；安装上述 targets 后，`flutter build apk --debug` 已可生成 Android debug APK。

验证：

```bash
flutter analyze
flutter test
```
