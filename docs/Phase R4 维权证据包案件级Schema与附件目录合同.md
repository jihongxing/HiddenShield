# Phase R4 维权证据包案件级 Schema 与附件目录合同

状态：`R4 foundation / 内部设计合同`

日期：`2026-07-14`

## 1. 目标

Phase R4 不再把单条版权记录 PDF 直接包装成“侵权事实证明”，而是在既有 `FormalReportDocument schema v2` 之上建立案件级 `RightsEvidencePackDocument schema v1`。

首版只解决五件事：

1. 冻结案件、版权事实、争议对象、侵权样本和附件目录的结构。
2. 为每项外部材料记录来源、采集时间、摘要、文件大小和附件编号。
3. 将系统自动观察、用户人工陈述和法律判断明确分层。
4. 保存引用正式版权报告及其 Manifest root digest 的谱系。
5. 为后续 PDF 页面、附件打包、律师评审和签字页提供稳定输入。

## 2. 能力边界

当前分类：`只能内部测试`

本证据包是技术辅助材料，不构成法律意见。

该 schema 可以表达：

- 用户提交了哪些版权记录和争议材料。
- 系统在什么时间处理了哪些文件。
- 每个附件的 SHA-256、字节数、来源和目录编号。
- 自动比对方法、输入、观察结果和限制。
- 用户或代理人的独立陈述。

该 schema 不能表达或暗示：

- HiddenShield 已认定侵权成立。
- 用户已取得绝对权属。
- 外部网页内容已由公证、司法鉴定或可信时间服务确认。
- 自动相似性观察等于法律上的实质性相似。
- 报告已被法院、公证机构、仲裁机构或鉴定机构采纳。

## 3. 顶层合同

`RightsEvidencePackDocument schema v1` 顶层字段：

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schemaVersion` | 是 | 固定为 `1` |
| `documentType` | 是 | 固定为 `rights_evidence_pack` |
| `packId` | 是 | 案件证据包唯一编号 |
| `status` | 是 | 首版只允许 `draft`、`review_ready` |
| `generatedAt` | 是 | 系统生成时间，不等于可信时间 |
| `case` | 是 | 案件标题、用途、当事人声明和管辖备注 |
| `copyrightFacts` | 是 | 引用正式版权报告与来源 Manifest root digest |
| `disputedObjects` | 是 | 被主张存在争议的对象 |
| `infringementSamples` | 是 | 外部样本及其来源、采集和附件引用 |
| `collectionEvents` | 是 | 采集、导入、摘要计算等过程记录 |
| `attachments` | 是 | 附件目录和文件完整性元数据 |
| `automatedFindings` | 是 | 系统自动观察，不得包含法律结论 |
| `humanStatements` | 是 | 用户、代理人或律师的独立陈述 |
| `limitations` | 是 | 固定能力边界和缺失项 |

## 4. 版权事实引用

每个 `copyrightFacts[]` 必须引用一个已经生成的正式报告：

- `reportId`
- `reportRootDigest`
- `recordId`
- `watermarkUid`
- `mediaKind`
- `factSnapshot`

`factSnapshot` 只能复制正式 `FormalReportDocument` 已有事实，不允许 R4 重新生成版权编号、重新推导水印结论或覆盖写入后验证状态。

## 5. 侵权样本与附件

每个 `infringementSamples[]` 必须具备：

- `sampleId`
- `disputedObjectId`
- `source`
- `capturedAt`
- `captureTimeStatus`
- `sha256`
- `bytes`
- `attachmentId`
- `collectorStatement`

每个 `attachmentId` 必须能在 `attachments[]` 中找到唯一对应项。附件目录必须保存：

- 原始文件名
- MIME 类型
- 角色
- SHA-256
- 文件大小
- 来源
- 获取方式
- 是否包含原始字节

首版不把普通本机时间写成可信时间。没有 TSA、公证或第三方采集证明时，`captureTimeStatus` 必须为 `device_claimed` 或 `unverified`。

## 6. 自动结论与人工陈述分离

`automatedFindings[]` 只允许技术观察，例如：

- 文件摘要是否相同。
- 已登记版权编号是否出现在输入元数据中。
- 感知哈希距离。
- 用户提供文本的结构化差异。

自动观察必须包含：

- `method`
- `inputAttachmentIds`
- `observation`
- `status`
- `limitations`

`humanStatements[]` 单独记录陈述人、角色、陈述文本和签署状态。系统不得把人工陈述复制到 `automatedFindings`，也不得把自动观察改写为“侵权成立”。

## 7. 首个 fixture

确定性 fixture：

`docs/contracts/rights-evidence-pack-v1.fixture.json`

该 fixture 使用 `.invalid` 域名、合成摘要和 `unverified` 时间状态，只用于 schema、PDF 和附件目录开发，不是现实案件材料。

合同门禁：

```text
npm run report:r4-contract
```

门禁检查：

- 案件级顶层字段完整。
- 正式报告 root digest 引用存在。
- 每个侵权样本均具备来源、时间、SHA-256、字节数和附件编号。
- 每个附件引用可解析。
- 自动观察与人工陈述分离。
- 不出现“侵权成立”“司法认可”“公证完成”等越界结论。

## 8. R4 后续顺序

1. [x] 基于该 schema 设计案件封面、证据目录、版权事实、争议对象、采集记录、自动观察、人工陈述、限制说明和附件索引页面。
2. [x] 定义案件包目录结构和附件文件名规范。
3. [x] 增加采集事件追加式日志与 Manifest 链。
4. [ ] 增加人工陈述确认和签字页，但不冒充数字签名。
5. [ ] 使用一个合成案件完成律师场景评审。

## 9. 八页高保真原型结果

产物：

- HTML：`docs/prototypes/rights-evidence-pack-r4/finalized.html`
- PDF：`docs/prototypes/rights-evidence-pack-r4/finalized.pdf`
- 指标：`docs/prototypes/rights-evidence-pack-r4/finalized.json`

页面：

1. 案件封面
2. 证据目录
3. 版权事实与来源谱系
4. 争议对象与侵权样本
5. 采集过程与操作记录
6. 自动技术观察
7. 人工陈述与确认
8. 限制说明与附件索引

验证结果：

- 使用 schema v1 fixture 动态渲染。
- 使用项目受控 Noto Sans SC / Noto Serif SC 字体。
- Chromium 输出固定为 8 页。
- 8 页均无纵向溢出。
- PDF 大小为 244274 bytes。
- PDF SHA-256 为 `8c7c0cb02894504b2234f6f07b9a66545915a71aad59b7d684bf1a0a694e59ca`。
- 原型固定显示 `not_signed`、`not_timestamped` 和 `not_evaluated`。

## 10. 当前推荐下一步

案件包目录和双追加式摘要链已完成：

- 顶层固定为 `case.json + case-manifest.json + attachments/`。
- 附件角色固定为 `original`、`working_copy`、`capture`、`external_receipt`。
- 采集事件和附件分别使用 `sha256_append_chain_v1`。
- fixture 覆盖路径逃逸、符号链接、未登记文件、附件篡改和事件篡改边界。

Tauri MockRuntime IPC QA 和桌面验证页六状态入口已完成。

立即实现下一项：在 Flutter 复用相同的稳定 JSON、事件链和附件链算法，完成 Android 对桌面案件包 fixture 的只读校验。

## 11. Flutter / Android 实施结果

- Flutter `RightsEvidencePackVerifier` 已完成。
- 桌面 fixture 与移动测试 fixture 由同步脚本保持字节一致，并由 R4 合同阻断漂移。
- 主机测试覆盖正常包与三类篡改边界。
- Android API 36 运行态六状态和包根摘要与桌面一致。
- 当前实现仍是只读技术完整性检查，不读取附件水印、不判断权属、侵权、签名可信或采集时间可信。

立即实施下一项：在移动验证页加入案件包目录选择和六状态只读展示，并完成 Android 外部存储案件包运行态 QA。

## 12. 移动验证页与非 AssetBundle QA

- 移动验证页已加入案件包目录选择、六状态卡片、root digest 对照和附件匹配数。
- Android 应用专属外部目录由应用进程创建，桌面 fixture 通过 `adb push` 写入六个物理文件。
- 页面触发的验证与直接验证器结果一致，未使用 AssetBundle 附件字节。
- 当前仍保持 `只能内部测试`；任意共享目录需要继续补 SAF tree URI 与持久授权。

立即实施下一项：为 Android 增加 SAF tree URI 案件包读取器，并完成 Download 目录系统选择器运行态 QA。

## 13. Android SAF Download 运行态结果

- Android 原生层已实现 tree URI 选择、持久只读授权、目录遍历和相对路径文件读取。
- Flutter 验证页可复用已授权目录，并保留重新选择入口。
- R4 fixture 已推入系统 Download，由 DocumentsUI 完成真实点击授权。
- 首次校验和应用强停重启后的第二次校验均返回同一六状态、四个匹配附件和同一 root digest。
- 当前分类继续为 `只能内部测试`；尚未覆盖第三方 DocumentsProvider、授权撤销、目录移动和 Provider 离线。

立即实施下一项：补充 SAF 失败矩阵与统一错误码，覆盖授权撤销、目录移动、附件删除和至少一个第三方 DocumentsProvider。

## 14. SAF 失败矩阵实施结果

- 四个文件访问错误码和中文提示已冻结，并由 Flutter 单测固定。
- Android 原生层在校验 tree URI 前区分 Provider 可用性与持久授权状态。
- SAF 附件读取异常会穿透附件完整性循环，避免被静默降级成普通 `missing` 状态。
- Download 目录完成附件删除、目录移动、授权撤销和恢复 QA。
- 独立只读 DocumentsProvider APK 使用同一 R4 fixture；基线校验与 Provider 禁用分类均通过。
- 当前仍为 `只能内部测试`，独立 QA Provider 不等同于真实商业云盘兼容。

立即实施下一项：在一个真实 Android 云盘 Provider 和 iOS File Provider 上完成同错误码矩阵与恢复动作 QA。
