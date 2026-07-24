import { createServer } from "node:http";
import { readFile, stat, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { chromium } from "playwright";

const args = parseArgs(process.argv.slice(2));
const repoRoot = path.resolve(args.repoRoot ?? path.join(import.meta.dirname, "..", ".."));
const outputPath = path.resolve(args.output ?? path.join(repoRoot, "output", "report-pdf-spike", "chromium-report.pdf"));
const metricsPath = path.resolve(args.metrics ?? path.join(path.dirname(outputPath), "chromium-metrics.json"));
const pagePath = "/docs/prototypes/copyright-evidence-report-r0/finalized.html";

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/__fonts/NotoSansSC-VF.ttf") {
      response.writeHead(200, { "content-type": "font/ttf", "cache-control": "no-store" });
      response.end(await readFile("C:\\Windows\\Fonts\\NotoSansSC-VF.ttf"));
      return;
    }
    if (url.pathname === "/__fonts/NotoSerifSC-VF.ttf") {
      response.writeHead(200, { "content-type": "font/ttf", "cache-control": "no-store" });
      response.end(await readFile("C:\\Windows\\Fonts\\NotoSerifSC-VF.ttf"));
      return;
    }
    const requested = url.pathname === "/" ? pagePath : url.pathname;
    const filePath = path.resolve(repoRoot, `.${decodeURIComponent(requested)}`);
    if (!filePath.startsWith(repoRoot)) {
      response.writeHead(403).end("Forbidden");
      return;
    }
    const body = await readFile(filePath);
    response.writeHead(200, { "content-type": contentType(filePath) });
    response.end(body);
  } catch (error) {
    response.writeHead(404).end(String(error));
  }
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const port = typeof address === "object" && address ? address.port : 0;

const launchStarted = performance.now();
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
const browser = await chromium.launch({
  headless: true,
  executablePath
});
const launchMs = performance.now() - launchStarted;
const page = await browser.newPage();

const renderStarted = performance.now();
await page.goto(`http://127.0.0.1:${port}${pagePath}`, { waitUntil: "networkidle" });
await page.selectOption("#sample-select", "image");
await page.waitForSelector("#report-stack:not(.is-switching)");
await page.evaluate(async () => {
  await document.fonts.ready;
  const style = document.createElement("style");
  style.textContent = `
    @font-face {
      font-family: "Noto Sans SC Spike";
      src: url("/__fonts/NotoSansSC-VF.ttf") format("truetype");
      font-weight: 100 900;
    }
    @font-face {
      font-family: "Noto Serif SC Spike";
      src: url("/__fonts/NotoSerifSC-VF.ttf") format("truetype");
      font-weight: 100 900;
    }
    :root {
      --font-sans: "Noto Sans SC Spike", "Microsoft YaHei UI", sans-serif;
      --font-serif: "Noto Serif SC Spike", "SimSun", serif;
    }
    html, body, body * {
      font-family: "Noto Sans SC Spike", sans-serif !important;
    }
    .cover-title, .section-title, .summary-statement h3, .boundary-column h3 {
      font-family: "Noto Serif SC Spike", serif !important;
    }
  `;
  document.head.append(style);
  await document.fonts.ready;
});
const renderReadyMs = performance.now() - renderStarted;

await writeFile(outputPath, await page.pdf({
  format: "A4",
  printBackground: true,
  preferCSSPageSize: true,
  margin: { top: "0", right: "0", bottom: "0", left: "0" }
}));
const totalMs = performance.now() - renderStarted;

const pageCount = await page.locator(".report-page").count();
const overflow = await page.evaluate(() => [...document.querySelectorAll(".report-page")].map((element, index) => ({
  page: index + 1,
  clientHeight: element.clientHeight,
  scrollHeight: element.scrollHeight,
  overflow: element.scrollHeight > element.clientHeight + 2
})));
const browserFontState = await page.evaluate(() => ({
  notoSansLoaded: document.fonts.check('16px "Noto Sans SC Spike"', "版权证据技术报告"),
  notoSerifLoaded: document.fonts.check('24px "Noto Serif SC Spike"', "版权证据技术报告"),
  bodyFont: getComputedStyle(document.body).fontFamily,
  titleFont: getComputedStyle(document.querySelector(".cover-title")).fontFamily
}));

await browser.close();
await new Promise((resolve) => server.close(resolve));

const pdf = await readFile(outputPath);
const raw = pdf.toString("latin1");
const metrics = {
  engine: "chromium",
  implementation: "Playwright page.pdf over Phase R0 HTML",
  outputPath,
  launchMs: round(launchMs),
  renderReadyMs: round(renderReadyMs),
  generationMs: round(totalMs),
  bytes: (await stat(outputPath)).size,
  sha256: createHash("sha256").update(pdf).digest("hex"),
  pageCount: (raw.match(/\/Type\s*\/Page\b/g) ?? []).length,
  domPageCount: pageCount,
  pageOverflow: overflow,
  fontEmbedding: inspectFonts(raw),
  browserFontState,
  signatureExtension: {
    nativeSupport: false,
    estimatedCost: "high",
    notes: "Chromium emits a finished PDF. PAdES/CMS signing requires a separate post-processing library or signing service, incremental update handling, certificate validation, timestamping and revocation support."
  }
};

await writeFile(metricsPath, `${JSON.stringify(metrics, null, 2)}\n`);
console.log(JSON.stringify(metrics, null, 2));

function inspectFonts(rawPdf) {
  const baseFonts = [...rawPdf.matchAll(/\/BaseFont\s*\/([^\s/<>\[\]()]+)/g)].map((match) => match[1]);
  return {
    embeddedFontFileObjects: (rawPdf.match(/\/FontFile(?:2|3)?\b/g) ?? []).length,
    toUnicodeMaps: (rawPdf.match(/\/ToUnicode\b/g) ?? []).length,
    subsetFontNames: [...new Set(baseFonts.filter((name) => /^[A-Z]{6}\+/.test(name)))],
    baseFonts: [...new Set(baseFonts)].slice(0, 20)
  };
}

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".json")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".ttf")) return "font/ttf";
  return "application/octet-stream";
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const key = values[index];
    if (!key.startsWith("--")) continue;
    parsed[key.slice(2)] = values[index + 1];
    index += 1;
  }
  return parsed;
}

function round(value) {
  return Math.round(value * 100) / 100;
}
