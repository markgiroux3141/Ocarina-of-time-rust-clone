# 0062: The aim: the whole game, Master Quest throughout, captured for the game editor

- **Status:** accepted (2026-10-13)
- **Date:** 2026-10-13
- **Builds on:** [ADR 0028](0028-phase-6-master-quest-and-the-decomp-upgrade.md) (Master Quest's
  Deku Tree), [ADR 0035](0035-custom-levels-from-the-overworld-editor.md) (custom levels from the
  overworld editor).

## Context

Eight phases in (2026-10-13), the roadmap plans milestone by milestone only to the end of Phase 8
(Castle Town, Hyrule Castle and Kakariko). Beyond that it had a rough list. About a quarter of the
game's 429 actor overlays are ported. The user set out the aim and two standing calls:

- **The aim:** replicate the game, but above all capture every game object and dynamic, so that
  we can make our own version of the game with the editor.
- **Master Quest the whole way:** ADR 0028 chose the debug ROM's Master Quest dungeons for the
  Deku Tree. That now holds for every dungeon.
- **The editor is the game editor:** the "overworld editor" (`crates/tools/overworld`,
  `crates/tools/overworld_editor`) will in time edit every part of the game. Its name undersells
  it.

## Decision

- **Two aims, in order of what they serve.** The port stays faithful (the C's functions, constants
  and bugs; tests from the C), because that is how the dynamics are captured exactly. The faithful
  port is the means. What the work is for is a game whose every object and dynamic is something
  the editor can place, configure and play. What "captured" means:
  - **Data:** every scene, room, collision mesh, path, camera, cutscene, text, sound and object
    is a pack record ([ADR 0008](0008-asset-pack.md); the runtime reads only the pack). An editor that writes
    those records can make anything the game is made of.
  - **Behaviour:** every actor is ported whole and faithfully, in code. The game's tables stay
    tables (imported, as the song tables and the player's rules are).
  - **Configuration:** what an actor takes from where it's placed (its params' bit fields, the
    rotation fields some use as settings, the switch, chest, collectible and event flags it reads
    and sets, the paths and objects it needs) is written down in a fixed form, so the editor can
    show it as named fields. From now on, each new actor's module docs end with a **Params**
    paragraph and a **Flags** paragraph in that form (below). The 104 actors already ported get
    theirs in one pass when the editor's actors tab is built (the game editor track's stage E3,
    ROADMAP.md).
- **Master Quest throughout** (extends ADR 0028). Every dungeon is the debug ROM's (gc-eu-mq-dbg),
  with no second ROM and no vanilla layouts. The overworld is the same in both.
- **"The game editor."** Docs call the editor the game editor from now on. The overworld builder
  is its first mode, with the roofs, storeys and dungeons modes
  ([EDITOR-ROOFS-STOREYS-DUNGEONS.md](../EDITOR-ROOFS-STOREYS-DUNGEONS.md)) next. The crates keep
  their names for now (renaming them would touch every path and script for nothing). If they're
  renamed, it's done as a change of its own.
- **The whole game outlined** in ROADMAP.md ("The whole game"): Phases 9 to 20 in the story's
  order, each with its scenes, dungeon and boss, the systems it brings and its exit. Each phase is
  still surveyed and split into milestones when it starts, as before; the outline is the order and
  the scope, not the split.
- **Coverage kept, not guessed:** `docs/COVERAGE.md` is generated (`scripts/run/coverage.bat`:
  `ootx census` and `scripts/coverage.py`). It lists every scene layer's actors by phase against
  what `oot_actors` ports, the ported actors whose docs note gaps, and the actors only something
  else spawns. The phases' scenes live in the script's `SCENES` table, so the outline and the page
  can't drift apart. Each milestone ends by refreshing it.
- **A milestone's definition of done** is written into ROADMAP.md ("How a milestone is done"):
  the standing rules, with the coverage page and the Params and Flags paragraphs added.

### The Params and Flags paragraphs

At the end of an actor's module docs (`//!`), in this form:

```text
//! Params: `type` bits 10 to 15 (7: a song's check; 1, 2, 4, 6: a song's cutscene; 5: the
//! royal tomb's wall); `song` bits 6 to 9 (0 to 5 Saria's to the storms, 6 the scarecrow's, 0xF
//! any); `switch_flag` bits 0 to 5 (0x3F: none). `world.rot.z`: the range, 40 a unit.
//! Flags: reads and sets the switch flag `switch_flag`; reads EVENTCHKINF_OPENED_DOOR_OF_TIME
//! (type 4), EVENTCHKINF_1D (type 6).
```

Each field is named as the C's `PARAMS_GET_*` or its local reads name it, with its bits and what
its values mean. A field the editor can't usefully set (a debug-only value) is still listed, and
marked "debug". The paragraphs are prose that a script can lift: `name` bits a to b (values).

## Consequences

- Each phase's scope is known ahead: COVERAGE.md says which actors a phase's scenes place and
  which are already ported, and the ROADMAP's outline says which systems come with it.
- The editor work stays its own track, in stages (ROADMAP.md, "The game editor track"), driven by
  the user's priorities. The game phases feed it: every actor ported is one the actors tab can
  offer.
- The actors ported before this ADR have their configuration in prose. One pass with the actors
  tab writes their Params and Flags paragraphs (BACKLOG #52).
  - **Since [ADR 0066](0066-the-actor-schema.md) (2026-10-14):** the configuration is also data,
    a schema per actor that its code reads through and a test checks against the code and the
    game's data. Each new actor comes with both the paragraphs and the schema, and #52's pass
    was done for all 131 actors then, ahead of the actors tab.
- The outline will be wrong in places (a story beat's actor first met earlier than planned, a
  system bigger than it looks). The survey at each phase's start corrects it, and the corrections
  go into the `SCENES` table and the outline together.
