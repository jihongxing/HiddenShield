import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { lintRlsPolicy } from "./verify-cloud-copyright-c3-contract.mjs";

const policy = await readFile("docs/contracts/cloud-copyright/c3-rls-policy-v1.sql", "utf8");
lintRlsPolicy(policy);

const mutations = [
  {
    name: "missing_force_rls",
    policy: policy.replace("ALTER TABLE cloud_copyright_records FORCE ROW LEVEL SECURITY;\n", ""),
  },
  {
    name: "bypass_rls",
    policy: policy.replaceAll("NOBYPASSRLS", "BYPASSRLS"),
  },
  {
    name: "set_role",
    policy: `${policy}\nSET ROLE hiddenshield_cloud_copyright_owner;\n`,
  },
  {
    name: "public_grant",
    policy: `${policy}\nGRANT SELECT ON cloud_copyright_records TO PUBLIC;\n`,
  },
  {
    name: "global_scope",
    policy: `${policy}\nSELECT set_config('app.workspace_id', 'ws_bad', false);\n`,
  },
];

for (const mutation of mutations) {
  assert.throws(
    () => lintRlsPolicy(mutation.policy),
    undefined,
    `RLS lint must reject mutation ${mutation.name}`,
  );
}

console.log(JSON.stringify({
  ok: true,
  gate: "cloud-copyright-c3-rls-lint-mutations-v1",
  rejectedMutations: mutations.map((mutation) => mutation.name),
}));
