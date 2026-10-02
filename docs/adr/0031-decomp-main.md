# 0031: Decomp commit: zeldaret/oot main at `52a510f`, through a name map

- **Status:** accepted, built in GAME-05 milestone 1 (2026-10-02)
- **Date:** 2026-10-02
- **Supersedes:** [ADR 0004](0004-decomp-commit.md)'s pin on `2f4c25d`. Carries out
  [ADR 0028](0028-phase-6-master-quest-and-the-decomp-upgrade.md)'s upgrade.

## Context

ADR 0004 pinned the decomp at `2f4c25d` (2022-10-04) until the asset pack existed, and set what
an upgrade must come with: a name map from the two commits' symbol files, the importer's C
readers fixed, every test and golden unchanged. ADR 0028 made the upgrade Phase 6's first
milestone, so the Deku Tree's ports could cite the decomp's current names
(`Player_Action_*`, the camera's, the dungeon actors').

The checkout is zeldaret/oot's main at
`52a510f379afd143aaa0375be9f1e190369572e1` (2026-09-30), cloned to `D:/OOT Modding/oot-main`.
The old checkout (`D:/OOT Modding/OTT decomp/z64oot`) stays as it is. Both build the user's ROM,
gc-eu-mq-dbg, byte for byte.

Main is laid out differently from `2f4c25d`:
- **Versions.** It builds several ROMs. The version's files are under `baseroms/gc-eu-mq-dbg/`:
  `config.yml` (the asset XMLs with their offsets, the tables' addresses in `code`, the binary
  blobs such as `aspMainData`), `segments.csv` (the ROM's files). The C takes the version's
  `#define`s (`OOT_VERSION`, `PLATFORM_N64`...). The XMLs have `<Version Pattern>` blocks.
- **Headers.** `z64.h` and the `z64*.h` headers are split and renamed (`actor.h`, `camera.h`,
  `player.h`, `skybox.h`, `transition.h`...).
- **Tables.** More tables are generated from `DEFINE_*` rows (the sequences, the sound effects,
  their parameters, the actor flags).
- **XMLs.** Offsets can be implied by the elements' sizes or relative. Overlay offsets are relative
  to the overlay's start. Palettes are named (`Tlut="name"`).
- **Names.** Many are new or changed: most of Player's and the camera's functions, `D_` statics,
  `unk_` fields, enums and `#define`s.

## Decision

- **Pin `52a510f`.** `oot.toml`'s `decomp` points at the new checkout. The pack's header
  records the commit it was read from (`decomp_commit`, as ADR 0004 asked).
- **A generated name map, kept in the repo** (`docs/name-map/`, names and addresses only):
  - built by `scripts/name_map.py` from the two builds' ELFs and the two checkouts' C;
  - each kind paired by what both commits share: functions and data by address (IDO's statics
    from `.mdebug`), fields by offset, enum members by value, `#define`s by family and value,
    files by their symbols;
  - a short table of hand-checked successors (`manual.tsv`) for what can't be paired
    mechanically;
  - the unpaired names listed (`unpaired.tsv`) and reviewed (`docs/name-map/README.md`).
  `scripts\run\name-map.bat` rebuilds both decomps in Docker (each with its own Dockerfile, in
  a volume, so the checkouts are only read) and regenerates it.
- **The citations follow the map.**
  - Comments, strings, tests and docs take the new C names verbatim.
  - The Rust identifiers named after C take the new names in Rust case:
    - functions in snake case, without the module's own prefix (`Player_` in `player.rs`);
    - statics in screaming snake case (`S_ACTION_HANDLER_LIST_IDLE`);
    - fields after the new member, with the unknown parts kept (`unk_6AE_rot_flags`).
  - The phase docs keep their history but cite code by the new names.
- **The importer reads main's layout.**
  - The symbol index follows `config.yml` and the XMLs' version blocks and implied offsets.
  - The C is preprocessed for gc-eu-mq-dbg (its `#if`s), with a macro evaluator for the tables
    built from `DEFINE_*` rows and from expressions.
  - The tables `2f4c25d` had in assembly are read from the ROM, at `config.yml`'s addresses.
- **Pack format 16.** The record names that came from decomp names that changed are renamed;
  among them, the cutscene layers' scripts are keyed by the XMLs' names (main names all of them)
  where they were keyed by offset (ADR 0023's `keys::cutscene_at`, kept for a script no XML
  names). The records' contents are unchanged except where main's C or XMLs correct the old
  ones (docs/GAME-05-deku-tree.md, milestone 1).

## Consequences

- New ports cite main's names. Look an old citation up with `python scripts/name_map.py find
  NAME`.
- A pack from `2f4c25d` is format 15 and isn't read; `oot import` builds format 16 from main.
- The goldens' traces carry C names (Player's action, the cutscene script, the entrance), so
  they were re-recorded. Each new trace is the old one's bytes once renamed through the map
  (`python scripts/name_map.py compare-trace`), and every render is the same bytes
  (golden/README.md).
- A later upgrade does the same: build both commits, regenerate the map, migrate the citations
  by it, and compare the packs (`scripts\run\decomp-check.bat`) and the traces.
