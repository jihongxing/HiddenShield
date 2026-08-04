import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";

const targets = [
  "docs/contracts/cloud-copyright",
  "docs/云版权库C3_RLS身份Receipt与PostgreSQLScope_QA设计评审.md",
  "docs/云版权库C3_外部配置与恢复演练交接模板.md",
  "docs/云版权库C3_旧同步与内部API隔离审计.md",
];
const findings = [];

for (const target of targets) {
  for (const path of await expand(target)) {
    const text = await readFile(path, "utf8");
    for (const [name, pattern] of [
      ["pem_private_key", /-----BEGIN (?:RSA |EC )?PRIVATE KEY-----/],
      ["jwt", /\beyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\b/],
      ["postgres_connection_with_password", /postgres(?:ql)?:\/\/[^/\s:]+:[^@\s]+@/i],
      ["literal_secret_assignment", /(?:password|access[_-]?token|refresh[_-]?token|private[_-]?key|client[_-]?secret)\s*[:=]\s*["'][^"<{\s][^"']{7,}["']/i],
    ]) {
      if (pattern.test(text)) {
        findings.push({ path, name });
      }
    }
  }
}

for (const path of await expand("docs/contracts/cloud-copyright")) {
  const text = await readFile(path, "utf8");
  if (path.includes("c3-") || path.endsWith("C3_README.md")) {
    if (/<[^>\n]+>|\b(?:TODO|TBD|placeholder)\b/i.test(text)) {
      findings.push({ path, name: "placeholder_in_contract_asset" });
    }
  }
}

assert.deepEqual(findings, [], `C3 artifacts must not contain literal secret material: ${JSON.stringify(findings)}`);
console.log(JSON.stringify({
  ok: true,
  gate: "cloud-copyright-c3-secret-scan-v1",
  scannedTargets: targets.length,
  findings: 0,
}));

async function expand(target) {
  const entries = await readdir(target, { withFileTypes: true }).catch(() => null);
  if (!entries) {
    return [target];
  }
  return entries
    .filter((entry) => entry.isFile())
    .map((entry) => join(target, entry.name))
    .filter((path) => path.includes("c3-") || path.endsWith("C3_README.md"));
}
