const HSLIC1_PREFIX = "HSLIC1";
const HSREQ1_PREFIX = "HSREQ1";
const HSRVL1_PREFIX = "HSRVL1";
const HSLIC1_SIGNATURE_DOMAIN = "HiddenShield-Offline-License-v1";
const HSREQ1_CHECKSUM_DOMAIN =
  "HiddenShield-Offline-Activation-Request-v1";
const HSRVL1_SIGNATURE_DOMAIN =
  "HiddenShield-Offline-Revocation-List-v1";
const INSTALLATION_ID_DOMAIN = "HiddenShield-Installation-v1";
const HSLIC1_PAYLOAD_KEYS = [
  "expiresAt",
  "installationId",
  "issuedAt",
  "keyId",
  "licenseId",
  "notBefore",
  "productCode",
  "schemaVersion",
] as const;
const HSREQ1_PAYLOAD_KEYS = [
  "appVersion",
  "createdAt",
  "installationId",
  "nonce",
  "platform",
  "requestId",
  "requestedProductCode",
  "schemaVersion",
] as const;
const HSRVL1_PAYLOAD_KEYS = [
  "generatedAt",
  "keyId",
  "listId",
  "listType",
  "revokedLicenseIds",
  "schemaVersion",
  "sequence",
] as const;
const TIMESTAMP_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;
const INSTALLATION_ID_PATTERN = /^[A-Za-z0-9_-]{43}$/;
const NONCE_PATTERN = /^[A-Za-z0-9_-]{22}$/;
const IDENTIFIER_PATTERN = /^[a-z0-9][a-z0-9._-]{2,63}$/;
const APP_VERSION_PATTERN =
  /^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$/;
const PLATFORMS = new Set([
  "windows",
  "macos",
  "linux",
  "android",
  "ios",
]);

export interface OfflineLicensePayloadV1 {
  expiresAt: string;
  installationId: string;
  issuedAt: string;
  keyId: string;
  licenseId: string;
  notBefore: string;
  productCode: "creator_offline";
  schemaVersion: 1;
}

export interface ActivationRequestPayloadV1 {
  appVersion: string;
  createdAt: string;
  installationId: string;
  nonce: string;
  platform: "windows" | "macos" | "linux" | "android" | "ios";
  requestId: string;
  requestedProductCode: "creator_offline";
  schemaVersion: 1;
}

export interface RevocationListPayloadV1 {
  generatedAt: string;
  keyId: string;
  listId: string;
  listType: "offline_license_revocations";
  revokedLicenseIds: string[];
  schemaVersion: 1;
  sequence: number;
}

export interface ParsedOfflineLicenseV1 {
  payload: OfflineLicensePayloadV1;
  payloadBytes: Uint8Array;
  signatureBytes: Uint8Array;
  signingMessage: Uint8Array;
}

export interface ParsedActivationRequestV1 {
  payload: ActivationRequestPayloadV1;
  payloadBytes: Uint8Array;
  checksumBytes: Uint8Array;
}

export interface ParsedRevocationListV1 {
  payload: RevocationListPayloadV1;
  payloadBytes: Uint8Array;
  signatureBytes: Uint8Array;
  signingMessage: Uint8Array;
}

export function parseOfflineLicenseV1(token: string): ParsedOfflineLicenseV1 {
  const [payloadBytes, signatureBytes] = decodeThreeSegmentToken(
    token,
    HSLIC1_PREFIX,
    "offline_license_invalid_format",
    64,
  );
  const payloadText = decodeUtf8(payloadBytes, "offline_license_invalid_format");
  const parsed = parseObject(payloadText, "offline_license_invalid_format");
  assertExactKeys(
    parsed,
    HSLIC1_PAYLOAD_KEYS,
    "offline_license_unknown_schema",
  );
  assertCanonical(parsed, payloadText);
  if (parsed.schemaVersion !== 1) {
    throw new Error("offline_license_unknown_schema");
  }
  if (parsed.productCode !== "creator_offline") {
    throw new Error("offline_license_feature_profile_invalid");
  }
  if (
    !matches(parsed.expiresAt, TIMESTAMP_PATTERN) ||
    !matches(parsed.installationId, INSTALLATION_ID_PATTERN) ||
    !matches(parsed.issuedAt, TIMESTAMP_PATTERN) ||
    !matches(parsed.keyId, IDENTIFIER_PATTERN) ||
    !matches(parsed.licenseId, IDENTIFIER_PATTERN) ||
    !matches(parsed.notBefore, TIMESTAMP_PATTERN)
  ) {
    throw new Error("offline_license_invalid_format");
  }

  return {
    payload: parsed as unknown as OfflineLicensePayloadV1,
    payloadBytes,
    signatureBytes,
    signingMessage: signingMessage(HSLIC1_SIGNATURE_DOMAIN, payloadBytes),
  };
}

export function parseActivationRequestV1(
  token: string,
): ParsedActivationRequestV1 {
  const [payloadBytes, checksumBytes] = decodeThreeSegmentToken(
    token,
    HSREQ1_PREFIX,
    "offline_license_request_invalid_format",
    12,
  );
  const payloadText = decodeUtf8(
    payloadBytes,
    "offline_license_request_invalid_format",
  );
  const parsed = parseObject(
    payloadText,
    "offline_license_request_invalid_format",
  );
  assertExactKeys(
    parsed,
    HSREQ1_PAYLOAD_KEYS,
    "offline_license_request_unknown_schema",
  );
  assertCanonical(
    parsed,
    payloadText,
    "offline_license_request_non_canonical_payload",
  );
  if (parsed.schemaVersion !== 1) {
    throw new Error("offline_license_request_unknown_schema");
  }
  if (parsed.requestedProductCode !== "creator_offline") {
    throw new Error("offline_license_request_product_invalid");
  }
  if (
    !matches(parsed.appVersion, APP_VERSION_PATTERN) ||
    !matches(parsed.createdAt, TIMESTAMP_PATTERN) ||
    !matches(parsed.installationId, INSTALLATION_ID_PATTERN) ||
    !matches(parsed.nonce, NONCE_PATTERN) ||
    typeof parsed.platform !== "string" ||
    !PLATFORMS.has(parsed.platform) ||
    !matches(parsed.requestId, IDENTIFIER_PATTERN)
  ) {
    throw new Error("offline_license_request_invalid_format");
  }

  return {
    payload: parsed as unknown as ActivationRequestPayloadV1,
    payloadBytes,
    checksumBytes,
  };
}

export function parseRevocationListV1(
  token: string,
): ParsedRevocationListV1 {
  const [payloadBytes, signatureBytes] = decodeThreeSegmentToken(
    token,
    HSRVL1_PREFIX,
    "offline_license_revocation_invalid_format",
    64,
  );
  const payloadText = decodeUtf8(
    payloadBytes,
    "offline_license_revocation_invalid_format",
  );
  const parsed = parseObject(
    payloadText,
    "offline_license_revocation_invalid_format",
  );
  assertExactKeys(
    parsed,
    HSRVL1_PAYLOAD_KEYS,
    "offline_license_revocation_unknown_schema",
  );
  assertCanonical(
    parsed,
    payloadText,
    "offline_license_revocation_non_canonical_payload",
  );
  if (parsed.schemaVersion !== 1) {
    throw new Error("offline_license_revocation_unknown_schema");
  }
  if (parsed.listType !== "offline_license_revocations") {
    throw new Error("offline_license_revocation_list_invalid");
  }
  if (!Number.isInteger(parsed.sequence) || Number(parsed.sequence) < 1) {
    throw new Error("offline_license_revocation_sequence_invalid");
  }
  if (
    !matches(parsed.generatedAt, TIMESTAMP_PATTERN) ||
    !matches(parsed.keyId, IDENTIFIER_PATTERN) ||
    !matches(parsed.listId, IDENTIFIER_PATTERN) ||
    !isSortedUniqueIdentifiers(parsed.revokedLicenseIds)
  ) {
    throw new Error("offline_license_revocation_list_invalid");
  }

  return {
    payload: parsed as unknown as RevocationListPayloadV1,
    payloadBytes,
    signatureBytes,
    signingMessage: signingMessage(HSRVL1_SIGNATURE_DOMAIN, payloadBytes),
  };
}

export async function verifyOfflineLicenseV1Signature(
  parsed: ParsedOfflineLicenseV1,
  publicKeyBytes: Uint8Array,
): Promise<boolean> {
  return verifyEd25519(
    parsed.signingMessage,
    parsed.signatureBytes,
    publicKeyBytes,
  );
}

export async function verifyActivationRequestV1Checksum(
  parsed: ParsedActivationRequestV1,
): Promise<boolean> {
  const digest = new Uint8Array(
    await crypto.subtle.digest(
      "SHA-256",
      toArrayBuffer(signingMessage(HSREQ1_CHECKSUM_DOMAIN, parsed.payloadBytes)),
    ),
  );
  return constantTimeEqual(parsed.checksumBytes, digest.slice(0, 12));
}

export async function verifyRevocationListV1Signature(
  parsed: ParsedRevocationListV1,
  publicKeyBytes: Uint8Array,
): Promise<boolean> {
  return verifyEd25519(
    parsed.signingMessage,
    parsed.signatureBytes,
    publicKeyBytes,
  );
}

export async function validateOfflineArtifactV1(
  artifactType: "license" | "activation_request" | "revocation_list",
  token: string,
  publicKeyBytes?: Uint8Array,
): Promise<void> {
  if (artifactType === "activation_request") {
    const parsed = parseActivationRequestV1(token);
    if (!(await verifyActivationRequestV1Checksum(parsed))) {
      throw new Error("offline_license_request_checksum_mismatch");
    }
    return;
  }
  if (!publicKeyBytes) {
    throw new Error("offline_license_unknown_key");
  }
  if (artifactType === "license") {
    const parsed = parseOfflineLicenseV1(token);
    if (!(await verifyOfflineLicenseV1Signature(parsed, publicKeyBytes))) {
      throw new Error("offline_license_signature_invalid");
    }
    return;
  }
  const parsed = parseRevocationListV1(token);
  if (!(await verifyRevocationListV1Signature(parsed, publicKeyBytes))) {
    throw new Error("offline_license_revocation_signature_invalid");
  }
}

export async function deriveInstallationIdV1(
  installationSecret: Uint8Array,
  salt: Uint8Array,
): Promise<string> {
  if (installationSecret.length !== 32 || salt.length !== 16) {
    throw new Error("offline_license_secure_storage_unavailable");
  }
  const digest = new Uint8Array(
    await crypto.subtle.digest(
      "SHA-256",
      toArrayBuffer(
        concatBytes(
          signingMessage(INSTALLATION_ID_DOMAIN, installationSecret),
          salt,
        ),
      ),
    ),
  );
  return encodeBase64Url(digest);
}

export function decodeBase64Url(value: string): Uint8Array {
  return decodeBase64UrlFor(value, "offline_license_invalid_format");
}

function encodeBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

function decodeThreeSegmentToken(
  token: string,
  prefix: string,
  errorCode: string,
  trailerLength: number,
): [Uint8Array, Uint8Array] {
  if (token.trim() !== token || /\s/.test(token)) {
    throw new Error(errorCode);
  }
  const segments = token.split(".");
  if (segments.length !== 3 || segments[0] !== prefix) {
    throw new Error(errorCode);
  }
  const payloadBytes = decodeBase64UrlFor(segments[1], errorCode);
  const trailerBytes = decodeBase64UrlFor(segments[2], errorCode);
  if (trailerBytes.length !== trailerLength) {
    throw new Error(errorCode);
  }
  return [payloadBytes, trailerBytes];
}

function decodeBase64UrlFor(value: string, errorCode: string): Uint8Array {
  if (!/^[A-Za-z0-9_-]+$/.test(value)) {
    throw new Error(errorCode);
  }
  try {
    const padding = "=".repeat((4 - (value.length % 4)) % 4);
    const binary = atob(
      value.replace(/-/g, "+").replace(/_/g, "/") + padding,
    );
    return Uint8Array.from(binary, (character) => character.charCodeAt(0));
  } catch {
    throw new Error(errorCode);
  }
}

function decodeUtf8(bytes: Uint8Array, errorCode: string): string {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new Error(errorCode);
  }
}

function parseObject(
  payloadText: string,
  errorCode: string,
): Record<string, unknown> {
  try {
    const parsed = JSON.parse(payloadText) as unknown;
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      throw new Error(errorCode);
    }
    return parsed as Record<string, unknown>;
  } catch (error) {
    if (error instanceof Error && error.message === errorCode) throw error;
    throw new Error(errorCode);
  }
}

function assertExactKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
  errorCode: string,
): void {
  if (JSON.stringify(Object.keys(value)) !== JSON.stringify(keys)) {
    throw new Error(errorCode);
  }
}

function assertCanonical(
  value: Record<string, unknown>,
  payloadText: string,
  errorCode = "offline_license_non_canonical_payload",
): void {
  if (JSON.stringify(value) !== payloadText) {
    throw new Error(errorCode);
  }
}

function matches(value: unknown, pattern: RegExp): value is string {
  return typeof value === "string" && pattern.test(value);
}

function isSortedUniqueIdentifiers(value: unknown): value is string[] {
  if (!Array.isArray(value)) return false;
  let previous: string | undefined;
  for (const item of value) {
    if (!matches(item, IDENTIFIER_PATTERN)) return false;
    if (previous !== undefined && item <= previous) return false;
    previous = item;
  }
  return true;
}

function signingMessage(domain: string, payloadBytes: Uint8Array): Uint8Array {
  return concatBytes(new TextEncoder().encode(`${domain}\0`), payloadBytes);
}

async function verifyEd25519(
  message: Uint8Array,
  signatureBytes: Uint8Array,
  publicKeyBytes: Uint8Array,
): Promise<boolean> {
  if (publicKeyBytes.length !== 32) {
    throw new Error("offline_license_unknown_key");
  }
  const publicKey = await crypto.subtle.importKey(
    "raw",
    toArrayBuffer(publicKeyBytes),
    { name: "Ed25519" },
    false,
    ["verify"],
  );
  return crypto.subtle.verify(
    { name: "Ed25519" },
    publicKey,
    toArrayBuffer(signatureBytes),
    toArrayBuffer(message),
  );
}

function constantTimeEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let index = 0; index < left.length; index += 1) {
    difference |= left[index] ^ right[index];
  }
  return difference === 0;
}

function concatBytes(left: Uint8Array, right: Uint8Array): Uint8Array {
  const output = new Uint8Array(left.length + right.length);
  output.set(left);
  output.set(right, left.length);
  return output;
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return Uint8Array.from(bytes).buffer;
}
