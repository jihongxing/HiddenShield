# Profile Mapping Questionnaire

This questionnaire gathers facts. It does not determine legal applicability.

## Jurisdictions

For each region, select `applicable`, `not_applicable` or `unknown` and attach the partner-controlled review reference:

- China (`CN`)
- European Union (`EU`)
- California, United States (`US-CA`)

## Content Flow

- Is the platform generating new images?
- Is it editing existing images?
- Can a single output contain both generated and user-supplied regions?
- Is the original asset retained?
- Is the exported file always PNG?

## Disclosure Surfaces

- platform UI
- exported file
- both
- downstream API metadata

## Technical Profiles

The sandbox image flow must include:

- `hiddenshield_v3_image_anchor_v1`

Optional technical Profiles require explicit selection:

- C2PA-compatible output
- additional metadata
- customer-managed issuer

## Regulatory Profiles

Regulatory Profile IDs must come from the approved HiddenShield Profile catalog. A selected Profile is not active until the corresponding entitlement and review reference exist.

## Data and Deployment

- hosted or private deployment
- CN / EU / US data residency
- media retention requirements
- whether the partner permits storage of content digest
- whether object-store transfer is allowed

## Output

The completed questionnaire must produce:

- requested Profile IDs
- unresolved legal questions
- explicit label surfaces
- issuer mode
- deployment mode
- data residency regions
- legal review reference or `null`
