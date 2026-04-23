# CI/CD 签名与发布配置

## 当前发布策略

当前生产发布链路采用 GitHub Actions 构建签名包并生成 Draft Release。

约束如下：

- 应用内自动更新已禁用，不再使用 Tauri updater，也不生成 `latest.json`。
- Windows 发布包必须使用真实代码签名证书签名。
- macOS 发布包必须完成签名与 notarization。
- 音视频处理依赖 `ffmpeg` 和 `ffprobe`，生产环境需通过系统 PATH 预装，不再运行时联网下载。

## GitHub Actions 触发方式

已配置的工作流文件为 [release.yml](/D:/codeSpace/HiddenShield/.github/workflows/release.yml)。

触发方式：

- 推送 tag：`vX.Y.Z`
- 手动触发 `Release` workflow，并填写 `release_tag`

工作流会先校验：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

这三处版本必须完全一致，且 tag 必须等于 `v<version>`。

## 必需 Secrets

在 GitHub 仓库 `Settings -> Secrets and variables -> Actions` 中配置以下 secrets。

### Windows 代码签名

| Secret | 必需 | 说明 |
|---|---|---|
| `WINDOWS_CERTIFICATE` | 是 | PFX 证书文件的 Base64 内容 |
| `WINDOWS_CERTIFICATE_PASSWORD` | 是 | PFX 证书密码 |
| `WINDOWS_TIMESTAMP_URL` | 是 | 代码签名时间戳服务地址，必须为 HTTPS |
| `WINDOWS_TSP` | 否 | 是否启用 RFC3161/TSP，填写 `true` 或 `false` |

将 PFX 转为 Base64 的示例：

```bash
base64 -i certificate.pfx > certificate.base64.txt
```

### macOS 签名

| Secret | 必需 | 说明 |
|---|---|---|
| `APPLE_CERTIFICATE` | 是 | `.p12` 签名证书的 Base64 内容 |
| `APPLE_CERTIFICATE_PASSWORD` | 是 | `.p12` 导出密码 |
| `KEYCHAIN_PASSWORD` | 是 | CI 临时 keychain 密码 |

工作流会自动导入证书并解析可用的 `Developer ID Application` 身份，无需手动填写 `APPLE_SIGNING_IDENTITY`。

### macOS 公证

二选一，推荐优先使用 App Store Connect API Key。

App Store Connect API Key 模式：

| Secret | 必需 | 说明 |
|---|---|---|
| `APPLE_API_KEY` | 是 | Key ID |
| `APPLE_API_ISSUER` | 是 | Issuer ID |
| `APPLE_API_KEY_CONTENT` | 是 | `.p8` 私钥原文内容 |

Apple ID 模式：

| Secret | 必需 | 说明 |
|---|---|---|
| `APPLE_ID` | 是 | Apple ID 邮箱 |
| `APPLE_PASSWORD` | 是 | App-Specific Password |
| `APPLE_TEAM_ID` | 是 | Apple Developer Team ID |

## 发布前检查

发版前必须先满足这些前置条件：

- 已取得真实 Windows 代码签名证书，并确认可用于桌面分发。
- 已取得 Apple Developer 账号、Developer ID Application 证书和 notarization 权限。
- 版本号已同步更新到：
  - [package.json](/D:/codeSpace/HiddenShield/package.json)
  - [Cargo.toml](/D:/codeSpace/HiddenShield/src-tauri/Cargo.toml)
  - [tauri.conf.json](/D:/codeSpace/HiddenShield/src-tauri/tauri.conf.json)
- 本地已完成至少一次 `npm run build` 与 `cargo check`。
- 运维侧已准备好面向用户的 FFmpeg 安装方案。

## 标准发布流程

1. 更新版本号，并提交到主分支。
2. 创建并推送 tag，例如：

```bash
git tag v0.1.0
git push origin v0.1.0
```

3. GitHub Actions 自动执行校验、签名、构建、公证。
4. Workflow 会生成 GitHub Draft Release，并上传安装包。
5. 人工验收签名、公证和安装流程后，再手动发布 Release。

如需手动补发某个版本，可在 GitHub Actions 页面运行 `Release` workflow 并填写已有 tag；工作流会直接检出该 tag 对应的提交进行构建。

## 发布后验收

建议最少完成以下验收动作：

- Windows：验证安装包数字签名链和时间戳可见。
- macOS：验证 `.app` 或 `.dmg` 已 notarized，且首次安装不触发未知开发者阻断。
- Windows/macOS：验证首次启动、核心转码、离线模式、联网取证开关均符合预期。
- 验证安装后的目标环境确实能找到 `ffmpeg` 与 `ffprobe`。

## 相关脚本

工作流依赖以下脚本：

- [verify-release.mjs](/D:/codeSpace/HiddenShield/scripts/release/verify-release.mjs)
- [inject-windows-signing.ps1](/D:/codeSpace/HiddenShield/scripts/release/inject-windows-signing.ps1)
- [import-apple-certificate.sh](/D:/codeSpace/HiddenShield/scripts/release/import-apple-certificate.sh)
- [prepare-apple-notarization.sh](/D:/codeSpace/HiddenShield/scripts/release/prepare-apple-notarization.sh)

这些脚本的目标是把仓库保持在“只差真实 secrets 就能发版”的状态；证书、私钥和 Apple 资质本身仍需由运维或发布负责人提供。
