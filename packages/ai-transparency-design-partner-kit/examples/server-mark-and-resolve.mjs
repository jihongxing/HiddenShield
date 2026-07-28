import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";

import {
  createAiTransparencyPlatformFacade,
  createAiTransparencySdk,
} from "@hiddenshield/ai-transparency-sdk";
import { buildResolverUrl } from "@hiddenshield/ai-transparency-design-partner-kit";

const apiBaseUrl = required("HIDDENSHIELD_AI_SANDBOX_API_BASE_URL");
const resolverBaseUrl = required("HIDDENSHIELD_AI_SANDBOX_RESOLVER_BASE_URL");
const credential = required("HIDDENSHIELD_AI_SANDBOX_CREDENTIAL");
const imagePath = required("HIDDENSHIELD_AI_SANDBOX_IMAGE_PATH");

const sdk = createAiTransparencySdk({
  baseUrl: apiBaseUrl,
  credential,
});
const platform = createAiTransparencyPlatformFacade(sdk);
const imageBytes = new Uint8Array(await readFile(imagePath));
const idempotencyKey = randomUUID();

const result = await platform.markAndConfirmGeneratedImage({
  admission: {
    licenseId: required("HIDDENSHIELD_AI_SANDBOX_LICENSE_ID"),
    tenantId: required("HIDDENSHIELD_AI_SANDBOX_TENANT_ID"),
    workspaceId: required("HIDDENSHIELD_AI_SANDBOX_WORKSPACE_ID"),
    issuerMode: required("HIDDENSHIELD_AI_SANDBOX_ISSUER_MODE"),
    regulatoryProfileId: required("HIDDENSHIELD_AI_SANDBOX_REGULATORY_PROFILE_ID"),
    technicalProfileIds: required("HIDDENSHIELD_AI_SANDBOX_TECHNICAL_PROFILE_IDS")
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
  },
  idempotencyKey,
  generationEventId: randomUUID(),
  subjectReference: randomUUID(),
  imageBytes,
});

const resolverUrl = buildResolverUrl({
  resolverBaseUrl,
  watermarkUid: result.submission.watermarkUid,
});
const resolverResponse = await fetch(resolverUrl);
if (!resolverResponse.ok) {
  throw new Error(`Resolver failed with HTTP ${resolverResponse.status}`);
}
const publicRecord = await resolverResponse.json();
if (
  publicRecord.resolutionStatus !== "confirmed" ||
  publicRecord.legalConclusion !== false
) {
  throw new Error("Resolver response failed closed");
}

console.log(JSON.stringify({
  markingSessionId: result.session.markingSessionId,
  manifestId: result.confirmation.manifestId,
  watermarkUid: result.submission.watermarkUid,
  meteringUnit: result.confirmation.meteringReceipt.meteringUnit,
  resolverStatus: publicRecord.resolutionStatus,
}));

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`missing ${name}`);
  return value;
}
