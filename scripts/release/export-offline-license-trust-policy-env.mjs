import { appendFileSync, readFileSync } from "node:fs";
import path from "node:path";

const policyPath = path.resolve(
  process.argv[2] || "config/offline-license-trust-policy.production.json",
);
const policy = JSON.parse(readFileSync(policyPath, "utf8"));

if (
  policy.schemaVersion !== 1 ||
  policy.policyType !== "offline_license_trust_policy" ||
  !Array.isArray(policy.keys) ||
  policy.keys.length === 0
) {
  throw new Error("invalid production offline license trust policy");
}

const activeKey = policy.keys.find(
  (key) =>
    key.status === "active" &&
    key.algorithm === "Ed25519" &&
    key.purposes?.includes("license") &&
    key.purposes?.includes("revocation"),
);
if (!activeKey) {
  throw new Error(
    "production offline license trust policy requires an active Ed25519 license/revocation key",
  );
}

const assignment = `HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON=${JSON.stringify(policy)}`;
if (process.env.GITHUB_ENV) {
  appendFileSync(process.env.GITHUB_ENV, `${assignment}\n`, "utf8");
  console.log(
    `Loaded offline license trust policy ${activeKey.keyId} into GitHub Actions environment`,
  );
} else {
  console.log(assignment);
}
