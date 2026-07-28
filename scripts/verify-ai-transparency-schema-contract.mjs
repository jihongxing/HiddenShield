import { readFileSync } from "node:fs";

const root = "docs/contracts/ai-transparency";
const fixtureNames = [
  "production-license-v1.fixture.json",
  "three-region-profile-entitlements-v1.fixture.json",
  "confirmed-marked-image-v1.fixture.json",
  "free-public-resolver-v1.fixture.json",
  "expired-license-rejection-v1.fixture.json",
  "profile-entitlement-rejection-v1.fixture.json",
  "duplicate-confirm-rejection-v1.fixture.json",
];

const schema = readJson(`${root}/ai-transparency-fixture-v1.schema.json`);
const publicResolverSchema = readJson(
  `${root}/public-resolver-v1.schema.json`,
);
const fixtures = Object.fromEntries(
  fixtureNames.map((name) => [name, readJson(`${root}/${name}`)]),
);

assertSchemaContract(schema);
assertFixtureEnvelopes(schema, fixtures);
assertProductionLicenseContract(fixtures);
assertProfileEntitlementContract(fixtures);
assertConfirmAndMeteringContract(fixtures);
assertPublicResolverContract(fixtures);
assertPublicResolverSchema(publicResolverSchema);
assertRejectionContract(fixtures);

console.log(
  `AI Transparency schema contract passed (${fixtureNames.length} fixtures)`,
);

function assertSchemaContract(candidate) {
  assert(
    candidate.$id ===
      "https://hiddenshield.local/contracts/ai-transparency/fixture-v1.schema.json",
    "fixture schema must keep the frozen v1 identifier",
  );
  assert(candidate.properties?.schemaVersion?.const === 1, "schemaVersion must be frozen at 1");
  assert(
    Array.isArray(candidate.properties?.fixtureType?.enum) &&
      candidate.properties.fixtureType.enum.length === fixtureNames.length,
    "schema must enumerate every frozen fixture type",
  );
  assert(
    Array.isArray(candidate.oneOf) && candidate.oneOf.length === fixtureNames.length,
    "schema must require a structural branch for every fixture type",
  );
}

function assertFixtureEnvelopes(candidate, fixtureMap) {
  const allowedTypes = new Set(candidate.properties.fixtureType.enum);
  for (const [name, fixture] of Object.entries(fixtureMap)) {
    assert(fixture.schemaVersion === 1, `${name}: schemaVersion must be 1`);
    assert(
      allowedTypes.has(fixture.fixtureType),
      `${name}: fixtureType must be present in the frozen schema`,
    );
    const branch = candidate.oneOf.find(
      (item) => item.properties?.fixtureType?.const === fixture.fixtureType,
    );
    assert(branch, `${name}: schema branch must exist`);
    for (const field of branch.required ?? []) {
      assert(
        Object.hasOwn(fixture, field),
        `${name}: required schema field missing: ${field}`,
      );
    }
  }
}

function assertProductionLicenseContract(fixtureMap) {
  const { license, credentialBinding, expectedAuthorization } =
    fixtureMap["production-license-v1.fixture.json"];

  assertId(license.licenseId, "atl_", "production license ID");
  assert(license.environment === "production", "fixture license must be production");
  assert(license.status === "active", "fixture license must be active");
  assert(
    license.issuerMode === "platform_managed",
    "fixture must freeze platform managed issuer mode",
  );
  assert(
    license.publicVerificationRequired === true,
    "production license must require public verification",
  );
  assert(
    Date.parse(license.expiresAt) > Date.parse(license.effectiveAt),
    "production license must expire after it becomes effective",
  );
  assert(
    credentialBinding.licenseId === license.licenseId,
    "credential binding must point to the production license",
  );
  assert(
    credentialBinding.scopes.includes("mark:image"),
    "production credential must allow image marking",
  );
  assert(
    expectedAuthorization.canUsePublicResolverWithoutCredential === true,
    "public resolver must stay free of credential requirements",
  );
  assert(
    expectedAuthorization.canUseEnterpriseBatchVerification === false,
    "production marking credential must not silently grant batch verification",
  );
}

function assertProfileEntitlementContract(fixtureMap) {
  const production =
    fixtureMap["production-license-v1.fixture.json"].license;
  const entitlements =
    fixtureMap["three-region-profile-entitlements-v1.fixture.json"];

  assert(
    entitlements.licenseId === production.licenseId,
    "three-region entitlements must bind to the production license",
  );
  assert(
    entitlements.entitlements.length === 4,
    "fixture must contain three regulatory profiles plus one technical profile",
  );
  const profileIds = entitlements.entitlements.map((item) => item.profileId);
  for (const profileId of [
    "cn_aigc_label_2025_image_export_v1",
    "eu_ai_act_article_50_2026_image_v1",
    "ca_ai_transparency_2026_image_v1",
    "c2pa_ai_output_2_4_image_v1",
  ]) {
    assert(profileIds.includes(profileId), `missing entitled profile: ${profileId}`);
  }
  assert(
    entitlements.entitlements.filter(
      (item) => item.profileKind === "regulatory",
    ).length === 3,
    "exactly three regional profiles must be regulatory",
  );
  assert(
    entitlements.entitlements.filter(
      (item) => item.profileKind === "technical",
    ).length === 1,
    "C2PA must remain a technical profile",
  );
  assert(
    entitlements.expectedAuthorization.unentitledProfileId ===
      "eu_transparency_code_signatory_2026_image_v1",
    "unentitled profile fixture must stay explicit",
  );
}

function assertConfirmAndMeteringContract(fixtureMap) {
  const fixture = fixtureMap["confirmed-marked-image-v1.fixture.json"];
  const response = fixture.expectedResponse;

  assertId(fixture.request.markingSessionId, "ats_", "marking session ID");
  assertWatermarkUid(response.watermarkUid);
  assertDigest(fixture.request.subjectDigest.value, "subject digest");
  assert(
    response.transparencyManifest.status === "active",
    "successful confirmation must create an active manifest",
  );
  assert(
    response.transparencyManifest.subjectDigest ===
      fixture.request.subjectDigest.value,
    "manifest must bind the confirmed subject digest",
  );
  assertDigest(
    response.transparencyManifest.manifestSha256,
    "manifest digest",
  );
  assert(
    fixture.request.markers.some(
      (marker) =>
        marker.markerType === "blind_watermark" &&
        marker.verifyStatus === "verified",
    ),
    "successful confirmation must include a verified blind watermark",
  );
  assert(
    fixture.request.explicitLabelReceipts.some(
      (receipt) =>
        receipt.profileId === "cn_aigc_label_2025_image_export_v1" &&
        receipt.requiredSurface === "both" &&
        receipt.verificationStatus === "verified",
    ),
    "CN export profile must include a verified file and UI label receipt",
  );
  assert(
    response.ledger.meteringUnit === "confirmed_marked_image" &&
      response.ledger.quantity === 1 &&
      response.ledger.ledgerStatus === "committed",
    "successful confirmation must create exactly one committed marking unit",
  );
  assert(
    fixture.invariants.activeManifestCountForWatermarkUid === 1 &&
      fixture.invariants.committedConfirmedMarkedImageCountForSession === 1,
    "successful confirmation must preserve single active manifest and single meter entry",
  );
  assert(
    fixture.invariants.chargeableForRetries === false &&
      fixture.invariants.chargeableForPublicVerification === false,
    "retries and public verification must remain excluded from marking metering",
  );
  assert(response.legalConclusion === false, "confirmation must not return a legal conclusion");
}

function assertPublicResolverContract(fixtureMap) {
  const confirm = fixtureMap["confirmed-marked-image-v1.fixture.json"];
  const fixture = fixtureMap["free-public-resolver-v1.fixture.json"];
  const response = fixture.expectedResponse;

  assert(
    fixture.request.method === "GET" &&
      fixture.request.authorization === null &&
      fixture.request.licenseId === null &&
      fixture.request.mediaUpload === null,
    "public resolver must require neither authorization, license, nor media upload",
  );
  assert(
    response.watermarkUid === confirm.expectedResponse.watermarkUid,
    "public resolver must resolve the confirmed watermark UID",
  );
  assert(
    fixture.invariants.requiresApiKey === false &&
      fixture.invariants.requiresLicenseId === false &&
      fixture.invariants.createsMarkingLedgerEntry === false &&
      fixture.invariants.createsBatchVerificationLedgerEntry === false &&
      fixture.invariants.storesUploadedMedia === false &&
      fixture.invariants.databaseWrites === 0 &&
      fixture.invariants.readsPublicViewsOnly === true,
    "public resolver must remain free and non-metered",
  );
  assert(
    fixture.invariants.assertsNonAiWhenNotFound === false &&
      response.legalConclusion === false &&
      response.schemaVersion === "hs-ai-public-resolver-v1" &&
      response.resolutionStatus === "confirmed" &&
      response.issuerTrustStatus === "not_evaluated",
    "public resolver must not infer non-AI content or legal conclusions",
  );
  assert(
    !("licenseId" in response) &&
      !("tenantId" in response) &&
      !("workspaceId" in response) &&
      !("subjectDigest" in response) &&
      !("ledgerEntryId" in response),
    "public resolver response must keep the frozen minimum field set",
  );
}

function assertPublicResolverSchema(candidate) {
  assert(
    candidate.$id ===
      "https://hiddenshield.internal/contracts/ai-transparency/public-resolver-v1.schema.json",
    "public resolver schema identifier must be frozen",
  );
  assert(
    Array.isArray(candidate.oneOf) &&
      candidate.oneOf.length === 2 &&
      candidate.oneOf.every((branch) => branch.additionalProperties === false),
    "public resolver confirmed and not-found responses must reject extra fields",
  );
  const confirmed = candidate.oneOf[0];
  assert(
    confirmed.properties?.legalConclusion?.const === false &&
      confirmed.properties?.issuerTrustStatus?.const === "not_evaluated",
    "public resolver must freeze legal and issuer trust boundaries",
  );
}

function assertRejectionContract(fixtureMap) {
  const expired = fixtureMap["expired-license-rejection-v1.fixture.json"];
  const profile =
    fixtureMap["profile-entitlement-rejection-v1.fixture.json"];
  const duplicate =
    fixtureMap["duplicate-confirm-rejection-v1.fixture.json"];

  assert(
    expired.expectedResponse.errorCode === "ai_license_expired" &&
      expired.invariants.watermarkUidReserved === false &&
      expired.invariants.ledgerCreated === false &&
      expired.invariants.outputMustBeFailClosed === true,
    "expired license must fail closed without reservation or metering",
  );
  assert(
    profile.expectedResponse.errorCode === "ai_profile_not_entitled" &&
      profile.invariants.scopeAloneIsSufficient === false &&
      profile.invariants.watermarkUidReserved === false &&
      profile.invariants.ledgerCreated === false &&
      profile.invariants.outputMustBeFailClosed === true,
    "profile entitlement must be enforced independently of API scope",
  );
  assert(
    duplicate.expectedResponse.errorCode === "ai_confirmation_conflict" &&
      duplicate.conflictingRequest.subjectDigest.value !==
        duplicate.existingConfirmation.subjectDigest &&
      duplicate.invariants.existingManifestMutated === false &&
      duplicate.invariants.secondManifestCreated === false &&
      duplicate.invariants.secondLedgerCreated === false &&
      duplicate.invariants.committedConfirmedMarkedImageCountForSession === 1,
    "conflicting duplicate confirmation must not mutate or re-meter the original result",
  );
}

function assertId(value, prefix, label) {
  assert(
    typeof value === "string" && value.startsWith(prefix) && value.length > prefix.length,
    `${label} must use ${prefix} opaque ID format`,
  );
}

function assertWatermarkUid(value) {
  assert(
    /^HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}$/.test(value),
    "watermark UID must preserve the frozen opaque anchor format",
  );
}

function assertDigest(value, label) {
  assert(
    typeof value === "string" && /^[a-f0-9]{64}$/.test(value),
    `${label} must be a lowercase SHA-256 hex digest`,
  );
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`AI Transparency schema contract failed: ${message}`);
  }
}
