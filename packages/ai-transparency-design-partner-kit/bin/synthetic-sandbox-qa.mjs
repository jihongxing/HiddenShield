import { readFile } from "node:fs/promises";

import { runSyntheticSandboxQa } from "../src/synthetic-sandbox-qa.mjs";

const template = JSON.parse(
  await readFile(
    new URL("../templates/design-partner-sandbox-kit.template.json", import.meta.url),
    "utf8"
  )
);

console.log(JSON.stringify(await runSyntheticSandboxQa(template), null, 2));
