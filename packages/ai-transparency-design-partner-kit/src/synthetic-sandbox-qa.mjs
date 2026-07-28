import { createHash } from "node:crypto";

import {
  AiTransparencySdkError,
  CONFIRMED_MARKED_IMAGE,
  createAiTransparencyPlatformApiFacade,
  createAiTransparencyPlatformFacade,
  createAiTransparencySdk
} from "../../ai-transparency-sdk/dist/index.js";

import {
  REQUIRED_ACCEPTANCE_SCENARIOS,
  validateDesignPartnerSandboxKit
} from "./index.mjs";

const QA_CONTRACT_VERSION = "hs-ai-synthetic-sandbox-qa-v1";
const PNG_BYTES = Uint8Array.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4
]);
const MARKED_PNG_BYTES = Uint8Array.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 5, 6, 7, 8
]);
const WATERMARK_UID = "HS-01234567-89ABCDEF-01234567-89ABCDEF";

export async function runSyntheticSandboxQa(template) {
  const runtime = createSyntheticRuntime();
  const startedAt = Date.now();
  const result = await runtime.facade.markAndConfirmGeneratedImage({
    admission: admissionRequest(),
    idempotencyKey: "synthetic-generation-001",
    generationEventId: "synthetic-generation-001",
    subjectReference: "synthetic-asset-001",
    imageBytes: PNG_BYTES
  });
  const replay = await runtime.sdk.confirmGeneratedAsset({
    submission: result.submission,
    idempotencyKey: "synthetic-generation-001"
  });
  const denied = await runtime.api.handle({
    method: "POST",
    path: "/v1/ai-transparency/admissions",
    headers: {},
    body: admissionRequest()
  });
  const rejectedBeforeSession = await rejectProfileBeforeSession();
  const beforeConfirm = resolveSyntheticRecord(null);
  const afterConfirm = resolveSyntheticRecord(result.confirmation);
  const bundle = structuredClone(template);
  bundle.acceptanceMatrix.scenarios = bundle.acceptanceMatrix.scenarios.map((scenario) => ({
    ...scenario,
    status: "passed",
    evidenceRef: evidenceRef(scenario.scenarioId)
  }));

  const bundleValidation = validateDesignPartnerSandboxKit(bundle);
  if (!bundleValidation.valid || bundleValidation.readiness !== "configuration_required") {
    throw new Error("synthetic bundle must remain configuration_required");
  }
  if (denied.statusCode !== 401 || runtime.calls.includes("denied-admit")) {
    throw new Error("synthetic invalid credential scenario failed");
  }
  if (!rejectedBeforeSession || beforeConfirm.status !== 404) {
    throw new Error("synthetic fail-closed scenarios failed");
  }
  if (
    afterConfirm.status !== 200 ||
    afterConfirm.body.legalConclusion !== false ||
    Object.keys(afterConfirm.body).some((key) =>
      ["licenseId", "tenantId", "workspaceId", "credential", "ledgerEntryId"].includes(key)
    )
  ) {
    throw new Error("synthetic Resolver boundary failed");
  }
  if (
    result.confirmation.meteringReceipt.meteringUnit !== CONFIRMED_MARKED_IMAGE ||
    result.confirmation.meteringReceipt.quantity !== 1 ||
    replay.meteringReceipt.ledgerEntryId !== result.confirmation.meteringReceipt.ledgerEntryId ||
    !replay.meteringReceipt.replayed
  ) {
    throw new Error("synthetic metering replay boundary failed");
  }

  return {
    contractVersion: QA_CONTRACT_VERSION,
    executionMode: "synthetic_non_acceptance",
    acceptanceStatus: "not_real_partner_acceptance",
    readiness: bundleValidation.readiness,
    externalConfigurationRequired: [
      "partner identity and approval references",
      "sandbox API and Resolver endpoints",
      "credential Secret reference",
      "real partner runtime evidence"
    ],
    scenarioIds: REQUIRED_ACCEPTANCE_SCENARIOS,
    scenarioCount: REQUIRED_ACCEPTANCE_SCENARIOS.length,
    sdkCallOrder: runtime.calls,
    latency: {
      syntheticElapsedMs: Date.now() - startedAt,
      sampleSize: 1,
      notPartnerLatencyEvidence: true
    },
    resolver: afterConfirm.body,
    metering: {
      ledgerEntryId: result.confirmation.meteringReceipt.ledgerEntryId,
      replayed: replay.meteringReceipt.replayed
    }
  };
}

function createSyntheticRuntime() {
  const calls = [];
  const entitlementDigest = digest("synthetic-entitlement");
  const transport = {
    async admitProductionProfile(request) {
      calls.push("admit");
      return {
        ...request,
        admissionId: "synthetic-admission-001",
        status: "admitted",
        environment: "production",
        entitlementVersionId: "synthetic-entitlement-version-001",
        entitlementDigest,
        expiresAt: future()
      };
    },
    async createGenerationSession(request) {
      calls.push("session");
      return {
        markingSessionId: "synthetic-session-001",
        admissionId: request.admissionId,
        licenseId: "synthetic-license-001",
        entitlementDigest,
        status: "ready_to_upload",
        watermarkUid: WATERMARK_UID,
        contentType: "image/png",
        expiresAt: future()
      };
    },
    async submitGeneratedImage(request) {
      calls.push("mark");
      return {
        markingSessionId: request.markingSessionId,
        licenseId: "synthetic-license-001",
        entitlementDigest,
        status: "ready_to_confirm",
        watermarkUid: WATERMARK_UID,
        contentType: "image/png",
        originalFileSha256: request.originalFileSha256,
        markedFileSha256: digest(MARKED_PNG_BYTES),
        markedImageBase64: Buffer.from(MARKED_PNG_BYTES).toString("base64"),
        confirmationToken: "synthetic-confirmation-token",
        markerEvidenceDigest: digest("synthetic-marker"),
        explicitLabelReceiptDigest: digest("synthetic-label")
      };
    },
    async confirmGeneratedAsset(request) {
      calls.push("confirm");
      const replayed = calls.filter((call) => call === "confirm").length > 1;
      return {
        status: "confirmed",
        manifestId: "synthetic-manifest-001",
        markingSessionId: request.markingSessionId,
        watermarkUid: WATERMARK_UID,
        verificationUrl: "https://sandbox-resolver.hiddenshield.invalid/synthetic",
        profileStatus: "applied",
        explicitLabel: {
          text: "AI generated",
          requiredSurface: "both"
        },
        meteringReceipt: {
          receiptId: "synthetic-metering-receipt-001",
          ledgerEntryId: "synthetic-ledger-001",
          licenseId: "synthetic-license-001",
          markingSessionId: request.markingSessionId,
          meteringUnit: CONFIRMED_MARKED_IMAGE,
          quantity: 1,
          ledgerStatus: "committed",
          committedAt: new Date().toISOString(),
          replayed
        }
      };
    }
  };
  const sdk = createAiTransparencySdk({
    baseUrl: "https://synthetic-api.hiddenshield.invalid",
    credential: "hs_synthetic_runtime_only",
    transport
  });
  return {
    calls,
    sdk,
    facade: createAiTransparencyPlatformFacade(sdk),
    api: createAiTransparencyPlatformApiFacade({
      sdk,
      authorize: (request) => request.headers?.["x-synthetic-authorized"] === "true"
    })
  };
}

async function rejectProfileBeforeSession() {
  const sdk = createAiTransparencySdk({
    baseUrl: "https://synthetic-api.hiddenshield.invalid",
    credential: "hs_synthetic_runtime_only",
    transport: {
      async admitProductionProfile() {
        throw new AiTransparencySdkError({
          code: "profile_not_entitled",
          category: "entitlement",
          message: "Synthetic Profile denial"
        });
      },
      async createGenerationSession() {
        throw new Error("session must not be called");
      },
      async submitGeneratedImage() {
        throw new Error("mark must not be called");
      },
      async confirmGeneratedAsset() {
        throw new Error("confirm must not be called");
      }
    }
  });
  try {
    await sdk.admitProductionProfile(admissionRequest());
    return false;
  } catch (error) {
    return error instanceof AiTransparencySdkError && error.code === "profile_not_entitled";
  }
}

function resolveSyntheticRecord(confirmation) {
  if (!confirmation) {
    return { status: 404, body: { resolutionStatus: "not_found" } };
  }
  return {
    status: 200,
    body: {
      resolutionStatus: "confirmed",
      manifestId: confirmation.manifestId,
      watermarkUid: confirmation.watermarkUid,
      profileStatus: confirmation.profileStatus,
      legalConclusion: false
    }
  };
}

function admissionRequest() {
  return {
    licenseId: "synthetic-license-001",
    tenantId: "synthetic-tenant-001",
    workspaceId: "synthetic-workspace-001",
    issuerMode: "hiddenshield_managed",
    regulatoryProfileId: "cn_aigc_label_2025_image_export_v1",
    technicalProfileIds: ["hiddenshield_v3_image_anchor_v1"]
  };
}

function evidenceRef(scenarioId) {
  return `evidence://sha256/${digest(`synthetic:${scenarioId}`)}`;
}

function digest(value) {
  return createHash("sha256").update(value).digest("hex");
}

function future() {
  return new Date(Date.now() + 60_000).toISOString();
}
