import { cp, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = path.join(
  repoRoot,
  "docs",
  "fixtures",
  "rights-evidence-pack-r4",
  "case-fixture-r4-0001",
);
const targetDir = path.join(
  repoRoot,
  "mobile_app",
  "test",
  "fixtures",
  "rights_evidence_pack_r4",
  "case-fixture-r4-0001",
);

await rm(targetDir, { recursive: true, force: true });
await mkdir(path.dirname(targetDir), { recursive: true });
await cp(sourceDir, targetDir, { recursive: true });

console.log(
  JSON.stringify(
    {
      status: "synced",
      sourceDir: path.relative(repoRoot, sourceDir).replaceAll("\\", "/"),
      targetDir: path.relative(repoRoot, targetDir).replaceAll("\\", "/"),
    },
    null,
    2,
  ),
);
