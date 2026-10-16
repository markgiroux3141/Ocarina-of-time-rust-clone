# 0074: Dodongo's Cavern's route, the skull and EffectBlure

- **Status:** accepted, built in GAME-09 milestone 3 (2026-10-16)
- **Date:** 2026-10-16
- **Builds on:** [ADR 0073](0073-death-mountain-trail-goron-city-and-the-bracelet.md) (the trail, the
  presets), [ADR 0072](0072-bombs-and-bomb-flowers.md) (the bombs), [ADR 0066](0066-the-actor-schema.md)
  (the schemas), [ADR 0068](0068-glb-export.md) (the characters' table), [ADR 0012](0012-actor-bakes.md)
  (the bakes).

## Context

Phase 10's third milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`): a child's way
through Dodongo's Cavern (Master Quest) from its entrance to the fall into King Dodongo's room,
by route B (the loop to the skull's bridge: the user's call). The survey left the climbs open;
`ootx` dumps no collision. On the way: the skull (`Bg_Dodoago`), the pillars (`Bg_Ddan_Jd`), the
Baby Dodongos, the Lizalfos, and (once the search had settled the way) the Poes, the Armos, the
dark maze's blocks, the elevator and the fire walls of rooms 1, 4 and 8. Three one-point
cutscenes, the scene's draw config, and `EffectBlure`, the trail behind the Lizalfos's sword, which
the user asked to have ported with it and wired to Link's sword and the Skulltula.

## Decision

1. **The way settled by a search over the collision**, not by a guess: a 25 grid of every floor
   (raycast down, again under each), steps under 12 (17 diagonally), climbs of 18 to 71 against a
   wall onto a floor with normal y at least 0.75 (`sAgeProperties[child]`), ladders (wall type 2)
   bottom to top, run-off jumps and the mid-air ledge grab tried with the port's Player. Its
   findings changed the survey's (the GAME-09 doc's milestone 3 section): the bomb bag's corridor
   is a child's only from room 4 (door 22); room 3's two levels aren't joined and its paired
   Lizalfos kill themselves at init when Link is more than 100 above or below them, so the lower
   pair is fought from room 1's door 9 and the upper from room 10's door 10; the lobby's only way
   up is the shortcut pillar and the ledge grab off its top. Rooms 1 and 4 are both on the way, so
   `En_Poh`, `En_Am`, `Bg_Gnd_Darkmeiro` and `Obj_Elevator` came into this milestone from
   milestone 5.
2. **The overlays whole**, each with its schema and its Params and Flags: `Bg_Dodoago`,
   `Bg_Ddan_Jd` (the flag's shift masked to 5 bits as the console does), `En_Dodojr`, `En_Zf`
   (Lizalfos and Dinolfos, the pairs' turns), `En_Poh` (the normal Poe and the composers, Sharp and
   Flat), `En_Am`, `Obj_Elevator`, `Bg_Hidan_Curtain`, `Bg_Gnd_Darkmeiro`. Helpers ported them in
   worktrees (removed). Not ported, logged where they're called: the rumble (#60), the point
   lights' shading and the light's glow, `Lights_GlowCheck`.
3. **`SDC_DODONGOS_CAVERN`** in `scene_table.rs`: the two scrolls, the entrance's texture by
   `nightFlag`, the lava floor's eight frames by `gameplayFrames`, and the eyes' env colour from
   `roomCtx.drawParams`, which `Bg_Dodoago` writes. The import's segment-texture check covers it.
4. **One-point cutscenes 3060, 3065 and 3380** (`Camera_SetCsParams` keyframes `D_80121904` and
   `D_801218B4`, read from the pack by name; the quake on 3065 and 3380).
5. **`EffectBlure`** (`z_eff_blure.c`) in `oot_game::effect::blure`: the elements, both inits, the
   five calc modes, `EffectBlure_AddVertex` and `_AddSpace`, the update, and the three draws (the
   strip, the simple quads with their alternate prim colours, the smooth Hermite strip). Its
   vertices move every frame, so its bakes put each vertex at the origin under its own matrix
   (`BakeSegment::Matrices` on segment 0x0C) and the draw sets the matrices' translations (the
   bones), with per-corner vertex colours. Its callers: `En_Zf`'s sword, Player's sword
   (`D_8085470C`, its effect index added at init and deleted at destroy), `En_St`'s and
   `En_Ssh`'s (`EFFECT_BLURE1`), `En_Arrow`'s four arrows.
6. **The runs are four short debug starts**, the story chain and the fights the hand test's (the
   user's standing rule): `Route::Pillar` (the island's flower thrown at the pit's boulder,
   the switch, the ladder, the shortcut pillar to the upper level: `Task::RunOntoPillar`, and
   `Task::ClimbTo` holding the stick along Link's facing while he hangs), `Route::BombBag` (the big
   chest), `Route::Skull` (bombs onto C-Right in the pause menu, one put down through each of the
   bridge's holes: the eyes, the jaw), `Route::Gravestone` (`Task::PullStone`, switch 0x20, door
   20, the fall). Each run's end a later one starts from is a preset checked against it:
   `cavern-bomb-bag`, `cavern-jaw` and `cavern-boss` (milestone 4's start). `cavern-lobby` and
   `cavern-loop` are debug starts (the loop done by hand). The presets from `cavern-open` on wear
   the Hylian Shield (the graveyard's shield grave, which the story chain's presets never opened).

## Consequences

- A child can go from the cavern's entrance to King Dodongo's room in the port; the user's hand
  test walks it whole.
- Every sword swing now leaves its trail: the goldens with Link's sword (and the Skulltulas'
  attacks) differ, each proven by a build with the trails off.
- `EffectBlure`'s per-vertex matrices are a pattern for other strip effects whose vertices move
  every frame (`EffectSpark` is a different, already-ported kind).
- Still not ported in the cavern: `En_Dodongo`, `En_Vm`, `En_G_Switch`, `Bg_Ddan_Kd`, the effects
  `D_Fire`, `Dead_Dd`, `Dead_Ds`, one-point 3050, rooms 2, 5, 6, 13, 14 and 15's ways (milestone 5).
