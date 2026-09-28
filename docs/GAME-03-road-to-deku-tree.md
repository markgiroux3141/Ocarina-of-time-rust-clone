# GAME-03: the road to the Deku Tree

**Goal:** Phase 4 of [ROADMAP.md](ROADMAP.md): a new save plays Kokiri Forest the game's way, up
to entering the Deku Tree:
- wake up;
- get the Kokiri Sword from its chest and the Deku Shield from the shop;
- Mido steps aside;
- the Deku Tree's cutscene opens his mouth.

| # | Milestone | Status |
|---|---|---|
| 1 | The inventory and getting items: `SaveContext`'s inventory, `Item_Give`, Player's get-item flow, `GetItem_Draw`, `En_Box`, Link's equipment on his model, the B and C items on the HUD | done |
| 2 | Crawlspaces and the training area: Player's crawl, the `CRAWLSPACE` camera, what stands between the start and the sword | to do |
| 3 | Mido and the shop: `En_Md`, `En_Ossan` (the Kokiri shop), `En_GirlA` | to do |
| 4 | Cutscenes, first part (`z_demo.c`): the scripts, `csCtx`, the entrance triggers, Player's cutscene modes | to do |
| 5 | Navi: `En_Elf` as Link's fairy, the Kokiri's fairies, the game's opening | to do |

**Phase exit:** a headless run from a new save to the Deku Tree scene, the game's way,
replacing GAME-02 milestone 4's flag preset.

The working rules are the same as for GAME-01 and GAME-02:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C.

Decisions are in [docs/adr/](adr/README.md) (0019 on).

## Milestone 1: the inventory and getting items

**Answer:** done. On a new save, Link opens the Kokiri Sword's chest: the slow opening with its
camera, the sword held up over his head, text 0xA4 with the sword's icon, and the sword owned.
B gets the sword when Start is pressed, the pause menu's stand-in: in the C, only the pause
menu's equipment screen puts it on B (text 0xA4 says so).

The tests: 205 pass, 1 ignored (195 before). The goldens are unchanged: 80 of 80 identical.

### What was built

1. **The save's inventory** (`oot_game::save`, [ADR 0019](adr/0019-inventory-and-saves.md)).
   - `Inventory` (the items, the ammo, the equipment owned, the upgrades, the quest items, the
     dungeon items and keys) and `ItemEquips` (the buttons, their slots and the equipment worn,
     one for each age), as `z64save.h` lays them out.
   - **`SaveContext::new` is `Sram_InitNewSave`** (`z_sram.c:162`): three hearts, no rupees, the
     Kokiri tunic and boots, no sword, no shield, nothing on the buttons.
   - **`SaveContext::debug` is the map select's `Sram_InitDebugSave`** (`z_sram.c:265`,
     `fileNum` 0xFF): most of the inventory, and for a child the Kokiri Sword on B, the
     slingshot on C-left and the Deku Shield. The spikes' `PlayState::new` uses it.
   - Each scene's saved flags (124 scenes): `Play_Init` loads them before the actors spawn,
     and the next reinit saves them (`Play_SaveSceneFlags`).
   - The C's macros and helpers: `CUR_EQUIP_VALUE`, `ALL_EQUIP_VALUE`, `CHECK_OWNED_EQUIP`,
     `Inventory_ChangeEquipment`, `Inventory_ChangeUpgrade`, `CUR_UPG_VALUE`, `CUR_CAPACITY`,
     `INV_CONTENT`, `AMMO`, `B_BTN_ITEM`, `C_BTN_ITEM`, the item-get and inf tables.
   - **The presets** (`deku-tree-open`, `deku-tree-dead`) now give the Kokiri Sword and the Deku
     Shield owned and worn, as the pause menu equips them:
     - `Item_Give` sets `inventory.equipment` bits 0 and 4 (owned);
     - `Inventory_ChangeEquipment` sets `equips.equipment` nibbles 0 and 1 to 1 (worn);
     - B gets `ITEM_SWORD_KOKIRI`, and `infTable[INFTABLE_1DX_INDEX]` is cleared.

     That's what Mido checks (`z_en_md.c`).
2. **`Item_Give` and `Item_CheckObtainability`** (`oot_game::item`), ported whole with
   `Health_ChangeBy` (double defence included), `Rupees_ChangeBy`, `Inventory_ChangeAmmo` and
   the upgrades. What Kokiri Forest gives: the Kokiri Sword and the Deku Shield (the owned bits
   only), sticks and nuts (the first one sets the upgrade and the slot), rupees, recovery hearts,
   pieces of heart (`questItems`' top bits).
3. **The item tables from the C** (`table/items`): the importer parses `sGetItemTable`'s
   `GET_ITEM` rows (the item, the draw id with its chest animation's sign, the text, the
   object) and `sDrawItemTable` (each draw id's function and lists), and `sItemActionParams`
   into Player's data.
4. **Getting items.**
   - `func_8002F434` (later decomps' `Actor_OfferGetItem`) and `func_8002F554` (it with 50 and
     10), `Actor_HasParent`, `func_8002DBD0` (world to actor coordinates), in
     `oot_game::get_item`.
   - Player:
     - the get-item interrupt `func_8083E5A8`: a chest's branch (the slow open with
       `gPlayerAnim_clink_demo_Tbox_open` and `CAM_SET_SLOW_CHEST_CS`, or the kick), and a
       touched item's;
     - `func_8083A434`, `func_8084E6D4` (the get-item action: `get_itemA`/`get_itemB`,
       `func_80835EA4`'s `CAM_SET_TURN_AROUND`, frame 21's `func_808332F4`),
       `func_8084DFF4` (the text, then `Item_Give`), `func_8084DFAC` and `func_8084DF6C`;
     - the do-action's Open and Grab;
     - the item held up (`Player_DrawGetItem`).
   - `En_Item00`: its get-item offers, the parent-based kill, the shield and tunic objects,
     and the drop's `FLEXIBLE` branch (magic, seeds, arrows, bombs).
5. **`GetItem_Draw`** (`oot_game::draw`). Each `GetItem_Draw*` function is ported as its
   pieces, and the importer bakes all of them: 167 bakes, `GetItem/<draw id>/<piece>`.
   `Gfx_TwoTexScroll` is a dynamic segment (`BakeSegment::Dynamic`), fed each frame's tile
   sizes. The placed recovery hearts draw now.
6. **The item's text.** `MESSAGE_ITEM_ICON` draws `icon_item_static`'s 32×32 icon, or
   `icon_item_24_static`'s 24×24 one, as `message/item<XX>` bakes. A piece of heart's text
   (0xC2) counts the pieces owned.
7. **`En_Box`** (`oot_actors::en_box`), the whole overlay:
   - the chest types (the appearing and falling ones included);
   - the offer, the opening for Link's age, and the treasure flag;
   - its bakes (the chest, the boss key chest's, the translucent lid);
   - the chest's cameras, `Camera_Demo3` (`CAM_SET_SLOW_CHEST_CS`) and `Camera_KeepOn4`
     (`CAM_SET_TURN_AROUND`).

   Left out, and logged: the one-point cutscene cameras, the song chests, drawing the
   lens-hidden chests, the fanfare, the sounds, the effects. The light (`Demo_Tre_Lgt`) and the
   sparkles (`Demo_Kankyo`) spawn as placeholders.
8. **Link's equipment on his model.**
   - Player reads the save: `Player_SetEquipmentData` (the shield, the tunic, the boots, B's
     sword). `func_80833DF8` uses B's item from the save.
   - Link's variants are keyed by the lists they draw (`player/<age>/<L>+<R>+<sheath>+<waist>`):
     134 variants, 16 with their own faces.
   - The child's empty sheath without a sword on B follows
     `Player_OverrideLimbDrawGameplayDefault` (`z_player_lib.c:1159`).
9. **The HUD's B and C items.**
   - `func_80083108`'s button rules (`INFTABLE_1DX`, what each item is allowed where).
   - The B and C icons (`hud/item00` to `hud/item59`) and the C items' ammo counts
     (`Interface_DrawAmmoCount`, `hud/ammo_digit0` to `9`).
10. **The pause menu's stand-in.**
    - `SaveContext::equip_from_pause_menu` does what the equipment screen does for one piece.
    - `equip_owned_unworn` equips the first owned, wearable piece of every type with nothing
      worn (`CHECK_OWNED_EQUIP`, `gEquipAgeReqs`).
    - The game binds it to Start (Enter), then runs `Player_SetEquipmentData`.
11. **A debug start:** `--room N --at x,y,z,yaw` with `--entrance` (the game): after
    `Play_Init`, change to that room and put Link there. `scripts\run\game-sword-chest.bat`
    starts in front of the chest.
12. **Engine:** DynaPoly `set_collision_disabled` and `disable_ceiling_collision`; a downward
    raycast that skips one bg actor (`BgCheck_EntityRaycastDown` with an actor's own
    collision).
13. **Pack format 8:** `table/items`, the get-item bakes, the `En_Box` bakes, the HUD's icons
    and digits, the message box's item icons, Link's variants by list.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 205 passed, 1 ignored |
| The sword chest (`oot_actors --test chest`: `the_kokiri_sword_chest_on_a_new_save`) | A new save: equipment 0x1100, B `ITEM_NONE`, Link's sheath empty. In front of room 2's chest (params 0x04E0), pushed out to 34 by its collision: the chest offers `-GI_SWORD_KOKIRI`, the do-action is Open. A: `clink_demo_Tbox_open`, Link at 29.43 in front of it facing it, `unk_1F4` 1, `CAM_SET_SLOW_CHEST_CS`. At its end: `link_demo_get_itemA`, `unk_850` 2, `CAM_SET_TURN_AROUND`. Frame 21: `unk_862` 0x74, `GetItem/73/0` drawn. Then text 0xA4 with `message/item3B`; equipment 0x1101 (owned), not worn, B still empty. A through both boxes to `TEXT_STATE_CLOSING`: Link stands, `GI_NONE`, treasure flag 0 set. B does nothing. Start's stand-in: sword value 1, B `ITEM_SWORD_KOKIRI`, `gLinkChildSwordAndSheathNearDL` in the sheath. B draws it and slashes |
| A piece of heart (`--test chest`: `a_piece_of_heart`) | A spawned `En_Item00` heart piece at Link's feet, one piece owned: held up (`unk_862` = the table's gi), text 0xC3 (0xC2 plus the one owned, before `Item_Give`), then two pieces; taken; no container |
| The debug start (`oot --test start`) | `--entrance ENTR_SPOT04_0 --room 2 --at -232,178,2211,0`: room 2, Link there, a new save, the chest offering the sword |
| `Item_Give` and the saves (unit tests) | Rupees, hearts, a piece; the sword's and shield's owned bits; sticks and nuts with their upgrades; a new file has nothing to fight with; the map select's child has the Kokiri Sword and Deku Shield; the presets own and wear both |
| `GetItem_Draw` (unit tests) | The recovery heart's scroll; `Matrix_ReplaceRotation` keeps the scale and the place |
| Golden traces and renders | 80 of 80 identical. The spike cases use `SaveContext::debug` (the old defaults); the playthrough and the Deku Tree render enter with the preset, which now owns and wears the same sword and shield. Nothing recorded (see `golden/README.md`) |
| Import | 11.9 s; 51.8 MB; format version 8 (474 actor bakes, 167 get-item bakes, 134 Link variants), into `out/data05` |
| The windows | Played by hand (`scripts\run\game-sword-chest.bat`): the chest's opening looks right; after the text and Start, the sword is on B and swings. The Project64 comparisons below are deferred polish |

### Decisions

- **[ADR 0019](adr/0019-inventory-and-saves.md):**
  - `SaveContext::new` is `Sram_InitNewSave`; `debug` is the map select's `Sram_InitDebugSave`,
    for the spikes;
  - the item tables are data from the C;
  - `GetItem_Draw` is baked as pieces, with `Gfx_TwoTexScroll` as a dynamic segment;
  - Link's variants are keyed by the lists they draw;
  - Player's requests grow (`ItemGive` after `StartTextbox`);
  - En_Box's cameras are ported, its light is a placeholder;
  - the pause menu's equipping has a stand-in on Start.
- **B gets the sword from the stand-in, not the chest.** The exit asks for B to get the sword.
  In the C, `Item_Give(ITEM_SWORD_KOKIRI)` only owns it, and the equipment screen puts it on B,
  so the exit's B comes from Start. The test checks that the chest leaves B empty.
- **Link's push-back after the opening is the C's.** While the opening's animation moves him
  (`moveFlags & 0x80`), `Player_UpdateCommon` skips his bgcheck (`z_player.c:10304`). When it
  ends, the chest's collision pushes him from 29.4 back out to 34 in one frame.

### Known gaps

- **The pause menu** isn't ported. Start equips every empty type's first owned piece; it never
  swaps pieces.
- **The crawl** (milestone 2): room 2's chest is reached by the debug start or the test's room
  change, not by walking.
- **`En_Box`:**
  - no light or sparkles over the big chest (`Demo_Tre_Lgt`, `Demo_Kankyo`: placeholders);
  - no fanfare or sounds;
  - no one-point cutscene cameras for the chests that appear or fall;
  - the song chests never appear (no ocarina);
  - the lens-hidden chests aren't drawn until opened.
- **The C items** aren't used (`func_80833DF8` only reads B), and nothing puts one on C yet, so
  their icons and ammo counts only draw for a save that has them.
- **Not in scope:** the effects (`EffectSs`), the magic meter, the B button's ammo count
  (minigames), the file select's name.
- **Carried over:** time passing, culling, the exit's circle wipe, small keys, audio, the
  cutscenes, Navi.

## Visual polish, deferred

Side-by-side comparisons against Project64 on the same ROM, for when the look is polished. They
don't block milestones: behaviour is checked against the C by the tests.

1. **The sword chest** (`scripts\run\game-sword-chest.bat`, against a new file):
   - the slow opening: Link's animation, the lid, the camera's move (`CAM_SET_SLOW_CHEST_CS`);
   - the turn to face the camera (`CAM_SET_TURN_AROUND`) and the sword held up: its size, its
     place over his head, when it appears;
   - text 0xA4: the icon's place and size, the quick text of the first line, the two boxes;
   - Link's stance as the text closes, and the push-back from the chest;
   - the missing light over the chest (known: `Demo_Tre_Lgt`).
2. **The HUD's B button:** empty on a new file, then the sword's icon once it's equipped. (The
   C buttons' icons can't be seen yet: the HUD only runs after `Play_Init`, whose new file has
   nothing on C.)
3. **The placed recovery hearts** in Kokiri Forest: their model, size and the texture's
   scroll.
4. **Link's model:** the empty sheath on a new file, the sword in it after equipping.
5. **From GAME-02, still open:**
   - the message box on the sign (0x031F) and on the Kokiri child (0x100A): the typing speed,
     the box's position, the glyphs' edges, the end icon's flashing;
   - the talk camera on the sign and the child (KEEP3), and in Link's house (KEEP0);
   - the HUD's layout and fades, and the A button's flip;
   - Z-targeting a Kokiri child: the camera and the bars;
   - Link's house: the pivot view with its skybox, and C-Up's fixed view;
   - the Deku Tree's mouth, open and dead (`--preset deku-tree-open`, `--preset
     deku-tree-dead`);
   - the stream at the ford (x ≈ 1250–1400): wading, and the climb out.

## Recommended next step

**GAME-03 milestone 2: crawlspaces and the training area,** scoped in
[ROADMAP.md](ROADMAP.md) (Phase 4): Player's crawl, the `CRAWLSPACE` camera, and whatever stands
between the start and the sword.

**Exit:** a scripted run from Link's house to the sword chest, which replaces the debug start.
