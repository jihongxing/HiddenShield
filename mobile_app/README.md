# HiddenShield Mobile

Flutter 双端 App 的移动端壳。

当前状态：

- 只做图片和 WAV 音频的本地确权入口。
- 本地不做视频盲水印。
- 预留 Rust / FRB 桥接层，后续接入真正的底层能力。
- UI 与桌面端保持同一套工作台、取证、版权库、设置结构。
- 图片嵌入测试页已接入文件选择、预览、重写开关和 preview 结果摘要。
- 音频嵌入页已接入 WAV 文件选择、重写开关和结果摘要。
- 取证页已接入图片/WAV 切换、文件选择和提取结果摘要。
- 移动端已加入 SQLite 版权库，写入和取证命中会持久化进入版权库时间线。
- 移动端已加入本地同步队列，写入和取证命中会进入待发送队列。
- 设置页已接入本机创作者身份、桌面端配对、匿名反馈开关、待同步队列数、本地模拟同步和桌面 HTTP 同步模式。

当前代码入口：

- `lib/main.dart`
- `lib/app/app.dart`
- `lib/app/mobile_app_state.dart`
- `lib/app/mobile_shell.dart`
- `lib/storage/vault_store.dart`
- `lib/sync/sync_transport.dart`
- `lib/features/workspace/workspace_page.dart`
- `lib/bridge/watermark_bridge.dart`
- `lib/bridge/local_preview_watermark_bridge.dart`
- `lib/bridge/rust_watermark_bridge.dart`
- `lib/src/rust/`（FRB 生成文件）

桥接说明：

- `PreviewWatermarkBridge` 是 Flutter 侧占位实现，用于保持页面和测试稳定。
- `RustWatermarkBridge` 已包住 FRB 生成的图片与 WAV 音频写入 / 提取 API，并支持提取。
- 默认 App 启动时会尝试初始化 Rust bridge，失败时回落到 preview bridge。
- `rust_builder/` 是 FRB `integrate` 生成的 Cargokit FFI plugin，用于 Android/iOS 构建 Rust 库。

版权库说明：

- 真机启动默认使用 `SQLiteVaultStore`，数据库文件为 `hidden_shield_mobile.db`。
- Widget test 和预览默认使用 `MemoryVaultStore`，避免测试依赖平台数据库插件。
- 当前数据库版本为 4。
- `vault_records` 表覆盖水印 UID、revision、父级 UID、重写原因、同步状态和创建时间。
- `sync_queue` 表记录待同步事件，当前事件类型为版权记录 upsert 和取证记录 upsert。
- `sync_profile` 表保存桌面端地址、配对码、配对状态和更新时间。

同步说明：

- `SyncTransport` 是同步传输抽象。
- `LocalMockSyncTransport` 是当前默认实现，只模拟本地发送，不访问网络。
- `DesktopHttpSyncTransport` 已提供真实 HTTP 传输骨架，请求协议见 `docs/移动端桌面同步协议草案.md`。
- 桌面配对使用桌面端设置页生成的配对码；桌面端会拒绝配对码缺失或不匹配的同步写入。
- 设置页可在“本地模拟”和“桌面 HTTP”之间切换；只有保存桌面地址和配对码后才能启用 HTTP 模式。
- 设置页“模拟同步”会消费 pending 队列，状态流转为 `pending -> syncing -> synced/failed`。
- 失败项会记录 `attempts` 和 `last_error`，后续接真实桌面同步时可复用同一套状态机。
- Android `debug` / `profile` 构建已允许局域网 HTTP，用于访问桌面端 `http://<局域网 IP>:47219`；`release` 构建不默认放宽明文 HTTP。
- 设置页提供“联调检查”和“同步诊断”，用于确认桌面地址、配对码、HTTP 通道、队列和最近错误。

联调诊断脚本：

```bash
HIDDENSHIELD_SYNC_URL=http://<desktop-lan-ip>:47219 HIDDENSHIELD_PAIRING_CODE=<code> npm run mobile:doctor
```

重新生成 FRB 绑定：

```bash
flutter_rust_bridge_codegen generate --rust-root rust --rust-input crate::api --dart-output lib/src/rust --c-output rust/include/hidden_shield_mobile_bridge.h --no-web
```

Android 构建前需要安装 Rust Android targets：

```bash
rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi i686-linux-android
```

当前 Windows 环境已完成 Cargokit 工程接线；安装上述 targets 后，`flutter build apk --debug` 可生成 Android debug APK。

验证：

```bash
flutter analyze
flutter test
```
