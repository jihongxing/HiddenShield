import assert from "node:assert/strict";
import { evaluateExternalReadiness } from "./cloud-copyright-c3-external-readiness.mjs";

const blocked = evaluateExternalReadiness({});
assert.equal(blocked.ok, true);
assert.equal(blocked.status, "blocked");
assert.equal(blocked.blockedBy, "external_configuration");
assert.ok(blocked.missing.length >= 7);

const valid = evaluateExternalReadiness({
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND: "jwks",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_ID: "iam://cloud-copyright-prod",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_METADATA_REF: "secret://identity/jwks-ref",
  HIDDENSHIELD_CLOUD_COPYRIGHT_WORKLOAD_IDENTITY_REF: "secret://identity/workload-ref",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_APP_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-app",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_INTERNAL_SERVICE_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-service",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_ROLE_BOOTSTRAP_EVIDENCE_REF: "evidence://iac/cloud-copyright-roles-v1",
});
assert.equal(valid.ok, true);
assert.equal(valid.status, "ready_for_review");
assert.equal(valid.nextGate, "review_0024_sql_and_identity_adapter_fixture");

const literalSecret = evaluateExternalReadiness({
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND: "mtls",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_ID: "iam://cloud-copyright-prod",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_METADATA_REF: "secret://identity/metadata",
  HIDDENSHIELD_CLOUD_COPYRIGHT_WORKLOAD_IDENTITY_REF: "-----BEGIN PRIVATE KEY-----",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_APP_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-app",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_INTERNAL_SERVICE_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-service",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_ROLE_BOOTSTRAP_EVIDENCE_REF: "evidence://iac/cloud-copyright-roles-v1",
});
assert.equal(literalSecret.ok, false);
assert.equal(literalSecret.reason, "secret_reference_invalid");
assert.equal(literalSecret.field, "HIDDENSHIELD_CLOUD_COPYRIGHT_WORKLOAD_IDENTITY_REF");

const placeholder = evaluateExternalReadiness({
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_KIND: "jwks",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_PROVIDER_ID: "iam://cloud-copyright-prod",
  HIDDENSHIELD_CLOUD_COPYRIGHT_IDENTITY_METADATA_REF: "<secret-manager-reference>",
  HIDDENSHIELD_CLOUD_COPYRIGHT_WORKLOAD_IDENTITY_REF: "secret://identity/workload-ref",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_APP_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-app",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_INTERNAL_SERVICE_ROLE_SECRET_REF: "vault://postgres/cloud-copyright-service",
  HIDDENSHIELD_CLOUD_COPYRIGHT_POSTGRES_ROLE_BOOTSTRAP_EVIDENCE_REF: "evidence://iac/cloud-copyright-roles-v1",
});
assert.equal(placeholder.ok, false);
assert.equal(placeholder.reason, "placeholder_reference_rejected");

console.log(JSON.stringify({
  ok: true,
  gate: "cloud-copyright-c3-external-readiness-v1",
  modes: ["blocked", "ready_for_review", "rejected_literal_secret", "rejected_placeholder"],
}));
