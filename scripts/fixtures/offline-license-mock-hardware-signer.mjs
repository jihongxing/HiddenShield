import { createPrivateKey, sign } from "node:crypto";
import { readFileSync } from "node:fs";

const TEST_SEED_HEX =
  "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const TEST_KEY_ID = "offline-test-k0";
const TEST_KEY_HANDLE = "fixture://offline-test-k0";
const PKCS8_ED25519_PREFIX = Buffer.from(
  "302e020100300506032b657004220420",
  "hex",
);

const request = JSON.parse(readFileSync(0, "utf8"));
if (
  request.schemaVersion !== 1 ||
  request.operation !== "ed25519_sign" ||
  request.keyId !== TEST_KEY_ID ||
  request.keyHandle !== TEST_KEY_HANDLE ||
  !["license", "revocation"].includes(request.purpose) ||
  typeof request.messageBase64Url !== "string"
) {
  process.exitCode = 2;
} else {
  const privateKey = createPrivateKey({
    key: Buffer.concat([
      PKCS8_ED25519_PREFIX,
      Buffer.from(TEST_SEED_HEX, "hex"),
    ]),
    format: "der",
    type: "pkcs8",
  });
  const signature = sign(
    null,
    Buffer.from(request.messageBase64Url, "base64url"),
    privateKey,
  );
  process.stdout.write(
    JSON.stringify({
      schemaVersion: 1,
      keyId: TEST_KEY_ID,
      signatureBase64Url: signature.toString("base64url"),
    }),
  );
}
