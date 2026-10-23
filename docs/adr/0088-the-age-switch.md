# 0088: The age switch and the time machine

- **Status:** accepted, built in GAME-11 milestone 1 (2026-10-23); hand-tested
- **Date:** 2026-10-23
- **Builds on:** [ADR 0019](0019-inventory-and-saves.md) (the save, `childEquips` and
  `adultEquips`), [ADR 0022](0022-cutscenes.md) (the cutscene destinations), [ADR 0047](0047-the-pause-menu.md)
  (the equipment page's stand-in), [ADR 0012](0012-actor-bakes.md) (bakes),
  [ADR 0066](0066-the-actor-schema.md) (the schemas).

## Context

Phase 12 (`docs/GAME-11-the-temple-of-time-and-adult-link.md`) makes Link an adult. The spikes
ported Player for both ages and the import bakes the adult, but nothing changed Link's age in
play: the thirteen cutscene destinations that set `play->linkAgeOnLoad` only logged it, the
equips were never swapped, `Bg_Toki_Swd` and `Door_Toki` were placeholders, and Player had no
time-travel arrival (start mode 1) and no updates for cutscene modes 18 and 30. The adult's
look also lacked what `Player_DrawImpl` draws after the skeleton (the gauntlets' plates, the
iron and hover boots), the tunic's colour at draw time (the bakes held the Kokiri Tunic's), and
the left hand's overrides (the pedestal's, the broken Giant's Knife).

## Decision

1. **`linkAgeOnLoad` on `PlayState`** (`link_age_on_load`, `LINK_AGE_ADULT` or `_CHILD`).
   `Scene_CommandPlayerEntryList` sets it to the save's age in `play_init_with`; the thirteen
   destinations set it (`cutscene.rs`'s terminator, 8 by the age it isn't, outside a cutscene
   layer); `reinit` (`Play_Destroy`) swaps the equips while the old age is current when it
   differs (`SaveContext::inventory_swap_age_equipment`, then `Player_SetEquipmentData`), before
   the actors go; and `Player_Destroy` makes it the save's age (`save.adult`), so the next
   `Play_Init` loads the new age's layer and Link.
2. **`Inventory_SwapAgeEquipment` whole** on `SaveContext`: the child's buttons kept with B as the
   Kokiri Sword; the first adult's (an `adultEquips` B of `ITEM_NONE`): the Master Sword on B, Deku
   Nuts on C-Left if owned, bombs on C-Down owned or not, the ocarina's slot on C-Right, and the
   Master Sword, Hylian Shield, Kokiri Tunic and Boots worn; later swaps restore each age's own,
   the child's with the Kokiri Sword worn; bottles and trade items re-read from their slots
   (`@bug (game)`: B's check reads `cButtonSlots[-1]`, `buttonItems[3]`, as the C's layout does); a
   worn shield no longer owned taken off. The survey's "C buttons empty" for the first adult was
   wrong: the C fills C-Down and C-Right.
3. **`Bg_Toki_Swd` and `Door_Toki` whole.** The pedestal reveals itself (`gRevealMasterSwordCs`),
   offers itself to a child or an adult with `EVENTCHKINF_55`, gives the child the Master Sword and
   plays `gPullMasterSwordCs` or `gPlaceMasterSwordCs` with `NA_BGM_MASTER_SWORD`, keeps itself
   Player's `interactRangeActor` through the cutscene (a new `PlayerIface::set_interact_range_actor`),
   and dims the temple on cutscene flag 1 (`roomCtx.drawParams[0]`, which
   `SDC_TEMPLE_OF_TIME` reads). `Door_Toki` is the closed Door of Time's DynaPoly; the port asks the
   DynaPoly only when the flag changes (its `set_collision_disabled` rebuilds the lookup on every
   call, the C's flag write doesn't).
4. **Player's time travel:** the adult's branch of `Player_ActionHandler_2` (the cutscene's sword
   in hand, `PLAYER_IA_SWORD_CS`), `Player_PutSwordInHand`, start mode 1
   (`Player_StartMode_TimeTravel`) and `Action::TimeTravelEnd`, mode 18's update (`func_80851A50`:
   the sword parented on the adult's frame 70 or the child's 87, the left hand's lists, the
   sounds) and mode 30's (`func_80851D80`). The left hand's lists set by a cutscene are
   `Player::left_hand_dlists`, cleared wherever `Player_SetModels` runs.
5. **The draw details:** `Loadout` gains `left_hand` (a cutscene's `leftHandDLists`) and
   `knife_broken` (`swordHealth <= 0`: the Biggoron group's second half), and
   `PlayerRules::hand_types` gives `sLeftHandType` and `sRightHandType` after the hands. The import
   bakes the new variants (150 to 220). Link's meshes take the tunic's colour from a port-only
   dynamic segment (`SEG_TUNIC`, 0x0F: the bake holds the loadout's tunic, the draw gives
   `currentTunic`'s). The adult's gauntlet plates (four bakes by the hands' types, the colour
   dynamic) and boots (two bakes) are drawn on the limbs' matrices, as the child's bracelet is.
   Pack format 33.
6. **The runs and presets:** `master-sword` (`ocarina-of-time` with 0x4B, 0x4F, 0x45, and what a
   child pulling the sword again has always seen: 0xA7, 0xC9, 0xC5 and 0x55; the debug start enters
   by the hall, `ENTR_TEMPLE_OF_TIME_0`, and moves to room 0: at `ENTR_TEMPLE_OF_TIME_4` the C
   draws the pull's time warp before the pull) and `master-sword-adult` (the pull's end: the
   sword, the swap, an adult). `Route::MasterSwordPull` and `Route::MasterSwordBack` with
   `Task::Pedestal`; the pull's end is checked against `master-sword-adult`.

## Consequences

- An adult can be made in play, and each age keeps its own buttons and equipment across the
  switch, as the game does. Adult starts still come from presets (`--adult` alone is a new save).
- The adult's cry on `TimeTravelEnd`'s frame 158 never plays: the adult's
  `link_demo_return_to_past` is 89 frames long, in the C too.
- The equipment page stays the stand-in (the user's call 3, Phase 20); it now meets swapped equips.
- Link's materials are dynamic in env colour: a draw without segment values shows the bake's
  (the Kokiri Tunic's).
