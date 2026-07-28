# Sandbox Acceptance Matrix

All scenarios are mandatory.

| Scenario | Expected evidence |
| --- | --- |
| `admission_success` | admitted license/Profile response |
| `profile_denied_fail_closed` | denial response and zero session |
| `session_ready_to_upload` | session/admission binding |
| `invalid_credential_zero_state_change` | unchanged session projection |
| `png_mark_write_after_read` | marked hash and verified V3 evidence |
| `confirm_single_metering_unit` | one committed `confirmed_marked_image` |
| `confirm_replay_no_duplicate_metering` | same ledger receipt on replay |
| `resolver_preconfirm_not_found` | anonymous 404 before confirm |
| `resolver_postconfirm_anonymous` | anonymous confirmed response |
| `resolver_minimum_public_fields` | exact-key and forbidden-field report |
| `secret_redaction` | logs/package scan with no raw Secret |
| `latency_budget_recorded` | p50, p95, failure rate and sample size |

## Status

- `not_run`: no evidence.
- `passed`: expected result and immutable evidence reference exist.
- `failed`: result violates the contract.
- `blocked_external`: endpoint, credential, partner runtime or other external dependency is unavailable.

`blocked_external` is not acceptance.

## Acceptance Rule

`sandbox_accepted` requires:

- `packageStatus=approved_for_sandbox`
- configured non-placeholder HTTPS endpoints
- every mandatory scenario `passed`
- content-addressed `evidence://sha256/{digest}` reference for every passed scenario
- no raw Secret fields

Production credential issuance remains a separate Gate.
