# Windows 候选签名顺序修复设计

状态：`已实施；MSI 提升权限安装验证待完成；当前 0.1.0 候选仍不得发布`

## 1. 目标与事实

当前候选的 NSIS 外层安装器已签名，但其新安装的 `hidden_shield.exe` 为 `NotSigned`。根因是对已完成打包的外层 NSIS / MSI 和测试安装目录中的 EXE 分别签名；外层安装器中封装的仍是打包前未签名 EXE。

下一候选必须满足以下不可变顺序：

1. 编译 inner `hidden_shield.exe`。
2. 由 Tauri 的 Windows signing 配置在写入 bundle type 信息之后、封装 inner EXE 之前完成 Authenticode 签名和时间戳。
3. 封装 NSIS 与 MSI，并对外层文件签名和时间戳验证。
4. 在全新安装目录验证 NSIS / MSI 各自产生的 `hidden_shield.exe` 仍为 `Valid`，且 signer 与候选 EXE 一致。

本设计只覆盖 Windows 桌面候选；移动端继续冻结。自签证书的受信范围仍仅限服务方与受管客户 trust store，不形成公共 CA 或 SmartScreen 承诺。

## 2. 签名候选模块

新增一个深模块：`WindowsSignedReleaseCandidate`。调用方只提供版本、源提交、证书来源、时间戳服务和 run ID；模块负责构建顺序、哈希绑定、签名、安装验证、证据和失败停止。

建议接口：

```text
Invoke-WindowsSignedReleaseCandidate
  -RunId <immutable-id>
  -SourceCommit <git-sha>
  -CertificateThumbprint <thumbprint>
  -TimestampUrl <rfc3161-url>
  -Target x64
```

接口不接受“对已安装 EXE 单独签名”或“对已打包候选补签 inner EXE”的参数。这两类动作不能修复已封装负载，必须直接失败。

内部 adapter：

- `CertificatePreparationAdapter`：导入和校验证书；不修改提交中的 `tauri.conf.json`。
- `TauriBuildAdapter`：只构建 inner EXE。
- `AuthenticodeSignerAdapter`：使用 SignTool 完成签名、时间戳和即时验证。
- `TauriBundleAdapter`：只打包，不重新编译、不再次签名 inner EXE。
- `InstallerProbeAdapter`：在隔离安装环境验证 NSIS / MSI 实际产生的 inner EXE。
- `CandidateEvidenceAdapter`：写入不可变 manifest、哈希和 Gate 摘要。

该模块的深度在于：调用方只关心“生成可分发候选或失败”，而不会分别编排编译、补签、安装目录和证据路径。

## 3. 下一候选的严格顺序

### 3.1 准备与冻结

1. 在独立 worktree 从指定 Git commit 开始，确认工作树干净、版本号一致、目标输出目录为空。
2. 导入证书并验证私钥、Subject、thumbprint、RFC3161 时间戳服务；当前验证端点为 `http://timestamp.digicert.com`。记录证书元数据，不记录 PFX、密码或私钥。
3. 创建 `candidate-manifest.json`，先写入 run ID、Git SHA、版本、证书 thumbprint、时间戳服务和预期产物名。
4. 不使用当前候选的 NSIS / MSI / 安装目录作为输入；它们只作为失败证据保留。

### 3.2 编译、类型信息写入与 inner EXE 签名

1. 注入 Tauri Windows signing 配置，但只允许在隔离 worktree 修改配置。
2. 运行 `npx tauri build --bundles msi,nsis -- --bin hidden_shield`。
3. Tauri 会先写入 bundle type 信息，再在封装 inner EXE 前调用 Authenticode signing；因此不得在该内部步骤之前用外部 SignTool 预签 EXE。
4. 立即以 `Get-AuthenticodeSignature` 和 `signtool verify /pa /all /v` 验证候选 EXE、NSIS 和 MSI 为 `Valid` 且存在时间戳。
5. 将候选 EXE 的 SHA-256、Signer Subject、thumbprint、时间戳结果写入 manifest。

若任一步失败，停止；不得继续打包，也不得以未签名 EXE 生成安装器。

### 3.3 受控打包

1. 同一 `tauri build --bundles msi,nsis` 调用在内部完成类型信息写入、inner EXE 签名与两类容器封装。
2. 找到唯一的候选 EXE、NSIS 和 MSI 输出；三者都必须为 `Valid`。
3. 将候选 EXE、NSIS、MSI 及后续两个实际安装 EXE 的独立 SHA-256 写入 manifest；哈希用于候选固定，不要求不同 bundle 的 inner EXE 相等。

Tauri 会对不同 bundle 写入类型信息，导致不同容器的 inner EXE 哈希可能不同。签名必须在 Tauri 写入该信息之后、容器封装之前完成；不得在打包后修改 inner EXE。

### 3.4 外层签名

1. 只对已完成的 NSIS 和 MSI 调用 `AuthenticodeSignerAdapter`。
2. 对 inner EXE、NSIS、MSI 三个源文件执行双验证：PowerShell Authenticode 与 SignTool。
3. 将三个最终 SHA-256、签名主体、thumbprint、时间戳和 SignTool 结果追加到 manifest。

若 Tauri 未对任一 outer wrapper 签名，`AuthenticodeSignerAdapter` 只允许补签 NSIS / MSI；它不能替代或覆盖 Tauri 内部时机的 inner EXE 签名。

Tauri 完成容器封装后会把工作目录中的裸 EXE 恢复为未签名构建输出；若要分发 standalone release EXE，编排模块可在此时单独签该裸 EXE。该操作只覆盖 standalone 分发文件，不得被当作 NSIS / MSI 内部 payload 的签名证据。

## 4. 安装后验签 Gate

`RC-RELEASE-001` 与 `RC-RELEASE-002` 必须分层，禁止再用单独签名的测试 installed EXE 代替实际安装结果。

| Gate | 目标 | 必须通过的证据 |
| --- | --- | --- |
| `RC-RELEASE-001` | 源候选签名拓扑 | Tauri 写入 bundle type 信息后生成的候选 EXE、NSIS、MSI 三者均 `Valid`；无“安装后单独补签”步骤 |
| `RC-RELEASE-002A` | NSIS 实际安装 | 新安装目录的 EXE 为 `Valid`、Signer / thumbprint 匹配；开始菜单、卸载入口、首次启动和媒体冒烟通过 |
| `RC-RELEASE-002B` | MSI 实际安装 | 新安装目录的 EXE 为 `Valid`、Signer / thumbprint 匹配；首次启动和媒体冒烟通过 |
| `RC-RELEASE-002C` | 干净离线 Windows | NSIS 与 MSI 分别在无预装 WebView2、物理断网的干净快照完成 `002A` / `002B`；记录 WebView2 安装行为，不将本机证据替代该 Gate |

`release:authenticode-gate:candidate` 的后续实现必须接收并验证五类路径：

1. pre-bundle / release inner EXE；
2. NSIS；
3. MSI；
4. NSIS 新安装 EXE；
5. MSI 新安装 EXE。

Gate 必须拒绝以下情况：

- outer NSIS / MSI 是 `Valid`，但任一新安装 EXE 为 `NotSigned`；
- 任何结果来自安装后单独签名；
- NSIS 与 MSI 未在隔离目录或独立快照分别验证；
- 证书主体、thumbprint 或时间戳不一致。

`uninstall.exe` 本轮只验证入口存在和可调用；它不作为 inner 应用 EXE 的签名替代品。若后续决定对卸载器作签名承诺，必须单独设计 NSIS 生成期签名能力与 Gate。

## 5. 证据与发布准入

每个候选写入：

```text
artifacts/windows-signed-release-candidate/<run-id>/
  candidate-manifest.json
  pre-bundle-inner-exe.json
  source-artifact-signatures.json
  nsis-installed-exe.json
  msi-installed-exe.json
  clean-offline-vm-summary.json
```

`candidate-manifest.json` 至少包含：

- Git SHA、版本、run ID、证书 Subject / thumbprint、时间戳服务；
- pre-bundle inner EXE、post-bundle inner EXE、NSIS、MSI 的 SHA-256；
- NSIS / MSI 各自新安装 EXE 的 SHA-256 与签名状态；
- Gate 结果、失败原因、安装目录仅作临时证据标识；
- `rebuildProhibitedAfterManifest=true`，防止验证中途替换产物。

只有 `RC-RELEASE-001`、`002A`、`002B`、`002C` 全部通过，才允许创建或保留 GitHub Draft Release。任一失败必须撤销 Draft Release 资格，不得通过手工复制、重新签安装目录或覆盖证据来“修复”候选。

## 6. 回滚与失败处理

- 当前 `0.1.0` 候选永久保留为失败证据，不重建、不覆盖、不对外分发。
- 签名、哈希或安装验签失败时，模块立即停止；不得进入媒体 Gate 或上传发布资产。
- 只允许回退到一份已经完整通过全部 Gate 的旧 manifest；不得回退到当前失败候选。
- 下一候选失败时保留哈希和日志，删除仅限该候选生成的临时安装目录；不删除用户标准安装、历史证据或 Git 工作内容。

## 7. 实施准入

实施前必须获得明确批准，并一次性完成以下变更：

1. 新增 `WindowsSignedReleaseCandidate` 编排脚本和 manifest schema。
2. 将 CI 从“打包后补签”切换为本设计的严格顺序。
3. 扩展 Authenticode candidate Gate，要求 NSIS 与 MSI 各自的实际新安装 EXE。
4. 在独立 worktree 构建新的候选，不触碰当前失败候选。
5. 先跑本机隔离安装 Gate，再恢复干净离线 Windows VM Gate。

在批准前，只允许审阅设计与准备隔离环境；不得运行构建、打包、签名或替换当前候选。

## 8. 2026-07-24 实施结果

- 已实现 `scripts/release/invoke-windows-signed-release-candidate.ps1`，通过临时 Tauri Windows signing 配置在 bundle type 写入后、容器封装前签名，并生成不可变 manifest。
- 已实现 `scripts/release/verify-windows-installed-payload.ps1`，分别安装 NSIS / MSI，要求实际安装出的 `hidden_shield.exe` 为 `Valid` 且证书 thumbprint 与候选一致；不同 bundle 的内部二进制哈希不再被错误地要求相同。
- 候选 `20260724-installed-payload-gate-075530` 的 NSIS 新安装 EXE 为 `Valid`；MSI 外层为 `Valid`，但当前非提升会话因 `Error 1925` 无法完成每机安装，Gate 保持失败并记录 `msi-install.log`。
- 下一步：在提升权限的干净 Windows 快照完成 MSI 安装后验签、无预装 WebView2 启动和媒体冒烟；完成前保持 `RC-RELEASE-001`、`RC-RELEASE-002` 阻断。
