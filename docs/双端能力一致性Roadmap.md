# HiddenShield 双端能力一致性 Roadmap

当前状态：Windows 桌面 `v0.1.3` RC / GA Gate `PASSED`（2026-07-26）；移动端继续冻结。

## 2026-07-26 中文宣传片桌面限定边界

状态：`桌面宣传片已生成；不形成移动端承诺`

- 本轮宣传片只介绍当前发布中的 Windows 桌面端，不展示或暗示移动端写入、验证、版权库、报告或云同步已经开放。
- 桌面流程仅覆盖图片 / 音频保护写入、读取验证、本地版权库和技术证据报告；宣传片演示素材与编号均为视觉样例，不进入正式跨端 fixture。
- 片中云版权库、SDK、API 统一标记为“未来规划”，个人作品身份统一标记为“终局愿景”；这些内容不代表桌面或移动端已有生产开放能力。
- 桌面限定原因是当前 `v0.1.3` 发布范围与移动端冻结状态，而不是形成长期桌面独占承诺；移动端解冻后必须重新执行图片 / 音频双向互验与页面级 QA，才可制作双端版本。
- 验证：宣传片工程、逐镜头文案和重新构建脚本位于 `docs/promo-video/`，成片输出位于 `output/promo-video/`。
- 风险：未来传播时若截取单个镜头，必须保留“未来规划”或“终局愿景”标签，防止云版权库、SDK、API 和作品身份被误读为当前双端能力。

下一双端一致性任务：

- 移动端继续冻结；若后续需要双端宣传片，先恢复移动端 release gate，并重新完成 desktop->mobile 与 mobile->desktop 图片 / 音频真实文件互验。

## 2026-07-21 桌面媒体正式支持范围

状态：`桌面发布候选范围已冻结；等待容量与上限 Gate`

- `docs/桌面媒体正式支持范围.md` 是桌面端图片 / 音频后续优化、预检、fixture 和 release Gate 的唯一产品范围标准。
- 图片候选上限为静态 PNG / JPEG / WebP、`100 MP`、`512 MiB`；最小规格只由 `watermark-core` 分块容量判定，不以 `1920×1080` 为门槛。
- 音频候选范围为 WAV / MP3 / FLAC / OGG / M4A、`8–48 kHz`、mono / stereo、`30 秒–20 分钟`、`512 MiB`，且输出保持原始采样率与声道。
- 移动端冻结，不为本范围新增实现、fixture、Gate 或产品承诺。

下一桌面媒体任务：

- 将图片 `100 MP / 512 MiB` 与音频 `20 分钟 / 512 MiB` 加入桌面和核心的不可绕过预检，再生成接近上限真实文件矩阵。

## 2026-07-21 音频规格产品边界正式收口

状态：`桌面端常见规格可对用户承诺；移动端冻结`

- 当前发布只推进桌面端；移动端不再新增功能、测试、fixture 或发布承诺，现有移动实现仅保留为冻结资产。
- 桌面端正式水印能力继续以 `watermark-core` 为唯一算法源，不增加重采样或降声道兜底。
- 当前桌面端产品口径为：HiddenShield 支持常见的 `8–48 kHz` 音频采样率和 `mono / stereo` 声道，并在保护过程中保持原始采样率与声道不变。
- `4–8` 声道及 `48 kHz` 以上采样率属于后续兼容性扩展范围；低于 `8 kHz`、高于 `48 kHz`、超过 `2` 声道以及短于单文件保护门槛的组合，不得由“常见规格支持”推导为当前承诺。
- 48 kHz 的 WAV / MP3 / FLAC / OGG / M4A、mono / stereo 基线写后回读 `10 / 10` 通过；广泛规格基线覆盖 `8–48 kHz` 的代表性组合，输出规格保持不变。
- 桌面 UI、帮助、报告和 release fixture 必须使用同一口径，并明确“原始采样率与声道保持不变”。
- `watermark-core/fixtures/audio-support-contract.json` 固化桌面最短 `30` 秒、`8–48 kHz`、`mono / stereo`、原规格保持与失败码；同时记录 PNG / JPEG / WebP 的 `1920×1080` 成功参考与 `320×240` 已知失败参考。两者都不是用户可见的图片最小尺寸承诺。`npm run watermark:audio-support-contract` 是桌面静态 release fixture Gate。
- 桌面通过 FFprobe 在文件选择和执行前阻断未知、超范围采样率/声道，不转换规格后再写入。

下一桌面媒体任务：

- 使用真实 `8 kHz mono`、`48 kHz stereo` 和 PNG / JPEG / WebP `1920×1080` 素材，在安装版完成写入、写后回读与只读验证，并记录每项耗时。

## 2026-07-21 图片 / 音频自检覆盖审计

状态：`常见音频规格边界已收口；真实格式 / 尺寸 / 声道扩展仍分层验证`

- 本轮只调用共享 `watermark-core` Gate，没有在桌面或移动端实现独立算法。
- 图片合成质量样本 8 / 8 通过；音频 6 / 6 可读取，5 / 6 通过质量门槛。
- 噪声底录音可读回但 SNR 不达标，桌面与未来移动端均不得宣称该类素材质量稳定。
- 当前不能把跨端合同通过扩张为所有格式、尺寸、时长、采样率和声道布局承诺；产品承诺限于已定义的常见规格边界。

下一双端一致性任务：

- 固化同一批真实图片 / 音频文件作为桌面与移动端未来恢复时共用的格式、尺寸、时长和声道 fixture。

## 2026-07-17 桌面正式候选 Gate 状态

状态：`桌面契约已对齐；GA 环境证据阻塞`

- `dual:contract` 已去除对已删除 `EnterpriseAuditView.vue` 的残留读取，当前改为检查桌面 Enterprise 产品入口继续保持删除，同时保留后端内部管理契约。
- `npm run dual:contract`、`npm run release:desktop-baseline`、`npm run commercial:contract` 已通过；本轮没有恢复移动端开发，也没有修改图片 / 音频 payload、写入、读取、验证、版权编号或跨端 fixture。
- Windows 正式候选仍缺 CA 签发的代码签名材料；当前主机也没有可用于“WebView2 缺失 + 物理断网”的干净 Windows Sandbox/VM，因此 GA Gate 不得升级为通过。
- 下一一致性任务：正式签名候选生成后，在同一候选上复跑图片 / 音频页面级读取验证、年度授权导入与重启持久化，并把干净 Windows 安装证据关联到该候选哈希。

## 2026-07-17 桌面安装包自包含边界

状态：`桌面 RC 安装启动通过；移动端继续冻结`

- 桌面正式 Cargo 包只保留 `hidden_shield` 产品 bin；图片 / 音频水印 QA、跨端报告 QA、服务方离线签发工具和离线发布 Gate 均迁为 examples。
- MSI / NSIS 安装后只包含产品主程序、卸载器和产品资源，不把内部测试工具误当作桌面能力交付。
- 正式安装版在 Vite 未启动、端口 `1420` 关闭时通过 CDP 读取 `http://tauri.localhost/` 产品正文，证明桌面 UI 使用内嵌前端而不是开发服务器。
- WebView2 使用 `offlineInstaller` 随安装包提供；本轮未修改图片 / 音频 payload、写入、读取、验证、版权编号或跨端 fixture。
- 移动端继续冻结，不因为桌面安装技术调整新增移动端发布前置条件。

验证：`release:desktop-baseline`、公开元数据合同、V3 迁移合同、报告合同、R4 证据包合同和 K0 离线许可证合同通过；自包含 Gate 证据位于 `artifacts/desktop-installer-self-contained/20260717071754/desktop-installer-self-contained-gate.json`。既有 `dual:contract` 仍被已删除 `EnterpriseAuditView.vue` 的历史读取路径阻断，属于待清理的合同残留。

下一一致性任务：在安装版中人工复验工作台、处理、验证、版权库、批量队列、年度授权、设置、帮助八个菜单，并用真实图片 / 音频确认写后回读结果与源码运行态一致。

## 2026-07-17 桌面处理流程减法

- 桌面普通图片 / 音频选择不再被完整盲水印负路径阻塞。
- 新版识别改为用户显式开启后按需执行；正式写入仍由共享核心阻止覆盖已有水印。
- 图片与音频统一使用五步产品进度：读取、准备、生成、验证、保存。
- 作品声明默认展开；验证页移除 Phase R4 研发阶段表达。
- 移动端继续冻结，本轮不新增移动端对齐任务。

验证：前端生产构建通过；浏览器 4K 无水印图片选择约 1.06 秒进入“图片已就绪”。

下一一致性任务：桌面 Release 下复验图片与音频的后端阶段事件均能正确映射到五步进度。

## 2026-07-17 桌面页面基线对齐

状态：`桌面图片 / 音频范围已收口；移动端继续冻结`

- 工作台、处理页和验证页统一只表达图片 / 音频能力。
- “发布 Gate”只保留在发布文档和证据中，不进入产品 UI。
- 处理页不再渲染 L1 / L2 / L3，验证页对视频扩展名和绕过入口均 fail closed。
- 版权库只展示当前图片 / 音频记录；历史视频资产保留在数据库和内部工具中，不作为当前产品能力。
- 可信时间统一采用三层语义：第三方时间戳回执、网络授时、本机创建时间（非第三方证明）。

验证：前端生产构建和桌面 Rust 检查通过。下一桌面一致性任务是在重新打包后的 Release 安装包中人工检查工作台、处理、验证和版权库四个页面。

## 2026-07-16 发布策略变更：移动端冻结与 RC / GA Gate

状态：`移动端暂停；新商业映射已冻结；桌面 RC 待重建复验`

本 Roadmap 从“双端发布总线”调整为“历史一致性记录与未来恢复依据”。当前执行规则如下：

- 冻结全部移动端新功能、视觉迁移、商业化接入、跨端补齐和常规 QA；移动端不再是当前开发目标或桌面发布前置条件。
- 后续开发与发布只允许桌面端和后端云服务。
- 已完成的图片 / 音频跨端 fixture、字段契约和互验结果继续保留，不删除、不降级，也不要求在当前桌面版本继续扩展移动端覆盖。
- 桌面端允许形成明确的平台独立发布基线：当前必须完成图片 / 音频离线验证以及服务方注册码签发、桌面离线验签。
- 桌面端全部视频入口必须隐藏或屏蔽；L1、L2、L3 均不再展示，不属于当前能力承诺或发布 Gate。
- 若未来恢复移动端开发，必须先更新本文档，重新定义恢复范围、数据迁移、共享术语和跨端 release gate，不能直接沿用冻结前的“已完成”状态。
- 当前权益只分为未付费与图片 / 音频年度基础权益：未付费不得批量，年度基础权益只增加图片 / 音频批量；两者的正式报告均按记录单独购买。
- `HSLIC1` 按年激活，不能直接授予 `report_export`；未来视频必须作为独立收费商品，当前继续隐藏。
- 桌面端当前不再展示历史 Free / Creator / Studio / Enterprise 名称、Enterprise 内部页面或团队套餐权益；移动端继续冻结，不要求同步实现本次桌面文案清理。

当前 Gate 分层：

- `RC Gate 待新映射复验`：旧签名候选安装包已在真实 WLAN 断开状态完成桌面图片 / 音频 V3/39 验证和许可证生命周期，但它早于“HSLIC1 不授予报告”的新商业映射，只保留为历史证据。
- `GA Gate 进行中`：仍需正式企业分发证书、干净 Windows 环境页面级证据、生产密钥托管及订单 / 退款 / 换机运营闭环。
- 没有干净 Windows VM 时，允许使用全新 `HiddenShieldReleaseQA` 本地用户复跑安装和页面级流程，作为 RC 的隔离账户补充证据；该证据不恢复移动端要求，也不替代 GA 的干净系统证据。

本次完成：

- 确认移动端冻结不等于删除移动端代码或破坏历史数据兼容。
- 确认当前桌面端不再等待移动端完成离线授权、视频能力或 UI 对齐。

验证：

- 文档基线已统一为桌面端 + 后端云。
- 桌面视频入口屏蔽由当前任务同步实施并通过桌面构建验证。
- 2026-07-16 桌面离线 Gate 已在真实 WLAN 断开、互联网不可达状态运行：当前签名安装版主进程可启动，图片 / 音频默认 V3/39 写读与验证 4 / 4 通过。
- 桌面离线注册码使用独立 Windows Credential Manager scope 与 SQLite 运行态完成有效、过期、重启持久化和撤销验证；本次没有修改或要求移动端实现。
- 新商业映射已通过桌面 Rust 权益测试、后端在线套餐映射测试、前端生产构建和桌面发布基线合同；当前用户可见套餐文案已收敛为未付费 / 图片音频年费，Studio 团队套餐卡已从版权库页面移除。
- 桌面导航、设置、帮助、法律文案和顶部权益标签已进一步统一为年度授权口径，旧云端 planName 不再直接显示。

风险：

- 后文仍包含冻结前的双端阶段记录，只能用于追溯，不再作为当前发布要求。
- 后端同步 payload 必须继续兼容已有移动端数据，冻结不能成为破坏历史记录的理由。
- 新映射尚未进入重建后的签名安装包，人工页面级回归也由发布负责人后续完成，因此当前 RC 仍保持“待新映射复验”。

下一一致性任务：

- 在全新 `HiddenShieldReleaseQA` Windows 本地用户下复跑安装版页面级图片 / 音频验证、HSREQ1 导出、HSLIC1 导入、注销登录和 HSRVL1 撤销，补充截图索引与用户可见状态证据；移动端继续保持冻结。

本文档是商业化落地阶段后的下一阶段执行总线。

阶段目标：让桌面端和移动端围绕图片、音频、版权库、验证、报告、同步和 L2 视频指纹存证保持同一套产品能力、同一套术语和同一套失败处理口径。

补充执行总线：

- `docs/双端视觉语言迁移实施总计划.md`
- `docs/双端版权记录字段一致性契约.md`

后续桌面端 / 移动端视觉语言迁移、组件替换、术语收口、设置与帮助改版、商业化页面对齐，统一按该计划执行。
后续版权记录字段、同步 payload、正式报告字段和双端持久化字段变更，统一按字段一致性契约执行。

## 1. 执行原则

- 桌面端和移动端默认应具备同等核心能力。
- 如果某项能力因系统限制无法完全一致，必须在 Roadmap 中说明限制、降级方式和用户可见文案。
- 不允许移动端继续暴露“桥接层”“临时直连”等技术性语言给普通用户。
- 不允许为了移动端实现方便而破坏源文件属性，例如音频单声道 / 双声道、采样率和格式处理必须保持可解释。
- 同一项功能的完成态、失败态、重写说明、版本态和验证态应使用一致的产品语言。
- 不同步原始媒体、加水印媒体和本地路径仍是双端同步硬边界。
- 双端保护副本互解是封版硬门槛：桌面端写入 / 加密的图片和音频保护副本必须能被原生移动端读取、验证或解密出同一版权编号和 payload；原生移动端写入 / 加密的图片和音频保护副本也必须能被桌面端读取、验证或解密出同一版权编号和 payload。
- 当前正式图片容器口径只收敛到 PNG / JPEG / WebP，音频容器口径只收敛到 WAV / MP3 / FLAC / OGG / M4A，视频 L1 / L2 当前正式可承诺容器收敛到 MP4 / MOV / MKV / WebM；AVI / M4V 仅保留为北极星目标。

## 2. 阶段总览

| 阶段 | 名称 | 状态 | 目标 |
| --- | --- | --- | --- |
| Phase A | 双端能力矩阵审计 | 已完成 | 建立桌面端 / 移动端当前能力对照表 |
| Phase B | 图片写入与验证一致性 | 已完成 | 对齐图片格式、写入完成态、提取与裁剪取证表现 |
| Phase C | 音频写入与验证一致性 | 已完成 | 对齐音频格式、30 秒规则、声道保持、提取稳定性 |
| Phase D | 版权库与报告一致性 | 已完成 | 对齐记录字段、详情页、筛选、报告入口与报告字段 |
| Phase E | 批量与队列一致性 | 已完成 | 对齐本地批量队列、失败重试、验证状态和订阅门禁表现 |
| Phase F | 云同步一致性 | 已完成 | 对齐同步数据模型、冲突处理、重试和隐私边界 |
| Phase G | L2 视频存证一致性 | 已完成 | 对齐 L2 存证展示、报告字段和移动端只读体验 |
| Phase H | 双端一致性合同与 QA | 已完成 | 建立自动化合同、手测清单和回归门禁 |
| Phase I | 共享水印核心与跨端互验 | 进行中 | 将图片、音频和未来视频正式能力收敛到共享核心，并把跨端互验升级为发布门禁 |

## 3. Phase A：双端能力矩阵审计

目标：

- 明确桌面端和移动端当前已经支持什么。
- 找出“桌面有、移动端缺失”或“移动端有、桌面端缺失”的差距。
- 把差距按用户影响排序。

任务：

- [x] 审计桌面端图片写入 / 提取 / 验证 / 重写能力。
- [x] 审计移动端图片写入 / 提取 / 验证 / 重写能力。
- [x] 审计桌面端音频写入 / 提取 / 格式 / 30 秒规则。
- [x] 审计移动端音频写入 / 提取 / 格式 / 30 秒规则。
- [x] 审计双端版权库字段、报告字段、同步字段。
- [x] 输出 `docs/双端能力一致性矩阵.md`。

验收标准：

- 每个核心能力都有桌面端、移动端、差距、优先级、建议处理方式。
- 明确哪些能力必须完全一致，哪些能力允许移动端阶段性只读。
- 下一阶段任务从矩阵中选择，不凭感觉推进。

## 4. Phase B：图片写入与验证一致性

目标：

- 图片单文件写入、提取、完成态和验证态在双端一致。

任务：

- [x] 对齐支持格式。
- [x] 对齐写入前预检。
- [x] 对齐写入完成后的结果卡。
- [x] 对齐水印版本态和重写提示。
- [x] 对齐提取失败、疑似命中、可信命中的文案。
- [x] 对齐裁剪、缩放、亮度、噪声、旋转 / 镜像攻击下的取证说明。

验收标准：

- 同一张图片在桌面端和移动端写入后均可进入版权库。
- 同一张被处理图片在双端验证页给出同口径结果。
- 双端不会对同一攻击场景给出冲突承诺。

## 5. Phase C：音频写入与验证一致性

目标：

- 30 秒以上音频版权保护能力在双端一致。

任务：

- [x] 对齐支持格式列表。
- [x] 对齐 30 秒以下音频拒绝或提示口径。
- [x] 确认移动端不强制把双声道破坏成单声道。
- [x] 对齐采样率、声道、编码转换后的用户可见说明。
- [x] 对齐音频写入完成后的可验证性结果。
- [x] 对齐音频提取失败、短片段不足、格式不支持的错误码与文案。

验收标准：

- 同一首 30 秒以上音频在双端写入和提取结果一致。
- 移动端不会因归一化处理破坏用户源文件属性承诺。
- 5 / 10 / 15 秒短片段不再作为产品承诺场景。

## 6. Phase D：版权库与报告一致性

目标：

- 双端版权库像同一套数据产品。

任务：

- [x] 对齐版权记录字段。
- [x] 对齐图片、音频、视频存证记录详情。
- [x] 对齐水印版本、父水印、重写原因和验证状态。
- [x] 对齐 L2 视频存证字段展示。
- [x] 对齐正式报告入口、门禁和报告字段。
- [x] 对齐“报告不是法律意见或司法鉴定”的提示。

验收标准：

- 同一条云同步记录在双端展示字段一致。
- 正式报告字段不泄漏原始媒体、本地路径或本地 bundle 路径。
- 双端报告口径一致。

## 7. Phase E：批量与队列一致性

目标：

- 本地批量作为 Creator 权益，在双端的门禁、队列和失败处理一致。

任务：

- [x] 对齐 Free 门禁行为。
- [x] 对齐 Creator 队列创建。
- [x] 对齐图片 / 音频批量项状态。
- [x] 对齐暂停、继续、取消和重试。
- [x] 对齐写入后验证失败的队列表现。

验收标准：

- Free 双端都不能创建正式批量任务。
- Creator 双端都能看懂队列状态和失败原因。
- 失败项不会悄悄进入版权库成功态。

## 8. Phase F：云同步一致性

目标：

- 双端同步行为可预测、可解释、可恢复。

任务：

- [x] 对齐同步字段白名单。
- [x] 对齐冲突解决策略。
- [x] 对齐失败重试和重试上限。
- [x] 对齐退出账户后的本地保留行为。
- [x] 对齐创作者身份、订阅权益和版权记录元数据同步。

验收标准：

- 同一账户下桌面端和移动端记录最终一致。
- 不同步原始媒体、加水印媒体和本地路径。
- 冲突解决不会保留互相矛盾的用户可见状态。
- 已登录且 `cloud_sync=true` 的 Creator / Studio / Enterprise 在正式 `auth/sessions`、`me`、refresh 路径下默认自动云同步双端版权库；Free 被后端同步 API 403 阻断；当前设备可通过 `sync-preferences` 暂停为 `manual_local_only` 或恢复为 `auto_cloud_vault`。

## 9. Phase G：L2 视频存证一致性

目标：

- L2 视频指纹存证在双端可理解、可展示、可报告。

任务：

- [x] 桌面端保留生成和提交 L2 指纹包能力。
- [x] 移动端支持查看 L2 存证记录。
- [x] 双端报告字段一致。
- [x] 双端都明确 L2 不是视频画面盲水印。
- [x] 不在移动端本地提供视频盲水印写入入口。

验收标准：

- 桌面端创建的 L2 存证记录可同步到移动端查看。
- 移动端不会暗示本地视频盲水印能力已经存在。
- 报告不包含原始视频、本地 bundle 路径或可还原画面的素材。

## 10. Phase H：双端一致性合同与 QA

目标：

- 把双端一致性从“人工记忆”变成“合同和 QA 门禁”。

任务：

- [x] 新增 `dual:contract`。
- [x] 检查双端术语、关键文案和能力入口。
- [x] 检查图片 / 音频 / 版权库 / 报告 / 同步 / L2 字段一致性。
- [x] 编写 `docs/双端能力一致性QA清单.md`。
- [x] 将关键检查接入 CI。

验收标准：

- 双端一致性变更可被脚本发现。
- 新增能力时必须同步评估桌面端和移动端。
- QA 清单能指导下一轮真机和 Tauri 手测。

## 11. Phase I：共享水印核心与跨端互验

目标：

- 停止围绕双端能力做零碎补丁，把正式水印能力收敛到同一套算法核心。
- `watermark-core` 是图片和音频正式水印写入、读取、payload 编码、版权编号、重写检测和写入后验证的唯一事实源。
- 版权编号必须从“创作者 / 设备派生短标识”收紧为正式保护副本写入后的记录级身份；在线优先由后端签发 / 确认唯一，离线使用 128-bit 级高熵本地编号兜底；同一创作者、同一设备写入不同作品时必须生成不同 `watermarkUid`，版本关系由 `parentWatermarkUid + revision` 表达。
- 未来视频盲水印写入与验证必须复用 `watermark-core`；云端只能作为 `watermark-core` 的执行、密钥、策略和自检编排包装层，不能成为第二套算法核心。
- 商业化 Phase 7 已落地的视频能力必须纳入一致性要求：L1 视频音轨水印、L2 视频指纹存证、L3 端云协同画面盲水印分层验收。
- 任一正式端写入的保护副本，必须能被支持同类媒体的另一正式端正确解析。
- 本版封版时，跨端互验不只看 UI 字段一致，还必须验证保护副本本体可互读 / 互验 / 互解。

本次进展：

- 后端已新增 `watermark-ids` 登记 API 与 registry / reissue 数据表。
- 桌面端和移动端版权库、存证摘要、正式报告、云同步 payload 已保存并展示 `watermarkIdIssueMode`、登记状态、登记收据、父编号、`revision`、payload protocol、payload bytes 和 payload auth status。
- 当前双端图片 / 音频正式写入已保持离线可用并在线优先接入后端 `reserve -> confirm`；后端不可用时落为 `offline_generated + pending_registration`，云同步前自动执行 `confirm / reconcile` 并回写为 `server_confirmed` 或 `offline_confirmed`。
- 同 UID 不同作品哈希已从旧的静默变体模型切换为 `pending_registry_reconcile`：桌面端和移动端版权库均展示登记仲裁入口；桌面端可在保护副本可访问时调用后端 `reissue` 并重写 V2 保护副本，移动端可创建重新签发任务并等待用户重新选择文件完成 payload 修复。

执行文档：

- `docs/共享水印核心与跨端互验推进计划.md`
- `docs/共享水印核心算法审计.md`
- `docs/当前真实能力边界说明.md`
- `docs/版权编号唯一性与版本链语义设计.md`
- `docs/音频噪声底跨端可读频带策略迁移设计.md`

任务：

- [x] 完成 Phase I 启动前算法审计，梳理桌面端、原生移动端、Web 预览、后端和视频相关算法影响面。
- [x] 固化 `watermark-core` 正式 API、payload 字段、版权编号唯一性规则、版本链语义和核心错误码；V2-119 payload、离线高熵编号、后端登记、双端数据层、双端图片 / 音频写入链路在线优先 `reserve -> confirm`、同步前自动 `confirm / reconcile`、仲裁 UI 与历史修复入口已完成。
- [x] 建立首个图片和音频跨端金样本 fixtures。
- [x] 覆盖桌面写入 / 移动端读取、移动端写入 / 桌面读取的图片互验。
- [x] 覆盖桌面写入 / 移动端读取、移动端写入 / 桌面读取的音频互验。
- [x] 新增 `watermark:cross-end-contract` 发布门禁命令，并接入 CI。
- [x] 扩展图片真实容器矩阵：PNG / JPEG / WebP 输入均纳入 mobile->desktop 与 desktop->mobile 双向互验门禁。
- [x] 扩展音频真实容器矩阵：WAV / MP3 / FLAC / OGG / M4A 已纳入门禁；非 WAV 样本使用仓库固定 fixture，CI 不依赖本机 FFmpeg。
- [ ] 裸 AAC / ADTS 需要补标准 fixture 或明确产品承诺收束到 M4A(AAC in MP4)。
- [x] 跨端互验失败归因固定为 `core_algorithm`、`mobile_normalize`、`desktop_transcode`、`bridge_contract`、`fixture_invalid`。
- [x] `desktop_transcode` 归因接入真实 FFmpeg fixture：桌面端 MP3 / FLAC / OGG / M4A 抽取为 WAV 后必须可进入 `watermark-core` 写入和提取。
- [x] 拆分 `watermark:cross-end-contract` 的 fast / release 两级门禁：fast 用于本地快速验证，release 继续作为 CI 完整门禁。
- [x] 收口移动端 Web 预览边界：不调用同核时只能作为 UI 预览，不得作为正式水印能力。
- [x] 在原生移动端同核互验稳定后，再补齐单文件保护副本保存 / 分享入口。
- [x] 视频一致性纳入 Phase I：L1 视频音轨水印补互验要求，L2 视频指纹存证补 bundle / notary / 同步 / 报告一致性要求，L3 进入实现前先完成 `watermark-core` 视频画面算法和云端执行包装设计。
- [x] 稳定噪声底音频频带迁移进入 read-only 扫描阶段：`docs/音频噪声底跨端可读频带策略迁移设计.md` 已固定新旧 extractor 兼容、双端读取迁移、fixture、回滚和正式阈值不降规则；`watermark:audio-noise-floor-migration-read-compat` 已验证生成式旧样本、桌面 file-backed 旧产物和 Android 原生 Rust bridge file-backed 旧产物读取兼容；当前只读候选扫描对 5 个旧 V3/39 fixture 均返回 `candidate_payload_not_found` 并 fallback 到 legacy V3。`protected-new-candidate/manifest.draft.json` 已固定未来新候选 fixture 草案和阻断矩阵，但尚未进入写入迁移实现。

验收标准：

- 图片和音频任一正式端写入，另一正式端可解析出同一版权编号和 payload。
- 同一创作者、同一设备连续写入不同作品时，桌面端和移动端都必须生成不同版权编号；在线使用后端签发编号，离线使用高熵本地编号并补登记；对已有水印作品作为新版写入时，必须生成新版权编号并通过 `parentWatermarkUid + revision` 保留版本链。
- 桌面端写入 / 加密的图片和音频保护副本可被原生移动端读取 / 验证 / 解密；原生移动端写入 / 加密的图片和音频保护副本可被桌面端读取 / 验证 / 解密。
- 正式能力路径不存在 preview marker、mock hash 或 mock copyright ID。
- Web 预览不再被用作正式双端能力验收依据，除非已接入同一共享核心。
- 任一端修改写入、读取、payload、版权编号、重写规则或验证规则时，发布门禁能发现跨端不兼容。
- 未来视频正式能力不出现桌面端、移动端、后端、云任务或脚本各自实现盲水印算法的分叉。
- L1 视频音轨水印继续保持本地可用、共享核心、跨端互验的一致性。
- L2 视频指纹存证继续保持三层摘要、manifest 隐私拒绝、不扣 `video_minutes`、双端版权库和报告字段一致。
- 工作台视频入口必须始终可见，且要明确拆成 L1 可用和 L2 锁定两层，不能把视频能力整块隐藏到用户看不到。

## 12. 当前推荐执行顺序

当前主线切换为 `docs/双端现有能力发布计划.md`：冻结 L3，优先把现有可承诺能力接入双端并发布版本。

1. 跑自动化发布门禁：`dual:contract`、`commercial:contract`、`commercial:ci`、`watermark:architecture-contract`、`watermark:cross-end-release`、`watermark:video-phase-contract`、桌面构建 / Tauri 测试 / Flutter 测试 / 后端测试。
2. 做桌面端运行态验收：图片、音频、验证、版权库、正式报告、云同步、本地批量、L2 视频指纹存证、成熟错误提示。
3. 做原生移动端运行态验收：图片、音频、验证、保护副本分享、复制存证摘要、版权库、正式报告草稿、云同步、本地批量、设置页反馈和日志。
4. 做跨端互验 / 互解：桌面写入移动端验证、移动端写入桌面验证，并用真实保护副本验证 desktop->mobile 与 mobile->desktop 都能读取 / 验证 / 解密出同一版权编号和 payload，至少覆盖图片和音频。
5. 做发布阻断项确认：Free / Creator / Studio 权益门禁、微信支付真实联调阻断、法务文案阻断、L3 不进入本版发布。
6. 按已确认的 `docs/版权编号唯一性与版本链语义设计.md` 继续实施后端编号签发、双端落库字段、同步登记仲裁和历史重复编号修复队列；`watermark-core` 离线高熵兜底已完成。

## 13. 回写记录

| 日期 | 变更 | 状态 |
| --- | --- | --- |
| 2026-06-19 | 商业化落地阶段收尾后，创建双端能力一致性 Roadmap，明确下一阶段主线从商业化细化切回桌面端与移动端核心能力一致性。 | 已完成 |
| 2026-06-19 | 完成 Phase A 双端能力矩阵审计，新增 `docs/双端能力一致性矩阵.md`；确认当前最大缺口在音频一致性：移动端非 WAV 30 秒预检、桌面端强制 44.1kHz 双声道、移动端验证结果模型弱于桌面端。下一步进入 Phase C。 | 已完成 |
| 2026-06-19 | 推进 Phase C：移动端音频写入页改用统一 metadata 预检，覆盖 WAV / MP3 / AAC / FLAC / OGG / M4A 的时长识别，无法确认时长时不生成保护副本；桌面端 FFmpeg 音频抽取改为根据 FFprobe 保留源采样率和声道，不再默认强制 44.1kHz 双声道；移动端状态文案统一为“本地处理”。下一步继续对齐双端音频验证结果模型和失败原因码。 | 进行中 |
| 2026-06-19 | 完成 Phase C：移动端新增 `MobileVerificationResult`，音频 / 图片验证页成功读取水印后会匹配本机版权库，输出 `matched_original`、`matched_hash_mismatch`、`watermark_detected_unregistered` 等桌面兼容原因码，并显示匹配状态和置信度；错误文案移除移动端不适用的 FFmpeg 表述。下一步进入 Phase B，继续对齐图片验证和完成态。 | 已完成 |
| 2026-06-19 | 推进 Phase B：移动端图片写入页显式限制并说明 JPG / PNG / BMP / TIFF / WebP，确认移动端生成 PNG 保护副本；补齐与桌面端一致的图片默认检测边界说明，包括二次保存、JPEG 压缩、轻度缩放 / 轻裁剪、局部遮挡、90 / 180 / 270 度旋转与水平 / 垂直镜像；图片验证结果复用 Phase C 新增的本机版权库匹配、置信度和原因码模型。下一步进入 Phase D，对齐版权库详情和正式报告字段。 | 已完成 |
| 2026-06-19 | 完成 Phase D：移动端版权库 L2 视频存证详情补齐收据签名、用量流水和指纹包大小；正式报告草稿补齐结构化字段清单、可信时间占位和 L2 隐私边界，继续明确不同步原始媒体、加水印媒体、本地路径，报告仅作为技术取证辅助材料。下一步进入 Phase E / F，继续对齐本地批量和云同步一致性。 | 已完成 |
| 2026-06-19 | 完成 Phase E：双端均保持 Free 不进入文件选择、不创建批量队列，Creator 可创建本地图片 / 音频队列；移动端批量音频处理复用 30 秒以上规则，短音频或无法确认时长会标记失败并可重试，不会进入版权库成功态；队列状态、暂停、继续、取消、失败重试和完成后验证失败表现与桌面端一致。下一步进入 Phase F 云同步一致性。 | 已完成 |
| 2026-06-19 | 加固 Phase E：短于 30 秒和无法确认时长的批量音频不再只显示失败状态，双端统一给出友好提示：说明未生成保护副本的原因，并提示选择 30 秒以上完整音频作品或更换可识别时长的完整音频文件后重试。 | 已完成 |
| 2026-06-19 | 加固 Phase C / E：桌面端 `SourceMeta` 增加时长确认状态，单个音频写入页可区分短于 30 秒与无法确认时长，并在按钮前给出友好阻断提示；桌面音频写入流水线对短音频和未知时长做后端硬阻断，不生成保护副本；移动端单个写入页和批量页保持同一提示口径。验证：`npm run build`、`flutter analyze`、移动端批量友好提示测试、Rust probe 测试通过。 | 已完成 |
| 2026-06-19 | 完成 Phase F：双端同步 payload 固化为版权记录白名单字段，移动端发送层和桌面端接收层都会过滤本地路径、保护副本路径、输入引用等字段；移动端拉取远端记录时统一按 UID / 哈希 / 版本解决冲突，同 UID 同哈希取最高版本，同 UID 不同哈希保留为变体；双端失败重试保持 5 次上限和退避，手动重试可重新进入队列；退出账户只清云身份、令牌、工作区、创作者档案和权益，本地版权库与本地队列保留。验证：移动端同步测试、Rust sync storage/cloud 测试通过。 | 已完成 |
| 2026-06-19 | 完成 Phase H：新增 `scripts/verify-dual-consistency-contract.mjs` 和 `npm run dual:contract`，检查双端 30 秒音频友好阻断、同步 payload 白名单、本地路径过滤、冲突策略、退出账户本地保留、L2 视频边界、报告法律边界以及隐藏桥接 / 临时直连文案；新增 `docs/双端能力一致性QA清单.md`，并把 `dual:contract` 接入 CI。验证：双端一致性合同 OK。 | 已完成 |
| 2026-06-19 | 完成 Phase G：复核桌面端 L2 视频指纹包生成、提交存证和版权库入库闭环；移动端工作台统一为“视频指纹存证”只读定位，明确只查看同步来的 L2 存证记录，不做本地视频盲水印且不会上传原始视频；双端版权库和正式报告字段保持存证编号、存证时间、收据签名、用量流水、指纹根、指纹包摘要、大小、采样帧、生成耗时和采样策略一致；`dual:contract` 与 `cloud-video:ui-contract` 已覆盖该边界。 | 已完成 |
| 2026-06-20 | 完成“双端产品语言最终对齐”：桌面端和移动端当前 UI 统一使用“图片写入 / 音频写入 / 验证 / 版权库 / 正式报告 / 云同步 / 视频音轨水印 / 视频指纹存证”；按钮、结果卡、失败提示、帮助页、订阅权益、版权库筛选和云同步说明去除旧的“取证页 / 图片保护 / 音频保护 / 证据报告导出 / 版权保护副本”等不一致表达；`dual:contract` 新增产品语言断言。验证：`npm run dual:contract`、`npm run build`、`flutter test test/widget_test.dart` 通过。下一步进行一次桌面 Tauri 与移动端真机人工 QA，按新版术语检查截图和交互。 | 已完成 |
| 2026-06-20 | 修复双端版权库字段与移动端写入结果可见性：移动端图片 / 音频写入结果卡补齐保护副本名称、保存 / 分享方式和入口，明确移动端不伪造本地路径；移动端 `VaultRecord`、桌面端 `VaultRecord` 与云同步 payload 补齐创作者身份、第三方验证和可信时间元数据；双端版权库卡片 / 详情展示缺失值为“未记录”，老记录兼容；同步白名单继续排除本地路径、原始媒体和保护副本路径。验证：`flutter analyze`、`cargo check`、`npm run dual:contract` 已通过，后续继续跑完整构建与 Flutter 测试。下一步按新增字段做一轮桌面 Tauri + 移动端真机写入/版权库截图 QA。 | 已完成 |
| 2026-06-20 | 补齐移动端版权信息能力，而不是只整理“未记录”展示：移动端图片 / 音频 / 本地批量写入成功后会请求 HTTP Date 网络授时，版权库记录写入“已记录网络授时”、来源、记录时间、第三方验证服务和验证路径；正式报告草稿生成后复制到系统剪贴板。桌面端修复继续账户后本机 `identity.json` 未补写创作者身份的问题，并在版权库列表和正式报告导出时对旧记录做创作者身份显示兜底，避免新报告继续出现“创作者身份：未记录”。验证：`npm run dual:contract`、`cargo check`、`cargo test report`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/sync_transport_test.dart test/widget_test.dart` 通过。下一步用真实桌面写入和移动端真机写入各生成一条新记录，对比版权库卡片、详情和报告复制文本。 | 已完成 |
| 2026-06-20 | 加固移动端可信时间入库闭环：新增后端 `/v1/trusted-time` 授时代理，移动端写入时优先请求 HiddenShield 后端获取 HTTP Date 网络授时，避免 Flutter Web 直接访问第三方站点被 CORS 或浏览器网络策略拦截；版权库详情新增“复制存证摘要”入口，与桌面端一键复制摘要能力对齐，Creator 正式报告仍保留独立入口。验证：`npm run dual:contract`、`cargo test`（feedback-backend）、`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/widget_test.dart` 通过。下一步重启双端和后端后，用移动端重新写入一张图片，确认版权库详情显示“已记录网络授时”和“HiddenShield 后端 HTTP Date”。 | 已完成 |
| 2026-06-20 | 修复移动端写入记录仍出现验证信息“未记录”和摘要格式漂移：`WatermarkWriteResult` 携带写入 payload seed，移动端入库时把写入时间、设备 ID 和回读片段写入版权记录；版权库详情对写入记录显示“写入后验证信息”，对验证记录显示“验证提取信息”，不再把不适用字段空值混入写入记录；移动端复制存证摘要的完成后验证状态统一为桌面同款“已通过 / 未通过 / 未记录”。验证：`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/widget_test.dart test/rewrite_preflight_test.dart`、`npm run dual:contract` 通过。下一步重启移动端后再生成一条新写入记录，确认摘要与桌面字段顺序一致且详情不再出现无意义“未记录”。 | 已完成 |
| 2026-06-20 | 补齐移动端设置中桌面已有的匿名反馈、体验改进、占用、问题反馈和导出日志能力：移动端匿名反馈接入后端 `/v1/anonymous-feedback/batches`，失败时保留本机队列并持久化；体验改进基于本机用量、同步失败、本地批量失败和匿名反馈状态生成风险摘要；占用展示本机记录估算而不伪造媒体路径；问题反馈补齐微信、邮箱复制入口；导出日志生成安全诊断文本，不包含媒体文件、本地路径、文件名或完整作品指纹。验证：`dart format`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/widget_test.dart`、`npm run dual:contract` 通过。下一步重启移动端和后端，手测设置页五项入口、复制联系方式、导出日志和发送反馈失败保留队列。 | 已完成 |
| 2026-06-20 | 修复移动端版权编号和写入后摘要入口一致性：移动端预览桥不再生成 `preview-img-* / preview-aud-*` 版权编号，改为与桌面端一致的 HS 展示前缀；移动端图片 / 音频写入结果卡新增“复制存证摘要”，直接复用 `MobileAppState.buildCopyrightSummary`，版权库详情也调用同一方法，避免两处摘要格式漂移。后续正式格式已收敛为完整 128-bit `HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`，Web 预览仍不得作为正式水印能力证据。验证：`flutter analyze`、`flutter test test/rewrite_preflight_test.dart test/mobile_app_state_test.dart test/widget_test.dart`、`npm run dual:contract` 通过。下一步重启移动端，重新写入图片，确认结果卡直接复制摘要且正式编号为完整长格式。 | 已完成 |
| 2026-07-02 | 收口版权编号长格式一致性：前端 mock、桌面已有水印错误解析 regex、移动端重写预检错误解析 regex、后端 / Tauri / 移动端相关测试 fixture 全部迁移到完整 128-bit `HS-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`；新增 `watermark:uid-format-contract` 并接入 `commercial:ci`，阻断正式 UI / mock / 自动化再出现旧 `HS-XXXX-XXXX-XXXX`。验证：`watermark:uid-format-contract`、`npm run build`、`flutter test test/mobile_app_state_test.dart test/rewrite_preflight_test.dart`、Tauri / 后端定向测试通过；全量 `commercial:ci` 已通过 UID format contract，后续 `watermark:cross-end-release` 因临时 Cargo target 磁盘空间不足中断。下一步用当前桌面 dev 窗口执行最近任务与记录人工 QA，确认不再生成新的短编号。 | 进行中 |
| 2026-06-20 | 修复移动端 Web 预览每次重启都要求登录和首登设置：根因是入口在 Web 环境直接创建 `MemoryVaultStore()`，导致 `SyncProfile` 中的账号、首登完成、创作者身份、权益和匿名反馈队列只存在内存中。移动端入口改走平台存储工厂，原生移动端继续使用 SQLite，Web 预览不再使用纯内存库保存首登资料，改为把 `SyncProfile` 持久化到浏览器 localStorage；该持久层只保存用户资料和设置，不保存原始媒体、保护副本、本地路径或受保护副本路径。`dual:contract` 新增断言防止入口回退到 `kIsWeb ? MemoryVaultStore()`。验证：`flutter analyze`、`flutter test test/rewrite_preflight_test.dart test/mobile_app_state_test.dart test/widget_test.dart`、`npm run dual:contract` 通过。下一步重启移动端 Web 预览，完成一次首登后再次重启，确认不再出现登录和基础设置。 | 已完成 |
| 2026-06-20 | 收敛双端用户可见错误口径：移动端登录和云同步失败不再把 `ClientException`、`Failed to fetch`、HTTP body 等技术性错误直接进入主提示，统一显示“暂时无法连接服务 / 登录状态已失效 / 授权不一致 / 服务暂时不可用”等可行动文案；桌面端工作台、验证页、版权库、设置和订阅页新增通用错误翻译，技术细节仅保留在控制台或诊断信息中。`dual:contract` 新增断言防止技术性错误不再直接进入主提示；移动端测试覆盖登录网络异常不泄漏 `ClientException`、`Failed to fetch` 和接口路径。验证：`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/widget_test.dart`、`npm run build`、`cargo test --manifest-path feedback-backend/Cargo.toml`、`cargo check --manifest-path feedback-backend/Cargo.toml`、`npm run dual:contract` 通过。下一步用关闭后端的方式手测移动端登录和桌面继续账户，确认用户只看到成熟产品提示。 | 已完成 |
| 2026-06-20 | 补齐移动端导入便捷性与桌面端一致：移动端工作台不再拆成“图片写入 / 音频写入”两个前置入口，改为“作品写入”，用户选择图片或音频后由系统按文件类型自动进入对应写入流程；移动端验证页移除图片 / 音频分段选择，选择疑似样本后自动识别类型并开始验证。合同新增断言：移动端工作台必须有自适应写入入口，验证页不得出现媒体类型分段按钮，文件选择覆盖图片和音频扩展。验证：`flutter analyze`、`flutter test test/widget_test.dart test/mobile_app_state_test.dart`、`npm run dual:contract` 通过。下一步在移动端 Web 手测选择一张图片和一段音频，确认入口都从“作品写入”开始且验证页无需手选类型。 | 已完成 |
| 2026-06-20 | 修复双端已有水印再次写入的失败路径：桌面端和移动端都把已有水印检测前移为写入前硬阻断，形成“已有水印写入前硬阻断”合同；未开启“作为新版写入”时直接停在开始前，不启动桌面 pipeline、不进入移动端写入桥；桌面进度事件、通用错误翻译和移动端写入错误翻译都统一为“检测到已有版权记录，如需生成新版请开启作为新版写入”，不再把 `Watermark embedding failed` 或 `watermark already exists in source media` 暴露给用户主提示；移动端预览桥也补上同样的二次写入防线。验证：`flutter analyze`、`flutter test test/rewrite_preflight_test.dart test/widget_test.dart test/mobile_app_state_test.dart`、`npm run build`、`cargo check --manifest-path src-tauri/Cargo.toml`、`npm run dual:contract` 通过。下一步用真实已写入图片和音频在双端手测不开新版 / 开新版两条路径。 | 已完成 |
| 2026-06-20 | 加固选择素材后的即时版权记录判断：桌面端和移动端在选择图片 / 音频后立即展示是否已有水印，并在未开启“作为新版写入”时直接禁用“生成保护副本 / 开始处理”按钮，避免用户等到点击开始后才知道需要新版写入；桌面端视频能力改为 L1 视频音轨水印与 L2 视频指纹存证双层展示，Creator 只解锁 L2 提交，Free 也能看到 L1 可用和 L2 锁定的分层定位。验证：`npm run build`、`flutter analyze`、`flutter test test/rewrite_preflight_test.dart test/widget_test.dart test/mobile_app_state_test.dart`、`npm run dual:contract` 通过。下一步用 Free 和 Creator 两个权益状态分别手测桌面视频导入页。 | 已完成 |
| 2026-06-20 | 修复桌面文件选择和移动端 Web 预览误导：桌面 DropZone 的文件选择器失败会在工作台直接给出用户提示，不再表现为“控件没反应”；移动端 Web 预览明确标记为非正式水印能力，写入和验证按钮禁用，避免生成无法被桌面端验证的预览 marker；原生移动端 Rust 桥新增图片 / 音频产物可被桌面同核 `watermark-core` 提取的互验测试。验证：`npm run build`、`flutter analyze`、`flutter test test/widget_test.dart test/rewrite_preflight_test.dart`、`cargo test --manifest-path mobile_app/rust/Cargo.toml api::tests`、`npm run dual:contract` 通过。下一步用原生移动端而非 Web 预览生成一张图片和一段音频，再分别在桌面端验证。 | 已完成 |
| 2026-06-20 | 将双端能力一致性升级为共享水印核心硬约束：`AGENTS.md` 明确 `watermark-core` 是图片 / 音频正式水印算法、payload、版权编号、重写检测和写入后验证的唯一事实源；新增 `docs/共享水印核心与跨端互验推进计划.md`，把跨端金样本互验、Web 预览边界、保护副本出口和未来视频同核设计列为 Phase I 主线。下一步执行 Phase I-1，固化 `watermark-core` 正式 API 与错误码契约。 | 进行中 |
| 2026-06-20 | 完成 Phase I 启动前算法审计，新增 `docs/共享水印核心算法审计.md`：确认当前桌面端和原生移动端已调用 `watermark-core`，但 payload 构造、桌面 FFmpeg 与移动 Symphonia 音频预处理、Web 预览 preview marker、未来视频边界仍是高风险漂移点；Phase I 必须先固化契约和金样本互验，不得直接迁移或重写核心算法。下一步按审计文档执行 Phase I-1。 | 进行中 |
| 2026-06-20 | 将商业化 Phase 7 已落地的视频能力纳入 Phase I 一致性规划：L1 视频音轨水印复用 `watermark-core` 音频算法并需要补视频成品抽音轨互验；L2 视频指纹存证已有 `VideoFingerprintBundle`、三层摘要、notary API、桌面生成/提交、移动端同步展示和报告字段，必须进入双端一致性门禁；L3 端云协同画面盲水印仍需先完成 `watermark-core` 视频画面算法和云端执行包装设计。 | 进行中 |
| 2026-06-20 | 将盲水印算法边界升级为全域硬约束：所有当前和未来盲水印写入、读取、验证、payload 编码、版权编号和重写检测只能在 `watermark-core` 实现；桌面、移动端、后端、云任务和脚本只能包装或调用核心，L3 云端视频画面盲水印也不能另起算法核心。 | 进行中 |
| 2026-06-20 | 开始 Phase I-1 契约落地：`watermark-core` 新增共享 payload builder 与契约测试；移动端原生桥改走 `WatermarkPayload::from_identity_and_media`，Dart 层不再派生 creator/file seed；CI 新增 `watermark:architecture-contract`。 | 进行中 |
| 2026-06-20 | 继续 Phase I-1 身份派生收口：按不做兼容/迁移的工业化要求，`watermark-core` 新增 `WatermarkIdentity` / `IdentityBuildInput`，桌面端 `identity.json` 只保存创作者身份和设备身份源数据，旧 `user_seed_hex` / `device_id_hex` 格式不再加载；未完成身份设置时写入直接阻断。 | 进行中 |
| 2026-06-20 | 完成 Phase I-4 移动端 Web 预览边界收口：Web preview bridge 仍可用于 UI 体验，但写入/验证结果统一标记为非正式水印；移动端状态层拒绝非正式结果进入版权库和云同步队列，`dual:contract` 与移动端状态测试固定该边界。验证：`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/rewrite_preflight_test.dart test/widget_test.dart`、`npm run dual:contract` 通过。下一步进入 Phase I-5，补齐原生移动端保护副本保存 / 分享出口。 | 已完成 |
| 2026-06-20 | 推进 Phase I-5 原生移动端保护副本出口：图片 / 音频单文件写入成功后，结果卡“保存或分享保护副本”会把正式写入 bytes 交给系统分享面板；图片使用 PNG 文件名与 `image/png`，音频使用 WAV 文件名与 `audio/wav`。该能力不生成伪本地路径，也不把保护副本路径写入云同步。验证：`flutter analyze`、`flutter test test/widget_test.dart test/mobile_app_state_test.dart`、`npm run dual:contract` 通过。下一步用原生移动端真机分别分享到相册 / 文件，再拿桌面端验证。 | 已完成 |
| 2026-06-20 | 继续 Phase I-1 payload 收口：移除正式 payload 的 precomputed builder 出口，桌面端图片、音频和视频音轨写入统一把 creator/device 源身份与媒体 SHA-256 交给 `watermark-core` 生成 payload；架构合同禁止正式 wrapper 调用 `WatermarkPayload::new/from_precomputed`，避免平台层重新实现 seed、版权编号或 file hash 截断规则。 | 进行中 |
| 2026-06-20 | 继续 Phase I-1 错误码收口：`watermark-core` 新增结构化 `WatermarkErrorCode`，桌面 pipeline、移动 Rust bridge、桌面 Vue 和移动 Dart 写入错误映射都优先按稳定 code 处理已有水印、缺创作者身份、payload 不完整、写入失败和提取失败；英文技术错误仅保留为兜底，不再作为产品主路径。 | 进行中 |
| 2026-06-20 | 启动 Phase I-2 / I-3 互验门禁：新增 `watermark:cross-end-contract`，用移动原生 Rust bridge 和桌面 `watermark-core` 路径覆盖图片、音频 mobile->desktop 与 desktop->mobile 双向互验，并接入 CI。 | 进行中 |
| 2026-06-20 | 继续 Phase I-2 真实容器矩阵：图片输入从 PNG 扩展到 PNG / JPEG / WebP，三类输入均覆盖桌面写入移动读取、移动写入桌面读取；音频非 WAV 容器保持待办，下一步必须补标准 fixture 或纯 Rust 编码策略后再作为 CI 硬门禁。 | 进行中 |
| 2026-06-20 | 继续 Phase I-2 音频真实容器矩阵：新增 MP3 / FLAC / OGG / M4A 仓库 fixture，移动端原生桥 bytes-only 解码增加容器文件头 hint；跨端合同现在运行完整 mobile Rust API 测试，非 WAV 输入归一化和移动写入产物桌面 core 提取都进入门禁。 | 进行中 |
| 2026-06-20 | 继续 Phase I-2 失败归因合同：`watermark:cross-end-contract` 分组运行 fixture 自检、图片桥接互验、WAV core 互验、非 WAV 移动归一化和非 WAV 产物提取；失败会输出稳定归因 code，后续桌面 FFmpeg 和 L1 视频音轨接入 `desktop_transcode` 分类。 | 进行中 |
| 2026-06-20 | 继续 Phase I-2 `desktop_transcode`：桌面端新增真实 FFmpeg 抽取 fixture 测试，MP3 / FLAC / OGG / M4A 经桌面 `extract_audio` 归一化成 WAV 后进入 `watermark-core` 写入与提取；跨端合同新增桌面转码归因分组。 | 进行中 |
| 2026-06-20 | 完成跨端互验 fast / release 拆分：新增 `watermark:cross-end-fast` 和 `watermark:cross-end-release`，原 `watermark:cross-end-contract` 保持为 release 别名，CI 不降级；fast 保留 fixture、图片和 WAV core，release 覆盖非 WAV 和桌面转码完整矩阵。 | 进行中 |
| 2026-06-20 | 推进 Phase I-6 视频一致性与商业化边界：L1 视频音轨水印新增真实 MP4 成品抽音轨回读测试，验证桌面视频 pipeline 通过 `AudioProtectionMode::VideoTrack` 复用 `watermark-core` 且可从成品视频抽出同一版权编号；新增 `watermark:video-phase-contract` 固定 L1 / L2 / L3 分层、L2 三层摘要 / notary / 同步 / 报告隐私边界、L3 未来能力和商业化扣费约束。验证：L1 cargo 测试、`npm run watermark:video-phase-contract`、`npm run watermark:cross-end-fast`、`npm run dual:contract` 通过。下一步补 L3 `watermark-core` 视频画面算法与云端策略包装设计文档。 | 进行中 |
| 2026-06-20 | 完成 L3 `watermark-core` 视频画面算法与云端策略包装设计文档：新增 `docs/Phase I-6 L3视频画面盲水印同核与云端策略设计.md`，冻结 L3 仍为未实现未来能力，算法 API 必须位于 `watermark-core`，云端只能做策略生成、密钥托管、任务调度、权益和额度账本、签名与自检编排；文档同时固定一次性策略包、防逆向、密钥边界、成品自检、云端验证、隐私同步和成功后扣费规则，并纳入 `watermark:video-phase-contract`。下一步在 `watermark-core` 内做最小视频帧模型与策略结构 spike，不接 UI、不接云端任务。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` 最小视频视觉契约 spike：核心库新增视频帧平面、feature bundle、payload build input、策略结构、自检结果、`video_strategy_v1` 和 L3 错误码，合成帧测试覆盖策略确定性和自检结构；`watermark:video-phase-contract` 已升级为检查 core 中存在这些契约。下一步在 core 内补合成帧 embed/extract roundtrip，仍不接桌面 UI、移动端或云端任务。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` 合成帧最小写入 / 提取闭环：Luma8 合成帧可按 `VideoVisualStrategy` 写入并读回正式 payload，错误路径继续输出 `strategy_invalid` / `visual_extract_failed`。该能力仍只用于 core fixture，不进入桌面端、移动端、后端或云端任务。下一步扩展为多帧冗余和提取驱动的自检 confidence。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` 多帧冗余与提取驱动自检：核心库可按同一 `VideoVisualStrategy` 写入多帧合成帧、从有效帧提取 payload，并按实际成功提取帧比例计算 self-check confidence；低于阈值返回 `self_check_failed`。该能力仍只用于 core fixture，不进入桌面端、移动端、后端或云端任务。下一步补 core 内扰动鲁棒性 fixture。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` 基础鲁棒性和性能基线：core 合成帧测试覆盖帧缺失、亮度偏移、本地擦除检测，并新增 12 帧 192x108 合成 roundtrip 的宽松性能基线。该能力仍只用于 core fixture，不进入桌面端、移动端、后端或云端任务。下一步补缩放 / 裁剪 / 压缩模拟。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` 裁剪 / 压缩模拟和分层性能预算：core 合成帧测试覆盖边缘裁剪保留策略区域、保真量化压缩、破坏性量化压缩；性能预算扩展为 4 帧、12 帧、24 帧三档。该能力仍只用于 core fixture，不进入桌面端、移动端、后端或云端任务。下一步形成真实鲁棒画面算法设计。 | 已完成 |
| 2026-06-20 | 完成 L3 真实鲁棒画面算法设计冻结：新增 `docs/Phase I-6 L3真实鲁棒画面盲水印算法设计.md`，明确首版真实算法为 Y 平面 8x8 DCT 中频系数相对关系写入，sync marker、ECC、复杂度预算和失败归因都必须在 `watermark-core`；synthetic LSB spike 只保留为 API / 性能门禁。下一步只做 core 内 `LumaDctMidBandV1` profile 和 DCT block 单测。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` DCT block 单测：新增 `LumaDctMidBandV1` profile、8x8 DCT forward / inverse helper 和中频系数 pair 写入 / 读取 true/false bit 测试；该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补 sync marker / ECC block fixture。 | 已完成 |
| 2026-06-20 | 完成 L3 `watermark-core` sync / ECC 与 DCT 帧级 fixture：新增 `sync_marker_v1`、轻量 ECC repeat bitstream、DCT bitstream block helper 和 `LumaDctMidBandV1` luma 帧级 roundtrip；正式 payload 可经 sync / ECC 后写入 8x8 DCT block 并从同一帧读回。该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补 DCT 多帧冗余、自检 confidence、扰动和性能基线。 | 已完成 |
| 2026-06-21 | 完成 L3 `watermark-core` DCT 多帧自检与性能基线：新增 DCT 多帧写入 / 提取 / 自检 helper，confidence 由实际读回正式 payload 的帧比例计算；测试覆盖完整 roundtrip、缺帧容忍、策略块擦除失败和 4 帧 512x512 性能基线；同一 8x8 block 内多个 coefficient pair 合并为一次 DCT / IDCT，避免按 bit 重复变换。该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补 DCT 频域扰动矩阵和复杂度预算。 | 已完成 |
| 2026-06-21 | 完成 L3 `watermark-core` DCT 频域扰动矩阵：新增统一亮度偏移、保守量化压缩和 2x 下采样再最近邻上采样测试；亮度 / 量化必须通过 DCT 自检，重采样当前必须返回 `self_check_failed`，明确算法还不能宣称抗缩放。该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补更细复杂度预算、帧抽样策略和真实视频帧解码边界。 | 已完成 |
| 2026-06-21 | 完成 L3 `watermark-core` core 复杂度预算和帧抽样策略：新增 `VideoVisualComplexityTier` / `VideoVisualComplexityBudget`、三档 staged 预算和确定性均匀抽帧函数；Small / Standard / High 分别固定为 4 / 8 / 12 采样帧、512 / 768 / 1024 candidate blocks 和 1.5s / 3s / 6s fixture 上限。该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补真实视频帧解码边界和固定 Y-plane fixture。 | 已完成 |
| 2026-06-21 | 完成 L3 `watermark-core` decoded Y-plane 边界：新增 `DecodedVideoLumaPlane`、`VideoLumaBitDepth`、`VideoLumaColorRange` 和 `video_frame_plane_from_decoded_luma`，固定 8/10/12-bit full/limited range 到 8-bit Y 的归一化、stride padding 丢弃、短 buffer 拒绝和非 DCT profile 拒绝；固定 10-bit limited Y-plane fixture 可完成 DCT payload roundtrip。该能力仍只用于 core 内部 staged API，不进入桌面端、移动端、后端或云端任务。下一步补真实视频容器解码到固定 Y-plane fixture 的测试边界。 | 已完成 |
| 2026-06-21 | 完成 L3 真实容器解码到 core Y-plane fixture：桌面 Tauri 测试 `l3_decoded_video_y_plane_fixture_enters_watermark_core` 使用 FFmpeg 生成受控 10-bit MP4，解码第一帧为 `gray10le` raw Y plane，并交给 `watermark-core::video_frame_plane_from_decoded_luma`；`watermark:cross-end-contract` release 分组已纳入该测试。该能力仍只用于测试层 staged fixture，不进入桌面端、移动端、后端或云端任务。下一步补真实容器解码出的 Y-plane fixture 到 DCT staged roundtrip 的测试桥。 | 已完成 |
| 2026-06-21 | 完成 L3 真实容器解码到 DCT staged roundtrip 测试桥：`watermark-core` 导出 DCT staged 写入 / 提取 / 自检 API，桌面 Tauri 测试 `l3_decoded_video_y_plane_fixture_roundtrips_dct_in_watermark_core` 使用 FFmpeg 生成 4 帧 10-bit MP4，解码为 `gray10le` Y plane 后只调用 core API 完成正式 payload 写入、读回和自检；`watermark:cross-end-contract` release 分组已纳入该测试。该能力仍只用于测试层 staged fixture，不进入桌面端、移动端、后端或云端任务。下一步补受控编码回写后的 DCT 自检门禁。 | 已完成 |
| 2026-06-21 | 完成 L3 受控编码回写后自检门禁：桌面 Tauri 测试 `l3_encoded_video_y_plane_fixture_self_checks_after_ffmpeg_roundtrip` 将写入后的 Y plane 经 FFmpeg `libx264 -crf 0` 编码为受控 MP4，再解码为 `gray10le` 后只调用 `watermark-core` staged API 提取和自检；当前基线为 4 帧中 3 帧读回、confidence 达到 0.75 阈值。该能力仍只用于测试层 staged fixture，不进入桌面端、移动端、后端或云端任务。下一步补受控有损压缩矩阵和失败归因。 | 已完成 |
| 2026-06-21 | 完成 L3 有损压缩失败边界：桌面 Tauri 测试 `l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary` 将同一写入 Y plane 经 FFmpeg CRF 12 和 CRF 38 编码回写后交给 `watermark-core` staged API 自检，两档都必须返回 `self_check_failed`；当前 staged 算法不能宣称抗有损二压。该能力仍只用于测试层 staged fixture，不进入桌面端、移动端、后端或云端任务。下一步先在 core 内提高有损压缩存活率，再补目标平台二压矩阵。 | 已完成 |
| 2026-06-21 | 完成 L3 `watermark-core` 多帧融合提取：DCT staged 提取在单帧逐一解码失败后，会在核心层收集同一策略的多帧 bitstream 并按位投票，再复用既有 sync / ECC / payload 解码；新增测试 `video_visual_dct_mid_band_multiframe_fuses_corrupted_payload_streams` 固定“单帧各自损坏但多帧融合可恢复”的边界，同时自检仍按单帧命中比例计算 confidence，避免把缺帧或擦除误判为满置信度。CRF 12 / CRF 38 仍返回 `self_check_failed`，不宣称抗有损二压，不接 UI、移动端、后端或云端任务。下一步继续只在 `watermark-core` 内提升真实有损压缩存活率。 | 已完成 |
| 2026-06-21 | 完成 L3 中等有损压缩存活边界：`watermark-core` 新增同步头汉明距离容错、DCT 写入强度常量和帧内最多 3 份 bitstream 重复副本；桌面 Tauri 测试 `l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary` 已升级为 CRF 12 必须通过自检、CRF 38 必须返回 `self_check_failed`。该能力仍只用于 core / 测试层 staged fixture，不进入桌面端、移动端、后端或云端任务，不上传用户视频、不扣 `video_minutes`。下一步补目标平台二压矩阵。 | 已完成 |
| 2026-06-21 | 完成 L3 目标平台二压矩阵首版：新增 Tauri release 测试 `l3_target_platform_transcode_matrix_classifies_dct_survival`，同一写入 Y plane 经 H.264 CRF 18 / CRF 23 二压后必须通过 `watermark-core` DCT 自检；CRF 38 必须返回 `self_check_failed`。该能力仍只用于 core / 测试层 staged fixture，不进入产品入口、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步只在 core 内处理缩放后二压失败。 | 已完成 |
| 2026-06-21 | 完成 L3 384p 缩放后二压存活：`watermark-core` 将 staged DCT 默认 coefficient pair 下移到更低 AC 频段，`l3_target_platform_transcode_matrix_classifies_dct_survival` 已固定 384p 缩放再回 512p 后 CRF 18 二压必须通过自检；CRF 38 仍保持 `self_check_failed`。该能力仍只用于 core / 测试层 staged fixture，不进入产品入口、不开放云端任务、不上传用户视频、不扣 `video_minutes`。512p 以下只保留为算法诊断小 fixture，不再作为商业主线继续发力。 | 已完成 |
| 2026-06-21 | 完成 L3 主战场分辨率矩阵：新增 `l3_main_resolution_transcode_matrix_covers_720p_1080p_2k`，720p / 1080p / 2K 三档短视频 fixture 经 H.264 CRF 23 / CRF 28 二压后都必须通过 `watermark-core` DCT 自检；三档中心裁切后补边再 CRF 23 二压也必须通过自检。512p 仅保留为快速小 fixture，不再作为商业主线。4K / 8K 暂记录为未来大型商业片、院线产品或高阶商业产品线，不进入当前默认 release 门禁。下一步补 720p / 1080p / 2K 主流码率地板、平台 profile 和成本 / 性能预算。 | 已完成 |
| 2026-06-21 | 完成 L3 首版平台 profile 矩阵：新增 `l3_main_resolution_platform_profiles_cover_720p_1080p_2k`，抖音 9:16 H.264 High CRF18 覆盖 720p / 1080p，小红书 3:4 H.264 High CRF17 覆盖 720p / 1080p，B站 16:9 H.264 High CRF18 覆盖 720p / 1080p / 2K；全部经过 FFmpeg 编码 / 解码后由 `watermark-core` DCT 自检。抖音 / 小红书 2K 竖屏不进入当前门禁，因为现有平台参数目标仍是 1080 级竖屏。下一步补主流码率地板和平台矩阵耗时预算。 | 已完成 |
| 2026-06-21 | 完成 L3 主流码率地板矩阵：新增 `l3_mainstream_bitrate_floor_matrix_covers_720p_1080p_2k`，720p H.264 2.5Mbps、1080p H.264 4.5Mbps、2K H.264 8Mbps 三档必须通过 `watermark-core` DCT 自检；低于这些地板的码率只记录风险边界，不作为当前算法优化目标。后续继续补策略密度预算和平台矩阵耗时预算。 | 已完成 |
| 2026-06-21 | 完成 L3 30 秒商业采样性能矩阵：新增 `l3_30s_commercial_sampling_performance_records_cost_breakdown`，生成 30 秒 30fps 源视频并抽 12 帧进入 staged DCT 流程，分段打印 FFmpeg 源生成 / 抽样、core 写入、采样帧码率回写、core 自检和总耗时；720p 2.5Mbps、1080p 4.5Mbps、2K 8Mbps 在 12 个采样帧 / 96 个策略区域下均通过自检。后续继续补策略密度预算和平台矩阵耗时预算。 | 已完成 |
| 2026-06-21 | 完成 L3 B站 HEVC 主流码率地板矩阵：新增 `l3_bilibili_hevc_mainstream_floor_records_cost_breakdown`，测试先探测 `libx265`，再以 30 秒 / 12 采样帧 / 96 策略区域口径验证 B站 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps；当前本机两档均通过，confidence 1.000。后续继续补策略密度预算和平台矩阵耗时预算。 | 已完成 |
| 2026-06-21 | 完成 L3 B站 H.264 / HEVC 成本对照矩阵：新增 `l3_bilibili_h264_hevc_cost_comparison_records_budget`，同一 30 秒 / 12 采样帧 / 96 策略区域口径下对照 B站 1080p H.264 4.5Mbps、1080p HEVC 4Mbps、2K H.264 8Mbps、2K HEVC 6.5Mbps；本机实测总耗时约 27.5s、28.4s、42.9s、44.2s，confidence 分别为 0.917、1.000、0.750、1.000。该矩阵仍只属于 `watermark-core` / Tauri 测试层 staged 能力。下一步补策略密度预算，优先复核 2K H.264 压线风险。 | 已完成 |
| 2026-06-21 | 完成 L3 2K H.264 策略密度预算矩阵：新增 `l3_2k_h264_strategy_density_budget_records_confidence_curve`，同一 30 秒 / 12 采样帧 / 2K H.264 8Mbps 口径下对照 96 / 128 / 160 策略区域；本机实测总耗时约 43.4s、43.7s、43.0s，confidence 分别为 0.917、0.833、0.833。该矩阵仍只属于 `watermark-core` / Tauri 测试层 staged 能力。下一步补平台矩阵耗时预算，并转向抽帧数量 / 区域质量预算，不继续盲目加策略区域数。 | 已完成 |
| 2026-06-21 | 完成 L3 2K H.264 抽帧数量预算矩阵：新增 `l3_2k_h264_sample_count_budget_records_confidence_curve`，同一 30 秒 / 2K H.264 8Mbps / 96 策略区域口径下对照 12 / 16 / 20 采样帧；本机实测总耗时约 43.5s、51.4s、59.4s，confidence 分别为 0.750、0.812、0.800。该矩阵仍只属于 `watermark-core` / Tauri 测试层 staged 能力。下一步补平台矩阵耗时预算，并评估区域质量预算；16 帧暂作为 2K H.264 候选预算点。 | 已完成 |
| 2026-06-21 | 完成 L3 2K H.264 区域质量预算矩阵：`watermark-core` 新增 `VideoVisualRegionSelectionMode` 和 `derive_video_visual_strategy_with_region_selection`，默认 `SeededRandom` 保持现有行为；Tauri 新增 `l3_2k_h264_region_quality_budget_records_confidence_curve`，同一 30 秒 / 2K H.264 8Mbps / 16 采样帧 / 96 策略区域口径下对照 seeded random、center safe grid 和 distributed grid。本机实测总耗时约 54.0s、51.9s、51.6s；seeded random 通过且 confidence 0.875，center safe grid 和 distributed grid 均 `self_check_failed`。下一步补平台矩阵耗时预算，区域质量后续转向内容感知 / 纹理感知候选。 | 已完成 |
| 2026-06-21 | 完成 L3 平台矩阵耗时预算：新增 `l3_platform_timing_budget_records_16frame_seeded_costs`，同一 30 秒 / 16 采样帧 / 96 策略区域 / seeded random 口径下覆盖抖音 1080x1920 H.264 4.5Mbps、小红书 1080x1440 H.264 6Mbps、B站 1920x1080 H.264 6Mbps 和 B站 2560x1440 H.264 8Mbps；本机实测总耗时约 33.5s、24.9s、33.5s、51.5s，confidence 分别为 0.812、0.875、1.000、0.938。该矩阵仍只属于 `watermark-core` / Tauri 测试层 staged 能力，不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步建立 L3 30 秒平台成本模型，并评估内容感知 / 纹理感知区域候选。 | 已完成 |
| 2026-06-21 | 完成 L3 30 秒平台成本模型：新增 `docs/Phase I-6 L3平台成本模型.md`，将平台耗时预算转成内部 `l3_cost_units`、`platform_weight` 和 `strategy_weight`；首版 1080p H.264 平台权重 1.25、2K H.264 平台权重 2.00、16 帧 / 96 区域 / seeded random 策略权重 1.00。该模型只用于容量规划、定价测算和套餐边界设计，不接双端 UI、不开放云端任务、不进入后端账本、不上传用户视频、不扣 `video_minutes`。下一步评估内容感知 / 纹理感知区域候选。 | 已完成 |
| 2026-06-21 | 完成 L3 TextureAware 核心候选：`watermark-core` 新增 `VideoVisualTextureHint` 和 `TextureAware` 区域选择模式，纹理评分、候选排序和策略区域派生全部在核心内完成，平台层只选择模式；默认 `SeededRandom` 策略 seed 不漂移。2K H.264 区域质量矩阵新增 texture-aware case，本机实测 30 秒 / 16 帧 / 96 区域下总耗时约 55.6s，confidence 1.000。该候选仍只属于 `watermark-core` / Tauri 测试层 staged 能力，不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步把 TextureAware 扩展到平台耗时矩阵。 | 已完成 |
| 2026-06-21 | 完成 L3 TextureAware 完整平台耗时矩阵：平台耗时测试新增 texture-aware 四档对照，抖音 1080p、小红书 1080p、B站 1080p、B站 2K 均通过且 confidence 1.000，总耗时约 33.0s、26.5s、33.9s、55.8s；成本模型将 TextureAware `strategy_weight` 暂定 1.00。该候选仍只属于 `watermark-core` / Tauri 测试层 staged 能力，不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步补 HEVC 对照并评估是否切 staged 默认策略。 | 已完成 |
| 2026-06-21 | 完成 L3 HEVC TextureAware 对照矩阵：新增 `l3_bilibili_hevc_texture_aware_records_cost_budget`，同一 30 秒 / 16 采样帧 / 96 策略区域 / TextureAware 口径下验证 B站 1080p HEVC 4Mbps 与 2K HEVC 6.5Mbps；本机实测两档均通过且 confidence 1.000，总耗时约 35.1s、57.7s。该候选仍只属于 `watermark-core` / Tauri 测试层 staged 能力，不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步评估是否切 staged 默认策略，并补切换后的 H.264 / HEVC 回归矩阵。 | 已完成 |
| 2026-06-21 | 完成 L3 默认 TranscodeStable 策略切换回归矩阵：`watermark-core` 默认策略改为 720p 保守预算，1080p / 2K 默认 TranscodeStable；新增 `l3_default_transcode_stable_h264_hevc_regression_records_cost_budget`，真实 FFmpeg 覆盖 720p H.264、1080p H.264、2K H.264、1080p HEVC、2K HEVC；在 TranscodeStable 确定性取点收紧后五档均通过，confidence 均为 1.000。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步扩默认 TranscodeStable 真实内容二压样本。 | 已完成 |
| 2026-06-21 | 完成 L3 默认策略真实素材多样性回归矩阵：新增 `l3_default_strategy_texture_diversity_records_cost_budget`，受控 FFmpeg 源覆盖 1080p 低纹理网格、1080p 高细节横屏、1080p 高细节竖屏和 2K 低纹理网格，四档真实 H.264 编码 / 解码后均通过，confidence 分别为 1.000、1.000、0.938、1.000。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步补真实素材风险边界矩阵。 | 已完成 |
| 2026-06-21 | 完成 L3 默认 TranscodeStable 后真实素材风险边界矩阵：`l3_default_strategy_real_content_risk_boundary_records_outcomes` 中低码率竖屏高细节通过但 confidence 0.875，极端程序化高频纹理和逐帧随机噪声均稳定归因为 `self_check_failed`。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步补默认 TranscodeStable 平台二压回归。 | 已完成 |
| 2026-06-21 | 完成 L3 平台二压风险矩阵：新增 `l3_platform_second_pass_transcode_risk_records_outcomes`，同一 30 秒 / 16 采样帧 / 96 区域口径下验证真实二次平台转码；1080p 竖屏高细节 6Mbps 再二压到 4.5Mbps 稳定 `self_check_failed`，2K 8Mbps 再二压到 6.5Mbps 压线通过 `passed:0.750`。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步只在 `watermark-core` / 测试层设计二压稳定性改进。 | 已完成 |
| 2026-06-21 | 完成 L3 平台二压稳定性诊断矩阵：新增 `l3_platform_second_pass_stability_diagnostics_records_budget_curve`，1080p 竖屏高细节 20 帧 / 96 区域和 16 帧 / 128 区域均仍失败；新增 `TranscodeStable` 核心区域模式后，1080p 16 帧 / 96 区域恢复到 `passed:0.812`，2K 20 帧 / 96 区域提升到 `passed:0.950`。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步扩展 `TranscodeStable` 平台矩阵并复核 2K 20 帧成本权重。 | 已完成 |
| 2026-06-21 | 完成 L3 TranscodeStable 平台泛化矩阵：新增 `l3_transcode_stable_second_pass_platform_matrix_records_generalization`，同一 30 秒 / 16 帧 / 96 区域口径下固定 720p 真实二压失败边界，并验证 1080p H.264、2K H.264、1080p HEVC 和 2K HEVC 二压；720p H.264 4Mbps -> 3Mbps 仍为 `self_check_failed`，其余四档通过；在稳定候选确定性取点收紧后 confidence 分别为 1.000、0.875、1.000、1.000。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。该证据已支撑 1080p / 2K staged 默认策略切到 TranscodeStable；下一步扩默认 TranscodeStable 真实内容二压样本，720p 二压保持风险边界。 | 已完成 |
| 2026-06-21 | 完成 L3 默认 TranscodeStable 平台二压成本权重复核：新增 `l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight`，使用 core default 路径验证 720p 风险边界和 1080p / 2K H.264 / HEVC 二压；首次运行暴露 1080p H.264 因 seed 抽样漂移失败，随后 `watermark-core` 将 TranscodeStable 区域选择收紧为稳定候选确定性取点。重跑后 720p 仍 `self_check_failed`，1080p H.264、2K H.264、1080p HEVC、2K HEVC confidence 分别为 1.000、0.875、1.000、1.000。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步扩默认 TranscodeStable 真实内容二压样本。 | 已完成 |
| 2026-06-21 | 完成 L3 默认 TranscodeStable 真实内容二压矩阵：新增 `l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes`，覆盖 1080p 高细节横屏 / 竖屏、2K 常规纹理和 2K 高细节 H.264；1080p 两档均 `passed:1.000`，2K 常规纹理 `passed:0.875`，2K 高细节稳定 `failed:self_check_failed`。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步只评估 2K 高细节 H.264 二压预算策略。 | 已完成 |
| 2026-06-21 | 完成 L3 2K 高细节 H.264 二压预算策略矩阵：新增 `l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes`，同一 30 秒 / 2K 高细节 H.264 源下对照 20 帧 / 96 区域、16 帧 / 128 区域和 10Mbps -> 8Mbps；加帧、加区域两档仍 `self_check_failed`，10Mbps -> 8Mbps 通过但 confidence 0.875。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步扩展 2K 高细节高码率候选样本，覆盖横屏高细节、低纹理、运动纹理和 HEVC 对照。 | 已完成 |
| 2026-06-21 | 完成 L3 2K 高码率内容候选矩阵：新增 `l3_2k_high_bitrate_content_candidate_matrix_records_outcomes`，同一 30 秒 / 16 帧 / 96 区域 / core default 口径覆盖 H.264 高细节、H.264 低纹理、H.264 运动纹理和 HEVC 高细节；H.264 高细节 10Mbps -> 8Mbps 通过但 confidence 0.875，H.264 低纹理和运动纹理均 `passed:1.000`，HEVC 高细节 8Mbps -> 6.5Mbps `passed:1.000`。该能力仍不接桌面 UI、不接移动端 UI、不开放云端任务、不上传用户视频、不扣 `video_minutes`。下一步只设计 2K 高码率 release 样本池和阈值策略。 | 已完成 |
| 2026-06-21 | 新增 `docs/当前真实能力边界说明.md` 作为双端能力承诺边界：双端能力表述必须先归类为“可对用户承诺 / 只能内部测试 / 明确不能承诺”，并同步回写该文档；L3 视频画面盲水印 staged 证据不得写成桌面端、移动端或云端已开放能力。下一步继续只设计 2K 高码率 release 样本池和阈值策略，不接 UI。 | 已完成 |
| 2026-06-21 | 完成 2K 高码率 release 样本池与阈值策略冻结：新增 `docs/Phase I-6 L3 2K高码率release样本池与阈值策略.md`，明确 24 个 2K 样本池、H.264 / HEVC 最低 confidence、失败归因和禁止商业包装门槛。该策略仍只用于 `watermark-core` / Tauri release 测试层，不能写成桌面端、移动端或云端已开放能力。下一步新增 `l3_2k_high_bitrate_release_sample_pool_records_thresholds` 门禁。 | 已完成 |
| 2026-06-22 | 完成 2K 高码率 release 样本池门禁：新增 `l3_2k_high_bitrate_release_sample_pool_records_thresholds`，默认 smoke 每个分组跑 1 个代表样本，完整 24 样本池需显式设置 `HIDDENSHIELD_L3_FULL_RELEASE_POOL=1`。本机 smoke 显示 H.264 高细节仍为 confidence 0.875，release 继续阻断；该结果不得写成桌面端、移动端或云端已开放视频画面盲水印能力。下一步跑完整 24 样本池并回写证据。 | 已完成 |
| 2026-06-22 | 根据商业发布判断冻结短期 L3 主线：新增 `docs/双端现有能力发布计划.md`，本版发布聚焦图片 / 音频同核写入验证、移动端保护副本出口、版权库 / 报告 / 云同步 / 本地批量、L1 视频音轨水印和 L2 视频指纹存证；L3 完整样本池长跑、UI、云任务和扣费全部后置。下一步执行发布候选自动化门禁和双端运行态验收。 | 进行中 |
| 2026-06-24 | 封版前补强共享水印架构门禁：`watermark:architecture-contract` 现在明确检查后端不得新增 `watermark-core` 外的写入 / 读取 / payload / 版权编号算法，Web preview 结果不得进入正式版权库、云同步或正式报告，且 Web preview 编号改为 `PREVIEW-...`，避免被误认为正式 `HS-...` 版权编号。L3 视频画面盲水印继续挂起，不作为本版发布任务。下一步执行发布候选自动化门禁和运行态验收。 | 进行中 |
| 2026-06-25 | 进入封版收口主线：新增 `docs/封版收口计划.md`，后续双端一致性工作只围绕当前可承诺能力做门禁复跑、运行态 QA、跨端互验、文案边界和阻断修复；不新增移动端或桌面端单边产品承诺。下一步先复跑发布门禁，再补齐 Android / iOS 原生运行态 QA。 | 进行中 |
| 2026-06-25 | 将双端保护副本互解升级为封版硬门槛：桌面端写入 / 加密的图片、音频保护副本必须被原生移动端读取 / 验证 / 解密，原生移动端写入 / 加密的图片、音频保护副本也必须被桌面端读取 / 验证 / 解密；通过标准是同一版权编号和 payload。下一步复跑 `watermark:cross-end-release`，并在真机 QA 中验证真实保护副本文件流转。 | 进行中 |
| 2026-06-25 | 完成封版双端自动化门禁复跑：`npm run dual:contract`、`npm run watermark:architecture-contract`、`npm run watermark:video-phase-contract`、`npm run watermark:cross-end-release` 和 `npm run commercial:ci` 均通过；跨端 release 门禁覆盖图片、WAV、MP3、FLAC、OGG、M4A、桌面 FFmpeg 音频 fixture 和 L1 视频音轨，L3 分组按冻结策略跳过。移动端视频入口已收口为同步只读的视频指纹存证，不开放本地或云端视频画面水印。下一步必须用 Android / iOS 真机或模拟器验证真实保护副本 desktop->mobile 与 mobile->desktop 文件流转。 | 进行中 |
| 2026-06-25 | 修复双端存证摘要与版权库时间展示的时区口径：桌面端与移动端把处理时间、入库时间、导出时间改为本地可读时间，同时保留可信时间的原始 GMT / ISO 回执用于取证材料；用户可见摘要不再直接暴露 UTC ISO 字符串。验证：待跑 `npm run build` 与 Flutter 测试。下一步复核桌面 / 移动端报告与版权库截图，确认本地时间展示一致。 | 进行中 |
| 2026-06-25 | 将图片写前预检收敛为共享核心快检：桌面端和移动端都不再用完整验证提取去判断“是否已有水印”，而是改为调用 `watermark-core` 的快速已有水印检测；桌面端因此不再被 `正在检查版权记录...` 的重路径长期阻塞，移动端图片预检也与桌面保持同一条快路径。下一步继续复跑跨端 release 门禁并核对图片 / 音频双向互解。 | 进行中 |
| 2026-06-26 | 收紧图片 / 音频写前预检的信息架构：桌面和移动端预检卡不再常驻铺开上一版编号、版本次数和完整证据说明，改为“主提示 + 动作提示 + 折叠详情”的层级；桌面端同时移除了“预检进行中先启动写入”的旧放行分支，确保加载态期间不再误导用户。验证：`npm run dual:contract`、`npm run tauri:build`、`flutter test test/rewrite_preflight_test.dart test/widget_test.dart`、`cargo test --manifest-path src-tauri\\Cargo.toml rewrite_preflight_maps_plain_no_valid_watermark_to_first_write -- --nocapture` 和 `cargo test --manifest-path src-tauri\\Cargo.toml rewrite_preflight_keeps_parent_uid_and_increments_revision -- --nocapture` 均通过；Windows NSIS 安装包已覆盖安装到 `D:\TestInstall\HiddenShield`。下一步在安装版桌面应用里分别打开未加水印与已加水印图片，确认折叠详情默认收起且展开后才出现证据字段。 | 已完成 |
| 2026-06-26 | 调整工作台视频能力可见性：视频能力从单一卡片改为 L1 视频音轨水印 + L2 视频指纹存证双层展示，避免 Free 用户误以为视频能力不存在，同时让 Creator 门禁更明确；同时把视频样本验证文案收束为“视频音轨中的 L1”语言，避免把 `.mp4` 误引导成纯音频样本。下一步复跑桌面验证页和工作台的 UI / 合同测试。 | 进行中 |
| 2026-06-26 | 推进双端视觉语言迁移核心可见层：桌面端和移动端统一采用 Stitch 深色视觉 token、8px 卡片、teal / copper 状态色和“批量队列”入口命名；桌面工作台补当前权益状态并接通 L2 存证订阅入口，移动工作台补当前权益 / 批量队列 / 创作者身份概览。能力边界保持不变，不新增 L3 或云端视频承诺。验证：`npm run build`、`flutter analyze` 通过。下一步继续做设置、帮助、版权库详情和报告购买态截图验收。 | 进行中 |
| 2026-06-26 | 继续推进双端视觉语言迁移边缘可见层：桌面端全局基础样式、设置、帮助、订阅、身份初始化、隐私授权、更新条、AI 内容标记、结果页、批量队列、验证状态卡和版权库状态徽标均改为 Stitch 深色 token 与 8px border-based 视觉；移动端设置同步健康卡和版权库详情 sheet 去除剩余硬编码视觉。能力与门禁保持原样，不新增桌面端或移动端单边承诺，不改变 L1 / L2 / L3 边界。验证：`npm run build`、`flutter analyze` 通过；产品代码模板词、emoji、移动端白色硬编码扫描通过；桌面端 Playwright 运行态截图覆盖首启、主壳子、批量、版权库、验证、设置、帮助和订阅。下一步继续移动端运行态截图 QA，并重点核对报告购买 / 导出、订阅支付状态、版权库详情和身份初始化。 | 进行中 |
| 2026-06-26 | 完成移动端商业状态运行态截图 QA：QA-only 入口 `mobile_app/tool/mobile_visual_qa.dart` 覆盖 Free 未购买、Free 已购买授权、Creator 订阅、支付通道未配置、退款撤销五态，并复用正式移动端主题 token。五态均保留双端一致的 Free / Creator / Studio / Enterprise、正式报告、单份版权详细报告、维权证据包、支付通道未配置和退款撤销口径；L2 / L3 边界继续显示为“L2 不是视频画面盲水印，L3 本地或云端视频画面盲水印不开放”。截图位于 `tmp-ui-qa/mobile/*-full.png`，像素检测确认非空深色运行态。验证：`flutter analyze`、`flutter build web -t tool/mobile_visual_qa.dart`、`npm run build`、模板词 / emoji / 移动端硬编码视觉扫描通过。下一步补正式移动端真实版权库记录详情交互 QA，并与桌面端同五态截图做并排对照。 | 进行中 |
| 2026-06-26 | 补齐移动端 flow 运行态视觉 QA：`mobile_flow_visual_qa.dart` 改为不导入 FRB 依赖链的 QA-only 纯视觉入口，覆盖工作台、智能处理、图片写入、音频写入、批量队列和验证，验证同一套移动端壳子、底部导航、状态徽标、报告 / 批量 / L1 / L2 / L3 边界文案。新增 `HiddenShieldCjk` 字体子集和主题 fallback，修复 Web QA 中文缺字；QA 编号使用 `PREVIEW-QA-...`，不进入正式版权库、报告或同步证据。验证：`npm run dual:contract`、`npm run commercial:contract`、`npm run report:contract`、`npm run build`、`flutter analyze`、两条移动端 QA Web 构建、移动端 flow / 商业状态截图像素检查和控制台错误检查均通过。下一步仍需 Android / iOS 真机或模拟器完成真实保护副本分享、版权库详情、正式报告草稿和跨端文件流转 QA。 | 进行中 |
| 2026-06-26 | 推进 Stitch 信息架构双端同模型迁移：桌面端新增固定左导航、顶栏、主舞台和右侧上下文面板，一级导航补齐处理、订阅与权益、设置、帮助与能力边界；移动端底部导航收敛为工作台、验证、版权库、批量、设置 5 项，帮助与订阅继续从设置和门禁进入。新增移动端 `HsWorkspaceContext` 与 `ContextSheet`，把桌面右侧上下文转译为底部 sheet；工作台、验证、版权库、批量、设置均接入同模型上下文。能力边界保持不变，不新增单边承诺，不把 L2 写成画面水印，不开放 L3。验证：`npm run dual:contract`、`npm run build`、`flutter analyze`、`flutter build web -t tool/mobile_flow_visual_qa.dart --pwa-strategy=none`、`flutter build web -t tool/mobile_visual_qa.dart --pwa-strategy=none` 通过。下一步用 Android / iOS 真机或模拟器验证 context sheet、版权库详情、正式报告草稿和保护副本分享。 | 进行中 |
| 2026-06-26 | 冻结处理页与版权库字段断舍离实施要求：处理页移除平台画幅 / 裁剪 / 黑边 / 编码模式后，版权库记录、存证摘要、正式报告草稿、同步 payload 和双端详情必须同步迁移到保护副本、输出策略、作品声明与授权策略字段。作品来源、训练许可、创作方式、人工编辑程度、真实性声明和自定义版权声明不得只作为 UI 标记，必须持久化进版权库记录，并由存证摘要从记录读取；`protectedCopyPath` 仅本机存储，不进入同步或正式报告。下一步待用户确认 Phase 2.5 字段模型后，再实施桌面端、移动端、报告和同步白名单迁移。 | 进行中 |
| 2026-06-26 | 完成处理页第一性原则字段迁移实施：桌面端处理页移除平台输出、画幅适配、裁剪 / 黑边和编码模式，移动端与桌面端统一使用保护副本、最小必要变更、作品声明与授权策略模型；桌面 / 移动版权库记录、存证摘要、正式报告摘要、云同步 payload 和远端记录合并均接入 `protectedCopyName`、`protectedCopyHash`、`outputStrategy`、作品来源、训练许可、创作方式、人工编辑程度、真实性声明和自定义版权声明字段；移动端图片 / 音频写入页已补声明折叠面板，用户填写后随正式记录持久化。`protectedCopyPath` 保持仅本机落库，不进入同步或正式报告；旧 `output_*` 和旧 AI 字段仅作历史迁移来源和兼容兜底。验证：`process:first-principles-contract`、`dual:contract`、`report:contract`、`commercial:contract`、`npm run build`、`cargo check --manifest-path src-tauri\\Cargo.toml`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart`、`flutter test test/widget_test.dart` 通过。下一步用桌面安装版与移动端真机各写入一条新记录，核对跨端同步后的详情、摘要和报告字段。 | 已完成 |
| 2026-06-26 | 确认版权编号语义升级：同一作品新版写入也生成新编号；在线默认由后端签发 / 确认唯一，以打通账号、同步、报告、团队版权库和商业化登记；离线或后端不可用时，桌面端和移动端仍可使用 128-bit 级高熵本地编号完成写入，并在联网后补登记；历史重复编号必须进入重新签发和保护副本修复流程；同 UID 不同哈希由后端登记库仲裁，不做普通变体静默合并。下一步实施后端编号 API、双端写入路径和同步登记仲裁。 | 已确认，待实施 |
| 2026-06-26 | 完成 Phase I 的 V2 核心实施：`watermark-core` 固化 `PAYLOAD_BYTES = 119`，正式 identity/media 构造路径使用 128-bit CSPRNG 离线记录身份，图片 / 音频 / 视频视觉容量随 V2-119 更新；移动端 Rust bridge 图片 / 音频跨端互验按 V2 语义通过，桌面 Tauri 编译通过。验证：`cargo test --lib`（watermark-core，77 passed）、`cargo test --all-targets --no-run`（watermark-core）、`cargo test`（mobile_app/rust，27 passed）、`cargo check`（src-tauri）。下一步实施后端 `reserve / confirm / reconcile / reissue`，并把 `watermarkIdIssueMode`、registry status、parent id、revision 和 payload protocol 字段落到双端版权库 / 报告 / 同步。 | 核心已完成，数据层待实施 |
| 2026-06-27 | 完成双端图片 / 音频写入流水线在线优先 `reserve -> confirm`：桌面端 Tauri 写入前请求后端编号并把 `watermarkUid` / `registryProofHash` 写入 V2 payload，写入后确认登记；移动端原生 Rust bridge 支持后端预留编号进入 payload，Flutter 写入页在写入前 reserve、写后 confirm。双端后端不可用时继续离线高熵写入并标记 `pending_registration`；云同步发送前自动对 `reserved` 调用 `confirm`、对 `pending_registration` 调用 `reconcile`，成功后回写本地版权库和同步 payload。验证：`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml --tests`、`flutter analyze`、`flutter test`、`cargo test --manifest-path mobile_app/rust/Cargo.toml --lib` 通过。下一步用真实后端运行桌面端和原生移动端各写入一张图片 / 一段音频，截图核对 `server_confirmed`、离线 `pending_registration`、同步后 `offline_confirmed` 三种状态。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 新增 `docs/双端版权记录字段一致性契约.md`，锁定“桌面端字段 -> 移动端字段 -> 同步 payload -> 报告字段”的字段链路；`dual:contract` 增加双端 payload 白名单同集合检查，以及关键版权记录字段在桌面 / 移动模型、SQLite schema、存储映射、同步发送、同步接收和正式报告中的落点检查；修复桌面端接收移动同步时过滤创作者、可信时间、第三方验证、输出策略和作品声明字段的问题，并用 Rust 同步入库测试确认移动 payload 可完整落到桌面版权库。下一步把真实桌面端与原生移动端各写入一条新记录后做跨端同步运行态 QA，核对详情、摘要和正式报告字段完全一致。 | 已完成 |
| 2026-06-27 | 完成真实后端双向版权字段运行态 QA：新增 `npm run dual:runtime-qa`，脚本会启动 `feedback-backend` 临时 SQLite 后端，分别模拟桌面端和移动端图片写入后的 `watermark-ids reserve -> confirm`、云同步 push/pull，并核对 desktop->mobile 与 mobile->desktop 的版权库详情、复制摘要、正式报告字段。QA 发现复制摘要缺少 `write_verification_at` 的“验证时间”，已同步补到桌面端 `buildCopyrightSummary` 和移动端 `buildCopyrightSummary`，并由 `dual:contract` 固定检查。验证：`npm run dual:runtime-qa`、`npm run dual:contract`、`npm run build`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart` 通过；证据文件：`tmp-ui-qa/dual-runtime/dual-vault-runtime-qa-1782539536860.md`。下一步用安装版桌面应用和 Android / iOS 原生运行态各写入真实图片保护副本，补人工截图验收。 | 已完成 |
| 2026-06-27 | 完成 Creator 默认自动云同步一致性主链路：后端 `auth/continue` 返回 `syncPolicy` 与 `cloudVaultCursor`，Free 同步 push / pull 被 403 阻断；桌面端继续账户后自动 pull / flush / pull，移动端登录或权益升级 Creator 后自动 pull / flush / pull，移动端本地 profile 持久化 `syncPolicy` 且不升 SQLite schema。云同步合同脚本覆盖 Free 阻断、fixture 升级 Creator 和 Creator push / pull；移动端状态测试覆盖 Creator 登录自动同步待队列。验证：后端 lib tests、Tauri sync tests、Flutter sync/state tests、真实临时后端 `npm run cloud:contract`、`flutter analyze`、双端 cargo check 通过。下一步补桌面安装版 + 原生移动端同账号自动同步、暂停、恢复截图 QA。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 完成真实后端 + 安装版桌面端 + 原生 Android 版权登记状态运行态 QA：桌面端和 Android 模拟器均覆盖图片 / 音频各一组 `server_confirmed`、后端不可用离线 `pending_registration`、同步前 reconcile 后 `offline_confirmed` 三态；Android 构建使用本地 Flutter storage mirror 解决 `x86_64_debug` jar 下载阻塞，并修复 APK 打包缺少 `libhidden_shield_mobile_bridge.so` 的 native bridge 问题。证据文件：`tmp-ui-qa/real-runtime-status/real-runtime-status-qa-1782541584143.md`，截图包含桌面 `desktop-runtime-status-1782541584143.png` 与移动端 `mobile-runtime-status-1782541584143-v4*.png`。验证：`npm run dual:contract`、`flutter analyze tool/real_runtime_qa.dart` 通过。下一步执行真实保护副本 desktop->mobile 与 mobile->desktop 文件流转 QA，核对导出、分享、验证、解密后的编号和 payload。 | 已完成 |
| 2026-06-27 | 完成真实保护副本双端文件流转 QA：新增 `npm run dual:protected-copy-file-flow-qa`，用 `watermark-core` 生成桌面 PNG / WAV 保护副本并经 adb 推入 Android app 沙盒，由原生移动端 Flutter + Rust bridge 读取；移动端再生成 PNG / WAV 保护副本，经 `run-as` 从 app 沙盒 pull 回桌面后由 `watermark-core` 读取。desktop->mobile 与 mobile->desktop 的图片 / 音频均读取出同一版权编号和 V2/119 payload。证据文件：`tmp-ui-qa/protected-copy-file-flow/1782552234524/protected-copy-file-flow-qa-1782552234524.md`，截图：`mobile-file-flow-1782552234524.png`、`desktop-file-flow-1782552234524.png`。当前图片 / 音频保护副本没有额外加密 envelope，因此“解密”项记录为 N/A；本轮按真实文件可读取并验证 V2 payload 闭环验收。下一步继续补历史重复编号重新签发 / 修复入口和同步仲裁 UI。 | 已完成 |
| 2026-06-27 | 完成历史重复编号重新签发 / 保护副本修复入口与同步仲裁 UI：双端同 UID 不同作品哈希改为保留记录并标记 `pending_registry_reconcile`，版权库显示登记仲裁入口；桌面端可调用后端 `reissue` 并在保护副本可访问时用 `watermark-core` 重写 V2 payload、回读验证新编号与父编号、更新保护副本摘要；移动端可创建重签任务并把记录落为 `reissue_required / pending_repair`，等待用户重新选择文件完成修复。验证：`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo test --manifest-path src-tauri/Cargo.toml sync::storage::tests --lib`、`cargo test --manifest-path src-tauri/Cargo.toml commands::vault::tests::reissue_payload_keeps_previous_uid_as_parent --lib`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/widget_test.dart`、`npm run build`、`npm run dual:contract`、`cargo test --manifest-path feedback-backend/Cargo.toml watermark_id --lib`、Rust fmt checks 均通过。下一步进入 iOS 真机 / 模拟器运行态 QA 和桌面安装包完整交互回归。 | 已完成 |
| 2026-06-27 | 完成双端自动云同步暂停 / 恢复一致性：后端 `PATCH /v1/me/sync-preferences` 以当前设备为粒度保存 `auto_sync_enabled`，Creator / Studio / Enterprise 可在 `auto_cloud_vault` 与 `manual_local_only` 间切换，Free 恢复返回 403；桌面端设置页和移动端设置开关均接入同一偏好 API。双端都保持“暂停只停止自动 pull / flush，不清空本地队列、不删除云端版权库、不取消手动同步权益”的语义；移动端 SQLite / Web profile 持久化 `syncPolicy`，桌面端 `cloud_sync_profile.json` 持久化同字段。验证：后端 lib tests、Tauri sync tests、Flutter sync/state tests、临时真实后端 `npm run cloud:contract`、`flutter analyze`、双端 cargo check 通过。下一步补桌面安装版 + 原生移动端同账号自动同步、暂停、恢复截图 QA。 | 已完成，运行态 QA 待补 |
| 2026-06-27 | 完成自动云同步暂停 / 恢复运行态 QA：新增 `npm run cloud:auto-sync-runtime-qa`，脚本启动临时真实 `feedback-backend`，同一 Creator 账号下模拟桌面端与移动端设备，验证 Creator 默认 `auto_cloud_vault`、移动端暂停后 `manual_local_only`、暂停期间手动 push / pull 仍可用、恢复后重新 `auto_cloud_vault` 并可继续读取桌面端新记录。证据文件：`tmp-ui-qa/auto-cloud-sync/auto-cloud-sync-runtime-qa-1782566485043.md`；截图：`desktop-auto-cloud-sync-1782566485043.png`、`mobile-auto-cloud-sync-1782566485043.png`。验证：`npm run cloud:auto-sync-runtime-qa` 通过。下一步在真实安装版桌面端和 Android / iOS 原生端进行人工交互复核，并继续正式 Auth API 迁移。 | 已完成 |
| 2026-06-27 | 完成正式 Auth API 双端主入口迁移：后端落地 `auth/challenges -> auth/sessions -> auth/refresh -> auth/logout -> me`，桌面端和移动端继续账户底层改走 `/v1/auth/sessions`，退出时调用 `/v1/auth/logout` 且保留本地版权库和队列；`auth/continue` 仅保留兼容 alias。`auth:contract` 覆盖 challenge 登录、密码登录、refresh 轮换、logout、`me`、Creator 自动同步和暂停状态保持；`cloud:ci` 已用正式 sessions 验证 Free 同步阻断和 Creator desktop -> mobile / mobile -> desktop push / pull。下一步补正式验证码 / 密码登录 UI 和安装版 / 真机交互复核。 | 已完成 |
| 2026-06-27 | 完成双端正式登录体验一致性：桌面端和移动端设置页 / 首次引导页均从“继续账户”升级为验证码 / 密码登录，验证码登录统一调用 `auth/challenges` 后再用 `challengeId + verificationCode` 创建 session；未登录本地使用、Creator 自动云同步、logout 保留本地版权库和队列的语义保持一致。后端同步补 Argon2id、新旧密码迁移、验证码发送限流、登录失败限流和可配置 OTP webhook。验证：后端 lib tests、Tauri check / sync tests、Flutter analyze / state tests、`npm run build`、`auth:contract`、`cloud:ci` 通过。下一步补设备管理 / 会话撤销 UI 与安装版 / 真机交互复核。 | 已完成 |
| 2026-06-27 | 完成双端设备管理 / 会话撤销一致性：后端新增 `GET /v1/devices`、`PATCH /v1/devices/{deviceId}`、`DELETE /v1/devices/{deviceId}`，桌面端和移动端设置页均展示设备列表、当前设备标记、活跃会话数、最近使用时间、重命名和撤销其他设备入口；撤销其他设备后对应 access token / refresh token 失效，本地版权库和同步队列不被删除。`auth:contract` 同时覆盖 OTP webhook 动态验证码投递，不再只验证 fixture。验证：后端 lib tests、Tauri check / sync tests、Flutter analyze / state tests、`npm run build`、`auth:contract`、`cloud:ci` 通过。下一步补安装版桌面端 + Android / iOS 真机设备撤销截图 QA 和生产短信 / 邮件供应商联调。 | 已完成 |
| 2026-06-28 | 完成安装版/运行态设备撤销截图 QA 的 Windows 桌面端 + Android 模拟器部分：桌面端在真实后端下展示设备列表、重命名 Android 设备、撤销 Android 设备；移动端被撤销 session 自动同步失败后展示恢复提示，点击“重新登录”后清除失效 token、保留账号标识并进入验证码 / 密码登录页。QA 同时修复桌面端撤销系统 dialog 权限异常、移动端恢复态不清除旧 token、移动端启动自动同步未触发恢复提示和登录页按钮布局问题。证据：`tmp-ui-qa/device-session-revoke/1782577422719/device-session-revoke-qa.md`。验证：`flutter analyze`、`flutter build apk --debug -t lib/main.dart --target-platform android-x64`、`npm run build`、`cargo check --manifest-path src-tauri/Cargo.toml` 通过。下一步补 iOS 真机同场景截图，并把 OTP 生产供应商签名 / 模板审核纳入上线门禁。 | Android 已完成，iOS 待补 |
| 2026-06-28 | 修复桌面端图片 / 音频写入与验证运行态回归：写入流水线的在线编号登记改为后台安全执行，避免用户只看到进度反馈但任务无法稳定完成；桌面验证层重新对齐 V2 payload 语义，已由 `watermark-core` 成功读取的正式保护副本不再被旧校验逻辑误判为无水印；WAV 保护副本验证改为直接交给共享核心读取，避免二次处理破坏音频水印。技术性错误不再直接进入主提示，用户可见提示继续保持“写入失败 / 无法验证 / 请检查文件或网络后重试”的产品文案，详细原因只进入日志和 QA 证据；已有水印写入前硬阻断语义不变。运行态 QA 通过真实桌面进程写入图片和 35 秒 WAV 音频，均得到 `server_confirmed`、`payloadProtocolVersion=2`、`payloadBytesLength=119`，并由桌面验证命令匹配本机版权库记录。验证：`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo test --manifest-path src-tauri/Cargo.toml v2_payload_decoded_by_core_is_high_confidence -- --nocapture`、`cargo test --manifest-path src-tauri/Cargo.toml wav_protected_copy_verification_reads_core_payload_without_transcoding -- --nocapture`、`cargo test --manifest-path watermark-core/Cargo.toml service_ -- --nocapture`、真实 Tauri IPC 图片 / 音频写入与验证闭环。下一步把同一修复后的安装版桌面端纳入完整交互回归，覆盖图片和音频手动选择路径。 | 已完成 |
| 2026-06-28 | 补齐移动端同类 V2 读取与验证入库缺口：移动端 Rust bridge 已从 `watermark-core` 成功解码的图片 / 音频保护副本中返回父编号、版本次数、payload protocol、payload bytes、编号签发模式、媒体类型和 payload 认证状态；Flutter `WatermarkReadResult`、验证页、验证记录入库和同步 payload 不再把读回记录固定为第 1 次或丢失 V2 字段。WAV 读取路径继续保持原样直交共享核心，非 WAV 才走移动端归一化。设备会话设置页顺手修复全宽按钮导致的测试布局异常。验证：`cargo test --manifest-path mobile_app/rust/Cargo.toml --lib -- --nocapture`、`flutter analyze`、`flutter test test/mobile_app_state_test.dart test/mobile_verification_result_test.dart`、`flutter test test/widget_test.dart` 通过。下一步在 Android / iOS 真机验证移动端读取桌面生成的新版 V2 父链记录，并核对版权库详情与同步 payload 字段。 | 已完成 |
| 2026-06-28 | 复跑移动端 V2 字段透传后的真实保护副本双端文件流转 QA：修复 `mobile_app/tool/file_flow_qa.dart` 对 nullable `mediaType` 的 QA 展示兜底后，`npm run dual:protected-copy-file-flow-qa` 在 Android 模拟器 `emulator-5554` 上通过。desktop->mobile 图片 / 音频均由原生移动端 Flutter + Rust bridge 读取出同一 `watermarkUid`、`revision=1`、`payloadProtocolVersion=2`、`payloadBytesLength=119`、`watermarkIdIssueMode=offline_generated`、`payloadAuthStatus=verified`；mobile->desktop 图片 / 音频 pull 回桌面后也由 `watermark-core` 读回同一编号和 V2/119 payload。证据文件：`tmp-ui-qa/protected-copy-file-flow/1782624239136/protected-copy-file-flow-qa-1782624239136.md`，截图：`mobile-file-flow-1782624239136.png`、`desktop-file-flow-1782624239136.png`。当前图片 / 音频保护副本仍没有额外加密 envelope，“解密”项保持 N/A。下一步补 iOS 真机同场景运行态 QA，并在发布候选安装包中复跑桌面端手动交互回归。 | Android 已完成，iOS 待补 |
| 2026-06-28 | 新增公开权利信号与训练许可扫描协议设计：`docs/公开权利信号与训练许可扫描协议设计.md` 明确不修改 `Watermark Payload Protocol V2`，不扩展 `PAYLOAD_BYTES = 119`，把 V2 定义为 `watermarkUid + registryProofHash + payloadAuthStatus` 的跨端锚点层；完整训练许可、撤销、替代、自定义条款和 C2PA / CAWG / IPTC / XMP / JSON-LD 映射进入后端 rights registry 与公开元数据层。当前仅为设计约束，不新增双端公开扫描能力、企业批量 API 或法律授权判断。下一步评审并冻结协议，再设计 `rights_manifests` 后端模型和公开查询 API。 | 设计已完成，待评审 |
| 2026-06-28 | 响应桌面端手动测试问题并同步复查移动端：LAN 同步端口占用不再显示为云后端故障；桌面 / 移动批量队列只处理 `queued` 项，启动时把上次中断的 `running` 项转为可重试失败，避免混合图片 / 音频队列卡在“待处理 + 写入中”；云同步状态把 `pending > 0` 明确显示为“等待上传，尚未完成云同步”，不再说“同步状态正常”；桌面视频 L1 正式入口改为单一 `video_audio_track` 保护副本，移除抖音 / B站 / 小红书平台压制路径，L2 云端指纹提交改为后台阻塞任务并返回明确成功 / 失败提示；桌面和移动端写入 / 验证结果统一展示处理耗时、验证耗时、保护副本位置或分享出口、payload 协议、编号签发、登记状态和 payload 认证。性能复查确认 core release 基准未退化：QA 图片 release 写入约 356ms、普通提取约 73ms、2% 裁剪深度取证约 2.35s；debug 构建会显著变慢，且此前 TSA / 网络授时实际会等待外部服务，现已增加 3 秒最佳努力上限，避免本地保护副本写入被外部授时拖住。下一步用同一批手测素材复跑安装版桌面端和 Android 原生端，确认真实 UI 耗时。 | 代码已完成，运行态复测待补 |
| 2026-06-28 | 新增感知质量发布门禁设计：`docs/感知质量发布门禁设计.md` 明确先用图片 PSNR / SSIM、音频 SNR / LUFS / 峰值差异、L1 视频音轨音频指标和 L3 staged 视频 VMAF 建立质量门禁，再决定 `Forensic` / `Balanced` 是否进入用户可选策略；当前不修改 V2 payload、不调整算法、不开放 L3、不新增用户可见强度选择。下一步实现 `watermark:quality-gate:fast`，先跑当前 Forensic 默认策略的图片 / 音频质量基线，再纳入 Balanced 对照。 | 设计已完成，工具待实施 |
| 2026-06-29 | 完成公开权利信号第一阶段双端接入：桌面端版权库详情和验证页、移动端版权库详情和验证页均可基于 `watermarkUid` 查询后端公开 rights registry，展示训练许可、扫描状态、锚点协议、manifest 版本 / 待回填状态，并统一提示“创作者声明与 registry 快照，不直接判断是否可训练”。后端新增 `rights_manifests`、公开查询 / 批量查询和管理员保护的内部 backfill；V3 仅作为 registry / 迁移桥语义接入，未修改正式 V2/119 媒体 payload 和 `watermark-core` codec。验证：`npm run build`、`flutter analyze`、后端公开权利 / backfill / V3 目标测试和 `cargo check --manifest-path feedback-backend/Cargo.toml` 通过。下一步补真实后端运行态 QA，使用桌面端与 Android 原生端各写入并同步一条记录后，截图核对公开权利卡片和本地声明一致。 | 阶段性完成，运行态 QA 待补 |
| 2026-06-29 | 完成公开权利信号桌面端 + Android 原生端真实后端运行态 QA：新增 `rights:runtime-qa`，脚本启动临时真实 `feedback-backend`，桌面端模拟图片 / 音频版权库记录的 `reserve -> confirm -> sync -> public rights query`，Android 原生 Flutter 工具通过真实 Rust bridge 写入图片 / 音频保护副本、云同步声明并查询同一公开 rights registry；四条记录的本地训练许可与公开训练许可一致，`scanStatus=registry_active`、`anchorProtocol=v2_migration_anchor`、`manifestVersion=1`、`legalConclusion=false`。证据文件：`tmp-ui-qa/public-rights-runtime/1782707328008/public-rights-runtime-qa-1782707328008.md`，截图同目录。后端同时新增公开元数据 sidecar 导出契约，但未把 C2PA / IPTC 嵌入媒体文件，也未修改 V2/119 payload。下一步把 sidecar 导出接到桌面端 / 移动端导出入口，并补 iOS 同场景运行态 QA。 | Android 已完成，iOS 待补 |
| 2026-06-29 | 完成公开元数据 JSON 双端导出入口：桌面端版权库详情页新增“导出 JSON”，调用 `GET /v1/public/rights/{watermarkUid}/metadata` 下载公开元数据 sidecar；移动端版权库公开权利卡新增“导出公开元数据 JSON”，通过系统分享面板导出同一 JSON。`dual:contract` 已新增双端导出入口合同，固定该能力不得被写成媒体内嵌 C2PA / IPTC。验证：`npm run dual:contract`、`npm run build`、`flutter analyze` 通过。下一步在 macOS + iOS 设备上复跑同场景运行态 QA，确认分享面板和 JSON 文件名。 | 双端入口已完成，iOS 运行态待补 |
| 2026-06-29 | 收口公开元数据 JSON 导出入口的移动端分享细节：移动端导出公开元数据 JSON 现在与保护副本分享一样传入 `fileNameOverrides` 和 `sharePositionOrigin`，降低 iOS / iPad 系统分享面板文件名和弹窗定位风险；`dual:contract` 已固定该要求。当前 Windows 环境 `flutter devices` 仅发现 Android 模拟器、Windows、Chrome、Edge，缺少 macOS / Xcode / iOS Simulator 或真机，因此 iOS 同场景运行态 QA 未执行，阻断记录为 `tmp-ui-qa/public-rights-runtime/ios-public-rights-metadata-qa-20260629-124447.md`。下一步在 macOS + iOS Simulator 或真机执行版权库详情“导出公开元数据 JSON”截图与文件内容核对。 | 双端入口已完成，iOS QA 被环境阻断 |
| 2026-06-29 | 暂时挂起 iOS 公开元数据导出运行态 QA，继续推进不依赖 macOS 的公开扫描 SDK：桌面端新增 `src/lib/public-rights-sdk.ts`，实现 `scanOne`、`scanBatch`、`resolvePolicy`、`formatUserMessage`，并把版权库详情页与验证页的公开权利文案切到 SDK 统一解释；`tauri-api.ts` 补齐 `POST /v1/public/rights/batch` 客户端封装。该 SDK 只解释 registry / manifest / warning 状态，不给出法律授权结论，也不改 V2 payload。下一步抽出移动端 Dart 同构 SDK，让桌面和移动端的错误码与文案继续收敛。 | 桌面 SDK 第一版完成，iOS QA 挂起 |
| 2026-06-29 | 完成移动端 Dart 同构公开扫描 SDK 第一版：新增 `mobile_app/lib/features/public_rights/public_rights_scanner.dart`，提供 `PublicRightsScanner.scanOne`、`resolvePublicRightsPolicy`、`formatPublicRightsUserMessage`，移动端版权库详情页和验证页均切到 SDK 统一解释公开权利状态；补 `public_rights_scanner_test.dart` 固定法律结论恒 false、不可直接当作训练授权和回填 pending 文案。下一步评审外部分发 SDK / 批量限流，并在具备 macOS 条件后恢复 iOS 同场景 QA。 | 双端 SDK 第一版完成，iOS QA 挂起 |
| 2026-06-29 | 完成公开元数据图片嵌入副本的桌面端首版，并同步移动端边界文案：桌面端版权库详情页保留“导出公开元数据 JSON”，新增“导出嵌入元数据图片副本”，先从 registry metadata 生成 PNG `iTXt` / JPEG `APP1` XMP 副本，包含 XMP、IPTC / PLUS JSON-LD 和 C2PA / CAWG 映射，不覆盖原保护副本、不修改 V2 payload；移动端版权库详情页继续只提供 JSON 分享，并明确图片嵌入副本首版在桌面端导出。验证新增 `rights:metadata-embed-contract` 和桌面 Rust 单元测试门禁。下一步补真实桌面端导出运行态 QA，并评估 Android 是否需要原生图片嵌入导出或只保留 JSON 分享。 | 桌面首版完成，移动端保持 JSON |
| 2026-06-29 | 完成桌面端公开元数据嵌入图片副本运行态 QA：新增 `rights:metadata-embed-runtime-qa`，脚本启动真实 `feedback-backend`，走 `reserve -> confirm -> sync -> GET /metadata`，分别生成 PNG / JPEG V2/119 图片保护副本，再复用桌面端嵌入逻辑导出副本并做字节级检查；PNG `iTXt` 与 JPEG `APP1` 均确认包含 `watermarkUid`、`manifestHash`、`legalConclusion=false`。证据：`tmp-ui-qa/public-metadata-embedded-image/1782734289529/public-metadata-embedded-image-qa-1782734289529.md`。下一步评估 Android 是否只维持 JSON 分享，或新增原生图片嵌入副本导出并补 Android 运行态 QA。 | 桌面运行态 QA 已完成 |
| 2026-06-29 | 完成 Android 是否需要原生“图片嵌入元数据副本”导出的评估与实现准备：结论是需要，但必须受保护副本文件字节可用性约束。移动端新增 `public_metadata_embedder.dart`，复用 registry metadata 和同一公开权利语义，支持 PNG `iTXt` 与 JPEG `APP1` 写入，并固定 `legalConclusion=false`；新增 `public_metadata_embedder_test.dart` 与 `rights:metadata-embed-android-runtime-qa`，Android 运行态从真实 Rust bridge 写入 PNG 保护副本、云同步 registry、拉取 `GET /metadata`，再做 PNG / JPEG 字节级检查。由于移动端版权库历史记录只保存保护副本名称 / 摘要，不保存文件字节或路径，版权库详情页仍先保留 JSON 分享和边界文案。下一步设计“重新选择 PNG / JPEG 保护副本并导出嵌入副本”入口，再接正式移动端 UI。 | Android 内核与 QA 门禁完成，UI 待设计 |
| 2026-06-29 | 完成移动端“重新选择 PNG / JPEG 保护副本并导出嵌入元数据图片副本”正式 UI 入口：图片版权库详情页公开权利卡保留“导出公开元数据 JSON”和历史记录无文件路径边界文案，同时新增“导出嵌入元数据图片副本”；入口通过 `FilePicker` 仅选择 PNG / JPG / JPEG，拉取 registry metadata，校验本地 `watermarkUid` 一致，复用 Dart 嵌入器写入 PNG `iTXt` / JPEG `APP1` 后用系统分享面板导出。验证新增 widget 回归和合同锚点。下一步在 Android 真机复跑点击选择文件到分享面板的人工/自动 QA，并在具备 macOS 条件后补 iOS 分享面板场景。 | Android UI 入口完成，运行态点击 QA 待补 |
| 2026-06-29 | 完成 Android 公开元数据图片嵌入副本端到端点击 QA：新增 `rights:metadata-embed-android-click-qa`，在 Android 模拟器打开图片版权库详情 QA 页面，点击“导出嵌入元数据图片副本”，分别对真实 PNG / JPEG 保护副本触发系统分享链路，并 pull 回分享前产物做字节级检查。PNG `iTXt` 与 JPEG `APP1` 均确认包含 `watermarkUid`、`manifestHash`、`legalConclusion=false`。证据：`tmp-ui-qa/android-public-metadata-embed-click/1782739643606/android-public-metadata-embed-click-qa-1782739643606.md`。下一步在 macOS + iOS Simulator 或真机上补同场景分享面板 QA，不能用 Android 结果替代。 | Android 点击 QA 已完成，iOS QA 待补 |
| 2026-06-29 | 暂挂 iOS 公开元数据 JSON / 图片嵌入副本运行态 QA，继续推进不依赖 macOS 的 V3 准备层：`watermark-core` 新增 V3 最小锚点 codec 准备层，固定 `PAYLOAD_V3_MINIMAL_ANCHOR_BYTES = 39`，只包含 `watermark_id + payloadProtocolVersion + auth_tag`；新增 `rights:v3-minimal-anchor-contract` 确认未接入图片 / 音频 / 视频正式写入读取路径，`PAYLOAD_BYTES = 119` 不变。下一步为 V3 设计跨端 fixture 与迁移桥接报告字段，不直接切默认写入。 | V3 codec 准备层完成，正式媒体迁移待设计 |
| 2026-06-29 | 完成 V3 跨端 fixture 与迁移桥接报告字段冻结及机器门禁：新增 `docs/V3跨端fixture与迁移桥接报告字段冻结合同.md` 和 `rights:v3-migration-contract`，明确 V3 媒体内只保留 `watermark_id + payloadProtocolVersion + auth_tag`，并冻结桌面 / Android / iOS 的图片与音频双向 fixture、V2 legacy 迁移桥、registry 对照、同步字段来源、正式报告显示差异和 feature gate 回滚门禁。`watermark-core` payload 层新增 `decode_watermark_payload_readonly`，可在 bytes 层只读识别 V2/119 与 V3/39；该任务不修改 V2/119 payload，不接默认写入，不把 Android QA 当 iOS QA。下一步进入 V3 图片 / 音频 fixture 只读解析实现。 | 机器合同完成 |
| 2026-06-29 | 完成 V3 staged PNG/WAV 容器只读 fixture QA：新增 `watermark-core/src/v3_readonly_fixture.rs`，用 PNG `tEXt` ancillary chunk 和 WAV `hsV3` RIFF chunk 承载 V3 minimal anchor bytes；桌面 `v3_readonly_fixture_qa`、移动端 `decode_v3_readonly_media_fixture_for_mobile` 和 `rights:v3-readonly-fixture-qa` 验证真实 PNG/WAV 容器中的 V3 anchor 可被桌面 / 移动 bridge 保留为 `watermarkUid`、`payloadProtocolVersion=3`、`payloadBytesLength=39` 和 `payloadAuthStatus=verified`。该 QA 不调用正式 image/audio/video 盲水印提取路径，不接默认写入，不替代 iOS QA。下一步设计正式盲水印算法路径中的 V3 只读 fixture，并继续保持 feature gate 关闭。 | staged 容器 QA 完成，正式盲水印 fixture 待补 |
| 2026-06-29 | 完成正式算法模块低层 V3 只读 packet fixture：图片模块新增 V3/39 sync packet 编解码单测，音频模块新增 V3/39 recovery packet 编解码单测，固定低层 packet 能只读解析 V3 minimal anchor 且 V2 decoder 会拒绝。默认 `WatermarkService::embed/extract`、图片 sync packet V2/119、音频 recovery packet V2/119 均未切换，不接默认写入，不替代 iOS QA。下一步把 V3 packet 只读能力接入受控提取候选路径，并继续保持 feature gate 关闭。 | 低层 packet fixture 完成，候选路径待补 |
| 2026-06-29 | 完成 V3 显式 readonly candidate reader：`watermark-core` 导出图片 sync packet 和音频 recovery packet 的只读候选入口，能在默认 V3 写入 gate 关闭时从正式算法承载位识别 `payloadProtocolVersion=3`、`payloadBytesLength=39`、`payloadAuthStatus=verified` 和 `watermarkUid`。默认 `WatermarkService::embed/extract`、桌面 / Android 版权库验证页、报告生成、同步回填和 iOS QA 均未切换到 V3；Android QA 不能替代 iOS。该 reader 的桌面端与 Android 原生 bridge 受控入口已由后一条记录完成。 | 显式候选 reader 完成 |
| 2026-06-29 | 完成 V3 受控验证入口与迁移桥显示字段：桌面端新增显式 `verify_suspect_readonly_candidate`，Android 原生 bridge 新增 `readReadonlyCandidate`，二者只在受控 QA / 内部入口中读取 V3/39 readonly candidate，不改变默认验证和默认写入。验证结果字段统一为 `payloadProtocolVersion`、`payloadBytesLength`、`payloadAuthStatus`、`watermarkIdIssueMode`、`mediaPayloadRole`，其中 V2 桥接显示 `v2_full_record`，V3 最小锚点显示 `v3_minimal_anchor` / `registry_resolved`；移动端默认 `read()` 和桌面默认 `verify_suspect` 仍保持 V2 正式路径。下一步补正式报告生成器与同步回填字段桥接 QA，并在 macOS + iOS Simulator 或真机恢复 iOS 同场景验证。 | 受控双端接入完成，正式报告 / 同步待补 |
| 2026-06-30 | 完成 V3 正式报告生成器 + 同步回填迁移桥字段 QA：桌面正式报告、移动端报告草稿、桌面云同步 payload、桌面 / 移动同步 allowlist 和真实后端同步运行态 QA 均覆盖派生 `media_payload_role`，V2 为 `v2_full_record`，V3 为 `v3_minimal_anchor`。新增 `rights:v3-report-sync-migration-qa` 复用真实后端 `dual:runtime-qa` 链路，固定 desktop->mobile V2 与 mobile->desktop V3 迁移桥字段；该 QA 不打开默认 V3 写入，不修改 `PAYLOAD_BYTES = 119`，不替代 iOS QA。下一步补三端真实运行态 V3 只读证据，iOS 仍需 macOS + Xcode + iOS Simulator 或真机环境。 | 报告 / 同步迁移桥 QA 完成 |
| 2026-06-30 | 补齐桌面 + Android 原生端 V3 readonly candidate 真实媒体文件运行态 QA 门禁：新增 `watermark-core` fixture-only helper 生成真实 PNG / WAV，其中 V3/39 minimal anchor 进入正式图片 sync packet 和音频 recovery packet 承载位；新增 `v3_readonly_candidate_runtime_qa`、Android `v3_readonly_candidate_runtime_qa.dart` 和 `rights:v3-readonly-candidate-runtime-qa`，固定桌面与 Android 原生端只通过显式 readonly candidate reader 读取 `payloadProtocolVersion=3`、`payloadBytesLength=39`、`payloadAuthStatus=verified`、`watermarkIdIssueMode=registry_resolved`、`mediaPayloadRole=v3_minimal_anchor`；默认 `WatermarkService::extract` / 移动端默认 `read()` 不路由 V3，由合同脚本检查。该 QA 不打开默认 V3 写入，不修改 `PAYLOAD_BYTES = 119`，不替代 iOS QA。下一步在具备 macOS + Xcode + iOS Simulator 或真机条件后补 iOS 同场景运行态证据，再评审 feature gate 写入。 | 桌面 / Android 真实媒体只读 QA 门禁已补 |
| 2026-06-30 | 完成 `watermark-core` 内部 QA 专用 V3 image/audio 写入 API 与 feature gate 回滚矩阵：新增显式 `V3InternalQaWriteGate` / `embed_v3_internal_qa_media`，只允许内部 QA 通过 `internal_qa` 写入 V3/39；`off` 和 `force_v2_rollback` 均回到默认 `WatermarkService::embed/extract` 的 V2/119。`rights:v3-feature-gate-rollback-contract` 已扩展为 `off -> internal_qa -> force_v2_rollback` 图片 / 音频六行自动矩阵，并静态检查默认 `WatermarkService`、移动端默认 `read()` 和正式路径未切 V3。该任务不打开默认 V3 写入，不替代 iOS QA。桌面端和 Android 原生端受控运行态 QA 已由后一条记录完成；macOS + Xcode + iOS Simulator 或真机恢复后仍需补 iOS 同场景证据。 | 内部 QA 写入 API 与回滚矩阵完成 |
| 2026-06-30 | 完成桌面端 + Android 原生端 internal_qa V3 写入运行态 QA：新增桌面 `v3_internal_qa_write_runtime_qa`、Android `v3_internal_qa_write_runtime_qa.dart` 和 `rights:v3-internal-qa-write-runtime-qa`，在受控运行态分别生成 V3/39 图片 / 音频样本，并同时用默认写入路径生成 V2/119 图片 / 音频样本；证据 `tmp-ui-qa/v3-internal-qa-write-runtime/1782758712380/v3-internal-qa-write-runtime-qa-1782758712380.md` 显示 desktop 与 android_native 的 `internal_qa` 均为 `v3_minimal_anchor`，`default_write` 均为 `v2_full_record`。正式桌面 scheduler、移动端默认 `write()`、默认验证页和同步 / 报告默认路径仍未切 V3；Android QA 不替代 iOS QA。下一步在 macOS + Xcode + iOS Simulator 或真机恢复后补 iOS internal_qa 写入同场景证据，再评审是否进入更严格的三端 feature gate release。 | 桌面 / Android internal_qa 写入运行态 QA 完成 |
| 2026-06-30 | 完成图片 / 音频 V3 默认算法迁移：`watermark-core` 默认 `WatermarkService::embed/extract` 已写读 V3/39 最小锚点，桌面 scheduler 与 Android 默认 `write/read` 已接入默认 V3；图片 V3 sync packet 改为 V3 专用容量布局，不再为 V2 dense payload 预留承载空间。`rights:v3-internal-qa-write-runtime-qa` 已验证桌面与 Android 的 internal_qa 和 default_write 均为 `v3_minimal_anchor` / V3/39；`rights:v3-feature-gate-rollback-contract` 仍验证显式 rollback 可产出 V2/119。V3 媒体内不再携带父 UID、原作品摘要、声明 bit 或训练许可，相关字段来自版权库 / 云版权库 / registry。iOS 默认 V3 写读 QA 仍因缺少 macOS + Xcode + iOS Simulator 或真机环境挂起。下一步恢复 iOS 环境后补三端默认 V3 运行态证据，并复跑完整保护副本文件流转 QA。 | 桌面 / Android 默认 V3 已切，iOS 待补 |
| 2026-06-30 | 收紧 V3 默认算法为“写 V3、读 V3”，不做默认 V2/V3 兼容读：`WatermarkService::extract`、桌面默认验证和 Android 默认 `read()` 只把 V3/39 minimal anchor 当默认成功结果；V2/119 仅保留在显式 `force_v2_rollback`、`embed_v2`、`extract_v2` 和迁移桥工具链中。`rights:v3-feature-gate-rollback-contract` 新证据 `tmp-ui-qa/v3-feature-gate-rollback/1782763052120/` 确认 `off` 与 `internal_qa` 均为 V3/39，只有 `force_v2_rollback` 为 V2/119；`rights:v3-internal-qa-write-runtime-qa` 新证据 `tmp-ui-qa/v3-internal-qa-write-runtime/1782763420804/` 确认桌面 + Android internal_qa 与 default_write 均为 V3/39；`rights:v3-readonly-candidate-runtime-qa` 新证据 `tmp-ui-qa/v3-readonly-candidate-runtime/1782763148914/` 确认显式 readonly candidate 只是迁移桥，默认读已 V3-only。iOS QA 仍挂起，不能用 Android 替代。下一步恢复 macOS + Xcode + iOS Simulator 或真机后补 iOS 默认 V3 写读同场景 QA。 | 默认 V3-only 已固化，iOS 待补 |
| 2026-06-29 | 冻结公开扫描 SDK 外部分发边界和匿名批量额度：后端将匿名批量查询最大 100 条提升为 `PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS`，并固定公开 SDK 稳定错误码集合；协议明确匿名 100 条只是技术保护上限，不是商业套餐额度，Enterprise API 仍需 API key、额度账本、调用审计和网关限流后才能评审。下一步设计外部分发包与 Enterprise API key / quota ledger 模型。 | 边界已冻结，商业 API 未开放 |
| 2026-06-29 | 完成 Enterprise 公开扫描 API key / quota ledger 草案，不改变双端能力：新增 `docs/Enterprise公开扫描API Key与额度账本模型草案.md`，只定义未来 API key、quota balance、quota ledger、调用审计和只读 scope；当前不新增桌面端或移动端入口，不开放企业批量扫描 API。下一步评审是否进入后端数据库迁移和内部管理命令。 | 草案完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise 公开扫描 API key / quota ledger 的后端内部模型第一步，仍不改变双端能力：后端新增四张企业 API / 额度 / 审计表和 Storage 内部命令测试，但没有新增桌面端或移动端入口，也没有开放 `/v1/enterprise/public-rights` 外部路由。下一步若继续推进，应先做内部管理 CLI / 后台入口，不接双端用户界面。 | 内部模型完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise 内部管理入口和 quota balance 初始化，仍不改变双端能力：后端新增受管理员 token 保护的 `/internal/enterprise/api-keys` 与 `/internal/enterprise/quota-balances`，内部 CLI 只调用这些内部入口；quota balance 初始化幂等且不会清空已用 / 预留额度。没有新增桌面端或移动端用户入口，也没有开放 `/v1/enterprise/public-rights` 外部路由。下一步如继续推进，仅做内部列表 / 查询 / 暂停 / 撤销管理，不接双端产品界面。 | 内部入口完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise API key 内部列表 / 查询 / 暂停 / 撤销，仍不改变双端能力：新增的管理能力全部位于 `/internal/enterprise/...` 和内部 CLI，只返回 key 元数据与 `keyPrefix`，不暴露 `keyHash` 或明文 key；没有新增桌面端或移动端用户入口，没有开放 `/v1/enterprise/public-rights` 外部路由，也没有接真实企业扫描扣费。下一步若继续推进，只做内部操作审计细分和后台 UI，不接双端产品界面。 | 内部 key 管理完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise 内部操作审计细分，仍不改变双端能力：新增 `enterprise_admin_audit_events` 只记录后台管理操作，不新增桌面端或移动端入口，不开放 `/v1/enterprise/public-rights` 外部路由，也不接真实企业扫描扣费。下一步如继续推进，只做内部只读后台 UI 或审计查询入口，不接双端产品界面。 | 内部审计完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise 内部只读审计查询，仍不改变双端能力：新增 `GET /internal/enterprise/admin-audit-events` 和内部 CLI `list-admin-audit-events`，仅供内部管理员按 operation / outcome / accountId / apiKeyId / occurredAt 查询后台管理审计；没有新增桌面端或移动端用户入口，没有开放 `/v1/enterprise/public-rights` 外部路由，也没有接真实企业扫描或 quota 扣费。下一步如继续推进，只做内部后台 UI 列表页和审计导出，不接双端产品界面。 | 内部审计查询完成，双端无新增入口 |
| 2026-06-29 | 完成 Enterprise 桌面内部后台审计列表页，仍不改变双端用户能力：新增 `EnterpriseAuditView` 只服务内部管理员，调用 `/internal/enterprise/admin-audit-events` 做筛选、分页和当前页 JSON 导出；它不是移动端用户入口，也不开放 `/v1/enterprise/public-rights` 外部路由，不接真实企业扫描或 quota 扣费。下一步如继续推进 Enterprise 管理面，应继续保持内部后台和双端用户能力分离。 | 桌面内部页完成，双端用户能力无新增 |
| 2026-06-29 | 完成 Enterprise API key 内部管理 UI，仍不改变双端用户能力：桌面端 `EnterpriseAuditView` 升级为内部管理员工作台，调用 `/internal/enterprise/...` 执行 API key 元数据 create / list / get / pause / revoke、quota balance 初始化和审计查询 / 导出；它不是移动端用户入口，不新增移动端页面，不开放 `/v1/enterprise/public-rights` 外部路由，也不接真实企业扫描或 quota 扣费。下一步若进入客户侧 Enterprise API，必须另走双端产品入口评审和外部路由合同，不把内部后台当用户能力。 | 桌面内部管理完成，双端用户能力无新增 |
| 2026-06-29 | 完成外部 Enterprise API 网关合同草案，仍不改变双端用户能力：新增的 `EnterpriseGatewayAuthContext`、`EnterpriseGatewayRateLimitPolicy`、`EnterpriseGatewayQuotaChargePlan`、`EnterpriseGatewayAuditContract`、`EnterpriseGatewayReadOnlyScanContract`、`ENTERPRISE_GATEWAY_REQUIRED_STEPS` 和 `ENTERPRISE_GATEWAY_STABLE_ERROR_CODES` 只服务 dry-run helper / 测试门禁，固定未来客户路由必须先做 API key 鉴权、scope 授权、`api_access` 检查、限流、只读解析、quota ledger 和 API audit；当前不新增桌面端或移动端用户入口，不开放 `/v1/enterprise/public-rights`，不接真实企业扫描或 quota 扣费。下一步若继续推进，只能先做内部 dry-run helper，不把合同当成已上线客户 API。 | 外部网关合同完成，双端用户能力无新增 |

| 2026-06-29 | 完成 Enterprise 内部 dry-run 网关校验 helper，仍不改变双端用户能力：后端纯函数只用模拟 key / scope / quota / item 数输出鉴权、限流、扣费和 audit 决策，不新增桌面端或移动端用户入口，不开放 `/v1/enterprise/public-rights` 外部路由，不接真实企业扫描或 quota 扣费。下一步若接内部校验入口，仍必须保持它是管理面能力，不把 dry-run 结果当成客户 API 已上线。 | 内部 dry-run helper 完成，双端用户能力无新增 |
| 2026-06-29 | 完成 Enterprise dry-run 网关校验内部入口和 CLI，仍不改变双端用户能力：后端新增 `POST /internal/enterprise/gateway-dry-run`，内部 CLI 新增 `dry-run-gateway`，只供管理员手工校验模拟 key / scope / quota / item 数的网关决策；不新增桌面端或移动端用户入口，不开放 `/v1/enterprise/public-rights`，不接真实企业扫描或 quota 扣费。下一步若继续推进，应只做内部 QA 样例，不把 dry-run 结果当成客户 API 已上线。 | 内部 dry-run 入口完成，双端用户能力无新增 |
| 2026-06-29 | 完成 Enterprise gateway dry-run 运行态 QA 门禁，仍不改变双端用户能力：新增 `scripts/verify-enterprise-gateway-dry-run-runtime-qa.mjs`，只通过内部 CLI 和内部路由验证六个网关决策样例及 `dry_run_gateway` 管理审计，不新增桌面端或移动端用户入口，不开放 `/v1/enterprise/public-rights`，不接真实企业扫描或 quota 扣费。下一步若进入真实客户 API，必须另走双端产品入口和客户文案评审。 | 内部 dry-run QA 完成，双端用户能力无新增 |
| 2026-06-30 | 完成公开元数据嵌入与 SDK / Enterprise 外部只读 API 的双端边界更新：桌面端 PNG / JPEG 嵌入副本支持 XMP / IPTC / JSON-LD 和官方 C2PA signed manifest；Android 仍支持重新选择 PNG / JPEG 保护副本后导出传播层嵌入副本；WAV / MP4 嵌入当前由桌面 QA 覆盖为容器级 registry metadata JSON packet；外部分发 TypeScript SDK 与后端 `POST /v1/enterprise/public-rights/batch` 都只解释 registry 公开权利信号，不新增桌面 / 移动端用户侧 Enterprise 管理入口，也不把训练许可显示为法律授权结论。下一步在 macOS + Xcode + iOS Simulator 或真机恢复后补 iOS JSON / 图片嵌入副本运行态 QA。 | 桌面 / Android 完成，iOS QA 挂起 |
| 2026-06-30 | 完成 V3 默认算法与质量 fast gate 的双端状态收口：桌面 scheduler 与 Android 原生默认 write/read 已切到共享核心 V3/39；V2/119 只保留为显式 rollback / 迁移工具链。`watermark:quality-gate:fast` 已覆盖 V3 图片 roundtrip 性能与质量、音频质量 fast 指标，但还不是完整 release SLA。下一步补 iOS 默认 V3 写读 QA，并设计 release 样本池后再讨论 `Forensic` / `Balanced` 用户可选策略。 | 桌面 / Android 默认 V3 完成，iOS / release gate 待补 |
| 2026-07-01 | 补齐 iOS 原生 runtime QA 入口：新增 `mobile_app/tool/ios_real_runtime_qa.dart`，把 iOS 运行命令、文案和结果视图拆成明确的 iOS 专属入口；新增 `scripts/verify-ios-real-runtime-watermark-status-qa.mjs` 和 `npm run dual:ios-real-runtime-status-qa`，脚本会先找 iOS device，再启动 `mobile_app/tool/ios_real_runtime_qa.dart`。当前 Windows 机器 `flutter devices` 仅检测到 Android 模拟器、Windows、Chrome、Edge，没有 iOS Simulator / 真机，因此脚本在本机返回“未找到 iOS device”。下一步在 macOS + Xcode + iOS Simulator 或真机环境复跑该命令并回填截图证据。 | iOS 入口已补，当前机器无 iOS device |
| 2026-06-30 | 完成公开权利 Enterprise 网关与音视频 C2PA 状态收口：桌面端公开元数据 AV QA 已升级为 WAV / MP4 传播层 + 官方 C2PA active manifest，Enterprise 只读批量 API 已在强制可信反向代理模式下验证 hash-only `clientFingerprintHash`、quota 扣减和 `legalConclusion=false`；该工作不新增移动端 Enterprise 用户入口，不改变桌面 / Android / iOS 的水印写读能力边界。视频默认仍只开放 L1 视频音轨水印和 L2 视频指纹存证，L3 继续 staged / internal，不接 UI、云任务、账本或 SLA。下一步补 iOS 公开元数据 / V3 默认写读 QA，并在生产 C2PA 证书链 / TSA 注入后复跑生产 staging QA。 | AV C2PA 与 Enterprise 网关已收口，iOS / 生产信任链待补 |
| 2026-06-30 | 新增非外部依赖 V3 payload release QA 与质量 release smoke：`rights:v3-media-payload-release-qa` 固定图片 / 音频 / L1 视频音轨默认写读 V3/39，L2 视频指纹存证明确为无媒体 payload 的不可逆 notary；`watermark:quality-gate:release` 已覆盖确定性图片、音频、L1 视频音轨质量样本池并纳入正式发布阻断组合。该工作不替代 iOS QA，不开放 L3，也不把 Forensic / Balanced 暴露为用户策略。下一步恢复 iOS 环境后补同场景 V3 默认写读证据，并扩展 full 样本池。 | 非外部依赖 release 门禁已纳入阻断，iOS / full 样本池待补 |
| 2026-06-30 | 推进图片北极星正式格式矩阵：`rights:v3-media-payload-release-qa` 从单 PNG 扩展为 PNG / JPEG / WebP / BMP 四类图片默认 V3/39 写读 release QA，并用 `rights:v3-media-payload-release-contract` 固定四类格式令牌；TIFF 仍保持候选输入，不进入当前正式承诺。下一步继续把音频 WAV / MP3 / FLAC / OGG / M4A / AAC(M4A/MP4 承载) 的正式归一化矩阵接入 release QA。 | 图片正式格式矩阵进入 V3 payload QA |
| 2026-06-30 | 继续推进视频北极星输入矩阵：L2 本地视频指纹生成白名单已扩到 MP4 / MOV / WebM / AVI / MKV / M4V；L1 视频音轨 release gate 已覆盖 MP4 / MOV / AVI / MKV / M4V / WEBM，WebM/Opus 成品回读已接入 release QA。相关提交：`8146cb1`（AAC / WebM 门禁），`636465e`（其余 core / contract 改动）。下一步把 release contract 的 WebM/Opus 回读证据和正式样本池沉淀到质量门禁。 | L2 入口推进，L1 WebM 已并入 release QA |
| 2026-06-30 | 收口双端一致性北极星关联决策：图片正式容器先收敛到 PNG / JPEG / WebP，按素材类型分层并要求视觉无损验证；音频加入 ABX 主观听感样本池和人类听觉无损验证；L1 视频音轨支持多音轨，但静音 / 极短 / 低于 30 秒视频明确拒绝；L2 指纹存证加入相似性阈值和争议处理流程；生产 C2PA 证书链和 TSA 仍是发布前硬门槛。该决策不会放宽双端同步字段白名单，也不会把 iOS 缺口交给 Android 结果替代。 | 北极星决策收口完成，双端边界未放宽 |
| 2026-07-01 | 推进 L3 视频画面盲水印正式化准备但不放宽双端承诺：后端 L3 `succeeded` 状态必须携带 `strategyDigest`、`selfCheckThreshold`、`selfCheckConfidence`、`checkedFrames`、`watermarkedMediaHash`、`serverReceiptSignature`，并要求 `confidence >= threshold` 后才允许扣 `video_minutes`；新增 `watermark:l3-video-visual-release-gate`，强制完整 24 个 2K 样本池、H.264 / HEVC 分组阈值、耗时和失败归因。桌面 / 移动正式入口、版权库 L3 记录、正式报告、跨端读取验证、失败文案和隐私边界尚未落地，移动端不能把 L2 或同步记录包装成 L3。下一步先跑并修复 L3 release gate，再设计双端正式入口和报告字段。 | L3 release candidate 门禁接入，双端正式能力待补 |
| 2026-07-01 | 完成 L3 release gate 长跑与后端 trusted completion 双端边界同步：`npm run watermark:l3-video-visual-release-gate` 已完整跑完 24 个 2K 样本池并通过，证据目录 `tmp-ui-qa/l3-video-visual-release-gate/1782888912515/`；`feedback-backend` 已把用户 bearer `succeeded` status update 拒绝为 `cloud_video_task_completion_requires_trusted_worker`，只允许 trusted worker/admin `POST /internal/video-tasks/:task_id/completion` 带 HMAC 收据完成并扣 `video_minutes`。这仍不改变双端用户能力：桌面 / 移动没有正式 L3 入口，没有 L3 版权库记录 / 报告 / 跨端读取验证 / 失败文案 / 隐私边界，移动端仍只能展示 L1 / L2 当前正式能力。下一步把双端受控 L3 入口、报告字段和跨端验证作为同一 release gate 设计。 | L3 gate 与收据链完成，双端产品面待补 |
| 2026-07-01 | 完成 L3 受控 worker 最小闭环且不放宽双端入口：`cloud-video:l3-worker-qa` 已创建内部 fixture L3 task，调用 `watermark-core` worker fixture 完成策略、写入、自检，普通用户 bearer 伪造 completion 被拒绝，trusted completion 固化收据并扣 `video_minutes`。该 QA 只证明内部 worker 包装可以消费 `watermark-core` 自检结果，不新增桌面 / 移动正式 L3 入口，不新增 L3 版权库记录、正式报告或跨端读取验证；当前 QA 区分任务 `watermarkUid` 与 core 派生 `payloadWatermarkUid`，真实 worker 仍需补 registry-reserved UID 与 core payload 绑定。下一步先把 worker 扩展到真实上传清单解析 / 转码沙箱 / UID 绑定 / 队列重放保护，再设计双端 Studio / Enterprise 受控入口。 | 受控 worker 完成，双端入口待真实 worker |
| 2026-07-01 | 完成 L3 真实 worker first-pass 且继续冻结双端入口：`cloud-video:l3-real-worker-first-pass-qa` 已走真实后端 reserve -> L3 task -> worker -> registry confirm -> trusted completion，固定受控上传清单解析、FFmpeg sandbox、registry-reserved UID 与 core payload 绑定，并强制 `payloadWatermarkUid === reserved.watermarkUid`。该 QA 不新增桌面 / 移动正式 L3 入口，不新增 L3 版权库记录、报告字段、跨端读取验证或用户失败文案；移动端仍不能把 L2 或同步记录包装成 L3。下一步先补任务领取幂等锁、队列重放保护、失败归因和真实输出封装，再设计双端 Studio / Enterprise 受控入口。 | 真实 worker first-pass 完成，双端入口仍冻结 |
| 2026-07-01 | 完成 L3 真实 worker 队列执行模型但继续冻结双端入口：后端新增内部 claim / failure API，completion HMAC 绑定 `workerId`、`attemptId`、`leaseToken`，真实 worker QA 已证明运行中任务不可重复领取、旧 attempt / 错 lease completion 被拒绝、retryable failure 可重排队、non-retryable failure 不扣费、重复 completion 不重复扣 `video_minutes`。该工作只推进受控后端 worker，不新增桌面 / 移动 L3 正式入口，不新增版权库 L3 记录、报告字段、跨端读取验证、失败文案或用户隐私边界。下一步先补受控对象读取、真实输出封装、用户可下载产物和 worker receipt 持久审计，再设计双端 Studio / Enterprise 受控入口。 | L3 队列闭环完成，双端入口仍冻结 |
| 2026-07-01 | 完成 L3 受控输出封装但继续冻结双端入口：真实 worker 已从受控上传对象读取 H.264 proxy、校验 manifest 哈希和字节数、调用 `watermark-core` 写入 / 自检 / packaged self-check，并将最终 MP4 写到 `controlled://l3-output/...`；后端 trusted completion 持久化 output ref / bytes / content type、worker receipt 和 receipt hash，HMAC 同步绑定这些字段。该工作证明后端受控队列能产出可下载形态的 L3 MP4，但桌面 / 移动仍没有正式 L3 入口、下载授权、版权库 L3 记录、报告字段、跨端读取验证、失败文案或隐私边界；移动端仍不能把 L2 或同步记录包装成 L3。下一步先接对象存储签名下载 / 下载授权，再把双端 Studio / Enterprise 受控入口、版权库和报告字段放入同一 release gate。 | L3 受控输出完成，双端入口仍冻结 |
| 2026-07-01 | 完成 L3 短期签名下载授权 API 与双端 Studio / Enterprise 受控入口：后端只允许 task owner 对已成功且固化 `controlled://l3-output/...`、`video/mp4`、media hash、receipt hash 的 L3 task 创建 `l3_output_download_authorization_v1`，签名 token 绑定 task / account / workspace / output / receipt / 过期时间；`cloud-video:ci` 覆盖 pending task 拒绝和 tampered token 拒绝。桌面工作台新增 `L3 受控入口 / 视频画面盲水印 release gate`，移动工作台新增 `视频指纹存证与 L3 受控入口`，两端均说明 Studio / Enterprise 受控申请、trusted worker receipt 和签名下载授权。该入口仍不创建普通用户 L3 任务，不保存原始视频、本地路径或未授权输出，不新增版权库 L3 记录、报告字段或跨端读取验证。下一步接普通用户对象存储、真实字节分发适配、下载入口和版权库 / 报告 / 跨端验证。 | 双端受控入口完成，正式下载与记录待补 |
| 2026-07-01 | 完成 L3 对象上传、真实字节分发和双端版权记录收据字段第一段：后端上传 / 下载授权已推进到 `object://l3-upload/...` 与 `object://l3-output/...`，`cloud-video:ci` 复核签名上传、worker 读取对象、真实 MP4 字节下载和 hash 绑定；桌面 / 移动版权库模型、SQLite、同步 payload、详情页和正式报告草稿新增 `video_visual_*` 字段，展示 L3 task、策略摘要、自检置信度 / 阈值 / 帧数、成品媒体摘要、worker receipt hash、字节数和 content type，并明确不保存对象 ref、签名 URL、本地路径或媒体字节。该工作仍只是 release candidate 产品面字段闭环，不代表 L3 已可售；下一步把 L3 完成任务的下载按钮、版权库写入触发和跨端运行态同步 / 报告验证接入同一 release gate。 | 对象存储与 L3 收据字段完成，正式产品流待补 |
| 2026-07-01 | 完成双端 L3 succeeded task 下载入库产品流第一段：桌面工作台和移动工作台都新增 taskId 输入与“下载并保存版权库”入口；两端状态层都调用后端 task 查询、下载授权和真实 MP4 字节下载，复核 `watermarkedMediaHash` / 字节数后才写入 `video_visual_*` 版权库记录并进入同步队列。`cloud-video:l3-product-flow-gate` 检查桌面 / 移动入口、版权库记录、正式报告字段、跨端同步字段和 object ref / 签名 URL / 本地路径排除项。该工作仍不代表正式可售：还缺真实后端下 desktop->mobile / mobile->desktop L3 运行态同步截图、正式创建 / 上传向导和失败文案验收。 | L3 双端下载入库 gate 完成，运行态 QA 待补 |
| 2026-07-01 | 完成真实后端下 L3 `video_visual_*` 双向同步运行态 QA：新增 `cloud-video:l3-cross-end-runtime-qa` 并纳入 `cloud-video:ci`，脚本启动临时 `feedback-backend`，同一账号下创建 desktop / mobile 两个 device，经后端 `watermark-ids reserve -> confirm` 生成 `video_visual` UID 后分别推送 `upsertVaultRecord`。QA 证据 `tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1782921329784.md` 覆盖 desktop->mobile 和 mobile->desktop，确认另一端版权库详情和正式报告投影完整读取 task、completedAt、strategyDigest、confidence、threshold、checkedFrames、mediaHash、receiptHash、outputBytes、contentType，并拒绝 object ref、签名 URL、本地路径和媒体字节。桌面详情页同步补齐 L3 完成时间、策略摘要、自检阈值 / 帧数 / 字节数 / content type。该工作仍不代表 L3 可售，正式创建 / 上传向导、失败文案和隐私边界仍待 gate。 | L3 双端运行态同步 QA 完成，可售仍待补 |
| 2026-07-01 | 完成桌面 / 移动 L3 创建上传向导一致性收口：桌面工作台新增“创建并上传 L3 任务”，移动工作台新增“选择 MP4 / 视频时长 / 创建并上传 L3 任务”，两端均进入同一后端对象上传、registry `video_visual` UID 预留和 `cloud_video_task_v1` 创建路径。两端文案一致说明“等待 trusted worker”，失败归因一致覆盖权益、登录、MP4 类型、时长、上传授权、哈希回读、任务创建和 worker failureCode，隐私边界一致为 `signed_object_upload_only_no_local_path_no_raw_video_sync`。创建成功只回填 taskId 供后续 succeeded 领取，不写版权库、不生成报告、不触发同步；正式记录仍只能来自 succeeded task 下载入库路径。下一步复跑 `cloud-video:l3-product-flow-gate` / `cloud-video:ci`，并补真实用户样本池下 desktop->mobile / mobile->desktop 创建后领取的运行态证据。 | L3 双端向导一致性入 gate |
| 2026-07-02 | 完成 L3 双端真实 MP4 最小证据链：`cloud-video:l3-sellable-runtime-qa` 使用真实后端登录 desktop / mobile 两个 device，分别覆盖 desktop 创建 / 领取 / 入库后 mobile 拉取读取，以及 mobile 创建 / 领取 / 入库后 desktop 拉取读取。两端同步读取均验证 `video_visual_task_id`、完成时间、策略摘要、自检置信度 / 阈值、checkedFrames、成品媒体摘要、worker receipt hash、输出字节数和 content type，并拒绝 object ref、签名 URL、本地路径和媒体字节。当时双端一致性仍未放宽到可售：移动端时长仍需手填，16:9 / 9:16 / 高帧率真实 MP4 样本池仍有容量边界；后续已补移动端可信视频尺寸 / 帧率探测、尺寸 / 帧率扩展样本池和稳定失败文案双端映射。 | L3 双端最小真实证据链完成 |
| 2026-07-02 | 完成 L3 双端尺寸 / 帧率 / 内容类型扩展样本池和容量预检对齐：`cloud-video:l3-sellable-runtime-qa` 复跑证据 `tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1782931358998.md` 覆盖 1024x1024 双端成功、1280x720 desktop->mobile 成功、608x1080 9:16 desktop->mobile 成功、1920x1080 mobile->desktop 成功、真实拍摄运动 fixture desktop->mobile 成功和字幕密集 mobile->desktop 成功。512x512@2fps / 8 帧改为双端一致的创建阶段输入限制：后端返回 `l3_strategy_capacity_insufficient`，桌面 / 移动上传向导展示“容量预检”，不创建 task、不扣 `video_minutes`。`cloud-video:l3-product-flow-gate`、`dual:contract` 和 `cloud-video:ci` 已通过。当前双端一致性仍未放宽到可售：移动端可信视频元数据探测、生产队列监控、SLA / 回滚和客户开通验收仍待补。下一步把生产队列运行态监控和移动端可信时长 / 尺寸探测接入同一 release gate。 | L3 双端扩展样本池与容量预检已对齐 |
| 2026-07-02 | 完成 L3 生产运营门禁与双端失败文案边界收口：`cloud-video:l3-production-ops-runtime-qa` 已纳入 `cloud-video:ci`，固定 queued / running / failed 队列快照、running lease、attempt SLA、retryable requeue、stale attempt replay protection、fatal no-charge hold、pending / failed 下载阻断和客服失败文案矩阵；`cloud-video:l3-product-flow-gate` 静态检查该门禁、文档和矩阵令牌。该工作不新增端侧算法，也不改变桌面 / 移动只保存 `video_visual_*` 收据元数据的隐私边界。当前双端一致性仍未放宽到可售：移动端可信视频元数据探测、生产 on-call 告警、对象存储清理策略和客户开通验收仍待补。下一步把移动端可信时长 / 尺寸探测接入创建向导，并让桌面 / 移动共享同一错误码文案资源。 | L3 生产运营门禁进入双端 release gate |
| 2026-07-02 | 完成移动端可信视频尺寸 / 帧率探测 + 对象存储清理策略 + 生产 on-call 告警 runbook 的双端边界收口：移动端创建向导选择 MP4 后优先读取容器 `mvhd` / `mdhd` / `tkhd` / `stts` / `stsz`，把可信时长、宽高、帧数和帧率传入同一条 L3 容量预检 / manifest 路径；探测只读媒体字节，不保存本地路径、对象 ref、签名 URL 或媒体字节到同步 / 报告。生产 ops QA 同步固化对象存储清理策略和 on-call 告警 runbook，仍不新增端侧 L3 算法。当前双端一致性仍未放宽到可售：真实生产 observability 面板 / 告警平台接入、客户开通验收和更大真实用户 MP4 目录样本池仍待补。下一步把双端 L3 真实机型视频探测失败兜底文案和客户开通验收接入同一 release gate。 | L3 移动可信探测与运营 runbook 入 gate |
| 2026-07-02 | 完成 L3 生产 observability 面板 / 告警平台接入 + 客户开通验收清单的双端边界收口：本轮新增的是后端 ops / 客户开通 gate，固定 dashboard 面板、告警路由 dry-run、客户验收步骤、隐私边界和跨端版权库 / 报告回读检查；桌面 / 移动仍只保存 `video_visual_*` 收据元数据，不保存对象 ref、签名 URL、本地路径或媒体字节。当前双端一致性仍未放宽到可售：真实告警平台配置验证、首个试点客户签字验收和更大真实用户 MP4 目录样本池仍待补。下一步把试点客户真实样本的 desktop->mobile / mobile->desktop 领取入库记录接入同一 release gate。 | L3 observability / alert / customer opening 双端边界入 gate |
| 2026-07-02 | 完成移动端视频 L1 / L2 消费边界修正：移动端工作台和设置页不再承诺本机 L1 视频写入，只承诺 L1 视频音轨验证；验证页支持 MP4 / MOV / MKV / WebM 作为视频音轨样本入口。移动端 L2 从“只读同步展示”推进到 Creator 权益下的轻量不可逆 metadata notary 提交，工作台新增“选择 L2 视频 / 提交 L2 指纹存证”，状态层生成 `mobile_video_fingerprint_metadata` manifest、调用后端 notary、写入版权库和同步队列；桌面端仍保留完整 `VideoFingerprintBundle -> notary -> vault` 流。`dual:contract`、`cloud-video:ui-contract`、`flutter test test/widget_test.dart` 与 `flutter test test/mobile_app_state_test.dart` 已通过。下一步把 L3 真实生产 webhook / 试点客户签字 / 真实样本目录 manifest 纳入双端可售阻断门禁，避免双端 UI 先于生产验收放宽。 | L1/L2 双端边界对齐，L3 可售仍需外部门禁 |
| 2026-07-02 | 补出 iOS 公开权利 / V3 专属运行态 QA 入口：新增 `mobile_app/tool/ios_public_rights_v3_runtime_qa.dart` 和 `rights:ios-public-rights-v3-runtime-qa`，在真实 iOS Simulator / 真机上覆盖公开权利 JSON、公开元数据 JSON、PNG 图片嵌入副本字节检查和默认 V3/39 写读，并把结果作为 `public-rights:completion-gate` 的 iOS artifact。当前 Windows 环境仍无法运行 iOS，因此状态是“入口完成、真实 iOS 证据待补”，Android 运行态不能替代该门禁。下一步在 macOS + Xcode 环境复跑并把 JSON artifact 提交给 completion gate。 | iOS 专属入口完成，真实设备证据待补 |
| 2026-07-02 | 扩充 `docs/封版收口计划.md` 的双端验证任务：RC1 运行态 QA 现在显式覆盖桌面端和原生移动端的图片 / 音频 / L1 / L2 / L3 release candidate 边界、公开权利与训练许可展示、公开元数据导出、正式报告 / 草稿、云同步、隐私排除项、关闭后端错误提示和 Android 页面级 QA；iOS、生产 C2PA/TSA、外部告警和客户签字继续作为 blocked artifact，不允许用 Android 或 dry-run 替代。下一步按 RC1 顺序复跑无外部依赖门禁，并把桌面安装版与 Android 页面级 QA 证据回写到封版计划。 | RC1 双端验证任务已纳入封版计划 |
| 2026-07-02 | 完成 RC1 无外部依赖双端自动化复跑：`dual:contract`、`watermark:cross-end-release`、Tauri release-scope、Flutter analyze / test、`cloud:ci`、`cloud-video:ci`、`rights:runtime-qa` 均通过；`rights:runtime-qa` 最新证据 `tmp-ui-qa/public-rights-runtime/1782976682337/public-rights-runtime-qa-1782976682337.md` 覆盖桌面端与 Android 原生端公开权利 runtime，`cloud-video:ci` 最新 L3 cross-end / sellable / production ops 证据分别为 `1782974308373`、`1782974335100` 和 `1782974387537`。本轮修正 Tauri 验证测试以符合 V3/39 默认锚点语义，V2 payload 只作为 legacy / 迁移路径；Android 运行态仍不能替代 iOS，`public-rights:completion-gate` 继续因 iOS QA 和外部 artifact 缺失保持 BLOCKED。下一步补桌面安装版完整人工 QA 和 Android 页面级 QA 截图索引。 | RC1 双端自动化通过，iOS 仍待外部环境 |
| 2026-07-02 | 新增 `docs/音频噪声底跨端可读频带策略迁移设计.md`：在 A/B/C 隔离实验均不能晋级正式算法候选后，先冻结稳定噪声底音频频带策略迁移的兼容规则，要求新 extractor 读旧样本、双端旧写新读、新写双端互读、fixture manifest、rollback flag、`audioStrategyVersion` 报告字段和 full gate 阈值不降全部设计完成后，才允许进入真正 `watermark-core` 算法迁移。当前不改变桌面 / 移动正式音频能力边界。 | 设计闸门完成，算法实现未开始 |
| 2026-07-02 | 实现 `watermark:audio-noise-floor-migration-contract`：只检查迁移设计文档、A/B/C 关闭结论、fixture schema 占位、禁止平台层算法漂移、`field-noise >= 44 dB` 和 `extractionConfidence >= 0.99` 等阈值不降规则；同时阻止 migration experiment / read compat / release 命令在 fixture schema 前提前暴露。下一步创建 fixture schema 和 read compat gate。 | M0 contract 完成，算法实现未开始 |
| 2026-07-02 | 实现 `watermark:audio-noise-floor-migration-read-compat`：新增 `watermark-core/fixtures/audio-noise-floor-migration/manifest.schema.json` 和最小 `manifest.example.json`，先用共享核心生成 `watermark_core_legacy`、`desktop_legacy`、`mobile_legacy` 三类旧 V3/39 field-noise 保护副本占位，并验证当前默认 extractor 与 legacy readonly candidate 均可读回同一长格式 UID、V3/39 和 `extractionConfidence >= 0.99`。该工作不改变桌面 / 移动正式音频能力边界，不接入新频带写入策略。下一步补真实端侧 file-backed fixture，再设计新 extractor 读取顺序和报告字段。 | read compat 完成，算法实现未开始 |
| 2026-07-02 | 补齐音频噪声底迁移真实旧产物 fixture 与字段设计：新增桌面端 `audio_noise_floor_migration_desktop_fixture` 和 Android 原生 Rust bridge `audio_noise_floor_migration_android_fixture` 生成入口，manifest 纳入 `desktop-file-backed-field-noise-v3` 与 `android-native-file-backed-field-noise-v3` 的 file-backed WAV、SHA-256、字节数和生成器来源；同时设计新 extractor 读取顺序和报告字段 `audioStrategyVersion / extractorPath / extractorFallbackPath / readCompatibilityMode`。该工作仍不改变桌面 / 移动正式音频能力边界。下一步只接只读 extractor candidate stub，不做新频带写入。 | file-backed read compat 完成，算法实现未开始 |
| 2026-07-02 | 实现音频噪声底迁移只读 new extractor candidate stub：仅在 `watermark:audio-noise-floor-migration-read-compat` 输出 `extractorPath=v3_recovery_2_8k_legacy`、`extractorFallbackPath=v3_noise_floor_migrated_band_v1_candidate -> v3_recovery_2_8k_legacy`、`readCompatibilityMode=legacy_v3_read_compat_candidate_stub_fallback`，并继续由当前默认 extractor / legacy readonly candidate 读取旧 V3/39 field-noise fixture。该工作不改变桌面 / 移动正式音频能力边界，不实现新频带读取或写入。下一步设计真实 read-only new extractor candidate 的接口和失败码，再决定是否进入迁移实现。 | stub fallback 完成，算法实现未开始 |
| 2026-07-02 | 设计并接入音频噪声底迁移真实 read-only candidate interface：`watermark-core` 新增 `extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes` / samples 入口和 `AudioNoiseFloorMigrationCandidateFailureCode`，`watermark:audio-noise-floor-migration-read-compat` 仅在该 gate 内调用候选接口，当前旧 V3/39 样本预期得到 `candidate_not_implemented_no_frequency_strategy` 后 fallback 到 `v3_recovery_2_8k_legacy`，并继续验证 `extractionConfidence >= 0.99`。该工作不改变桌面 / 移动正式音频能力边界，不实现新频带算法，不接正式写入。下一步补候选接口失败码矩阵摘要，再评审是否进入真正频带读取算法。 | candidate interface 完成，算法实现未开始 |
| 2026-07-02 | 补齐音频噪声底迁移候选接口失败码矩阵摘要：`watermark:audio-noise-floor-migration-read-compat` 顶层报告新增 `candidateFailureMatrix`，固定 `candidate_not_implemented_no_frequency_strategy`、`candidate_input_invalid`、`candidate_audio_too_short`、未来 `candidate_payload_not_found / candidate_payload_invalid` 的 expected handling、gate disposition 和当前观测数；当前旧 V3/39 样本必须全部 fallback 到 `v3_recovery_2_8k_legacy` 且 `extractionConfidence >= 0.99`。该工作不改变桌面 / 移动正式音频能力边界，不实现新频带算法。下一步评审是否进入 read-only 新频带扫描设计。 | failure matrix 完成，算法实现未开始 |
| 2026-07-03 | 评审并进入 read-only 新频带扫描阶段：共享核心 `v3_noise_floor_migrated_band_v1_candidate` 只读扫描三组候选频带，`watermark:audio-noise-floor-migration-read-compat` 输出 `candidateScanAttempted=true`、`candidateScanProfiles` 和 `candidateFailureCode=candidate_payload_not_found`；生成式旧样本、桌面 file-backed 旧产物和 Android 原生 Rust bridge file-backed 旧产物均 fallback 到 `v3_recovery_2_8k_legacy`，同一 UID 可读且 `extractionConfidence=1.0`。该工作不改变桌面 / 移动正式音频能力边界，不写媒体，不接 UI / mock / release 默认路径。下一步设计 protected-new-candidate fixture manifest 和新旧样本 read-compat 阻断矩阵。 | read-only scan 完成，写入迁移未开始 |
| 2026-07-03 | 设计 `protected-new-candidate` manifest 草案和 read-compat 阻断矩阵：新增 draft-only `watermark-core/fixtures/audio-noise-floor-migration/protected-new-candidate/manifest.draft.json`，要求旧 V3/39 样本必须 candidate miss 后 fallback，新候选样本必须 candidate 命中；未来 desktop / Android 新候选 file-backed fixture 只能由平台 wrapper 调用 `watermark-core` 生成，legacy-only fallback、payload invalid、stub regression 均阻断。该工作不创建真实新候选 WAV，不改变双端正式音频能力边界。RC1 决策为暂停新候选 writer 实验，将 `field-noise` 标记为 release blocker / known limitation，并回到桌面安装版与 Android 页面级人工 QA。 | new candidate draft 完成，writer 暂停，RC1 QA 继续 |
| 2026-07-03 | 启动双端云同步可靠性收口：新增 `docs/本地版权库与云版权库同步可靠性设计.md`，要求桌面端和移动端都以后端 `auth/sessions` / `auth/refresh` / `me` / `entitlements/current` 返回的权益快照为唯一正式门禁，本地 profile 只作为缓存；同步队列需要覆盖新增记录自动 flush、stale `syncing` 恢复、401 refresh 重放、403 权益阻断、断线续传、去重、限流和一致文案。该任务不改变水印算法、payload、版权编号或同步隐私白名单。下一步先做桌面端 S0 修复并把同一语义同步到移动端合同。 | 同步可靠性设计完成，双端实现待做 |
| 2026-07-03 | 重新审计双端云同步可靠性设计以适配 SQLite + PostgreSQL 双后端状态：确认端侧本地 SQLite 队列仍是 S0/S1 主战场，`feedback-backend` SQLite dev/test adapter 仍是默认无外部依赖验收路径，PostgreSQL adapter 仅有 disposable runtime QA 与 P4 import smoke，生产 readiness 仍 BLOCKED。双端一致性下一步不应直接做 Postgres 生产切换，而应先固定桌面端和移动端都以后端远端权益快照覆盖本地缓存、403 显示权益阻断、401 refresh 失败显示重新登录、同步 payload 隐私白名单不变；Postgres 只作为 optional smoke 和后续 S3 双 adapter 合同。下一步执行 `cloud:sync-reliability-contract` 最小双端语义检查。 | 同步可靠性审计完成，S0 双端合同待做 |
| 2026-07-03 | 推进云同步可靠性 S0/S1 双端一致性基础：桌面端已把后端权益快照作为正式事实源，手动 flush 前刷新 `/v1/me`，401 自动 refresh 后保存 token，403 / Free 写入 `blocked_by_entitlement` 队列诊断；`cloud:sync-reliability-contract` 检查 UI 诊断、payload 隐私白名单、SQLite dev/test adapter 和 PostgreSQL adapter 的 Free 403 语义。队列层已覆盖 stale `syncing` 恢复和 `synced` 不重传。本轮尚未完成移动端同等 runtime QA、启动 / 前台 / 网络恢复 debounce 触发，也未完成 S3 per-event disposition / payload hash；下一步继续把同一事实源与诊断语义扩展到移动端运行态验证。 | S0/S1 桌面基础完成，双端 runtime QA 待补 |
| 2026-07-03 | 完成云同步可靠性 S0-S3 本机可验证基础：桌面端普通图片 / 音频 / L1 pipeline 入队后触发后台 best-effort auto sync，仍以后端权益快照为唯一事实源；UI 和诊断输出 blocked / auth / HTTP status / stale recovery；后端 SQLite 与 PostgreSQL adapter 均返回 per-event `eventResults`，保存 `payload_hash` / `entity_revision`，同 `clientEventId` 变更 payload 不再静默吞掉。新增 `cloud:sync-runtime-qa` 把桌面安装版 + Android 原生端 Creator / Free / 网络恢复 / event disposition 真机证据机器化为 blocked gate，当前 artifact `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783058057700.json`。下一步不再继续设计，直接执行 S4 桌面安装版与 Android runtime QA 并回填 artifact。 | S0-S3 本机通过，S4 双端 QA 阻断已机器化 |
| 2026-07-03 | 执行 S4 双端云同步 runtime evidence 首轮：真实后端已启动，最新版桌面安装版已构建并静默安装到 `D:\TestInstall\HiddenShield`，Android 模拟器 `emulator-5554` 在线并启动原生 App。新增 `cloud:sync-runtime-evidence` 入口后生成四类证据：桌面安装版 `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783062976375.json` 达到安装包定位、release exe 和 installed exe 启动 smoke；Android 原生 `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783062976375.json` 达到设备在线、APK 构建安装、App 前台和截图，`tool/real_runtime_qa.dart` 未输出云同步语义通过；网络恢复 `tmp-ui-qa/cloud-sync-runtime/network-resume-sync-runtime-1783062976375.json` 仍缺真实启动 / 前台 / 断网恢复驱动；后端 event disposition `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783062976375.json` 通过。强制 ready 证据 `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783063284616.json` 继续 BLOCKED。下一步补双端专用云同步 QA runner，输出同账号 Creator 自动同步、Free 权益阻断、队列诊断和隐私白名单 artifact。 | S4 首轮实跑，双端语义未 ready |
| 2026-07-03 | 完成 S4 双端云同步专项 runtime ready：桌面安装版 hidden automation channel 与 Android 专用 `mobile_app/tool/cloud_sync_runtime_qa.dart` runner 均已实现并实跑，双端输出 Creator pull / flush / pull、重复 flush `duplicate`、Free `blocked_by_entitlement`、队列诊断、隐私白名单 JSON artifact。最新证据 runId `1783067038401`：桌面安装版 `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783067038401.json`、Android 原生 `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783067038401.json`、网络恢复汇总 `tmp-ui-qa/cloud-sync-runtime/network-resume-sync-runtime-1783067038401.json`、后端 event disposition `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783067038401.json` 均 ready；强制 ready `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json` 通过。该工作不改变水印 payload、版权编号、算法或同步隐私白名单；Android 页面级 QA、iOS QA、真实 OS 断网拨测和完整安装版人工 QA 仍待补。下一步按封版计划补桌面安装版与 Android 页面级 QA 截图索引。 | S4 云同步专项 ready，页面级 QA 继续 |
| 2026-07-03 | 补桌面安装版 Batch 2 首组页面级 QA 截图索引：`tmp-ui-qa/desktop-batch2-qa/07-image-result-window.png`、`15-verify-image-result.png`、`13-audio-result.png`、`16-verify-audio-result.png`、`08-vault-after-image.png`、`18-formal-report-export-click.png` 和 `22-backend-off-error-visible.png` 分别覆盖桌面图片 / 音频写入验证、版权库、正式报告和关闭后端成熟错误提示；报告产物 `E:\Users\jihx\AppData\Roaming\com.hiddenshield.desktop\reports\formal_report-hsr-fb47bc23c2d1e667.md` / `.json` 保持隐私排除项。桌面云同步暂停 UI 未通过，点击 `暂停自动同步` 后提示登录状态失效且未切到 `manual_local_only`，证据 `tmp-ui-qa/desktop-batch2-qa/20-cloud-sync-paused.png`；Android 页面级 QA、iOS QA、真实 OS 断网拨测和剩余桌面页面仍待补。下一步先修复桌面暂停 / 恢复自动同步页面路径，再复测并继续 Android 页面级 QA。 | 桌面首组页面 QA 部分通过，暂停 UI 阻断 |
| 2026-07-03 | 修复并复测桌面设置页自动云同步暂停 / 恢复：桌面命令 `set_desktop_cloud_auto_sync_enabled` 在调用 `PATCH /v1/me/sync-preferences` 前先刷新 `/v1/me`，access token 过期时使用 refresh token 换新并保存；refresh 也失效时设置页清空本机失效 profile，显示重新登录表单和 `登录状态已失效，请重新登录后再调整自动云同步。`。安装版复测证据：`tmp-ui-qa/desktop-batch2-qa/25-expired-profile-relogin-after-click.png` 覆盖过期登录重新登录引导，`27-creator-relogin-ready-for-pause.png`、`28-cloud-sync-paused-after-fix.png`、`29-cloud-sync-resumed-after-fix.png` 证明真实 Creator 可从 `auto_cloud_vault` 切到 `manual_local_only` 并恢复。验证：`npm run cloud:sync-reliability-contract`、Tauri sync tests、`cargo check`、`npm run build`、`npm run tauri:build` 通过。下一步继续桌面安装版 Batch 2 剩余页面级 QA，并补 Android 页面级截图索引。 | 暂停 UI 阻断解除，页面级 QA 继续 |
| 2026-07-03 | 继续桌面安装版 Batch 2 剩余页面级 QA：本地批量图片 / 音频通过并写入版权库 #25 / #26；设置反馈、复制微信、匿名反馈本地保留和日志导出隐私扫描通过；L2 桌面完整指纹包生成与 notary 入库通过，记录 #28 保存 notary、bundle 摘要、采样帧和生成耗时。双端一致性阻断仍有两项：L1 桌面生成的 36 秒 MP4 在处理页内部验证通过，但独立验证页读取失败，必须先修复桌面 L1 成品验证链路后才能继续移动端 L1 验证口径对齐；公开权利后端接口返回 `legalConclusion=false` 且 metadata 隐私白名单通过，但桌面版权库 UI 查询 / JSON 导出 / 嵌入副本导出失败，疑似 WebView fetch / CORS，必须先修复桌面页面链路再补 Android 页面级公开权利截图索引。耗时证据 `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-remaining-summary.json`：图片写入 `4081ms`、音频写入 `392ms`、L1 视频写入 `3278ms`、L2 bundle `738ms`、图片验证 `22ms`、音频验证 `37ms`、L1 失败验证 `829ms`。下一步优先修复 L1 独立验证和公开权利 UI fetch/CORS，再复跑桌面对应页面并进入 Android 页面级 QA。 | 桌面剩余 QA 完成覆盖，双端 L1 / 公开权利仍阻断 |
| 2026-07-03 | 桌面端 L1 / 公开权利两个双端一致性前置阻断已解除：L1 视频音轨成品 MP4 独立验证页改为复用多候选音轨抽取，复测同一保护副本命中 `HS-8AC03224-3A9A66CA-037F4F93-BA5E84D1`、置信度 `100%`、耗时 `300ms`，证据 `tmp-ui-qa/desktop-batch2-qa/56e-l1-video-verify-after-fix-result.png`；公开权利 / 公开元数据后端已允许 Tauri WebView Origin，桌面版权库 UI 刷新、JSON 导出和嵌入元数据图片副本导出均通过，证据 `tmp-ui-qa/desktop-batch2-qa/58-public-rights-refresh-after-cors-fix.png`、`59-public-metadata-json-export-after-cors-fix.png`、`60-public-metadata-embedded-export-after-cors-fix.png`。本次不改变水印 payload、版权编号、watermark-core 算法或同步隐私白名单；下一步必须在 Android 原生端补页面级 L1 验证、公开权利 / 训练许可展示、公开元数据入口、版权库和报告草稿截图，不能用桌面复测替代移动端证据。 | 桌面阻断解除，Android 页面级 QA 待补 |
| 2026-07-03 | 进入 Android 阻断相关页面级 QA 并补移动端 bridge 修复：同一桌面 L1 MP4 保护副本首次在 Android 验证页失败，错误为 `audio_sample_rate_missing`，根因是 Symphonia `default_track` 选到视频轨；移动端 bridge 改为优先选择含音频 `sample_rate` 的轨道后，同一文件读回 `HS-8AC03224-3A9A66CA-037F4F93-BA5E84D1`、V3/39、置信度 `100%`。公开权利图片保护副本在 Android 验证页读回 `HS-0E0A015B-4FEA4271-86F9A4B9-53B58EAB`、`registry 已生效`、`禁止 AI / ML 训练` 和“不是法律授权结论”边界文案。汇总 `tmp-ui-qa/desktop-batch2-qa/android-page-level-qa-summary.json`，截图 `android-page-qa-23/24/25/26`；本轮不改变水印 payload、版权编号或 `watermark-core` 算法。下一步继续 Android 全量 Batch 2：图片 / 音频写入、保护副本分享、版权库、报告草稿、L2 metadata notary、公开元数据导出入口和关闭后端错误提示。 | Android 阻断相关页面通过，全量页面 QA 待补 |
| 2026-07-04 | 修复双端云版权库 cursor 语义并复测桌面公开权利页面：后端 auth snapshot 现在只返回设备级 cloud cursor，未成功 pull 的新设备保持空 cursor；`/v1/sync/changes` 会取客户端 cursor 与服务端设备 cursor 中较早者，防止旧桌面 profile 或未来 cursor 跳过历史云事件。SQLite 单测覆盖设备 A push 后设备 B 首次登录 / 首次 pull，PostgreSQL adapter 已编译对齐，`cloud:sync-reliability-contract` 检查该语义。安装版桌面版权库复测通过：云端 metadata-only 记录可被拉取，公开权利显示 `registry 已生效` 与 `禁止 AI / ML 训练`，JSON 导出通过；本地图片记录 #25 有保护副本路径时嵌入公开元数据 PNG 导出通过。证据 `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-public-rights-sync-cursor-summary-20260704.json`。该证据不替代 Android 剩余页面级 QA、iOS QA 或移动端公开元数据导出入口验证；下一步继续 Android 图片 / 音频写入、保护副本分享、版权库、报告草稿、L2 metadata notary、公开元数据入口和关闭后端成熟错误。 | 桌面公开权利复跑通过，Android / iOS 剩余 |
| 2026-07-04 | 完成桌面安装版 Batch 2 页面级 QA 证据核验：`tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json` 汇总本地批量、L1 视频音轨写入 / 独立验证、L2 桌面完整视频指纹 notary、公开权利 / 训练许可 / 公开元数据、设置反馈和日志导出；当前安装版 sanity 截图为 `tmp-ui-qa/desktop-batch2-qa/97-desktop-batch2-current-sanity.png`。该结论只关闭桌面安装版页面组，不替代 Android 剩余页面、iOS、真实 OS 断网拨测或历史 `vault_records.file_type` backfill 风险；桌面 / 移动正式能力边界仍按既有文档执行。下一步继续 Android 图片 / 音频写入、保护副本分享、版权库详情、报告草稿、L2 metadata notary、公开元数据入口和关闭后端成熟错误。 | 桌面页面组完成，移动端继续 |
| 2026-07-04 | 完成 Android Batch 2 剩余页面级 QA：`npm run dual:android-batch2-page-qa` 最终 runId `1783106946906` 在 Android 模拟器 `emulator-5554` 上覆盖图片 / 音频默认 V3/39 写入、保护副本系统分享、版权库详情、报告草稿、L2 metadata notary、公开元数据 JSON 分享入口和关闭后端成熟错误。证据：`tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json`，截图目录 `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/screenshots/`。本轮确认图片记录 `HS-349D0670-7EEF9A24-CF151F69-051618F5`、音频记录 `HS-F8CFC5AD-907C5159-E38C402C-A1B14675` 均为 V3 / 39 bytes、`server_confirmed`、`verified`；L2 只提交不可逆 metadata notary，不上传原始视频或保存本地路径；公开元数据 `legalConclusion=false`。该证据不替代 iOS，不改变水印 payload、版权编号、`watermark-core` 算法或同步隐私白名单。下一步处理历史 `vault_records.file_type` backfill 风险，再整理 RC1 双端 QA 证据索引。 | Android 页面组完成，iOS / backfill 风险待补 |
| 2026-07-04 | 完成 `vault_records.file_type` 历史 backfill 与双端记录类型合同：桌面本地批量 #22 / #25 / #26 一类历史图片 / 音频记录不再因 SQLite 默认值持久化为 `video`；v18 migration 按文件名 / 保护副本名 / 本地保护副本路径扩展名回填 `image` / `audio`，并用 L2 / L3 视频收据字段防止视频 notary 误改。新入库记录显式写入共享推断结果，桌面云同步 event 与 changes response 的 `kind` 同步复用该推断，避免移动端拉取或云版权库摘要产生类型漂移。新增 `vault:file-type-backfill-contract`，RC1 双端证据汇总到 `docs/RC1双端QA总索引.md`。本次不改变水印 payload、版权编号、`watermark-core` 算法或同步隐私白名单。下一步复跑 `dual:contract`、`vault:file-type-backfill-contract` 和定向 Tauri DB tests。 | file_type 双端语义风险解除，RC1 索引完成 |
| 2026-07-04 | 完成 `vault_records.file_type` 修复进入 RC1 聚合验收：`npm run commercial:ci` 已完整复跑通过并包含 `Vault file_type backfill contract`，输出 `vault:file-type-backfill-contract OK` 和 `HiddenShield commercial CI OK`；`docs/RC1双端QA总索引.md` 的自动化门禁栏已记录本轮 `file_type` backfill、新入库显式类型和同步 `kind` 推断进入商业化聚合验收。该工作不改变水印 payload、版权编号、`watermark-core` 算法、同步隐私白名单或双端正式 UI 默认路径；L3 production readiness、iOS 页面级 QA 和真实 OS 断网拨测仍保持独立 blocked / 待补。下一步按封版计划整理 RC1 无外部依赖验收包，并补 iOS blocked artifact 与真实 OS 断网拨测记录。 | file_type 已进入 RC1 聚合验收 |
| 2026-07-04 | 完成 RC1 无外部依赖验收包与双端剩余阻断证据：新增 `docs/RC1无外部依赖验收包.md` 和 `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.json`，集中链接桌面 Batch 2、Android Batch 2、云同步专项、PostgreSQL disposable、`commercial:ci` 和 blocked gates。官方 iOS runner 在 Windows 环境生成 blocked artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/ios-qa-blocked-20260704.json`，明确 Android 不能替代 iOS；真实 OS 断网拨测记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/os-network-disconnect-drill-record-20260704.json` 明确当前自动网络恢复 evidence 未执行真实 OS network toggle。该工作不改变双端能力、同步 payload 或 `watermark-core`；下一步执行真实 OS 断网人工拨测并补 Windows / Android 恢复证据。 | RC1 验收包完成，双端剩余阻断明确 |
| 2026-07-04 | 完成 Android 真实 OS 断网 / 恢复拨测并更新 RC1 双端证据：新增 `rc1:os-network-disconnect-drill`，Android 模拟器 `emulator-5554` 真实关闭 data / wifi 后 `10.0.2.2:43188` 不可达，恢复后 ping / port probe 均可达；证据 `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-os-network-disconnect-drill-20260704.json`，断网 / 恢复截图 `android-network-off-20260704.png` / `android-network-restored-20260704.png`，并关联同版 Android Batch 2 成熟错误提示、云同步队列诊断和隐私白名单 artifact。Windows 桌面端真实 OS 断网仍 blocked：当前安装版连接本机 loopback `127.0.0.1:43188`，关闭 Wi-Fi / Ethernet 不能证明云同步离线，当前会话也没有提权 firewall / proxy 许可；阻断证据 `tmp-ui-qa/rc1-no-external-acceptance/20260704/desktop-os-network-disconnect-drill-20260704.json`，聚合状态 `partial_ready_desktop_blocked`。本次不改变双端能力、同步 payload、版权编号或 `watermark-core`。下一步把 RC1 验收包交给 release owner，并另排 Windows 桌面端提权网络拨测。 | Android OS 断网 ready，Windows 桌面端阻断保留 |
| 2026-07-04 | 完成 RC1 双端验收交接与 Windows 桌面端断网复跑安排：release owner 评审请求已写入 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.json` / `.md`，要求明确 Android ready、Windows desktop blocked、iOS blocked 和外部生产项 blocked 的 go / no-go 决策；Windows 桌面端提权断网拨测安排已写入 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill-schedule-20260704.json` / `.md`，固定提权 firewall / proxy 或 LAN / staging backend 两种路径、断网/恢复截图、队列状态、成熟错误和隐私白名单通过标准。本次不改变双端正式能力、同步 payload、版权编号或 `watermark-core`；Windows 桌面端断网仍需 release owner 指派环境后复跑。下一步等待 owner 安排提权 QA operator 或 staging/LAN backend 窗口。 | 评审请求已提交，Windows 复跑待排期 |
| 2026-07-04 | 完成 release owner 双端 go / no-go 决策：`tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.json` / `.md` 明确 RC1 验收包可进入评审，但最终签字在 Windows 桌面端真实 OS 断网通过或书面豁免前为 NO-GO。Windows 桌面端复跑窗口已指定为 2026-07-04 20:30-21:30 Asia/Shanghai，首选提权 firewall / proxy，备选 LAN / staging backend；仍需产出断网前 / 断网中 / 恢复后截图、队列状态、成熟错误和隐私白名单证据。本次不改变双端能力、同步 payload、版权编号或 `watermark-core`。下一步在指定窗口执行 Windows 桌面端拨测并把聚合记录从 `partial_ready_desktop_blocked` 更新为 `ready` 或附书面豁免。 | 评审 GO，最终签字待 Windows 拨测 |
| 2026-07-04 | Windows 桌面端断网复跑窗口执行但未解除阻断：20:30 Asia/Shanghai 窗口内确认当前进程非管理员、后端健康但仍为 loopback `127.0.0.1:43188`、无 LAN / staging backend 配置；禁用 Wi-Fi / Ethernet 不能切断 loopback 且可能破坏工作会话，因此未执行伪断网。执行记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/windows-desktop-os-network-drill-execution-20260704.json` / `.md`，聚合记录保持 `partial_ready_desktop_blocked`。本次不改变双端能力、同步 payload、版权编号或 `watermark-core`。下一步必须由 release owner 提供提权 Windows QA operator 或非 loopback backend 后重跑。 | Windows 断网阻断未解除 |
| 2026-07-10 | 修复 RC 审查发现的桌面云同步冲突误清风险：桌面端 now 解析后端 `eventResults`，`accepted` / `duplicate` 才清本地队列，`conflict_payload_changed` / `rejected_invalid_event` 继续保留 failed 诊断和错误码；`cloud:sync-reliability-contract` 已新增桌面消费断言，单测 `desktop_flush_event_results_keep_conflicts_failed` 固定 conflict / rejected 不能被误标 `synced`。验证通过 `cargo test --manifest-path src-tauri/Cargo.toml desktop_flush_event_results_keep_conflicts_failed --lib`、`npm run cloud:sync-reliability-contract` 和完整 `npm run commercial:ci`。该修复不改变双端同步 payload、版权编号、`watermark-core`、移动端正式能力或 Windows / iOS blocked 边界；下一步由 release owner 复核 RC 修复提交，并继续补 Windows 桌面端非 loopback / 提权断网证据或书面豁免。 | 桌面 eventResults 消费缺口修复 |
## 14. PDF 正式报告双端一致性回写（2026-07-14）

状态：规划完成，尚未进入实现

本次完成：

- 新增 `docs/面向司法使用场景的版权证据报告PDF升级规划.md`。
- 固定 PDF、JSON、Manifest 必须来自同一个不可变报告模型快照。
- 固定双端共用 schema、字段字典、状态模型、模板版本、免责声明和 fixture。
- 固定用户声明、系统计算和第三方材料三类事实必须在双端使用相同标签与解释。
- 固定正式水印事实继续由 `watermark-core` 或正式端包装层提供，PDF renderer 不重新实现编号、payload、写入、读取或验证算法。

当前平台限制：

- 桌面端当前导出 Markdown + JSON。
- 移动端当前生成同字段报告草稿，尚未形成与桌面相同的 PDF 文件交付。
- 如果 R1 首版只在桌面生成 PDF，移动端必须展示明确 fallback 文案，并在 R3 完成同 schema 生成或云端签发与跨端校验。

计划门禁：

- 桌面生成 / 移动校验。
- 移动生成 / 桌面校验。
- Android 运行态 QA。
- iOS 环境恢复后的同场景 QA。
- 同一云同步记录在双端的事实字段、状态和限制说明一致。

验证：

- 本次未修改双端代码，未运行现有跨端报告合同。

风险：

- 不能长期形成“桌面正式 PDF、移动草稿”的产品承诺分裂。
- 不能让不同端对 TSA、registry、L2 / L3 字段使用不同可信等级。

下一双端一致性任务：

- 在报告 Phase R0 建立图片、音频、L2 和异常状态的共享 schema v2 fixture，并把 PDF / JSON / Manifest 字段一致性加入 `report:contract` 设计。

2026-07-14 Phase R0 执行补充：

- 已建立图片、音频与 L2 视频三类共享原型样本，使用同一模板和状态语义。
- 已形成 `schema-v2-draft.json`，双端后续应共同消费该合同演进结果。
- 当前原型运行在独立 HTML 中，尚未接桌面 Tauri command 或移动端 Dart 报告生成器。
- 异常状态 fixture、桌面生成 / 移动校验和移动生成 / 桌面校验仍未执行。

下一双端一致性任务：

- 将三类原型样本转换为仓库级共享 JSON fixture，并补充验证失败与材料缺失样本，作为后续 `report:contract` 输入。

2026-07-14 PDF renderer 技术选型回写：

- 双实现 Spike 已使用独立共享图片 JSON fixture，不从桌面 UI 或移动端 UI 重新拼装字段。
- Phase R1 首版将由桌面 Chromium worker 生成高保真 PDF，但 schema / JSON / Manifest 合同继续作为双端共享事实源。
- Rust 原生报告不形成另一套产品字段或法律口径，只作为 fallback / 参考实现。
- 移动端当前仍不能承诺本地生成同款 PDF；后续必须通过同 schema 云端签发或跨端校验消除差异。

下一双端一致性任务：

- 将 `tools/report-pdf-spike/image-sample.json` 演进为仓库级 report schema v2 fixture，并补充音频、L2、验证失败和材料缺失样本。

2026-07-14 Phase R1 桌面 PDF 最小集成：

- 桌面正式报告已消费 `FormalReportDocument schema v2`，生成 `report.pdf + report.json + manifest.json`。
- 桌面 PDF 仅投影现有图片、音频、L2 视频记录字段，不新增桌面专属水印事实或验证结论。
- 桌面最近导出 UI 使用 PDF / JSON / Manifest 术语，免责声明继续明确不构成司法鉴定或法律意见。
- 移动端继续使用同字段正式报告草稿，不在本阶段本地生成 PDF。
- 移动用户需要在桌面端执行 PDF 三件套导出；产品文案必须写成“桌面端导出 PDF，移动端查看同字段草稿”，不得写成“双端均可导出 PDF”。
- Manifest schema 与 `FormalReportDocument schema v2` 将作为后续移动校验输入；移动生成 / 桌面校验尚未完成。
- 桌面图片 fixture 连续三次通过 4 页、无溢出和 `<3000 ms` warm gate。

风险：

- Windows sidecar 安装包尚未验收。
- 音频、L2 视频正式 `FormalReportDocument` 运行态导出仍需加入同一门禁矩阵。

下一双端一致性任务：

- 将图片、音频、L2 视频三类正式跨端 fixture 接入 `report:pdf-r1-gate`，并在移动端实现 Manifest 只读校验草案，验证桌面生成 / 移动读取字段一致。

2026-07-14 Phase R2 完整性校验回写：

- 桌面 Manifest schema v2 已固定文件列表、摘要链、root digest、版本和替代关系字段。
- 桌面校验 UI 分离“文件匹配”“未签名”“未加盖报告包可信时间”，该术语将作为移动端只读校验基线。
- 移动端当前仍未读取 Manifest，因此 R2 尚不能承诺跨端校验。
- 桌面重新生成报告时保留历史目录和 `supersedesReportId`；移动端后续必须显示 active / superseded 差异，不能只保留最新一份。
- 二维码状态固定为 `not_issued`，双端都不能展示可扫描的正式校验二维码。

下一双端一致性任务：

- 在 Flutter 新增 Manifest schema v2 解析与只读 SHA-256 链校验，使用桌面生成的图片、音频、L2 fixture 完成 Android 运行态验证。

2026-07-14 Phase R3 Android 跨端校验回写：

- Flutter 已按桌面 Manifest schema v2 实现 `sha256_chain_v1` 只读校验，读取范围固定为 `manifest.json`、`report.json`、`report.pdf`。
- 移动端将文件完整性、Manifest 链、文档合同、数字签名、报告包可信时间分开显示，不把 `matched` 包装成签名可信或可信时间有效。
- 桌面 Chromium worker 已生成图片、音频、L2 视频三类正式报告 fixture；主机测试和 Android API 36 模拟器运行态测试全部通过。
- 移动校验器不调用 `watermark-core` bridge、不读取原媒体、不新增水印事实，只校验报告包字节和报告合同。
- 当前双端策略为“桌面生成 PDF 三件套、移动端本地只读校验”；移动端仍只生成同字段草稿，不承诺移动 PDF 导出。

验证：

- `flutter test test/report_bundle_verifier_test.dart` 覆盖三媒体匹配与 PDF 篡改。
- `flutter test integration_test/report_bundle_android_test.dart -d emulator-5554` 在 Android API 36 完成桌面生成 / 移动校验。
- `report:contract` 与 `dual:consistency-contract` 固定 schema、链算法、边界术语和三媒体跨端测试。

风险：

- 移动签发交接包 / 桌面校验及最终 PDF 生成已完成；iOS 运行态 QA 尚未完成。
- 三类 PDF fixture 当前通过 Flutter assets 注入测试 APK，约增加 2.2 MB，发布构建前必须改为仅测试资产或外置夹具。

下一双端一致性任务：

- 复用同一测试矩阵补齐 iOS 移动交接包生成与桌面导入。

2026-07-14 报告链路异常中断恢复验证：

- 桌面 R1 Chromium worker、受控中文字体和同一 `FormalReportDocument` 三件套输出已重新验证，未出现代码或资源丢失。
- 图片、音频、L2 视频三类桌面报告包重新生成后，Flutter 主机校验继续全部通过。
- `report:contract` 与 `dual:contract` 继续固定桌面生成、移动只读校验以及签名/可信时间边界。
- 双端策略已细化为：桌面生成完整 PDF，移动端只读校验并生成未渲染签发交接包。

下一双端一致性任务：

- 在 iOS 真机生成移动交接包，并在桌面完成最终 PDF 导入 QA。

2026-07-14 Phase R3 移动到桌面反向校验回写：

- 移动端新增 `formal_report_handoff`，仅包含 `report.json` 与 Manifest schema v2，不包含伪造或占位 PDF。
- 桌面校验器按报告类型执行文件合同：完整报告要求 PDF + JSON，移动交接包只允许 JSON，并要求 `mobile_handoff / not_rendered`。
- 桌面校验结果新增 `reportType` 与 `documentContractStatus`，验证页新增可选择任意报告目录的跨端校验入口。
- Flutter 确定性 fixture 已由 Rust 桌面测试直接读取；Android API 36 运行态已生成同 schema 交接包。
- 桌面已可把交接包转换为最终 PDF；移动 UI 仍必须使用“桌面签发交接包”，不得写成“移动端导出正式 PDF”。

下一双端一致性任务：

- 在 iOS 真机复跑交接包生成，并核对桌面最终 Manifest 的来源 root digest。

2026-07-14 Phase R3 桌面最终渲染回写：

- 桌面新增 `import_mobile_report_handoff`，外部移动交接包校验通过后调用同一常驻 Chromium worker 生成最终 PDF 三件套。
- 最终 Manifest 记录移动来源 reportId、sourceKey、root digest 和 `flutter_mobile` 平台；最终报告本身使用新的桌面报告编号。
- 桌面验证页在交接包匹配后显示“生成最终 PDF”，完成后进入既有最近导出和正式报告校验流程。
- 移动事实中的字符串记录 ID通过确定性 SHA-256 映射生成桌面内部 `recordId`；原始移动 reportId 和 sourceKey 保留在 Manifest 谱系中。
- 导入要求桌面 Creator `report_export` 权益，不把未签名交接包视为单份购买授权凭证。

下一双端一致性任务：

- 在 iOS 真机完成交接包生成、桌面导入、最终 Manifest root digest 对照和 PDF 分页 QA。

2026-07-14 Phase R3 Tauri 运行态 QA 与 Phase R4 案件合同回写：

- 新增 Tauri MockRuntime QA，直接读取 Flutter `mobile-image` fixture，并调用与桌面 IPC 共用的 `import_mobile_report_handoff` 核心。
- QA 使用真实 `AppState`、Creator 权益和常驻 Chromium worker，完整断言 PDF、JSON、Manifest、来源 root digest 与 `sha256_chain_v1`。
- 本次运行结果为 4 页、约 746640 bytes、978 ms；`report:contract` 与 `dual:contract` 已固定该 QA 入口。
- Phase R4 已新增跨端共享的案件级 `RightsEvidencePackDocument schema v1` fixture；它引用正式报告 root digest，不复制水印算法。
- 案件级自动观察和人工陈述为独立数组，桌面与移动后续不得将用户陈述包装成系统结论。

风险：

- 当前 R4 只有 schema、fixture 和合同门禁，尚无移动采集 UI、桌面案件编辑器、附件原件打包或跨端同步。
- iOS 的 R3 真机运行态 QA 仍未完成，不能因为 Tauri MockRuntime 通过而视为 iOS 已验收。

下一双端一致性任务：

- 先冻结 R4 案件级字段的桌面 / 移动显示词典和附件编号规则，再分别实现桌面只读原型与移动采集草稿，避免形成平台专属事实字段。

2026-07-14 Phase R4 八页原型双端边界回写：

- 案件级 HTML/PDF 原型已直接读取共享 schema v1 fixture，未加入桌面专属事实字段。
- 页面字段固定为案件、正式报告谱系、争议对象、样本、采集事件、附件、自动观察、人工陈述和限制说明。
- `ATT-01` 当前只是逻辑附件编号；尚未冻结桌面 / 移动共享的包内相对路径、文件角色和同步策略。
- PDF 仍由桌面 Chromium 生成，移动端不得据此宣称已支持案件级 PDF 导出。

下一双端一致性任务：

- 定义跨端共享的 `CaseAttachmentRef`、附件角色枚举和包内相对路径规则，并生成桌面/移动均可读取的案件包 fixture。

2026-07-14 Phase R4 案件包跨端合同回写：

- `CaseAttachmentRef` 的物理字段已固定为 attachment ID、sequence、role、相对路径、媒体类型、来源、获取方式、派生来源、字节数和 SHA-256。
- 双端共享角色枚举固定为 `original`、`working_copy`、`capture`、`external_receipt`。
- 包内路径只允许 `/` 分隔的 `attachments/...` 相对路径，禁止绝对路径、反斜杠、路径逃逸和符号链接。
- 合成 fixture 已包含四类附件和四类采集事件；桌面与移动后续必须使用同一 event / attachment chain 算法。

下一双端一致性任务：

- 在桌面先实现只读案件包校验器，再将同一 fixture 和算法移植到 Flutter，形成桌面生成/移动校验的 R4 互验门禁。

2026-07-14 Phase R4 桌面只读校验器回写：

- Tauri 已实现 `verify_rights_evidence_pack`，复算与 Node 合同相同的稳定 JSON 事件摘要、事件链、附件链和包级 root digest。
- TypeScript 已冻结六状态返回字段和附件逐项结果。
- Rust 测试证明附件字节篡改只破坏附件完整性，事件修改破坏事件链和包级目录合同，未登记文件破坏目录与附件完整性。
- Flutter 尚未实现同一算法，当前不能宣称 R4 案件包跨端互验完成。

下一双端一致性任务：

- 先完成桌面运行态 QA 和验证页展示，再将同一 fixture、状态命名与稳定 JSON 算法移植到 Flutter。

2026-07-14 Phase R4 桌面运行态与 UI 回写：

- Tauri MockRuntime 已直接调用注册的 `verify_rights_evidence_pack` IPC 命令，验证 camelCase 六状态返回合同。
- 桌面验证页已展示目录合同、附件完整性、采集事件链、附件链、签名和可信时间，并可展开四类附件结果。
- 桌面状态命名现已冻结，Flutter 必须使用相同的 matched / mismatch / not_signed / not_timestamped / present_unverified。
- 当前仅桌面完成，R4 跨端一致性尚未达标。

下一双端一致性任务：

- 在 Flutter 实现 `RightsEvidencePackVerifier`，使用同一物理 fixture 完成主机测试和 Android 运行态复算。

2026-07-14 Phase R4 Flutter / Android 跨端案件包校验回写：

- Flutter 已新增独立 `RightsEvidencePackVerifier`，复用桌面冻结的六状态命名、稳定 JSON、事件链、附件链和包级 root digest 算法。
- 移动 fixture 由桌面生成目录按字节同步；R4 合同会递归比较文件清单和每个文件字节，禁止双端 fixture 漂移。
- Flutter 主机测试覆盖正常包、附件篡改、事件篡改、未登记附件和稳定 JSON 键排序。
- Android API 36 运行态通过 AssetManifest 读取同一案件包，四类完整性状态均为 `matched`，签名与可信时间分别保持 `not_signed / not_timestamped`。
- Android 复算包根摘要为 `4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33`，与桌面声明值完全一致。
- 当前 APK 中的 fixture 仅用于 integration test，不代表移动端已经提供用户可见的案件包目录选择入口。

下一双端一致性任务：

- 在移动验证页接入目录选择与六状态只读卡片，并使用 Android 外部文件目录中的真实案件包副本完成一次非 AssetBundle 运行态 QA。

2026-07-15 Phase R4 移动验证页与外部目录 QA 回写：

- Flutter 验证页已接入 `FilePicker.getDirectoryPath`，用户可选择案件包目录并触发只读校验。
- 页面使用与桌面一致的六状态卡片：目录合同、附件完整性、采集事件链、附件链、数字签名和可信时间。
- 页面同时展示案件编号、证据包编号、声明 / 复算 root digest、附件匹配数和限制说明。
- Android API 36 运行态由应用进程通过 `path_provider` 获取系统授权的应用外部目录，主机随后 `adb push` 六个物理文件；校验阶段不再读取 AssetBundle。
- 外部目录结果六状态与桌面一致，root digest 仍为 `4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33`。
- 当前 QA 证明应用专属外部目录读写与页面数据流成立；任意共享目录的 SAF tree URI 持久授权尚未形成独立发布门禁。

下一双端一致性任务：

- 为 Android 增加 SAF tree URI 目录读取适配器并保存持久授权，使用 Download 中的案件包完成系统文件选择器点击 QA。

2026-07-15 Phase R4 Android SAF tree URI 回写：

- Android 原生新增 `ACTION_OPEN_DOCUMENT_TREE`、`takePersistableUriPermission`、SharedPreferences URI 存储和 `DocumentFile` 递归读取桥接。
- Flutter 验证页在 Android 改用 SAF 字节读取器；应用重启后可显示“校验已授权目录”，并允许重新选择案件包目录。
- API 36 模拟器已从 `/sdcard/Download/HiddenShield-R4-QA/case-fixture-r4-0001` 经系统 DocumentsUI 点击授权。
- 首次选择和强停重启后的复验均返回四项 `matched`、`not_signed`、`not_timestamped`，root digest 均为 `4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33`。
- 当前只验证 Android Download / 系统 DocumentsUI，不把网盘或任意第三方 DocumentsProvider 视为已兼容。

下一双端一致性任务：

- 增加一个第三方 DocumentsProvider 或云盘测试矩阵，并验证授权撤销、目录移动和文件删除后的六状态失败提示一致性。

2026-07-15 Phase R4 SAF 失败矩阵与错误合同回写：

- 冻结四个跨端文件访问错误码：`evidence_pack_authorization_revoked`、`evidence_pack_directory_missing`、`evidence_pack_attachment_missing`、`evidence_pack_provider_unavailable`。
- Flutter 验证页展示固定中文提示，并将 SAF 附件缺失与普通完整性 mismatch 分离。
- Android QA 已覆盖授权释放、目录移动、附件删除及恢复路径。
- 新增独立包名 `com.hiddenshield.qa.documentsprovider` 的只读 DocumentsProvider，直接打包同一桌面 R4 fixture。
- 第三方 Provider 基线六状态与 Download 一致；禁用 Provider 后稳定返回 `evidence_pack_provider_unavailable`。
- 当前 Provider 是确定性内部 QA Provider，不代表 Google Drive、OneDrive 或厂商云盘已经验收。

下一双端一致性任务：

- 在 iOS File Provider 和一个真实 Android 云盘 Provider 上复用四错误码，并完成双端提示与恢复动作对照 QA。

2026-07-15 阻塞项记录：

- `BLOCK-R4-PROVIDER-01`：真实 Android 云盘 Provider 与 iOS File Provider 失败矩阵。

### 2026-07-15 Phase K0 离线许可证跨端解析合同

状态：`已完成`

已完成：

- 桌面 TypeScript、Tauri Rust 和 Flutter Dart 共用同一 `HSLIC1` fixture。
- 三端统一解析 `schemaVersion`、`licenseId`、`productCode`、`installationId`、`keyId`、`issuedAt`、`notBefore`、`expiresAt`。
- 三端统一使用 `HiddenShield-Offline-License-v1\0` 域分隔消息验证 Ed25519。
- 三端对 454 字符固定 token 返回一致字段结果和 `signatureValid=true`。
- 三端对合法编码但被修改的 payload 返回签名无效。

验证：

- `npm run license:k0-cross-end`：通过。
- `npm run build`：通过。

### 2026-07-15 Phase K0 完成与 K1 内部签发器

状态：`Phase K0 已完成；Phase K1 内部最小集已完成`

已完成：

- TypeScript、Rust、Dart 统一解析 `HSREQ1`、`HSLIC1` 和 `HSRVL1`。
- 三端统一 checksum、签名域、canonical 字段顺序和 16 条错误码向量。
- Rust 内部签发器以同一共享合同签发，未创建桌面或移动端私有格式。
- 签发器属于内部运营工具，不构成桌面端产品能力，也不要求移动端提供私钥或签发入口。

验证：

- `npm run license:k0-cross-end`：通过。
- `npm run license:k1-cli-qa`：通过。

限制：

- 双端当前只有解析合同与测试代码，没有用户激活页面或安全存储。
- Phase K2 桌面先实现 installation identity 时，Flutter 必须继续复用相同 identity 派生、token 和错误合同。

下一双端一致性任务：

- Phase K2 桌面激活实现前，先冻结 installation identity 的 Base64URL-SHA256 测试向量，作为后续 Flutter K3 的跨端门禁。
- 状态：`blocked / 用户明确暂不推进`。
- 已完成前置：Android Download、独立 QA DocumentsProvider、四错误码和恢复路径。
- 未完成：真实云盘登录 / 离线 / 占位文件行为，以及 iOS File Provider 真机 QA。
- 解阻条件：产品重新确认优先级，并提供可用的真实 Android 云盘测试账户和 iOS 真机 / File Provider 环境。
- 该阻塞项不阻止本地 CDKEY 设计，但在解阻前不得扩大案件包 Provider 兼容承诺。

当前双端规划转向：

- 先设计离线 CDKEY / 本地许可证合同；后续实现必须保持桌面、Android、iOS 使用同一许可证 schema、签名域和错误码。

### 2026-07-15 Phase K3 Flutter / 移动端激活实现

状态：`代码完成；真机跨端签发验收待完成`

已完成：

- Flutter 继续复用 K0 冻结的 installation identity 向量和 `HSREQ1` / `HSLIC1` / `HSRVL1` 合同，没有创建移动端私有格式。
- 移动端激活请求、文件、粘贴和二维码路径传递同一原始 token。
- 单安装实例策略已落地：绑定到桌面或其他移动安装实例的许可证返回 `offline_license_device_mismatch`。
- Android / iOS 使用平台安全存储；Web、Windows、macOS 和 Linux Flutter 构建不开放移动离线授权。
- 离线权益只影响本地批量和正式报告；云同步、云批量、云视频、优先队列、团队空间和 API 不参与本地合并。

验证：

- installation identity 共享向量继续通过。
- 新增 K3 测试覆盖设备绑定、到期、安全存储失败、feature merge 和全部云 feature 关闭。
- `flutter analyze`：通过。
- 聚焦许可证测试：8 tests 通过。
- 全量 `flutter test` 被用户中止，未作为完成证据。

风险：

- 尚未取得“内部 Rust CLI 对移动端真实 `HSREQ1` 签发，Android / iOS 真机导入”的运行态证据。
- iOS Keychain entitlement、Android Keystore、相机权限和二维码扫描仍需真机确认。

下一双端一致性任务：

- 以同一个内部签发公钥完成 Android 和 iOS 各一次移动请求签发/导入，并验证许可证复制到另一安装实例稳定返回设备不匹配。

### 2026-07-17 桌面验证页音频结果布局修复

状态：`已完成`

已完成：

- 修复桌面验证页在固定侧栏压缩内容区后，文件选择列与音频版权记录列发生视觉重叠的问题。
- 验证工作区在视口宽度不超过 `1420px` 时切换为单列，文件选择、检测范围、验证结论和版权存证按顺序展示。
- 结果列、验证结论和版权存证容器统一限制为当前网格宽度，长版权编号、文件名和摘要允许安全换行。
- 本次仅调整桌面布局，不改变图片/音频验证算法、结果字段、版权编号或跨端读取合同。

验证：

- `npm run build`：通过。
- `1338×892` 音频命中记录页面级复核：文件选择列与结果列切换为单列，垂直间距 `16px`，无重叠。
- `1366×768` 音频命中记录页面级复核：两列等宽单列排列，垂直间距 `16px`，无重叠。
- `1920×1080` 音频命中记录页面级复核：恢复双列排列，左右间距 `16px`，无重叠。
- 页面控制台无错误。

风险：

- 移动端仍按当前发布基线冻结，本次不扩展移动端 UI。

下一双端一致性任务：

- 使用桌面 Release 在 `1366×768` 和 `1920×1080` 两种窗口尺寸复核音频命中、图片命中和未命中三种结果布局。

### 2026-07-17 桌面全局工作区与长文本防溢出对齐

状态：`已完成`

已完成：

- 桌面八个菜单统一使用“弹性主工作区 + `360–420px` 上下文面板 + `16px` 间距”的全局布局。
- 移除右侧上下文网格轨道中的未使用空白，页面组件不再通过逐页扩大宽度解决全局空间分配问题。
- 主工作区与上下文面板增加通用长文本防溢出规则，覆盖段落、强调值、表格字段、链接和代码文本。
- DropZone 单独增加 `width/max-width/min-width` 收缩边界和文件名 `overflow-wrap:anywhere`，不依赖具体音频或图片名称。
- 本次不改变图片/音频读取、验证、版权编号、报告字段或 `watermark-core` 跨端合同。

验证：

- `1920×1080` 八菜单修改前后截图已固化；主区/上下文实际间距由 `129.33px` 收敛为 `16px`。
- 八菜单 document、主工作区、上下文面板横向溢出均为 `0px`。
- 长文件名 `desktop-watermark-audio-input_watermarked_watermarked.wav` 的 DropZone 重叠由 `33.73px` 降为 `0px`。
- `npm run build`、`npm run release:desktop-baseline`、`npm run commercial:contract` 和 `git diff --check`：通过。
- 对比证据：`tmp/release-qa/appshell-grid-20260717/appshell-grid-comparison.jpg`。

风险：

- 移动端继续冻结，本轮没有同步移动端 UI。
- 浏览器 Mock 不能替代桌面 Release 在 Windows DPI 缩放下的最终视觉验收。

下一双端一致性任务：

- 使用桌面 Release 在 `1366×768`、`1920×1080` 以及 Windows `125%` 缩放下复核八菜单，并确认长图片名、长音频名和长版权编号均不产生横向重叠。

### 2026-07-17 基础存证摘要跨端字段矩阵 V1

状态：`合同已冻结；移动端继续冻结`

已完成：

- 基础摘要字段统一分为默认展示、条件展示、付费报告和禁止展示，不允许桌面与未来移动端各自定义摘要语义。
- 图片与音频使用同一记录、验证、登记、时间和声明字段；媒体详情按类型条件展示。
- V2 / V3 只影响技术字段，不改变基础产品字段名称。
- 本地时间、网络授时和已验证 TSA 时间必须跨端使用相同语义。
- 批量记录未来必须保存批次编号与批次内序号，不能只显示前端队列信息。

验证：

- 字段合同：`docs/基础存证摘要字段矩阵.md`。
- 当前未修改 watermark-core、payload codec、跨端 fixture 或移动端代码。

风险：

- 桌面 `VaultRecord` 尚未暴露数据库已有的 `file_type`。
- 批次追溯、媒体参数和核心版本尚无共享数据库字段。

下一双端一致性任务：

- 桌面完成 P0 摘要投影后，新增图片 / 音频摘要 fixture；移动端恢复前必须复用同一 fixture 和 `HS-SUMMARY-1` 字段顺序。

### 2026-07-15 Phase K2–K4 双端安全一致性

状态：`代码与共享自动化门禁完成；发布候选包真机 QA 待完成`

已完成：

- 桌面和移动继续复用 K0 的 token、canonical JSON、签名域、installation identity 和错误码，不存在平台私有 CDKEY。
- 双端 trust policy 统一支持 key status、license/revocation purpose、有效期和默认空生产公钥 ring。
- 双端均在平台安全存储保存最高可信 UTC；桌面同时保留 SQLite 镜像，超过 300 秒回拨 fail closed。
- 双端撤销列表均按 `keyId` 在平台安全存储保留完整集合并实现 sequence + digest 高水位：回放拒绝、同 digest 幂等、同 sequence 不同 digest 拒绝，轮换新 key 不会删除旧撤销；桌面数据库单独回滚会被 keyring 锚点识别。
- 双端均记录许可证替换审计；桌面 migration 20 与移动安全存储分别承载防回滚状态。
- 桌面/移动离线权益都只合并本地批量和正式报告，所有云 feature 保持服务端权威。

验证：

- `npm run license:k4-contract`：通过，3 个 key 与 11 条共享策略向量。
- Rust K2/K4 定向测试：10 tests 通过。
- Flutter K3/K4 聚焦测试：12 tests 通过，覆盖双 keyId 撤销保留与 `issuedAt` 时间语义一致性。
- `npm run build`、Rust `internal-qa` check、Flutter analyze：通过。

限制：

- 桌面使用编译期 `HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON`；移动使用同名 `--dart-define`。未注入时均拒绝生产许可证。
- iOS、Android 和桌面签名候选包尚未使用同一非 fixture 公钥完成运行态互验。

下一双端一致性任务：

- 从 Android 真机和 Windows 签名候选包分别导出 `HSREQ1`，用同一内部非 fixture key 签发两份单 seat 许可证，并交叉导入验证双方稳定返回 `offline_license_device_mismatch`。

### 2026-07-15 Phase K4 签名候选包双端复跑

状态：`内部 QA 通过；正式分发签名与 iOS 待补`

已完成：

- Windows Authenticode 内部 QA 候选包签名有效：`hidden_shield.exe` SHA-256 `37701E70499FA8A744FF1CEEC68E96DDA5CEC16A546FCB97F3C0222D16222BF6`，NSIS installer SHA-256 `D64C1DC75F4A9E1B070C8906FEC95AD87D2DF475C140A83402B3CEBD77F9A9A3`，证书 thumbprint `86E012CE09DBDA9853A7F8E164233E9952019625`。
- Android release APK 使用非 debug 内部 QA release keystore：APK SHA-256 `4D939CA2FC34A7C0FED76198F41A6DA9627D1D930D60B704D4FC19B358316565`，signer SHA-256 digest `bf7fc80e1d130fc592fce4b8277bf75d0eb9b2dfe8923b90946a102c78db4b95`。
- Windows 和 Android 均使用同一 trust policy `offline-internal-2026-q3-qa` 完成真实 `HSREQ1 -> HSLIC1 -> 重启 -> HSRVL1 撤销 -> 交叉设备不匹配` QA。
- Windows runtime 证据 `tmp/offline-license-internal-qa/20260715-195231/windows-final-runtime-qa-evidence.json`；Android UI 证据覆盖 `android-final-05-license-imported.png`、`android-final-06-after-restart.png`、`android-final-11-revocation-imported.png`、`android-final-19-cross-device-mismatch.png`。

限制：

- Windows 本轮为内部 QA 自签 Authenticode，不是公开 CA 或正式企业分发签名。
- Android 本轮为内部 QA release keystore，不是 Play/App Store 生产 keystore。
- iOS Keychain 与 iOS 分发签名仍未同场复跑；Android 模拟器证据不能替代 iOS。

下一双端一致性任务：

- 用正式生产/企业分发证书、Android 生产 keystore 和 iOS 分发签名材料复跑三端同一门禁，并补一条桌面许可证导入 Android、Android 许可证导入桌面的显式互拒证据。

### 2026-07-17 桌面基础存证摘要 P0 对齐

状态：`桌面投影完成；移动端继续冻结`

- 桌面 `buildCopyrightSummary` 已按 `HS-SUMMARY-1` 统一标题、身份声明、验证说明、登记状态、时间证明和隐私边界。
- 可选字段在拼接前过滤，等待登记不再生成“未记录”收据，自定义版权声明为空时整行省略。
- 水印协议只展示版本，载荷字节长度不再进入基础摘要；载荷验证改为产品字段“载荷完整性校验”。
- 本次未修改移动端、数据库、共享 fixture 或跨端模型；不把桌面投影完成解释为移动端一致性已经验收。
- 当前仍缺 `mediaType` 只读字段，因此图片尺寸与音频时长的跨端条件化合同尚未完成。

验证：

- 桌面发布合同已增加 P0 摘要静态约束。

下一双端一致性任务：

- 新增图片和音频各一份 `HS-SUMMARY-1` fixture，先固定桌面输出，再作为未来移动端恢复开发时的同序字段合同。

### 2026-07-17 桌面 P0 页面投影对齐

状态：`桌面三处展示完成；移动端继续冻结`

- `ResultPage.vue` 已按当前媒体类型区分图片与音频结果；音频只展示时长和文件大小。
- `CopyrightCard.vue` 已成为处理结果与版权库详情复用的 P0 主展示，统一身份、验证、登记、时间和技术字段。
- `VaultView.vue` 删除紧邻版权卡的重复旧字段，历史版本抽屉同步采用相同产品口径。
- 历史记录中的旧成功验证消息只在展示层归一化，不修改共享 payload、数据库记录或移动端模型。
- 本次未建立移动端 UI 一致性证据，也未恢复任何移动端开发任务。

验证：

- `npm run build` 与 `npm run release:desktop-baseline` 通过。

下一双端一致性任务：

- 建立桌面图片 / 音频 P0 页面 fixture 和截图基准，作为未来移动端恢复时的 UI 字段顺序与条件展示合同。

### 2026-07-17 桌面 P0 UI 最终收口

- 图片结果页移除帧率展示，音频继续只展示时长和文件大小。
- 桌面版权时间统一使用稳定格式，不再依赖 Windows 区域化上午 / 下午格式。
- 第三方时间证明服务使用产品名称展示，FreeTSA 原始 endpoint 不进入基础 UI。
- 移动端继续冻结，本次未修改移动模型或跨端数据合同。

下一双端一致性任务：

- 将最终图片 / 音频页面截图与字段顺序固化为未来移动端恢复时的对齐基准。

### 2026-07-17 桌面右侧上下文退役

状态：`桌面改为单列工作区；移动端继续冻结`

- 桌面删除 `ContextPanel` 与统一 `WorkspaceContext` 数据结构，八个一级菜单直接使用全部剩余工作区宽度。
- 左导航与顶栏继续保留当前年度授权标签，业务导航、权益门禁和能力边界没有变化。
- 移动端历史 `ContextSheet` 不在本轮修改范围内；移动端开发冻结期间不要求为了桌面壳子变化同步改造，也不得据此新增移动端产品承诺。
- 桌面在 `1600×1000` 与 `1024×768` 下完成八菜单截图 QA；年度授权页发现并修复一处 embedded 面板横向溢出。
- 截图证据：`tmp-ui-qa/single-column-shell/wide/`、`tmp-ui-qa/single-column-shell/narrow/`。

下一双端一致性任务：

- 移动端恢复开发前，重新评审是否保留移动 `ContextSheet`，不得自动把已退役的桌面右侧上下文重新引入桌面。
## 2026-07-21 音频采样率一致性修复与后续 Gate

- 已修复默认 V3 WAV 写入对 `44.1 kHz` 频带的硬编码：核心现在使用媒体实际采样率选择频带，输出继续保留输入 `WavSpec`。
- 真实控制结果：44.1 kHz 的 31 秒 WAV / MP3 mono、stereo `6 / 6` 通过；48 kHz 同类控制组修复后 `6 / 6` 通过。30 秒 48 kHz 的 WAV / MP3 / FLAC / OGG / M4A、mono / stereo `10 / 10` 通过，输出采样率和声道均未改变。
- 此修复避免通过桌面/移动端输出归一化规避问题，正式算法仍只在 `watermark-core` 内维护。
- 修复后仍需完成桌面写后回读、桌面读移动写、移动读桌面写与扰动回归；在这些 Gate 通过前，不能承诺“所有采样率和后续变换均可稳定保护与验证”。
- 性能复核同时确认矩阵必须使用 Rust `release` 构建；debug 结果不得作为产品性能证据。
- 48 kHz 五种容器 × mono/stereo release 扰动矩阵已完成：240 个产物全部保持原采样率/声道；基线、重编码、音量变化和 MP3 往返 `50 / 50` 通过。
- 5–15 秒裁剪矩阵只有 `56 / 190` 通过，且受起始位置和 MP3 往返影响明显。桌面与移动端不得出现“任意短片段都能验证”的不一致承诺。
- 广泛采样率/声道基线共 34 组，22 组通过；当前已验证到 48 kHz 的 1–8 声道组合。4 kHz、88.2/96/192 kHz 的失败来自 `watermark-core` 容量策略，不得由任一端私自改变输出规格规避。
- 双端协议需要记录并保持原始 `sample_rate`、`channels`，同时共享同一支持范围和失败原因码；“任意 kHz / 任意声道”在独立 Gate 通过前不属于产品承诺。

下一双端任务：补 48 kHz 保规格的双向跨端 fixture，并统一桌面/移动端对低于 30 秒片段验证失败的原因码与用户提示。
## 2026-07-22 桌面高位深音频闭环

- 桌面安装版已对真实 `24-bit WAV / 24-bit FLAC / float32 WAV` 的 mono / stereo 完成写入、写后回读、独立核心读取、只读验证和量化统计，证据为 `artifacts/desktop-high-bit-depth-audio-gate/20260722-final/summary.json`。
- 本轮只升级桌面产品口径；移动端保持冻结，不新增 fixture、Gate 或用户承诺，因此该能力当前明确不是双端一致性承诺。
- 桌面与移动仍共享 `watermark-core` 的 WAV 读写算法；后续恢复移动端时，必须先补移动写 / 桌面读、桌面写 / 移动读的同规格 fixture，才能把高位深承诺扩展为双端能力。
- 风险：当前安装版高位深矩阵是 48 kHz、31 秒代表样本，不替代 20 分钟、512 MiB 或更高资源边界 Gate。
- 下一双端一致性任务：保持移动端冻结，先完成桌面图片常规尺寸与接近 100 MP 的正式 Gate；恢复移动端前不得提前复制本条高位深文案。

## 2026-07-22 桌面图片正式边界闭环

- 桌面安装版已完成 PNG / JPEG / WebP 常规尺寸、约 99.92 MP、精确 512 MiB、100 MP + 1 和 512 MiB + 1 的资源 Gate。
- 正式桌面边界为静态三格式输入、容量判断、100 MP、512 MiB、PNG 输出和尺寸保持；50–100 MP 显示高资源警告。
- 移动端继续冻结，本轮不新增移动 fixture、UI 或产品承诺。
- 风险：近 100 MP 峰值超过 6 GiB、处理接近 20 分钟；恢复移动端前必须单独定义更低资源边界，不能复制桌面上限。
- 证据：`artifacts/desktop-image-resource-gate/20260722-final/summary.json`。
- 下一双端一致性任务：保持移动端冻结，先完成桌面音频 20 分钟 / 512 MiB 边界。

## 2026-07-22 V3 图片空间恢复桌面正式链路接入

- 状态：`桌面正式写入/只读验证已接入并通过安装版 Gate；移动端继续冻结；不得形成跨端正式承诺`。
- 共享核心 `spatial-recovery-v1` 已接入桌面正式 V3 图片链路，载荷继续使用 V3/39，布局版本与载荷协议版本分离；不读取或写入 V2。
- 1920×1080 的十六宫格真实裁切 `16 / 16` 均恢复同一 V3 UID；五档尺寸的四分之一宽 × 四分之一高滑动窗口几何模拟均为零缺口。
- 当前共享核心已在不改变位置推导、`HSR1` layout ID 和 V3 UID 的前提下替换为局部 Haar 变换域候选；十六宫格 `16 / 16`、36 个关键边界滑动 `1/16` 真实裁切、四类干净图误报和 PNG→JPEG/WebP 重编码恢复 Gate 均通过。
- 桌面安装版三张真实摄影照片 PSNR `44.19–51.59 dB`、SSIM `0.9952–0.9982`，每张十六宫格 `16 / 16` 与滑动裁切 `36 / 36` 均通过；近 100 MP 样本约 `20.61 分钟`、峰值 `6.58 GiB` 并通过双重只读验证。
- 风险：当前移动端冻结；桌面仍缺缩放、旋转、严重重压缩和统计规模误报证据，不能据此形成移动端或跨端裁切恢复承诺。
- 证据：`artifacts/desktop-image-spatial-recovery-gate/20260722-local-transform/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-visual/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-spatial-100mp/summary.json`。
- 下一双端任务：保持移动端不动，先完成桌面扩大扰动与误报 Gate；桌面边界评审通过后，再单独规划移动端只读适配和跨端 fixture。

## 2026-07-22 桌面 V3 图片空间恢复边界闭合

- 桌面正式写入、写后回读、只读验证、版权库修复和独立 QA 读取继续统一调用 `watermark-core::WatermarkService`；桌面未复制布局、UID、排列或纠错算法。
- 最终安装版三张真实照片分别通过十六宫格 `16/16`、关键滑动裁切 `36/36`、90/180/270 度旋转、85% 缩放、JPEG/WebP quality 75/60 共 `8/8`；PSNR `44.11–51.29 dB`、SSIM `0.9951–0.9981`。
- 34 个 Windows 内置图片源生成三格式 `102` 个干净变体，误报 `0`；近 100 MP 样本写读、资源与超限拒绝 Gate 通过。
- 本轮只升级桌面产品口径。移动端保持冻结，没有新增移动 fixture、UI 文案或发布承诺，因此这不是双端一致性完成声明。
- 移动端恢复开发时，必须直接消费当前 `watermark-core` 布局，并补桌面写/移动读、移动写/桌面读的裁切、旋转、缩放和重编码 fixture；在此之前不得把桌面空间恢复能力写入移动端销售或帮助文案。
- 证据：`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final-installed/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-image-complete-final/false-positive-summary.json`。

下一双端一致性任务：

- 继续冻结移动端；先执行桌面内部 RC 评审。未来恢复移动端时，以当前 V3 图片 fixture 建立双向跨端读取 Gate，不重新实现图片算法。

## 2026-07-22 桌面音频资源 Gate 收口

- 状态：桌面完成；移动端冻结。
- 桌面新增精确音频资源边界：`30 秒–20 分钟`、`≤512 MiB`、`8–48 kHz`、mono / stereo，并保持原采样率与声道。
- 安装版通过精确 `20:00` 与 `512 MiB` 写读验证；`20:01` 与 `512 MiB + 1 byte` 均在桌面执行入口拒绝且不创建版权记录。
- 图片 `spatial-recovery-v1` 算法和产品口径保持冻结；音频继续使用独立时频 / QIM 链路，不引入图片承载算法。
- 双端风险：移动端尚未执行相同资源样本，因此 UI、帮助、报告和销售文案不得暗示移动端具备相同 `20 分钟 / 512 MiB` 能力。
- 验证：`artifacts/desktop-audio-resource-gate/20260722-final-v2/summary.json`。

下一双端一致性任务：

- 保持移动端代码冻结，仅在桌面内部 RC 通过后，为移动端单独制定资源 Gate 和设备分级方案。

## 2026-07-22 桌面媒体内部 RC 阻断

- 最终桌面候选的图片 WebP q60 恢复出现错误 UID，桌面媒体 RC 不通过。
- 移动端继续冻结，不接收本轮桌面代码、fixture、UI 文案或产品承诺。
- WebP q60 不得出现在任何双端一致性完成声明中；未来移动端恢复时也不能直接继承该边界。
- 音频桌面资源 Gate 和高位深 Gate 继续保留，但五格式最终安装版 Gate 与合法上包络组合仍需补齐。
- 统一证据：`docs/桌面媒体内部RC评审.md`、`artifacts/desktop-media-internal-rc/20260722/summary.json`。

下一双端一致性任务：

- 保持移动端冻结；先关闭桌面 WebP q60 错误 UID 阻断并完成同一候选复验。

## 2026-07-22 RC-MEDIA-001 桌面修复进展

- 桌面共享核心已将 `spatial-recovery-v1` 精确读取改为共识优先，固定真实照片三 UID WebP q60 回归 `3/3` 通过，安装版综合图片 Gate 通过。
- 桌面 WebP q60 产品边界保持；移动端继续冻结，不继承本次代码、fixture、证据或承诺。
- 本次没有引入桌面私有算法，诊断与修复均位于 `watermark-core`；未来恢复移动端时仍需重新执行跨端写读与同一扰动边界 Gate。
- 证据：`artifacts/image-webp-q60-uid-regression/20260722-green/summary.json`、`artifacts/desktop-image-spatial-recovery-gate/20260722-webp-q60-core-fix-installed/summary.json`。

下一双端一致性任务：

- 保持移动端冻结；桌面先完成 RC-MEDIA-001 的三照片 × 三 UID × 八变换完整关闭矩阵。

## 2026-07-22 RC-MEDIA-001 关闭

- 桌面三照片 × 三独立 UID × 八变换矩阵 `72/72` 通过，`RC-MEDIA-001` 已关闭。
- 修复与关闭 Gate 均依赖共享 `watermark-core`，没有桌面私有 UID 修正或格式特判。
- 移动端继续冻结，不继承桌面 WebP q60 证据；未来恢复移动端时仍需单独完成跨端写读矩阵。
- 证据：`artifacts/desktop-media-internal-rc/20260722/rc-media-001-closure.json`。

下一双端一致性任务：

- 保持移动端冻结；桌面转向 `RC-MEDIA-002` 默认核心测试清理，不新增跨端产品承诺。

## 2026-07-22 RC-MEDIA-002 V3-only 收口

- 状态：`CLOSED`。默认 `watermark-core` release suite 为 `108 passed / 0 failed`，正式 V3 图片服务测试 `5/5`。
- 图片协议边界：正式图片只支持 V3/39；V2 图片写读和 rollback 统一返回 `v2_image_rollback_retired`，桌面消费方不得保留私有 V2 兜底。
- 测试治理：六项旧 V2 图片测试已从默认套件移除；`npm run watermark:legacy-rollback-suite` 只验证图片拒绝合同和隔离音频 legacy rollback。
- 移动端继续冻结，不因本次桌面 RC 关闭自动获得图片或音频承诺。
- 下一双端一致性任务：桌面先完成 `RC-MEDIA-003`；移动端恢复前必须以同一 V3-only 图片合同重新建立跨端 fixture。

## 2026-07-22 RC-MEDIA-003 桌面安装版音频矩阵

- 状态：`CLOSED`。桌面最终安装候选完成 WAV / MP3 / FLAC / OGG / M4A × mono / stereo `10/10`。
- 所有单元保持 48 kHz 与原声道，写后回读、独立核心读取、安装版只读验证和 V3 UID 精确一致。
- 输出统一为 WAV；无损输入保持有效位深，有损输入不承诺保持源容器或编码。
- 移动端继续冻结，本次证据不自动形成移动端五格式产品承诺。
- 下一双端一致性任务：桌面完成 `RC-MEDIA-004`；未来恢复移动端时复用同一 fixture manifest 和共享核心读取合同。

## 2026-07-22 RC-MEDIA-004 桌面音频上包络

- 状态：`CLOSED`。最终安装候选通过 `20 分钟 / 48 kHz / stereo / 24-bit` 写读与规格保持。
- 完成链路约 `57.5 秒`，主进程峰值约 `1.215 GiB`；取消不落库，但约 `45.8 秒`后才达到 CPU 静默。
- 桌面取消文案只能承诺“已取消且不会创建版权记录”，不能承诺底层资源瞬时释放。
- 移动端继续冻结，不继承该 20 分钟高位深资源承诺。
- 下一双端一致性任务：保持移动端冻结，桌面进入 `RC-RELEASE-001` 签名候选验证。

## 2026-07-22 RC-RELEASE-001 桌面签名候选

- 状态：`CLOSED`。桌面 NSIS、MSI、release EXE 和当前 installed EXE 四文件签名 Gate 通过。
- 本轮是 Windows 桌面平台特定发布能力，不改变移动端冻结状态，也不形成 Android / iOS 签名承诺。
- 自签证书只在受管 trust store 中受信，不能对双端用户表述为公共 CA 信任。
- 下一双端一致性任务：保持移动端冻结，桌面执行 `RC-RELEASE-002` 干净离线 Windows 安装和媒体验证。

## 2026-07-22 RC-RELEASE-002 挂起与桌面签名后冒烟

- `RC-RELEASE-002` 按用户决定暂时挂起，但继续阻断桌面内部 RC 和对外发布。
- 桌面同一已签名 installed EXE 已通过图片 PNG / JPEG / WebP `3/3` 和音频五格式 × mono / stereo `10/10` 冒烟；该证据不外推到移动端。
- 移动端继续冻结，不接收桌面签名、安装器、WebView2 或本轮媒体证据，也不形成移动端发布承诺。
- 下一双端一致性任务：保持移动端冻结，先执行桌面 `0.1.0` RC 证据索引完整性审计；`RC-RELEASE-002` 恢复前不得将桌面 RC 标记为通过。

## 2026-07-23 桌面 NSIS 安装负载签名失败

- 桌面 current-user NSIS 安装已确认外层安装器签名有效、快捷方式和卸载项正常，但新安装应用 EXE 为 `NotSigned`。
- 当前桌面候选拒绝发布；移动端继续冻结，不继承任何桌面签名或安装结论。
- 下一双端一致性任务：保持移动端冻结，等待桌面下一候选完成内层 EXE 预签名和新安装 EXE 验签后，再评估桌面 RC 恢复条件。

## 2026-07-26 独立感知质量实验室平台边界

- 新增 `tools/perceptual-quality-lab` Windows 独立内部工具，用于手动图片 / 音频前后素材比较和单人 ABX；它不接入桌面主程序，也不属于移动端能力。
- 质量指标来自 `watermark-core::quality`，没有新增桌面专属或移动端专属水印算法、payload、验证规则、版权记录或报告字段。
- 移动端继续冻结；本任务不要求移动端实现文件选择、热力图、FFmpeg 对齐或 ABX 页面，也不得把 Windows 工具结果包装成双端用户承诺。
- 用户可见边界固定为：独立实验室只供内部测试，主程序和移动端当前能力口径不变。
- 验证：独立前端构建、ABX `3/3` 测试、Rust 媒体辅助 `2/2` 测试和共享 quality `3/3` 测试通过。
- release/full gate 当前仍为图片 / `field-noise` 客观指标阻断；该结果不外推到移动端，也不能由 Windows 单人 ABX 放行。
- 下一双端一致性任务：保持移动端冻结，先固定并修复桌面 quality gate 当前图片阻断，再复跑既有跨端 fixture，确认质量 API 重构没有影响正式双端互读事实。
## 2026-07-26 内容身份与公共信任层待办挂起

- 已新增 `docs/内容身份与公共信任层待办.md`，完整记录 V3 最小锚点之上的公共信任层问题、建议目标架构、未来实施顺序、跨端验收要求、风险和恢复条件。
- 状态：`suspended_by_user`。本轮只更新文档，不修改桌面、移动、后端、`watermark-core`、数据库、API、SDK、fixture、报告或用户文案。
- 双端一致性边界保持不变：Watermark ID 格式不变，图片 / 音频 V3/39 默认 payload 不变，桌面写移动读和移动写桌面读仍是未来任何信任层工作的发布门禁。
- 当前风险：payload HMAC auth tag 只能表达载荷认证，不能表达第三方可独立验证的 issuer 签名；现有 registry receipt 和公开 HMAC manifest 原型不能包装为生产非对称信任链。
- 待办已补充“V3 当前版本的进化与优化方向”：冻结 V3/39 媒体锚点，分别版本化 credential、rights manifest、evidence、issuer/key document 和 verification result；未来优化优先发生在媒体外信任层，不通过增加默认 payload flags 推进。
- 待办已补充“候选架构公理审查”和“内容摘要与派生关系待冻结问题”：六条候选公理均按当前实现修正后记录；精确字节 SHA-256、未来 canonical digest、感知指纹和显式 derivation relation 必须分离，任何未来内容绑定合同都需保持桌面 / 移动 / 后端语义一致。
- 验证结果：文档级记录，无运行态或算法变更，因此未新增跨端测试；既有跨端正式能力与发布状态不因本待办改变。
- 下一双端一致性任务：继续执行当前已排定的 RC / GA 和质量门禁任务；只有待办恢复后，才先冻结 Rust / TypeScript / Dart 共用的 Signed Registry Credential canonical fixture 和 Trust Status Vocabulary。

## 2026-07-26 独立实验室预览与指标说明修正

- Windows 独立内部工具的图片预览改为 PNG data URL，分割模式改为完整图层裁剪，修复同步观察无法正确加载或对齐的问题。
- 图片 / 音频指标新增面向一般测试者的含义、好坏方向和参考区间；这些说明不进入桌面主程序或移动端正式产品文案。
- 本轮没有新增桌面或移动水印能力，没有形成平台差异承诺，也没有改变移动端冻结状态。
- 指标解释继续引用同一 `watermark-core::quality` 口径；诊断区间不替代跨端 fixture、release/full gate 或至少五人的正式感知验收。

下一双端一致性任务：

- 在 Windows 独立工具完成真实图片 / 音频交互验收后，复跑既有图片与音频跨端 fixture，确认本轮展示层修正与正式双端互读链路完全隔离。

## 2026-07-26 Windows 独立实验室 ABX 图片修正

- Windows 独立内部工具的图片 ABX A / B / X 改为 PNG data URL，修复本地 asset URL 导致的空白预览。
- 该修正不进入 HiddenShield 桌面主程序或移动端，不形成新的平台能力承诺。
- 水印写读、payload、验证、质量 gate 和移动端冻结状态均不变。

下一双端一致性任务：

- 完成 Windows 图片 ABX 10 轮真实交互回归后，继续执行既定跨端 fixture 与质量 gate，不将实验室显示结果外推为双端能力。

## 2026-07-26 Windows ABX Blob v2 修正

- Windows 独立实验室 ABX 图片改为复用同步观察预览并创建前端 Blob URL，替代真实验收仍失败的独立 data URL 资源链路。
- 本轮仍是内部 Windows 工具展示修正，不涉及桌面主程序、移动端、共享水印算法或跨端能力承诺。
- 下一双端一致性任务：先完成新 EXE 的 PNG / JPEG ABX 真实回归，再继续既定跨端 fixture 与质量 gate。

## 2026-07-26 Windows ABX 答题与音频播放修正

- Windows 独立实验室新增粘滞答题栏和自适应观察区，并将音频 ABX 改为内存 WAV Blob 播放。
- 本轮不进入桌面主程序或移动端，不形成新的双端功能承诺；音频写读、payload、验证和移动端冻结状态均不变。
- 下一双端一致性任务：完成图片 / 音频各 10 轮 Windows ABX 回归后，继续既定跨端 fixture 与质量 gate。

## 2026-07-26 Windows 感知质量实验室基线冻结

- `20260726-abx-v3` 已冻结为 Windows 独立实验室首个内部可用基线，人工验收确认整体通过。
- 该基线不属于桌面主程序或移动端正式能力，不改变移动端冻结状态。
- 后续实验室或共享质量变更必须复跑图片 10 轮与音频 10 轮 ABX，并使用新目录冻结下一基线。
- 下一双端一致性任务：保持本基线只用于内部质量验证，继续既定跨端 fixture 与正式质量 gate。

## 2026-07-26 桌面发布措辞审计与移动端边界

- 桌面验证、版权库、报告和帮助文案已明确认证标签、编号签发、完整性校验、时间材料和法律结论边界。
- 本轮不修改移动端运行能力、共享字段、payload、报告合同或跨端读取；移动端继续冻结。
- 桌面“版权编号签发”仍保留为 Watermark ID 生成 / 分配术语，但帮助与冲突修复文案明确它不代表数字签名。
- 桌面 RC / GA 仍需提升权限、无预装 WebView2 的干净 Windows 证据；该平台验收不外推到移动端。
- 下一双端一致性任务：完成桌面干净 Windows Gate 后，再复跑 `dual:contract`，确认本轮纯文案调整未改变双端合同。

## 2026-07-26 桌面 Windows 发布 Gate 通过

- Windows QA operator 已在提升权限环境完成桌面 `v0.1.3` 的 installed-payload 验签，并在物理断网条件下完成图片与 WAV / MP3 / FLAC / M4A / AAC 五种音频格式冒烟。
- 本轮按 release owner 指令采用操作员验收声明，不继续追踪或独立复核产物。
- `RC-RELEASE-002` 已关闭，桌面内部 RC 与桌面 GA 发布 Gate 改为 `PASSED`。
- 本次结论只关闭 Windows 桌面发布环境 Gate，不代表移动端解冻，也不新增桌面专属水印算法、payload、报告字段或跨端能力承诺。
- 公共信任层继续挂起；图片 / 音频跨端互验和共享核心单一事实源约束保持不变。
- 下一双端一致性任务：在不修改桌面已通过候选的前提下复跑 `dual:contract`，将结果附入 GA 发布清单，并继续保持移动端冻结状态。

## 2026-07-26 跨端合同 CI 兼容性修复

- 移动端图片预检已从退役的 V2 导入迁移为优先调用共享 `WatermarkService::extract`，并保留现有只读候选兼容路径；未重新导出 V2 API。
- 桌面 L1 视频音轨固定的 `44.1 kHz / mono` WAV 在 AAC 复用时显式传入 `-ac 1`，兼容 FFmpeg 8 对 `FL` 单声道布局的严格校验。
- 已通过移动端 35 项单测、跨端 release contract、L1 单轮与发布容器矩阵、以及 V3 feature-gate rollback contract。
- 本轮不修改 `watermark-core` 算法、payload、共享字段或移动端冻结状态，也不新增任何平台能力承诺。
- 下一双端一致性任务：等待 GitHub Actions Ubuntu / Windows 矩阵完成后，将通过结果关联到此次 CI 修复提交。

## 2026-07-26 v0.1.3 当前 main 双平台绿色基线

- GitHub Actions 运行 `30218394724` 在 `main` 提交 `5a9769ba0e865adb29e7849c3681c3ddec52c254` 上完成，Windows、Ubuntu 和云同步合同三个 Job 均为 `success`。
- Windows Job `89836421460` 与 Ubuntu Job `89836421478` 均通过前端构建、双端一致性合同、共享水印架构合同、跨端互验合同、视频分层合同、桌面 Rust 测试、离线许可 CI、Flutter analyze 和 Flutter tests。
- 云同步合同与 E2E Job `89836421470` 通过；本次结果关闭上一节“等待 GitHub Actions 矩阵”的待办。
- 该运行已写入 `docs/桌面v0.1.3发布清单.md`，作为 `v0.1.3` 发布后的当前 `main` 双平台绿色基线，不替代 Windows installed-payload 验签和物理断网媒体冒烟。
- 本轮只冻结 CI 证据，不改变移动端冻结状态、桌面或移动端能力承诺、共享字段、payload、算法或跨端读取边界。
- 下一双端一致性任务：任何候选重建或双端合同变更都必须取得新的 Ubuntu / Windows 全绿运行，并与 `30218394724` 对比后追加记录。

## 2026-07-27 AI 图片平台生成时标识 MVP 双端一致性约束

- 状态：`design_frozen`。AI 图片平台生成时标识的主交付面是平台 SDK、后端 API 和验证接口；本轮不新增桌面或移动端生成入口，不解除移动端冻结。
- 统一字段：后续桌面、Android 和 iOS 只读查看必须使用同一组 AI 来源字段、Evidence 等级、锚点状态、元数据签名状态、issuer 信任状态、Profile 状态、warning 和 `legalConclusion=false`。
- 统一文案：三端都必须区分“平台签名声明”“Registry 签名声明”“用户自声明”“不支持的证据”“无效证据”和“未发现支持的标识”；未发现标识不得表述为人工创作或非 AI。
- 正式媒体约束：平台 SDK 写入的 AI 图片正式锚点必须由 `watermark-core` 写入，并覆盖平台写入 -> 桌面读取、平台写入 -> Android 读取、平台写入 -> iOS 读取；Android QA 不得替代 iOS QA。
- 存储与同步：AI Transparency Manifest、Evidence 和 Marker Binding 在进入正式端点前必须定义本地版权库、云同步白名单、报告摘要和隐私边界；不得只在平台 SDK 返回值或单端 UI state 中存在。
- 下一双端一致性任务：在实现前冻结 AI 来源只读字段的跨端 schema fixture，并将平台写入图片加入现有图片跨端互验矩阵。

## 2026-07-27 Internal AI Image Executor 跨端前置证据

- 已完成：backend internal-only 图片 executor 已通过 `watermark-core` V3 写入与同核心回读，再以 PostgreSQL confirm 固化 AI Transparency 记录；未修改 desktop、Android 或 iOS 代码。
- 一致性边界：executor 返回的 PNG 保护副本必须作为“平台写入” fixture，供桌面、Android 和 iOS 调用各自正式读取路径；本轮 backend QA 不能替代任何端侧读取证据。
- 用户可见边界：当前不向任一端承诺平台生成时标识、UI 标签渲染、公共验证或法规合规。
- 下一双端一致性任务：新增平台写入 PNG fixture 与 desktop/Android/iOS 读取结果合同，覆盖 V3 UID、auth status、metadata 缺失和 `legalConclusion=false`。

## 2026-07-27 平台 Executor PNG 跨端 Fixture

- 状态：`desktop_and_shared_mobile_bridge_verified_ios_runtime_pending`。
- 已完成：冻结 Executor 输出 PNG、含测试 metadata 与 metadata-stripped fixture；Desktop 正式读取代码路径和 Android/iOS 共用 mobile Rust bridge 均读取同一 V3/39 UID 与 auth 结果。
- 一致性边界：metadata 剥离验证盲水印不依赖 PNG metadata；`legalConclusion=false` 固定不变，未发现或无效标识不得反推非 AI。
- 未完成：Android/iOS 产品 UI 均未新增入口；尤其 iOS 尚无 macOS/iOS runtime 证据，共用 bridge 的宿主测试不能替代 iOS QA。
- Gate：SDK、公共 Resolver、production credential 和生产发放继续关闭。
- 下一双端一致性任务：在 macOS/iOS runtime 复跑冻结 fixture，并归档原始与 metadata-stripped 两类读取结果。

## 2026-07-27 iOS Runtime Gate 挂起

- 状态：`suspended_external_environment`。
- 原因：当前没有可用的 macOS/iOS runtime，无法将共用 mobile Rust bridge 宿主测试升级为 iOS 实际运行时或设备证据。
- 保留证据：Desktop 与共用 mobile bridge fixture 测试继续作为内部回归 Gate，但不得降低或替代 iOS runtime Gate。
- 解挂条件：提供可执行 iOS runtime 的 macOS CI、模拟器或设备环境，并复跑原始与 metadata-stripped PNG fixture。
- 可并行任务：继续冻结并验证不依赖 iOS runtime 的第三方 PNG 元数据共存与剥离互操作合同；SDK、公共 Resolver 与生产发放继续关闭。

## 2026-07-27 第三方 PNG 元数据共存内部互验

- 已完成：Desktop 正式读取路径与 Android/iOS 共用 mobile Rust bridge 均读取 external metadata 共存和 stripped fixture，并得到同一 V3/39 UID/auth。
- 边界：metadata fixture 明确为 `untrusted`，不代表 C2PA、平台签名或第三方数字水印；iOS runtime 的实际环境 Gate 仍挂起。
- 下一双端一致性任务：取得真实第三方可再分发参考样本后，按其允许的处理链运行互操作 Benchmark；SDK、公共 Resolver 与生产发放继续关闭。

## 2026-07-27 第三方公开 C2PA Metadata Benchmark

- 已完成：桌面内部 QA 使用公开 Apache-2.0 C2PA fixture 验证 manifest 可读取且不会被归类为 HiddenShield V3 anchor。
- 未覆盖：此项未接入 Android 或 iOS 读取 UI；iOS runtime Gate 仍为外部环境挂起，不能以桌面 C2PA Reader 代替。
- 下一双端一致性任务：取得适合移动端再分发的第三方样本和处理链许可后，定义移动端只读显示与跨端结果矩阵。

## 2026-07-27 第三方视觉水印与 V3 内部子矩阵

- 已完成：桌面共享核心对 MIT 许可的外部视觉水印样本完成“写前无 V3 → V3 写入 → 写后 verified 回读”。
- 边界：该视觉水印样本与公开 C2PA fixture 不同资产，未产生 Android/iOS 结果；iOS runtime Gate 持续挂起。
- 下一双端一致性任务：取得同资产可再分发样本后，定义 Desktop/Android/iOS 的只读 V3 与 C2PA/视觉水印状态矩阵；SDK、公共 Resolver 与生产发放继续关闭。

## 2026-07-27 最终 PNG C2PA 状态一致性

- 已完成：内部 QA 将最终 PNG 明确分类为 `manifest_absent_after_png_reencode`，同时 V3 为 verified。
- 一致性要求：未来 Desktop/Android/iOS 必须分别展示 C2PA 状态与 V3 状态，不得因 V3 成功隐藏 C2PA 缺失。
- Gate：iOS runtime 继续挂起；post-embed resign 或兼容容器通过前，不开放三层对外能力。

## 2026-07-27 Post-Embed 双层读取内部证据

- 已完成：桌面内部 QA 对同一最终 PNG 同时读取 active C2PA manifest 与 verified V3。
- 一致性边界：Android/iOS 尚未读取 post-embed 最终 fixture；iOS runtime 继续挂起，桌面证据不得替代移动端。
- 下一双端一致性任务：冻结 post-embed 最终 PNG fixture，并加入 Desktop/Android 共用 bridge 读取；iOS runtime 继续保持挂起。

## 2026-07-27 Production Post-Embed Command 双端合同

- 已冻结：最终签名 PNG 的 C2PA 状态与 V3 状态必须作为独立字段进入未来 Desktop/Android/iOS 只读结果，任一失败不得被另一层成功遮蔽。
- final hash 必须是三端报告、同步和验证引用的唯一交付文件 hash；unsigned V3 hash 仅供内部 signer audit。
- Gate：本轮不新增端侧入口；iOS runtime 继续挂起，production command 与 SDK 继续关闭。

## 2026-07-27 Post-Embed Signing Schema 一致性 Gate

- 已冻结：最终结果必须独立携带 C2PA readback、V3 readback、final signed hash、signer receipt reference 和 `legalConclusion=false`。
- 七类 fixture 已覆盖成功和失败语义；本轮不新增 Desktop/Android/iOS UI。
- Gate：iOS runtime 继续挂起；internal command 未实现前不生成新的跨端正式 fixture。

## 2026-07-27 Internal Post-Embed Signing PostgreSQL Gate

- backend internal-only command 已完成 PostgreSQL success、四类签发/回读拒绝、confirm rollback/orphan-signing 和 duplicate replay 七类事务验证。
- 本次未修改 Desktop、Android、iOS UI、vault、报告字段或公开验证措辞；`watermark-core` 仍是 V3 写入/读取唯一算法源，backend 不新增第二套水印实现。
- 当前事务 QA 的 C2PA/V3 readback 为受控 provider interface；它验证命令编排和数据库原子性，不替代既有真实媒体 post-embed prototype，也不构成 iOS runtime 证据。
- iOS/macOS runtime Gate 继续按环境依赖挂起；SDK、公共 Resolver、production credential 与跨端产品承诺保持关闭。
- 下一双端任务：在 durable artifact finalize Gate 完成后，冻结成功 signer 输出的跨端 fixture 交付合同；待 iOS runtime 可用时复跑 Desktop/Android/iOS 对同一最终 PNG 的 C2PA 状态与 V3 UID/auth 读取。

## 2026-07-27 Signing Reservation 与 Artifact Recovery 双端边界

- backend 已保证 artifact finalize 完成前不返回最终 PNG；Desktop/Android/iOS 未来只能接收 execution `confirmed`、artifact `finalized` 且 final hash 匹配的产物。
- `artifact_pending`、`reserved`、`signed_staged` 和 `orphaned` 均为内部状态，不得进入端侧 vault、报告、同步或用户成功提示。
- 本次未修改三端读取器或 UI；九类 fixture 是 backend 合同与事务证据，不替代最终 PNG 跨端媒体读取 fixture。
- iOS runtime 继续挂起，SDK、公共 Resolver 与 production credential 继续关闭。
- 下一双端任务：完成四崩溃点恢复 Gate 后，冻结仅含 `confirmed/finalized` 产物的跨端交付 fixture，并复跑 Desktop/mobile Rust bridge。

## 2026-07-28 Adapter Receipt 与崩溃恢复双端边界

- backend 已证明四个崩溃窗口不会把 `reserved`、`signed_staged` 或 `artifact_pending` 误交付为成功产物；端侧未来仍只允许消费 `confirmed/finalized` 且 final hash 与 finalize receipt 一致的 PNG。
- signer/object-store receipt 为内部信任与恢复证据，不进入 Desktop/Android/iOS vault 的用户可编辑字段，不作为用户可见法规结论。
- 本次未修改 Desktop、Android、iOS reader、UI 或报告；十三场景 QA 只证明 backend orchestration、外部成本幂等模拟和 PostgreSQL 原子性。
- iOS/macOS runtime 继续作为环境依赖挂起；SDK、公共 Resolver、production credential 和跨端发布保持关闭。
- 下一双端任务：冻结 `confirmed/finalized` 交付 envelope fixture，绑定 final hash、signer receipt ref、artifact finalize receipt ref，并在 Desktop/mobile Rust bridge 验证拒绝非 finalized 状态。

## 2026-07-28 Internal Recovery Worker 双端边界

- recovery worker 只改变 backend internal execution 状态，不新增 Desktop、Android、iOS UI、vault、报告或同步字段。
- `eligible/leased/retry_scheduled/dead_letter` 均为内部运维状态，不得同步到端侧或显示为用户成功/失败结论。
- 只有 worker 最终恢复到 `confirmed/finalized` 的产物才可进入未来交付 envelope；dead-letter 不返回媒体，不进入客户成功计量。
- iOS/macOS runtime 继续作为环境依赖挂起；SDK、公共 Resolver、production credential 和跨端发布保持关闭。
- 下一双端任务：实现 backend `confirmed/finalized` delivery envelope contract，并让 Desktop/mobile bridge fail-closed 拒绝 recovery 非 completed 状态。

## 2026-07-28 Dead-Letter Inspect / Requeue 双端边界

- inspect/requeue 全部是 backend internal 运维能力，不新增 Desktop、Android、iOS UI、vault、报告或同步字段。
- `dead_letter`、change request、approval、execution 和 inspection audit 不得被端侧解释为媒体真实性、法规结论或用户可操作状态。
- requeue 后只有恢复至 `confirmed/finalized` 且 final hash、signer receipt 和 artifact finalize receipt 一致的产物，才可进入未来跨端交付。
- 本次未修改端侧 reader 或共享核心；iOS/macOS runtime 继续作为环境依赖挂起。
- SDK、公共 Resolver、production credential、客户自助 requeue 和跨端发布继续关闭。
- 下一双端任务：实现统一 delivery envelope fixture，并让 Desktop/mobile Rust bridge 拒绝非 finalized、recovery 未 completed、hash mismatch 或 receipt mismatch。

## 2026-07-28 Confirmed / Finalized Delivery Envelope 双端 Gate

- Desktop 与 mobile Rust bridge 已接入 `watermark-core::validate_ai_delivery_envelope`，不各自实现状态或摘要规则。
- 两端共同读取 `success-v1.fixture.json`，对相同 final bytes、signer receipt、finalize receipt 和 Profile identity 得到一致接受结果。
- Desktop 已覆盖 artifact 非 finalized、media hash mismatch、signer receipt mismatch；mobile 已覆盖 recovery 非 completed、finalize receipt mismatch、Profile identity mismatch。
- 任一拒绝均不返回 envelope digest、final hash、watermark UID 或 Profile digest，禁止进入未来 vault/import。
- 本次未新增用户 UI、同步字段或公开验证措辞；iOS/macOS runtime 继续作为环境依赖挂起。
- 下一双端任务：冻结 delivery retrieval/import fixture，让 Desktop/mobile 在 artifact 下载后先验证 envelope，再允许写入 vault。

## 2026-07-28 Delivery Retrieval 双端 Import Admission

- Desktop 与 mobile Rust bridge 已新增同语义 import admission，输入均为 envelope、最终 bytes、signer receipt、finalize receipt 和 retrieval receipt。
- 两端均直接调用 `watermark-core::validate_ai_delivery_import`，没有复制 receipt digest、final hash、Profile identity 或状态判断。
- 共享 fixture 同时覆盖成功 admission 与 receipt mismatch；拒绝响应在两端均不暴露 vault/import 所需 ID、摘要或 watermark UID。
- 当前只冻结 bridge Gate，未接入客户 vault/import UI；因此能力仍为 `只能内部测试`，不得形成任一端独有的产品承诺。
- iOS runtime 实机验证继续作为环境阻塞项挂起；Rust mobile bridge 合同测试已通过。
- 下一双端任务：在未来内部下载调用层强制消费 admission result，并为 Desktop/Android 增加 tampered bytes 与 expired authorization 的端到端导入拒绝 fixture。

## 2026-07-28 Delivery Revoke / Resource Budget 双端边界

- 本次预算和 revoke 全部位于 backend 授权/对象读取层，Desktop/mobile import admission 合同未变化。
- 两端仍只能接收 backend 成功返回的完整 retrieval package，并继续调用同一 `validate_ai_delivery_import`。
- revoked、rate limited、size、MIME 或 timeout 失败均无 bytes/package，因此不得在任一端创建 vault/import 临时记录。
- 当前未新增 Desktop/mobile 用户 UI 或错误文案，不形成任一端独有产品承诺；iOS runtime 继续挂起。
- 下一双端任务：未来接入内部下载调用层时，为五类新失败码冻结一致的不可导入状态和用户不可见内部诊断映射。

## 2026-07-28 Delivery Security Observability 双端边界

- monitoring、aggregate audit export、retention 和 cleanup 全部位于 backend，不修改 Desktop/mobile bridge 或 vault/import 合同。
- observability summary 不包含媒体、authorization、delivery envelope 或 watermark 标识，不得进入端侧 vault、报告或同步 payload。
- 外部客户 UI 继续关闭；Desktop/mobile 不显示 internal alert code，避免把内部安全信号误表述为法规或用户结论。
- 下一双端任务保持为未来下载入口的统一不可导入映射；本次无新增双端 runtime 阻塞。

## 2026-07-28 Delivery Security Incident / Cleanup Runner 双端边界

- incident projection、ack/resolve 与 cleanup runner 均为 backend internal orchestration，不新增 Desktop、Android 或 iOS 用户能力。
- 双端不得展示 internal incident status、alert code、审批状态或 runner 状态为水印验证结果、法规结论或交付成功证明。
- 本次未改变 delivery envelope、retrieval receipt、`AiDeliveryImportAdmission` 或 Desktop/mobile bridge 行为，因此无需新增平台分叉。
- iOS runtime Gate 继续按既有状态挂起；该环境限制不阻塞 PostgreSQL incident/runner Gate。
- 下一双端任务保持不变：正式端侧下载/import 入口只能消费共享 bridge 已批准的 `AiDeliveryImportAdmission`。

## 2026-07-28 生产导向 MVP 双端校准

- backend 生产控制面被正式纳入 AI Transparency MVP，但不因此新增 Desktop/mobile 产品承诺。
- 双端继续只承担共享核心读取、delivery envelope/retrieval receipt 校验和 import admission，不复制授权、审批、signing recovery、incident 或 outbox 状态机。
- 平台 SDK/API facade 将作为独立 B2B 接入面实现，不借用 Desktop/mobile UI 作为生产平台控制台。
- 下一双端任务保持：正式端侧下载/import 入口只能消费 `AiDeliveryImportAdmission`；平台 SDK 接入不改变该约束。

## 2026-07-28 Incident Inspect / Notification Outbox 双端边界

- incident inspect/list、outbox、lease 与 replay 全部位于 backend internal orchestration，不新增 Desktop、Android 或 iOS 用户能力。
- outbox payload 不得进入端侧 vault、报告、同步 payload 或水印验证 UI。
- Desktop/mobile 不得把 pending/leased/retry 状态表述为通知成功、媒体真实性或法规结论。
- 本次未改变 delivery envelope、retrieval receipt、`AiDeliveryImportAdmission` 或 bridge，双端无需新增实现。
- iOS runtime Gate 继续挂起，不阻塞 PostgreSQL outbox Gate。
- 下一双端任务保持不变：正式端侧下载/import 入口只能消费共享 bridge 已批准的 `AiDeliveryImportAdmission`。
## 2026-07-28 Platform API 双端边界

- 本次实现位于 backend 与 server SDK，不新增 Desktop、Android 或 iOS 用户界面承诺。
- 正式图片标识仍由共享 `watermark-core` 写入和回读；端侧不得复制 admission、confirm、payload 或摘要算法。
- internal endpoint 返回的 marked PNG 仍须通过既有 Desktop/mobile Rust bridge 校验后才能进入端侧 vault/import。
- iOS runtime Gate 继续挂起；本次 PostgreSQL E2E 不替代跨端 runtime 互验。
- 下一双端任务保持为：公共 Resolver 结果字段与端侧验证措辞使用同一 Profile/manifest/marker 术语，不引入平台专属结论。
## 2026-07-28 免费公共 Resolver 双端措辞

- Resolver 输出固定使用 Manifest、marker、evidence、Profile 和 warning 术语，不引入 Desktop/mobile 专属结论。
- Desktop/mobile 后续展示 `not_found` 时必须使用“未找到 confirmed record，不等于非 AI”的同一措辞。
- `issuerTrustStatus=not_evaluated` 与 `legalConclusion=false` 不得被端侧改写为“可信”“合规”或“人工创作”。
- 本次未新增端侧 runtime；iOS Gate 继续挂起。
- 下一双端任务：设计伙伴样例中的 Resolver link 和端侧查看文案必须复用同一 Schema 字段。
## 2026-07-28 设计伙伴 Sandbox 接入包一致性记录

- 已冻结 server-only SDK/API 示例和匿名 Resolver link contract，不新增 Desktop、Android 或 iOS 独占的标识写入算法或产品承诺。
- 伙伴 mark 路径继续经 backend 调用 `watermark-core`；Desktop/mobile bridge 只消费既有 confirmed/finalized delivery envelope 与公共 Resolver 最小字段。
- Resolver 文案在各端统一保持“已确认标识记录”与 `legalConclusion=false`，不得显示为法律结论、真实性保证或平台背书。
- 当前接入包分类为 `只能内部测试`，真实双端伙伴 import/vault 体验不因该包自动开放。
- iOS runtime Gate 继续作为环境依赖挂起，本任务不以缺失 iOS runtime 阻塞 server-side Sandbox 包。

下一双端任务：首个真实伙伴形成 confirmed PNG fixture 后，将该 fixture 纳入 Desktop/Android 正式读取与 envelope 摘要 fail-closed 回归；iOS 在 runtime 可用后补测。

## 2026-07-28 Synthetic Sandbox QA 一致性边界

- synthetic QA 只验证 server-side SDK/facade 与公共 Resolver 响应 shape，不产出 Desktop、Android 或 iOS 可承诺的 protected-copy fixture。
- synthetic marked PNG bytes 不是 `watermark-core` 写入产物，不得导入端侧 vault、用于跨端读取报告或替代正式 fixture。
- 真实伙伴输出 confirmed PNG 后仍必须执行 Desktop/Android 读取和摘要 fail-closed；iOS runtime Gate 保持挂起。
