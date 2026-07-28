# Synthetic Sandbox QA

`synthetic-sandbox-qa` is a local, deterministic rehearsal of the design-partner flow. It exercises the SDK/facade response contract and a minimum Resolver response shape without network, PostgreSQL, `watermark-core`, a partner identity, endpoint, credential, approval or partner runtime.

Its only successful result is:

```text
executionMode=synthetic_non_acceptance
acceptanceStatus=not_real_partner_acceptance
readiness=configuration_required
```

It never produces `sandbox_accepted`, a production credential, an external provider receipt, legal conclusion, SLA evidence, billable usage or a real partner acceptance record.

Run:

```bash
npm run synthetic-sandbox-qa
```

When an external partner is available, replace this rehearsal with a partner-specific bundle and the 12 real acceptance scenarios. Do not reuse synthetic evidence references.
