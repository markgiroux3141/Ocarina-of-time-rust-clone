# From spikes to the game: architecture plan

The four spikes answered the feasibility question. From the user's ROM plus the decomp we can:
- Decode every model, texture, animation, scene and sound.
- Draw a real scene the way the game draws it.
- Run faithful ports of `z_player.c`, `z_camera.c`, `z_bgcheck.c` and friends that match the C numerically.

This document plans the move from spike code to the codebase that becomes the full game:
- It lists what transfers.
- It lays out the target architecture: engine vs game, and how assets are stored.
- It gives the order of work.

It's a starting point: the decisions below are recommendations, and each one gets an ADR (architecture decision record) when it's adopted. The ADRs are in [adr/](adr/README.md), and [GAME-01-foundation.md](GAME-01-foundation.md) records how the plan was carried out. Where the work diverged from the plan, this document now says what was done, marked **(as built)**.

## 1. What we have, and what happens to it

About 28,000 lines of Rust in 8 crates, with 87 tests. The verdicts in the table below:
- **keep**: moves as-is, maybe renamed.
- **refactor**: the logic is right but its shape isn't.
- **tool**: stays, but only at import or test time.
- **replace**: spike scaffolding.

| Area | Where now | Verdict | Goes to |
|---|---|---|---|
| ROM access: dmadata, Yaz0, file table | `oot_core::{rom, yaz0}` | keep | importer |
| Decomp XML symbol index (names → offsets) | `oot_core::symbols` | keep | importer |
| F3DEX2 interpreter, TMEM/texture decode, combiner decode | `oot_core::{gbi, texture, combiner}` | keep; split the *types* (material, batch, draw list) from the *interpreter* | types → engine gfx; interpreter → importer (and available at runtime for procedural lists, see §3.4). **(As built:** types and the combiner in `eng_gfx`; the interpreter and TMEM decoding in `eng_gbi`, an engine crate the importer uses**)** |
| Collision header decode/encode, scene header commands, room shapes | `oot_core::{collision, scene, room}` | keep | importer (decode) + engine/game types (runtime structs). **(As built:** `CollisionHeader` in `eng_collision`, its codec in `oot_import::z64`; the header commands decoded by `oot_import::{scene, room}` into `oot_game::scene`'s records (milestone 2), which `oot_game::play_scene` executes at runtime (milestone 4)**)** |
| Skeletons (flex/LOD), animation formats | `oot_core::{skeleton, anim}` | keep | engine anim. **(As built:** the types in `eng_anim`; the z64 binary decoders in `oot_import::z64`**)** |
| Player draw rules read from `z_player_lib.c` | `oot_core::player` | tool → port | rules become Rust code; the tables become pack data; the C reader stays as a test oracle |
| C table reader | `oot_core::csrc` | tool | importer: extracts tables into the pack; tests compare pack vs C |
| Scene draw-config interpreter (runs `z_scene_table.c` functions) | `oot_core::drawcfg` | tool → port | each draw config becomes a Rust function in the game; the interpreter stays as the oracle that tests them |
| Synthetic character "Tock", test course | `oot_core::synth`, `oot_game::course` | keep | test fixtures |
| wgpu renderer (RDP combiner in WGSL, render-mode pipelines, fog, lights, headless readback) | `oot_render` (one 790-line file) | refactor | engine gfx: split into device/pipelines/materials/passes. **(As built:** its own crate, `eng_render` (device, view, pipelines, materials, model, passes), so `eng_gfx` stays free of wgpu**)** |
| Math (binary angles, `Math_SinS` tables, `Math_*StepTo*`, SkinMatrix) | `oot_game::math`, `dyna::srt_matrix`, `skeleton::local_transform` | keep | engine math |
| bgcheck: static + DynaPoly + water boxes | `oot_game::{bgcheck, dyna}` | keep; rename `StaticCollision` → `CollisionContext`; surface-type *meaning* moves to the game | engine collision |
| SkelAnime + AnimationContext queue | `oot_game::skelanime` | keep | engine anim. **(As built:** `oot_game::skelanime`. `z_skelanime.c`'s Link functions read Player's animation data and move the actor, so it's game framework**)** |
| Input: `PadMgr`/`padutils`, device mapping, Retro-Bit profile | `oot_game::input`, `oot_pad` | keep; fix the inverted dependency (`oot_pad` → `oot_game`) | engine input (`eng_input::{pad, device}`) |
| Actor base physics (`Actor_MoveForward`, `Actor_UpdateBgCheckInfo`) | `oot_game::actor` | refactor | game framework: the base `Actor` of the actor system |
| Player (`z_player.c` port: movement, ledges, targeting, sword, swimming) | `oot_game::player` (3.6k lines) | keep the logic; refactor into an actor, split into modules by action group | game content |
| Camera (`Camera_Normal1`), target context, environment lights, foot IK | `oot_game::{camera, target, env, footik}` | keep | game framework |
| Game data tables loaded from decomp C at startup | `oot_game::data` | replace | pack data, typed, loaded from the pack |
| `World` (a fixed struct: Player + targets + platforms, tick order hard-coded) | `oot_game::world` | replace | `PlayState` + actor system (§3.3). Keep its frame order and 20 Hz step + interpolation. **(As built:** moved to `oot_actors::world` in milestone 1, since it names Player and the platform; replaced in milestone 3 by `oot_game::play::PlayState` and `oot_game::actor_ctx`, and deleted**)** |
| `Bg_Ydan_Hasi` | `oot_game::bg_ydan_hasi` | keep | first actor in the content crate |
| Play app: window, scripts, contact sheets, CLI | `oot_play` (874-line `main.rs`) | replace | app shell (engine) + game binary + a dev sandbox that keeps the scripted headless runs. **(As built:** `eng_app`; `oot` is a library (the play client) plus the game binary; `oot_sandbox` keeps every `oot_play` flag**)** |
| Viewer, validation CLI | `oot_viewer`, `ootx` | keep | tools |
| Extractor (editable glTF/PNG/WAV/MIDI/JSON) | `oot_extract` | keep | tools. Its sequence player (tick-accurate) and VADPCM decoder seed the audio engine; its text decoder seeds the message system |

Also carried over: the working rules that made the spikes reliable (§4) and the test style (§6).

## 2. Principles

1. **No game data in the repo, ever.** The repo holds code, documentation and non-derived metadata (names, offsets, hashes). Everything Nintendo-made is produced on the user's machine from their ROM.
2. **The engine knows nothing about Zelda.** Game code depends on the engine, never the reverse. A crate-dependency check in CI enforces it.
3. **Faithful by default.**
   - Game systems are ports of the decomp, function by function, with the decomp name in a comment.
   - Numeric behaviour is tested against the C.
   - Deliberate deviations are listed in the code and the docs.
4. **The runtime never reads the decomp or parses C.** Reading C at runtime was a spike shortcut. C *logic* becomes Rust code. C *data tables* are extracted at import time into the asset pack. The C readers (`csrc`, `drawcfg`) become test oracles.
5. **One asset path.** Original assets, mods and custom levels all end up in the same engine-native format and load through the same code.

## 3. Target architecture

### 3.1 Layers and crates

```
apps        oot (the game)      oot_sandbox (dev: test course, scripts, sheets)      tools: ootx, oot_viewer, oot_extract
              │                    │                                                   │
content     oot_actors  (one module per decomp overlay: ovl_player_actor, ovl_Bg_Ydan_Hasi, ...)
              │
game        oot_game    (decomp src/code: PlayState, actor system, camera, kankyo, target, scene/room
              │          manager, collision_check, save/flags, message, ...)
              │
engine      eng_app  eng_gfx  eng_anim  eng_collision  eng_input  eng_asset  eng_math  (eng_audio later)
              │
import      oot_import  (ROM + decomp → asset pack; uses the interpreters, decoders and C readers)
```

**(As built, milestone 1:)** the folders are `crates/{engine, import, game, apps, tools}`. The engine has `eng_gfx` (types), `eng_gbi` (the interpreter) and `eng_render` (wgpu) instead of one gfx crate. `oot_import` sits beside the game, not under the engine. From milestone 2 it depends on the engine and the game (it builds the game's records), and no runtime crate depends on it. The rules are checked by `crates/tools/layering` ([ADR 0002](adr/0002-crate-layout.md)).

The split mirrors the decomp itself:
- `src/code` is mostly game framework.
- `src/overlays` is content.
- The engine is what's left when the Zelda-specific meaning is taken out: an N64-style renderer with the combiner/render-mode material model, skeletal animation, triangle-mesh collision with surface data words, pads, the fixed-step loop, and asset I/O.

`eng_` is a placeholder prefix until the engine is named.

Notes on a few boundaries:
- **Surface types.** `eng_collision` returns the raw surface data words. What they mean (floor types, wall flags, conveyors) belongs in `oot_game`.
- **Materials.** `eng_gfx`'s material model is the N64 one (combiner, othermode, tiles, render mode), since that's what every asset uses. Modern materials can be added later next to it.
- **Player** lives in `oot_actors`, like any overlay. It's big, so it gets its own module tree split by action group (ground movement, ledges and climbing, targeting, items and sword, water).

### 3.2 Assets: import once, load from a pack

```
user's ROM ──┐
             ├─ oot_import ──► asset pack (in the user's data dir) ──► eng_asset ──► game
decomp (dev) ┘   (decode, interpret DLs, extract tables, validate)
custom levels (glTF + JSON) ─── oot_import ──► mod pack ──┘ (layered over the base pack by asset name)
```

- **When:**
  - `oot import --rom <path>` runs on first launch or on demand.
  - The pack is written to the user's data directory, e.g. `%APPDATA%/<game>/packs/<rom-sha1>.pak`, never the repo. **(As built:** `%LOCALAPPDATA%\oot-clone\packs\<rom-sha1>.pak`, since it's a rebuildable cache; `packs/default` names the current one, and `OOT_PACK` / `OOT_DATA_DIR` override it**)**
  - The header records the ROM's SHA-1, the importer version and the pack format version. A stale pack is rebuilt automatically.
- **What:** engine-native, ready to load, no N64 decoding at runtime:
  - Meshes: interpreted display lists, i.e. vertex buffers plus material records, keeping the dynamic-parameter markers the spikes already use.
  - Textures: decoded RGBA8, plus the original format for effects that need it.
  - Skeletons, standard and Player animations, collision headers.
  - Scenes and rooms as typed structs: headers, lights, room shapes, actor/spawn/exit lists, paths, water boxes.
  - Game tables (sAgeProperties, D_808540F4, camera settings, light configs, …) as typed records, each with its source symbol.
  - Text, and audio (later).
- **How:**
  - One file: header, index, then compressed blobs (e.g. zstd). The types are serialized with serde into a compact binary format (postcard or bincode).
  - Assets are keyed by their decomp name (`object_link_boy/gLinkAdultSkel`, `spot04_scene/room_0`), which also gives stable IDs (hash of the key).
  - A dev mode loads loose files from a folder for fast iteration.
- **Mods and custom levels:**
  - The level plan in [ASSET-EXTRACTION.md](ASSET-EXTRACTION.md) (glTF + JSON levels that refer to original textures by decomp name) becomes an importer input.
  - Its output is a small mod pack layered over the base pack.
- **The decomp at import time:**
  - Today the importer needs the decomp checkout for XML offsets and C tables. That's fine for development.
  - Shipping to people who only have a ROM needs a decomp-free import: a committed manifest of names, offsets and types (metadata, not game data), and table values read from the ROM's code and overlay files at their addresses. Getting those addresses means building the decomp once for its symbol map. This is a later phase; the format should allow it from the start.

### 3.3 Runtime

- **Loop.**
  - The game logic runs at a fixed 20 Hz (`R_UPDATE_RATE` 3) and rendering interpolates between frames, as in the spikes.
  - Interpolation becomes generic: every actor and camera keeps previous and current render state (transforms, joint tables). A teleport flag skips blending.
- **Frame order.** One documented function mirrors `Play_Update` → `Actor_UpdateAll` → cameras → `Play_Draw`. The spikes showed the order matters:
  - Water checks run after the move and bgcheck.
  - DynaPoly rebuilds after the BG category updates.
  - Previous transforms are stored at the end of `Actor_UpdateAll`.
  - Targets update after actors, cameras after actors.
- **PlayState**: the owner of everything in play: collision context, actor context, cameras, environment, the room context, the target context, and the frame counters.
- **Actor system** (`z_actor.c`):
  - The `ActorContext` holds category lists (`ACTORCAT_*`).
  - The base `Actor` holds what `z_actor` needs: world/home/shape/focus, velocity, bgcheck info, flags, params, room, scale.
  - Each actor type has a profile (the `ActorInit` equivalent: id, category, object dependency, init/update/draw/destroy).
  - Actors are spawned from room and scene actor lists and by other actors. Kill and delete follow `Actor_UpdateAll`'s rules.
  - Ownership: actors live in a generational arena. During its update, an actor is taken out of its slot, so it can have `&mut PlayState` including access to the other actors; then it's put back.
  - Unported actor ids get a placeholder that draws a marker and logs, so every scene loads.
- **Rooms and scenes** (`z_room.c`, `z_scene.c`):
  - Loading from the pack, room changes (`Actor::room` becomes real), transition actors, exits.
  - Draw configs are ported Rust functions called per frame.
- **Drawing:**
  - Actors and rooms submit draw commands to OPA and XLU lists, like `POLY_OPA_DISP` / `POLY_XLU_DISP`: a mesh handle, a matrix or joint palette, and per-draw material parameters (segment bindings for textures, prim/env colours, tile scroll).
  - The engine renders the lists in order. This is the spikes' dynamic-segment system (`SegmentValues`, `uv_dyn`, `env_dyn`) made general.

### 3.4 The rendering model (the main ADR)

Actor draw code in the decomp builds display lists at runtime. There are three ways to handle that:

- **A. Interpret display lists every frame** (the Ship of Harkinian / Fast3D approach). Draw functions port almost verbatim. It costs per-frame interpretation, and custom non-N64 content is awkward.
- **B. Baked meshes plus a draw API.** Display lists are interpreted at import time, and actor draw functions are translated to "draw mesh X with these parameters". This is faster and cleaner, and suits custom levels, but every draw function needs translating and procedural geometry needs engine features.
- **C. Hybrid (recommended).** B for everything static, which is the vast majority. For the procedural cases (effects, skybox, a few actors), a small runtime command buffer goes through the same interpreter. The spikes already do exactly this for scene draw configs.

## 4. Working rules carried over from the spikes

- Port by function, keeping the decomp's names in comments.
- Every constant cites its source function or table.
- Faithful quirks and bugs are kept and marked `@bug (game)`.
- Numbers come from the decomp/ROM, never guessed.
- Tests derive their expected values from the C, not from the port.
- Visual checks are headless contact sheets in the git-ignored `out/`.
- Rust 1.95 is pinned. Existing files are CRLF; keep line endings. `CARGO_TARGET_DIR=target/<session>` keeps concurrent sessions apart.

## 5. Phases

Each phase has exit criteria and ends with its doc section. The spikes stay reproducible at every step, since their tests and sheets are the regression suite.

**Phase 0: foundation (no behaviour changes)**
- Commit and tag the spike state.
- Write the ADRs:
  - Crate layout.
  - Asset pack.
  - Rendering model.
  - Actor ownership.
  - ROM version and decomp commit (see §7).
- Restructure the workspace into the layers above, moving code without changing behaviour.
- Add the crate-dependency check.
- **Exit:**
  - All 87 tests pass.
  - The spike 04 sheets re-render identically (compare image hashes, never commit images).
  - The sandbox runs the old scripts.

**Phase 1: the asset pack**
- `oot_import` writes the pack: meshes, textures, skeletons, animations, collision, scenes and rooms, and game tables.
- `eng_asset` loads it, with a loose-folder dev mode.
- The sandbox and all tests switch to the pack. There's no ROM or decomp access at runtime.
- **Exit:**
  - The import covers every scene and object; the counts match the XMLs and `ootx` scans.
  - Pack-rendered sheets match the ROM path.
  - Import time and pack size are recorded.
  - Tests compare every extracted table against `csrc`.
- **(As built, [GAME-01](GAME-01-foundation.md) milestone 2:)**
  - `eng_asset` is the container. `oot_game::pack` names the records. `oot_import::pack` writes them in about 9 s, into 40 MB (198 MB before compression).
  - State-dependent meshes are baked (ADR 0009):
    - rooms per game layer at a fixed time;
    - Link as 64 meshes plus a face-texture swap.
  - Running with no C at runtime meant porting the draw configs the golden renders use now: Kokiri Forest (listed under Phase 2), Hyrule Field and the Deku Tree. The others keep their frame-0 values until ported.
  - The importer depends on the game, since it builds the game's own types (ADR 0002, amended).

**Phase 2: the runtime framework**
- `PlayState`, the actor system and the frame order.
- Player, the camera, the dummy target and `Bg_Ydan_Hasi` become actors.
- Generic interpolation.
- The OPA/XLU draw submission.
- The scene and room manager: room changes, exits, spawning from actor lists with placeholders.
- Port Kokiri Forest's draw config as Rust, tested against `drawcfg`.
- **(As built, [GAME-01](GAME-01-foundation.md) milestone 3, the first half:)**
  - `PlayState` in `oot_game::play` runs the decomp's frame order (`Actor_UpdateAll`, `AnimationContext_Update`, the cameras, then `Play_Draw`'s state changes). Switching from the spikes' order changed no trace.
  - The actor system is a generational arena with the actor taken out of its slot during its update (ADR 0007).
  - Interpolation is generic: each actor captures a `RenderState`, and the renderer blends two frames.
  - Draw submission is `eng_gfx::DrawCmd`s into OPA/XLU lists, drawn through a mesh cache keyed by pack record (ADR 0006). Moving the shadow into the XLU list changed a few pixels of the course goldens.
  - Kokiri Forest's draw config was already ported in milestone 2.
- **(As built, [GAME-01](GAME-01-foundation.md) milestone 4, the second half:)**
  - Scenes are entered by `Play_Init` from an entrance (`oot_game::play_scene`). A scene change rebuilds the play state from the save context, as the game starts a new game state (ADR 0010).
  - The room and object contexts keep the game's frame timing: a room load finishes the next frame; room objects load a frame after the swap.
  - Every id spawns through `Actor_Spawn` with its `ActorInit` from the pack: ported actors by their constructor, the rest as placeholders with their real category, flags, object and room.
  - `En_Holl` changes rooms; Player's exit and void checks and start modes are ported; the fade transitions are ported frame for frame, and the others are approximated.
  - The sandbox keeps the spikes' view of a scene unless `--entrance` is given, so the goldens are unchanged.
  - Link walks into his house from the porch, since ladder climbing isn't ported. Interiors are prerendered rooms, and their backgrounds and fixed cameras are Phase 3 work.
- **Exit:**
  - Kokiri Forest loads from the pack with every actor placement either implemented or a placeholder.
  - Link walks into his house and back out, and the room and scene transitions work.
  - The movement, camera and water tests still pass through the new runtime.

**Phase 3: the Kokiri Forest vertical slice**
- The first real actors:
  - Signs, with a minimal message box from the text data.
  - `Bg_Treemouth`, doors, bushes, rocks and grass.
- Collision checks (`z_collision_check.c`), enough for the sword to cut grass and hit the dummy.
- The Normal camera's missing modes for targeting.
- A minimal HUD.
- Save context and flags.
- **Exit:** a short scripted playthrough (house → sign → cut grass → Deku Tree entrance) runs headless as a regression test.

**Later, roughly in order:**
- The audio engine, built from the extractor's sequence player and VADPCM decoder.
- Enemies and damage: Deku Baba, Skulltula.
- Items and C buttons.
- The full message system.
- Pause menu and saving.
- Cutscenes.
- The first full dungeon (the Deku Tree and Gohma).
- Then breadth: the rest of the decomp's 427 actor overlays and 36 effect overlays.
- A parallel track for custom levels (glTF → mod pack), and the decomp-free import for sharing.

## 6. Testing

- **Unit tests** for engine code: math, collision, the interpreter, pack round-trips.
- **Decomp-derived numeric tests** for game code, as in the spikes. Expected values come from C formulas and tables.
- **Scripted gameplay traces:** the headless runner feeds pad input per frame and asserts on actions, positions and animations. It also produces the contact sheets.
- **Render regression:** headless renders compared by hash or image metrics. Only the hashes and metrics are committed; the images stay in `out/`.
- **Oracles:**
  - `drawcfg` checks the ported draw configs.
  - `csrc` checks the pack's tables.
  - The ROM path checks the pack path.
- **Import validation:** asset counts against the XMLs, and 0 unknown opcodes or unresolved references beyond the documented four.

## 7. Decisions

Recommended defaults. The first session confirms or changes them with the user and records each one as an ADR.

| Decision | Recommendation | Why |
|---|---|---|
| Commit the spikes before restructuring | Yes, and tag it `spikes-complete` | Moving files over uncommitted work loses history |
| ROM version to target | Keep gc-eu-mq-dbg for development. Decide early whether the game should have the vanilla (non-MQ) dungeons, which needs a different ROM version | The debug ROM's dungeons are Master Quest. Overworld areas like Kokiri Forest are the same |
| Decomp commit | Stay on the current commit through Phase 1, then evaluate upgrading (named functions such as `Player_Action_*`, multi-version support) with a name map for the existing citations | An upgrade now would churn every citation mid-restructure |
| Rendering model | Hybrid (§3.4) | Faithful where it matters, clean where it can be |
| Actor ownership | Generational arena, take-out-during-update | Lets decomp-style cross-actor access work under Rust's borrow rules without `RefCell` everywhere |
| Pack format | Single file, index + zstd blobs + serde binary, keyed by decomp names | Simple, versionable, layerable for mods |
| Engine name | Placeholder `eng_` until the user picks one | Naming shouldn't block the work |
| Decomp-free import for end users | Later phase; keep the pack format ready for it | Not needed while the team has the decomp |

## 8. Risks

- **The long tail of actors** (427 actor overlays plus 36 effects) is most of the work. The placeholder actor, and porting in the order scenes need them, keep progress visible.
- **Borrowing.** The decomp's code freely reaches into other actors and global state. The arena and take-out pattern handles most of it; the rest needs deliberate APIs, not `unsafe`.
- **Frame-order bugs** show up only in interactions. The single documented update order and the scripted traces are the defence.
- **Pack format churn** in early phases. Version it from day one, and rebuild automatically.
- **Legal hygiene.** Keep the no-game-data rule absolute: packs, extracted folders, renders and goldens stay local, and the repo carries only code, docs, names, offsets and hashes.
