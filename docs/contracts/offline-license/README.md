# HiddenShield 离线许可证合同 v1

## Phase K4 信任策略

`trust-policy-v1.schema.json` 冻结客户端公钥 ring 与轮换边界。密钥状态只能为
`active`、`verify_only` 或 `disabled`；验证时还必须匹配 `license` /
`revocation` 用途和密钥有效时间。生产构建不得信任 fixture 中公开的测试公钥。

`runtime-security-state-v1.schema.json` 冻结最小防回滚状态：历史最高可信 UTC
时间，以及按 `keyId` 保存的撤销序列号与 payload digest 高水位。更低序列号视为
回放；相同序列号但 digest 不同视为签发侧分叉。安全存储或该状态无法读取时必须
fail closed。

发布完整性以操作系统安装包签名链为权威。进程内可执行文件自哈希只允许用于诊断，
不得作为开放离线权益的可信依据。

生产公钥策略必须在构建期注入，不能从用户可写目录动态加载：

- Tauri / Rust：构建环境变量 `HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON`
- Flutter：`--dart-define=HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON=<json>`

两端未注入策略时均 fail closed。`internal-qa` 和单元测试可以使用公开 fixture
公钥，但该路径不得作为普通 production build 的信任来源。

## 1. 主载体

Phase K0 冻结主许可证载体为：

```text
HSLIC1.<payload>.<signature>
```

- `HSLIC1`：固定 ASCII 前缀。
- `payload`：`license-payload-v1.schema.json` 对应 UTF-8 canonical JSON 的 Base64URL 编码，不带 `=` padding。
- `signature`：64 字节 Ed25519 签名的 Base64URL 编码，不带 `=` padding。
- 完整字符串不得包含空格、换行或其他前后缀。
- 首期正式合同将完整 token 长度限制为 `300–500` 个 ASCII 字符。

## 2. Canonical JSON

v1 使用受限 canonical JSON：

- UTF-8，无 BOM。
- 不允许空白格式化。
- 不允许未知字段。
- 对象字段按 schema 中冻结的 ASCII 字段顺序输出。
- 字符串使用标准 JSON 转义。
- `schemaVersion` 是十进制整数。
- 验证器必须确认解析后重新序列化的字节与原始 payload 字节完全相同。

许可证字段顺序固定为：

```text
expiresAt
installationId
issuedAt
keyId
licenseId
notBefore
productCode
schemaVersion
```

## 3. 签名消息

签名消息是以下字节的直接拼接：

```text
UTF8("HiddenShield-Offline-License-v1") || 0x00 || canonicalPayloadBytes
```

Ed25519 签名不覆盖 Base64URL 文本，而是覆盖解码后的 canonical payload 字节。域分隔符和 NUL 字节不得改变。

## 4. 安全边界

- 客户端只保存签发公钥，不保存签发私钥或对称生成秘密。
- `docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json` 中的私钥 seed 是公开测试材料，禁止用于真实许可证。
- `productCode=creator_offline` 只表示固定本地权益模板，不允许 payload 自由声明云端能力。
- `creator_offline` 的具体 feature allowlist 由客户端商业化契约冻结，不进入可自由组合的签名字段。
- Phase K0 只冻结载体、schema 和验证向量，不代表已经具备可销售的激活能力。

## 5. 共享测试向量

固定向量：

```text
docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json
```

该向量必须同时通过：

- TypeScript：`npm run license:k0-contract`
- Rust：`cargo test --manifest-path src-tauri/Cargo.toml offline_license`
- Dart：`flutter test test/offline_license_test.dart`

## 6. HSREQ1 激活请求

载体：

```text
HSREQ1.<payload>.<checksum>
```

- `payload`：`activation-request-payload-v1.schema.json` 对应 canonical JSON 的 Base64URL-NoPad。
- `checksum`：`SHA-256(UTF8("HiddenShield-Offline-Activation-Request-v1") || 0x00 || payloadBytes)` 的前 12 字节，Base64URL-NoPad 后固定 16 字符。
- checksum 只检测复制、二维码或文件传输错误，不证明请求来自可信设备。
- 固定字段顺序：`appVersion`、`createdAt`、`installationId`、`nonce`、`platform`、`requestId`、`requestedProductCode`、`schemaVersion`。

固定向量：

```text
docs/fixtures/offline-license-k0/hsreq1-v1-valid.json
```

## 7. HSRVL1 签名撤销列表

载体：

```text
HSRVL1.<payload>.<signature>
```

- 签名消息：`UTF8("HiddenShield-Offline-Revocation-List-v1") || 0x00 || payloadBytes`。
- `revokedLicenseIds` 必须按 ASCII 升序排列且不得重复。
- `sequence` 必须从 1 开始单调递增；客户端不得接受低于本机已见序号的列表。
- 固定字段顺序：`generatedAt`、`keyId`、`listId`、`listType`、`revokedLicenseIds`、`schemaVersion`、`sequence`。

固定向量：

```text
docs/fixtures/offline-license-k0/hsrvl1-ed25519-v1-valid.json
```

## 8. 错误合同

跨端稳定错误向量：

```text
docs/fixtures/offline-license-k0/offline-license-errors-v1.json
```

错误向量使用“有效 fixture + 确定性 mutation + 期望错误码”，三端必须执行相同字节变换并返回完全相同的错误码。

## 9. Installation identity

派生公式：

```text
Base64URL-NoPad(
  SHA-256(
    UTF8("HiddenShield-Installation-v1") || 0x00 ||
    installationSecret[32] ||
    salt[16]
  )
)
```

- `installationSecret` 必须由系统安全随机数生成，只进入平台安全存储。
- salt 可以与 installation metadata 一同持久化。
- 数据库和日志不得保存 secret。
- 固定跨端向量：`docs/fixtures/offline-license-k0/installation-identity-v1.json`。
