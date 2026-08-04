const requiredReferences = [
  "HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_ID",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_METADATA_REF",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_WORKLOAD_IDENTITY_REF",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_APP_ROLE_SECRET_REF",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_INTERNAL_SERVICE_ROLE_SECRET_REF",
  "HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_ROLE_BOOTSTRAP_EVIDENCE_REF",
];

const evaluation = evaluateExternalReadiness(process.env);
console.log(JSON.stringify(evaluation));

if (evaluation.status === "rejected") {
  process.exitCode = 1;
}

export function evaluateExternalReadiness(environment) {
  const missing = requiredReferences.filter((name) => !environment[name]?.trim());
  if (missing.length > 0) {
    return {
      ok: true,
      status: "blocked",
      mode: "dry_run",
      blockedBy: "external_configuration",
      missing,
      actions: [
        "does_not_create_0024",
        "does_not_start_identity_adapter",
        "does_not_connect_postgres",
        "does_not_register_internal_api",
      ],
    };
  }

  const providerKind = environment.HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND.trim();
  if (!["jwks", "mtls"].includes(providerKind)) {
    return rejected("identity_provider_kind_invalid");
  }

  const referenceValues = requiredReferences
    .filter((name) => name !== "HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND")
    .map((name) => [name, environment[name].trim()]);
  for (const [name, value] of referenceValues) {
    if (looksLikePlaceholder(value)) {
      return rejected("placeholder_reference_rejected", name);
    }
    if (looksLikeLiteralSecret(value) || !isReference(value)) {
      return rejected("secret_reference_invalid", name);
    }
  }

  return {
    ok: true,
    status: "ready_for_review",
    mode: "dry_run",
    providerKind,
    actions: [
      "does_not_create_0024",
      "does_not_start_identity_adapter",
      "does_not_connect_postgres",
      "does_not_register_internal_api",
    ],
    nextGate: "review_0024_sql_and_identity_adapter_fixture",
  };
}

function rejected(reason, field) {
  return {
    ok: false,
    status: "rejected",
    mode: "dry_run",
    reason,
    ...(field ? { field } : {}),
  };
}

function isReference(value) {
  return /^(?:secret|vault|sm|keyvault|evidence|iam):\/\/[A-Za-z0-9._/@:=+-]+$/.test(value);
}

function looksLikeLiteralSecret(value) {
  return (
    value.includes("-----BEGIN") ||
    value.startsWith("eyJ") ||
    value.includes("://") && /:.*@/.test(value) ||
    /(?:password|token|private[_-]?key|client[_-]?secret)=/i.test(value) ||
    value.length > 256
  );
}

function looksLikePlaceholder(value) {
  return /<[^>]+>|(?:example|placeholder|todo|tbd)/i.test(value);
}
