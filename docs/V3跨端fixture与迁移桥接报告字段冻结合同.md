# HiddenShield V3 跨端 fixture 与迁移桥接报告字段冻结合同

更新时间：2026-06-29

本文档冻结 V3 media payload 正式接入前必须满足的跨端 fixture、迁移桥接、同步 payload、报告字段、V2 / V3 显示差异和回滚门禁。它是 `docs/公开权利信号与训练许可扫描协议设计.md` 的执行合同，也是后续改 `watermark-core`、桌面端、移动端、后端和 QA 脚本前的准入依据。

## 1. 冻结结论

- V3 的媒体内最小锚点只允许包含 `watermark_id`、`payloadProtocolVersion` 和 `auth_tag`。
- 完整权利声明、训练许可、声明版本、撤销、替代、自定义条款、registry proof、rights manifest hash、公开元数据映射均迁入版权库 / 云版权库 / registry / 公开元数据层。
- V2 只作为旧记录读取、迁移桥接、重签参考和 QA 对照，不作为新写入协议继续扩展。
- 当前 `PAYLOAD_BYTES = 119` 不得被直接修改；V3 写入路径必须另起协议配置、fixture 和 release gate。
- 公开权利信号永远不是法律授权结论，所有 V2 / V3 报告和公开扫描结果必须保持 `legalConclusion=false`。
- Android 或 Web QA 不能替代 iOS QA；iOS 运行态 QA 在缺少 macOS + Xcode + iOS Simulator 或真机环境时只能记录为挂起。

## 2. V3 媒体内字段冻结

| 字段 | V3 媒体内归类 | 最终落点 | 同步 / 报告口径 | 说明 |
| --- | --- | --- | --- | --- |
| `watermark_id` / `watermarkUid` | 必留 | 媒体内 + 本地版权库 + 云版权库 + registry | 同步 `watermark_uid`；报告展示版权编号 | 唯一检索锚点，连接 registry 和公开元数据。 |
| `payloadProtocolVersion` | 必留 | 媒体内 + 本地版权库 + 云版权库 | 同步 `payload_protocol_version`；报告展示 `V3` | 扫描器据此进入 V3 最小锚点解析路径。 |
| `auth_tag` | 必留 | 媒体内；验证结果入本地 / 云端记录 | 报告只展示认证状态，不展示 tag 原值 | 只证明锚点未被篡改，不证明法律授权。 |
| `payloadBytesLength` | 派生字段 | 本地版权库 + 云版权库 | 同步 `payload_bytes_length`；报告展示字节长度 | 由读取结果记录。V3 当前准备层为 `39` bytes，最终正式长度必须由 codec 合同冻结后进入 fixture。 |
| `payloadAuthStatus` | 派生字段 | 本地版权库 + 云版权库 | 同步 `payload_auth_status`；报告展示认证状态 | `verified` 只表示 payload 验证通过，不表示训练许可可用。 |
| `mediaPayloadRole` / `media_payload_role` | 派生字段 | 同步 payload + 报告，不入媒体 payload，不入 DB | V2 为 `v2_full_record`，V3 为 `v3_minimal_anchor` | 只用于迁移期显示差异，不能作为授权语义来源。 |
| `revision` | 必迁出 | 本地版权库 + 云版权库 + registry | 同步 `revision`；报告展示版本次数 | V3 媒体内不再保存，版本链由 registry / 本地记录承接。 |
| `parentWatermarkUid` | 必迁出 | 本地版权库 + 云版权库 + registry | 同步 `parent_watermark_uid`；报告展示上一版编号 | 旧 V2 读取结果可桥接；新 V3 从 registry 查询。 |
| `watermarkIdIssueMode` | 必迁出 | 本地版权库 + 云版权库 + registry | 同步 `watermark_id_issue_mode`；报告展示签发模式 | 媒体内不再保存签发语义。 |
| `mediaType` | 可迁 | 本地版权库 + 云版权库；必要时由容器和 UI 上下文派生 | 同步 `kind`；报告展示记录类型 | 不作为 V3 最小锚点必需字段。 |
| `originalHashPrefix` | 必迁出 | 本地版权库 + 云版权库 + registry | 同步 `sha256`；报告展示作品指纹 | 媒体内不再保存作品 hash 前缀，避免 payload 膨胀。 |
| `registryProofHash` | 必迁出 | registry + rights manifest + 公开元数据 | 报告展示 registry / manifest 状态 | 不写入 V3 媒体 payload，避免把 registry 快照误当长期协议字段。 |
| `rightsManifestHash` | 必迁出 | registry + 公开元数据副本 | 公开权利卡展示 manifest hash | 只属于事实源 / 传播层，不属于盲水印 payload。 |
| 作品来源 / 创作方式 / 人工编辑 / 真实性 / 训练许可声明 | 必迁出 | 本地版权库 + 云版权库 + registry + 公开元数据 | 同步声明字段；报告展示创作者声明 | 不再进入 V3 媒体 payload。 |
| 自定义版权声明 / 条款 URL / 撤销记录 | 必迁出 | registry + 公开元数据 | 报告展示声明摘要和 registry 状态 | V3 媒体内只保留可验证锚点。 |

## 3. 跨端 fixture 矩阵

V3 正式接入前必须新增以下 fixture。任何一项未通过，都不得把 V3 切为默认写入。

| Fixture ID | 媒体 | 写入端 | 读取端 | 输入 | 期望读取字段 | 期望 registry 字段 | 发布门禁 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `v3_image_desktop_write_mobile_read` | 图片 PNG / JPEG | 桌面端 | Android 原生端 + iOS 原生端 | 桌面端 V3 保护副本 | `watermarkUid`、`payloadProtocolVersion=3`、V3 payload length、`payloadAuthStatus=verified` | `rightsManifest.status=active`、训练许可来自 registry | release blocking |
| `v3_image_mobile_write_desktop_read` | 图片 PNG / JPEG | Android 原生端 + iOS 原生端 | 桌面端 | 移动端 V3 保护副本 | 同上 | 同上 | release blocking |
| `v3_audio_desktop_write_mobile_read` | 音频 WAV + 当前正式容器集合 | 桌面端 | Android 原生端 + iOS 原生端 | 桌面端 V3 保护副本 | 同上 | 同上 | release blocking |
| `v3_audio_mobile_write_desktop_read` | 音频 WAV + 当前正式容器集合 | Android 原生端 + iOS 原生端 | 桌面端 | 移动端 V3 保护副本 | 同上 | 同上 | release blocking |
| `v2_legacy_read_bridge_image` | 图片 PNG / JPEG | 历史 V2 样本 | 桌面端 + Android + iOS | V2/119 保护副本 | `watermarkUid`、`payloadProtocolVersion=2`、`payloadBytesLength=119`、`payloadAuthStatus`、可桥接版本链 | registry 可覆盖旧快照 | release blocking |
| `v2_legacy_read_bridge_audio` | 音频 WAV + 当前正式容器集合 | 历史 V2 样本 | 桌面端 + Android + iOS | V2/119 保护副本 | 同上 | registry 可覆盖旧快照 | release blocking |
| `registry_overrides_v3_payload` | 图片 + 音频 | 任一端 | 任一端 | 同一 UID 的 V3 样本 | 媒体内不含训练许可 bit | 训练许可只从 active rights manifest 读取 | release blocking |
| `registry_conflict_marks_conflict` | 图片 + 音频 | 任一端 | 任一端 | 公开元数据与 registry 冲突样本 | 锚点读取成功 | `scanStatus=conflict` 或等价冲突状态，不输出法律结论 | release blocking |
| `v3_feature_gate_rollback` | 图片 + 音频 | 桌面端 + 移动端 | 桌面端 + 移动端 | V3 默认 + 显式 rollback 矩阵 | `off` 默认正式路径写读 V3/39，`internal_qa` 受控写 V3/39；图片 `force_v2_rollback` 必须拒绝，音频 legacy rollback 仅在隔离套件写读 V2/119 | registry 查询不受写入版本影响 | release blocking |

补充要求：

- iOS fixture 必须在 macOS + Xcode + iOS Simulator 或真机环境补跑，不能由 Android、Web 或 Flutter desktop 代替。
- 视频视觉 L3 仍属于 staged 能力，V3 视频视觉 bitstream fixture 只能作为内部算法门禁，不得写成正式 L3 用户能力。
- fixture 产物必须固定到仓库或受控生成脚本，不能依赖开发者本机临时文件。

## 4. 同步 payload 字段冻结

V3 不新增“完整授权语义”同步字段；它复用当前版权记录字段链路，并明确字段来源。

| 同步字段 | V2 迁移桥来源 | V3 来源 | 接收端处理 | 报告口径 |
| --- | --- | --- | --- | --- |
| `watermark_uid` | V2 payload 读取 | V3 最小锚点读取 | 必须落库 | 版权编号 |
| `payload_protocol_version` | V2 payload 读取，固定 2 | V3 payload 读取，固定 3 | 必须落库 | Payload 协议 |
| `payload_bytes_length` | V2 payload 读取，固定 119 | V3 codec 读取的正式长度 | 必须落库 | Payload 协议 |
| `media_payload_role` | 派生 `v2_full_record` | 派生 `v3_minimal_anchor` | 不落 DB；同步 payload 和报告生成器按协议版本派生 / 透传 | 媒体载荷角色 |
| `payload_auth_status` | V2 auth 校验 | V3 auth tag 校验 | 必须落库 | Payload 认证状态 |
| `revision` | V2 旧字段桥接或本地记录 | registry / 本地版本链 | 必须落库 | 版本次数 |
| `parent_watermark_uid` | V2 旧字段桥接或本地记录 | registry / 本地版本链 | 必须落库 | 上一版编号 |
| `watermark_id_issue_mode` | V2 旧字段桥接或登记记录 | registry / 本地登记记录 | 必须落库 | 编号签发模式 |
| `watermark_id_registry_status` | registry 查询或本地登记状态 | registry 查询或本地登记状态 | 必须落库 | 登记状态 |
| `watermark_id_registry_receipt` | registry receipt | registry receipt | 可为空但不得丢弃 | 登记收据 |
| `work_source_declaration` 等作品声明字段 | V2 快照只作兜底；优先本地 / registry | 本地 / registry | 必须落库 | 创作者声明 |
| `training_permission_declaration` | V2 coarse bit 只作旧记录兜底；优先 registry | registry | 必须落库 | 训练许可声明 |

同步禁止事项：

- 禁止把 `auth_tag` 原值、registry 私有证明、key material、本地保护副本路径或媒体文件本体进入同步 payload。
- 禁止把 V3 媒体 payload 当作完整授权来源同步。
- 禁止在 registry 查询失败时静默把 V2 快照展示成最新授权；必须标注迁移桥来源或待查询状态。

## 5. 报告与 UI 显示差异

| 场景 | 版权库详情 | 验证页 | 正式报告 / 摘要 | 公开权利卡 |
| --- | --- | --- | --- | --- |
| V2 旧记录，registry active | 显示 `V2 / 119 bytes`、迁移桥可读、registry active | 显示 payload verified 和 V2 迁移桥 | Payload 协议为 V2；声明以 registry 为准 | `anchorProtocol=v2_migration_anchor` |
| V2 旧记录，registry 不可达 | 显示 `V2 / 119 bytes`、来自迁移桥快照 | 显示已读到旧 payload，但 registry 未查询 | 报告必须写明“旧记录快照，未查询最新 registry” | `scanStatus=registry_unavailable` 或等价状态 |
| V2 旧记录，registry conflict | 显示冲突，需要人工复核 | 显示锚点有效但权利声明冲突 | 报告标注 conflict，不输出法律结论 | `scanStatus=conflict`，`legalConclusion=false` |
| V3 新记录，registry active | 显示 `V3 / <length> bytes`、最小锚点 | 显示锚点 verified，声明来自 registry | Payload 协议为 V3；声明以 registry 为准 | `anchorProtocol=v3_minimal_anchor` |
| V3 新记录，registry 不可达 | 显示锚点可读但授权声明待查询 | 显示不能判断训练许可 | 报告只展示锚点与待查询状态 | 不得输出允许训练结论 |
| V3 新记录，registry conflict | 显示冲突，需要人工复核 | 显示锚点有效但公开元数据 / registry 冲突 | 报告标注 conflict | `legalConclusion=false` |

统一文案边界：

- `payloadAuthStatus=verified` 的含义是“媒体锚点校验通过”，不是“授权有效”。
- `training_permission_declaration` 是创作者声明或 registry 快照，不是法律授权意见。
- V3 记录不能显示“媒体内含训练许可”。
- V2 旧记录不能显示“V2 仍是未来协议”。

## 6. 迁移桥接规则

V2 迁移桥只服务旧记录：

1. 先读取 V2/119 payload，拿到 `watermarkUid`、版本链、issue mode、media type、registry proof snapshot 和认证状态。
2. 再查询本地版权库 / 云版权库 / registry 的 active rights manifest。
3. 如果 registry active，报告和公开权利卡以 registry 为权利事实源。
4. 如果 registry 不可达，允许展示 V2 快照，但必须标注“旧记录迁移桥快照，未查询最新 registry”。
5. 如果 registry 与公开元数据或 V2 快照冲突，必须标记 conflict，并保持 `legalConclusion=false`。
6. 旧记录重签或修复时可以读取 V2 字段作为桥接输入，但新写入目标仍是 V3 最小锚点。

V3 不使用迁移桥解释授权：

1. V3 媒体只提供 `watermarkUid`、protocol 和 auth status。
2. 版本链、声明、训练许可、撤销、替代、manifest hash 均从本地 / 云端 / registry 获取。
3. registry 不可用时，V3 只能显示“锚点可读，授权声明待查询”。

## 7. 回滚门禁

V3 正式写入必须分阶段开启：

| 阶段 | 允许行为 | 禁止行为 | 退出条件 |
| --- | --- | --- | --- |
| R0 codec 准备 | 新增 V3 encode / decode 单测和合同检查 | 接入正式图片 / 音频写入默认路径 | V3 codec 单测通过，V2/119 合同不变 |
| R1 只读解析 | 桌面 / 移动端可识别 V3 fixture | 默认写入 V3 | V2 旧样本和 V3 fixture 均可读 |
| R2 feature gate 写入 | 内部 gate 开启后写 V3 | 面向用户默认写 V3 | 跨端 fixture、registry 对照、报告合同、同步合同通过 |
| R3 运行态 QA | 桌面 + Android + iOS 各写入 / 读取真实样本 | 用 Android 替代 iOS | 三端证据齐全，质量门禁通过 |
| R4 默认切换 | 新写入默认 V3/39，默认读取只接受 V3/39 | 把 V2 旧记录继续当默认算法、把迁出字段塞回媒体 payload | V3-only 图片合同和隔离 legacy 套件通过 |

回滚要求：

- `off` 是当前默认正式路径，必须写读 V3/39，不允许静默回到 V2。
- 图片 `force_v2_rollback`、`embed_v2` 和 `extract_v2` 已退役，必须稳定返回 `v2_image_rollback_retired`；音频 V2 回滚仅在隔离 legacy 套件中验证。
- V3 读取失败时不得删除或覆盖本地版权库记录。
- V2 旧记录读取、跨端互验、报告和同步能力必须保持 release blocking。
- 回滚期间公开元数据 JSON / PNG / JPEG 嵌入副本仍以 registry 为事实源，不依赖 V3 媒体 payload。

R2 最小 gate 方案已冻结在 `docs/V3 feature gate写入与回滚验证方案.md`：

- 默认图片 / 音频正式路径已写 V3/39。
- `internal_qa` 只能由内部 QA 命令显式传入，产物只进入 QA 目录或受控 fixture；它不再代表默认 V3 开关。
- `force_v2_rollback` 对图片必须拒绝；音频旧版回滚只作为隔离迁移 / 回滚工具链。
- `rights:v3-feature-gate-rollback-contract` 已验证图片 rollback 返回 `v2_image_rollback_retired`、音频 legacy rollback 与默认 V3 路径相互隔离；默认 `WatermarkService` 图片写读为 V3/39。

## 8. 实现前检查清单

进入 V3 实现前必须先完成：

- 更新 `watermark-core` 合同脚本，确认 V3 正式路径不会修改 V2 `PAYLOAD_BYTES = 119`。
- 在 `watermark-core` 增加 V3 图片 / 音频 fixture 生成和读取测试。
- 已在桌面端受控 `verify_suspect_readonly_candidate` 和 Android 原生受控 `readReadonlyCandidate` 中增加 V2 / V3 结果字段，不用布尔值隐式判断；默认 `WatermarkService::extract` 和移动端默认 `read()` 已切 V3。
- 在双端版权记录字段契约中固定 V3 字段来源、同步 payload 和报告字段，并把当前受控入口状态与正式报告 / 同步待接入状态分开记录。
- 在公开权利 SDK 中固定 V3 anchor protocol 文案。
- 在报告生成器中固定 V2 迁移桥、V3 最小锚点、registry unavailable 和 conflict 四类展示。
- 在 QA 脚本中固定桌面、Android、iOS 三端证据路径；iOS 缺环境时只能挂起，不能通过。
- 已新增 `rights:v3-readonly-candidate-runtime-qa` 作为桌面 + Android 原生端真实媒体文件证据：桌面生成真实 PNG / WAV，V3/39 分别进入正式图片 sync packet 与音频 recovery packet 承载位；桌面端和 Android 原生端通过显式 readonly candidate reader 读取 `payloadProtocolVersion=3`、`payloadBytesLength=39`、`payloadAuthStatus=verified`、`watermarkIdIssueMode=registry_resolved`、`mediaPayloadRole=v3_minimal_anchor`；默认 `WatermarkService::extract` / 移动端默认 `read()` 也已路由 V3。
- 已新增 `docs/V3 feature gate写入与回滚验证方案.md` 和 `rights:v3-feature-gate-rollback-contract`；`watermark-core` 默认写入已为 V3/39，图片显式 rollback 稳定拒绝，音频 legacy rollback 留在隔离套件。
- 已新增 `rights:v3-internal-qa-write-runtime-qa` 作为桌面 + Android 原生端运行态证据：桌面端和 Android 原生端均通过受控内部 QA 入口生成 V3/39 图片 / 音频样本，并在同一运行态验证默认写入路径也生成 V3/39。
- 在 `docs/当前真实能力边界说明.md` 中更新能力边界后，才允许对外描述 V3 媒体 codec 状态。

## 9. 推荐实施顺序

1. 先做 V3 只读解析和 fixture，不改默认写入。
2. 再接同步 / 报告字段合同，确保 V2 / V3 均可展示。
3. 再做 feature gate 写入，并保留 V2 fallback。
4. 再跑桌面端、Android 原生端、iOS 原生端跨端互验和真实运行态 QA。
5. 默认写入已从 V2 切到 V3；后续重点转为 iOS 同场景 QA、感知质量门禁和性能基准。

下一步：等 macOS + Xcode + iOS Simulator 或真机环境恢复后补齐 iOS 默认 V3 写读同场景证据，并复跑三端保护副本文件流转与感知质量门禁。
