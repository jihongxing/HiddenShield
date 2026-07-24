# @hiddenshield/public-rights-sdk

HiddenShield public rights scanner SDK for registry-backed training-permission lookups.

This package exposes the same semantics used by HiddenShield desktop and mobile:

- `scanOne(watermarkUid)`
- `scanBatch(watermarkUids)`
- `resolvePolicy(scanResult)`
- `formatUserMessage(result)`

The SDK never returns a legal authorization conclusion. `legalConclusion` is always `false`; callers must treat results as creator declarations and registry snapshots that may require human or legal review.

```ts
import { createPublicRightsScanner } from "@hiddenshield/public-rights-sdk";

const scanner = createPublicRightsScanner({
  baseUrl: "https://registry.example.com",
});

const result = await scanner.scanOne("wm_...");
console.log(result.message);
```

Enterprise batch scanning can use a HiddenShield Enterprise API key:

```ts
const scanner = createPublicRightsScanner({
  baseUrl: "https://registry.example.com",
  apiKey: process.env.HIDDENSHIELD_ENTERPRISE_API_KEY,
});

const rows = await scanner.scanBatch(["wm_1", "wm_2"]);
```

The package is prepared for external distribution but is not published by this repository.
