# HiddenShield 生产 C2PA 证书申请与 Secret 注入 Checklist

更新时间：2026-06-30

本文用于把生产 C2PA 证书申请、私钥托管、TSA 开通和 staging / production secret 注入流程固定为上线门禁。它是 `docs/生产C2PA证书链_TSA_SDK发布_Enterprise客户开通Runbook.md` 的前置 checklist。

本文不保存、不示例、不粘贴任何真实证书私钥、API key、TSA token 或 secret value。所有输出仍必须保持 `legalConclusion=false`。

## 1. 适用边界

| 项目 | 结论 |
| --- | --- |
| 目标 | 为 PNG / JPEG 公开元数据嵌入副本配置生产可信 C2PA signer 和 TSA |
| 不包含 | 音频 / 视频官方 C2PA 容器级 signed manifest、法律授权结论、训练许可自动授权判断 |
| 当前工程变量 | `HIDDENSHIELD_C2PA_SIGN_CERT_PEM`、`HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM`、`HIDDENSHIELD_C2PA_SIGNING_ALG`、`HIDDENSHIELD_C2PA_TSA_URL` |
| 必须通过 | `npm run rights:metadata-embed-production-staging-qa` |
| 禁止替代 | 不能用 self-signed、ephemeral development certificate、普通 `rights:metadata-embed-runtime-qa` 或截图口头说明替代生产信任链验证 |

## 2. 角色与职责

| 角色 | 职责 |
| --- | --- |
| Release owner | 确认 CA / TSA provider、签发材料、证据包和上线口径 |
| Security owner | 审批私钥生成、KMS / HSM / secret manager 权限、轮换和吊销策略 |
| Backend / desktop owner | 验证变量注入、运行 staging QA、确认 `c2paSignerStatus=configured_certificate_chain` |
| Legal / compliance owner | 确认组织身份、证书用途、客户说明和“非法律授权结论”口径 |
| Ops owner | 维护 TSA 可用性监控、告警、回滚开关和证书到期提醒 |

## 3. CA 选择

发布负责人必须完成以下检查：

| 检查项 | 要求 | 状态 |
| --- | --- | --- |
| CA / trust provider 名称 | 记录 provider、联系人、合同或工单编号 | 发布前填写 |
| C2PA 适用性 | 证书用途应支持 C2PA claim signing 或 provider 明确支持 Content Credentials / C2PA 签名场景 | 发布前填写 |
| 组织身份 | 证书主体应能对应 HiddenShield / 公司主体 / 产品发布主体 | 发布前填写 |
| 证书链 | 需要完整 PEM chain，包含验证所需中间证书 | 发布前填写 |
| 证书有效期 | 记录 notBefore / notAfter，并在到期前至少 30 天触发轮换 | 发布前填写 |
| 吊销方式 | 记录 OCSP / CRL / provider 控制台吊销方式 | 发布前填写 |
| 算法 | 必须映射到当前工程支持的 `Ed25519`、`ES256`、`ES384`、`ES512`、`PS256`、`PS384`、`PS512` 之一 | 发布前填写 |

阻断条件：

- provider 只能给 self-signed / demo cert。
- 证书用途无法说明适用于 C2PA / Content Credentials。
- 证书链不完整或证书主体无法对应发布主体。
- CA / trust provider 要求私钥出现在邮件、IM、工单附件或仓库中。

## 4. CSR 与私钥生成

生产私钥优先在 KMS / HSM / secret manager 受控环境中生成。只有在 provider 明确要求 CSR 时，才生成 CSR。

建议流程：

1. Security owner 创建受控 key material。
2. 生成 CSR，CSR 中只包含组织、产品和必要 subject 信息。
3. CSR 发给 CA / trust provider。
4. 私钥不离开托管边界；不得发给外部人员、不得进入 git、不得进入 QA evidence。
5. CA 返回证书链后，由 Security owner 验证证书和私钥匹配。

需要记录：

| 项 | 记录内容 | 状态 |
| --- | --- | --- |
| key custody | KMS / HSM / secret manager 路径或受控主机编号，不记录私钥值 | 发布前填写 |
| CSR 工单 | CSR 生成时间、审批人、provider 工单编号 | 发布前填写 |
| algorithm | 与 `HIDDENSHIELD_C2PA_SIGNING_ALG` 完全一致 | 发布前填写 |
| cert/key match | 记录校验命令输出摘要，不包含私钥 | 发布前填写 |
| access policy | 谁可以读取 / 更新 secret，谁可以触发签名 QA | 发布前填写 |

本地或 staging 校验可以使用 openssl / provider 工具确认 cert 与 key 匹配，但输出中不得包含 private key PEM。

## 5. TSA 开通

`HIDDENSHIELD_C2PA_TSA_URL` 是生产可信时间戳声明的上线条件。未配置 TSA 时，不能宣称“生产可信时间戳链路已完成”。

必须记录：

| 检查项 | 要求 | 状态 |
| --- | --- | --- |
| TSA provider | 服务方、合同或工单编号 | 发布前填写 |
| TSA URL | RFC 3161 TSA endpoint，写入 secret manager 或受控配置 | 发布前填写 |
| auth 方式 | 如需 token / mTLS / IP allowlist，记录托管方式，不记录 secret value | 发布前填写 |
| timeout / retry | 记录导出失败策略和告警阈值 | 发布前填写 |
| 可用性验证 | staging 中运行 `rights:metadata-embed-production-staging-qa` 并归档证据 | 发布前填写 |
| 降级策略 | TSA 不可用时暂停“生产 TSA 已上线”对外描述，允许 registry / XMP / IPTC / JSON-LD 降级 | 发布前填写 |

## 6. Secret Manager 注入

staging 和 production 必须分别配置，不得共用开发 ephemeral 证书。

| 变量 | 来源 | 注入要求 | 泄露风险 |
| --- | --- | --- | --- |
| `HIDDENSHIELD_C2PA_SIGN_CERT_PEM` | CA 返回的完整证书链 PEM | secret manager 注入；允许写入运行环境，不写入仓库 | 中 |
| `HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM` | KMS / HSM / secret manager 托管私钥 | secret manager 注入或签名服务读取；不得进入日志 / evidence | 高 |
| `HIDDENSHIELD_C2PA_SIGNING_ALG` | 证书 / 私钥算法映射 | 普通 env 或 secret manager 均可；必须与 key 一致 | 低 |
| `HIDDENSHIELD_C2PA_TSA_URL` | TSA provider endpoint | 受控配置；如 URL 含 token 必须按高敏 secret 处理 | 中 / 高 |

注入检查：

```powershell
npm run rights:metadata-embed-production-staging-qa
```

该命令只输出变量是否缺失，不输出 secret value。缺任一变量时必须失败，并生成 `tmp-ui-qa/public-metadata-production-staging/<runId>/public-metadata-production-staging-qa-<runId>.md`。

## 7. 复跑命令

staging secret 注入完成后，在同一运行环境执行：

```powershell
npm run rights:metadata-embed-contract
npm run rights:metadata-embed-production-staging-qa
npm run public-rights:production-readiness-contract
```

验收条件：

- `rights:metadata-embed-production-staging-qa` 通过。
- PNG / JPEG 的 `c2paSignerStatus` 均为 `configured_certificate_chain`。
- PNG / JPEG 的 `hasC2paActiveManifest` 均为 `true`。
- `legalConclusion=false` 仍存在。
- QA evidence 中没有 private key、证书正文、TSA token、Enterprise 明文 key 或 `keyHash`。
- runbook 检查清单更新为通过，并附证据路径。

不得使用以下结果作为通过：

- `ephemeral_development_certificate_not_publicly_trusted`
- 只有 `rights:metadata-embed-runtime-qa` 通过
- 只有 C2PA active manifest 可读，但 signer status 不是 `configured_certificate_chain`
- 只有人工截图，缺少 QA JSON / Markdown evidence

## 8. 验收截图与证据包

每次 staging / production 上线必须归档：

| 证据 | 路径 / 记录 |
| --- | --- |
| CA / trust provider 审批记录 | 发布前填写 |
| CSR / 证书签发工单编号 | 发布前填写 |
| 私钥托管路径摘要 | 发布前填写，不含私钥值 |
| TSA provider 与可用性记录 | 发布前填写 |
| `rights:metadata-embed-production-staging-qa` Markdown | 发布前填写 |
| `rights:metadata-embed-production-staging-qa` JSON | 发布前填写 |
| PNG / JPEG check JSON | 发布前填写 |
| secret 泄露扫描结果 | 发布前填写 |
| runbook 检查清单更新 diff | 发布前填写 |

推荐泄露扫描：

```powershell
rg -n "PRIVATE KEY|BEGIN CERTIFICATE|hsent_live_|keyHash|HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM=.*[A-Za-z0-9+/]{20}" tmp-ui-qa docs scripts
```

允许命中变量名和文档说明；不允许命中真实 PEM、明文 API key、TSA token 或 key hash。

## 9. 轮换与吊销

证书轮换要求：

1. 新证书先注入 staging。
2. 运行 `rights:metadata-embed-production-staging-qa`。
3. 抽样检查旧证书签名材料仍可读，新签名只使用新证书。
4. production 切换后记录切换时间和证据路径。
5. 到期、泄露、组织信息变化或 provider 风险时立即吊销旧证书。

TSA 轮换要求：

1. 新 TSA URL 先进入 staging。
2. 记录 provider、SLA、认证方式和失败策略。
3. 运行生产 staging QA。
4. production 切换后监控失败率。

## 10. 上线红线

- 不得把 self-signed / ephemeral cert 写成生产可信 C2PA 证书。
- 不得把普通 active manifest QA 写成生产 trust chain QA。
- 不得把 C2PA 签名或 TSA 写成法律授权结论。
- 不得把 WAV / MP4 HiddenShield propagation packet 写成官方音视频 C2PA signed manifest。
- 不得把私钥、TSA token、明文 Enterprise API key 或 `keyHash` 写入仓库、日志、QA evidence、截图或客户文档。

## 11. 当前状态

| 日期 | 状态 | 证据 |
| --- | --- | --- |
| 2026-06-30 | 当前环境未注入生产等价 C2PA cert/key/alg/TSA；`rights:metadata-embed-production-staging-qa` 正确阻塞，不能对外宣称生产 C2PA/TSA 已上线 | `tmp-ui-qa/public-metadata-production-staging/1782796410144/public-metadata-production-staging-qa-1782796410144.md` |

下一步：Release owner 选择 CA / TSA provider 并完成 CSR、私钥托管与 secret manager 注入，然后运行 `npm run rights:metadata-embed-production-staging-qa`。
