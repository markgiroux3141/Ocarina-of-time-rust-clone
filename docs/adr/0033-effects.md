# 0033: The effects

- **Status:** accepted, built in GAME-05 milestone 3a (2026-10-05)
- **Date:** 2026-10-05
- **Builds on:** [ADR 0012](0012-actor-bakes.md) (actor bakes),
  [ADR 0027](0027-sound-effects.md) (sound sources as named positions) and
  [ADR 0032](0032-damage-death-and-the-game-over-stand-in.md) (Player's `Rand` calls through
  `PlayIo`).

## Context

Milestone 3a needed the game's effects:
- `z_effect_soft_sprite.c`, the soft sprites (`EffectSs`): a table of 85 small effects, each run
  by its overlay's init, update and draw;
- `z_effect.c`'s three other kinds (`Effect`): the sparks, the sword's trails and the shield's
  particles.

The Deku Baba (ported whole but its effects in milestone 2), the withered Deku Baba, the Keese,
Player's burning and shock, the collision check's hit marks and blood all spawn them. Five
things needed a decision:
- **Where they live and when they run.** The C keeps `sEffectSsInfo` as a static, and runs the
  updates in `Play_Update` and the draws at the end of `Actor_DrawAll`.
- **Their draws change game state.** Some draws call `Rand` (a spark's sizes), and some change
  the effect (`Effect_Ss_Fire_Tail` moves to Link's body parts). The renderer draws blended
  states between game frames, and its draws must not touch the game.
- **Their graphics.**
  - Most draw a texture with a setup display list and colours that change every frame.
  - The sparks build their vertices every frame (`GRAPH_ALLOC`'d `Vtx` with the frame's
    colours).
  - One display list lives in an overlay, not an object: `Effect_Ss_Fhg_Flash`'s `sShockDL`,
    with KSEG0 pointers into the overlay.
- **Which overlays.** There are 37 soft sprite types; this milestone's callers need eight.
- **Spawning from Player**, who is out of the actor arena during his own update.

## Decision

- **The effects are play state** (`oot_game::effect`): `PlayState::effect_ss`, the table with
  its `searchStartIndex`, and `PlayState::effect_ctx` for `z_effect.c`'s.
  - `EffectSs_UpdateAll` and `Effect_UpdateAll` run where `Play_Update` runs them, after the
    actors and the cutscenes.
  - An effect's `update` and `draw` are enums of the ported overlays' functions (`SsUpdate`,
    `SsDraw`), and its `regs` keep the overlays' names.
  - `EffectSs_Spawn`, `EffectSs_FindSlot` (priorities) and the
    `z_effect_soft_sprite_old_init.c` helpers are ported with their names, through `SsSpawn`,
    which borrows the table, the game's `Rand` and the object banks.
- **The draws run once per game frame,** where `Actor_DrawAll` ends, in `PlayState::tick_with`
  after the actors' draw-time state.
  - They make their `Rand` calls and changes then, in the C's order.
  - Their `DrawCmd`s are kept (`PlayState::effect_draws`) and appended after the actors' by
    `PlayState::draw`, unblended.
  - An effect drawn at 20 Hz is what the console shows.
- **The graphics are bakes** (ADR 0012), one per texture and setup an overlay draws with. The
  colours are dynamic segment values (`SegmentValues`' prim and env on segment 0xE).
  - **Per-vertex colours:** the engine takes a draw's vertex colours (`DrawParams::vertex_colors`,
    the mesh's batched triangle-list order), for the sparks.
  - **Overlay display lists:** the importer binds a bake's overlay file (`ovl_*`) to segment 0
    as RAM holds it (zeros up to its link address's low 24 bits, then the file, from
    `segments.csv`'s VRAM start).
- **Eight soft sprite overlays are ported, the ones called:** `Dust`, `Hahen`, `HitMark`,
  `En_Fire`, `En_Ice`, `Dead_Db`, `Fire_Tail` and `Fhg_Flash` (its shock). From `z_effect.c`,
  the sparks and the shield particles (with their point light).
  - Spawning another type logs it and does nothing, as the C does for a type with no init.
  - `EffectBlure` (the sword's trail) finds no slot. Its `Rand`-free draw is a later polish.
- **Player spawns through `PlayIo`** (ADR 0032), which lends him the table, `Rand` and the
  stop list.
- **Sounds at an effect's position:** `SfxPos::EffectSsPos(index)` and `EffectSsVec(index)` (ADR
  0027's named sources). A slot reused or deleted stops its sounds (the C's pointer goes stale);
  the stops are flushed after each spawn.

## Consequences

- Every `Rand` call the effects make is now made, in the C's order. That moves the draws of
  everything after them. With ADR 0034's wall recoil, the Kokiri playthrough's bushes needed a
  new order to get a drop (its documented risk).
- The effects' draws can't be interpolated between game frames. The console doesn't do it either.
- A new effect type is an overlay module, its `SsUpdate`/`SsDraw` arms and its bakes. The
  pack's format changes when bakes are added.
- Pack format 18 (`out/data15`), together with ADR 0034's held-shield variants.
