# Phase R4 案件包物理目录与追加式摘要链合同

状态：`R4 bundle contract / 内部测试`

日期：`2026-07-14`

## 1. 目录合同

案件包顶层只允许三个入口：

```text
case-fixture-r4-0001/
├── case.json
├── case-manifest.json
└── attachments/
    ├── original/
    ├── working-copy/
    ├── capture/
    └── external-receipt/
```

规则：

- `case.json` 是案件级事实、引用和陈述文档。
- `case-manifest.json` 是目录、文件摘要、事件链、附件链和包级 root digest。
- `attachments/` 是唯一附件根目录。
- 相对路径统一使用 `/`，禁止 `\`、绝对路径和 `..` 逃逸。
- 禁止符号链接、目录联接和指向包外的真实路径。
- 顶层和 `attachments/` 中的所有文件必须被合同登记；未登记文件导致校验失败。
- 已登记附件不可原位覆盖。修订必须追加新附件、增加新 `sequence`，并通过关系字段引用旧附件。

## 2. 附件角色

角色描述材料在案件中的来源关系，不描述其真实性或法律效力。

### `original`

- 含义：提交人主张为源作品或源证据的材料。
- 不代表：作者身份、权属、首次形成时间或真实性已被证明。
- `derivedFromAttachmentId`：必须为 `null`。

### `working_copy`

- 含义：为检查、转换、脱敏、标注或分析生成的派生副本。
- 不得替换原件。
- `derivedFromAttachmentId`：必填，并指向已登记附件。
- v1 fixture 要求其来源角色为 `original`。

### `capture`

- 含义：记录外部争议对象的截图、下载件、拍摄件或录屏。
- 不代表：采集过程已获得可信时间、公证或第三方见证。
- `derivedFromAttachmentId`：必须为 `null`。

### `external_receipt`

- 含义：外部平台、服务商或第三方提供的回执、响应或确认材料。
- 不代表：签发主体、签名、时间和法律效力已验证。
- `derivedFromAttachmentId`：必须为 `null`。

首版冻结枚举：

```text
original
working_copy
capture
external_receipt
```

## 3. 采集事件追加链

算法：`sha256_append_chain_v1`

Genesis：

```text
HiddenShield-Rights-Evidence-Pack-Event-Chain-v1
```

事件对象先执行递归 key 排序的稳定 JSON 序列化，再计算：

```text
eventDigest = SHA256(stableJson(event))
```

每个链节点：

```text
chainDigest = SHA256(
  sequence + "\n" +
  eventId + "\n" +
  eventDigest + "\n" +
  previousChainDigest
)
```

约束：

- `sequence` 从 1 连续递增。
- 新事件只能追加到末尾。
- 已有事件的顺序、内容和 chain digest 不允许改写。
- 普通设备时间必须标记为 `device_claimed` 或 `unverified`。
- 事件链匹配不等于事件时间、操作主体或陈述内容真实可信。

## 4. 附件追加链

算法：`sha256_append_chain_v1`

Genesis：

```text
HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1
```

每个链节点：

```text
chainDigest = SHA256(
  sequence + "\n" +
  attachmentId + "\n" +
  relativePath + "\n" +
  role + "\n" +
  fileBytes + "\n" +
  fileSha256 + "\n" +
  previousChainDigest
)
```

约束：

- 附件 ID、路径、角色、字节数和文件摘要全部进入链。
- 修改附件字节、重命名、改变角色或重新排序都会改变 attachment root digest。
- 新附件只能以新序号追加。
- 删除中间附件或重新编号会导致链校验失败。

## 5. 包级 Root Digest

`case.json`、事件链和附件链通过下式汇总：

```text
rootDigest = SHA256(
  "HiddenShield-Rights-Evidence-Pack-Root-v1" + "\n" +
  caseJsonSha256 + "\n" +
  eventRootDigest + "\n" +
  attachmentRootDigest
)
```

当前 fixture：

- 事件 root digest：`94caa1faf5afa39626fe8dd5021c879c66d5772c3ad8649fe3933696f0b171b7`
- 附件 root digest：`3f979e4c8fb605173402a39ee8de985f9db1f097d54a407dd48aac60a2699a90`
- 包级 root digest：`4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33`

## 6. 安全与能力边界

当前 `case-manifest.json`：

- `signature.status = not_signed`
- `trustedTime.status = not_timestamped`

因此追加式摘要链只能：

- 检测相对于已知 root digest 的文件、事件和目录变化。
- 保留追加顺序和派生关系。
- 为未来签名、可信时间和审计提供稳定输入。

不能：

- 阻止能够替换全部文件的攻击者重新计算整套未签名摘要。
- 证明附件来源真实。
- 证明采集时间可信。
- 证明回执签发主体或签名有效。
- 形成侵权、权属或司法采纳结论。

## 7. Fixture 与命令

物理 fixture：

`docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001`

生成：

```text
npm run report:r4-bundle
```

校验：

```text
npm run report:r4-bundle-contract
```

校验覆盖：

- 顶层目录白名单。
- 未登记附件阻断。
- 路径逃逸与符号链接阻断。
- 四类附件角色及派生关系。
- `case.json`、附件字节和 SHA-256。
- 采集事件链和附件链完整复算。
- 追加事件不改变既有链前缀。
- 附件篡改和事件篡改检测。

## 8. 当前推荐下一步

Tauri 只读 `verify_rights_evidence_pack` 已实现：

- 输入：案件包目录。
- 输出：目录合同、附件完整性、事件链、附件链、数字签名和可信时间六类独立状态。
- 附件逐项返回 expected / actual 字节数、SHA-256 和状态。
- 同时返回 Manifest 声明 root digest 与本机复算 root digest。
- 校验过程不写入、不修复、不重新生成 Manifest。

验证：

```text
npm run report:r4-tauri-contract
```

当前推荐下一步：

- Tauri MockRuntime QA 已通过已注册 IPC 命令返回 camelCase JSON，并断言六类状态、四类附件和声明 / 复算 root digest。
- 桌面验证页已新增案件包目录选择、六状态卡片、root digest 对照和附件逐项结果。
- UI 明确声明只复算目录、文件和摘要链，不读取媒体水印，不判断侵权、签发主体或时间可信。

当前推荐下一步：

- 在 Flutter 实现同一 `case-manifest.json` 只读校验器，并使用该案件包 fixture 完成 Android 运行态跨端复算。

## 9. Flutter / Android 跨端复算

已实现：

- `RightsEvidencePackVerifier` 使用与桌面相同的稳定 JSON 规范化。
- 事件链和附件链均使用 `sha256_append_chain_v1`。
- 包根摘要使用 `sha256_case_event_attachment_roots_v1`。
- 返回字段固定为目录合同、附件完整性、事件链、附件链、数字签名和可信时间六类状态。
- 移动 fixture 与桌面 fixture 按文件清单和原始字节保持一致。

验证：

```text
npm run mobile:report-r4-contract
npm run mobile:report-r4-android
npm run report:r4-contract
```

Android API 36 结果：

- `directoryContractStatus = matched`
- `attachmentIntegrityStatus = matched`
- `eventChainStatus = matched`
- `attachmentChainStatus = matched`
- `signatureStatus = not_signed`
- `trustedTimeStatus = not_timestamped`
- root digest：`4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33`

当前推荐下一步：

- 将验证器接入 Flutter 验证页目录选择，并使用 Android 外部文件目录而非测试 assets 完成真实文件访问 QA。

## 10. 移动验证页与物理外部目录

已完成：

- Flutter 验证页使用系统目录选择入口调用案件包验证器。
- 六状态、root digest、附件匹配数和限制说明均为只读展示。
- Android integration test 由应用获取专属外部目录并通知主机。
- 主机通过 `adb push` 写入 `case.json`、`case-manifest.json` 和四类附件，共六个物理文件。
- 验证器和页面随后从真实目录读取，未使用 AssetBundle 字节。

验证：

```text
npm run mobile:report-r4-external-android
```

结果：

- 六个物理文件推入成功。
- 四类完整性状态均为 `matched`。
- `signatureStatus = not_signed`。
- `trustedTimeStatus = not_timestamped`。
- 声明 / 复算 root digest 完全一致。

当前推荐下一步：

- 增加 Android SAF tree URI 读取适配器，并从 Download 目录通过系统文件选择器完成持久授权 QA。

## 11. Android SAF tree URI 合同

已完成：

- 原生通道固定为 `com.hiddenshield.hidden_shield_mobile/rights_evidence_saf`。
- `pickTree` 使用 `ACTION_OPEN_DOCUMENT_TREE`，申请只读、前缀和可持久化授权。
- `getPersistedTree` 仅在 URI 仍存在于 `persistedUriPermissions` 时返回授权目录。
- `readFile` 只接受安全的 `/` 分隔相对路径，并通过 `DocumentFile.fromTreeUri` 逐级读取。
- `listDirectory` 返回与桌面 / Dart 文件读取器一致的顶层目录、附件路径和三项安全状态。
- Flutter 继续使用同一 `RightsEvidencePackVerifier`，SAF 不复制摘要链算法。

运行态 QA：

```text
npm run mobile:report-r4-saf-click-android
```

API 36 结果：

- fixture 位于 `/sdcard/Download/HiddenShield-R4-QA/case-fixture-r4-0001`。
- 系统 DocumentsUI 完成目录点击和授权确认。
- 首次校验与强停重启后的授权复验均通过。
- 四项完整性状态均为 `matched`。
- `signatureStatus = not_signed`。
- `trustedTimeStatus = not_timestamped`。
- 声明 / 复算 root digest 完全一致。

当前推荐下一步：

- 增加授权撤销、目录移动、附件删除和第三方 DocumentsProvider 的失败矩阵，并冻结用户可见错误码。

## 12. SAF 失败矩阵与统一错误码

冻结合同：

| 场景 | 错误码 | 用户提示 |
| --- | --- | --- |
| 授权被撤销 | `evidence_pack_authorization_revoked` | 目录授权已失效，请重新选择案件包目录。 |
| 目录移动或删除 | `evidence_pack_directory_missing` | 案件包目录已移动或删除，请重新选择。 |
| 登记附件缺失 | `evidence_pack_attachment_missing` | 案件包附件缺失，请恢复原目录内容后重试。 |
| Provider 不可用 | `evidence_pack_provider_unavailable` | 文件提供方当前不可用，请恢复对应应用或改选本地目录。 |

Android 运行态矩阵：

- Download fixture 删除 `ATT-03` 后返回附件缺失；恢复文件后重新匹配。
- 移动整个案件包目录后返回目录缺失；恢复目录后重新匹配。
- 释放持久 URI 权限后返回授权失效。
- 独立 APK `com.hiddenshield.qa.documentsprovider` 暴露同一 fixture，基线完整匹配。
- 禁用该 Provider 后返回 Provider 不可用；未改变签名、可信时间和法律边界。

运行门禁：

```text
npm run mobile:report-r4-saf-failure-matrix
```

当前推荐下一步：

- 在真实 Android 云盘 Provider 与 iOS File Provider 上复用四错误码，并记录 OEM / Provider 差异。
