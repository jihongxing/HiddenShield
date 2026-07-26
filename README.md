# HiddenShield 隐盾

面向创作者的本地优先图片与音频版权保护工具。`v0.1.3` 为 Windows 桌面发布版本：在本地生成保护副本、回读技术校验结果，并保存本地版权记录。

> 版权编号与技术校验结果不等于国家登记、发行方数字签名、实名认证、法定权属确认或司法采信。

## 下载 v0.1.3

- [NSIS 安装器](https://github.com/jihongxing/HiddenShield/releases/download/v0.1.3/HiddenShield_0.1.3_x64-setup.exe)
- [MSI 安装器](https://github.com/jihongxing/HiddenShield/releases/download/v0.1.3/HiddenShield_0.1.3_x64_en-US.msi)
- [SHA-256 校验清单](https://github.com/jihongxing/HiddenShield/releases/download/v0.1.3/SHA256SUMS.txt)
- [发布清单与资产边界](docs/桌面v0.1.3发布清单.md)
- [GitHub Release](https://github.com/jihongxing/HiddenShield/releases/tag/v0.1.3)

安装前可在 PowerShell 中校验下载文件：

```powershell
Get-FileHash .\HiddenShield_0.1.3_x64-setup.exe -Algorithm SHA256
```

将结果与 `SHA256SUMS.txt` 中对应文件的值比较。`v0.1.3` 同时发布 Tauri updater `.sig` 和 `latest.json`，但自动更新仍处于内部验证范围；请以手动下载安装作为当前可靠更新方式。

## 当前可用能力

- **图片保护与校验**：本地生成保护副本并回读同一版权编号与载荷。
- **音频保护与校验**：在支持规格内处理本地音频并回读技术校验结果。
- **本地优先**：原始媒体、保护副本和本地路径默认不因保护流程上传。
- **离线基础流程**：安装后在物理断网条件下，图片和 WAV / MP3 / FLAC / M4A / AAC 的保护与读取冒烟已通过。
- **本地版权记录**：保存作品和保护记录，方便后续查找与技术校验。

## 使用前须知

- Windows 10/11 x64，需安装 WebView2 Runtime。
- 音频处理依赖 `ffmpeg` 与 `ffprobe`，请将其加入系统 `PATH`；应用不会在运行时联网下载它们。
- GitHub Release 中的公开安装器使用 Tauri Ed25519 updater 签名，不声明为公开 Authenticode 签名包，首次安装仍可能出现 Windows 发布者或 SmartScreen 提示。
- 当前发布不包含真实支付、公共信任层、移动端新功能或视频画面盲水印。

## 明确边界

- 图片恢复范围、音频后处理限制和版权编号语义以 [当前真实能力边界说明](docs/当前真实能力边界说明.md) 为准。
- 视频音轨、视频指纹及 L3 视频画面能力不属于 `v0.1.3` 对外承诺。
- 感知质量实验室是独立 Windows 内部工具，不属于主桌面正式功能或“绝对无感”证明。
- 正式报告、完整性摘要和时间材料不提供数字签名、权属确认或法律效力结论。

## 开发

```bash
npm install
npx tauri dev
```

构建 Windows 桌面包：

```bash
npx tauri build --bundles msi,nsis
```

发布构建、Tauri updater 签名与 GitHub Actions 配置见 [CI/CD 签名与发布配置](docs/CI-CD-签名配置.md)。发布前请阅读 [桌面 v0.1.3 发布清单](docs/桌面v0.1.3发布清单.md) 和 [当前真实能力边界说明](docs/当前真实能力边界说明.md)。

## 项目结构

```text
src/                         Vue 前端
src-tauri/                   Tauri / Rust 桌面后端
watermark-core/              图片与音频盲水印共享核心
mobile_app/                  冻结的原生移动端资产
tools/perceptual-quality-lab/ 独立内部感知质量实验室
docs/                        能力边界、发布与技术文档
```

## 参考文档

- [变更记录](CHANGELOG.md)
- [桌面 v0.1.3 发布清单](docs/桌面v0.1.3发布清单.md)
- [当前真实能力边界说明](docs/当前真实能力边界说明.md)
- [watermark-core 能力说明](docs/watermark-core能力说明.md)
- [用户协议](docs/用户协议.md)

## 许可

本项目采用 [MIT License](LICENSE)。
