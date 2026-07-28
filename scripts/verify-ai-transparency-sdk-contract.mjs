import { readFileSync } from "node:fs";

const files = {
  packageJson: readFileSync(
    "packages/ai-transparency-sdk/package.json",
    "utf8",
  ),
  source: readFileSync(
    "packages/ai-transparency-sdk/src/index.ts",
    "utf8",
  ),
  readme: readFileSync(
    "packages/ai-transparency-sdk/README.md",
    "utf8",
  ),
  fixture: readFileSync(
    "docs/contracts/ai-transparency/platform-sdk-facade-v1.fixture.json",
    "utf8",
  ),
  schema: readFileSync(
    "docs/contracts/ai-transparency/platform-sdk-facade-v1.schema.json",
    "utf8",
  ),
};

const fixture = JSON.parse(files.fixture);
const packageJson = JSON.parse(files.packageJson);

const assertions = [
  [
    packageJson.name === "@hiddenshield/ai-transparency-sdk",
    "package name",
  ],
  [packageJson.engines?.node === ">=20", "trusted Node runtime"],
  [
    fixture.schemaVersion ===
      "hs-ai-transparency-platform-sdk-facade-v1",
    "fixture schema version",
  ],
  [
    fixture.metering.unit === "confirmed_marked_image" &&
      fixture.metering.quantity === 1 &&
      fixture.metering.ledgerStatus === "committed",
    "metering contract",
  ],
  [
    fixture.releaseBoundary.publicEndpoint === "closed" &&
      fixture.releaseBoundary.productionCredentialIssuance === "closed",
    "release boundary",
  ],
  [
    files.source.includes("admitProductionProfile") &&
      files.source.includes("createGenerationSession") &&
      files.source.includes("submitGeneratedImage") &&
      files.source.includes("confirmGeneratedAsset"),
    "SDK flow surface",
  ],
  [
    files.source.includes("createAiTransparencyPlatformFacade") &&
      files.source.includes("createAiTransparencyPlatformApiFacade") &&
      files.source.includes("markAndConfirmGeneratedImage"),
    "platform facade",
  ],
  [
    files.source.includes("marked_image_digest_mismatch") &&
      files.source.includes("metering_receipt_invalid") &&
      files.source.includes("service_unavailable") &&
      files.source.includes("MAX_GENERATED_IMAGE_BYTES"),
    "fail-closed error model",
  ],
  [
    files.source.includes('CONFIRMED_MARKED_IMAGE = "confirmed_marked_image"') &&
      files.source.includes('ledgerStatus: "committed"'),
    "confirmed marked image receipt",
  ],
  [
    files.readme.includes("does not implement watermark algorithms") &&
      files.readme.includes("not approved for browser"),
    "runtime and watermark boundary",
  ],
  [
    files.schema.includes(
      "HiddenShield AI Transparency Platform SDK and Facade v1",
    ),
    "JSON Schema",
  ],
];

const failed = assertions.filter(([passed]) => !passed);
if (failed.length > 0) {
  for (const [, name] of failed) {
    console.error(`failed: ${name}`);
  }
  process.exit(1);
}

console.log(
  JSON.stringify({
    ok: true,
    packageName: packageJson.name,
    flow: fixture.flow,
    meteringUnit: fixture.metering.unit,
    releaseBoundary: fixture.releaseBoundary,
  }),
);
