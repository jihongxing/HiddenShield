# HiddenShield 桌面媒体内部 RC 评审

评审日期：2026-07-22

评审结论：`BLOCKED`

评审范围：

- 桌面静态图片写入、写后回读、只读验证、裁切 / 变换恢复、视觉质量、误报和资源边界。
- 桌面音频格式、采样率、声道、高位深、时长和容量边界。
- 当前 `0.1.0` 安装候选、默认 `watermark-core` 测试和发布证据完整性。
- 移动端继续冻结，不纳入本轮 RC，也不继承桌面承诺。

机器可读证据：

- `artifacts/desktop-media-internal-rc/20260722/summary.json`
- `artifacts/desktop-media-internal-rc/20260722/release-blockers.md`

## 1. 已通过证据

### 图片

- 冻结图片候选曾完成三张真实照片的 `16 / 16` 十六宫格、`36 / 36` 滑动裁切和 `8 / 8` 独立变换恢复，PSNR `44.10–51.27 dB`、SSIM `0.9951–0.9981`。
- 近 100 MP 样本写入、写后回读、独立核心读取和只读验证通过。
- 34 个 Windows 图片源生成 PNG / JPEG / WebP 共 `102` 个干净变体，误报 `0`。
- 当前源码中的五个正式 V3 图片服务测试全部通过。

### 音频

- 安装版 `20:00` 与精确 `512 MiB` 允许边界通过，`20:01` 与 `512 MiB + 1 byte` 正确拒绝。
- `24-bit WAV`、`24-bit FLAC`、`float32 WAV` 的 mono / stereo 安装版 Gate `6 / 6` 通过，采样率、声道、样本类型和有效位深保持。
- 本地真实文件基线覆盖 WAV / MP3 / FLAC / OGG / M4A × mono / stereo，`10 / 10` 写后立即读取通过。
- 音频资源边界测试、前端构建、共享核心架构契约和音频支持契约通过。

## 2. 关键复验发现

在最终音频安装候选：

- 安装可执行文件 SHA-256：`bb70a401239306947ae518f85601e5d18d47f4c9b5068a2a3f834cad12452bb8`。
- 冻结图片完整 Gate 的前两张真实照片全部通过。
- 第三张 `windows-theme-c-img29.jpg` 在 WebP quality 60 恢复时失败。
- 原保护副本 UID：`HS-9214D504-63C9EFDF-5376CA9B-9A81A854`。
- WebP q60 独立核心读取 UID：`HS-9214D504-63C9EFDF-5336CA9A-9A81A854`。
- 该样本不是单纯“未读取到水印”，而是返回了错误 UID，因此属于取证身份完整性阻断。
- 同一保护副本的旋转 90 / 180 / 270、85% 缩放、JPEG q75 / q60、WebP q75 均恢复正确 UID。

结论：

- 先前单次 `WebP q60` 成功不能证明该边界对不同 UID 稳定。
- 当前不得继续把 WebP quality 60 恢复列为已通过的用户承诺。
- 图片算法保持冻结，直到对失败根因做出明确修复或正式收窄产品口径。

## 3. 发布阻断项

| ID | 级别 | 范围 | 阻断项 | 解除条件 |
| --- | --- | --- | --- | --- |
| `RC-MEDIA-001` | Critical | 桌面图片 | 最终组合候选的 WebP q60 返回错误 UID | 对失败照片建立 UID 多样性回归；修复共享核心或移除 WebP q60 承诺；三张真实照片 × 至少三个独立 UID × 八个变换全部 UID 精确一致 |
| `RC-MEDIA-002` | High | `watermark-core` | 默认完整测试 `110 passed / 7 failed` | 默认 release suite 全绿；六个旧图片测试迁移到明确的 rollback / legacy suite 或修复；暂停范围内 L3 性能测试不得继续污染桌面媒体默认门禁 |
| `RC-MEDIA-003` | High | 桌面音频 | 五格式 × mono / stereo 的 `10 / 10` 证据只来自本地核心基线，不是最终安装候选 Gate | 在最终安装版执行 WAV / MP3 / FLAC / OGG / M4A × mono / stereo 写读与规格保持 Gate，并将证据写入 `artifacts/` |
| `RC-MEDIA-004` | High | 桌面音频 | 未覆盖合法上包络组合 | 执行 `20:00 + 48 kHz + stereo + 高位深` 安装版资源 Gate，记录峰值内存、取消、规格、写后回读和只读验证 |
| `RC-RELEASE-001` | Critical | 桌面发布 | NSIS、MSI、release exe 和 installed exe 均为 `NotSigned` | 对最终候选完成 Authenticode 签名，通过候选签名 Gate，并使用同一签名产物复跑媒体冒烟 |
| `RC-RELEASE-002` | High | 桌面发布 | 尚无干净离线 Windows VM 证明 | 在无预装 WebView2、物理断网的干净 Windows VM 完成安装、启动、图片验证和音频验证 |

## 4. 非阻断限制

- 噪声底 / 环境底噪音频的感知质量继续明确不承诺。
- 低于 30 秒的音频裁切恢复不属于当前 standalone audio 产品承诺。
- L3 视频画面能力不属于本次桌面媒体 RC；其性能测试失败仍需单独处理。
- 移动端冻结，不得把桌面图片恢复或音频资源结论写入移动端 UI、帮助、销售或报告。

## 5. RC 决策

- 不批准当前桌面媒体内部 RC。
- 不批准当前 `0.1.0` 候选对外发布。
- 不继续扩大图片或音频产品口径。
- 保留已通过证据，但任何旧证据不得覆盖最终同一候选的失败结果。

## 6. 推荐下一步

优先处理 `RC-MEDIA-001`：围绕 `windows-theme-c-img29.jpg + WebP q60` 建立 UID 多样性固定回归，确认错误 UID 的触发位和恢复共识路径；随后决定修复共享核心还是把 WebP q60 从正式承诺收窄到 WebP q75。

## 7. RC-MEDIA-001 修复复核

- 结论：选择修复共享核心，不把正式承诺收窄为 WebP q75。
- 根因：首个 checksum-valid 独立包在 UID 位 `73`、`95` 双位翻转后仍发生 8-bit 校验碰撞；旧读取顺序在 25 包正确共识前提前返回该错误 UID。
- 修复：精确读取器改为直接共识、软纠错共识、独立包降级顺序；V3 UID、承载布局、裁切扫描和桌面消费方不变。
- 验证：固定照片三独立 UID WebP q60 回归 `3/3`；安装候选综合图片 Gate、102 样本零误报和架构契约通过。
- RC 判定：整体仍为 `BLOCKED`。RC-MEDIA-001 的核心缺陷已修复，但按本评审原解除条件仍需补三真实照片 × 三 UID × 八变换矩阵；其余五项阻断未改变。
- 证据：`artifacts/image-webp-q60-uid-regression/20260722-diagnostic/summary.json`、`artifacts/image-webp-q60-uid-regression/20260722-green/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-webp-q60-core-fix-installed/summary.json`。

## 8. 更新后的推荐下一步

使用当前安装候选补齐三张真实照片 × 三独立 UID × 八个承诺变换的精确 UID 矩阵，满足 RC-MEDIA-001 的完整解除条件。

## 9. RC-MEDIA-001 正式关闭

- 三张真实照片分别使用三个互不重复的独立 UID，八个承诺变换全部执行，共 `72` 个变换单元。
- UID 精确匹配 `72/72`，独立核心读取 `72/72`，安装版只读验证 `72/72`。
- 三轮均使用 installed exe SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40`。
- RC 生成器复跑后，`RC-MEDIA-001` 已进入 `resolvedBlockers`，活动阻断项由六项降为五项。
- 整体判定继续为 `BLOCKED`，不批准内部 RC 或对外发布。
- 证据：`artifacts/desktop-media-internal-rc/20260722/rc-media-001-closure.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。

## 10. 更新后的推荐下一步

优先处理 `RC-MEDIA-002`：使默认 `watermark-core` release suite 全绿，或将明确的 legacy / rollback-only 预期迁移到独立且有所有权说明的测试套件。

## 11. RC-MEDIA-002 正式关闭

- 六项失败测试引用的是已退役 V2 图片 API，和当前 V3 图片正式能力不一致；这些测试已从默认 release suite 移除。
- 正式图片写入、读取和验证只支持 V3/39；V2 图片写读与 rollback 稳定返回 `v2_image_rollback_retired`。
- 默认 `watermark-core` release suite 为 `108 passed / 0 failed`，正式 V3 图片服务测试保持 `5/5`。
- `npm run watermark:legacy-rollback-suite` 独立验证图片拒绝合同和音频 legacy rollback。
- RC 生成器复跑后，`RC-MEDIA-001`、`RC-MEDIA-002` 均进入 `resolvedBlockers`；活动阻断项剩四项，整体仍为 `BLOCKED`。

## 12. 更新后的推荐下一步

执行 `RC-MEDIA-003`：使用最终安装候选完成 WAV / MP3 / FLAC / OGG / M4A × mono/stereo 基线，并把证据写入 `artifacts/desktop-media-internal-rc/20260722/`。

## 13. RC-MEDIA-003 正式关闭

- 最终安装候选完成 WAV / MP3 / FLAC / OGG / M4A × mono / stereo 共 `10/10`。
- 每个单元均通过正式安装版写入、写后回读、独立共享核心读取、安装版只读验证和 V3 UID 精确一致。
- 十个 fixture 均保持 48 kHz 与原 mono / stereo 声道；WAV 16-bit 与 FLAC 24-bit 还保持有效位深。
- 整轮耗时约 `53.3 秒`；单单元约 `4.0–9.8 秒`。
- RC 生成器复跑后，`RC-MEDIA-001`、`RC-MEDIA-002`、`RC-MEDIA-003` 均进入 `resolvedBlockers`。
- 证据：`artifacts/desktop-audio-format-channel-gate/20260722-final/summary.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。

## 14. 更新后的推荐下一步

执行 `RC-MEDIA-004`：使用最终安装候选验证 `20:00 + 48 kHz + stereo + 高位深`，并记录峰值内存、取消行为、输出规格、写后回读与只读验证。

## 15. RC-MEDIA-004 正式关闭

- 最终 installed exe SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40` 完成 `20:00 / 48 kHz / stereo / 24-bit FLAC`。
- 输出为 `24-bit PCM WAV`，时长、采样率、声道和有效位深保持；写后回读、独立 V3 核心读取和安装版只读验证均命中同一 UID。
- 完成场景约 `57.5 秒`；主进程峰值约 `1.215 GiB`，进程树工作集求和峰值约 `2.151 GiB`。
- 取消约 `14 ms`确认，不创建版权记录；约 `45.8 秒`后达到连续 CPU 静默。取消并非底层瞬时抢占。
- RC 生成器复跑后，`RC-MEDIA-001` 至 `RC-MEDIA-004` 均进入 `resolvedBlockers`。
- 证据：`artifacts/desktop-audio-upper-envelope-gate/20260722-final/summary.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。

## 16. 更新后的推荐下一步

执行 `RC-RELEASE-001`：对当前 NSIS、MSI、release executable 和 installed executable 完成 Authenticode 签名并通过 candidate Gate，禁止重建候选。

## 17. RC-RELEASE-001 正式关闭

- 当前 NSIS、MSI、release EXE 和 installed EXE 已使用同一 `HiddenShield Release Signing` 自签证书原地签名，未重新构建。
- 四文件 `Get-AuthenticodeSignature` 和 SignTool `/pa /all /v` 均通过并带时间戳。
- candidate Gate 验证四个签后 SHA-256 与签名证据一致；四个篡改副本均返回 `HashMismatch`。
- 签名证据：`artifacts/authenticode-signing/20260722-rc-release-001/self-signed-authenticode-evidence.json`。
- Gate 证据：`artifacts/authenticode-gate/20260722-rc-release-001/authenticode-gate.json`。
- RC 生成器复跑后，`RC-MEDIA-001` 至 `RC-MEDIA-004`、`RC-RELEASE-001` 均进入 `resolvedBlockers`。
- 自签证书不代表公共 Windows 信任；整体 RC 仍为 `BLOCKED`，仅剩 `RC-RELEASE-002`。

## 18. 更新后的推荐下一步

执行 `RC-RELEASE-002`：在干净离线 Windows 快照分别安装签名 NSIS 和 MSI，验证新安装 EXE 的 Authenticode 状态、无预装 WebView2 启动、图片验证和音频验证，禁止重新构建。

## 19. RC-RELEASE-002 挂起

- 用户决定暂时挂起干净离线 Windows 快照验证；该决定只改变执行顺序，不构成通过、豁免或风险接受。
- `RC-RELEASE-002` 状态记为 `suspended_by_user`，继续作为唯一活动发布阻断项。
- 当前 `0.1.0` 桌面媒体内部 RC 和对外发布仍不批准；禁止把本机、已安装目录或预装 WebView2 环境的证据替代干净离线 Windows 证明。

## 20. 签名后同一候选媒体冒烟

- 已签名 installed EXE SHA-256：`17ea6c9dc0595bccf75ac4248a7b01f0abe9794adde574b0f3c5eb41e3b32a24`，Authenticode 状态保持 `Valid`。
- 同一已签名 EXE 完成 PNG / JPEG / WebP 常规 1920×1080 图片写入、写后回读、独立核心读取和安装版只读验证，结果 `3/3`。
- 同一已签名 EXE 完成 WAV / MP3 / FLAC / OGG / M4A × mono / stereo，结果 `10/10`；采样率、声道、V3 UID 和正式输出规格检查全部通过。
- `RC-RELEASE-001` 的关闭条件现同时要求四文件签名 Gate、篡改失效和签名后同一 installed EXE 媒体冒烟，不再只依赖签名元数据。
- 证据：`artifacts/desktop-image-resource-gate/20260722-post-sign-smoke/summary.json`、`artifacts/desktop-audio-format-channel-gate/20260722-post-sign-smoke/summary.json`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。

## 21. 更新后的推荐下一步

保持 `RC-RELEASE-002` 挂起且阻断，执行桌面 `0.1.0` RC 证据索引完整性审计：逐项核对候选哈希、证据引用、发布阻断状态和产品边界文案，不重新构建，也不声明离线安装通过。

## 22. 本机标准用户 NSIS 安装失败

- 使用现有签名 NSIS 原文件安装到 `%LOCALAPPDATA%\HiddenShield` 成功；开始菜单、桌面快捷方式和 Windows 卸载项均指向新安装目录。
- 外层 NSIS SHA-256 `b705cd1249947057cab65e0cdb268dbbd50a2cd5fd2a0717e20f3e8ca9ad474b` 为 Authenticode `Valid`，但新安装 `hidden_shield.exe` SHA-256 `37d88d648dec4a90d9afb4579331bbef06c14d3c47f65f5dea6d61545fb58c40` 为 `NotSigned`；新安装 `uninstall.exe` 也为 `NotSigned`。
- 结论：当前签名只覆盖外层 NSIS，不覆盖其内嵌应用负载；当前 `0.1.0` 安装候选应拒绝发布。`RC-RELEASE-002` 状态升级为 `failed_local_install_vm_suspended`：本机已确认签名失败，干净离线 VM 部分仍按用户决定挂起。
- 连带结论：首次安装沿用了旧测试安装定位并覆盖原先单独签名的测试 installed EXE；统一 RC 已重新打开 `RC-RELEASE-001`，不再将那份已被覆盖的签名证据视为当前候选状态。
- 不重建当前候选；后续新候选必须先签 inner release EXE，再打包并签 NSIS / MSI，随后重新验证新安装 EXE。
- 证据：`artifacts/nsis-local-standard-install/20260723/summary.json`。

## 23. 更新后的推荐下一步

冻结并拒绝当前安装候选；先制定下一候选的“inner EXE 预签名 → 打包 NSIS / MSI → 外层签名 → 新安装 EXE 验签”签名顺序修复方案，不执行重建，等待明确批准后再实施。

## 24. 下一候选签名顺序设计

- 已完成设计，未执行构建或签名。采用 `build --no-bundle → inner EXE 预签名 → bundle --no-sign → 外层 NSIS / MSI 签名 → 两类新安装 EXE 验签` 的不可变顺序。
- `RC-RELEASE-001` 负责源候选签名拓扑；`RC-RELEASE-002A/B` 分别验证 NSIS / MSI 的实际新安装 EXE；干净离线 Windows 验证保留为 `RC-RELEASE-002C`。
- 禁止对安装目录 EXE 单独补签后作为通过证据；新安装 EXE 必须为 `Valid`，且 SHA-256 与 pre-bundle inner EXE 完全一致。
- 详细设计：`docs/Windows候选签名顺序修复设计.md`。

## 25. 更新后的推荐下一步

等待批准后在独立 worktree 实现 `WindowsSignedReleaseCandidate` 编排模块和安装后验签 Gate；批准前不得重建、打包、签名或替换当前失败候选。

## 26. Windows 安装后签名 Gate 已实施

- 已实施 Tauri 原生 Windows 打包期签名、不可变候选 manifest、NSIS / MSI 新安装 EXE 验签与篡改检测编排；候选 Gate 不再将打包完成后被 Tauri 清理的中间 EXE 当作发布载荷。
- 候选 `20260724-installed-payload-gate-075530`：NSIS 外层及新安装 `hidden_shield.exe` 均为 Authenticode `Valid`，签名主体为 `CN=HiddenShield Release Signing`，新安装 EXE SHA-256 为 `c4f544d3b52db481ecf12f4a6d9f3f31edcf744cf32d4c6005a911553c89686c`。
- MSI 外层为 `Valid`，但本机标准会话安装返回 `1603 / Error 1925`；Gate 输出 `installed-payloads/msi-install.log` 并失败，未将 NSIS 结果升级为 MSI 通过。
- 当前结论：`RC-RELEASE-001` 与 `RC-RELEASE-002` 继续阻断，桌面媒体内部 RC 与外部发布均不批准。
- 证据：`artifacts/windows-signed-release-candidate/20260724-installed-payload-gate-075530/candidate-manifest.json`、`artifacts/windows-signed-release-candidate/20260724-installed-payload-gate-075530/installed-payloads/msi-install.log`。

## 27. 更新后的推荐下一步

在提升权限的干净 Windows 快照复跑 `release:windows-signed-candidate`，完成 MSI 新安装 EXE 验签、无预装 WebView2 启动和签名后图片 / 音频冒烟；未完成前不得合并为可发布主干。
