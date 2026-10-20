# 0080: The rest of Dodongo's Cavern

- **Status:** accepted, built in GAME-09 milestone 5 (2026-10-19)
- **Date:** 2026-10-19
- **Builds on:** [ADR 0072](0072-bombs-and-bomb-flowers.md) (the bombs, another actor's fuse),
  [ADR 0074](0074-dodongos-cavern-route-the-skull-and-effect-blure.md) (the cavern's route,
  `EffectBlure`, the one-point cutscenes' table), [ADR 0066](0066-the-actor-schema.md) (the
  schemas), [ADR 0068](0068-glb-export.md) (the characters' table), [ADR 0059](0059-castle-towns-actors.md)
  (an overlay's statics).

## Context

Phase 10's fifth and last milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`): the four
actors Phase 10's scenes place that weren't ported (`En_Dodongo`, `En_Vm`, `En_G_Switch`,
`Bg_Ddan_Kd`), their effects (`Effect_Ss_D_Fire`, `Dead_Dd`, `Dead_Ds`), one-point 3050, and rooms
2, 5, 6, 13, 14 and 15 whole. Four things had to be settled from the C first: what drops the
falling stairs and what 3050 shows; how the silver rupees and their tracker share state and what
they open; what the Dodongo and the Beamos need beyond themselves; and which rooms a child reaches.

## Decision

1. **The falling stairs** (`Bg_Ddan_Kd`) are the C's: two different explosions in their cylinder,
   80 apart and within 13 frames, each quenched (`params` 2: its AT and its setting off of
   neighbours stop). Room 2's Bomb Flower rows are their fuse. **One-point 3050** joins
   `OnePointCutscene_SetInfo`'s cases: the spline on `CAM_SET_CS_3` from the pack's
   `D_8012013C`/`D_8012021C`, 210 frames, 1020 after it, Player held, this ROM's chime and quake, and
   the C's write into its own point list kept.
2. **The silver rupees' count is the overlay's static** (`sCollectedCount`, ADR 0059's
   `overlay_static`), zeroed by the tracker's init and freed with the last `En_G_Switch`. Every
   type of `En_G_Switch` is ported; the shooting gallery's writes (`En_Syateki_Itm`, Phase 20) are
   logged.
3. **The Dodongo and the Beamos needed nothing new of Player.** What they need of the framework was
   there; two helpers they and older actors share moved into it: `eng_math::vec3f_pitch`
   (`Math_Vec3f_Pitch`) and `MtxF::to_zyx_rot_s` (`Matrix_MtxFToZYXRotS`). The effects' spawner
   carries `play->state.frames`, their draw `play->objectCtx` (`D_Fire`'s first texture and its
   object check).
4. **The runs are four short debug starts** (`Route::FallingStairs`, `SilverRupees`, `ShieldChest`,
   `TimeBlocks`), no fights: where a room's enemies would be in the way the start marks it cleared,
   and the silver rupees' starts with four of the five taken. The presets `cavern-stairs` and
   `cavern-rooms` (the milestone's end) are checked against their runs; `cavern-song-of-time` is a
   debug start (`fairy-magic` has no Song of Time).
5. **The sandbox takes `--room`**, as the game does, so goldens can start in a room.

## Consequences

- Every actor placed in Phase 10's scenes is ported (COVERAGE: 33 of 33); the phase's exit review
  is next.
- The shooting gallery's rupees and horseback archery's pots wait on their games (BACKLOG #98); a
  Bombchu swallowed by a Dodongo on the Bombchus (#99).
- Milestone 3's lift test changed: room 2's lift and the stairs share a top at 480 before the fall.
