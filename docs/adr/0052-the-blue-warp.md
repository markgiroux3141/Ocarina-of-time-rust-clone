# 0052: The blue warp: `Door_Warp1` for this ROM's Deku Tree, a bake segment bound to a draw's matrix, and the arrival by blue warp

- **Status:** accepted, built in GAME-05 milestone 6b (2026-10-09)
- **Date:** 2026-10-09
- **Builds on:** [ADR 0022](0022-cutscenes.md) (the cutscene layers, Player's cue),
  [ADR 0029](0029-one-point-cutscenes.md) (the one-point cases ported as scenes need them),
  [ADR 0050](0050-queen-gohma.md) (she spawns the warp).

## Context

Queen Gohma's death leaves the blue warp (`Door_Warp1`, `z_door_warp1.c`, 950 lines in
functions). Its params pick one of 13 kinds; this ROM's Deku Tree reaches two: the child warp
(`WARP_DUNGEON_CHILD`, 0) that takes Link out of her room, and the destination warp
(`WARP_DESTINATION`, 6) that Kokiri Forest's arrival layer places. The child warp gives the
Kokiri Emerald and sends Link to `ENTR_KOKIRI_FOREST_0` with cutscene 0xFFF1: layer 5, where Link
arrives by blue warp (Player's start mode 2, not ported) and the Deku Tree talks; its terminator
goes on to a chain of cutscenes in five other scenes (Ganondorf's tale, the world's creation),
back to Kokiri Forest for the Triforce and the Deku Tree's death (with `Demo_Effect`), and ends at
`ENTR_KOKIRI_FOREST_11`.

The warp's draw (`DoorWarp1_DrawWarp`) calls `gWarpPortalDL` twice. The list loads two matrices
from segments: 0x0A, the warp's position; 9, a ring above it raised and widened each frame. The
bakes had no way to take a matrix the draw computes. The list's combiner blends its two scrolled
tiles by `LOD_FRACTION`, which the shader takes as 0.

The user chose (2026-10-09) to port the warp's two kinds and Player's arrival, and to play Kokiri
Forest's first cutscene to its terminator, the rest of the chain left to the backlog.

## Decision

- **`Door_Warp1` is ported for its two kinds** (`oot_actors::door_warp1`): `DoorWarp1_Init`,
  `_Destroy`, `_ChooseInitialAction`, `_SetupWarp` (whole: its cases are assignments),
  `_WarpAppear`, `_PlayerInRange`, `_ChildWarpIdle`, `_ChildWarpOut` (its Deku Tree branch: the
  emerald, the flags and the forest's two entrances; the Dodongo's Cavern and Jabu-Jabu branches
  logged), `_Destination`, `_DoNothing`, `_Update`, `_DrawWarp`, `_Draw`. The other kinds' setups
  and actions are logged and the warp does nothing (`Action::NotPorted`): the adult warp and its
  crystal, the blue and purple crystals, the clear flag's warp, Ruto's, `WARP_UNK_7`,
  `func_809998A4`. `sWarpTimerTarget` is an overlay static.
- **A bake segment can be a draw's matrix:** `BakeSegment::Matrix(bone)`. The importer binds the
  segment as a one-entry matrix map, so the vertices a list loads under its `gSPMatrix` follow that
  bone; the draw passes the matrices as its bones (`DrawCmd::bones`, the transform the identity).
  The warp's bake is `gWarpPortalDL` with segment 0x0A on bone 0 and segment 9 on bone 1, its
  scroll (`Gfx_TwoTexScroll`) and its prim and env colours dynamic. `DoorWarp1_DrawWarp`'s change
  to `unk_19C` runs in `draw_update`, once per game frame.
- **One-point 9703 is ported** (`OnePointCutscene_SetInfo`): its keyframes from the view, the
  adult's heights.
- **Player's arrival by blue warp is ported:** `Player_StartMode_BlueWarp` (800 up, held, the
  arrival pose) and `Player_Action_BlueWarpArrive` (the slowed fall, the landing, then Kokiri
  Forest's pending cutscene mode or standing; over the script's cue while it runs).
- **Kokiri Forest's layer 5 plays with what's ported** (`Bg_Treemouth`, Navi's cues,
  `Object_Kankyo`); its destination warp is killed at its init, as in the game (Link isn't within
  100 of it then). Its terminator starts the transition to the castle town's cutscene map, whose
  scenes and the chain after them aren't ported (BACKLOG #23).
- **`LOD_FRACTION` stays 0** in the shader (BACKLOG #24): what the RDP gives it with texture LOD off
  is to be checked before the warp's second tile is blended.

## Consequences

- Gohma's blue warp works: Link floats up in its light, fades out white with the Kokiri Emerald,
  and arrives by blue warp before the Deku Tree. Every boss's warp after can follow the kinds
  logged here.
- Any bake can take matrices a draw computes (a part raised, scaled or turned apart from the
  rest).
- The Gohma golden's last screenshot shows the warp now (proved by taking the warp out).
