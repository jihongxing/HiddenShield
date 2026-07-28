# Phase C/D 外部 Gate 证据模板

状态：`configuration_required_external_only`

`phase-c-d-evidence.template.json` 是首个设计伙伴与真实 provider 的统一证据索引。它只收集 opaque reference 和不可变 evidence digest，不包含 Secret、媒体或真实性结论。

## Phase C

- 伙伴身份、Sandbox bundle、12 场景 evidence 与商业签署引用必须齐备。
- 全部 evidence 必须由独立 Evidence Intake 接收为 `received_for_review` 后，才可进入 Sandbox Gate 验真。

## Phase D

- IAM、KMS/HSM、signer、object-store、notification recovery 的独立 evidence 必须齐备。
- Security approval 是评审前置，不等于 provider 已激活。

模板中的任何 `replace-me`、空 evidence 或 `configuration_required` 均保持 Gate 挂起。只有独立 Gate 的真实验真结果可改变发布边界。
