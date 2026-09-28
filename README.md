# OoT Clone: decoding spikes

Rust spikes testing whether Ocarina of Time's models, materials, animations and (later) game logic can be pulled from a ROM with the decomp's help. The repo contains no game data; everything is read at runtime from a ROM you supply.

## Layout

| Crate | What it is |
|---|---|
| `crates/oot_core` | ROM reader (byte order, dmadata, Yaz0), decomp XML symbol index, F3DEX2 display-list interpreter, TMEM/texture decoding, colour-combiner decode, flex/LOD skeletons, standard + Link animation formats, Player draw rules read from decomp C (`player.rs`, `csrc.rs`), and a synthetic test object (`synth.rs`) |
| `crates/oot_render` | wgpu renderer for decoded draw lists: CPU skinning, RDP combiner emulated in WGSL, render-mode pipeline variants, offscreen target with readback |
| `crates/oot_viewer` | Interactive viewer (eframe) for Link from your ROM and the synthetic test character, plus headless `--screenshot` / `--sheet` modes |
| `crates/ootx` | Validation CLI: runs the decoders over the real ROM and reports statistics as JSON (no images), plus `extract` |
| `crates/oot_extract` | Asset extractor: textures → PNG, skeletons/animations/props → glTF, scenes → glTF + JSON, audio → WAV/MIDI, text → JSON, into a git-ignored folder |
| `crates/oot_game` | Game logic ported from the decomp, no rendering: N64 pad input (`padmgr`/`padutils`), Player (`z_player.c`: movement, ledges and climbing, Z-targeting, the sword, swimming and diving), Link's SkelAnime and animation queue, foot IK (`func_8008F87C`), actor physics (`z_actor.c`), bgcheck with water boxes and DynaPoly (`z_bgcheck.c`, `dyna.rs`), a moving platform (`Bg_Ydan_Hasi`), the Normal camera (`z_camera.c`), the target context, time-of-day lights (`z_kankyo.c`), the synthetic test course, and a 20 Hz world with interpolation snapshots |
| `crates/oot_pad` | Controllers mapped to an N64 pad: gilrs devices with per-pad profiles (the Retro-Bit N64 USB pad by raw HID code, generic XInput/SDL pads), a keyboard fallback, and the `ootpad` probe |
| `crates/oot_play` | Playable spikes 03–04: Link from your ROM in the test course or a real scene drawn from the ROM (`--scene spot04`: rooms, animated textures, lights and fog), with the game camera, targeting, the sword, swimming and a moving platform, plus headless `--sheet` / `--trace` / `--screenshot` for scripted runs |

`oot_core` also has `collision.rs` (`CollisionHeader` decode/encode), `scene.rs` (scene header commands), `room.rs` (room shapes, segment binding, scene drawing) and `drawcfg.rs` (the scene draw-config interpreter, read from `z_scene_table.c`).

## Setup

1. Copy `oot.example.toml` to `oot.toml` and point it at your ROM and decomp checkout (already done on this machine; `oot.toml` is git-ignored).
2. `rust-toolchain.toml` pins Rust 1.95 for this directory only (egui 0.36 needs it). Your global default is untouched.

## Commands

```sh
cargo build --release

# Validation against your ROM (numbers + JSON in out/)
target/release/ootx info
target/release/ootx player-anims --object object_link_boy
target/release/ootx player-anims --object object_link_child --root-scale 0.64
target/release/ootx scan-skeletons            # every skeleton + animation in the decomp XMLs
target/release/ootx player-draw               # Link's draw list for every age x model group x shield
target/release/ootx scan-scenes [--all-layers]   # every scene's rooms through the interpreter (out/scene_scan.json)
target/release/ootx dump-room --scene spot04 --room 0 --png out/s04_tex   # one room's draw lists and textures

# Viewer: Link from your ROM (default) or the synthetic test character "Tock"
target/release/oot_viewer                                   # interactive, starts on adult Link
target/release/oot_viewer --age child
target/release/oot_viewer --screenshot out/link.png --anim link_normal_walk --frame 6
target/release/oot_viewer --sheet out/link_sheet.png        # rows: wait, walk, run, slash, jump
target/release/oot_viewer --sheet out/sword.png --group SWORD --shield HYLIAN --anims link_fighter_normal_kiru,link_fighter_defense_wait
target/release/oot_viewer --subject tock --sheet out/tock_sheet.png

cargo test -p oot_core

# Play: Link with Player movement ported from the decomp (N64 pad or keyboard)
target/release/oot_play                                    # test course, adult Link
target/release/oot_play --scene spot04 --child             # Kokiri Forest from the ROM (rooms, lights, fog, collision)
target/release/oot_play --scene spot04 --time 19:00 --target 200   # evening, with a dummy Z-target
target/release/oot_play --script swim --step 8 --sheet out/swim.png   # also: platform, target, parallel, sword, hang, climb50/70/100, tour
target/release/oot_play --script run-roll --sheet out/run_roll.png --trace out/run_roll.json
target/release/ootpad list                                 # controllers, ids, mapping profile
target/release/ootpad calibrate                            # prints a [pad.buttons] table for oot.toml

cargo test -p oot_game                                     # movement, camera, foot IK, ledges, targeting, sword, water, platform, scene
```

Play controls: N64 stick to move, A to roll / jump / dive, B for the sword, Z to target, C-left/C-right to turn the follow camera. On keyboard: WASD/arrows, Shift to walk, Space for A, E for B, Q for Z, J/L for C-left/right. F1 toggles the collision wireframe, F2 the HUD, F3 switches between the game camera and spike 03's follow camera, F4 toggles foot IK, Tab switches age, Backspace respawns.

Viewer controls: left-drag to orbit, right-drag to pan, scroll to zoom. The sidebar switches between Link and Tock and has the animation list (with a filter for Link's 573), playback, frame scrub and interpolation. For Link it adds age, model group, shield, tunic, running fists, LOD, and eye/mouth overrides; for Tock, face and emblem colour. Both have a skeleton overlay and per-material combiner/texture details. Rendered images go to `out/`, which is git-ignored.

## Extracted assets (local only)

```sh
target/release/ootx extract                          # everything, into extracted/
target/release/ootx extract --only textures,models   # parts: raw, textures, models, scenes, audio, text
```

This writes editable copies of the game's assets to `extracted/`: PNG textures, glTF models with their animations, scenes with collision and actor data, WAV samples and sequences, and message text. The folder is git-ignored and contains Nintendo's data derived from your ROM, so keep it local and never commit or share it. `extracted/README.md` explains the layout, and [docs/ASSET-EXTRACTION.md](docs/ASSET-EXTRACTION.md) has what's covered, how it was checked, and the plan for custom levels.

See [docs/SPIKE-01-findings.md](docs/SPIKE-01-findings.md), [docs/SPIKE-02-link-viewer.md](docs/SPIKE-02-link-viewer.md), [docs/SPIKE-03-player-movement.md](docs/SPIKE-03-player-movement.md) and [docs/SPIKE-04-kokiri-forest.md](docs/SPIKE-04-kokiri-forest.md) for results.
