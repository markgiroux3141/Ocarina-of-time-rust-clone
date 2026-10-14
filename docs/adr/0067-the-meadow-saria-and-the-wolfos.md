# 0067: The meadow: Saria whole (her Kokiri Forest greeting too), the Wolfos, and the route through the woods

- **Status:** accepted, built in GAME-08 milestone 1 (2026-10-14)
- **Date:** 2026-10-14
- **Builds on:** [ADR 0066](0066-the-actor-schema.md) (each actor's schema), [ADR 0065](0065-the-graveyard-the-royal-tomb-and-the-suns-song.md)
  (the helpers' worktrees), [ADR 0058](0058-the-culling-volume.md) (the culling volume).

## Context

Phase 9 (GAME-08) starts with its exit: child Link with Zelda's letter through the Lost Woods to
the Sacred Forest Meadow, past its Wolfos and its maze, to Saria, who teaches Saria's Song. The
survey (`docs/GAME-08-woods-meadow-and-ranch.md`) found:

- The Lost Woods' maze is data already working: rooms joined by `En_Holl` planes, a wrong turn's
  corridor ending in an exit back to Kokiri Forest, Saria's Song louder the right way
  (`En_River_Sound` 0x0C, ported).
- Not ported: `En_Sa` (Saria, 860 lines), `En_Wf` (the Wolfos, 1,454), `Bg_Spot05_Soko` (the
  meadow's gate and blocks, 114); and what the Wolfos calls: eight `z_actor.c` helpers
  (`Actor_FindNearby`, `Actor_GetProjectileActor`, `Actor_IsLockedOn`, `Actor_OtherIsLockedOn`,
  `Actor_TestFloorInDirection`, `func_800354B4`, `func_800355B8`, `func_800359B8`),
  `CollisionCheck_CylSideVsLineSeg` and `EffectSsKiraKira`.
- `En_Sa` is also Kokiri Forest's Saria, left over from Phase 3: on a new save her init starts
  `gKokiriForestSariaGreetingCs` as soon as the forest's room 0 loads. Porting her changes every
  run and test that starts a new save there.

## Decision

1. **The shared code first, here:** the `z_actor.c` helpers in `oot_game::actor_ctx`;
   `CollisionCheck_CylSideVsLineSeg` in `oot_game::collision_check`; `Actor_GetProjectileActor`
   in the `oot_actors` crate root (it reads an arrow's fields), with the C's @bug: it reads every
   projectile as a hookshot's `timer` (0x210), for an arrow the top half of `unk_210.x`;
   `EffectSsKiraKira` whole (its three update modes, `_SpawnSmall`, `_SpawnSmallYellow`,
   `_SpawnDispersed`, `_SpawnFocused`). `Actor_UpdateAlphaByDistance` (`oot_game::npc`) gains
   its cutscene branch (a quarter of the distance to the view's eye), shared by `En_Md` and
   `En_Sa`.
2. **The actors whole**, each with its Params and Flags paragraphs and its schema: `En_Sa` (every
   type: the greeting, standing in the forest and in her house, waiting and playing in the
   meadow), `En_Wf` (both Wolfos), `Bg_Spot05_Soko`. Two helpers ported `En_Sa` and `En_Wf` in
   their own worktrees.
3. **Saria's greeting is the game's, everywhere.** A new save's first walk out of Link's house
   plays it, as the game does:
   - the new-save routes play through it (`Task::Exit`'s settle presses A through a text that
     waits; no route met one there before), and their tests expect its text (0x1001) and script;
   - the tests that start a new save in Kokiri Forest for something else give their save the state
     after it (`common::after_sarias_greeting`: `INFTABLE_SARIA_GREETED_LINK`);
   - the save presets past Link's first walk out (`kokiri_sword_and_deku_shield`'s and
     `sword-and-40-rupees`') set that flag (`save::left_the_house`);
   - the tests counting Kokiri Forest's fairies count hers.
4. **The exit's run (`Route::Meadow`, `graveyard-sun`, from `ENTR_LOST_WOODS_0`):** the woods' right
   turns by their planes (east, north, east, north, north, west, north out of room 8), the owl's
   talk in room 2 answered no; the Wolfos fought (`Task::FightWolfos`: locked on, the shield up,
   the sword when it's open); the maze's Mad Scrubs: three bounced off the shield and chased
   (`Task::ChaseScrub`: a running scrub freezes where the camera doesn't see it, the culling
   volume's skip, so Link runs at it before locking on) and killed; the fourth run past (its nut
   breaks on the edge of the dip Link would stand in); the ramp's two run past (three hits); past
   z -2220, Saria's lesson, the song played back, and the song given at the script's end.
5. **The coverage table's corrections** (the user's call): Lon Lon Ranch's layer 6 is the
   credits', Kokiri Forest's 10 to 13 the credits' and the Deku Sprout's, `Obj_Dekujr` Phase 14's.

## Consequences

- Every Kokiri Forest test or run on a new save now meets Saria's greeting unless its save says
  otherwise; nine goldens changed with her (`golden/README.md`).
- The `z_actor.c` helpers serve the enemies to come (the Wolfos's kind: `Actor_OtherIsLockedOn`
  and `func_800354B4` are common).
- A Mad Scrub's nut doesn't come back off the shield on the meadow's ramp or from its dip; whether
  the game does the same there is unchecked (BACKLOG #64).
