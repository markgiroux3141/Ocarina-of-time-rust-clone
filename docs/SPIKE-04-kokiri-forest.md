# Spike 04: Kokiri Forest from the ROM, then the next layers of Player

**Question:** can a real scene be drawn from the ROM the way the game draws it (rooms, segments, animated textures, scene lights and fog), with Link walking on its collision? And how far can the next layers of Player and the camera be ported on top?

**Answer (milestone 1):** yes. Kokiri Forest renders from the ROM with textures, the game's lights and fog, scrolling water, and `Room_DrawCullable`'s per-frame culling and order. Link walks it on the real collision. The same loader interprets all 110 scenes (141 distinct header layers) with 0 unknown opcodes, and leaves only the 4 unresolved segment references the extractor already documents, none of them in Kokiri Forest.

**All milestones:**

| # | Milestone | Status | Tests |
|---|---|---|---|
| 1 | Kokiri Forest textured: rooms, segments, draw config, lights/fog, OPA/XLU order, real collision | done | `scene` (7) |
| 2 | `z_camera.c` Normal camera (`Camera_Normal1`, `CAM_SET_NORMAL0`, OREG, camera bgcheck), follow camera on F3 | done | `camera` (6) |
| 3 | Foot IK (`func_8008F87C`) | done | `footik` (3) |
| 4 | Ledge grab, hang, climb-up, climbing onto ledges | done | `ledge` (7) |
| 5 | Z-targeting and the sword | done | `targeting` (8), `sword` (4) |
| 6 | Water and swimming | done | `water` (10) |
| 7 | DynaPoly collision with one moving platform | done | `platform` (8) |
| 8 | Sound | left out (see its section) | — |

`cargo test --workspace` runs 87 tests, all passing (spike 03's `movement` suite included). Each section below has what was ported and from where, the results, the findings and the known gaps.

## Milestone 1: Kokiri Forest rendered with textures

### What was ported, and from where

| Rust | Decomp | Notes |
|---|---|---|
| `oot_core::scene` | `z_scene.c` (`Scene_CommandAlternateHeaderList`, `...RoomList`, `...SpecialFiles`, `...LightSettingsList`, `...SkyboxSettings`) | layer selection (an alternate header replaces the rest of the main header; adult night falls back to adult day), room list, keep object, `EnvLightSettings`, skybox id/config/light mode |
| `oot_core::room::Room`, `RoomShape` | `z_scene.c` room commands, `scene.h` `RoomShape*` | room header per layer; shape types 0 (normal), 1 (image: its DL pair only) and 2 (cullable, with bounding spheres) |
| `oot_core::room::SceneDraw` | `Play_Draw`, `Room_Draw*`, `Scene_Draw` | segments 0 (`code` RAM image), 2 scene, 3 room, 4 `gameplay_keep`, 5 keep object; 8..0xD from the draw config; `Gfx_SetupDL_25Opa`/`Xlu` before the room DLs |
| `oot_core::drawcfg` (moved from `oot_extract`) | `Scene_DrawConfig*` in `z_scene_table.c` | the extractor's C interpreter now takes runtime state (`gameplayFrames`, age, night flag, scene layer, day time) and is re-run every frame |
| `oot_core::gbi` dynamic segments | `Gfx_TexScroll`, `Gfx_TwoTexScroll`, the Spot04 `displayListHead` DLs | tile sizes and env/prim colours set inside a per-frame segment are tagged on the material; the renderer re-applies them every frame (UV offset `-(Δuls / 4) / width`) |
| `oot_core::room::cullable_order` | `Room_DrawCullable` | project each sphere centre with the view-projection matrix; keep `-r < z` and `z - r < lightCtx.fogFar`; draw by ascending `z - r` (both OPA and XLU, nearest first) |
| `oot_core::room::gu_perspective` | libultra `guPerspective` | the game's clip-space z for the cull test |
| `oot_game::env` | `Environment_Update` (light-settings part), `Environment_LerpWeight`, `LERP`/`LERP16`, `Scene_CommandTimeSettings`, `Gfx_SetFog`/`gSPFogPosition` | time-based blending for `LIGHT_MODE_TIME`, fixed setting for `LIGHT_MODE_SETTINGS`, sun/moon directions, the clamps into `LightContext` |
| `oot_render` | F3DEX2 lighting and fog | two directional lights plus ambient; per-vertex fog `z_ndc * fm + fo` (in 1/256ths) against the game's projection; `G_RM_FOG_SHADE_A` blend; `G_FOG` replaces shade alpha |
| `oot_play::rooms` | `Play_Draw` order | OPA buffer (rooms, then Link), then XLU (rooms, then the circle shadow); clear colour = fog colour |

Read from the decomp at runtime: `scene_table.h` (scene ids, `SDC_*`), `object_table.h` (keep object file), `sSceneDrawConfigs` and every draw config function (`z_scene_table.c`), `sTimeBasedLightConfigs` (`z_kankyo.c`, with `CLOCK_TIME` expanded), `SDC_*` enum (`scene.h`). `oot_core::csrc` gained `define_rows`, `parse_enum`, `parse_defines` (moved from the extractor) and now keeps commas inside parentheses (`CLOCK_TIME(4, 0)`) in one initializer atom.

### Reuse of the extractor

`oot_extract/src/scenes.rs` already decoded every scene. Rather than write a second loader, its shared parts moved into `oot_core`:

- `drawcfg` (the C interpreter, ~1300 lines) is now `oot_core::drawcfg`, with a `State` for runtime values. The extractor calls it with `State::default()`, which keeps its old frame-0 child-day assumptions.
- The segment binder (`SceneCtx`, `Keeps`, `code_ram_image`, `run_dls`) is now `oot_core::room`. The extractor uses it unchanged.
- The table helpers are now in `oot_core::csrc`.

The extractor keeps its JSON-producing header decoder and its own `parse_shape` (it records every shape field in JSON). `oot_core::room::RoomShape` is the typed version the game uses.

### Decisions

- **Each shape entry is interpreted in a fresh interpreter.** `Room_DrawCullable` draws entries in a different order every frame, so an entry can't rely on RDP state left by another one. Each entry starts from setup DL 25 plus the draw config's direct commands. Triangle totals equal the extractor's single-pass numbers (Kokiri 2060, all main layers 168,566), so no entry depends on a predecessor's state for its geometry.
- **Meshes are built once. Only the dynamic segments change per frame.** The draw config is re-run every game frame (20 Hz, a fraction of a millisecond), and only the tile sizes and colours of segments bound to allocated DLs are read back into material uniforms. Draw configs that swap texture *pointers* per frame (lava and waterfall frames in other scenes) would need a rebuild and aren't handled. Kokiri has none.
- **Time of day** defaults to 10:00, the time a new save starts at (`Sram_InitNewSave`). `--time HH:MM` changes it. Kokiri's rooms say "keep the time, time speed 0", so time doesn't move there.
- **Room lighting is real.** The Kokiri room DLs set `G_FOG | G_LIGHTING` (`D9FFFFFF 00030000`), so room geometry is lit by the environment's ambient and two directional lights (`Play_Draw` binds them before `Scene_Draw`). That is why OoT's outdoor areas darken with the time of day.
- **All rooms are drawn.** The game draws `curRoom` and `prevRoom`, switched by transition actors (`En_Holl`). Room transitions aren't ported, so every room of the scene stays loaded and drawn.

### Results

`ootx scan-scenes [--all-layers]` loads every scene through the runtime loader and interprets every room entry:

| | Main layer | All distinct layers |
|---|---|---|
| Scenes | 110 | 110 |
| Headers scanned | 110 | 141 |
| Rooms / shape entries | 401 / 1900 | 456 / 2533 |
| Triangles | 168,566 (extractor: 168,566) | 199,500 |
| Materials reading dynamic segments | 409 | 498 |
| Unknown opcodes | 0 | 0 |
| Unresolved segment references | 4: `hairal_niwa2` segments 9/A/B, `MIZUsin` segment 6 (as documented by the extractor) | same 4 |
| Time | 0.3 s | 0.7 s |

Kokiri Forest (`spot04`, child day):

| Check | Expected (source) | Port |
|---|---|---|
| Rooms, keep, draw config | `ROOM_LIST(3)`, `OBJECT_GAMEPLAY_FIELD_KEEP`, `SDC_KOKIRI_FOREST` | 3, `gameplay_field_keep`, `Scene_DrawConfigKokiriForest` |
| Room shapes | all `ROOM_SHAPE_TYPE_CULLABLE` | 3 cullable rooms, 28 entries, 2060 triangles, 0 unknown opcodes, 0 unresolved |
| Water scroll, frames 0..1000 | `Gfx_TwoTexScroll`: seg 9 tile 0 `(127 - f%128, f%128)`, tile 1 `(f%128, f%128)`; seg 8 with `f*10` on t | exact |
| Stream UV shift at frame 40 | `(+40/4/32, -40/4/32)` on a 32x32 tile | exact |
| Segments 0xA/0xB | env `(128,128,128,128)` and `(128,128,128, 500 * 0.1)` | exact |
| Lights at 10:00 | `sTimeBasedLightConfigs[0][3]` = setting 1 unblended: ambient (80,80,80), light 1 (255,255,255), fog (200,200,150), near 994, far 5800 | exact |
| Sun direction at 10:00 | `-(sin(-5461)·120), cos·120, cos·20` = (59, 103, 17) | exact |
| Dawn blend at 7:00 | `LERP` of settings 0 and 1 by `Environment_LerpWeight` | exact |
| Fog factor | `gSPFogPosition(994, 1000)` = (21333, −21077) | exact |
| Load time | | 3 ms (after the ROM is open) |

Visual checks (in git-ignored `out/`):

- `s04_spot04_spawns.png`: all 12 spawns. The follow camera often starts inside a house trunk or the tunnel walls at doorway spawns. That's the stand-in camera; milestone 2 replaces it.
- `s04_spot04_spawn0.png`: Link's house ladder, the bridge and waterfall behind, fogged.
- `s04_water_pair.png`: frames 8 and 12. Only the stream and waterfall change between them.
- `s04_sheet_spot04_walk.png` + `s04_trace_spot04_walk.json`: running out of Mido's doorway along the path on the real collision.
- `s04_other_scenes.png`: Hyrule Field (adult, at the drawbridge) and the Deku Tree (adult, `LIGHT_MODE_SETTINGS`).

New tools: `ootx scan-scenes`, `ootx dump-room --scene S --room N [--png DIR]` (every batch's material, render mode, textures, dynamic bindings and bounds), and `oot_play --spawn N --time HH:MM --collision --view eye,at --frames N`.

### Known gaps (milestone 1)

- **Actors aren't drawn.** The pale strip in the Deku Tree's mouth (spawn 1) is the clear colour showing through where the tree's jaw actor, `Bg_Treemouth`, would be. The texture there is a black-to-transparent gradient. Signs, grass, rocks and the Kokiri are actors too.
- **No skybox.** Kokiri uses `SKYBOX_UNSET_1D`, which the game fills with the fog colour, so it's exact there. Scenes with `SKYBOX_NORMAL_SKY` (Hyrule Field) get the fog colour as a flat sky.
- **All rooms drawn**, with no room transitions.
- Per-frame texture *pointer* animation (other scenes' draw configs) and prerendered backgrounds (shape type 1) aren't handled.
- Light-setting overrides, weather configs and the `adj*` adjustments aren't modelled (all neutral outside cutscenes).

## Milestone 2: the Normal camera

**Answer:** `Camera_Normal1` for `CAM_SET_NORMAL0` / `CAM_MODE_NORMAL` is ported with everything it calls, including the camera bgcheck. It is now the default camera in `oot_play`, and Player reads its input yaw from it. The spike 03 follow camera stays available: F3 in the window, `--follow-camera` headless.

### What was ported, and from where

| Rust (`oot_game::camera`) | Decomp | Notes |
|---|---|---|
| `GameCamera::new` | `Camera_Init`, `Camera_InitDataUsingPlayer`, `func_80057FC4` | eye at r 180, pitch 0x71C behind Player; `inputDir.y` = shape yaw; NORMAL0 because Kokiri's rooms have `behaviorType1` 0 |
| `GameCamera::update` | `Camera_Update` (player part) | `xzSpeed`, `speedRatio` (via `func_8002DCE4` = `R_RUN_SPEED_LIMIT`/100 × `OREG(8)`), ground raycast (`BgCheck_EntityRaycastDown5`), 200-frame out-of-bounds fallback, `inputDir` when the mode doesn't set it, up vector |
| `normal1` | `Camera_Normal1` | read-only data from `CAM_FUNCDATA_NORM1` scaled by Player height (`R_CAM_YOFFSET_NORM`), swing start timer, update-rate LERPs, slope pitch, at/eye, pitch clamp (79.65° to −85°), eye bgcheck, `inputDir`, fov, roll, at LERP scale |
| helpers | `Camera_CalcAtDefault`, `Camera_CalcSlopeYAdj`, `Camera_ClampDist`, `Camera_CalcDefaultYaw`, `Camera_CalcDefaultPitch`, `Camera_InterpolateCurve`, `Camera_LERPCeilF/S/Vec3f`, `Camera_ClampLERPScale`, `Camera_GetPitchAdjFromFloorHeightDiffs` (with its statics and even/odd frame alternation), `Camera_CalcUpFromPitchYawRoll` | |
| swing and bgcheck | `func_80046E20`, `func_80045508`, `Camera_BGCheckInfo`, `Camera_BGCheck`, `Camera_BGCheckCorner` (`func_800427B4` → `Math3D_PlaneVsLineSegClosestPoint`, `Math3D_PlaneVsPlaneNewLine`, `Math3D_LineVsLineClosestTwoPoints`), `Camera_GetFloorYLayer`, `Math3D_Cos` | `BgCheck_CameraLineTest1` / `BgCheck_CameraRaycastDown2` map onto the existing static bgcheck with `COLPOLY_IGNORE_CAMERA` |
| maths | `z_olib.c` (VecSphGeo geo conversions, `ClampMin/MaxDist`, `DistNormalize`), `Math_FAtan2F` (Taylor series, `math64.c`), `CAM_DEG_TO_BINANG` | |

Read from `z_camera_data.inc.c` at runtime: `sOREGInit` (53 values) and `sSetNormal0ModeNormalData` (`CAM_FUNCDATA_NORM1(-20, 200, 300, 10, 12, 10, 35, 60, 60, 0x0003)`).

Frame order is as in `Play_Update`: Player updates with last frame's `inputDir.y`, then the camera updates. The render interpolates eye, at and fov between game frames like everything else. Headless traces now record the camera's eye, at, distance, fov and input yaw per frame.

### Results

`cargo test -p oot_game --test camera` (6 tests):

| Check | Expected (source) | Port |
|---|---|---|
| OREG and NORMAL0 data | `sOREGInit`, `CAM_FUNCDATA_NORM1(...)` | read, 53 values |
| `Math_FAtan2F` | `atan2` | within 2e-6 rad |
| Init | at = pos + 68; eye r 180, pitch 0x71C, yaw = rot − 0x7FFF; `inputDir.y` = rot | exact (pitch within 20 binang: libultra's 1023/1024 sine table) |
| Normal0 limits, adult | yNormal = 1 − 0.1 + 0.1·68/h = 1.0; ×0.68: yOffset −13.6, dist 136..204 | exact; child (h 44): 92.8..139.2 |
| Standing still | `at.y` → pos + 68 − 13.6; pitch → `CAM_DEG_TO_BINANG(10)`; dist stays within limits | yes |
| Recentre after stopping | `startSwingTimer` = `OREG(50) + OREG(51)` = 40 frames of hold, then yaw LERPs to rot − 0x7FFF | within 0x100 after 200 frames |
| Backed against a wall | the at→eye line test keeps the eye in front of the wall | yes |
| Movement tests (spike 03) | | all 18 still pass with the game camera driving Player |

Visual: `s04_sheet_camera_tour.png` (run, curve left, curve right, stop; the camera lags into the turns and recentres after the hold), `s04_cam_spawns.png` (all 12 spawns).

### Findings

- **`sp94` in `Camera_Normal1` is clamped the wrong way round.** The code is `if (sp94 > 1.0f) sp94 = 1.0f; if (sp94 > -1.0f) sp94 = -1.0f;`, so the "acceleration" is always at most −1. Kept as shipped: it's why `Camera_CalcDefaultYaw`'s velocity is `2·curve − 1`.
- **The fov update rate LERPs from `yOffsetUpdateRate`**, not from itself (`Camera_LERPCeilF(.., camera->yOffsetUpdateRate, ..)`). Kept as shipped.
- **Stopping holds the camera for 40 frames (2 s) before it recentres.** While `startSwingTimer > 0` and Player is still, `Camera_CalcDefaultYaw`'s speed factor `Camera_InterpolateCurve(0.5, speedRatio)` is 0.
- **Doorway spawns start inside the doorway.** Kokiri's house and tunnel exits have Player params like `0x0F05`: the low byte is a bg-camera index that `Player_Init` passes to `Camera_RequestBgCam`, which switches to a scene camera setting from the collision's bg-camera list. Only NORMAL0 is ported, so those spawns start with the eye inside the door frame until Link walks out. Spawn 0 (params `0x0FFF`, no bg camera) is unaffected.

### Known gaps (milestone 2)

- Only NORMAL0's NORMAL mode. Other modes (targeting/parallel, jump, free fall, climb, talk, battle), other settings, bg-camera changes from floor polys and spawn params, the underwater and hot-room camera, quakes, the low-health wiggle and the debug camera aren't ported.

## Milestone 3: foot IK

**Answer:** `func_8008F87C` is ported. On slopes, Link's feet rest on the floor instead of sinking into it.

- **Where it runs.** In the game it runs from `Player_OverrideLimbDrawGameplayCommon` for `PLAYER_LIMB_L_THIGH` / `R_THIGH`, while drawing. It adds the correction to `skelAnime.jointTable` itself (thigh −θ, shin +φ, foot +θ−φ on Z), so the change persists into the next frame's `SkelAnime_InterpFrameTable` morphs. The port therefore runs it on Player's joint table after `finish_frame` (the AnimTaskQueue queue), in `World::tick`, where `Player_Draw` would come. It doesn't run in the renderer.
- **The solve.** The hip and ankle come from the model-space chain: `Actor_Draw`'s matrix, Player's root override (child scale 0.64, `unk_6C4`), then each limb's translate + ZYX rotation. The floor is probed 300 units down the shin plus 15 (`BgCheck_EntityRaycastDown4`). If the ankle is below `floor + D_80126068[age]`, a law-of-cosines solve on the thigh/shin lengths (`D_80126058`, `D_80126060`) bends the knee so the ankle reaches that height. Legs are only ever lifted: a downhill foot keeps its animated pose and can float a few units.
- Read from `z_player_lib.c`: `D_80126038` (shin offset, equal to the skeletons' shin joint positions, which the test checks), `D_80126050`, `D_80126058` (`SQ(13.04f)`, `SQ(6.95f)`, with the macro expanded), `D_80126060`, `D_80126068`, `D_80126070`. The limb hierarchy comes from `gLinkAdultSkel` / `gLinkChildSkel` in the ROM.

| Check (`cargo test -p oot_game --test footik`) | Expected | Port |
|---|---|---|
| Constants | as listed in `z_player_lib.c` | equal; shin offsets equal the skeleton joints |
| Standing on flat ground | ankles ≥ floor + 5, nothing changed | not adjusted |
| Standing across the 20.6° ramp | uphill ankle moved to floor + 5 | 83.12 against 83.45; the downhill leg is untouched (80.6 against 76.6) |
| Spike 03 movement tests | unchanged | all pass with IK on |

Visual: `s04_ik_ramp.png` (IK off, then on). `oot_play --no-foot-ik` and F4 toggle it.

Not modelled: the `unk_6C2` root tilt (diving) and fire-floor footprints (`EffectSsGFire_Spawn`).

## Milestone 4: ledge grab, hang, climb-up and climbing onto ledges

**Answer:** ported. Link grabs a ledge he walks slowly off, hangs, climbs back up with the stick or lets go with A. Running into a wall hops knee-high steps and climbs ledges of each height class. Tall ledges end in a jump and a mid-air grab.

| Rust (`oot_game::player`) | Decomp | Notes |
|---|---|---|
| `func_8083A6AC` | same | line back towards `prevPos` at foot height; a near-vertical face (`|normal.y| < 600`) starts a hang with `link_normal_fall` |
| `func_8083A5C4` | same | moves Player 1 unit past the face onto the top, facing the face normal; `Action::Hang` |
| `Action::Hang` | `Player_Action_8084BBE4` | grab frame (11 for a walk-off, 1 for a mid-air grab) sets `av1.actionVar1` ±1; any stick direction ≥ 55 (`controlStickSpinAngles`) climbs (groups 38/41), A lets go (`func_80837B60` bakes the hanging body's root offset in, then `func_80837B9C` falls) |
| `Action::ClimbUp` | `func_8083A9B8`, `Player_Action_8084BDFC` | climb animation at speed 1.3; at the end `Player_ApplyAnimMovementScaledByAge(1)` applies its x/z root motion |
| interrupt 12 | `Player_ActionHandler_12` | in the running list `sActionHandlerList8`. Class 1 (18 ≤ h < `unk_1C`) hops after 3 frames of pushing (`link_normal_jump`, vy = 0.08h + 5.5). Classes 2 and 3 step up after 6 frames or on A (100 / 150 step animations: position moved onto the top at once, `shape.yOffset` pulled down by (h − 41·s) or (h − 59·s) × 100). Class 4 (≥ `unk_14`) starts `link_normal_250jump_start` |
| `Action::ClimbLedge` | `Player_Action_80845668` | eases `shape.yOffset` back at 150 per frame; the tall-ledge variant jumps on frame 8 with vy = min(h, `unk_0C`)·0.072 |
| mid-air grab | `Player_Action_8084411C` | falling into a class ≥ 2 wall under 150 high: `pos.y += wallHeight`, `func_8083A5C4` with group 39, facing the wall |
| `Player_ApplyAnimMovementScaledByAge`, `func_80837B60`, `func_80837B9C`, `func_80832224` | same | root-motion baking, letting go |

`shape.yOffset` is new on `Actor`. It goes through the snapshot to the renderer and the foot IK, as `Actor_Draw` adds `yOffset × scale.y`. Anim groups 38–41 read from `D_80853914` are `link_normal_fall_up_free`, `link_normal_jump_climb_hold_free`, `..._wait_free` and `..._up_free`. The synthetic course gained three ledges (50, 70, 100 high) for the visual checks.

### Results

`cargo test -p oot_game --test ledge` (7 tests). Thresholds are adult `sAgeProperties`: `unk_1C` 41, `unk_18` 59, `unk_14` 79.4, `unk_0C` 111, `unk_34` 70.

| Check | Expected (source) | Port |
|---|---|---|
| 30-high step | hop, `link_normal_jump`, vy 0.08·30 + 5.5 = 7.9, ends on top | exact |
| 50-high ledge | `link_normal_100step_up`, y = 50 at once, yOffset −900, then −750 | exact; ends at yOffset 0 on top |
| 70-high ledge | `link_normal_150step_up`, yOffset −1100 (eased only after frame 5) | exact |
| 100-high ledge | `250jump_start` → jump vy 7.2 → mid-air grab (`jump_climb_hold_free`) at y 100 facing the wall → climb (`jump_climb_up_free`) → standing on top | exact |
| Walking slowly off the 150 plateau | grab with `link_normal_fall` at x = edge − 1, facing out; y back on the top the next frame; a stick of 30 keeps hanging; a full stick climbs (`fall_up_free`) back onto the plateau | yes |
| A while hanging | lets go from the body's position below the edge, lands on the ground | yes |
| Walking off the 40 block | drop ≤ `unk_34`: falls with `link_normal_landing_wait`, no grab | yes |

**Spike 03 tests that changed** (they encoded the stubs, not the game):

- `step_up_limit`: walking climbs up to 17, as before. Hopping now covers 18 to 40, and 41 is the first class-2 climb.
- `walls_stop_at_the_player_radius` now uses a boundary wall, because the 40-high block is hopped.
- `walking_off_a_ledge…` now expects the grab.

Visual: `s04_sheet_climb50.png`, `s04_sheet_climb100.png` (the jump, grab and climb), `s04_sheet_hang.png` (game camera), `s04_sheet_hang_side.png` (fixed side view: turning, dropping, hanging on the face, climbing back).

### Known gaps (milestone 4)

- **Climbable walls** (`WALL_FLAG_3` vines and ladders, `FLOOR_PROPERTY_6`): climbing down onto them (`Player_Action_8084BF1C`), the mid-air wall grab (`func_8083EC18`) and wall climbing aren't ported. They're recorded in `Player::notes`, and Link falls instead.
- **The camera doesn't switch to `CAM_MODE_LEDGE_HANG`**, so hanging from a tall ledge is seen from behind and above, with most of Link hidden below the edge.
- `shape.feetFloorFlag` (feet touching a floor also lets go of a hang) isn't computed. Dynamic-collision prompts (`WALL_FLAG_6` on dyna walls) and the swimming step-up out of water wait for milestones 6 and 7. Sounds are hooks only.

## Milestone 5: Z-targeting and the sword

**Answer:** ported. Z locks on to a dummy target, or enters parallel mode with nothing to target. Link circles, walks back, side-hops and backflips while targeting. B draws the sword on the upper-body layer and slashes; repeated slashes combo, and A puts the sword away. The hand and sheath models follow the model group (adult Master Sword + Hylian Shield, child Kokiri Sword + Deku Shield).

### Targeting (5a)

| Rust | Decomp | Notes |
|---|---|---|
| `Player::Player_UpdateZTargeting` | same | the Z timer `zTargetActiveTimer`; "Switch" Z-targeting (`zTargetSetting` 0): a press locks `arrowPointedActor` (the next candidate `arrowHoverActor` if it's already locked) and keeps it via `PLAYER_STATE2_LOCK_ON_WITH_SWITCH` until pressed again; nothing to target → `Player_SetParallel` parallel mode (`PLAYER_STATE1_PARALLEL`, `targetYaw` = facing, squared up to a wall in front); `Attention_ShouldReleaseLockOn` drops a target out of the leash range once `zTargetActiveTimer` < 6 |
| `Player_UpdateHostileLockOn` / `B2C` / `BCC` / `C04`, `Player_ClearZTargeting`, `Player_ReleaseLockOn` | same | locked on (`PLAYER_STATE1_HOSTILE_LOCK_ON`) needs a hostile target (`ACTOR_FLAG_ATTENTION_ENABLED \| ACTOR_FLAG_HOSTILE`) |
| `Player_GetMovementSpeedAndYaw` (target yaw), `func_8083DC54` / `func_8083DB98` (look at the target), `Player_UpdateShapeYaw` (facing: towards the target once the reticle has locked, `targetYaw` in parallel mode) | same | spike 03's `Player_UpdateShapeYaw` port lacked these branches |
| `Action::TargetIdle` | `Player_Action_80840450` | lock-on stance, the two waits blended by the leading foot (`unk_870`) |
| `Action::ParallelIdle` / `ParallelWalk` / `ParallelBackwalk` / `ParallelBackBrake(End)` | `Player_Action_808407CC`, `Player_Action_80840DE4`, `Player_Action_808414F8`, `Player_Action_8084170C`, `Player_Action_808417FC` | the side walk plays at `linearVelocity × MREG(95)/100`, signed by direction; `func_80841138` blends the back walk and back run |
| `Action::TargetRun`, `Sidestep`, `TargetBackwalk`, `TargetBackBrake` | `Player_Action_8084227C`, `Player_Action_8084193C` (with `func_80841860`), `Player_Action_808423EC`, `Player_Action_8084251C` | chosen by `func_8083FC68` / `func_8083FD78` |
| interrupt 10 | `Player_ActionHandler_10`, `func_8083BCD0` | A while targeting: forward rolls; left/right hop (vy 3.5, speed 8.5), back flips (vy 5.8, speed 6); `currentYaw = facing + (dir << 14)`; landings from `D_80853D4C` (locked-on variant when locked); gravity −1.2 in the air while locked on |
| `oot_game::target` | `Attention_Update`, `Attention_FindActor`, `Attention_FindActorInCategory`, `Attention_WeightedDistToPlayerSq`, `Attention_ActorIsInRange`, `Attention_ActorOnScreen`, `Actor_UpdateAll` distances | runs after the actors each frame: the candidate within range, on screen (320×240 projection) and in line of sight (`BgCheck_CameraLineTest1`); the reticle size `reticleRadius` steps 500 → 80, then `reticleSpinCounter` counts and Player may turn to face the target |

Read at runtime: `D_80853D4C` (hop animations) and `sAttentionRanges` (`ATTENTION_RANGES(range, leash)` per target mode, from `z_actor.c`). The dummy target is a box with targetMode 3; the reticle is three triangles sized by `reticleRadius`.

### The sword (5b)

| Rust | Decomp | Notes |
|---|---|---|
| `skel2`, `UpperAction` | `skelAnime2`, `func_82C` (`func_8083485C`, `Player_UpperAction_Sword`, `Player_UpperAction_ChangeHeldItem`) | the upper-body layer |
| `Player_UpdateUpperBody` | same | runs inside `Player_TryActionHandlerList` (and in the air); while the upper action is active its joints replace the upper body (`sUpperBodyLimbCopyMap`), or the whole body when standing on the wait/fidget; queued as a copy from `skelAnime2`'s table |
| `Player_UpdateItems`, `Player_ProcessItemButtons` | same | B presses `Player_UseItem(B item)`; held B with the item in hand sets `sHeldItemButtonIsHeldDown` |
| `Player_UseItem`, `Player_InitItemActionWithAnim`, `Player_InitItemAction`, `Player_SetModelGroup` | same | a different item with a change animation (`sItemChangeTypes[from][to]` ≠ 0) queues it (`PLAYER_STATE1_START_CHANGING_HELD_ITEM`); the same item flags the press (`sUseHeldItem`) |
| `Player_StartChangingHeldItem`, `Player_UpperAction_ChangeHeldItem`, `Player_WaitToFinishItemChange` | same | the change animation (`sItemChangeInfo`, backwards for negative entries, ×2 with an item) plays on `skelAnime2`; the item swaps on its swap frame; once the sword is in hand `sUseHeldItem` is set and the slash starts that frame |
| interrupt 7, `Action::Attack` | `Player_ActionHandler_7`, `func_8083BB20`, `func_80837818`, `func_80837948`, `Player_Action_808502D0`, `func_8084285C`, `Player_StartAnimMovement`, `Player_CanSpinAttack` | attack by stick direction (`D_80854480`: forward stab only when targeting, else forward/right/left slashes); three of the same in a row → its combo (+2); animation-driven lunge (`moveFlags` 0x209); the stab's speed 15 on frame 0; the active-weapon window from `D_80854190`; the end animation (locked-on variant when locked) into the stance |
| interrupt 6 | `Player_ActionHandler_Roll` | A with a sword in hand puts it away (`Player_UseItem(ITEM_NONE)`) |

Read at runtime: `gPlayerModelTypes` (animation type per model group), `sActionModelGroups`, `PLAYER_AP_*` / `PLAYER_MODELGROUP_*` / `PLAYER_MWA_*`, `sItemChangeInfo`, `sItemChangeTypes`, `D_80854190`, `D_80854480`, `sUpperBodyLimbCopyMap`. The B button carries the age's sword (Master / Kokiri) as a fixed loadout.

### Results

`cargo test -p oot_game --test targeting` (8) and `--test sword` (4):

| Check | Expected (source) | Port |
|---|---|---|
| Z with nothing to target | `PLAYER_STATE1_PARALLEL`, `targetYaw` = facing, `ParallelIdle`; stick left strafes, stick back walks back, facing held; releasing Z ends it | yes |
| Z at the dummy | locks (`focusActor`, `PLAYER_STATE1_HOSTILE_LOCK_ON`), `TargetIdle` with group 7; stays locked with Z released (Switch); Z again unlocks | yes |
| Facing an off-axis target | turns to the yaw of its focus once the reticle locks | within 0x200 |
| Stick sideways / back while locked | `Sidestep`, facing the target; `func_8083FC68`'s step back (> 6.8) and forward run (> 6) are out of reach of the stick's 6.0, so all lock-on movement sidesteps | yes |
| Left hop | `link_fighter_Lside_jump`, vy 3.5, speed 8.5, yaw + 0x4000, lands with `..._endL` | exact |
| Backflip | `link_fighter_backturn_jump`, vy 5.8, yaw + 0x8000, lands with `..._endR`, `PLAYER_STATE2_19` cleared | exact |
| Leash | lost when √(350·525) ≈ 428.7 is crossed (the leash scales the squared distance), not before `zTargetActiveTimer` < 6 | yes |
| B from standing | `fighter2free` backwards at −2.4 on `skelAnime2`; the swap on its frame 9; the slash (`Lside_kiru`, stick neutral, not targeting) the same frame; model group SWORD, animation type 1; the end animation into standing | exact |
| B ×3 | slash, slash, `RIGHT_COMBO_1H` | yes |
| Locked on, stick forward + B | `pierce_kiru` (stab) with the speed-15 lunge | yes |
| A with the sword | queues the change; `fighter2free` forwards at 1.2 next frame; back to DEFAULT / type 0 | yes |

Visual: `s04_sheet_target.png` (lock-on, circling, a side hop, a backflip, release), `s04_target_locked.png` (the reticle), `s04_sheet_sword.png` and `s04_sheet_sword_child.png` (draw and slash, slash, combo finisher, stance, putting it away). `oot_play --target 200` places a dummy in front of any spawn (also in Kokiri Forest). The keyboard's Q is Z and E is B.

### Known gaps (milestone 5)

- **The camera doesn't switch for targeting.** `Camera_KeepOn1` (lock-on) and `Camera_Parallel1` (Z) aren't ported, so the Normal camera keeps following and the target can leave the screen during hops. This is the most visible gap in this milestone.
- **No hits.** Weapon colliders, `func_80842DF4` (hits, recoil off walls), damage and the actor collision check aren't ported; `meleeWeaponState` marks the active window only.
- Not ported: the jump slash (A with the sword while targeting, and `func_8083BBA0` in the air), spin attacks (the quick spin is detected; its effects aren't), the shield (R: `func_80834758`, `func_80834B5C`), other items and C buttons, and talking to friendly targets (they target as `PLAYER_STATE1_FRIENDLY_ACTOR_FOCUS` but nothing reacts).
- The dummy has no Navi, no BGM-enemy tracking and no `targetPriority` users.

## Milestone 6: water and swimming

**Answer:** ported. Water boxes set `yDistToWater` and the bgcheck water flags. Link starts swimming once the water is deeper than the age's `unk_2C`, treads water, swims (half speed), strafes while holding Z, dives on A (pitched head-down), rises and surfaces, and wades out once the water is shallower than `unk_24`. He can also climb out onto a ledge from the water. Kokiri Forest's stream is deep enough for child Link to swim, but adult Link wades through it, as in the game.

### What was ported, and from where

| Rust | Decomp | Notes |
|---|---|---|
| `StaticCollision::water_surface` | `BgCheck_GetWaterSurface` (`z_bgcheck.c`) | first box whose room (`(properties >> 13) & 0x3F`) matches `curRoom`, or is 0x3F (all rooms); skips bit-19 boxes; strict x/z extent |
| `Actor::update_bg_check_info` water part, `Actor::room` | `Actor_UpdateBgCheckInfo` (`UPDBGCHECKINFO_FLAG_2`), `BgCheck_GetWaterSurfaceAllHack` | `yDistToWater = surface - pos.y`; `BGCHECKFLAG_WATER` while ≥ 0, `WATER_TOUCH` on the first frame (ripples aren't spawned) |
| `Player::func_8083D53C` | same | runs after the move and bgcheck in `Player_UpdateCommon`; enters water past `unk_2C` (`func_8083D36C`), leaves below `unk_24` (`Player_SetupTurnInPlace` + `func_8083D0A8`); not during `Player_Action_80845668` / `Player_Action_8084BDFC` (the ledge climbs) |
| `func_8083D36C`, `func_8083D0A8`, `func_80832340` | same | `PLAYER_STATE1_27` / `PLAYER_STATE2_10`; if `STATE2_10` was already set (falling back in), `func_8083D12C` is forced and `av1.actionVar1 = 1` |
| speed scale 0.5 | `sWaterSpeedFactor` in `Player_UpdateCommon` | applies to every animation and velocity that uses it (spike 03's `speed_scale`) |
| `func_8084B000` | same | buoyancy: towards −5 (sinking) above `unk_28`, +2 (rising) below it; sets `STATE2_10` below 100; gravity 0 |
| `func_8084AEEC`, `func_8084B158`, `func_8084DBC4` | same | stroke acceleration only on stroke frames 10–20; speed cap `R_RUN_SPEED_LIMIT`/100 × 0.8; yaw at 1600; stroke playback speed from the speed (×2 on A/B) |
| `Action::Swim` / `SwimMove` / `SwimTarget` | `Player_Action_8084D610`, `Player_Action_8084D84C` (+ `func_8084D530`), `Player_Action_8084DAB4` (+ `func_8084D980`, `func_8083FD78`) | interrupt list `sActionHandlerList11` = {0, 12, 5, −4}; forward / back / `Rside` / `Lside` strokes while targeting |
| `Action::Dive` | `Player_Action_8084DC48`, `func_8083D12C`, `func_8083D330` | three phases: the start animation (vy 0, then −2 at frame 20); swim down while A is held and `yDistToWater` < 120 (`D_80854784[0]`, no scale upgrade); then float and pitch back (`unk_6C2` → −10000) and rise at min(depth·0.018 + 4, 8) |
| `Action::Surface` | `Player_Action_8084E1EC` | `link_swimer_swim_deep_end`, then treading water |
| `LookRotations::root_pitch` in `gfx::pose_player` | `Player_OverrideLimbDrawGameplayCommon` | the root limb gets T(pos.x, (cos `unk_6C2` − 1)·200 + pos.y, pos.z) · RotX(`unk_6C2`); `unk_6C2` steps back to 0 by 400 per frame in `Player_UpdateCommon` |

Read at runtime: `sAgeProperties` (`unk_24` / `unk_28` / `unk_2C` / `unk_30`: adult 36 / 44.8 / 56 / 68, child 22 / 29.6 / 32 / 48), `R_RUN_SPEED_LIMIT`, and the water boxes from the scene's collision header. Climbing out onto a ledge from the water (`link_swimer_swim_15step_up`) came for free from milestone 4's `Player_ActionHandler_12`, which is in the swim interrupt list.

**Test course:** a pool was added at x∈[−950, −500], z∈[650, 950], with the water surface at y −20, a deep end at −150, and a 26.6° ramp out along +x. `CollisionBuilder::water_box` writes the box, and the course round-trips through the binary format as before. The oot_play course view draws water boxes as translucent planes.

### Results

`cargo test -p oot_game --test water` (10):

| Check | Expected (source) | Port |
|---|---|---|
| Water box query | room match or 0x3F, bit 19 skipped, strict edges | yes |
| Age thresholds | `sAgeProperties` values above | exact |
| Wading in down the ramp | swimming on the first frame with `yDistToWater` > 56 (the check follows the move); running before that; `sWaterSpeedFactor` = 0.5 | exact frame |
| Treading water | settles 44.8 below the surface (`unk_28`); vy within [−5, 2]; falling in plays `swim_deep_end` | mean 44.8 ± 1.5 |
| Strokes | top speed 6·0.8 = 4.8; each decelerating frame is exactly v − (0.02·v + 0.05) | exact |
| Dive | vy 0 until the start animation's frame 20, then −2; `unk_6C2` = 16000; bottoms out between 110 and 130 below the surface (stops at 120, plus the last stroke); surfaces rising with `yDistToWater` < 68; back to treading water | yes |
| Wading out | `PLAYER_STATE1_27` cleared on the first frame with `yDistToWater` < 36; speed scale back to 1 | exact frame |
| Child | swims past 32; floats at 29.6 | yes |
| Z in water | parallel mode, `SwimTarget`, side stroke, facing held | yes |
| Kokiri Forest | one box, surface −12, room 0, over x 73..1473, z −588..452; the deepest bed is −60 (48 deep): adult stands with the water flag set and `yDistToWater` 48; child swims 29.6 below the surface | yes |

Visual: `s04_sheet_swim.png` (the course: run down the ramp, surface, tread water, dive head-down, rise, surface, swim back and run out) and `s04_sheet_swim_kokiri.png` (child Link in the Kokiri stream: `oot_play --scene spot04 --child --at 1230,-12,-130,0 --script tread`). `--at x,y,z,yaw` places Link anywhere, including in a scene, and `--step` sets the contact sheet's frame stride.

### Findings

- Kokiri Forest's water is 48 deep at most, which is between the child's swim threshold (32) and the adult's (56). The stream is a swimming area only for the age the scene is built for.
- Walking into water from dry land still plays the surfacing animation. `func_8083D36C` always sets `PLAYER_STATE2_10`, and `func_8083D12C`'s surfacing branch fires on the first frame the buoyancy pushes up within `unk_30` of the surface. This is the C's behaviour, not a port artefact.
- Once treading water, Link bobs by about ±2 units indefinitely: the buoyancy has no damping term, only the two asymmetric steps.

### Known gaps (milestone 6)

- **No underwater camera.** `CAM_MODE` changes for diving (`Camera_Normal3` / the dive setting) aren't ported, so the Normal camera follows Link under the surface.
- Not ported:
  - Iron boots (`Player_Action_8084E30C`, `Player_Action_8084E368`) and the Zora scale depths `D_80854784[1..2]`.
  - Picking items up from the bottom (`Player_ActionHandler_2`).
  - Entering a scene in water (`Player_SetStartingMovement` → `Player_Action_8084D7C4`).
  - Water currents / `WaterBox` light settings and camera settings.
  - Splashes (`func_8083CFA8`), ripples and bubbles (`func_8083D6EC`), and the swim sounds.
- `Actor::room` stays 0; room transitions aren't modelled, so water boxes bound to other rooms won't register.

## Milestone 7: DynaPoly collision with one moving platform

**Answer:** ported. Actor-owned collision meshes are rebuilt every frame from their actor's transform and tested by every entity check (floor, walls, ceiling, line tests) after the static mesh, as in `BgCheck_*Impl`. An actor standing on one is carried by the platform's motion. The platform is `Bg_Ydan_Hasi`'s floating block (params `HASI_WATER_BLOCK`), the Deku Tree B1 block that slides on the water. Its collision (`gDTSlidingPlatformCol`) and model (`gDTSlidingPlatformDL`) come from `object_ydan_objects` in the ROM, and it floats in a water channel added to the test course.

**Why this platform:** it's the nearest moving platform to Kokiri Forest (the Deku Tree). It slides ±165 horizontally and bobs ±2 vertically, and its top face, built from truncated vertices, moves in whole units. Its update function is self-contained, and it floats on water, which uses milestone 6.

### What was ported, and from where

| Rust | Decomp | Notes |
|---|---|---|
| `dyna::Dyna::set_bg_actor` | `DynaPoly_SetBgActor`, `BgActor_SetActor` | previous transform's `rot.x` offset by one so the first update expands; invalidates the lookup |
| `Dyna::update_context`, `add_to_lookup` | `DynaPoly_UpdateContext`, `DynaPoly_AddBgActorToLookup` (`DynaPolyInfo_expandSRT`) | transform by `SkinMatrix_SetTranslateRotateYXZScale` (T·RotYXZ·S, `SkinMatrix_SetRotateYXZ`'s entries), truncate to s16 (`BgCheck_Vec3fToVec3s`), min/max y from the float vertices (with the `else if` quirk), bounding sphere = mean vertex with radius 1.1 × the farthest s16 vertex (both truncated to `Sphere16`), normals from `Math3D_SurfaceNorm`, floor/wall/ceiling lists by `ny` with head insertion; an unchanged transform only rebuilds the lists (and then honours the ceiling-disabled flag, which the full path doesn't) |
| `Dyna::update_prev_transforms` | `DynaPoly_UpdateBgActorTransforms` | end of `Actor_UpdateAll` |
| `StaticCollision::raycast_down` (dyna part) | `BgCheck_RaycastDownDyna`, `BgCheck_RaycastDownDynaList` | skipped below `minY` or outside the sphere in x/z; `CheckYIntersectApprox1` (det 300); the walls pass only if nothing was found yet (`WALLS_SIMPLE`) |
| `sph_vs_dyna_wall`, `check_wall` | `BgCheck_SphVsDynaWall`, `BgCheck_SphVsDynaWallInBgActor`, `BgCheck_CheckWallImpl` | the sphere grown by the radius (as s16) against x/z and xy/yz; both passes from one start; after a dyna push, a one-face static wall line test from `posPrev` stops it pushing Link through scenery |
| `check_ceiling` (dyna part) | `BgCheck_CheckDynaCeiling`, `BgCheck_CheckDynaCeilingList` | from the static result; `testPos.y = sign(ny)·checkHeight + ceilingY` |
| `check_line` (dyna part) | `BgCheck_CheckLineAgainstDyna`, `..AgainstBgActor`, `..BgActorSSList`, `Math3D_LineVsSph` | only with `CHECK_DYNA`: walls, floors, then ceilings of each bg actor whose y range and sphere the segment touches |
| `PolyId { bg, idx }`, `StaticCollision::surface` | the (`CollisionPoly*`, `bgId`) pairs, `SurfaceType_GetData` | surface types from the owning mesh's header; `BgCheck_ComputeWallDisplacement` keeps its bug of reading the previous wall's flag 27 from the scene's table |
| `Actor::floor_bg_id`, carry at the start of `update_bg_check_info`, `Dyna::carry` | `Actor_UpdateBgCheckInfo`, `DynaPolyActor_TransformCarriedActor`, `DynaPolyActor_UpdateCarriedActorPos`, `DynaPolyActor_UpdateCarriedActorRotY` | while grounded on a bg actor with `DYNA_TRANSFORM_POS`: pos = cur · prev⁻¹ · pos; with bit 1 also the yaw (and Player's `currentYaw`); the crushed check (`BGCHECKFLAG_CRUSHED` when floor and ceiling come from different meshes more than 15 apart) now has a second mesh to compare with |
| `bg_ydan_hasi::BgYdanHasi` | `BgYdanHasi_Init`, `BgYdanHasi_UpdateFloatingBlock` | scale 0.1 (`ICHAIN_VEC3F_DIV1000(scale, 100)`) then x/z 0.15; y = water + 20 + 2·sin(timer·π/25); x/z = home + facing · sin((frames & 0xFF)·π/128)·165 |
| `World::tick_with` order | `Actor_UpdateAll` | BG category first (platform update, `DynaPoly_UpdateContext`), then Player, …, then `DynaPoly_UpdateBgActorTransforms` |
| `gfx::platform_draw_list` | `BgYdanHasi_Draw` (`Gfx_DrawDListOpa`) | segment 6 = `object_ydan_objects`, posed by the bg actor's transform (the same matrix `Actor_Draw` builds) |

Read from the ROM: `gDTSlidingPlatformCol` (0x7798) and `gDTSlidingPlatformDL` (0x7508), both located through `object_ydan_objects.xml`.

**Decisions:**
- `StaticCollision` keeps its name but now holds `dyna` as well, so it plays the role of the game's `CollisionContext`. Every existing caller gets dyna collision without changes.
- `SkinMatrix_Invert` (Gaussian elimination) is replaced by glam's inverse. The carried position differs only at float rounding: the offset from the platform drifts by less than 0.01 over 200 frames.
- The game reads the platform's water height from the scene's `waterBoxes[1]`. The course passes the channel's surface instead.
- The course gained a water channel at x∈[350, 950], z∈[−300, −100] (floor −150, water −20) for the platform to slide in. oot_play spawns the platform whenever the course is loaded.

### Results

`cargo test -p oot_game --test platform` (8):

| Check | Expected (source) | Port |
|---|---|---|
| The ROM collision | 8 vertices ±500 × −400..0, 12 polys, all `COLPOLY_IGNORE_CAMERA` | exact |
| Expanded vertex | (−500, 0, −500) → (575, 0, −125) at yaw 0x4000, scale 0.15 / 0.1 | exact |
| Bounding sphere, y range, lists | centre (650, −20, −200), radius 118 (118.73 truncated), y −40..0; floors [1, 0], ceilings [11, 10], walls [9..2] | exact |
| Motion | x and y from `BgYdanHasi_UpdateFloatingBlock` for 300 frames | < 1e-3 |
| Carrying | offset from the platform held (drift < 0.01 over 200 frames); Link's y = the platform y truncated (the s16 top face); `floorBgId` 0 | yes |
| Leaving it | running off the far end jumps the gap and lands on the bank, `floorBgId` back to `BGCHECK_SCENE` (50) | yes |
| Raycast | the dyna top (0) over the channel bed (−150); the static floor elsewhere and below `minY` | exact |
| Wall / ceiling / line | pushed to 575 − 18 by the side wall; ceiling y −70 (−40 − 30); an entity line stops on the top, a camera line (`IGNORE_CAMERA`) and a line without `CHECK_DYNA` reach the bed | exact |
| Surface types | from the platform's header: the top is wall type 0, the sides wall type 1 (`WALL_FLAG_0`) | yes |

The existing 100+ tests pass unchanged after the refactor. `PolyId` became `{ bg, idx }`, and each check gained its dyna part.

Visual: `s04_sheet_platform.png`: the textured block sliding and bobbing in the channel with Link riding it, then running off the far end and jumping to the bank (`oot_play --script platform --step 4`).

### Findings

- Link's height on the platform moves in whole units (−1, 0, 1) while the platform bobs smoothly. This is because the dyna vertex list is `Vec3s`, so the top face is at the truncated platform y. The game does the same.
- Every poly of this platform is flagged `COLPOLY_IGNORE_CAMERA`, so the camera's line tests go through it.

### Known gaps (milestone 7)

- **One platform type.** Other `DynaPolyActor`s (elevators, push blocks, rotating platforms) need their own actor ports. The dyna layer itself handles any number of bg actors, and rotation via `DPM_ROTATE`, which the tests don't exercise.
- **Player's platform-specific code isn't ported:**
  - Grabbing, pushing and pulling blocks: `Player_ActionHandler_5` (interrupt 5), `func_8083F9D0`, `func_8084B840`.
  - The roll bonk's push on a dyna actor (`Player_Action_Roll`).
  - Climbing a dyna wall (`Player_Action_8084BF1C`).
  - `DynaPoly_SetPlayerOnTop` / `SetPlayerAbove` interaction flags (nothing reads them yet).
  - The `BGACTOR_1` delete path and `DynaPoly_DeleteBgActor`.
- **Camera:** the camera ignores this platform (by flag). Camera bgcheck against dyna meshes without the flag goes through the same `check_line`, but that path is untested.
- **Scenes:** actors aren't spawned from scene actor lists, so there are no platforms in Kokiri Forest. Spawning `Bg_*` actors from the room's actor list is the natural next step.

## Milestone 8: sound (optional): left out

Not started, by choice. The other seven milestones used the time. Sound is also a different kind of work from everything else in this spike:
- Even a minimal "play Link's footsteps and sword swings" needs the audio engine itself: the sequence player (`seqplayer.c`), soundfonts and sample banks from `Audiobank` / `Audiotable`, VADPCM decoding, the synthesis loop (`synthesis.c`), and the sfx channel allocator behind `Audio_PlaySfxGeneral`.
- None of that shares code with what's here.
- The hooks are ready. The ported code already names the calls where sounds happen, e.g. `Player_PlaySfx(&this->actor, NA_SE_...)`, `Player_PlayVoiceSfx` (voice), and the stroke sounds in `func_8084D530`. The surface sfx types (`SurfaceType_GetSfxOffset`) are also in `bgcheck`. A future audio spike can attach to these without touching Player again.

## Recommended next spikes

1. **Actors from the scene.** Spawn the room and scene actor lists (`Actor_Spawn` from `SCENE_CMD_ACTOR_LIST`) with a small set of ported actors:
   - `Bg_Treemouth` (fills the Deku Tree's mouth; see milestone 1).
   - Kokiri Forest's bushes, rocks and signs.
   - `Bg_Ydan_Hasi` in the Deku Tree, using milestone 7's dyna layer.
   This is what makes scenes feel inhabited, and it forces an actor lifecycle (init/update/draw, object dependencies, categories).
2. **The rest of the camera.** Port `Camera_KeepOn1` / `Camera_Parallel1` (targeting), the hang and climb modes, the dive camera, and `bgCamIndex` settings from scene collision (doorway spawns, milestone 2). These are the most visible gaps in milestones 2, 4, 5 and 6.
3. **Collision checks between actors (`z_collision_check.c`).** Colliders, `CollisionCheck_AT/AC/OC`, damage tables, and sword hits on the dummy (milestone 5's missing half). Recoil, knockback and Link taking damage follow from it.
4. **Items and C buttons.** Bottles, the slingshot/bow (first-person aim), bombs and the hookshot. Each is a Player action group like the sword, and the item-change machinery from milestone 5 is in place.
5. **Scene transitions.** Exits, doors, room loads (`Room_Change`, which would make `Actor::room` real), and the title card.
6. **Audio** (milestone 8's scope) as its own spike.

## Commands

```sh
# Tests (skip without oot.toml)
cargo test -p oot_game                       # movement, camera, footik, ledge, targeting, sword, water, platform, scene
cargo test --workspace

# Scenes
cargo run -p ootx -- scan-scenes [--filter spot] [--all-layers]     # out/scene_scan.json
cargo run -p ootx -- dump-room --scene spot04 --room 0 --png out/s04_tex

# Headless play (contact sheets / traces / screenshots in out/)
cargo run -p oot_play --release -- --scene spot04 --script forward --sheet out/s04_sheet_spot04_walk.png
cargo run -p oot_play --release -- --script swim --step 8 --sheet out/s04_sheet_swim.png
cargo run -p oot_play --release -- --scene spot04 --child --at 1230,-12,-130,0 --script tread --step 10 --sheet out/s04_sheet_swim_kokiri.png
cargo run -p oot_play --release -- --script platform --step 4 --sheet out/s04_sheet_platform.png
cargo run -p oot_play --release -- --script target --sheet out/s04_sheet_target.png
cargo run -p oot_play --release -- --script sword [--child] --sheet out/s04_sheet_sword.png

# Interactive
cargo run -p oot_play --release -- [--scene spot04] [--time 18:30] [--child] [--target 200]
#   WASD/arrows stick, Shift walk, Space A, E B, Q Z, J/L C-left/right, F3 follow/game camera, F4 foot IK,
#   F1 collision, Tab age, Backspace respawn
```

Every other flag is described in `oot_play --help`: `--spawn`, `--collision`, `--view`, `--follow-camera`, `--no-foot-ik`, `--frames`, `--at`, `--step`, `--trace`, `--screenshot`, `--wire`.
