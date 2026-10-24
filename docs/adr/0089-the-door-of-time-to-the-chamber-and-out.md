# 0089: The Door of Time to the Chamber of the Sages and out

- **Status:** accepted, built in GAME-11 milestone 2 (2026-10-23); hand-tested
- **Date:** 2026-10-23
- **Builds on:** [ADR 0088](0088-the-age-switch.md) (the age switch, `Bg_Toki_Swd`),
  [ADR 0022](0022-cutscenes.md) (the cutscene destinations), [ADR 0014](0014-prerendered-backgrounds.md)
  (the prerendered scenes), [ADR 0066](0066-the-actor-schema.md) (the schemas).

## Context

Milestone 1 made the age switch and the pedestal work from debug starts. Phase 12's exit is the
story's way from `ocarina-of-time` to adult Link outside the Temple: the Temple's intro, the Door
of Time, the reveal and the first pull, Ganondorf, the Chamber of the Sages (Rauru and the Light
Medallion), the return and Sheik's awakening, and the walk out. Its actors were merged with
milestone 1 but had never run in their cutscenes. The Chamber's draw config and
`chamberCutsceneNum` weren't ported, and the adult field's ruins (`Bg_Spot00_Break`) were a
placeholder (the user's call 5).

## Decision

1. **`SDC_CHAMBER_OF_THE_SAGES`** in `oot_game::scene_table` (`z_scene_table.c:408`): the three
   scrolls, checked against the C interpreter with the others (`oot_import`'s pack test).
2. **`chamberCutsceneNum` on `SaveContext`**, outside the save (`SaveContext_Init` and the file
   select start it at `CHAMBER_CS_FOREST`; it's written in the slot's copy of `SaveContext` at
   0x1415, never read back). `z_demo.c`'s destinations 29 to 31 set it. Its readers are
   placeholders of later phases (`Demo_Sa`, `Demo_Du`, `Demo_Im`; `Door_Warp1`'s adult warps, the
   other writer, aren't ported): each logs, outside a cutscene layer, when the number would start
   its medallion's script.
3. **`Bg_Spot00_Break` whole** (the broken drawbridge and the three barbed wire fences on the
   field's adult layers; killed for a child). Phase 12's coverage row is complete.
4. **The story's runs, chained by presets:**
   - `Route::DoorOfTime` (`ocarina-of-time`, the Temple's entrance): the intro, the Ocarina of
     Time onto C-Down from the item page, the Song of Time 31 from the altar's song spot (it takes
     the ocarina only within 50), `gDoorOfTimeOpeningCs`. Its end is **`door-of-time`**.
   - `Route::FirstPull` (`door-of-time`): round the altar, through the doorway; the reveal, the
     pull, the cutscene map's Ganondorf, the Chamber's layer 4 to terminator 18's transition. Its
     end is **`light-medallion`**, taken as the transition starts: on the Temple's next
     `Play_Init`, `En_Xc` 9 sets `EVENTCHKINF_C5` at once.
   - `Route::Awakening` (`light-medallion`, `ENTR_TEMPLE_OF_TIME_4`): Sheik's script and the return's
     time warp (`EVENTCHKINF_C9`). Its end is **`adult-awake`, Phase 13's start**.
   - `Route::TempleExit` (`adult-awake`, a debug start in room 0 where the awakening leaves Link,
     entered by the hall as milestone 1's): out to `shrine_r`, **the exit**.
   - `Route::Ruins` (`adult-awake`, the Temple's steps): the ruined market round its round fountain
     and its eight ReDeads (every one stays over 160 away; `EnRd_Idle` hears Link only within
     150), the ruined market entrance, the field by the broken drawbridge (`ENTR_HYRULE_FIELD_7`).
     No fight.
5. **The run tasks:** `Task::WalkUntil` takes its points in turn, as `Task::Walk` does (it aimed at
   the first point Link wasn't within 30 of, which made him turn back and forth at a waypoint;
   every earlier use has one point, so nothing else changes). `Task::MenuCursor` keeps its moves
   along the row, then the column, but where that would bring the cursor back to a slot it has
   been on (the item page skips empty slots and wraps rows), it takes the shortest way of the
   cursor's moves by `KaleidoScope_DrawItemSelect`'s rules instead. Earlier runs never revisit a
   slot, so their inputs are unchanged.
6. **`En_Rl`'s turn of Link** (`func_80AE7668`) stays a write to Player's actor
   (`play.actors.actor_mut`): Rauru's update runs while Player is in the arena, and the C writes
   only `world.rot.y` and `shape.rot.y`; `PlayerIface::set_yaw` would also write `player->yaw`.
   Its static (`D_80AE81AC`) stays per instance (one Rauru per scene).
7. **Milestone 3's `En_Xc` type 2** (the Prelude: actions 29 to 44, the harp's draws) was ported
   alongside by a helper and merged early; its bake `En_Xc/harp` makes **pack format 34**.

## Consequences

- The story plays from `ocarina-of-time` to adult Link in the field, in five short runs; the
  presets `door-of-time`, `light-medallion` and `adult-awake` hold what the story has by each.
- The Temple's cutscene layers 4 and 6 hold the scripts `En_Xc` 9 plays on layer 2; no destination
  enters them (`CS_INDEX_0` and `CS_INDEX_2` at the Temple are the map select's only).
- Adult Link meets the ruins' prerendered cameras: `shrine_r` and `enrui` `PREREND_FIXED`,
  `market_ruins` `PREREND_PIVOT` (`SCENE_CAM_TYPE_FIXED_MARKET`).
- `chamberCutsceneNum`'s medallion scripts wait for their phases (14 to 17).
