# HiddenShield Domain Glossary

## Rights Evidence Pack

A case-level collection that organizes copyright facts, disputed objects, collected materials, technical observations, human statements, limitations, and attachment references. It is technical support material, not a legal judgment.

## Attachment

A byte-preserving material referenced by a stable attachment identifier. An attachment has exactly one role and may declare a derivation relationship to another attachment.

## Original

Material supplied as the claimed source work or source evidence. “Original” describes its role in the case, not proof of authorship, ownership, authenticity, or priority.

## Working Copy

A derived copy created for inspection, conversion, redaction, annotation, or analysis. It must identify the attachment from which it was derived and must not replace that attachment.

## Capture

Material recording an external disputed object or observation target, such as a page capture, downloaded file, photograph, or screen recording. Its role does not imply trusted collection.

## External Receipt

A receipt, response, acknowledgment, or other artifact issued outside HiddenShield. Its presence does not mean the issuer, signature, timestamp, or legal effect has been verified.

## Collection Event

An ordered statement that a material was received, captured, derived, hashed, or recorded. Device time and trusted time are distinct event-time statuses.

## Automated Observation

A reproducible technical result produced by a declared method over declared inputs. It is separate from human statements and legal conclusions.

## Human Statement

An attributed assertion made by a claimant, representative, lawyer, or other person. Inclusion in a pack does not turn the assertion into a system-verified fact.

## Basic Record Summary

A free, copyable projection of one local copyright record. It contains core identifiers, hashes, write-after-read verification, registration status, time-proof status, and creator declarations. It is not a formal report, third-party notarization, official registration, identity verification, or legal conclusion.

## Copyright Evidence Technical Report

A record-level paid deliverable that expands a Basic Record Summary with structured process history, protocol and software versions, receipt digests, verification details, limitations, report integrity metadata, and machine-readable attachments. It is technical support material, not a legal judgment.

## Local Copyright Record

A HiddenShield record created from local processing and verification facts. A local record may later receive registry or trusted-time materials, but its existence alone does not prove authorship, ownership, priority, authenticity, or third-party registration.

## Cloud Copyright Workspace

A tenant boundary that groups private copyright record projections, active memberships, change history, and audit history. It does not contain original media, local paths, plaintext creator seeds, or credentials.

## Transport Admission

The permission for a locally validated outbox item to enter a controlled cloud copyright transport flow. It is not permission for a client to call an internal API or database directly.

## Request Scope

The transaction-local account, workspace, device, membership, and request identity established by a verified internal identity receipt. Client-provided values do not establish or override Request Scope.

## Internal API Admission

The server-side decision that a verified internal caller may invoke one scoped operation. It is distinct from public API authorization, SDK licensing, and client credentials.
