# HiddenShield PDF 报告签名密钥托管与撤销模型

状态：Phase R2 设计合同，生产签名尚未实现。

## 1. 当前状态

- `manifest.signature.status = not_signed`
- `signerKeyId = null`
- `certificateChainStatus = not_evaluated`
- `revocationStatus = not_applicable`
- `signedAt = null`

当前离线完整性校验只证明报告包文件与本次 Manifest 摘要链匹配，不证明签发主体身份。

## 2. 密钥分层

必须区分三类密钥：

| 环境 | 用途 | 是否允许进入正式报告 |
| --- | --- | --- |
| fixture | 单测、合同测试 | 否 |
| staging | 集成测试、TSA 和撤销演练 | 否 |
| production | 用户正式报告签发 | 仅通过发布门禁后 |

任何桌面安装包、移动安装包、前端资源、日志、数据库和同步 payload 都不得包含 production 私钥。

## 3. 托管要求

production 私钥必须由 KMS、HSM 或等效受控签名服务托管：

- 私钥不可导出。
- 每次签名使用可审计的 `signerKeyId`。
- 调用主体使用最小权限和短期凭证。
- 签名请求记录 report ID、Manifest root digest、调用主体和审计事件编号。
- 密钥管理员与报告签发服务账号分离。
- 生产密钥创建、启用、停用、轮换和销毁需要双人审批。

桌面和移动端只能提交待签摘要或调用签名服务，不得持有生产私钥。

## 4. 签名处理边界

签名作为 PDF 和 Manifest 生成后的确定性后处理阶段：

1. 固化 `report.pdf` 与 `report.json`。
2. 生成 Manifest 摘要链。
3. 对 Manifest root digest 和报告标识签名。
4. 对 PDF 执行 CMS/PAdES 增量签名。
5. 获取 RFC 3161 时间戳。
6. 写入证书链、时间戳和撤销信息。
7. 重新执行文件、签名和时间戳验证。

水印算法、版权编号、payload 和写后验证事实仍由 `watermark-core` 与正式包装层提供，签名服务不得修改。

## 5. 轮换

- 每个签名记录必须保存稳定 `signerKeyId`，不能只保存证书主题名称。
- 新密钥启用后，旧密钥进入 verify-only 状态。
- 历史报告继续使用原签名和证书链验证，不重新伪造签发时间。
- 轮换必须有重叠验证期和回滚窗口。

## 6. 撤销

撤销状态至少包括：

- `good`
- `revoked`
- `unknown`
- `not_checked`

撤销来源包括 OCSP、CRL 和 HiddenShield 报告撤销登记。三者必须分别展示，不能把内部报告撤销等同于证书撤销。

密钥泄露时：

1. 立即停用签名权限。
2. 撤销对应证书。
3. 标记受影响时间窗和报告集合。
4. 在线校验页显示 `signatureStatus = revoked`。
5. 发布事件说明与重新签发策略。
6. 保留原始报告和审计记录，不静默替换。

## 7. 长期验证

进入可承诺阶段前必须完成：

- PAdES profile 选择。
- 证书链归档。
- RFC 3161 时间戳验证。
- OCSP / CRL 响应归档。
- 过期证书历史验证。
- 长期验证数据更新策略。
- 密钥、证书、TSA 和撤销服务故障演练。

## 8. 下一实现任务

- 在 production KMS/HSM 和可信证书尚未确定前，保持所有正式报告 `not_signed`，仅推进离线摘要链与跨端校验。
