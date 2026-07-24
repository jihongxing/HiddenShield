# HiddenShield Enterprise 生产客户开通检查单模板

更新时间：2026-06-30

本文档用于不依赖外部 C2PA 证书 / TSA 的 Enterprise 客户开通内部验收。它只覆盖 HiddenShield 已实现的公开权利只读批量查询、API key custody、quota、审计、限流、回滚和客户支持信息收口；生产 C2PA trust chain / TSA 仍按 `docs/生产C2PA证书申请与Secret注入Checklist.md` 挂起处理。

## 1. 客户信息

| 项目 | 填写 |
| --- | --- |
| 客户名称 |  |
| accountId |  |
| workspaceId |  |
| 合同 / 试点编号 |  |
| 客户技术联系人 |  |
| 客户业务负责人 |  |
| HiddenShield release owner |  |
| HiddenShield support owner |  |
| 开通窗口 |  |
| 回滚窗口 |  |

## 2. 开通范围

| 项目 | 要求 | 验收 |
| --- | --- | --- |
| API 范围 | 只开放 `POST /v1/enterprise/public-rights/batch` | [ ] |
| 禁止路由 | 不开放客户侧 `/v1/enterprise/api-keys`、`/v1/enterprise/quotas`、客户自助 key 控制台 | [ ] |
| 法律结论 | 所有响应固定 `legalConclusion=false` | [ ] |
| 数据边界 | 不上传原始媒体、不上传保护副本、不返回本地路径 | [ ] |
| 权利事实源 | registry / rights manifest 为事实源，公开元数据只作传播层 | [ ] |

## 3. API Key Custody

| 项目 | 要求 | 验收 |
| --- | --- | --- |
| 明文生成方 | 仅由受管理员 token 保护的内部签发入口生成 | [ ] |
| 明文展示 | `cleartextApiKey` 只在签发 / 轮换响应中显示一次 | [ ] |
| 入库字段 | 只保存 `keyHash`、`keyPrefix`、scope、状态和元数据 | [ ] |
| 审计字段 | 审计不得记录明文 key 或完整 `keyHash` | [ ] |
| 交付方式 | 使用客户确认的安全交付通道 | [ ] |
| 轮换策略 | 已记录 grace period、deadline 和旧 key paused -> revoked 流程 | [ ] |

建议命令：

```powershell
npm run enterprise:key-issuance-runtime-qa
node scripts/enterprise-internal-admin.mjs issue-api-key --json-file <payload.json>
node scripts/enterprise-internal-admin.mjs rotate-api-key --api-key-id <id> --json-file <payload.json>
node scripts/enterprise-internal-admin.mjs revoke-expired-rotations --json-file <payload.json>
```

## 4. Quota 与限流

| 项目 | 要求 | 验收 |
| --- | --- | --- |
| quota type | `public_rights_scan_units` | [ ] |
| quota balance | 合同周期、included units、overage 策略已初始化 | [ ] |
| quota ledger | 成功 batch 写入 committed debit | [ ] |
| quota 不足 | 返回稳定拒绝，不扣减 used units | [ ] |
| 可信代理 | 生产模式配置 `HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET` 与 `HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY=true` | [ ] |
| 指纹限流 | 只保存 hash-only `clientFingerprintHash`，按 API key + 指纹分桶 | [ ] |

建议命令：

```powershell
node scripts/enterprise-internal-admin.mjs init-quota-balance --json-file <payload.json>
npm run enterprise:gateway-dry-run-runtime-qa
npm run enterprise:public-rights-runtime-qa
```

## 5. 小流量验收

| 场景 | 预期 | 验收 |
| --- | --- | --- |
| 缺少 API key | 401 / stable error | [ ] |
| scope 不足 | 拒绝且不扣 quota | [ ] |
| `api_access=false` | 拒绝且不扣 quota | [ ] |
| 成功 1-2 条 batch | 返回 registry 公开权利信号，`legalConclusion=false`，扣减对应 units | [ ] |
| quota 不足 | 拒绝且不扣减 used units | [ ] |
| revoked key | 拒绝且写 audit | [ ] |

## 6. 审计与回滚

| 项目 | 要求 | 验收 |
| --- | --- | --- |
| API audit | 成功 / 拒绝路径均写入 endpoint、account、workspace、apiKey、requestId、outcome | [ ] |
| client fingerprint | audit 只记录 hash，不记录明文 IP 或明文指纹 | [ ] |
| admin audit | create / issue / rotate / pause / revoke / quota init / dry-run 均可查询 | [ ] |
| 暂停客户 | 可在 15 分钟内 pause key 或关闭 `api_access` | [ ] |
| 撤销客户 | revoke 后不可恢复，需重新签发新 key | [ ] |
| 回滚验证 | 回滚后 batch 请求失败且不再扣 quota | [ ] |

建议命令：

```powershell
node scripts/enterprise-internal-admin.mjs list-admin-audit-events --account-id <accountId> --limit 50
node scripts/enterprise-internal-admin.mjs pause-api-key --api-key-id <id> --reason "<reason>"
node scripts/enterprise-internal-admin.mjs revoke-api-key --api-key-id <id> --reason "<reason>"
```

## 7. 客户交付材料

| 材料 | 要求 | 验收 |
| --- | --- | --- |
| API base URL | 已确认环境、网关和可信代理路径 | [ ] |
| SDK 包 | 如使用 SDK，必须完成 pack dry-run；外部 npm 发布另走发布流程 | [ ] |
| 错误码表 | 已交付稳定错误码与重试建议 | [ ] |
| SLA 口径 | 仅承诺公开权利信号查询可用性，不承诺法律授权结论 | [ ] |
| 支持升级 | 客户联系人、HiddenShield support owner、升级路径已确认 | [ ] |

建议命令：

```powershell
npm run rights:sdk-pack-dry-run
npm run public-rights:production-readiness-contract
```

## 8. 最终签字

| 角色 | 姓名 | 日期 | 备注 |
| --- | --- | --- | --- |
| Release owner |  |  |  |
| Support owner |  |  |  |
| Security reviewer |  |  |  |
| Customer owner |  |  |  |

未完成任一必填项时，不得把该客户标记为“Enterprise 生产客户已开通完成”。
