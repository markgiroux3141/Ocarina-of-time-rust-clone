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
| 2 | Crawlspaces and the training area: Player's crawl, the `CRAWLSPACE` camera, what stands between the start and the sword | done |
| 3 | Mido and the shop: `En_Md`, `En_Ossan` (the Kokiri shop), `En_GirlA`, the pause menu's stand-in in the play frame | done |
| 4 | Cutscenes, first part (`z_demo.c`): the scripts, `csCtx`, the entrance triggers, Player's cutscene modes | done |
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
  change, not by walking. *(Done in milestone 2.)*
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

## Milestone 2: crawlspaces and the training area

**Answer:** done. On a new save, a scripted run walks from Link's bed to the Kokiri Sword's
chest the game's way and opens it:
- out of the house and down the ladder;
- west through the village and up the ramp onto the plateau;
- into the crawlspace by its sign, through it and out into the training area (room 2);
- round the boulder's corridors, waiting where its path doesn't reach;
- up to the chest, and its text read.

The debug start stays in `game-sword-chest.bat` as a shortcut.

The tests: 222 pass, 1 ignored (205 before). The goldens:
- the Deku Tree playthrough's trace differs only in its actor count (the wonder items), and is
  re-recorded;
- the new run's trace is a new case;
- 81 of 81.

### What was built

1. **The scene path lists** (`SCENE_CMD_ID_PATH_LIST`, [ADR 0020](adr/0020-crawlspaces-paths-and-knockdown.md)).
   - `oot_import::scene::path_list` reads them like the exit list: up to the next pointer
     target, while each entry points at points in the scene file.
   - They go into the pack as `LayerData.paths` (`oot_game::scene::Path`, format 9), and play
     reads them as `PlayState::setup_path_list` (`play->setupPathList`).
   - `ootx scene-info` prints them. Kokiri Forest has three; path 2 is the boulder's loop.
2. **Player's crawl** (`oot_actors::player`).
   - `func_8083F0C8`, the crawlspace's branch of `func_8083F7BC` (interrupt 5):
     - at a crawlspace's wall (`WALL_FLAG_4`, `_5`), for a child, within 8 of its triangles'
       middle, A says "Enter" (`PLAYER_STATE2_16`, the do-action `DO_ACTION_ENTER`);
     - on A: `PLAYER_STATE2_18`, Link lined up and facing the wall,
       `gPlayerAnim_link_child_tunnel_start` with `moveFlags` 0x9D, and `func_8083A40C` once
       the item is away.
   - `func_8084C760`, the crawl: the speed is the stick's `rel.stick_y` × 0.03, backwards too.
   - `func_8083F570`: head first into the far wall, out with `tunnel_end`; feet first into the
     mouth's inner wall, out with `tunnel_start` played backwards.
   - `func_8084C81C`: standing at the end, the crawl over.
   - What the crawl flag changes elsewhere was ported already: the smaller bgcheck, no wall
     interaction, no exits, no do-action, the disabled buttons. What was missing:
     - no water check or damage while crawling (`Player_UpdateCommon`);
     - no limbs drawn with the camera inside Link (`Player_OverrideLimbDrawGameplay_80090440`,
       from `projectedPos.z`).
   - The rest of `func_8083F7BC`: A at a pushable wall (`WALL_FLAG_6`) shows "Grab"; pushing
     itself (`func_8083F72C`) isn't ported.
3. **`Camera_Subj4`** (`CAM_SET_CRAWLSPACE`), the function `sCameraSettings` gives the setting.
   - Its first call each frame asks for the second at the end of `Play_Draw` and keeps the
     frame's `xzSpeed`.
   - The second eases in over 10 frames, then, while Link moves, puts the eye on the crawlspace's
     line at Link, bobbing and swaying, with `at` 10 ahead.
   - It moves Link onto the line, to the ground, facing along it (`camera->player`), through
     `GameCamera::player_write`, which `PlayState` applies right after the update.
   - The line's ends are the bg camera's second and second-to-last points
     (`Camera_GetBgCamFuncDataUnderPlayer`).
4. **`En_Goroiwa`** (`oot_actors::en_goroiwa`), the whole overlay.
   - What Kokiri Forest's boulder (0x0C02) uses:
     - rolling round path 2 at `R_EN_GOROIWA_SPEED` 920 (9.2, reached in steps of 0.3);
     - `EnGoroiwa_MoveAndFall` with gravity and the floor check (bit 10);
     - the loop back to point 0 (`ENGOROIWA_LOOPMODE_ONEWAY`);
     - the roll (`EnGoroiwa_UpdateRotation`, on `oot_game::sys_matrix`'s `MtxF`);
     - the hit on Link: back the way it came if he's ahead, the knockdown asked of him, a hop,
       50 frames without colliding, a 6-frame wait.
   - Ported too, though this boulder never reaches them: the climbs and drops between points
     (`EnGoroiwa_MoveUp`, `EnGoroiwa_MoveDown`), the round trip, the breaking loop.
   - Left out: the sounds, the quake of a drop, the dust, splashes, ripples and fragments (their
     `Rand_ZeroOne` calls are made), the circle shadow.
5. **Player's knockdown**, pulled forward from Phase 6 as far as the boulder needs it:
   - `func_808382DC`: the crush and void-floor respawns, the knockback an actor asks for
     (`unk_8A0` to `unk_8A8`, cleared each update), the body hit, the hurting walls and
     floors;
   - `func_80837C0C` for kinds 0 to 2, with `Health_ChangeBy` and the invincibility timer;
   - the stagger (`func_8084370C`), the knockdown (`func_8084377C`), lying down
     (`func_80843954`), getting up (`func_80843A38`);
   - `Player_InflictDamage`, `Player_InBlockingCsMode`;
   - the invincibility timer's other users: the roll (`func_80837AFC(-10)` at frame 8), and the
     fall damage, which now takes its half heart or heart (`func_80843E64`).

   Actors ask through `oot_game::actor_ctx::func_8002f698` and its short forms
   (`PlayerIface::set_knockback`).
6. **`En_Wonder_Item`** (`oot_actors::en_wonder_item`), the whole overlay.
   - Room 2's two (0x123F): mode 2, a proximity drop of a green rupee that collects itself.
   - Room 0 also has an interact switch, both multitags and their tag points.
   - The tag points' shared positions are overlay statics, kept in the play state
     (`PlayState::overlay_statics`).
   - The bomb soldier's `En_Heishi2` spawns as a placeholder.
7. **The playthroughs as routes** (`oot_actors::playthrough::Route`):
   - `DekuTree` (unchanged) and `SwordChest`;
   - new steering tasks: `Crawl`, `WaitBoulder`, `Hurry`, `OpenChest`;
   - new steps: `Crawlspace`, `TrainingArea`, `Boulder`, `Chest`;
   - the sandbox's `--script sword-chest`;
   - the golden case `sword_chest` (its trace).
8. **Run scripts:** `game-new-save.bat` (the route by hand), `test-sword-route.bat`,
   `sandbox-sword-chest.bat`; `game-sword-chest.bat` kept as a shortcut.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 222 passed, 1 ignored |
| The Kokiri Sword run (`playthrough` test `a_new_save_to_the_kokiri_sword`) | 1916 frames. House at frame 30, out of the door 154, down the ladder 257, into the crawlspace 981, out in room 2 1199, past the boulder 1463, the chest done 1916. Two waits for the boulder: 0 and 94 frames. At the crawl: "Enter" shown before A; Link at x -785 facing +z, `tunnel_start`. Out: room 2, the crawlspace's camera seen while crawling, bg camera 14 (`DUNGEON0`) after. Past the corridors: still 3 hearts, both wonder items' green rupees. The chest: text 0xA4 only, equipment 0x1101 (the sword owned), B still empty, treasure flag 0 set, Link standing. The boulder never within reach |
| The Deku Tree run (`kokiri_forest_to_the_deku_tree`) | Passes unchanged |
| The crawl (`crawl`: 5 tests) | "Enter" at the mouth; A: `func_8083F0C8`'s place (x -785, 1059 less the wall distance), yaw 0, `tunnel_start`, `moveFlags` 0x8D after the first frame (0x9D less 0x10); the crawl after the put-away; the speed 53 × 0.03; on the line (x -784, y 120, yaw 1) and not drawn while crawling; room 2 loaded by z 1219..1300; out at the far wall with `tunnel_end` and standing past z 1379 on bg camera 14. `Camera_Subj4`'s ease-in and its eye on the line. Backwards out of the mouth (`tunnel_start` at speed -1) into room 0 on `NORMAL0`. No "Enter" 12 off the middle; none for adult Link |
| The boulder (`boulder`: 4 tests) | Placed at path 2's point 0, facing point 1; steps of 0.3 to 9.2; 36 frames a side, snapping onto each point; turned by the distance over 59.5 each frame; no path (0xFF), no boulder. The hit on Link standing in its way: a quarter heart, 20 frames of invincibility, `front_shit`; next frame knocked down backwards at 2 (`front_downA`), then `front_downB`, `front_down_wake`, standing, then the timer counting down. The boulder reversed, hopping (5 up, 0.15 of its speed), 50 frames without colliding, back on its way after a 6-frame wait |
| The wonder items (`wonder_item`: 3 tests) | Room 2's proximity drop: nothing at 60, a green rupee at 40 that collects itself. Room 0's free multitag: its tag points gone at spawn, a blue rupee and switch 0x20 after both points. The ordered one: gone with nothing on the wrong point first, the rupee and switch 0x23 in order, gone when its 81 frames run out |
| The path lists (`oot_import --test pack`: `path_lists_match_the_xmls`) | 82 layer headers match the XMLs' `NumPaths` (42 headers have lists the XMLs don't name); the pack's points are the ROM's; Kokiri Forest's path 2 as listed |
| `sys_matrix` (unit tests) | A yaw and a pitch read back through `Matrix_MtxFToYXZRotS`; `RAD_TO_BINANG` truncates |
| Golden traces and renders | `playthrough` re-recorded: only its actor counts differ (frames 125 to 1434: the four tag points, then the ford's multitag timing out). New case `sword_chest` (the same bytes over two runs). 81 of 81 |
| Import | 12.7 s; 51.8 MB; format version 9, into `out/data06` |
| The windows | Not yet played by hand: `scripts\run\game-new-save.bat` |

### Decisions

- **[ADR 0020](adr/0020-crawlspaces-paths-and-knockdown.md):**
  - the path lists are pack data, checked against the XMLs;
  - `Camera_Subj4` writes Player back through `player_write`;
  - Player's knockdown is pulled forward from Phase 6;
  - overlay statics live in the play state;
  - `sys_matrix.c`'s rotations are ported on an `MtxF`;
  - the playthroughs are routes of steering tasks;
  - the crawl's one-point cutscenes aren't ported.
- **The run waits for the boulder rather than timing it.** `WaitBoulder` waits in two places its
  path doesn't reach: the bottom corridor west of the loop, until the boulder is on its way up
  the middle; the alcove west of the top-left corner, until it's on its way along the top. From
  each, Link follows it at full tilt. The walks are shorter than its 144-frame lap with about 25
  frames to spare.
- **The chest is opened from its mound running.** The Kokiri Sword's chest stands on a mound whose
  front faces are steep (normal y 0.55). A slow walk stops at its foot; a run gets up it, as a
  player's does.
- **A box break waits for A** in the steering (`MSGMODE_TEXT_AWAIT_INPUT`): `Message_GetState`
  reports it as its fallback, `TEXT_STATE_DONE_FADING`, and text 0xA4 has one.
- **What `Camera_Subj4` writes wins for that frame.** On the frame Link climbs out, Player sets
  his yaw from the wall, and the camera's draw-time update, still the crawlspace's, sets it
  along the line again. That's the C's order; the tests expect it.

### Known gaps

- **The crawl:**
  - the one-point cutscenes on the way out (9601, 9602): the next floor's bg camera takes over;
  - the crawl's sounds (`func_80832924`'s tables, `Camera_Subj4`'s `func_800F4010`);
  - pushing and pulling walls and blocks (`func_8083F72C`).
- **Damage:**
  - no dying: at 0 health Link carries on (logged);
  - kinds 3 and 4 (frozen, shocked), the hit while swimming, burning: logged, not ported;
  - the hit's red flash (the draw's fog), the rumble and the sounds.
- **`En_Goroiwa`:** the effects (dust, splashes, ripples, fragments), the drop's quake, the
  sounds, the circle shadow.
- **`En_Wonder_Item`:** the bomb soldier is a placeholder; the debug arrows aren't drawn.
- **Carried over:** the pause menu (Start's stand-in), the shop, Mido, the cutscenes, Navi,
  audio. *(The shop and Mido: done in milestone 3.)*

## Milestone 3: Mido and the shop

**Answer:** done. On a new save, a scripted run goes from Link's bed to past Mido, the game's way:
- the Kokiri Sword (milestone 2's run), with room 2's two blue rupees picked up on the way;
- Start (the pause menu's stand-in, now in the play frame) puts the sword on B;
- back past the boulder and through the crawlspace, then 42 rupees, all of them the C's placed
  ones;
- in the Kokiri shop, the Deku Shield bought through the shopkeeper's browsing, the get-item
  flow and the price;
- Start again to wear the shield;
- east over the ford to Mido: he says 0x1033, sets `EVENTCHKINF_04`, walks aside along path 1,
  and Link walks past him.

The tests: 238 pass, 1 ignored (222 before). The goldens:
- the Deku Tree playthrough's and the sword run's traces are re-recorded (Mido's `Rand` calls
  and his fairy);
- the new run's trace is a new case;
- 82 of 82.

### What was built

1. **`En_Md`** (`oot_actors::en_md`), Mido, the whole overlay.
   - `EnMd_Init`:
     - `EnMd_ShouldSpawn`: Kokiri Forest before Zelda's letter and the goodbye
       (`EVENTCHKINF_40`, `_1C`); his house after them, for child Link; the Lost Woods always;
     - his fairy (`En_Elf`, `FAIRY_KOKIRI`: a placeholder);
     - blocking, or at path 1's last point with `EVENTCHKINF_04` (`EnMd_SetMovedPos`).
   - Blocking (`func_80AAB948`): 60 from his home towards Link, facing him, his animation's speed
     by how far Link has gone round him. His collider is immovable, so Link can't pass.
   - Talking (`func_800343CC` through `oot_game::npc::talk_update`):
     - his text (`EnMd_GetText`: Kokiri Forest, his house, the Lost Woods);
     - the conversation's state (`func_80AAAF04`): 0x102F sets `EVENTCHKINF_02` and
       `INFTABLE_0C` as it closes; 0x1033 (and 0x1067) returns 2;
     - the box count (`func_80AAAC78`), which picks his gestures for each box
       (`func_80AAAA24`: eleven animation sequences, `func_80AAA274` to `func_80AAA890`, some
       played backwards by `func_80AAA250`).
   - Stepping aside: `EVENTCHKINF_04`, then along path 1 at 1.5 (`func_80AABD0C`,
     `EnMd_FollowPath` with `Math_FAtan2F`'s double product). Within 10 of its last point he
     stops and stays (`func_80AAB8F8`). With the Kokiri Emerald, the goodbye instead:
     `MSGMODE_PAUSED`, then `Message_CloseTextbox`, `EVENTCHKINF_1C` and he goes.
   - The head and torso tracking (`func_80AAB158`: `func_80034A14` preset 2,
     `func_800347E8`), the idle sway, the blinks, and the fade beyond 400 of Link
     (`func_80AAB5A4`; `func_80034DD4` is new in `oot_game::npc`).
   - The draw (`EnMd_Draw`): six bakes, an eye and a pass each:
     - opaque at full alpha (`func_80034BA0`), translucent while fading (`func_80034CC4`);
     - `EnMd_OverrideLimbDraw`'s head and torso turns and sway, and `EnMd_PostLimbDraw`'s
       focus.
   - The Lost Woods (`SCENE_SPOT10`), ported against `msgCtx.ocarinaMode`:
     - his texts;
     - `PLAYER_STATE2_23` near him;
     - waiting for Saria's Song (`func_80AABC10`).

     Nothing there plays the ocarina: the check (`func_8010BD58`) is logged if it would start.
   - His house (`SCENE_KOKIRI_HOME4`): his texts (0x1028, 0x1046, `EVENTCHKINF_0F`), always
     opaque. Its layer 0 has no `En_Md`.
2. **`En_Ossan`** (`oot_actors::en_ossan`), the shopkeepers.
   - **All 27 states** (`sStateFunc`) and their helpers:
     - talking and the choice (`EnOssan_State_Idle`, `_StartConversation`,
       `_FacingShopkeeper`);
     - the camera's turn to a shelf and back (`EnOssan_UpdateCameraDirection`:
       `Camera_SetCameraData`'s `data2`, which `Camera_Data4` adds to bg camera 1's yaw);
     - browsing (the stick's accumulation, `EnOssan_CursorRight`, `_CursorLeft`,
       `_CursorUpDown`);
     - choosing (the item off the shelf and back: `EnOssan_TakeItemOffShelf`,
       `_ReturnItemToShelf`);
     - buying: `EnOssan_HandleCanBuyItem`'s answers (0x84, 0x85, 0x86, 0x96, the get-item flow
       through `func_8002F434` within 120);
     - the price after the item's text (`buyEventFunc`), 0x6B, continuing or ending
       (`EnOssan_EndInteraction`);
     - the other shops' states: the milk, the egg, the Goron bombs, the masks and their
       payback, the Hylian Shield's discount.
   - Talking hides Link (`PLAYER_STATE2_29`), turns the viewpoint to the shop's browsing camera
     (`Play_SetShopBrowsingViewpoint`), and sets `YREG(31)`.
   - `EnOssan_Init`'s checks for every type. **Only the Kokiri shopkeeper** goes on
     (`EnOssan_InitKokiriShopkeeper`: `object_km1`'s skeleton playing
     `object_masterkokiri_Anim_0004A8`, his fairy). The other ten types log that they aren't
     ported and stand as placeholders.
   - The Kokiri shopkeeper's draw: three bakes, one per eye (`object_masterkokirihead`'s head on
     limb 15, fixed colours on segments 8 and 9). The cursor (`EnOssan_DrawCursor`) and the
     stick prompts (`EnOssan_DrawStickDirectionPrompts`), with their pulses, as three sprites
     in the overlay.
3. **`En_GirlA`** (`oot_actors::en_girla`), the shop items, the whole overlay:
   - `shopItemEntries`' 50 rows, with every `EnGirlA_CanBuy_*`, `_ItemGive_*` and
     `_BuyEvent_*` function;
   - the stock changes (`EnGirlA_TryChangeShopItem`, `_SetItemOutOfStock`,
     `_UpdateStockedItem`);
   - the mask texts;
   - the spin while selected (`EnGirlA_Update2`);
   - the draw, `GetItem_Draw` turned by `yRotation`.

   New in `oot_game::item`: `Inventory_HasEmptyBottle`, `func_800849EC` (the Giant's Knife).
4. **`En_Tana`** (`oot_actors::en_tana`), the shelves: the wooden ones' list, and the stone ones
   with their textures (two bakes).
5. **The pause menu's stand-in in the play frame** ([ADR 0021](adr/0021-mido-the-shop-and-the-pause-stand-in.md)).
   `PlayState::kaleido_setup_update` runs where `Play_Update` calls `KaleidoSetup_Update`:
   - before the actors;
   - only with no message box (`msgMode == MSGMODE_NONE`);
   - not during a transition or `Play_InCsMode`.

   Start then runs `pause_menu_equip`. The game passes Start on as input; the scripted runs press
   it.
6. **The message box:**
   - `YREG(31)` is `MessageContext::yreg_31`, read in the five places `z_message_PAL.c` reads
     it;
   - `Message_Update`'s `averageY` is computed as the C computes it (the `s16` screen positions
     promoted to `int`). The port's `i16` sum overflowed with an actor far off screen.
7. **Player:**
   - not drawn under `PLAYER_STATE2_29` (`Player_Draw`);
   - `PlayerIface::change_state_flags2` and `set_equipment_data`.
8. **Saves:**
   - the `deku-tree-open` and `deku-tree-dead` presets set `EVENTCHKINF_04` (Mido has stepped
     aside);
   - a new preset, `sword-and-40-rupees` (the Kokiri Sword worn and 40 rupees), for trying the
     shop by hand.
9. **The run** (`oot_actors::playthrough::Route::MidoShop`, the sandbox's `--script mido-shop`):
   - new tasks:
     - `Pick`: onto a placed item until it's collected;
     - `SlashSwitch`: B beside an interact switch until it's gone, then its drop;
     - `BuyShield`: the shop's phases, each keyed on the shopkeeper's state and the box;
     - `Equip`: Start until nothing more would be equipped;
     - `WaitMido`;
   - the walks stop to read a text that opens by itself (the forced `En_Wonder_Talk2` by the
     shop);
   - `Talk` can talk to Mido;
   - `OpenChest`, `Exit` and `Crawl` take their step.

   The Deku Tree run now goes round Mido where he stands aside.
10. **Pack format 10:** the new bakes (Mido 6, the shopkeeper 3, the stone shelves 2, the shop's
    sprites 3), into `out/data07`.
11. **Run scripts:**
    - `game-shop.bat`: the shortcut to the shop's door on the new preset;
    - `test-mido-shop.bat`;
    - `sandbox-mido-shop.bat`;
    - `game-new-save.bat`'s notes cover the whole way now.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 238 passed, 1 ignored |
| The run (`playthrough` test `a_new_save_to_mido_and_the_shop`) | 5731 frames, Link never hurt. The steps, with the frame each ends on: the chest 2064 (12 rupees: room 2's two green and two blue); the sword on B 2065; back on the plateau 2571; the sign's switch 2856 (switch 0x13, 17); into Mido's house 3356 (19); his chests 3824 (30, treasure flags 0 to 3); into the shop 4236 (37, the forced text 0x218 read on the way); the shield 4779 (42 with the shop's own, then 2); both worn 4780 (`PLAYER_SHIELD_DEKU`); out 4838; Mido 5626 (0x1033, 0x10D2, 0x10D3, 0x1034; `EVENTCHKINF_04`, walking at 1.5); aside 5665 (within 10 of (1412, 0, 211)); past him 5731. The shop's texts 0x9E, 0x83, 0x9F, 0x89, 0x4C, 0x6B; Link hidden while browsing; after it `YREG(31)` 0 and the fixed view. While he blocks, Mido is 60 from his home towards Link on every frame. Four boulder waits: 33, 100, 0 and 33 frames |
| The Deku Tree run (`kokiri_forest_to_the_deku_tree`) | 1873 frames (1971). Mido stands at path 1's end (`func_80AAB874`); Link passes within 80 of him, round him by the bank west of him. The first bush now drops (Mido's `Rand` calls) |
| The Kokiri Sword run (`a_new_save_to_the_kokiri_sword`) | Passes; only its actor counts differ (Mido's fairy) |
| Mido (`mido`: 7 tests) | Blocking: 60 from (1522, 0, 105) towards Link, facing him, target mode 6. The first talk: 0x102F, 0x10D0, 0x10D1, 0x1030; `EVENTCHKINF_02` and `INFTABLE_0C` set; still blocking; then 0x1030 alone. Gestures: at 0x102F's first box sequence 1, `gMidoRaiseHand1Anim` then `gMidoHaltAnim`. With both worn: 0x1033's four texts, `EVENTCHKINF_04`, waypoint 1, 2.25 a frame (speed 1.5 times `R_UPDATE_RATE` × 0.5), stopped within 10 of path 1's end with play speed 0, then 0x1034. With `EVENTCHKINF_04`: at (1412, 0, 211) from the start. The fade: 0x14 a step at first, targetability off, down to 0, back near Link. No Mido with `EVENTCHKINF_40` or `_1C`. His six bakes |
| The shop (`shop`: 8 tests) | The shopkeeper at (0, 0, -26), scale 0.01, text 0x9E, not targetable; the eight items at `sShopkeeperStores[0]`'s offsets from the shelves (0, 0, -20), turned 0xEAAC or 0x1554, scale 0.25, 24 up, with their prices and texts. Talking: `PLAYER_STATE2_29`, viewpoint 2 (bg camera 1, `PIVOT_SHOP_BROWSING`), `YREG(31)` 1, 0x83. The stick right: the camera at -10, -20, -25, -27.5 ... to -30 (`data2` -30), the cursor on slot 0, 0x9F, the shield spinning. A: 0x89, the shield to (17, 58, 10). Buy: `GI_SHIELD_DEKU` offered, the fixed view, Link shown, the shield off the shelf; Player's get-item; 0x4C with the shield owned, not worn, nothing charged; at its end 40 charged and 0x6B; B: idle, `YREG(31)` 0. 39 rupees: 0x85, back to 0x9F. A shield owned: 0x86. Nuts owned: a quick buy (0x84, 5 more nuts, 15 charged). The left shelf: slot 4 at +30, the seeds 0x86 without a slingshot, back to the shopkeeper past the inner end, 0x10BA on "Talk to the owner". Start does nothing while talking and equips both after |
| Golden traces and renders | `playthrough` and `sword_chest` re-recorded (their changes above); new case `mido_shop` (the same bytes over two runs); every render unchanged: 82 of 82 |
| Import | 12.7 s; 51.9 MB; format version 10, into `out/data07` |
| The windows | Not yet played by hand: `scripts\run\game-shop.bat`, and the whole way from `game-new-save.bat`. Headless screenshots checked: Mido in Kokiri Forest, the shopkeeper behind his counter, the browsing view with both prompts and the cursor, the buy prompt with the shield off the shelf, and the shield held up over 0x4C |

### Decisions

- **[ADR 0021](adr/0021-mido-the-shop-and-the-pause-stand-in.md):**
  - the pause menu's stand-in in the play frame, as `KaleidoSetup_Update`;
  - `YREG(31)` in the message box;
  - `En_Ossan`'s states whole, only the Kokiri shopkeeper's body;
  - `En_GirlA`'s items as a table of functions, the items taken out while called;
  - Player's state flags through its interface;
  - `EVENTCHKINF_04` in the presets;
  - the route on placed rupees only.
- **The rupees are the C's placed ones.** 42 can be reached on foot:
  - room 2's two blue rupees (En_Item00 0x0F01 and 0x0E01): 10, beside the two green ones the
    sword run already got (2);
  - the switch by the plateau's sign (En_Wonder_Item 0x1A53): 5;
  - four green rupees below the ramp and south of the village: 4;
  - Mido's house's chests (two blue, one green, a heart): 11;
  - the free multitag's two tag points: 5;
  - the shop's own proximity drop right of the counter: 5.

  Left out:
  - the three proximity drops on the plateau (0x1214, 0x1215, 0x1256): they hang 45 above its
    floor, beyond their reach of 30, and need a jump;
  - the high blue rupee on Fado's platform (y 180);
  - the one behind Mido's house (y 53, a step of 60 up);
  - the ordered multitag by the ford: its two points are 486 apart, with 80 frames between
    them.
- **Start right after the chest**, as text 0xA4 says: the switch needs the sword on B.
- **The Deku Tree run goes round Mido.** Its path went through where he stands aside. The bank
  west of him leaves about 50 between the stream and his collider.
- **The walks read a text that opens by itself.** In the C, the forced `En_Wonder_Talk2` by the
  shop holds Link (Player's cutscene mode 8, not ported) until it's read. The run stops and
  reads it, as a player would; before, a walk left it open and walked on.

### Known gaps

- **The pause menu** is still Start's stand-in. It never swaps a worn piece.
- **`En_Ossan`:**
  - the other ten shopkeepers are placeholders (their inits, objects, skeletons and draws);
  - the sounds and prints;
  - the unused collider.
- **`En_GirlA`:** the items' highlight (`hiliteFunc`) isn't drawn.
- **The stick prompt's rectangle** reads 24 rows of a 16-row texture, and the C's bottom rows are
  whatever TMEM holds. The sprite repeats the texture instead.
- **Link's shadow** isn't drawn while he's hidden in the shop. The C's `Actor_Draw` still draws
  it; the port draws it inside Player's draw.
- **Mido:**
  - his fairy is a placeholder (Navi, milestone 5);
  - the ocarina in the Lost Woods;
  - the goodbye's cutscene;
  - the circle shadow's alpha.
- **Forced texts** don't hold Link (Player's cutscene modes: milestone 4). *(Done in milestone 4.)*
- **Carried over:** the cutscenes, Navi, audio, the effects. *(The cutscenes: done in milestone 4.)*

## Milestone 4: cutscenes, first part

**Answer:** done. Talking to the Deku Tree plays his cutscenes as the C does: in his meadow his
first talk starts by itself (`D_808BCE20`), the camera takes the script's shots, Link is walked
in, the tree speaks and asks; yes plays `D_808BD520`, which opens his mouth (`EVENTCHKINF_05`).
Entering the Deku Tree the first time plays its intro (`gDekuTreeIntroCs`). A scripted run from
Link's bed on a new save now goes past Mido, through the talk and into the Deku Tree, with no
save preset.

The tests: 244 pass, 1 ignored (238 before). The goldens:
- 21 sheets: the letterbox in their first frames (`D_8011D3F0`, below);
- the Mido and shop run's trace: the forced text by the shop now holds Link;
- the Deku Tree run's trace: the Deku Tree's intro;
- the new run's trace is a new case;
- 83 of 83.

### What was built

1. **The scripts in the pack** ([ADR 0022](adr/0022-cutscenes.md)), pack format 11:
   - every script is the ROM's bytes, big-endian, keyed `cutscene/<file>/<symbol>`
     (`oot_game::cutscene::CutsceneScript`);
   - the 73 scene scripts the XMLs name, each walked from its offset to `CS_END`;
   - the 27 `CutsceneData` arrays of the actors' C: `oot_import::cutscene` builds each array's
     words with `z64cutscene_commands.h`'s own macros, finds them in the overlay's file in the
     ROM, and stores the ROM's bytes. All 27 are found;
   - `sEntranceCutsceneTable` (34 rows) and every script's key (`table/cutscenes`);
   - a scene layer's `SCENE_CMD_ID_CUTSCENE_DATA` (`LayerData.cutscene`);
   - `ootx cutscene [name]` lists the scripts and the table, or prints a script's commands.
2. **`z_demo.c`** (`oot_game::cutscene`), onto the play state, with the C's names:
   - the state machine: `func_80064558` and `func_800645A0` after `Actor_UpdateAll`, their state
     tables, `func_8006472C`'s fade (`unk_0C`), `func_80068ECC`'s start (with the cutscene
     camera, `D_8015FCC8`), `func_80068C3C`, `func_80068D84`, `func_80068DC0`'s end;
   - `Cutscene_ProcessCommands`, command by command: the cues (`linkAction`, `npcActions`, by
     the C's type lists), the camera lists (1, 2, 5, 6) and single points (7, 8), the text
     command (the frame held at its end while a box is up, a choice's branches), the misc
     actions, the lighting, the time, the music, the rumble, the transition fill and the
     terminator (every destination);
   - `Cutscene_HandleEntranceTriggers`, `Cutscene_HandleConditionalTriggers`,
     `Cutscene_SetSegment`, `gSaveContext.cutsceneTrigger`, `nextCutsceneIndex`;
   - `Play_Init`: `Environment_Init`'s `D_8015FCC8 = 1` (`z_kankyo.c:419`, found by scanning the
     ROM for stores to it: the decomp only names the debug D-pad's), the scene layer's script,
     the entrance triggers;
   - `Play_InCsMode`; `KaleidoSetup_Update` now checks it; `func_8002DF54` and `func_8002DF38`;
     `Flags_SetEnv` and its kin (`play->envFlags`);
   - the file's statics (`DemoStatics`) carry over from one play state to the next.
3. **Cameras:**
   - three sub camera slots, `activeCamId`, and `Play_CreateSubCamera`,
     `Play_ChangeCameraStatus`, `Play_ClearCamera`, `Play_ClearAllSubCameras`,
     `Play_CameraChangeSetting`, `Play_CameraSetAtEye`, `Play_CameraSetFov`, `Play_CopyCamera`;
   - every camera updates each frame, the active one last; the view, the input direction and
     the render state follow the active camera;
   - `Camera_Update`'s statuses (cut, waiting, active) and its interface branches;
   - `z_camera.c`'s shared state as `CameraGlobals` (`sCameraInterfaceFlags`,
     `sCameraInterfaceAlpha`, `D_8011D3F0`, `sOOBTimer`, `sNextUID`);
   - `Camera_Demo1` (`CAM_SET_CS_0`), the spline (`func_800BB0A0`, `func_800BB2B4`),
     `Camera_SetCSParams`, `Camera_ResetAnim`, `Camera_SetParam`, `Camera_Copy`.
4. **Player's cutscene modes** (`oot_actors::player`):
   - `Player_UpdateCommon`'s cutscene block: a script puts Link in mode 6 (a cue with a mode in
     `D_808547C4`) or 0x31 (held);
   - the action `func_80852E14` (`Action::Cutscene`), reached through `func_8083B998`
     (interrupt 0, new) and `func_8083B040` (interrupt 13, and the actions that check it) into
     `func_8083ADD4`; also at a talk's end (`func_8084B530`);
   - `func_80852B4C`'s dispatch of `D_80854B18` and `D_80854E50` for modes 1, 3, 4, 6, 7, 8 and
     0x31: `func_808515A4`, `func_808514C0`, `func_80851688`, `func_80851998`, `func_808519C0`
     (`func_80845964` with a cue), `func_80852C50`, `func_808529D0`, `func_80852A54`,
     `func_80852944`, `func_8083C148`, `func_8083B010`;
   - `PlayerIface::set_cs_mode`, so other actors' `func_8002DF54` reach Player.
5. **The actors:**
   - `Bg_Treemouth` starts its scripts where the C does: `D_808BCE20` on the first approach,
     `D_808BD2A0` when Z-targeted afterwards, `D_808BD520` or `D_808BD790` by the answer (with
     `D_8015FCC0` to `C4` reset);
   - `En_Wonder_Talk2`'s forced texts hold Link (mode 8) until they're read (mode 7).
6. **The message box:** its cutscene checks (`csCtx.state == 0` for the do-action's "Return",
   and for the interface's return at a box's close with the main camera active).
7. **The runs:**
   - `Route::NewSaveDekuTree` (the sandbox's `--script new-save-deku-tree`): the Mido and shop
     run, then east through the passage into the meadow, the talk (`Task::TreeTalk`: A through
     the texts, yes), over the open jaw and in;
   - the walks no longer stop for a forced text: Link is held, and the walk presses A;
   - the sandbox's trace records the cutscene (script, state, frame, Player's mode, the active
     camera) on the frames one runs.
8. **Run scripts:** `game-deku-tree-talk.bat`, `test-cutscenes.bat`,
   `sandbox-new-save-deku-tree.bat`; `game-new-save.bat`'s notes go on to the Deku Tree.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 244 passed, 1 ignored |
| The scripts (`cutscene` test `the_deku_trees_scripts_are_the_c_s`) | `D_808BCE20` from `ovl_Bg_Treemouth`: `CS_BEGIN_CUTSCENE(12, 3000)`; its commands in order (0x15, the player cues, two eye lists, two at lists, the texts, the misc, 46, 62, the BGM and its fade), the walk ending at the script's end; list 46 fills `npcActions[0]`, 62 (`CS_CMD_SET_ACTOR_ACTION_9`) `npcActions[8]`. Cue 2: frames 0..33, (2614, 0, -451) to (2808, 0, -559), `rot` (0x54B2, 0, 0) (the macro's fourth argument is `rot.x`), `normal` the float bits of 5.878788. The first eye list: five points, the last `CS_CMD_STOP`; the at list's third point waits 1000. The other three scripts walk to their ends. `sEntranceCutsceneTable`: 34 rows; `ENTR_YDAN_0`, either age, `EVENTCHKINF_A8`, `gDekuTreeIntroCs` from `ydan_scene` (`CS_BEGIN_CUTSCENE(4, 1270)`) |
| The spline (`the_b_spline_through_four_points`) | At u 0, (p0 + 4 p1 + p2) / 6; 1/30 of a key a frame with both next points at 30; the next key after 30 steps (31 for the `f32` sum); done when three points are left |
| The first talk (`the_deku_trees_first_talk_and_yes_open_his_mouth`) | From `ENTR_SPOT04_1`: `EVENTCHKINF_0C`, `D_808BCE20`, `cutsceneIndex` 0xFFFD, `CS_STATE_SKIPPABLE_INIT`; sub camera 1 active on `CAM_SET_CS_0` at the script's eye (2753, 46, -354), the main camera waiting. Link in mode 6, moved to cue 2's start (2614, -1, -451: more than 50 away, a child in Kokiri Forest 1 lower), then cue 4. Text 0x107D at frame 41; 0x1015, then 0x1016's question with the script held at 169. Yes: `D_808BD520`, `EVENTCHKINF_05`, the same sub camera at (3740, -141, -530); the mouth's cue 3 read at frame 22, opening by 0.01 a frame; 0x1017; the end at frame 100, then 10 frames of `unk_0C` to idle, the main camera active, the sub camera cleared, Link standing (mode 7, then 0); the mouth open to 1 |
| No (`no_says_0x1018_and_the_tree_waits_to_be_targeted`) | `D_808BD790`, no `EVENTCHKINF_05`, the mouth back to `func_808BC8B8`, 0x1018; afterwards no script until he's Z-targeted |
| The Deku Tree's intro (`entering_the_deku_tree_the_first_time_plays_its_intro`) | `Play_Init` at `ENTR_YDAN_0`: `EVENTCHKINF_A8`, `gDekuTreeIntroCs`, `cutsceneTrigger` 2, no title card; the first frame starts it; Link held (mode 0x31, `PLAYER_STATE1_29`) instead of walking in; the end at frame 165; mode 0x31 ends by itself. With `EVENTCHKINF_A8` set, no intro |
| A forced text (`a_forced_text_holds_link_until_it_is_read`) | By the shop's door: text 0x218 opens, Link in mode 8 (`Action::Cutscene`, `PLAYER_STATE1_29`); the stick moves him less than 1 in 5 frames; read, mode 7 and free |
| The exit run (`playthrough` test `a_new_save_into_the_deku_tree`) | 7576 frames, no preset. As the Mido and shop run to past Mido (5731), then the talk over at 7056 (texts 0x107D, 0x1015, 0x1016, 0x1017), at the open jaw 7328 (`unk_168` 1, the mouth at (3869, -263, -1163)), into the mouth 7367, the Deku Tree settled 7576 after its intro. The scripts in order: `D_808BCE20`, `D_808BD520`, `gDekuTreeIntroCs`, the sub camera active during them |
| The other runs | The Deku Tree run (on its preset): the intro holds Link in the Deku Tree, 2028 frames (1873). The Mido and shop run: the forced text holds Link for 83 frames, and the run ends on the same frame, 5731. The Kokiri Sword run: unchanged |
| `D_8011D3F0` (`zcamera`, `talk`) | The main camera's first three updates after `Camera_Init` hold the interface at 0x3200 (the letterbox's target 32, alpha type 2); a transition's 0xF200 keeps alpha type 2 through the fade-in, so the HUD comes back after it. The tests that started in those frames now wait them out |
| Golden traces and renders | 21 sheets, `mido_shop` and `playthrough` re-recorded (golden/README.md: the letterbox in the sheets' first frames; the hold at the shop; the Deku Tree's intro), new case `new_save_deku_tree` (the same bytes over two runs): 83 of 83 |
| Import | 19.1 s; 52.0 MB; format version 11 (73 scene scripts, 27 overlay scripts), into `out/data08` |
| The windows | Not yet played by hand: `scripts\run\game-deku-tree-talk.bat`, and the whole way from `game-new-save.bat`. Headless screenshots checked: the script's shots letterboxed, the tree's text as his mouth opens, the Deku Tree's intro looking up the trunk |

### Decisions

- **[ADR 0022](adr/0022-cutscenes.md):**
  - the scripts are the ROM's bytes, walked as `Cutscene_ProcessCommands` walks them, with
    offsets for its pointers;
  - the overlays' scripts are built from their C's macros only to find them in the ROM;
  - `z_demo.c` is ported onto the play state, its statics carried across `Play_Init`;
  - sub cameras, and `z_camera.c`'s shared state as `CameraGlobals`;
  - Player's cutscene modes as one action, the modes Kokiri Forest uses ported.
- **`D_8015FCC8` is 1 in play.** The decomp at this commit names it only where the debug D-pad
  replays set or clear it; a scan of the ROM's code for every store to its address found
  `Environment_Init`'s, which sets it on every `Play_Init`. Without it, no script would move the
  camera.
- **`D_8011D3F0` is ported,** though it changes every scene's first frames: `Camera_Init` sets it,
  and the letterbox and the HUD's fade at a scene's start are the C's.
- **A cue's facing is `rot.y`, which the Deku Tree's cues leave 0.** `CS_PLAYER_ACTION`'s fourth
  argument lands in `rot.x`; `func_808529D0` reads `rot.y`. Link is put at cue 2's start facing
  0 and turns towards its end as he walks, as in the C.
- **The run answers yes with A on the first choice,** as a player keeps the cursor. The test of
  no picks the second with the stick.
- **The Deku Tree run keeps its preset.** It's GAME-02's exit test; the new run is the one without.

### Known gaps

- **Cutscene layers:** the pack holds scene layers 0 to 3, so a `cutsceneIndex` of 0xFFF0 and up
  (a new file's 0xFFF1 in Link's house, Navi's wake-up; the layers terminators ask for) loads the
  normal layer, logged. A new file here enters with 0 (GAME-03 milestone 5).
- **Player's cutscene modes:** only 1, 3, 4, 6, 7, 8 and 0x31; the other modes' starts and
  updates are logged. Swimming in a cutscene (`func_80851368`, `func_808513BC`) isn't ported.
- **Not ported, logged:** the scripts' lights, weather, fog, skybox changes, quakes, the title
  card (the Deku Tree's intro has one), the screen tint, the Sun's Song, the ocarina texts
  (`func_8010BD58`), `linkAgeOnLoad`, the music and the rumble.
- **Navi's cues** (`npcActions[8]` in the first talk) have no actor to read them (milestone 5).
- **The one-point cutscenes** (the crawl's 9601 and 9602, `OnePointCutscene_Init`, the camera's
  parent and child chain, `Camera_Finish`'s timer): not ported; BACKLOG #3 stays.
- **`func_8083B998`'s C-Up** into first person isn't ported (noted when pressed).
- **The debug D-pad replays** (D-Left, D-Up) only work in a cutscene layer, which never loads.
- **Carried over:** Navi, audio, the effects, the pause menu.

## Visual polish, deferred

Side-by-side comparisons against Project64 on the same ROM, for when the look is polished. They
don't block milestones: behaviour is checked against the C by the tests.

1. **The Deku Tree's talk** (`scripts\run\game-deku-tree-talk.bat`): the script's shots and the
   spline's pace, Link's walk in and where he stops, the letterbox's size, the mouth's opening,
   the Deku Tree's intro (its title card isn't drawn), and the letterbox at every scene's start.
2. **The Kokiri shop and Mido** (`scripts\run\game-shop.bat`):
   - the shopkeeper behind his counter, his blinks and his idle animation;
   - the browsing camera's turn to each shelf, the cursor's size and pulse, the stick prompts'
     places and pulses (their bottom rows are known to differ);
   - an item's spin, and its move off the shelf towards Link;
   - Mido's gestures through his texts, his walk aside, and his fade by distance.
3. **The crawlspace and the training area** (`scripts\run\game-new-save.bat`):
   - the crawlspace's view (`Camera_Subj4`): the ease-in, the bob and sway, Link hidden;
   - `tunnel_start` and `tunnel_end`, and the camera handing over on the way out (the one-point
     cutscenes aren't ported);
   - the boulder's roll and size; Link's stagger, knockdown and getting up; the hit's red flash
     (known: not drawn).
4. **The sword chest** (`scripts\run\game-sword-chest.bat`, against a new file):
   - the slow opening: Link's animation, the lid, the camera's move (`CAM_SET_SLOW_CHEST_CS`);
   - the turn to face the camera (`CAM_SET_TURN_AROUND`) and the sword held up: its size, its
     place over his head, when it appears;
   - text 0xA4: the icon's place and size, the quick text of the first line, the two boxes;
   - Link's stance as the text closes, and the push-back from the chest;
   - the missing light over the chest (known: `Demo_Tre_Lgt`).
5. **The HUD's B button:** empty on a new file, then the sword's icon once it's equipped. (The
   C buttons' icons can't be seen yet: the HUD only runs after `Play_Init`, whose new file has
   nothing on C.)
6. **The placed recovery hearts** in Kokiri Forest: their model, size and the texture's
   scroll.
7. **Link's model:** the empty sheath on a new file, the sword in it after equipping.
8. **From GAME-02, still open:**
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

**GAME-03 milestone 5: Navi** ([ROADMAP.md](ROADMAP.md), Phase 4):
- `En_Elf` as Link's fairy (following, the target reticle's `naviRefPos`, C-Up and her text),
  and the Kokiri children's fairies;
- the game's opening: the pack's cutscene layers (4 and up) and the new file's entrance on
  `cutsceneIndex` 0xFFF1, Navi waking Link in his house;
- Navi's cues in the Deku Tree's talk (`npcActions[8]`).

**Exit:** a new save starts as the game does, and C-Up talks to Navi. With it, the phase's exit:
the headless run from a new save into the Deku Tree, the game's way from its very start.
