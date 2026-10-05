# 0002: Crate layout, and a test that enforces it

- **Status:** accepted
- **Date:** 2026-09-27

## Context

The spikes grew as 8 crates with blurred boundaries:
- `oot_core` mixed N64 decoding, ROM access, C parsing and Player's draw rules.
- `oot_game` mixed engine-level code (collision, maths, input) with Zelda logic.
- `oot_pad` depended on `oot_game`: the wrong way round.
- The renderer was one 790-line file.

The plan (ARCHITECTURE-PLAN.md §3.1) separates an engine that knows nothing about Zelda from the game built on it, and mirrors the decomp: `src/code` is game framework, `src/overlays` is content.

## Decision

Crates live in one folder per layer under `crates/`:

| Layer | Crates | May depend on |
|---|---|---|
| `engine/` | `eng_math`, `eng_gfx`, `eng_gbi`, `eng_render`, `eng_anim`, `eng_collision`, `eng_input`, `eng_app` | engine only |
| `import/` | `oot_import` | engine |
| `game/` | `oot_game` (framework) | engine |
| `game/` | `oot_actors` (content) | engine, `oot_game` |
| `apps/` | `oot` (the game; library + binary), `oot_sandbox` | everything except tools |
| `tools/` | `ootx`, `oot_viewer`, `oot_extract`, `ootpad`, `layering`, `overworld`, `overworld_editor` (ADR 0035) | everything except apps |

On top of the layers:
- GPU and windowing crates (wgpu, eframe, egui, winit, pollster) are allowed only in `eng_render` and `eng_app` among the engine, import and game crates.
- `gilrs` is allowed only in `eng_input`.
- Engine crates, and only engine crates, are named `eng_*`.

`crates/tools/layering/tests/layers.rs` checks all of this on every crate's normal, dev and build dependencies (`cargo metadata`). It runs as part of `cargo test --workspace`. It fails on a violation, and also on a listed exception that is no longer used, so each rule only ever tightens.

### Where things went, and why

| Code | Crate | Why |
|---|---|---|
| Binary angles, `sins`/`coss`, `Math_*StepTo*`, `guPerspective`, the update rate | `eng_math` | libultra and `z_lib.c` maths, nothing Zelda-specific. The table *loader* (reading `sintable` from C) went to `oot_import::tables`, so the engine never reads C. |
| Draw-list types: `Material`, `Batch`, `DrawList`, `SegmentValues`, the combiner, `WrapMode`, `DecodedImage` | `eng_gfx` | The engine's mesh format, shared by the interpreter, the renderer and (next) the asset pack. No GPU code, so the game can build draw lists without linking wgpu. |
| F3DEX2 interpreter, TMEM decoding, `model::build_draw_list` | `eng_gbi` | The plan puts the interpreter in the importer *and* at runtime (for procedural display lists, ADR 0006), so it's an engine crate, separate from the types. It re-exports the `eng_gfx` types it produces. |
| wgpu renderer | `eng_render` | Split into `device`, `view`, `pipelines`, `materials`, `model`, `passes`. It depends on `eng_gfx` only, not on the interpreter. |
| Skeleton, joint table and animation types, posing | `eng_anim` | |
| `z_bgcheck.c` port: static mesh, DynaPoly, water boxes | `eng_collision` | `StaticCollision` is renamed `CollisionContext`, the game's name for it. It returns raw surface data words. The two bits `z_bgcheck.c` itself reads (soft floor, bit 27) stay inside. |
| Pad state (`PadMgr`, `padutils`) and devices (gilrs, keyboard) | `eng_input` | This fixes the inverted `oot_pad` → `oot_game` dependency. The config file name is passed in (`PadConfig::find(dir, "oot.toml")`). |
| Window shell, offscreen target as an egui image, keyboard layout, PNG output | `eng_app` | Shared by `oot`, `oot_sandbox` and `oot_viewer`, which each had a copy. |
| z64 binary decoders (skeletons, standard and Link animations, collision headers) | `oot_import::z64` | These are the Zelda engine's formats, so not the engine's. They are extension traits (`ParseSkeleton`, `CollisionCodec`, …), so `Skeleton::parse(..)` still reads the same with the trait in scope. |
| ROM, Yaz0, XML symbols, `csrc`, `drawcfg`, scene and room decoding, Player's draw rules, the synthetic test object | `oot_import` | Everything that reads the ROM or the decomp. |
| Actor base, SkelAnime for Link, camera, target context, environment, foot IK, game data, test course | `oot_game` | `z_skelanime.c` is game framework: its Link functions take Player's animation data and move the actor, so it isn't generic engine animation. |
| Surface type meaning (`SurfaceType_Get*`, `WALL_FLAG_*`) | `oot_game::surface` | A trait on `CollisionContext`. |
| `Room_DrawCullable`'s ordering (`cullable_order`) | `oot_game::room` | Per-frame game logic, not decoding. |
| Player, `Bg_Ydan_Hasi` | `oot_actors` | Overlays. |
| `World` (the spikes' fixed play state) | `oot_actors::world` | Transitional: it names Player and `Bg_Ydan_Hasi` directly, so it can't live in `oot_game`. Milestone 3 replaces it with `PlayState` in `oot_game`. |
| Play client: assets, scene drawing, the interactive window | `oot` (library) | Shared by the `oot` binary (the game: Kokiri Forest, child Link) and `oot_sandbox` (all of `oot_play`'s flags, scripts, sheets and traces). |

### Transitional exceptions

Until the asset pack (milestone 2), the game reads its tables from the decomp's C and some assets straight from the ROM at startup, so two edges break the rules and are listed in the check:
- `oot_game` → `oot_import`
- `oot_actors` → `oot_import`

Milestone 2 removes both, and the check then enforces that the runtime never reaches the ROM or the decomp.

### Amendment (milestone 2): the importer builds the game's records

With the asset pack, the edges turned round:
- The pack's records are the game's own types: `GameData`, `SceneData`, `LinkVariant` and so on.
- `oot_import` builds them, so **import may depend on the engine, `oot_game` and `oot_actors`**, and nothing in the game depends on import.
- The transitional exceptions are gone, and the list in the check is empty.
- The loaders that parsed C moved from the game into `oot_import::tables`, as extension traits:
  - `GameData::load(p)`;
  - `CameraData`, `FootIkData`, `EnvTables`, `PlayerRules` (with `LoadPlayerRules`).
- The record types moved into the game: the scene header types (`oot_game::scene`) and Player's draw rules (`oot_game::player_lib`). `oot_import` re-exports them.

| Layer | May depend on (as built) |
|---|---|
| engine | engine only |
| `oot_game` | engine |
| `oot_actors` | engine, `oot_game` |
| import | engine, `oot_game`, `oot_actors` |
| apps | everything except tools. The `oot` client depends on import only for `oot import` and the first-launch import. |
| tools | everything except apps |

## Consequences

- Engine code can't name Zelda. `eng_render` and `eng_collision` can be reused or tested alone.
- The game and content crates don't link wgpu, eframe or gilrs.
- Paths changed throughout: `oot_core::gbi` → `eng_gbi::gbi`, `oot_game::bgcheck` → `eng_collision::bgcheck`, `oot_game::input` → `eng_input::pad`, `oot_pad` → `eng_input::device`, `oot_render` → `eng_render`, `oot_game::player` → `oot_actors::player`.
- `oot_play` is now `oot_sandbox` (same flags), and the viewer, ootx and the extractor are under `crates/tools/`.
