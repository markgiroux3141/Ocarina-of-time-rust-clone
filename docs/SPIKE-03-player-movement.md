# Spike 03: Link moving in an environment, with N64 controller input

**Question:** can Link's movement logic be ported from the decomp faithfully enough that walking, running and rolling under live control behave like the game?

**Answer:** yes. Player's movement path from `z_player.c` runs in Rust: standing, turning, the walk/run blend, rolling, auto-jumping off ledges, falling and landing. So do the actor physics from `z_actor.c`, the static half of `z_bgcheck.c`, and Link's SkelAnime and the AnimTaskQueue queue. Speeds, accelerations, jump heights and roll length match values derived from the decomp constants, to the frame. It plays with the user's N64 USB pad on the synthetic course and on real scene collision loaded from the ROM.

The trickiest parts weren't the state machine. They were engine details that change every number: the 1.5× update scale, the order of the frame, how Link's animation frames reach the joint table, and libultra's slightly off sine table.

## Controller

### Which device

| | |
|---|---|
| Device | Retro-Bit N64 USB pad, VID `2563` PID `0575`, serial `AZ-RB-N64P-215`, HID product "SWITCH CO.,LTD. Controller (Dinput)" |
| HID report (read with hidapi) | 13 buttons, 1 hat, 4 byte axes (X, Y, Z, Rz; idle `80 83 80 80`), then pressure and accelerometer bytes |
| N64 button → raw HID index | C-Left 0, B 1, A 2, C-Down 3, L 4, R 5, Z 6, C-Right 8, C-Up 9, Start 12; stick on axes 0/1; D-pad on the hat |

The raw indices come from the user's Hide and Seek Level Builder repo (`native/crates/game/src/gamepad.rs`), where they were confirmed by pressing each button on this pad. On 2026-09-27 the stick was also confirmed live in `oot_play`: the user's input read (+80, +80) at full diagonal. The HUD now lists the raw codes held, so each button can be checked in the play window.

### gilrs vs SDL3

Both libraries were built and pointed at the pad on this machine with a probe (scratchpad, not committed).

| | gilrs 0.11 (WGI backend) | sdl3 0.20 (SDL 3.4.16, static) |
|---|---|---|
| Sees the pad | yes, as "PS3 Controller", `SdlMappings` | yes, as "Retro Controller". Its HIDAPI PS3 driver matches `2563:0575` by id (`controller_list.h`: "Retro-bit, Retro Fighters Controllers") |
| Semantic layout | generic ShanWan PS3 entry: raw 2 = South, 1 = East, 3 = West, 0 = North. C-Up (raw 9) lands on Start | PS3 driver: raw 0 = West, 1 = South, 2 = East, 3 = North, 6/7 = triggers (as axes), 12 = Guide. C-Up lands on Start and Start on Guide |
| Raw access | native codes = HID button index, 1:1 with the report | joystick indices are the driver's PS3 order, one remap away from the report |
| Build | pure Rust | CMake + MSVC. Here it needed `CMAKE_GENERATOR="Visual Studio 17 2022"` (CMake 4.1 doesn't know VS 18) and a short target path (MSBuild's 260-char limit) |
| Focus | input only while the app's window is focused (WGI). A console probe sees the device but no events | background events via a hint |

**Choice: gilrs.** Neither library knows N64 semantics for this pad, so an N64 table keyed by VID/PID is needed either way. gilrs exposes the HID order directly, needs no C toolchain, and already works with this pad in the user's other project. SDL3 stays the fallback if an adapter shows up that WGI can't see.

### Mapping to the game's input

- `oot_pad::Profile::retro_bit_n64` binds N64 buttons to raw codes and the stick to the mapped left-stick axes, with gilrs' dead-zone filters off. `Profile::generic` covers XInput/SDL pads: A = South, B = West, Z = LT, R = RT/RB, L = LB, C = right stick. Keyboard is the fallback: WASD/arrows, Shift to walk, Space = A, J/L = C-left/right.
- Stick scaling: full tilt = ±80 N64 units (an original stick reads about ±80). The game then applies its own processing, ported unchanged:
  - `PadUtils_UpdateRelXY`: dead zone ±7, clamp at 60
  - `Lib_GetControlStickData`: magnitude capped at 60, angle `Math_Atan2S(y, -x)`
  - camera-relative yaw: `Camera_GetInputDirYaw` + stick angle
- `PadMgr` semantics: the pad is polled every display frame. Presses and releases accumulate and are handed to the game once per game frame, then cleared (`PadMgr_UpdateInputs` / `PadMgr_RequestPadData`), so a tap shorter than 50 ms still registers.
- Overrides go in an optional `[pad]` table in `oot.toml` (`stick_range`, `[pad.buttons] A = "b2"`). `ootpad calibrate` prints that table after asking for each button.

## What was ported, and from where

| Rust | Decomp | Notes |
|---|---|---|
| `player::Player::update` | `Player_UpdateCommon` (non-cutscene path) | frame order: prevPos, velocity, position, bg check, ledge check, gravity, flag clears, look decay, stick, action, move-actor request, facing, homePos |
| `Player_ProcessSceneCollision` | same | Player's bg check: wall-interact line probe, `unk_880` run limit near walls, wall-top probe and climb class, floor slopes |
| `func_8083AA10`, `func_8083A4A8`, `func_80838940` | same | leaving the ground: auto-jump (`IREG(66..69)`) or fall |
| `Player_ProcessControlStick`, `Player_CalcSpeedAndYawFromControlStick`, `Player_GetMovementSpeedAndYaw` | same | stick history, target speed curves, camera-relative yaw |
| actions | `Player_Action_Idle` standing, `Player_Action_80842180` run, `Player_Action_TurnInPlace` turn, `Player_Action_Roll` roll (with bonk), `Player_Action_8084411C` midair/landing | entered through `Player_SetupAction` (setup action) and the helpers around it |
| walk/run blend | `func_80841EE4`, `func_80841CC4`, `func_8084029C`, `func_8083BF50` | 29-frame phase `unk_868`; walk↔run weight from `REG(35..38, 48)`; slope climb blend; stop animation chosen by phase |
| idle | `Player_ChooseNextIdleAnim` | alternates wait and random fidgets from `sFidgetAnimations` (`Rand_ZeroOne` LCG) |
| look/lean | `func_8083DC54`, `func_8083DDC8`, `func_80836AB8`, `Player_ScaledStepBinangClamped`, `func_80847298`, `Player_UpdateShapeYaw` | head looks at the floor ahead; torso leans into fast turns |
| interrupts | `Player_TryActionHandlerList` with `sActionHandlerListIdle/24/14`; `Player_ActionHandler_Roll`, `Player_TryRoll`, `Player_SetupRoll` | only index 6 (roll on A with the stick forward) is in scope |
| landing | `func_80843E64` | fall damage at ≥ 400 / 800 (stagger animation), landing roll for 80 < fall < 800 with the stick forward |
| `actor.rs` | `Actor_UpdateVelocityXZGravity`, `Actor_UpdatePos`, `Actor_UpdateBgCheckInfo`, `func_8002E2AC`, `func_8002E234` | velocity, gravity cap, position × 1.5, wall/ceiling/floor, GROUND/TOUCH/LEAVE flags |
| `bgcheck.rs` | `StaticLookup_AddPolyToSSList`, `BgCheck_RaycastDownImpl/StaticList`, `BgCheck_CheckWallImpl`, `BgCheck_SphVsStaticWall`, `BgCheck_ComputeWallDisplacement`, `BgCheck_CheckStaticCeiling`, `BgCheck_CheckLineImpl/AgainstSSList`, `CollisionPoly_LineVsPoly`, `SurfaceType_*` | plus the `Math3D_TriChkPointPara{X,Y,Z}Impl`, `Math3D_CirSquareVsTriSquare`, `Math3D_PointDistSqToLine2D` and plane helpers from `sys_math3d.c` |
| `skelanime.rs` | `LinkAnimation_Update/Loop/Once/Morph/Change/*SetSpeed/LoadTo*/BlendTo*/InterpJointMorph/OnFrame`, `SkelAnime_InterpFrameTable`, `SkelAnime_UpdateTranslation`, `AnimationContext_*` | morph frames, the request queue, root motion |
| `math.rs` | libultra `sins`/`coss`, `Math_Atan2S`, `Math_StepToF`, `Math_AsymStepToF`, `Math_ScaledStepToS`, `Math_SmoothStepToS` | |
| `input.rs` | `padmgr.c`, `pad.c`, `Lib_GetControlStickData` | |
| draw-time | `Player_OverrideLimbDrawGameplayCommon` | child root scaling, head/upper-body look rotations |

### Read from the decomp at runtime (nothing copied into the repo)

| Table | Source | Used for |
|---|---|---|
| `sBootData` + the `XREG(n) = bootRegs[k]` / `= literal` lines of `Player_SetBootData` | `z_player_lib.c` | every REG value Player reads (`REG(19)` accel 200, `REG(27)` turn 2000, `REG(43)` brake 800, `REG(45)` run limit 600, `REG(48)` 370, `REG(68)` gravity −100, `IREG(66..69)` 590/750/125/200, `MREG(95)`, ...). The mapping is parsed from the function body |
| `sAgeProperties` (17 floats per age, expressions like `70.0f * (11.0f / 17.0f)` evaluated) | `z_player.c` | wall radius 18, ceiling height 56, ledge-grab drop 70, climb classes 41/59/79.4, root scale 11/17 for child |
| `D_80853914` (`GET_PLAYER_ANIM`, 46 groups × 6 types) and `sFidgetAnimations` | `z_player.c` | every animation choice |
| `sintable`, `sAtan2Tbl` | `libultra/gu/sintable.inc.c`, `sys_math_atan.c` | angle maths |
| `PLAYER_LIMB_*`, `PLAYER_ANIMGROUP_*`, `PLAYER_ANIMTYPE_*` | `player.h` | limb and table indices |

Link's 573 animations come from the ROM as in spike 02.

## Engine details that matter

- **`R_UPDATE_RATE` = 3 in gameplay** (60 Hz / 3 = 20 Hz logic). `R_UPDATE_RATE * 0.5` = 1.5 multiplies position integration (`Actor_UpdatePos`), animation playback and `Math_ScaledStepToS`. So a speed of 6.0 moves 9 units per frame, and a Link animation at speed 1.0 advances 1.5 source frames per game frame. The spike 02 viewer plays one frame per tick, so its playback is a third slower than in game.
- **Animation data arrives by DMA.** `AnimTaskQueue_AddLoadPlayerFrame` starts a DMA straight into the frame table, and the dmamgr thread outranks the game thread. Copy, interpolate and move-actor requests are queued and run after every actor has updated. The port applies loads immediately and queues the rest in order, with `AnimTaskQueue_DisableTransformTasksForGroup` suppressing copy/interp as in the game. The immediate loads matter: after a hard landing, `Player_Action_Idle` writes a bob into `jointTable[0].y` right after the frame load. The bob is visible in game, so the load must already have landed.
- **Frame order.** Player reads the camera's input yaw from the previous frame, because cameras update after actors. `prevPos` is last frame's `homePos`, which is before root motion.
- **Rendering at 60+ fps.** The world runs fixed 20 Hz ticks. The renderer interpolates between the last two snapshots: position, facing, every joint angle (the short way round), the look rotations and the camera.
- **World units.** Link's model is drawn at actor scale 0.01 (`Actor_Draw`: translate, rotate yaw, scale).

## Collision

- `oot_core::collision` decodes and encodes the game's `CollisionHeader`: 0x2C header, `Vec3s` vertices, 16-byte `CollisionPoly` (type, 3 flagged vertex indices, s16 normal, s16 dist), `SurfaceType` pairs, `WaterBox`es. `CollisionBuilder` makes polys the way the tools do: normal = normalized edge cross product × 0x7FFF, dist = −n·v0.
- The synthetic course (`oot_game::course`) is built with it, encoded to bytes and decoded back before use. It has 100-unit floor tiles, a 20.6° ramp to a 150-high plateau, a 30° ramp, eight 15-unit stairs, a 40-high block, a 30° diagonal slab, a 450-deep pit and boundary walls.
- **Single static-lookup cell.** The game grids each scene into subdivisions whose lists are sorted by lowest vertex. The port builds one cell with the same insertion routine, so polys come in the same relative order. Subdivisions overlap by `BGCHECK_SUBDIV_OVERLAP` = 50, which is more than any entity radius (Player uses 18), so every query sees the same candidate polys.
- **Real scenes (stretch goal): done.** `oot_core::scene` reads the scene header commands (`SCENE_CMD_ID_COLLISION_HEADER`, spawn list) and `oot_play --scene <name>` puts Link at spawn 0 on that collision. It's drawn solid by surface class, and `--wire` / F1 adds the wireframe.

| Scene | Vertices | Polys | Surface types | Spawns |
|---|---|---|---|---|
| `spot04` Kokiri Forest | 1081 | 1692 | 46 | 12 |
| `spot00` Hyrule Field | 1162 | 1579 | 61 | 18 |
| `ydan` Deku Tree | 1399 | 2321 | 31 | 2 |

Link walks, rolls and slides along walls in all three.

## Results

`cargo test -p oot_game` runs 26 tests: 8 unit tests, plus 18 that replay scripted inputs against the ROM and decomp. Expected values are derived from decomp constants, not from the port.

| Check | Expected (source) | Port |
|---|---|---|
| REG/IREG values, age properties | `sBootData[0]`, `sAgeProperties` | equal |
| Tables | atan = round(atan(i/1024)·0x8000/π); sin = **trunc(sin(i·π/2046)·32767)** | 0 / 1025 and 0 / 1024 entries differ |
| Full stick from rest | speed 0, 2, 4, 6, 6… (`REG(19)`/100, capped by `R_RUN_SPEED_LIMIT`/100 = 6.0; the stick curve alone gives 6.58) | exact |
| Distance, 40 frames full stick | (2 + 4 + 36·6)·1.5 = 333 | 333.0 |
| Half stick (raw 30 → rel 23) | start curve ((1 − cos(3·450))²·30 + 7)·0.14 = 0.98 | 0.98 |
| Release stick | brake 1.5/frame: 4.5, 3, 1.5, 0, then a walk-end animation picked by phase | exact |
| Roll | starts on the A frame; lasts ⌈20 / (1.25·1.5)⌉ = 11 frames; top speed 1.5 × 6 = 9 | 11 frames, 138 units, 9.0 |
| Roll needs stick forward | `controlStickDirections` = 0 (stick ≥ 55, pointing forward) | idle + A and sideways + A don't roll |
| Turn in place (rel magnitude 19) | no speed below magnitude 20; 0x4000 at 1200·1.5 per frame = 10 frames | 10 |
| Run off the plateau | vy = `IREG(67)`/100 = 7.5, `link_normal_run_jump`; peak from gravity −1 and ×1.5 | exact; lands, stands |
| Walk off slowly | no jump (speed ≤ 3), `link_normal_landing_wait`, falls to the ground | yes |
| Land with stick forward (fall 150) | 80 < fallDistance < 800 → landing roll | Roll |
| Fall into the pit (450) | fall damage −8 (≥ 400), stagger; vy capped at −20 | yes |
| Run into the 40-high block | stops at face + 18 (`wallCheckRadius`); `unk_880` → 0.1 | z = 368.0, speed 0.1 |
| Run into the 30° wall | slides; speed ≈ 6 × 5461 × 0.00008 = 2.62 | 2.64 |
| 20.6° ramp | 6.58 − 8·sin²(slope) = 5.6 | 5.57–5.61 |
| 15-unit stairs | climbs to 120 | 120 |
| Step-up limit at full speed | (measured) | **17 units** |
| The brief's script (40 frames forward, then A, then idle 30) | roll on frame 41, grounded throughout, ends standing | 402.8 units travelled |
| Root motion | `SkelAnime_UpdateTranslation` rotated by yaw, × 0.01, root reset to `baseTransl` | exact |

Visual checks (headless, written to git-ignored `out/`):

- `s03_sheet_run-roll.png`: the stand, the walk-to-run blend and the run cycle. The roll ends in a bonk off the diagonal slab, then Link recovers to standing.
- `s03_sheet_ledge.png`: auto-jump arc off the plateau and landing.
- `s03_sheet_pit.png`: long fall and stagger.
- `s03_sheet_stairs.png`, `s03_sheet_turn.png`.
- `s03_scene_spot04*.png` / `s03_sheet_spot04_child.png`: child Link in Kokiri Forest, with and without the wireframe.
- `s03_window.png`: the interactive window and HUD.

Each sheet has a matching `s03_trace_*.json` with per-frame input, action (with its decomp name), position, speeds, yaw, grounded state, animation and walk phase.

## Findings worth knowing

- **Steps above about 17 units block Link.** While grounded, `func_8002E2AC` sets `velocity.y = −4` (with `UPDBGCHECKINFO_FLAG_3`). The next frame's wall line test, at `checkHeight` 26 above the predicted position, therefore runs about 18.5 above the feet, and a riser that tall stops Link. This is the game's own behaviour, and probably why OoT stairs use ramp collision under the visible steps.
- **libultra's sine table is `trunc(sin(i·π/2046)·32767)`**, so entry 1023 is exactly 90° and `sins` runs 1023/1024 slow. Generating the table with π/2048 differs in 997 of 1024 entries. The port reads the shipped table.
- **The 1.5× update scale is everywhere.** Porting a function without it gives plausible-looking movement that is a third too slow.
- **The stick curve has two regimes.** Starting from rest uses `(1 − cos((m − 20)·450))²·30 + 7` (arg4 = 0.018), so a magnitude under 20 turns in place. Once moving, the linear `m·0.8` is used. Both are then scaled by 0.14 and clamped by `unk_880`.

## Known gaps

- **Not ported (stubbed as "not taken"):** Z-targeting and parallel mode, sword and items (B, jump slash, spin), shield, talking, doors, pushing blocks, climbing and ledge grabs, swimming, ice and hover boots, slippery slopes (`Player_HandleSlopes`), conveyors, void-outs and exits, damage from actors. The ledge-grab and mid-air grab checks run and record a note, then Link falls.
- Dynamic (`DynaPoly`) collision and water boxes. The raycast covers static polys only.
- Foot IK (`func_8008F87C`), so feet don't adapt to slopes. The upper-body item pose (`skelAnime2`) isn't used either.
- The camera is a stand-in follow camera (C-left/C-right orbit, pulled in front of walls), not `z_camera.c`.
- No sound. Footstep and voice triggers are ported as timing only.
- The Retro-Bit button mapping comes from the other repo's hardware check. The stick was confirmed live; the buttons are ready to check with the HUD's raw codes or `ootpad calibrate`.
- Rendering: scene collision is drawn, but room meshes are not.

## Recommended next spikes

1. **Scene and room rendering.** Room mesh headers and display lists through the existing interpreter, with scene lights and fog. Kokiri Forest collision already loads, so Link could walk the real level visually.
2. **`z_camera.c` Normal camera** (`Camera_Normal1`): the other half of how OoT feels. Player already reads `Camera_GetInputDirYaw` from it.
3. **Z-targeting and sword.** The first combat slice: `Player_UpdateZTargeting`, the parallel/target actions, side hops and backflips (the midair code already handles `PLAYER_STATE2_19`), slashes (`func_80837948`).
4. **Ledge grab and climbing** (`func_8083A5C4`, `Player_Action_8084BBE4`, `Player_ActionHandler_12`). The detection already runs.
5. **DynaPoly collision and one simple actor** (a platform or a push block), the first actor/Player interaction.
6. **Retail ROM support** (from spike 01).

## Commands

```sh
cargo build --release
target/release/oot_play                          # course, adult; --child; --scene spot04
target/release/oot_play --script run-roll --sheet out/s03_run_roll.png --trace out/s03_run_roll.json
target/release/oot_play --scene spot04 --child --script run-roll --wire --screenshot out/forest.png
target/release/ootpad list | watch 10 | calibrate
cargo test -p oot_game                           # 26 tests; the ROM tests skip without oot.toml
```

Controls in `oot_play`: N64 pad (stick, A roll, C-left/right camera), or WASD/arrows, Shift walk, Space A, J/L C-left/right. F1 toggles the collision wireframe, F2 the HUD, Tab switches age, Backspace respawns. Set `OOT_PAD_DEBUG=1` and `RUST_LOG=info` to log raw pad events.
