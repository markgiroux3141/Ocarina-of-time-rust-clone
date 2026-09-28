# GAME-02: the Kokiri Forest vertical slice

**Goal:** Phase 3 of [ARCHITECTURE-PLAN.md](ARCHITECTURE-PLAN.md) §5: Kokiri Forest playable as in the game, one ported system or actor at a time, each checked against the C.

| # | Milestone | Status |
|---|---|---|
| 1 | The collision check; Kokiri's props with their real models; `En_Ko`; ladder and vine climbing; Z-targeting polish | done |
| 2 | `En_Door`, the prerendered backgrounds and the fixed cameras | done |
| 3 | The message box, Player talking, items and a minimal HUD | done |
| 4 | `Bg_Treemouth` and a headless scripted playthrough | next |

The working rules are the same as for GAME-01:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack;
- ports go function by function with the decomp's names, every constant is cited, and faithful bugs are marked `@bug (game)`;
- test expectations come from the C.

Decisions are in [docs/adr/](adr/README.md) (0011 to 0017 so far).

## Milestone 1: collisions, props, the first NPC, climbing, Z-targeting

**Answer:** done. The sword cuts Kokiri's bushes and signs and bounces off its rocks, and Link bumps into all three. The Kokiri children stand, animate, fade with distance, turn their heads to Link and block the way to the Lost Woods. Link climbs the ladder to his house and back down. Z-targeting has the game's camera, bars and reticle.

The tests: 157 pass, 1 ignored (151 before the Z-targeting work). The goldens are re-recorded for the camera modes: 28 of 78 hashes, each explained in `golden/README.md`.

### What was built

1. **The collision check** ([ADR 0011](adr/0011-collision-check.md)).
   - `oot_game::collision_check` ports `z_collision_check.c`: the collider types (cylinder, joint spheres, triangles, quads), their init and update, `CollisionCheck_SetAT/AC/OC`, the 16 AT-against-AC shape pairs, the quad's nearest-AC rule, OC pushes by mass, and `CollisionCheck_Damage` with damage tables.
   - `eng_collision::math3d` has the `sys_math3d.c` primitives the checks use.
   - The checks run where `Play_Update` runs them: after the room load, before `Actor_UpdateAll`.
   - Player registers its body cylinder, both sword quads (placed from the left hand's draw, `func_80090480`) and the shield quad, with the C's damage flags per attack (`D_80854488`).
2. **Kokiri's props, with their real models.**
   - `Obj_Hana` (a flower, rock or bush), `En_Ishi` (rocks: the sword bounces off, `AC_HARD`), `En_Kusa` (bushes: cut, stumps, regrowth) and `En_Kanban` (signs: cut along the slash into flying pieces, with the cut mark).
   - Their display lists come from the pack, the multi-list ones as actor bakes ([ADR 0012](adr/0012-actor-bakes.md)).
   - Placeholders for them are gone from room 0.
3. **`En_Ko`, the first NPC with a skeleton.**
   - The whole overlay, apart from talking. It picks the children that spawn for the story's progress (`EnKo_CanSpawn`) and waits for its objects. Each child's animation comes from `sOsAnimeLookup` / `sAnimationInfo`. It fades by Link's distance (`func_80A98DB4`, untargetable when faint), and turns its head and torso (`func_80034A14` and the `D_80116130` presets). Child 3 guards the Lost Woods until Link has the Kokiri Emerald.
   - It's drawn as `EnKo_Draw` draws it: tunic and boots colours per draw, and 14 baked variants (head, eyes, opaque or translucent).
   - New framework under it:
     - `oot_game::skelanime_std` (`SkelAnime` for standard animations);
     - `oot_game::npc` (the talk offers and `func_800343CC`, the head tracking, the limb sway);
     - the save's event, item and info tables;
     - `Rand` on `PlayState`;
     - `Actor_SpawnAsChild` (each child's fairy);
     - cached skeletons and animations in `GameAssets`.
4. **Ladder and vine climbing.**
   - `func_8083F7BC` (walking into a climbable wall) and `func_8083EC18` (onto it: lined up to a ladder's rungs, or turning round at a ladder's top).
   - `func_808458D0` (the item put away first), `func_8084BF1C` (climbing: a rung per animation, sideways on vines) and `func_8084C5F8` (stepping off at the top or bottom).
   - The climbable-edge branch of `func_8083A6AC`, so walking off the porch hangs from the ladder's top and climbs down.
   - `sAgeProperties` is now read whole (33 fields), with the climbing animations.
5. **Z-targeting polish** ([ADR 0013](adr/0013-camera-modes-and-screen.md)).
   - **Camera modes.** `Player_UpdateCamAndSeqModes` asks for a mode each frame, and `Camera_ChangeModeFlags` accepts, refuses or resets it. The pack now holds all 21 NORMAL0 modes' functions and data.
   - **`Camera_Parallel1`** (Z with nothing targeted): the swing behind Link over `R_CAM_DEFAULT_ANIM_TIME` frames with mode changes refused meanwhile, then the target distance and pitch. With it come `Camera_CalcAtForParallel` and `func_800458D4`.
   - **`Camera_KeepOn1`** (locked on to a non-enemy, such as the Kokiri or a sign), with `Camera_CalcAtForLockOn`.
   - **The letterbox:** `shrink_window.c` and `Camera_UpdateInterface`, 10 rows a frame towards 26, 27 or 32. It's drawn as black bars under the overlay.
   - **The real reticle** (`func_8002C124`): `gZTargetLockOnTriangleDL` after `SETUPDL_57`, four spinning triangles per trail entry in the orthographic overlay. It flies in from the screen's centre as it locks and fades when the target is lost, in the category's colour (`sNaviColorList`).
   - **The target arrow:** `gZTargetArrowDL` over the next candidate.
   - **Placeholders stop counting as targets:** they spawn with their profile's flags minus `ACTOR_FLAG_0`, and a test checks that neither the candidate search nor Z picks one.
   - **Two fixes to the target context against `func_8002C7BC`:**
     - a new target now calls `func_8002BE98` (the reticle restarts at 500);
     - the before-lock drop uses `Actor_ProjectPos`'s test, not the candidate search's screen box.
   - **Engine:** `DrawLists::overlay_2d` (the orthographic `OVERLAY_DISP`) and `DrawLists::letterbox_rows`, drawn by `Renderer::render_screen`.
6. **Tools:**
   - `ootx pack-ls <prefix>` lists records;
   - `ootx dl-dump <file> <symbol>` prints a display list's commands and vertices from the ROM;
   - `oot_sandbox --entrance ... --at` places Link standing.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 157 passed, 1 ignored |
| Collision check (`oot_game --test collision_check`, 8; `math3d` unit tests, 15) | OC pushes by mass (equal masses, immovable, type and height skips); a sword quad hitting a bush on both sides; damage from the toucher's table entry; the nearest-quad rule; pairs skipped by type, owner and death; the context's list sizes. The shape pairs themselves are covered through `math3d`'s tests |
| Props (`oot_actors --test props`, 3) | Kokiri's props initialise as their C does (collider dims, flags, types); the sword cuts a village bush; a slash cuts a piece off a sign (the piece, the cut mark's timer from the C). No test covers the rock's bounce (`AC_HARD`) yet |
| `En_Ko` (`--test kokiri`, 3) | The children that spawn and their start animations; the fade (40 a frame, targetable from alpha 10); child 3's guard position, 80 from home facing Link |
| Climbing (`--test climb`, 2) | Up the ladder to the porch (lined up within 8 of the rungs, off the top onto y 100); off the porch onto the ladder's top, facing it, and down to the ground |
| Z-targeting (`--test zcamera`, 5; `letterbox` unit test) | Parallel1's swing lands behind Link and refuses mode changes while animating; the bars reach 27 in 10-row steps after the swing; a request during the swing goes through the frame after it ends; a Kokiri lock uses FOLLOWTARGET / KeepOn1 with bars and the NPC colour; the reticle flies in, locks at 120, and fades 0x100 → 136 → 16 → 0; placeholders are never candidates or locked |
| Camera data (`--test camera`) | 21 NORMAL0 modes, with the NORM1, PARA1 and KEEP1 values checked against `z_camera_data.c` |
| Golden traces and renders | Re-recorded: 28 of 78 changed (see `golden/README.md`), 50 unchanged |
| Import | 9.0 s; format version 4 (camera modes; the reticle's two bakes, 29 actor bakes in all) |
| The windows | Bushes, rocks, the sign and its cutting, the Kokiri, and the ladder checked in the windowed sandbox. The Z-target bars and the reticle on a Kokiri checked in a headless screenshot (27-row bars, light-blue triangles) |

### Decisions

- **[ADR 0011](adr/0011-collision-check.md):** colliders stay in their actors; the context holds references, and the checks take the colliders out and put them back. The hit elements read later are copied at hit time.
- **[ADR 0012](adr/0012-actor-bakes.md):** actors declare what they draw (lists or a skeleton, bound segments, a prelude), and the importer bakes it. Per-draw colours are dynamic segments; states that change geometry or the render mode are separate bakes.
- **[ADR 0013](adr/0013-camera-modes-and-screen.md):**
  - camera modes dispatch on the imported `CAM_FUNC`, and unported modes run Normal1 on NORMAL's data;
  - the letterbox is game state drawn as engine bars;
  - 2D overlay draws are an engine list in the interface's orthographic projection, widened with the aspect;
  - the reticle moves at the game's 20 Hz.
- **`Camera_KeepOn1` came with the milestone**, beyond what it asked for (Parallel1 and the bars). Every lock-on in Kokiri Forest is on a friendly actor, so without KeepOn1 none of them would show the Z camera or the bars.

### Known gaps

- **Camera:**
  - `Camera_Battle1` isn't ported, so locking on to an enemy frames like the Normal camera with no bars. Kokiri Forest has no enemies; the sandbox's course dummy is one.
  - The same Normal1 fallback covers TALK (`Camera_KeepOn3`), jumps and falls (`Jump1`), climbing (`Jump2`), hanging (`Uniq1`) and first person (`Subj3`), with a restart at each mode change.
  - `func_80043F94` (scenes with the skybox disabled), the interface alpha, the mode-change sounds, and KeepOn1's `interactRangeActor` case are left out.
- **Reticle:** no Navi (`naviRefPos`), no lock-on sounds. The reticle moves at 20 Hz while the world is blended at the display rate, so it can trail a moving target by up to one game frame.
- **Talking:** actors offer to talk (`func_8002F1C4`), but Player doesn't accept on A and there's no message box, so `En_Ko`'s and `En_Kanban`'s talk states stay 0. That's milestone 3.
- **Props:** no lifting or throwing (Player can't lift), no drops (`En_Item00`), no leaves, fragments, dust or sounds. The bugs in bushes are `En_Insect` placeholders. `En_Kanban`'s shadow, ocarina repair and hammer quake aren't ported.
- **`En_Ko`:** no paths (child 3 with the emerald), no Fado trade, and each child's fairy is an `En_Elf` placeholder.
- **Climbing:** grabbing a climbable wall from the air (`func_8083EC18` from `func_8084411C`) and crawlspaces aren't ported, and there's no climbing sound. The vine code is shared with the ladder, but only the ladder has a test.
- **Collision check:** hit effects and sounds, `CollisionCheck_LineOC`, colliders without an actor.
- **Placeholders** can't be targeted, even where the real actor can (Mido, Saria). Each one becomes targetable when it's ported.
- **Carried over from GAME-01:** prerendered rooms, doors, time passing, culling (see GAME-01's known gaps).

## Milestone 2: doors, prerendered rooms, fixed cameras

**Answer:** done. Kokiri Forest's interiors look as in the game:
- Link's house and the other houses open on their pivot camera with the room's 360° picture around it, and C-Up switches to the fixed camera over the room's JPEG;
- the shop opens on its fixed camera;
- Link's porch, some Kokiri entrances and every exit have their scene cameras.

`En_Door` opens: Link lines up, plays his side's animation and walks through. A scene-exit door starts its exit; a room door loads the room behind it, with the door camera and its bars.

The tests: 171 pass, 1 ignored (157 before). The goldens are unchanged: 78 of 78.

### What was built

1. **Camera settings and the scene's bg cameras** ([ADR 0015](adr/0015-camera-settings-and-bg-cameras.md)).
   - **The import:**
     - all of `sCameraSettings`: 65 settings with their valid modes, priorities and flags, and every `sCamSet*Modes` array (`CameraData::settings`);
     - each scene's `BgCamInfo` list, in its collision header (`CollisionHeader::bg_cams`), read as far as the scene names it;
     - `SCENE_CMD_MISC_SETTINGS`' camera type (`LayerData::scene_cam_type`).
   - **The setting changes:** `Camera_ChangeSettingFlags` (with the priority rule), `Camera_ChangeBgCamIndex` (once a frame), `Camera_ChangeDoorCam`, `func_80057FC4` (a room's first setting: `FREE0` when prerendered, `DUNGEON0` for `ROOM_BEHAVIOR_TYPE1_1`, else `NORMAL0`) and `func_8005B1A4`.
   - **`Camera_Update`'s floor check:** the bg camera under Player, once Player is within 2 of the floor.
   - **`Play_Init`:** `func_8005AC48(0xFF)`, the start camera from Player's params, and the viewpoint (`VIEWPOINT_PIVOT` in the houses, `VIEWPOINT_LOCKED` in the shops).
   - **`Play_Update`:** `Play_ChangeViewpointBgCamIndex` every frame, and C-Up's toggle, refused in shops and while `Player_InCsMode`.
   - **The mode functions:**
     - `Camera_Fixed3` (`PREREND_FIXED`), `Camera_Unique7` (`PREREND_PIVOT`), `Camera_Unique6` (`FREE0`) and `Camera_Data4` (`PIVOT_SHOP_BROWSING`), for the interiors;
     - `Camera_Fixed4` (`PIVOT_IN_FRONT`, Link's porch), `Camera_Unique0` (`START1`, some Kokiri entrances) and `Camera_Fixed2` (`PIVOT_CRAWLSPACE`), for Kokiri Forest;
     - `Camera_Unique2` (`SCENE_TRANSITION`), for exits;
     - `Camera_Special9` (`DOORC`) and `Camera_Unique3` (`DOOR0`), for doors;
     - `Camera_CheckOOB`.
   - **The fallback:** an unported function runs its setting's NORMAL function, so TALK in a house keeps the house camera.
2. **Prerendered rooms** ([ADR 0014](adr/0014-prerendered-backgrounds.md)).
   - **The backgrounds:**
     - the importer decodes each background (`oot_import::background`: JPEG to RGBA5551, as `Jpeg_Decode` leaves it) into a screen quad in the room's record, single and multi-image rooms alike;
     - `Room_DrawImage`'s choice (`oot_game::room::image_background`): drawn only with `CAM_SET_PREREND_FIXED`, a multi-image room's by the camera's bg camera or its override;
     - the engine draws it in the orthographic space in the middle of the OPA list (`DrawParams::screen`), after the room's geometry and without depth, so the geometry keeps only its depth under it.
   - **The room skyboxes:**
     - `oot_game::skybox` writes out `Skybox_Init`'s display lists (`func_800AEFC8`, `func_800ADBB0`) and `SkyboxDraw_Draw`'s palette loads after `SETUPDL_40`;
     - the importer reads `Skybox_Setup` for the 22 room skyboxes and bakes each;
     - they're drawn at the eye after the rooms, with any camera but `PREREND_FIXED`.
3. **Doors** ([ADR 0016](adr/0016-player-requests.md)).
   - **`En_Door`, the whole overlay** except text, keys and sounds:
     - `sDoorInfo`'s object and door lists by scene;
     - `EnDoor_SetupType` (locked, ajar, checkable, evening doors, Talon's);
     - the double door's other half;
     - `EnDoor_Idle` offering the door to Player (20 across, 50 through, facing within 0x3000);
     - the opening animation for Player's side and age;
     - the ajar doors' swing;
     - `EnDoor_Draw` with `EnDoor_OverrideLimbDraw`'s side choice, from 12 bakes (5 door lists × 2 sides, and the two ajar faces).
   - **Player's side:**
     - `func_80839800` in the interrupt lists (index 1): A opens the door; Link lines up 22 from it and plays `PLAYER_ANIMGROUP_9`..`_12`, moved by the animation (0x28F);
     - a scene-exit door runs `func_80839034` on the floor beyond it (entrance speed 2);
     - any other door gets the door camera (`Camera_ChangeDoorCam` with the side's bg camera, timers 38/26/10) and loads the room behind it;
     - `func_80845EF8`: after the animation Link stands, the old room goes (`func_80097534`), the camera is told (`func_8005B1A4`) and the void-out point moves.
   - **Also on Player:**
     - `func_80835E44(CAM_SET_SCENE_TRANSITION)` on exits (unless `Play_CamIsNotFixed` says the scene's camera is fixed);
     - `CAM_SET_FREE0` on void-outs;
     - `func_80845CA4`'s camera check (`unk_14C & 0x10`).
   - **`PlayRequest`:** Player queues what it does to the camera, the rooms and the door, applied right after its update. `En_Door` writes Player's door fields through the arena.
4. **Tools:**
   - `ootx scene-info` shows the camera type, the skybox, each room's backgrounds and the bg camera list;
   - the sandbox has the `cup`, `open` and `door` scripts, and its traces of entrance runs show the camera setting, mode, bg camera and viewpoint.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 171 passed, 1 ignored |
| Interiors (`oot_actors --test prerendered`, 6) | Link's house: its bg cameras, its single 320x240 background (RGBA5551), its skybox (`SKYBOX_HOUSE_LINK`, four faces, 256 triangles, no z). Entering: `FREE0`, then the pivot on the first frame (the eye at (0, 34, 0), fov 60). C-Up is refused while Link walks in; after it, the fixed camera (eye (-118, 345, 47), `at` 150 along the rotation, fov 46.83) with the background, and back. The shop starts fixed (eye (-100, 100, 260), fov 50) and refuses C-Up; its skybox has two faces. The porch's floor gives `PIVOT_IN_FRONT`, whose eye reaches (-97, 170, 906). `START1` from spawn 1's params holds its eye at (3778, 288, -608), fov 45, until Link has moved 10. Exits use `SCENE_TRANSITION` in Kokiri Forest and keep the pivot in the house |
| Doors (`--test door`, 2) | A Kakariko house's scene-exit door: `DOOR_DL_DEFAULT_FIELD_KEEP`; Player gets `PLAYER_DOORTYPE_HANDLE` and direction -1; A gives `clink_demo_doorA_link` at z 188, the exit to `ENTR_SPOT01_6` at entrance speed 2, and the door's `gDoorChildOpeningLeftAnim` at speed 1.5. Souko's room door: `clink_demo_doorB_link` at x 1198, `CAM_SET_DOORC` with the door and its timers, room 2 loading behind room 1, the door moving to room 2, the bars at 32, then room 1 dropped, Link through, the respawn point moved, and the door idle again |
| Camera (`--test camera`, 7) | Every setting's data against `z_camera_data.c` (NORMAL0, PREREND_FIXED and _PIVOT with their holes, DOORC, SCENE_TRANSITION, FREE0, PIVOT_SHOP_BROWSING, PIVOT_IN_FRONT); `Camera_ChangeSettingFlags`' priority (-2 before -1), -99, prevSetting and the bg camera index flags; `Camera_ChangeBgCamIndex` once a frame; `Camera_ChangeDoorCam` refused while `DOORC` runs |
| Import (`oot_import --test pack`) | Every scene's bg camera list covers what its floors, water boxes, spawns, doors, viewpoints and multi-image rooms name |
| Unit tests | The collision codec round-trips a bg camera list; a background is a copy-mode screen quad; a raw RGBA16 background decodes; the skybox generator's vertex counts (4, 2 and 3 faces) and its tile rows (the second half starts at t 124) |
| Golden traces and renders | 78 of 78 identical (the golden cases don't enter by `Play_Init`) |
| Import | 10.0 s; 47.2 MB; format version 5 (camera settings, bg cameras, backgrounds, 22 skybox bakes, 41 actor bakes) |
| Headless screenshots | Link's house (pivot with its skybox, and fixed with its JPEG), the shop, the four other Kokiri houses both ways, Link's porch, `START1`, the market alley's three backgrounds, a Kakariko house's door and souko's room door (the door swinging, the door camera and its bars) |

### Decisions

- **[ADR 0014](adr/0014-prerendered-backgrounds.md):** backgrounds are decoded by the importer (a standard JPEG decoder, then RGBA5551) into screen quads, drawn in the OPA list with the fixed camera. The room skyboxes are bakes of `Skybox_Init`'s generated display lists.
- **[ADR 0015](adr/0015-camera-settings-and-bg-cameras.md):** all camera settings are data from the C, the bg cameras are part of the collision header, the setting changes are ported as they are, and an unported function runs its setting's NORMAL one. The floor's bg cameras need `Play_Init`'s flags, so the spikes' view is unchanged.
- **[ADR 0016](adr/0016-player-requests.md):** Player queues what it does to the camera, the rooms and other actors, applied right after its update; actors write each other through the arena.
- **The house skyboxes came with the milestone**, beyond what it asked for (the JPEG backgrounds). The houses' default view is the pivot camera, which shows the skybox, not the JPEG: without it, entering a house would show only its depth-only geometry.

### Known gaps

- **Doors:**
  - checkable doors' text and ajar doors' 0xD0 text (the message box, milestone 3);
  - small keys (locked doors never open);
  - the lock's chains (`Actor_DrawDoorLock`), the sounds, the bubbles of a door opened underwater;
  - sliding doors (`Door_Shutter`: the Deku Tree's), `Door_Killer`.
- **The exit's circle wipe** is still GAME-01's 20-frame fade, so a scene-exit door's opening is mostly under black.
- **Camera:**
  - `Camera_KeepOn0` (TALK in a house) and the other unported functions run their setting's NORMAL function;
  - `TOWER_CLIMB`'s Normal2 and `CRAWLSPACE`'s Subj4 run Normal1 on NORMAL0's data;
  - the underwater and hot-room settings;
  - DynaPoly floors' own bg cameras;
  - the Sacred Forest Meadow's adult exception;
  - the door parameters aren't aliased with the other functions' data (ADR 0015).
- **Shops:** browsing (`PIVOT_SHOP_BROWSING`) needs `En_Ossan` to switch the viewpoint. The shopkeeper is a placeholder.
- **Backgrounds:** a pixel may be one 5-bit step off the console's JPEG decode. The quake offset and `Room_GetImageMultiBgEntry`'s write into Player's params aren't modelled.
- **Carried over:** talking, items, the HUD (milestone 3); time passing, culling.

## Milestone 3: the message box, talking, items, the HUD

**Answer:** done. Link can now:
- read the signs;
- talk to the Kokiri;
- check the spot by the window in his house, and the market's checkable doors.

The text is in the game's message box, with its typing, arrows and choices.

The talk camera swings in (`Camera_KeepOn3` outdoors, `Camera_KeepOn0` in the houses). Bushes and rocks drop rupees and hearts, and Link picks them up.

The HUD shows the hearts, the rupee count, the B and C buttons, and the A button flipping to its do-action. Its fades follow the camera and the message box.

The tests: 190 pass, 1 ignored (171 before). The goldens are unchanged: 78 of 78.

### What was built

1. **The message box** (`oot_game::message`, `z_message_PAL.c`; [ADR 0017](adr/0017-interface-sprites.md)).
   - **The import:** the English message table and text (`oot_import::text`, with the table addresses as `tools/msgdis.py` finds them) into `table/messages`.
   - **Opening a box:** `Message_StartTextbox`, `Message_ContinueTextbox`, and `Message_OpenText` (English only): the box type and position from `typePos`, the colours, `Interface_ChangeAlpha`.
   - **`Message_Update`'s modes for plain boxes:**
     - the start delay;
     - the growth (`Message_GrowTextbox`) and the position: `XREG(94)` against the midpoint of Player's and the talker's screen heights;
     - the typing, the waits, the fades and the persistent boxes;
     - two- and three-way choices (`Message_HandleChoiceSelection`);
     - A to advance or close;
     - `Message_GetState` and `Actor_TextboxIsClosing`.
   - **`Message_Decode`:**
     - the control codes: colours, line breaks, boxes, shifts, waits, fades, the choice markers, the quick-text flags, the sound and event codes;
     - Link's name, the numbers and the high scores;
     - the heart piece count (`QUEST_HEART_PIECE_COUNT`).
   - **`Message_Draw`:**
     - the four box types (black, wooden, blue, ocarina) from `message_static`;
     - the glyphs from `nes_font_static`, at `sFontWidths` and `R_TEXT_CHAR_SCALE`, with the shadow;
     - the end icons (the arrow, the square, the triangle), flashing.
   - **The text registers** are `z_construct.c`'s defaults (`TextRegs`).
2. **Talking** (Player, `z_actor.c`, `z_camera.c`).
   - **Player:**
     - `func_8083B644` in the interrupt list (index 4): A talks to the target or `targetActor`;
     - `func_80853148` and `func_8083A2F8`: an NPC after putting the item away, anything else at once;
     - the talk action `func_8084B530`, until `Actor_TextboxIsClosing`;
     - Player's `ACTOR_FLAG_8` keeps the target (`func_8083224C` in `func_80836BEC`);
     - `func_808473D4`'s do-action (Check, Speak, Open, Next, Return and the rest);
     - the ajar door's text 0xD0.
   - **Player's writes** go through `PlayRequest` (ADR 0016): the talk request (`Actor_ProcessTalkRequest`), the text, `Message_StartTextbox`, the do-action.
   - **The talk camera:**
     - `Camera_KeepOn3` (TALK on NORMAL0) swings over `initTimer` frames, retries the line of sight against the collision and `CollisionCheck_LineOCCheck`, and holds until `unk_14C & 8`;
     - its first call asks for another `Camera_Update` at the end of `Play_Draw` (`view.unk_124`), which `PlayState` now runs;
     - `Camera_KeepOn0` (TALK in the prerendered rooms): the eye from the bg camera, the yaw turn and the fov scale.
   - **`Camera_UpdateInterface`'s alpha half:** `sCameraInterfaceAlpha`, and `Interface_ChangeAlpha` from the setting's interface flags.
3. **The talkers:**
   - `En_Kanban` and `En_Ko` show their texts. `En_Ko`'s `func_80A97738` sets its `infTable` flags on closing and answers choices;
   - `En_Wonder_Talk2`: the whole overlay except Player's cutscene modes (`func_8002DF54`);
   - `En_Door`'s checkable doors (`EnDoor_WaitForCheck`, `EnDoor_Check`).
4. **Items** (`oot_game::item`, `oot_actors::en_item00`).
   - **The import:** `sItemDropIds` and `sDropQuantities` into `table/item_drops`.
   - **`En_Item00`:**
     - `EnItem00_Init` for the rupees, recovery hearts, heart pieces and containers, and the collectibles (with the scene's collectible flag);
     - `func_8001DFC8` (resting and spinning), `func_8001E1C8` (bouncing), `func_8001E304` (a drop's pop);
     - pickup within 30 across, then 15 frames over Link's head (`EnItem00_Collected`);
     - `func_8001F404`: what Link can't use is cancelled, and a heart at full health is a green rupee.
   - **`En_Item00`'s draws:**
     - the rupees' five colours (`gRupeeDL`);
     - the collectibles, billboarded: `gItemDropDL` on segment 1, with the view's billboard matrix;
     - the heart piece and container.
   - **The drops:** `Item_DropCollectible` and `Item_DropCollectibleRandom`, from `En_Kusa` (its table, or seeds and hearts) and `En_Ishi` (`EnIshi_DropCollectible`).
   - **`Item_Give`:** rupees (through the accumulator), recovery hearts, heart containers and pieces; `Health_ChangeBy`, `Rupees_ChangeBy`.
5. **The HUD** (`oot_game::interface`, `z_parameter.c`, `z_lifemeter.c`).
   - **The import:** `sRestrictionFlags` into `table/interface`, with the scene names resolved against the scene table.
   - **`Interface_Init`** in `Play_Init`, with `Health_InitMeter` and `Interface_SetSceneRestrictions`.
   - **`Interface_Update`**, after `Message_Update`:
     - `func_80083108` for the cases that apply: climbing, the scene's B restriction, `restrictions.all`;
     - the alpha types (`func_80082850`'s 1 to 13, then 50 and 52; `func_8008277C`, `func_80082644`);
     - the health and rupee accumulators;
     - the beating heart and the heart colours (`Health_UpdateBeatingHeart`, `Health_UpdateMeter`);
     - the A button's flip.
   - **`Interface_Draw`**, before and after the reticle:
     - `Health_DrawMeter`: quarter hearts, the beating heart;
     - the rupee icon and the wallet's digits, coloured by the count;
     - the B and C buttons with the empty-C arrows, and the B sword's icon;
     - the A button and its label, turning in their own viewport (`func_8008A8B8`).
   - **The save** has what these read: health, rupees, the wallet, the B and C items and their status, Link's name, the language.
6. **Shared pieces:**
   - `oot_game::gbi`, a display-list writer, shared with the skybox;
   - `oot_game::sprite`, the sprite bakes (ADR 0017);
   - `CollisionCheck_LineOCCheck` over the frame's OC colliders;
   - `ViewInfo`'s billboard matrix;
   - the engine divides a projective transform by w.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 190 passed, 1 ignored |
| The sign (`oot_actors --test talk`) | Params 0x031F. "Check" shows a frame after the sign offers. A starts the talk action, sets Player's `ACTOR_FLAG_8`, and opens text 0x031F in a wooden box at the bottom (34, 142). The box grows over eight frames to 256x64, the text types out, and A closes it, with `Actor_TextboxIsClosing` for one frame |
| The talk camera (`--test talk`) | Ten frames of swing, `at` closing 1/animTimer of the gap each frame; fov 45; the bars at 32; then it holds |
| A Kokiri child (`--test talk`) | "Speak"; the item is put away first; 0x1005 read through; `INFTABLE_1E` set on closing; then 0x1006 |
| Link's window, and the HUD while talking (`--test talk`) | The spot by the window: 0x22A with "Check" (not an NPC). While talking: alpha type 5 from KEEP3's flags 0x3500; B and C fade out by 32 a frame, A dips to 32 and comes back, the hearts stay |
| Doors (`--test door`, 3) | Adds a market house's checkable door: text 0x20D on A |
| Items (`--test items`, 4) | The drop tables against `z_en_item00.c` (table 2's row, `sDropQuantities`); `func_8001F404`'s cancellations. A bush's green rupee pops up at 8 with gravity -0.9, lands and grows to 0.015; Link collects it into the accumulator, and it's gone 15 frames later. A recovery heart heals 0x10 |
| HUD (unit tests, 4) | The fade-in after alpha type 50, 32 a frame, with the minimap stopping at 170 in the overworld; the A flip's angles 10466, -15700, -5233, 0; the meter's sprites for 2.5 of 3 hearts and their quarter-pixel rectangles; the two rupee digits; Link's house disabling B (alpha 70) |
| Other unit tests | The GBI writer's encodings (`G_CC_MODULATEIA_PRIM` is 0xFC119623 0xFF2FFFFF); the sprite quads; `Item_Give` |
| Golden traces and renders | 78 of 78 identical: the golden cases don't enter by `Play_Init`, which starts the HUD and the messages |
| Import | 11.2 s; 47.3 MB; format version 6. New: the messages, the drop tables, the restrictions, and 261 actor bakes (147 for the message box, 56 for the HUD) |
| Headless screenshots | The HUD in Kokiri Forest; the sign ("Check", the talk camera, the box and "Return"); a Kokiri child's text; Link's house (B and C dimmed, "Next", the blue records box); a bush's rupee |

### Decisions

- **[ADR 0017](adr/0017-interface-sprites.md):** the message box and the HUD are baked sprites.
  - Each texture and its setup is baked once as a quad, and drawn in `overlay_2d` with a transform and per-draw colours.
  - The text's typing and the icons' flashing run once per game frame.
  - The engine divides a projective transform by w.
- **The talk camera's second update** (`view.unk_124`) runs at the end of the game frame, after the actors' draw-time state, as at the end of `Play_Draw`.
- **Billboards take the view's matrix** (`ViewInfo::billboard`, the transpose of the view's rotation), as `Matrix_ReplaceRotation` with `play->billboardMtxF` does. The drop's display list is baked with an identity matrix in segment 1.
- **The HUD and the messages start with `Play_Init`,** so the sandbox's scene views (and the goldens) stay the spikes'.
- **A new play state isn't shown before its first frame.** `Play_Main` runs `Play_Update` (which starts the fade in) before its first `Play_Draw`, but the window draws at 60 Hz and could present the state `Play_Init` just made, unfaded and from its initial camera, for a display frame on every scene change. Until its first game frame, `screen_fill` now gives the fill the previous state ended on (its finished fade-out), or black after the first `Play_Init`.

### Known gaps

- **Messages:**
  - the item icons in text (`MESSAGE_ITEM_ICON`) and the backgrounds (`MESSAGE_BACKGROUND`);
  - the ocarina modes, the credits, German and French;
  - the text sounds.
- **Talking:** Player's cutscene modes (`func_8002DF54`) aren't ported, so `En_Wonder_Talk2`'s forced texts don't hold Link. There's no Navi, so no C-Up.
- **Items:**
  - placed recovery hearts aren't drawn: `GetItem_DrawRecoveryHeart` needs `Gfx_TwoTexScroll`'s dynamic tiles;
  - the inventory isn't kept, so the get-item items aren't given: sticks, nuts, seeds, magic, keys, shields, and heart pieces given by `func_8002F554`;
  - the shadows, the sparkles and the sounds;
  - the effects' random numbers aren't drawn in the C's order.
- **HUD:**
  - the magic meter, the minimap, the timers, the ammo counts;
  - the C items' icons and the B label (only the ocarina loads one);
  - double defence's hearts, the low-health alarm, the pause menu;
  - `func_80083108`'s riding, minigame, fishing and water cases, and the restrictions by item type (nothing is on the C buttons).
- **Carried over:** time passing, culling, the exit's circle wipe, small keys.

## Recommended next step

**Milestone 4: `Bg_Treemouth` and a headless scripted playthrough,** as planned:
- the Deku Tree's mouth opens after the talk;
- a scripted run goes from Link's house to the Deku Tree, with checks along the way (the signs, a Kokiri child, the drops).

Checks first (interactive, in the windowed game or against Project64):
- the message box side by side with Project64: the typing speed, the box's position against the talker, and the glyphs' edges (ADR 0017's half texel);
- the talk camera's swing on a sign and on a Kokiri child, and the house's KEEP0 turn;
- the HUD's layout and fades (entering, talking, climbing a ladder), and the A button's flip;
- still pending from milestones 1 and 2: the Z-target camera and bars on a Kokiri child, and Link's house's pivot and fixed views.
