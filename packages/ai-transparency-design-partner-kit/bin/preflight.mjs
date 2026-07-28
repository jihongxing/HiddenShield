#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { validateDesignPartnerSandboxKit } from "../src/index.mjs";

const bundlePath = process.argv[2];
if (!bundlePath) {
  console.error("usage: hiddenshield-ai-partner-preflight <partner-kit.json>");
  process.exit(2);
}

let bundle;
try {
  bundle = JSON.parse(await readFile(resolve(bundlePath), "utf8"));
} catch (error) {
  console.error(JSON.stringify({
    valid: false,
    readiness: "invalid",
    errors: [`could not read partner kit: ${error.message}`],
    warnings: []
  }));
  process.exit(2);
}

const result = validateDesignPartnerSandboxKit(bundle);
console.log(JSON.stringify(result, null, 2));
if (!result.valid) process.exit(1);
