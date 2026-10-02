# 0008: Asset pack: one versioned file per ROM, keyed by decomp names

- **Status:** accepted, built in GAME-01 milestone 2
- **Date:** 2026-09-27

## Context

The spikes read everything at startup: the ROM through the decomp's XML offsets, and game tables by parsing the decomp's C (`csrc`, `drawcfg`). That's fine for exploring, but not for a game.
- The runtime would depend on a decomp checkout.
- Every launch re-decodes and re-parses.
- Mods have no way in.

## Decision

- **An importer, run once:** `oot import` (and automatically on first launch) decodes the ROM, interprets the static display lists, extracts the C tables, validates, and writes a pack.
- **Where:** in the per-user data folder, `%LOCALAPPDATA%\oot-clone\packs\<rom-sha1>.pak` on Windows, `$XDG_DATA_HOME/oot-clone/packs` elsewhere. Never in the repo.
  - `packs/default` names the SHA-1 of the pack the game uses (the last import).
  - `OOT_PACK` points the game and the tests at another pack file or loose folder.
  - `OOT_DATA_DIR` moves the whole data folder.
- **What:** engine-native data that's ready to use, with no N64 decoding at runtime:
  - meshes (`eng_gfx::DrawList`, with their dynamic-segment markers) and decoded textures;
  - skeletons and animations (standard and Player's);
  - collision headers;
  - scenes and rooms as typed structs;
  - every game table the code reads (`GameData`, the camera data, the environment's light configs, items, Player's draw rules).
- **How:** a crate, `eng_asset`, that knows no record types:
  - The file is `ASSETPAK`, a bincode header (format version, importer and importer version, ROM SHA-1, facts such as the decomp commit), a zstd-compressed index, then zstd-compressed bincode blobs.
  - Records are keyed by `kind/<file>/<symbol>` with the decomp's names (`mesh/object_ydan_objects/gDTSlidingPlatformDL`, `room/spot04_scene/0/1`). The game's list of names is `oot_game::pack::keys`, shared by the importer and the game.
- **A stale pack is rebuilt:** a different format version, importer version or ROM hash means re-import. `GamePack::open` refuses a stale pack. The client re-imports the default pack by itself, but never one named by `OOT_PACK` or `--pack`.
- **Dev mode:** the same records as loose files in a folder (`oot import --loose <dir>`: `header.json` plus one uncompressed `<name>.bin` per record). They open the same way.
- **Mods and custom levels:** `eng_asset::Assets` layers sources by name. A mod pack's records override the base pack's.
- **Decomp-free import for end users** is a later phase. The format already allows it: the pack has no dependency on how the importer found an asset.

### Refinements made while building it

- **Identical records are stored once:** they're matched on content, and every name points to the same blob. Blobs are laid out in the order of the sorted names, so two imports of the same ROM give the same records byte for byte. Only the header's import time and the manifest's timings differ.
- **Types that serialize maps use `BTreeMap`:** a `HashMap` iterates in a random order per map, which made identical draw lists serialize differently.
- **The pack also holds a manifest** (`meta/manifest`): for every kind of XML element, how many are listed, how many are in the pack, and why the rest aren't. It also has the scene statistics `ootx scan-scenes` reports, and the import's timings. The tests compare it with the XMLs and the scans.
- **State-dependent meshes** (room meshes per scene layer, Link's face) are baked as ADR 0009 describes.

## Consequences

- The runtime depends on `eng_asset` and never on `oot_import`. The layering test enforces this (ADR 0002). The game runs with no `oot.toml`, no ROM and no decomp, as long as the pack exists.
- `csrc` and `drawcfg` are now import tools and test oracles. `oot_import`'s `tests/pack.rs` compares every table with `csrc`, each ported draw config with `drawcfg`, and the meshes with the ROM path.
- The pack is Nintendo's data. Like `extracted/`, it stays on the machine that made it.
- Sizes for gc-eu-mq-dbg:

  | | |
  |---|---|
  | Pack file | 40.4 MB |
  | Records | 10,838 |
  | Distinct blobs | 8,831 |
  | Before compression | 197.8 MB |
  | Loose folder | 373 MB |
  | Import | about 9 s on this machine |

## Amendment (milestone 4)

Format version 2:
- `table/actors` (`oot_game::actor_table::ActorTable`: the actor table with every `ActorProfile`);
- the entrance table in `table/scenes`;
- per scene layer the entrance list, exit list, transition actors and the keep object's id;
- per room its object list.

A format-1 pack is stale and is rebuilt.
