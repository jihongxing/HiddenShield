# @hiddenshield/ai-transparency-sdk

Internal-only server-side SDK for HiddenShield AI generated image transparency marking.

The package implements the production-oriented orchestration contract:

1. production license and Profile admission
2. marking session creation
3. PNG submission and marked-image digest verification
4. atomic confirm
5. `confirmed_marked_image` metering receipt validation

The SDK does not implement watermark algorithms. HiddenShield backend services remain wrappers around the shared `watermark-core`.

```ts
import {
  createAiTransparencyPlatformApiFacade,
  createAiTransparencyPlatformFacade,
  createAiTransparencySdk,
} from "@hiddenshield/ai-transparency-sdk";

const sdk = createAiTransparencySdk({
  baseUrl: "https://api.example.com",
  credential: process.env.HIDDENSHIELD_AI_TRANSPARENCY_CREDENTIAL!,
});

const facade = createAiTransparencyPlatformFacade(sdk);
const result = await facade.markAndConfirmGeneratedImage({
  admission: {
    licenseId: "license_platform_prod",
    tenantId: "tenant_platform",
    workspaceId: "workspace_platform",
    issuerMode: "hiddenshield_managed",
    regulatoryProfileId: "cn_aigc_label_2025_image_export_v1",
    technicalProfileIds: ["hs_ai_image_v3_c2pa_v1"],
  },
  idempotencyKey: "generation-event-001",
  generationEventId: "generation-event-001",
  subjectReference: "asset-001",
  imageBytes: pngBytes,
});
```

Framework-neutral API routing can wrap the same SDK:

```ts
const api = createAiTransparencyPlatformApiFacade({
  sdk,
  authorize: async (request) =>
    request.headers?.["x-platform-service-token"] ===
    process.env.PLATFORM_SERVICE_TOKEN,
});

const response = await api.handle({
  method: "POST",
  path: "/v1/ai-transparency/admissions",
  headers,
  body,
});
```

Production credentials must only be injected into trusted server runtimes. The package is not approved for browser, desktop bundle, mobile bundle, public npm release, or production credential issuance.
