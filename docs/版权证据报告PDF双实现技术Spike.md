# HiddenShield 版权证据报告 PDF 双实现技术 Spike

更新时间：2026-07-14

## 1. 目标

使用 Phase R0 的同一图片样本，对比：

- HTML / Chromium PDF
- Rust 原生 PDF

比较维度：

- 中文字体嵌入结构
- 四页分页稳定性
- 单次生成耗时
- 文件大小
- 数字签名扩展成本
- 视觉还原和后续维护成本

本 Spike 不修改正式报告运行态，不代表 PDF 导出已经进入 Creator、Free 单份报告或维权证据包交付。

## 2. 实现位置

- 统一图片样本：`tools/report-pdf-spike/image-sample.json`
- Chromium 生成器：`tools/report-pdf-spike/generate-chromium.mjs`
- Rust 原生生成器：`tools/report-pdf-spike/src/main.rs`
- PDF 结构检查器：`tools/report-pdf-spike/src/bin/inspect_pdf.rs`
- 对比编排：`tools/report-pdf-spike/run-spike.mjs`
- 运行命令：`npm run report:pdf-spike`
- QA 输出目录：`tmp/report-pdf-spike/`

Chromium 路径复用：

- `docs/prototypes/copyright-evidence-report-r0/finalized.html`

Rust 路径使用：

- `printpdf 0.10.1`
- `lopdf 0.39`
- `C:\Windows\Fonts\NotoSansSC-VF.ttf`
- `C:\Windows\Fonts\NotoSerifSC-VF.ttf`

## 3. 最终实测

本机环境：

- Windows
- Rust `1.93.0`
- Playwright `1.61.0`
- Google Chrome / Chromium headless

多次 warm run 的代表范围：

| 维度 | HTML / Chromium | Rust 原生 |
|---|---:|---:|
| 页数 | 4 | 4 |
| 生成耗时 | 4.49 到 4.93 秒 | 62 到 81 ms |
| 浏览器启动 | 220 到 308 ms | 不适用 |
| 文件大小 | 1,947,543 bytes | 161,402 bytes |
| 相对大小 | 1.000 | 0.083 |
| FontFile 对象 | 0 | 2 |
| Type3 字形字体 | 480 | 0 |
| Type0 CID 字体 | 0 | 2 |
| ToUnicode 映射 | 480 | 2 |
| 视觉还原 | 高 | 中 |

耗时只统计生成器运行，不包含首次 Cargo release 编译。

Chromium 的 `page.pdf()` 每次启动独立浏览器和加载两套约 42 MB 的 Noto 可变字体，因此当前耗时明显高于产品目标。后续若选择 Chromium，需要通过常驻渲染进程、静态字体子集或字体缓存继续优化。

## 4. 中文字体结论

### HTML / Chromium

- 浏览器计算样式确认使用 `Noto Sans SC Spike` 和 `Noto Serif SC Spike`。
- 生成 PDF 后，Noto 可变字体被转换为大量 Type3 字形字体。
- 共发现 480 个 Type3 字体字典和 480 个 ToUnicode 映射。
- 没有独立 FontFile / FontFile2 / FontFile3 对象。
- 文本仍可提取，但字体结构复杂，文件达到约 1.95 MB。

这不等于“字体缺失”，而是 Chromium 将使用到的字形以内嵌 Type3 字形程序形式保存。对普通阅读和复制文本可用，但对长期归档、字体审计、PDF/A 和后续签名验收不够理想。

### Rust 原生

- `printpdf` 对两套 Noto 字体执行子集化。
- PDF 中有 2 个 Type0 CID 字体、2 个 ToUnicode 映射和 2 个实际 FontFile2 对象。
- 可提取字体子集大小约为：
  - Noto Sans SC：90,696 bytes
  - Noto Serif SC：21,400 bytes
- PDF 总大小约 161 KB。

Rust 字体结构更适合后续归档和验证，但当前可变字体默认实例的权重控制不足，视觉上偏细；正式实现需改用经过许可和打包评审的静态字体文件，或实现明确的 variable font axis 选择。

## 5. 分页与视觉结论

### HTML / Chromium

- 四页 DOM 均无 `scrollHeight > clientHeight` 溢出。
- PDF 结构检查为 4 页。
- 直接复用 Phase R0 视觉稿。
- 封面、执行摘要、证据链、限制说明的层级、间距和表格表现稳定。

### Rust 原生

- 固定生成 4 个 `PdfPage`，结构检查为 4 页。
- 封面与限制说明页可达到可读水平。
- 初版执行摘要出现列表密度和换行问题，经过一次手工调整后消除重叠。
- 仍存在手工换行不自然、字体偏细、信息密度较低等差距。

这次修正本身证明了 Rust 原生的主要成本不在 PDF 写出，而在中文排版引擎、字体实例、自动分页和模板迭代。

## 6. 数字签名扩展成本

以下为工程估算，不是已经实现或验证的能力。

### HTML / Chromium

估算：**6 到 10 人日**

原因：

- Chromium 只能输出完成态 PDF。
- 需要独立 PDF 后处理器执行 incremental update。
- 需要 CMS / PAdES 签名、ByteRange、证书链、RFC 3161 时间戳、OCSP / CRL 和撤销状态。
- 需要确认 Type3 字体和后处理不会破坏 PDF/A / 长期验证目标。

### Rust 原生

估算：**4 到 7 人日**

原因：

- `printpdf` 可暴露 `lopdf::Document` 供同进程后处理，集成边界更短。
- 仍没有可直接宣称完成的 PAdES 能力。
- 同样需要证书托管、CMS、时间戳、撤销和长期验证。

两条路径都不应在 R1 直接宣传“可信数字签名已上线”。签名必须作为渲染后的独立确定性阶段，并由 Manifest 记录签名前后摘要与状态。

## 7. 决策

### Phase R1

选择 **HTML / Chromium** 作为首版主渲染器。

原因：

- 能直接交付已批准的高保真模板。
- CSS 分页和设计迭代成本低。
- 与桌面端报告预览、视觉回归和后续模板版本管理一致。

R1 前必须增加：

- 常驻或复用 Chromium 进程，避免每份报告重新启动。
- 替换 Noto 可变字体为经许可审核的静态中文字体子集。
- 将生成时间压到 3 秒目标以内。
- PDF 结构门禁：4 页、文本可提取、无溢出、字体策略符合预期。

### Rust 原生

保留为：

- 离线最小报告 fallback
- 灾备报告
- 字体嵌入与归档参考实现
- 后续签名后处理验证工具

暂不作为高保真主模板。

## 8. 验证

- `cargo check --manifest-path tools/report-pdf-spike/Cargo.toml --bins`
- `npm run report:pdf-spike`
- 两份 PDF 均为 4 页。
- 两份 PDF 均可提取中文文本。
- Chromium 四页 DOM 无溢出。
- Rust 生成器无 `printpdf` warning。
- 已人工查看两份 PDF 的封面、执行摘要和限制说明页。

## 9. 风险

- 当前字体来自 Windows 系统目录，不能直接作为产品分发方案。
- Chromium Type3 字形结构不适合作为最终归档字体策略。
- Rust 可变字体权重偏细，不能直接作为正式视觉。
- 当前签名成本仅为架构估算，需单独 PAdES spike 验证。
- 当前样本仍是脱敏设计 fixture，不是正式用户版权库快照。

## 10. 推荐下一步

执行 **Phase R1 Chromium 渲染器最小集成**：

1. 将 `FormalReportDocument` 序列化为独立 HTML 模板数据。
2. 使用常驻 Chromium worker 生成 PDF。
3. 换用仓库内受控静态中文字体子集。
4. 输出 `report.pdf + report.json + manifest.json`。
5. 加入 3 秒耗时、4 页分页、字体结构、文本提取和隐私字段门禁。
