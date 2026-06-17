import { spawn } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright";

const PORT = 43210;
const BASE_URL = `http://127.0.0.1:${PORT}`;

function waitForServer(url, timeoutMs = 30_000) {
  const startedAt = Date.now();
  return new Promise((resolve, reject) => {
    const timer = setInterval(async () => {
      try {
        const response = await fetch(url);
        if (response.ok) {
          clearInterval(timer);
          resolve();
        }
      } catch {
        // Keep waiting until timeout.
      }
      if (Date.now() - startedAt > timeoutMs) {
        clearInterval(timer);
        reject(new Error(`Timed out waiting for ${url}`));
      }
    }, 500);
  });
}

async function selectFile(page, filePath) {
  const chooserPromise = page.waitForEvent("filechooser");
  await page.getByText("拖入或选择文件").click();
  const chooser = await chooserPromise;
  await chooser.setFiles(filePath);
}

async function main() {
  const tmp = await mkdtemp(join(tmpdir(), "hiddenshield-audio-preflight-"));
  const shortAudio = join(tmp, "short_10s.wav");
  const longAudio = join(tmp, "long_42s.wav");
  await writeFile(shortAudio, Buffer.from("mock wav"));
  await writeFile(longAudio, Buffer.from("mock wav"));

  const server = spawn(
    "npm",
    ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(PORT)],
    { shell: process.platform === "win32", stdio: "pipe" },
  );

  let browser;
  try {
    await waitForServer(BASE_URL);
    try {
      browser = await chromium.launch();
    } catch (error) {
      if (String(error).includes("Executable doesn't exist")) {
        console.error(
          "Playwright Chromium is not installed. Run `npx playwright install chromium` before this optional UI check.",
        );
        process.exitCode = 2;
        return;
      }
      throw error;
    }
    const page = await browser.newPage();
    await page.goto(BASE_URL, { waitUntil: "networkidle" });

    await selectFile(page, shortAudio);
    await page.getByText("音频时长不足").waitFor({ timeout: 10_000 });
    const shortButton = page.getByRole("button", { name: "生成保护副本" });
    if (!(await shortButton.isDisabled())) {
      throw new Error("Expected short audio to disable the protect button");
    }

    await selectFile(page, longAudio);
    await page.getByText("音频已就绪").waitFor({ timeout: 10_000 });
    const longButton = page.getByRole("button", { name: "生成保护副本" });
    if (await longButton.isDisabled()) {
      throw new Error("Expected 30s+ audio to enable the protect button");
    }

    console.log("Audio duration preflight verified");
  } finally {
    await browser?.close();
    server.kill();
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
