import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  createAiTransparencyPlatformFacade,
  createAiTransparencySdk,
} from "../packages/ai-transparency-sdk/dist/index.js";

const baseUrl = required("HIDDENSHIELD_AI_PLATFORM_QA_BASE_URL");
const credential = required("HIDDENSHIELD_AI_PLATFORM_QA_CREDENTIAL");
const licenseId = required("HIDDENSHIELD_AI_PLATFORM_QA_LICENSE_ID");
const tenantId = required("HIDDENSHIELD_AI_PLATFORM_QA_TENANT_ID");
const workspaceId = required("HIDDENSHIELD_AI_PLATFORM_QA_WORKSPACE_ID");
const imagePath = required("HIDDENSHIELD_AI_PLATFORM_QA_IMAGE_PATH");

const post = async (path, body, presentedCredential = credential) => {
  const response = await fetch(`${baseUrl}${path}`, {
    method: "POST",
    headers: {
      authorization: `Bearer ${presentedCredential}`,
      "content-type": "application/json",
      "x-hiddenshield-sdk-version": "0.1.0",
    },
    body: JSON.stringify(body),
  });
  const payload = await response.json();
  if (!response.ok) {
    const error = new Error(payload.message ?? "platform API rejected request");
    error.code = payload.errorCode;
    throw error;
  }
  return payload;
};

const transport = {
  admitProductionProfile: (request) =>
    post("/v1/ai-transparency/admissions", {
      ...request,
      environment: "production",
      mediaType: "image",
    }),
  createGenerationSession: (request) =>
    post("/v1/ai-transparency/sessions", request),
  submitGeneratedImage: (request) =>
    post("/v1/ai-transparency/images/mark", {
      markingSessionId: request.markingSessionId,
      contentType: request.contentType,
      originalFileSha256: request.originalFileSha256,
      imageBase64: Buffer.from(request.imageBytes).toString("base64"),
    }),
  confirmGeneratedAsset: (request) =>
    post("/v1/ai-transparency/images/confirm", request),
};

const sdk = createAiTransparencySdk({
  baseUrl: "https://internal.hiddenshield.local",
  credential,
  transport,
});
const facade = createAiTransparencyPlatformFacade(sdk);
const imageBytes = new Uint8Array(await readFile(imagePath));
const idempotencyKey = "platform-api-e2e-idempotency";
await assert.rejects(
  () =>
    post("/v1/ai-transparency/admissions", {
      licenseId,
      tenantId,
      workspaceId,
      issuerMode: "hiddenshield_managed",
      regulatoryProfileId: "cn_aigc_label_2025_image_export_v1",
      technicalProfileIds: ["missing-technical-profile"],
      environment: "production",
      mediaType: "image",
    }),
  (error) => error.code === "profile_not_entitled",
);
const admission = await facade.admit({
    licenseId,
    tenantId,
    workspaceId,
    issuerMode: "hiddenshield_managed",
    regulatoryProfileId: "cn_aigc_label_2025_image_export_v1",
    technicalProfileIds: ["hiddenshield_v3_image_anchor_v1"],
});
const session = await facade.createSession({
  admission,
  idempotencyKey,
  generationEventId: "generation-platform-api-e2e",
  subjectReference: "subject-platform-api-e2e",
});
const preConfirmResponse = await fetch(
  `${baseUrl}/v1/ai-transparency/public/resolve/watermarks/${encodeURIComponent(session.watermarkUid)}`,
);
assert.equal(preConfirmResponse.status, 404);
assert.equal((await preConfirmResponse.json()).resolutionStatus, "not_found");
const originalFileSha256 = createHash("sha256").update(imageBytes).digest("hex");
await assert.rejects(
  () =>
    post(
      "/v1/ai-transparency/images/mark",
      {
        markingSessionId: session.markingSessionId,
        contentType: "image/png",
        originalFileSha256,
        imageBase64: Buffer.from(imageBytes).toString("base64"),
      },
      "hsai_live_invalid_platform_api_e2e_credential",
    ),
  (error) => error.code === "credential_invalid",
);
const submission = await facade.submitImage({ session, imageBytes });
await assert.rejects(
  () =>
    post("/v1/ai-transparency/images/confirm", {
      markingSessionId: submission.markingSessionId,
      confirmationToken: submission.confirmationToken,
      markedFileSha256: "0".repeat(64),
      idempotencyKey,
    }),
  (error) => error.code === "marked_image_digest_mismatch",
);
const confirmation = await facade.confirm({ submission, idempotencyKey });

assert.equal(admission.status, "admitted");
assert.equal(session.status, "ready_to_upload");
assert.equal(submission.status, "ready_to_confirm");
assert.equal(confirmation.status, "confirmed");
assert.equal(
  confirmation.meteringReceipt.meteringUnit,
  "confirmed_marked_image",
);
assert.equal(confirmation.meteringReceipt.quantity, 1);
assert.equal(confirmation.meteringReceipt.replayed, false);

await assert.rejects(
  () =>
    post("/v1/ai-transparency/images/confirm", {
      markingSessionId: submission.markingSessionId,
      confirmationToken: submission.confirmationToken,
      markedFileSha256: submission.markedFileSha256,
      idempotencyKey: "different-confirm-idempotency-key",
    }),
  (error) => error.code === "confirm_conflict",
);
const replay = await facade.confirm({
  submission,
  idempotencyKey,
});
assert.equal(replay.meteringReceipt.ledgerEntryId, confirmation.meteringReceipt.ledgerEntryId);
assert.equal(replay.meteringReceipt.replayed, true);

const publicByWatermark = await publicResolve(
  `/v1/ai-transparency/public/resolve/watermarks/${encodeURIComponent(submission.watermarkUid)}`,
);
assert.deepEqual(
  Object.keys(publicByWatermark).sort(),
  [
    "claimType",
    "evidenceLevel",
    "evidenceVerificationStatus",
    "generatedAt",
    "issuerTrustStatus",
    "legalConclusion",
    "manifestId",
    "manifestStatus",
    "markerStatus",
    "markers",
    "metadataSignatureStatus",
    "profiles",
    "resolutionStatus",
    "schemaVersion",
    "warnings",
    "watermarkDetectionStatus",
    "watermarkUid",
  ].sort(),
);
assert.equal(publicByWatermark.resolutionStatus, "confirmed");
assert.equal(publicByWatermark.manifestId, confirmation.manifestId);
assert.equal(publicByWatermark.watermarkUid, submission.watermarkUid);
assert.equal(publicByWatermark.legalConclusion, false);
assert.equal(publicByWatermark.watermarkDetectionStatus, "verified");
assert.equal(publicByWatermark.metadataSignatureStatus, "not_present");
assert.equal(publicByWatermark.issuerTrustStatus, "not_evaluated");
for (const forbiddenField of [
  "licenseId",
  "tenantId",
  "workspaceId",
  "markingSessionId",
  "admissionId",
  "providerId",
  "systemName",
  "modelId",
  "subjectDigest",
  "ledgerEntryId",
  "confirmationToken",
]) {
  assert.equal(forbiddenField in publicByWatermark, false);
}
const publicByManifest = await publicResolve(
  `/v1/ai-transparency/public/resolve/manifests/${encodeURIComponent(confirmation.manifestId)}`,
);
assert.deepEqual(publicByManifest, publicByWatermark);
const missingResponse = await fetch(
  `${baseUrl}/v1/ai-transparency/public/resolve/watermarks/HS-00000000-00000000-00000000-00000000`,
);
assert.equal(missingResponse.status, 404);
const missing = await missingResponse.json();
assert.deepEqual(Object.keys(missing).sort(), [
  "legalConclusion",
  "resolutionStatus",
  "schemaVersion",
  "warnings",
].sort());
assert.equal(missing.resolutionStatus, "not_found");
assert.equal(missing.legalConclusion, false);

process.stdout.write(
  `${JSON.stringify({
    scenarioId: "sdk_api_facade_http_postgres",
    admissionId: admission.admissionId,
    markingSessionId: session.markingSessionId,
    manifestId: confirmation.manifestId,
    ledgerEntryId: confirmation.meteringReceipt.ledgerEntryId,
    replayed: replay.meteringReceipt.replayed,
    publicResolverStatus: publicByWatermark.resolutionStatus,
    status: "passed",
  })}\n`,
);

async function publicResolve(path) {
  const response = await fetch(`${baseUrl}${path}`, {
    headers: {
      origin: "https://public-verifier.example",
    },
  });
  assert.equal(response.status, 200);
  assert.match(response.headers.get("cache-control") ?? "", /^public,/);
  assert.equal(response.headers.get("x-content-type-options"), "nosniff");
  assert.equal(response.headers.get("access-control-allow-origin"), "*");
  return response.json();
}

function required(name) {
  const value = process.env[name];
  if (!value) throw new Error(`missing ${name}`);
  return value;
}
