# HiddenShield AI Transparency Schema Fixture v1

状态：`fixture_frozen_with_internal_sdk_and_postgresql_gates`

本目录为 `docs/AI生成内容标识数据库与API_Schema合同.md` 的固定 JSON 向量。

所有 fixture 均为合成数据，不包含真实客户、真实密钥、真实图片、真实签名或真实价格。

## 共享实体

```text
licenseId: atl_fixture_prod_0001
tenantId: tenant_fixture_aigc_0001
workspaceId: workspace_fixture_prod_0001
markingSessionId: ats_fixture_success_0001
watermarkUid: HS-A1B2C3D4-E5F60718-192A3B4C-5D6E7F80
```

## 固定向量

| 文件 | 场景 | 预期 |
| --- | --- | --- |
| `production-license-v1.fixture.json` | production 授权与凭据绑定 | 可创建标识会话 |
| `three-region-profile-entitlements-v1.fixture.json` | 中国、欧盟、加州和 C2PA Profile 授权 | 四条 active entitlement |
| `confirmed-marked-image-v1.fixture.json` | 成功 confirm | 一个 active Manifest 和一条 committed 计量 |
| `free-public-resolver-v1.fixture.json` | 公共 confirmed Manifest 解析 | 无 API key、无写入、无计量、最小字段、`legalConclusion=false` |
| `expired-license-rejection-v1.fixture.json` | 过期授权 | `ai_license_expired`，不创建会话或计量 |
| `profile-entitlement-rejection-v1.fixture.json` | 未获授权 Profile | `ai_profile_not_entitled`，不 reserve UID |
| `duplicate-confirm-rejection-v1.fixture.json` | 同一 session 冲突 confirm | `ai_confirmation_conflict`，不新增计量 |
| `platform-sdk-facade-v1.fixture.json` | server-side SDK 与 framework-neutral API facade | admission → session → mark → confirm，唯一 `confirmed_marked_image` receipt |

完全相同的 confirm 重放应在未来实现中返回原始成功结果，不产生第二条 ledger；本目录冻结的重复 confirm 场景是“同一 session 使用不同摘要或不同 confirm 请求”的冲突拒绝。

## 当前边界

- 原始业务 fixture 已对应 PostgreSQL 内部控制面；SDK/facade fixture 已对应内部 TypeScript package，但不代表真实平台端点、npm 发布或公共 Detector 已开放。
- `signature` 和 `bindingDigest` 为不可验证的 fixture 占位值，禁止作为生产签名材料使用。
- 公开 Resolver 仅按 `watermarkUid` 或 `manifestId` 查询 `0020` 冻结的三个公共 PostgreSQL view，不读取或上传媒体文件。
- 公共响应独立遵循 `public-resolver-v1.schema.json`，不得返回 license、tenant/workspace、session/admission、subject digest、模型、ledger、credential 或 confirmation token。
