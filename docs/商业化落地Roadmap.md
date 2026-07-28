# HiddenShield 商业化落地 Roadmap

当前桌面发布状态：`v0.1.3` RC / GA Gate `PASSED`（2026-07-26）；真实支付与公共信任层仍未进入当前发布范围。

## 2026-07-26 中文社交媒体宣传片

状态：`1080p 主片已生成；待发布负责人终审`

- 新增可复现宣传片工程 `docs/promo-video/`，输出中文横屏 `16:9` 主片与 720p 预览，目标受众为摄影师、设计师、音乐创作者、投资人和潜在合作伙伴。
- 宣传结构明确分为三层：桌面端图片 / 音频写入、验证、本地版权库和技术证据报告属于当前能力；云版权库、SDK、API 明示为“未来规划”；个人作品身份明示为“终局愿景”。
- 云版权库段落只承诺未来同步必要版权元数据、登记状态和权利声明，不宣称当前已有生产 PostgreSQL、外部企业 SLA、公开生产 API 或完整原始媒体托管。
- 报告段落明确“提供技术证据，不替代法律权属认定”；演示作品、版权编号和桌面界面均为宣传片视觉素材，不进入正式版权库、报告、同步 payload 或跨端 QA 证据。
- 当前宣传片仅介绍桌面端，与 `v0.1.3` 当前发布范围一致；移动端冻结且不在片中出现，不形成新的移动端能力承诺。
- 配音修订：原 Windows 系统合成女声因机械感不作为推荐发布版本；新增 `zh-CN-XiaoxiaoNeural` 温暖中文神经人声版，旁白改为短句、自然停顿和面向观众的对话式表达，旧版继续保留用于对照。
- 验证：推荐拟人配音成片为 H.264/AAC、`1920×1080`、30fps、48kHz，时长约 213.67 秒，平均音量约 `-19.1 dB`、峰值约 `-1.0 dB`；逐镜头 MP3、分镜图、时长清单和重新构建脚本已保留。原系统配音版时长约 171.57 秒。
- 风险：当前推荐版使用联网生成的中文神经人声，不是 OpenAI ChatGPT 原声；发布前仍需人工完整观看并确认音色、停顿、中文专有名词读法、社交平台压缩后的文字可读性，以及片尾“加入粉丝群 / 领取下载链接”是否与实际运营承接一致。
- 下一商业化任务：由发布负责人完整观看 1080p 主片并签字确认，随后基于同一工程剪出 60 秒创作者版和 30 秒投放版，不新增超出当前能力边界的文案。

## 2026-07-25 对外产品承诺文档

状态：`正式客户版已生成；批准对外`

- 发布负责人已确认生产 HSLIC1 trust policy 客户端导入与干净 Windows 安装验证通过。
- 桌面已生成 `HiddenShield_产品能力与版权编号验证说明_v0.1.3.docx`，统一图片 / 音频支持规格、图片独立变换恢复范围、音频后处理排除项和 `HS-...` 版权编号语义。
- 文档未扩展现有能力：图片仅列已验证的独立变换；音频不承诺经过重新编码、重采样、剪辑或信号编辑后的稳定读取；版权编号不包装为法律确权或第三方登记。
- 风险：客户版只覆盖桌面图片 / 音频，不覆盖图片组合扰动、音频后处理稳定恢复、视频、Web 预览正式水印或移动端。
- 下一任务：把客户版文档纳入发布附件和销售文案审查清单，后续任何能力扩展必须同步更新能力边界和客户版。

## 2026-07-24 授权签发台内部原型

状态：`内部原型已实现；未进入对外发布`

- 签发流程已收敛为“选择 `.hsreq` → 签发授权”：客户参考号自动生成 `YYYYMMDD-00001` 形式的 UTC 日序号，操作员固定为 `ops-jihx`，到期时间固定为签发时刻起 364 天。
- 正式 key 和 DPAPI 口令恢复文件仅从当前 Windows 用户的服务方目录读取；交付内容自动写入 `Documents/HiddenShield-License-Delivery/<客户参考号>/`，其中 `.hslicense` 用于交付，审计 JSON 留存服务方。
- 经授权将正式加密软件 key 的口令轮换为用户指定的 9 字符值，DPAPI 恢复文件同步更新；旧 key 和旧 DPAPI 文件保留受 ACL 保护的同目录备份以便回滚。
- 安全降级：软件 key 的最低口令合同由 16 字符降为 8 字符，当前 9 字符口令的离线暴力破解风险高于原候选基线；该运营便利性调整不改变 Ed25519 签名、客户端公钥验签或设备绑定，但不能复用“至少 16 字符口令”的历史 Gate 结论。
- 新增独立 Tauri 入口“HiddenShield 授权签发台”，仅面向服务方受控电脑；它不进入客户版安装包，也不替代客户版的 HSREQ1 导出/HSLIC1 导入流程。
- 原型只提供导入并校验 `.hsreq`、自动签发 `.hslicense`、复制 HSLIC1 长码及保存审计文件。
- 签发台通过现有 `offline_license_issuer` CLI 执行签名，未在前端或第二套业务逻辑中复制 Ed25519、请求解析或许可证编码算法。
- 私钥路径和口令仅用于当前签名进程；原型不持久化口令、不写入口令审计，也不把密钥纳入客户安装包。当前仅支持仓库外受控目录的加密软件密钥。
- 验证：`npm run build`、`vite build --config vite.issuer.config.ts`、`cargo check --bin license_issuer_app`、`cargo fmt --check` 和 issuer 4 项单元测试均通过；已用脱敏 HSREQ1 调用签发引擎的 `inspect-request` 并返回 `status=valid`。正式 key 已完成重加密，并通过脱敏 HSREQ1 的 HSLIC1 签发和生产公钥验签。
- 修复：签发台曾传递 RFC3339 `+00:00` 到期时间，而现有 HSLIC1 canonical 格式只接受 `Z` 后缀 UTC 时间，导致 `offline_license_invalid_format`。已改用 `SecondsFormat::Secs` + `Z`；同一真实 HSREQ1 直接签发复现从失败变为通过，失败产生的四个空交付目录和当天序号已安全回收。
- 修复：`20260724-final-clean-candidate-080921` 的本地候选构建未注入 `HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON`，已安装 EXE 因而返回 `offline_license_unknown_key`，不能导入生产 HSLIC1。候选编排现会在 Tauri build 前加载并导出生产 trust policy；客户端同时会明确提示需要更新安装包。
- 验收：使用真实本地 HSREQ1 经授权签发台生成 HSLIC1 后，在已签名 NSIS 候选的实际安装载荷中完成离线导入和激活验证；许可证由生产公钥复验通过。
- 安装限制：`20260724-license-trust-fix-104500` 的 NSIS / MSI 外层签名均为 `Valid`，候选隔离目录中的 NSIS 实际安装 EXE 也为 `Valid`；MSI 在非提升会话仍返回 `1603`，且同版本 NSIS 覆盖安装未替换主程序，故主安装目录覆盖更新仍不能作为正式候选通过证据。
- 风险：签发台尚未作为独立签名/安装包交付，且仍依赖本地已构建 CLI；HSM/KMS、双人审批、订单/退款/换机运营闭环仍未实现。

下一商业化任务：

- 在提升权限的干净 Windows 环境复验 MSI 安装、同版本覆盖更新与生产 HSLIC1 导入；随后评估恢复至少 16 字符服务方口令的迁移方案。

## 2026-07-21 桌面 RC 离线生命周期复验

状态：`RC Gate 通过；GA Gate 进行中`

- 当前签名 `0.1.0` 安装包已通过自包含安装、安装后 UI 启动和安装后 EXE 签名验证。
- 使用 production HSLIC1 key 完成 `HSREQ1 -> HSLIC1 -> 重启状态 -> 过期拒绝 -> HSRVL1 撤销 -> 重启状态` 生命周期。
- 激活映射确认 `batch_processing=true`、`report_export=false`；撤销后本地批量权益关闭，报告导出仍关闭。
- issuer 期限协议已固定为最大 `365 × 24` 小时；新增测试覆盖恰好 365 天允许、超出 1 秒拒绝。RC 复验使用 364 天。
- 脱敏证据：`artifacts/desktop-offline-release-gate/20260721/summary.json`。
- 验证结果：issuer 4/4 测试通过，安装版 UI Gate 通过，Authenticode Candidate Gate 通过，HSLIC1 Signer Candidate Gate 通过。
- 风险：自签发布者不具备公共 Windows 信任；干净 Windows GA 证据、正式分发信任和生产运营闭环仍未完成。

下一商业化任务：

- 在未预装证书的普通 Windows 与已部署证书的专用客户环境分别复跑安装警告、发布者信任和启动结果，并继续保留 GA 为进行中。

## 2026-07-17 零成本双 Gate 发布基线

状态：`免费签名路径、长期密钥、自签证书、GitHub secrets 与正式候选 Gate 已完成`

### HSLIC1 Signer Gate

- 年度 HSLIC1 / HSRVL1 继续使用 Ed25519 非对称签名；不退化为机器码加 salt 或客户端内置 HMAC 秘密。
- 服务方私钥改为 `Argon2id + XChaCha20-Poly1305` 口令加密软件文件。正式 issuer 允许 `keygen`、`--key` 和 `--password-env`，审计固定记录 `signerType=software_encrypted_file`。
- 候选 Gate 要求真实加密密钥文件、至少 16 字符的口令环境变量和脱敏 HSREQ1；必须完成一年期 HSLIC1、HSRVL1、错误口令拒绝、公钥复验和脱敏审计。
- 客户端仍只持有公钥并完全离线验签。免费方案降低的是服务方私钥托管强度，不改变客户端 HSLIC1 签名算法。

### Self-Signed Authenticode Gate

- Windows EXE、MSI、NSIS 使用独立的自签 Code Signing 证书，不与 HSLIC1 Ed25519 key 共用。
- Release workflow 通过 GitHub 加密 secrets `WINDOWS_SELF_SIGNED_CERTIFICATE` 与 `WINDOWS_SELF_SIGNED_CERTIFICATE_PASSWORD` 导入 PFX，并在当前用户 Root / TrustedPublisher 中建立本次构建信任。
- Gate 要求三个候选在构建/受控验证环境中为 Authenticode `Valid`，证据 SHA-256 完全匹配，单字节篡改后签名不再为 `Valid`。
- 自签证书不建立公共 Windows 信任。普通用户仍可能看到未知发布者或 SmartScreen 提示；只有预装 HiddenShield 证书的专用客户机器能够建立发布者信任。

### 发布决策

- Google Cloud KMS 与 Azure Artifact Signing 不再是当前 RC/GA 必选项，相关 adapter、合同和脚本仅保留为未来付费安全增强。
- 当前不建设后端在线许可证验证；注册码仍执行 `HSREQ1 -> 服务方离线签发 HSLIC1 -> 桌面本地验签`。
- `npm run license:security-attack-gate:candidate` 继续汇总软件 HSLIC1 Signer Gate、自签 Authenticode Gate、复制数据库 Gate和完整快照回滚已知限制。
- 免费基线明确接受：服务方机器被完全控制时软件私钥可能泄露；公开下载的自签安装包不具备公共 CA 发布者身份；完整系统快照回滚仍是纯离线模型的已知限制。

验证：

- 正式 issuer 已解除加密文件私钥的 `internal-qa` 限制，K1 QA 改为使用无 feature 的生产构建。
- 新增 `scripts/run-software-hslic1-signer-gate.mjs`、`scripts/release/initialize-self-signed-authenticode.ps1` 和 `scripts/release/write-self-signed-authenticode-evidence.ps1`。
- Release workflow 已移除 Azure OIDC 与 Artifact Signing Client Tools，改用自签 PFX secrets、Tauri certificate thumbprint 和独立签名证据 Gate。
- `npm run license:software-signer-contract` 通过，覆盖加密 key、HSLIC1、HSRVL1、错误口令拒绝、软件 signer 审计和外部 signer 兼容。
- 使用生产 issuer 生成的临时非 fixture key 完成候选级软件签发验证，证据 `artifacts/hslic1-signer-gate/20260717191658/hslic1-signer-gate.json`；测试私钥已删除，该证据只证明 Gate 流程，不是正式发布 key 证据。
- 使用临时 `CN=HiddenShield Release Signing` 自签证书对现有 Release EXE/MSI/NSIS 副本完成候选级签名与篡改验证，证据 `artifacts/authenticode-gate/20260717191204/authenticode-gate.json`；测试 PFX 和证书库条目已删除。
- MSI 篡改测试已从固定偏移翻转改为通过 Windows Installer API 修改 `ProductName`，避免误落在不参与 Authenticode 哈希的空闲扇区。
- `npm run build`、`npm run release:desktop-baseline`、`npm run commercial:contract` 和合同模式 `npm run license:security-attack-gate` 均通过。
- 无长期软件 key、PFX 和正式候选路径时，最新候选汇总证据 `artifacts/offline-license-security-gate/20260717192022/offline-license-security-gate.json` 正确调用软件 signer 并保持 `blocked_candidate_evidence`。
- 当前 Windows 用户已作为服务方身份，在仓库外 `%USERPROFILE%\.hiddenshield-service-provider\production-signing\20260717\` 长期保存正式 HSLIC1 key、自签 PFX、DPAPI 口令恢复文件、恢复说明和 SHA-256 清单；目录 ACL 只允许当前用户与 SYSTEM。
- 正式 HSLIC1 key 为 `offline-production-2026-07-17-v1`，公钥 `idGJrKyJC86KSMGA5rCDRNN9ZG2Vj7ii7RSNUdLHK1U`，已写入 `config/offline-license-trust-policy.production.json`，并确认嵌入 Release EXE。
- 正式自签证书 subject 为 `CN=HiddenShield Release Signing`，thumbprint `4F14DA0B5558359183E86F35486A08A34F38EAE5`，有效期至 2029-07-17；PFX 与 HSLIC1 key 使用不同随机口令和不同恢复文件。
- GitHub encrypted secrets `WINDOWS_SELF_SIGNED_CERTIFICATE`、`WINDOWS_SELF_SIGNED_CERTIFICATE_PASSWORD` 已配置；Release workflow 构建前加载生产 trust policy，打包后会重签 Tauri 恢复为未签名状态的最终独立 EXE。
- Tauri Release 已使用 RFC3161 `http://timestamp.digicert.com` 完成 EXE、MSI、NSIS 及打包依赖签名；最终三件套 Authenticode 状态均为 `Valid`。
- 正式候选汇总 `npm run license:security-attack-gate:candidate` 已通过，证据 `artifacts/offline-license-security-gate/20260717194912/offline-license-security-gate.json`：Authenticode、软件 HSLIC1 signer、数据库复制均通过，完整快照回滚保持 `known_limitation_reproduced`。

下一商业化任务：

- 将服务方签名目录复制到一块离线加密备份介质，并在另一台未预装 HiddenShield 根证书的 Windows 环境验证安装警告口径，再在专用客户环境导入证书后验证发布者信任与安装启动。

## 2026-07-17 托管 KMS 双 Gate 发布基线（未来付费增强）

状态：`托管签名架构与合同适配已完成；真实云资源、身份和正式候选证据待补`

### Authenticode Gate

- **代码签名私钥**只用于 Windows EXE、MSI、NSIS 的 Authenticode 签名，不签发 HSLIC1 / HSRVL1。
- 正式基线固定使用 Azure Artifact Signing。HiddenShield 构建机只使用 Azure 身份、Artifact Signing account、certificate profile 和 SignTool dlib 请求摘要签名，不保存 PFX 或本地代码签名私钥。
- `npm run release:authenticode-gate:candidate` 要求 `HIDDENSHIELD_SIGNED_EXE_PATH`、`HIDDENSHIELD_SIGNED_MSI_PATH`、`HIDDENSHIELD_SIGNED_NSIS_PATH` 指向同一候选构建；三个文件原始状态必须为 `Valid`，单字节篡改后必须不再为 `Valid`。
- `scripts/release/sign-with-azure-artifact-signing.ps1` 按 SignTool `dlib + metadata.json` 合同签名并生成脱敏证据；候选 Gate 强制要求 `provider=azure_artifact_signing` 且证据 SHA-256 与 EXE/MSI/NSIS 完全一致。原 PFX/KSP 脚本只保留内部 QA 和迁移兼容。

### HSLIC1 Signer Gate

- **许可证签发私钥**只用于年度 HSLIC1 和 HSRVL1，算法固定为 Ed25519，不签署 EXE、MSI、NSIS。
- 正式基线固定使用 Google Cloud KMS `EC_SIGN_ED25519`。issuer 只提交冻结的原始 signing message，通过 `asymmetricSign.data` 获取签名，并使用配置公钥再次复验。
- 允许 Google Cloud KMS `SOFTWARE`、`HSM` 或 `HSM_SINGLE_TENANT` protection level；不再要求 HiddenShield 自购或本地连接 HSM/USB Key。签发协调器不得读取、导出或持久化私钥材料。
- `scripts/signers/hslic1-google-cloud-kms-signer.mjs` 使用 Application Default Credentials，校验 key resource、算法、protection level、Ed25519 公钥、请求 CRC32C、响应 CRC32C 和实际 key version。
- `npm run license:hslic1-signer-gate:candidate` 要求真实 `managed_kms` signer config 与脱敏 HSREQ1；fixture/mock、本地测试 endpoint、文件私钥或非 Google KMS adapter 一律拒绝。原 PKCS#11 adapter 仅保留兼容和迁移用途。

### 汇总攻击 Gate

- `npm run license:security-attack-gate:candidate` 现在汇总独立的 Authenticode Gate、HSLIC1 Signer Gate、复制数据库 Gate 和完整快照回滚已知限制，不再把两套私钥统称为“生产签名私钥”。
- 当前机器尚未配置 Google Cloud KMS ADC、真实 CryptoKeyVersion、Azure Artifact Signing account/profile 或 Artifact Signing Client dlib，因此不得用测试 endpoint 代替真实云服务，也不得声明候选 Gate 通过。
- 2026-07-17 验证结果：`license:hslic1-signer-gate` 合同模式通过，`release:authenticode-gate` 正确记录正式候选缺失，KSP thumbprint 注入合同通过；候选汇总证据 `artifacts/offline-license-security-gate/20260717155606/offline-license-security-gate.json` 为 `blocked_candidate_evidence`。
- 独立阻塞证据：`artifacts/authenticode-gate/20260717155610/authenticode-gate.json` 缺 EXE/MSI/NSIS，`artifacts/hslic1-signer-gate/20260717155610/hslic1-signer-gate.json` 缺真实 signer config 与 HSREQ1；数据库复制仍为 `passed`，完整快照仍为 `known_limitation_reproduced`。
- 服务方配置模板 `config/hslic1-signer.production.example.json` 只包含 Google KMS resource、公钥、key handle 和非秘密参数；Azure 模板为 `config/azure-artifact-signing.example.json`，两者均不保存凭据或私钥。
- 2026-07-17 服务方机器复探测仍只发现 TPM/AMD PSP、Microsoft 系统 KSP 和内部 QA 自签代码签名证书；未发现 HSM/USB Key、智能卡读卡器、厂商 PKCS#11 DLL、正式 CA 代码签名证书或签名环境变量。探测证据：`artifacts/signing-provider-probe/20260717-service-provider.json`。
- 已生成不含真实客户设备指纹的协议有效 HSREQ1：`artifacts/service-provider-signing-onboarding/20260717/release-gate-sanitized.hsreq`，SHA-256 `5B35E7599DA442C9F68F8B4A74D3D66C9C7C3C98BE7E34403261E4313F6F27A1`，并由生产构建的 `offline_license_issuer inspect-request` 验证为 `valid`。
- 使用该 HSREQ1 复跑候选汇总后，HSLIC1 Signer Gate 已收敛为仅缺 `signerConfig`；最新证据 `artifacts/offline-license-security-gate/20260717160231/offline-license-security-gate.json`、`artifacts/hslic1-signer-gate/20260717160234/hslic1-signer-gate.json`。Authenticode Gate 仍缺正式 EXE/MSI/NSIS，证据 `artifacts/authenticode-gate/20260717160234/authenticode-gate.json`。
- 2026-07-17 `npm run license:managed-signing-contract` 已通过，证明 Google KMS Ed25519 原文签名、公钥/算法/protection level/CRC32C 校验、Azure Artifact Signing metadata/SignTool 命令和两套 fail-closed 合同成立。
- Release workflow 已移除 `WINDOWS_CERTIFICATE` / PFX 路径，改用 GitHub OIDC `azure/login@v2`、Artifact Signing Client Tools 和 Tauri `bundle.windows.signCommand` 在打包阶段逐文件签名；打包后独立运行 `release:authenticode-gate:candidate` 并上传签名证据。
- 最终合同验证通过：`license:managed-signing-contract`、`license:hardware-signer-contract` 兼容合同、`commercial:contract`、`release:desktop-baseline`、Rust example check、前端生产构建和 `npm audit`（0 vulnerability）。
- 最新候选汇总证据 `artifacts/offline-license-security-gate/20260717170133/offline-license-security-gate.json` 正确冻结 Azure Artifact Signing 与 Google Cloud KMS 两套决策；数据库复制为 `passed`，完整快照为 `known_limitation_reproduced`。
- 最新独立阻塞证据：`artifacts/authenticode-gate/20260717170136/authenticode-gate.json` 缺真实 Azure 签名 EXE/MSI/NSIS；`artifacts/hslic1-signer-gate/20260717170136/hslic1-signer-gate.json` 已使用脱敏 HSREQ1，当前仅缺真实 Google KMS `signerConfig`。
- 2026-07-17 已在服务方机器安装并验证用户级 Google Cloud CLI `576.0.0` 与 Azure CLI `2.88.0`；Google CLI 账号登录成功，但可见的 3 个 GCP project 均未启用 Billing，账号也看不到可用 Billing Account，因此无法启用 Cloud KMS 或创建 `EC_SIGN_ED25519` CryptoKeyVersion。
- 2026-07-17 Azure 设备登录成功，但账号返回 `No subscriptions found`，因此无法创建 Artifact Signing account、identity validation、certificate profile、OIDC RBAC 或有效的 `AZURE_SUBSCRIPTION_ID`。
- GitHub 仓库当前未配置 Azure OIDC secrets；包含 Azure Artifact Signing 的 Release workflow 仍只存在于本地未提交改动，不能把远端旧 workflow 的运行结果作为本轮托管签名证据。
- 2026-07-17 再次执行 `npm run license:security-attack-gate:candidate`，证据 `artifacts/offline-license-security-gate/20260717182136/offline-license-security-gate.json` 为 `blocked_candidate_evidence`：数据库复制通过、完整快照限制复现，Authenticode 与 HSLIC1 Signer 因真实云材料缺失继续阻塞。
- 下一商业化任务：先为选定 GCP project 绑定有效 Billing Account，并为 Azure tenant 开通有效 Subscription；随后完成 GCP ADC、Cloud KMS key/IAM、Azure Artifact Signing identity validation/account/profile、GitHub OIDC secrets 和本地 workflow 变更的受控提交，再运行 Release workflow 与候选攻击 Gate。

## 2026-07-17 生产离线许可证安全基线（已被零成本基线取代）

状态：`架构已迁移为托管 KMS；真实云签名候选证据待补`

- 最终决定不建设后端在线许可证验证。`feedback-backend` 不新增许可证在线激活、周期租约或启动强制联网检查，图片 / 音频年度授权继续以 `HSREQ1 -> HSLIC1 -> 本地验签` 为唯一发布路径。
- 生产许可证签名必须由 Google Cloud KMS 隔离执行，签发器只持有 resource name、公钥和最小权限身份，通过托管签名 API 提交 canonical signing message；该 Ed25519 key 与 Azure Artifact Signing 代码签名证书完全独立。
- 当前 Argon2id + XChaCha20-Poly1305 加密私钥文件路径正式降级为 `internal-qa`，不能作为生产密钥托管方案；正式构建不启用 `internal-qa` 时拒绝 `keygen` 和 `--key / --password-env` 文件签名路径。
- 新增 `npm run license:security-attack-gate`，固定验证数据库复制被 Installation Secret 交叉校验拒绝，并复现完整快照回滚在无外部单调锚点条件下无法可靠识别。
- 新增 `npm run license:security-attack-gate:candidate`，要求提供正式 Authenticode EXE、MSI、NSIS；原始文件必须为 `Valid`，单字节篡改后必须不再为 `Valid`。
- 完整快照回滚是纯离线产品的已知限制：同时回滚 SQLite、操作系统安全存储、系统时间和应用数据时，当前客户端不能可靠判断回滚已经发生。本 Gate 的通过含义是“限制已复现、已披露、未虚假承诺”，不是“攻击已被阻止”。
- 2026-07-17 合同验证通过：生产无 `internal-qa` 构建可完成隔离签名许可证与撤销列表，拒绝 `keygen`、文件私钥和含私钥字段的 signer 配置；K0、K1、K4、商业合同、双端合同和桌面基线均通过。
- 最新合同 Gate 证据：`artifacts/offline-license-security-gate/20260717152623/offline-license-security-gate.json`。数据库复制为 `passed`，完整快照为 `known_limitation_reproduced`，二进制篡改因缺少正式签名 EXE / MSI / NSIS 为 `blocked_formal_signed_candidate_required`；候选强制模式阻塞证据为 `artifacts/offline-license-security-gate/20260717152627/offline-license-security-gate.json`。
- GA 仍需补齐：真实 Google KMS key/version 与 IAM 审计、真实 Azure Artifact Signing account/profile、正式 Authenticode 候选，以及三类 Gate 对同一候选哈希的脱敏证据。
- 下一商业化任务：由 release owner 配置两套云签名资源与最小权限身份，在不启用 `internal-qa` 的签发器上签发年度 HSLIC1，并执行 `license:security-attack-gate:candidate`。

## 2026-07-17 正式签名候选与干净离线 GA Gate 历史阻塞记录

状态：`dual:contract 已恢复；正式签名候选与干净离线 Windows GA 证据阻塞`

- `scripts/verify-dual-consistency-contract.mjs` 已移除对已删除桌面 `EnterpriseAuditView.vue` 的读取依赖，改为断言 Enterprise 产品页面与桌面入口必须保持删除；`npm run dual:contract`、`npm run release:desktop-baseline`、`npm run commercial:contract` 均通过。
- 本节记录迁移到托管签名前的历史状态。旧 PFX 注入脚本当时要求私钥、Code Signing EKU 和有效期，并拒绝内部 QA 或自签名证书；该路径现已由 Azure Artifact Signing 基线取代。
- 当时本机构建环境只有内部 QA 自签代码签名证书，未生成正式签名候选；该阻塞现在改由 Azure Artifact Signing account/profile、OIDC 身份和 Client Tools 是否就绪来判断。
- 本机未安装 Windows Sandbox、Hyper-V PowerShell、VMware、VirtualBox 或 Docker 命令行环境；当前会话也不能安全卸载宿主机 WebView2 或禁用物理网卡。因此“WebView2 完全缺失 + 物理断网 + 干净 Windows”仍不能记为通过。
- 风险：不得把现有未签名安装包、内部 QA 自签名包或“内嵌 WebView2 offlineInstaller”的静态证据包装为正式 GA 候选或干净系统通过证据。
- 当前后续任务以文档顶部“托管 KMS 双 Gate 发布基线”为准：配置 Azure Artifact Signing 与 Google Cloud KMS 后，再执行正式签名构建、Authenticode `Valid` 校验和物理断网安装启动 Gate。

## 2026-07-17 安装包自包含启动 Gate

状态：`RC 自包含启动证据通过；GA 仍待正式签名与干净离线 Windows 环境`

- 新增 `npm run release:desktop-installer-self-contained`，固定执行前端生产构建、Tauri 正式打包、端口 `1420` 关闭、NSIS 静默安装、安装内容检查和 WebView2 CDP 页面检查。
- `npm run tauri:build` 固定只构建 `hidden_shield` 产品主程序；内部水印 QA、报告 QA、离线注册码签发工具和桌面离线 Gate 均迁为 Cargo examples，不再进入 MSI / NSIS。
- Windows 安装包将 WebView2 安装模式冻结为 `offlineInstaller`。MSI 与 NSIS 均内嵌离线 WebView2 安装载荷，不要求客户机器具备 Node.js、Vite、Rust 或联网下载 WebView2。
- 2026-07-17 最新候选产物：
  - MSI：`HiddenShield_0.1.0_x64_en-US.msi`，234,897,408 bytes，SHA-256 `a754382d0e2358acda5e64b14b5780922f1496b6cae5dbf62a35df3722efcb55`。
  - NSIS：`HiddenShield_0.1.0_x64-setup.exe`，227,814,675 bytes，SHA-256 `4092beba79fbe6acaa3698e2eca7ae208b0dbfd56511e23572b82f0d87e2c7cb`。
- NSIS 已静默安装到隔离 QA 目录，安装内容只包含 `hidden_shield.exe`、`uninstall.exe` 和产品资源；不包含服务方签发工具或内部 QA EXE。
- 启动时 Vite 与端口 `1420` 均未运行；CDP 读取到 `pageUrl=http://tauri.localhost/`、`documentReadyState=complete`、标题 `HiddenShield` 和完整工作台正文，未命中 `localhost:1420`、拒绝连接或 `ERR_CONNECTION_REFUSED`。
- 脱敏证据：`artifacts/desktop-installer-self-contained/20260717071754/desktop-installer-self-contained-gate.json`。

风险与 GA 边界：

- 当前主机已有 WebView2 Runtime `150.0.4078.65`，本轮没有破坏性卸载共享系统运行时，也没有关闭物理网卡；“WebView2 完全缺失 + 物理断网安装”仍需干净 Windows VM 或等价一次性环境补证。
- 当前 MSI、NSIS 和安装后主程序均为 `NotSigned`，只能作为内部 RC 候选，不能作为 GA 公开分发包。

下一商业化任务：使用正式企业代码签名证书重新构建同一候选，并在缺失 WebView2 的干净 Windows 环境中断网安装 NSIS / MSI、启动工作台，再复验图片、音频和年度注册码流程。

## 2026-07-17 处理流程产品化减法

状态：`已完成代码、浏览器验证和自包含安装包构建；待安装版页面级人工验收`

- 普通图片 / 音频选择后不再自动执行完整盲水印深度提取，立即进入可处理状态。
- “这是已有作品的新版”成为按需入口；只有用户开启后才识别上一版版权信息和版本关系。
- 作品声明与授权策略默认展开，确保生成保护副本前可见。
- 底部百分比主进度条替换为读取作品、准备版权信息、生成保护副本、验证保护结果、保存版权记录五个产品步骤。
- 验证页将“Phase R4 · 只读校验”改为“校验维权证据包”，保留与逐份付费证据包对应的完整性校验价值。

验证结果：

- `npm run build` 通过。
- 浏览器上传 3840×1080、529,758 bytes 的无水印 PNG，约 1.06 秒完成文件选择并显示“图片已就绪”，页面没有出现“正在检查版权记录”。

下一商业化任务：重新构建桌面 Release，并在 Tauri 真实文件路径下复验普通图片即时就绪、新版按需识别及五步进度映射。

## 2026-07-17 桌面产品表达与持续收入入口对齐

状态：`已完成代码对齐；待重新打包并执行桌面页面级人工验收`

本轮在既有 RC / GA 发布基线上完成以下收口：

- 工作台不再向用户展示“离线验证 / 发布 Gate”等内部发布术语，改为“离线能力 / 图片音频可用 / 无需联网即可读取与验证”。
- 处理页和验证页只允许图片 / 音频。视频选择继续 fail closed，处理页不再渲染 L1 / L2 / L3 区块，验证页不得调用视频验证。
- 版权库新增显著的“报告与维权服务”商业卡片，固定展示 `版权详细报告 ¥19.9 / 份` 与 `维权证据包 ¥49.9 / 份`；年度注册码仍不包含任何正式报告。
- 版权存证将第三方时间戳、网络授时和本机创建时间明确区分；没有第三方材料时不得使用含义不清的“未记录”，也不得伪造第三方证明。
- 新记录持久化实际成功的 TSA 来源或网络授时来源；历史记录可在允许联网时通过“补充可信时间”重新获取并保存材料。

验证结果：

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过；仅保留既有未使用代码警告。

风险与后续：

- 处理页内部仍保留冻结视频兼容代码，但所有相关产品区块均以不可达条件阻断；正式安装包发布前继续由桌面发布合同和人工页面检查双重确认。
- 下一商业化任务是为两种记录级商品补充可持久化的“已购买 / 已退款撤销”前端状态，而不是依赖最近导出历史推断授权。

## 2026-07-16 当前发布基线（RC Gate / GA Gate，覆盖后文历史阶段口径）

状态：`新商业权益基线已冻结；RC 需按新映射重建复验；GA Gate 待 RC 后继续`

本基线自 2026-07-16 起覆盖后文仍保留的双端、L1 / L2 / L3 和云端视频历史推进记录：

1. 冻结全部移动端新功能开发、体验对齐和发布验收；移动端不再是当前目标，也不再阻塞桌面版本发布。仅允许必要的安全修复、数据兼容和下架维护，任何例外必须先更新 Roadmap。
2. 当前开发和发布范围只包括桌面端与后端云服务。
3. 桌面端发布必须完成离线验证：无网络时可对当前正式支持的图片 / 音频执行读取、验证并给出明确结果与限制。
4. 必须完成服务方注册码生成与桌面端离线验签：服务方持有签发私钥，桌面端只持有公钥；注册码必须覆盖安装绑定、套餐、有效期、签名、撤销和审计，云能力继续保持服务端权威。
5. 屏蔽桌面端全部视频能力入口和用户可见承诺。L1 视频音轨、L2 视频指纹存证和 L3 视频画面候选均不属于当前发布能力、商业权益或发布 Gate；相关底层实现与历史数据只保留为内部兼容资产。

### 当前商业权益矩阵

当前发布只允许以下商业模型，覆盖后文保留的 Free / Creator / Studio / Enterprise 历史设计：

| 维度 | 未付费 | 图片 / 音频年度基础权益 |
| --- | --- | --- |
| 图片 / 音频单文件处理与验证 | 可用 | 可用 |
| 本地版权库 | 可用 | 可用 |
| 图片 / 音频批量处理 | 不可用 | 可用 |
| 正式报告 | 按记录单独购买 | 按记录单独购买 |
| HSLIC1 | 无 | 一年期激活 / 按年续期 |
| 视频 | 当前不可用 | 当前不可用；未来独立收费 |

冻结规则：

- `report_export` 不属于未付费或年度基础权益，不能由 HSLIC1、年度在线订阅或历史 Creator 映射直接授予。
- 报告授权必须是记录级 purchase grant；已付年度基础权益也不能绕过逐份购买。
- `creator_offline` 仅作为 HSLIC1 V1 token 的兼容产品代码保留，用户界面必须显示为“图片 / 音频年费授权”。
- 云同步及未来云能力继续由服务端权威决定，不得由离线许可证开放。
- 视频当前隐藏且不可售；恢复时必须建立独立商品、权益、计费和发布 Gate。

### 桌面端基线对齐

- 桌面导航、设置、帮助中心、法律文案和权益摘要统一使用“未付费 / 图片音频年费 / 年度授权”，不再展示 Free / Creator / Studio / Enterprise 套餐名称。
- Enterprise 内部管理页面已从桌面产品入口和构建依赖中移除；相关后端模型与内部 CLI 只作为服务方内部资产保留，不属于桌面权益。
- 桌面只展示 `batch_processing` 与服务端权威的 `cloud_sync`；团队空间、API、优先队列、云端批量、视频和历史套餐字段不得进入当前权益摘要。
- 即使云端历史快照仍返回旧 `planName` / `entitlementLabel`，桌面也必须归一显示为“未付费”或“图片 / 音频年费”。

### RC Gate

用途：允许当前候选包进入内部试用、受控客户验证和小范围发布评审，不等于公开商业 GA。

必须满足：

- 当前五项发布基线全部生效，移动端和全部视频能力均不阻塞 RC。
- 使用正式安装包候选在 Windows 物理断网状态完成桌面图片 / 音频读取与验证。
- 使用非 fixture 服务方私钥完成 `HSREQ1 -> HSLIC1 -> 桌面导入 -> 重启持久化 -> 到期拒绝 -> HSRVL1 撤销 -> 重启持久化`。
- 安装包、安装后主程序、媒体验证和许可证生命周期必须有可提交的脱敏证据摘要；私钥、密码、完整 token 与运行数据库不得进入仓库。

状态：`历史媒体与许可证生命周期证据保留；新商业映射复验待完成`

验收结果：

- 本次已冻结发布口径，并要求桌面 UI 不再接受视频输入、不再展示视频处理、视频存证或云端视频权益入口。
- 本次不删除 `watermark-core`、后端任务、历史记录字段或内部 QA，以避免不可逆迁移和历史数据损坏。
- 2026-07-16 已用当前代码重新构建 NSIS 候选包，并通过 Tauri `signCommand` 在封装正确时点完成内部 QA Authenticode 签名；安装器 SHA-256 为 `BC20B84F261EEDB3EBD3F539695E8A285AD9297F3D8F7EA773A30D8C83AB7684`，独立安装目录中的 `hidden_shield.exe` 签名为 `Valid`。
- 真实 WLAN 断开期间互联网探测为不可达，安装版主进程启动后持续存活；安装包内同源 runtime gate 对图片 / 音频各执行 internal QA 与默认 V3/39 写读，共 4 条全部通过，`payloadAuthStatus=verified`。
- 新生成非 fixture Ed25519 签发密钥 `offline-internal-2026-07-16-gate`，私钥与 DPAPI 加密密码仅保存在忽略目录；公开 trust policy 已加入对应公钥。
- 新安装实例完成旧映射下的 `HSREQ1 -> HSLIC1 -> 分进程重启读取 -> 过期拒绝 -> HSRVL1 撤销 -> 分进程重启读取`：有效许可证重启后保持 `active`，过期许可证返回 `offline_license_expired`，撤销后保持 `revoked`。
- 可提交证据摘要为 `artifacts/desktop-offline-release-gate/20260716/summary.json`；私钥、密码、HSREQ1、HSLIC1、HSRVL1 与 runtime SQLite 不进入仓库。
- 2026-07-16 商业权益映射调整后，上述安装包不再是当前 RC 候选：旧证据只证明图片 / 音频离线能力和许可证生命周期，不证明“HSLIC1 仅授予批量且不授予报告”的新映射。必须重建安装包，并复验 `batch_processing=true`、`report_export=false` 后才能恢复 RC 通过状态。
- 新映射已在桌面有效权益合并、HSLIC1 本地 feature map 和后端在线套餐映射中统一强制 `report_export=false`；记录级报告购买授权继续由独立 purchase grant 校验。
- 桌面基线对齐已移除 Enterprise 导航与页面、Studio 团队空间展示、旧套餐帮助问答和旧订阅文案，并将历史云端套餐名在桌面统一归一化。
- 自动验证已通过：`cargo test --manifest-path src-tauri/Cargo.toml entitlements::tests --lib`、`cargo test --manifest-path feedback-backend/Cargo.toml fixture_billing_payment_success_updates_entitlement_and_is_idempotent --lib`、`npm run build`、`npm run release:desktop-baseline`、`npm run commercial:contract`、`npm run billing:contract`、`npm run report:contract`。

### GA Gate

用途：公开商业发布、正式收费和对外 SLA。GA 必须在 RC 通过后独立评审，不能把 RC 的内部 QA 证据直接解释为生产完成。

状态：`进行中`

必须满足：

- 使用正式企业分发证书构建并验证安装包与安装后主程序。
- 在干净 Windows VM 复跑安装、页面级图片 / 音频离线验证、HSREQ1 导出、HSLIC1 导入、重启、到期和撤销；没有干净 VM 时，允许先用全新本地 Windows 用户形成补充证据，但该证据不替代 GA 的干净系统证据。
- 生产签发私钥进入 HSM 或等价托管，具备双人操作、公钥轮换、撤销列表发布和审计。
- 订单、退款、设备迁移、客服审批与许可证签发 / 撤销进入同一运营闭环。
- 页面截图、错误分类、耗时、安装日志、签名链和回滚说明形成可审计证据包。

风险：

- 现有历史阶段、契约脚本和移动端代码仍包含旧视频与双端口径，只能作为历史实现证据，不得反向恢复为当前发布承诺。
- 当前仅完成源码、文档和自动合同收口，尚未生成采用新商业映射的签名安装包；旧安装包哈希不得继续标记为当前 RC。
- 本轮 Windows 包使用内部 QA 自签 Authenticode 证书，不是公开 CA 或正式企业分发证书；仍需在干净 Windows VM 使用生产签名材料复跑，才能通过 GA Gate。
- 本轮验证了签发、到期和撤销技术闭环，但订单绑定、退款、客服迁移、HSM / 密钥托管、公钥轮换和撤销列表分发仍未进入生产运营闭环。

下一商业化任务：

- 重建采用新权益映射的桌面安装包；先运行自动合同确认 HSLIC1 只开放图片 / 音频批量且 `report_export=false`，再由人工在 `HiddenShieldReleaseQA` 用户下复验安装、页面级图片 / 音频、HSREQ1 / HSLIC1 / HSRVL1 与重启持久化。

## 0. Roadmap 规则

本文档是 HiddenShield 商业模式落地的执行总线。后续涉及订阅、权益、批量处理、云同步、云端视频、团队空间、支付、商业化页面的任务，都必须先对齐本文档。

与视觉迁移配套的执行总线：

- `docs/双端视觉语言迁移实施总计划.md`
- `docs/用户体系与登录注册体系规划.md`

商业化页面、订阅页、报告购买页、设置页和帮助页在迁移时必须同时满足本文档与视觉迁移总计划的约束。
用户注册、登录、会话、设备、创作者档案、工作区和版权编号登记的正式接入，统一按 `docs/用户体系与登录注册体系规划.md` 实施。

每次完成相关任务后，必须回写：

- 对应阶段状态。
- 已完成内容。
- 验收结果。
- 新发现风险。
- 下一步任务。

状态枚举：

- `未开始`
- `进行中`
- `已完成`
- `暂停`
- `阻塞`

## 1. 商业化原则

### 1.1 产品边界

- 单文件本地图片 / 音频写入与验证是 Free 入口。
- 本地批量处理是 Creator 订阅权益。
- Free 不提供小批量试用。
- 云同步是 Creator 订阅权益；已登录且 `cloud_sync=true` 的 Creator / Studio / Enterprise 用户应默认自动云同步双端版权库，不再要求手动开启。
- 正式报告默认是 Creator 订阅权益；Free 用户可按份购买“单份版权详细报告”或“维权证据包”，购买后只解锁对应记录 / 案件的报告，不改变订阅等级。
- 本版只预留 Studio 团队空间入口、成员权限模型、共享版权库模型和团队审计模型；真实团队成员管理、共享操作和团队报告后置。
- 云端视频盲水印是未来高阶能力，采用订阅 + 额度。
- 云端批量与本地批量分开计费。

### 1.2 隐私边界

默认不上传：

- 原始图片。
- 加水印后的图片。
- 原始音频。
- 加水印后的音频。
- 原始视频。
- 加水印后的视频。
- 本地文件路径。

首期云端只同步：

- 账户。
- 工作区。
- 设备。
- 创作者档案。
- 订阅权益。
- 版权记录元数据。
- 验证记录。
- 审计记录。
- 同步状态摘要。
- 版权编号登记元数据：`watermarkUid`、签发模式、登记收据、原作品摘要、保护副本摘要、父编号和版本次数。
- Creator 自动云同步的版权库内容白名单：版权记录基础字段、验证字段、登记字段、作品声明字段、报告购买授权状态、L2 不可逆指纹存证字段。

### 1.3 计费边界

- 本地批量不按次扣点，作为 Creator 订阅期内权益。
- 云端视频按处理分钟数或合同额度计费。
- 云端任务失败、取消、崩溃、格式不支持不扣额度。
- 成功完成的云端任务才入账。

## 2. 阶段总览

| 阶段 | 名称 | 状态 | 目标 |
| --- | --- | --- | --- |
| Phase 0 | 商业模式与 Roadmap 固化 | 已完成 | 统一商业化执行口径 |
| Phase 1 | 权益模型与后端契约 | 已完成 | 定义订阅、功能、额度和账本 API |
| Phase 2 | 前端订阅与权益页面 | 已完成 | 桌面端和移动端展示一致的套餐、权益和升级入口 |
| Phase 3 | 本地批量订阅门禁 | 已完成 | Free 阻止批量执行，Creator 开放本地批量 |
| Phase 4 | 云同步订阅门禁 | 已完成 | 将云同步绑定到 Creator 权益 |
| Phase 5 | 正式报告订阅门禁 | 已完成 | 将正式报告和批量摘要绑定到 Creator 权益 |
| Phase 6 | Studio 团队能力预留 | 已完成 | 团队空间、席位、共享版权库和审计模型 |
| Phase 7 | 视频云端能力预留 | 已完成 | 云端视频任务、分钟额度、队列和账本模型 |
| Phase 8 | 支付与订阅状态闭环 | 阶段性完成 | 支付、试用、宽限期、过期回收 |
| Phase 9 | 商业化验收与上线 | 阶段性完成 | 端到端验证、文案、合规和指标 |

### 2.1 状态确认口径

本次状态收口以“本阶段目标是否已经由代码、契约脚本或文档固定”为准：

- Phase 0-3 已完成，不再挂未完成项。
- Phase 4 已完成的是“云同步订阅门禁 + 正式 `auth/sessions` 路径下的 Creator 默认自动云同步 + 当前设备暂停 / 恢复自动同步”：Free 不能启用正式云同步，后端同步 API 对 Free push / pull 返回 403；Creator / Studio / Enterprise 在 `auth/sessions`、`me`、refresh 中返回 `syncPolicy=auto_cloud_vault`，用户暂停后保持 `manual_local_only`；双端登录或权益升级后自动 pull / flush / pull；`PATCH /v1/me/sync-preferences` 可把当前设备切换为 `manual_local_only` 或恢复自动同步。真实双设备截图 QA 归入 `docs/用户体系与登录注册体系规划.md` 后续实施。订阅过期、恢复订阅、宽限期属于 Phase 8 支付与订阅状态生命周期。
- Phase 5 已完成的是“Creator 正式报告门禁与双端报告能力”。Studio 团队报告不再作为 Phase 5 未完成项，归入 Studio 团队能力后续上线验收。
- Phase 6 已完成的是“Studio 团队能力预留”：模型、入口、权益门禁和不暴露未完成能力。真实团队成员管理、共享版权库操作和团队报告不属于本阶段上线范围。
- Phase 7 已完成的是“L1 视频音轨水印可用 + L2 视频指纹存证闭环”。L3 端云协同画面盲水印已进入 release candidate 准备：独立 `watermark:l3-video-visual-release-gate` 已强制跑完整 24 个 2K 样本池并过线，后端任务成功态已拆到 trusted worker/admin completion API 并绑定完整自检收据，受控 fixture worker 已能调用 `watermark-core` 完成策略、写入、自检和 completion；真实 worker first-pass 已完成受控上传清单解析、FFmpeg sandbox、registry-reserved UID 与 core payload 绑定、claim / lease / replay protection 和失败归因。但真实用户视频对象读取、真实输出封装、用户可下载产物、桌面 / 移动正式入口、正式报告、版权库、跨端验证和用户可见 SLA 仍未完成。
- Phase 7 当前承诺的视频容器口径收口到 MP4 / MOV / MKV / WebM；AVI / M4V 仅保留为北极星目标，不写成当前承诺。L1 支持多音轨但静音 / 极短 / 低于 30 秒视频明确拒绝，L2 继续作为不可逆画面指纹存证。
- Phase 8 已阶段性完成：支付 provider 抽象层、首期微信支付、订阅 webhook、entitlement 更新链路、payment session、查单补偿和双端入口已由代码、文档和合同脚本固定。Free 单份报告付费已完成后端 fixture 购买会话与授权核销，并已接入双端版权库购买入口与记录级导出核销；后端已完成真实微信一次性商品下单、查单 / webhook 授权和退款撤销授权的可测试核心。真实微信商户联调、真实支付回调验收、退款撤销运行态验收和双端付费 QA 属于生产上线准备。
- Phase 9 已阶段性完成：商业化验收 checklist、双端 QA 记录、法务条款草案、商业指标看板、管理员 token 鉴权和访问审计已落地。法务审阅与生产 token 配置属于正式上线准备。

`Roadmap 回写记录` 保留当时任务推进状态，不作为当前阶段状态判断依据；当前状态以阶段总览和各 Phase 的“状态 / 当前边界 / 下一步任务”为准。

### 2.2 本次状态收口验证

已执行并通过：

- `npm run cloud:ci`
- `npm run usage:contract`
- `npm run report:contract`
- `npm run team:contract`
- `npm run cloud-video:ci`

说明：

- `npm run cloud:contract` 需要已有云端后端监听 `127.0.0.1:43188`，单独执行时如果未启动后端会失败；状态收口以会自动启动临时后端的 `npm run cloud:ci` 为准。
- `npm run cloud-video:ci` 与 `npm run cloud:ci` 都默认使用 `127.0.0.1:43188`，不能并行执行；串行执行已通过。

## 3. Phase 0：商业模式与 Roadmap 固化

状态：已完成

目标：

- 固化商业模式。
- 固化本地批量作为 Creator 订阅权益。
- 固化后续任务必须回写 Roadmap。
- 更新 AGENTS.md。

任务：

- [x] 编写 `docs/商业模式规划.md`。
- [x] 明确 Free / Creator / Studio / Enterprise 分层。
- [x] 明确本地批量不提供 Free 小批量试用。
- [x] 编写 `docs/商业化落地Roadmap.md`。
- [x] 更新 `AGENTS.md`，要求商业化任务按 Roadmap 实施并回写。

验收标准：

- 文档明确当前能力、未来能力、订阅权益和额度边界。
- AGENTS.md 明确 Roadmap 约束。

验收结果：

- 已完成。商业化相关任务必须按 Roadmap 执行并回写。
- 已明确 Free / Creator / Studio / Enterprise 术语。
- 已明确本地批量处理是 Creator 订阅权益，Free 不提供小批量试用。

## 4. Phase 1：权益模型与后端契约

状态：已完成

目标：

建立后端和客户端共享的商业化契约。

后端能力：

- 账户继续使用 / 登录注册合一。
- 权益快照接口。
- 功能开关接口。
- 本地批量权益：`batch_processing`。
- 云同步权益：`cloud_sync`。
- 正式报告权益：`report_export`。
- 云端批量权益：`cloud_batch_processing`。
- 云端视频权益：`cloud_video_processing`。
- 优先队列权益：`priority_queue`。
- 团队权益：`team_workspace`。

数据模型：

- `accounts`
- `workspaces`
- `devices`
- `creator_profiles`
- `entitlements`
- `usage_ledger`
- `quota_balances`
- `subscription_events`
- `watermark_id_registry`
- `watermark_id_reissue_jobs`

任务：

- [x] 定义后端 API 草案。
- [x] 定义 entitlement JSON schema。
- [x] 定义 usage ledger schema。
- [x] 定义 quota ledger schema。
- [x] 更新桌面端和移动端权益类型。
- [x] 增加契约测试。
- [x] 新增版权编号后端签发 / 登记 / 确认 / 重新签发契约：`POST /v1/watermark-ids/reserve`、`confirm`、`reconcile`、`reissue` 已落地，后端持久化 `watermark_id_registry` 和 `watermark_id_reissue_jobs`，并覆盖幂等签发、确认、离线补登记、冲突和重新签发测试。
- [x] 双端版权库 / 存证摘要 / 正式报告 / 云同步 payload 已落地 `watermarkIdIssueMode`、登记状态、登记收据、父编号、`revision`、payload protocol、payload bytes 和 payload auth status。
- [x] 桌面端和移动端图片 / 音频写入流水线接入在线优先 `reserve -> confirm`；后端不可用时继续使用 V2 本地高熵编号并标记 `pending_registration`，云同步发送前执行 `confirm / reconcile` 并回写本地版权库和同步 payload。

验收标准：

- 桌面端和移动端能读取同一份权益快照。
- 权益字段支持 `batch_processing`、`cloud_sync`、`cloud_video_processing`。
- 失败、取消、崩溃不入账。

验收结果：

- 已新增 `docs/商业化契约与权益模型.md`，固化 Free / Creator / Studio / Enterprise、feature map、usage ledger、quota ledger 和 API 草案。
- 后端 `POST /v1/auth/continue` 返回完整商业化 feature map：`cloud_sync`、`batch_processing`、`report_export`、`cloud_batch_processing`、`cloud_video_processing`、`priority_queue`、`team_workspace`、`api_access`。
- 桌面端本地 `EntitlementState` 已补齐 `planCode` 和 `features`，移动端同步档案已补齐同一组 features。
- `scripts/verify-cloud-sync-contract.mjs` 已把完整 feature map 纳入合同测试。

验证：

- `cargo test --manifest-path src-tauri/Cargo.toml db::billing::tests --lib`
- `cargo test --manifest-path feedback-backend/Cargo.toml continue_account_returns_session_and_persists --lib`
- `npm run cloud:ci`

已落地商业化契约：

- `POST /v1/watermark-ids/reserve`：在线签发版权编号。
- `POST /v1/watermark-ids/confirm`：写入成功后确认登记保护副本摘要和验证状态。
- `POST /v1/watermark-ids/reconcile`：离线编号联网后补登记或重新签发。
- `POST /v1/watermark-ids/reissue`：历史重复编号修复任务，返回替换编号和 repair job。

本次回写：

- 后端新增 `watermark_id_registry` 与 `watermark_id_reissue_jobs`，API 响应返回 `registryId`、`watermarkUid`、`watermarkIdIssueMode`、`registryStatus`、`registryReceipt`、`registryProofHash`、`payloadProtocolVersion`、`payloadBytesLength`、`parentWatermarkUid` 和 `revision`。
- 桌面端 SQLite schema 升级到 16，移动端 SQLite schema 升级到 13；双端记录、报告和同步 payload 均保存登记 / payload 字段。
- 双端写入流水线已在线优先接入 `reserve -> confirm`，并在离线写入后的云同步前执行 `confirm / reconcile`，把状态推进到 `server_confirmed` 或 `offline_confirmed`。
- 同 UID 不同作品哈希的同步记录已进入 `pending_registry_reconcile` 登记仲裁，不再静默合并；双端版权库已提供历史重复编号重新签发 / 保护副本修复入口。桌面端保护副本可访问时会调用后端 `reissue` 并用 `watermark-core` 重写 payload，移动端先创建重签任务并等待用户重新选择文件完成修复。

商业价值：

- 版权编号登记成为用户端与后端账号、云同步、正式报告、团队版权库和未来维权服务的连接点。
- Free 仍可本地写入；联网登记、云同步和更完整的报告 / 团队能力可进入 Creator / Studio / Enterprise 的商业闭环。

风险：

- 后端不可用时不能阻断本地图片 / 音频写入主线，必须使用高熵本地编号兜底。
- 离线编号在联网前只能表述为“本地生成，待登记”，不能包装成“后端已登记唯一”。

风险：

- Phase 1 只完成契约和快照字段，不包含支付闭环和正式门禁。
- 本地历史库会在 v9 迁移时补齐默认 Free feature map，正式上线前仍需做升级包回归。

## 5. Phase 2：前端订阅与权益页面

状态：已完成

目标：

让桌面端和移动端都能清楚展示当前权益、套餐差异和升级路径。

桌面端页面：

- 订阅方案页。
- 当前权益状态。
- Free / Creator / Studio / Enterprise 对比。
- 批量处理权益说明。
- 云同步权益说明。
- 视频云端能力显示为“规划中 / 未来能力”，不写成已支持。

移动端页面：

- 设置页权益卡。
- 订阅方案页。
- Creator 权益说明。
- 批量处理权益说明。
- 云同步权益说明。

任务：

- [x] 统一套餐文案。
- [x] 统一权益标签。
- [x] 桌面端订阅页面改版。
- [x] 移动端订阅页面 / 权益卡设计。
- [x] 增加空状态和过期状态。

验收标准：

- 两端术语一致：Free、Creator、Studio、Enterprise。
- 两端都明确：批量处理是 Creator 订阅权益。
- 两端都不暴露技术词作为卖点。

验收结果：

- 桌面端订阅弹窗已改为 `Free / Creator / Studio / Enterprise` 四档对比。
- 移动端设置页新增当前权益卡与订阅方案抽屉。
- 本地批量处理明确为 Creator 订阅权益。
- 云端视频被明确为未来高阶能力。

验证结果：

- `npm run build`
- `flutter analyze`
- `flutter test test/widget_test.dart --name "opens subscription plans from settings"`

风险：

- 全量移动端 widget 测试仍存在既有文案断言不一致，与本次订阅页改版无关。

## 6. Phase 3：本地批量订阅门禁

状态：已完成

目标：

将本地批量处理设计成成熟的 Creator 订阅服务。

Free 行为：

- 可以看到批量入口。
- 点击后展示订阅说明和升级入口。
- 不进入文件选择。
- 不创建批量队列。
- 不提供小批量试用。

Creator 行为：

- 可进入批量文件选择。
- 可创建本地批量队列。
- 可暂停 / 取消。
- 可单项重试 / 全部重试。
- 每个成功文件写入版权库。
- 每个文件保存完成后验证状态。

Studio 行为：

- 继承 Creator。
- 增加更高并发上限。
- 预留团队任务归属。

任务：

- [x] 定义批量任务模型。
- [x] 定义批量任务状态。
- [x] 桌面端批量入口门禁。
- [x] 移动端批量入口门禁。
- [x] 批量队列 UI 设计。
- [x] 批量失败重试机制。
- [x] 批量队列本地持久化。
- [x] 图片批量写入版权库。
- [x] 移动端常见音频格式桥接。
- [x] 音频批量写入版权库。
- [x] 批量 telemetry 和 usage ledger。

验收标准：

- Free 无法执行本地批量。
- Creator 可以执行本地批量。
- 本地批量不扣点。
- 失败项不影响已完成项入库。
- 桌面端和移动端行为一致。

当前进展：

- 已新增 `docs/本地批量订阅门禁与队列设计.md`，定义 BatchJob、BatchItem、任务状态和失败处理策略。
- 桌面端已新增“本地批量”一级入口，Free 在文件选择前被阻断，Creator 可进入队列页。
- 移动端工作台已新增“本地批量”入口，Free 在文件选择前被阻断，Creator 可进入队列页。
- 桌面端和移动端已支持 Creator 创建本地批量队列。
- 队列已支持暂停、继续、取消和失败项重试的状态流转。
- 桌面端和移动端已支持 BatchJob / BatchItem 本地持久化，重启后可恢复最近队列。
- 已完成图片批量的第一条真实纵切：桌面端和移动端都能顺序执行图片写入，完成后立即验证，并把成功项写入版权库。
- 移动端音频入口已与桌面端保持同格式能力，支持 MP3 / AAC / FLAC / OGG / M4A 等常见音频输入，并保持源音频声道布局。
- 已接入音频批量真实执行链路：桌面端和移动端都能顺序执行音频写入，完成后立即验证，并把成功项写入版权库。
- 桌面端和移动端处理统计口径已统一，新增 `usage:contract` 防止两端 usage ledger 字段分叉。

风险：

- 本地批量 usage ledger 当前只作为本地观测和商业分析，不用于扣减本地批量权益。
- 音频批量复用单文件 30 秒保护边界，短音频会失败并留在队列中。
- 后续若增加并发执行器，必须继续复用单文件图片 / 音频写入后的验证能力，避免批量路径绕开稳定性门槛。

## 7. Phase 4：云同步订阅门禁

状态：已完成

目标：

将正式云同步绑定到 Creator 权益，并在正式用户体系阶段升级为“已登录 Creator 自动云同步双端版权库”。

Free 行为：

- 可本地使用。
- 可继续账户并同步创作者身份 / 权益状态。
- 可看到云同步说明。
- 未订阅时不能启用正式云同步。

Creator 行为：

- 默认自动启用云同步。
- 同步版权库、验证记录、创作者档案和权益状态。
- 自动把版权库同步白名单字段保存到云端，供桌面端和移动端自动拉取合并。

任务：

- [x] 桌面端云同步门禁。
- [x] 移动端云同步门禁。
- [x] 过期后同步降级策略已迁移到 Phase 8 订阅状态生命周期。
- [x] 权益恢复后同步恢复策略已迁移到 Phase 8 订阅状态生命周期。
- [x] CI 契约测试。

验收标准：

- Free 不能开启正式云同步。
- Creator 默认自动开启云同步；用户可暂停自动同步，但不应把“开启云同步”作为 Creator 的必选手动步骤。
- 不同步原始媒体和本地路径。
- 不同步原始媒体、保护副本文件和本地路径。

当前进展：

- Free 默认权益 `cloud_sync=false`。
- 桌面端和移动端已拆分“继续账户”和“正式云同步”：Free 可登录账户，但同步队列、拉取云变更和同步开关被门禁阻断。
- 桌面端 Tauri 正式云同步命令已增加权益硬门槛，避免前端绕过。
- 云同步 CI 契约已覆盖 Free 默认 `cloud_sync=false`、桌面端命令层权益门禁、云端继续账户默认权益。

当前边界：

- Phase 4 不包含真实支付状态变化后的过期 / 恢复自动处理。
- 过期、宽限期、恢复订阅、退款撤销统一归入 Phase 8，避免云同步门禁阶段长期处于“进行中”。
- Phase 4 已补齐后端 `syncPolicy`、Free 403 阻断、桌面端和移动端 Creator 自动 push / pull 合同，并已落地正式 `auth/challenges -> auth/sessions -> auth/refresh -> auth/logout -> me` 主链路与 `PATCH /v1/me/sync-preferences` 暂停 / 恢复当前设备自动同步；真实双设备截图 QA 仍归入用户体系后续任务。

验证：

- `npm run cloud:contract`
- `npm run cloud:e2e`
- `npm run cloud:ci`

## 8. Phase 5：正式报告订阅门禁

状态：已完成

目标：

将正式报告、批量摘要和法务报告绑定到 Creator / Studio；同时把 Free 单份报告付费定义为 Phase 8 一次性商品，而不是 Phase 5 订阅权益。

任务：

- [x] 定义报告等级。
- [x] Free 只显示基础验证结果。
- [x] Creator 导出单条报告入口门禁。
- [x] Creator 导出批量摘要入口门禁。
- [x] Studio 团队报告已迁移到 Studio 后续上线验收，不作为 Phase 5 Creator 报告门禁验收项。
- [x] CI 契约测试。

验收标准：

- Free 未购买时不导出正式报告；购买后可导出对应记录的单份版权详细报告或对应案件的维权证据包。
- Creator 可以导出报告。
- 报告不包含原始媒体。

当前进展：

- 基础摘要：Free 可复制基础验证摘要 / 基础存证摘要，不视为正式报告。
- 正式报告：由 `report_export` 权益控制，Creator 起开放。
- Free 单份付费报告：单份版权详细报告 19.9 元 / 份，维权证据包 49.9 元 / 份；后端 fixture 已支持一次性 purchase session 和授权核销，双端版权库已接入购买入口与单记录导出核销，真实微信一次性商品后端核心已落地；真实商户参数、公网 HTTPS 回调、真实下单 / 回调 / 查单 / 退款撤销验收仍待执行。
- 桌面端验证页、版权库单条记录、版权库批量摘要入口已接入正式报告门禁。
- 移动端验证页、版权库记录详情已接入正式报告门禁。
- `report:contract` 已加入跨端契约，防止正式报告入口绕过 `report_export`。
- 桌面端已实现正式报告 Markdown + JSON 文件导出，并在成功后写入 `report_export` usage ledger。
- 移动端已实现同字段正式报告草稿生成与 `report_export` usage ledger 入账，不把正式报告计入视频用量。
- 正式报告明确不包含原始媒体、加水印媒体和本地媒体路径。
- 桌面端导出后已提供最近导出记录、打开报告目录、复制 Markdown 路径和复制 JSON 路径。
- 桌面端最近导出记录已持久化，验证页导出的正式报告也会回写版权库最近导出列表，重启后仍可打开目录或复制报告路径。

当前边界：

- Phase 5 已完成 Creator 级正式报告和批量摘要门禁。
- Free 单份报告付费不改变 Phase 5 完成状态，作为 Phase 8 一次性购买扩展推进。
- Studio 团队报告依赖真实团队共享版权库与团队审计闭环，归入 Studio 后续上线验收。

验证：

- `npm run report:contract`
- `cargo test --manifest-path src-tauri/Cargo.toml commands::report::tests --lib`
- `flutter test test/mobile_app_state_test.dart`

## 9. Phase 6：Studio 团队能力预留

状态：已完成

目标：

为 Studio 和 Enterprise 做数据模型和页面预留。

任务：

- [x] workspace role 模型。
- [x] team member 模型。
- [x] shared vault 权限模型。
- [x] team audit log 模型。
- [x] Studio 页面入口预留。

验收标准：

- 不影响 Creator。
- 不提前暴露未完成能力为可用能力。

当前进展：

- 已新增 `docs/Studio团队版权库模型设计.md`。
- 已明确 Studio 团队能力由 `team_workspace` 控制，Creator 不受影响。
- 已定义 workspace、team member、shared vault record、team audit log 四类核心实体。
- 已明确团队共享只同步版权元数据、成员权限和审计日志，不同步原始媒体、加水印媒体和本地路径。
- 已统一桌面端和移动端团队能力术语：团队空间、成员权限、共享版权库、团队审计。
- 桌面端和移动端已预留 Studio 团队空间入口，统一由 `team_workspace` 权益控制，只展示状态和边界，不开放未完成的团队管理动作。
- 已新增 `team:contract`，防止团队入口只改一端或绕过 `team_workspace` 语义。
- 反馈后端已落地 `GET /v1/team/workspaces/current`、`GET|POST /v1/team/workspaces`、`GET|POST /v1/team/workspaces/:workspace_id/members`、`PATCH /v1/team/members/:member_id`、`GET /v1/team/workspaces/:workspace_id/vault`、`POST /v1/team/workspaces/:workspace_id/vault/share`、`GET /v1/team/workspaces/:workspace_id/audit-logs`，并补齐 workspace seed、member CRUD、shared library share、audit log roundtrip 测试。
- 个人 workspace seed 已自动写入，团队能力仍只对 `team_workspace=true` 账号开放；Creator 仍不受影响。

当前边界：

- Phase 6 是“预留阶段”，不承诺真实团队成员管理、共享记录操作、席位计费或团队报告已经上线。
- 真实 Studio 团队功能需要在支付与订阅状态闭环之后进入独立实施阶段。
- 当前反馈后端的团队路由与数据模型已经可用，但仍只属于团队能力预留范畴，不代表团队商品、席位计费或 Studio 团队报告已经开放。

验证：

- `npm run team:contract`

## 10. Phase 7：视频云端能力预留

状态：已完成

当前真实能力边界以 `docs/当前真实能力边界说明.md` 为准；涉及可售能力、内部测试能力、明确不能承诺的商业化表述时，必须先同步回写该文档。

目标：

为未来视频云端盲水印做任务、额度和队列模型。

任务：

- [x] 定义视频能力三档：本地音频水印、画面指纹存证、端云协同画面盲水印。
- [x] 明确每档上传内容、成本模型、验证方式和套餐归属。
- [x] 定义云端视频任务模型。
- [x] 定义视频分钟额度。
- [x] 定义任务状态。
- [x] 定义成功扣额度规则。
- [x] 定义失败不扣额度规则。
- [x] 桌面端视频云能力入口按 L1 可用、L2 锁定两层展示。
- [x] 移动端视频云能力入口按 L1 可用、L2 锁定两层展示。

验收标准：

- 不承诺本地视频盲水印。
- 不上传原始视频，除非用户明确发起云端视频任务。
- 云端视频按分钟额度设计。

当前进展：

- 已新增 `docs/Phase 7 视频云端能力设计.md`。
- 已新增 `docs/Phase 7 L2视频指纹技术Spike.md`。
- 已将视频能力拆成 L1 本地音频盲水印、L2 画面指纹存证、L3 端云协同画面盲水印。
- L1 继续复用本地 Tauri 2 + Rust + FFmpeg + 音频 QIM 盲水印，不进入 quota ledger。
- L2 定义为不可逆画面指纹存证和相似性验证增强，不误称为画面盲水印。
- L3 定义为 `cloud_video_processing` 高阶能力，默认不上传原始视频，由云端生成一次性策略包，客户端本地渲染和自检。
- 已明确 L3 用户取消、格式不支持、服务异常、策略生成失败、自检失败均不扣视频分钟额度；成功完成后才扣额度。
- `docs/Phase 7 视频云端能力设计.md` 已补齐 `cloud_video_tasks`、`video_minutes`、任务状态流转和 `upload_manifest` 契约。
- 桌面端工作台当前展示 L1 视频音轨水印和 L2 视频指纹存证；不提供视频画面水印入口。
- 移动端工作台当前展示 L1 视频音轨水印和同步来的 L2 视频指纹存证记录；不开放本地或云端视频画面水印，不上传原始视频。
- 已新增 `cloud-video:contract`，防止云端视频入口误写成已支持能力或绕过上传边界。
- 已新增本地 `video_fingerprint_spike` 工具，可生成 `VideoFingerprintBundle` 并评估缩放、二压、中心裁剪攻击召回率。
- 已使用 `E:\Users\jihx\Pictures\*.mp4` 中 10 个真实视频样本完成 L2 技术 spike：
  - 第一版整帧 + 固定局部块为 25/30，通过缩放和二压，但中心裁剪只有 5/10。
  - 加入不可逆 `crop_windows` 裁剪候选窗口摘要后为 30/30，缩放、二压、中心裁剪均为 10/10。
  - 结论：L2 可以进入云端指纹存证 API 草案，但 API 必须包含整帧摘要、局部块摘要、裁剪候选窗口摘要三层字段，不能只保存整帧 root。
- 已新增 `docs/Phase 7 L2云端指纹存证API草案.md`，固化 L2 请求 / 响应、三层摘要字段、manifest 隐私拒绝、错误码、usage ledger 口径和不扣 `video_minutes` 的边界。
- `feedback-backend` 已新增 L2 存证最小后端契约：`POST /v1/video-fingerprints/notaries`、`VideoFingerprintNotaryRequest`、`video_fingerprint_notaries`、`cloud_usage_ledger`，并覆盖 manifest 隐私拒绝、缺少 `crop_window_fingerprint_root` 拒绝、成功请求不扣 `video_minutes` 的测试。
- 已新增 L2 HTTP 级 contract / E2E：`cloud-video:e2e` 验证 401、manifest 隐私拒绝、缺少裁剪窗口拒绝、workspace 拒绝和成功收据返回；`cloud-video:ci` 可启动临时 `feedback-backend` 并连续运行静态契约和 HTTP E2E。
- 桌面端 L2 存证 client 对接点已固定：`CloudSyncClient::create_video_fingerprint_notary`、`VideoFingerprintNotaryRequest`、`VideoFingerprintNotaryReceipt` 和 Tauri 命令 `create_video_fingerprint_notary` 已接入保存的账户上下文，只传不可逆 bundle / receipt，不接 UI，不上传视频文件。
- 桌面端已新增 `video_fingerprint_bundle_to_notary_request` 内部构造函数，将 `VideoFingerprintBundleForNotary` 的整帧摘要、局部块摘要、裁剪候选窗口摘要映射为 `VideoFingerprintNotaryRequest`，生成 `local_block_fingerprint_root`、`crop_window_fingerprint_root`、`fingerprint_root`，并固定 `video_upload_manifest_v1` 只上传不可逆 bundle 摘要，不上传原始视频、加水印视频或本地路径。
- `video_fingerprint_spike` 的 `bundle.json` 输出字段已与 `VideoFingerprintBundleForNotary` 对齐为 camelCase，并新增不联网 JSON 解析单元测试，确保真实 spike bundle 形态可以构造成 `VideoFingerprintNotaryRequest`。
- 已新增文件级 smoke：测试会写入 `bundle.json` fixture，从磁盘读取后计算文件 `sha256:` 和 bytes，再走 `video_fingerprint_bundle_to_notary_request`，验证 `upload_manifest.items[0]` 与真实 bundle 文件摘要和大小一致。
- 已新增 `cloud-video:bundles` 独立脚本，可递归校验 `video_fingerprint_spike` 输出目录中的所有 `bundle.json`，确认三层摘要、manifest 隐私边界、文件 sha256 / bytes 和可存证请求构造；`cloud-video:ci` 已接入该脚本。
- 桌面端工作台已新增 L2“视频指纹存证”实验入口：用户选择本地 `bundle.json` 后，由 Tauri 后端读取、计算 bundle 文件 sha256 / bytes、构造 `VideoFingerprintNotaryRequest` 并提交云端存证；入口只处理不可逆 bundle，不上传原始视频、加水印视频或本地路径。
- 桌面端已将 L2 从“导入已有 bundle”推进到产品内闭环：选择视频后可在本机生成不可逆 `bundle.json`，展示指纹包路径、摘要、采样帧数和耗时，再由用户确认提交云端存证。
- 新增 `generate_video_fingerprint_bundle` Tauri 命令和 `video_fingerprint_bundles` 本地输出目录，生成过程复用 FFmpeg 探测缓存，只写本地不可逆 bundle，不自动提交云端。
- `cloud-video:contract` 已加入桌面端生成入口、提交入口、不可逆 bundle 生成器和隐私边界校验。
- 桌面端视频指纹存证成功后会写入版权库记录，保存 `notary receipt`、`sourceHash`、`fingerprintRoot`、`bundleSha256`、bundle 大小、采样帧数、生成耗时和采样策略，不保存原始视频路径、本地 bundle 路径或媒体文件。
- 移动端版权库已补齐同一组 L2 视频存证字段，支持通过云同步接收桌面端视频存证记录，并在版权库中按“视频”筛选、搜索和查看存证编号、指纹根、bundle 摘要与采样策略。
- 桌面端正式报告已纳入 L2 视频指纹存证字段：`video_notary_id`、`video_notary_at`、收据签名、usage ledger、`video_fingerprint_root`、`video_bundle_sha256`、bundle 大小、采样帧数、生成耗时和采样策略；报告继续排除原始视频、本地 bundle 路径和媒体文件。
- 移动端正式报告草稿已同步展示同一组 L2 视频存证字段，并继续把导出行为计入 `report_export`，不计入 `video_minutes` 或视频用量。
- `report:contract` 已扩展到 L2 视频存证报告字段，固定桌面端报告、移动端草稿和隐私边界。
- 已新增 `cloud-video:ui-contract` 并接入 `cloud-video:ci`，固定桌面端“生成指纹包 -> 提交存证 -> 保存到版权库”和移动端“同步视频存证记录 -> 版权库查看 -> 正式报告草稿”的 UI 闭环。
- 移动端 widget 测试已覆盖同步后的视频指纹存证记录详情，确认可查看存证编号、指纹根、bundle 摘要和采样策略，且不暴露本地 bundle 路径或原始视频路径。

当前边界：

- Phase 7 已完成 L1 视频音轨水印和 L2 视频指纹存证的本地生成、云端存证、双端版权库展示、正式报告字段和合同测试。
- 后续双端一致性 Phase I 会继续为视频能力补充跨端算法与互验门禁：L1 视频音轨水印需要抽音轨互验，L2 视频指纹存证需要固化 bundle / notary / 同步 / 报告一致性，L3 端云协同画面盲水印在进入用户可见能力前必须接入正式 worker、双端入口、版权库、报告、跨端验证和失败文案。
- L3 端云协同画面盲水印已进入 release candidate 准备，不在当前用户承诺范围内；`npm run watermark:l3-video-visual-release-gate` 已跑完整 24 个 2K 样本池并通过，证据目录为 `tmp-ui-qa/l3-video-visual-release-gate/1782888912515/`；后端 `succeeded` 已从用户 bearer status update 拆到 trusted worker/admin `POST /internal/video-tasks/:task_id/completion`，要求 `strategyDigest`、`selfCheckThreshold`、`selfCheckConfidence`、`checkedFrames`、`watermarkedMediaHash`、output 字段、worker receipt 和 `serverReceiptSignature` 全部存在且 `confidence >= threshold`，通过后才允许写 `video_minutes`。
- 真实 worker 链路已覆盖上传清单解析、FFmpeg sandbox、registry-reserved UID 与 core payload 绑定、队列重放保护、失败归因、真实 MP4 输出、worker receipt 持久审计、普通对象上传、真实对象存储字节分发和短期签名下载授权；桌面 / 移动已展示 Studio / Enterprise L3 对象上传 release-gate 入口，并完成 succeeded task 下载入库、正式报告字段、同步字段和真实后端 desktop->mobile / mobile->desktop 运行态读取 QA。正式创建 / 上传向导、失败文案、隐私边界和完整用户可见操作流仍未作为可售能力上线。

风险：

- L2 只能作为辅助证据，不能替代水印命中。
- L3 策略包需要继续做防逆向设计，避免泄露服务端主密钥和嵌入规律；当前 worker 已证明 `watermark-core` 策略、写入、自检、trusted completion、对象上传、真实字节分发、签名下载授权、双端 succeeded task 领取、版权库 `video_visual_*` 写入、报告字段、同步队列和真实后端双向运行态读取可以闭环，但正式创建 / 上传向导、失败文案和隐私边界仍未上线为可售能力。
- 不同硬件编码器和平台二压策略会影响 L3 画面水印存活率，后续必须建立鲁棒性测试矩阵。
- 不能把 L2 指纹存证包装成 L3 视频画面盲水印，也不能用 L2 相似性证据替代 L1/L3 的水印命中互验。

下一步任务：

- 下一刀把桌面 / 移动正式创建 / 上传向导、用户可见失败文案和隐私边界接进同一条 release gate；完成前不得把 L3 写成可售 SLA，也不得由普通 status update 扣 `video_minutes`。商业化继续保留 Phase 8 / Phase 9 的外部上线准备事项，真实收费上线仍受微信支付真实联调和法务审阅阻断。

## 11. Phase 8：支付与订阅状态闭环

状态：阶段性完成

目标：

完成从“展示订阅方案”到“真实权益生效”的闭环。

任务：

- [x] 支付 provider 选择。
- [x] 订阅 webhook / entitlement 更新链路设计。
- [x] `billing:contract` 静态契约。
- [ ] 订阅创建。
- [ ] 订阅续费。
- [ ] 订阅取消。
- [ ] 试用期。
- [ ] 宽限期。
- [ ] 过期回收。
- [ ] 收据校验。
- [x] report purchase 退款与撤销授权可测试核心。
- [x] Free 单份版权详细报告 fixture 一次性购买与授权核销。
- [x] Free 维权证据包 fixture 一次性购买与授权核销。
- [x] Free 单份报告双端购买入口。
- [x] Free 单份报告真实微信一次性商品后端可测试核心。

验收标准：

- 订阅状态在桌面端和移动端一致。
- 权益过期后门禁生效。
- 恢复订阅后权益恢复。

当前边界：

- Phase 8 已完成设计冻结，支付不能写死到单一 provider；后端必须先抽象 `BillingProvider` 适配层。
- 首期 provider 固定为微信支付 APIv3，Stripe 保留为海外扩展 provider。
- 桌面端和移动端只消费后端返回的 `paymentAction` / `managementAction`，不保存支付凭证、商户密钥或 provider secret，也不自行修改正式权益。
- 订阅状态由后端 provider webhook 写入 `entitlements`，再投影到现有账户权益快照，双端继续消费同一份 `CloudEntitlement`。
- 已定义 `billing_customers`、`subscriptions`、`subscription_events`、`entitlements` 四类模型，以及 payment session、subscription management、current entitlement、provider webhook API。
- 已定义支付成功、续费、失败宽限、取消、过期、退款 / dispute 撤销的 provider 中立状态机。
- 已新增 `docs/Phase 8 支付与订阅状态闭环设计.md` 和 `billing:contract`，用于固定 Phase 8 设计边界。
- 后端已新增 provider 中立 `BillingProvider` 抽象、本地 `fixture` provider、`billing_customers`、`subscriptions`、`subscription_events`、`entitlements` schema、`/v1/billing/payment-sessions` 和 `/v1/billing/webhooks/fixture`。
- 后端 fixture 状态机已覆盖支付成功、重复事件幂等、失败宽限、退款降级到 Free feature map。
- 后端已新增微信支付 Native adapter 的可测试核心：构造 Native 下单请求、生成 APIv3 Authorization、验证平台回调签名、AES-256-GCM 解密 resource、校验金额并映射为标准 `BillingEvent`。
- 后端已接入微信支付运行时配置读取，支持从环境变量或文件读取 AppID、商户号、商户私钥、平台公钥、APIv3 key 和 notifyUrl。
- 后端 `payment-sessions` 在 `preferredProvider=wechat_pay` 时会走真实 Native 下单 HTTP client；微信配置缺失时返回 `wechat_pay_not_configured`，不会静默降级到 fixture。
- 后端已新增正式 `/v1/billing/webhooks/wechat-pay` 路由，完成微信平台回调验签、resource 解密、金额校验、标准 `BillingEvent` 映射和 entitlement 状态迁移。
- 桌面端订阅面板已接入 `create_billing_payment_session`，Creator / Studio 可创建微信支付会话并展示二维码 / H5 支付动作；未继续账户或支付通道未配置时给出明确提示。
- 移动端订阅 Sheet 已接入同一套 payment session API，Creator / Studio 可创建支付会话并展示支付动作状态；未登录或支付通道未配置时给出明确提示。
- 后端已新增 `/v1/entitlements/current`，桌面端和移动端均提供“刷新权益”动作；支付动作创建后可重新拉取云端权威 entitlement，并按 active / grace / expired / free 显示完成态文案。
- 已新增 `docs/Phase 8 支付状态补偿机制设计.md`，明确 payment session 账本、provider order 查单补偿、手动 reconcile、后台补偿任务和双端轻量轮询边界。
- 后端已落地 fixture 支付状态补偿闭环：新增 `billing_payment_sessions` 账本、payment session 状态查询、手动 reconcile API、fixture `query_order`，并验证“支付成功但 webhook 未到”也能恢复 entitlement。
- 双端接入支付会话状态与轻量轮询已落地：桌面端和移动端在支付会话创建后读取 session 状态、调用 reconcile，并在前 2 分钟内轻量确认；手动按钮统一为“确认支付”，不再把支付完成态表达成单纯刷新权益。
- 后端后台补偿任务与 provider 退避节流已落地：服务启动后按 `HIDDENSHIELD_BILLING_RECONCILE_INTERVAL_SECS` 周期扫描 `created/pending` 且到达 `next_check_after` 的支付会话，fixture provider 会复用 `query_order` 和 `apply_billing_event` 恢复权益；非 fixture provider 会更新检查时间并退避，不会在没有真实 provider 查单实现时本地判定支付成功。
- 微信支付会话已不再用 fixture session 占位持久化，后端会保存 `provider=wechat_pay` 与真实 `provider_order_id`；微信订单只能通过微信 webhook 或 provider adapter 查单结果写入正式 entitlement。
- 真实微信查单 adapter 的可测试核心已落地：新增 `out_trade_no` 查单请求签名、`WechatOrderQueryResponse`、`trade_state` 到 `BillingOrderStatus` 的映射、金额校验、失败态处理，以及 `wechat_pay` order status 复用标准 `apply_billing_event` 的后端测试。
- 后台补偿任务在配置微信支付 adapter 后可对 `wechat_pay` session 执行查单；查单失败或无微信配置时仍只退避，不本地开通权益。
- 项目当前仍没有真实商户沙箱 / 生产联调、真实支付回调验收、退款撤销运行态验收和双端付费 QA。
- 已新增 `docs/Phase 8 微信一次性商品联调Checklist.md`，固化真实微信一次性商品联调所需商户参数、环境变量、公网 HTTPS 回调、下单验收、支付成功验收、查单补偿、退款撤销、双端运行态 QA、日志留存和上线阻断项。
- Phase 4 中迁移出的过期降级、权益恢复、宽限期回收，统一在 Phase 8 处理。
- 真实微信支付接入必须只发生在 provider adapter 层，业务层继续只消费标准 `BillingEvent`。
- Free 单份报告付费已新增设计文档 `docs/Phase 8 Free单份报告付费设计.md`，并完成后端 fixture 闭环：一次性商品 allowlist、report purchase session、`report_purchase_grants` 授权核销、状态查询和重复确认幂等；桌面端和移动端版权库已接入购买入口，正式报告导出支持 Creator `report_export` 或单记录有效授权二选一通过；后端已接入真实微信一次性商品 Native 下单、`purchaseType=report_purchase` attach 分流、查单 / webhook 成功授权和退款撤销授权，不能复用订阅升级语义直接开通 `report_export`。

下一步任务：

- 准备真实微信商户参数和公网 HTTPS 回调环境，按 `docs/Phase 8 微信一次性商品联调Checklist.md` 执行一次性商品真实下单联调。

## 12. Phase 9：商业化验收与上线

状态：阶段性完成

目标：

确保商业化功能能真实上线。

任务：

- [x] Phase 9 商业化验收 checklist。
- [x] `commercial:contract` 静态契约。
- [x] `commercial:ci` 自动化总门禁。
- [x] 桌面端订阅页面 QA。
- [x] 移动端订阅页面 QA。
- [x] 权益门禁 QA。
- [x] 批量订阅服务 QA。
- [x] 云同步订阅门禁 QA。
- [x] 隐私政策草案更新。
- [x] 用户协议草案更新。
- [x] 支付条款草案更新。
- [x] 首期指标看板。

验收标准：

- Free / Creator / Studio 行为一致且可解释。
- 核心付费路径可完成。
- 法务文案和产品行为一致。
- 商业指标可追踪。

当前进展：

- 已新增 `docs/Phase 9 商业化上线验收Checklist.md`。
- Checklist 已拆分自动化验收、桌面端人工验收、移动端人工验收、后端与支付验收、法务与文案验收、指标看板验收、上线阻断项。
- 已明确真实微信商户沙箱 / 生产联调需要你提供微信支付商户号、AppID、商户 API 证书 / 私钥、平台公钥、APIv3 key 和公网 HTTPS 回调域名。
- 已新增 `commercial:contract`，用于固定 Phase 9 checklist、Roadmap 回写和自动化验收命令清单。
- 已新增 `commercial:ci`，串行运行商业化自动验收命令，并确保 `cloud:ci` 与 `cloud-video:ci` 不并行执行。
- 已新增 `docs/Phase 9 商业化双端QA记录.md`，按桌面端、移动端、权益门禁、本地批量、云同步、报告 / 视频存证和支付联调边界记录 PASS / BLOCKED。
- 双端商业化证据验收已通过：订阅页面、Creator 本地批量、正式云同步、正式报告、Studio 团队预留、L2 视频指纹存证展示和移动端隐藏桥接 / 临时直连均已确认。
- 已新增 `docs/Phase 9 隐私政策草案.md`、`docs/Phase 9 用户协议草案.md`、`docs/Phase 9 支付与订阅条款草案.md`，并同步更新现有 `docs/隐私政策.md`。
- 桌面端和移动端已统一展示 Phase 9 边界文案：默认不同步原始媒体 / 加水印媒体 / 本地路径，L2 视频是指纹存证不是画面盲水印，正式报告不是法律意见或司法鉴定，确认支付只触发查单或刷新。
- 已新增 `docs/Phase 9 商业指标看板设计.md`，首期固定云端聚合接口 `/v1/commercial/metrics/overview` 与双端本机商业健康摘要。
- 后端商业指标聚合已覆盖继续账户、权益分布、支付会话、本地批量、正式报告、云同步、L2 视频指纹存证和匿名失败分类；响应内固定隐私边界，不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希。
- 桌面端和移动端设置页已展示同一口径的商业健康摘要；桌面端展示最近正式报告次数，移动端只展示最近报告状态，避免虚构本机累计值。
- 已新增 `commercial:metrics` 并接入 `commercial:ci`。
- `/v1/commercial/metrics/overview` 已接入管理员 token 鉴权和 `admin_audit_events` 访问审计；管理员 token 由系统配置 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN` 提供，未配置时默认拒绝访问。

当前边界：

- Phase 9 已完成双端商业化证据验收、法务条款草案、首期指标看板和指标接口管理员鉴权；尚未执行真实微信商户联调、正式法务审阅和生产环境管理员 token 配置。
- 本轮 QA 以代码、合同脚本和自动化测试为证据；正式发布前仍建议执行一次运行态 Tauri / 移动端真机交互验收。
- 真实微信支付联调、法律顾问审阅确认和生产环境 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN` 配置仍是正式收费上线前阻断项。

验证：

- `npm run commercial:contract`
- `npm run commercial:metrics`
- `npm run commercial:ci`

## 13. 当前推荐执行顺序

商业化落地已阶段性收尾，后续默认不再继续商业化细枝末节。

1. 下一阶段主线切换到 `docs/双端能力一致性Roadmap.md`：先完成 Phase A 双端能力矩阵审计。
2. 商业化只保留外部上线准备事项：真实微信商户联调、法律顾问审阅、生产环境 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN` 配置。
3. 如后续重新进入商业化，应先回到本文档确认是否属于正式上线准备或新商业能力扩展。

## 14. Roadmap 回写记录

| 日期 | 变更 | 状态 |
| --- | --- | --- |
| 2026-06-18 | 创建商业化落地 Roadmap，固化本地批量作为 Creator 订阅权益，并更新 AGENTS.md 要求后续商业化任务回写 Roadmap。 | 已完成 |
| 2026-06-18 | 补充 AGENT.MD 兼容入口的商业化 Roadmap 约束，并统一商业模式文档中的 Studio 套餐命名。 | 已完成 |
| 2026-06-18 | 完成 Phase 1：新增商业化契约文档，补齐后端 / 桌面端 / 移动端权益字段，并将云同步合同测试扩展到完整 feature map。 | 已完成 |
| 2026-06-18 | 完成 Phase 2：统一桌面端和移动端订阅 / 权益页面为 Free / Creator / Studio / Enterprise 四档。 | 已完成 |
| 2026-06-18 | 推进 Phase 3：新增本地批量任务模型文档，并完成桌面端 / 移动端的本地批量入口、Free 门禁和 Creator 队列页占位。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：桌面端 / 移动端 Creator 队列页接入批量文件选择、BatchJob / BatchItem 内存状态、暂停 / 继续 / 取消和失败项重试。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：桌面端新增 local_batch_jobs / local_batch_items SQLite 表和 Tauri 命令，移动端新增 VaultStore 批量队列持久化，队列可跨重启恢复。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：桌面端 / 移动端接入图片批量真实执行链路，顺序写入、完成后验证并入版权库，音频批量暂缓。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：明确移动端音频能力必须与桌面端保持同格式支持，开始补齐常见音频格式到 WAV 的本地归一化桥接，音频批量需等待该桥接验证通过。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：桌面端 / 移动端本地批量统一处理图片和音频，复用单文件写入后验证，成功项写入版权库。 | 进行中 |
| 2026-06-18 | 推进 Phase 3：桌面端批量入口补齐 local_batch telemetry，移动端新增 usage ledger 本地持久化与设置页摘要，成功写入项统一计入账本。 | 已完成 |
| 2026-06-18 | 收口 Phase 3：桌面端与移动端处理统计文案和字段口径统一，新增 `usage:contract` 跨端契约脚本，Roadmap 状态更新为已完成。 | 已完成 |
| 2026-06-18 | 推进 Phase 4：Free 默认 `cloud_sync=false`，桌面端 / 移动端拆分继续账户与正式云同步，并在桌面端 Tauri 云同步命令增加权益硬门槛。 | 进行中 |
| 2026-06-18 | 完成 Phase 4 CI 契约门禁：云同步合同测试改为验证 Free 默认关闭，补充桌面端命令层与云端继续账户权益测试。 | 已完成 |
| 2026-06-18 | 推进 Phase 7：新增视频云端能力设计，将视频能力拆为 L1 本地音频水印、L2 画面指纹存证、L3 端云协同画面盲水印，并明确上传边界、成本模型、验证方式、套餐归属和成功后才扣额度规则。 | 进行中 |
| 2026-06-18 | 推进 Phase 5：拆分基础摘要与正式报告，桌面端 / 移动端正式报告入口统一绑定 `report_export`，新增 `report:contract` 跨端契约。 | 进行中 |
| 2026-06-18 | 推进 Phase 5：桌面端落地 Markdown / JSON 正式报告导出并入账，移动端落地同字段报告草稿与 report 用量类型，报告不包含媒体文件与本地路径。 | 进行中 |
| 2026-06-18 | 推进 Phase 5：桌面端导出后体验补齐最近导出记录、打开报告目录、复制 Markdown / JSON 路径，并纳入 `report:contract`。 | 进行中 |
| 2026-06-18 | 推进 Phase 5：桌面端最近导出记录持久化，验证页导出也回写最近导出历史，重启后仍可打开报告目录和复制报告路径。 | 进行中 |
| 2026-06-18 | 推进 Phase 6：新增 Studio 团队版权库模型设计，定义 workspace role、team member、shared vault 权限和 team audit log，并明确团队共享不包含媒体文件和本地路径。 | 进行中 |
| 2026-06-18 | 推进 Phase 6：桌面端和移动端预留 Studio 团队空间入口，统一 `team_workspace` 权益门禁和共享边界文案，并新增 `team:contract`。 | 进行中 |
| 2026-06-18 | 推进 Phase 7：补齐云端视频任务、视频分钟额度、任务状态和上传清单契约；当前封版已收口为桌面端只展示 L2 视频指纹存证，移动端只读展示同步来的 L2 视频指纹存证记录，不开放本地或云端视频画面水印。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增 L2 视频指纹技术 spike 文档和 `video_fingerprint_spike` 本地工具，用于生成 VideoFingerprintBundle 并评估缩放、二压、裁剪攻击召回率。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：使用 10 个真实视频样本完成 L2 spike。第一版整帧 / 固定块为 25/30，加入不可逆裁剪候选窗口摘要后为 30/30，决定 L2 API 必须包含整帧、局部块、裁剪候选窗口三层摘要。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增 L2 云端指纹存证 API 草案，明确请求 / 响应、三层不可逆摘要、manifest 隐私拒绝、错误码、usage ledger 和不扣视频分钟边界。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：feedback-backend 新增 L2 存证最小契约实现 `create_video_fingerprint_notary`，覆盖 manifest 隐私拒绝、裁剪窗口必填和不扣 `video_minutes` 的后端测试。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增 `cloud-video:e2e` 和 `cloud-video:ci`，用真实 HTTP 请求验证 L2 存证 401、manifest 拒绝、裁剪窗口必填、workspace 拒绝和成功收据返回。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：桌面端新增 L2 存证 client 和 Tauri 命令占位，固定不可逆 bundle / receipt 对接点，不接 UI，不上传视频文件。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：桌面端新增 `video_fingerprint_bundle_to_notary_request`，将三层不可逆视频指纹摘要映射为 L2 存证请求，并用单元测试固定 manifest 隐私、三层 root、确定性和裁剪敏感性。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：`video_fingerprint_spike` 的 `bundle.json` 输出改为 camelCase，与 `VideoFingerprintBundleForNotary` 对齐，并新增不联网 JSON 解析测试，确保真实 spike bundle 形态可构造成 L2 存证请求。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增文件级 `bundle.json` smoke，按真实文件 bytes 计算 `sha256:` 并构造 L2 存证请求，固定 upload manifest 与不可逆 bundle 文件摘要一致。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增 `cloud-video:bundles` 脚本并接入 `cloud-video:ci`，批量校验 spike 输出目录中的 `bundle.json` 可构造成隐私安全的 L2 存证请求。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：桌面端工作台新增“视频指纹存证”实验入口，可选择本地不可逆 `bundle.json` 并提交云端存证，后端计算文件摘要并保证不上传视频文件或本地路径。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：桌面端新增本地不可逆视频指纹包生成入口，选择视频后生成 `video_fingerprint_bundles/.../bundle.json`，展示摘要 / 耗时 / 采样帧并由用户确认提交云端存证；`cloud-video:contract`、`cloud-video:bundles`、`cloud-video:ci` 均通过。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：视频指纹存证结果接入双端版权库模型。桌面端提交后保存云端收据与 bundle 摘要，移动端可通过云同步接收并展示同字段视频存证记录；两端均不保存原始视频路径、本地 bundle 路径或媒体文件。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：视频指纹存证记录接入双端正式报告。桌面端 Markdown / JSON 与移动端报告草稿均展示 L2 存证收据、fingerprintRoot、bundleSha256、bundle 大小、采样帧、耗时和采样策略，并扩展 `report:contract` 固定不泄漏媒体路径。 | 进行中 |
| 2026-06-19 | 推进 Phase 7：新增 `cloud-video:ui-contract` 并接入 `cloud-video:ci`，固定桌面端生成指纹包、提交存证、保存到版权库与移动端同步查看的 UI 闭环；移动端 widget 测试覆盖视频存证详情展示与路径不泄漏。 | 进行中 |
| 2026-06-19 | 收口 Phase 0-8 状态：Phase 4/5/6/7 按当前代码与合同测试确认为已完成，真实支付生命周期统一保留在 Phase 8，L3 端云协同画面盲水印明确为未来能力。 | 已完成 |
| 2026-06-19 | 完成商业化 Roadmap 状态收口验证：`cloud:ci`、`usage:contract`、`report:contract`、`team:contract`、`cloud-video:ci` 均通过；文档明确 `cloud:contract` 单跑需先启动后端，云同步与云视频 CI 需串行避免端口冲突。 | 已完成 |
| 2026-06-19 | 启动 Phase 8：新增支付与订阅状态闭环设计，调整为 provider 抽象层优先，首期 provider 固定为微信支付 APIv3，Stripe 作为海外扩展；webhook 统一驱动 entitlement，双端只消费权益快照，并新增 `billing:contract` 静态契约。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：后端新增 provider 中立 `BillingProvider`、本地 fixture provider、四张 billing schema、payment session API、fixture webhook 和 entitlement 状态机测试；`billing:contract` 已纳入后端实现检查。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：新增微信支付 Native adapter 可测试核心，覆盖 Native 下单请求签名、APIv3 Authorization、平台回调验签、AES-GCM resource 解密、金额校验和 `wechat_pay` 到标准 `BillingEvent` 的映射。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：微信支付 adapter 接入后端运行时，新增环境变量 / 文件配置读取、真实 Native 下单 HTTP client、正式 `/v1/billing/webhooks/wechat-pay` 路由，并保持配置缺失时显式返回 `wechat_pay_not_configured`。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：桌面端订阅面板和移动端订阅 Sheet 接入 payment session 入口，Creator / Studio 可请求后端创建微信支付动作，未继续账户或通道未配置时显示产品化提示；`billing:contract` 已覆盖双端入口。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：新增 `/v1/entitlements/current` 云端权益刷新接口，桌面端 `refresh_billing_entitlement` 会同步更新本地 entitlement_state，移动端订阅 Sheet 也提供刷新权益按钮并持久化云端权益快照；`billing:contract` 覆盖双端刷新入口。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：新增支付状态补偿机制设计，固定 `billing_payment_sessions`、session 状态查询、手动 reconcile、后台补偿任务、provider `query_order` 与双端轻量轮询边界；客户端仍不得自行判定支付成功或写正式权益。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：落地支付状态补偿 fixture 闭环，新增 `billing_payment_sessions` 账本、payment session 状态查询、手动 reconcile API 和 fixture `query_order`；后端测试覆盖“支付成功但 webhook 未到”也能恢复 Creator entitlement。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：落地后端后台支付补偿任务与 provider 退避节流，服务启动后按 `next_check_after` 扫描 pending session；fixture 可自动查单恢复权益，`wechat_pay` session 在真实查单 adapter 前只退避不本地开通，避免绕过微信 webhook。 | 进行中 |
| 2026-06-19 | 推进 Phase 8：落地真实微信查单 adapter 可测试核心，固定 `out_trade_no` 查单请求、`trade_state` 映射、金额校验和标准 `BillingOrderStatus` 入账路径；后台补偿在配置微信支付后可处理 `wechat_pay` session，仍未进入真实商户联调。 | 进行中 |
| 2026-06-19 | 启动 Phase 9：新增 `docs/Phase 9 商业化上线验收Checklist.md`，拆分自动化、双端人工、后端支付、法务文案、指标看板和上线阻断项，并新增 `commercial:contract` 固定商业化验收入口。 | 进行中 |
| 2026-06-19 | 推进 Phase 9：新增 `commercial:ci` 自动化总门禁，串行运行商业化合同、billing、usage、report、team、桌面构建、后端测试、Tauri 测试、Flutter analyze/test、cloud:ci 和 cloud-video:ci。 | 进行中 |
| 2026-06-19 | 推进 Phase 9：新增 `docs/Phase 9 商业化双端QA记录.md`，完成桌面端 / 移动端商业化证据验收，并将 QA 记录纳入 `commercial:contract`；真实微信联调、法务条款和指标看板仍为收费上线阻断项。 | 进行中 |
| 2026-06-19 | 推进 Phase 9：补齐隐私政策、用户协议和支付订阅条款草案，并同步桌面端 / 移动端条款边界文案；`commercial:contract` 已固定不同步媒体 / 路径、L2 不是视频盲水印、报告不是法律意见、确认支付不直接开通等口径。 | 进行中 |
| 2026-06-19 | 推进 Phase 9：完成首期商业指标看板设计与实现，新增云端 `/v1/commercial/metrics/overview` 聚合接口、双端商业健康摘要和 `commercial:metrics` 合同；指标只统计账户、权益、支付会话、功能次数、同步状态和匿名失败分类，不采集媒体、路径、文件名或完整哈希。 | 已完成 |
| 2026-06-19 | 推进 Phase 9：给 `/v1/commercial/metrics/overview` 增加系统配置管理员 token 鉴权和 `admin_audit_events` 访问审计；未配置 `HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN` 时默认拒绝访问，审计不保存 token、媒体、路径、文件名或完整哈希。 | 已完成 |
| 2026-06-19 | 商业化落地阶段性收尾：Phase 8 / Phase 9 状态改为阶段性完成，外部上线准备事项保留为真实微信商户联调、法务审阅和生产 token 配置；下一阶段主线切换到 `docs/双端能力一致性Roadmap.md`。 | 已完成 |
| 2026-06-20 | 完成双端产品语言最终对齐涉及的商业化文案收敛：Creator 权益从“证据报告导出 / 报告导出”统一为“正式报告”，报告仍由 `report_export` 权益控制；同步说明中的“取证记录”统一为“验证记录”；L2 视频能力说明从“相似性取证增强”统一为“相似性验证增强”。验证：`npm run commercial:contract` 与 `npm run report:contract` 通过。下一步补一次订阅页与正式报告入口的运行态截图验收。 | 已完成 |
| 2026-06-20 | 配合双端一致性 Phase I 收紧商业化视频算法边界：正式盲水印 payload、版权编号、媒体哈希片段和身份派生只能由 `watermark-core` 生成；桌面端 L1 视频音轨水印、图片和音频写入已切到同一 core builder；未来 L3 端云协同画面盲水印不得在后端或云任务另起算法核心，只能包装 `watermark-core` 或其部署产物。 | 进行中 |
| 2026-06-20 | 配合 Phase I 补齐结构化水印错误码：正式水印失败会输出稳定 code，双端用户文案不再依赖英文技术错误；该 code 体系后续可用于 L3 端云协同画面盲水印的失败归因、重试策略和“成功后才扣视频分钟额度”判定。 | 进行中 |
| 2026-06-20 | 配合 Phase I 接入首个跨端互验发布门禁：`watermark:cross-end-contract` 已覆盖图片和音频 mobile->desktop、desktop->mobile 双向互验并进入 CI；未来 L1 视频音轨和 L3 视频画面盲水印上线前也必须进入同类互验门禁，不能只靠单端演示。 | 进行中 |
| 2026-06-20 | 配合 Phase I-2 扩展真实图片容器门禁：PNG / JPEG / WebP 输入已进入双端互验硬门禁；后续商业化视频 L1 音轨和 L3 画面盲水印上线前，也必须先建立不依赖本机环境运气的标准 fixture 或同核编码策略，再进入发布门禁。 | 进行中 |
| 2026-06-20 | 配合 Phase I-2 扩展真实音频容器门禁：MP3 / FLAC / OGG / M4A 使用仓库固定 fixture 进入 `watermark:cross-end-contract`，验证移动端归一化后仍进入同一 `watermark-core` payload，且产物可被桌面 core 提取；未来 L1 视频音轨也应以固定容器 fixture 锁定抽取后的同核互验。 | 进行中 |
| 2026-06-20 | 配合 Phase I-2 固化跨端互验失败归因：发布门禁失败会按 `core_algorithm`、`mobile_normalize`、`desktop_transcode`、`bridge_contract`、`fixture_invalid` 分类；后续 L1 视频音轨和 L3 端云协同画面盲水印必须沿用该归因体系，避免把转码失败、核心失败和 fixture 损坏混成一个商业化阻断项。 | 进行中 |
| 2026-06-20 | 配合 Phase I-2 接入 `desktop_transcode` 真实门禁：桌面 FFmpeg 对 MP3 / FLAC / OGG / M4A fixture 抽取成 WAV 后必须能进入 `watermark-core` 写入和提取；这为后续商业化 L1 视频音轨水印提供“视频/音频转码层”和“核心算法层”分离验收基础。 | 进行中 |
| 2026-06-20 | 配合 Phase I-2 拆分跨端互验 fast / release：`watermark:cross-end-fast` 服务本地快速检查，`watermark:cross-end-release` 和原 `watermark:cross-end-contract` 保持完整发布门禁；商业化上线仍以 release 门禁为准，不用 fast 代替完整矩阵。 | 进行中 |
| 2026-06-20 | 配合 Phase I-4 收口移动端 Web 预览商业边界：Web 预览不调用 `watermark-core` 时只能作为 UI 体验，不能生成正式保护副本、正式版权库记录、正式报告证据或云同步 payload；商业化验收、订阅权益和跨端互验不得把 Web preview marker 当作正式能力。 | 进行中 |
| 2026-06-20 | 配合 Phase I-5 补齐原生移动端单文件保护副本出口：图片 / 音频正式写入成功后可通过系统分享面板保存或交给其他应用；该出口不保存媒体路径、不上传保护副本、不把本地路径计入云同步或商业指标。正式商业验收仍需真机分享到相册 / 文件后由桌面端验证。 | 进行中 |
| 2026-06-20 | 配合 Phase I-6 视频一致性收口：L1 视频音轨水印已纳入 release 门禁，验证成品 MP4 抽音轨后仍能由 `watermark-core` 读出同一版权编号，且 L1 本地处理不扣 `video_minutes`；新增 `watermark:video-phase-contract` 固定 L2 三层指纹存证不替代 L3 画面盲水印、L3 仍为未来 `cloud_video_processing` 能力，进入实现前必须先补算法、策略包、防逆向、密钥边界、客户端自检和成功后扣费设计。 | 进行中 |
| 2026-06-20 | 完成 L3 商业化实现前设计冻结：新增 `docs/Phase I-6 L3视频画面盲水印同核与云端策略设计.md`，明确 L3 仍未开放，Free / Creator 不公开承诺正式 L3，Studio Beta / Enterprise 才进入未来 `cloud_video_processing`；只有策略包生成成功、客户端渲染成功、成品自检通过并固化云端收据后才扣 `video_minutes`，用户取消、策略失败、自检失败、客户端渲染失败和服务异常均不扣费。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前核心化：`watermark-core` 新增最小视频视觉契约和 L3 错误码，先锁定策略、feature bundle、自检结果和 `video_strategy_v1` schema；当前仍不开放 `cloud_video_processing`、不扣 `video_minutes`、不接 UI 和云端任务。下一步必须先在 core 合成帧上完成 embed/extract roundtrip，再设计 Studio Beta / Enterprise 的任务包装。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前 core roundtrip：`watermark-core` 已能在合成帧 fixture 上写入并读回正式 payload，但这仍是算法内部验证，不代表 Studio Beta / Enterprise 的 `cloud_video_processing` 已可销售；不开放任务、不上传视频、不扣 `video_minutes`。下一步需要多帧冗余和自检 confidence 后，才允许讨论商业任务包装。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前多帧自检：`watermark-core` 已在合成帧 fixture 上完成多帧冗余写入、任一有效帧提取和提取驱动 confidence；这仍不代表 `cloud_video_processing` 可销售，不开放 Studio Beta / Enterprise 任务、不上传视频、不扣 `video_minutes`。下一步先补 core 内扰动鲁棒性 fixture，再讨论任务包装。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前性能与鲁棒性基线：`watermark-core` 合成帧 fixture 已覆盖帧缺失、亮度偏移、本地擦除检测，并新增 12 帧合成 roundtrip 性能基线；这不是商业 SLA，不开放 `cloud_video_processing`、不上传视频、不扣 `video_minutes`。下一步先补缩放 / 裁剪 / 压缩模拟，再评估 Studio Beta / Enterprise 的任务包装成本。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前复杂度预算：`watermark-core` 合成帧 fixture 已覆盖裁剪 / 压缩模拟，并把性能预算扩展到 4 / 12 / 24 帧三档；这仍不是可销售的 `cloud_video_processing` 能力，不上传视频、不扣 `video_minutes`。下一步必须先形成真实鲁棒画面算法设计和成本模型，再评估 Studio Beta / Enterprise 任务包装。 | 进行中 |
| 2026-06-20 | 完成 L3 商业化实现前真实算法设计冻结：新增 `docs/Phase I-6 L3真实鲁棒画面盲水印算法设计.md`，首版算法路线为 Y 平面 8x8 DCT 中频系数相对关系写入，并明确小 / 标准 / 高阶视频复杂度预算；这仍不开放 `cloud_video_processing`，不上传视频、不扣 `video_minutes`。下一步只允许 core 内 DCT block 单测，不做商业任务包装。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前 DCT block 验证：`watermark-core` 已新增 `LumaDctMidBandV1` profile 和 8x8 DCT block 写入 / 读取 bit 单测；这仍不是可销售的 `cloud_video_processing` 能力，不上传视频、不扣 `video_minutes`。下一步只允许 core 内 sync marker / ECC fixture，不做商业任务包装。 | 进行中 |
| 2026-06-20 | 配合 L3 商业化实现前 DCT 帧级验证：`watermark-core` 已新增 `sync_marker_v1`、轻量 ECC repeat bitstream、DCT bitstream block helper 和 `LumaDctMidBandV1` luma 帧级 payload roundtrip；这仍不是可销售的 `cloud_video_processing` 能力，不上传视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许 core 内 DCT 多帧冗余、自检 confidence、扰动和性能基线，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 DCT 多帧自检：`watermark-core` 已新增 DCT 多帧写入 / 提取 / 自检 helper，confidence 由实际读回正式 payload 的帧比例计算，并加入缺帧、擦除失败和 4 帧 512x512 性能基线；同一 8x8 block 内多个 coefficient pair 合并为一次 DCT / IDCT。该能力仍不是可销售的 `cloud_video_processing`，不上传视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许 core 内 DCT 频域扰动矩阵和复杂度预算，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 DCT 频域扰动矩阵：`watermark-core` 已覆盖统一亮度偏移、保守量化压缩和 2x 下采样再最近邻上采样；亮度 / 量化必须通过自检，重采样当前必须返回 `self_check_failed`，明确不能把该 staged API 宣称为已抗缩放的商业视频画面盲水印。仍不开放 `cloud_video_processing`，不上传视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许 core 内复杂度预算、帧抽样策略和真实视频帧解码边界，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 core 复杂度预算：`watermark-core` 已新增 `VideoVisualComplexityTier` / `VideoVisualComplexityBudget`、三档 staged 预算和确定性均匀抽帧函数；这些预算只约束 core fixture，不是商业 SLA，也不能作为 Studio / Enterprise 的 `cloud_video_processing` 售卖承诺。仍不开放任务、不上传视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许 core 内真实视频帧解码边界和固定 Y-plane fixture，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 decoded Y-plane 边界：`watermark-core` 已新增 `DecodedVideoLumaPlane`、`VideoLumaBitDepth`、`VideoLumaColorRange` 和 `video_frame_plane_from_decoded_luma`，统一 Y plane 归一化、stride padding 和 profile 拒绝规则；固定 10-bit limited Y-plane fixture 可完成 DCT payload roundtrip。这仍不是可销售的 `cloud_video_processing`，不接真实视频文件、不上传视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许真实视频容器解码到固定 Y-plane fixture 的测试边界，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前真实容器解码边界：桌面 Tauri 测试已用 FFmpeg 生成受控 10-bit MP4，解码第一帧为 `gray10le` raw Y plane 并进入 `watermark-core::video_frame_plane_from_decoded_luma`；该测试进入 release 跨端合同。这仍不是可销售的 `cloud_video_processing`，不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许真实容器解码出的 Y-plane fixture 到 DCT staged roundtrip 的测试桥，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前真实容器 DCT staged roundtrip：`watermark-core` 已导出 DCT staged 写入 / 提取 / 自检 API，桌面 Tauri release 测试使用 FFmpeg 生成 4 帧 10-bit MP4，解码为 `gray10le` 后只调用 core API 完成正式 payload 写入、读回和自检。这仍不是可销售的 `cloud_video_processing`，不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补受控编码回写后的 DCT 自检门禁，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前受控编码回写自检：桌面 Tauri release 测试将写入后的 Y plane 经 FFmpeg `libx264 -crf 0` 编码为受控 MP4，再解码为 `gray10le` 后只调用 `watermark-core` staged API 提取和自检；当前基线为 4 帧中 3 帧读回、confidence 达到 0.75 阈值。这仍不是可销售的 `cloud_video_processing`，不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补受控有损压缩矩阵和失败归因，不做商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前有损压缩失败边界：桌面 Tauri release 测试已固定 CRF 12 和 CRF 38 编码回写后必须返回 `self_check_failed`，说明当前 staged 算法不能宣称抗有损二压。这仍不是可销售的 `cloud_video_processing`，不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许先在 `watermark-core` 内提高有损压缩存活率，再讨论平台二压矩阵和商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前多帧融合提取：`watermark-core` 新增 DCT 多帧 bitstream 多数投票恢复路径，能覆盖多帧分散 bit 损坏，但 CRF 12 / CRF 38 有损二压仍必须 `self_check_failed`。这仍不是可销售的 `cloud_video_processing`，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许继续在核心层提升真实有损压缩存活率，再评估平台二压矩阵和商业成本模型。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前中等有损存活边界：`watermark-core` 新增 sync marker 容错、DCT 写入强度常量和帧内 bitstream 重复副本，release 测试已固定 CRF 12 可通过自检、CRF 38 仍失败。这仍不是可销售的 `cloud_video_processing`，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许先补目标平台二压矩阵和成本模型，再讨论商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前目标平台二压矩阵首版：release 测试已固定 CRF 18 / CRF 23 存活、CRF 38 失败；该矩阵仍只证明 core / 测试层 staged 能力，不代表 `cloud_video_processing` 可销售，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许先在 core 内处理缩放后二压失败，再评估商业任务包装。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 384p 缩放后二压存活：`watermark-core` 低频 AC DCT pair 后，release 矩阵已固定 384p 缩放再回 512p 后 CRF 18 二压通过，CRF 38 仍失败。这仍不是可销售的 `cloud_video_processing`，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。512p 以下只保留为算法诊断小 fixture，不再作为商业主线继续发力。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前主战场分辨率矩阵：release 测试已覆盖 720p / 1080p / 2K，经 H.264 CRF 23 / CRF 28 二压后通过 core DCT 自检；三档中心裁切后补边再 CRF 23 二压也通过自检。当前商业主战场明确为 720p、1080p、2K。4K / 8K 暂作为未来大型商业片、院线产品或高阶商业产品线，不进入当前默认门禁、不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`。下一步只允许补主战场主流码率地板、平台 profile 和成本 / 性能预算。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前平台 profile 矩阵：release 测试已覆盖抖音 9:16 H.264 High CRF18 的 720p / 1080p、小红书 3:4 H.264 High CRF17 的 720p / 1080p、B站 16:9 H.264 High CRF18 的 720p / 1080p / 2K，并记录单 case 耗时约 4.0s 到 8.8s。这仍不是可销售的 `cloud_video_processing`，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补策略密度预算、平台矩阵耗时预算和成本模型。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前主流码率地板矩阵：release 测试已固定 720p H.264 2.5Mbps、1080p H.264 4.5Mbps、2K H.264 8Mbps 三档通过 core DCT 自检，单 case 耗时约 4.6s、6.1s、8.7s。低于主流地板的码率只记录风险边界和用户限制，不作为当前算法优化目标；不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补策略密度预算和平台矩阵成本模型。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 30 秒采样性能矩阵：测试已生成 30 秒 30fps 源视频并抽 12 帧，分段记录 FFmpeg 源生成 / 抽样、core 写入、采样帧码率回写、core 自检和总耗时；720p 2.5Mbps 约 19.1s、1080p 4.5Mbps 约 29.3s、2K 8Mbps 约 44.7s 通过。该结果依赖 12 个采样帧 / 96 个策略区域，仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补策略密度预算和平台矩阵成本模型。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 B站 HEVC 主流码率地板矩阵：新增测试会先探测 `libx265`，当前本机已实测 1080p HEVC 4Mbps 和 2K HEVC 6.5Mbps 在 30 秒 / 12 采样帧 / 96 策略区域口径下通过 core DCT 自检，confidence 均为 1.000；总耗时约 28.7s 和 44.3s。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补策略密度预算和平台矩阵成本模型。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 B站 H.264 / HEVC 成本对照矩阵：新增 `l3_bilibili_h264_hevc_cost_comparison_records_budget`，同一 30 秒 / 12 采样帧 / 96 策略区域口径下对照 1080p H.264 4.5Mbps、1080p HEVC 4Mbps、2K H.264 8Mbps、2K HEVC 6.5Mbps；本机实测总耗时约 27.5s、28.4s、42.9s、44.2s，confidence 分别为 0.917、1.000、0.750、1.000。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补策略密度预算，优先把 2K H.264 从压线状态提升到更稳定阈值。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 2K H.264 策略密度预算矩阵：新增 `l3_2k_h264_strategy_density_budget_records_confidence_curve`，同一 30 秒 / 12 采样帧 / 2K H.264 8Mbps 口径下对照 96 / 128 / 160 策略区域；本机实测总耗时约 43.4s、43.7s、43.0s，confidence 分别为 0.917、0.833、0.833。该结果说明单纯增加策略区域数不能稳定提升 2K H.264 置信度，仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补平台矩阵耗时预算，并转向抽帧数量 / 区域质量预算。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 2K H.264 抽帧数量预算矩阵：新增 `l3_2k_h264_sample_count_budget_records_confidence_curve`，同一 30 秒 / 2K H.264 8Mbps / 96 策略区域口径下对照 12 / 16 / 20 采样帧；本机实测总耗时约 43.5s、51.4s、59.4s，confidence 分别为 0.750、0.812、0.800。该结果说明 16 帧暂是 2K H.264 候选预算点，20 帧成本继续上升但置信度回落；仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补平台矩阵耗时预算，并评估区域质量预算。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 2K H.264 区域质量预算矩阵：`watermark-core` 新增区域选择模式 `SeededRandom`、`CenterSafeGrid`、`DistributedGrid`，默认仍为现有 `SeededRandom`；Tauri 新增 `l3_2k_h264_region_quality_budget_records_confidence_curve`，同一 30 秒 / 2K H.264 8Mbps / 16 采样帧 / 96 策略区域口径下对照三种区域质量策略。本机实测总耗时约 54.0s、51.9s、51.6s；seeded random 通过且 confidence 0.875，center safe grid 和 distributed grid 均 `self_check_failed`。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补平台矩阵耗时预算，区域质量后续转向内容感知 / 纹理感知候选。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前平台矩阵耗时预算：新增 `l3_platform_timing_budget_records_16frame_seeded_costs`，同一 30 秒 / 16 采样帧 / 96 策略区域 / seeded random 口径下覆盖抖音 1080x1920 H.264 4.5Mbps、小红书 1080x1440 H.264 6Mbps、B站 1920x1080 H.264 6Mbps 和 B站 2560x1440 H.264 8Mbps；本机实测总耗时约 33.5s、24.9s、33.5s、51.5s，confidence 分别为 0.812、0.875、1.000、0.938。4.5Mbps 仍是 1080p 主流地板，但平台候选预算采用 6Mbps 覆盖小红书 3:4 与 B站 1080p 稳定性风险。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许建立 L3 30 秒平台成本模型，并继续评估内容感知 / 纹理感知区域质量候选。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 30 秒平台成本模型：新增 `docs/Phase I-6 L3平台成本模型.md`，把平台耗时矩阵转成内部 `l3_cost_units`、`platform_weight` 和 `strategy_weight`；首版 1080p H.264 权重 1.25、2K H.264 权重 2.00、16 帧 / 96 区域 / seeded random 策略权重 1.00。该模型只用于容量规划、定价测算和套餐边界设计，不能进入 UI、后端账本或用户报告；仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接云端任务。下一步只允许在测试层评估内容感知 / 纹理感知区域质量候选，再决定是否调整成本权重。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 TextureAware 区域候选：`watermark-core` 新增 `VideoVisualTextureHint` 和 `TextureAware`，核心内完成高纹理 block 评分、候选排序和策略区域派生；2K H.264 区域质量矩阵新增 texture-aware case，本机实测 30 秒 / 16 帧 / 96 区域下 confidence 1.000、总耗时约 55.6s，优于 seeded random 的 0.875。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许把 TextureAware 扩展到 1080p / 2K 平台耗时矩阵，再决定是否调整 `strategy_weight`。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 TextureAware 完整平台矩阵：平台耗时测试新增 texture-aware 四档对照，抖音 1080p、小红书 1080p、B站 1080p、B站 2K 均通过且 confidence 1.000，总耗时约 33.0s、26.5s、33.9s、55.8s。TextureAware 提升稳定性但未提高平台成本，`strategy_weight` 暂定 1.00。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补 HEVC 对照矩阵，再评估是否切 staged 默认策略和调整成本权重。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 TextureAware HEVC 对照矩阵：新增 `l3_bilibili_hevc_texture_aware_records_cost_budget`，同一 30 秒 / 16 采样帧 / 96 策略区域 / TextureAware 口径下验证 B站 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps；本机实测两档均通过且 confidence 1.000，总耗时约 35.1s、57.7s。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许评估是否切 staged 默认策略，并补切换后的 H.264 / HEVC 回归矩阵。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前默认 TranscodeStable 策略切换回归矩阵：`watermark-core` 默认策略改为 720p 保留保守预算，1080p / 2K 默认 TranscodeStable；新增 `l3_default_transcode_stable_h264_hevc_regression_records_cost_budget`，真实 FFmpeg 覆盖 720p H.264 2.5Mbps / 12 帧、1080p H.264 6Mbps / 16 帧、2K H.264 8Mbps / 16 帧、1080p HEVC 4Mbps / 16 帧、2K HEVC 6.5Mbps / 16 帧；在 TranscodeStable 确定性取点收紧后五档均通过，confidence 均为 1.000。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许扩默认 TranscodeStable 真实内容二压样本和成本权重复核。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前默认策略真实素材多样性回归矩阵：新增 `l3_default_strategy_texture_diversity_records_cost_budget`，受控 FFmpeg 源覆盖 1080p 低纹理网格、1080p 高细节横屏、1080p 高细节竖屏和 2K 低纹理网格，四档真实 H.264 编码 / 解码后均通过，confidence 分别为 1.000、1.000、0.938、1.000，总耗时约 56.5s、43.1s、39.8s、80.5s。逐帧随机噪声和程序化高频纹理记录为风险边界。该结果仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补真实素材风险边界矩阵。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前默认 TranscodeStable 后真实素材风险边界矩阵：`l3_default_strategy_real_content_risk_boundary_records_outcomes` 中低码率竖屏高细节 H.264 4.5Mbps 通过但 confidence 0.875；极端程序化高频纹理和逐帧随机噪声均稳定返回 `self_check_failed`。该结果明确低码率竖屏高细节不能作为默认商业预算，极端纹理不能包装成当前可售能力；仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许补默认 TranscodeStable 平台二压回归。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前平台二压风险矩阵：新增 `l3_platform_second_pass_transcode_risk_records_outcomes`，真实 FFmpeg 覆盖 1080p 竖屏高细节 6Mbps -> 4.5Mbps 与 2K 8Mbps -> 6.5Mbps 两个二压场景；1080p 高细节稳定 `self_check_failed`，2K 压线通过 `passed:0.750`。该结果说明 L3 仍未达到可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许在 `watermark-core` / 测试层设计二压稳定性改进。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前平台二压稳定性诊断矩阵：新增 `l3_platform_second_pass_stability_diagnostics_records_budget_curve`，1080p 竖屏高细节加帧或加区域仍失败；新增 `TranscodeStable` 核心区域模式后，1080p 16 帧 / 96 区域恢复到 `passed:0.812`；2K 20 帧 / 96 区域提升到 `passed:0.950`，但总耗时约 77.3s。该结果说明二压稳定性优先来自核心区域候选质量，仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步扩展 `TranscodeStable` 平台矩阵并复核 2K 20 帧成本权重。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 TranscodeStable 平台泛化矩阵：新增 `l3_transcode_stable_second_pass_platform_matrix_records_generalization`，同一 30 秒 / 16 帧 / 96 区域口径下固定 720p 真实二压失败边界，并覆盖 1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压；720p H.264 4Mbps -> 3Mbps 仍为 `self_check_failed`，其余四档通过；在稳定候选确定性取点收紧后 confidence 分别为 1.000、0.875、1.000、1.000。该结果已支撑 `TranscodeStable` 进入 1080p / 2K staged 默认路径，但仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步扩默认 TranscodeStable 真实内容二压样本，720p 二压保持风险边界。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前默认 TranscodeStable 平台二压成本权重复核：新增 `l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight`，直接走 core default 路径覆盖 720p、1080p、2K 和 HEVC 二压。首次运行发现 1080p H.264 因 TranscodeStable 仍受 task_id / seed 抽样漂移影响而失败；随后 `watermark-core` 收紧为稳定候选确定性取点。重跑后 720p 仍为 `self_check_failed`，1080p H.264、2K H.264、1080p HEVC、2K HEVC confidence 分别为 1.000、0.875、1.000、1.000；`strategy_weight` 暂不高于 1.00，二压总耗时进入平台权重复核。仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步扩默认 TranscodeStable 真实内容二压样本。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前默认 TranscodeStable 真实内容二压矩阵：新增 `l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes`，同一 30 秒 / 16 帧 / 96 区域 / core default 口径覆盖 1080p 高细节横屏、1080p 高细节竖屏、2K 常规纹理和 2K 高细节 H.264；1080p 两档均 `passed:1.000`，2K 常规纹理 `passed:0.875`，2K 高细节稳定 `failed:self_check_failed`。该结果明确 2K 高细节不能进入当前默认商业承诺；仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许评估 2K 高细节 H.264 二压预算策略。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 2K 高细节 H.264 二压预算策略矩阵：新增 `l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes`，同一 30 秒 / 2K 高细节 H.264 源下对照 20 帧 / 96 区域、16 帧 / 128 区域和 10Mbps -> 8Mbps；加帧和加区域两档仍 `self_check_failed`，提高码率到 10Mbps -> 8Mbps 后通过但 confidence 0.875。该结果说明 2K 高细节当前应走码率预算分档和样本扩展，而不是把采样密度包装成商业 SLA；仍不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许扩展 2K 高细节高码率候选样本，覆盖横屏高细节、低纹理、运动纹理和 HEVC 对照。 | 进行中 |
| 2026-06-21 | 配合 L3 商业化实现前 2K 高码率内容候选矩阵：新增 `l3_2k_high_bitrate_content_candidate_matrix_records_outcomes`，同一 30 秒 / 16 帧 / 96 区域 / core default 口径覆盖 H.264 高细节、H.264 低纹理、H.264 运动纹理和 HEVC 高细节；H.264 高细节 10Mbps -> 8Mbps 通过但 confidence 0.875，H.264 低纹理和运动纹理均 `passed:1.000`，HEVC 高细节 8Mbps -> 6.5Mbps `passed:1.000`。该结果支持进入 2K 高码率 release 样本池设计，但仍不是可销售 SLA，不开放 Studio / Enterprise 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许设计 2K 高码率 release 样本池、最低 confidence 要求和失败归因。 | 进行中 |
| 2026-06-21 | 新增 `docs/当前真实能力边界说明.md` 作为产品、销售、研发共用边界口径：当前可对用户承诺的是图片 / 音频同核写入验证、双端版权库字段、移动端保护副本出口、L1 视频音轨水印和 L2 视频指纹存证；L3 视频画面盲水印、2K 高码率候选和成本模型仍只能内部测试，不能包装成 Studio / Enterprise 可售 SLA。后续任何商业化能力表述必须同步回写该文档。 | 已完成 |
| 2026-06-21 | 完成 2K 高码率 release 样本池与阈值策略冻结：新增 `docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md`，固定 H.264 10Mbps -> 8Mbps、HEVC 8Mbps -> 6.5Mbps、24 个 2K 样本池、H.264 非风险样本最低 confidence 0.950、HEVC 非风险样本最低 confidence 0.970 和禁止商业包装门槛。该策略仍不是可销售 SLA，不开放 Studio / Enterprise L3 任务、不上传用户视频、不扣 `video_minutes`。下一步只允许新增 release 门禁测试。 | 已完成 |
| 2026-06-22 | 完成 2K 高码率 release 样本池门禁：新增 `l3_2k_high_bitrate_release_sample_pool_records_thresholds`，默认 smoke 每组 1 个代表样本，完整 24 样本池通过 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1` 显式长跑。本机默认 smoke 继续由 H.264 高细节 confidence 0.875 / `confidence_below_threshold` 阻断 release；仍不是可销售 SLA，不开放 Studio / Enterprise L3 任务、不上传用户视频、不扣 `video_minutes`、不接 UI 或云端任务。下一步只允许长跑完整 24 样本池并回写证据。 | 已完成 |
| 2026-06-22 | 商业发布主线切换：新增 `docs/双端现有能力发布计划.md`，短期不再继续 L3 完整样本池长跑、UI、云任务或扣费设计；本版发布聚焦现有可承诺能力接入双端并完成自动化门禁、运行态验收、微信支付真实联调阻断确认和法务审阅阻断确认。 | 进行中 |
| 2026-06-24 | 封版前补强架构门禁：`watermark:architecture-contract` 固定后端不得新增 `watermark-core` 外盲水印算法，Web preview 不得进入 vault / sync / report，也不得展示正式 `HS-...` 版权编号暗示；L3 视频画面盲水印继续挂起，不进入 UI、云任务、账本扣费、订阅权益或销售话术。下一步进入发布候选自动化门禁和运行态验收。 | 进行中 |
| 2026-06-25 | 冻结 Free 单份报告付费设计：Free 继续免费查看版权库和复制基础摘要，单份版权详细报告定价 19.9 元 / 份，维权证据包定价 49.9 元 / 份；该能力作为 Phase 8 一次性商品扩展推进，后续必须新增 report purchase session、商品 allowlist、授权核销、退款撤销和双端购买入口，不能直接复用 Creator 订阅 `report_export`。 | 进行中 |
| 2026-06-25 | 落地 Free 单份报告付费后端 fixture 闭环：新增 `report_purchase_sessions` 和 `report_purchase_grants`，开放 `/v1/billing/report-purchase-sessions` 创建 / 查询 / reconcile API；fixture 支付成功后只对对应 `vault_record_id` 写入授权，不改变 Free entitlement，也不打开 `report_export`。验证覆盖 19.9 元单份版权详细报告、49.9 元维权证据包、非法商品拒绝和重复确认幂等。下一步接入桌面端版权库购买入口。 | 进行中 |
| 2026-06-25 | 接入桌面端 Free 单份报告付费入口：桌面端版权库已为 Free 用户展示“购买版权详细报告 / 购买维权证据包”入口；Tauri 本地新增 `report_purchase_grants` 授权表，正式报告导出支持 Creator `report_export` 或单记录有效授权二选一通过。已通过 `npm run report:contract`、`npm run billing:contract`、`npm run commercial:contract`、`npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml --lib commands::report::tests`、`cargo test --manifest-path src-tauri/Cargo.toml --lib db::billing::tests`。下一步接入移动端版权库记录详情购买入口。 | 进行中 |
| 2026-06-25 | 接入移动端 Free 单份报告付费入口：移动端版权库记录详情已展示“购买版权详细报告 / 购买维权证据包”入口，接入 report purchase session 创建、查询、reconcile 和 `reportPurchaseGrantsJson` 本地授权持久化；移动端正式报告导出支持 Creator `report_export` 或当前记录有效单份授权二选一通过。已通过 `flutter test test/mobile_app_state_test.dart`，并补强 `report:contract` / `billing:contract` 的移动端检查。下一步接入真实微信一次性商品支付和退款撤销。 | 进行中 |
| 2026-06-25 | 接入 Free 单份报告真实微信一次性商品后端核心：`preferredProvider=wechat_pay` 可创建 report purchase 微信 Native 订单，微信 attach 使用 `purchaseType=report_purchase` 与记录 / 商品字段分流；查单 / webhook 成功只写 `report_purchase_grants`，退款 / 撤销将授权置为 `revoked`，不改变 Free entitlement，也不打开 Creator `report_export`。已通过 `cargo test --manifest-path feedback-backend/Cargo.toml --lib` 并补强 `billing:contract`。下一步执行真实微信商户联调、真实支付回调验收、退款撤销运行态验收和双端付费 QA。 | 进行中 |
| 2026-06-25 | 补齐微信一次性商品真实联调准备：新增 `docs/Phase 8 微信一次性商品联调Checklist.md`，固化商户参数、后端环境变量、`/v1/billing/webhooks/wechat-pay` 公网回调、19.9 元 / 49.9 元商品下单验收、支付成功授权、查单补偿、退款撤销、双端运行态 QA、日志留存和上线阻断项；`billing:contract` 已纳入该 checklist。下一步准备真实商户参数和公网 HTTPS 回调环境，启动真实下单联调。 | 进行中 |
| 2026-06-25 | 修正商业化封版边界：Free 单份报告付费明确纳入本版封版范围，商品固定为 19.9 元单份版权详细报告和 49.9 元维权证据包；不新增其他支付商品。真实微信商户参数、公网 HTTPS 回调、真实下单、回调、查单、退款撤销和双端授权互认属于上线验收项，未配置时产品展示支付通道未完成配置，而不是把能力降级为未来功能。下一步先复跑商业化自动化门禁，再准备真实微信商户参数和公网 HTTPS 回调环境。 | 进行中 |
| 2026-06-25 | 完成封版商业化自动化复跑与口径收口：`npm run commercial:ci` 通过，覆盖商业合同、指标、双端一致性、billing、usage、report、team、watermark architecture、video phase、cross-end release、桌面构建、后端测试、Tauri release-scope 测试、Flutter analyze / test、cloud:ci 和 cloud-video:ci。Free 单份报告付费、Studio 团队预留、L3 冻结和移动端 L2 视频指纹存证只读口径已同步到 UI、README、法务草案和历史归档声明。下一步准备真实微信商户参数、公网 HTTPS 回调和法务审阅，完成 19.9 元 / 49.9 元一次性商品真实联调。 | 进行中 |
| 2026-06-25 | 完成 Windows NSIS 候选安装包：`HiddenShield_0.1.0_x64-setup.exe` 已生成并通过 silent install smoke，SHA256 `C42A683FC9100F166441A19A90D440A92A541669D02C92BE5271DB3A4CA70A11`；同时将 `video_fingerprint_spike` 研发工具迁出 Tauri 正式应用 crate，避免内部 spike 二进制进入用户安装包。MSI 仍受 WiX 下载阻断，当前 Windows 交付以未签名 NSIS 安装器和 release exe 为候选产物。下一步执行安装版桌面端完整交互验收。 | 进行中 |
| 2026-06-26 | 配合双端视觉语言迁移收口商业入口：桌面端工作台 L2 视频指纹存证的 Creator 门禁按钮已接通订阅与权益面板；桌面和移动端将用户入口统一为“批量队列”，设置 / 订阅中的 Free / Creator / Studio / Enterprise 口径保持一致；Free 单份报告购买入口仍保留在版权库记录层，不改变 `report_export` 权益规则。验证：`npm run build`、`flutter analyze` 通过。下一步继续核对报告购买 / 导出、订阅支付中 / 已解锁 / 未配置支付通道等状态截图。 | 进行中 |
| 2026-06-26 | 配合双端视觉语言迁移继续收口商业化可见层：桌面端订阅与权益面板、设置页商业指标 / 云同步 / 反馈入口、帮助中心、隐私授权弹窗、身份初始化和版权库相关报告入口统一到 Stitch 深色 token；移动端设置同步健康卡继续使用 Free / Creator / Studio / Enterprise 共享口径。Free 单份版权详细报告 / 维权证据包入口和授权核销规则未改变，Creator `report_export`、本地批量和云同步门禁未改变，未新增 L3 视频画面盲水印或云端视频商业承诺。验证：`npm run build`、`flutter analyze` 通过；产品代码模板词、emoji、移动端白色硬编码扫描通过；桌面端 Playwright 截图覆盖订阅与权益、设置、帮助、版权库和验证入口。下一步做商业化运行态状态截图 QA，重点覆盖 Free 未购买、Free 已购买授权、Creator 订阅、支付通道未配置和退款撤销后的报告导出状态。 | 进行中 |
| 2026-06-26 | 完成移动端商业状态截图 QA：新增 `mobile_app/tool/mobile_visual_qa.dart` 作为 QA-only 运行态入口，使用正式移动端 Stitch token 展示版权库详情、报告导出、购买入口和支付状态；截图覆盖 Free 未购买单份报告、Free 已购买当前记录授权、Creator 订阅生效、支付通道未配置、退款撤销后授权失效。QA 修正可导出场景按钮状态，确保 Free 已购买授权和 Creator 订阅显示可导出，支付未配置不允许真实付款，退款撤销不改变 Free 订阅状态。截图证据位于 `tmp-ui-qa/mobile/free-unpaid-full.png`、`free-paid-full.png`、`creator-full.png`、`payment-unconfigured-full.png`、`refund-revoked-full.png`。验证：`flutter analyze`、`flutter build web -t tool/mobile_visual_qa.dart`、`npm run build`、模板词 / emoji / 移动端硬编码视觉扫描通过。下一步把相同五态接入正式移动端版权库详情真实数据路径，并在配置真实微信商户参数后执行 19.9 元 / 49.9 元真实下单、回调、查单和退款撤销验收。 | 进行中 |
| 2026-06-26 | 复核移动端商业状态截图 QA 并修复 Web 字体证据：新增 `HiddenShieldCjk` 子集字体和主题 fallback，避免 QA Web / 截图环境中文缺字；商业 QA 样张编号改为 `PREVIEW-QA-MOBILE-20260626`，不再显示成正式 `HS-...` 版权编号。重新覆盖 Free 未购买、Free 已购买授权、Creator 订阅、支付通道未配置、退款撤销五态截图，确认 Free 未购买 / 退款撤销显示购买或升级后导出，Free 已购买和 Creator 显示可导出，支付通道未配置显示明确阻断且不允许真实付款。验证：`npm run commercial:contract`、`npm run report:contract`、`npm run build`、`flutter analyze`、`flutter build web -t tool/mobile_visual_qa.dart --pwa-strategy=none`、商业状态截图像素检查和控制台错误检查均通过。真实微信商户参数、公网 HTTPS 回调、19.9 元 / 49.9 元真实下单、查单、webhook 和退款撤销运行态验收仍是收费上线阻断项。 | 进行中 |
| 2026-06-26 | 配合 Stitch 信息架构迁移重构商业入口：桌面端把“订阅与权益”提升为一级导航并支持嵌入主舞台，不再只作为旧侧栏 modal；右侧上下文面板展示当前方案、报告权益、云同步、支付通道未配置和退款撤销授权边界。移动端保留设置/门禁进入订阅与权益的产品模型，并通过 ContextSheet 展示报告授权、Creator 权益和支付未配置状态说明。本次未改变 Free 单份报告购买、Creator `report_export`、批量、云同步或支付规则，未新增 L3 视频商业承诺。验证：`npm run commercial:contract`、`npm run report:contract`、`npm run build`、`flutter analyze`、两条移动 Web QA 构建通过。下一步在 Tauri 安装版和移动真机中补 Free 未购买、Free 已购买授权、Creator、支付通道未配置、退款撤销五态的新版 IA 截图。 | 进行中 |
| 2026-06-26 | 配合处理页第一性原则迁移收口商业化字段边界：处理页不再承载平台输出、画幅适配、裁剪 / 黑边和编码模式，商业化可售口径回到保护副本、版权记录、正式报告、云同步和授权声明；Free 单份报告、Creator 正式报告和云同步都从版权库记录读取保护副本名称 / 摘要、输出策略和作品声明字段。`protectedCopyPath` 不进入报告、同步或商业指标；训练许可和作品来源仍是用户声明，不作为 AI 检测、真实性鉴定或法律授权结论售卖。本次未新增支付商品、套餐权益或 L3 视频画面盲水印承诺。验证：`commercial:contract`、`report:contract`、`process:first-principles-contract`、`npm run build`、`cargo check --manifest-path src-tauri\\Cargo.toml`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart`、`flutter test test/widget_test.dart` 通过。下一步在真实微信商户参数准备前，先用 fixture 继续验收 Free 单份报告五态和新版字段报告展示。 | 已完成 |
| 2026-06-26 | 新增版权编号登记商业化方向：正式编号在线默认由后端签发 / 确认唯一，作为客户端与账号、云同步、正式报告、团队版权库和未来维权服务的连接点；离线或后端不可用时仍允许本地高熵编号写入，联网后补登记。历史重复编号进入重新签发和保护副本修复队列，不只提示旧模型。下一步实现 `watermark_id_registry`、签发 / 确认 / reconcile API 和双端编号登记状态。 | 已确认，待实施 |
| 2026-06-27 | 完成版权编号登记商业化主链路：后端 `watermark-ids` API 已被桌面端和移动端图片 / 音频写入流水线在线优先调用，后端返回的编号和 proof hash 进入 V2 payload；写入成功后 `confirm` 登记原作品摘要、保护副本摘要和写入后验证状态；后端不可用时仍保持离线写入，云同步发送前自动 `confirm / reconcile` 并回写 registry receipt。该能力为未来账号版权登记、正式报告、团队版权库和维权服务建立了客户端到后端的连接点。验证：`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml --tests`、`flutter analyze`、`flutter test`、`cargo test --manifest-path mobile_app/rust/Cargo.toml --lib` 通过。下一步用真实后端环境补 Free / Creator 两种账号下的写入、离线、同步补登记运行态 QA，并把登记状态纳入正式报告购买五态截图。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 完成版权编号登记商业化闭环补强：同 UID 不同作品哈希进入 `pending_registry_reconcile` 登记仲裁，双端版权库提供历史重复编号重新签发入口；桌面端保护副本可访问时会调用后端 `reissue` 并用共享核心重写 V2 payload、回读验证替换编号和旧编号父链、更新保护副本摘要，移动端创建重签任务并等待重新选择文件完成修复。该入口把本地版权库、后端登记库和未来团队版权库 / 维权服务的编号修复流程连起来。验证：Tauri check / sync storage tests / reissue payload parent 单测 / Flutter analyze + tests / npm build / dual contract / backend watermark-id tests / Rust fmt checks 均通过。下一步补 iOS 真机运行态 QA 与桌面安装包完整交互回归。 | 已完成 |
| 2026-06-27 | 完成真实后端版权登记状态运行态 QA：桌面端与原生 Android 在同一真实后端下分别完成图片 / 音频在线 `server_confirmed`、离线 `pending_registration`、同步补登记 `offline_confirmed` 截图验收，证明商业化登记主链路已可连接账号、版权库、报告和同步 payload。证据文件：`tmp-ui-qa/real-runtime-status/real-runtime-status-qa-1782541584143.md`。本次未新增付费商品、套餐权益或 L3 视频画面盲水印承诺；真实收费上线仍受微信支付真实联调、退款撤销运行态和法务审阅阻断。下一步把登记状态并入 Free 单份报告五态的真实数据路径截图，并继续准备微信真实商户联调。 | 已完成 |
| 2026-06-27 | 完成用户注册与登录体系规划：新增 `docs/用户体系与登录注册体系规划.md`，把研发期 `auth/continue` 升级路径规划为正式 `auth/challenges -> auth/sessions -> auth/refresh -> auth/logout -> me` 体系，并明确账户、工作区、设备、创作者档案、会话、权益和版权 ID 服务的绑定关系。规划要求后端版权 ID `reserve / confirm / reconcile / reissue` 必须绑定认证后的 `accountId + workspaceId + deviceId + creatorProfileId`，未登录仍保持本地高熵编号写入，登录后补登记。该条为规划记录，正式 Auth API 已在后续 2026-06-27 记录中完成；下一步迁移双端正式验证码 / 密码登录 UI。 | 已规划，后续已实施 |
| 2026-06-27 | 实施 Creator 默认自动云同步主链路：后端 `auth/continue` 返回 `syncPolicy=blocked_by_entitlement / auto_cloud_vault` 和 `cloudVaultCursor`，Free 同步 push / pull 被后端 403 阻断；桌面端继续账户后自动 pull 云端、flush 本地版权库队列、再次 pull；移动端登录或权益升级为 Creator 后执行同一自动 pull / flush / pull，并把 `syncPolicy` 持久化到本地 profile。云同步合同脚本已覆盖 Free 阻断、fixture 升级 Creator、Creator push / pull；移动端测试覆盖 Creator 登录自动拉取并同步待队列。仍默认不同步原始媒体、保护副本文件、本地路径和 creator seed 明文。验证：`cargo test --manifest-path feedback-backend/Cargo.toml --lib`、`cargo test --manifest-path src-tauri/Cargo.toml --lib commands::sync::tests`、`flutter test test/sync_transport_test.dart test/mobile_app_state_test.dart`、`npm run cloud:contract`、`flutter analyze`、`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path feedback-backend/Cargo.toml` 通过。下一步补真实桌面端 + 原生移动端同账号自动同步、暂停、恢复截图 QA。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 完成 Creator 自动云同步暂停 / 恢复偏好：后端新增 `PATCH /v1/me/sync-preferences`，偏好落到当前 `cloud_devices.auto_sync_enabled`；Creator / Studio / Enterprise 可在 `auto_cloud_vault` 与 `manual_local_only` 间切换，Free 恢复自动同步返回 403。桌面端设置页新增自动同步状态与暂停 / 恢复入口，恢复后执行 pull / flush / pull；移动端设置开关改为调用服务端偏好并持久化 `syncPolicy`，权益刷新不会静默覆盖用户暂停。暂停不删除云端版权库、本地版权库或本地队列，手动同步仍按 `cloud_sync=true` 权益允许。验证：后端 lib tests、Tauri sync tests、Flutter sync/state tests、临时真实后端 `npm run cloud:contract`、`flutter analyze`、双端 cargo check 通过。下一步补桌面安装版 + 原生移动端同账号自动同步、暂停、恢复截图 QA，并继续正式 Auth API。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 完成 Creator 自动云同步暂停 / 恢复运行态 QA：新增 `cloud:auto-sync-runtime-qa`，在临时真实后端下验证同一 Creator 账户桌面端与移动端默认自动同步、移动端暂停为 `manual_local_only`、暂停期间手动同步仍保留、恢复为 `auto_cloud_vault` 后继续自动拉取。证据文件：`tmp-ui-qa/auto-cloud-sync/auto-cloud-sync-runtime-qa-1782566485043.md`，桌面 / 移动截图同目录。该 QA 不上传原始媒体、保护副本文件、本地路径或 creator seed 明文。下一步继续正式 Auth API 与真实安装版 / 真机人工交互复核。 | 已完成 |


| 2026-06-27 | 完成正式 Auth API 商业化主链路：后端新增 `auth/challenges -> auth/sessions -> auth/refresh -> auth/logout -> me`，桌面端和移动端继续账户底层切到 `/v1/auth/sessions`，`auth/continue` 降级为兼容 alias；Creator 升级后 `auth/sessions` / `me` / refresh 均返回 `auto_cloud_vault`，用户暂停后保持 `manual_local_only`。新增 `auth:contract` 覆盖 challenge 登录、密码登录、token 轮换、logout、Creator 权益和暂停同步状态；`cloud:ci` 已切到正式 sessions 并验证 Free 阻断、Creator 双端 push / pull。当前仍不把 fixture 验证码、salted SHA-256 密码 hash 和未限流登录包装为生产安全完成态；下一步进入桌面 / 移动正式验证码登录 UI 与生产安全强化。 | 已完成 |
| 2026-06-27 | 完成 Phase U-2 / U-3 登录注册体验与认证安全基础：后端新密码写入切到 Argon2id，旧 salted SHA-256 仅在成功登录后迁移；`auth/challenges` 支持动态验证码与 `HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT` webhook 投递，未配置时保留 fixture 供本地 / 合同测试；验证码发送限流和登录失败限流已接入。桌面端与移动端设置页、首次引导页均升级为验证码 / 密码登录 UI，保留未登录本地使用入口；Creator 登录后自动云同步语义不变。验证：后端 lib tests、Tauri check / sync tests、Flutter analyze / state tests、`npm run build`、`auth:contract`、`cloud:ci` 通过。下一步进入设备管理 / 会话撤销 UI、真实短信 / 邮件供应商联调和第三方登录准备。 | 已完成 |
| 2026-06-27 | 完成 Phase U-4 账户设备管理与 OTP webhook 联调：后端新增账户设备列表、设备重命名、撤销其他设备会话 API，桌面端和移动端设置页均新增“设备与会话”面板；撤销设备会关闭该设备 session，但不删除本地版权库、本地队列或云端版权库。`auth:contract` 新增本地 OTP webhook server，配置 `HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT` 后验证验证码不返回 `fixtureCode`，且使用 webhook 收到的动态验证码完成登录；合同同时覆盖设备列表、重命名、拒绝撤销当前设备和撤销其他设备后旧 token 失效。本次未新增付费商品或套餐权益，只增强 Creator 云同步和版权登记所需账户可信边界。验证：后端 lib tests、Tauri check / sync tests、Flutter analyze / state tests、`npm run build`、`auth:contract`、`cloud:ci` 通过。下一步接入真实短信 / 邮件供应商生产参数、模板审核、送达率告警，并补安装版桌面端 + Android / iOS 真机设备撤销截图 QA。 | 已完成 |
| 2026-06-28 | 完成设备撤销商业化账户可信边界运行态 QA：真实后端 + Windows 桌面端 + Android 模拟器覆盖设备列表、重命名、撤销另一端设备、被撤销端自动同步失败和重新登录恢复入口；移动端恢复后失效 token 被清除，保留账号标识进入正式验证码 / 密码登录页，避免被撤销设备继续以 Creator 云同步会话身份运行。新增 `docs/Auth OTP短信邮件供应商生产接入Checklist.md`，明确真实短信 / 邮件生产接入仍阻断于供应商选择、生产凭证、短信签名、邮件域名、模板 ID、模板审核、delivery 签名和送达告警。本次未新增套餐权益、收费商品或真实短信 / 邮件承诺。验证：`flutter analyze`、`flutter build apk --debug -t lib/main.dart --target-platform android-x64`、`npm run build`、`cargo check --manifest-path src-tauri/Cargo.toml`。下一步提供真实短信 / 邮件供应商参数与审核模板后，实施 delivery 签名和生产联调。 | 运行态 QA 已完成，生产 OTP 待参数 |
| 2026-06-28 | 修复版权编号登记主链路的桌面运行态阻塞问题：桌面端图片 / 音频写入在线优先调用 `watermark-ids reserve -> confirm` 时，原实现直接在 async pipeline 中创建 `reqwest::blocking::Client`，真实后端 profile 存在时会触发 Tokio runtime drop panic，表现为用户只看到写入进度但没有稳定完成 / 验证结果。现已将 reserve / confirm blocking HTTP 调用移入 `spawn_blocking`，并通过真实桌面进程写入图片和 35 秒 WAV 音频确认 `watermarkIdIssueMode=server_confirmed`、`watermarkIdRegistryStatus=server_confirmed`、V2/119 payload 和本机版权库匹配验证均成功；WAV 验证路径同时改为直读共享核心，避免转码破坏音频水印。本次不新增套餐权益、付费商品或 L3 视频承诺。下一步在安装版桌面端复跑图片 / 音频登记状态与报告字段展示，确保 Free / Creator 商业入口读取到一致的登记状态。 | 已完成 |
| 2026-06-28 | 补齐移动端版权登记字段的验证侧商业闭环：移动端从正式保护副本读取时已保留 V2 protocol、payload bytes、编号签发模式、父编号、版本次数和认证状态，并把这些字段写入验证记录、版权库详情和同步 payload，避免 Creator 云同步或未来团队版权库只收到 UID 而缺少登记语义。本次不新增套餐权益、收费商品、真实短信 / 邮件承诺或 L3 视频画面水印承诺。下一步在真实 Android / iOS 端用已登记保护副本复核 Free / Creator 版权库详情和正式报告草稿展示。 | 已完成 |
| 2026-06-28 | 完成移动端版权登记字段运行态证据补强：Android 模拟器通过 `npm run dual:protected-copy-file-flow-qa` 复跑真实保护副本文件流转，desktop->mobile 与 mobile->desktop 图片 / 音频均读回同一版权编号和 V2/119 payload；移动端 QA JSON 明确保存 `revision`、签发模式、认证状态和媒体类型，避免 Creator 云同步、团队版权库或未来维权服务只拿到裸 UID。证据文件：`tmp-ui-qa/protected-copy-file-flow/1782624239136/protected-copy-file-flow-qa-1782624239136.md`。本次不新增套餐权益、收费商品、真实短信 / 邮件承诺或 L3 视频画面水印承诺。下一步补 iOS 真机运行态 QA，并在 Free / Creator 报告真实数据路径里复核这些登记字段展示。 | Android 已完成，iOS 待补 |
| 2026-06-28 | 新增公开权利信号与训练许可扫描协议设计：`docs/公开权利信号与训练许可扫描协议设计.md` 固化“不修改 V2 payload”的商业化前置约束，后续企业训练许可扫描、批量 API 或公开查询服务应使用 `watermarkUid` 查询 rights registry，并通过 C2PA / CAWG / IPTC / XMP / JSON-LD 公开元数据传播完整授权声明；V2 仅作为跨端可验证锚点。本次不新增 `api_access` 权益、不开放企业批量扫描、不新增收费商品，也不把训练许可声明包装为法律授权结论。下一步先评审协议和 `rights_manifests` 数据模型，再决定是否纳入 Studio / Enterprise 后续商业阶段。 | 设计已完成，待评审 |
| 2026-06-28 | 补充公开权利协议的长期瘦身方向：`docs/公开权利信号与训练许可扫描协议设计.md` 现已补充字段分层表和 V3 瘦身路线，目标是在最小可用锚点前提下追求性能与鲁棒性的最佳性价比，把版本链 / 签发 / 权利语义迁到 registry 与公开元数据层；这是长期架构设计，不是当前可售能力，也不改变现有套餐、权益或 API 边界。下一步仅评审 `rights_manifests` 与 V3 迁移策略，不进入商业化实现。 | 设计已完成，待评审 |
| 2026-06-29 | 收紧公开权利协议的正式迁移草案：`docs/公开权利信号与训练许可扫描协议设计.md` 进一步冻结 V3 三段式迁移顺序，明确 `watermark_id` / `auth_tag` / `payloadProtocolVersion` 作为 S0 媒体锚点，版本链与登记证据走 S1，声明语义与审计语义走 S2；`可迁` 和 `必迁出` 均指向版权库 / 云版权库 / registry / 公开元数据层，不是删除字段。验证：文档回写完成，未触及运行时代码。下一步评审 `rights_manifests` 数据模型与公开扫描输出结构，决定是否进入协议冻结。 | 进行中 |
| 2026-06-29 | 继续收口公开权利扫描契约：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充 `rights_manifests` 三段式字段分组、最小状态集、`GET /v1/public/rights/{watermarkUid}` / `POST /v1/public/rights/batch` 的返回骨架，以及 `policyResolution` 冲突解释口径；仍只做协议设计，不开放批量 API，也不新增 `api_access` 权益。下一步把这份草案和后端数据模型、公开扫描 SDK、UI 文案一起评审后再定稿。 | 进行中 |
| 2026-06-29 | 继续冻结公开权利协议草案：`docs/公开权利信号与训练许可扫描协议设计.md` 已增加“协议冻结草案”小节，明确 `rights_manifests` 最小契约、扫描结果最小契约、文案冻结原则和待定项；这仍是设计冻结，不是接口上线。下一步按同一冻结口径评审数据库 schema、公开扫描 SDK 和产品文案。 | 进行中 |
| 2026-06-29 | 继续压实公开权利协议后端草案：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充 `rights_manifests` 独立表 schema 草案、唯一约束、索引和查询约定，并明确不与 `watermark_id_registry` 混表。下一步按该 schema 草案评审数据库迁移和 API 草图。 | 进行中 |
| 2026-06-29 | 继续补齐公开权利协议迁移路径：`docs/公开权利信号与训练许可扫描协议设计.md` 已加入 `rights_manifests` 旧记录回填草案，明确先回填 registry 事实源，再回填版权库声明，最后生成 `active` 版本，并保留 `disputed` 和人工替代路径。下一步评审回填 job 的批次策略和失败回退。 | 进行中 |
| 2026-06-29 | 完成公开权利信号第一阶段实现：后端新增 `rights_manifests`、公开只读 `GET /v1/public/rights/{watermarkUid}`、批量只读 `POST /v1/public/rights/batch` 和管理员保护的内部 backfill；云同步版权库声明会生成 active manifest，桌面端与移动端版权库 / 验证页可展示 registry 训练许可快照、锚点协议、manifest 状态和“非法律结论”边界。V3 当前仅作为 registry / 迁移桥语义接受 `33..64 bytes` 最小锚点登记，不修改 `watermark-core` V2 payload，不新增 `api_access` 权益，不开放 Studio / Enterprise 企业批量训练许可商品。验证：后端公开查询 / V3 / backfill 测试、`cargo check --manifest-path feedback-backend/Cargo.toml`、`npm run build`、`flutter analyze` 通过。下一步按 Roadmap 评审是否把公开扫描 SDK 与企业批量 API 纳入 Studio / Enterprise 阶段，先不要直接售卖。 | 阶段性完成 |
| 2026-06-29 | 继续细化公开权利协议回填策略：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充后台回填 job 的批次粒度、失败重试、幂等和 `disputed` 处理规则，防止迁移时把 registry 压住或静默覆盖冲突记录。下一步据此评审迁移脚本与后台队列实现。 | 进行中 |
| 2026-06-29 | 继续压实公开权利协议接口草图：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充公开查询、批量查询和迁移回填 job 三条最小 API 路径草图，明确公开查询只读、批量查询只返回公开结构、回填 job 作为内部命令或后台接口。下一步按这三条路径评审后端实现分工。 | 进行中 |
| 2026-06-29 | 继续补强公开权利扫描 SDK 草图：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充 `scanOne`、`scanBatch`、`resolvePolicy`、`formatUserMessage` 方法草图，以及错误码与用户文案映射原则，确保 SDK 不再发明第二套结构。下一步按这个 SDK 口径评审前端和后台调用方。 | 进行中 |
| 2026-06-29 | 继续压实公开权利协议实现分工：`docs/公开权利信号与训练许可扫描协议设计.md` 已补充后端 A / 后端 B / SDK / 桌面端前端 / 移动端前端 / 文档产品的责任边界草案，并明确“前端（桌面端 + 移动端）”的双端职责，避免后续实现重复劳动。下一步按这个分工评审任务切分。 | 进行中 |
| 2026-06-29 | 继续补齐公开权利协议实现顺序：`docs/公开权利信号与训练许可扫描协议设计.md` 已加入“先回填事实源、再开放公开查询、最后接入桌面端 / 移动端前端”的实现顺序草案，避免前端先于后端事实源消费不稳定数据。下一步按顺序拆解任务依赖。 | 进行中 |
| 2026-06-29 | 完成公开权利信号真实后端运行态 QA 与公开元数据 sidecar 导出契约：`rights:runtime-qa` 在真实 `feedback-backend` 下覆盖桌面端和 Android 原生端图片 / 音频各一条带训练许可声明的记录，公开 rights registry 查询均为 `registry_active` 且 `legalConclusion=false`，证据文件为 `tmp-ui-qa/public-rights-runtime/1782707328008/public-rights-runtime-qa-1782707328008.md`；后端新增 `GET /v1/public/rights/{watermarkUid}/metadata`，输出 C2PA / CAWG Training and Data Mining、IPTC / PLUS Data Mining、XMP、JSON-LD 的 sidecar JSON 契约。本次仍不新增 `api_access` 权益、不开放企业批量训练许可商品、不做媒体文件内嵌元数据写入，也不修改 V2/119 payload。下一步按 Studio / Enterprise 阶段评审公开扫描 SDK 与企业批量 API 是否进入商业化 Roadmap。 | 阶段性完成 |
| 2026-06-29 | 完成公开元数据 JSON 的双端版权库导出入口：桌面端版权库详情页可下载 `GET /v1/public/rights/{watermarkUid}/metadata` 返回的 sidecar JSON，移动端版权库详情页可通过系统分享面板导出同一 JSON；移动端分享已补文件名覆盖和 iOS / iPad 弹窗定位参数。该能力服务 Creator / 未来 Studio 的权利公示基础，但仍不是收费企业批量 API、不是媒体内嵌 C2PA / IPTC 写入器、不是法律授权结论，也不新增 `api_access` 权益。验证：`dual:contract`、桌面构建、Flutter analyze / test 和后端公开权利测试复跑；iOS 同场景运行态 QA 因当前 Windows 环境无 macOS / Xcode / iOS 设备被阻断，证据为 `tmp-ui-qa/public-rights-runtime/ios-public-rights-metadata-qa-20260629-124447.md`。下一步先在 macOS + iOS Simulator 或真机补运行态截图，再评审公开扫描 SDK 是否进入 Studio / Enterprise 阶段。 | 阶段性完成，iOS QA 待补 |
| 2026-06-29 | 在 iOS QA 暂时挂起后，完成公开扫描 SDK 的桌面第一版商业化前置能力：`src/lib/public-rights-sdk.ts` 封装单条 / 批量公开查询、训练许可策略解析和用户文案映射，桌面版权库详情与验证页统一使用 SDK 解释 `backfill_pending`、`backfill_disputed`、撤销、替代和 registry 不可用等状态。该能力仍只服务阶段性公开查询体验，不开放收费企业批量训练许可商品，不新增 `api_access` 权益，不把 `canTreatAsTrainingAllowed` 设为真，也不提供法律授权结论。下一步先做移动端 Dart 同构 SDK，再评审外部分发 SDK / Studio / Enterprise API 边界。 | 桌面 SDK 第一版完成 |
| 2026-06-29 | 完成公开扫描 SDK 的移动端同构第一版：移动端版权库详情页和验证页已改用 `PublicRightsScanner.scanOne`、`resolvePublicRightsPolicy`、`formatPublicRightsUserMessage` 解释公开 registry 结果，和桌面端同样固定 `legalConclusion=false`、`canTreatAsTrainingAllowed=false`，避免把训练许可声明包装为自动可训练授权。该能力仍不新增套餐权益、不开放收费企业批量 API、不提供外部分发 SDK 包。下一步评审 SDK 外部分发形态、匿名限流、批量查询额度和 Studio / Enterprise API 边界。 | 双端 SDK 第一版完成 |
| 2026-06-29 | 冻结公开扫描 SDK 外部分发与批量额度商业边界：匿名公开批量查询当前硬上限为 100 条，只是反滥用技术保护，不是 Studio / Enterprise 可售额度；稳定错误码集合已写入协议并由后端测试固定。外部分发包首版只能只读查询，不开放回填、撤销、替代、重签、manifest 写入或媒体内嵌 C2PA/IPTC。Enterprise API 若进入商业化，必须先新增 `api_access` 权益、API key 身份、额度账本、调用审计、网关限流和观测指标；当前仍不得售卖企业批量训练许可检查。下一步设计 API key / quota ledger 数据模型后再评审是否纳入 Studio / Enterprise 阶段。 | 边界已冻结，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 公开扫描 API key / quota ledger 数据模型草案：新增 `docs/Enterprise公开扫描API Key与额度账本模型草案.md`，定义 `enterprise_api_keys`、`enterprise_quota_balances`、`enterprise_quota_ledger`、`enterprise_api_audit_events`、只读 scope、`public_rights_scan_units`、API key 状态机和 quota ledger 状态机；`docs/商业化契约与权益模型.md` 已挂接该模型。当前只做设计和合同，不新增数据库迁移、不开放 Enterprise API 路由、不把 `api_access` 自动开通为售卖权益。下一步评审后再决定是否先实现数据库迁移和内部管理命令。 | 草案完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 公开扫描 API key / quota ledger 第一阶段内部实现：后端 schema 新增 `enterprise_api_keys`、`enterprise_quota_balances`、`enterprise_quota_ledger`、`enterprise_api_audit_events`，Storage 新增内部命令用于创建 API key 元数据、记录 quota ledger 和 audit event，并由后端测试固定非法写 scope 被拒、quota 幂等唯一和不暴露明文 key。当前仍不开放 `/v1/enterprise/public-rights` 路由、不提供后台 UI、不自动开通 `api_access`、不售卖企业批量训练许可检查。下一步实现内部管理 CLI / 后台入口和 quota balance 初始化逻辑。 | 内部模型完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise API key 内部管理入口和 quota balance 初始化：后端新增受管理员 token 保护的 `POST /internal/enterprise/api-keys` 与 `POST /internal/enterprise/quota-balances`，内部 CLI `scripts/enterprise-internal-admin.mjs` 可调用上述入口；quota balance 初始化按 `accountId + workspaceId + quotaType + periodStart + periodEnd` 幂等 upsert，允许调整合同额度与超额策略但不重置 `usedUnits` / `reservedUnits`。本次仍不开放 `/v1/enterprise/...` 外部 API、不生成或返回明文 API key、不接网关限流、不做 quota 自动扣减结算、不新增 `api_access` 售卖闭环。下一步实现内部管理列表 / 查询 / 暂停 / 撤销能力，并继续保持外部 Enterprise API 关闭。 | 内部入口完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise API key 内部列表 / 查询 / 暂停 / 撤销：后端新增受管理员 token 保护的 `GET /internal/enterprise/api-keys`、`GET /internal/enterprise/api-keys/{apiKeyId}`、`POST /internal/enterprise/api-keys/{apiKeyId}/pause`、`POST /internal/enterprise/api-keys/{apiKeyId}/revoke`，内部 CLI 同步支持 list / get / pause / revoke；返回值只包含 key 元数据和 `keyPrefix`，不暴露 `keyHash` 或明文 key。暂停 / 撤销只改变内部状态，不接真实企业扫描、不接网关限流、不扣 quota、不开放 `/v1/enterprise/...` 外部 API。下一步实现内部操作审计细分和只读后台 UI，继续保持外部 Enterprise API 关闭。 | 内部 key 管理完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 内部操作审计细分：新增 `enterprise_admin_audit_events`，将后台管理操作按 `create_api_key`、`list_api_keys`、`get_api_key`、`pause_api_key`、`revoke_api_key`、`init_quota_balance` 记录 operation / outcome / endpoint / account / workspace / apiKey / target / reason / details；`admin_audit_events` 继续只记录管理员 token 是否通过。该审计不代表外部企业 API 调用审计已经上线，不接真实企业扫描、不扣 quota、不开放 `/v1/enterprise/...`。下一步实现内部只读后台 UI 或审计查询入口，继续保持外部 Enterprise API 关闭。 | 内部审计细分完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 内部只读审计查询入口：后端新增受管理员 token 保护的 `GET /internal/enterprise/admin-audit-events`，支持按 `operation`、`outcome`、`accountId`、`apiKeyId`、`fromOccurredAt`、`toOccurredAt` 和 `limit` 查询 `enterprise_admin_audit_events`；内部 CLI 新增 `list-admin-audit-events`。该查询本身不再写入新的 Enterprise admin audit event，避免审计日志自污染；管理员 token 校验仍由通用 `admin_audit_events` 记录。本次仍不开放 `/v1/enterprise/...` 外部 API、不接真实企业扫描、不扣 quota、不生成明文 API key。下一步评审内部后台 UI 列表页和审计导出能力，继续保持外部 Enterprise API 关闭。 | 内部审计查询完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 内部后台审计列表页：桌面端新增 `EnterpriseAuditView`，只调用 `GET /internal/enterprise/admin-audit-events`，支持 operation / outcome / accountId / apiKeyId / occurredAt 筛选、按时间游标分页和当前页 JSON 导出；管理员 token 仅保存在页面内存，内部服务地址可本地保存用于调试。商业合同已固定该页面不得调用 `/v1/enterprise/...`。本次仍不开放外部 Enterprise API、不接真实企业扫描、不扣 quota、不生成明文 API key。下一步评审 Enterprise API key 内部管理 UI，继续保持外部 Enterprise API 关闭。 | 内部审计列表页完成，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise API key 内部管理 UI：桌面端 `EnterpriseAuditView` 升级为 Enterprise 内部管理工作台，同一页面接入 `/internal/enterprise/api-keys` create / list / get、`/pause`、`/revoke`、`/internal/enterprise/quota-balances` 初始化和 `/internal/enterprise/admin-audit-events` 审计查询 / 当前页 JSON 导出；页面明确 `keyHash` / `keyPrefix` 来自线下密钥托管流程，管理员 token 仅保存在页面内存。商业合同固定前端不得调用 `/v1/enterprise/...`，本次仍不开放外部 Enterprise API、不生成或返回明文 API key、不接真实企业扫描、不扣 quota、不新增 `api_access` 售卖闭环。下一步设计外部 Enterprise API 网关鉴权、限流和只读扣费合同，仍先不开放客户路由。 | 内部管理 UI 完成，商业 API 未开放 |
| 2026-06-29 | 完成外部 Enterprise API 网关合同草案：新增 `EnterpriseGatewayAuthContext`、`EnterpriseGatewayRateLimitPolicy`、`EnterpriseGatewayQuotaChargePlan`、`EnterpriseGatewayAuditContract`、`EnterpriseGatewayReadOnlyScanContract`、`ENTERPRISE_GATEWAY_REQUIRED_STEPS` 和 `ENTERPRISE_GATEWAY_STABLE_ERROR_CODES`，只固定未来外部路由必须先经过 API key 鉴权、scope 授权、`api_access` 检查、限流、只读解析、quota ledger 和 API audit，当前仍不开放 `/v1/enterprise/...` 客户路由、不接真实企业扫描、不扣真实额度。下一步如继续推进，应只做 dry-run helper / 测试门禁，不把合同误写成已经上线的客户 API。 | 外部网关合同完成，客户路由未开放 |
| 2026-06-28 | 收口桌面手动测试暴露的商业化可信体验问题：云同步待上传时不再显示“同步状态正常”，避免 Creator 云同步权益被误解为已完成；桌面和移动批量队列可从中断的 `running` 状态恢复为可重试失败，避免 Creator 批量能力卡死；图片 / 音频 / 视频写入成功提示统一展示端到端耗时、保护副本位置 / 分享出口、payload 协议、编号签发、登记状态和 payload 认证，便于 Free 单份报告、Creator 报告和未来维权服务读取同一证据维度；视频 L1 正式入口回到单一音轨保护副本，不再包装平台压制或画幅适配能力。本次不新增套餐权益、收费商品、企业 API 或 L3 视频画面水印承诺。下一步用真实 Free / Creator 账号复跑安装版桌面端的图片写入、批量写入、待同步提示和报告字段展示。 | 代码已完成，运行态复测待补 |
| 2026-06-29 | 完成 Enterprise 内部 dry-run 网关校验 helper：新增 `EnterpriseGatewayDryRunRequest`、`EnterpriseGatewayDryRunDecision` 和 `dry_run_enterprise_gateway_readonly_scan`，只输入模拟 API key 元数据、required scope、rate-limit 窗口、quota balance 快照和 item 数，输出鉴权、scope、`api_access`、限流、quota 扣费计划和 API audit 决策；拒绝路径固定不扣真实额度且 `legalConclusion=false`。本次仍不开放 `/v1/enterprise/...` 客户路由、不读写 quota ledger、不接真实企业批量扫描、不生成明文 API key。下一步如继续推进，应先把 helper 接入内部管理命令或受管理员 token 保护的内部校验入口。 | 内部 dry-run helper 完成，客户路由未开放 |
| 2026-06-29 | 完成 Enterprise dry-run 网关校验内部入口和 CLI：新增受管理员 token 保护的 `POST /internal/enterprise/gateway-dry-run`，复用 `dry_run_enterprise_gateway_readonly_scan` 返回鉴权、scope、`api_access`、限流、quota 和 audit 决策，并写入 `dry_run_gateway` 内部管理审计；`scripts/enterprise-internal-admin.mjs dry-run-gateway` 可手工触发。该入口不写 quota ledger、不写外部 API audit、不接真实企业扫描、不开放 `/v1/enterprise/...` 客户路由。下一步应补内部运行态样例和失败矩阵 QA。 | 内部 dry-run 入口完成，客户路由未开放 |
| 2026-06-29 | 完成 Enterprise gateway dry-run 运行态 QA 门禁：新增 `scripts/verify-enterprise-gateway-dry-run-runtime-qa.mjs` 和 `npm run enterprise:gateway-dry-run-runtime-qa`，启动临时后端并通过内部 CLI `dry-run-gateway` 固定 success、scope_denied、api_access_disabled、rate_limited、quota_exhausted、api_key_revoked 六个样例，同时查询 `dry_run_gateway` 管理审计确认每个 requestId 落账。该门禁已纳入 `run-commercial-ci.mjs`，仍不开放 `/v1/enterprise/...` 客户路由、不写真实 quota ledger、不接真实企业扫描。下一步应设计真实 Enterprise API key 明文签发 / key custody 流程草案。 | 内部 dry-run 运行态 QA 完成 |
| 2026-06-29 | 完成真实 Enterprise API key 明文签发 / key custody 流程草案：在 `docs/Enterprise公开扫描API Key与额度账本模型草案.md` 中定义后端可信执行环境或内部 CLI 生成明文 key、明文只显示一次、KMS/HSM 或环境 secret 计算 `keyHash`、`keyPrefix` 只做定位、轮换以新 key + 旧 key paused/revoked 方式处理、撤销不可恢复且审计不保存明文或 `keyHash`。本次仍不实现真实明文签发、不开放 `/v1/enterprise/...`、不新增客户自助 API key 控制台、不自动开通 `api_access=true`。下一步应评审该 custody 草案后，再决定是否实现受管理员 token 保护的内部签发命令。 | key custody 草案完成，客户路由未开放 |
| 2026-06-29 | 完成 Enterprise API key 内部明文签发入口：后端新增受管理员 token 保护的 `POST /internal/enterprise/api-key-issuances`，由可信后端生成 256 bit 随机 `hsent_live_...` 明文 key，使用 `HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET` 和 `HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET_VERSION` 计算 `hmac-sha256:v1:<secretVersion>:<digest>` 入库，响应只返回一次 `cleartextApiKey`；内部 CLI 新增 `issue-api-key`。`enterprise_admin_audit_events` 新增 `issue_api_key` operation，审计 details 只记录 keyPrefix、scope、hashAlgorithm、交付通道和 recipientRef，不记录明文或 `keyHash`；列表 / 查询 / 桌面内部页仍只返回 key 元数据。本次仍不开放 `/v1/enterprise/...` 客户路由、不接真实企业扫描、不扣真实 quota、不新增客户自助 API key 控制台、不自动开通 `api_access=true`。下一步补 `issue-api-key` 内部运行态 QA，验证明文只显示一次、后续查询无明文 / 无 `keyHash`、审计无泄露。 | 内部签发完成，客户路由未开放 |
| 2026-06-29 | 完成 Enterprise API key 内部轮换命令：后端新增受管理员 token 保护的 `POST /internal/enterprise/api-keys/{apiKeyId}/rotate`，内部 CLI 新增 `rotate-api-key`；轮换复用同一套明文签发 / `keyHash` 入库流程生成新 active key，旧 key 立即进入 `paused`，响应只返回一次新 `cleartextApiKey`，`rotate_api_key` 审计以旧 `apiKeyId` 为主体、新 `apiKeyId` 为 target，记录 grace period / deadline / delivery 摘要且不记录明文或 `keyHash`。运行态 QA 已扩展 issue -> rotate -> revoke 链路，验证旧 key paused 后可手工 revoked，新 key 保持 active，后续 list / get / audit 不泄露明文或 `keyHash`，外部 `/v1/enterprise/...` 路由继续关闭。本次仍不接真实企业扫描、不扣真实 quota、不开放客户自助 API key 控制台、不自动开通 `api_access=true`。下一步实现自动 grace period revoke job 或内部巡检命令。 | 内部轮换完成，客户路由未开放 |
| 2026-06-29 | 完成 Enterprise 过期轮换自动撤销内部巡检命令：后端新增受管理员 token 保护的 `POST /internal/enterprise/api-key-rotations/revoke-expired`，内部 CLI 新增 `revoke-expired-rotations`；巡检从 `rotate_api_key` 管理审计 details 的 `rotationDeadlineAt` 读取到期时间，只处理到期且仍为 `paused` 的旧 key，自动调用 revoke 并分别写入 `revoke_api_key` 明细审计和 `revoke_expired_rotations` 汇总审计。运行态 QA 已改为 issue -> rotate -> sweep-before-deadline -> sweep-after-deadline 链路，验证未到期不撤销、到期后旧 key revoked、新 key active、list / get / audit 不泄露明文或 `keyHash`、外部 `/v1/enterprise/...` 路由继续关闭。本次仍不接真实企业扫描、不扣真实 quota、不开放客户自助 API key 控制台、不自动开通 `api_access=true`。下一步把 `revoke-expired-rotations` 接入受控 cron / 运维 runbook，并保留人工 dry-run 验证步骤。 | 内部自动撤销巡检完成，客户路由未开放 |
| 2026-06-30 | 完成公开权利商业化剩余主线第一轮落地：桌面 PNG / JPEG 公开元数据嵌入副本已通过官方 `c2pa` Rust SDK 写入 signed manifest，QA 无生产证书时使用 ephemeral development certificate 并明确不代表公开信任锚；WAV 使用 RIFF `hsPM` chunk、MP4 / M4A / MOV 使用 `uuid` box 写入 registry metadata JSON packet；外部分发 TypeScript SDK 包 `packages/public-rights-sdk` 已具备 `scanOne` / `scanBatch` / `resolvePolicy` / `formatUserMessage`；外部客户侧仅开放 `POST /v1/enterprise/public-rights/batch`，通过 API key、scope、`api_access=true`、DB rate-limit、quota ledger committed debit 和 API audit 执行只读批量查询，客户侧 key / quota 管理路由仍不存在，所有返回固定 `legalConclusion=false`。下一步补生产 C2PA 证书 / TSA 运维 runbook、外部 SDK 发布清单和 Enterprise 客户文档，不开放任何法律授权结论。 | 阶段性完成，生产信任链 / 发布清单待补 |
| 2026-06-30 | 完成 V3 感知质量与性能 fast gate：新增 `watermark:quality-gate:fast` 与 `watermark:quality-gate:contract`，覆盖 V3 图片 roundtrip PSNR / SSIM / 耗时和音频 SNR / LUFS / 峰值差异 / 新增 clipping；当前 fast gate 是开发回归门禁，不等于完整 release SLA，也不开放 `Forensic` / `Balanced` 用户可选策略。下一步设计 `watermark:quality-gate:release` 的真实样本池，并把 VMAF 保持在 L3 staged / internal full gate。 | fast gate 完成，release 样本池待补 |
| 2026-06-30 | 完成生产 C2PA 证书链 + TSA + SDK 发布 + Enterprise 客户开通 Runbook：新增 `docs/生产C2PA证书链_TSA_SDK发布_Enterprise客户开通Runbook.md`，把 `HIDDENSHIELD_C2PA_SIGN_CERT_PEM` / `HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM` / `HIDDENSHIELD_C2PA_SIGNING_ALG` / `HIDDENSHIELD_C2PA_TSA_URL`、图片 C2PA active manifest QA、音视频传播层边界、`packages/public-rights-sdk` contract / typecheck / pack dry-run、Enterprise quota balance / key issuance / dry-run / 真实 batch 小流量验证、pause / revoke / rotate / revoke-expired 回滚演练和 SLA / 错误码 / 客户联系人检查一次性固化；新增 `public-rights:production-readiness-contract` 并纳入 `commercial:ci`。下一步按 runbook 做一次 staging 演练，填完整发布检查清单。 | Runbook 与机器门禁完成，staging 演练待补 |
| 2026-06-30 | 完成生产公开权利 runbook staging 演练：SDK 侧通过 `rights:sdk-package-contract`、SDK typecheck 和新增 `rights:sdk-pack-dry-run`，并修正原 `npm --prefix packages/public-rights-sdk pack --dry-run` 在当前 Windows/npm 环境下误打根包的问题；Enterprise 侧通过 dry-run gateway 六样例、API key 一次性明文签发 / rotate / revoke-expired、真实 `POST /v1/enterprise/public-rights/batch` 小流量扣 2 units、quota 不足拒绝和后端 quota/audit 单测；图片公开元数据、音视频传播层和音视频 C2PA active manifest QA 通过。当前阻塞项是 staging shell 未注入生产等价 C2PA cert/key/alg/TSA，不能宣称生产 C2PA trust chain 或 TSA 已上线；真实客户联系人、生产 SLA owner 和升级路径仍需 release owner 签字。证据：`tmp-ui-qa/public-rights-production-staging/1782795869702/public-rights-production-staging-runbook-qa-1782795869702.md`。新增 `rights:metadata-embed-production-staging-qa` 专用门禁，要求 secret manager 注入后复跑并确认 `c2paSignerStatus=configured_certificate_chain`。下一步通过 secret manager 注入生产等价 C2PA 证书链 / TSA 后运行该命令。 | Staging 演练部分通过，生产 C2PA/TSA 阻塞 |
| 2026-06-30 | 补齐生产 C2PA 证书申请与 Secret 注入 Checklist：新增 `docs/生产C2PA证书申请与Secret注入Checklist.md`，固定 CA / trust provider 选择、CSR 与私钥生成、KMS / HSM / secret manager 私钥托管、TSA provider / RFC 3161 endpoint 开通、四个 C2PA/TSA 环境变量注入、`rights:metadata-embed-production-staging-qa` 复跑、验收截图 / QA evidence、泄露扫描、轮换 / 吊销和上线红线；机器合同 `public-rights:production-readiness-contract` 已检查该 checklist。下一步由 Release owner 选择 CA / TSA provider，完成证书申请和 secret manager 注入后复跑生产 staging QA。 | Checklist 完成，证书/TSA 获取待外部处理 |
| 2026-06-30 | 完成 Enterprise 可信反向代理 / IP 指纹限流与音视频官方 C2PA 运行态收口：后端新增 `HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET`、`HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY`，只信任携带共享密钥的反向代理头，API audit / gateway 响应只保存 hash-only `clientFingerprintHash`，并按 API key + 指纹分桶限流；`enterprise:public-rights-runtime-qa` 已在强制可信代理模式下验证成功扣额和 quota 拒绝。WAV / MP4 公开元数据导出已升级为传播层 + 官方 C2PA active manifest QA。视频默认能力仍只开放 L1 音轨水印和 L2 指纹存证，L3 保持 staged / internal，不进入 UI、云任务、账本或 SLA。下一步仍是注入生产 C2PA 证书链 / TSA 并复跑生产 staging QA，同时由 release owner 完成首个 Enterprise 客户 SLA / 支持联系人 / 回滚窗口签字。 | 网关加固完成，生产信任链和客户签字待补 |
| 2026-06-30 | 新增非外部依赖商业化收口材料：`docs/Enterprise生产客户开通检查单模板.md` 固定客户信息、API 范围、key custody、quota / 限流、小流量验收、审计 / 回滚和交付材料签字项；该模板不依赖外部 C2PA 证书 / TSA，也不表示真实客户已签字上线。同步新增 `watermark:quality-gate:release` 和 `rights:v3-media-payload-release-qa`，并已将其纳入当前发布前非外部依赖阻断组合。首次运行结果显示 V3 payload 图片 / 音频 / L1 视频音轨写读 QA 通过，随后复跑固定图片样本池也通过。下一步由 release owner 用该模板填首个试点客户，并继续扩展 full 样本池与真实素材基准。 | 检查单模板与 V3 payload QA 完成，release 阻断组合已启用 |
| 2026-06-30 | 补充 `docs/HiddenShield北极星能力与性能指标.md` 第一版决策收口：图片北极星明确按素材类型分层，正式容器收敛到 PNG / JPEG / WebP，注入后必须做视觉无损验证；音频北极星加入 ABX 主观听感样本池和人类听觉无损验证；L1 视频音轨明确支持多音轨并拒绝静音 / 极短 / 低于 30 秒视频；L2 指纹存证加入相似性阈值和争议处理流程；L3 保持未来高阶商业能力；云端版权库 UI 个人库优先；Studio 团队共享库不支持跨成员转让 / 归档 / 撤销；Enterprise 外部 API 长期只读；生产 C2PA 证书链和 TSA 继续作为发布前硬门槛。 | 北极星决策收口完成，商业化边界未放宽 |
| 2026-07-01 | 推进 L3 release candidate 准备：`feedback-backend` 的 L3 `succeeded` 状态改为必须携带策略摘要、自检阈值、自检置信度、抽检帧数、加水印产物哈希和服务端收据签名，且 `confidence >= threshold` 后才允许写入 `video_minutes`；新增独立 `watermark:l3-video-visual-release-gate`，强制完整 24 个 2K 样本池、H.264 / HEVC 分组阈值、耗时和失败归因。该工作不开放 L3 用户入口、不上传真实视频、不生成正式报告、不承诺 Studio / Enterprise 已包含 L3。下一步运行该 gate 并按阻断样本修算法或阈值，再设计可信 worker/admin completion API。 | L3 release candidate 门禁接入，用户承诺未开放 |
| 2026-07-01 | 完成 L3 24 样本 release gate 与 trusted completion 收据链路：`npm run watermark:l3-video-visual-release-gate` 已跑完整 24 个 2K 样本池并通过，证据目录 `tmp-ui-qa/l3-video-visual-release-gate/1782888912515/`，H.264-HD / H264-LT / H264-MT / HEVC-HD / HEVC-MIX 非风险样本 confidence 均为 1.000，H264-RISK 正确归因为 `risk_boundary_expected`；后端新增 trusted worker/admin `POST /internal/video-tasks/:task_id/completion`，用户 bearer `succeeded` 更新返回 `cloud_video_task_completion_requires_trusted_worker`，只有 HMAC 收据绑定 task / strategy / confidence / checkedFrames / media hash 后才写 `video_minutes`。桌面 / 移动正式入口、真实 worker、版权库 L3 记录、正式报告、跨端验证、失败文案和隐私边界仍未完成，不得写成 L3 已可售。下一步接真实 worker 执行链路和双端受控入口。 | L3 算法门禁与收据门完成，产品正式化待补 |
| 2026-07-01 | 完成 L3 受控 worker 最小闭环：新增 `watermark-core/src/bin/l3_controlled_worker_fixture.rs` 和 `cloud-video:l3-worker-qa`，内部 fixture worker 调用 `watermark-core` 生成 `VideoFeatureBundle`、正式 payload、`VideoVisualStrategy`，通过 DCT 写入和成品帧自检得到 `strategyDigest`、`selfCheckConfidence=1.0`、`checkedFrames=4`、`watermarkedMediaHash`；普通用户 bearer 伪造 `succeeded` 仍被拒绝，trusted worker/admin completion 固化收据并写入 `video_minutes`。该闭环只处理内部 fixture / 受控上传清单，不开放用户视频上传、真实转码、桌面 / 移动 L3 入口或可售 SLA；当前 QA 区分任务 `watermarkUid` 与 core 派生的 `payloadWatermarkUid`，正式 worker 还需补 registry-reserved UID 与 core payload 绑定。下一步把 worker 扩展到真实上传清单解析、转码沙箱、UID 绑定、队列重放保护和失败归因。 | 受控 worker 闭环完成，真实 worker 待补 |
| 2026-07-01 | 完成 L3 真实 worker first-pass：新增 `l3_real_worker_first_pass` 和 `cloud-video:l3-real-worker-first-pass-qa`，真实后端 E2E 先 reserve `video_visual` registry UID，再创建带 `l3_controlled_upload_proxy`、`controlled://l3-upload-proxy/...`、`l3_ffmpeg_transcode_sandbox_v1`、`h264_controlled_proxy_v1` 的 L3 task；worker 解析受控上传清单、运行 FFmpeg sandbox、把 registry-reserved UID 绑定进 `watermark-core` payload、自检后 confirm registry 并通过 trusted completion 扣 `video_minutes`。该工作仍不开放普通用户上传、不生成用户可下载视频、不新增桌面 / 移动入口、不承诺 L3 可售 SLA。下一步补任务领取与幂等锁、队列重放保护、失败归因和真实输出视频封装。 | 真实 worker first-pass 完成，正式产品面待补 |
| 2026-07-01 | 完成 L3 真实 worker 队列执行模型：后端新增内部 claim / failure API，任务记录持久化 worker、attempt、lease hash、attempt count、lease expiry 和失败归因；trusted completion HMAC 绑定 `workerId`、`attemptId`、`leaseToken`，只有当前有效 claim 能成功并扣 `video_minutes`，旧 attempt / 错 lease / 重复 completion 均被拒绝。`cloud-video:ci` 已通过，真实 worker first-pass QA 覆盖运行中任务不可重复领取、retryable failure 重排队、non-retryable failure 不扣费和重复 completion 不重复扣费。该工作仍不开放普通用户上传、不生成用户可下载视频、不新增桌面 / 移动入口、不承诺 L3 可售 SLA。下一步补受控对象读取、真实输出封装、用户可下载产物和 worker receipt 持久审计。 | L3 队列闭环完成，输出封装待补 |
| 2026-07-01 | 完成 L3 受控对象读取、真实输出 MP4 封装和 worker receipt 持久审计：真实 worker 从受控对象根读取 `controlled://l3-upload-proxy/...` proxy 文件并校验 manifest `sha256` / `bytes`，调用 `watermark-core` 完成写入、自检和最终 MP4 packaged self-check，输出 `controlled://l3-output/<taskId>/<taskId>.l3-watermarked.mp4`；后端 trusted completion HMAC 绑定并持久化 output ref / bytes / content type、worker receipt JSON 和 receipt hash，`watermarkedMediaHash` 绑定最终 MP4 文件哈希。`cloud-video:ci` 已通过，证明受控队列现在能承载可下载形态的 L3 MP4 产物，而不只是 proxy / hash 闭环。该工作仍不开放普通用户对象存储、不提供签名下载 URL、不新增桌面 / 移动入口、不承诺 Studio / Enterprise L3 可售 SLA。下一步接普通用户对象存储签名下载 / 下载授权，并把桌面 / 移动 Studio / Enterprise 受控入口、版权库 L3 记录、正式报告字段和跨端验证纳入同一 release gate。 | 受控输出封装完成，用户产品面待补 |
| 2026-07-01 | 完成 L3 输出短期签名下载授权 API 与 Studio / Enterprise 受控入口：后端新增用户侧 `POST /v1/video-tasks/:task_id/output-download-authorizations` 和签名解析 `GET /v1/video-tasks/:task_id/output-download?token=...`，只对已成功并固化 output / media hash / receipt hash 的 L3 task 签发 `l3_output_download_authorization_v1`；E2E 固定 pending task 返回 `cloud_video_task_output_not_ready`，篡改 token 返回 forbidden。桌面和移动视频工作台均展示 Studio / Enterprise 受控 L3 release-gate 入口，说明 trusted worker receipt、`controlled://l3-output/...` 和签名下载授权边界。该工作仍不开放普通用户对象存储、不提供真实对象存储字节分发、不生成版权库 L3 记录或正式报告、不承诺 Studio / Enterprise L3 可售 SLA。下一步接普通用户对象存储上传、真实字节分发适配、桌面 / 移动下载入口、版权库 L3 记录、正式报告字段和跨端读取验证。 | 签名下载授权与受控入口完成，正式产品面待补 |
| 2026-07-01 | 完成 L3 普通对象上传、真实对象存储字节分发和版权记录收据字段第一段：后端新增 `POST /v1/video-tasks/object-upload-authorizations` 与 `PUT /v1/video-object-store/upload?token=...`，上传 token 绑定账号 / workspace / creator / object ref / SHA-256 / 字节数 / content type / 过期时间；`l3_real_worker_first_pass` 从 `object://l3-upload/...` 读取，输出 `object://l3-output/...`，签名下载返回真实 MP4 字节并复核 `watermarkedMediaHash`。桌面 / 移动版权库模型、SQLite、同步 payload 和正式报告草稿新增 `video_visual_*` 收据字段，只保存 task、策略、自检、媒体哈希、receipt hash、字节数和 content type，不保存对象 ref、签名 URL、本地路径或媒体字节。该工作仍不把 L3 定义为可售 SLA；正式创建 / 下载产品流、版权库写入触发、跨端运行态验证、失败文案和隐私边界仍需纳入 release gate。下一步实现桌面 / 移动 L3 任务完成后的下载与版权库写入操作流，并用真实同步记录验证跨端报告一致。 | 对象存储与收据字段完成，正式产品流待补 |
| 2026-07-01 | 完成 L3 succeeded task 端到端产品流第一段：桌面 Tauri 新增 `save_l3_video_visual_task_to_vault`，移动端新增 `saveL3VideoVisualTaskToVault`，两端都要求 task 为 `succeeded` / `video_visual`、`confidence >= threshold`、`checkedFrames > 0`、`object://l3-output/...`、`video/mp4`、media hash、worker receipt hash 和 server receipt 完整后，才创建下载授权、下载 MP4 字节、复核 SHA-256 / 字节数并写入版权库 `video_visual_*` 字段和同步队列。新增 `cloud-video:l3-product-flow-gate` 并接入 `cloud-video:ci`，固定桌面 / 移动下载按钮、版权库记录、正式报告字段、跨端同步字段和隐私排除项。该工作仍不等于可售 SLA；正式创建 / 上传向导、跨端真实运行态验证、失败文案和隐私边界仍待补。下一步做真实后端 desktop->mobile / mobile->desktop L3 记录同步运行态 QA。 | L3 succeeded task 产品流入 gate，正式可售仍待补 |
| 2026-07-01 | 完成 L3 创建上传向导 + 失败文案 + 隐私边界：桌面新增 `create_l3_video_visual_upload_task`，移动端新增 `createL3VideoVisualUploadTaskFromBytes`，两端 Studio / Enterprise 入口均按“准备上传 -> 上传受控对象 -> 创建云端 L3 任务 -> 等待 trusted worker”推进，只创建 `hybrid_visual_watermark` queued 任务，不写版权库、不标记 succeeded、不触发 `video_minutes`。失败文案覆盖权益、登录、MP4 类型、时长、上传授权、哈希回读、任务创建和 worker failureCode；隐私边界固定 `signed_object_upload_only_no_local_path_no_raw_video_sync`，同步 / 报告仍只保存 `video_visual_*` 收据元数据。当时仍是 MP4-only release gate 且移动端时长需手填；后续已补移动端可信视频尺寸 / 帧率探测、生产队列运营、SLA / 回滚 / 客服文案、对象清理策略和 on-call runbook。客户开通验收和生产 observability 面板 / 告警平台接入完成前，仍不能写成 Studio / Enterprise 已可售。下一步复跑 `cloud-video:l3-product-flow-gate` 与 `cloud-video:ci`，再补生产可售验收清单。 | L3 创建上传向导入 gate，可售 SLA 待验收 |
| 2026-07-02 | 完成 L3 真实用户 MP4 样本池运行态 QA 第一版：新增 `cloud-video:l3-sellable-runtime-qa` 并纳入 `cloud-video:ci`，真实后端下以 desktop / mobile 两个 device 分别创建 MP4 上传任务、签名上传对象、预留 `video_visual` UID、运行 trusted worker、固化 receipt、扣 `video_minutes`、签名下载 MP4、构造 `video_visual_*` 版权记录并推送云同步，由另一端读取版权库详情和正式报告投影。证据 `tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782925001403.md` 覆盖 `desktop_square_motion_mp4` 与 `mobile_square_detail_mp4` 两个最小样本。该工作仍不等于可售 SLA：1024x576 与 1024x1024/2fps 样本曾触发 `strategy_invalid` 容量边界；后续已补移动端可信探测、生产队列监控 / 回滚 / 客服文案、对象清理策略和 on-call runbook，客户开通验收和生产 observability 面板 / 告警平台接入仍待补。下一步扩展 16:9 / 9:16 / 高帧率真实用户 MP4 样本池并修复容量不足失败归因。 | 真实 MP4 最小证据链完成，可售 SLA 待扩样本池 |
| 2026-07-02 | 完成 L3 真实 MP4 尺寸 / 帧率扩展样本池第二轮并冻结 512x512@2fps 产品输入限制：`cloud-video:l3-sellable-runtime-qa` 复跑证据 `tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782931358998.md` 覆盖 1024x1024、1280x720、608x1080 9:16、1920x1080、真实拍摄运动 fixture 和字幕密集 MP4 成功样本；`mobile_square_small_high_fps_strategy_invalid` 改为任务创建阶段 `input_rejected`，返回稳定错误码 `l3_strategy_capacity_insufficient`，不创建 task、不返回 `usageLedgerId`、不扣 `video_minutes`。后端任务创建、桌面上传向导和移动上传向导均接入容量预检；本轮决定当前可售主战场不为 512x512@2fps / 8 帧短视频立即改 `watermark-core` 策略容量，后续只有产品明确支持低容量短视频时才进入 core 策略容量改造。该工作仍不等于可售 SLA：生产队列监控、SLA / 回滚 / 客服文案、客户开通验收和更大真实用户 MP4 样本池仍待补。`cloud-video:l3-product-flow-gate`、`cloud-video:contract`、`dual:contract`、后端 cloud_video_task 单测和 `cloud-video:ci` 已通过。下一步把生产队列运行态监控纳入 L3 可售验收。 | L3 扩展样本池与容量输入限制完成，仍为 release gate |
| 2026-07-02 | 完成 L3 生产运营门禁第一版：新增 `cloud-video:l3-production-ops-runtime-qa` 并纳入 `cloud-video:ci` 与 `cloud-video:l3-product-flow-gate`，真实后端下固定 `l3_production_queue_monitor_snapshot_v1`、`l3_production_worker_attempt_sla_v1`、worker retryable failure 回队列、旧 attempt replay protection、fatal failure no-charge hold、pending / failed 下载授权 `cloud_video_task_output_not_ready` 和客服失败文案矩阵；证据 `tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782931405830.md`。矩阵覆盖 `l3_strategy_capacity_insufficient`、`sandbox_transcode_failed`、`core_strategy_failed`、`strategy_invalid`、`self_check_failed`、`self_check_confidence_below_threshold`、`worker_receipt_invalid`、`manifest_invalid` 和下载未就绪。该工作仍不等于可售 SLA：真实生产 observability 面板、on-call 告警、对象存储清理策略、客户开通验收、移动端可信视频尺寸 / 帧率探测和更大真实素材目录样本池仍待补。下一步把移动端可信视频元数据探测和对象存储清理 / 告警 runbook 接入同一 release gate。 | L3 生产队列监控与客服矩阵进入 CI |
| 2026-07-02 | 完成 L3 移动端可信视频尺寸 / 帧率探测 + 对象存储清理策略 + 生产 on-call 告警 runbook：移动端创建向导选择 MP4 后读取 ISO BMFF `tkhd` / `stts` / `stsz` 元数据，优先把宽高、帧数、帧率和时长传入同一容量预检 / manifest 路径；`cloud-video:l3-production-ops-runtime-qa` 新增 `l3_object_storage_cleanup_policy_v1` 和 `l3_production_on_call_alert_runbook_v1`，固定上传 / 下载 token TTL、failed/canceled upload 清理、succeeded output 保留、hash mismatch 隔离、队列 backlog、lease stuck、失败峰值、receipt 校验失败、清理失败和 billing guard 告警；`cloud-video:l3-product-flow-gate` 已检查源码、QA schema 和文档令牌。该工作仍不等于 L3 可售 SLA：真实生产 observability 面板 / 告警平台接入、客户开通验收和更大真实用户 MP4 目录样本池仍待补。下一步把生产监控面板和客户开通验收清单接入同一 release gate。 | L3 可信探测 / 清理策略 / on-call runbook 进入 CI |
| 2026-07-02 | 完成 L3 生产 observability 面板 / 告警平台接入 + 客户开通验收清单的 release gate 接入：`cloud-video:l3-production-ops-runtime-qa` 新增 `l3_production_observability_dashboard_v1`、`l3_alert_platform_integration_v1`、`l3_alert_platform_delivery_dry_run_v1`、`l3_customer_opening_acceptance_checklist_v1` 和 `l3_customer_opening_acceptance_dry_run_v1`，固定队列 / attempt / receipt / object store / billing / customer impact 面板、告警平台路由、dedupe payload、隐私边界和客户开通验收步骤；`cloud-video:l3-product-flow-gate` 已检查脚本、QA 文档、可售清单、能力边界和 Roadmap 令牌。该工作仍不等于 L3 可售 SLA：真实告警平台配置验证、首个试点客户签字验收和更大真实用户 MP4 目录样本池仍待补。下一步把真实告警平台配置验证和试点客户签字样本接入同一 release gate。 | L3 observability / alert / customer opening gate 完成 |
| 2026-07-02 | 推进视频 L1 / L2 / L3 可售收口：移动端 L1 已从“本地写入”假承诺收紧为“可验证 L1 视频音轨水印，保护副本由桌面生成”；移动端 L2 新增 Creator 云同步权益下的轻量不可逆 metadata 指纹 notary 提交，真实调用 `/v1/video-fingerprints/notaries`，写入版权库 `video_notary_*` 字段并进入同步队列，移动端不上传原始视频、不保存本地路径、不把 L2 包装为盲水印。`cloud-video:ci` 本轮复跑通过，最新 L3 sellable runtime 证据为 `tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782938523428.md`，production ops 证据为 `tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1782938569583.md`。L3 代码门禁、双端产品流、真实 worker、扣费守卫和 ops dry-run 已闭环，但真实告警平台配置验证、首个试点客户签字验收和更大真实用户 MP4 目录样本池仍是可售 SLA 阻断项。下一步建立 `cloud-video:l3-production-readiness-contract`，把真实 webhook / on-call 配置、试点客户签字 artifact 和真实样本目录 manifest 做成显式 release blocker。 | L1/L2 可售消费推进完成，L3 外部验收阻断 |
| 2026-07-02 | 完成 L3 production readiness contract 接入：新增 `cloud-video:l3-production-readiness-contract` 并纳入 `cloud-video:ci`，默认输出 `blocked` 报告，确认真实告警平台配置验证、首个试点客户签字验收和更大真实用户 MP4 目录样本池仍是外部可售阻断；设置 `HIDDENSHIELD_L3_REQUIRE_PRODUCTION_READY=1` 后，必须提供真实 HTTPS webhook、`l3_alert_platform_real_delivery_validation_v1` JSON、`l3_pilot_customer_signoff_v1` 签字记录和至少 24 个真实用户 MP4 样本 manifest 才能通过。下一步由 release owner 提供真实 artifact 后强制模式复跑。 | L3 外部可售阻断已机器化 |
| 2026-07-02 | 完成公开权利协议最终收口门禁接入：新增 `rights:ios-public-rights-v3-runtime-qa`，用 iOS 原生 Flutter 运行态覆盖公开权利 JSON、公开元数据 JSON、PNG 图片嵌入副本字节检查和默认 V3/39 写读；新增 `public-rights:completion-gate`，把生产 C2PA/TSA、iOS QA、外部 npm 发布、release 样本池和首批客户签字 5 个收口项变成 JSON artifact 强制门禁。默认运行输出 BLOCKED 证据，不伪造外部完成；设置 `HIDDENSHIELD_PUBLIC_RIGHTS_REQUIRE_COMPLETE=1` 后必须提供 5 份真实 artifact 才能通过。下一步由 release owner 在具备 CA/TSA、macOS+iOS、npm 发布权限、真实样本池和客户签字后强制模式复跑。 | 公开权利收口外部阻断已机器化 |
| 2026-07-02 | 扩充 `docs/封版收口计划.md` 为 RC1 无外部依赖验收总表：把视频 L1 / L2 / L3 分层边界、L3 production readiness blocked artifact、公开权利信号与训练许可传播层、Enterprise 只读公开扫描、V3/39 默认锚点、双端报告 / 同步 / 隐私排除项和 RC1 命令顺序纳入封版计划。该回写不放宽商业化承诺：真实微信支付、生产 C2PA/TSA、真实告警平台、首个试点客户签字、外部 npm 发布和 iOS QA 仍保持外部阻断；L3 后续仍必须沿用 `watermark-core` 视频画面算法和云端执行包装设计，不能在后端、云任务、桌面端或移动端另起盲水印算法核心。下一步先执行 `npm run commercial:ci`，再按 RC1 顺序补桌面安装版人工 QA 和 Android 页面级 QA。 | RC1 封版计划已扩充，外部商业化阻断不变 |
| 2026-07-02 | 完成 RC1 无外部依赖自动化复跑与商业化边界修复：`commercial:ci` 总命令因 20 分钟工具超时被截断，已按脚本步骤拆分复跑，商业合同、Enterprise runtime QA、公开权利 production readiness、计费 / 用量 / 报告 / 团队合同、架构合同、视频分层合同、前端构建、后端测试、Tauri release-scope、Flutter analyze / test、`cloud:ci`、`cloud-video:ci`、`watermark:cross-end-release`、`watermark:quality-gate:release`、`rights:v3-media-payload-release-qa` 和 `rights:runtime-qa` 均通过。修复项：自助 billing session 收紧为 Creator / Studio，Enterprise 仍走受控客户开通流程；`watermark:architecture-contract` 精确允许 L3 wrapper receipt / 容量预检元数据但继续禁止 core 外算法；Tauri 验证测试按 V3/39 默认锚点更新。`public-rights:completion-gate` 和 `cloud-video:l3-production-readiness-contract` 按预期保持 BLOCKED。下一步补桌面安装版完整人工 QA，并记录 Android 页面级 QA 设备与截图。 | RC1 自动化通过，外部商业化阻断不变 |
| 2026-07-02 | 补齐 Enterprise 非自助订阅合同口径：`docs/商业化契约与权益模型.md` 明确 `enterprise` 仍是合法权益层级，可用于企业合同开通、内部客户开通、API key / quota / 审计和服务端权益快照；但自助订阅 payment session 仅允许 Creator / Studio，Enterprise 必须走合同 / 客户确认、内部 quota 初始化、内部 API key 签发或轮换、网关 dry-run、生产运行态验收和审计留痕，不允许 0 元自助订单绕过企业开通。验证：`npm run billing:contract` 通过，`cargo test --manifest-path feedback-backend/Cargo.toml fixture_billing_rejects_enterprise_self_service_and_workspace_mismatch --lib` 通过。下一步继续桌面安装版完整人工 QA，并把 Android 页面级 QA 截图索引补入封版计划。 | Enterprise 自助支付边界已固定 |
| 2026-07-03 | 启动本地版权库与云版权库同步可靠性收口：新增 `docs/本地版权库与云版权库同步可靠性设计.md`，把会员权益单一事实源、Creator 自动同步触发、token refresh、stale `syncing` 恢复、断线续传、去重、限流、诊断文案和双端运行态 QA 固化为 Phase S0-S4。真实 QA 已发现本地 Creator 缓存与后端 Free 快照冲突、新记录停留 `pending / attempts=0`、旧 `syncing` 不恢复等问题；在 S0/S1 修复前，Creator 自动云同步不能记为 RC1 人工 QA 通过。下一步优先实现 S0：后端权益快照覆盖本地缓存，并加最小 `cloud:sync-reliability-contract`。 | 同步可靠性设计完成，运行态修复待做 |
| 2026-07-03 | 冻结云版权库 PostgreSQL 迁移设计：新增 `docs/云版权库PostgreSQL迁移设计.md`，明确当前 `feedback-backend` SQLite 只保留用于本地 / 开发 / 合同 smoke / RC1 无外部依赖验收，生产云版权库、Enterprise 公开扫描 API、quota ledger、API audit、支付 webhook、团队共享库和云视频任务必须迁移到 PostgreSQL；文档固化迁移表清单、SQLite 风险、SQL 差异、Enterprise 并发写入要求、压测门槛、P0-P6 实施阶段和当前不改代码边界。本次不替换 `rusqlite`、不改 Storage API、不改同步 payload。下一步评审 P0，评审通过后再做 P1 数据库抽象层与 `cloud:db-portability-contract`。 | PostgreSQL 迁移设计完成，代码未动 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P0 评审并启动 P1 最小抽象层：`docs/云版权库PostgreSQL迁移设计.md` 已补 P0 评审结论，确认迁移表清单覆盖当前后端生产相关表，压测门槛覆盖同步、Enterprise、quota 幂等、支付、L3 claim、审计查询和恢复演练；P1 新增 `feedback-backend/src/database.rs`，引入 `DatabaseBackendKind`、`DatabaseConfig`、生产环境 SQLite 禁用规则和 PostgreSQL skeleton，`Storage` 改为通过 `open_with_database_config` 打开 SQLite dev/test adapter；新增 `cloud:db-portability-contract` 固定 SQLite 现有路径不退和 Postgres adapter 仍为显式 skeleton。本次不迁移业务 SQL、不引入真实 Postgres driver、不改同步 payload。下一步在 P1 基础上评估 `sqlx` 并抽 auth / sync / registry 的首批 repository trait。 | P0 通过，P1 最小合同接入 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P1.2：`feedback-backend` 新增 feature-gated `sqlx` Postgres 依赖，默认构建仍不启用；新增 `repository.rs`，先抽 `AuthRepository`、`CloudSyncRepository`、`WatermarkRegistryRepository` 三组 trait，并由当前 SQLite `Storage` 委托实现，避免一次性改 handler；`database.rs` 在 `postgres` feature 下暴露 `PostgresPool`、`PgPoolOptions` 和 P1 schema smoke SQL，覆盖 auth / sync / registry 首批表；`cloud:db-portability-contract` 升级为双路径合同，检查 SQLite adapter、Postgres skeleton、repository trait、schema smoke，并强制 `cargo check --features postgres`。本次仍不连接真实 PostgreSQL、不执行 migrate、不改同步 payload。下一步进入 P2：建立真实 Postgres migration 目录和 dev/staging `migrate up` smoke。 | P1.2 完成，Postgres 依赖已 feature-gated |
| 2026-07-03 | 完成 PostgreSQL 迁移 P2 非连接型 migration contract：新增 `feedback-backend/migrations/postgres/0001_auth_sync_registry.up.sql` 与 `.down.sql`，把 auth / cloud sync / watermark registry / `rights_manifests` 的首批 schema smoke 从 Rust 字符串迁到真实 migration 文件；`database.rs` 改为 `include_str!` 引用 migration，新增 `cloud:postgres-migration-contract` 检查 up/down 文件、表清单、索引、JSONB / TIMESTAMPTZ / BIGSERIAL / BOOLEAN、禁止 SQLite 语法和 Rust `postgres` feature smoke，并把该门禁接入商业化 CI。当前仍不连接生产库、不执行真实 migrate、不切换业务 SQL。下一步准备本地 disposable Postgres 或 CI service，执行 P2.1 `migrate up/down` 真实空库 smoke。 | P2 migration 文件与合同完成 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P2.1 disposable migrate smoke：新增 `feedback-backend/src/bin/postgres_migrate_smoke.rs` 和 `scripts/run-postgres-migrate-smoke.mjs`，`cloud:postgres-migrate-smoke` 可使用 `HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL` / `DATABASE_URL` 指向的临时库，或在有 Podman / Docker 时自动启动一次性 `postgres:16-alpine`；Rust smoke 会真实执行 `0001_auth_sync_registry.up.sql` / `.down.sql`，校验表、索引、partial unique index、关键列类型和回滚后空 schema，并拒绝不含 localhost/127.0.0.1 与 `hiddenshield_migrate_smoke` 的 URL。本机已用 Podman 5.7.1 跑通真实 disposable Postgres 往返，结果 `upTablesChecked=11`、`indexesChecked=11`、`rollback=empty_schema_verified`。默认商业化 CI 仍只跑非连接型合同，真实 DB smoke 作为环境具备时的 P2.1 gate。 | P2.1 Podman 实跑通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P2.2 smoke artifact：`cloud:postgres-migrate-smoke` 现在会把每次 disposable Postgres 真实 up/down 结果写入 `tmp-ui-qa/postgres-migration/postgres-migrate-smoke-<timestamp>.json`，schema 为 `postgres_migration_smoke_artifact_v1`，包含 Podman / Docker runtime 版本、镜像、容器名、端口、数据库名、`upTablesChecked`、`indexesChecked`、`rollback=empty_schema_verified`、安全约束和容器清理结果；artifact 不记录密码或完整连接 URL。下一步将该 artifact 路径补入 RC1 封版计划或 release evidence 索引。 | P2.2 artifact 完成 |
| 2026-07-03 | 完成 PostgreSQL 迁移证据索引与 P3 前置设计：`docs/封版收口计划.md` 已补最新 disposable smoke artifact `tmp-ui-qa/postgres-migration/postgres-migrate-smoke-1783021160601.json`，记录 Podman 5.7.1、`postgres:16-alpine`、11 张表、11 个索引和 `rollback=empty_schema_verified`；`docs/云版权库PostgreSQL迁移设计.md` 固化 P3 repository 顺序为 auth -> cloud sync -> watermark registry，并定义 `auth:postgres-runtime-qa`、`cloud:sync-postgres-runtime-qa`、`watermark:registry-postgres-runtime-qa` 的最小运行态 QA。当前仍不切默认运行路径、不连接生产库、不改变同步 payload 或版权编号。下一步进入 P3.1：实现 `AuthRepository` Postgres adapter 和 `auth:postgres-runtime-qa`。 | P3 前置设计完成 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P3.1 auth 读写 adapter：新增 feature-gated `PostgresAuthRepository`，只实现 `AuthRepository` 的 challenge、session、refresh、logout、list devices、revoke device；新增 `auth:postgres-runtime-qa`，用 disposable `hiddenshield_auth_runtime_qa` 跑真实 Postgres migration up/down 和 auth 运行态 QA。最新证据 `tmp-ui-qa/postgres-auth-runtime/auth-postgres-runtime-qa-1783025723013.json` 覆盖 fixture challenge、challenge session、password session、同账号双设备一致、refresh rotation、旧 refresh 拒绝、设备列表、device revoke、logout 后 refresh 拒绝，并明确 `syncRepositoryWritePath=not_executed`、`registryRepositoryWritePath=not_executed`。当前仍不接正式 UI / mock / release 默认路径，不连接生产库，不启动 sync / registry Postgres 写路径。下一步先评审 P3.1 artifact，再进入 P3.2 `CloudSyncRepository` Postgres adapter 与 `cloud:sync-postgres-runtime-qa`。 | P3.1 auth adapter 通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P3.2 cloud sync 读写 adapter：评审 `auth-postgres-runtime-qa-1783025723013.json` 后确认 P3.1 证据可接受，新增 feature-gated `PostgresCloudSyncRepository`，只实现 `CloudSyncRepository` 的 push batch 与 pull changes；新增 `cloud:sync-postgres-runtime-qa`，用 disposable `hiddenshield_sync_runtime_qa` 跑真实 Postgres migration up/down 和 sync 运行态 QA。最新证据 `tmp-ui-qa/postgres-sync-runtime/cloud-sync-postgres-runtime-qa-1783038415955.json` 覆盖 desktop push、重复 `client_event_id` 幂等、mobile 初次 pull、重复 pull 空变更、cursor resume、wrong device 拒绝、Free push 403，并明确 `registryRepositoryWritePath=not_executed`。当前仍不接正式 UI / mock / release 默认路径，不连接生产库，不启动 registry Postgres 写路径。下一步先评审 P3.2 artifact，再进入 P3.3 `WatermarkRegistryRepository` Postgres adapter 与 `watermark:registry-postgres-runtime-qa`。 | P3.2 sync adapter 通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P3.3 watermark registry 读写 adapter：评审 `cloud-sync-postgres-runtime-qa-1783038415955.json` 后确认 P3.2 证据可接受，新增 feature-gated `PostgresWatermarkRegistryRepository`，只实现 `WatermarkRegistryRepository` 的 reserve / confirm / reconcile / reissue；新增 `watermark:registry-postgres-runtime-qa`，用 disposable `hiddenshield_registry_runtime_qa` 跑真实 Postgres migration up/down 和 registry 运行态 QA。最新证据 `tmp-ui-qa/postgres-registry-runtime/watermark-registry-postgres-runtime-qa-1783051039045.json` 覆盖 server reserve、同一 request id 幂等 reserve、server confirm、offline reconcile、冲突检测、reissue job、长格式 UID 保持，并明确 `syncRepositoryWritePath=not_executed`、`formalUiMockReleaseDefaultPath=not_switched`。当前仍不接正式 UI / mock / release 默认路径，不连接生产库。下一步先评审 P3.3 artifact，再设计 `cloud:postgres-runtime-qa` 聚合门禁。 | P3.3 registry adapter 通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P3.4 聚合门禁：新增 `cloud:postgres-runtime-qa` 串行复跑 auth / sync / registry 三组 disposable Postgres runtime QA，并输出 `cloud_postgres_runtime_qa_aggregate_v1`。最新证据 `tmp-ui-qa/postgres-runtime-aggregate/cloud-postgres-runtime-qa-1783053449984.json`，子证据为 `tmp-ui-qa/postgres-auth-runtime/auth-postgres-runtime-qa-1783053450477.json`、`tmp-ui-qa/postgres-sync-runtime/cloud-sync-postgres-runtime-qa-1783053459156.json`、`tmp-ui-qa/postgres-registry-runtime/watermark-registry-postgres-runtime-qa-1783053469951.json`。`cloud:db-portability-contract` 已检查该聚合入口，仍不接正式 UI / mock / release 默认路径，不连接生产库。下一步进入 P4 SQLite -> PostgreSQL 导入 smoke。 | P3.4 聚合门禁通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P4 本机导入 smoke：新增 `cloud:postgres-import-smoke`，把 in-memory SQLite fixture 中的首批 auth / sync / registry / rights manifest 数据导入 disposable PostgreSQL，验证 10 张表、14 行数据、row count、primary-key hash aggregate、二次导入幂等、逻辑引用、唯一约束和回滚空 schema。最新证据 `tmp-ui-qa/postgres-import/postgres-import-smoke-1783053193204.json`，`idempotentRerun=row_counts_unchanged`、`hashAggregate=primary_key_hash_match`、`rollback=empty_schema_verified`。该证据不读取真实用户 SQLite、不代表 staging 数据迁移完成，也不改变同步 payload、版权编号或商业权益。下一步进入 P5，把真实 staging 压测、备份恢复、observability、切换 runbook 和 release owner 签字做成强制门禁。 | P4 本机导入 smoke 通过 |
| 2026-07-03 | 完成 PostgreSQL 迁移 P5/P6 外部阻断机器化：新增 `cloud:postgres-production-readiness-gate`，默认输出 `cloud_postgres_production_readiness_gate_v1` blocked artifact，强制 ready 模式要求真实 staging 压测、备份 / PITR / 恢复演练、observability、切换 runbook 和 release owner signoff artifact；新增 `cloud:postgres-sqlite-shutdown-gate`，检查生产禁用 SQLite 结构已存在但必须等待 P5 通过证据。最新 blocked 证据：`tmp-ui-qa/postgres-production-readiness/cloud-postgres-production-readiness-gate-1783053429272.json`、`tmp-ui-qa/postgres-sqlite-shutdown/cloud-postgres-sqlite-shutdown-gate-1783053429239.json`。当前不得宣称生产云版权库、Enterprise API 或支付 / quota 账本已切 PostgreSQL。下一步由 release owner 准备真实 staging / 生产前 artifact 后强制模式复跑。 | P5/P6 外部项 BLOCKED |
| 2026-07-03 | 重新审计本地版权库与云版权库同步可靠性设计：确认该文档创建早于 SQLite + PostgreSQL 双后端迁移完成，因此已补充三层边界：端侧本地 SQLite 队列、`feedback-backend` SQLite dev/test adapter、feature-gated PostgreSQL adapter。审计结论是原始 blocker 仍成立，但 S0/S1 必须先修端侧事实源刷新、队列恢复和 SQLite dev/test 合同；`payload_hash`、`entity_revision`、per-event ingestion result 和云端投影表属于 S3 双 adapter 后端任务，不能在 S0/S1 中伪造成已完成。新增计划中的 `cloud:sync-reliability-contract` 默认无外部依赖，Postgres 层作为 optional disposable smoke；生产 readiness 仍受 P5/P6 blocked gate 控制。下一步执行 S0 最小合同，固定“本地 Creator 缓存 + 后端 Free”必须以后端 Free 为准。 | 同步可靠性审计完成，S0 待实现 |
| 2026-07-03 | 推进本地版权库与云版权库同步可靠性 S0/S1：桌面端手动 flush 前刷新 `/v1/me`，以后端权益快照覆盖本地缓存；401 会先 refresh auth session 并保存新 token；Free / `blocked_by_entitlement` 会阻断正式云同步并写入队列诊断，不再让本地 Creator 缓存覆盖后端 Free。`cloud_sync_queue` 已增加 `last_error_code`、`last_http_status`、`blocked_reason`、`lease_until`，stale `syncing` 可恢复为 pending，`synced` 重复入队不重传；新增 `cloud:sync-reliability-contract` 固定这些无外部依赖合同。已跑 `cargo test --manifest-path src-tauri/Cargo.toml sync::storage::tests::cloud_queue --lib` 通过；下一步复跑 `npm run cloud:sync-reliability-contract`、`cargo check --manifest-path src-tauri/Cargo.toml` 和 `npm run build`，再继续 S1 auto flush 触发 / S2 结构化状态机。 | S0/S1 本机基础已实现，运行态 QA 待补 |
| 2026-07-03 | 完成云同步可靠性 S0-S3 本机可验证切片并机器化 S4 阻断：普通 pipeline 入队后会触发后台 best-effort auto sync；队列诊断已扩展 pending / syncing / failed / blocked / synced、错误码、HTTP status、blocked reason；后端 SQLite 与 feature-gated PostgreSQL adapter 均增加 `eventResults`、`payload_hash`、`entity_revision`，重复同 payload 返回 `duplicate`，同 `clientEventId` 变更 payload 返回 `conflict_payload_changed`。验证通过：`npm run cloud:sync-reliability-contract`、`cargo check --manifest-path src-tauri/Cargo.toml`、`npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml sync::storage::tests::cloud_queue --lib`、`cargo test --manifest-path feedback-backend/Cargo.toml push_and_pull_cloud_events_round_trip --lib`、`npm run cloud:sync-postgres-runtime-qa`、`npm run cloud:postgres-migrate-smoke`、`npm run cloud:postgres-import-smoke`、`npm run cloud:db-portability-contract`。S4 已新增 `cloud:sync-runtime-qa` readiness gate，artifact `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783058057700.json` 当前 BLOCKED，等待桌面安装版、Android、网络恢复和 event disposition runtime QA 证据。 | S0-S3 本机通过，S4 runtime QA BLOCKED |
| 2026-07-03 | 执行云同步 S4 真实环境 evidence 首轮：真实 `feedback-backend` 已在 `127.0.0.1:43188` 运行，`npm run tauri:build` 成功生成最新版 MSI / NSIS 安装包，NSIS 已静默安装到 `D:\TestInstall\HiddenShield`，Android 模拟器 `emulator-5554` 在线并能启动原生 App；新增 `cloud:sync-runtime-evidence` 采集四类 artifact。结果：后端 event disposition 证据 `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783062976375.json` 通过；桌面安装版证据 `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783062976375.json` 证明安装包 / release exe / installed exe 均存在且启动 smoke 通过，未证明安装版内 Creator / Free 同步语义；Android 证据 `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783062976375.json` 证明 APK 构建安装和 App 前台运行但 Flutter runner 240 秒超时；网络恢复证据 `tmp-ui-qa/cloud-sync-runtime/network-resume-sync-runtime-1783062976375.json` 仍缺真实生命周期驱动。强制 ready 复跑 `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783063284616.json` 按预期 BLOCKED。下一步先补桌面安装版 cloud sync automation channel 和 Android 专用 cloud sync runner，再复跑强制 ready。 | S4 首轮实跑，端侧语义仍 BLOCKED |
| 2026-07-03 | 完成云同步 S4 商业化专项运行态证据：桌面安装版新增 hidden cloud sync QA automation channel，Android 新增专用 `mobile_app/tool/cloud_sync_runtime_qa.dart` runner；两端 artifact 均覆盖 Creator pull / flush / pull、重复事件不重传、Free `blocked_by_entitlement`、队列诊断和同步 payload 隐私白名单。最新证据 runId `1783067038401`：桌面安装版 `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783067038401.json`、Android 原生 `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783067038401.json`、网络恢复汇总 `tmp-ui-qa/cloud-sync-runtime/network-resume-sync-runtime-1783067038401.json`、后端 event disposition `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783067038401.json` 均 `ok=true`；强制 ready `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json` 通过。该证据只放行 Creator 云同步专项 S4，不代表生产 PostgreSQL、真实支付、iOS QA、完整安装版页面级 QA 或 L3 可售 SLA 已完成。下一步补桌面安装版完整页面级人工 QA，并记录 Android 页面级截图索引。 | S4 云同步专项 ready，RC1 页面级 QA 继续 |
| 2026-07-03 | 补桌面安装版 Batch 2 第一组页面级 QA：安装版 `D:\TestInstall\HiddenShield\hidden_shield.exe` 在真实后端下完成图片 / 音频写入与验证、版权库入库和 Creator 正式报告导出，截图目录 `tmp-ui-qa/desktop-batch2-qa/`，正式报告产物 `E:\Users\jihx\AppData\Roaming\com.hiddenshield.desktop\reports\formal_report-hsr-fb47bc23c2d1e667.md` / `.json` 明确排除原始媒体、保护副本和本地路径。关闭后端后设置页设备刷新展示成熟错误 `服务暂时不可用，请稍后重试`，未暴露端口 / HTTP 原始异常 / 堆栈。发现页面级云同步暂停 UI 失败：点击 `暂停自动同步` 后未切到 `manual_local_only`，提示 `登录状态已失效，请重新登录后再试`，截图 `tmp-ui-qa/desktop-batch2-qa/20-cloud-sync-paused.png`；该项阻断桌面云同步暂停人工 QA 通过，但不推翻 S4 automation artifact。下一步修复暂停 / 恢复自动同步的登录刷新或重新登录引导，再复测该页面。 | 桌面 Batch 2 首组 QA 部分通过，暂停 UI 阻断 |
| 2026-07-03 | 修复并复测桌面设置页自动云同步暂停 / 恢复：`set_desktop_cloud_auto_sync_enabled` 现在先刷新 `/v1/me`，access token 过期时使用 refresh token 换新后再调用 `PATCH /v1/me/sync-preferences`；refresh 也失效时设置页清理本机失效 profile，并显示重新登录表单和 `登录状态已失效，请重新登录后再调整自动云同步。`。`cloud:sync-reliability-contract` 新增按钮路径 refresh/re-auth 断言。验证：`npm run cloud:sync-reliability-contract`、`cargo test --manifest-path src-tauri/Cargo.toml commands::sync::tests --lib`、`cargo check --manifest-path src-tauri/Cargo.toml`、`npm run build`、`npm run tauri:build` 均通过；安装版复测截图 `tmp-ui-qa/desktop-batch2-qa/25-expired-profile-relogin-after-click.png`、`27-creator-relogin-ready-for-pause.png`、`28-cloud-sync-paused-after-fix.png`、`29-cloud-sync-resumed-after-fix.png` 证明真实 Creator 可从 `auto_cloud_vault` 切到 `manual_local_only` 并恢复。下一步继续桌面 Batch 2 剩余页面级 QA。 | 暂停 UI 阻断解除 |
| 2026-07-03 | 继续桌面安装版 Batch 2 剩余页面级 QA：本地批量图片 / 音频队列通过，`hs-batch2-qa-image-source.png` 与 `hs-batch2-qa-audio-source.wav` 均完成 `verified`，证据 `tmp-ui-qa/desktop-batch2-qa/37-local-batch-after-manual-select-before-start.png`、`39-local-batch-final.png`；设置反馈 / 日志导出通过，导出日志隐私扫描无本地路径或媒体名；L2 桌面完整 bundle -> notary -> vault 通过，记录 #28 `video_notary_id=vfn_413e4ebf_8116c573`、bundle 生成 `738ms`。本轮发现两个商业化 RC1 阻断：L1 视频处理页显示完成后验证通过，但独立验证页读取同一成品 MP4 失败，置信度 `0%`；公开权利 / 公开元数据后端接口 200 且 JSON 隐私白名单通过，但桌面 UI 查询、JSON 导出和嵌入副本导出均提示无法连接服务，疑似 WebView fetch / CORS 缺口。性能观测写入汇总 `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-remaining-summary.json`：图片写入 `4081ms`、音频写入 `392ms`、L1 视频写入 `3278ms`、L2 bundle `738ms`、图片验证 `22ms`、音频验证 `37ms`。下一步先修复 L1 独立验证链路和公开权利 UI fetch/CORS，再复跑对应桌面安装版页面级 QA。 | 桌面剩余 QA 覆盖完成，L1 与公开权利 UI 阻断 |
| 2026-07-03 | 修复并只复跑桌面安装版两个 RC1 商业化阻断：L1 视频独立验证页现在对同一成品 MP4 读取 `HS-8AC03224-3A9A66CA-037F4F93-BA5E84D1`、置信度 `100%`、耗时 `300ms`，截图 `tmp-ui-qa/desktop-batch2-qa/56e-l1-video-verify-after-fix-result.png`；公开权利 / 公开元数据修复 Tauri WebView Origin CORS 后，版权库 UI 刷新显示 `registry 已生效`，JSON 导出落盘到 `E:\Users\jihx\Downloads\hiddenshield-public-rights-HS-0E0A015B-4FEA4271-86F9A4B9-53B58EAB.json`，嵌入元数据图片副本落盘到 `E:\Users\jihx\AppData\Roaming\com.hiddenshield.desktop\public-rights-metadata\HS-0E0A015B-4FEA4271-86F9A4B9-53B58EAB-public-rights-embedded.png`，证据 `tmp-ui-qa/desktop-batch2-qa/58-public-rights-refresh-after-cors-fix.png`、`59-public-metadata-json-export-after-cors-fix.png`、`60-public-metadata-embedded-export-after-cors-fix.png`、`public-rights-cors-after-fix.json`。新增复测汇总 `tmp-ui-qa/desktop-batch2-qa/rc1-blocker-fix-rerun-summary.json`；图片 4081ms vs 音频 392ms 判断为图片 release pipeline 包含 1024x1024 PNG 全量解码、DWT/DCT/SVD 写入、PNG 重编码、双哈希和写后验证，而音频样本是 30 秒 44.1kHz 单声道 PCM WAV 快路径。下一步进入 Android 页面级 QA，优先覆盖 L1 验证、公开权利 / 公开元数据、版权库和报告草稿截图索引。 | 桌面 RC1 阻断解除，Android 页面 QA 待补 |
| 2026-07-03 | Android 阻断相关页面级 QA 已补首轮证据：同一桌面 L1 MP4 保护副本在旧移动端 bridge 下因 MP4 default track 指向视频轨而失败，错误 `audio_sample_rate_missing`；修复为优先选择含音频采样率的轨道后，Android 验证页读回 `HS-8AC03224-3A9A66CA-037F4F93-BA5E84D1`、V3/39、置信度 `100%`。公开权利图片保护副本在 Android 验证页读回 `HS-0E0A015B-4FEA4271-86F9A4B9-53B58EAB`、公开权利 `registry 已生效`、训练许可 `禁止 AI / ML 训练`，并展示“不是法律授权结论”边界文案。证据 `tmp-ui-qa/desktop-batch2-qa/android-page-level-qa-summary.json` 和截图 `android-page-qa-23-l1-verify-after-track-fix-result.png`、`android-page-qa-24-l1-verify-after-track-fix-result-details.png`、`android-page-qa-25-public-rights-image-verify-result.png`、`android-page-qa-26-public-rights-image-details.png`。该证据不等于 Android 全量页面级 QA、iOS QA、真实支付或生产 C2PA/TSA 已完成。下一步继续 Android Batch 2 剩余页面：图片 / 音频写入、保护副本分享、版权库、报告草稿、L2 metadata notary、公开元数据导出入口和关闭后端成熟错误。 | Android 阻断相关页面通过，全量页面 QA 待补 |
| 2026-07-04 | 修复云版权库增量同步 cursor 漏拉风险并完成桌面公开权利 / 训练许可 / 公开元数据复跑：后端 SQLite / feature-gated PostgreSQL auth snapshot 均改为设备级 `cloud_vault_cursor`，`/v1/sync/changes` 取客户端 cursor 与服务端设备 cursor 中较早者，避免新设备或旧 profile 跳过已有云版权库事件。验证通过 `new_device_session_uses_device_cursor_before_first_pull`、`push_and_pull_cloud_events_round_trip`、`cloud:sync-reliability-contract` 和 `cargo check --features postgres`。安装版 `D:\TestInstall\HiddenShield\hidden_shield.exe` 在真实后端下拉取新云端版权记录，公开权利显示 `registry 已生效`、训练许可 `禁止 AI / ML 训练`、`legalConclusion=false`；公开元数据 JSON 下载通过，本地图片 #25 的嵌入公开元数据 PNG 导出通过。汇总 `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-public-rights-sync-cursor-summary-20260704.json`。该工作只解除桌面 RC1 页面级阻断，不代表生产 C2PA/TSA、iOS QA、真实支付、生产 PostgreSQL 或 L3 可售 SLA 已完成。下一步继续 Android Batch 2 剩余页面级 QA。 | 桌面公开权利阻断解除，Android 页面 QA 继续 |
| 2026-07-04 | 完成桌面安装版 Batch 2 页面级 QA 证据核验：当前后端 `/v1/health` 正常，安装版 `D:\TestInstall\HiddenShield\hidden_shield.exe` 与 2026-07-04 构建包一致，WebView CDP sanity 截图 `tmp-ui-qa/desktop-batch2-qa/97-desktop-batch2-current-sanity.png` 仍显示 Creator 版权库、图片 / 音频 / L1 / L2 / 云端公开权利记录。新增汇总 `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json`，将本地批量、L1 视频音轨、L2 视频指纹存证、公开权利 / 训练许可、公开元数据、设置反馈和日志导出归档为桌面安装版已完成。该结论不代表 Android 全量页面、iOS、真实支付、生产 PostgreSQL、生产 C2PA/TSA 或 L3 可售 SLA 已完成。下一步继续 Android Batch 2 剩余页面级 QA。 | 桌面安装版 Batch 2 页面 QA 完成 |
| 2026-07-04 | 完成 Android Batch 2 剩余页面级 QA：新增 `mobile_app/tool/android_batch2_page_qa.dart` 与 `dual:android-batch2-page-qa`，在 Android 模拟器 `emulator-5554` + disposable `feedback-backend` 下覆盖图片 / 音频写入、保护副本系统分享、版权库详情、Creator 报告草稿、L2 metadata notary、公开元数据 JSON 分享入口和关闭后端成熟错误。最终证据 runId `1783106946906`：汇总 `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json`，截图目录 `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/screenshots/`，拉取产物包含图片 / 音频保护副本、报告草稿和公开元数据 JSON。图片写入 `1229ms`、音频写入 `4215ms`、L2 metadata notary `vfn_93f80e98_dd73a1ca`；报告隐私扫描未命中原始媒体、保护副本字节、本地路径、对象 ref 或签名 URL。该证据不替代 iOS、真实支付、生产 PostgreSQL、生产 C2PA/TSA、真实 OS 断网拨测或 L3 可售 SLA。下一步先处理本机可推进的历史 `vault_records.file_type` backfill / contract 风险。 | Android Batch 2 剩余页面 QA 完成，iOS / backfill 风险待补 |
| 2026-07-04 | 解除本地版权库 `vault_records.file_type` 商业化数据风险并整理 RC1 双端 QA 总索引：SQLite schema 升级到 v18，历史 `file_type='video'` 且扩展名可确定的图片 / 音频记录会 backfill 为 `image` / `audio`，L2 / L3 视频收据字段存在时保持 `video`；新入库记录通过 `infer_vault_record_file_type(record)` 显式写入类型，桌面云同步和 changes response 的 `kind` 复用同一推断，避免图片 / 音频在云版权库 payload 语义上回退为 video。新增 `vault:file-type-backfill-contract`，并把桌面、Android、云同步、报告隐私、PostgreSQL、iOS blocked 和外部生产 blocked 证据汇总到 `docs/RC1双端QA总索引.md`。本次不改变水印 payload、版权编号、watermark-core 算法、同步隐私白名单或正式 UI / mock / release 默认路径。下一步复跑 `npm run commercial:ci` 与 `npm run vault:file-type-backfill-contract`，作为 RC1 无外部依赖验收入口。 | file_type 风险解除，RC1 QA 总索引完成 |
| 2026-07-04 | 完成 RC1 商业化聚合复跑：`npm run commercial:ci` 完整通过并输出 `HiddenShield commercial CI OK`；`scripts/run-commercial-ci.mjs` 已纳入 `Vault file_type backfill contract`，本轮输出 `vault:file-type-backfill-contract OK`，使 `vault_records.file_type` 修复进入商业化聚合验收。同步修正 Enterprise / cloud sync / cloud video CI helper 的后端启动方式与 cloud-video 动态端口，避免 workspace 多 bin 和固定 `43188` 端口抢占造成误报。最新证据包括 `tmp-ui-qa/enterprise-gateway-dry-run-runtime/1783112617490/enterprise-gateway-dry-run-runtime-qa-1783112617490.json`、`tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1783113491319.json`、`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1783113504563.json`、`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1783113550753.json`；L3 production readiness 继续以 `tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1783113551653.json` 保持外部 BLOCKED。下一步进入 RC1 无外部依赖验收包整理，商业化外部项仍集中在真实微信支付、法务审阅、生产 C2PA/TSA、L3 客户签字和生产 PostgreSQL 切换。 | RC1 商业化聚合通过，外部上线项仍 BLOCKED |
| 2026-07-04 | 完成 RC1 无外部依赖验收包整理：新增 `docs/RC1无外部依赖验收包.md`，机器摘要 `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.json`，人工摘要 `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.md`；集中汇总 `commercial:ci` 最新输出、桌面 / Android QA、PostgreSQL disposable 证据和 blocked artifact。iOS official runner 在当前 Windows 环境生成 blocked artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/ios-qa-blocked-20260704.json`；真实 OS 断网拨测记录为 `tmp-ui-qa/rc1-no-external-acceptance/20260704/os-network-disconnect-drill-record-20260704.json`，保持 manual required。该验收包不开放真实支付、不放宽生产 C2PA/TSA、不把 L3 写成可售 SLA，也不把 disposable PostgreSQL 当生产切换。下一步执行真实 OS 断网人工拨测，并准备 release owner 评审。 | RC1 验收包完成，OS 断网待人工拨测 |
| 2026-07-04 | 完成 RC1 真实 OS 断网拨测收口：新增 `rc1:os-network-disconnect-drill`，在 Android 模拟器 `emulator-5554` 上真实执行 `svc data disable` / `svc wifi disable`，确认断网后 `10.0.2.2:43188` 不可达、恢复后可达，并保存断网 / 恢复截图 `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-network-off-20260704.png`、`tmp-ui-qa/rc1-no-external-acceptance/20260704/android-network-restored-20260704.png`；Android artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-os-network-disconnect-drill-20260704.json` 已关联队列诊断、成熟错误提示和隐私白名单证据。Windows 桌面端 artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/desktop-os-network-disconnect-drill-20260704.json` 仍 blocked：当前安装版连接 loopback `127.0.0.1:43188`，且本会话不能建立提权 firewall / proxy 阻断；聚合记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/os-network-disconnect-drill-record-20260704.json` 状态为 `partial_ready_desktop_blocked`。该收口不开放真实支付、不替代 iOS、不代表生产 PostgreSQL 或 L3 可售 SLA。下一步交给 release owner 做 RC1 验收包评审，并安排提权桌面网络拨测。 | Android OS 断网 ready，Windows 桌面端阻断保留 |
| 2026-07-04 | 提交 RC1 release owner 评审请求并安排 Windows 桌面端提权断网拨测：新增评审请求 artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.json` / `.md`，明确请求 release owner 对 `ready_for_release_owner_review_with_desktop_os_network_blocked` 做 go / no-go 决策；新增 Windows 桌面端复跑安排 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill-schedule-20260704.json` / `.md`，给出提权 firewall / proxy 与 LAN / staging backend 两条可执行路径、必需截图、队列状态、成熟错误和隐私白名单验收条件。该安排只解除流程交接，不把 Windows 桌面断网阻断标记为通过，也不改变商业权益、生产支付、生产 C2PA/TSA、生产 PostgreSQL 或 L3 可售边界。下一步等待 release owner 指派提权 QA operator 或 staging/LAN backend 窗口。 | 评审请求已提交，桌面拨测待 owner 排期 |
| 2026-07-04 | 完成 release owner RC1 go / no-go 决策并指定 Windows 桌面端拨测窗口：新增 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.json` / `.md`，结论为 RC1 验收包进入评审 GO，但最终 RC1 签字 NO-GO，直到 Windows 桌面端真实 OS 断网拨测通过或 release owner 书面豁免。Windows 桌面端提权拨测窗口指定为 2026-07-04 20:30-21:30 Asia/Shanghai，首选 `elevated_firewall_or_proxy`，备选 `lan_or_staging_backend`，输出目录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/`。外部 blocked 项仍按 RC1 无外部依赖范围接受，不代表生产支付、生产 C2PA/TSA、L3 可售 SLA 或生产 PostgreSQL 切换已完成。下一步在指定窗口执行 Windows 桌面端拨测并回写结果。 | RC1 评审 GO，最终签字待 Windows 拨测 |
| 2026-07-04 | 执行 Windows 桌面端提权断网拨测窗口并记录阻断：20:30 Asia/Shanghai 窗口内完成环境判定，当前会话不是管理员，安装版仍使用 loopback `127.0.0.1:43188`，未提供 LAN / staging backend；因此不能真实切断桌面 app 到后端路径，也不能把 `os-network-disconnect-drill-record-20260704.json` 更新为 ready。执行记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/windows-desktop-os-network-drill-execution-20260704.json` / `.md`。RC1 包仍可评审，但最终签字继续 NO-GO，直到 release owner 提供提权 QA operator、LAN / staging backend 或书面豁免。下一步由 release owner 补齐可执行环境后重跑 Windows 桌面端断网拨测。 | Windows 拨测窗口执行 blocked，最终签字仍 NO-GO |
| 2026-07-10 | 修复 RC 发布审查发现的桌面云同步 per-event disposition 消费缺口：桌面端 `CloudSyncBatchResult` 现在解析后端 `eventResults`，flush 只把 `accepted` / `duplicate` 清为 `synced`，把 `conflict_payload_changed` / `rejected_invalid_event` 保持为 failed 并写入稳定错误码，避免云端冲突被误清队列。`cloud:sync-reliability-contract` 已补桌面端消费断言；验证通过 `cargo test --manifest-path src-tauri/Cargo.toml desktop_flush_event_results_keep_conflicts_failed --lib`、`npm run cloud:sync-reliability-contract` 和 `npm run commercial:ci`，后者输出 `HiddenShield commercial CI OK`，最新 runtime 证据包括 `tmp-ui-qa/enterprise-gateway-dry-run-runtime/1783625469201/enterprise-gateway-dry-run-runtime-qa-1783625469201.json`、`tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1783626639064.json`、`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1783626662534.json`、`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1783626712107.json`；L3 production readiness 仍以 `tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1783626713381.json` 保持外部 BLOCKED。该修复不改变商业权益、同步隐私白名单、生产 PostgreSQL、生产 C2PA/TSA、真实支付或 L3 可售边界。下一步由 release owner 复核本次 RC 审查修复提交，并继续安排 Windows 桌面端真实 OS 断网通过或书面豁免。 | RC disposition 缺口修复，commercial:ci 通过 |
| 2026-07-10 | 完成发布材料和对外文案二次清扫：README、CHANGELOG、现行用户协议 / 隐私政策、Phase 9 用户协议 / 隐私政策 / 支付与订阅条款草案、商业模式规划和历史 SaaS PRD 均已收口，避免把 L3、生产 C2PA / TSA、生产 PostgreSQL 或真实支付写成已上线。关键调整包括：TSA 改为“配置后辅助证据”、真实支付改为“支付通道配置并联调后启用”、Enterprise 云视频改为“合同 / 开通验收后且不包含当前未开放 L3”、L3 计费改为“未来计费原则草案”。`docs/当前真实能力边界说明.md` 同步新增 TSA / C2PA 和生产 PostgreSQL 对外表达建议与禁止表达。验证：`git diff --check` 通过；风险词复扫确认目标文档中的 L3、TSA、PostgreSQL、真实支付和 SLA 表述均带有 future / blocked / 配置 / 验收边界。下一步建议 release owner 用清扫后的 README、CHANGELOG 和三份 Phase 9 法务草案做 RC 对外材料签字评审。 | 对外文案误售风险已收口，待 RC 签字评审 |
## 15. 版权证据报告 PDF 升级回写（2026-07-14）

状态：规划完成，尚未进入实现

本次完成：

- 新增 `docs/面向司法使用场景的版权证据报告PDF升级规划.md`。
- 将报告升级定义为 `PDF 主报告 + JSON 机器附件 + Manifest 完整性清单 + 可选证据附件`，而不是仅将 Markdown 打印为 PDF。
- 固定三层成熟度：L1 版权证据技术报告 PDF、L2 可独立校验的签名证据包、L3 司法协作证据包。
- 保持 Free / Creator / Studio / Enterprise 术语不变。
- 保持现有单份版权详细报告 19.9 元 / 份、维权证据包 49.9 元 / 份的商品方向，但新增边界：在案件模型、侵权样本采集、附件保全和比对链路完成前，49.9 元维权证据包不得宣传为完整案件证据包。

商业化状态变化：

- Phase 5 报告门禁仍为已完成。
- PDF 升级作为 Phase 5 的质量升级子路线，不改变现有 `report_export` 权益定义。
- L1 PDF 可进入 Creator 正式报告与 Free 单份报告交付。
- L2 签名、在线校验和撤销需要新增签发与密钥托管能力。
- L3 司法协作能力必须经过法务、第三方服务和真实案件流程验收后才能形成新商品承诺。

验证：

- 本次为产品与技术规划，未修改运行时代码，未执行现有报告测试。
- 已核对现有桌面 Markdown + JSON、移动端同字段草稿、usage ledger、单份购买授权和隐私边界。

风险：

- 当前报告可证明系统记录和技术验证事实，不能直接证明绝对权属、侵权成立或司法采纳。
- 当前维权证据包商品名可能让用户预期包含案件级取证，需要在 R0 先冻结包含项与排除项。

下一商业化任务：

- 执行报告 Phase R0，冻结 schema v2、报告状态模型、PDF 四页关键原型和商品包含项，再决定 L1 PDF renderer 技术方案。

2026-07-14 Phase R0 执行补充：

- 已完成图片、音频、L2 视频共用的四页高保真 HTML 原型，路径为 `docs/prototypes/copyright-evidence-report-r0/finalized.html`。
- 已完成 schema v2 与 Manifest 首轮草案。
- 已验证 Chromium 打印音频与 L2 视频样本均为 4 页 PDF，三档屏幕预览无水平溢出。
- 当前仍为设计与合同原型，不改变 `report_export` 运行态，不代表 PDF 已进入 Creator 或 Free 单份报告交付。
- 商品包含项仍需律师评审和异常 fixture 后冻结。

下一商业化任务：

- 完成 HTML/CSS -> PDF 与 Rust 原生 PDF 技术 spike，并据真实产物成本冻结 L1 PDF renderer。

2026-07-14 PDF renderer 技术选型回写：

- 已完成 `docs/版权证据报告PDF双实现技术Spike.md`。
- Phase R1 主渲染器冻结为 HTML / Chromium，原因是视觉还原、模板迭代和复杂分页成本明显优于手工 Rust 排版。
- Rust 原生保留为离线最小报告、灾备与归档参考，不进入首版 Creator / Free 单份报告高保真主路径。
- 当前 Chromium 单次独立进程 warm run 约 4.49 到 4.93 秒，尚未达到规划中的 3 秒目标；必须通过常驻进程和受控静态字体优化后才能进入正式交付。
- 数字签名工程估算不代表能力已上线，不改变当前商品边界。

下一商业化任务：

- 实现 Phase R1 Chromium 最小集成，并通过 3 秒性能、4 页分页、文本提取、字体策略和隐私字段门禁后再接正式 `report_export`。

2026-07-14 Phase R1 Chromium 最小集成回写：

状态：`内部实现完成，发布与可售门禁未通过`

- 桌面正式报告导出已从 Markdown + JSON 切换为独立目录中的 `report.pdf + report.json + manifest.json`。
- 三件套由同一 `FormalReportDocument schema v2` 生成，保持 Creator / 单份报告授权与 `report_export` 用量记账逻辑不变。
- 引入常驻 Chromium worker、项目内受控 Noto 中文字体和 3 秒 warm generation 门禁。
- Manifest 已记录 PDF / JSON 摘要、文件大小、模板、字体、页数和渲染耗时；签名状态固定为 `not_signed`。
- 最近导出 UI 已改为展示和复制 PDF、JSON、Manifest 路径。
- `npm run report:pdf-r1-gate` 连续三次通过，图片 fixture 均为 4 页、无溢出，约 1.04 到 1.87 秒。
- `npm run report:contract`、Rust 报告与 worker 单测、前端生产构建均通过。

风险与商业边界：

- Node / Playwright / Chromium sidecar 尚未完成安装包离线分发验收，因此不得写成所有已安装版本均可使用 PDF。
- 当前无 PDF 数字签名、PDF/A、可信时间戳长期验证或在线校验，不得升级“维权证据包”的可信签发承诺。
- 移动端仍不生成 PDF，不能宣传双端均可签发。

下一商业化任务：

- 完成干净 Windows 虚拟机安装包验收，并把 sidecar 体积、首次启动耗时、失败回退文案和卸载清理纳入 Creator / 单份报告发布清单。

2026-07-14 Phase R2 证据包完整性校验回写：

- Manifest 已升级为 schema v2，增加 SHA-256 摘要链、root digest、报告版本和替代关系。
- 桌面最近导出区域已增加“校验报告包”，分别展示文件完整性、签名和可信时间状态。
- 当前商业状态仍为内部测试：文件匹配不能包装成签名可信、可信时间有效或司法效力。
- 报告重新生成会产生新 `bundleVersion` 并保留 `supersedesReportId`，不得静默覆盖历史报告。
- 在线二维码与签名密钥仅完成设计合同，未形成可售能力。

验证：

- `report_bundle_verification_detects_file_tampering` 可检测 PDF 篡改。
- `report:contract` 检查摘要链、校验入口、替代关系和未签名状态。

风险：

- 未签名 Manifest 可被能够同时修改全部文件的攻击者重算，当前只适用于损坏检测和包内一致性检查。
- 生产签名、报告撤销和在线验证仍依赖未来 KMS/HSM、证书和服务端投入。

下一商业化任务：

- 保持 R2 为 Creator 内部验收能力，进入 R3 移动只读校验；在生产签名服务与安装包门禁完成前，不新增“可信签发”商品承诺。

2026-07-14 Phase R3 Android 跨端校验回写：

- Creator 报告包现可由 Flutter 移动端离线读取 Manifest schema v2，并校验 `report.pdf`、`report.json` 的文件摘要与 SHA-256 链。
- 桌面生成的图片、音频、L2 视频三类报告包已通过主机测试与 Android API 36 运行态验证。
- 移动 UI 明确区分文件匹配、未签名和未加盖报告包可信时间，不新增“可信签发”“司法级认证”或“在线可验证”商品承诺。
- 当前能力继续归类为内部测试；移动端不生成 PDF，生产签名、可信时间、撤销和在线校验均未上线。

验证：

- 主机测试覆盖三媒体匹配和 PDF 篡改检测。
- Android 集成测试完成桌面生成 / 移动校验。
- `report:contract` 与 `dual:consistency-contract` 固定跨端校验边界。

风险：

- 测试 PDF assets 约增加 2.2 MB，不能直接作为正式移动安装包长期交付方式。
- 移动签发交接包 / 桌面校验与最终渲染已完成；iOS QA、生产签名和在线签发仍未完成。

下一商业化任务：

- 在不扩大对外承诺的前提下完成 iOS 交接包生成和桌面导入 QA。

2026-07-14 报告链路异常中断恢复验证：

- Phase R1 常驻 Chromium worker、受控中文字体、三件套原子输出和 3 秒生成门禁均已恢复确认，无需重复实现。
- R1 门禁连续三次通过；图片、音频、L2 视频三类报告包重新生成均低于 1.3 秒。
- Rust 报告合同、报告导出合同、双端一致性合同和移动三媒体完整性测试全部通过。
- 商业能力边界不变：当前仍不能销售或承诺生产签名、可信时间、在线校验、撤销服务或司法认可。

产品决策：

- 3 秒 Chromium 生成门禁已获批准，Windows 干净虚拟机 sidecar 验收不作为当前阻塞任务；该发布风险保留，但不继续占用当前 Phase R3 主线。

2026-07-14 Phase R3 移动签发交接包回写：

- Creator / 单份报告授权用户可在移动端生成未渲染 `report.json + manifest.json` 交接包，并通过系统分享面板转交桌面。
- 桌面可以离线校验移动交接包，并在 Creator `report_export` 权益下生成最终 PDF 三件套。
- UI 明确使用“桌面签发交接包”，没有将其包装成移动正式 PDF、数字签名或可信时间服务。
- Android API 36 与 Rust 桌面 fixture 校验均通过；能力继续归类为内部测试。

下一商业化任务：

- 在 iOS 真机补齐交接包生成和桌面导入 QA；生产签名上线前继续保持内部测试边界。

2026-07-14 Phase R3 桌面导入商业规则回写：

- `import_mobile_report_handoff` 复用现有 `report_export` 权益和使用量记录，生成结果进入桌面最近报告历史。
- 未签名交接包不承担支付授权证明，因此当前不接受仅凭移动单份 purchase grant 在未登录 Creator 桌面环境中生成最终 PDF。
- 最终 Manifest 保存来源 root digest，便于未来云端签发、审计和争议时对照移动提交字节。
- 当前仍未接入 CMS/PAdES、RFC 3161、撤销或在线签发服务，不得把桌面导入写成“可信签发”。

下一商业化任务：

- 设计已登录同账户场景下的单份报告 grant 跨端核销合同，并要求服务端签名授权，而不是信任未签名移动 Manifest。

2026-07-14 Phase R3 Tauri 运行态 QA 与 Phase R4 启动回写：

- 新增 `report:mobile-handoff-runtime-qa`，在 Tauri MockRuntime 中构造真实数据库状态和 Creator 权益，直接使用 Flutter 移动交接 fixture 调用桌面导入核心。
- QA 已完整校验最终 PDF、JSON、Manifest、文件 SHA-256、摘要链和来源 handoff root digest；本次 4 页 PDF 约 746640 bytes，生成耗时 978 ms。
- 运行态 QA 不改变商业能力分类：交接包和最终报告仍未数字签名、未获得报告包可信时间，也没有在线校验或撤销服务。
- Phase R4 已新增案件级 `RightsEvidencePackDocument schema v1` 合同与合成 fixture，商品结构开始从“单条版权报告”分离为“案件材料组织”。
- R4 fixture 强制包含争议对象、侵权样本来源、采集时间状态、附件摘要、自动观察、人工陈述和限制说明。

商业风险：

- “维权证据包”当前仍不能作为已完成的案件取证、公证、鉴定或法律结论商品销售。
- 合成 fixture 和 schema 合同不代表真实采集工具、附件原件保全、签字、律师审查或司法协作已经上线。

下一商业化任务：

- 基于 R4 schema 冻结“维权证据包”首版商品包含项与排除项，并用八页案件级 PDF 原型完成一次律师场景评审后再调整定价页承诺。

2026-07-14 Phase R4 八页案件级原型回写：

- 已使用合成案件 fixture 生成八页高保真 HTML/PDF，覆盖案件封面、证据目录、版权事实、争议对象、采集记录、自动观察、人工陈述、限制说明与附件索引。
- PDF 使用项目受控中文字体，输出 8 页、244274 bytes，分页无溢出。
- 原型固定展示未签名、未加盖报告包可信时间和未形成法律结论，不扩大当前商品承诺。
- 当前仍缺少真实附件原件打包、网页采集、追加式采集日志、签署、律师评审和第三方可信服务。

下一商业化任务：

- 冻结“维权证据包”首版物理交付目录和附件包含项，并据此编写商品页明确包含/排除清单，律师评审完成前不得上线案件级取证承诺。

2026-07-14 Phase R4 案件包物理交付合同回写：

- 首版交付目录固定为 `case.json + case-manifest.json + attachments/`。
- 商品附件角色固定为原件、工作副本、外部对象采集件和外部回执；角色名称不承担真实性或法律效力承诺。
- 采集事件与附件分别建立追加式 SHA-256 链，可检测相对于已知 root digest 的修改、删除、重排和未登记文件。
- 当前 Manifest 仍为 `not_signed / not_timestamped`，摘要链不能包装成防篡改签发或可信取证。
- 合成 fixture 不代表真实附件采集、平台回执验证或原件保管服务已经可售。

下一商业化任务：

- 实现桌面只读案件包校验入口，并基于六类校验状态冻结商品页的“包含文件、可检测问题、明确不保证事项”清单。

2026-07-14 Phase R4 Tauri 只读案件包校验回写：

- 新增 `verify_rights_evidence_pack`，不要求报告导出权益、不修改案件包，只返回六类独立校验状态。
- 校验附件实际字节、采集事件链、附件链、目录白名单、包级 root digest、签名声明和可信时间声明。
- 正常 fixture 与三类篡改场景 Rust 测试通过。
- 当前只有命令和前端类型合同，尚未形成用户可见入口，因此不能宣传桌面产品已提供案件包校验。

下一商业化任务：

- 接入桌面验证页并完成运行态 QA，再据用户可见状态冻结维权证据包商品的完整性检查说明。

2026-07-14 Phase R4 案件包运行态 QA 与桌面入口回写：

- MockRuntime QA 通过真实 IPC 命令验证 camelCase JSON、六类状态、附件逐项结果和 root digest 对照。
- 桌面验证页已增加“维权证据包完整性”入口，不受 Creator 报告导出权益限制。
- UI 将 matched、未签名和未加盖可信时间分开展示，并固定“不读取媒体水印、不判断侵权”的边界。
- 当前仍为内部测试，尚未完成 Flutter 跨端复算、安装包人工 QA 或律师评审。

下一商业化任务：

- 完成 Flutter / Android 案件包只读校验后，冻结商品页可承诺的离线完整性检查范围和失败提示。

2026-07-14 Phase R4 Flutter / Android 案件包完整性 QA 回写：

- Flutter 主机与 Android API 36 已对桌面生成的同一案件包 fixture 完成只读复算。
- 六状态命名、附件逐项结果和包级 root digest 与桌面一致；Android 运行态未要求 Creator 权益。
- 当前 fixture 通过测试 assets 注入 APK，仅证明算法和序列化跨端一致，不代表真实用户目录访问、文件授权和发布安装包已经验收。
- 商品页仍不得承诺数字签名、可信时间、附件来源真实性、侵权认定或媒体水印重验。
- “跨端离线案件包完整性检查”继续保持内部测试，等待移动端真实目录入口、安装包 QA 和失败提示评审。

下一商业化任务：

- 在移动端加入真实目录选择和六状态只读展示，完成 Android 外部存储案件包 QA 后，冻结商品页“可检测问题 / 明确不保证事项”文案。

2026-07-15 Phase R4 移动目录入口与外部存储 QA 回写：

- 移动验证页已提供案件包目录选择和六状态只读展示，不要求 Creator 权益。
- Android API 36 已从应用专属外部目录读取主机推入的六个物理文件，不再依赖 APK 测试 assets。
- UI 固定展示未签名、未加盖可信时间，并明确不判断侵权、签发主体或采集时间可信。
- 当前尚未完成任意共享目录的 SAF 持久授权、发布安装包文件管理器矩阵和律师文案评审，因此仍不升级商品承诺。

下一商业化任务：

- 完成 Android SAF tree URI 与真实 Download 案件包选择 QA，再冻结商品页“离线完整性检查”的支持目录、失败提示和明确排除项。

2026-07-15 Phase R4 Android SAF Download QA 回写：

- Android 已完成 Download 案件包的系统文件选择器点击授权、只读校验和应用重启后授权复用。
- 该能力不要求 Creator 权益，未引入订阅、支付或新的商业权限判断。
- 商品承诺仍保持内部测试：当前只覆盖系统 DocumentsUI 的本地 Download，不承诺所有网盘、厂商文件管理器和企业文档 Provider。
- 数字签名、可信时间、附件来源真实性、媒体水印重验和法律结论仍是明确排除项。

下一商业化任务：

- 在支持矩阵完成至少一个第三方 DocumentsProvider 和授权撤销失败 QA 后，冻结“离线案件包完整性检查”的商品页支持范围与错误提示。

2026-07-15 Phase R4 SAF 失败提示与 Provider 矩阵回写：

- Android 已完成四类失败提示和恢复路径门禁，用户不再只看到通用异常文本。
- 独立 QA DocumentsProvider 已证明非 Download tree URI 的读取与 Provider 下线分类可运行。
- 当前仍不升级商业承诺：内部 QA Provider 不能替代真实云盘、OEM 文件管理器、iOS File Provider 或发布安装包矩阵。
- 商品文案可内部冻结四类提示，但对外仍不得承诺任意云盘兼容、签名可信、可信时间、水印重验或法律结论。

下一商业化任务：

- 使用一个真实 Android 云盘 Provider 和 iOS File Provider 完成安装包 QA 后，评审是否将“本地与受支持 Provider 的离线完整性检查”升级为附属用户能力。

### 2026-07-17 桌面全局工作区宽度与商业页面布局对齐

状态：`已完成`

已完成：

- 全局 `AppShell` 从 `8fr / 4fr` 比例列调整为“主工作区弹性填充 + 右侧上下文 `360–420px`”，消除右侧列内部未被上下文面板使用的空白。
- `1920×1080` 下主工作区宽度由 `1066.66px` 增加到 `1180px`，主工作区与上下文面板的实际间距由 `129.33px` 收敛为 `16px`。
- 工作台、处理、验证、版权库、批量队列、年度授权、设置、帮助与能力边界八个菜单已完成修改前后截图对照。
- 年度授权、报告购买、同步状态和权益上下文仅调整布局，没有改变“图片/音频年度基础权益 + 报告逐份付费 + 未来视频独立收费”的商业合同。
- 增加主工作区与上下文面板的通用长文本换行规则，长版权编号、文件名、摘要和说明不得撑破全局网格。
- DropZone 独立增加宽度收缩和文件名强制换行规则；长音频文件名的视觉重叠由 `33.73px` 降为 `0px`。

验证：

- 八个菜单的 document、主工作区和上下文面板横向溢出均为 `0px`。
- `npm run build`：通过。
- `npm run release:desktop-baseline`：通过。
- `npm run commercial:contract`：通过。
- `git diff --check`：通过。
- 对比证据：`tmp/release-qa/appshell-grid-20260717/appshell-grid-comparison.jpg`。

风险：

- 本轮是浏览器 Mock 页面级布局证据，仍需在桌面 Release/WebView2 中复核系统缩放和窗口拖拽。
- 本轮不改变套餐、注册码、报告购买或云端 entitlement 行为。

下一商业化任务：

- 使用桌面 Release 在 Windows `100%`、`125%` 缩放下复核年度授权、版权库报告入口和未付费批量队列的主区/上下文对齐。

### 2026-07-17 基础存证摘要字段矩阵 V1

状态：`字段合同已冻结；代码与数据库迁移待实施`

已完成：

- 新增 `docs/基础存证摘要字段矩阵.md`，冻结基础存证摘要、版权证据技术报告和维权证据包三层字段边界。
- 基础摘要固定为免费能力，对未付费和年度基础权益用户展示相同字段，不受 `report_export` 控制。
- 正式报告继续按 `copyright_report_single` 和 `rights_evidence_pack_single` 记录级 purchase grant 授权。
- 字段分为默认展示、条件展示、付费报告、永久禁止四类，并标记现有、派生、API 暴露、数据库迁移和报告模型五种数据支持状态。
- 冻结 P0 正确性、P1 批量与媒体追溯、P2 付费报告可复验性三阶段实施顺序。

商业边界：

- 基础摘要不展示价格、套餐、购买状态或营销文案，避免证据事实投影与商业 CTA 混合。
- 原始诊断、完整过程链、软件版本、回执摘要、签名和 Manifest 保持付费报告价值。
- 年度注册码不包含任何报告授权。

验证：

- `npm run commercial:contract`：通过。
- `npm run release:desktop-baseline`：通过。
- 字段矩阵结构检查：通过。
- `git diff --check`：通过。

风险：

- 当前代码仍输出旧标题、“可信时间（本机时间）”、“登记收据：未记录”和 `Payload ... bytes` 等未分层字段。
- 批次追溯、媒体格式和软件版本需要数据库迁移，不得只在前端临时拼接。

下一商业化任务：

- 按矩阵 P0 改造 `buildCopyrightSummary`，先完成时间、身份、登记、验证说明、隐私页脚和空行过滤，不同时实施 P1 / P2 数据库扩展。

2026-07-15 商业化阻塞与 CDKEY 设计决策：

- `BLOCK-R4-PROVIDER-01` 已按产品决策设为阻塞：真实 Android 云盘 Provider 与 iOS File Provider 暂不推进。
- 在阻塞解除前，案件包 Provider 兼容继续保持 `只能内部测试`，不得升级商品承诺。
- 新增 `docs/CDKEY离线激活与本地许可证设计.md`，规划不依赖真实支付渠道的本地签发和离线激活。
- 首期只允许 `Creator（离线授权）` 开放 `batch_processing` 与 `report_export`。
- `cloud_sync`、`cloud_batch_processing`、`cloud_video_processing`、`priority_queue`、`team_workspace`、`api_access` 继续由服务端 entitlement / quota / membership 权威控制。
- 设计采用 Ed25519 非对称签名、单安装实例绑定、365 天有效期和内部离线 Rust CLI；不采用客户端内置对称秘密或传统短 CDKEY。
- 当前仅完成设计，尚未实现签发工具、激活 UI、许可证存储或权益合并。

下一商业化任务：

- 执行 Phase K0：冻结离线许可证 request / license / revocation schema v1、canonical encoding、签名域、错误码和跨语言测试向量。

2026-07-15 Phase K0 HSLIC1 主载体与跨端测试向量：

- Phase K0 状态更新为 `进行中`，主许可证载体子项已完成。
- 主载体正式冻结为 `HSLIC1.<payload>.<signature>`，长度合同为 300–500 个 ASCII 字符。
- license payload v1 使用 8 个固定字段和受限 canonical JSON，不允许未知字段、空白格式化或任意 feature map。
- `creator_offline` 权益继续由客户端固定 allowlist 映射，token 不能声明云同步、云视频、团队空间或 API。
- Ed25519 固定向量长度为 454 字符，公开测试 seed 仅用于测试，禁止用于真实签发。
- TypeScript、Rust、Dart 已对同一 fixture 得到一致字段结果并验证同一签名。
- 合法重编码后的字段修改在三端均返回签名无效。

验收结果：

- `npm run license:k0-cross-end`：通过。
- `npm run build`：通过。
- `npm run commercial:contract`：通过。
- `flutter analyze lib/licensing/offline_license.dart test/offline_license_test.dart`：通过。
- Rust 编译仍输出既有 unused/dead-code warning，本次未扩大处理。

当前风险：

- activation request schema v1、revocation list schema v1 和完整错误响应向量尚未冻结。
- 尚无正式签发密钥、签发 CLI、安全存储、激活页面或中央权益合并器。
- 本地二进制仍可能被管理员级攻击者修改，签名合同不能等同于不可破解 DRM。

下一商业化任务：

- 完成 Phase K0 剩余合同：冻结 `HSREQ1` 激活请求、签名撤销列表和跨端错误测试向量，然后才能进入 Phase K1 内部签发 CLI。

2026-07-15 Phase K0 完成与 Phase K1 内部签发 CLI：

- Phase K0 状态更新为 `已完成`。
- `HSREQ1.<payload>.<checksum>` 已冻结，checksum 使用域分隔 SHA-256 前 96 bit，只承担传输错误检测。
- `HSRVL1.<payload>.<signature>` 已冻结，撤销 ID 必须排序去重，sequence 从 1 单调递增。
- Rust、TypeScript、Dart 已对 16 条共享错误向量返回完全一致错误码。
- Phase K1 内部最小签发 CLI 已完成，二进制为 `offline_license_issuer`。
- CLI 支持加密密钥生成、请求检查、许可证签发/验证、撤销列表签发/验证及审计输出。
- 私钥采用 Argon2id + XChaCha20-Poly1305 加密，密码不进入命令行参数。
- CLI 没有 feature map 或云能力参数，只能签发 `creator_offline`。

验收结果：

- `npm run license:k0-cross-end`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml offline_license`：通过，2 个合同测试与 3 个签发器测试通过。
- `npm run license:k1-cli-qa`：通过。
- K1 运行态生成 474 字符许可证与 381 字符撤销列表，公钥验证均通过。
- 错误密码、未知模板参数和非法请求均被稳定拒绝。
- 签发与撤销审计文件均生成。

当前风险：

- 尚未建立生产签发账户、NTFS ACL、正式密钥备份、双人审批、HSM 或密钥轮换操作手册。
- 用户端尚未生成真实 installation identity，也没有许可证导入、安全存储和中央权益合并。
- 完全离线许可证仍不能即时获知撤销列表更新。

下一商业化任务：

- 进入 Phase K2：在 Tauri 实现 installation identity、`HSREQ1` 导出、`HSLIC1` 导入、安全存储和中央本地权益合并，同时保持全部云能力服务端权威。

### 2026-07-15 Phase K3 Flutter / 移动端离线激活

状态：`代码完成；等待 Android / iOS 真机验收`

已完成：

- Flutter 使用冻结的 installation identity 派生、`HSREQ1` / `HSLIC1` / `HSRVL1` parser 和错误合同。
- Android / iOS 的 installation secret、salt、许可证 token 和撤销列表 token 由 `flutter_secure_storage` 保存；Web 和其他平台 fail closed。
- 公钥 ring 只从 `--dart-define HIDDENSHIELD_OFFLINE_LICENSE_PUBLIC_KEYS_JSON=...` 注入，默认空 ring。
- 设置页支持创建、复制、分享和二维码展示同一原始 `HSREQ1`，并通过文件、粘贴或二维码导入同一原始许可证 / 撤销载荷。
- SQLite 只保存许可证状态镜像和审计，不把权威秘密或 token 写入 `SyncProfile`。
- 离线 Creator 只合并 `batch_processing` 和 `report_export`；所有云能力继续只认服务端 entitlement。
- onboarding 新增“仅使用本机”路径。

验证：

- `flutter analyze`：通过，0 issues。
- `flutter test test/offline_license_test.dart test/licensing_k3_test.dart`：通过，8 tests。
- `flutter test`：用户中止，未取得完整套件结果；残留测试进程已停止。

风险：

- 尚未完成 Android / iOS 真机 KeyStore / Keychain、相机二维码和系统文件分享验收。
- 默认公钥 ring 为空；未注入正式公钥的构建会拒绝所有许可证。
- 完全离线撤销仍依赖用户导入更新后的签名撤销列表。

下一商业化任务：

- 用内部签发器对 Android 或 iOS 真机导出的 `HSREQ1` 签发一份测试许可证，注入对应非 fixture 公钥，完成单安装绑定、到期、撤销和重启后持久化 QA。

2026-07-15 Phase K2 / K4 桌面激活与安全发布门禁：

- Phase K2 桌面内部最小集已完成：OS keyring installation secret、migration 19 状态/撤销/追加式审计、5 个 Tauri 命令、设置页导入导出和中央权益解析器均已接入。
- 生产 Tauri 不再注册 `set_entitlement_state`；直接调用本地批量或正式报告也必须重新验证签名许可证，SQLite feature map 不具备授权权威。
- 离线 Creator 只开放 `batch_processing` 和 `report_export`；云同步、云批量、云视频、优先队列、团队空间和 API 保持服务端权威。
- Phase K4 冻结共享 trust policy、公钥状态/用途/有效期、撤销 sequence + digest 高水位和 300 秒可信时间回拨阈值。
- K1 审计补齐强制操作员标识、独立序列号、canonical payload SHA-256、结果字段及可选替换关系/原因；这些转移字段只进入审计，不修改冻结的 v1 token。
- migration 20 保存桌面最高可信 UTC；Flutter 在平台安全存储保存同一高水位。双端均拒绝撤销回放与同序列号分叉，并记录许可证替换审计。
- 正式发布完整性由 OS 安装包签名链承担，进程内 self-hash 不作为权益权威。

验收结果：

- `npm run build`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml --lib --features internal-qa`：通过。
- Rust K2/K4 定向测试：中央权益 10 tests、migration 20、identity 复制测试通过；许可证状态与 acceptance audit 已验证事务原子性。
- `flutter analyze`：通过。
- Flutter K3/K4 聚焦测试：12 tests 通过，包含双 keyId 撤销轮换与未来 `issuedAt` 拒绝。
- `npm run license:k4-contract`：3 个 key、11 条策略向量通过。

当前风险：

- 能力仍分类为 `只能内部测试`，尚无真实签发公钥和正式签名安装包证据。
- v1 token 不含应用最低版本字段；不得静默改变 v1，未来版本门禁只能通过 v2 schema 增加。
- 完全离线设备不能实时获得撤销更新，也不能可靠抵抗管理员回滚整个系统快照。

下一商业化任务：

- 生成一套非 fixture 内部签发密钥并配置桌面编译期 trust policy 与移动 `--dart-define`，对 Windows 签名候选包和 Android release 候选包各完成一次真实 `HSREQ1 -> HSLIC1 -> 重启 -> 撤销` QA。

2026-07-15 Phase K4 内部 trust policy 与候选包真实 QA：

- 已生成非 fixture 内部 QA 签发密钥，私钥与密码仅保存在忽略目录 `tmp/offline-license-internal-qa/20260715-090347/`，仓库只新增公钥 trust policy `config/offline-license-trust-policy.internal-qa.json`。
- 统一 trust policy 使用 `offline-internal-2026-q3-qa`，公钥 `0eTxbCnZtqslZm1q3DuSXH7mUmpydpg7zNWdZ93uAV0`，用途限定为 `license` / `revocation`，并要求 OS 包签名链作为发布完整性权威。
- Android release APK 已用该 trust policy 完成真实 `HSREQ1 -> HSLIC1 -> 重启 -> HSRVL1 撤销 -> 第二安装实例设备不匹配` QA：有效许可证显示 `Creator（离线授权）`，重启后保持有效，导入撤销列表后显示“已撤销”，清空 App 数据形成第二安装实例后导入第一实例许可证返回“许可证绑定到另一安装实例，不能在本机使用。”。
- Android 候选包：`mobile_app/build/app/outputs/flutter-apk/app-release.apk`，SHA-256 `8AEAA93F0F8AD70E229C044959F6227BB58F155C11C8DD1E6DA312E180C5A349`；`apksigner verify --print-certs` 通过但证书为 Android Debug certificate，因此仍不能视为生产发布签名。
- Windows 候选包阻塞：`src-tauri/target/release/bundle/nsis/HiddenShield_0.1.0_x64-setup.exe` SHA-256 `28EED6A24C5BB279E9D0A6367833C956A482B3AC4DC06A1D704D9A1E9B8D97DD`，`hidden_shield.exe` SHA-256 `DE9D3390C8771A3928536EAF90AD3C5F3C5A8A9E3BC75702F13411617FBFC750`，二者 `Get-AuthenticodeSignature` 均为 `NotSigned`，不得声明 Windows Authenticode 候选包 QA 通过。

验收结果：

- Android release 候选包内部 QA：通过，但仅限 debug certificate 签名的 release APK。
- Windows Authenticode 候选包 QA：blocked，原因是候选安装包与 exe 未签名。
- QA 证据、真实 HSREQ/HSLIC/HSRVL、审计 JSON 与截图/XML 保存在 `tmp/offline-license-internal-qa/20260715-090347/`；不得提交私钥、密码或许可证 token。

下一商业化任务：

- 生成或提供 Authenticode 签名的 Windows 安装候选包，以及生产/内部分发 Android release keystore 签名 APK 后，复跑同一 `HSREQ1 -> HSLIC1 -> 重启 -> 撤销 -> 交叉设备不匹配` 门禁，再评审是否从 `只能内部测试` 升级。

2026-07-15 Phase K4 签名候选包复跑结果：

- Windows Authenticode 内部 QA 签名完成：`src-tauri/target/release/hidden_shield.exe` SHA-256 `37701E70499FA8A744FF1CEEC68E96DDA5CEC16A546FCB97F3C0222D16222BF6`，`src-tauri/target/release/bundle/nsis/HiddenShield_0.1.0_x64-setup.exe` SHA-256 `D64C1DC75F4A9E1B070C8906FEC95AD87D2DF475C140A83402B3CEBD77F9A9A3`，`Get-AuthenticodeSignature` 均为 `Valid`；证书 subject `CN=HiddenShield Internal QA Code Signing 2026 Q3, O=HiddenShield Internal QA, C=CN`，thumbprint `86E012CE09DBDA9853A7F8E164233E9952019625`。
- Android release APK 已使用非 debug 内部 QA release keystore 签名：`mobile_app/build/app/outputs/flutter-apk/app-release.apk` SHA-256 `4D939CA2FC34A7C0FED76198F41A6DA9627D1D930D60B704D4FC19B358316565`；`apksigner verify --print-certs` 通过，signer DN `CN=HiddenShield Android Internal QA Release 2026 Q3, O=HiddenShield Internal QA, C=CN`，signer SHA-256 digest `bf7fc80e1d130fc592fce4b8277bf75d0eb9b2dfe8923b90946a102c78db4b95`。
- Windows 桌面 runtime 与 Android release 候选包均使用统一 trust policy `config/offline-license-trust-policy.internal-qa.json` 和 keyId `offline-internal-2026-q3-qa` 完成真实 `HSREQ1 -> HSLIC1 -> 重启 -> HSRVL1 撤销 -> 交叉设备不匹配` QA。
- Windows 证据：`tmp/offline-license-internal-qa/20260715-195231/windows-final-runtime-qa-evidence.json`，license `lic_bc3fa92606df5ba05292c699`，重启后 `active`，撤销后 `revoked`，第二安装实例导入返回 `offline_license_device_mismatch`。
- Android 证据：`tmp/offline-license-internal-qa/20260715-195231/android-final-real.hsreq`、`android-final-license-audit.json`、`android-final-revocation-audit.json`、`android-final-05-license-imported.png`、`android-final-06-after-restart.png`、`android-final-11-revocation-imported.png`、`android-final-19-cross-device-mismatch.png`；license `lic_5c63a53141f6b5633046a3d4`，撤销列表 `rvl_android_final_20260715 / #3`。
- 总汇总：`tmp/offline-license-internal-qa/20260715-195231/final-signed-package-license-qa-summary.json`。

状态与风险：

- 本轮解除“未签名 Windows 候选包 / Android debug certificate”阻断，但能力仍分类为 `只能内部测试`。
- Windows 使用内部 QA 自签 Authenticode 证书，Android 使用内部 QA release keystore；二者均不是公开 CA、Play/App Store 或正式企业分发签名材料。
- 私钥、签发密码、Android keystore、license token 和 revocation token 均在忽略目录保留为 QA 证据，不得提交或作为生产材料。
- 商业化升级仍缺订单、退款、客服迁移、密钥托管、密钥轮换、撤销分发、正式证书/keystore 管理和法务交付闭环。

下一商业化任务：

- 用正式生产/企业分发签名材料替换内部 QA 证书与 keystore，在干净 Windows VM 和 fresh Android 设备上复跑同一门禁，并把订单、退款、客服迁移和密钥托管流程纳入 Phase K5 验收。

## 2026-07-17 基础存证摘要 P0 投影

状态：`桌面摘要投影已完成；数据库与付费报告模板未启动`

- `buildCopyrightSummary` 已切换为 `HS-SUMMARY-1` 的本地版权记录口径，明确基础摘要不是第三方公证、官方登记或法律权属结论。
- 创作者身份改为“显示名称 + 用户本地声明 + 未进行实名认证”，避免把用户填写名称表达为平台核验身份。
- 本机时间、网络授时和第三方时间戳回执已分开表达；无 TSA 时不再输出“可信时间”。
- 等待联网登记时不再展示空登记收据；只有已确认登记且存在真实收据时才展示登记收据编号。
- 写后回读验证不再使用“可取证”结论；基础摘要继续对未付费与年度授权用户保持相同，不新增 `report_export` 权益。
- 隐私页脚已明确原始媒体和保护副本未上传，同时保留版权元数据可能按同步设置或联网登记状态同步的边界。
- 本次未启动数据库迁移、`VaultRecord` 字段扩展、IPC/API 扩展、正式报告模板、P1/P2 或视频字段工作。

验证：

- `npm run release:desktop-baseline` 增加 P0 静态回归合同，覆盖新标题、身份边界、时间语义、登记收据、验证说明、隐私页脚和空行过滤。

风险：

- 当前 `VaultRecord` 尚未暴露数据库已有 `file_type`，因此媒体类型和图片/音频专属参数仍不能按冻结矩阵完整条件化展示。
- 图片与音频真实记录摘要 fixture 尚未建立，当前验证以类型构建和发布合同为主。

下一商业化任务：

- 建立一组图片与音频真实摘要 fixture，固定未付费与年度授权用户完全相同的 `HS-SUMMARY-1` 输出，再评审是否启动 `file_type` 的只读 API 暴露。

## 2026-07-17 基础存证 P0 UI 对齐

状态：`桌面处理结果、版权卡与版权库详情已对齐；数据库未变更`

- 音频处理结果不再展示 `0x0` 分辨率和 `0fps`，改为音频时长与源文件 / 保护副本大小。
- 音频文件增大提示改为“保护副本采用可稳定验证的音频格式”，不再使用图片“画质”口径。
- 处理结果、版权卡和版权库历史详情统一使用创作者显示名称、用户本地声明、保护副本验证、版权编号生成方式、联网登记状态、第三方时间证明、水印协议版本和载荷完整性校验。
- 页面不再展示 payload 字节长度、原始枚举 `offline_generated / pending_registration / verified` 或“保护副本可取证”成功说明。
- 等待联网登记不展示空收据；无 TSA 时不展示可信时间；历史成功记录在展示层归一化，不修改数据库存量消息。
- 本次未修改权益、价格、正式报告购买、数据库、Rust model、IPC/API、移动端或视频能力。

验证：

- `npm run build`：通过。
- `npm run release:desktop-baseline`：通过，新增三组件 P0 UI 静态合同。

下一商业化任务：

- 使用真实 MP3 与真实图片各处理一次，人工确认结果页、版权卡和版权库详情三处字段一致，再固化图片 / 音频页面级截图证据。

### 2026-07-17 P0 UI 最终收口

- 图片处理结果不再展示无意义的 `0fps`；帧率行仅保留给已冻结的视频类型内部兼容分支。
- 版权记录相关处理时间、验证时间、网络授时时间和可信时间统一为 `YYYY-MM-DD HH:mm:ss`。
- FreeTSA endpoint 在基础 UI 中显示为“FreeTSA 时间戳服务”，不再暴露原始接口 URL。
- 本次未修改数据库、Rust model、IPC/API、权益、报告购买或 `watermark-core`。
- `npm run build`、`npm run release:desktop-baseline`：通过。

下一商业化任务：

- 重新构建桌面 Release，用真实图片和音频复验最终展示并保存页面级截图证据。

## 2026-07-17 桌面单列工作区基线

状态：`右侧工作台上下文已退役；八菜单布局 QA 通过`

- 桌面 `AppShell` 已删除全局 `ContextPanel`、`context` 属性和右侧渲染，主工作区改为单列 `minmax(0, 1fr)`。
- `App.vue` 不再构建每个菜单的 `WorkspaceContext`；`workspace-context.ts` 只保留未付费 / 图片音频年费标签函数。
- 右侧上下文中的方案、报告、云同步和快捷跳转均为重复展示，删除后不影响处理、验证、版权库、批量、年度授权、设置或帮助的真实业务能力。
- 年度授权 embedded 面板已修复基于整个视口计算宽度的问题，在左导航存在时按主工作区宽度收缩。
- 宽屏 `1600×1000` 与窄屏 `1024×768` 下，八个菜单均无右侧空白列、无横向溢出、无浏览器控制台错误。
- QA 截图保存在 `tmp-ui-qa/single-column-shell/wide/` 与 `tmp-ui-qa/single-column-shell/narrow/`。

验证：

- `npm run build`：通过。
- `npm run release:desktop-baseline`：通过。
- `npm run commercial:contract`：通过。

下一商业化任务：

- 用 Tauri Release 对工作台、处理、验证、版权库、批量队列、年度授权、设置和帮助执行一次同尺寸人工截图复验，确认 Web mock 与桌面 WebView 布局一致。
## 2026-07-22 桌面高位深音频产品口径回写

- 状态：`桌面高位深音频 Gate 通过；不改变当前 RC / GA 许可证与签名状态`。
- 已完成：当前 0.1.0 源码重新构建并安装，真实 `24-bit WAV / 24-bit FLAC / float32 WAV` 的 mono / stereo 共 `6 / 6` 完成写入、写后回读、独立核心读取、只读验证和量化统计。
- 产品口径：桌面端可承诺上述高位深输入统一输出 WAV，并保持源采样率、声道、整数 / 浮点类型和有效位深；不得扩展为移动端、float64、全部 32-bit integer 容器或零差异音频承诺。
- 验证：安装版构建 Gate 为 `artifacts/desktop-installer-self-contained/20260722-high-bit-depth-v2/desktop-installer-self-contained-gate.json`；媒体证据为 `artifacts/desktop-high-bit-depth-audio-gate/20260722-final/summary.json`。
- 风险：首个候选真实暴露 24-bit 被升格为 32-bit 的 FFprobe 映射缺陷，现已修复并增加回归；20 分钟、512 MiB 与图片 100 MP 边界仍未完成。
- 下一 Roadmap 任务：运行桌面 PNG / JPEG / WebP 常规尺寸与接近 100 MP 的安装版资源 Gate，并把结果映射回 `docs/桌面媒体正式支持范围.md`。

## 2026-07-22 桌面图片正式发布边界回写

- 状态：`桌面图片格式、像素、文件大小、输出与资源 Gate 通过；不改变当前 RC / GA 签名和许可证状态`。
- 已完成：PNG / JPEG / WebP 常规尺寸 `9 / 9`、约 99.92 MP `3 / 3`、精确 512 MiB `1 / 1` 完成安装版写入、写后回读、独立核心读取和只读验证。
- 拒绝边界：100 MP + 1 与 512 MiB + 1 byte 均拒绝；桌面选择器与运行时只开放正式三格式。
- 产品口径：统一输出 PNG 并保持尺寸；50–100 MP 为高资源大图模式，建议至少 16 GiB 内存，不承诺普通图片级时延。
- 风险：近 100 MP 产品管线约 17–19 分钟、峰值 6.25–6.57 GiB；该能力不能外推到移动端或低内存设备。
- 验证：完整资源证据为 `artifacts/desktop-image-resource-gate/20260722-final/summary.json`，最终候选冒烟为 `artifacts/desktop-image-resource-gate/20260722-final-candidate-smoke/summary.json`，安装候选证据为 `artifacts/desktop-installer-self-contained/20260722-image-resource-final/desktop-installer-self-contained-gate.json`。
- 下一 Roadmap 任务：执行桌面音频 20 分钟 / 512 MiB 允许与拒绝 Gate，完成媒体资源边界收口。

## 2026-07-22 V3 图片强裁切能力重新阻断发布

- 状态：`资源 Gate 保持通过；图片取证布局重新进入 internal-QA，内部 RC 不应把 1/8 或 1/16 裁切恢复作为已完成能力`。
- 产品决策：正式图片协议只面向 V3；载荷协议与图片承载布局独立版本化，当前布局候选为 `spatial-recovery-v1`。
- 核心候选结果：保持位置推导、`HSR1` layout ID 和 V3 UID 不变，将恢复包替换为局部 Haar 变换域承载；1920×1080 十六宫格裁切 `16 / 16`、36 个关键边界滑动 `1/16`、四类干净图误报和 PNG→JPEG/WebP 重编码恢复 Gate 均通过。
- 桌面链路：共享核心已经接入 `WatermarkService::embed_v3` 和正式只读验证；三张安装版真实摄影照片 PSNR `44.19–51.59 dB`、SSIM `0.9952–0.9982`，每张十六宫格 `16 / 16` 与滑动裁切 `36 / 36` 均通过。
- 资源结果：`9992×10000` 近 100 MP 安装版样本约 `20.61 分钟`、峰值 `6.58 GiB`，独立核心和安装版只读验证均通过。
- 安装候选说明：NSIS / MSI 与安装目录均成功生成；旧 `desktop-installer-self-contained` UI 探测曾因页面 URL 瞬时为 `about:blank` 标记失败，但正文已经加载，随后新安装版 Gate 在 `http://tauri.localhost/` 正常加载并完成全部媒体验证。该旧 Gate 记录不改写为通过。
- 限制：仍没有缩放、旋转、严重重压缩和统计规模误报证据。
- 商业风险：当前结果仍是 internal-QA，宣传强裁切恢复会超过真实能力；原有 PNG / JPEG / WebP、100 MP、512 MiB 等资源口径可以保留，但不能等同于完整取证发布就绪。
- 证据：`artifacts/desktop-image-spatial-recovery-gate/20260722-local-transform/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-visual/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-100mp/summary.json`、`artifacts/desktop-installer-self-contained/20260722-spatial-recovery/desktop-installer-self-contained-gate.json`。
- 下一 Roadmap 任务：保持移动端和音频任务暂停，执行缩放、旋转、更低质量 JPEG/WebP 重压缩和至少 100 张干净照片误报 Gate；通过后评审桌面强裁切用户承诺与内部 RC。

## 2026-07-22 桌面 V3 图片正式发布边界闭合

- 状态：`图片算法、桌面消费链、安装器、视觉质量、恢复矩阵、误报统计和近 100 MP 资源 Gate 通过；进入内部 RC 评审，不自动改变签名与许可证结论`。
- 核心：正式 API 去除图片 `internal_qa` 命名；`spatial-recovery-v1` 使用固定魔数、分散排列、逆排列共识、流式扫描和受限软纠错，桌面所有正式路径统一消费 `WatermarkService`。
- 用户口径：桌面静态 PNG/JPEG/WebP 统一输出同尺寸 PNG；支持轴对齐宽高各 `1/4` 的裁切区域恢复，以及 90/180/270 度旋转、85% 缩放、JPEG/WebP quality 75/60 的独立恢复。
- 安装版结果：三张真实照片每张 `16/16` 十六宫格、`36/36` 关键滑动裁切、`8/8` 变换恢复全部通过；PSNR `44.11–51.29 dB`、SSIM `0.9951–0.9981`。
- 误报与性能：34 个 Windows 内置图片源生成三格式 `102` 个干净变体，误报 `0`，平均 `184 ms`；近 100 MP 产品处理约 `6.36 秒`、端到端约 `12.70 秒`、峰值约 `0.70 GiB`。
- 风险：当前证据不覆盖组合扰动、任意角度、低于 quality 60、低于 80% 缩放、动画图片或移动端同等能力；不得将这些边界扩大为营销承诺。
- 验证：`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final-installed/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final/false-positive-summary.json`、`artifacts/desktop-installer-self-contained/20260722-image-complete-final/desktop-installer-self-contained-gate.json`。
- 下一 Roadmap 任务：冻结图片算法与用户口径，使用最终安装版执行桌面内部 RC 评审；RC 通过后恢复音频 `20 分钟 / 512 MiB` 资源 Gate。

## 2026-07-22 桌面正式媒体资源边界收口

- 状态：图片算法与产品口径冻结；桌面音频资源 Gate 完成；移动端继续暂停。
- 已完成：桌面音频 `30 秒–20 分钟`、`≤512 MiB`、`8–48 kHz`、mono / stereo 的前端预检、共享核心限制和不可绕过执行校验。
- 安装版结果：精确 `20:00` 与 `512 MiB` 均完成写入、写后回读、独立核心读取和只读验证；`20:01` 与 `512 MiB + 1 byte` 均拒绝且不创建版权记录。
- 商业口径：桌面可以按上述边界描述正式音频能力；不得声称音频使用图片空间恢复算法，不得把桌面资源结果外推到移动端，也不得承诺超限素材可通过转换规避边界。
- 发布风险：当前资源上限分别验证，没有把 `20 分钟 + 512 MiB + 48 kHz + stereo + 高位深` 叠加为单一极端 SLA；安装器离线 Gate 仍保留“宿主机已有 WebView2、未物理断网”的 GA 环境限制。
- 证据：`artifacts/desktop-audio-resource-gate/20260722-final-v2/summary.json`、`artifacts/desktop-installer-self-contained/20260722-audio-resource-v2/desktop-installer-self-contained-gate.json`。
- 下一 Roadmap 任务：基于冻结图片证据与本次音频证据执行桌面媒体内部 RC 评审，形成单一 RC 摘要和发布阻断项清单。

## 2026-07-22 桌面媒体内部 RC 评审结果

- 结论：`BLOCKED`，不批准当前 `0.1.0` 桌面媒体 RC，也不批准对外发布。
- Critical 媒体阻断：最终组合候选在真实照片 WebP q60 变体上返回错误 UID。
- High 媒体阻断：默认 `watermark-core` 完整测试为 `110 passed / 7 failed`；五格式 mono / stereo 未形成最终安装候选 Gate；音频合法上包络组合未验证。
- Critical 发布阻断：NSIS、MSI、release exe 和 installed exe 均未完成 Authenticode 签名。
- High 发布阻断：尚未在无预装 WebView2、物理断网的干净 Windows VM 完成安装和媒体验证。
- 移动端继续冻结，不影响本轮阻断判定，也不得作为桌面缺口的替代证据。
- 统一摘要：`docs/桌面媒体内部RC评审.md`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。
- 下一 Roadmap 任务：优先关闭 WebP q60 错误 UID 阻断；随后在同一候选上重跑图片完整 Gate、音频五格式安装版 Gate和高位深上包络 Gate，再进入签名与干净离线 VM 验证。

## 2026-07-22 RC-MEDIA-001 共识读取修复

- 状态：共享核心错误 UID 根因已定位并修复；桌面 WebP q60 承诺保持，不收窄到 q75；整体桌面媒体 RC 仍为 `BLOCKED`。
- 根因：WebP q60 使首个独立包 UID 位 `73`、`95` 翻转但碰撞旧 8-bit 校验，而精确读取器在 25 包共识前提前返回该包。
- 实现：`watermark-core` 精确读取改为共识优先，桌面消费方无私有兜底。固定真实照片三 UID 回归 `3/3`，新安装候选综合图片 Gate、102 样本误报 Gate和架构契约通过。
- 候选：installed exe SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`；安装器 Gate 状态 `passed_with_ga_environment_limitations`。
- 未解除项：RC-MEDIA-001 仍需按原条件补三照片 × 三 UID × 八变换；RC-MEDIA-002 至 RC-RELEASE-002 均未关闭，不能批准内部 RC 或对外发布。
- 证据：`artifacts/image-webp-q60-uid-regression/20260722-green/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-webp-q60-core-fix-installed/summary.json`、`artifacts/desktop-installer-self-contained/20260722-webp-q60-core-fix/desktop-installer-self-contained-gate.json`。
- 下一 Roadmap 任务：先补 RC-MEDIA-001 完整三照片 × 三 UID × 八变换关闭矩阵，再处理默认核心测试红灯和音频最终安装候选缺口。

## 2026-07-22 RC-MEDIA-001 正式关闭

- 状态：`CLOSED`。三张真实照片 × 每张三个独立 UID × 八个承诺变换共 `72/72` 通过。
- 同一候选：installed exe SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`；三轮证据均引用该文件。
- 精确性：独立共享核心读取 `72/72`，安装版只读验证 `72/72`，每张照片三个 UID 均唯一。
- 产品边界：保留桌面 WebP q60，不扩大到低于 q60、组合扰动或移动端；整体桌面媒体 RC 仍为 `BLOCKED`。
- 剩余阻断：`RC-MEDIA-002`、`RC-MEDIA-003`、`RC-MEDIA-004`、`RC-RELEASE-001`、`RC-RELEASE-002`。
- 证据：`artifacts/desktop-media-internal-rc/20260722/rc-media-001-closure.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。
- 下一 Roadmap 任务：优先处理 `RC-MEDIA-002` 默认 `watermark-core` release suite 红灯，然后处理音频最终安装候选矩阵。

## 2026-07-22 RC-MEDIA-002 正式关闭

- 状态：`CLOSED`。默认 `watermark-core` release suite 为 `108 passed / 0 failed`，正式 V3 图片服务测试 `5/5`。
- 根因结论：六项失败均为已退役 V2 图片 API 测试，不属于当前正式图片能力；不修复或重新开放 V2 图片路径。
- 产品边界：正式图片只支持 V3/39；V2 图片写读和 rollback 返回 `v2_image_rollback_retired`。
- 测试治理：legacy / rollback-only 进入 `npm run watermark:legacy-rollback-suite`，其中图片只验证拒绝合同，音频旧版回滚保持隔离。
- 剩余阻断：`RC-MEDIA-003`、`RC-MEDIA-004`、`RC-RELEASE-001`、`RC-RELEASE-002`；整体桌面媒体 RC 仍为 `BLOCKED`。
- 下一 Roadmap 任务：执行 `RC-MEDIA-003`，通过最终安装候选完成五格式 × mono/stereo 音频基线并归档到 `artifacts/`。

## 2026-07-22 RC-MEDIA-003 正式关闭

- 状态：`CLOSED`。最终 installed exe SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40` 完成五格式 × mono / stereo `10/10`。
- 质量与取证：每个单元均通过写后回读、独立核心读取、安装版只读验证、V3 UID 精确匹配和采样率 / 声道保持。
- 输出边界：保护副本固定为 WAV；WAV / FLAC 无损输入保持有效位深，有损输入不承诺保持源编码。
- 性能快照：整轮约 `53.3 秒`，单单元约 `4.0–9.8 秒`，不存在异常长跑。
- 剩余阻断：`RC-MEDIA-004`、`RC-RELEASE-001`、`RC-RELEASE-002`；整体桌面媒体 RC 仍为 `BLOCKED`。
- 下一 Roadmap 任务：执行 `RC-MEDIA-004` 的 `20:00 + 48 kHz + stereo + 高位深` 安装版资源组合 Gate。

## 2026-07-22 RC-MEDIA-004 正式关闭

- 状态：`CLOSED`。最终安装候选通过 `20:00 / 48 kHz / stereo / 24-bit FLAC -> 24-bit PCM WAV`。
- 取证结果：时长、采样率、声道和有效位深保持；写后回读、独立 V3 核心读取和安装版只读验证精确匹配。
- 性能快照：完成约 `57.5 秒`，主进程峰值约 `1.215 GiB`，进程树工作集求和峰值约 `2.151 GiB`。
- 取消边界：约 `14 ms`确认、不创建版权记录；底层约 `45.8 秒`后 CPU 静默，不能宣传瞬时释放资源。
- 媒体阻断项 `RC-MEDIA-001` 至 `RC-MEDIA-004` 已全部关闭；整体 RC 仍因 `RC-RELEASE-001`、`RC-RELEASE-002` 为 `BLOCKED`。
- 下一 Roadmap 任务：对当前最终候选执行 Authenticode 签名与 candidate Gate，不重新构建。

## 2026-07-22 RC-RELEASE-001 正式关闭

- 状态：`CLOSED`。当前 NSIS、MSI、release EXE 和 installed EXE 已原地签名，未重新构建。
- 证书：自签 `CN=HiddenShield Release Signing`，thumbprint `4F14DA0B5558359183E86F35486A08A34F38EAE5`；仅适用于服务方和受管客户 trust store。
- 四文件 Authenticode 状态均为 `Valid`，SignTool `/pa /all /v` 均通过并检测到时间戳；candidate Gate 的四个篡改副本均为 `HashMismatch`。
- 签后 SHA-256：NSIS `b705cd1249947057cab65e0cdb268dbbd50a2cd5fd2a0717e20f3e8ca9ad474b`；MSI `f6bae8fdb7c5e26e5e3b41df7d9fa3acfed3f5d2ada6944602518e1b39d3a876`；release EXE `d050a4a93b4bfb8a39c6628a437ba33faa2f4b45777f9bbe362d6bf3588c8c96`；installed EXE `17ea6c9dc0595bccf75ac4248a7b01f0abe9794adde574b0f3c5eb41e3b32a24`。
- 整体 RC 仍为 `BLOCKED`，唯一剩余阻断为 `RC-RELEASE-002` 干净离线 Windows 安装证明。
- 下一 Roadmap 任务：在独立干净离线快照分别安装签名 NSIS / MSI，验证内层 EXE 签名和媒体冒烟，不重新构建。

## 2026-07-22 RC-RELEASE-002 挂起与签名后候选复验

- 执行状态：用户决定暂时挂起 `RC-RELEASE-002`；该项保持 `suspended_by_user / blocking`，不视为通过、豁免或可延期发布。
- 商业发布结论：当前 `0.1.0` 桌面内部 RC 和对外发布继续为 `BLOCKED`；移动端继续冻结。
- 同一已签名 installed EXE SHA-256 `17ea6c9dc0595bccf75ac4248a7b01f0abe9794adde574b0f3c5eb41e3b32a24` 完成图片 PNG / JPEG / WebP `3/3` 和音频五格式 × mono / stereo `10/10` 写读冒烟，签名复核仍为 `Valid`。
- RC 门禁加固：`RC-RELEASE-001` 现要求签名证据、candidate Gate、篡改失效和签名后同一 installed EXE 媒体冒烟同时通过。
- 明确限制：本机已签名 installed EXE 冒烟不能证明 NSIS / MSI 新安装出的内层 EXE 已签名，也不能替代无预装 WebView2、物理断网的干净 Windows 快照。
- 证据：`artifacts/desktop-image-resource-gate/20260722-post-sign-smoke/summary.json`、`artifacts/desktop-audio-format-channel-gate/20260722-post-sign-smoke/summary.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。
- 下一 Roadmap 任务：保持 `RC-RELEASE-002` 挂起且阻断，执行桌面 `0.1.0` RC 证据索引完整性审计，核对候选哈希、证据引用和产品边界文案。

## 2026-07-23 NSIS 标准用户安装签名失败

- 现有签名 NSIS 已在本机标准 current-user 目录 `%LOCALAPPDATA%\HiddenShield` 完成安装，开始菜单、桌面快捷方式和卸载注册表均正确创建。
- 失败结论：签名 NSIS 外层为 `Valid`，但新安装 `hidden_shield.exe` 为 `NotSigned`，SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`；说明安装器内嵌负载未被签名。
- 商业发布状态：拒绝当前 `0.1.0` 安装候选；`RC-RELEASE-002` 为 `failed_local_install_vm_suspended`，干净离线 VM 仍挂起且不能用本机安装替代。
- 签名 Gate 状态：安装也覆盖了原先单独签名的测试 installed EXE，统一 RC 已重新打开 `RC-RELEASE-001`；当前候选存在两个 Critical 发布阻断，不得对外分发。
- 约束：本任务不重建当前候选。未来候选必须遵循“签 inner EXE → 打包 NSIS / MSI → 签外层安装器 → 验证新安装 EXE”的顺序。
- 证据：`artifacts/nsis-local-standard-install/20260723/summary.json`。
- 下一 Roadmap 任务：输出签名顺序修复设计与候选替换准入条件，等待批准后才构建新候选。

## 2026-07-23 下一 Windows 候选签名顺序设计

- 状态：`DESIGNED / WAITING_FOR_APPROVAL`；未构建、未签名、未替换当前失败候选。
- 设计结论：采用显式顺序 `tauri build --no-bundle → 签 inner EXE → 验签与哈希冻结 → tauri bundle --no-sign → 签 NSIS / MSI → NSIS / MSI 各自新安装 EXE 验签`。
- Gate 重构：`RC-RELEASE-001` 只验证源候选签名拓扑；`RC-RELEASE-002A/B` 分别验证 NSIS / MSI 新安装 EXE，`RC-RELEASE-002C` 保留为干净离线 Windows 证明。禁止对安装目录 EXE 单独补签后充当 Gate 证据。
- 证据模型：未来候选使用单一 manifest 绑定 pre-bundle inner EXE、outer wrappers 与两个实际新安装 EXE 的 SHA-256、签名主体、thumbprint 和时间戳。
- 回滚：当前 `0.1.0` 候选继续拒绝并保留为失败证据；只允许回退到完整通过 manifest 的旧候选。
- 详细设计：`docs/Windows候选签名顺序修复设计.md`。
- 下一 Roadmap 任务：等待批准后，在独立 worktree 实现编排模块、candidate Gate 与 CI 顺序；不得重建当前候选。
## 2026-07-24 Phase R1 报告交付视觉收敛

- 状态：正式报告继续输出 PDF、JSON、Manifest 三件套；本轮仅收敛 PDF 交付视觉，不改变报告 schema、证据字段、摘要链、权益或报告包校验合同。
- 实现：封面移除容易被误解为在线校验能力的二维码视觉占位，改为“PDF · JSON · Manifest”离线报告包说明，并明确 `manifest.json` 仅用于同目录文件一致性校验，不代表数字签名或在线校验；Chromium 正式模板升级为 `R1.1`。
- 商业与法律边界：当前报告仍未提供 PDF 数字签名、可信时间、在线校验、报告撤销或司法认可；不得将本轮视觉改造宣传为可信签发或法律证明能力。
- 验证：`npm run report:pdf-r1-gate` 通过，输出 `4` 页、无溢出、生成 `1097.35 ms`、`752795 bytes`；`npm run report:contract` 通过，正式报告包合同不变。
- 下一 Roadmap 任务：完成 R1.1 PDF 门禁复验后，以真实单文件记录确认封面、摘要、证据链与限制说明的阅读体验。

## 2026-07-24 本地 Fixture Mock 支付与真实报告导出复验

- 状态：`只能内部测试`。本地 `fixture` 支付提供方用于开发闭环验证，不创建真实订单、不调用真实收款渠道，也不得出现在生产支付入口或对外销售承诺中。
- 修复：`cloud:backend` 明确指定服务端二进制，避免新增 QA 二进制后 `cargo run` 无法启动本地 Mock 后端；桌面报告购买会话的创建、状态查询和确认改在 `spawn_blocking` 中执行，避免在 Tokio 异步线程内销毁阻塞 HTTP 客户端导致 panic。
- 验证：本机 Mock 后端健康检查通过；`fixture` 会话创建、状态查询和授权回写通过。桌面端以真实图片记录 `Abstract3.jpg` 导出 `hsr-e3794157b4079fa2`：R1.1、`4` 页、无分页溢出、`1022.52 ms`、`723307 bytes`，并生成 `report.pdf`、`report.json`、`manifest.json` 与 SHA-256 摘要链。
- 风险与边界：该授权来自测试 provider，不代表用户已付款、订单已结算或具备退款、对账、税务、风控、支付回调验签或真实渠道可用性。
- 下一 Roadmap 任务：为开发版增加明确的 Mock 支付视觉标记与专用启动开关；在真实支付接入前继续保持报告购买为内部测试。

## 2026-07-24 Mock 支付显式开关与报告语言收敛

- 状态：`只能内部测试`。新增 `npm run tauri:dev:mock-payment` 专用开发启动开关；仅显式开启后，报告购买区域才显示“开发测试 · Mock 支付”“模拟授权”“不扣款”并允许 fixture 授权。普通开发或正式构建不再默认请求 fixture，购买按钮明确显示“真实支付暂未接入”。
- 报告语言：PDF 改为“版权保护报告”，面向创作者呈现作品信息、保护结果、已确认记录、仍需关注事项和使用边界；不再在 PDF 中展示 schema、payload 协议、字节数、渲染器、内部模型或文件摘要值。原始技术字段仍保留在 JSON、Manifest 与内部诊断中，未改变证据合同。
- 边界：本轮不接入真实支付，不改变价格、授权范围、报告签名、可信时间、在线校验或法律效力承诺。
- 下一 Roadmap 任务：复跑 PDF 与前端构建门禁，并以 Mock 开关启动开发版完成一次“模拟购买并导出”的可见标记复验。
- 合同锚点：Free 单份付费报告保持逐记录授权；双端版权库已接入购买入口与单记录导出核销。
## 2026-07-26 商业潜力与融资估值底稿

- 新增内部文档 `docs/HiddenShield商业潜力与融资估值评估.md`，用于未来投资人沟通、融资估值推演、市场优先级和融资前数据室准备。
- 文档明确区分代码资产转让价值、战略收购价值、无收入阶段融资估值和未来 ARR / 平台采用情景估值；所有区间均为内部工作假设，不是第三方资产评估或融资保证。
- 当前基准假设：仓库中未见可验证的 ARR、付费客户、续费、正式平台合同或已授权核心专利，因此当前合理投前估值按人民币 `2000万～3500万元` 讨论；若存在未披露商业证据，必须补充数据后重新评估。
- 当前市场进入建议：先以电商 / 广告内容团队作为 B2B 切口，通过本地批量保护、版权库、权利声明和证据报告验证付费；AI 内容标识合规 SDK、C2PA / 媒体合作和公共信任层作为后续增长曲线。
- 能力边界：估值文档不得把内部测试、挂起能力、生产 C2PA / TSA、公共信任层、L3 视频或司法采纳包装为当前可售能力；任何投资材料继续引用 `docs/当前真实能力边界说明.md`。
- 验证结果：仅新增和同步文档，无代码、商业权益、价格、支付、API、数据库或产品能力变更。
- 风险：估值对客户、收入、团队、知识产权、第三方 benchmark 和正式 GA 高度敏感；代码规模和开发投入不能替代市场验证。
- 下一商业化任务：未来 30 天围绕电商 / 广告内容团队完成至少 15 家客户访谈，筛选 3 家付费设计伙伴，并用真实订单、处理量和持续使用意向更新估值底稿。

## 2026-07-26 桌面报告签名认证法律效力措辞审计

- 状态：桌面 UI、版权报告、帮助中心和用户协议文案审计完成；公共信任层继续挂起。
- 报告购买说明现明确：当前报告包含版权信息、技术验证结果、时间回执状态和完整性摘要，但不提供包级数字签名或法律权属结论。
- PDF 报告将“验证通过”收敛为“技术校验通过”，并明确载荷认证标签与 Manifest 摘要匹配不等于发行方数字签名、实名认证或法定权属确认。
- 本轮不改变价格、支付、授权、报告 schema、Manifest 合同、TSA 实现或任何商业权益。
- 当前商业发布仍为 `BLOCKED`：本机会话非管理员且用户级 WebView2 已安装，不能完成提升权限干净 Windows Gate。
- 验证计划：复跑前端生产构建、报告 PDF gate、报告合同、商业 readiness 和双端合同。
- 验证结果：前端构建、报告 PDF gate、报告合同、商业 readiness、双端合同、V3 quality gate contract 与 `git diff --check` 通过；报告 bundle Rust gate 被既有 `probe.rs` 测试初始化缺少 `auto_update_enabled` 字段阻断，未归因于本轮文案修改。
- 下一商业化任务：由提升权限的干净 Windows QA operator 完成 MSI / NSIS 新安装、内层 EXE 验签和物理断网图片 / 音频冒烟。

## 2026-07-26 桌面 RC / GA Windows Gate 关闭

- 状态：`PASSED`。桌面 `v0.1.3` 已由 Windows QA operator 在提升权限环境手工运行 `scripts/release/verify-windows-installed-payload.ps1`，安装负载验签 Gate 通过。
- 物理断网图片冒烟与 WAV / MP3 / FLAC / M4A / AAC 五种音频格式冒烟通过。
- 本轮按 release owner 指令采用操作员验收声明作为最终签字依据，不继续追踪或独立复核产物。
- `RC-RELEASE-002` 已关闭；桌面内部 RC 和桌面 GA 发布 Gate 均改为 `PASSED`。
- 版本口径：当前通过候选为 `v0.1.3`；既有 `v0.1.0` 安装候选和其失败日志继续只作为历史证据保存。
- 商业边界不变：真实支付仍未接入，公共信任层继续挂起，Enterprise、云端视频和其他未达门禁能力不得随桌面 GA 一并承诺。
- 风险：正式分发文件必须与最终通过候选保持同一版本、哈希和签名身份；任何重建、重签或安装器替换都必须重新执行 Windows installed-payload 与物理断网媒体冒烟。
- 下一商业化任务：冻结桌面 GA 分发清单与回滚包，然后按既定电商 / 广告内容团队方向启动付费设计伙伴验证，不把尚未接入的真实支付包装为可用能力。

## 2026-07-26 桌面 v0.1.3 GitHub Release 资产发布

- 状态：`PUBLISHED_WITHOUT_PUBLIC_ROLLBACK`。`v0.1.3` NSIS、MSI、Tauri updater 签名、`latest.json`、`SHA256SUMS.txt` 与 JSON 发布清单已公开。
- 资产哈希、公开分发边界及下载入口见 `docs/桌面v0.1.3发布清单.md`。
- 公开资产使用 Tauri Ed25519 updater 签名；当前 GitHub Actions 发布工作流不提供公开 Authenticode 签名，因此不把该渠道包装为 Windows 发行方签名。
- 风险：`v0.1.2` 仍为 Draft Release，当前没有可对用户承诺的公开回滚安装包。
- 下一商业化任务：确认并公开一份可接受的回滚版本后更新发布清单，再开始面向付费设计伙伴的受控分发。

## 2026-07-27 AI 图片平台生成时标识 MVP 设计冻结

- 状态：`design_frozen`。新增 `docs/AI生成内容标识基础设施MVP设计.md`，提前恢复公共信任层中的一个最小子集：面向 AIGC 图片平台的生成时 AI 来源标识基础设施。
- 商业定位：未来平台 SDK / 私有部署 / Registry / 批量验证的 B2B 增长方向，不进入当前桌面图片 / 音频年费权益，不改变现有价格、支付、套餐、配额或桌面导航。
- 最小切口：只做平台已知事实的生成时标识，不做上传任意未知素材后的通用 AI 概率检测；先覆盖 AI 图片生成和 AI 图片编辑，音频、视频、文本和通用检测路由不进入本期 MVP。
- 设计要求：采用“显式标签数据 + C2PA 等标准元数据 + `watermark-core` 鲁棒锚点 + AI Transparency Manifest + Evidence + Registry / Resolver”的多层结构；不在 V3 / 39-byte payload 中加入 AI flag、provider、模型、法规 Profile 或 Evidence 语义。
- 客户验证：现有电商 / 广告内容团队设计伙伴验证不取消；新增 AIGC 图片平台访谈和设计伙伴筛选，优先验证真实生成链路是否需要鲁棒锚点、Registry、Profile 适配和统一验证接口。
- 能力边界：本次只有文档设计冻结，不代表已符合中国、欧盟、美国或任何平台规则，不代表生产 C2PA 信任链、平台签名 Evidence、Detector API 或平台 SDK 已上线。
- 下一商业化任务：在开始任何数据库或 SDK 代码前，筛选至少 3 家 AIGC 图片平台设计伙伴，确认其输出格式、吞吐、延迟、私有部署、签名主体、检测入口和首个 Compliance Profile 需求。

## 2026-07-27 AI 图片平台标识 SDK 法规与商业审计

- 状态：`conditional_design_pass`。已审计 `docs/AI生成内容标识基础设施MVP设计.md`，并按中国、欧盟、加州的不同主体、媒体和流转环节重构为法规 Profile、技术 Profile 和商业授权三层模型。
- 已完成：将中国图片导出的显式标识改为文件 / UI 回执和验证要求；将 C2PA 定位为技术 Profile 而不是合规结论；将平台付费 SDK 与普通用户免费基础验证、收费企业批量验证分离；冻结 production / sandbox、issuer mode、scope、`confirmed_marked_image` 计量和 fail-closed 授权行为。
- 商业模式：未来采用年度平台授权、已确认标识量、法规 Profile 包、信任与部署包、企业批量验证包；不按失败、重试、内部写后读或普通用户基础验证收费。价格不在文档阶段冻结，必须由设计伙伴的真实输出量、延迟预算、合规风险和采购约束验证。
- 验证：已通过中国网信办标识办法、欧盟 AI Act Article 50 透明度实践框架、加州 AI Transparency Act 现行文本的桌面研究核验；`git diff --check` 待本轮文档收口后复跑。
- 风险：HiddenShield 提供的是技术控制、证据和授权组件，不替代客户的法务责任；加州适用客户的免费检测义务、欧盟 provider / deployer 责任和中国导出显式标识必须由每个 Profile 的外部法务审查确认。
- 下一商业化任务：与 3 家 AIGC 图片平台完成授权、免费基础验证、计量、签发主体、数据驻留、导出标识和私有部署访谈，并据此冻结首个 production SKU 的合同和非价格化报价结构。

## 2026-07-27 AI 图片平台标识 SDK 数据库/API 合同冻结

- 状态：`schema_contract_frozen_no_implementation`。新增 `docs/AI生成内容标识数据库与API_Schema合同.md`，冻结 AI Transparency SDK 的逻辑数据库模型与 HTTP API 表面。
- 已完成：冻结 `licenseId`、`issuerMode`、Profile entitlement、`confirmed_marked_image`、免费公共 Resolver、sandbox / production、scope、幂等会话、confirm 原子事务、显式标签回执和错误码。
- 商业计量：`confirmed_marked_image` 是唯一的 V1 标识计量单位；创建会话、失败、重试、重复确认、内部 write-after-read 和普通用户公共单文件验证均不得产生标识计量。该 ledger 不等同真实支付扣款。
- 免费验证：V1 公共 Resolver 不需要 API key 或 license，不消费配额且不读取用户媒体文件；它不自动满足任何客户所在地的完整媒体检测义务，后续 Detector 需求须由已购 Profile 和法务审查决定。
- 风险：现有 Enterprise API key、quota 和 audit 只能作为底层实现基础，不能直接复用其公开权利查询的授权和收费语义；生产 SDK 在 JSON fixture、迁移、contract test、法务 Profile 审查和平台试点前仍不得销售或分发。
- 下一商业化任务：冻结不依赖运行时代码的授权 / 计量 JSON fixture 和非价格化 production SKU 合同字段，然后再评审数据库迁移。

## 2026-07-27 AI 图片平台标识 SDK 授权与计量 Fixture 冻结

- 状态：`schema_and_fixture_frozen_no_implementation`。新增 `docs/contracts/ai-transparency/`，冻结 production license、三地 Profile entitlement、成功 `confirmed_marked_image`、免费公共 Resolver、过期授权、Profile 拒绝和冲突 confirm 的 JSON 向量。
- 已完成：统一 `licenseId`、tenant、workspace、Watermark ID、幂等键和计量预期；明确完全相同的 confirm 重放未来应返回原结果且不重复计量，而摘要或请求不一致的重复 confirm 必须返回 `ai_confirmation_conflict`。
- 验证计划：全量 JSON 解析、共享 ID 关联检查、`git diff --check`；本轮不运行迁移、不创建 API、不发放 production credential、不销售或分发 SDK。
- 风险：fixture 的签名、摘要和平台字段均为合成占位值，不能作为生产验签、性能或法规合规证据；三地 Profile 的法务解释仍需外部审查。
- 下一商业化任务：先实现 fixture 的 schema / contract test，验证授权、计量和公共验证不收费边界；只有 contract test 通过后才评审数据库迁移和首个 production SKU 合同。

## 2026-07-27 AI 图片平台标识 SDK 合同迁移骨架

- 状态：`storage_schema_created_runtime_not_implemented`。fixture schema / contract test 已通过后，新增 PostgreSQL `0002_ai_transparency_schema`，并在本地 SQLite 初始化中建立等价的表与索引骨架。
- 已完成：冻结并落地 production 授权唯一性、Profile entitlement、SDK credential binding、幂等 marking session、AI Transparency Manifest、Evidence、Marker Binding、显式标签回执与 `confirmed_marked_image` ledger 的存储约束；PostgreSQL smoke 已执行 `0001` → `0002` 的真实 up/down，验证 20 张表、21 个索引、partial-index 与 7 项数据库约束回归断言。
- 验证：`cargo run --manifest-path feedback-backend/Cargo.toml --features postgres --bin postgres_migrate_smoke` 在一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke` 通过；`npm run ai-transparency:schema-contract` 通过（7 个 fixture）；`cargo test --manifest-path feedback-backend/Cargo.toml` 通过（81/81）；`cargo check --manifest-path feedback-backend/Cargo.toml --features postgres` 与 `git diff --check` 通过。
- 风险：回归测试发现并修复了显式导出文件标签摘要的 SQL `NULL` 检查漏洞；SQLite 镜像与 PostgreSQL 迁移仍只提供存储边界，未实现 API、原子 confirm 事务、生产凭据、扣款、Resolver、Detector 或 SDK。
- 下一商业化任务：评审内部 license / Profile 管理的最小接口与审计模型；在 confirm 原子事务、法务 Profile 审查和平台试点前不得销售或分发 SDK。

## 2026-07-27 AI 图片平台标识内部授权与审计 V1

- 状态：`internal_read_only_v1`。新增 `docs/AI生成内容标识内部授权管理与审计合同.md`，并实现 admin-token 保护的内部 license 查询与 Profile entitlement 校验。
- 已完成：`GET /internal/ai-transparency/licenses/:license_id` 与 `POST /internal/ai-transparency/profile-entitlements/check`；两接口只读业务数据、无 credential / session / ledger 创建，并按成功、拒绝、失败写入独立 `ai_transparency_admin_audit_events`。
- 验证：新增存储与路由测试通过；一次性 PostgreSQL 16 smoke 验证 `0001` → `0002` up/down、21 张表、22 个索引和 7 项约束回归；`cargo test --manifest-path feedback-backend/Cargo.toml` 通过（83/83）、PostgreSQL feature 编译通过、fixture contract 通过。
- 风险：内部 endpoint 仍复用全局 admin token，不是双人审批、细粒度 RBAC 或生产 license 写入工作流；现有 PostgreSQL 运行时 adapter 也尚未实现。
- 下一商业化任务：先冻结 license / Profile 写入、续期、暂停、撤销的双人审批与审计合同；在该合同、confirm 原子事务与外部法务 Profile 审查通过前继续禁止 SDK、production credential、公共 Resolver 和商业化发放。

## 2026-07-27 AI 图片平台标识授权写入双人审批合同冻结

- 状态：`four_eyes_write_contract_frozen_no_implementation`。新增 `docs/AI生成内容标识授权写入双人审批与审计状态机合同.md`，冻结 license / Profile 的 create、renew、suspend、revoke 的 maker-checker、系统执行和 append-only 审计状态机。
- 已冻结：request / approval / execution 三段式合同、request digest、版本冲突、审批过期、生产引用要求、操作角色矩阵、license / Profile 状态转换、稳定 reason code、终态不可恢复和审计 fail-closed。
- 架构结论：现有 `ai_profile_entitlements` 的 `(licenseId, profileId)` 逻辑唯一键无法承载 revoked / expired 后的可追溯重新授权；实现前必须演进为版本化 entitlement 模型，禁止直接覆盖或复活历史记录。
- 风险：现有单一 admin token 不具备 actor 身份、角色、审批人分离或生产 RBAC 语义；本轮没有新增数据库、写接口、审批 UI、通知、production license、credential、SDK 或计量。
- 下一商业化任务：冻结 actor / role 身份来源、versioned Profile entitlement 数据模型与 change request / approval / execution / audit fixture，再实现状态机 contract test；通过前继续禁止所有 production 发放和 SDK 分发。

## 2026-07-27 AI 图片平台标识身份、版本化 Entitlement 与状态机 Fixture 冻结

- 状态：`identity_schema_fixture_contract_test_frozen_no_write_implementation`。新增 `docs/AI生成内容标识审批身份与版本化Entitlement_Schema合同.md` 和 `docs/contracts/ai-transparency-approval/`，并冻结 actor / role、版本化 entitlement、change request、approval、execution、audit 六类 JSON fixture。
- 已完成：Internal IAM 是唯一 actor 身份来源；requester / approver 为不同 human、system executor 不能审批；版本链、单 active version、production regulatory legal review、digest 绑定、version conflict 无目标写入和 append-only audit 已由可执行 contract test 断言。
- 验证：`npm run ai-transparency:approval-contract` 通过（6 fixtures）；既有 `npm run ai-transparency:schema-contract` 通过（7 fixtures）；`git diff --check` 通过。
- 风险：actor identity、role binding、versioned entitlement、request / approval / execution / audit 仍未进入数据库或运行时；request digest canonicalization、其余 operation desiredState Schema、真实并发测试及生产 RBAC / 外部依据真实性来源均未完成。
- 下一商业化任务：只评审版本化 entitlement 与审批状态机数据库迁移设计；在迁移、真实并发测试、confirm 原子事务、外部法务 Profile 审查和平台试点前继续禁止 license / Profile 写入、production license、credential、SDK、公共 Resolver 与商业化发放。

## 2026-07-27 AI 图片平台标识审批状态机数据库迁移设计评审

- 状态：`conditional_design_pass_no_migration`。新增 `docs/AI生成内容标识审批状态机数据库迁移设计评审.md`，只评审未来 `0003_ai_transparency_approval_state_machine`，未创建 SQL、SQLite schema 或运行时写接口。
- 设计结论：采用 additive migration；新增 actor role snapshot、versioned entitlement、change request、approval、execution、append-only audit 和 target lock；版本表作为真相源，现有 `ai_profile_entitlements` 暂作 current projection。
- 并发 Gate：未来 executor 必须通过单一深事务 module 锁 target key 和 request；真实测试必须覆盖双连接 renew 冲突、重复执行、同目标冲突、audit 故障回滚和无 credential / session / ledger 副作用。
- Backfill 边界：旧 entitlement 可生成 synthetic version / request / approval / audit，但必须明确 `migrated_legacy_without_four_eyes`，不能伪装生产双人审批证据。
- 阻断：request digest canonicalization、各 operation desiredState Schema、Internal IAM 验真 interface、合同/法务/安全引用验真、synthetic backfill 措辞和 PostgreSQL 并发 harness 尚未冻结。
- 下一商业化任务：只冻结上述六项阻断和 `0003` migration fixture / Schema Contract；在 migration 与真实并发测试通过前继续禁止所有 production license、credential、SDK、公共 Resolver 和商业化发放。

## 2026-07-27 AI 图片平台标识 0003 迁移前置 Gate 冻结

- 状态：`pre_migration_gates_frozen_0003_creation_permitted_not_started`。新增 `docs/AI生成内容标识0003迁移前置Gate合同.md`、八类 desiredState Schema 与两类 pre-migration fixture；本轮未创建 `0003`。
- 已完成：冻结 `hs-ai-change-request-digest-v1` 固定向量、八类 operation desiredState、Internal IAM 与外部 reference 验真 interface、synthetic backfill 非生产措辞、PostgreSQL 双连接 harness interface 和六类竞态场景。
- 验证：`npm run ai-transparency:approval-contract` 通过（8 fixtures）；既有 `npm run ai-transparency:schema-contract` 保持通过；前置 Gate 允许开始创建 `0003`，不允许 production 发放。
- 风险：IAM/reference adapter、digest runtime implementation、desiredState runtime validator、0003 migration、真实 PostgreSQL 双连接测试和写入事务均未实现；fixture/harness 合同不是并发测试成功证据。
- 下一商业化任务：创建 `0003_ai_transparency_approval_state_machine` 及 PostgreSQL migration tests，再实现 PostgreSQL 双连接并发 harness；SQLite 只保留本地 migration regression。在真实 migration 与并发测试通过前继续禁止写接口、production license、credential、SDK、公共 Resolver 和商业化发放。

## 2026-07-27 AI Transparency 0003 数据库状态机落地

- 状态：`postgres_database_schema_and_concurrency_primitives_verified`。已创建 PostgreSQL `0003_ai_transparency_approval_state_machine` additive migration；SQLite migration 仅接入本地 schema 初始化和回归。
- 数据模型：新增 actor/role snapshot、change request、versioned Profile entitlement、approval、execution、append-only audit 和 target lock；现有 `ai_profile_entitlements` 继续作为 projection，并新增 current version metadata。
- 验证：一次性本地 PostgreSQL `hiddenshield_migrate_smoke_0003` 完成 `0001 -> 0002 -> 0003 -> down`，检查 28 张表、28 个索引和 13 项约束回归；PostgreSQL 两个独立连接完成 6 个冻结并发场景，credential/session/Manifest/ledger 计数均为 0。SQLite migration 测试只证明本地 schema 兼容性。
- 商业边界：本轮只交付授权治理数据库基础，不包含 SDK、公共 Resolver、生产 credential、付费计量或生产 License/Profile 发放；数据库并发 primitive 通过不等于 operation-specific 原子事务、IAM/reference 验真和法务 Profile Gate 已通过。
- 风险：synthetic legacy backfill、真实 change command transaction module、confirm 原子事务和生产级 IAM/reference adapter 仍是生产发放阻断项；不得据此对外宣传三地法规合规或平台已可接入。
- 下一商业化任务：实现并测试单一内部 change-command 原子事务 module，逐场景落地 request/approval/execution/audit/projection 写入，并保持 SDK、公共 Resolver 和生产 credential 发放关闭。

## 2026-07-27 AI Transparency 内部 Change Command 原子事务

- 状态：`postgres_internal_atomic_change_command_verified_no_production_issuance`。新增统一内部 `InternalChangeCommand`；PostgreSQL 实现是唯一生产事务语义，SQLite adapter 只用于本地单元合同。
- 原子边界：target lock、幂等检查、同目标 in-flight 检查、request、approval、execution、version history、current projection 与 append-only audit 均位于同一数据库事务；audit 写入失败会回滚 request、approval、execution、version 和 projection。
- 并发验证：六个冻结场景已从直接锁表测试升级为 PostgreSQL 真实双连接 command 测试，并得到冻结的 winner、request status、target version、active version count、audit sequence 和 stable reason code。
- 零副作用：所有场景继续断言 `ai_sdk_credential_bindings`、`ai_marking_sessions`、`ai_transparency_manifests` 和 `ai_marking_ledger` 为 `0`，未发放 credential、未创建 marking session、未写 Manifest、未计量。
- 商业边界：module 未连接 HTTP/API/SDK 或生产 credential 发放；身份与外部依据仍由 fixture snapshot 提供，尚未接入真实 Internal IAM、contract/legal/security reference adapter 或法务 Profile 审查。
- 验证：Rust `85` tests、PostgreSQL feature check、`0001 -> 0002 -> 0003 -> down` smoke、两份 AI Transparency contract 与 `git diff --check` 通过；PostgreSQL 真实双连接 command harness 通过。
- 下一商业化任务：把 fail-closed Internal IAM 和 contract/legal/security reference adapter 接入 change-command 前置校验，并新增 adapter unavailable、scope mismatch、expired binding 与 reference mismatch 的 PostgreSQL 零写入测试；继续禁止 SDK、公共 Resolver 和所有生产发放。

## 2026-07-27 AI Transparency Fail-Closed IAM 与引用校验边界

- 状态：`fail_closed_adapter_boundary_verified_no_production_provider`。change-command 现在必须接收 Internal IAM 与 approval reference adapter；任一 adapter 拒绝、过期、scope 不匹配或 unavailable 时，在事务开启前返回稳定拒绝码。
- IAM：preflight 对 requester、approver、executor 分别要求 `ai_transparency_requester`、适用 approver role、`system_executor`；输入仅包含 token hash、role、tenant/workspace/environment/operation，不持久化 raw token。
- 引用：production regulatory Profile grant/renew 要求 legal review；technical 要求 security review；license create/renew 预留 contract 检查。仅保存 opaque reference ID，不复制原合同或法务文件。
- 验证：PostgreSQL 覆盖 IAM `invalid`、`expired`、`scope mismatch`、`unavailable` 及 reference `invalid`、`expired`、`scope mismatch`、`unavailable`；每项 request、audit、projection、credential、session、Manifest、ledger 均为零写入。SQLite 同类测试不计入生产 Gate。
- 商业边界：当前是 adapter seam 与 fail-closed enforcement，不是生产 IAM 或外部合同系统上线；没有 HTTP/API/SDK 暴露，没有 production credential 或 License/Profile 发放。
- 下一商业化任务：在受控内部环境实现真实 Internal IAM/reference provider client、receipt 校验与 provider health policy，并新增 provider 签名、时钟、scope digest 与 failover 审计测试；继续禁止 SDK、公共 Resolver 和所有生产发放。

## 2026-07-27 AI Transparency 受控 Provider Receipt 与法务 Profile Gate 评审

- 状态：`internal_provider_receipt_verified_legal_profile_gate_blocked`。新增 `docs/AI生成内容标识受控内部Provider与Receipt验证合同.md`，实现受控内部 provider client：验证 provider/key ID、HMAC-SHA256 签名、active/granted、issued/expires 与 IAM/reference scope digest。
- 验证：两库 harness 使用有效 signed receipt 驱动既有 command 成功场景；IAM/reference 签名无效、过期、scope digest 不匹配、provider health unavailable 和 transport unavailable 均 fail-closed，且保持零写入。
- 法务 Profile Gate：`BLOCKED`。当前 Profile 仅冻结名称、原则与部分技术映射；未提供每个 CN/EU/US(加州) Profile 的适用主体/分发场景/必需控制/例外/法规来源/有效期/owner/变更记录/法务签署证据，不能作为 production regulatory entitlement 依据。
- 商业边界：HMAC receipt 是内部受控测试协议，不是生产 KMS/HSM、非对称签名、跨组织 trust 或法律意见；不解锁 SDK、公共 Resolver、production credential 或生产 License/Profile 发放。
- 下一商业化任务：先建立 CN/EU/US(加州) regulatory Profile 的法律控制矩阵与外部法律审查签署/有效期/变更流程，再评审是否创建 production Profile entitlement；在完成前继续禁止所有生产发放。

## 2026-07-27 AI Transparency Regulatory Profile 法律控制矩阵与外部审查包

- 状态：`external_counsel_package_prepared_all_profiles_blocked`。新增 `docs/AI生成内容标识Regulatory_Profile法律控制矩阵.md` 与 `docs/AI生成内容标识外部法务审查包.md`，覆盖 CN、EU、US（加州）图片 regulatory Profile 的审查输入、证据、控制映射和签署 receipt 模板。
- 已准备：官方来源审查起点、适用主体/分发场景问题清单、控制矩阵字段、source/control digest、有效期、owner、变更/撤销触发条件和外部法务 receipt 验收规则。
- 法务 Gate：`BLOCKED`。没有外部法务签署 receipt；CN Profile 在 MVP 与数据库/审批合同之间存在 canonical ID 不一致；每个司法辖区均缺少逐条已审查控制、来源快照、适用条件、例外、有效期与签署证据。
- 商业边界：审查包不是法律意见、认证或 production entitlement；当前内部 provider HMAC receipt 不得验证或替代外部法务签名。
- 下一商业化任务：聘请具备 CN/EU/US（加州）相应资质的外部法务，逐 Profile 完成控制矩阵、source snapshot 和签署 receipt；receipt 验证通过后仅重新评审 Profile Gate，不自动开启 production issuance。

## 2026-07-27 AI Transparency 授权人内部审计 Gate

- 状态：`owner_audit_approved_not_external_legal_opinion`。授权产品负责人批准 Regulatory Profile Gate 用于继续内部工程推进。
- 严格边界：该批准不是外部法律意见、法律签署、监管认可或合规认证；没有 external counsel receipt，CN canonical Profile ID 仍未决。
- 未解锁项：production entitlement、production credential、SDK、公共 Resolver、法规合规营销和任何生产发放继续关闭。
- 下一商业化任务：实现并验证 marking session 的单一 PostgreSQL confirm 原子事务；只在 PostgreSQL 真实双连接测试中形成 Gate 证据，验证通过后再评审受控内部标识执行链。

## 2026-07-27 AI Transparency PostgreSQL Confirm 原子事务

- 状态：`postgres_confirm_transaction_verified_internal_only`。实现 PostgreSQL-only confirm command 和 `0004_ai_transparency_confirm_audit` additive migration。
- 原子边界：Manifest、Evidence、Marker binding、显式 label receipt、pending/committed `confirmed_marked_image` ledger、confirm audit 和 session 状态切换位于同一 PostgreSQL 事务。
- 验证：一次性 `hiddenshield_migrate_smoke_confirm_20260727` PostgreSQL 16 数据库完成 29 表/29 索引 up/down smoke；真实双连接并发一胜一败，重复 confirm 只提交一次，ledger/audit 故障均零部分写入。
- 风险与未完成：尚无 production marking session 创建链、production credential/provider/KMS、线上观测或 production execution chain；当前不构成可销售、可集成或合规声明能力。
- 商业边界：不开放 HTTP/API/SDK，不签发 production credential，不启用公共 Resolver 或任何生产发放。
- 下一商业化任务：冻结并实现 production credential custody 与受控内部 marking session 创建链；在真实 provider/KMS、撤销和运行审计 Gate 通过前，不开放 SDK、公共 Resolver 或生产发放。

## 2026-07-27 AI Transparency Production Credential Custody

- 状态：`postgres_credential_custody_and_ready_session_verified_internal_only`。冻结 production credential custody/marking session 合同并实现 PostgreSQL `0005_ai_transparency_credential_custody`。
- Custody：明文 `hsai_live_...` 只返回一次；数据库只保存 prefix、HMAC-SHA256 hash、pepper version、custody key ID、scope、issuer mode、有效期、撤销和使用时间，不保存明文或可逆密文。
- Session Gate：只有 credential、production license 和全部 requested Profile entitlement 均有效时，内部命令才原子创建 `ready_to_confirm` session、更新 `last_used_at` 并追加 runtime audit。
- 验证：一次性 `hiddenshield_migrate_smoke_credential_20260727` PostgreSQL 16 库完成 30 表/32 索引 up/down；8 个场景覆盖未授权签发、有效 credential、暂停、过期、scope denied、inactive license、Profile denied 和并发 idempotency。
- 商业边界：当前仍为内部 QA custody provider/KMS 配置；没有客户自助签发、HTTP/SDK、公共 Resolver、支付计量或生产 SLA，不得对外发放。
- 下一商业化任务：实现 credential rotate/revoke、真实 Internal IAM receipt 和 KMS/HSM pepper rotation；旧 credential 撤销/轮换 Gate 通过后再评审受控试点 credential 发放。

## 2026-07-27 AI Transparency Credential Lifecycle

- 状态：`postgres_rotate_revoke_and_versioned_pepper_verified_internal_only`。实现 PostgreSQL `0006_ai_transparency_credential_lifecycle`、credential rotate/revoke 原子命令与 versioned pepper 校验。
- 生命周期：rotate 在同一事务创建 active pepper replacement、撤销旧 credential、写 lifecycle audit；revoke 原子撤销 credential 并写 audit。旧 credential 在提交后无法再创建 `ready_to_confirm`。
- IAM/KMS 接入：新增可复用 `InternalIamAuthorizationAdapter` 的 custody receipt adapter；pepper 模型支持 active/retained versions。当前仅 QA adapter，不能称为已接入真实生产 IAM/KMS/HSM。
- 验证：一次性 `hiddenshield_migrate_smoke_lifecycle_20260727` PostgreSQL 16 库完成 31 表/34 索引 up/down；10 个 custody 场景通过，包括 rotate 后旧 key 拒绝、新 `qa-v2` replacement 成功和 revoke 后拒绝。
- 商业边界：无生产 provider endpoint、HSM/KMS、在线撤销传播、客户控制台或 SDK；不得向真实客户发放 production credential。
- 下一商业化任务：部署/配置真实 Internal IAM receipt provider 与 KMS/HSM pepper provider，完成 provider unavailable、expired/scope mismatch、pepper retirement、并发 rotate/revoke 和恢复演练 Gate。

## 2026-07-27 Production Provider Deployment Package

- 状态：`production_provider_deployment_package_ready_internal_only`。新增 `docs/AI生成内容标识Production_Provider_Deployment_Package.md` 与 `config/ai-transparency-production-provider.env.example`，冻结只含 Secret 引用的 production custody 配置合同。
- 实现：production runtime 在 custody enabled 时缺少任一 IAM/KMS/runbook 配置即拒绝启动；四个 PostgreSQL custody 命令在开事务前统一检查 Internal IAM、KMS health 与 active pepper readiness，任一不可用即 fail-closed。
- 验证：provider readiness 单元测试、全量 Rust 测试和两份 AI Transparency JSON contract 已通过；一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke_provider_20260727` 完成 31 表/34 索引 up/down smoke，并执行 11 个 custody 场景。provider unavailable 时 issue/session/rotate/revoke 均在事务前 fail-closed：仅受控 seed credential/runtime audit 存在，零 marking session、零 `last_used_at`、零 lifecycle audit，随后 QA provider 恢复后正常 session 场景通过。
- 商业边界：模板与 Gate 不是实际 provider 部署、密钥托管、production credential 或 SDK 授权。真实 endpoint、工作负载身份、KMS/HSM pepper 和恢复演练证据缺失时，所有生产发放继续关闭。
- 风险：当前 readiness probe 只冻结 adapter 边界和安全默认值；仓库不提供 production 可复用的“永远就绪”实现，QA 放行实现仅存在于测试 binary。
- 下一商业化任务：由平台/安全提供真实 Internal IAM receipt 与 KMS/HSM reference 后，实现受控 adapter，完成 receipt 过期/scope mismatch、pepper retirement、并发 rotate/revoke 与恢复演练证据。

## 2026-07-27 Internal Image Marking Executor

- 状态：`internal_image_marking_executor_verified_no_external_release`。新增 `docs/AI生成内容标识Internal_Image_Marking_Executor合同.md`，并实现仅内部 PostgreSQL 命令。
- 执行链：已存在的 `ready_to_confirm` session 只读校验后，由 backend 调用 `watermark-core` 正式 V3 图片写入与同核心回读；回读 UID、V3 与 auth status 均通过后，才调用既有 confirm 原子事务。
- 验证：一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke_executor_20260727` 中，custody command 创建的 ready session 成功进入 confirmed；Manifest、Evidence、Marker、label receipt、committed `confirmed_marked_image` ledger 和 confirm audit 均为一条。无效 session 不返回保护副本且六类 confirm 记录均为零。
- 商业边界：输出是内部 PNG 保护副本与 platform UI 标签计划；不写生产 C2PA/TSA 签名，不开放 HTTP、SDK、公共 Resolver、客户 credential 或计量收费。
- 下一商业化任务：冻结 AI 图片平台输入/输出与 metadata/显式标签 fixture，并建立平台写入 -> 桌面/Android/iOS 读取的跨端互验矩阵；继续禁止外部 SDK 和公共 Resolver。
## 2026-07-27 Executor PNG 跨端 Fixture Gate

- 状态：`internal_fixture_verified_except_ios_runtime`。已冻结并生成 internal Executor 输出 PNG、含测试 metadata 版本和 metadata-stripped 版本，三者保持同一 `watermark-core` V3/39 UID 与认证结果。
- 已验证：backend 生成时写后回读、Desktop 正式读取代码路径、Android/iOS 共用 mobile Rust bridge 读取代码路径及 metadata 剥离后读取均通过定向测试；该宿主 bridge 结果不等同于 iOS 实际 runtime 证据。
- 商业边界：fixture 不解锁付费 SDK、公共 Resolver、production credential、客户发放、生产计量或法规合规宣传；免费公共验证边界仍只保留在冻结合同中，尚未实现。
- 风险：真实 Internal IAM/KMS/HSM provider 与 iOS macOS/device runtime Gate 未完成，平台集成与 production entitlement 均保持关闭。
- 下一商业化任务：在 macOS/iOS 正式 runtime 复跑固定 PNG fixture，并取得与 Desktop/Android 相同 UID、V3/39、auth 和 metadata-stripped 结果；通过前不得开始 SDK 发放。

## 2026-07-27 第三方 PNG 元数据共存商业边界

- 已完成：冻结 untrusted 第三方 PNG metadata 共存 fixture 合同，不将自造 metadata 作为 C2PA、平台签名、外部水印或跨平台接受证据。
- 商业影响：该合同仅减少未来 SDK 互操作设计的不确定性，不构成可售 SDK、收费验证、production entitlement 或法规合规能力。
- 已验证：一次性 PostgreSQL Executor QA 生成 fixture 后，静态 chunk/digest、Desktop 和 mobile Rust bridge 读取 Gate 均通过。
- 下一商业化任务：取得真实第三方参考样本与验收授权后开展处理链 Benchmark；生产 SDK 发放仍同时受 iOS runtime 与真实信任链 Gate 阻断。

## 2026-07-27 公开 C2PA Fixture 商业边界

- 已完成：使用 Apache-2.0 的公开 C2PA fixture 建立内部只读互操作 QA，验证不与 HiddenShield V3 anchor 混淆。
- 商业边界：公开测试语料不构成平台集成、平台验收、收费验证服务、production entitlement 或 C2PA 生产信任链承诺。
- 下一商业化任务：获得可验证的第三方平台或水印样本及适用处理链授权后，再评审处理链 Benchmark 是否可作为设计伙伴证据。

## 2026-07-27 公开视觉水印子矩阵商业边界

- 已完成：MIT 许可的外部视觉水印样本通过共享核心 V3 写入/回读，且公开 C2PA fixture 非混淆检查随同 Benchmark 执行。
- 商业边界：两个样本并非同一资产，不能作为平台互操作、设计伙伴验收、SDK 发放或收费验证证据。
- 下一商业化任务：取得同资产三层样本或提供方组合工具链授权后，评审其处理链保留率；iOS runtime 和生产信任链 Gate 继续阻断生产发放。

## 2026-07-27 PNG C2PA 输出容器商业 Gate

- 结果：当前 V3 PNG 输出不保留输入 C2PA manifest，分类为 `manifest_absent_after_png_reencode`。
- 商业影响：在 post-embed C2PA 重新签发或兼容容器方案通过前，不得销售“同时保留 C2PA 与 HiddenShield anchor”的 SDK 能力。
- 下一商业化任务：比较 post-embed resign 与兼容容器方案的密钥托管、延迟、成本、计量和失败回滚，再冻结生产方案。

## 2026-07-27 Internal Post-Embed Resign 商业边界

- 已完成：internal-only ephemeral signer 原型证明最终 PNG 可同时读取 C2PA active manifest 与 verified V3。
- 商业边界：C2PA 为非受信任本地自签，不能销售为生产签名、平台接受、合规结果或付费 SDK 能力。
- 风险：production signer receipt、KMS/HSM、Profile entitlement、失败回滚、最终 hash 绑定、延迟和计量均未冻结。
- 下一商业化任务：冻结 production post-embed signing command 与成本/计量合同；真实 credential 和 SDK 发放继续关闭。

## 2026-07-27 Production Post-Embed Signing 合同冻结

- 已完成：冻结 production command 的 signer receipt、Profile entitlement、最终 hash、双回读、失败不返回产物和计量排除合同。
- 商业语义：只有最终签名 PNG 完成双回读并成功 confirm 的 `confirmed_marked_image` 才计量；外部 signer 成本但 confirm 失败只进入内部异常成本账。
- 风险：外部 signer 与 PostgreSQL 非单事务，必须依靠 idempotency、orphan-signing event、结果隔离和相同 request digest 重试。
- 下一商业化任务：冻结并测试 JSON schema/fixtures，再评审 signer 供应商成本、超时预算、重试上限和套餐计量单位；SDK 发放继续关闭。

## 2026-07-27 Post-Embed Signing Schema Gate

- 已完成：三份 Schema 与七类 fixture contract test 全绿，成功计量仅限一次 committed `confirmed_marked_image`；所有失败与 duplicate replay 均零新增客户计量。
- 已冻结：signer 成功但 confirm rollback 进入内部 orphan-signing 成本/异常账，不进入客户成功用量。
- 风险：真实 signer 延迟、供应商成本、超时、重试上限和生产证书链仍无外部配置，继续挂起。
- 下一商业化任务：实现 internal-only command 并记录每阶段耗时与失败分类，为 signer 成本和套餐计量评审提供内部数据。

## 2026-07-27 Internal Post-Embed Signing PostgreSQL Gate

- 状态：`internal_post_embed_signing_postgres_gate_passed_no_external_release`。新增 internal-only post-embed signing command module 与 `0007_ai_transparency_post_embed_signing` additive migration。
- 前置 Gate：命令重新核验 active production license、active production credential、post-embed scope、production issuer mode、custody 字段，以及三项 active versioned Profile entitlement；受控 authorization、signer、双回读和 artifact store 均通过 interface 注入，缺失真实 provider 时不得替换为默认放行实现。
- 事务语义：signer 在 PostgreSQL confirm 事务外隔离；signer receipt/final hash 与 C2PA/V3 双回读通过后，signing projection、Manifest、Evidence、Marker、显式标签 receipt、`confirmed_marked_image` ledger、confirm audit 和 signing audit 才在同一事务提交。confirm 失败时上述成功写入全部回滚，只在独立补偿事务记录 `orphaned` execution 与 append-only `orphan_signing` audit，产物隔离且不返回。
- 验证：一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke_post_embed_signing` 完成 33 表、36 索引的 0001–0007 up/down smoke；七类 fixture 均升级为真实事务 QA。success 仅产生一次计量；signer rejected、receipt/hash mismatch、C2PA readback failure、V3 readback failure 均零 confirm/ledger；confirm rollback 仅保留 orphan 证据；duplicate replay 不再次调用 signer、不新增 audit/ledger，并返回既有已提交产物。
- 商业边界：当前 QA 使用受控 signer/readback 与内存 artifact store，不是生产 signer、受信任 C2PA chain、持久化产物服务、SDK、公共 Resolver、客户 credential 或 SLA。真实 IAM/KMS/HSM/signer 配置继续作为外部依赖挂起。
- 下一商业化任务：实现同 idempotency key 的 PostgreSQL signing reservation/lease 与 durable artifact finalize/recovery Gate，证明并发 replay 不产生第二次外部 signer 成本，并冻结 artifact commit 失败后的可恢复状态。

## 2026-07-27 Signing Reservation 与 Artifact Recovery 商业 Gate

- 状态：`internal_signing_reservation_artifact_recovery_verified`。新增 `0008_ai_transparency_signing_reservation_artifact_recovery`，冻结 `reserved → signed_staged → artifact_pending → confirmed/orphaned` 状态机。
- 并发成本 Gate：同 idempotency key 使用 PostgreSQL advisory lock 和唯一 reservation projection 串行化；两个真实 PostgreSQL 连接并发执行时，signer invocation、execution、signer receipt、confirm 和 `confirmed_marked_image` 均只有一次，第二连接只 replay 已提交结果。
- 跨崩溃幂等：由 idempotency key/request digest 确定性生成 `signerInvocationKey` 并绑定 signer receipt。真实 signer adapter 必须把该 key 传给供应商幂等接口；否则只能承诺 live 并发最多一次调用，不能承诺进程崩溃窗口绝不产生第二笔供应商成本。
- 产物与计量 Gate：confirm 后 durable finalize 未完成时状态为 `artifact_pending`，不返回产物，ledger 保持 `pending`，客户计量为零；恢复成功后 execution `confirmed` 与 ledger `committed` 在同一 PostgreSQL 事务提交，不重签、不重复 confirm、不重复计量。
- 验证：`hiddenshield_migrate_smoke_signing_reservation` PostgreSQL 16 库完成 33 表/39 索引、0001–0008 up/down；九类 JSON fixture 与九场景真实事务 QA 全绿。
- 商业边界：当前 durable store 与 signer 为受控内存/QA adapter，不是生产对象存储、真实 signer、供应商幂等证明或 SLA。
- 下一商业化任务：实现 production signer idempotency adapter contract 与 durable object store adapter contract，并加入 reservation 后、signer 返回后、stage 后、confirm 后四个进程崩溃点恢复演练。

## 2026-07-28 Adapter Receipt 与四崩溃点恢复商业 Gate

- 状态：`internal_adapter_receipt_crash_recovery_verified`。新增 `0009_ai_transparency_adapter_receipts_crash_recovery`，production signer receipt 冻结 result reference、幂等 disposition、billable invocation identity；object-store stage/finalize receipt 冻结 execution、invocation key、final hash、object version、idempotency key、durability status、有效期和 provider signature。
- 成本 Gate：reservation 后恢复仅发起一次 signer 请求；signer 返回后与 artifact stage 后恢复允许两次 adapter 请求，但受控 provider 对同一 invocation key 返回 replay，billable invocation 始终为 1；artifact stage 唯一写入始终为 1。
- 事务 Gate：confirm 后崩溃保留 `artifact_pending` 与 pending ledger，恢复只执行幂等 finalize；四类恢复最终均只有一个 execution、一个 confirm audit、一个 manifest、一个 committed ledger 和一个最终 artifact。
- 验证：一次性 PostgreSQL 16 数据库 `hiddenshield_migrate_smoke_crash_recovery` 完成 33 表/42 索引、0001–0009 up/down；十三类 JSON contract、十三场景真实 PostgreSQL QA、92 个 backend library tests 全绿。
- 商业边界：当前 signer/object-store 是受控 QA provider，不代表真实供应商 receipt 签名、对象存储 durability、kill/restart 编排、生产 SLA 或供应商账单争议证据。
- 下一商业化任务：实现 internal-only recovery worker，扫描 expired `reserved` 与超时 `artifact_pending`，加入退避、dead-letter、成本异常和恢复审计指标；真实 provider 配置到位后再执行进程 kill/restart 演练。

## 2026-07-28 Internal Recovery Worker 商业 Gate

- 状态：`internal_recovery_worker_verified`。新增 `0010_ai_transparency_post_embed_recovery_worker` 与 internal-only batch worker。
- 扫描 Gate：仅扫描 signer lease 已过期的 `reserved`、超过 finalize timeout 的 `artifact_pending` 和 worker lease 已过期的 recovery claim。
- 并发 Gate：PostgreSQL `FOR UPDATE SKIP LOCKED` 保证双 worker 对同一 execution 最多一个 claim；claim、attempt、worker lease 与 audit 原子提交。
- 成本与计量：reserved 恢复继续复用稳定 signer invocation key；artifact pending 恢复不重新签发、不重复 confirm 或 ledger。失败按指数退避，达到最大次数进入 dead-letter，不产生客户成功计量。
- 验证：一次性 PostgreSQL 16 库完成 34 表/45 索引、0001–0010 up/down；expired reserved、artifact timeout、三次退避 dead-letter、双 worker 单 claim 四类 worker QA 全绿；append-only audit UPDATE/DELETE 均被拒绝。
- 商业边界：当前是可由内部调度器调用的 batch module，不是已部署 production daemon、生产监控、真实 provider 恢复、客户 SLA 或供应商成本担保。
- 下一商业化任务：冻结 internal dead-letter inspect/requeue command，并接入既有双人审批状态机；禁止运维人员直接 UPDATE dead-letter projection。

## 2026-07-28 Internal Dead-Letter Inspect / Requeue 商业 Gate

- 状态：`internal_dead_letter_governance_verified`。新增 `0011_ai_transparency_dead_letter_requeue_command`、internal inspect/requeue command 与 PostgreSQL QA。
- 授权边界：inspect 仅允许 readonly auditor；production requeue 固定经过 requester、独立 security approver、security reference 验真和 system executor。
- 成本边界：requeue 本身不计客户成功量；仅后续 worker 恢复至 confirmed/finalized 后沿用既有单次 signer、artifact、confirm 和 ledger 规则。
- 并发 Gate：approved requeue 持有 execution 行锁期间，worker `FOR UPDATE SKIP LOCKED` claim 为 0；提交后仅一次恢复成功。
- 验证：35 表/46 索引、0001–0011 up/down；inspect append-only、摘要拒绝、同人审批拒绝、未审批执行拒绝、重复 submit、五段审计、审计故障全回滚均通过。
- 商业边界：仍不是客户控制台、收费运维 API、production daemon、真实 provider 演练、SDK、公共 Resolver、production credential 或 SLA。
- 下一商业化任务：冻结 `confirmed/finalized` delivery envelope 与内部交付授权，明确可计量成功产物的 final hash、receipt、Profile 和恢复状态绑定。

## 2026-07-28 Confirmed / Finalized Delivery Envelope 商业 Gate

- 状态：`internal_delivery_envelope_verified`。新增 `0012_ai_transparency_confirmed_delivery_envelope`、append-only envelope projection 和 Desktop/mobile 共享校验。
- 成功交付边界：仅 `confirmed + finalized + recovery completed` 可生成 envelope；其他 signing/artifact/recovery 状态均零交付。
- 计量边界：envelope 创建和 replay 不新增计量；收费成功量仍只来自已 committed 的 `confirmed_marked_image` ledger。
- Profile 商业绑定：envelope 固定携带 entitlement version/digest、technical Profile ids 和 regional Profile id，避免交付层脱离购买授权与地区 Profile。
- 验证：PostgreSQL 36 表/47 索引、0001–0012 up/down；创建/replay、append-only、recovery 未完成拒绝及 Desktop/mobile 同 fixture fail-closed 全绿。
- 商业边界：当前不是客户下载 API、SDK response、公共 Resolver、客户 vault record、production object-store retrieval、法规结论或 SLA。
- 下一商业化任务：冻结 internal delivery authorization/retrieval command，加入 entitlement、短期下载授权、object-store receipt 和 envelope digest Gate。

## 2026-07-28 Internal Delivery Authorization / Retrieval Gate

- 状态：`internal_delivery_authorization_retrieval_postgres_gate_passed_no_external_release`。
- 已完成：冻结 60–900 秒短期单次下载授权，绑定 active License、当前 versioned Profile entitlement、object-store finalize receipt digest 与 delivery envelope digest；明文 token 仅返回一次，数据库只保存 SHA-256。
- 已完成：PostgreSQL `0013_ai_transparency_delivery_authorization_retrieval` 增加授权投影和 append-only 下载审计；并发检索最多一个连接读取对象并返回 package，replay、错误 token 和过期授权 fail-closed。
- 已完成：成功 package 增加 retrieval receipt；Desktop/mobile 必须经共享 `watermark-core::validate_ai_delivery_import` 才能继续 vault/import，拒绝响应不暴露可导入摘要。
- 计量边界：下载授权、下载失败、下载成功和端侧 admission 都不新增 `confirmed_marked_image` 客户用量；外部 signer 成本仍只由既有签发合同处理。
- 商业边界：仍无 SDK、公共 Resolver、客户下载 UI、生产 credential、生产 object-store/IAM/KMS/HSM/signer 或 SLA，不得作为可销售下载能力。
- 验证：PostgreSQL 16 smoke 通过 38 表、49 索引与 0001–0013 up/down；真实 signing QA 通过授权并发一胜一败、单次对象读取、replay/invalid/expired、Profile revoke-after-grant、artifact unavailable、tampered bytes 拒绝和 audit UPDATE/DELETE 拒绝。
- 下一商业化任务：冻结 internal delivery revoke 与下载资源预算（最大 bytes、content-type、限速、超时），再评审付费 SDK 的 delivery API 套餐与计量模型。

## 2026-07-28 Internal Delivery Revoke / Resource Budget Gate

- 状态：`internal_delivery_revoke_resource_budget_postgres_gate_passed_no_external_release`。
- 固定预算：单个 PNG 最大 64 MiB、content type 仅 `image/png`、object-store 读取超时 5 秒、每 License 每分钟最多 30 次 claim；预算固化到 authorization grant 和数据库 CHECK，调用方不能放大。
- 撤销：同作用域 `ai_transparency_security_approver` 可将 active authorization 原子置为 revoked；重复撤销幂等，consumed/expired 不可撤销，revoke/retrieve 并发最多一方成功。
- 失败成本：rate limited 在对象读取前拒绝且不消费 authorization；size/MIME/timeout 在 claim 后失败并消费 authorization，全部零 package、零新增 `confirmed_marked_image` 计量。
- 审计：新增 append-only `authorization_revoked`；只记录 revoker snapshot 与 revoke reason SHA-256，不记录原因原文、token、bytes 或 Secret。
- 验证：PostgreSQL 16 通过 39 表、50 索引、0001–0014 up/down；真实 QA 覆盖 revoke/replay、并发冲突、超限、错误 MIME、timeout 和限速。
- 商业边界：仍无 SDK、公共 Resolver、客户下载/import UI、生产 credential、生产 provider 或 SLA；不得作为收费下载能力发布。
- 下一商业化任务：实现内部 rate-limit window cleanup 与安全观测摘要，再冻结付费 SDK delivery API 的套餐单位、超额策略和客户可见错误边界。

## 2026-07-28 Delivery Security Observability 商业 Gate

- 状态：`internal_delivery_security_observability_postgres_gate_passed_no_external_release`。
- 保留：rate-limit minute window 保留 24 小时；聚合 security metric snapshot 保留 90 天，保留期内禁止修改或删除。
- 告警：15 分钟固定窗口评估 integrity、revoked access burst、rate pressure、artifact availability 和 failure ratio；阈值已冻结并进入 fixture/QA。
- 审计导出：只允许 readonly auditor 导出最多 31 天的 aggregate-only summary；不返回 raw audit、媒体标识、authorization/envelope ID、token、bytes 或 Secret。
- Cleanup：system executor 使用 `SKIP LOCKED` 每批清理最多 1,000 条到期 rate window 和 metric snapshot；并发不重复删除。
- 计量边界：summary、export、cleanup 和 alert evaluation 均不产生客户计量，也不改变 `confirmed_marked_image` ledger。
- 验证：PostgreSQL 16 通过 41 表、53 索引、0001–0015 up/down；真实 QA 覆盖 critical summary、90 天 retention、31 天导出上限、错误角色零写入、并发 cleanup 和 audit 不可变。
- 商业边界：无客户安全仪表盘、外部告警渠道、SDK、公共 Resolver、production provider 或 SLA。
- 下一商业化任务：实现 internal security incident ack/resolve 与定时 cleanup runner，再评审客户可见 delivery reliability 指标和付费套餐边界。

## 2026-07-28 Delivery Security Incident / Cleanup Runner 商业 Gate

- 状态：`internal_delivery_security_incident_runner_postgres_gate_passed_no_external_release`。
- 已完成：新增 `0016_ai_transparency_delivery_security_incident_runner`，把 15 分钟 warning/critical summary 在同一事务内投影为 active incident；同 scope/alert key 并发最多保留一个 active incident，resolved 后复发创建新 incident。
- 治理：`ack_delivery_security_incident` 与 `resolve_delivery_security_incident` 强制复用既有 change request、requester/approver 四眼、execution 与 append-only audit；digest mismatch、同 actor 审批和 stale control version 均零业务写入。
- 调度：15 分钟 schedule、5 分钟 lease、`FOR UPDATE SKIP LOCKED` 单 claim、1–60 分钟指数退避和 append-only runner audit 已冻结；runner 复用既有 cleanup command。
- 计量边界：incident projection、ack/resolve、schedule、runner 与通知状态均不产生客户计量，也不改变 `confirmed_marked_image` ledger。
- 外部依赖：PagerDuty、邮件和短信 adapter 继续挂起；当前不得伪造通知 receipt，也不得因缺少通知配置阻塞内部 incident/cleanup Gate。
- 验证：PostgreSQL 16 通过 45 表、58 索引、0001–0016 up/down 与空 schema rollback；真实 QA 覆盖 active incident 并发唯一、ack→resolve、execution replay、resolved 后复发、三类零写入拒绝、双 runner 单 claim 及双类 audit 不可变；backend 92/92 tests 通过。
- 商业边界：当前仍为 `只能内部测试`，不是客户可见 reliability dashboard、通知 SLA、SDK、公共 Resolver、生产 credential 发放或生产事故响应服务。
- 下一商业化任务：冻结 internal incident inspect/list 与 provider-neutral durable notification outbox 合同，在不接入真实 PagerDuty/邮件/短信前实现 outbox 去重、lease、重放和 audit Gate。

## 2026-07-28 Incident Inspect / Notification Outbox 商业 Gate

- 状态：`internal_incident_inspect_notification_outbox_postgres_gate_passed_no_provider_delivery`。
- 已完成：新增 `0017_ai_transparency_delivery_security_notification_outbox`，提供 scope-bound incident inspect/list、provider-neutral durable outbox、唯一 dedupe key、5 分钟 lease、expired lease reclaim、幂等 replay 与 append-only audit。
- 事务边界：incident opened、became critical、acknowledged、resolved 的 outbox enqueue 与 incident projection/change execution 位于同一 PostgreSQL 事务。
- 商业安全：outbox 仅有 `pending`、`leased`、`retry_scheduled`，不存在 `sent`、`delivered` 或 provider success；当前不接受 provider receipt，不产生通知计费或 SLA 证据。
- 数据边界：inspect/list 与 payload 不返回媒体、authorization、delivery envelope、token、bytes、Secret、收件人或 provider endpoint。
- 验证：PostgreSQL 16 通过 48 表、62 索引、0001–0017 up/down 与空 schema rollback；真实 QA 覆盖 incident inspect/list、错误角色拒绝、重复 enqueue 去重、双连接单 item claim、replay 幂等、二次 claim、expired lease reclaim 及双类 audit 不可变。
- 外部依赖：PagerDuty、邮件和短信 endpoint、Secret、路由、域名认证与模板审批继续挂起；不得伪造 provider delivery。
- 商业边界：仍为 `只能内部测试`，SDK、公共 Resolver、客户 incident UI/API、生产 credential 发放和通知 SLA 继续关闭。
- 下一商业化任务：冻结 provider adapter receipt、destination policy 与 outbox completion/dead-letter 合同；在真实 provider 配置到位前先实现 fail-closed adapter interface、receipt schema 和零发送模拟 Gate。

## 2026-07-28 生产导向 MVP 定义校准

- 决策：License/Profile、四眼审批、credential custody、signing recovery、delivery security、incident 和 notification outbox 均属于 AI Transparency 付费 B2B 产品的 MVP 核心控制面，不作为范围偏离处理。
- MVP 定义：最小可生产、可授权、可审计、可恢复的基础设施，而不是只完成 SDK happy path 的演示原型。
- 发布纪律不变：生产导向不代表当前可销售；真实 provider、平台接入、SDK、公共 Resolver、production credential 和 SLA Gate 未通过前继续 `只能内部测试`。
- 顺序约束：完成 provider adapter receipt/completion/dead-letter 内部 Gate 后，主线必须转向平台 SDK、最小 API facade、免费 Resolver 和设计伙伴接入，避免无限扩展内部运维控制面。
- provider adapter receipt 与零发送模拟 Gate 已完成。
- 下一商业化任务：启动 `packages/ai-transparency-sdk` 的授权、Profile admission、marking session、confirm 和 receipt 最小 surface。

## 2026-07-28 Notification Provider Delivery 商业 Gate

- 状态：`internal_notification_delivery_postgres_gate_passed_no_external_provider_release`。
- 已完成：冻结 `docs/AI生成内容标识Notification_Provider_Adapter与Delivery_Gate合同.md`、JSON Schema 与 fixture，并新增 PostgreSQL additive migration `0018_ai_transparency_notification_delivery_gate`。
- 策略边界：destination policy 在 adapter 调用前绑定，固定 policy identity/version/digest、adapter、event/priority scope、attempt budget 与 retry budget。
- Receipt 边界：provider receipt 绑定 notification、payload、policy、adapter 和 invocation key；mismatch、过期或 lease mismatch 均零写入。
- 事务边界：receipt、outbox completed 投影与 append-only audit 同事务提交；completion replay 不重复 receipt。
- 恢复边界：支持 retry/dead-letter、dead-letter recovery idempotency 和 expired lease recovery count。
- 商业安全：sandbox-only zero-send simulation 强制 `deliveryClaimed=false`，不得作为外部通知送达、SLA 或客户计费证据。
- 验证：PostgreSQL 16 通过 49 表/66 索引、0001–0018 up/down smoke 与完整事务 QA。
- 外部依赖：真实通知 endpoint/Secret/routing、provider receipt authenticity 和生产恢复演练继续挂起。
- 下一商业化任务：实现授权付费 `packages/ai-transparency-sdk` 最小 API surface，并绑定 production license、Profile entitlement 与 `confirmed_marked_image` receipt。

## 2026-07-28 AI Transparency SDK / Platform Facade 商业 Gate

- 状态：`internal_server_sdk_and_framework_neutral_facade_verified_no_backend_endpoint_release`。
- 已完成：新增 `@hiddenshield/ai-transparency-sdk`，冻结 production admission、session、PNG submission、confirm 和 `confirmed_marked_image` receipt 合同。
- 授权边界：SDK 仅允许 trusted server runtime，要求非占位 production credential 和 HTTPS；credential 不进入 response、receipt、错误或客户端 bundle。
- Profile 边界：admission 固定绑定 license、tenant/workspace、issuer mode、regional/technical Profile、entitlement version/digest 与过期时间。
- 完整性边界：SDK 对原图和 marked PNG 分别计算 SHA-256；marked bytes 与服务端摘要不一致时 confirm 零调用。
- 计量边界：只接受 `confirmed_marked_image + quantity=1 + committed`，并要求 receipt license/session 与 admission/session 一致；duplicate replay 复用原 ledger。
- Facade：新增四个固定路径的 framework-neutral API handler，强制平台侧 authorization callback 后调用 SDK。
- 验证：JSON contract、TypeScript build、9 个 SDK/facade tests 与 npm pack dry-run 通过；包内容仅包含 dist、README 和 package metadata。
- 商业边界：当前包未发布、真实 HiddenShield 后端四端点不存在、生产 credential 不发放、无客户 SLA 或设计伙伴验收。
- 下一商业化任务：实现 PostgreSQL-backed admission/session/mark/confirm internal endpoint，形成 SDK → API → 现有控制面/执行器/confirm 的真实闭环。

## 2026-07-28 PostgreSQL Platform API 商业 Gate

- 状态：`internal_sdk_to_postgresql_platform_api_gate_passed_no_public_release`。
- 已完成：新增 `0019_ai_transparency_platform_api` 与独立 PostgreSQL-only Axum router，实现 admission、session、mark、confirm 四个 internal endpoint。
- 授权边界：admission 校验 production credential/license/tenant/workspace/issuer mode，并绑定 regulatory/technical versioned Profile entitlement set digest。
- Custody 边界：session 创建复用现有 credential custody；缺失或不可用 provider 继续 fail-closed。
- 执行边界：mark 复用 image marking executor 与 `watermark-core`，完成 PNG 写入和写后回读，但不提前创建 Manifest 或计量。
- 计量边界：confirm 在 PostgreSQL 原子事务中复用 confirm command，只提交一条 `confirmed_marked_image + quantity=1 + committed`；duplicate replay 复用原 ledger。
- 安全边界：无效 credential 在 session claim 前拒绝；Profile 拒绝、PNG/hash mismatch、confirmation token/hash mismatch 均不得产生计量。
- 验证：一次性 PostgreSQL 16 完成 SDK → facade → HTTP → Axum → PostgreSQL E2E；0001–0019 up/down smoke、backend 92 tests、SDK 9 tests 通过。
- 商业边界：能力继续 `只能内部测试`；SDK 未发布，公网 gateway、production credential、真实 IAM/KMS/HSM 和客户 SLA 继续关闭。
- 下一商业化任务：实现免费公共 Resolver 最小只读接口，冻结匿名无计量边界与公共字段最小化，然后准备真实设计伙伴接入包。

## 2026-07-28 免费公共 Resolver 商业 Gate

- 状态：`internal_anonymous_public_resolver_gate_passed_no_public_deployment`。
- 已完成：新增 `0020_ai_transparency_public_resolver`、三个 confirmed-only PostgreSQL 公共 view 和独立匿名 Axum Resolver。
- 免费边界：无需 API key、license admission 或媒体上传；不扣 quota，不创建 ledger，不写 platform internal audit。
- 字段边界：只返回 Manifest identity/status、claim、marker/evidence summary、Profile status、时间、固定 warning 与 `legalConclusion=false`。
- 隐私边界：不返回 license、tenant/workspace、session/admission、subject digest、provider/system/model、ledger、credential、token、signer/object-store/custody receipt。
- 语义边界：未找到记录不等于非 AI；issuer trust 固定 `not_evaluated`；接口不提供法律、版权或作者身份结论。
- 验证：未 confirm 记录不可见；confirm 后匿名 UID/Manifest 双查询一致；not-found 最小响应；查询后 ledger 保持 1、platform audit 保持 5。
- 发布边界：当前仍为内部 runtime Gate；公网域名、CDN/WAF/DDoS、IP rate-limit、隐私日志策略、SLA 和生产发布继续关闭。
- 下一商业化任务：冻结真实设计伙伴 sandbox 接入包、Profile mapping questionnaire、示例集成和验收矩阵。

## 2026-07-28 真实设计伙伴 Sandbox 接入包商业 Gate

- 状态：`internal_design_partner_sandbox_kit_verified_external_partner_configuration_required`。
- 已完成：新增 private package `@hiddenshield/ai-transparency-design-partner-kit`，冻结 onboarding、CN/EU/US-CA Profile mapping questionnaire、server-only SDK/API 示例、Resolver link contract 与 12 场景验收矩阵。
- 商业模式边界：Evaluation/Sandbox 接入用于验证年度平台授权、Profile 包、部署包与 `confirmed_marked_image` 计量假设，不包含 production credential、公开 npm 发布、客户 SLA 或法律意见。
- Secret 边界：伙伴 bundle 只允许 `secret://` 引用；真实 endpoint、credential、联系人和法务/采购材料必须由外部系统注入，不得写入可分发包。
- 验收边界：`sandbox_accepted` 要求真实非占位 HTTPS endpoint、12 场景全部通过且均有不可变 evidence；`blocked_external` 不计为通过。
- Resolver 边界：链接只绑定 watermark UID 或 Manifest ID，继续匿名、无计量、最小公共字段和 `legalConclusion=false`。
- 验证：root contract verifier、5 个 package tests、模板 preflight 与 root 聚合命令通过；未配置模板稳定返回 `configuration_required`。
- 风险：尚无真实设计伙伴身份、Sandbox endpoint、Secret 注入、运行流量和书面验收，因此不能用于价格确认、SLA、生产 entitlement 或收入确认。
- 下一商业化任务：选择首个 AIGC 图片平台设计伙伴，签署 Sandbox/数据处理边界，注入外部配置并完成 12 场景证据化验收，再用真实月量、延迟和采购反馈更新 Evaluation License 与平台授权报价假设。

## 2026-07-28 Synthetic Sandbox QA 补充 Gate

- 状态：`internal_synthetic_rehearsal_verified_not_partner_acceptance`。
- 已完成：新增 deterministic synthetic Sandbox QA，复用 SDK/facade success、Profile 拒绝、无效授权、mark/confirm、单次计量/replay 和最小 Resolver 响应形状。
- 输出边界：固定 `executionMode=synthetic_non_acceptance`、`acceptanceStatus=not_real_partner_acceptance`、`readiness=configuration_required`。
- 商业边界：synthetic 运行不产生设计伙伴 acceptance、生产 entitlement、可计费用量、价格证据、SLA、法律结论或收入确认。
- 验证：8 个接入包测试与 root synthetic verifier 通过；每次运行生成 12 个 content-addressed synthetic evidence reference，但不得在真实 partner bundle 中复用。
- 下一商业化任务：保持 synthetic QA 作为招募前回归；取得首个伙伴外部配置后重新执行真实 12 场景，再更新价格和 Evaluation License 假设。

## 2026-07-28 AI Transparency CI 必跑 Gate

- 状态：`ci_required_contract_gate_implemented`。
- 已完成：新增独立 GitHub Actions `AI Transparency contract gate`，运行 `npm run ai-transparency:ci`。
- Gate 内容：SDK contract/test、设计伙伴 package contract/test、template preflight 与 synthetic Sandbox QA。
- 边界：CI 只执行无外部依赖的确定性合同回归；不连接 PostgreSQL、真实伙伴 endpoint、Secret、provider 或生产 credential。
- 验证：本地 `npm run ai-transparency:ci` 通过；synthetic 输出仍固定为非真实 acceptance。
- GitHub Gate：仓库 ruleset `main-and-master-required-checks` 已激活，匹配 `refs/heads/main` 与 `refs/heads/master`，并将 `AI Transparency contract gate` 与既有三个 CI checks 设为 required；工作流合并后的首个 PR 将产生该 check。
- 下一商业化任务：保持该 Gate 为合并前检查；取得真实伙伴外部配置后另行新增受控、不可伪造的真实 Sandbox evidence Gate。

## 2026-07-28 AI Transparency Phase B 发布出口状态校准

- 状态：`internal_platform_api_and_public_resolver_verified_external_provider_activation_pending`。
- 已完成：PostgreSQL-backed admission/session/mark/confirm internal API 与 confirmed-only 免费公共 Resolver 均已通过独立 Gate；它们不再是 Phase B 未完成出口。
- 仍挂起：真实 Internal IAM、KMS/HSM、signer、object-store 与通知 provider 的 Secret 注入、真实性校验和恢复演练均依赖外部环境与配置。
- 商业边界：internal API、Resolver、SDK 和 synthetic Sandbox QA 继续只用于内部测试或招募前演练；不得据此发布 SDK、发放 production credential、开放公网 Resolver、承诺 SLA 或确认收入。
- 验证：PR #2 的 Ubuntu、Windows、Cloud sync contract/E2E 和 AI Transparency required contract gate 全部通过。
- 下一商业化任务：取得首个真实设计伙伴及其外部配置后，执行受控的 12 场景 Sandbox evidence 验收；在此之前保持 production provider activation 与真实伙伴验收挂起。

## 2026-07-28 AI Transparency External Readiness 配置包

- 状态：`configuration_required_external_only`。
- 已完成：新增面向基础设施、安全与设计伙伴的统一交接包，包含 provider/partner/approval 引用模板、最小权限清单、恢复演练出口和 12 场景 Sandbox 验收入口。
- Secret 边界：模板仅允许 `secret://`、KMS/HSM URI、HTTPS endpoint、`runbook://` 和不可变 evidence 引用；不包含 token、私钥、pepper material、客户媒体或真实伙伴身份。
- 验证：`npm run ai-transparency:external-readiness` 断言模板保持 `configuration_required`，拒绝明文 Secret、非 HTTPS endpoint 和伪造的伙伴 acceptance；该检查纳入 AI Transparency CI Gate。
- 商业边界：配置包不构成 provider activation、Sandbox acceptance、生产 entitlement、收入确认、SDK 发布或 SLA。
- 下一商业化任务：由基础设施与首个设计伙伴在各自受控系统填写真实引用；随后执行 provider recovery 演练与 12 场景证据化 Sandbox 验收。

## 2026-07-28 AI Transparency External Readiness 双模式验收

- 状态：`template_and_internal_review_preflight_verified_external_execution_pending`。
- 已完成：将 readiness preflight 冻结为 `template` 与 `review` 两种模式；前者只允许 `configuration_required` 占位模板，后者只允许无占位符的 `ready_for_internal_review` 引用 manifest。
- Fail-closed：review 拒绝 HTTP、localhost、`*.example` / `*.test` / `*.invalid` endpoint、明文 Secret、错误 URI scheme、未配置 KMS provider、占位符和预先填入的伙伴 acceptance evidence。
- 验证：加入完整引用 fixture 与 unsafe reference 拒绝 fixture，`npm run ai-transparency:external-readiness` 同时验证两种成功/拒绝路径，并由 AI Transparency CI Gate 强制执行。
- 商业边界：`ready_for_internal_review` 不等于 provider activation、Sandbox acceptance、生产 entitlement、SDK 发布、收入确认或 SLA。
- 下一商业化任务：基础设施与伙伴提交受控 manifest 后先执行 review preflight；通过后才进入隔离环境的 provider recovery 与 12 场景 evidence Gate。
