# HiddenShield PDF 报告在线校验与二维码合同

状态：Phase R2 设计合同，在线服务尚未部署。

## 1. 当前边界

- 当前报告包只支持 `manifest.json` 驱动的离线 SHA-256 摘要链校验。
- 当前 PDF 不生成可扫描二维码。
- 当前 Manifest 固定记录：
  - `verification.offlineMode = sha256_chain_v1`
  - `verification.onlineStatus = not_deployed`
  - `verification.qrStatus = not_issued`
  - `verification.onlineVerificationUrl = null`
- 离线文件匹配不等于签名可信、可信时间有效、权属成立或司法采纳。

## 2. 未来在线校验标识

在线服务上线后，每份签发报告应拥有不可猜测的 `verificationId`，并与以下字段绑定：

- `reportId`
- `manifest.integrity.rootDigest`
- `bundle.sourceKey`
- `bundle.bundleVersion`
- `signature.signerKeyId`
- 签发状态与撤销状态

二维码只编码 HTTPS 校验地址和短期展示参数，不写入创作者姓名、文件名、媒体摘要、本地路径或权利声明正文。

建议路由合同：

```text
GET /verify/report/{verificationId}
```

## 3. 响应状态必须分离

在线校验页和 API 必须分别返回：

- `fileIntegrityStatus`
  - `matched`
  - `mismatch`
  - `files_unavailable`
- `signatureStatus`
  - `valid`
  - `invalid`
  - `revoked`
  - `expired`
  - `not_signed`
  - `not_evaluated`
- `trustedTimeStatus`
  - `valid`
  - `invalid`
  - `not_timestamped`
  - `not_evaluated`
- `reportStatus`
  - `active`
  - `superseded`
  - `revoked`

禁止将上述状态合并成单一的“可信”“司法有效”或“法院认可”结论。

## 4. 二维码签发门禁

只有同时满足以下条件才允许 `qrStatus = issued`：

1. 在线校验服务已部署并通过生产运行态验收。
2. 报告已使用生产签名密钥完成签名。
3. 签名证书链和撤销查询可用。
4. Manifest root digest 已登记到在线校验记录。
5. 校验页能够显示 active / superseded / revoked 状态。
6. 隐私与删除策略通过法务评审。

任何 fixture、自签名证书、测试域名或本地服务只能使用 `qrStatus = test_only`，不得进入用户正式报告。

## 5. 替代与撤销

- 新版本报告必须保留 `supersedesReportId`。
- 被替代报告在线状态改为 `superseded`，但历史文件摘要不得删除或重写。
- 发现密钥泄露、错误事实模型、错误签发主体或法律要求时，可将报告标记为 `revoked`。
- 撤销记录必须包含原因码、操作主体、操作时间和审计事件编号。

## 6. 下一实现任务

- Phase R3 前先实现离线 Manifest 跨端只读校验；在线 API、二维码和生产签发保持关闭。
