# HiddenShield Mobile Rust Bridge

This crate is the mobile-facing Rust boundary for the Flutter app.

Current scope:

- Wrap `watermark-core` behind mobile-friendly structs.
- Keep APIs byte-based so Flutter owns file picking and sandbox access.
- Prove image embed/extract roundtrip before adding platform packaging.

Current exported Rust API:

- `embed_image_for_mobile`
- `extract_image_for_mobile`

Known setup note:

- `flutter_rust_bridge_codegen` is not yet committed into the repo workflow.
- The first bridge milestone is this Rust API crate plus tests.
- The next milestone is adding generated Dart bindings and Android/iOS build wiring.

Validation:

```bash
cargo test
```
