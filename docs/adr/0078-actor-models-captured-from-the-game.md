# 0078: The actors in the editor's views: captured from the game's own draws, drawn by the game's renderer

- **Status:** accepted, built (2026-10-19)
- **Date:** 2026-10-19
- **Builds on:** [ADR 0077](0077-the-actors-tab.md) (the actors tab), [ADR 0066](0066-the-actor-schema.md)
  (the schema and its object needs), [ADR 0006](0006-rendering-model.md) (draw submission),
  [ADR 0002](0002-crate-layout.md) (the layers), docs/EDITOR-ACTORS.md ("Decisions", 1).

## Context

The actors tab's first build drew every placed actor as a marker in its group's colour. On
2026-10-17 the user chose how they should look: by capturing what each actor's own draw code
emits, not from the glTF character export (ADR 0068), so that every kind of actor (pots, chests,
switches, set pieces, enemies, characters) shows with the faces, heads and colours the game binds
for its configuration, and every actor ported later gets its look for nothing. Edge cases could
wait: the user goes through the actors to tune them.

## Decision

1. **The capture is the game's** (`oot capture-actors`, `crates/apps/oot/src/capture.rs`), not a
   tool's: the layering keeps tools from linking the `oot` client (ADR 0002), and the client
   already resolves mesh keys as it plays (`Meshes`: Link's, the pack's, the rooms'). For each
   placeable actor in params 0 and in each configuration the game's scenes place it with (the
   most placed 24, as the palette offers them), and each configuration a level asks for besides
   (`extra.json`): a fresh play state in a flat custom level entered by `Play_Init`, the actor's
   objects loaded (the actor table's, by its row, and its schema's `ObjectNeed`s), Link 260 in front
   facing it, the actor spawned at the origin and run up to 10 frames (the first frame it draws
   after that, up to 40). Then its own `draw` and its children's are called once more, and the
   commands resolved and flattened into one mesh in the actor's space.
2. **A capture is one still `eng_gfx::DrawList`** (`eng_gfx::flatten`): each draw's mesh posed by
   its transform and bones exactly as `eng_render` poses it, with what the draw sets for itself
   baked in (its dynamic segments' env, prim and tile scroll, its vertex colours, its own texels).
   A draw's point lights and fog belong to the frame: left out. Stored per actor in
   `out\editor\actors\<overlay>.bin` (the pack's record encoding, compressed), with a manifest of
   every configuration tried: captured (and the setting it was caught in), drew nothing, or failed
   (and why), its frame, triangles and size. Derived from the ROM: never committed.
3. **Where an actor draws nothing as first placed**, it's tried again as if in that placement's
   scene (`play->sceneId`: 46 actors branch on it), then as the other age, then at the other time
   of day. Its own draws must be there; its children's come with them (a Kokiri child's fairy),
   except for spawners whose children are all of their look (`Obj_Mure`, `Obj_Mure2`).
4. **Stale captures are made again**: each file carries its stamp (the pack's id, the game
   binary's and the actor module's last change, the format). The editor runs the capture in the
   background when it starts and when the level has a configuration the manifest hasn't tried; a
   full run takes about 5 s, an up-to-date one about 1 s.
5. **The editor draws them with the game's renderer** (`eng_render`), in the same pass and depth
   buffer as the level (`Renderer::set_view` and `draw_models`, new): the colour combiner, the
   game's lights (the capture setting's) and the level's fog. Each placed actor's capture is posed
   at its place, turned by its y rotation as the game turns it; one without its own capture shows
   the nearest captured configuration (the same params, else the fewest differing bits). The
   palette's rows, its "New ..." list, the inspector's "In the game" list and its header show
   thumbnails drawn the same way on the GPU (not `overworld::thumb`'s rasteriser: it can't
   combine). The plan shows each capture's outline from above.
6. **Invisible actors show their area** (`actors::area`): a translucent cylinder or box from their
   settings or their code's constants (Navi's spots, song and talk spots, the weather's range, an
   enemy spawner's), a short hand-written list.
7. **Picking** goes by the capture's box (models before areas, never from inside an area), the
   marker for the rest.

## Consequences

- The actors look as the game draws them in the editor, for every ported actor, with no
  per-actor editor code; 942 of the 1314 configurations tried (139 of the 175 placeable actors)
  are captured on the first run.
- What isn't caught is listed in the manifest with why: cutscene-only actors (`Demo_*`,
  `En_Viewer`), projectiles and effects only other actors spawn, characters that need flags, an
  item or a path (BACKLOG #90 to #93).
- `SceneTable::object_id` takes the last object of a name: the game's object table has empty
  duplicates of `object_md` and `object_ny` before the real ones (found by the capture: Mido's
  object loaded empty).
- Shadows aren't captured (they're drawn by the actor system on the ground under the actor, not by
  its draw code).
