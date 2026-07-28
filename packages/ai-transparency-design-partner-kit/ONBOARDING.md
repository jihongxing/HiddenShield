# Sandbox Onboarding

## Gate 1 — Partner Identity

Record only external references:

- partner ID
- legal name reference
- technical contact IAM reference
- security contact IAM reference

Do not copy personal contact details into the bundle.

## Gate 2 — Use Case

Freeze:

- AI generated and/or AI manipulated image flow
- PNG output boundary
- expected monthly confirmed images
- peak requests per second
- mark-and-confirm latency budget
- hosted or private deployment
- requested data residency

## Gate 3 — Profile Mapping

Complete `PROFILE_MAPPING.md` and the JSON questionnaire.

`unknown` is an accepted questionnaire answer. It is not an approved Profile decision.

## Gate 4 — External Configuration

Inject:

- HTTPS sandbox API endpoint
- HTTPS sandbox Resolver endpoint
- `secret://` credential reference

The package must never contain the credential value.

## Gate 5 — Technical Preflight

Run:

```bash
node bin/preflight.mjs partner-kit.json
```

Structural errors block all testing. `configuration_required` means the bundle is valid but not accepted.

## Gate 6 — Acceptance

Run every mandatory scenario in `ACCEPTANCE_MATRIX.md`.

Each passed scenario requires a content-addressed `evidence://sha256/{digest}` reference. External environment blockers use `blocked_external`, never `passed`.

## Gate 7 — Written Sign-off

Sandbox acceptance requires:

- partner technical sign-off
- partner security sign-off
- HiddenShield engineering review
- HiddenShield commercial owner review
- explicit acknowledgement of no production credential, no SLA and no legal opinion

Record each sign-off as an external approval reference in `onboarding.approvalReferences`. Placeholder approval references keep readiness at `configuration_required`.

Sandbox acceptance does not authorize production issuance.
