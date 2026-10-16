# 0073: Death Mountain Trail, Goron City and the Goron's Bracelet

- **Status:** accepted, built in GAME-09 milestone 2 (2026-10-15)
- **Date:** 2026-10-15
- **Builds on:** [ADR 0072](0072-bombs-and-bomb-flowers.md) (the bombs), [ADR 0066](0066-the-actor-schema.md)
  (the schemas), [ADR 0064](0064-kakariko-lifting-and-the-village.md) (Player's lift, carry and
  throw), [ADR 0068](0068-glb-export.md) (the characters' table).

## Context

Phase 10's second milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`): the trail and
Goron City up to the Goron's Bracelet, and the boulder at Dodongo's Cavern's mouth that the
bracelet's Bomb Flower opens. Fifteen overlays a child meets there weren't ported (`En_Go2` alone
is 2,113 lines), and four systems: the room's wind (the trail's flag), one-point cutscenes 4180,
4210 and 4220, Goron City's draw config, and the child's bracelet drawn on Link.

## Decision

1. **The overlays whole**, each with its schema and its Params and Flags: `En_Go2` (every child
   type and the bazaar's; the adult's types 1, 2 and 3, Link the Goron, Biggoron and the Fire
   Temple's Gorons, are placeholders chosen in its init after the init's own kills, as `En_In`'s
   race was: the user's call), `En_Du` (the Fire Temple's meeting too; one-point 3330 isn't
   ported and logs), `Bg_Spot18_Shutter` (the adult's sideways door too), `Bg_Spot18_Obj` (the
   adult's pull too), `En_Tite` (the blue too), `Bg_Spot16_Bombstone`, `Bg_Bombwall`, `En_Hata`,
   `Bg_Spot18_Basket`, `Bg_Spot18_Futa`, `En_Gm` (the knife's sale: the user's call),
   `Obj_Hamishi`, `Obj_Hsblock`. Five helpers ported them in worktrees; `En_Part`'s Tektite
   pieces and `En_Encount1`'s Tektites came with `En_Tite`.
2. **The room's wind:** `SCENE_CMD_ID_WIND_SETTINGS` is read at import (`RoomData::wind`, the
   pack's format 29) and sets `envCtx.windDirection` and `windSpeed` as the room loads
   (`Scene_CommandWindSettings`). `windSpeed` starts at 0 in a room without the command: the
   C's `Environment_Init` doesn't set it (`@bug (game)`), and the play state is zeroed.
3. **One-point cutscenes 4180, 4210, 4220 and 4221** in `oot_game::onepoint` (the boulder, the
   urn's two, Darunia's door for an adult), from `z_onepointdemo.c`'s cases; their keyframe
   tables were already in the pack by name.
4. **`SDC_GORON_CITY`:** its OPA scroll on segment 8, and its XLU entrance texture on the same
   segment by `nightFlag` (`segment_textures`), checked against the import's C interpreter.
5. **The child's bracelet on Link:** `gLinkChildGoronBraceletDL` loads one of the flex
   skeleton's limb matrices (`0x0D0002C0`, the left forearm) after the skeleton has drawn. A new
   bake segment, `BakeSegment::SkeletonMatrices`, binds segment 0x0D to the skeleton's matrix
   order (`Skeleton::flex_matrix_map`), so the list's vertices follow Link's bones; Player draws
   it with his pose when `Player_GetStrength` is above none, as `Player_DrawImpl` does (not in
   first person nor crawling behind the camera).
6. **The runs are short debug starts** (the user's standing preference, re-stated during this
   milestone): `Route::Darunia` from Goron City's bottom floor (the lullaby at his door, Saria's
   Song, the bracelet) and `Route::Boulder` from the cliff above the cavern (the Bomb Flower
   thrown down onto the boulder). The whole story chain (Kakariko's gate, the trail's Tektites,
   the city's walks, back up to the cliff) is the hand test's. Each run's end is a preset
   (`goron-bracelet`, `cavern-open`: milestone 3's start), checked against the run.
7. **Medigoron's animation** lives in `object_gm`, which has no skeleton: `foreign_anims()`
   decodes it on `gGoronSkel` at import (as `En_Viewer`'s are).

## Consequences

- Death Mountain Trail and Goron City are whole for a child but for the summit (milestone 6:
  `En_Encount2`, `En_Fire_Rock`, the fairy) and the adult's parts above.
- The trail's rolling Goron, the Tektites and the city's big rolling Goron run the C's
  `Rand`-free and `Rand` logic as it is; no scripted run depends on them.
- Pack format 29: packs must be re-imported (`out/data26`).
- A list drawn after a flex skeleton that loads its matrices can now be baked; the adult's
  gauntlets and boots (`sBootDListGroups`, the gauntlet plates) can use the same segment when
  they're drawn.
