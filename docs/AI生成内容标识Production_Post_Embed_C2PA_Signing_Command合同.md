# AI 生成内容标识 Production Post-Embed C2PA Signing Command 合同

## 状态与边界

- 状态：`contract_frozen_implementation_forbidden_until_provider_ready`。
- 命令仅允许内部受控调用，不提供 HTTP route、SDK、公共 Resolver、客户 credential 或生产发放。
- 本合同冻结生产语义，不表示已具备 production signer、KMS/HSM、受信任证书链、平台验收或法律合规。
- `EphemeralSigner`、本地测试证书和任何 QA receipt 永远不能满足本合同的 production credential 要求。

## 正确处理顺序

生产处理顺序固定为：

1. 校验有效 license、requested regulatory/technical Profile entitlement、`ready_to_confirm` session、Internal IAM receipt 和 production signer readiness。
2. 使用 `watermark-core` 生成 V3 PNG，并完成第一次 V3 write-after-read。
3. 对该未签名 V3 PNG 执行 production post-embed C2PA signing。
4. 对最终签名 PNG 执行 C2PA 回读与 V3 回读。
5. 计算最终签名 PNG SHA-256，并验证 signer receipt 中的最终 hash。
6. 只有上述步骤全部成功，才允许调用 PostgreSQL confirm 原子事务。
7. confirm 必须将最终签名 PNG hash 写入 Manifest、Evidence、Marker Binding、label receipt、ledger 和 audit；未签名 V3 PNG hash 只能作为内部中间 hash。
8. confirm 成功后才允许返回最终签名 PNG；任何失败均不得返回未签名或已签名产物。

禁止先 confirm 未签名 PNG、再追加 C2PA。已确认记录的 subject/final hash 不允许被后续签发覆盖。

## Command Input

```json
{
  "schemaVersion": "hs-ai-production-post-embed-signing-command-v1",
  "markingSessionId": "session-*",
  "executionId": "execution-*",
  "licenseId": "license-*",
  "credentialId": "credential-*",
  "watermarkUid": "HS-*",
  "requestedProfileIds": [
    "hiddenshield_v3_image_anchor_v1",
    "c2pa_post_embed_signing_v1",
    "regional_profile_id"
  ],
  "profileEntitlementVersion": 1,
  "unsignedV3PngSha256": "sha256",
  "unsignedV3PngBytes": "internal-bytes-reference",
  "signerProviderId": "provider-*",
  "signerCredentialRef": "secret-or-kms-reference",
  "iamReceipt": {},
  "signerAuthorizationReceipt": {},
  "idempotencyKey": "stable-request-key"
}
```

## Profile Entitlement Gate

命令必须同时满足：

- license 状态为 active，未过期、暂停或撤销。
- production credential 为 active，未过期、撤销或被 replacement 取代。
- requested technical Profile 必须包含：
  - `hiddenshield_v3_image_anchor_v1`
  - `c2pa_post_embed_signing_v1`
- requested regional Profile 必须与 session、license、signer scope 和当前 entitlement version 一致。
- entitlement 必须明确允许：
  - `mediaType=image/png`
  - `claimType=ai_generated`
  - `issuerMode=production_platform`
  - `signingOrder=watermark_then_c2pa`
- Profile 缺失、版本不匹配、scope mismatch、暂停或撤销时 fail-closed，且 signer 不得被调用。

## Signer Authorization Receipt

production signer 调用前的 authorization receipt 必须由受控 provider 验真，并至少绑定：

```json
{
  "receiptId": "sign-auth-*",
  "providerId": "provider-*",
  "operation": "ai_transparency_post_embed_c2pa_sign",
  "actorId": "workload-or-operator-id",
  "role": "ai_transparency_production_signer",
  "licenseId": "license-*",
  "credentialId": "credential-*",
  "markingSessionId": "session-*",
  "executionId": "execution-*",
  "profileEntitlementDigest": "sha256",
  "unsignedV3PngSha256": "sha256",
  "signerCredentialRefDigest": "sha256",
  "scopeDigest": "sha256",
  "issuedAt": "timestamp",
  "expiresAt": "timestamp",
  "providerSignature": "signature"
}
```

receipt 无效、过期、scope mismatch、digest mismatch、provider unavailable 或签名无效时，在 signer 调用和数据库事务前 fail-closed。

## Production Signer Receipt

signer 成功必须返回不可伪造 receipt，至少包含：

```json
{
  "schemaVersion": "hs-ai-production-c2pa-signer-receipt-v1",
  "signerReceiptId": "sign-result-*",
  "providerId": "provider-*",
  "operation": "c2pa_post_embed_sign",
  "markingSessionId": "session-*",
  "executionId": "execution-*",
  "watermarkUid": "HS-*",
  "profileEntitlementDigest": "sha256",
  "unsignedV3PngSha256": "sha256",
  "finalSignedPngSha256": "sha256",
  "c2paActiveManifestLabel": "label",
  "c2paClaimDigest": "sha256",
  "certificateChainDigest": "sha256",
  "signerKeyId": "kms-or-hsm-key-id",
  "signerKeyVersion": "version",
  "signerInvocationKey": "sha256",
  "signerResultRef": "provider-result-reference",
  "idempotencyDisposition": "created|replayed",
  "billableInvocationId": "provider-billable-invocation-id",
  "signatureAlgorithm": "allowed-profile-value",
  "signedAt": "timestamp",
  "receiptExpiresAt": "timestamp",
  "providerSignature": "signature"
}
```

receipt 必须通过 provider 签名、时效、operation、scope、Profile digest、unsigned hash、final hash、key status 和 certificate chain policy 验证。

## 最终文件 Hash 绑定

- `finalSignedPngSha256` 是唯一可进入 confirmed Manifest subject、Evidence subject、Marker Binding subject、ledger subject 和 confirm audit subject 的文件 hash。
- `unsignedV3PngSha256` 必须保留在 signer receipt 和内部 execution audit，用于证明签发前输入，但不得作为最终交付 hash。
- signer 返回 bytes 的本地 SHA-256 必须与 signer receipt 的 `finalSignedPngSha256` 完全一致。
- confirm 事务收到的 final hash、signer receipt final hash 和最终 bytes 本地 hash 任一不一致即拒绝。
- final hash 必须进入 request digest 和 idempotency/result projection；相同 idempotency key 不得对应不同 final hash。

## 双回读 Gate

最终签名 PNG 必须同时通过：

### C2PA

- active manifest 存在。
- hard binding 指向最终签名 PNG，不能出现 content hash mismatch。
- manifest label、claim digest、certificate chain digest、key ID/version 与 signer receipt 一致。
- validation findings 必须符合 requested Profile allowlist；production Profile 禁止 ephemeral/self-signed/untrusted chain。

### HiddenShield V3

- `watermark-core` 读取同一 `watermarkUid`。
- payload protocol 为 V3，长度为 39 bytes。
- `payloadAuthStatus=verified`。
- C2PA 签发后不得改变 V3 UID、auth 或 rewrite 状态。

任一回读失败均不得 confirm、不得计量成功、不得返回最终产物。

## 原子性与失败语义

外部 signer 调用与 PostgreSQL 事务无法形成单一数据库事务，因此冻结以下补偿语义：

- signer 调用前：零 confirm 数据库写入。
- signer 失败或超时：丢弃中间 V3 bytes，不返回产物，不产生成功计量。
- signer 成功但 receipt 验真、双回读或 final hash 失败：隔离并销毁已签名 bytes；不得 confirm，不产生成功计量。
- signer 成功且双回读成功，但 PostgreSQL confirm 回滚：不得返回产物；记录受控 orphan-signing operational event，后续只能重试相同 request digest/idempotency key，不得创建第二个不同签名结果。
- confirm 成功：一次且仅一次写入最终 hash、signer receipt reference、committed ledger 和 confirm audit，然后返回最终签名 PNG。
- 重复命令：相同 idempotency key/request digest 返回既有成功 projection；digest 不同则拒绝。

## 计量

- 仅 PostgreSQL confirm 成功的 `confirmed_marked_image` 进入商业计量。
- signer rejected、provider unavailable、receipt invalid、双回读失败、hash mismatch、confirm rollback、重复命令重放和公共 Resolver 均不计量。
- 外部 signer 已产生供应商成本但 confirm 未成功时，只进入内部成本/异常账，不进入客户成功用量。

## Audit

append-only audit 至少记录：

- command received / precheck passed or rejected
- signer authorization receipt verified
- signer invoked / signer receipt verified
- C2PA readback result
- V3 readback result
- final hash verified
- confirm committed or rolled back
- artifact returned or withheld
- orphan-signing operational event（如适用）

audit 不得记录明文 credential、private key、完整 Secret reference 或可重放 provider token。

## 当前 Gate

- 合同已冻结；command、authorization/signer receipt 与 versioned Profile entitlement JSON Schema 已冻结。
- success、signer rejected、receipt/hash mismatch、C2PA readback failure、V3 readback failure、confirm rollback、duplicate replay、concurrent reservation、artifact finalize recovery，以及 reservation 后、signer 返回后、artifact stage 后、confirm 后四个崩溃恢复，共十三类 fixture 已通过 `ai-transparency:post-embed-signing-contract`。
- internal-only command、`0007_ai_transparency_post_embed_signing`、`0008_ai_transparency_signing_reservation_artifact_recovery`、`0009_ai_transparency_adapter_receipts_crash_recovery` 与 PostgreSQL confirm 集成已完成；十三类 fixture 已在一次性 PostgreSQL 16 数据库中升级为真实事务测试。
- 同 idempotency key 通过 PostgreSQL advisory lock 跨连接串行化；execution 状态机冻结为 `reserved → signed_staged → artifact_pending → confirmed`，confirm 失败进入 `orphaned`。
- signer receipt 必须绑定稳定 `signerInvocationKey`；live 双连接并发最多一次 signer invocation。进程在 signer 返回后、receipt/stage 持久化前崩溃时，跨重试成本去重仍要求真实 signer provider 接受该 key 作为幂等键。
- `artifact_pending` 不返回产物且 ledger 保持 `pending`、客户计量为零；durable finalize 成功后 execution 与 ledger 在同一 PostgreSQL 事务进入 `confirmed/committed`，恢复不重新签发、不重复 confirm 或计量。
- production signer receipt 额外冻结 `signerResultRef`、`idempotencyDisposition` 与 `billableInvocationId`；production object-store stage/finalize receipt 必须绑定 execution、稳定 signer invocation key、最终文件 hash、object version、command idempotency key、durability status、有效期与 provider signature。
- 四个崩溃点的受控 PostgreSQL harness 已证明：恢复后最多一个 billable signer invocation、一个唯一 artifact stage 写入、一个 confirm projection 和一个 committed ledger；signer 返回后或 stage 后恢复允许 adapter 请求次数为二，但第二次必须返回 `replayed` receipt。
- 当前受控 QA interface 不是 production IAM/KMS/HSM、signer、C2PA trust chain 或真实 durable object store。
- 在真实 provider、production certificate chain、signer/object-store receipt 验真 adapter、durable object store 和真实进程 kill/restart 演练完成前，禁止 production command 外部开放、SDK、公共 Resolver、真实 credential 和客户发放。
