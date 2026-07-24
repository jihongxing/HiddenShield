# HiddenShield 生产 C2PA 证书链、TSA、SDK 发布与 Enterprise 客户开通 Runbook

更新时间：2026-06-30

本文是公开权利信号与训练许可扫描进入生产发布前的上线门禁。它把四件事一次性固化：生产 C2PA 证书链与 TSA 配置、公开扫描 SDK 外部分发、Enterprise API 客户开通 / 限额 / SLA、回滚与停用流程。

生产 C2PA 证书申请、CSR、私钥托管、TSA 开通和 secret manager 注入细则见 `docs/生产C2PA证书申请与Secret注入Checklist.md`。生产 trust chain 未通过该 checklist 和 `rights:metadata-embed-production-staging-qa` 前，不得对外宣称“生产 C2PA signed manifest / TSA 已上线”。

本文不改变能力边界：

- 公开元数据和 Enterprise API 只输出 registry 中的作品声明与训练许可声明，不输出法律授权结论。
- `legalConclusion` 必须保持 `false`。
- 图片 PNG / JPEG、WAV / MP4 当前均支持官方 `c2pa` SDK signed manifest 运行态写入；未配置生产证书链 / TSA 时只允许表述为 ephemeral / QA signer 证据，不得宣称生产可信 C2PA trust chain 已上线。
- V3 媒体 payload 只保留最小锚点；完整授权声明继续由版权库 / 云版权库 / registry / 公开元数据层承接。

## 1. 发布判定

公开权利生产发布必须同时满足下列条件：

| 门禁 | 必须满足 | 阻断条件 |
| --- | --- | --- |
| C2PA signer | 生产环境配置 `HIDDENSHIELD_C2PA_SIGN_CERT_PEM`、`HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM`、`HIDDENSHIELD_C2PA_SIGNING_ALG`，推荐配置 `HIDDENSHIELD_C2PA_TSA_URL` | 导出结果仍显示 `ephemeral_development_certificate_not_publicly_trusted` |
| C2PA trust chain | 证书链、私钥用途、有效期、吊销状态、组织标识、TSA 时间戳均由发布负责人复核 | 证书过期、链不完整、私钥来源不明、TSA 不可用 |
| 图片嵌入 QA | `rights:metadata-embed-runtime-qa` 必须确认 PNG / JPEG 的 XMP、APP1/iTXt、C2PA active manifest、`watermarkUid`、`manifestHash`、`legalConclusion=false` | 任一格式缺 manifest、UID 或法律边界字段 |
| 音视频 C2PA QA | `rights:metadata-embed-av-runtime-qa` 必须确认 WAV / MP4 存在 C2PA active manifest、WAV `hsPM` / MP4 `uuid` 传播层和 `legalConclusion=false` | 缺 active manifest，或把 ephemeral QA signer 写成生产 trust chain |
| SDK 发布 | SDK 包通过合同、typecheck、包内容预检、README 边界检查和版本号检查 | README 没有声明 `legalConclusion=false` 或包内暴露写入 / 回填 / 撤销能力 |
| Enterprise API | 客户只使用 `POST /v1/enterprise/public-rights/batch`；客户侧 key 管理和 quota 管理路由仍不开放 | 出现 `/v1/enterprise/api-keys` 或 `/v1/enterprise/quotas` 客户路由 |
| Quota / audit | 每个客户都有 active API key、`public_rights:batch_read` scope、`api_access=true`、active quota balance、rate-limit policy、quota ledger 和 API audit | dry-run 通过但真实 quota balance 未初始化 |
| SLA / rollback | 客户合同、运维值班、错误码、限流、暂停 / revoke / rotate / revoke-expired 流程完成演练 | 无法在 15 分钟内暂停 key 或关闭客户访问 |

## 2. 生产 C2PA 证书链与 TSA

### 2.1 环境变量

生产环境必须由受控 secret manager 注入以下变量：

| 变量 | 必填 | 说明 |
| --- | --- | --- |
| `HIDDENSHIELD_C2PA_SIGN_CERT_PEM` | 是 | C2PA signer 证书链 PEM，必须包含可验证链路所需证书。 |
| `HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM` | 是 | 与证书匹配的私钥 PEM；不得写入仓库、日志、QA evidence 或桌面导出结果。 |
| `HIDDENSHIELD_C2PA_SIGNING_ALG` | 是 | 当前支持 `Ed25519`、`ES256`、`ES384`、`ES512`、`PS256`、`PS384`、`PS512`。 |
| `HIDDENSHIELD_C2PA_TSA_URL` | 强烈建议 | RFC 3161 TSA 地址；未配置时不能宣称生产可信时间戳链路已完成。 |

发布负责人必须记录：

- 证书签发方、主题、组织标识、用途、notBefore / notAfter。
- 私钥托管位置和访问审批记录。
- TSA 服务方、SLA、失败重试策略和告警入口。
- 证书吊销检查方式。

### 2.2 发布前命令

在配置生产 signer 的同一运行环境中执行：

```powershell
npm run rights:metadata-embed-contract
npm run rights:metadata-embed-runtime-qa
npm run rights:metadata-embed-production-staging-qa
```

验收要求：

- PNG / JPEG 都存在 C2PA active manifest。
- 导出结果不得出现 `ephemeral_development_certificate_not_publicly_trusted`。
- `rights:metadata-embed-production-staging-qa` 必须确认 `c2paSignerStatus` 为 `configured_certificate_chain`；普通 `rights:metadata-embed-runtime-qa` 只能证明 active manifest 可写可读，不能单独证明生产信任链通过。
- `legalConclusion=false` 必须仍然存在。

如果生产 signer 不可用，允许继续发布 registry / sidecar / JSON 导出，但必须禁用“生产 C2PA signed manifest 已上线”的对外描述。

### 2.3 证书轮换

证书轮换必须走双签或灰度期：

1. 先部署新证书到 staging，运行图片嵌入 QA。
2. 抽样导出 PNG / JPEG，使用 `c2pa::Reader` 或等价工具确认 active manifest 可读。
3. 生产切换后保留旧证书只读验证材料，不再用于新签。
4. 记录轮换时间、旧证书停用时间、验证证据路径。

回滚条件：

- 新证书导出的 manifest 不可读。
- TSA 超时导致导出失败率超过约定阈值。
- 证书链被外部验证工具标记为不可受信。

## 3. SDK 外部分发门禁

当前外部分发包为 `packages/public-rights-sdk`。

### 3.1 包能力边界

SDK 只允许提供：

- `createPublicRightsScanner`
- `scanOne`
- `scanBatch`
- `resolvePolicy`
- `formatUserMessage`

SDK 不允许提供：

- registry 写入、回填、撤销、替代、重签。
- 媒体文件盲水印读取 / 写入。
- C2PA / IPTC / XMP 媒体嵌入写入；音视频当前支持 WAV / MP4 官方 C2PA signed manifest 运行态证据，移动端仍只做 PNG / JPEG 传播层嵌入。
- 法律授权判断。

SDK 输出必须保持：

- `legalConclusion=false`
- `canTreatAsTrainingAllowed=false`
- Enterprise API key 只用于调用 `POST /v1/enterprise/public-rights/batch`

### 3.2 发布前命令

```powershell
npm run rights:sdk-package-contract
npm --prefix packages/public-rights-sdk run typecheck
npm run rights:sdk-pack-dry-run
```

发布检查：

- `packages/public-rights-sdk/README.md` 必须写明不是法律授权结论。
- `package.json` 的版本号必须由发布负责人确认。
- `exports` 和 `types` 必须存在。
- dry-run 包内容必须包含 `dist/index.js`、`dist/index.d.ts`、README 和 package.json。
- dry-run 包内容不得包含测试私钥、证书、临时 QA evidence、数据库、内部 token、仓库根目录源码、`tmp-ui-qa`、`feedback-backend`、`mobile_app`、`src-tauri` 或 `watermark-core`。

发布后验证：

```powershell
npm view @hiddenshield/public-rights-sdk version
```

当前 SDK 状态必须按 `not published` 处理，直到发布负责人完成 npm 发布和发布后验证。如果 npm 发布失败，不得把 SDK 写成“外部已发布”；只能表述为“仓库内分发包已准备”。

## 4. Enterprise 客户开通流程

### 4.1 客户准入

客户开通前必须具备：

- 已签订 Enterprise 合同或内部试点授权。
- 明确 `accountId`、`workspaceId`、客户名称、技术联系人、账务联系人。
- 明确每周期包含额度、超额策略、rate-limit、IP / client label 要求。
- 确认客户理解输出是公开权利声明和 registry 状态解释，不是法律授权结论。

### 4.2 开通顺序

1. 初始化 quota balance。
2. 签发 API key。
3. 执行 gateway dry-run。
4. 用真实 `POST /v1/enterprise/public-rights/batch` 跑小流量验证。
5. 确认 quota ledger、used units、API audit、last used at 均更新。
6. 交付客户接入信息和错误码文档。

内部 CLI 示例：

```powershell
npm run enterprise:internal-admin -- init-quota-balance --json
npm run enterprise:internal-admin -- issue-api-key --json
npm run enterprise:internal-admin -- dry-run-gateway --json
```

真实运行态门禁：

```powershell
npm run enterprise:gateway-contract
npm run enterprise:gateway-dry-run-runtime-qa
npm run enterprise:key-issuance-runtime-qa
npm run enterprise:public-rights-runtime-qa
```

后端测试门禁：

```powershell
cargo test --manifest-path feedback-backend/Cargo.toml enterprise_public_rights_external_batch_charges_quota_and_audits -- --nocapture
```

### 4.3 API key custody

API key 明文规则：

- 明文只允许在签发 / 轮换成功响应或受控终端输出中显示一次。
- 后续 list / get / audit / UI 不得返回明文。
- `keyHash` 只入库，不得进入响应、日志、审计 details、截图或客户文档。
- `keyPrefix` 只用于定位 key，不是认证材料。

轮换规则：

- 使用 `rotate-api-key` 签发新 active key。
- 旧 key 立即进入 paused。
- grace period 到期后由 `revoke-expired-rotations` 自动撤销旧 key。
- 轮换审计必须记录 deadline、delivery channel、recipient ref，但不得记录明文或 `keyHash`。

## 5. Enterprise quota / SLA / 观测

### 5.1 Quota

当前唯一正式 quota type：

```text
public_rights_scan_units
```

扣减规则：

- 一次 batch 请求按 item 数计算 chargeable units。
- 成功路径写入 quota ledger committed debit。
- 成功路径更新 quota balance `used_units`。
- 拒绝路径不扣减真实额度，但必须记录 API audit。
- `legalConclusion=false` 不受客户套餐影响，永远不能改成 true。

### 5.2 Rate-limit

上线前必须确认：

- API key rate-limit policy 已入库。
- DB rate-limit window 正常工作。
- 生产可信反向代理 / IP 指纹限流策略已评审；后端已支持 `HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET`、`HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY=true`、hash-only `clientFingerprintHash` 和按 API key + 指纹分桶限流。
- 客户侧超限返回稳定错误码，不泄露内部实现。

### 5.3 SLA

首批客户建议采用受控 SLA：

| 指标 | 初始门槛 |
| --- | --- |
| 可用性 | best-effort / pilot，不承诺金融级 SLA |
| p95 延迟 | 先以真实 QA 和客户试点观测为准 |
| 批量上限 | 按合同配置，不使用匿名 100 条上限作为商业额度 |
| 错误码稳定性 | 必须遵守 `ENTERPRISE_GATEWAY_STABLE_ERROR_CODES` |
| 审计留存 | 遵守 Enterprise 合同和数据保留策略 |

客户 SLA 不能覆盖法律结论、AI 生成检测、真实性鉴定或平台授权判断。

## 6. 回滚、暂停与事故处理

### 6.1 C2PA 回滚

如果生产 C2PA signer 异常：

1. 暂停“生产 C2PA signed manifest”对外描述。
2. 允许降级为 XMP / IPTC / JSON-LD 嵌入和 registry sidecar。
3. 记录导出结果 `c2paManifestStatus`。
4. 复跑 `rights:metadata-embed-runtime-qa`。

### 6.2 SDK 回滚

如果 SDK 发布后发现错误：

1. 立即停止文档推广。
2. 发布 patch 版本或弃用问题版本。
3. README 标记受影响版本和替代版本。
4. 确认 SDK 仍不输出法律结论。

### 6.3 Enterprise 客户暂停

如果客户滥用、欠费、密钥泄露或合同终止：

1. 使用内部 pause 或 revoke。
2. 查询 admin audit 和 API audit，确认最后一次调用。
3. 必要时 rotate 新 key，并让旧 key 进入 paused -> revoked 链路。
4. 通知客户只读 batch API 暂停原因。

客户侧 key 管理和 quota 管理路由当前仍不开放；所有变更通过内部管理员 token 和 CLI / 内部后台执行。

## 7. 发布证据包

每次生产上线必须留存：

- C2PA 证书链复核记录。
- TSA 可用性复核记录。
- `rights:metadata-embed-runtime-qa` 证据路径。
- `rights:metadata-embed-av-runtime-qa` 证据路径。
- SDK contract / typecheck / `rights:sdk-pack-dry-run` 输出。
- Enterprise gateway / key issuance / public rights runtime QA 输出。
- 客户 quota balance 初始化记录。
- API key 签发 / 交付记录，不包含明文归档。
- rollback 演练记录。

## 8. 发布检查清单

| 检查项 | 状态 |
| --- | --- |
| 生产 C2PA cert/key/alg/TSA 已配置并验证 | 2026-06-30 staging：阻塞。当前 shell 未注入 `HIDDENSHIELD_C2PA_SIGN_CERT_PEM`、`HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM`、`HIDDENSHIELD_C2PA_SIGNING_ALG`、`HIDDENSHIELD_C2PA_TSA_URL`；需通过 `rights:metadata-embed-production-staging-qa` 后才能宣称生产 C2PA trust chain 或 TSA 完成 |
| 图片 PNG / JPEG C2PA active manifest QA 通过 | 2026-06-30 staging：运行态通过，证据 `tmp-ui-qa/public-metadata-embedded-image/1782795869702/public-metadata-embedded-image-qa-1782795869702.md`；但因未配置生产证书 / TSA，只能算 runtime signer QA，不算生产信任链通过 |
| 音视频 C2PA active manifest QA 通过且未宣称生产 trust chain | 2026-06-30 staging：通过，证据 `tmp-ui-qa/public-metadata-embedded-av/1782801102377/public-metadata-embedded-av-qa-1782801102377.md`；WAV / MP4 均读到 C2PA active manifest，但 signer 仍为 `ephemeral_development_certificate_not_publicly_trusted` |
| SDK contract/typecheck/pack dry-run 通过 | 2026-06-30 staging：通过 `rights:sdk-package-contract`、SDK typecheck、`rights:sdk-pack-dry-run`；已修正原 runbook 中 `npm --prefix ... pack --dry-run` 在当前 npm 行为下误打根包的问题 |
| SDK README 保留 `legalConclusion=false` 边界 | 2026-06-30 staging：通过 `rights:sdk-package-contract` |
| Enterprise API key 已签发且明文只显示一次 | 2026-06-30 staging：通过，证据 `tmp-ui-qa/enterprise-key-issuance-runtime/1782795845980/enterprise-key-issuance-runtime-qa-1782795845980.md` |
| Quota balance 已初始化 | 2026-06-30 staging：通过，证据 `tmp-ui-qa/enterprise-public-rights-runtime/1782795725674/enterprise-public-rights-runtime-qa-1782795725674.md` |
| Dry-run gateway 通过 | 2026-06-30 staging：通过，证据 `tmp-ui-qa/enterprise-gateway-dry-run-runtime/1782795725740/enterprise-gateway-dry-run-runtime-qa-1782795725740.md` |
| 真实 batch API 小流量验证通过 | 2026-06-30 staging：通过，证据 `tmp-ui-qa/enterprise-public-rights-runtime/1782795725674/enterprise-public-rights-runtime-qa-1782795725674.md`，覆盖缺 key 401、成功扣 2 units、quota 不足拒绝、`legalConclusion=false` |
| pause / revoke / rotate / revoke-expired 演练通过 | 2026-06-30 staging：通过，证据 `tmp-ui-qa/enterprise-key-issuance-runtime/1782795845980/enterprise-key-issuance-runtime-qa-1782795845980.md` |
| 客户错误码、限流、SLA、联系人和升级路径已确认 | 2026-06-30 staging：部分通过。稳定错误码、限流 dry-run 和 quota 拒绝已验证；真实客户联系人、生产 SLA owner 和升级路径仍需 release owner 签字 |

## 9. 下一步

`public-rights:production-readiness-contract` 已作为机器检查固定本文档、协议文档、商业化 Roadmap、能力边界、SDK 包、C2PA 环境变量名和 Enterprise 客户路由红线。

2026-06-30 已完成一次 staging 演练，证据见 `tmp-ui-qa/public-rights-production-staging/1782795869702/public-rights-production-staging-runbook-qa-1782795869702.md`。下一步应通过 staging secret manager 注入生产等价 C2PA 证书链、私钥、签名算法和 TSA URL，复跑 `rights:metadata-embed-production-staging-qa` 与本 runbook；在该项通过前，不得对外宣称“生产 C2PA signed manifest / TSA 已上线”。
