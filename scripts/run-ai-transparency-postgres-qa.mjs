import { spawn, spawnSync } from "node:child_process";
import { randomInt } from "node:crypto";
import { assertDisposablePostgresDatabaseUrl } from "./ai-transparency-postgres-qa-contract.mjs";

const image = process.env.HIDDENSHIELD_POSTGRES_TEST_IMAGE || "postgres:16-alpine";
const password = process.env.HIDDENSHIELD_POSTGRES_TEST_PASSWORD || "hiddenshield";
const databaseName = "hiddenshield_migrate_smoke_ai_transparency_qa";
const externalUrl =
  process.env.HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL || process.env.DATABASE_URL;
const runners = [
  "postgres_migrate_smoke",
  "ai_transparency_approval_concurrency_qa",
  "ai_transparency_confirm_concurrency_qa",
  "ai_transparency_credential_custody_qa",
  "ai_transparency_image_marking_executor_qa",
  "ai_transparency_external_evidence_review_qa",
  "ai_transparency_platform_api_qa",
  "ai_transparency_post_embed_signing_qa",
];

let containerName = null;
let databaseUrl = externalUrl
  ? assertDisposablePostgresDatabaseUrl(externalUrl)
  : null;
let runtime = null;

try {
  if (!databaseUrl) {
    runtime = detectContainerRuntime();
    if (!runtime) {
      throw new Error(
        "ai-transparency:postgres-qa requires HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Podman/Docker.",
      );
    }
    const port = String(randomInt(35433, 45432));
    containerName = `hiddenshield-ai-transparency-qa-${Date.now()}-${randomInt(1000, 9999)}`;
    databaseUrl = `postgres://postgres:${password}@127.0.0.1:${port}/${databaseName}`;
    await run(runtime, [
      "run",
      "--detach",
      "--name",
      containerName,
      "-e",
      `POSTGRES_PASSWORD=${password}`,
      "-e",
      `POSTGRES_DB=${databaseName}`,
      "-p",
      `${port}:5432`,
      image,
    ]);
    await waitForPostgres(runtime, containerName);
  }

  for (const runner of runners) {
    await run("cargo", [
      "run",
      "--manifest-path",
      "feedback-backend/Cargo.toml",
      "--features",
      "postgres",
      "--bin",
      runner,
    ], {
      env: { ...process.env, HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: databaseUrl },
    });
  }
  console.log(
    JSON.stringify({
      ok: true,
      schemaVersion: "hs-ai-transparency-postgres-qa-suite-v1",
      runners,
      database: externalUrl ? "external_disposable_url" : "ephemeral_container",
    }),
  );
} finally {
  if (containerName && runtime) {
    await run(runtime, ["rm", "--force", containerName], { allowFailure: true });
  }
}

async function waitForPostgres(containerRuntime, name) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const result = spawnSync(command(containerRuntime), ["exec", name, "pg_isready", "-U", "postgres"], {
      encoding: "utf8",
      shell: process.platform === "win32",
    });
    if (result.status === 0) {
      return;
    }
    await sleep(1_000);
  }
  throw new Error("timed out waiting for disposable PostgreSQL");
}

function run(bin, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command(bin), args, {
      env: options.env || process.env,
      stdio: "inherit",
      shell: process.platform === "win32",
    });
    child.on("exit", (code) => {
      if (code === 0 || options.allowFailure) {
        resolve();
      } else {
        reject(new Error(`${bin} ${args.join(" ")} failed with exit code ${code}`));
      }
    });
    child.on("error", reject);
  });
}

function detectContainerRuntime() {
  for (const candidate of ["podman", "docker"]) {
    const result = spawnSync(command(candidate), ["--version"], {
      encoding: "utf8",
      shell: process.platform === "win32",
    });
    if (result.status === 0) {
      return candidate;
    }
  }
  return null;
}

function command(bin) {
  if (process.platform !== "win32") {
    return bin;
  }
  return { cargo: "cargo.exe", podman: "podman.exe", docker: "docker.exe" }[bin] || bin;
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
