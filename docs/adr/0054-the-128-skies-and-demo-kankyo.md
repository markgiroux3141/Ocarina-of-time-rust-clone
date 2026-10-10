# 0054: The outdoor skies, `Demo_Kankyo`, and a room hidden

- **Status:** accepted, built in GAME-06 milestone 1b (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0022](0022-cutscenes.md) (the
  cutscene layers and the actors' cues), [ADR 0053](0053-demo-effect-curve-skeletons-and-vertices-by-source.md)
  (`Demo_Effect`, the chain's first half).

## Context

The Kokiri Emerald's chain after part 1 (BACKLOG #23) goes through four scenes besides Kokiri
Forest: the cutscene map (`hiral_demo`, parts 2, 3 and 8), Gerudo Valley (4 and 5) and Death
Mountain Trail (6). Milestone 1a left them with three things missing:
- **No outdoor sky.** The port drew only the room skyboxes (`drawType` 1 and 2, the houses' 360°
  images). The outdoor skies are `SKYBOX_DRAW_128`: four 128x64 side faces and a 128x128 top (the
  cutscene map has a bottom too), each face two CI8 textures blended by the primitive alpha
  (`SETUPDL_40`), their two 128-colour palettes the halves of one 256-colour TLUT. The normal sky
  has twelve texture files (`gNormalSkyFiles`), two shown at a time by the time of day
  (`gTimeBasedSkyboxConfigs`), loaded by `Environment_UpdateSkybox`'s DMA state machine and
  blended by `envCtx.skyboxBlend`.
- **`Demo_Kankyo`** (`z_demo_kankyo.c`, 1,015 lines): the creation's blue rain, Din's tumbling
  rocks, and types the chain doesn't reach (clouds, the Door of Time, the warp songs' sparkles).
  Its rain and its sparkles change their state in their draws, with `Rand` calls.
- **A room hidden:** the rain (in the cutscene map) and the rocks set
  `play->roomCtx.curRoom.segment = NULL`, and so does `CS_MISC_HIDE_ROOM`. `Room_Draw` draws
  nothing for that room until the next room load.

The user chose (2026-10-09) to port `Demo_Kankyo` whole and to play the whole chain.

## Decision

- **The 128 skies are bakes, one per pair of textures.** `Skybox_CalculateFace128` and
  `Skybox_Draw`'s commands are written out (`oot_game::skybox::bake_128`) and baked by the
  importer: the two textures on segments 7 and 8, the two palettes on 9 as one buffer
  (`BakeSegment::Files`: whole files one after another), the vertices, the face lists and
  `SETUPDL_40`, the blend a dynamic prim colour on segment 0xF (0xC is the importer's cull
  builtin). The bakes are the cutscene map's (six faces, its fixed `holy0`/`holy1` files), the
  overcast sunset's, and the normal sky's: every pair a config shows together, and the
  cross pairs a config change can show (`normal_sky_pairs`). The app draws the sky first, at the
  eye, turned by `skyboxCtx.rot`.
- **What is loaded follows the C.** `SkyboxContext` (`oot_game::skybox`) keeps the two texture
  indices and the two palette halves loaded. `Skybox_Init` (`Skybox_Setup`'s pair by
  `skyboxTime`, its blend; a storm kept from the last scene picks config 1) runs before
  `Environment_Init`, which resets `skybox1Index`/`skybox2Index` to 99 and keeps the blend.
  `Environment_UpdateSkybox` runs at `Play_Draw`: the time-based entry, a config change's
  blend, `Environment_UpdateStorm`, then the DMA states. A load started in a call completes in the
  next (the C's `osRecvMesg(OS_MESG_NOBLOCK)` fails on a DMA just started), so a texture's pair
  changes a frame after its index, its palette two frames after that. The bake drawn is the pair
  of the two textures loaded, with their own palettes: the two frames in which the C draws a new
  texture with the old palette aren't drawn. The cutscene map takes config 3's blend, as the C
  does.
- **`Demo_Kankyo` is ported whole** (`oot_actors::demo_kankyo`): every init case, the seven
  actions, the seven draws and their helpers (`func_80989B54`, `DemoKankyo_Vec3fAddPosRot`,
  `func_800BB2B4`'s splines), and `Environment_WarpSongLeave`. The rain's and the sparkles' state
  changes (their modes, their `Rand` calls, the warp's leave, the sparkles' end) run in
  `draw_update`, once per game frame, from `play->view`; they leave the matrices for `draw`. The
  warp-in's sound is played in `draw_sfx`. The draws are bakes: the rain (its colours dynamic,
  `SETUPDL_20`), the rocks, the clouds (`gDust5Tex`, `SETUPDL_61`), the Door of Time's halves, the
  light plane (its scroll dynamic); the sparkles reuse `Demo_Effect`'s flash. The camera points
  are the C's tables, checked against it by a script.
- **A hidden room is an unloaded one.** `roomCtx.curRoom.segment = NULL` clears `Room::loaded`;
  `RoomContext::drawn` already leaves unloaded rooms out, and the next room load sets it again.
  `CS_MISC_HIDE_ROOM` does the same.
- **The chain's other actors:** `Bg_Spot09_Obj` (Gerudo Valley's bridges and tent) and
  `Bg_Spot16_Doughnut` (Death Mountain's cloud ring) whole; Gerudo Valley's and Death Mountain
  Trail's scene draw configs, checked against the C interpreter as the others are.

## Consequences

- The whole chain plays from the blue warp to `ENTR_KOKIRI_FOREST_11` with its skies, its rain,
  its rocks and its lights; Hyrule Field, Death Mountain and the other normal-sky scenes draw
  their sky at the scene's time (milestone 3 makes the time pass).
- The 128 skies are 56 bakes in the pack. A pair not baked (none the configs show) would draw no
  sky.
- The Door of Time's collision (`Door_Toki`) is a placeholder until the Temple of Time (Phase 8).
  The warp songs' sparkles and their leave are reached only in the tests until the ocarina; their
  `player->actor.draw = NULL` is logged, and `Interface_SetSubTimerToFinalSecond` is a no-op (no
  sub-timer runs in the port).
- `Bg_Spot16_Doughnut` isn't in Death Mountain Trail's layer 4: its object isn't in the layer's
  list, so `Actor_Spawn` refuses it, as the C's does.
- `LOD_FRACTION` (BACKLOG #24) isn't read by any of these lists' combiners; it stays open.
