import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";

const repoRoot = path.resolve(import.meta.dirname, "..");
const prototypeDir = path.join(
  repoRoot,
  "docs",
  "prototypes",
  "rights-evidence-pack-r4",
);
const htmlPath = path.join(prototypeDir, "finalized.html");
const pdfPath = path.join(prototypeDir, "finalized.pdf");
const metricsPath = path.join(prototypeDir, "finalized.json");
const coverPreviewPath = path.join(prototypeDir, "cover-preview.png");
const attachmentPreviewPath = path.join(prototypeDir, "attachment-preview.png");
const pagePath = "/docs/prototypes/rights-evidence-pack-r4/finalized.html";
const fontDir = path.join(repoRoot, "src-tauri", "resources", "report-pdf", "fonts");

await mkdir(prototypeDir, { recursive: true });

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    if (url.pathname === "/favicon.ico") {
      response.writeHead(204).end();
      return;
    }
    if (url.pathname === "/__fonts/NotoSansSC-Controlled.ttf") {
      return await sendFile(response, path.join(fontDir, "NotoSansSC-Controlled.ttf"), "font/ttf");
    }
    if (url.pathname === "/__fonts/NotoSerifSC-Controlled.ttf") {
      return await sendFile(response, path.join(fontDir, "NotoSerifSC-Controlled.ttf"), "font/ttf");
    }
    const requested = url.pathname === "/" ? pagePath : decodeURIComponent(url.pathname);
    const filePath = path.resolve(repoRoot, `.${requested}`);
    if (!filePath.startsWith(repoRoot)) {
      response.writeHead(403).end("Forbidden");
      return;
    }
    return await sendFile(response, filePath, contentType(filePath));
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
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].find((candidate) => candidate && existsSync(candidate));

if (!executablePath) throw new Error("No Chromium-compatible browser executable found");

const browser = await chromium.launch({ headless: true, executablePath });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  await page.goto(`http://127.0.0.1:${port}${pagePath}`, { waitUntil: "networkidle" });
  await page.waitForFunction(
    () => document.documentElement.dataset.prototypeReady === "true",
  );
  await page.evaluate(async () => {
    await document.fonts.ready;
  });

  const pageCount = await page.locator(".report-page").count();
  assert(pageCount === 8, `expected 8 report pages, received ${pageCount}`);

  const overflow = await page.evaluate(() =>
    [...document.querySelectorAll(".report-page")].map((element, index) => ({
      page: index + 1,
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      overflow: element.scrollHeight > element.clientHeight + 2,
    })),
  );
  assert(!overflow.some((item) => item.overflow), "prototype contains page overflow");

  const fontState = await page.evaluate(() => ({
    sansLoaded: document.fonts.check(
      '16px "Noto Sans SC Controlled"',
      "维权证据包",
    ),
    serifLoaded: document.fonts.check(
      '24px "Noto Serif SC Controlled"',
      "案件材料",
    ),
    bodyFont: getComputedStyle(document.body).fontFamily,
    titleFont: getComputedStyle(document.querySelector(".page-title")).fontFamily,
  }));
  assert(fontState.sansLoaded && fontState.serifLoaded, "controlled fonts did not load");

  await page.locator('[data-page="1"]').screenshot({ path: coverPreviewPath });
  await page.locator('[data-page="8"]').screenshot({ path: attachmentPreviewPath });

  const pdfBytes = await page.pdf({
    format: "A4",
    printBackground: true,
    preferCSSPageSize: true,
    margin: { top: "0", right: "0", bottom: "0", left: "0" },
  });
  await writeFile(pdfPath, pdfBytes);

  const metrics = {
    status: "passed",
    schemaVersion: 1,
    documentType: "rights_evidence_pack",
    fixturePath: "docs/contracts/rights-evidence-pack-v1.fixture.json",
    htmlPath: path.relative(repoRoot, htmlPath).replaceAll("\\", "/"),
    pdfPath: path.relative(repoRoot, pdfPath).replaceAll("\\", "/"),
    previewPaths: [
      path.relative(repoRoot, coverPreviewPath).replaceAll("\\", "/"),
      path.relative(repoRoot, attachmentPreviewPath).replaceAll("\\", "/"),
    ],
    pageCount,
    pdfBytes: pdfBytes.length,
    pdfSha256: createHash("sha256").update(pdfBytes).digest("hex"),
    overflow,
    controlledFonts: [
      "NotoSansSC-Controlled.ttf",
      "NotoSerifSC-Controlled.ttf",
    ],
    fontState,
    capabilityBoundary: {
      signatureStatus: "not_signed",
      trustedTimeStatus: "not_timestamped",
      legalConclusionStatus: "not_evaluated",
    },
  };
  await writeFile(metricsPath, `${JSON.stringify(metrics, null, 2)}\n`);
  console.log(JSON.stringify(metrics, null, 2));
} finally {
  await browser.close();
  server.close();
}

async function sendFile(response, filePath, type) {
  const body = await readFile(filePath);
  response.writeHead(200, {
    "content-type": type,
    "cache-control": "no-store",
  });
  response.end(body);
}

function contentType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".json")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".ttf")) return "font/ttf";
  return "application/octet-stream";
}

function assert(condition, message) {
  if (!condition) throw new Error(`R4 prototype generation failed: ${message}`);
}
