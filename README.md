# OoT Clone

A Rust reimplementation of Ocarina of Time, built from a ROM you supply with the decomp's help. The game logic is ported from the decomp function by function and tested against its C. The repo contains no game data: everything is read from your ROM on your machine.

Four spikes proved the approach (decoding, a Link viewer, Player movement, Kokiri Forest). [docs/GAME-01-foundation.md](docs/GAME-01-foundation.md) turns them into the codebase the game grows from, following [docs/ARCHITECTURE-PLAN.md](docs/ARCHITECTURE-PLAN.md). The decisions are recorded in [docs/adr/](docs/adr/README.md).

## Layout

Crates live in one folder per layer. The engine knows nothing about Zelda, and `cargo test -p layering` enforces the dependency rules ([ADR 0002](docs/adr/0002-crate-layout.md)).

| Layer | Crate | What it is |
|---|---|---|
| engine | `eng_math` | Binary angles, libultra `sins`/`coss` and `guPerspective`, `Math_Atan2S`, the `z_lib.c` step helpers, the 20 Hz update rate |
| | `eng_gfx` | The engine's mesh format: draw lists of batches sharing an N64 `Material` (combiner, othermode, tiles, render mode), per-frame segment values. No GPU code |
| | `eng_gbi` | F3DEX2 display-list interpreter and TMEM texture decoding, which build `eng_gfx` draw lists |
| | `eng_render` | wgpu renderer: CPU skinning, the RDP combiner emulated in WGSL, render-mode pipelines, fog and lights, an offscreen target with readback |
| | `eng_anim` | Skeletons (standard, LOD, flex), joint tables, animations, posing |
| | `eng_collision` | `z_bgcheck.c`: the `CollisionContext` with the static mesh, DynaPoly meshes and water boxes, returning raw surface data words |
| | `eng_input` | The N64 pad state (`padmgr`/`padutils`), and gilrs devices with per-pad profiles (the Retro-Bit N64 USB pad by raw HID code, generic XInput/SDL pads) plus a keyboard fallback |
| | `eng_audio` | The N64 audio library (`audio_*.c`, the audio thread) ported whole, and its RSP microcode: sequences, notes, envelopes, reverb; offline into buffers, or through the output device (cpal) |
| | `eng_app` | App shell: the window, an offscreen target shown in egui, the keyboard layout, PNG output |
| import | `oot_import` | Everything that reads the ROM or the decomp: dmadata, Yaz0, the XML symbol index, the z64 asset decoders, scenes and rooms, Player's draw rules, the C table reader (`csrc`), the scene draw-config interpreter (`drawcfg`), the synthetic test object |
| game | `oot_game` | Game framework from the decomp's `src/code`, no rendering: `PlayState` and its frame (`z_play.c`), entering scenes and changing rooms (`Play_Init`, the room and object contexts, exits, the transitions and the fade), spawning by id with placeholders for unported actors, the save context, the actor system and base (`z_actor.c`), render blending, Link's SkelAnime, the Normal camera (`z_camera.c`), the target context, time-of-day lights (`z_kankyo.c`), foot IK, surface types, game data, the synthetic test course |
| content | `oot_actors` | Actors from the decomp's overlays: Player (`z_player.c`: movement, ledges and climbing, Z-targeting, the sword, swimming and diving, entrances, exits and voids), `En_Holl` (the room-change planes) and `Bg_Ydan_Hasi`, the sandbox's dummy Z-target; the constructors `Actor_Spawn` uses (`overlays()`), `play_entrance` (`Play_Init`), `PlayExt` (typed access), and scripted-play helpers |
| apps | `oot` | The game: plays a scene from your ROM (Kokiri Forest by default). Its library is the play client the sandbox shares |
| | `oot_sandbox` | Dev sandbox: the test course, the debug views, and scripted headless runs (`--sheet` / `--trace` / `--screenshot`) |
| tools | `ootx` | Validation CLI: runs the decoders over the real ROM and reports statistics as JSON (no images), plus `extract`, `import` and `scene-info` (a scene's spawns, exits and placements, from the pack) |
| | `oot_viewer` | Interactive viewer for Link from your ROM and the synthetic test character, plus headless `--screenshot` / `--sheet` |
| | `oot_extract` | Asset extractor: textures → PNG, skeletons/animations/props → glTF, scenes → glTF + JSON, audio → WAV/MIDI, text → JSON, into a git-ignored folder |
| | `ootpad` | Controller probe: lists pads with their mapping profile, prints a `[pad.buttons]` table |
| | `layering` | The crate-layering check (a test) |

## Setup

1. Copy `oot.example.toml` to `oot.toml` and point it at your ROM and decomp checkout: zeldaret/oot at commit `52a510f` ([ADR 0031](docs/adr/0031-decomp-main.md)), the ROM gc-eu-mq-dbg (already done on this machine; `oot.toml` is git-ignored).
2. `rust-toolchain.toml` pins Rust 1.95 for this directory only (egui 0.36 needs it). Your global default is untouched.
3. Build the asset pack once: `target/release/oot import` (about 9 s). The game launches it by itself if there's no pack.

## The asset pack

The game never reads the ROM or the decomp at runtime: `oot import` decodes the ROM once, with the decomp's help, into an asset pack ([ADR 0008](docs/adr/0008-asset-pack.md)). The pack holds:
- every texture, skeleton, animation, display list and collision header the decomp's XMLs name;
- every scene and room, for each of the four game layers;
- Link's meshes;
- the game tables the C defines, including the entrance table and the actor table with every actor's `ActorProfile`.

The pack goes to `%LOCALAPPDATA%\oot-clone\packs\<rom-sha1>.pak` (about 40 MB). Like `extracted/`, it's Nintendo's data and stays on your machine.

- `oot import --loose <dir>` writes the same records as loose files instead (the dev mode); play or test with `--pack <dir>` or `OOT_PACK=<dir>`.
- `OOT_PACK` points the game and the tests at another pack; `OOT_DATA_DIR` moves the whole data folder.
- A pack from an older importer is stale: the game re-imports the default one by itself.
- `out/import_manifest.json` (from `ootx import`) lists what the import covered and why anything was left out.

## Commands

On Windows, [scripts/run/](scripts/run/README.md) has batch files for the common ones, with the current build and pack folders set in one place. `scripts\run\menu.bat` lists them.

```sh
cargo build --release

# The asset pack
target/release/oot import                     # from oot.toml's ROM and decomp (or --rom / --decomp)
target/release/oot import --loose out/pack    # loose records instead, for inspecting and diffing
target/release/ootx import                    # the same, plus the manifest in out/import_manifest.json

# Validation against your ROM (numbers + JSON in out/)
target/release/ootx info
target/release/ootx player-anims --object object_link_boy
target/release/ootx player-anims --object object_link_child --root-scale 0.64
target/release/ootx scan-skeletons            # every skeleton + animation in the decomp XMLs
target/release/ootx player-draw               # Link's draw list for every age x model group x shield
target/release/ootx scan-scenes [--all-layers]   # every scene's rooms through the interpreter (out/scene_scan.json)
target/release/ootx dump-room --scene spot04 --room 0 --png out/s04_tex   # one room's draw lists and textures
target/release/ootx scene-info --scene spot04 [--layer 1]              # spawns, entrances, exits, transition actors, placements (from the pack)
target/release/ootx cutscene [gDekuTreeMeetingCs]                             # the pack's cutscene scripts and sEntranceCutsceneTable, or one script's commands

# Viewer: Link from your ROM (default) or the synthetic test character "Tock"
target/release/oot_viewer                                   # interactive, starts on adult Link
target/release/oot_viewer --age child
target/release/oot_viewer --screenshot out/link.png --anim link_normal_walk --frame 6
target/release/oot_viewer --sheet out/link_sheet.png        # rows: wait, walk, run, slash, jump
target/release/oot_viewer --sheet out/sword.png --group SWORD --shield HYLIAN --anims link_fighter_normal_kiru,link_fighter_defense_wait
target/release/oot_viewer --subject tock --sheet out/tock_sheet.png

# The game: enters Kokiri Forest by Play_Init (ENTR_KOKIRI_FOREST_0), child Link (N64 pad or keyboard).
# Every placement spawns (ported, or a placeholder: P shows them), rooms change, exits work.
target/release/oot
target/release/oot --entrance ENTR_KOKIRI_FOREST_3                # outside Link's house: walk in through the door
target/release/oot --scene spot00 --adult --time 18:00     # any scene's entrance to spawn --spawn N, age and time
target/release/oot --entrance ENTR_KOKIRI_FOREST_1 --preset deku-tree-open   # at the Deku Tree, his mouth open (a debug save preset;
                                                           # deku-tree-dead: the tree dead too)
target/release/oot --entrance ENTR_LINKS_HOUSE_0             # a new save in Link's bed: the ramp, the crawlspace, the boulder, the
                                                           # Kokiri Sword's chest, then 40 rupees, the shop and Mido
target/release/oot --entrance ENTR_KOKIRI_FOREST_4 --preset sword-and-40-rupees   # outside the Kokiri shop with the sword worn and 40
                                                           # rupees: buy the Deku Shield, Enter (Start) twice to wear it
                                                           # (the pause menu's closing equips it)

# Sandbox: the test course and the debug views
target/release/oot_sandbox                                 # test course, adult Link
target/release/oot_sandbox --scene spot04 --child          # Kokiri Forest, the spikes' view (every room, Player alone)
target/release/oot_sandbox --scene spot04 --child --entrance --placeholders   # the game's way in, with markers on unported actors
target/release/oot_sandbox --entrance ENTR_KOKIRI_FOREST_3 --child --script house --trace out/house.json --screenshot out/house.png --shots-at 12,60,120
                                                           # headless: into Link's house and back out
target/release/oot_sandbox --entrance ENTR_LINKS_HOUSE_0 --child --preset deku-tree-open --script playthrough --trace out/playthrough.json
                                                           # headless: GAME-02's run from Link's bed into the Deku Tree
target/release/oot_sandbox --entrance ENTR_LINKS_HOUSE_0 --child --script sword-chest --trace out/sword_chest.json
                                                           # headless: GAME-03's run from Link's bed to the Kokiri Sword
target/release/oot_sandbox --entrance ENTR_LINKS_HOUSE_0 --child --script mido-shop --trace out/mido_shop.json
                                                           # headless: on to the Deku Shield from the shop, both worn, past Mido
target/release/oot_sandbox --entrance ENTR_LINKS_HOUSE_0 --child --script new-save-deku-tree --trace out/new_save_deku_tree.json
                                                           # headless: on past Mido, the Deku Tree's talk (cutscenes) and into him
target/release/oot_sandbox --entrance ENTR_LINKS_HOUSE_1 --child --script cup --screenshot out/home.png --shots-at 29
                                                           # the house's pivot camera and skybox, then C-Up: the fixed camera and its picture
target/release/oot_sandbox --entrance ENTR_LON_LON_BUILDINGS_2 --child --at=1190,140,150,16384 --script open --sheet out/door.png
                                                           # A at a door: En_Door, the door camera, the room behind it
target/release/oot_sandbox --scene spot04 --time 19:00 --target 200   # evening, with a dummy Z-target
target/release/oot_sandbox --script swim --step 8 --sheet out/swim.png   # also: platform, target, parallel, sword, hang, climb50/70/100, tour
target/release/oot_sandbox --script run-roll --sheet out/run_roll.png --trace out/run_roll.json
target/release/ootpad list                                 # controllers, ids, mapping profile
target/release/ootpad calibrate                            # prints a [pad.buttons] table for oot.toml

# Tests (the game's tests read the pack and skip without one; the importer's need oot.toml too)
cargo test --workspace
cargo test -p oot_actors                                   # movement, camera, foot IK, ledges, targeting, sword, water, platform, scenes
cargo test -p oot_actors --test scenes                     # Kokiri's placements, the En_Holl room change, Link's house and back, every scene entering
cargo test -p oot_actors --test prerendered --test door    # the interiors' cameras, backgrounds and skyboxes; En_Door
cargo test -p oot_actors --test playthrough                # Bg_Treemouth, and the scripted runs from Link's bed (the Deku Tree, the Kokiri Sword)
cargo test -p oot_actors --test crawl --test boulder       # the crawlspace and its camera; the rolling boulder and Link's knockdown
cargo test -p oot_game                                     # Kokiri Forest's rooms, draw configs and environment
cargo test -p oot_import --test pack                       # the pack against the ROM path: tables vs the C, draw configs vs the interpreter, meshes, counts, the scene lists
cargo test -p layering                                     # the crate layering rules

# Render regression: every sandbox script and viewer sheet, hashed against golden/renders.sha256
python scripts/golden.py check --bin-dir target/release
```

Concurrent sessions build into their own folder: `CARGO_TARGET_DIR=target/<name> cargo build --release`.

Play controls: N64 stick to move, A to roll / jump / dive, B for the sword, Z to target, C-left/C-right to turn the follow camera. On keyboard: WASD/arrows, Shift to walk, Space for A, E for B, Q for Z, J/L for C-left/right. F1 toggles the collision wireframe, F2 the HUD, F3 switches between the game camera and spike 03's follow camera, F4 toggles foot IK, P the placeholder markers, Tab switches age (entering again), Backspace respawns (in a scene entered by an entrance: `Play_TriggerVoidOut`, back to where Link came in). Enter (Start) opens the pause menu (its frame and item page; ADR 0047): WASD moves the cursor, J/K/L equip the item under it on C-Left/Down/Right, R and Q (R and Z) turn the pages, Enter closes it. Closing it equips every owned piece of a type with nothing worn, the sword also on B (the equipment page's stand-in; ADR 0019). At a crawlspace's mouth A says Enter: the stick forward crawls, back backs out.

Viewer controls: left-drag to orbit, right-drag to pan, scroll to zoom. The sidebar switches between Link and Tock and has the animation list (with a filter for Link's 573), playback, frame scrub and interpolation. For Link it adds age, model group, shield, tunic, running fists, LOD, and eye/mouth overrides; for Tock, face and emblem colour. Both have a skeleton overlay and per-material combiner/texture details. Rendered images go to `out/`, which is git-ignored.

## Extracted assets (local only)

```sh
target/release/ootx extract                          # everything, into extracted/
target/release/ootx extract --only textures,models   # parts: raw, textures, models, scenes, audio, text
```

This writes editable copies of the game's assets to `extracted/`: PNG textures, glTF models with their animations, scenes with collision and actor data, WAV samples and sequences, and message text. The folder is git-ignored and contains Nintendo's data derived from your ROM, so keep it local and never commit or share it. `extracted/README.md` explains the layout, and [docs/ASSET-EXTRACTION.md](docs/ASSET-EXTRACTION.md) has what's covered, how it was checked, and the plan for custom levels.

See [docs/SPIKE-01-findings.md](docs/SPIKE-01-findings.md), [docs/SPIKE-02-link-viewer.md](docs/SPIKE-02-link-viewer.md), [docs/SPIKE-03-player-movement.md](docs/SPIKE-03-player-movement.md) and [docs/SPIKE-04-kokiri-forest.md](docs/SPIKE-04-kokiri-forest.md) for the spikes' results. They name the crates as they were then (`oot_core`, `oot_play`, …); [ADR 0002](docs/adr/0002-crate-layout.md) maps them to the new ones.
