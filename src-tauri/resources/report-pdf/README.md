# HiddenShield Report PDF Resources

This directory is bundled with the desktop application and is the controlled input set for the Phase R1 Chromium renderer.

- `template.html` is the frozen R1 four-page report template.
- `pretext.js` provides deterministic Chinese line-layout assistance.
- `fonts/NotoSansSC-Controlled.ttf` and `fonts/NotoSerifSC-Controlled.ttf` are application-controlled Noto CJK font files.
- `chromium-worker.mjs` is a JSON-lines worker that keeps one Chromium browser and page warm.

The worker consumes a serialized `FormalReportDocument`. It does not create watermark facts, copyright identifiers, verification results, registry receipts, or trusted-time claims.
