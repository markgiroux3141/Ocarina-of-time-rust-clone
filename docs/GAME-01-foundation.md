# GAME-01: the foundation

**Goal:** turn the four spikes into the codebase the full game grows from, following [ARCHITECTURE-PLAN.md](ARCHITECTURE-PLAN.md): an engine that knows nothing about Zelda, the game and its actors on top, an asset pack so the runtime never touches the ROM or the decomp, and a real actor system with scenes, rooms and spawning.

| # | Milestone | Plan phase | Status |
|---|---|---|---|
| 1 | Restructure: the new crate layout, no behaviour change | Phase 0 | done |
| 2 | The asset pack | Phase 1 | done |
| 3 | PlayState and the actor system | Phase 2, first half | done |
| 4 | Scenes, rooms and spawning | Phase 2, second half | done |

Decisions are recorded in [docs/adr/](adr/README.md). Four were confirmed with the user before anything moved:
- the spike state is committed and tagged (ADR 0001);
- the ROM version (ADR 0003);
- the decomp commit (ADR 0004);
- the engine name (ADR 0005).

## Milestone 1: the restructure

**Answer:** done. The spikes' 28k lines now sit in 18 crates across five layers, with a test that enforces the layering. All 87 spike tests pass unchanged, and the spike renders and traces are bit-identical: 78 of 78 hashes, from 54 headless cases.

### The layout

```
crates/
  engine/   eng_math  eng_gfx  eng_gbi  eng_render  eng_anim  eng_collision  eng_input  eng_app
  import/   oot_import
  game/     oot_game (framework)   oot_actors (content)
  apps/     oot (game: library + binary)   oot_sandbox
  tools/    ootx  oot_viewer  oot_extract  ootpad  layering
```

| Crate | Lines | What it holds | From |
|---|---|---|---|
| `eng_math` | 260 | binary angles, `sins`/`coss`/`Math_Atan2S`, `Math_*StepTo*`, `guPerspective`, the 20 Hz rate | `oot_game::math`, `oot_core::room::gu_perspective` |
| `eng_gfx` | 505 | draw-list types (`Material`, `Batch`, `DrawList`, `SegmentValues`, `DynTile`), the combiner, texture formats | split out of `oot_core::{gbi, texture}`, plus `combiner` |
| `eng_gbi` | 965 | the F3DEX2 interpreter, TMEM decoding, `build_draw_list` for skeletons | the rest of `oot_core::{gbi, texture}`, `model` |
| `eng_render` | 861 | the wgpu renderer, now `device` / `view` / `pipelines` / `materials` / `model` / `passes` | `oot_render` (one file) |
| `eng_anim` | 190 | skeleton, joint table and animation types, posing | `oot_core::{skeleton, anim}` without the decoders |
| `eng_collision` | 1419 | `z_bgcheck.c`: `CollisionContext` (was `StaticCollision`), DynaPoly, water boxes | `oot_game::{bgcheck, dyna}`, `oot_core::collision` without the codec |
| `eng_input` | 647 | pad state (`PadMgr`, `padutils`) and devices | `oot_game::input` + `oot_pad` |
| `eng_app` | 105 | window shell, offscreen target as an egui image, keyboard layout, PNG output | copies in `oot_play` and `oot_viewer` |
| `oot_import` | 4718 | ROM, Yaz0, XML symbols, z64 decoders (`z64`), C tables (`csrc`, `tables`), `drawcfg`, scenes and rooms, Player's draw rules, the synthetic Tock | `oot_core` |
| `oot_game` | 3385 | actor base, SkelAnime, camera, target context, environment, foot IK, `surface`, `room`, game data, test course | `oot_game` |
| `oot_actors` | 5655 | Player, `Bg_Ydan_Hasi`, the transitional `World`; 9 of the 10 test suites | `oot_game` |
| `oot` | 1032 | the play client library, and the game binary (Kokiri Forest, child Link) | `oot_play` |
| `oot_sandbox` | 364 | every `oot_play` flag and script, headless sheets, traces and screenshots | `oot_play` |
| tools | | `ootx`, `oot_viewer`, `oot_extract`, `ootpad` (was a bin of `oot_pad`), `layering` | |

[ADR 0002](adr/0002-crate-layout.md) gives the reason for each placement. The ones that weren't obvious:
- **`skelanime` stayed in the game.** `z_skelanime.c`'s Link functions take Player's animation data and move the actor, so it isn't engine code. The plan had it in `eng_anim`, which holds only the data types.
- **The z64 binary decoders went to `oot_import::z64`,** as extension traits. `Skeleton::parse(..)` and `CollisionHeader::parse(..)` keep their spelling with the trait imported. The engine has the types and algorithms, not the Zelda file formats.
- **The interpreter is its own engine crate (`eng_gbi`).** It's needed at import time and, for procedural display lists, at runtime ([ADR 0006](adr/0006-rendering-model.md)). The renderer depends only on the types.
- **Surface types:** `CollisionContext::surface_word` returns the raw data words. Their meaning is `oot_game::surface::SurfaceType`, a trait with `SurfaceType_Get*` and `WALL_FLAG_*`. The two bits `z_bgcheck.c` reads itself stay in the engine: soft floors in `BgCheck_RaycastDownImpl`, and bit 27 in the wall displacement.
- **`World` moved to `oot_actors`,** since it names Player and the platform. It's transitional until milestone 3's `PlayState`.
- **`oot_pad` → `oot_game` is gone:** devices and pad state are both `eng_input`. The config file name is a parameter.

### How it was moved

- Every file moved with `git mv`, so the renames are staged and history follows them.
- Module paths were rewritten by script. The only code changes were:
  - `DrawList::intern_material` / `intern_texture`, replacing the interpreter's direct access to the list's private lookups;
  - the extension traits;
  - `PadConfig::find(dir, file)`;
  - `eng_app`'s shared window pieces.
- Line endings were preserved. The 14 files that are CRLF in the working tree still are. Git normalizes all of them to LF in the index (`core.autocrlf`).

### Results

| Check | Before (`spikes-complete`) | After |
|---|---|---|
| `cargo test --workspace` | 87 passed, 1 ignored (`smoke`, a manual dump) | 89 passed, 1 ignored: the same 87, plus the 2 layering tests |
| `python scripts/golden.py check` (54 cases, 78 hashes) | recorded; identical on a second run | **78/78 identical** |
| Windows (`oot`, `oot_sandbox`, `oot_viewer`) | | start, and run for 8 s without errors; `oot` loads Kokiri Forest (3 rooms, 2060 triangles, 3 ms) |
| Layering check | | passes. Mutation checks: `eng_math` → wgpu and `eng_app` → `oot_game` both fail it; cycles are rejected by cargo itself |
| Clippy | | 50 style warnings, all in moved spike code. None added, none fixed (no behaviour changes in this milestone) |

The golden cases cover:
- every sandbox script on the course (`run-roll`, `ledge`, `pit`, `stairs`, `walls`, `turn`, `idle`, `climb50/70/100`, `hang`, `target`, `parallel`, `sword`, `swim`, `platform`, `ramp-stand`), as contact sheets and JSON traces, plus the follow camera, the child and a fixed side view;
- Kokiri Forest: `forward` and `tour` for both ages, treading water in the stream, all 12 spawns, 07:00 and 19:00 (with a target), water frames 8 and 12, the collision view, a fixed view;
- Hyrule Field and the Deku Tree;
- the viewer's Link sheets (adult, child, sword), a Link screenshot, and the Tock sheet.

### Decisions

- [ADR 0001](adr/0001-spike-baseline.md): the baseline commit and tag, and the golden hashes.
- [ADR 0002](adr/0002-crate-layout.md): the layout and the layering rules.
- [ADR 0003](adr/0003-rom-version.md), [0004](adr/0004-decomp-commit.md), [0005](adr/0005-engine-name.md): the ROM, the decomp commit and the engine name, as confirmed.
- [ADR 0006](adr/0006-rendering-model.md), [0007](adr/0007-actor-ownership.md), [0008](adr/0008-asset-pack.md): the rendering model, actor ownership and the asset pack. Decided now, built in milestones 2 and 3.

### Known gaps

- **Two transitional edges** remain: `oot_game` → `oot_import` and `oot_actors` → `oot_import`. The game still reads its tables from the decomp's C and a few assets from the ROM at startup. The layering test lists them, and fails once they're unused, so milestone 2 must delete them.
- `World` is still the spikes' fixed play state (milestone 3).
- The golden hashes are specific to this machine's GPU and driver (ADR 0001).
- The `oot` binary is minimal: the same client as the sandbox, without the debug flags, starting in Kokiri Forest.

## Milestone 2: the asset pack

**Answer:** done.
- `oot import` builds a 40 MB pack from the ROM in about 9 s, in the per-user data folder.
- The game, the sandbox and every test read only the pack. The sandbox runs with no `oot.toml`, no ROM and no decomp.
- The import covers every scene and object: every count matches the XMLs and the `ootx` scans, and everything left out has a reason.
- The golden renders are identical from the pack: 78 of 78, also through the loose-folder dev mode.
- Every extracted table equals what `csrc` reads.

### What was built

| Piece | Where | What it does |
|---|---|---|
| The pack container | `eng_asset` | Named serde records (bincode + zstd) in one versioned file (`ASSETPAK`, header, index, blobs) or a loose folder. Identical records are stored once, the layout is deterministic, sources layer for mods, and `is_current` checks the versions and the ROM's SHA-1 |
| The game's pack | `oot_game::pack` | The contract: `FORMAT_VERSION`, `IMPORTER_VERSION`, record names (`keys`), where packs live (`%LOCALAPPDATA%\oot-clone\packs`, `OOT_PACK`, `OOT_DATA_DIR`), and `GamePack`'s typed access (`game_data`, `scene`, `room`, `link_variant`, …) |
| Record types | `oot_game::{scene, player_lib, data, env, camera, footik}` | Serde on `GameData`, `CameraData`, `FootIkData`, `EnvTables`. New: `SceneTable`, `SceneData` / `LayerData` / `RoomData` / `EntryMesh`, `PlayerRules` (moved from the importer), `LinkVariant` / `LinkFaces`, `pack::Texture`, `pack::Manifest` |
| The importer | `oot_import::pack` (+ `objects`, `tables`) | Tables → objects (on all cores) → Link → scenes → manifest. The C loaders moved here from the game as extension traits (`GameData::load(p)` with `LoadGameData`), and the texture, skeleton and animation readers are shared with the extractor (`objects`) |
| Draw configs in Rust | `oot_game::scene_table` | `Scene_DrawConfigSpot04` (Kokiri Forest, moved up from milestone 4), `Spot00` (Hyrule Field), `Ydan` (the Deku Tree) and `Default`, with `Gfx_TwoTexScroll` / `Gfx_TexScroll` from `z_rcp.c`, and `Math_StepToS` in `eng_math` |
| Texture provenance | `eng_gbi` | Every TMEM word remembers the segment it was loaded from, and each decoded texture gets the mask of the words it reads (`TextureImage::source_segments`). This finds Link's face textures (ADR 0009) |
| The command | `oot import [--rom] [--decomp] [--loose DIR]`, `ootx import` | The first launch imports by itself when the default pack is missing or stale |

Serde was added to the engine types (`DrawList` and everything in it, skeletons, animations, collision, `Tables`). `DrawList` got a content `PartialEq`, and `Stats` uses `BTreeMap`s so records serialize the same way every time.

The extractor now uses the importer's shared readers (`oot_import::objects`). Its textures and models output is byte-identical to before: 7277 files, same hash.

### What the pack holds

`meta/manifest` of the gc-eu-mq-dbg pack (`ootx import` also writes it to `out/import_manifest.json`):

| Kind (XML element) | Listed | In the pack | The rest |
|---|---|---|---|
| Texture | 4960 | 4960 | |
| Skeleton (with its full mesh) | 194 | 184 | 7 Skin limbs, 2 Curve limbs, 1 in an overlay: the same 10 `ootx scan-skeletons` can't read |
| Animation | 1156 | 1054 | 100 in animation-only objects (`object_os_anime`, `object_zl2_anime*`, …) whose skeleton only the actor names; 2 in an overlay |
| PlayerAnimation | 573 | 573 | |
| DList (standalone mesh) | 3481 | 1456 | 1887 limbs (in their skeleton's mesh), 117 in overlays, 21 in scene/room files (in the room meshes) |
| Collision | 203 | 201 | 2 in overlays |
| Scene | 110 | 110 | |
| Room | 401 | 401 | every room of every game layer: 456 distinct, 2533 entries |
| Limb, LimbTable, PlayerAnimationData | 3236 | inside their parents | |
| Cutscene, Path | 102 | 0 | not used yet (milestone 4+) |
| Array, Blob, LegacyAnimation, CurveAnimation, Mtx, Symbol | 154 | 0 | not used by the game |
| Link: meshes per age, model group and hand state | | 64 | plus the eye and mouth textures per age |
| Tables | | `table/math`, `player`, `player_rules`, `env`, `scenes` | |

Overlay assets use relocated code addresses (`0x80xxxxxx`), so they can't be read without the overlay's relocations. The actors that own them will read them when they're ported.

### Results

| Check | Expected | Result |
|---|---|---|
| Golden renders from the pack (`scripts/golden.py check`) | the ROM path's 78 hashes | 78/78 identical; also 78/78 with `OOT_PACK` pointing at a loose folder |
| No ROM or decomp at runtime | | the sandbox renders `spot04_forward` identically from a folder with no `oot.toml`. `oot_game`, `oot_actors` and the sandbox don't depend on `oot_import`, and the layering test enforces it |
| Tables vs `csrc` (`oot_import/tests/pack.rs`) | equal | `sintable` / `sATan2Tbl`; `GameData` with all 573 animations' frames, the REGs, `sAgeProperties`, the item tables, the camera data, the foot IK constants, the rigs and the target ranges; `PlayerRules`; `sTimeBasedLightConfigs`; the scene and object tables: all equal |
| Ported draw configs vs `drawcfg` | equal | Spot04, Spot00, Ydan and Default over 12 frames × 42 states: equal, except Hyrule Field after 18:30, where the C's `Math_StepToS` gives alpha 5 and the interpreter (which doesn't run it) 0 |
| Room meshes and collision vs the ROM path | equal | Kokiri Forest, Hyrule Field and the Deku Tree, all four layers: equal, including the night layers built for 19:00 against the midnight bake |
| Link vs interpreting the ROM | equal | 64 variants × 4 faces equal. The importer checks all 32 faces of the default group and the last face of the others |
| Objects vs the ROM | equal | `gDTSlidingPlatformCol`, `gDTSlidingPlatformDL` (as the spike interpreted it), 60 textures' pixels and texels |
| Counts vs the XMLs and `ootx scan-scenes --all-layers` | every kind accounted for; 110 scenes, 141 headers, 456 rooms, 2533 entries, 168,566 main-header triangles, 0 unknown opcodes, 4 unresolved | exact (the per-layer bake has 199,506 triangles over all layers against the scan's 199,500 with the default state: night and adult geometry) |
| Deterministic | two imports give the same records | identical, except the header's import time and the manifest's timings |
| `cargo test --workspace` | | 99 passed, 1 ignored |

| Import (this machine) | Seconds |
|---|---|
| Tables (C and Player's animations) | 1.1 |
| Objects (textures, skeletons, animations, display lists, collision) | 6.4 |
| Link (64 variants, 8 eyes and 4 mouths per age, about 280 interpreter builds with the checks) | 0.3 |
| Scenes (110 × 4 layers) | 1.0 |
| Write | 0.1 |
| **Total** | **9.0** |

| Pack | |
|---|---|
| File | 40.4 MB (`079b855b….pak`) |
| Records / distinct blobs | 10,838 / 8,831 |
| Before compression | 197.8 MB |
| Loose folder | 373 MB |
| Runtime: open / Player data / Link's 64 meshes / Kokiri Forest | 4 ms / 20 ms / 30 ms / 2 ms |

### Decisions

- **[ADR 0008](adr/0008-asset-pack.md)** (as built): the pack lives in `%LOCALAPPDATA%` rather than `%APPDATA%`, because it's a rebuildable cache and shouldn't roam. `packs/default` names the current one. Blobs are deduplicated and deterministic, and the pack carries a manifest.
- **[ADR 0009](adr/0009-baked-state.md)**:
  - Rooms are baked per game layer at a fixed time (10:00, or midnight for night layers).
  - Link is 64 baked meshes plus a face-texture swap, validated against the interpreter.
  - Draw configs are ported one scene at a time, and unported ones keep their frame-0 values.
- **[ADR 0002](adr/0002-crate-layout.md)**, amended: the importer builds the game's records, so it depends on the game and not the other way round. The C loaders moved from the game into `oot_import::tables`.
- **The course's collision** is built directly, no longer through encode/decode. A test checks it equals its round trip.
- **Animations in files with several skeletons** are decoded for the largest one. Joint *j* reads index entry *j*, so a smaller skeleton uses the first joints.

### Known gaps

- **Unported draw configs.** 49 of the 53 scene draw configs are static in the game (their textures don't scroll). They're ported as scenes need them.
- **Not in the pack yet:**
  - the 100 animations of animation-only objects;
  - Skin and Curve skeletons (Epona and the horses, the treasure chest lid, the time-warp effect);
  - overlay assets (they need relocation);
  - cutscenes and paths (milestone 4+);
  - text and audio (later).
- **Keep objects.** Standalone meshes are baked with `gameplay_field_keep` on segment 5. Dungeon objects that read `gameplay_dangeon_keep` there resolve against the wrong file; their unresolved references say so.
- **Draw config state per rendered frame.** The draw config runs when the renderer sees a new game frame, not in the game loop, so a slow display could skip a `Math_StepToS`. It moves into the frame when `PlayState` owns it (milestone 3).
- **No decomp-free import yet:** the importer still needs the decomp for the XML offsets and the C tables (ADR 0008's later phase).

## Milestone 3: PlayState and the actor system

**Answer:** done. The spikes' fixed `World` is gone. The game runs through:
- `PlayState` (`oot_game::play`), in the decomp's frame order;
- an actor system (`oot_game::actor_ctx`): a generational arena, category lists, profiles, spawn and kill;
- generic render blending;
- draw submission into OPA/XLU lists with per-draw material parameters.

Player, the dummy target and `Bg_Ydan_Hasi` are actors, and the cameras run from `PlayState`. All 64 gameplay tests pass through it. Every golden trace and every room render is unchanged. The course renders changed by a few pixels each, because the circle shadow now draws in the XLU list.

### What was built

| Piece | Where | The decomp |
|---|---|---|
| `PlayState`: collision, the actor context, the cameras, the target context, `gameplayFrames`, the input, the debug switches, the respawn point | `oot_game::play` | `PlayState` (`z_play.c`) |
| The frame (`tick_with`), documented step by step | `oot_game::play` | `Play_Update` → `Actor_UpdateAll` → `AnimationContext_Update` → `Camera_Update`, then what `Play_Draw` changes |
| The actor context: generational handles, `ACTORCAT_*` lists newest first, take out / put back, insert / remove, `downcast` | `oot_game::actor_ctx` | `ActorContext`, `Actor_AddToCategory`, `Actor_RemoveFromCategory` |
| `ActorImpl` (`update`, `animation_update`, `draw_update`, `render_state`, `draw`, `destroy`, `as_player`), `ActorProfile`, `PlayerIface` | `oot_game::actor_ctx` | `ActorInit`, `GET_PLAYER` |
| The base `Actor`: id, category, flags, params, focus, `targetMode`, `targetPriority`, `isTargeted`, `freezeTimer`, the distances and yaw to Player, killed | `oot_game::actor` | `Actor` (`z64actor.h`), `Actor_Init`, `Actor_Kill`, `Actor_UpdateAll`'s distances |
| Render blending: `RenderState` (position, rotation, scale, joint table, and extra angles, values and switches), `RenderFrame`, the teleport flag | `oot_game::play` | (the N64 draws each frame it updates) |
| Draw submission: `MeshKey`, `DrawCmd`, `DrawParams`, `DrawLists` | `eng_gfx::submit` | `POLY_OPA_DISP` / `POLY_XLU_DISP` |
| The list renderer: `MeshCache` (upload on first use, one instance per use in a frame, re-skin on change), `MeshSource`, `render_lists` | `eng_render::lists` | |
| The target context over actors (targets are actors with `ACTOR_FLAG_0`), and the reticle as submitted lines | `oot_game::target` | `func_8002C7BC`, `func_80032AF0` |
| Player as an actor (`Player_InitVars`), with its draw (posing moved from the app; mesh key with the face; the circle shadow in the XLU list) and `PlayerIface` | `oot_actors::player` | `Player_Update`, `Player_Draw` |
| `Bg_Ydan_Hasi` as an actor (its transform in the base actor, its draw) | `oot_actors::bg_ydan_hasi` | `Bg_Ydan_Hasi_InitVars`, `BgYdanHasi_Draw` |
| The dummy Z-target as an actor | `oot_actors::dummy_target` | (sandbox only) |
| `new_play`, `PlayExt` (`player()`, `targets()`, `platform(i)`, `spawn_target`, `spawn_platform`) | `oot_actors` | |
| The follow camera, `CameraKind`, `CamView` | `oot_game::camera` (from `world.rs`) | (spike 03's camera) |

The play client now builds each frame as `Room_Draw` (the rooms' entries in `Room_DrawCullable` order, with the draw config's segment values as draw parameters), then `Actor_DrawAll` from the blended frame. Its mesh source resolves Link's variants with the face, the pack's meshes, the loaded rooms, the course and the built-in meshes.

### The frame

| # | Step | Decomp |
|---|---|---|
| 1 | `gameplayFrames++`, the input | `Play_Update` |
| 2 | Every category in order, each actor newest first: `prevPos`, distances and yaw to Player, the update if due. Killed actors deleted. After BG, `DynaPoly_UpdateContext`. Then the target context with the last drawn view, and `DynaPoly_UpdateBgActorTransforms` | `Actor_UpdateAll` |
| 3 | Every actor's queued animation requests | `AnimationContext_Update` |
| 4 | The follow camera, `Camera_Update` | `Play_Update` |
| 5 | Every actor's draw-time state (Player's foot IK), and the view for the next frame's target context | `Play_Draw` |
| 6 | The sandbox's void-out | |
| 7 | Render states captured | |

This was done in two steps:
- **First, the spikes' `World` order exactly.** It had `AnimationContext_Update` and the foot IK straight after Player's update, and the target context after the cameras with the new view. This proved the new runtime reproduces the spikes bit for bit: all traces identical.
- **Then the decomp's order.** No test and no golden changed. The differences it could make (the enemies' distances to Player before its animation's root motion, a one-frame-older view for the on-screen test) don't show in any scripted run.

### Results

| Check | Result |
|---|---|
| Gameplay tests through `PlayState` (`cargo test -p oot_actors`) | 64 passed (camera 6, foot IK 3, ledges 7, movement 18, platform 8, sword 4, targeting 8, water 10), `smoke` ignored as before. The tests use `PlayExt` (`w.player()`, `w.spawn_target(p)`, `w.target(0)`) and are otherwise unchanged |
| Actor system (`cargo test -p oot_game --test actors`, 7) | generational handles; lists newest first, in category order; take out and put back; `Actor_UpdateAll`'s category order, with a spawned actor waiting a frame at the head of its list; `Actor_Kill` then deletion when the loop reaches it (and `destroy`); an updating actor isn't in its slot but reaches the others; `freezeTimer` as `DECR`; blending the short way round, switches halfway, no blending into a teleport or a new actor |
| Golden traces (27) | identical, in both frame orders |
| Golden renders | the room renders (Kokiri Forest, Hyrule Field, the Deku Tree, the viewer) identical. 25 course images and the scene collision view differ by 9 to 3,263 pixels each: the soles of Link's boots, where the translucent shadow now blends over them. Re-recorded (`golden/README.md`) |
| `cargo test --workspace` | 106 passed, 1 ignored |
| The windows | `oot` and `oot_sandbox` open, and play with blending at the display rate |

### Decisions

- **[ADR 0007](adr/0007-actor-ownership.md)** (as built):
  - Actors embed the base `Actor` and implement `ActorImpl`.
  - Updates take the actor out of its slot.
  - Player is reached through `PlayerIface` and the content crate's `PlayExt`.
- **[ADR 0006](adr/0006-rendering-model.md)** (as built):
  - Actors and rooms submit `DrawCmd`s into OPA/XLU lists.
  - The renderer caches meshes by key, with one instance per use.
  - Segment textures are part of the mesh key; segment values are per-draw parameters.
- **Keep the spikes' behaviour first, then switch to the decomp's order.** Comparing the two made the order change measurable. It turned out to change nothing.
- **The draw-time state (foot IK) runs once per game frame**, where `Play_Draw` would, not per rendered frame. It writes into the joint table the next frame's animation blends from.
- **`RenderState` is generic.** Actors pack what they need into angles, values and switches (Player: its look angles, its speed and `yOffset`, its face and model group). The draw function that packed them reads them back, so the blending needs no per-actor code.

### Known gaps

- **Culling isn't ported.** Every actor counts as in view (`ACTOR_FLAG_6`), so none skips its update or draw. `isDrawn` isn't tracked, so killed actors are deleted in the same frame rather than the next.
- **The draw config runs outside the frame.** It runs when the renderer sees a new game frame, not in `Play_Update` or `Play_Draw`. It moves into the scene system with the rooms (milestone 4).
- **Actors are constructed directly** (`new_play`, `spawn_target`, `spawn_platform`), not spawned from an id and params. Spawning from the scene's actor lists is milestone 4.
- **Only baked meshes.** The runtime display-list path (ADR 0006's procedural case) has no user yet.

## Milestone 4: scenes, rooms and spawning

**Answer:** done.
- A scene is entered the way the game enters it, by `Play_Init` from an entrance. Every actor placement spawns: as a ported actor, or as a placeholder with its real profile.
- Rooms load and change through the room context and `En_Holl`, and exits lead to other scenes through the transition.
- **Kokiri Forest:** its 78 placements in the first room all spawn (plus Navi and both `En_Holl` planes). A headless scripted run walks Link from his porch into his house and back out.
- **Every scene:** all 110 enter and play, as child and as adult.
- The spikes' view of a scene (every room drawn, Player alone) remains the sandbox's default, so every golden trace and render is unchanged.

### What was built

| Piece | Where | The decomp |
|---|---|---|
| Pack format 2: the actor table with every `ActorInit` (`table/actors`), the entrance table, and each scene layer's entrance list, exit list, transition actors and keep object id, and each room's object list | `oot_import` (`tables::LoadActorTable`, `room::load_entrances`, `scene::list_extent`), `oot_game::{actor_table, scene}` | `actor_table.h` + `<Name>_InitVars`, `entrance_table.h`, `SCENE_CMD_ID_ENTRANCE_LIST` / `EXIT_LIST` / `TRANSITION_ACTOR_LIST` / `OBJECT_LIST` |
| `ootx scene-info`: a scene layer's spawns, entrances, exits, transition actors, and each room's objects and placements by name, from the pack | `ootx` | |
| `SaveContext`: entrance, age, time, respawn points, entrance speed, next transition type | `oot_game::save` | `gSaveContext` |
| `Play_Init`: the scene layer (with Hyrule Field's and Kokiri Forest's special cases), `Play_SpawnScene`, the header, the first room, Player, the cameras; a scene change rebuilds the play state (`reinit`) | `oot_game::play_scene` | `Play_Init`, `Play_SpawnScene`, `Play_InitScene`, `Scene_Command*`, `func_80096FE8`, `func_800304DC` |
| The room context: current and previous room, loads finishing the next frame, the room header, `func_80097534` | `oot_game::room::RoomContext`, `play_scene` | `func_8009728C`, `func_800973FC`, `func_80097534`, `EnHoll_SwapRooms` |
| The object context: banks, `Object_Spawn`, the room object list swap, loads one frame late | `oot_game::object_ctx` | `Object_InitBank`, `Object_Spawn`, `Object_UpdateBank`, `Scene_CommandObjectList`, `func_800982FC` |
| Spawning by id: the profile from the table, the ported constructor or a `Placeholder`, init deferred until the object loads (`Uninit`), the room's actor list, transition actors, the room-change and object kills | `oot_game::spawn`, `play` | `Actor_Spawn`, `Actor_Init`, `Actor_SpawnEntry`, `Actor_SpawnTransitionActors`, `Actor_UpdateAll`, `func_80031B14`, `func_80031A28`, `Actor_Delete` |
| The transition: `Play_Update`'s trigger and modes, `TransitionFade`, the fills and the instant cut, `Scene_SetTransitionForNextEntrance`, void-outs and respawns | `oot_game::{transition, play_scene}` | `Play_Update`, `Play_SetupTransition`, `z_fbdemo_fade.c`, `Play_TriggerVoidOut`, `Play_TriggerRespawn`, `Play_SetupRespawnPoint` |
| `PlayIo`: the save, the transition and the scene flags, lent to Player's update | `oot_game::play_scene` | (`play->` and `gSaveContext` writes) |
| Player: `Player_Init`'s respawn point and start modes 0, 8..15 (walk in, stand), Navi's spawn, the exit and void check, the exit walk, the void fall | `oot_actors::player` | `Player_Init`, `D_80854738`, `func_8083CA20`/`54`/`9C`, `func_8083C910`, `func_80838E70`, `func_80839034`, `func_80845CA4`, `func_80845BA0`, `func_80845964`, `func_80838F5C`, `func_80838FB8`, `func_8084F88C`, `func_8083CF5C` |
| `En_Holl`: the room-change planes, every kind | `oot_actors::en_holl` | `z_en_holl.c` |
| `Math_SmoothStepToF` | `eng_math` | `z_lib.c` |
| Scripted-play helpers: steer the stick towards a point, find an exit's floor | `oot_actors::script` | |
| The play client: the scene from `PlayState` (rooms from the room context, the draw config's values per game frame), the fade over the frame, placeholder markers (P), entrances (`oot --entrance`, `oot_sandbox --entrance`) | `oot` | `Play_Draw`'s `Room_Draw` of `curRoom` and `prevRoom`, `TransitionFade_Draw` |
| The sandbox's `house` script and `--shots-at` | `oot_sandbox` | |

The M3 gap "the draw config runs outside the frame" is closed: `Scene_Draw` runs once per game frame in `PlayState` (step 5 of the frame), and the renderer uses its values.

### Kokiri Forest from the pack

`oot_sandbox --scene spot04 --entrance` (or `oot`): `ENTR_SPOT04_0`, child, 10:00, layer 0.

| | Count | Notes |
|---|---|---|
| Rooms | 3 | Room 0 the village (the first room); 1 the path to the Deku Tree; 2 the Kokiri Sword's maze |
| Placements (actor lists) | 78 / 9 / 12 | Room 0's all spawn in the first frame (`numSetupActors`) |
| Transition actors | 2 | Both `En_Holl` kind 4 (horizontal planes 200 wide): 0 between rooms 0 and 2 (in the crawlspace), 1 between rooms 1 and 0 |
| Exits | 12 | Exit 4 `ENTR_LINK_HOME_1`; the floors use 2..7 and 9..11 |
| Actors after the first frame | 82 | Player, 2 `En_Holl` (ported); Navi and the 78 placements (placeholders) |
| Actors waiting for their object after frame 1 | the ones on room 0's objects (`object_gs`, `object_md`, `object_kanban`, …) | `Play_Init`'s room load swaps them in; frame 1's `Object_UpdateBank` starts the DMA and frame 2's finishes it, so they initialise in frame 2 (and update from frame 3) |

The room change (`en_holl_changes_rooms_and_deletes_the_old_rooms_actors`):
- walking from the village towards the Deku Tree, `En_Holl` 1 requests room 1 50 to 100 units past its plane;
- the next frame the load finishes and `EnHoll_NextAction` lets room 0 go;
- `func_80031B14` deletes room 0's 78 placements, `En_Holl` 0 (whose destroy re-arms its entry) and Navi's placeholder (the real Navi leaves the rooms in her init: a known gap);
- room 1's 9 placements spawn, and `En_Holl` 1 stays, now in room 1.

### Into Link's house and back out

`oot_sandbox --entrance ENTR_SPOT04_3 --child --script house --trace out/m4/house.json` (and the test `link_walks_into_his_house_and_back_out`). The script steers the stick towards each exit's floor, as a player would.

| Frame | Scene | What happens |
|---|---|---|
| 1 | Kokiri Forest | `Play_Init` by `ENTR_SPOT04_3`: spawn 3 on the porch (−31, 100, 1073), params `0x0DFF`: start mode 13 (`func_8083CA20`: the exit walk with `unk_850` −20, standing, since `linearVelocity` is 0). The fade in (`TRANS_TYPE_FADE_BLACK_FAST`) is set up: alpha 255 |
| 9 | | The fade is done: 7 updates of `R_UPDATE_RATE` 3 reach `transFadeDuration` 20 |
| 20 | | `unk_850` reaches 0: `func_8083CF5C` stands (`func_80840BC8`) |
| 31 | | The script turns Link back to the door |
| 42 | | On the floor with exit 4 (z 1126.6): `nextEntranceIndex` `ENTR_LINK_HOME_1`, `TRANS_TRIGGER_START`, the exit walk at the entrance speed |
| 51 | Link's house | The fade out took 9 frames; `Play_Init` by `ENTR_LINK_HOME_1`: spawn 1 (−4, 0, −114), `0x0EFF`: start mode 14, walking in at speed 2 for 15 frames |
| 52 | | The room's 3 placements (and Navi): 5 actors; the fade in starts |
| 66 | | The walk in ends and `func_8083CF5C` runs on (`func_8083C858`), then stands at z −65 |
| 94 | | Back on the house's exit floor (exit 2: `ENTR_SPOT04_3`) |
| 103 | Kokiri Forest | Spawn 3 again, standing; 82 actors from frame 104 |
| 133 | | Done |

What the test asserts, all derived from the C:
- the spawn positions and start modes the entrances name;
- the exit indices (4, and 2 in the house);
- the fade taking 9 frames each way;
- the walk in (speed 2, then past z −94);
- two scene changes;
- the void-out point: the entrance Link came in by, `func_80845C68`.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 116 passed, 1 ignored |
| Scene tests (`cargo test -p oot_actors --test scenes`, 6) | Kokiri's placements; the `En_Holl` room change; the house walk; a void-out back to where Link came in; the ported profiles against the actor table; every scene entering and playing 30 frames as child and adult (220 of 220) |
| Pack oracles (`cargo test -p oot_import --test pack`, 10) | New: the entrance and actor tables against the C (1,556 entrances; 471 actor rows, every actor with its `ActorInit`); the length-less lists cover every exit a floor uses and the entrance each leads to (1,000+ exits); transition-actor rooms and object lists in range |
| Object banks (`object_ctx` unit test) | Room objects load a frame after the swap; a room change keeps matching banks and drops the rest |
| Golden traces and renders | 78 of 78 identical (the sandbox's scenes stay the spikes' view unless `--entrance`) |
| Import | 9.1 s, 40.5 MB, 10,839 records |
| The windows | `oot` enters Kokiri Forest by `ENTR_SPOT04_0` and plays; `oot_sandbox --entrance ENTR_SPOT04_3` too |

### Decisions

- **[ADR 0010](adr/0010-scenes-and-spawning.md):**
  - a scene change rebuilds the play state from the save, as `Play_Init` does;
  - actor profiles are read from the overlays' C, keyed by symbol (keeping `Boss_Dodongo`'s wrong id and the four zeros);
  - unported ids spawn placeholders with their real profiles;
  - room and object loads keep the game's frame timing;
  - Player writes to play through `PlayIo`;
  - the length-less lists are bounded like the decomp's extraction bounds them.
- **ADR 0007 and 0008 amended** for spawning by id (`Uninit`) and format 2.
- **The spikes' scene view stays the sandbox's default.** Entering by an entrance changes what's on screen (only the loaded rooms, placeholders if asked). So it's an opt-in flag, `--entrance`, rather than a change to the goldens. The game binary always enters by an entrance.
- **The house run starts on the porch.** Link's house is up a ladder, and ladder climbing isn't ported. `ENTR_SPOT04_3` (arriving from the house) puts Link at the door; the run goes in and comes back out to the same spot.

### Known gaps

- **Prerendered rooms aren't drawn.**
  - Link's house and the other interiors are `ROOM_SHAPE_TYPE_IMAGE`: a JPEG background plus a little geometry. Only the geometry draws, flat-coloured.
  - The fixed cameras those rooms use aren't ported either: the scene's bg camera list, `Camera_ChangeBgCamIndex` from the spawn params, `CAM_SET_SCENE_TRANSITION`. In a house the camera is the Normal camera.
- **Only `En_Holl` changes rooms.** Doors (`En_Door`, `Door_Shutter`) are placeholders. So rooms behind doors (every dungeon, Kokiri's shop and houses have exits instead) can't be entered, and the maze's crawlspace needs crawling.
- **Placeholders keep the room they spawned in.** Actors whose init takes them out of the rooms (Navi, `Object_Kankyo`, `En_Kusa`, `En_Ishi`…) are deleted by a room change. They also don't run their init, so their params-dependent behaviour (killing themselves on flags, spawning children) doesn't happen.
- **Transitions:** only the fades, fills and the instant cut run as in the game. Wipes, the triforce and circles (doors, grottos) run as a 60-step black fade; the sandstorm and cutscene fills end at once. No title cards, music or sound.
- **Player:**
  - start modes 1..7 (the pedestal, warps, blue warps, the jump into the Kokiri opening) stand still instead;
  - entering deep water treads water instead of `func_8084D7C4`;
  - conveyor floors don't steer the exit walk;
  - the `ENTR_RETURN_*` groups (fountains, shops) aren't ported;
  - `Play_TriggerRespawn`'s Ganon's Castle and Hyrule Field special cases are left out.
- **Time doesn't pass**, and the environment's lights are computed once per scene load (`Environment_Update` isn't ported).
- **Scene layer special cases:** no quest items or event flags exist. Child Hyrule Field is always layer 0; adult Kokiri Forest always layer 2.
- **The sandbox's contact sheets** draw every tile with the last frame's rooms and draw-config values, so a sheet across a scene change shows the last scene. Use `--shots-at` for runs that change scene.
- **Carried over from milestone 3:**
  - culling isn't ported (every actor is in view);
  - `isDrawn` isn't tracked;
  - an actor's init can spawn others before the actor itself is in its list.

## Recommended next step

**Phase 3, the Kokiri Forest vertical slice, starting with the actors Kokiri's first room spawns most** (ARCHITECTURE-PLAN §5, Phase 3). The framework is now there for them:
- the placements spawn with their profiles;
- the objects are loaded;
- the room changes and exits work.

The order I'd take:
1. **Draw the placeholders' real models where it's cheap.** Many Kokiri props (`En_Kusa`, `En_Ishi`, `Obj_Hana`, `En_Kanban`) are one display list from their object, already in the pack.
2. **Port the first NPC with its skeleton,** `En_Ko` (the Kokiri children, 8 in room 0). It needs the collision-check context (`CollisionCheck_*`, colliders), which every actor after it needs too.
3. **Port `En_Door` and the room shape's prerendered backgrounds together.** That opens the houses and the shop, and the fixed cameras come with them.
4. **Then the systems the plan lists:** the message box for `En_Wonder_Talk2` and the signs, items (`En_Item00`), and ladder climbing (so the house run can start from the ground).

Before that, one cheap check: compare a Kokiri screenshot with Project64 at the porch spawn. It would confirm the room context draws exactly what the game draws when Link stands near the `En_Holl` planes.
