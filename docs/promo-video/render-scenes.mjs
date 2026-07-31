import { chromium } from "playwright";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const outputDir = path.resolve(scriptDir, "../../output/promo-video/scenes");
const scenes = JSON.parse(await readFile(path.join(scriptDir, "scenes.json"), "utf8"));
const renderUrl = pathToFileURL(path.join(scriptDir, "render.html")).href;

await mkdir(outputDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
});

for (let index = 0; index < scenes.length; index += 1) {
  const sceneNumber = String(index + 1).padStart(2, "0");
  await page.goto(`${renderUrl}?scene=${index + 1}`, { waitUntil: "networkidle" });
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({
    path: path.join(outputDir, `${sceneNumber}-${scenes[index].id}.png`),
    type: "png",
  });
}

await browser.close();
console.log(`Rendered ${scenes.length} scenes to ${outputDir}`);
