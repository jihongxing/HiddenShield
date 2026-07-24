# Project Agent Instructions

Scope: this repository.

Follow the repo-specific instructions already provided in the thread and keep changes aligned with the existing desktop/mobile architecture.

## Hard Constraint

Every completed task must end with a concrete recommended next step.

The recommendation must be:

- specific
- actionable
- tied to the current project state

Do not end a task response with a generic "if you want" offer. Always name the next best step.

Commercialization work must follow `docs/商业化落地Roadmap.md`.

For any task that touches subscription, entitlement, batch processing, cloud sync monetization, cloud video processing, team workspace, payment, pricing, or commercial UI:

- Check `docs/商业化落地Roadmap.md` before changing code or docs.
- Keep the implementation aligned with the roadmap phase and acceptance criteria.
- Update `docs/商业化落地Roadmap.md` after completing the task.
- Record completed work, status changes, validation results, risks, and the next roadmap task.
- Do not implement commercial behavior that conflicts with the roadmap without first updating the roadmap.

Desktop/mobile consistency work must follow `docs/双端能力一致性Roadmap.md`.

For any task that touches desktop/mobile parity, image or audio capability alignment, vault fields, verification wording, report fields, sync behavior, L2 video notary display, mobile/desktop UX wording, or cross-platform QA:

- Check `docs/双端能力一致性Roadmap.md` before changing code or docs.
- Check `docs/共享水印核心与跨端互验推进计划.md` before changing any image, audio, video, watermark payload, copyright ID, rewrite/preflight, verification, or protected-copy export behavior.
- Keep desktop and mobile behavior aligned unless an explicit platform limitation is documented.
- Update `docs/双端能力一致性Roadmap.md` after completing the task.
- Record completed work, status changes, validation results, risks, and the next dual-end consistency task.
- Do not introduce a desktop-only or mobile-only product promise without documenting the reason and fallback.

Shared watermark core and cross-end verification are hard constraints.

For any formal HiddenShield watermark capability:

- `watermark-core` is the single source of truth for all current and future blind-watermark write, read, verification, payload encoding, copyright ID generation, rewrite detection, and write-after-read verification algorithms.
- Image, audio, video-audio-track, and future video-visual blind-watermark algorithms must live in `watermark-core`. Desktop, native mobile, backend, cloud jobs, and UI code may only call or wrap `watermark-core`; they must not implement their own blind-watermark embedding, extraction, detection, payload encoding, copyright ID generation, or rewrite rules.
- If a future cloud service is needed for video visual watermarking, the service must be an execution wrapper around `watermark-core` or a deployable artifact built from it. The cloud service is allowed to handle scheduling, entitlement, key custody, strategy delivery, and self-check orchestration, but not to become a second algorithm source.
- A protected copy written by any formal endpoint must be readable by every other formal endpoint that supports that media type. This is a release gate, not a best-effort target.
- Cross-end fixtures and tests must cover desktop-written/mobile-read and mobile-written/desktop-read for images and audio before a related task is considered complete.
- Future video write/verify work must use `watermark-core` for blind-watermark algorithms and must stay aligned with `docs/商业化落地Roadmap.md`.
- Web preview code must not be treated as formal watermark capability unless it calls the same shared core, for example through WASM. Preview-only markers, mock IDs, or UI demo artifacts must not enter formal vault records, reports, sync payloads, or cross-end QA evidence.
- If a platform limitation prevents full parity, document the limitation, fallback, and user-visible wording in both `docs/双端能力一致性Roadmap.md` and `docs/共享水印核心与跨端互验推进计划.md` before shipping the change.

For any task that changes `watermark-core`, its public API, payload, fixtures, benchmarks, gates, or algorithm behavior:

- Update `docs/watermark-core能力说明.md` in the same task.
- Record the capability change, current performance snapshot, external exposure boundary, and any new limitation or rollback path.
- If the change affects user-facing capability or boundary wording, also follow `docs/当前真实能力边界说明.md`.

Current capability boundary statements must follow `docs/当前真实能力边界说明.md`.

For any task that states, changes, sells, documents, or implies what HiddenShield can currently do:

- Check `docs/当前真实能力边界说明.md` before changing code, docs, UI copy, help text, sales wording, subscription wording, roadmap language, or reports.
- Classify the capability as `可对用户承诺`, `只能内部测试`, or `明确不能承诺`.
- Update `docs/当前真实能力边界说明.md` whenever the current capability boundary changes or when new boundary wording is introduced.
- Do not present L3 video visual watermark staged tests as a user-facing, commercial, cloud, billing, or SLA capability unless this document is updated first and the required release gates are met.

## Working Rules

- Prefer minimal, local changes.
- Do not revert unrelated user changes.
- Keep mobile and desktop behavior aligned unless explicitly told otherwise.
- When planning app work, preserve shared terminology across platforms.
- For commercial features, preserve the shared Free / Creator / Studio / Enterprise terminology across desktop, mobile, backend, and docs.
