# 0012: Actor bakes: actors declare their meshes, the importer bakes them

- **Status:** accepted, built in GAME-02 milestone 1
- **Date:** 2026-09-28

## Context

ADR 0006 chose baked meshes plus a draw API, and ADR 0009 bakes what runtime state changes. Until now the pack held each object's display lists and skeletons as they're listed in the decomp XMLs.

Actors don't draw an object's lists as listed. For example:
- `En_Kanban` calls the post's list, then `gSignRectangularDL` from `gameplay_keep`;
- `En_Ko` draws a skeleton with a head limb from another object, the tunic and boots colours on segments 8 and 9, the eyes' texture on segment 0x0A, and a render mode on 0x0C that depends on its alpha;
- the Z-target reticle is `gLockOnReticleTriangleDL` after `Gfx_SetupDL_57`, not `_25`.

What an actor draws depends on its C draw function. So the content crate knows it, and the importer can't guess it. The runtime can't run display lists either (ADR 0006).

## Decision

- **Actors declare `MeshBake` records** (`oot_game::pack::MeshBake`), collected by `oot_actors::bakes()`, which includes `oot_game::target::bakes()` for `z_actor.c`'s reticle. Each record has:
  - a name (baked to `bake/<name>`);
  - the object bound on segment 6, with the keeps on 4 and 5;
  - its own segments, each one of:
    - `Texture{file, symbol}`: a texture bound to the segment;
    - `DynamicColor{env, prim}`: a colour set per draw;
    - `Commands(...)`: a display list written out as words, such as a render mode, or an `sSetupDL` entry, which is data in `code` and not in any XML;
  - a prelude: the segments to call after `Gfx_SetupDL_25Opa`, before the body;
  - a body: display lists in order, or a skeleton with limb overrides (a limb's list from another file).
- **The importer bakes them** (`ObjectSegments::bake_mesh`) with the same interpreter as the rest of the pack. A bake that leaves an address unresolved or meets an unknown opcode fails the import.
- **Per-draw colours use dynamic segments.** A `DynamicColor` segment becomes a tiny list of `G_SETENVCOLOR` / `G_SETPRIMCOLOR` commands marked dynamic. The draw passes `SegmentValues` in `DrawParams`, and the renderer writes them into that mesh instance's materials.
- **A state that changes the geometry or the render mode is a separate bake**, picked at draw time: `En_Ko` has one per head, eye and pass (opaque or translucent), 14 in all.
- The pack's format version went to 3 for the `bake/` records, and to 4 when `CameraData` changed.

## Consequences

- Porting an actor's draw means writing down its `Draw` function's lists and segments as data next to the port, citing the C, and choosing which state is per-draw (colours) and which is a separate bake.
- The variants multiply: every combination of a geometry or render-mode state needs its own mesh. That's fine for the NPCs so far. An actor with many independent toggles would need the runtime display-list path ADR 0006 keeps open.
- A setup list other than `SETUPDL_25` is written out as its commands, with the `gbi.h` macros it comes from in a comment (`oot_game::target::bakes`).
