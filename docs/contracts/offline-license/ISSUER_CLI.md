# HiddenShield 内部离线许可证签发 CLI

## 与 Authenticode 的术语隔离

- 本 CLI 只属于 **HSLIC1 Signer Gate**，只签发年度 `HSLIC1` 和 `HSRVL1`。
- Windows EXE、MSI、NSIS 由独立 **Authenticode Gate** 和自签 Code Signing 证书处理。
- HSLIC1 Ed25519 私钥与 Authenticode 私钥必须是不同 key，不允许复用 key handle、轮换批次或审计记录。
- 客户桌面只导入注册码并使用公钥离线验签，不持有签发私钥或签发口令。

## 安全边界

- 二进制名称：`offline_license_issuer`。
- 只供 HiddenShield 内部运营或授权管理员在受控机器运行。
- 不得打包进用户安装程序，不得向客户交付。
- 当前免费生产路径使用 Argon2id 派生的 256-bit 密钥和 XChaCha20-Poly1305 加密 Ed25519 seed。
- 签发时私钥会短暂进入 `offline_license_issuer` 进程内存；服务方机器被完全控制时不能承诺私钥不可导出。
- 密钥文件同时绑定 `keyId` 和 Ed25519 公钥作为 AEAD AAD；密码错误、文件损坏或元数据被修改均拒绝解密。
- 密码只通过指定环境变量读取，不接受命令行明文密码。
- 密码至少 16 个字符；正式环境应使用密码管理器生成的随机长密码。
- Windows 正式签发目录必须额外配置 NTFS ACL；Unix 密钥文件创建为 `0600`。
- CLI 不接受 feature map 或 Studio / Enterprise 模板，只能签发冻结的 `creator_offline`。

## 构建

生产软件签发器：

```text
cargo build --manifest-path src-tauri/Cargo.toml --example offline_license_issuer
```

## 命令

生成加密签发密钥：

```text
offline_license_issuer keygen --output issuer-key.json --key-id offline-ed25519-2026-q3 --password-env HIDDENSHIELD_LICENSE_KEY_PASSWORD
```

检查 `HSREQ1`：

```text
offline_license_issuer inspect-request --request request.hsreq
```

生产签发许可证：

```text
offline_license_issuer issue --key issuer-key.json --password-env HIDDENSHIELD_LICENSE_KEY_PASSWORD --request request.hsreq --expires-at 2027-07-15T00:00:00Z --operator-id operator-001 --output license.hslicense --audit-output license-audit.json
```

验证许可证：

```text
offline_license_issuer verify-license --license license.hslicense --public-key PUBLIC_KEY_BASE64URL
```

生产签名撤销列表：

```text
offline_license_issuer sign-revocations --key issuer-key.json --password-env HIDDENSHIELD_LICENSE_KEY_PASSWORD --input revocation-draft.json --operator-id operator-001 --output revocations.hsrvl --audit-output revocation-audit.json
```

验证撤销列表：

```text
offline_license_issuer verify-revocations --revocations revocations.hsrvl --public-key PUBLIC_KEY_BASE64URL
```

## 撤销草稿

```json
{
  "listId": "rvl_2026_0001",
  "generatedAt": "2026-07-15T00:00:00Z",
  "sequence": 1,
  "revokedLicenseIds": ["lic_..."]
}
```

CLI 会对 `revokedLicenseIds` 排序并拒绝重复项。`keyId`、`listType` 和 `schemaVersion` 由签发器固定。

## 托管 KMS 隔离签名协议（未来付费增强）

配置示例：

```json
{
  "schemaVersion": 1,
  "signerType": "managed_kms",
  "keyId": "offline-production-2026-q3",
  "publicKeyBase64Url": "PUBLIC_KEY_BASE64URL",
  "keyHandle": "gcp-kms://projects/PROJECT_ID/locations/global/keyRings/hiddenshield-license/cryptoKeys/hslic1-ed25519/cryptoKeyVersions/1",
  "command": "C:\\Program Files\\nodejs\\node.exe",
  "arguments": [
    "D:\\HiddenShield\\scripts\\signers\\hslic1-google-cloud-kms-signer.mjs",
    "--crypto-key-version",
    "projects/PROJECT_ID/locations/global/keyRings/hiddenshield-license/cryptoKeys/hslic1-ed25519/cryptoKeyVersions/1",
    "--key-id",
    "offline-production-2026-q3",
    "--expected-public-key-base64url",
    "PUBLIC_KEY_BASE64URL"
  ]
}
```

- `command` 必须是绝对路径。
- 配置拒绝未知字段，不得加入私钥、seed、PFX、PEM 或密码。
- 签名请求以单个 JSON 对象写入 stdin，包含 `operation=ed25519_sign`、`keyId`、`keyHandle`、`purpose` 和 `messageBase64Url`。
- 签名器以单个 JSON 对象写入 stdout，包含 `schemaVersion`、`keyId` 和 `signatureBase64Url`。
- HiddenShield 使用配置公钥复验返回的 64 字节 Ed25519 签名；无效签名、错误 Key ID、错误协议或非零退出码全部 fail closed。

### Google Cloud KMS 实际接入

HSLIC1 V1 算法保持 Ed25519。正式 key version 必须使用 `EC_SIGN_ED25519`；adapter 将冻结 signing message 放入 `asymmetricSign.data`，不预先哈希。

`scripts/signers/hslic1-google-cloud-kms-signer.mjs` 会：

- 通过 Application Default Credentials 获取最小权限身份。
- 调用 `getPublicKey` 并要求 algorithm 为 `EC_SIGN_ED25519`。
- 比对 key version、Ed25519 公钥和允许的 protection level。
- 为请求 `data` 提交 CRC32C，并要求 `verifiedDataCrc32c=true`。
- 校验返回签名 CRC32C 与 64 字节长度。
- 将签名交回 issuer，由 issuer 使用冻结公钥再次复验。

服务方身份只需要目标 key version 的 `cloudkms.cryptoKeyVersions.useToSign` 和读取公钥所需权限。生产配置不得加入 service-account JSON、access token 或其他凭据；凭据通过 ADC、Workload Identity 或受控服务账号环境提供。

非秘密模板：`config/hslic1-signer.production.example.json`。

旧 `--hardware-signer-config`、PKCS#11 adapter 和 `external_hardware` signer type 只保留兼容与迁移，不属于当前 GA 必需路径。

当前免费正式 Gate：

```text
set HIDDENSHIELD_HSLIC1_SOFTWARE_KEY_PATH=C:\HiddenShield\issuer-key.json
set HIDDENSHIELD_HSLIC1_SOFTWARE_KEY_PASSWORD=从密码管理器注入的长口令
set HIDDENSHIELD_HSLIC1_REQUEST_PATH=C:\HiddenShield\release-gate.hsreq
npm run license:hslic1-signer-gate:candidate
```

## 审计

许可证和撤销列表签发均要求独立 `--audit-output` 和 `--operator-id`，记录：

- 事件类型。
- `licenseId` 或 `listId`。
- `keyId`。
- 产品、安装实例或撤销数量。
- 签发时间和有效期。
- canonical payload 的 SHA-256。
- 最终 token 的 SHA-256。

许可证审计还记录独立 `serialNumber`。转移或人工重签时必须同时传入
`--replaces-license-id` 与 `--reason`；两者只进入审计，不修改冻结的 v1 签名
payload。

审计文件不包含私钥、密码或明文 seed。

## QA

```text
npm run license:k1-cli-qa
```

QA 使用临时目录和临时密钥，验证：

- 密钥生成与加密保存。
- `HSREQ1` 导入。
- 许可证签发和公钥验证。
- 撤销列表签发和公钥验证。
- 错误密码拒绝。
- 未知模板参数拒绝。
- 非法或损坏的激活请求拒绝。
- 许可证和撤销审计文件存在。
