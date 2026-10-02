# 0019: The inventory and saves: `SaveContext::new` is a new file, the get-item draws as baked pieces with a dynamic scroll, Link's variants by their lists, and a stand-in for the pause menu's equipping

- **Status:** accepted, built in GAME-03 milestone 1; extends ADR 0012 (actor bakes), ADR 0016 (Player's requests) and ADR 0018 (debug save presets)
- **Date:** 2026-09-28

## Context

Up to GAME-02, `SaveContext` held what play needed and no more: the health, the rupees, the
flags, B's item. Its `new` gave Link the Kokiri Sword on B and the Deku Shield, so the spikes and
the playthrough had them. GAME-03 plays Kokiri Forest from a new file, where Link starts with
neither and gets them in the forest.

**Two saves matter.**
- `Sram_InitNewSave` (`z_sram.c:162`): a new file. Three hearts, no rupees, no sword, no
  shield, the Kokiri tunic and boots.
- `Sram_InitDebugSave` (`z_sram.c:265`), which the map select loads with `fileNum` 0xFF: most
  of the inventory, the Master Sword for an adult, and for a child the Kokiri Sword on B, the
  slingshot on C-left and the Deku Shield.

The spikes' view and its goldens were made with the old defaults, which match the debug save's
child equipment.

**Getting an item draws it.** `Player_DrawGetItem` draws `GetItem_Draw(play, unk_862 - 1)` over
Link's head. `sDrawItemTable` names one of about 40 `GetItem_Draw*` functions per draw id, with
its display lists. Some functions bind `Gfx_TwoTexScroll` to segment 8 each frame, a tile scroll
that changes with the frame count. That's also what kept the placed recovery hearts from drawing
in GAME-02 (`En_Item00` draws them through `GetItem_Draw`).

**Link's model follows his equipment.** The hands, the sheath and the waist come from the model
group's lists, the shield and B's sword (`Player_OverrideLimbDrawGameplayDefault`,
`z_player_lib.c:1159`: a child's sheath with no sword on B is empty). GAME-02 baked one variant
per model group, all with the sword and the shield.

**The C doesn't equip what it gives.** `Item_Give(ITEM_SWORD_KOKIRI)` only sets the owned bit
(`OWNED_EQUIP_FLAG`). The sword goes on B only in the pause menu's equipment screen
(`KaleidoScope_UpdateEquipment`, `z_kaleido_equipment.c`: `Inventory_ChangeEquipment`,
`infTable[INFTABLE_INDEX_1DX] = 0`, `buttonItems[0] = cursorItem`), and text 0xA4 tells the
player to do that. The pause menu isn't ported.

## Decision

- **`SaveContext::new` is `Sram_InitNewSave`,** faithful to `sNewSavePlayerData`,
  `sNewSaveEquips` and `sNewSaveInventory`, with the name "LINK" (the file select's name entry
  isn't ported).
  - `SaveContext::debug` is the map select's `Sram_InitDebugSave` (`fileNum` 0xFF, its magic
    reset and enabled buttons).
  - The spikes' `PlayState::new` uses `debug`. Its child has the equipment it had before, so the
    spike goldens don't change.
  - The game, the sandbox's `--entrance` and the tests enter with `new`. The presets apply on
    top of it: `deku-tree-open` and `deku-tree-dead` now give the Kokiri Sword and the Deku
    Shield, owned and equipped as the pause menu equips them (`kokiri_sword_and_deku_shield`:
    `inventory.equipment` bits 0 and 4, `equips.equipment` nibbles 0 and 1 set to 1).
- **The save holds the C's inventory:**
  - `Inventory`: the items, the ammo, the equipment owned, the upgrades, the quest items, the
    dungeon items and keys;
  - `ItemEquips`: the buttons, their slots and the equipment worn, one for each age;
  - each scene's saved flags (`sceneFlags`, 124 scenes), loaded by `Play_Init` before the actors
    spawn and saved on the next reinit (`Play_SaveSceneFlags`), so an opened chest stays open.
  
  `Item_Give` and `Item_CheckObtainability` are ported whole, with the helpers they use
  (`Health_ChangeBy`, `Rupees_ChangeBy`, `Inventory_ChangeAmmo`, the upgrades).
- **The item tables are data from the C.** The importer parses `sGetItemTable`'s `GET_ITEM`
  rows and `sDrawItemTable` into `table/items`, and `sItemActions` into Player's data. The
  runtime never reads the C.
- **`GetItem_Draw` is baked as pieces** (`oot_game::draw`). Each `GetItem_Draw*` function is
  ported as the pieces it draws: the display lists it draws under one matrix into one buffer,
  after one setup list, with its scroll and whether the rotation is the billboard. The importer
  bakes every piece of every draw id: 167 bakes, `GetItem/<draw id>/<piece>`.
- **`Gfx_TwoTexScroll` is a dynamic segment** (`BakeSegment::Dynamic`). The bake runs its
  commands at frame 0 and marks them dynamic. The materials whose tile sizes they set read them
  from the draw's `SegmentValues`, which the draw builds each frame. It's ADR 0012's dynamic
  colour, extended to tile commands.
- **Link's variants are keyed by the lists they draw:** `player/<age>/<left hand>+<right
  hand>+<sheath>+<waist>` (`-` for none). The importer enumerates the loadouts (model group,
  fists, shield, sword), resolves each one's lists and bakes each distinct set once: 134
  variants. 16 of them carry their own faces, where a list leaves a different mouth texture in
  TMEM. The draw resolves the loadout from the save (B's sword, the shield worn), as the limb
  override does.
- **Player's requests grow** (ADR 0016): `SetParent`, `ChestOpen`, `SetCameraData`,
  `DropCollectible`, and `ItemGive`. `ItemGive` is applied after `StartTextbox`, as the C calls
  them, so a heart piece's text counts the pieces before the new one.
- **En_Box's camera is ported.** `CAM_SET_SLOW_CHEST_CS` (`Camera_Demo3`) and
  `CAM_SET_TURN_AROUND` (`Camera_KeepOn4`, with `Camera_SetCameraData`'s data2 9) run as the C
  runs them, so no ADR 0015 fallback is needed. Left out:
  - the one-point cutscene cameras of the chests that fall or appear (`OnePointCutscene_Init`,
    `OnePointCutscene_Attention`);
  - the fanfare and the sounds;
  - the dust and smoke effects (their `Rand_ZeroOne` calls are made);
  - the light, `Demo_Tre_Lgt` (a SkelCurve), and the sparkles, `Demo_Kankyo`, which spawn as
    placeholders.
- **The pause menu's equipping has a stand-in.** `SaveContext::equip_from_pause_menu` does what
  the equipment screen does for one piece. `equip_owned_unworn` equips, for every type with
  nothing worn, the first owned piece Link's age can wear (`CHECK_OWNED_EQUIP` and
  `gEquipAgeReqs`). `PlayExt::equip_owned_unworn` then runs `Player_SetEquipmentData`, as the
  menu's closing does. The game binds it to Start (Enter), when `KaleidoSetup_Update` would open
  the menu: no transition, not `Play_InCsMode`, and (the port's own condition) no message box.
- **A debug start reaches rooms the port can't walk to yet.** The game's `--room N --at
  x,y,z,yaw`, with `--entrance`, change room after `Play_Init` (`Room_RequestNewRoom`, a frame,
  `Room_FinishRoomChange`) and put Link there. `game-sword-chest.bat` uses it for room 2's chest, which
  is behind the crawlspace (GAME-03 milestone 2).

## Consequences

- **A new save has no sword until the chest.** The exit's B comes from the stand-in, not from
  the chest: after the text, Start puts the sword on B. A test checks both halves: the chest
  leaves B empty (`ITEM_NONE`, the sword owned but not worn), and the stand-in fills it.
- **The stand-in isn't the menu.** It never swaps a worn piece for another, and it equips every
  empty type at once. When the pause menu is ported, it replaces the stand-in, and the key goes
  to the menu.
- **The goldens don't change.** The spike cases use the debug save, whose child equipment is the
  old default. The playthrough and the Deku Tree render enter with the preset, which now gives
  the same sword and shield through the owned and worn bits.
- **The pack grows** to format 8: 51.8 MB (from 47.35), 474 actor bakes, 167 get-item bakes and
  134 Link variants.
- **A big chest opens without its light.** The one visible difference in the opening, until
  `Demo_Tre_Lgt` is ported.
