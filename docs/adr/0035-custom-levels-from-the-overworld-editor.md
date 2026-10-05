# 0035: Custom levels from the overworld editor, played with child Link

- **Status:** accepted, built (2026-10-05)
- **Date:** 2026-10-05
- **Builds on:** [ADR 0002](0002-crate-layout.md) (the layers), [ADR 0008](0008-asset-pack.md)
  (the pack), [ADR 0010](0010-scenes-and-spawning.md) (scenes, rooms and the spikes' view).

## Context

The overworld editor (Kokiri Forest-style levels from an outline, regions, paths and painted
terrain) grew in the Blender MCP repo and moved here, where the game, its assets and its actors
are. Four things needed a decision:

- **Where it lives.** The editor is a tool, but the game has to play what it makes.
- **Its textures.** The Kokiri texture library was cut out of `spot04.glb` by a Blender script.
- **Playing a level.** The game enters scenes from the pack. A custom level isn't in it, and the
  editor wants to play every edit with child Link and the pad, as pd-walk did in the old repo.
- **What it grows into.** A general play mode: actors placed in a level, the hookshot's targets,
  exits between levels.

## Decision

- **Two tools crates.** `crates/tools/overworld` is the builder: engine-neutral, no GPU, a library
  and the `overworld` CLI. `crates/tools/overworld_editor` is the editor (egui, its own wgpu 3D
  view). Tools may depend on everything but apps, so the layering test accepts them.
- **The game reads the export, not the editor's document.** Apps may not depend on tools, so the
  boundary is the builder's engine-neutral export (`level.json`: objects of triangles with
  per-corner UVs, baked vertex colours, a material and a collision surface role per triangle,
  plus `textures/`). `oot_import::level` turns it into the game's records. That's where the
  architecture plan puts custom levels ("custom levels (JSON) → oot_import → mod pack").
- **Textures from the extract.** `overworld kit-textures` (`overworld::kit`) reads `spot04.glb`
  directly (a small glb reader): each render material's PNG, wrap from its sampler, alpha from
  `n64_blend`, opacity from `baseColorFactor`, culling from `n64_cull`. The materials' roles come
  from a table by material index, guarded by the size and wrap of the textures the theme uses,
  so a reordered extraction fails instead of mixing textures up. The output in
  `out/overworld/textures/kokiri` is pixel-identical to the Blender kit, except five textures
  that mirror on both axes, which the Blender script had read as clamp. The editor makes it on
  first start.
- **Playing: the spikes' view, in memory.** `oot_sandbox --level <dir>` (and `Options::level`)
  loads the export with `oot_import::level::load` and plays it through `new_play_at`, the
  sandbox's way into a scene, with no pack changes:
  - **One room**, normal shape, every triangle in one entry: TEXEL0 × SHADE, unlit, the baked
    colours as shade (the way the game's rooms draw), bilinear, the scene's fog
    (`fog_blend`). Opaque and cutout materials go in OPA, translucent ones in XLU at their
    opacity (vertex alpha).
  - **Collision** through `CollisionBuilder` (now indexed by a hash map, which makes big meshes
    fast and gives the same result). Roles map to Kokiri Forest's own surface types:
    - ground and ordinary walls: spot04's 10 (grass footsteps);
    - the edge of the world's cliffs: 14 (no ledge grab);
    - vines: 15 (climbable).

    The bg camera index is cleared, since there are no bg cameras, so the camera stays on
    NORMAL0. Water surfaces aren't collision: each pond is a water box at its surface.
  - **Axes:** (x, z, −y) from the export's x east, y north, z up, a rotation, so windings keep.
    Link starts on the highest floor at the level's middle, facing north (−z), unless the editor
    passes `--at`.
  - **Light:** Kokiri Forest's own environment at the time of day (`load_all_rooms("spot04")`'s
    lights), since the overworld theme is Kokiri's.
  - **Live reload:** the app checks `level.json` twice a second and, when it changes, rebuilds
    the scene and collision with Link where he stands. The editor's ▶ Play button exports, runs
    the sandbox from the repo root (so it finds `oot.toml`'s pad settings and the pack), and
    keeps exporting every rebuild.
- **Limits checked up front:** collision vertex indices are 13 bits, so a level's collision may
  have at most 8192 vertices. Bigger levels are refused with a hint to build at a lower detail
  (the editor's Medium or Low). Coordinates must fit in s16.

## Consequences

- An edit in the editor is playable with child Link and the USB pad a moment later.
  `sketch_hills` at Low detail loads in about 30 ms: 5011 triangles, 4894 collision polys on
  2534 vertices.
- The spikes' view means no actors by id, no exits, no `Play_Init`. A floor with an exit or void
  property would stop on a black screen (`reinit` returns early without assets), so the roles
  carry none. Falling out of the level respawns.
- **Toward a general play mode,** the next steps are:
  - write the level as `scene/`, `room/` and `col/` records into a loose mod pack, layered over
    the game's (`Assets::layer`), with a scene table row and entrances, so `Play_Init` enters it;
  - actors placed in the editor (the room's actor and object lists), and hookshot targets
    (surface word 1 bit 17, once the hookshot is ported);
  - exits between custom levels and the game's scenes;
  - rooms for big levels, which also lifts the 8192-vertex limit per header.
- `oot_import` gains the `image` dependency, for the levels' textures.
