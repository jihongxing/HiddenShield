# AI 生成内容标识第三方 C2PA 公开 Fixture 合同

## 来源与许可

- Fixture：`docs/fixtures/ai-transparency-third-party-c2pa-v1/contentauth-c2pa-fixtures-C.jpg`。
- 来源：Content Authenticity Initiative 的 `contentauth/c2pa-fixtures` 公开仓库，路径 `C.jpg`。
- 许可证：Apache-2.0；来源、revision、文件 SHA-256 和预期均固定在同目录 `manifest.json`。
- 获取日期：2026-07-27。

## 证据边界

- 该文件是明确许可的公开 C2PA 测试 fixture，证明本项目可以读取第三方 C2PA manifest container。
- 它不是 AIGC 平台的生产生成样本，不提供任何平台验收授权，不证明 CN/EU/US 合规、C2PA 信任链有效性或第三方数字水印兼容性。
- `externalPlatformAcceptanceAuthorized=false` 与 `legalConclusion=false` 固定不变。

## Benchmark

1. 使用本项目锁定的 `c2pa` crate 读取 JPEG，要求存在 active C2PA manifest。
2. 使用 `watermark-core` 读取同一 JPEG，要求未发现 HiddenShield V3 anchor；不得把第三方 C2PA metadata 误判为 HiddenShield 标识。
3. 原始 fixture 的 SHA-256、来源路径、许可证和预期发生变化即阻断。
4. iOS runtime Gate 独立保持 `suspended_external_environment`；SDK、公共 Resolver、production credential 与生产发放继续关闭。
