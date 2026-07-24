# CI/CD 签名与发布配置

## 托管 KMS 双签名系统

### Authenticode Gate

- 对象：Windows release EXE、安装后 EXE、MSI、NSIS。
- 密钥：Azure Artifact Signing 托管的代码签名证书与私钥。
- 接口：SignTool + Artifact Signing Client `Azure.CodeSigning.Dlib.dll` + 无秘密 metadata JSON。
- Gate：`npm run release:authenticode-gate:candidate`。

Azure Artifact Signing 示例：

```powershell
.\scripts\release\sign-with-azure-artifact-signing.ps1 `
  -SigntoolPath $env:HIDDENSHIELD_SIGNTOOL_PATH `
  -DlibPath $env:HIDDENSHIELD_AZURE_SIGNING_DLIB_PATH `
  -Endpoint $env:HIDDENSHIELD_AZURE_SIGNING_ENDPOINT `
  -CodeSigningAccountName $env:HIDDENSHIELD_AZURE_SIGNING_ACCOUNT `
  -CertificateProfileName $env:HIDDENSHIELD_AZURE_SIGNING_PROFILE `
  -Files $releaseExePath,$installedExePath,$msiPath,$nsisPath `
  -EvidenceOutput "artifacts/authenticode-signing/azure-evidence.json"
```

### HSLIC1 Signer Gate

- 对象：年度 HSLIC1 与 HSRVL1。
- 密钥：Google Cloud KMS 独立 `EC_SIGN_ED25519` key version。
- 接口：Application Default Credentials + Cloud KMS `getPublicKey` / `asymmetricSign`。
- Gate：`npm run license:hslic1-signer-gate:candidate`。

两套 key 不得复用。客户机器不安装云 SDK、不持有云凭据，也不接触任何私钥。

## 当前发布策略

当前发布链路采用 GitHub Actions 构建签名包并生成 Draft GitHub Release。GitHub Releases 是公开更新源；Tauri updater 使用独立 Ed25519 私钥签名更新资产，不依赖 Azure、PFX 或 Windows 代码签名订阅。

约束如下：

- updater 公钥已固化在 `src-tauri/tauri.conf.json`；私钥仅允许配置为 GitHub `production` Environment Secret `TAURI_SIGNING_PRIVATE_KEY`，并以 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 解锁。
- Windows 发布包由 Tauri updater 签名校验完整性和来源；由于本方案不提供 Authenticode，首次安装仍可能显示未知发布者或 SmartScreen 提示。
- `v0.1.1` 的自动发布仅生成 Windows 更新资产；macOS 必须在 Apple 签名与 notarization 凭据配置完成后另行恢复。
- 音视频处理依赖 `ffmpeg` 和 `ffprobe`，生产环境需通过系统 PATH 预装，不再运行时联网下载。

### 自动更新启用门槛

在以下条件全部满足前，Release workflow 不得生成或发布 updater 消费的 `latest.json`、更新包签名或面向用户的“自动更新已可用”说明：

1. GitHub `production` Environment 已配置 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
2. GitHub Release 对用户可公开访问；私有仓库 Release 不能直接作为无凭据客户端更新源。
3. 从上一 updater-enabled 版本完成 Windows NSIS 升级冒烟，且 updater 签名、安装后版本和本地数据保留均通过。

客户端默认每 24 小时检查一次已签名更新清单；用户可在“设置 → 应用更新”关闭后台检查或手动检查。检查失败不会阻断当前版本工作。

现有 `0.1.0` 不包含 updater，必须手动安装首个 updater-enabled `0.1.1`；`0.1.2` 起的 Windows 正式版本会生成 `latest.json` 与 `.sig`，可供 `0.1.1` 应用内升级。

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

### Windows 自签 Authenticode

| Secret | 必需 | 说明 |
|---|---|---|
| `WINDOWS_SELF_SIGNED_CERTIFICATE` | 是 | 自签 Code Signing PFX 的 Base64 内容 |
| `WINDOWS_SELF_SIGNED_CERTIFICATE_PASSWORD` | 是 | PFX 导出密码，至少 16 个字符 |

使用 `npm run release:authenticode:self-signed-init -- -PfxOutput <path> -CertificatePassword <password>` 生成证书。PFX 不得提交仓库；工作流只从 GitHub encrypted secrets 临时导入。

2026-07-17 已配置：

- `WINDOWS_SELF_SIGNED_CERTIFICATE`
- `WINDOWS_SELF_SIGNED_CERTIFICATE_PASSWORD`
- 生产证书 thumbprint：`4F14DA0B5558359183E86F35486A08A34F38EAE5`
- 生产 trust policy：`config/offline-license-trust-policy.production.json`

自签证书只在服务方构建机和预装证书的专用客户机器上受信任。它不能消除普通用户环境中的未知发布者或 SmartScreen 提示。

GitHub Actions 生产工作流使用 RFC3161/TSP 模式和 `http://timestamp.digicert.com`；本地自签脚本默认使用 `http://timestamp.sectigo.com`。两者都只用于获取带数字签名的时间戳响应，不能被解释为代码签名证书的公共信任来源。

### 2026-07-22 RC-RELEASE-001 四文件候选

- 当前锁定候选在不重新构建的前提下，对 NSIS、MSI、release EXE 和当前 installed EXE 原地完成自签 Authenticode。
- 四个文件均由 `CN=HiddenShield Release Signing`、thumbprint `4F14DA0B5558359183E86F35486A08A34F38EAE5` 签名，`Get-AuthenticodeSignature` 与 SignTool `/pa /all /v` 均通过。
- `release:authenticode-gate:candidate` 现在要求四个路径，并对每个副本执行篡改后失效验证。
- 本轮本地自签脚本使用 RFC3161 时间戳服务；当前 Windows 候选统一使用已验证的 `http://timestamp.digicert.com`，四个签名均检测到时间戳。
- 自签证书只适用于服务方与受管客户 trust store，不等于公共 CA 或 SmartScreen 信誉。
- 签名前清单、签名证据和 SignTool 日志位于 `artifacts/authenticode-signing/20260722-rc-release-001/`。
- 下一步：在干净离线 Windows 快照分别安装签名 NSIS / MSI，确认新安装的内层 EXE 仍为 `Valid`。

### 软件 HSLIC1 签发

HSLIC1 私钥不进入 GitHub Actions。它保存在服务方专用本地用户目录中的口令加密文件，并由 `offline_license_issuer` 在人工授权后签发年度注册码和撤销列表。

Google Cloud KMS 与 Azure Artifact Signing 配置只保留为未来付费增强，不属于当前 Release workflow 必需 secrets。

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

- 已生成非 fixture 软件 HSLIC1 key，并将公钥加入桌面编译期 trust policy。
- 已生成 `CN=HiddenShield Release Signing` 自签 Code Signing PFX，并配置两个 Windows GitHub secrets。
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
git tag v0.1.1
git push origin v0.1.1
```

3. GitHub Actions 自动执行校验、签名、构建、公证。
4. Workflow 会生成 GitHub Draft Release，并上传安装包。
5. 人工验收签名、公证和安装流程后，再手动发布 Release。

如需手动补发某个版本，可在 GitHub Actions 页面运行 `Release` workflow 并填写已有 tag；工作流会直接检出该 tag 对应的提交进行构建。

## 发布后验收

建议最少完成以下验收动作：

- Windows：在安装了 HiddenShield 自签证书的验证环境中确认 EXE、MSI、NSIS 为 `Valid`，并记录普通未预置信任环境的安全警告。
- macOS：验证 `.app` 或 `.dmg` 已 notarized，且首次安装不触发未知开发者阻断。
- Windows/macOS：验证首次启动、核心转码、离线模式、联网取证开关均符合预期。
- 验证安装后的目标环境确实能找到 `ffmpeg` 与 `ffprobe`。

## 相关脚本

工作流依赖以下脚本：

- [verify-release.mjs](/D:/codeSpace/HiddenShield/scripts/release/verify-release.mjs)
- [initialize-self-signed-authenticode.ps1](/D:/codeSpace/HiddenShield/scripts/release/initialize-self-signed-authenticode.ps1)
- [inject-windows-signing.ps1](/D:/codeSpace/HiddenShield/scripts/release/inject-windows-signing.ps1)
- [sign-with-self-signed-authenticode.ps1](/D:/codeSpace/HiddenShield/scripts/release/sign-with-self-signed-authenticode.ps1)
- [write-self-signed-authenticode-evidence.ps1](/D:/codeSpace/HiddenShield/scripts/release/write-self-signed-authenticode-evidence.ps1)
- [import-apple-certificate.sh](/D:/codeSpace/HiddenShield/scripts/release/import-apple-certificate.sh)
- [prepare-apple-notarization.sh](/D:/codeSpace/HiddenShield/scripts/release/prepare-apple-notarization.sh)

这些脚本的目标是让 Windows 免费发布路径不依赖 GCP Billing 或 Azure Subscription。Apple 资质仍需由发布负责人单独提供；未来如升级托管签名，可重新启用现有 Google KMS 与 Azure Artifact Signing adapter。
