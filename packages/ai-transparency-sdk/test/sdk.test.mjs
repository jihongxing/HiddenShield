import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import {
  AiTransparencySdkError,
  CONFIRMED_MARKED_IMAGE,
  createAiTransparencyPlatformApiFacade,
  createAiTransparencyPlatformFacade,
  createAiTransparencySdk,
} from "../dist/index.js";

const png = Uint8Array.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4,
]);
const markedPng = Uint8Array.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 5, 6, 7, 8,
]);
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const future = () => new Date(Date.now() + 60_000).toISOString();

const admissionRequest = {
  licenseId: "license-production-001",
  tenantId: "tenant-platform-001",
  workspaceId: "workspace-platform-001",
  issuerMode: "hiddenshield_managed",
  regulatoryProfileId: "cn_aigc_label_2025_image_export_v1",
  technicalProfileIds: ["hs_ai_image_v3_c2pa_v1"],
};

function successTransport(overrides = {}) {
  const calls = [];
  const transport = {
    async admitProductionProfile(request) {
      calls.push("admit");
      return {
        ...request,
        admissionId: "admission-001",
        status: "admitted",
        environment: "production",
        entitlementVersionId: "entitlement-version-001",
        entitlementDigest: digest(Buffer.from("entitlement")),
        expiresAt: future(),
      };
    },
    async createGenerationSession(request) {
      calls.push("session");
      return {
        markingSessionId: "session-001",
        admissionId: request.admissionId,
        licenseId: admissionRequest.licenseId,
        entitlementDigest: digest(Buffer.from("entitlement")),
        status: "ready_to_upload",
        watermarkUid: "wm_ai_001",
        contentType: "image/png",
        expiresAt: future(),
      };
    },
    async submitGeneratedImage(request) {
      calls.push("mark");
      return {
        markingSessionId: request.markingSessionId,
        licenseId: admissionRequest.licenseId,
        entitlementDigest: digest(Buffer.from("entitlement")),
        status: "ready_to_confirm",
        watermarkUid: "wm_ai_001",
        contentType: "image/png",
        originalFileSha256: request.originalFileSha256,
        markedFileSha256: digest(markedPng),
        markedImageBase64: Buffer.from(markedPng).toString("base64"),
        confirmationToken: "confirmation-token-001",
        markerEvidenceDigest: digest(Buffer.from("marker-evidence")),
        explicitLabelReceiptDigest: digest(Buffer.from("label-receipt")),
      };
    },
    async confirmGeneratedAsset(request) {
      calls.push("confirm");
      return {
        status: "confirmed",
        manifestId: "manifest-001",
        markingSessionId: request.markingSessionId,
        licenseId: admissionRequest.licenseId,
        entitlementDigest: digest(Buffer.from("entitlement")),
        watermarkUid: "wm_ai_001",
        verificationUrl: "https://verify.example.com/wm_ai_001",
        profileStatus: "applied",
        explicitLabel: {
          text: "AI generated",
          requiredSurface: "both",
        },
        meteringReceipt: {
          receiptId: "metering-receipt-001",
          ledgerEntryId: "ledger-001",
          licenseId: admissionRequest.licenseId,
          markingSessionId: request.markingSessionId,
          meteringUnit: CONFIRMED_MARKED_IMAGE,
          quantity: 1,
          ledgerStatus: "committed",
          committedAt: new Date().toISOString(),
          replayed: false,
        },
      };
    },
    ...overrides,
  };
  return { transport, calls };
}

function createSdk(transport) {
  return createAiTransparencySdk({
    baseUrl: "https://api.hiddenshield.test",
    credential: "hs_prod_credential_for_internal_test",
    transport,
  });
}

test("platform facade executes admission, session, mark, confirm in order", async () => {
  const { transport, calls } = successTransport();
  const facade = createAiTransparencyPlatformFacade(createSdk(transport));
  const result = await facade.markAndConfirmGeneratedImage({
    admission: admissionRequest,
    idempotencyKey: "generation-event-001",
    generationEventId: "generation-event-001",
    subjectReference: "asset-001",
    imageBytes: png,
  });

  assert.deepEqual(calls, ["admit", "session", "mark", "confirm"]);
  assert.equal(result.confirmation.status, "confirmed");
  assert.equal(
    result.confirmation.meteringReceipt.meteringUnit,
    CONFIRMED_MARKED_IMAGE,
  );
  assert.equal(result.confirmation.meteringReceipt.quantity, 1);
  assert.equal(result.confirmation.meteringReceipt.ledgerStatus, "committed");
});

test("marked image digest mismatch fails closed before confirm", async () => {
  const { transport, calls } = successTransport({
    async submitGeneratedImage(request) {
      calls.push("mark");
      return {
        markingSessionId: request.markingSessionId,
        licenseId: admissionRequest.licenseId,
        entitlementDigest: digest(Buffer.from("entitlement")),
        status: "ready_to_confirm",
        watermarkUid: "wm_ai_001",
        contentType: "image/png",
        originalFileSha256: request.originalFileSha256,
        markedFileSha256: digest(Buffer.from("wrong")),
        markedImageBase64: Buffer.from(markedPng).toString("base64"),
        confirmationToken: "confirmation-token-001",
        markerEvidenceDigest: digest(Buffer.from("marker-evidence")),
        explicitLabelReceiptDigest: digest(Buffer.from("label-receipt")),
      };
    },
  });
  const facade = createAiTransparencyPlatformFacade(createSdk(transport));

  await assert.rejects(
    () =>
      facade.markAndConfirmGeneratedImage({
        admission: admissionRequest,
        idempotencyKey: "generation-event-002",
        generationEventId: "generation-event-002",
        subjectReference: "asset-002",
        imageBytes: png,
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "marked_image_digest_mismatch" &&
      error.category === "integrity",
  );
  assert.deepEqual(calls, ["admit", "session", "mark"]);
});

test("license or Profile admission failure stops all later operations", async () => {
  const { transport, calls } = successTransport({
    async admitProductionProfile() {
      calls.push("admit");
      throw new AiTransparencySdkError({
        code: "profile_not_entitled",
        category: "entitlement",
        message: "Profile denied",
      });
    },
  });
  const facade = createAiTransparencyPlatformFacade(createSdk(transport));

  await assert.rejects(
    () =>
      facade.markAndConfirmGeneratedImage({
        admission: admissionRequest,
        idempotencyKey: "generation-event-003",
        generationEventId: "generation-event-003",
        subjectReference: "asset-003",
        imageBytes: png,
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "profile_not_entitled",
  );
  assert.deepEqual(calls, ["admit"]);
});

test("invalid metering receipt fails closed", async () => {
  const { transport } = successTransport({
    async confirmGeneratedAsset(request) {
      return {
        status: "confirmed",
        manifestId: "manifest-invalid-metering",
        markingSessionId: request.markingSessionId,
        watermarkUid: "wm_ai_001",
        verificationUrl: "https://verify.example.com/wm_ai_001",
        profileStatus: "applied",
        explicitLabel: {
          text: "AI generated",
          requiredSurface: "both",
        },
        meteringReceipt: {
          receiptId: "metering-receipt-invalid",
          ledgerEntryId: "ledger-invalid",
          licenseId: admissionRequest.licenseId,
          markingSessionId: request.markingSessionId,
          meteringUnit: "image_attempt",
          quantity: 1,
          ledgerStatus: "committed",
          committedAt: new Date().toISOString(),
          replayed: false,
        },
      };
    },
  });
  const facade = createAiTransparencyPlatformFacade(createSdk(transport));

  await assert.rejects(
    () =>
      facade.markAndConfirmGeneratedImage({
        admission: admissionRequest,
        idempotencyKey: "generation-event-004",
        generationEventId: "generation-event-004",
        subjectReference: "asset-004",
        imageBytes: png,
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "metering_receipt_invalid",
  );
});

test("metering receipt for another license fails closed", async () => {
  const { transport } = successTransport({
    async confirmGeneratedAsset(request) {
      return {
        status: "confirmed",
        manifestId: "manifest-wrong-license",
        markingSessionId: request.markingSessionId,
        watermarkUid: "wm_ai_001",
        verificationUrl: "https://verify.example.com/wm_ai_001",
        profileStatus: "applied",
        explicitLabel: {
          text: "AI generated",
          requiredSurface: "both",
        },
        meteringReceipt: {
          receiptId: "metering-receipt-wrong-license",
          ledgerEntryId: "ledger-wrong-license",
          licenseId: "license-other-tenant",
          markingSessionId: request.markingSessionId,
          meteringUnit: CONFIRMED_MARKED_IMAGE,
          quantity: 1,
          ledgerStatus: "committed",
          committedAt: new Date().toISOString(),
          replayed: false,
        },
      };
    },
  });
  const facade = createAiTransparencyPlatformFacade(createSdk(transport));

  await assert.rejects(
    () =>
      facade.markAndConfirmGeneratedImage({
        admission: admissionRequest,
        idempotencyKey: "generation-event-wrong-license",
        generationEventId: "generation-event-wrong-license",
        subjectReference: "asset-wrong-license",
        imageBytes: png,
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "metering_receipt_invalid",
  );
});

test("duplicate confirm replay accepts the same committed ledger receipt", async () => {
  const { transport } = successTransport({
    async confirmGeneratedAsset(request) {
      return {
        status: "confirmed",
        manifestId: "manifest-001",
        markingSessionId: request.markingSessionId,
        watermarkUid: "wm_ai_001",
        verificationUrl: "https://verify.example.com/wm_ai_001",
        profileStatus: "applied",
        explicitLabel: {
          text: "AI generated",
          requiredSurface: "both",
        },
        meteringReceipt: {
          receiptId: "metering-receipt-001",
          ledgerEntryId: "ledger-001",
          licenseId: admissionRequest.licenseId,
          markingSessionId: request.markingSessionId,
          meteringUnit: CONFIRMED_MARKED_IMAGE,
          quantity: 1,
          ledgerStatus: "committed",
          committedAt: new Date().toISOString(),
          replayed: true,
        },
      };
    },
  });
  const sdk = createSdk(transport);
  const admission = await sdk.admitProductionProfile(admissionRequest);
  const session = await sdk.createGenerationSession({
    admission,
    idempotencyKey: "generation-event-005",
    generationEventId: "generation-event-005",
    subjectReference: "asset-005",
  });
  const submission = await sdk.submitGeneratedImage({
    session,
    imageBytes: png,
  });
  const confirmation = await sdk.confirmGeneratedAsset({
    submission,
    idempotencyKey: "generation-event-005",
  });

  assert.equal(confirmation.meteringReceipt.replayed, true);
  assert.equal(confirmation.meteringReceipt.ledgerEntryId, "ledger-001");
});

test("production configuration rejects placeholder credentials and HTTP", () => {
  assert.throws(
    () =>
      createAiTransparencySdk({
        baseUrl: "https://api.hiddenshield.test",
        credential: "change-me-placeholder",
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "credential_invalid",
  );
  assert.throws(
    () =>
      createAiTransparencySdk({
        baseUrl: "http://api.hiddenshield.test",
        credential: "hs_prod_credential_for_internal_test",
      }),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "credential_invalid",
  );
});

test("remote backend error maps to the stable fail-closed model", async () => {
  const sdk = createAiTransparencySdk({
    baseUrl: "https://api.hiddenshield.test",
    credential: "hs_prod_credential_for_internal_test",
    fetchImpl: async () =>
      new Response(
        JSON.stringify({
          errorCode: "ai_license_expired",
          message: "License expired",
          retryable: false,
        }),
        {
          status: 403,
          headers: {
            "content-type": "application/json",
            "x-request-id": "request-remote-001",
          },
        },
      ),
  });

  await assert.rejects(
    () => sdk.admitProductionProfile(admissionRequest),
    (error) =>
      error instanceof AiTransparencySdkError &&
      error.code === "license_expired" &&
      error.category === "entitlement" &&
      error.httpStatus === 403 &&
      error.requestId === "request-remote-001",
  );
});

test("framework-neutral API facade authorizes then calls the SDK", async () => {
  const { transport, calls } = successTransport();
  const api = createAiTransparencyPlatformApiFacade({
    sdk: createSdk(transport),
    authorize: (request) => request.headers?.["x-service-token"] === "allowed",
  });

  const denied = await api.handle({
    method: "POST",
    path: "/v1/ai-transparency/admissions",
    headers: {},
    body: admissionRequest,
  });
  assert.equal(denied.statusCode, 401);
  assert.deepEqual(calls, []);

  const admitted = await api.handle({
    method: "POST",
    path: "/v1/ai-transparency/admissions",
    headers: { "x-service-token": "allowed" },
    body: admissionRequest,
  });
  assert.equal(admitted.statusCode, 201);
  assert.equal(admitted.body.status, "admitted");
  assert.deepEqual(calls, ["admit"]);
});
