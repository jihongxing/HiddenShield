const SCHEMA_VERSION = "hs-ai-design-partner-sandbox-kit-v1";
const IMMUTABLE_EVIDENCE_REF = /^evidence:\/\/sha256\/[a-f0-9]{64}$/;
const PLACEHOLDER_REFERENCE = /(?:replace[-_]?me|placeholder)/i;

export const REQUIRED_ACCEPTANCE_SCENARIOS = Object.freeze([
  "admission_success",
  "profile_denied_fail_closed",
  "session_ready_to_upload",
  "invalid_credential_zero_state_change",
  "png_mark_write_after_read",
  "confirm_single_metering_unit",
  "confirm_replay_no_duplicate_metering",
  "resolver_preconfirm_not_found",
  "resolver_postconfirm_anonymous",
  "resolver_minimum_public_fields",
  "secret_redaction",
  "latency_budget_recorded"
]);

export function validateDesignPartnerSandboxKit(bundle) {
  const errors = [];
  const warnings = [];
  if (!isRecord(bundle)) {
    return {
      valid: false,
      readiness: "invalid",
      errors: ["bundle must be an object"],
      warnings
    };
  }
  requireExact(bundle.schemaVersion, SCHEMA_VERSION, "schemaVersion", errors);
  requireOneOf(
    bundle.packageStatus,
    ["configuration_required", "ready_for_internal_review", "approved_for_sandbox", "suspended"],
    "packageStatus",
    errors
  );
  validateOnboarding(bundle.onboarding, errors, warnings);
  validateQuestionnaire(bundle.profileMappingQuestionnaire, errors);
  validateResolverLink(bundle.resolverLink, errors);
  validateAcceptanceMatrix(bundle.acceptanceMatrix, errors);
  rejectRawSecrets(bundle, errors);

  const readiness = calculateReadiness(bundle, errors);
  return {
    valid: errors.length === 0,
    readiness,
    errors,
    warnings
  };
}

export function buildResolverUrl({ resolverBaseUrl, watermarkUid, manifestId }) {
  requireHttpsUrl(resolverBaseUrl, "resolverBaseUrl");
  const identifiers = [watermarkUid, manifestId].filter(
    (value) => typeof value === "string" && value.trim()
  );
  if (identifiers.length !== 1) {
    throw new Error("exactly one of watermarkUid or manifestId is required");
  }
  const baseUrl = resolverBaseUrl.replace(/\/+$/, "");
  if (watermarkUid) {
    if (!/^HS-[A-F0-9]{8}(?:-[A-F0-9]{8}){3}$/.test(watermarkUid)) {
      throw new Error("watermarkUid is invalid");
    }
    return `${baseUrl}/v1/ai-transparency/public/resolve/watermarks/${encodeURIComponent(watermarkUid)}`;
  }
  return `${baseUrl}/v1/ai-transparency/public/resolve/manifests/${encodeURIComponent(manifestId)}`;
}

function validateOnboarding(value, errors, warnings) {
  if (!isRecord(value)) {
    errors.push("onboarding must be an object");
    return;
  }
  for (const field of [
    "partnerId",
    "partnerLegalNameRef",
    "technicalContactRef",
    "securityContactRef",
    "sandboxApiBaseUrl",
    "credentialSecretRef",
    "resolverBaseUrl",
    "useCase",
    "issuerMode",
    "deploymentMode",
    "outputContentType"
  ]) {
    requireString(value[field], `onboarding.${field}`, errors);
  }
  requireExact(value.environment, "sandbox", "onboarding.environment", errors);
  validateHttps(value.sandboxApiBaseUrl, "onboarding.sandboxApiBaseUrl", errors);
  validateHttps(value.resolverBaseUrl, "onboarding.resolverBaseUrl", errors);
  if (
    typeof value.credentialSecretRef === "string" &&
    !value.credentialSecretRef.startsWith("secret://")
  ) {
    errors.push("onboarding.credentialSecretRef must use secret://");
  }
  requireOneOf(
    value.issuerMode,
    ["hiddenshield_managed", "customer_managed", "platform_signed"],
    "onboarding.issuerMode",
    errors
  );
  requireOneOf(
    value.deploymentMode,
    ["hosted", "private"],
    "onboarding.deploymentMode",
    errors
  );
  requireExact(
    value.outputContentType,
    "image/png",
    "onboarding.outputContentType",
    errors
  );
  requirePositiveInteger(
    value.expectedMonthlyConfirmedImages,
    "onboarding.expectedMonthlyConfirmedImages",
    errors
  );
  requirePositiveInteger(value.peakRequestsPerSecond, "onboarding.peakRequestsPerSecond", errors);
  requirePositiveInteger(value.markConfirmLatencyBudgetMs, "onboarding.markConfirmLatencyBudgetMs", errors);
  const acknowledgements = value.acknowledgements;
  if (
    !isRecord(acknowledgements) ||
    acknowledgements.nonProduction !== true ||
    acknowledgements.noLegalOpinion !== true ||
    acknowledgements.noSla !== true ||
    acknowledgements.noProductionCredential !== true
  ) {
    errors.push("onboarding acknowledgements must freeze all sandbox boundaries");
  }
  const approvalReferences = value.approvalReferences;
  if (!isRecord(approvalReferences)) {
    errors.push("onboarding.approvalReferences must be an object");
  } else {
    for (const field of [
      "partnerTechnicalSignoffRef",
      "partnerSecuritySignoffRef",
      "hiddenShieldEngineeringApprovalRef",
      "hiddenShieldCommercialApprovalRef"
    ]) {
      requireString(
        approvalReferences[field],
        `onboarding.approvalReferences.${field}`,
        errors
      );
    }
  }
  if (String(value.sandboxApiBaseUrl).includes(".invalid")) {
    warnings.push("sandbox API endpoint still requires external configuration");
  }
  if (String(value.resolverBaseUrl).includes(".invalid")) {
    warnings.push("resolver endpoint still requires external configuration");
  }
}

function validateQuestionnaire(value, errors) {
  if (!isRecord(value)) {
    errors.push("profileMappingQuestionnaire must be an object");
    return;
  }
  if (!Array.isArray(value.jurisdictions) || value.jurisdictions.length !== 3) {
    errors.push("profileMappingQuestionnaire.jurisdictions must include CN, EU, and US-CA");
  } else {
    const regions = value.jurisdictions.map((item) => item?.region).sort();
    if (JSON.stringify(regions) !== JSON.stringify(["CN", "EU", "US-CA"])) {
      errors.push("profileMappingQuestionnaire jurisdictions must be CN, EU, and US-CA");
    }
    for (const item of value.jurisdictions) {
      requireOneOf(
        item?.applicability,
        ["applicable", "not_applicable", "unknown"],
        `jurisdiction.${item?.region}.applicability`,
        errors
      );
    }
  }
  if (
    !Array.isArray(value.requestedProfileIds) ||
    !value.requestedProfileIds.includes("hiddenshield_v3_image_anchor_v1") ||
    value.requestedProfileIds.length < 2
  ) {
    errors.push("requestedProfileIds must include the V3 anchor and a regulatory Profile");
  }
  if (
    !Array.isArray(value.contentModes) ||
    !value.contentModes.every((mode) => ["ai_generated", "ai_manipulated"].includes(mode))
  ) {
    errors.push("contentModes must use frozen claim types");
  }
  if (
    !Array.isArray(value.explicitLabelSurfaces) ||
    value.explicitLabelSurfaces.length === 0
  ) {
    errors.push("explicitLabelSurfaces must not be empty");
  }
}

function validateResolverLink(value, errors) {
  if (!isRecord(value)) {
    errors.push("resolverLink must be an object");
    return;
  }
  requireExact(value.schemaVersion, "hs-ai-public-resolver-v1", "resolverLink.schemaVersion", errors);
  requireOneOf(
    value.lookupPreference,
    ["watermark_uid", "manifest_id"],
    "resolverLink.lookupPreference",
    errors
  );
  if (value.requiresAuthorization !== false || value.metered !== false) {
    errors.push("resolverLink must remain anonymous and unmetered");
  }
  if (value.legalConclusion !== false) {
    errors.push("resolverLink.legalConclusion must be false");
  }
  for (const field of ["confirmedWording", "notFoundWording"]) {
    requireString(value[field], `resolverLink.${field}`, errors);
  }
}

function validateAcceptanceMatrix(value, errors) {
  if (!isRecord(value) || !Array.isArray(value.scenarios)) {
    errors.push("acceptanceMatrix.scenarios must be an array");
    return;
  }
  const scenarioIds = new Set(value.scenarios.map((scenario) => scenario?.scenarioId));
  if (
    value.scenarios.length !== REQUIRED_ACCEPTANCE_SCENARIOS.length ||
    scenarioIds.size !== REQUIRED_ACCEPTANCE_SCENARIOS.length
  ) {
    errors.push("acceptanceMatrix must contain each mandatory scenario exactly once");
  }
  for (const requiredScenario of REQUIRED_ACCEPTANCE_SCENARIOS) {
    if (!scenarioIds.has(requiredScenario)) {
      errors.push(`acceptanceMatrix missing ${requiredScenario}`);
    }
  }
  for (const scenario of value.scenarios) {
    requireOneOf(
      scenario?.status,
      ["not_run", "passed", "failed", "blocked_external"],
      `acceptanceMatrix.${scenario?.scenarioId}.status`,
      errors
    );
    if (scenario?.mandatory !== true) {
      errors.push(`acceptanceMatrix.${scenario?.scenarioId} must be mandatory`);
    }
    if (
      scenario?.status === "passed" &&
      !IMMUTABLE_EVIDENCE_REF.test(String(scenario?.evidenceRef ?? ""))
    ) {
      errors.push(
        `acceptanceMatrix.${scenario?.scenarioId} passed without immutable evidenceRef`
      );
    }
  }
}

function calculateReadiness(bundle, errors) {
  if (errors.length > 0) return "invalid";
  if (bundle.packageStatus === "suspended") return "suspended";
  const scenarios = bundle.acceptanceMatrix?.scenarios ?? [];
  const allPassed = scenarios.every((scenario) => scenario.status === "passed");
  const endpointsConfigured =
    isConfiguredHttpsEndpoint(bundle.onboarding.sandboxApiBaseUrl) &&
    isConfiguredHttpsEndpoint(bundle.onboarding.resolverBaseUrl);
  const referencesConfigured = [
    bundle.onboarding.partnerId,
    bundle.onboarding.partnerLegalNameRef,
    bundle.onboarding.technicalContactRef,
    bundle.onboarding.securityContactRef,
    bundle.onboarding.credentialSecretRef,
    ...Object.values(bundle.onboarding.approvalReferences ?? {})
  ].every(isConfiguredReference);
  if (
    bundle.packageStatus === "approved_for_sandbox" &&
    allPassed &&
    endpointsConfigured &&
    referencesConfigured
  ) {
    return "sandbox_accepted";
  }
  return "configuration_required";
}

function rejectRawSecrets(value, errors, path = []) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => rejectRawSecrets(item, errors, [...path, String(index)]));
    return;
  }
  if (!isRecord(value)) return;
  for (const [key, item] of Object.entries(value)) {
    const currentPath = [...path, key];
    if (
      typeof item === "string" &&
      /(credential|api.?key|secret|token)$/i.test(key) &&
      !/ref$/i.test(key)
    ) {
      errors.push(`${currentPath.join(".")} must be an external reference, not raw secret material`);
    }
    rejectRawSecrets(item, errors, currentPath);
  }
}

function requireHttpsUrl(value, field) {
  if (typeof value !== "string" || !/^https:\/\//i.test(value)) {
    throw new Error(`${field} must use HTTPS`);
  }
}

function validateHttps(value, field, errors) {
  if (typeof value !== "string" || !/^https:\/\//i.test(value)) {
    errors.push(`${field} must use HTTPS`);
  }
}

function isConfiguredHttpsEndpoint(value) {
  if (typeof value !== "string" || !/^https:\/\//i.test(value)) return false;
  try {
    const hostname = new URL(value).hostname.toLowerCase();
    return (
      hostname !== "localhost" &&
      hostname !== "127.0.0.1" &&
      hostname !== "::1" &&
      !hostname.endsWith(".invalid") &&
      !hostname.endsWith(".test") &&
      !hostname.endsWith(".example") &&
      !["example.com", "example.net", "example.org"].includes(hostname)
    );
  } catch {
    return false;
  }
}

function isConfiguredReference(value) {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    !PLACEHOLDER_REFERENCE.test(value)
  );
}

function requireString(value, field, errors) {
  if (typeof value !== "string" || !value.trim()) {
    errors.push(`${field} is required`);
  }
}

function requirePositiveInteger(value, field, errors) {
  if (!Number.isInteger(value) || value <= 0) {
    errors.push(`${field} must be a positive integer`);
  }
}

function requireExact(actual, expected, field, errors) {
  if (actual !== expected) errors.push(`${field} must equal ${expected}`);
}

function requireOneOf(actual, expected, field, errors) {
  if (!expected.includes(actual)) {
    errors.push(`${field} must be one of ${expected.join(", ")}`);
  }
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
