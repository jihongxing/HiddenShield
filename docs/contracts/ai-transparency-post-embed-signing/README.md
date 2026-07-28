# Production Post-Embed Signing Contract Fixtures

- 四份 Schema 分别冻结 command、authorization/signer receipt、production object-store receipt 与 versioned Profile entitlement。
- 十三份 fixture 覆盖 success、signer rejected、receipt/hash mismatch、C2PA readback failure、V3 readback failure、confirm rollback、duplicate replay、双连接 concurrent reservation、artifact finalize recovery，以及 reservation、signer 返回、artifact stage、confirm 四个崩溃点恢复。
- signer receipt 必须绑定由 idempotency key 与 request digest 确定性派生的 `signerInvocationKey`、provider result reference、幂等 disposition 与唯一 billable invocation；object-store receipt 必须绑定同一 invocation key、最终 hash、object version 和 command idempotency key。
- Fixture 仅定义 internal contract，不代表 production signer、真实 KMS/HSM、受信任 certificate chain、SDK 或客户发放。
- 合同日期：2026-07-27。
