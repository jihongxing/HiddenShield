# HiddenShield CDKEY 离线激活与本地许可证设计

状态：Phase K0–K4 内部最小实现完成；2026-07-17 冻结零成本软件签名与攻击 Gate 基线
能力分类：`只能内部测试`

## 两套私钥与 Gate

### Authenticode Gate（不属于许可证签发）

- EXE、MSI、NSIS 由独立自签 Code Signing 证书签名。
- 它的目标是在服务方和已安装信任根的专用客户环境中识别发布者与二进制篡改。
- 它不得签发 HSLIC1 / HSRVL1，也不得与许可证签发 key 共用。
- 自签证书不具备公共 Windows 信任，普通用户可能看到未知发布者或 SmartScreen 提示。
- 执行：`npm run release:authenticode-gate:candidate`。

### HSLIC1 Signer Gate

- 许可证签发 key 固定为 Ed25519，只签 HSLIC1 / HSRVL1。
- 服务方使用口令加密软件密钥文件；客户机器无需硬件、云账号或联网，并且只持有验证公钥。
- 执行：`npm run license:hslic1-signer-gate:candidate`。
- 候选 Gate 必须用真实非 fixture 加密密钥文件完成一年期 HSLIC1 与 HSRVL1 签名，并由 issuer 使用公钥复验；临时 fixture key 不能作为候选证据。

两套 Gate 的汇总命令仍为：

```text
npm run license:security-attack-gate:candidate
```

## 2026-07-17 正式免费签名材料

- 正式 HSLIC1 key ID：`offline-production-2026-07-17-v1`。
- 正式 Ed25519 公钥：`idGJrKyJC86KSMGA5rCDRNN9ZG2Vj7ii7RSNUdLHK1U`。
- 桌面生产 trust policy：`config/offline-license-trust-policy.production.json`。
- 正式自签证书 subject：`CN=HiddenShield Release Signing`。
- 正式自签证书 thumbprint：`4F14DA0B5558359183E86F35486A08A34F38EAE5`。
- 私钥、PFX 与 DPAPI 口令恢复文件只保存在仓库外服务方目录，不进入 Git、构建日志或 Gate 证据。
- GitHub 只保存 PFX Base64 与 PFX 密码两个 encrypted secrets；HSLIC1 私钥和口令不进入 GitHub Actions。
- 正式候选 Gate 证据：`artifacts/offline-license-security-gate/20260717194912/offline-license-security-gate.json`。

## 2026-07-17 冻结决策

- 不建设后端在线许可证验证，不在 `feedback-backend` 增加许可证在线激活、周期租约、启动联网校验或服务端合法性依赖。
- 生产许可证签名使用 `Argon2id + XChaCha20-Poly1305` 口令加密 Ed25519 seed 文件；桌面端只持有公钥。该 Ed25519 key 与自签 Authenticode PFX 完全独立。
- `offline_license_issuer keygen`、`--key` 和 `--password-env` 属于当前正式免费路径；签发口令不得写入仓库、配置文件、审计或命令行参数。
- Google Cloud KMS、PKCS#11 和外部隔离签名协议继续保留为未来付费增强，不属于当前候选 Gate 必选项。
- 完整快照回滚是已知限制：如果攻击者同时恢复 SQLite、系统安全存储、系统时间和应用数据，纯离线客户端没有外部单调事实可用于证明回滚发生。

## 安全 Gate 验收

### Authenticode Gate：篡改二进制

- 输入必须是同一正式候选的 Authenticode `Valid` release EXE、installed EXE、MSI 和 NSIS。
- Gate 对副本执行单字节修改，修改后的 Authenticode 状态必须不再为 `Valid`。
- 该 Gate 证明篡改可被签名验证识别，不承诺 Windows 必然阻止管理员运行修改后的文件。

### 客户端攻击 Gate：复制数据库

- 将 SQLite 与 HSLIC1 复制到使用不同系统安全存储 secret 的安装实例。
- 必须返回 `offline_license_installation_identity_mismatch` 或 `offline_license_device_mismatch`。
- `batch_processing` 必须保持关闭。

### 客户端攻击 Gate：完整快照回滚

- 测试同时回滚数据库、安全存储锚点、系统时间和应用数据。
- 当前预期结果是旧的有效许可证状态可能重新变为可用。
- Gate 状态固定为 `known_limitation_reproduced`；发布必须披露限制，不得将其包装成已阻止攻击。

执行命令：

```text
npm run license:security-attack-gate
npm run license:security-attack-gate:candidate
```

2026-07-17 合同证据：`artifacts/offline-license-security-gate/20260717152623/offline-license-security-gate.json`。当前数据库复制 Gate 已通过，完整快照限制已复现；候选强制模式阻塞证据 `artifacts/offline-license-security-gate/20260717152627/offline-license-security-gate.json` 记录仍缺 Authenticode `Valid` 的 EXE、MSI 和 NSIS。

## 1. 目标

在不接入真实支付渠道、不要求激活设备联网的前提下，为 HiddenShield 提供可由内部人员在本地签发、由用户在本机离线激活的许可证机制。

首期目标：

- 内部签发工具可在受控电脑本地生成许可证。
- 桌面端可离线导入或粘贴许可证。
- 许可证可绑定单个应用安装实例，降低直接分享激活码的价值。
- 客户端只持有验证公钥，不持有签发私钥。
- 离线许可证只能开放本地能力，不得伪造云同步、云视频、团队空间或 Enterprise API 权益。
- 当前只展示未付费与图片 / 音频年度基础权益；旧套餐代码仅保留内部兼容。

非目标：

- 不承诺无法破解。
- 不依靠隐藏算法、混淆字符串或客户端内置对称密钥作为主要安全措施。
- 不用离线许可证证明付款、退款、账户所有权或合同履约。
- 不允许客户端自行签发许可证。
- 不让离线许可证绕过服务端 quota、API key、云任务或团队权限检查。

## 2. 核心安全结论

### 2.1 不采用客户端内置对称密钥

如果生成算法和验证算法共享同一个秘密，并把秘密放进桌面或移动客户端，攻击者最终可以从二进制中提取该秘密并批量生成合法 CDKEY。

因此首期必须使用非对称签名：

- 内部签发工具持有私钥。
- 正式客户端只内置公钥。
- 客户端只验证签名，不具备生成合法许可证的能力。

### 2.2 完全离线的短 CDKEY 存在长度与安全冲突

一个可离线验证、包含产品、设备绑定、有效期和签名的许可证，无法同时保持传统 20 至 30 位短 CDKEY。

推荐交付形态：

1. `.hslicense` 许可证文件，首选。
2. 一段可复制的 `HSLIC1...` Base64URL / Base32 许可证码。
3. 二维码，适合移动端导入。

不推荐：

- 只包含随机序列号的短码，因为离线客户端没有可信数据库判断该序列号是否已签发。
- 在客户端内置序列号生成秘密或完整有效码列表。

## 3. 产品边界

首期只签发“图片 / 音频年费授权”，按年激活与续期。

HSLIC1 V1 为兼容现有 schema 继续使用 `productCode=creator_offline`，但该代码不得作为用户可见套餐名，只开放本地图片 / 音频批量能力：

```json
{
  "cloud_sync": false,
  "batch_processing": true,
  "report_export": false,
  "cloud_batch_processing": false,
  "cloud_video_processing": false,
  "priority_queue": false,
  "team_workspace": false,
  "api_access": false
}
```

规则：

- 未付费用户不能使用批量处理；有效 HSLIC1 只开放本地图片 / 音频批量处理。
- 正式报告对未付费和已付费用户都按记录单独购买，HSLIC1 不得直接授权 `report_export`。
- 云同步必须继续由服务端 entitlement 开放。
- 云能力、团队能力和 API 权限不得由离线许可证开放。
- 单份报告购买授权仍是记录级 purchase grant，不与 CDKEY 混用。
- 后端不得信任客户端上传的离线许可证状态来开放云能力。
- 视频当前不属于许可证权益；未来视频必须使用独立商品和收费规则。

## 4. 角色与密钥

### 4.1 许可证签发根

推荐算法：

- 签名：Ed25519。
- 摘要：SHA-256。
- 编码：Phase K0 已冻结受限 canonical JSON；字段顺序、UTF-8、无空白和未知字段拒绝规则见 `docs/contracts/offline-license/README.md`。
- 签名域：`HiddenShield-Offline-License-v1`。

密钥：

- `issuerPrivateKey`：只存在内部签发环境。
- `issuerPublicKey`：内置于桌面端和移动端验证器。
- `keyId`：支持公钥轮换，例如 `offline-ed25519-2026-q3`。

私钥管理要求：

- 不进入 Git。
- 不进入桌面端、移动端或安装包。仅允许公开测试向量使用明确标记为 test-only 的固定测试 seed，且不得用于真实签发。
- 不通过普通聊天、邮件或工单发送。
- 首期至少使用加密私钥文件、强口令、操作系统访问控制和离线备份。
- 正式规模扩大后迁移到硬件令牌、HSM 或等价密钥托管。

### 4.2 安装实例身份

不使用 MAC、硬盘序列号或完整硬件指纹作为主绑定值。

首次运行时生成：

- 256 bit 随机 `installationSecret`。
- 随机 `installationIdSalt`。
- `installationId = Base64URL-NoPad(SHA-256("HiddenShield-Installation-v1" || secret || salt))`，固定 43 个 ASCII 字符。

存储：

- Windows：DPAPI。
- macOS / iOS：Keychain。
- Android：Android Keystore 包装后的本地秘密。
- 数据库只保存 `installationId` 和状态镜像，不保存明文 secret。

许可证绑定 `installationId`。复制许可证文件到另一安装实例时，绑定检查失败。

## 5. 激活请求

用户先从客户端导出激活请求：

```json
{
  "schemaVersion": 1,
  "requestType": "offline_license_activation_request",
  "requestId": "req_...",
  "installationId": "sha256:...",
  "platform": "windows",
  "appVersion": "0.1.0",
  "requestedProductCode": "creator_offline",
  "createdAt": "2026-07-15T00:00:00Z",
  "nonce": "base64url..."
}
```

交付形态：

- `.hsreq` 文件。
- 可复制请求码。
- 二维码。

激活请求不包含：

- 本地媒体路径。
- 原始作品或保护副本。
- 账户密码。
- `installationSecret`。
- 硬盘序列号、MAC 或完整设备标识。

## 6. 许可证载荷

Phase K0 冻结 v1 签名载荷为：

```json
{"expiresAt":"2027-07-15T00:00:00Z","installationId":"Base64URL-SHA256","issuedAt":"2026-07-15T00:00:00Z","keyId":"offline-ed25519-2026-q3","licenseId":"lic_...","notBefore":"2026-07-15T00:00:00Z","productCode":"creator_offline","schemaVersion":1}
```

约束：

- 字段顺序固定，不允许未知字段或空白格式化。
- 不在签名载荷中放置可自由组合的 `features`；客户端必须用 `productCode=creator_offline` 映射冻结的 feature allowlist。
- `planCode`、`planName`、`entitlementSource`、转移策略和应用版本策略属于验证后的本地权益快照或后续 schema，不得由 v1 token 任意声明。
- 正式 schema：`docs/contracts/offline-license/license-payload-v1.schema.json`。
- 主载体 schema：`docs/contracts/offline-license/hslic1-token-v1.schema.json`。

签名：

```text
signature = Ed25519.sign(
  issuerPrivateKey,
  "HiddenShield-Offline-License-v1\0" || canonicalPayload
)
```

封装：

```text
HSLIC1.<base64url(canonicalPayload)>.<base64url(signature)>
```

`.hslicense` 文件可以保存同一封装，不增加第二种签名算法。

Phase K0 固定测试向量为 454 字符：

```text
docs/fixtures/offline-license-k0/hslic1-ed25519-v1.json
```

## 7. 签发流程

内部工具建议为独立 Rust CLI，不进入面向用户的安装包。

流程：

1. 操作员导入 `.hsreq`。
2. 工具验证 schema、请求时间、nonce 和 installationId 格式。
3. 操作员选择固定产品模板，不允许任意编辑 feature map。
4. 工具确认许可证期限、应用大版本和备注。
5. 操作员解锁签发私钥。
6. 工具生成 `licenseId`、序列号和 nonce。
7. 工具对 canonical payload 签名。
8. 输出 `.hslicense`、可复制许可证码和签发审计记录。

签发审计至少记录：

- `licenseId`
- `serialNumber`
- `requestId`
- `installationId`
- `productCode`
- `keyId`
- `issuedAt`
- `expiresAt`
- 操作员标识
- payload SHA-256
- 签发结果

审计中不得保存私钥或 `installationSecret`。

## 8. 客户端激活流程

1. 用户打开“离线许可证”页面。
2. 客户端确保安装实例身份已生成并可从安全存储读取。
3. 用户导入 `.hslicense`、粘贴许可证码或扫描二维码。
4. 客户端解码封装并拒绝未知 schema。
5. 按 `keyId` 选择内置公钥。
6. 重新生成 canonical payload 并验证 Ed25519 签名。
7. 验证签名域、产品模板和 feature allowlist。
8. 验证 `installationId` 与本机一致。
9. 验证 `notBefore`、`expiresAt`；v1 不含应用版本门禁，未来只能通过新 schema 增加，不能静默修改 v1。
10. 验证许可证未出现在本机导入的签名撤销列表。
11. 原子写入加密许可证存储和本地审计事件。
12. 重新计算 `EffectiveEntitlementState`。

每次执行受限本地能力时必须重新读取有效许可证状态；不能只依赖 UI 激活时写入的布尔值。

## 9. 权益合并

不直接覆盖现有云端 `entitlement_state`。

建议新增：

- `offline_license_state`
- `offline_license_audit`
- `effective_entitlement_snapshot`

合并规则：

```text
本地 feature = cloud entitlement OR valid offline license
云端 feature = cloud entitlement only
```

示例：

- `batch_processing`：云 Creator 或有效离线 Creator 均可开放。
- `report_export`：云 Creator、有效离线 Creator或有效单份 purchase grant。
- `cloud_sync`：只接受云 entitlement。
- `cloud_video_processing`：只接受服务端 entitlement 和 quota。
- `team_workspace`：只接受服务端 workspace membership。
- `api_access`：只接受服务端 Enterprise API key / entitlement。

建议 `billingSource`：

- `offline_cdkey`
- `cloud_subscription`
- `single_purchase`
- `mixed`

## 10. 到期、续期与永久授权

推荐首期使用最长 365 天的离线许可证，由内部工具手工续签。

原因：

- 完全离线环境无法实时撤销已泄露许可证。
- 有限期限可以限制私钥泄露、误签发和许可证转卖的长期影响。
- 续签不要求真实支付，只要求重新导出请求或确认原 installationId。

如果提供永久许可证，必须明确：

- 离线撤销无法即时生效。
- 许可证泄露后只能通过新版本移除旧公钥、签名撤销列表或人工升级处理。
- 不能承诺阻止管理员级攻击者回滚本机状态。

## 11. 撤销与转移

### 11.1 撤销

无服务端时采用签名撤销列表：

```json
{
  "schemaVersion": 1,
  "listType": "offline_license_revocations",
  "keyId": "offline-ed25519-2026-q3",
  "generatedAt": "...",
  "sequence": 12,
  "revokedLicenseIds": ["lic_..."]
}
```

撤销列表由签发私钥签名，通过应用升级或用户手工导入。

限制：

- 未导入新撤销列表的完全离线设备不会即时获知撤销。
- 因此撤销列表不能替代许可证有效期。

### 11.2 转移

标准流程：

1. 旧设备生成停用请求。
2. 内部工具把旧 `licenseId` 加入后续撤销列表。
3. 新设备生成新的 `.hsreq`。
4. 内部工具签发新许可证并记录 `replacesLicenseId`。

旧设备丢失时只能走人工审核重签，必须记录操作员和原因。

## 12. 防破解措施

必须实施：

- 非对称签名，私钥不进入客户端。
- 设备安装实例绑定。
- 固定产品模板和 feature allowlist。
- 所有受限操作统一经过权益决策器。
- 数据库、内存和 UI 不作为唯一授权依据。
- 许可证原文和解析状态使用原子写入。
- 验证器拒绝未知 schema、未知 keyId、错误签名域和重复字段。
- 签名验证使用成熟密码库，不自行实现 Ed25519。
- 许可证导入、替换、拒绝和清除均写本地审计。
- 测试覆盖载荷修改、签名替换、设备复制、过期、版本越界和公钥轮换。

可选加固：

- 二进制混淆和符号裁剪。
- 多处调用中央权益判断，降低只改一个 UI 分支即可绕过的风险。
- 安全存储与数据库状态交叉校验。
- 记录本机观察到的最大可信时间，发现明显时钟回拨时进入人工确认。
- 对签发工具实施双人审批或硬件令牌。

明确限制：

- 拥有管理员权限并能修改客户端二进制的攻击者，理论上可以移除本地授权检查。
- 完全离线系统无法可靠阻止系统时钟和整个磁盘快照回滚。
- 本方案提高伪造与分享成本，不等于 DRM 不可破解。

## 13. 稳定错误码

建议冻结：

- `offline_license_invalid_format`
- `offline_license_unknown_schema`
- `offline_license_unknown_key`
- `offline_license_signature_invalid`
- `offline_license_device_mismatch`
- `offline_license_not_yet_valid`
- `offline_license_expired`
- `offline_license_app_version_unsupported`
- `offline_license_feature_profile_invalid`
- `offline_license_revoked`
- `offline_license_secure_storage_unavailable`
- `offline_license_clock_rollback`
- `offline_license_artifact_from_future`
- `offline_license_key_disabled`
- `offline_license_key_inactive`
- `offline_license_key_purpose_invalid`
- `offline_license_trust_policy_invalid`
- `offline_license_revocation_replay`
- `offline_license_revocation_equivocation`
- `offline_license_request_invalid_format`
- `offline_license_request_unknown_schema`
- `offline_license_request_non_canonical_payload`
- `offline_license_request_checksum_mismatch`
- `offline_license_request_product_invalid`
- `offline_license_revocation_invalid_format`
- `offline_license_revocation_unknown_schema`
- `offline_license_revocation_non_canonical_payload`
- `offline_license_revocation_signature_invalid`
- `offline_license_revocation_list_invalid`
- `offline_license_revocation_sequence_invalid`
- `offline_license_issuer_password_missing`
- `offline_license_issuer_password_too_short`
- `offline_license_issuer_wrong_password_or_corrupt_key`
- `offline_license_issuer_unknown_option`
- `offline_license_issuer_output_exists`
- `offline_license_issuer_output_conflict`

用户提示不得显示私钥、签名原文、完整 installation secret 或内部策略细节。

## 14. 分阶段实施

### Phase K0：合同与测试向量

状态：`已完成`

- 冻结 request、license、revocation schema v1。
- 冻结 canonical encoding、签名域和错误码。
- 生成 Rust、TypeScript、Dart 共享验证向量。
- 固定 `creator_offline` feature allowlist。

2026-07-15 已完成：

- 主许可证载体冻结为 `HSLIC1.<payload>.<signature>`。
- token 长度合同冻结为 `300–500` 个 ASCII 字符。
- license payload v1 冻结为 8 个固定字段，不允许未知字段。
- 签名消息冻结为 `UTF8("HiddenShield-Offline-License-v1") || 0x00 || canonicalPayloadBytes`。
- 固定 Ed25519 测试向量长度为 454 字符。
- TypeScript、Rust、Dart 对同一 fixture 得到一致字段结果并通过签名验证。
- 三端均确认合法编码的 payload 字段修改会导致签名验证失败。

2026-07-15 后续完成：

- 激活请求冻结为 `HSREQ1.<payload>.<checksum>`，checksum 为带域 SHA-256 的前 96 bit。
- 签名撤销列表冻结为 `HSRVL1.<payload>.<signature>`。
- Rust、TypeScript、Dart 已共同通过 16 条固定错误向量。
- 三端错误优先级统一为格式、canonical、schema/产品/列表规则、checksum/签名。

验收：

- 任意字段修改都会导致签名失败。
- 三端对同一 fixture 得到一致验证结果。

### Phase K1：内部本地签发 CLI

状态：`已完成（内部最小集）`

- 新增独立 Rust CLI。
- 支持生成签发密钥、导入请求、签发、检查和撤销列表。
- 私钥加密保存，不进入仓库。
- 输出签发审计 JSON。

2026-07-15 已完成：

- 新增内部 Rust 二进制 `offline_license_issuer`。
- 支持 `keygen`、`inspect-request`、`issue`、`verify-license`、`sign-revocations`、`verify-revocations`。
- 私钥 seed 使用显式 Argon2id v19 `m=19456,t=2,p=1` 派生密钥和 XChaCha20-Poly1305 加密。
- 密钥文件通过 AEAD AAD 绑定 `keyId` 与 Ed25519 公钥。
- 密码只从环境变量读取，至少 16 字符，并在进程内使用自动清零容器。
- CLI 不接受 feature map 或 Studio / Enterprise 模板，只能签发 `creator_offline`。
- 许可证与撤销列表均输出独立审计 JSON 和最终 token SHA-256。
- 签发与撤销命令强制 `operatorId`；许可证审计包含独立 `serialNumber`、canonical payload SHA-256、签发结果，并可记录 `replacesLicenseId + reason` 转移元数据。
- 可重复运行态 QA 已覆盖正确签发、双向验签、错误密码、未知模板和非法请求。

验收：

- CLI 不能签发云能力。
- 错误密码、未知模板和非法请求均拒绝。

当前限制：

- Windows 生产签发目录仍需由运营侧配置专用账户和 NTFS ACL。
- 当前没有双人审批、硬件令牌、HSM、正式密钥备份或灾难恢复流程。
- 签发 CLI 尚未接入用户端激活、安全存储和权益合并。

### Phase K2：桌面离线激活

状态：`已完成（内部最小集）`

- Tauri 生成 installation identity 和 `.hsreq`。
- 导入 `.hslicense` 并验证签名、绑定、期限和版本。
- 新增本地许可证状态页与清除入口。
- 由中央权益合并器开放本地批量和正式报告。

验收：

- 复制许可证到另一安装实例必须失败。
- 修改数据库 feature map 不得获得有效授权。
- 云同步和云视频仍保持关闭。

2026-07-15 已完成：

- migration 19 已落地 installation identity、签名许可证、撤销列表和追加式审计。
- Windows / macOS / Linux 使用 OS keyring 保存 256 bit installation secret，SQLite 只保存盐、派生 ID 和 secret fingerprint。
- Tauri 已注册请求导出、许可证导入、状态读取、清除和撤销列表导入命令。
- 桌面设置页已提供 `.hsreq` 导出、`.hslicense` 文件/长字符串导入、撤销列表导入和清除入口。
- 中央权益解析器在每次本地批量与正式报告执行前重新验证签名、绑定、期限、撤销和安全存储。
- 生产运行态不再注册可直接修改 entitlement 的 `set_entitlement_state`。
- 数据库 feature map 篡改、复制许可证、过期和撤销测试均 fail closed。

### Phase K3：移动端只读与激活

状态：`已完成（代码与共享合同）；Android / iOS 真机发布 QA 待完成`

- Flutter 复用相同 schema、错误码和测试向量。
- Android / iOS 使用平台安全存储。
- 二维码和文件导入保持同一许可证载荷。

验收：

- 桌面签发的许可证可被移动验证器读取。
- 是否允许一份许可证激活移动端由 seat policy 明确决定，不能默认共享。

2026-07-15 已完成：

- Flutter 已接入同一 installation identity、`HSREQ1`、`HSLIC1`、`HSRVL1` 和错误码合同。
- Android Keystore / iOS Keychain 通过 `flutter_secure_storage` 保存 installation secret、salt、许可证、撤销列表和可信时间高水位。
- 设置页支持文件、粘贴和二维码 token 输入；二维码不创建移动端私有载荷。
- seat policy 冻结为单安装实例：桌面许可证可解析，但在移动安装实例导入时返回设备不匹配，移动端必须重新导出请求签发。
- 本地授权只合并批量处理与正式报告；全部云能力仍由服务端权威决定。

### Phase K4：轮换、撤销与发布门禁

状态：`内部安全门禁已完成；正式 OS 签名安装包 QA 待完成`

- 内置公钥 ring。
- 签名撤销列表。
- 许可证替换与转移审计。
- 时钟回拨、磁盘复制、降级安装和二进制篡改专项测试。

2026-07-15 已完成：

- 冻结 `trust-policy-v1`：密钥状态为 `active` / `verify_only` / `disabled`，并绑定 license / revocation 用途和有效时间。
- 桌面通过编译期 `HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON` 嵌入生产公钥 ring；移动端通过同名 `--dart-define` 嵌入同一策略。公开 fixture 测试公钥不进入普通生产信任路径。
- migration 20 与桌面 OS keyring 双写最高可信 UTC，移动安全存储保存同一高水位；超过 300 秒的时钟回拨 fail closed。
- 撤销列表按 `keyId` 在 SQLite 与 OS keyring / 移动安全存储保存全部 token 及 sequence + digest 高水位：低序列号拒绝，相同序列号同 digest 幂等，相同序列号不同 digest 视为 equivocation；密钥轮换不会覆盖旧 keyId 的撤销集合，仅回滚桌面数据库会被安全锚点识别。
- 桌面和移动均记录许可证替换审计；v1 不伪造不存在的 `replacesLicenseId` 签名字段。
- 桌面许可证状态、替换审计和导入成功审计在同一 SQLite 事务提交；移动端 Keychain/Keystore 与 SQLite 无法形成跨存储 ACID 事务，正式发布前仍需引入可恢复安全 envelope 或补偿协议。
- 磁盘/数据库复制由 installation secret 与 SQLite identity 交叉校验拒绝。
- 未知 schema 已拒绝，因此旧程序无法把 v2 当 v1；真正的 app-version 最低版本门禁必须在未来 v2 增加。
- 发布完整性权威冻结为 Windows / macOS / Android / iOS 的 OS 包签名链；进程内 self-hash 只能诊断，不能开放权益。
- `license:k4-contract` 与 `license:k4-runtime-qa` 已固化为可重复门禁。

当前外部阻塞：

- 仓库没有正式签发公钥、Windows Authenticode、macOS Developer ID、Android release keystore 或 iOS Distribution 证书；因此正式签名安装包篡改/降级 QA 不能在当前环境伪造为已通过。
- Android / iOS 真机 KeyStore、Keychain、文件分享和相机二维码仍需发布候选包验证。

## 15. 当前推荐决策

首期采用：

- `Creator（离线授权）`
- 单安装实例绑定
- 365 天有效期
- `.hslicense` 文件 + 可复制长许可证码
- Ed25519 签名
- 内部离线 Rust CLI 签发
- 只开放 `batch_processing` 和 `report_export`
- 所有云能力保持服务端权威

不采用传统短 CDKEY，也不把私钥或对称生成秘密放进客户端。
