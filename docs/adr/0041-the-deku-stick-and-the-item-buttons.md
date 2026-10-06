# 0041: The Deku Stick and Player's item buttons

- **Status:** accepted, built in GAME-05 milestone 4b (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0016](0016-player-requests.md) (Player's requests),
  [ADR 0019](0019-inventory-and-saves.md) and
  [ADR 0021](0021-mido-the-shop-and-the-pause-stand-in.md) (the inventory, the Start stand-in),
  [ADR 0033](0033-effects.md) (effects) and
  [ADR 0040](0040-switches-torches-webs-and-the-map-data.md) (the torches and webs that read the
  stick).

## Context

The user pulled the Deku Stick forward from milestone 5 into 4b: used from C-Left only, a save
preset and the Start stand-in putting owned sticks there, the C buttons in full and the pause
menu staying in milestone 5. Milestone 4a had ported the stick's readers (`Obj_Syokudai`,
`Bg_Ydan_Sp`, `Player_IsBurningStickInRange`). Player's item code was the sword's path only:
B's item, `Player_UseItem` taking an action param, `heldItemId` kept as an action param, no init
functions, no item change sounds.

Six things needed a decision:
- **How whole the item functions are.** `Player_UseItem` and `Player_InitItemAction` have branches
  for items no button can hold yet (nuts, the lens, spells, masks, the ocarina and bottles,
  explosives, the hookshot, the bow and slingshot), which start actions and actors not ported.
- **The save at hand.** `Player_UseItem` reads `AMMO()` and the explosives' list, but it is
  reached from `Player_SetupAction` (through `func_80834644` and `Player_FinishItemChange`), which
  the port calls with the game data only.
- **Water.** The scope said "put out in water". The C does it through the interface:
  `func_80083108` disables B and the C buttons for `Player_GetEnvironmentalHazard`'s values from
  `PLAYER_ENV_HAZARD_UNDERWATER_FLOOR` to `_UNDERWATER_FREE`, which includes `_SWIMMING`; the port had
  no hazard.
- **B with the stick out.** B's item is the Kokiri Sword.
- **A debug start by a lit torch.** Room 0's golden torches are lit by switch 0x27, a temporary
  flag (0x20 to 0x3F) no save holds.
- **The exit's route** round room 0's middle floor.

## Decision

- **`Player_ProcessItemButtons`, `Player_UseItem`, `Player_InitItemAction` (with
  `sItemActionInitFuncs`), `Player_StartChangingHeldItem`, `Player_UpperAction_ChangeHeldItem`,
  `Player_FinishItemChange`, `Player_CanUpdateItems`, `Player_UpdateItems`, `func_80837818`,
  `Player_CanSpinAttack`, `func_80842AC4`, `func_80842A88`, `func_80842B7C`, `func_80842CF0` and
  `Player_UpdateBurningDekuStick` are ported whole.** The branches for items no button can hold
  keep their checks; where they would start something unported (the nut throw's action, the
  lens's magic, the spells' and cutscene items' `unk_6AD` 4, `En_Bom`, `Arms_Hook`) they log
  (the user's choice). `heldItemId` is an item (`ITEM_*`) again, `heldItemButton`,
  `currentMask`, `unk_85C`, `unk_858` and `unk_834` are Player's fields.
- **Player keeps a view of the ammo** (`ammo_view`, the save's `inventory.ammo`) and of the
  explosives' count, refreshed at the start of its update and after its own
  `Inventory_ChangeAmmo`: `Player_UseItem` reads those, so `Player_SetupAction` needs no save.
  Player updates before every category that could change them within its frame.
- **The C's water:** `Player_GetEnvironmentalHazard` is ported whole (a hot room, under water
  past 80 or 300 frames, swimming; its hot room and iron boots texts once, `envHazardTextTriggerFlags`)
  on the play state, called where `Interface_Update` calls it, and `func_80083108`'s water branch
  (B and the C buttons disabled, the hookshot's C button kept on the floor in the iron boots) with
  `sEnvHazard`. In water no button has the stick, and the swim's actions run the item code
  (`Player_TryActionHandlerList` with the upper body), so the first swimming frame puts it away:
  it's out (`unk_860` zeroed). The sword goes away in water the same way. (A first reading of the C
  missed that `_SWIMMING` lies inside that range, and the port briefly let the stick burn on in
  water; the user saw it by hand.)
- **B is the sword's:** with the stick out, B changes to the sword (`Player_UseItem` with another
  item); the stick is swung by pressing its C button again (the item in hand: `sUseHeldItem`).
  Faithful, tested.
- **The stick in hand** is drawn where `Player_PostLimbDrawGameplay` draws it: the left hand's
  matrix, translated and turned, stretched by `unk_85C`, `gLinkChildLinkDekuStickDL` (an object
  mesh already in the pack), in Link's OPA list with his hit flash. Its tip is tracked every
  frame (`melee_weapon_info[0]`), the weapon info only while it swings. `Effect_Ss_Stick` is
  ported whole (the broken half; the adult's blade too).
- **The pause menu's stand-in** also puts owned sticks on an empty C-Left for child Link
  (`SaveContext::equip_item_on_c_left`, as `KaleidoScope_UpdateItemEquip` swaps the C buttons).
  A preset, `deku-tree-sticks`: `deku-tree-inside` and `Item_Give(ITEM_DEKU_STICKS_10)` on C-Left.
- **`--switch` on the game** sets switch flags after `Play_Init` (and `--room`), as a debug start;
  the golden torches then light on their first update. `Route::start_switches` does the same for
  the scripted runs.
- **The exit's run** (`Route::Stick`) plays the room as it is: Navi's hint by the torch is read
  with the stick held to the flame (it stays at 200), the gap at 338 to 352 degrees is jumped at
  a run, and the stick burns out just as the web burns (36 frames left on contact).

## Consequences

- C-Left takes a stick out from the debug start or after Start; the torches light it, the webs
  burn from it, timed torches light from it; a hit breaks it; it burns down.
- `Player_FinishItemChange`'s sounds are new everywhere the sword comes out
  (`NA_SE_IT_SWORD_PICKOUT`): the `mido_shop_audio` golden changed.
- Player's `Player_UpdateItems` now has the C's conditions (the main camera, no cutscene, alive):
  no item is used while an attention camera holds the view.
- In water B and the C buttons are disabled: the stick goes out and the sword away, and the
  `playthrough` golden's Link climbs out of the stream with nothing in hand.
- Milestone 5 adds the C buttons' other items onto the same functions, and the pause menu.
