# AI 生成内容标识第三方 PNG 元数据共存 Fixture 合同

## 状态与范围

- 状态：`internal_fixture_verified_ios_runtime_pending`。
- 本合同只覆盖 PNG ancillary metadata 与 HiddenShield V3 图片 anchor 的字节级共存、metadata 剥离和 anchor 读取；不模拟、签发、验证或宣称 C2PA、Content Credentials、平台签名、外部数字水印或法律合规。
- 外部数字水印互操作需要提供方的可再分发参考样本、处理链与验收授权后单独立项；不得用自造 metadata fixture 代替该证据。
- 本合同为 internal-only 互操作 Gate，不解锁 SDK、公共 Resolver、production credential、客户发放或生产信任声明。

## Fixture 输入

在 `docs/fixtures/ai-transparency-platform-executor-v1/` 的 Executor 输出 PNG 基础上，已生成 `platform-executor-v3-with-external-metadata.png`：

- 保留原 PNG 像素、V3/39 anchor 和既有完整 PNG chunks。
- 增加两个无可信含义的 PNG `tEXt` ancillary chunks：
  - `external_provenance_fixture=untrusted_test_metadata_v1`
  - `external_metadata_namespace=example.invalid/ai-provenance`
- fixture metadata 必须被明确标为 `untrusted`；不得携带 issuer、签名、Evidence 等级、Profile pass 或 `legalConclusion=true`。
- `manifest.json` 必须记录文件路径、SHA-256、预期 metadata keys、`payloadProtocolVersion=3`、`payloadBytesLength=39`、`payloadAuthStatus=verified` 和 `legalConclusion=false`。

## 最小互验矩阵

| 场景 | 预期 |
| --- | --- |
| Executor 输出 + 外部测试 metadata | `watermark-core` 读取同一 UID、V3/39、`verified`。 |
| 外部测试 metadata 剥离后 | `watermark-core` 仍读取同一 UID、V3/39、`verified`。 |
| Desktop 正式读取 | 读取上述两个版本，结果与 manifest 一致。 |
| Android / iOS 共用 mobile Rust bridge | 读取上述两个版本，结果与 manifest 一致；iOS runtime 证据仍独立挂起。 |

## 禁止推论

- metadata 共存不证明第三方系统会保留 HiddenShield anchor，也不证明 HiddenShield 会保留或识别第三方 metadata。
- metadata 剥离后的 anchor 读取不证明 metadata 来源、签名、Manifest、Evidence 或显式标签仍存在。
- 本 Gate 不能替代真实第三方平台转码、重编码、裁剪、格式转换、C2PA 验签或外部水印互操作 Benchmark。

## 实现 Gate

1. 已仅扩展 internal Executor QA fixture 生成器，未修改 `watermark-core` API、算法、payload、阈值或媒体协议。
2. 已由静态合同脚本校验 PNG chunk、metadata keys、manifest digest 与 metadata-stripped 输出不再含测试 metadata。
3. Desktop 与 mobile Rust bridge 定向测试已覆盖共存版与剥离版。
4. 通过本 Gate 后仍保持 SDK、公共 Resolver 与生产发放关闭；iOS runtime Gate 的挂起状态不变。
