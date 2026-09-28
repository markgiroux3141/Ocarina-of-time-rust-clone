# 0005: Engine name: the `eng_` placeholder prefix

- **Status:** accepted
- **Date:** 2026-09-27

## Context

The engine crates need a name. Picking one isn't urgent, and it shouldn't block the restructure.

## Decision

- Engine crates use the prefix `eng_` (`eng_math`, `eng_gfx`, `eng_gbi`, `eng_render`, `eng_anim`, `eng_collision`, `eng_input`, `eng_app`, and `eng_asset` from milestone 2).
- The layering test enforces the prefix both ways: every crate in `crates/engine/` is `eng_*`, and nothing outside it is.

## Consequences

Renaming later is mechanical:
1. Rename the folders.
2. Replace `eng_` in the `Cargo.toml` files and the `use` paths.
3. Change the prefix in `crates/tools/layering/tests/layers.rs` and this ADR.

No file formats or public data carry the name, and the asset pack's format identifier (ADR 0008) doesn't use it either.
