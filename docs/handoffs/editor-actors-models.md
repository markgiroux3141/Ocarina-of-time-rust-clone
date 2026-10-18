# Handoff: the editor's actors, as the game draws them (then their texts)

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-17, after the
actors tab's first build (ADR 0077) and the user's decisions recorded in
[docs/EDITOR-ACTORS.md](../EDITOR-ACTORS.md) ("Decisions").

---

## Prompt

We're continuing the game editor's actors tab (stage E3). Read these first:
`docs/EDITOR-ACTORS.md` (the whole of it, especially "Decisions" and "Models in the views:
capturing what the game draws"), `docs/adr/0077-the-actors-tab.md`, `docs/adr/0066-the-actor-schema.md`
(with its 2026-10-17 amendment), `crates/tools/overworld_editor/README.md`, and
`docs/ROADMAP.md`'s "The game editor track" and "How a phase starts and how a milestone is done".

**The goal:** in the editor, every placed actor shows as the game draws it, standing still, in the
3D view, and the Actors palette shows each actor (and each of its configurations) as a
thumbnail, like the Props palette's. The user chose to **capture what the game's own draw code
emits** rather than use the glTF character export. Edge cases are fine at first; get it mostly
right, and the user will go through the actors with us to tune them.

**Where things are:**
- The editor: `crates/tools/overworld_editor/src/actors.rs` (the catalogue: `oot_actors::PROFILES`,
  the palette's names and groups, the game's placements and variants read from the pack on a
  thread) and `src/app/actor_ui.rs` (the markers in the plan and 3D: `paint_actors`,
  `paint_actors_3d`; the palette: `actor_palette`, `list_row`, `variant_row`; the inspector).
  The props' thumbnails: `src/app/thumbs.rs` over `crates/tools/overworld/src/thumb.rs` (a
  software rasteriser). The 3D view: `src/view3d.rs` (wgpu; `set_level` uploads the level's
  textured meshes).
- What an actor draws: its `ActorImpl::draw(&self, rs, play, view, out: &mut DrawOut)` fills
  `DrawLists` (`crates/engine/eng_gfx/src/submit.rs`): `opa` and `xlu` lists of `DrawCmd { mesh:
  MeshKey { name, segment_textures }, transform: Mat4, bones: Vec<Mat4>, params: DrawParams }`.
  A mesh is an `eng_gfx::DrawList` (`draw.rs`): batches of triangles whose `Vertex { bone, pos,
  normal, color, uv }` positions are in their bone's space, with materials and decoded RGBA
  textures. The game resolves a key in `crates/apps/oot/src/lib.rs` (`impl MeshSource for
  Meshes`): `player/...` through `LinkGfx`, `room/...` from the scene, else the pack
  (`pack.assets.try_get::<DrawList>(name)`). Find how the renderer applies `segment_textures`
  (eyes, mouths) and `DrawParams` (env/prim colours, vertex colours, texels) and apply the ones
  that matter to the look. The bakes: `docs/adr/0012-actor-bakes.md`; skin skeletons (horses):
  `docs/adr/0030-skin-skeletons.md`.
- Spawning an actor headless with its objects: `crates/apps/oot/tests/level.rs` shows the way
  (a `level.json` with `actors`, `oot::load_assets` with `level_in_game`, `oot::new_play`, ticks).
  `oot_import::level::write_scene` loads each actor's objects (its profile's and its schema's
  `ObjectNeed`s). Or build a minimal play state directly; whatever is simplest and faithful.
- The pack: run anything that reads it with `OOT_DATA_DIR=<repo>\out\data26` (the scripts'
  data folder, `scripts\run\_env.bat`); the default `%LOCALAPPDATA%` pack is a stale format.
  The editor is run with `scripts\run\editor.bat` (menu 147).

**A plan to start from** (adjust it after a short survey, and tell the user what changed):
1. *The capture*: a tool command (in `ootx`, or the editor's own background thread if that's
   simpler) that, for each placeable actor and configuration (the catalogue's variants, and
   params 0), spawns it, runs its init and a few frames, takes its `DrawLists` for that frame,
   resolves and flattens them into world triangles relative to the actor (position at the
   origin, facing its yaw 0), and writes them under `out\editor\actors\` with a manifest
   (captured, nothing drawn, failed: and why). Stamp each capture with the pack's id and the
   actor module's modification time, so stale ones are redone. Never commit captures (they come
   from the ROM).
2. *The 3D view*: the editor loads the captures and draws each placed actor's (by its overlay,
   params and setting rotation; else its nearest variant; else the marker) at its place and
   facing, with the level's lighting and fog rules; selection and picking keep working
   (pick by the capture's bounds, as well as the marker).
3. *The thumbnails*: the palette's rows, "New ..." variants and the inspector's "In the game"
   list show the captures through `overworld::thumb`'s rasteriser (or a sibling of it that takes
   a captured mesh), made once and cached like the props'.
4. *Invisible actors*: the ones with nothing to capture that are triggers or spots get their
   area drawn instead (a translucent cylinder or box of the radius their code uses; a short
   hand-written list, from their schema fields or code constants).
5. *The plan*: a small top-down silhouette of the capture under the dot (optional if time is
   short).
6. *The edge cases*, logged in the manifest and shown to the user as a list: actors that draw
   nothing outside their home scene, time, age or flags (try capturing them in their own scene's
   setting: their first placement in the game), ones that need Link near, effects and
   particles (leave out first).

**Then, if there's room in the same session, the texts** (EDITOR-ACTORS.md, "Texts: what an
actor says"): a `texts` list in the schema (id, when, note) checked against each module's text
ids, the inspector's Dialogue section (the lines, rendered as the game's text box), the level's
text overrides read by the game's message lookup for the custom scene, and a text encoder with a
round-trip test over the whole message table. Ask the user before starting it if the models
took most of the session.

**How to work** (the user's standing preferences, from the project's memory):
- Follow the original: the capture uses the game's own draw code, not a reimplementation. Don't
  engineer features the game doesn't have where a simple constraint does.
- Never script long routes or fights for testing; give a hand-test list (keys and places) at the
  end instead. No side-by-side checklists against an emulator.
- The real game's look wins over a reading of the code.
- Milestone checks: `cargo test --release --workspace` (with `OOT_DATA_DIR` as above) and
  `cargo test -p layering` pass, the release build has no warnings; new or changed docs: the
  EDITOR-ACTORS.md "Built" and "Next", an ADR for real decisions (next free: ADR 0078), BACKLOG
  rows for what's left (next free: #90), ROADMAP's E3 row.
- Commit and push only when the user asks.

**Hand back:** what was built, what the capture gets right and wrong (counts from the manifest,
the worst cases by name), screenshots of the editor with actors drawn (the editor can be
screenshotted with PowerShell: bring its window to the front, `CopyFromScreen` its rect), the
checks run, and a hand-test list.
