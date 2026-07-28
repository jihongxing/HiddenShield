# HiddenShield AI Transparency Design Partner Kit

Controlled onboarding package for an AI image platform design partner.

## Boundary

- Sandbox only.
- No production credential is included or generated.
- No legal opinion, compliance guarantee, SLA, public deployment or production issuer trust is claimed.
- Partner identity, endpoint and credential values must be injected through external references.
- The current SDK uses the production-equivalent admission contract inside an isolated sandbox deployment. Sandbox IDs and credentials must never be promoted to production.

## Contents

- `ONBOARDING.md`: onboarding sequence and ownership.
- `PROFILE_MAPPING.md`: CN / EU / US-CA Profile questionnaire.
- `examples/server-mark-and-resolve.mjs`: server-side SDK/API and anonymous Resolver flow.
- `templates/design-partner-sandbox-kit.template.json`: partner-specific bundle template.
- `schemas/design-partner-sandbox-kit-v1.schema.json`: frozen bundle schema.
- `ACCEPTANCE_MATRIX.md`: mandatory acceptance evidence.
- `bin/preflight.mjs`: fail-closed package validation.

## Start

```bash
cp templates/design-partner-sandbox-kit.template.json partner-kit.json
node bin/preflight.mjs partner-kit.json
```

The untouched template must return:

```text
valid=true
readiness=configuration_required
```

It must not return `sandbox_accepted` until partner identity, approval, endpoint and Secret references are configured and every mandatory acceptance scenario has content-addressed evidence.

`npm run ai-transparency:ci` is the required repository Gate for SDK/partner-kit contract regression, including synthetic Sandbox QA.

## External Inputs

The following values are intentionally not included:

- partner legal identity and contacts
- sandbox API and Resolver endpoints
- sandbox credential Secret
- approved Profile mapping and legal review reference
- partner output volume and latency evidence
- data residency and private deployment approval

## Release

This package is `private: true`. It is a controlled design-partner artifact, not a published npm SDK or a production credential issuance channel.
