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
  audio.

## Visual polish, deferred

Side-by-side comparisons against Project64 on the same ROM, for when the look is polished. They
don't block milestones: behaviour is checked against the C by the tests.

1. **The crawlspace and the training area** (`scripts\run\game-new-save.bat`):
   - the crawlspace's view (`Camera_Subj4`): the ease-in, the bob and sway, Link hidden;
   - `tunnel_start` and `tunnel_end`, and the camera handing over on the way out (the one-point
     cutscenes aren't ported);
   - the boulder's roll and size; Link's stagger, knockdown and getting up; the hit's red flash
     (known: not drawn).
2. **The sword chest** (`scripts\run\game-sword-chest.bat`, against a new file):
   - the slow opening: Link's animation, the lid, the camera's move (`CAM_SET_SLOW_CHEST_CS`);
   - the turn to face the camera (`CAM_SET_TURN_AROUND`) and the sword held up: its size, its
     place over his head, when it appears;
   - text 0xA4: the icon's place and size, the quick text of the first line, the two boxes;
   - Link's stance as the text closes, and the push-back from the chest;
   - the missing light over the chest (known: `Demo_Tre_Lgt`).
3. **The HUD's B button:** empty on a new file, then the sword's icon once it's equipped. (The
   C buttons' icons can't be seen yet: the HUD only runs after `Play_Init`, whose new file has
   nothing on C.)
4. **The placed recovery hearts** in Kokiri Forest: their model, size and the texture's
   scroll.
5. **Link's model:** the empty sheath on a new file, the sword in it after equipping.
6. **From GAME-02, still open:**
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

**GAME-03 milestone 3: Mido and the shop,** scoped in [ROADMAP.md](ROADMAP.md) (Phase 4):
- `En_Md`, blocking the path until Link has the sword and a shield (`EVENTCHKINF` flags);
- `En_Ossan` (the Kokiri shop only), with its browsing camera, and the shelf items (`En_GirlA`).

**Exit:** buy the Deku Shield with the rupees from the bushes, and Mido lets Link through. The
sword run's waits and routes (`oot_actors::playthrough`) are the pieces to extend it with.
