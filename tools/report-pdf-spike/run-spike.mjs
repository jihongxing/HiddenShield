import { mkdir, readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..", "..");
const outputDir = path.join(repoRoot, "tmp", "report-pdf-spike");
await mkdir(outputDir, { recursive: true });

const chromiumPdf = path.join(outputDir, "chromium-report.pdf");
const chromiumMetrics = path.join(outputDir, "chromium-metrics.json");
const rustPdf = path.join(outputDir, "rust-native-report.pdf");
const rustMetrics = path.join(outputDir, "rust-native-metrics.json");
const chromiumInspection = path.join(outputDir, "chromium-inspection.json");
const rustInspection = path.join(outputDir, "rust-native-inspection.json");

await run(process.execPath, [
  path.join(import.meta.dirname, "generate-chromium.mjs"),
  "--repoRoot", repoRoot,
  "--output", chromiumPdf,
  "--metrics", chromiumMetrics
], repoRoot);

await run("cargo", [
  "run",
  "--release",
  "--manifest-path", path.join(import.meta.dirname, "Cargo.toml"),
  "--bin", "report_pdf_spike",
  "--",
  "--sample", path.join(import.meta.dirname, "image-sample.json"),
  "--font-sans", "C:\\Windows\\Fonts\\NotoSansSC-VF.ttf",
  "--font-serif", "C:\\Windows\\Fonts\\NotoSerifSC-VF.ttf",
  "--output", rustPdf,
  "--metrics", rustMetrics
], repoRoot);

for (const [input, output] of [
  [chromiumPdf, chromiumInspection],
  [rustPdf, rustInspection]
]) {
  await run("cargo", [
    "run",
    "--release",
    "--manifest-path", path.join(import.meta.dirname, "Cargo.toml"),
    "--bin", "inspect_pdf",
    "--",
    "--input", input,
    "--output", output
  ], repoRoot);
}

const chromium = JSON.parse(await readFile(chromiumMetrics, "utf8"));
const rust = JSON.parse(await readFile(rustMetrics, "utf8"));
const chromiumPdfInspection = JSON.parse(await readFile(chromiumInspection, "utf8"));
const rustPdfInspection = JSON.parse(await readFile(rustInspection, "utf8"));
const comparison = {
  generatedAt: new Date().toISOString(),
  sample: "image",
  outputs: { chromiumPdf, rustPdf },
  comparison: {
    generationMs: {
      chromium: chromium.generationMs,
      chromiumLaunch: chromium.launchMs,
      rust: rust.generationMs
    },
    bytes: {
      chromium: chromium.bytes,
      rust: rust.bytes,
      rustVsChromiumRatio: round(rust.bytes / chromium.bytes)
    },
    pages: {
      chromium: chromiumPdfInspection.pageCount,
      rust: rustPdfInspection.pageCount
    },
    fontEmbedding: {
      chromium: {
        ...chromiumPdfInspection,
        browserFontState: chromium.browserFontState
      },
      rust: {
        ...rustPdfInspection,
        requestedSubset: rust.fontEmbedding.requestedSubset,
        sourceFonts: rust.fontEmbedding.sourceFonts
      }
    },
    signatureExtension: {
      chromium: {
        ...chromium.signatureExtension,
        estimatedEngineeringDays: "6-10"
      },
      rust: {
        ...rust.signatureExtension,
        estimatedEngineeringDays: "4-7"
      }
    }
  },
  recommendation: buildRecommendation(chromium, rust)
};

await writeFile(path.join(outputDir, "comparison.json"), `${JSON.stringify(comparison, null, 2)}\n`);
await writeFile(path.join(outputDir, "comparison.md"), markdown(comparison));
console.log(JSON.stringify(comparison, null, 2));

function run(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit", shell: false });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`)));
  });
}

function buildRecommendation(chromium, rust) {
  return {
    phaseR1: "Phase R1 首版采用 HTML / Chromium 作为主渲染器，因为它能直接复用已批准的四页视觉原型，分页和复杂布局的维护成本明显低于手工 Rust 排版。",
    integrityAndSigning: "JSON / Manifest 必须独立于渲染器生成。数字签名作为确定性的 PDF 后处理阶段接入，避免证据合同与 Chromium 或 Rust 实现绑定。",
    rustNativeRole: "Rust 原生渲染器保留为离线最小报告、灾备和字体归档参考实现；在自动排版和字体权重控制成熟前，不作为高保真主模板。",
    observed: `本机实测 Chromium ${chromium.generationMs} ms / ${chromium.bytes} bytes；Rust ${rust.generationMs} ms / ${rust.bytes} bytes。`
  };
}

function markdown(result) {
  const c = result.comparison;
  return `# HiddenShield PDF 双实现 Spike 对比

生成时间：${result.generatedAt}

| 维度 | HTML / Chromium | Rust 原生 |
|---|---:|---:|
| 页数 | ${c.pages.chromium} | ${c.pages.rust} |
| 生成耗时 | ${c.generationMs.chromium} ms（浏览器启动 ${c.generationMs.chromiumLaunch} ms） | ${c.generationMs.rust} ms |
| 文件大小 | ${c.bytes.chromium} bytes | ${c.bytes.rust} bytes |
| 嵌入字体对象 | ${c.fontEmbedding.chromium.embeddedFontFileObjects} | ${c.fontEmbedding.rust.embeddedFontFileObjects} |
| Type3 字形字体 | ${c.fontEmbedding.chromium.type3FontDictionaries} | ${c.fontEmbedding.rust.type3FontDictionaries} |
| Type0 CID 字体 | ${c.fontEmbedding.chromium.type0FontDictionaries} | ${c.fontEmbedding.rust.type0FontDictionaries} |
| ToUnicode 映射 | ${c.fontEmbedding.chromium.toUnicodeMaps} | ${c.fontEmbedding.rust.toUnicodeMaps} |
| 数字签名扩展成本 | ${c.signatureExtension.chromium.estimatedCost}，${c.signatureExtension.chromium.estimatedEngineeringDays} 人日 | ${c.signatureExtension.rust.estimatedCost}，${c.signatureExtension.rust.estimatedEngineeringDays} 人日 |
| 视觉还原度 | 高，直接复用 Phase R0 HTML | 中，需手工维护坐标、换行和密度 |

## 结论

- Phase R1 主渲染器：${result.recommendation.phaseR1}
- 完整性与签名：${result.recommendation.integrityAndSigning}
- Rust 原生定位：${result.recommendation.rustNativeRole}
- 本机观测：${result.recommendation.observed}
- 字体结论：Chromium 将 Noto 字形转换为大量 Type3 字体字形并附 ToUnicode；Rust 原生 PDF 嵌入两套可提取的 Noto TTF 子集。面向归档与长期验证时，Rust 字体结构更清晰。
- 数字签名人日是本次架构评估，不是已经实现或验证的能力；两条路径都仍需 CMS / PAdES、证书校验、RFC 3161、撤销与长期验证工作。
`;
}

function round(value) {
  return Math.round(value * 1000) / 1000;
}
