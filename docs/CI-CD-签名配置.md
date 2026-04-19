# CI/CD 签名与发布配置

## GitHub Secrets 配置清单

在 GitHub 仓库 Settings → Secrets and variables → Actions 中配置以下 Secrets：

### Tauri Updater 签名密钥

| Secret 名称 | 说明 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater 签名私钥（通过 `tauri signer generate -w ~/.tauri/myapp.key` 生成） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码 |

### Windows 代码签名

| Secret 名称 | 说明 |
|---|---|
| `WINDOWS_CERTIFICATE` | EV Code Signing 证书 PFX 文件的 Base64 编码 |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX 证书密码 |

将 PFX 转为 Base64：
```bash
base64 -i certificate.pfx -o certificate_base64.txt
```

### macOS 签名与公证

| Secret 名称 | 说明 |
|---|---|
| `APPLE_CERTIFICATE` | Apple Developer 证书 P12 文件的 Base64 编码 |
| `APPLE_CERTIFICATE_PASSWORD` | P12 证书密码 |
| `APPLE_SIGNING_IDENTITY` | 签名身份（如 `Developer ID Application: Your Name (TEAM_ID)`） |
| `APPLE_ID` | Apple ID 邮箱（用于 Notarization） |
| `APPLE_PASSWORD` | App-Specific Password（在 appleid.apple.com 生成） |
| `APPLE_TEAM_ID` | Apple Developer Team ID |

## 发布流程

1. 本地开发完成后，更新 `src-tauri/tauri.conf.json` 中的 `version`
2. 提交代码并推送
3. 创建 tag 并推送：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
4. GitHub Actions 自动触发构建
5. 构建完成后在 Releases 页面生成 Draft Release
6. 检查产物无误后发布 Release

## Updater 端点

构建产物中会自动生成 `latest.json`，上传到 Release Assets。

`tauri.conf.json` 中的 updater endpoint 配置为：
```
https://github.com/YOUR_ORG/hidden-shield/releases/latest/download/latest.json
```

发布后需将 `YOUR_ORG` 替换为实际的 GitHub 组织/用户名。

## 生成 Updater 签名密钥

```bash
npx @tauri-apps/cli signer generate -w ~/.tauri/hiddenshield.key
```

将生成的私钥内容设置为 `TAURI_SIGNING_PRIVATE_KEY`，公钥填入 `tauri.conf.json` 的 `plugins.updater.pubkey`。
