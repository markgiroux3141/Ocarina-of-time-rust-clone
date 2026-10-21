# 0082: Jabu-Jabu's Belly to the boomerang: the breathing walls, the switches, lifts and spouts, and the way in

- **Status:** accepted, built in GAME-10 milestone 4 (2026-10-21), hand-tested
- **Date:** 2026-10-21
- **Builds on:** [ADR 0012](0012-actor-bakes.md) (the bakes and their dynamic segments),
  [ADR 0081](0081-water-bottles-and-the-underwater-view.md) (the view's distortion, the currents),
  [ADR 0029](0029-one-point-cutscenes.md) (the one-point cutscenes), [ADR 0066](0066-the-actor-schema.md)
  (the schemas).

## Context

Phase 11's fourth milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`): inside Lord Jabu-Jabu
(Master Quest) from his mouth to the boomerang. The scene's draw config wasn't ported, so its flesh
didn't scroll, the view didn't breathe and its walls didn't pulse; its switches, moving parts and
spouts and the six enemies on the way were placeholders. The survey had left the climbs and the
water open.

`Scene_DrawConfigJabuJabu` does three things the other ported configs don't: it sets the view's
distortion every frame, it keeps its own statics across frames (and scenes: they're the code
segment's), and it puts a **matrix** on segment 0x0D (`Matrix_Scale(1.005, sin × 0.8, 1.005)`,
`MATRIX_FINALIZE`) which rooms 2 and 8 to 12 (and Barinade's room 1) multiply onto a matrix of their
own before loading the flesh panels' vertices (`gSPMatrix(0x0D000000, G_MTX_NOPUSH | G_MTX_MUL)`).
The importer had bound every config matrix to the identity, so the panels stood still at full
thickness, which the game never shows (their thickness swings between ±0.8).

## Decision

1. **A config's matrix is a site matrix in the room's list.** `eng_gbi` gets
   `Segment::SiteMatrix`: each `gSPMatrix` of it gives the vertices under it a bone of their own
   and records the matrix it was multiplied onto in the draw list (`DrawList::site_bases`, the
   pack's format 30, `out/data27`); the room draw poses each bone as its base times the config's
   segment matrix of the frame (`DrawConfigState::segment_matrix`, OPA only, as the C binds it).
   The renderer's skinning does the rest. Only Jabu-Jabu's config binds a matrix (the importer's
   `Bind::Mtx`), so no other room changes. A matrix loaded over a site bone ends it; one multiplied
   onto it is logged as unresolved (no list does that).
2. **The config's statics** (`D_8012A39C`, `D_8012A3A0`, `D_8012A398`) live in
   `scene_table::JabuStatics` on the draw state and are carried over a scene change as the other
   code-segment statics are. Its view distortion is an output of the config, applied after the
   frame's `View_StepDistortion` (as `Scene_Draw` follows `View_Apply`). `roomCtx.drawParams[0]`,
   set by a hit on the flesh (`func_8002F9EC`), now comes from Player's sword too (a
   `PlayRequest::RoomDrawParam0`).
3. **Barinade's room's scroll** is the C's unsigned `(127 - gameplayFrames) % 128`; the importer's
   comparison asks the C interpreter (which does it signed) for `frames % 128`, with a note.
4. **The three set pieces whole:** `Bg_Bdan_Switch` (blue, yellow, heavy, tall, the toggle),
   `Bg_Bdan_Objects` (Big Octo's platform with Big Octo a placeholder until milestone 8, the
   elevator, room 14's water box, the falling platform with one-point 3100), `En_Siofuki` (the
   spouts). `Camera_Special0` (`CAM_SET_PIVOT_VERTICAL`, 3100's camera) is ported with it.
   `PlayerIface` gains `player_speed`, `set_player_speed` and `set_pushed` (`player->speedXZ`,
   `pushedYaw`, `pushedSpeed`), which the spout writes as the C does.
5. **The six enemies whole**, by three helpers: `En_Bubble`, `En_Bili`, `En_Brob` (a DynaPoly
   actor, as the C), `En_Weiyer`, `En_Eiyer` (`PlayState::adopt_from_init` for the leader's
   clones), `En_Rr` (the theft through the existing `Inventory_DeleteEquipment`). No fight is
   scripted: their tests drive their states, and `game-jabu.bat` starts beside them.
6. **The way, settled by a search over the floors** (GAME-09 milestone 3's, rebuilt for this:
   every floor of each 20-unit cell, walks, climbs to 71, falls, walls by line tests) and by the
   water boxes:
   - room 1's elevator stops 95 short of both corridors: it's boarded by a run-off jump as it
     comes level on its way down (`Route::JabuElevator`);
   - the elevator's shaft is walled (x -300) from 62 under the pit's water (`waterBoxes[1]`, -1248)
     up: the yellow switch 0x19's ledge (-1233, 160 over the pit's west ramp) is reached by diving
     under that wall (`Task::DiveUnder`) and climbing out of the west pool;
   - **door 14 is shut to Link from room 1 until the boomerang's chest**: room 1's spout 0x15C6
     stands across it (200 high, x 241 to 441 round the door's 313) and pushes him back; its
     switch 0x17 is set by nothing in Master Quest, so it drops only for good, with the chest's
     flag 6. Door 14 is the way *out* of room 14 (`Route::JabuDoor14`); the way in is room 3's
     door 12 (Ruto on the blue switch 0x3C: milestones 5 and 8);
   - **room 14's chest ledge** (-1073) is joined to nothing a walk, a climb or the water reaches
     (220 over the floor, 135 over the raised water): the boomerang's run starts on it (a debug
     start, as the brief set out); how a child gets up is left to the hand test.
7. **The runs chain by presets:** `jabu-jabu` (`Route::JabuFish`'s end), `jabu-room-1`
   (`JabuCow`'s: the slingshot equipped from the menu, as the story's save has it on no button),
   `jabu-room-14` (`JabuElevator`'s), `jabu-boomerang` (`JabuBoomerang`'s); each end is checked
   against the next (`ends_as_preset`), less the debug start's room clear.

## Consequences

- Jabu-Jabu breathes, scrolls and pulses as the game does, and a sword or seed on its flesh
  quickens the pulse. The site matrix is general: any later config that binds a matrix gets it.
- The pack's format is 30 (`out/data27`); every room list carries `site_bases` (empty but for
  Jabu-Jabu's and Barinade's).
- The story's way to room 14 needs Ruto (milestones 5 and 8); the milestone's runs use debug
  starts past it, as the brief planned.
