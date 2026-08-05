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
## Commercial Language

**基础状态**:
The unpaid desktop product state. It includes the local single-file image/audio workflow and local copyright vault, but does not imply batch, cloud, team, API, or report-export entitlement.
_Avoid_: Free 套餐, 免费版套餐

**图片 / 音频年费**:
The only current user-visible paid desktop plan. Its time-bounded annual entitlement grants image/audio batch processing and metadata-only cloud sync, but does not itself imply formal-report export, team access, API access, or video processing.
_Avoid_: Creator 套餐, Creator 订阅

**年度授权**:
The trial, active, grace, expiry, renewal, and revocation lifecycle of the 图片 / 音频年费 plan. It is an entitlement term, not a separate user-visible plan.

**单份报告商品**:
A record- or case-scoped purchase that grants one named report deliverable without changing the user's desktop plan or other capability entitlements.
_Avoid_: 报告套餐, 报告订阅

**能力权益**:
An explicit server- or license-issued authorization for one capability, such as batch processing, cloud sync, report export, team workspace, API access, or cloud video units. The current annual plan grants batch processing and cloud sync; other capabilities require their own product or contract authorization.
_Avoid_: 套餐等级

**组织产品线**:
A future contract-backed product for teams or enterprises, such as a team workspace or enterprise integration. It is not a third or fourth desktop subscription tier.
_Avoid_: Studio 套餐, Enterprise 套餐

**Legacy Plan Code**:
An existing persisted or wire-level value such as `free`, `creator`, `studio`, or `enterprise`. It is compatibility data only and must not be introduced as new user-facing copy or used alone for authorization.

## Composite Media Language

**苹果实况照片（Live Photo）**:
A single user-visible media asset composed of a still-photo resource, a paired short-video resource that may contain audio, and association metadata such as the asset identifier and key-photo selection. It is not a standalone image encoding and must not be treated as an ordinary HEIC, JPEG, PNG, or WebP file.

**实况照片复合保护**:
A future HiddenShield capability that protects and verifies the still-photo and paired-video resources as one logical asset, preserves their association metadata and Live Photo behavior, and produces one coherent verification result. Protecting only the exported still frame is not 实况照片复合保护.
_Avoid_: 支持 Live Photo（when only a still frame is processed）, 实况图片格式
