# 0090: Sheik's Prelude, time travel both ways and the warp

- **Status:** accepted, built in GAME-11 milestone 3 (2026-10-24); hand-tested
- **Date:** 2026-10-24
- **Builds on:** [ADR 0088](0088-the-age-switch.md) (the age switch, the pedestal),
  [ADR 0089](0089-the-door-of-time-to-the-chamber-and-out.md) (the story's presets, `En_Xc` type 2
  merged), [ADR 0022](0022-cutscenes.md) (the cutscenes).

## Context

Phase 12's last milestone: Sheik's Prelude of Light in the Temple of Time, the Master Sword put back
and pulled again in the story's order, and the Prelude's warp. `En_Xc` type 2 was ported with
milestone 2 but hadn't run in `gTempleOfTimePreludeCs`. The warp songs' action, box choice and
arrival were ported in Phase 7, but Link stayed drawn through the warp-out's white-out and the
arrival (BACKLOG #26: `player->actor.draw = NULL` had no counterpart). The sub-timer half of #26,
`Interface_SetSubTimerToFinalSecond`, came with the timers' port since.

## Decision

1. **Player's draw switch.** `ActorImpl::has_draw` (`actor->draw != NULL`, default true):
   `Actor_DrawAll` tests it after the culling, as the C, so an actor without a draw isn't drawn,
   its shadow neither (`isDrawn` false). Player's is off for start mode 0 (as before, `inert`) and
   while `hidden`: set by `Demo_Kankyo`'s warp-out on its timer's 15 (the white at its fullest,
   `PlayerIface::set_draw`) and by start mode 5 (`Player_StartMode_WarpSong`), cleared by
   `func_80852280` (the warp-in script's first action, through `Player_Action_WaitForCutscene`, and
   cutscene action 46). BACKLOG #26 closes.
2. **The Prelude needs the Forest Temple** (`EVENTCHKINF_48`), Phase 14's: its run starts from a
   debug preset, **`prelude`** (`adult-awake` with 0x48 and the Forest Medallion, as the Forest
   Temple's blue warp gives them), entered by the hall as the story does. The story's chain keeps
   its order: `adult-awake` goes on to Phase 13.
3. **The runs and presets:**
   - `Route::Prelude` (`prelude`): into room 0, `En_Xc` 9 gives the song; Sheik's harp, the song
     played back, her nut. Its end is **`prelude-learned`**.
   - `Route::SwordBack` (`prelude-learned`, where the Prelude leaves Link): a child again; its end
     is **`child-again`**.
   - `Route::SwordAgain` (`child-again`): an adult again. Its end is `child-again` with the pull's
     `Item_Give(ITEM_SWORD_MASTER)` and the swap back: `Inventory_SwapAgeEquipment` never clears what
     it stored, so `adultEquips` now holds the adult's buttons and `childEquips` the Master Sword's
     nibble in the child's equipment (the C's).
   - `Route::PreludeWarp` (`prelude-learned`, the field by the broken drawbridge): the song, "warp?"
     yes, the warp-out, the arrival on the light pad (`ENTR_TEMPLE_OF_TIME_7`).

## Consequences

- Phase 12 is built whole: adult Link, time travel both ways, the Prelude and its warp.
- A warp song now hides Link from the white-out until the warp-in's script gives him back (40
  frames and more into the arrival), as the game does; the other warp songs' arrivals share it.
- Any actor can now go undrawn by its own state (`has_draw`), not only by its culling.
