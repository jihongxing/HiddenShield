import { chromium } from "playwright";
import path from "node:path";
import { fileURLToPath } from "node:url";

const currentDirectory = path.dirname(fileURLToPath(import.meta.url));
const sourcePath = path.join(currentDirectory, "HiddenShield-product-strategy-ppt-4k.html");
const outputPath = path.join(currentDirectory, "HiddenShield-product-strategy-ppt-4k.png");

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 3840, height: 2160 },
  deviceScaleFactor: 1,
});

await page.goto(`file:///${sourcePath.replaceAll("\\", "/")}`, {
  waitUntil: "networkidle",
});
await page.evaluate(() => document.fonts.ready);
await page.screenshot({
  path: outputPath,
  fullPage: false,
  animations: "disabled",
});

await browser.close();
console.log(outputPath);
