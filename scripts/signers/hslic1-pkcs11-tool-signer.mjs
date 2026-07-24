import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const options = parseArguments(process.argv.slice(2));
const request = JSON.parse(readFileSync(0, "utf8"));
const pin = process.env[options.pinEnv];

if (!pin) fail(`missing PIN environment variable: ${options.pinEnv}`);
if (
  request.schemaVersion !== 1 ||
  request.operation !== "ed25519_sign" ||
  request.keyHandle !== options.keyHandle ||
  !["license", "revocation"].includes(request.purpose)
) {
  fail("invalid HSLIC1 external signer request");
}

const tempRoot = mkdtempSync(path.join(tmpdir(), "hiddenshield-pkcs11-"));
try {
  const inputPath = path.join(tempRoot, "message.bin");
  const outputPath = path.join(tempRoot, "signature.bin");
  writeFileSync(
    inputPath,
    Buffer.from(request.messageBase64Url, "base64url"),
  );
  const result = spawnSync(
    options.pkcs11Tool,
    [
      "--module",
      options.module,
      "--token-label",
      options.tokenLabel,
      "--login",
      "--pin",
      `env:${options.pinEnv}`,
      "--sign",
      "--mechanism",
      "EDDSA",
      "--label",
      options.keyLabel,
      "--input-file",
      inputPath,
      "--output-file",
      outputPath,
    ],
    { encoding: "utf8", windowsHide: true },
  );
  if (result.status !== 0) {
    fail(`PKCS#11 signing failed: ${result.stderr || result.stdout}`);
  }
  const signature = readFileSync(outputPath);
  if (signature.length !== 64) {
    fail(`PKCS#11 Ed25519 signature must be 64 bytes, got ${signature.length}`);
  }
  process.stdout.write(
    JSON.stringify({
      schemaVersion: 1,
      keyId: request.keyId,
      signatureBase64Url: signature.toString("base64url"),
    }),
  );
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

function parseArguments(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index]?.replace(/^--/, "");
    const value = args[index + 1];
    if (!key || !value) fail("invalid PKCS#11 signer arguments");
    parsed[key] = value;
  }
  for (const required of [
    "pkcs11-tool",
    "module",
    "token-label",
    "key-label",
    "key-handle",
    "pin-env",
  ]) {
    if (!parsed[required]) fail(`missing --${required}`);
  }
  return {
    pkcs11Tool: parsed["pkcs11-tool"],
    module: parsed.module,
    tokenLabel: parsed["token-label"],
    keyLabel: parsed["key-label"],
    keyHandle: parsed["key-handle"],
    pinEnv: parsed["pin-env"],
  };
}

function fail(message) {
  console.error(message);
  process.exit(2);
}
