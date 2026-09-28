# 0006: Rendering model: hybrid

- **Status:** accepted; the draw submission was built in GAME-01 milestone 3
- **Date:** 2026-09-27

## Context

Actor draw code in the decomp builds display lists at runtime. The options (ARCHITECTURE-PLAN.md §3.4):
- **A. Interpret display lists every frame** (the Ship of Harkinian / Fast3D approach). Draw functions port almost verbatim. It costs per-frame interpretation, and custom non-N64 content is awkward.
- **B. Baked meshes plus a draw API.** Display lists are interpreted once, at import time. Draw functions become "draw mesh X with these parameters". This is faster and suits custom levels, but every draw function needs translating, and procedural geometry needs engine features.
- **C. Hybrid:** B for everything static, and a runtime command buffer through the same interpreter for the procedural cases (effects, the skybox, a few actors).

The spikes already work this way:
- Link, rooms and the platform are interpreted once into draw lists.
- What changes per frame is carried as dynamic segment values (`SegmentValues`, `DynTile`, `env_dyn`/`prim_dyn`) that the renderer applies as material parameters.
- Scene draw configs run every frame, and only their tile sizes and colours are read back.

## Decision

- **Hybrid (C).**
- Static display lists are interpreted at import time into `eng_gfx::DrawList`s stored in the pack.
- Draw code submits "mesh + matrix or joint palette + per-draw material parameters (segment values, prim/env, tile scroll)" to OPA and XLU lists, which the renderer draws in order, as `POLY_OPA_DISP` / `POLY_XLU_DISP` do.
- For procedural display lists, the runtime builds a small command buffer and runs it through `eng_gbi`'s interpreter. That's why the interpreter is an engine crate and not import-only (ADR 0002).

## Consequences

- Milestone 1 already separates the types (`eng_gfx`) from the interpreter (`eng_gbi`) and the renderer (`eng_render`). The renderer depends only on the types.
- Milestone 3 builds the submission lists. The spikes' `GpuModel::set_segment_values` becomes a per-draw parameter instead of a per-model mutation.
- Draw configs that swap texture *pointers* per frame (lava, waterfalls in some scenes) need either a rebuild through the runtime interpreter or texture-slot parameters. That's decided when the first such scene is ported.

## As built (milestone 3)

- **Actors submit draws.** `ActorImpl::draw` gets the actor's blended render state and pushes `eng_gfx::DrawCmd`s into the frame's `DrawLists`, the `POLY_OPA_DISP` / `POLY_XLU_DISP` pair. Rooms submit their entries the same way (`Room_Draw` / `Room_DrawCullable` order).
- **A draw command** is:
  - a `MeshKey`: an asset-pack record name, or one of the app's built-in meshes, plus the textures the draw binds to segments (Link's eyes on 8 and mouth on 9);
  - a model matrix;
  - a bone palette for skinned meshes;
  - `DrawParams`: this frame's dynamic segment values (texture scroll, env and prim colours).
- **The renderer** (`eng_render::MeshCache`, `Renderer::render_lists`):
  - uploads a key's mesh the first time it's drawn, from the app's `MeshSource`;
  - gives a key drawn several times in a frame one instance per use;
  - re-skins an instance only when its pose changes, and writes the draw's segment values into its dynamic materials;
  - draws the OPA list, then the XLU list.
- **The shadow moved to the XLU list.** On the test course, the spike had drawn Link's circle shadow before Link. It's now in the XLU list after every opaque draw, as `Play_Draw` does: a few pixels of the course renders changed, where the translucent shadow now blends over the soles of Link's boots (`golden/README.md`).
- **No runtime display lists yet.** Nothing so far builds a display list at runtime: every draw is a baked mesh.
