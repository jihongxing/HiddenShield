import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { createInterface } from "node:readline";
import { existsSync } from "node:fs";
import { readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { performance } from "node:perf_hooks";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.HIDDENSHIELD_PLAYWRIGHT_MODULE ?? "playwright");

const args = parseArgs(process.argv.slice(2));
const resourceDir = path.resolve(args.resourceDir ?? import.meta.dirname);
const templatePath = path.join(resourceDir, "template.html");
const fontSansPath = path.join(resourceDir, "fonts", "NotoSansSC-Controlled.ttf");
const fontSerifPath = path.join(resourceDir, "fonts", "NotoSerifSC-Controlled.ttf");
const maxGenerationMs = Number(args.maxGenerationMs ?? 3000);

for (const requiredPath of [templatePath, fontSansPath, fontSerifPath]) {
  if (!existsSync(requiredPath)) {
    throw new Error(`Missing report PDF resource: ${requiredPath}`);
  }
}

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    const relativePath = url.pathname === "/" ? "template.html" : decodeURIComponent(url.pathname.slice(1));
    const filePath = path.resolve(resourceDir, relativePath);
    if (!filePath.startsWith(`${resourceDir}${path.sep}`) && filePath !== resourceDir) {
      response.writeHead(403).end("Forbidden");
      return;
    }
    const body = await readFile(filePath);
    response.writeHead(200, {
      "content-type": contentType(filePath),
      "cache-control": "public, max-age=31536000, immutable"
    });
    response.end(body);
  } catch (error) {
    response.writeHead(404).end(String(error));
  }
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const port = typeof address === "object" && address ? address.port : 0;

const executablePath = [
  process.env.HIDDENSHIELD_CHROMIUM_PATH,
  chromium.executablePath(),
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe"
].find((candidate) => candidate && existsSync(candidate));

if (!executablePath) {
  throw new Error("No Chromium-compatible browser executable found");
}

const launchStarted = performance.now();
const browser = await chromium.launch({
  headless: true,
  executablePath,
  args: ["--disable-background-networking", "--disable-component-update"]
});
const page = await browser.newPage({ viewport: { width: 1280, height: 1600 } });
await page.goto(`http://127.0.0.1:${port}/template.html`, { waitUntil: "networkidle" });
await page.addStyleTag({
  content: `
    @font-face {
      font-family: "HiddenShield Noto Sans SC";
      src: url("/fonts/NotoSansSC-Controlled.ttf") format("truetype");
      font-weight: 400;
      font-style: normal;
    }
    @font-face {
      font-family: "HiddenShield Noto Serif SC";
      src: url("/fonts/NotoSerifSC-Controlled.ttf") format("truetype");
      font-weight: 700;
      font-style: normal;
    }
    :root {
      --font-sans: "HiddenShield Noto Sans SC", sans-serif;
      --font-serif: "HiddenShield Noto Serif SC", serif;
    }
    html, body, body * {
      font-family: "HiddenShield Noto Sans SC", sans-serif !important;
    }
    .cover-title, .section-title, .summary-statement h3, .boundary-column h3 {
      font-family: "HiddenShield Noto Serif SC", serif !important;
    }
    .cover-verification {
      display: none !important;
    }
  `
});
await page.evaluate(async () => {
  await document.fonts.load('16px "HiddenShield Noto Sans SC"', "版权证据技术报告");
  await document.fonts.load('24px "HiddenShield Noto Serif SC"', "执行摘要与证据链");
  await document.fonts.ready;
});
await page.pdf({
  format: "A4",
  printBackground: true,
  preferCSSPageSize: true,
  margin: { top: "0", right: "0", bottom: "0", left: "0" }
});

writeMessage({
  type: "ready",
  launchMs: round(performance.now() - launchStarted),
  executablePath,
  resourceDir
});

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  if (!line.trim()) continue;
  let request;
  try {
    request = JSON.parse(line);
    if (request.type === "shutdown") break;
    if (request.type !== "render") throw new Error(`Unsupported request type: ${request.type}`);
    writeMessage(await renderReport(request));
  } catch (error) {
    writeMessage({
      type: "result",
      requestId: request?.requestId ?? null,
      ok: false,
      error: error instanceof Error ? error.message : String(error)
    });
  }
}

await browser.close();
await new Promise((resolve) => server.close(resolve));

async function renderReport(request) {
  const started = performance.now();
  const sample = formalDocumentToTemplateSample(request.document);
  await page.evaluate(async (value) => {
    if (!window.HiddenShieldReportTemplate) {
      throw new Error("Report template bridge is unavailable");
    }
    await window.HiddenShieldReportTemplate.render(value);
  }, sample);
  await page.waitForSelector("#report-stack:not(.is-switching)");
  await page.evaluate(async () => {
    await document.fonts.ready;
  });

  const pageOverflow = await page.evaluate(() => [...document.querySelectorAll(".report-page")].map((element, index) => ({
    page: index + 1,
    clientHeight: element.clientHeight,
    scrollHeight: element.scrollHeight,
    overflow: element.scrollHeight > element.clientHeight + 2
  })));
  if (pageOverflow.some((entry) => entry.overflow)) {
    throw new Error(`Report page overflow detected: ${JSON.stringify(pageOverflow)}`);
  }

  const pdf = await page.pdf({
    format: "A4",
    printBackground: true,
    preferCSSPageSize: true,
    margin: { top: "0", right: "0", bottom: "0", left: "0" }
  });
  const generationMs = performance.now() - started;
  if (generationMs > maxGenerationMs) {
    throw new Error(
      `REPORT_PDF_GENERATION_BUDGET_EXCEEDED: ${round(generationMs)}ms > ${maxGenerationMs}ms`
    );
  }

  await writeFile(request.outputPath, pdf);
  const outputStat = await stat(request.outputPath);
  return {
    type: "result",
    requestId: request.requestId,
    ok: true,
    generationMs: round(generationMs),
    pageCount: pageOverflow.length,
    bytes: outputStat.size,
    sha256: createHash("sha256").update(pdf).digest("hex"),
    pageOverflow,
    fontState: await page.evaluate(() => ({
      sansLoaded: document.fonts.check('16px "HiddenShield Noto Sans SC"', "版权证据技术报告"),
      serifLoaded: document.fonts.check('24px "HiddenShield Noto Serif SC"', "执行摘要与证据链")
    }))
  };
}

function formalDocumentToTemplateSample(document) {
  const records = Array.isArray(document.records) ? document.records : [];
  const record = records[0] ?? {};
  const reportType = document.reportType === "batch_summary" ? "批量摘要" : "单份报告";
  const verificationStatus = record.writeVerificationStatus ?? "未记录";
  const verificationPassed = /verified|通过|success/i.test(verificationStatus);
  const registryStatus = record.payloadRegistry?.watermarkIdRegistryStatus ?? "未记录";
  const tsaPresent = record.trustedTime?.tsaTokenPresent === true;
  const originalHash = record.originalHash ?? "未记录";
  const protectedHash = record.protectedCopy?.hash ?? "未记录";
  const payloadVersion = record.payloadRegistry?.payloadProtocolVersion ?? 0;
  const revision = record.revision ?? 1;
  const creator = record.creatorDisplayName ?? "未提供";
  const workTitle = stripExtension(record.fileName ?? `${reportType} ${document.reportId}`);
  const shortUid = String(record.watermarkUid ?? document.reportId ?? "UNAVAILABLE").slice(-9);

  const verified = [
    originalHash !== "未记录" ? "原始媒体 SHA-256 已写入版权库记录。" : "原始媒体摘要未记录。",
    protectedHash !== "未记录" ? "保护副本 SHA-256 已记录。" : "保护副本摘要未记录。",
    verificationPassed ? "保护副本完成写后读取验证。" : `写后读取状态为 ${verificationStatus}。`,
    "报告隐私策略排除原始媒体、保护副本和本地路径。"
  ];
  const gaps = [
    `版权编号登记状态为 ${registryStatus}。`,
    tsaPresent ? "TSA token 已记录，但本报告尚未执行独立签名长期验证。" : "未记录 TSA token。",
    "创作者与作品来源属于用户声明，不等于平台完成身份或权属鉴定。"
  ];
  if (records.length > 1) {
    gaps.unshift(`本报告为批量摘要，共包含 ${records.length} 条记录；封面展示首条记录。`);
  }

  return {
    templateVersion: "报告版本 R1.1",
    workTitle,
    workShort: workTitle.slice(0, 18),
    reportId: document.reportId,
    watermarkUid: record.watermarkUid ?? "未记录",
    creator,
    exportedAt: document.exportedAt,
    statusLabel: verificationPassed ? "保护已验证" : "待补充",
    statusCode: verificationPassed ? "记录可复核" : "请查看说明",
    shortCode: shortUid,
    coverStatus: verificationPassed
      ? "作品保护副本已完成读取验证。报告将已确认的记录与仍需补充的材料分别呈现，便于后续归档与沟通。"
      : "本报告已保存当前作品记录；部分保护或登记材料仍待补充，请结合后续说明查看。",
    summaryHeadline: verificationPassed
      ? "作品保护记录已完成验证"
      : "作品保护记录仍有材料待补充",
    summaryNarrative: `本报告汇总 ${records.length} 条作品保护记录，展示作品信息、保护结果和使用边界。报告不包含原始媒体、保护副本或本地路径。`,
    confidence: verificationPassed ? "通过" : "待补",
    scoreCaption: verificationPassed ? "保护副本读取验证完成" : "请查看待补充材料",
    mediaType: mediaTypeLabel(record.fileName, record.durationSecs),
    mediaSpec: record.resolution || (record.durationSecs ? `${Number(record.durationSecs).toFixed(2)} 秒` : "未记录"),
    createdAt: record.createdAt ?? "未记录",
    payload: payloadVersion > 0 ? "已建立作品保护记录" : "保护记录待补充",
    revision: `第 ${revision} 个保护版本`,
    originalHash: originalHash !== "未记录" ? "校验摘要已归档" : "未记录",
    verified,
    gaps,
    chain: [
      ["版权库记录", "读取正式版权记录和声明字段。", "verified"],
      ["事实模型", `构建 FormalReportDocument schema v${document.schemaVersion}。`, "verified"],
      ["写后验证", `状态：${verificationStatus}。`, verificationPassed ? "verified" : "external"],
      ["可信时间", tsaPresent ? "TSA token 状态已记录。" : "未记录 TSA token。", tsaPresent ? "external" : ""],
      ["PDF 渲染", "常驻 Chromium worker 使用冻结模板与受控中文字体生成。", "verified"],
      ["JSON 快照", "序列化同一 FormalReportDocument。", "verified"],
      ["Manifest", "记录 PDF 与 JSON 的 SHA-256、大小和生成信息。", "verified"],
      ["数字签名", "当前未签名；后续作为确定性后处理阶段接入。", "external"]
    ],
    timeline: [
      [record.createdAt ?? "未记录", "建立版权记录", "保存作品摘要、版权编号和声明字段。"],
      [record.writeVerificationAt ?? "未记录", "写后读取验证", record.writeVerificationMessage ?? verificationStatus],
      [document.exportedAt, "生成报告事实快照", `生成 ${records.length} 条记录的 FormalReportDocument。`],
      [document.exportedAt, "生成报告三件套", "输出 report.pdf、report.json 与 manifest.json。"]
    ],
    integrity: [
      ["原始媒体", originalHash !== "未记录" ? "摘要已记录" : "未记录", compactHash(originalHash)],
      ["保护副本", protectedHash !== "未记录" ? "摘要已记录" : "未记录", compactHash(protectedHash)],
      ["TSA 材料", tsaPresent ? "token 已记录" : "未记录", record.trustedTime?.tsaSource ?? "none"],
      ["Registry", registryStatus, record.payloadRegistry?.watermarkIdRegistryReceipt ?? "无收据"],
      ["报告 Manifest", "本次生成", "包含 PDF / JSON 摘要"]
    ],
    generatorNote: "本报告由 HiddenShield 根据当前作品保护记录生成。报告包包含报告、记录数据与校验清单；当前 PDF 未附数字签名。",
    boundaryVerified: [
      "报告字段来自本次 FormalReportDocument 快照。",
      originalHash !== "未记录" ? "原始媒体摘要已记录。" : "未记录原始媒体摘要。",
      verificationPassed ? "保护副本完成写后读取验证。" : `写后读取状态为 ${verificationStatus}。`,
      "报告不包含原始媒体、保护副本或本地路径。"
    ],
    boundaryDeclared: [
      `权利声明主体：${creator}。`,
      `作品来源声明：${record.rightsDeclaration?.workSourceDeclaration ?? "未提供"}。`,
      `AI 训练许可声明：${record.rightsDeclaration?.trainingPermissionDeclaration ?? "未提供"}。`,
      "声明内容由填写者承担真实性责任。"
    ],
    boundaryExcluded: [
      "不证明声明主体拥有排他的法定著作权。",
      "不证明第三方作品构成侵权。",
      "不替代公证、司法鉴定或律师法律意见。",
      "不承诺任何平台、仲裁机构或法院必然采纳。"
    ],
    actionOneTitle: registryStatus.includes("confirm") ? "保留登记收据" : "补充登记确认",
    actionOneBody: registryStatus.includes("confirm")
      ? "妥善保留登记收据，并在后续报告版本中持续校验。"
      : "联网后完成版权编号登记确认，并将真实 receipt 纳入报告附件。",
    actionTwoTitle: "保留原始证据材料",
    actionTwoBody: "原始媒体不包含在报告中，应在独立介质中按原始摘要妥善保管。"
  };
}

function parseArgs(values) {
  const result = {};
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (!value.startsWith("--")) continue;
    const [rawKey, inlineValue] = value.slice(2).split("=", 2);
    result[rawKey] = inlineValue ?? values[++index];
  }
  return result;
}

function writeMessage(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".ttf")) return "font/ttf";
  return "application/octet-stream";
}

function compactHash(value) {
  if (!value || value === "未记录" || value.length <= 28) return value || "未记录";
  return `${value.slice(0, 14)}…${value.slice(-14)}`;
}

function stripExtension(value) {
  return value.replace(/\.[^.]+$/, "");
}

function mediaTypeLabel(fileName = "", durationSecs = 0) {
  const extension = path.extname(fileName).slice(1).toUpperCase() || "UNKNOWN";
  if (["PNG", "JPG", "JPEG", "WEBP", "BMP", "TIFF"].includes(extension)) return `图片 · ${extension}`;
  if (["WAV", "MP3", "FLAC", "M4A", "AAC", "OGG"].includes(extension)) return `音频 · ${extension}`;
  if (["MP4", "MOV", "MKV", "WEBM"].includes(extension)) return `L2 视频记录 · ${extension}`;
  return durationSecs > 0 ? `时序媒体 · ${extension}` : `媒体文件 · ${extension}`;
}

function round(value) {
  return Math.round(value * 100) / 100;
}
