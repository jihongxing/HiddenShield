import { createPublicKey } from "node:crypto";
import { readFileSync } from "node:fs";

const options = parseArguments(process.argv.slice(2));
const request = JSON.parse(readFileSync(0, "utf8"));
const resourceName = options.cryptoKeyVersion;
const expectedKeyHandle = `gcp-kms://${resourceName}`;

if (
  request.schemaVersion !== 1 ||
  request.operation !== "ed25519_sign" ||
  request.keyId !== options.keyId ||
  request.keyHandle !== expectedKeyHandle ||
  !["license", "revocation"].includes(request.purpose) ||
  typeof request.messageBase64Url !== "string"
) {
  fail("invalid HSLIC1 Google Cloud KMS signer request");
}

const message = Buffer.from(request.messageBase64Url, "base64url");
const publicKeyResponse = await requestJson(
  `${options.apiBaseUrl}/v1/${resourceName}/publicKey`,
  "GET",
);
if (
  publicKeyResponse.name !== resourceName ||
  publicKeyResponse.algorithm !== "EC_SIGN_ED25519" ||
  !options.allowedProtectionLevels.has(publicKeyResponse.protectionLevel)
) {
  fail("Google Cloud KMS key contract mismatch");
}

const rawPublicKey = ed25519RawPublicKey(publicKeyResponse.pem);
if (rawPublicKey.toString("base64url") !== options.expectedPublicKeyBase64Url) {
  fail("Google Cloud KMS public key does not match HSLIC1 trust policy");
}

const dataCrc32c = crc32c(message);
const signResponse = await requestJson(
  `${options.apiBaseUrl}/v1/${resourceName}:asymmetricSign`,
  "POST",
  {
    data: message.toString("base64"),
    dataCrc32c: String(dataCrc32c),
  },
);
if (
  signResponse.name !== resourceName ||
  signResponse.verifiedDataCrc32c !== true ||
  !options.allowedProtectionLevels.has(signResponse.protectionLevel) ||
  typeof signResponse.signature !== "string"
) {
  fail("Google Cloud KMS signing response contract mismatch");
}

const signature = Buffer.from(signResponse.signature, "base64");
if (
  signature.length !== 64 ||
  Number(signResponse.signatureCrc32c) !== crc32c(signature)
) {
  fail("Google Cloud KMS signature integrity check failed");
}

process.stdout.write(
  JSON.stringify({
    schemaVersion: 1,
    keyId: request.keyId,
    signatureBase64Url: signature.toString("base64url"),
  }),
);

async function requestJson(url, method, data) {
  if (options.testMode) {
    const response = await fetch(url, {
      method,
      headers: {
        Authorization: `Bearer ${process.env.HIDDENSHIELD_GOOGLE_KMS_TEST_TOKEN || "test"}`,
        "Content-Type": "application/json",
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    if (!response.ok) {
      fail(`Google Cloud KMS test endpoint rejected request: ${response.status}`);
    }
    return response.json();
  }

  const { GoogleAuth } = await import("google-auth-library");
  const auth = new GoogleAuth({
    scopes: ["https://www.googleapis.com/auth/cloudkms"],
  });
  const client = await auth.getClient();
  const response = await client.request({ url, method, data });
  return response.data;
}

function parseArguments(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index]?.replace(/^--/, "");
    const value = args[index + 1];
    if (!key || !value) fail("invalid Google Cloud KMS signer arguments");
    parsed[key] = value;
  }
  for (const required of [
    "crypto-key-version",
    "key-id",
    "expected-public-key-base64url",
  ]) {
    if (!parsed[required]) fail(`missing --${required}`);
  }
  const resourcePattern =
    /^projects\/[a-z][a-z0-9-]{4,28}[a-z0-9]\/locations\/[a-z0-9-]+\/keyRings\/[A-Za-z0-9_-]+\/cryptoKeys\/[A-Za-z0-9_-]+\/cryptoKeyVersions\/[1-9][0-9]*$/;
  if (!resourcePattern.test(parsed["crypto-key-version"])) {
    fail("invalid Google Cloud KMS CryptoKeyVersion resource name");
  }
  const testMode = process.env.HIDDENSHIELD_GOOGLE_KMS_TEST_MODE === "1";
  const apiBaseUrl =
    parsed["api-base-url"] || "https://cloudkms.googleapis.com";
  if (
    apiBaseUrl !== "https://cloudkms.googleapis.com" &&
    (!testMode || !/^http:\/\/127\.0\.0\.1:\d+$/.test(apiBaseUrl))
  ) {
    fail("custom Google Cloud KMS endpoint is allowed only in contract test mode");
  }
  return {
    cryptoKeyVersion: parsed["crypto-key-version"],
    keyId: parsed["key-id"],
    expectedPublicKeyBase64Url: parsed["expected-public-key-base64url"],
    allowedProtectionLevels: new Set(
      (parsed["allowed-protection-levels"] || "SOFTWARE,HSM,HSM_SINGLE_TENANT")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
    ),
    apiBaseUrl,
    testMode,
  };
}

function ed25519RawPublicKey(pem) {
  if (typeof pem !== "string" || !pem.includes("BEGIN PUBLIC KEY")) {
    fail("Google Cloud KMS did not return an Ed25519 public key");
  }
  const der = createPublicKey(pem).export({ format: "der", type: "spki" });
  const prefix = Buffer.from("302a300506032b6570032100", "hex");
  if (
    der.length !== prefix.length + 32 ||
    !der.subarray(0, prefix.length).equals(prefix)
  ) {
    fail("Google Cloud KMS public key is not Ed25519 SPKI");
  }
  return der.subarray(prefix.length);
}

function crc32c(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0x82f63b78 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function fail(message) {
  console.error(message);
  process.exit(2);
}
