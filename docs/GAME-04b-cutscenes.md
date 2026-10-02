# GAME-04b: cutscenes

**Goal:** finish the cutscenes properly, between Phase 5 (audio, done) and Phase 6 (the Deku
Tree), so later work doesn't have to think about them. The user decided against an "already
seen" skip (2026-10-01): the cutscene system built in GAME-03 (milestones 4 and 5) gets what it
left out, and the camera gets the one-point cutscenes and the modes still on the fallback.

This phase stays on decomp `2f4c25d`'s names. Phase 6's first milestone (the decomp upgrade,
[ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md)) migrates them with the
rest.

| # | Milestone | Status |
|---|---|---|
| 1 | One-point cutscenes (`z_onepointdemo.c`): `OnePointCutscene_Init`, `_SetInfo`, `_Attention`, `_EndCutscene`, the cameras' parent and child chain, `Camera_Finish`'s timer, `Camera_Unique9`, `Camera_Demo9`, `Camera_Demo5`; the crawlspace's exit (9601, 9602), `En_Box`'s fall (4500) and its attention calls; the tables in the pack | done |
| 2 | The camera modes still on the Normal1 fallback (ADR 0013): `Camera_Jump1` (JUMP, FREEFALL), `Camera_Jump2` (CLIMB, CLIMBZ), `Camera_Uniq1` (HANG, HANGZ), and what the cutscenes need | done |
| 3 | Cutscene audio: the scripts' `CS_CMD_PLAYBGM`, `_STOPBGM`, `_FADEBGM`; `z_demo.c`'s own sounds; Player's cutscene-mode voices and sounds | done |
| 4 | The rest of `z_demo.c`'s commands: the misc actions (rain, lightning, ...), the lighting override (`envCtx.lightSettingOverride`), what the opening's and the Deku Tree's scripts need | done (the drawing of rain, bolts and flash: milestone 6) |
| 5 | Title cards: `TitleCard_InitPlaceName` (`CS_MISC` 15) and the scene-entry title cards (`showTitleCard`), the place names' textures in the pack | done |
| 6 | The opening's nightmare: `En_Viewer` (Zelda and Impa on the horse, Ganondorf), `Bg_Spot00_Hanebasi` (the drawbridge), the rain and lightning | done (Ganondorf's cape and the lightning's flash: known gaps) |
| 7 | Cutscene polish: `Camera_Demo1`'s splines, the letterbox, the narration's placement and Link's poses checked against the C by tests; then the opening and the Deku Tree's talk by hand | done (the by-hand look is the user's) |

Navi's sparkles and glow (BACKLOG #9) are effects, and the C-Up prompt (#8) is the HUD: not this
phase unless they fall out of it.

The working rules are the same as for the earlier phases:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C;
- the scripted runs play the cutscenes, and a change in a run's trace is a golden change to
  explain.

Decisions are in [docs/adr/](adr/README.md) (0029 on).

## Milestone 1: one-point cutscenes

**Answer:** done. Climbing out of the crawlspace plays its one-point cutscene (9601; backing out
the way in, 9602): a sub camera takes the view along `Camera_Demo9`'s spline up and out in front
of Link while the main camera waits, then the main camera takes over from where the spline ended
(`Camera_Copy`), with no jump: BACKLOG #3 is fixed. The one-point system is ported as the C has
it, for the Deku Tree's actors to call in Phase 6: the cameras' queue, the attention cutscenes
(`Camera_Demo5` into `Camera_Unique9`), the fixed shots, `Camera_Finish`. `En_Box` starts its
fall's shot (4500) and its attention cutscenes.

The tests: 286 pass, 1 ignored (282 at the phase's start). The goldens: 85 hashes, 61 cases; the
five runs through the crawlspace re-recorded (golden/README.md).

### What was built

1. **The tables in the pack** (pack format 15, `CameraData::onepoint`): every `OnePointCsFull`,
   `CutsceneCameraPoint` and `s16` definition of `z_onepointdemo_data.c` (75 keyframe tables, 10
   point lists, 12 shorts) and `Camera_Demo5`'s eight keyframe tables from `z_camera_data.c`,
   read from the C (`oot_import::tables::load_onepoint_data`).
2. **`z_onepointdemo.c`** (`oot_game::onepoint`, [ADR 0029](adr/0029-one-point-cutscenes.md)):
   `OnePointCutscene_Init` (the queue in front of the parent, the statuses, the lower priority
   cutscenes removed), `_SetInfo` (the cases listed in the ADR), `_SetAsChild`,
   `_RemoveCamera`, `_EndCutscene`, `_Attention` (the category order, the timers by category),
   `_AttentionSetSfx`, `_CheckForCategory`; `func_8005B198`; the statics
   (`OnePointStatics`), carried over scene changes.
3. **`z_play.c`:** `func_800C0808`, `func_800C08AC`, `Play_SetCameraRoll`, `func_800C0D34`;
   `play->view` (`PlayState::view`).
4. **`z_camera.c`:**
   - `Camera_Unique9` (`CAM_SET_CS_C`), whole: the keyframes' advance and their `unk_01` (the
     interface, `D_8011D3AC`, Player's mode), the at's and eye's target kinds (fixed, from the
     view or the camera, round the target from Player's side, round an actor's focus, world or
     shape), the fov and roll, the actions (15, 16, 21: copies; 1 to 4, 9 to 12: the
     interpolations; 13: the turn; 24; 18 and 19: the copy to the parent), the bgcheck (0x80) and
     Player held (0x40);
   - `Camera_Demo9` (`CAM_SET_CS_3`), whole: the splines round the main camera's Player, this
     camera's player or its target, the wait, the finishing actions;
   - `Camera_Demo5` (`CAM_SET_CS_ATTENTION`), whole: its eight branches and their tables, the
     timer lengthened by the return's keyframes, the chime, Player held, then `CAM_SET_CS_C`;
   - `Camera_InitPlayerSettings` for a sub camera; `Camera_Finish`; `Camera_LERPFloorF`; the
     debug ROM's D-Right ending a timed camera; the camera's own target read each frame (the
     update had read the main camera's for every camera).
5. **Player:** `func_8083F570` starts 9601 and 9602 (`PlayRequest::OnePointCutscene`);
   `currentBoots` and outside writes of `stateFlags1` through `PlayerIface`.
6. **`En_Box`:** `EnBox_FallOnSwitchFlag`'s 4500 and `EnBox_Fall`'s end of it,
   `EnBox_AppearOnSwitchFlag`'s and `EnBox_AppearOnRoomClear`'s attention cutscenes (with
   `_CheckForCategory`), `EnBox_AppearInit` waiting for the attention camera (`func_8005B198`).
7. **Tests:** `oot_actors --test crawl` (`the_way_out_is_a_one_point_cutscene`),
   `oot_actors --test onepoint` (an attention cutscene on a Kokiri child, the falling chest's
   shot), `oot_import --test pack` (`the_one_point_tables_are_the_roms`).
8. **Run scripts:** `test-onepoint.bat`, `game-crawlspace.bat` (menu 36, 37); `_env.bat` on
   `game13` and `data12`.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 286 passed, 1 ignored |
| The tables (`the_one_point_tables_are_the_roms`) | All 105 the ROM's bytes in `code`, each at its `D_` address's offset; the settings they name are `z64camera.h`'s (`CAM_SET_CS_C` 0x3C, `CAM_SET_CS_3` 0x2A, `CAM_SET_FREE2` 0x22) |
| The crawlspace's exit (`the_way_out_is_a_one_point_cutscene`) | On `func_8083F570`'s frame: sub camera 1 active, 9601, `CAM_SET_CS_3`, the main camera its parent, `CAM_STAT_UNK3`, back on its `prevSetting` (NORMAL0); `D_80120308` and `D_80120398`, action 1 (the 0x1000 taken off by the first frame), 90 frames. Its first view: the splines at u 0, `(p0 + 4 p1 + p2) / 6` of the tables' first points, turned round Link by his yaw, within 0.01; the fov `(40 + 4 x 40.000004 + 50) / 6`. 92 updates of the sub camera (`animTimer` 90 to -1, then the finish), in 67 frames: on 25 of them the main camera, back on the crawlspace's bg camera, asks for `view.unk_124` and `Play_Draw` updates the active camera again (as the C does). Then the sub camera cleared, the main camera active and alone in the queue, starting within 40 of the cutscene's last eye and at, on room 2's bg camera 14 (`CAM_SET_DUNGEON0`) |
| Link's yaw at the exits (`crawl`'s other tests) | 0 where it was 1: 9601's and 9602's `SetInfo` take the main camera off CRAWLSPACE in Player's update, so `Camera_Subj4` doesn't write his yaw that frame |
| An attention cutscene (`an_attention_cutscene_on_a_far_npc`) | On a Kokiri child 900 from Link: sub camera 1, 5010, `CAM_SET_CS_ATTENTION`, timer 100 (an NPC), at the main camera's at and eye, `data1` `NA_SE_SY_CORRECT_CHIME`. Its first frame: `D_8011D9F4` (the last branch), `[1].timerInit` 13 (`eyeTargetDist * 0.005 + 8`, no bgcheck hit from Link's head), the timer 100 + 13 + 1, `CAM_SET_CS_C`; keyframe 0's at round the child's focus from Link's side, its eye 300 behind it then bgchecked into the hillside (1 off the wall), fov 60; the chime once at no position; Link's mode 1, the cutscene action the next frame. It lasts 101 + 13 frames (keyframe 0's 100, keyframe 1's 13, keyframe 2's copy); then the main camera active and Link's mode 7 (`Camera_Finish`) |
| The falling chest's shot (`the_falling_chests_shot`) | 4500: `CAM_SET_FREE2`, the at 40 above the floor under the actor's focus, the eye 150 at pitch 0x3E8 along its yaw, fov 50, roll 0, timer 9999; Link held (mode 8); `OnePointCutscene_EndCutscene` ends it at the end of the frame, mode 7 |
| Golden traces and renders | `sword_chest`, `mido_shop`, `mido_shop_audio`, `new_save_deku_tree`, `new_file_deku_tree` re-recorded: the same bytes to the first crawlspace exit, then the camera's yaw steers the routes differently; the same steps, the runs 2 to 11 frames shorter. Every render and `playthrough` unchanged: 85 hashes |
| Import | 21 s, format 15, into `out/data12` |
| Headless screenshots | The sword route's exit: the spline's first shot in front of Link as he climbs out, letterboxed; the normal camera behind him after |

### Decisions

- **[ADR 0029](adr/0029-one-point-cutscenes.md)**: the tables in the pack and their statics on
  the play state; the queue on the cameras; `CamRequest`; `CamActor`; the cases ported.
- **Navi starts no one-point cutscene.** BACKLOG #3 named "Navi's attention cutscenes
  (`OnePointCutscene_Attention`, which `Actor_SetNaviToActor`'s callers start)": in this decomp
  neither `En_Elf` nor `z_actor.c` calls `OnePointCutscene_*`; the attention calls are the
  dungeon actors' (`En_Box`, `Door_Shutter`, `Obj_Switch`, `Obj_Syokudai` and others).
- **The double update while on the crawlspace's floor is kept**: it's the C's arithmetic (the
  waiting main camera's `Camera_Subj4` asks for `view.unk_124`).

### Known gaps

- **Player's cutscene modes** the attention camera asks for when Player is the target (12, 69)
  and 3050's (5): logged (`Door_Shutter` on Player, Phase 6). Milestone 3 ports the generic
  modes.
- **`SetInfo`'s other cases** (other scenes' actors, most with quakes): logged; quakes
  (`Quake_Add`) aren't ported.
- **`En_Box`'s song chests** (`func_809C9700`'s attention) wait for the ocarina.

## Milestone 2: the camera modes on the fallback

**Answer:** done. The three modes Link uses in Kokiri Forest that ran `Camera_Normal1` on
NORMAL's data (ADR 0013's fallback) have their own functions now: `Camera_Jump1` (JUMP,
FREEFALL), `Camera_Jump2` (CLIMB, CLIMBZ) and `Camera_Unique1` (HANG, HANGZ). Going down the
ladder from Link's house the camera stays behind him at the CLIMB data's distance (BACKLOG #2 is
fixed). The cutscenes' and one-point cutscenes' own settings were all ported in milestone 1 or
before (`CAM_SET_CS_0`, `_CS_3`, `_CS_C`, `_CS_ATTENTION`, `_FREE2`, `_TURN_AROUND`), so nothing
else was needed for them.

The tests: 289 pass, 1 ignored. The goldens: 85 hashes, 61 cases; 28 re-recorded
(golden/README.md).

### What was built

1. **`Camera_Jump1`**, whole: the at in the air against the height Link left the ground at
   (`func_800458D4`), the eye's distance eased and clamped to `[distMin, distMax]`, its pitch to
   `[R_CAM_MIN_PITCH_2, R_CAM_MAX_PITCH]`, its yaw swinging behind him (`Camera_CalcDefaultYaw`)
   or round a wall (its own `SwingAnimation`), its height eased by `OREG(31)`; the input
   direction while active; its `@bug (game)` (`fovUpdateRate` eased from `yOffsetUpdateRate`).
2. **`Camera_Jump2`**, whole: the floor under Link at the start (`Camera_GetFloorY`), the turn
   behind him over `R_CAM_DEFAULT_ANIM_TIME`, then within `yawAdj` of it; the distance within
   `[minDist (1 - f), maxDist (1 + f)]`; level near the ground and below the top of the climb
   (`Camera_GetFloorYNorm` 25 ahead, 2.2 heights up), the pitch within 0x2AF8; out of walls by a
   level check.
3. **`Camera_Unique1`**, whole: the at with the slope (`Camera_CalcAtDefault`), the pitch eased
   to the HANG data's target, the yaw turning towards Link's waist's side when it's far off
   (`yawTargetAdj`), `Camera_LERPClampDist`.
4. `func_80046E20` (`swing_anim`) takes its `SwingAnimation`, as the C passes it.
5. **Tests:** `oot_game`'s `camera::tests` (the pack's camera data, no collision):
   `camera_jump2_turns_behind_link_on_the_ladder`, `camera_jump1_keeps_its_distance_in_the_air`,
   `camera_unique1_hangs_at_its_pitch_target`.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 289 passed, 1 ignored |
| `Camera_Jump2` (CLIMB) | Its first frame: the yaw halfway to behind Link (`Camera_LERPCeilS(behind, yaw, 0.5, 0xA)`), the distance the moved at's to the old eye clamped to the CLIMB data's range, the pitch kept with no floor anywhere, `pitchUpdateRateInv` and `rUpdateRateInv` 100; after the timer, within `yawAdj` of behind him |
| `Camera_Jump1` (JUMP) | From an eye 20 away and steep: `eyeNext`'s horizontal distance `distMin cos(R_CAM_MAX_PITCH)`, its height eased by `OREG(31)`, `xzOffsetUpdateRate` from 1/10000 towards `OREG(2)` by `OREG(25)` |
| `Camera_Unique1` (HANG) | The pitch eased towards the HANG data's target by `pitchUpdateRateInv` (itself eased towards 100), the yaw target moving by `((diff / R_CAM_DEFAULT_ANIM_TIME) / 4) * 3` a frame towards Link's waist's side, the yaw following it by `Camera_LERPFloorS(.., 0.5, 0x2710)` every frame, the roll 0 |
| The ladder (headless, the sword route's frame 205) | The camera behind Link and below the porch, about 120 away (the CLIMB data's), the ladder centred |
| Golden traces and renders | 28 re-recorded: the course scripts' climbs, hangs, falls and jumps (camera fields only), the four Kokiri Forest spawns in the air, and the routes from the ladder on (their steering follows the camera's yaw); every other case the same bytes |

The angles in the tests are compared within 0x10: the C measures them back through
`Math_FAtan2F`'s Taylor series, which differs from the sines that placed the points by about
that much.

### Known gaps

- **Still on the fallback:** BATTLE (`Camera_Battle1`, Phase 6's enemies), CHARGE
  (`Camera_Battle4`, the spin attack), first person and the items' aiming (`Camera_Subj3`), the
  hookshot (`Camera_Special5`), the swim's (`Camera_Jump3`, with `Camera_UpdateWater`), and the
  scene settings that use `Camera_Normal2`, `_Normal3` and the others. None is reached by a
  cutscene.
- **The Mido route's walk** to the plateau switch's rupee now pushes Link against the sign for
  about 150 frames before it gets round: the route's steering, not the game.

## Milestone 3: cutscene audio

**Answer:** done. The scripts play their music: the Deku Tree's talk fades the forest's music out
as it starts and plays his theme at frame 140, yes stops it and plays the next; Navi's flight
through the village has its own. `z_demo.c`'s own sounds play (the Deku Tree's death, the
Triforce's flash, the sandstorm, the white-outs, the skip's chime) and the rain's and
lightning's nature ambience channels are set. Player's cutscene modes are now driven by the C's
two tables (`D_80854B18`, `D_80854E50`, in the pack) through their typed handlers
(`D_80854AA4`), so every mode that's only an animation (with or without its sound table) plays
as the C's, and the named mode functions are ported where what they use is: Link groans as he
tosses in bed, sighs and slips off it as he sits up, and the other modes' sound tables play.

The tests: 292 pass, 1 ignored. The goldens: 85 of 85 unchanged (sounds don't reach the traces).

### What was built

1. **`Cutscene_Command_PlayBGM`, `_StopBGM`, `_FadeBGM`** (`CsCmdMusicChange`: the sequence, plus
   one, at byte 1; `CsCmdMusicFade`: the type, the start and end frames): `func_800F595C`,
   `func_800F59E8`, and `SEQCMD_STOP_SEQUENCE` of the main bgm (the fanfare's for type 3) over the
   fade's frames.
2. **`func_80064824`'s sounds:** `NA_SE_EV_DEKU_DEATH` (misc 11 at 0x30F), `NA_SE_EV_TRIFORCE_FLASH`
   (misc 13 as its fade starts), `NA_SE_EV_SAND_STORM` (misc 32, every frame); the rain's
   (`NATURE_CHANNEL_RAIN`, ports 4 and 1) and lightning's (`NATURE_CHANNEL_LIGHTNING`, port 0)
   nature ambience channels on misc 1 and 2.
3. **`Cutscene_Command_Terminator`'s** `NA_SE_SY_PIECE_OF_HEART` when A, B or Start skips a scene
   outside normal play; **`Cutscene_Command_TransitionFX`'s** white-outs (`NA_SE_SY_WHITE_OUT_S`
   in the Chamber of Sages, `NA_SE_EV_WHITE_OUT` in the Temple of Time and the Great Fairies'
   fountains, and through `func_800788CC` in Ganon's castle's collapse).
4. **Player's cutscene modes from the C's tables:** the pack holds `D_80854B18` and `D_80854E50`
   (`GameData::cs_mode_starts`, `cs_mode_updates`: each entry's type and its animation, sound
   table or function, read from `z_player.c`); `func_80852B4C` dispatches on them: the 18 typed
   handlers of `D_80854AA4` (`func_80851008` to `func_808512E0`, with `func_80850F1C`,
   `func_80850F9C`, `func_80833064`, `func_80833114`, `func_80851294`), and the named functions:
   `func_80851750`, `func_80851788`, `func_80851828`, `func_808518DC` and `func_8085190C` (mode
   12, the attention camera's on Link), `func_808519EC`, `func_80851B90`, `func_80851BE8`,
   `func_80851CA4`, `func_80851DEC`, `func_80851E28`, `func_80851E64`, `func_80852048`,
   `func_80852080`, `func_80852174`, `func_808521B8`, `func_808521F4`, `func_8085225C`,
   `func_80852280`, `func_80852328`, `func_80852358`, `func_80852388`, `func_80852450`,
   `func_80852480`, `func_80852564`, `func_808525C0`, besides the ones already ported; with their
   sound tables (`D_80855188`, `D_808551B4`, `D_808551C8`, `D_808551D8`, `D_808551E0`,
   `D_808551E8`, `D_808551F0`, `D_808551F8`, `D_80854AF0`, `D_80854B00`, `D_80854B14`).
   `func_80851E90` groans (`NA_SE_VO_LI_GROAN`), `func_80851FB0` plays `D_808551BC`.
5. **A fix:** `func_808515A4` played its wait with `LinkAnimation_PlayOnceSetSpeed(D_808535E8)`
   where the C's `func_80832264` is `LinkAnimation_PlayOnce` (the same unless `D_808535E8` is 0.5).
6. **Tests:** `oot_actors --test cutscene` (`the_deku_trees_talk_plays_its_music`,
   `link_groans_and_sighs_as_navi_wakes_him`), `oot_actors --test onepoint`
   (`an_attention_cutscene_on_link_himself`: mode 69 through the table).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 292 passed, 1 ignored |
| The Deku Tree's talk (`the_deku_trees_talk_plays_its_music`) | From the scripts' entries: `D_808BCE20`'s fade (type 4, frames 0 to 20) queues `0x101400FF` (the main bgm stopped over 20 frames) on the script's first frame; its PLAY_BGM (0x4C) plays sequence 0x4B on frame 140; `D_808BD520`'s STOP_BGM stops it on 90 (op 1, no fade), its PLAY_BGM plays 0x3C on 99 |
| The wake-up (`link_groans_and_sighs_as_navi_wakes_him`) | The groan (`NA_SE_VO_LI_GROAN` plus the child's voice offset) at Link on the frame mode 39 starts, in the narration and in the wake-up; mode 40's sigh and two slips on its animation's frames 35, 236 and 256, which at 1.5 animation frames a game frame fall `ceil(n / 1.5)` game frames from the mode's start, exactly |
| Mode 69 (`an_attention_cutscene_on_link_himself`) | The attention camera on Link: `D_8011D6AC`, keyframe 1's timer 29, the timer 30; Link's mode 69, whose start (type 3) plays `link_hatto_demo` once at 2/3 with a morph; mode 7 at the end |
| Golden traces and renders | 85 of 85 unchanged |
| Import | format 15, into `out/data12` (the two tables added) |

### Decisions

- **Player's cutscene modes are data**: the two tables come from the C through the pack, and the
  port matches function names; a new mode's function is one arm. (`D_808547C4`, the cue-to-mode
  table, stays as GAME-03 wrote it.)

### Known gaps

- **Mode functions not ported**, logged: the swimming ones (`func_80851368`, `func_808513BC`), the
  ocarina's (`func_80851D2C`), the sword and items in hand (`func_80851A50`, `func_80851D80`,
  `func_80852298`, `func_80852608`, `func_80852648`, `func_8085283C`, `func_808528C8`: they need
  `func_80846720`), `func_808524B0` (`func_80837704`), `func_808524D0` and `func_80852514`
  (`func_80844E68`), `func_808526EC` (the ocarina's sparkles, an effect). None is in a played
  scene.
- **`func_80852388`'s hand model** (`rightHandType`) isn't modelled.
- **The weather's own state** (the rain's precipitation, lightning, the storm's end: misc 18) is
  milestone 4's.

## Milestone 4: the rest of `z_demo.c`'s commands

**Answer:** done for the state; the weather's drawing is milestone 6's. The environment is
`play->envCtx` across frames now (`oot_game::env::EnvCtx`): `Environment_Update`'s lights run
every frame with the light setting override and its blend, a time-based config's change, and the
`adj*` adjustments; the rain's drops grow and shrink (`Environment_UpdateRain`); the lightning's
strike and bolts run (`Environment_UpdateLightningStrike`, `Environment_DrawLightning`'s states,
with their `Rand_ZeroOne` calls where `Play_Draw` makes them). The scripts' misc actions and the
lighting command drive them: in the nightmare it rains (misc 1) and bolts strike (misc 2).

Which commands the played scripts use: the nightmare misc 1 (rain), 2 (lightning, six), 3 (env
flag 0), the transition fills and `CS_CMD_09` (the rumble: no rumble pak); Link's house misc 12
(the end) and 14 (the viewpoint); the Deku Tree's intro misc 12 and 15 (the title card,
milestone 5); the Deku Tree's talk misc 12. None uses `CS_CMD_SET_LIGHTING`.

The tests: 295 pass, 1 ignored. The goldens: 85 of 85 unchanged.

### What was built

1. **`EnvCtx`** (`play->envCtx`): `Environment_Init`'s values at `Play_Init` (the scene's light
   mode, its time settings' day and sky), and `Environment_Update`'s light part every frame
   (`environment_update_lights`, into the scene's lights the renderer reads): the override
   switch (`prevLightSetting`, `lightBlend` 0), the time-based configs with `changeLightEnabled`'s
   blend into the next config, the settings' blend by the setting's rate (`fogNear >> 10` x 4) or
   `lightBlendRateOverride`, `LIGHT_SETTING_OVERRIDE_FULL_CONTROL`, the adjustments clamped as
   s16 sums (dirLight2 with `adjLight1Color`), fog near 996 and far 12800 at most; the skybox's
   time following the day in a cutscene layer.
2. **The weather's state:** `precipitation[]` and `Environment_UpdateRain`; `EnvStatics`
   (`gWeatherMode`, `gLightningStrike` and `sLightningFlashAlpha`, `sLightningBolts`, reset by
   `Environment_Init`); `Environment_AddLightningBolts`; the strike's three states with the
   ambient light's flash and the thunder (`NATURE_CHANNEL_LIGHTNING`) when `lightningState` is
   on; the bolts' states (placed 9500 ahead of the view's eye, waiting 3 frames per slot, eight
   textures), giving `PlayState::lightning_bolts` and `lightning_flash` for the renderer.
3. **`func_80064824`'s actions:** 1 (rain), 2 (lightning), 6 (`adjFogFar`), 7 (the light config's
   change over 60 frames; the skybox's change kept, not drawn), 9 (snow's maximum), 18 (the
   storm's end: no more rain, `STORM_REQUEST_STOP`, the time, the weather cleared with the rain's
   channel), 22 and 23 (`D_801614B0`, the screen's tint, kept), 26 (the light setting by the time
   of day), 27 (the ambient light's flicker), 28 and 29 (`unk_11DE9`: `Actor_UpdateAll`
   skipped), 32 (the sandstorm's state), 34 (time backwards by `gTimeSpeed`: 0).
4. **`Cutscene_Command_SetLighting`:** `lightSettingOverride` (the setting, minus one) and
   `lightBlend` 1.
5. **Tests:** `oot_actors --test environment`: `the_nightmares_rain_and_lightning`,
   `a_light_setting_override_blends_in`, `a_lightning_strike_flashes_the_ambient_light`.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 295 passed, 1 ignored |
| The nightmare (`the_nightmares_rain_and_lightning`) | Misc 1 on the script's first tick: `PRECIP_RAIN_MAX` 20, the drops up by 2 on exactly the frames that are multiples of 8; misc 2 at frame 20: the three bolts waiting (3, 6 and 9 frames), 9500 ahead of the view's eye in xz, 4000 to 5000 up; the strike's state START with `lightningState` off (no En_Weather_Tag in the scene: no flash runs, as in the C); bolt i drawn from 3 (i + 1) frames after, its textures 0 to 7 a frame each; misc 2 at 24 finds no free slot |
| A light override (`a_light_setting_override_blends_in`) | In the Deku Tree (`LIGHT_MODE_SETTINGS`): the override to setting 1 switches (`prevLightSetting` 0) and the ambient light blends by setting 1's rate / 255 a frame, `LERP` truncated to u8, exactly |
| A lightning strike (`a_lightning_strike_flashes_the_ambient_light`) | With lightning on: the flash at alpha 100 then 200 (`[200, 200, 255]`), the ambient adjustments (80, 80, 100) a frame, the lights the next frame raised by them; 20 frames down by 10 to the wait, the adjustments back to 0 |
| Golden traces and renders | 85 of 85 unchanged (the lights computed each frame equal the ones a scene loads with; the nightmare's new `Rand` calls don't reach the traces) |

### Decisions

- **The lights are computed every frame** (`Environment_Update`'s order: after the cameras), not
  only at load; `SceneState::load` keeps its first computation for the frame before the first
  update.
- **The weather's drawing waits for milestone 6** (the rain's lines, the bolts' textures, the
  flash's fill) with the nightmare's actors.

### Known gaps

- **Not drawn yet:** the rain, the bolts, the lightning's flash (milestone 6), the screen's tint
  (`VisMono`), the sandstorm, the snow, the skybox's change.
- **Logged:** quakes (misc 16, 17), the title card (misc 15, milestone 5), the room's segment
  (misc 24), the Sun's Song (33), the scarecrow's song (35); `CS_CMD_09`'s rumble.
- **The weather at `Play_Init`** (`retainWeatherMode`, `Environment_UpdateStorm`) isn't ported.

## Milestone 5: title cards

**Answer:** done (BACKLOG #5). Entering a scene by an entrance that shows its title card fades
the scene's place name in and out (`Player_Init`'s `TitleCard_InitPlaceName`), and the Deku
Tree's intro shows "Inside the Deku Tree" from its `CS_MISC` 15. The place names are in the pack:
each scene's title file (`scene_table.h`'s second column), its first language's IA8 144 by 24
texture, baked as a sprite (`title/g_pn_XX`).

The tests: 296 pass, 1 ignored. The goldens: one render re-recorded, `spot04_treemouth_open`
(its frame 40 now shows "Kokiri Forest"); every trace unchanged.

### What was built

1. **`TitleCardContext`** (`oot_game::title_card`, `play->actorCtx.titleCtx`):
   `TitleCard_Init`, `TitleCard_InitPlaceName`, `TitleCard_Update` (at the end of
   `Actor_UpdateAll`: after the delay, in by 10, the intensity by 20, while the 80 frames last,
   then out by 30 and 70), `TitleCard_Clear`, and `TitleCard_Draw` (at the end of
   `Actor_DrawAll`, before the HUD: the texture rectangle centred on (160, 120), its primitive
   colour the intensity, grey, with the alpha).
2. **`Player_Init`'s title card:** on a fresh entry (`respawnFlag` 0 or below -1), with
   `showTitleCard` set, a title file, the entrance's `ENTRANCE_INFO_DISPLAY_TITLE_CARD_FLAG`, no
   cutscene layer, and (Dodongo's Cavern) `EVENTCHKINF_B0` or (the night shop) `EVENTCHKINF_25`;
   then `showTitleCard` back on.
3. **`CS_MISC` 15** (`func_80064824`): `TitleCard_InitPlaceName` with the loaded scene's title.
4. **The pack:** `SceneEntry::title_file` (from `scene_table.h`, empty for `none`), and the place
   names' sprite bakes (`title_card::bakes`, `Gfx_SetupDL_52NoCD`'s modes).
5. **Tests:** `oot_actors --test environment`'s `entering_kokiri_forest_shows_its_place_name`
   (`g_pn_31`, 20 frames of delay, the alphas frame by frame, the draw's rectangle and colour);
   `oot_actors --test cutscene`'s `entering_the_deku_tree_the_first_time_plays_its_intro` (no
   card from `Player_Init` with `showTitleCard` false, `g_pn_06` from the script's misc 15 on its
   first frame); `oot_import --test pack`'s `tables_match_the_c` (the title files, every bake
   present).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 296 passed, 1 ignored |
| Kokiri Forest's card (`entering_kokiri_forest_shows_its_place_name`) | `title/g_pn_31` after `Play_Init`, delay 20, duration 80; the alpha 0 for 19 frames, up by 10 to 255 over the next 79, then down by 30; drawn from (88, 108) to (231, 131.75) |
| The Deku Tree's intro | No card from `Player_Init` (the intro's entrance cutscene clears `showTitleCard`); misc 15 on the script's frame 0 starts `g_pn_06`; the headless screenshot at frame 50 shows "Inside the Deku Tree" between the letterbox's bars |
| Golden traces and renders | `spot04_treemouth_open/shot.png` re-recorded (the card at frame 40, alpha 210); the other 84 unchanged |

### Decisions

- **The first language only** (`gSaveContext.language` 0, English): the title files hold each
  language's texture after the first; the bakes take offset 0.
- **The bake has no cull.** `SETUPDL_52` loads `G_CULL_BACK`, but the card is a texture
  rectangle, which the geometry mode doesn't reach; the bake draws it as two triangles, and with
  the cull the renderer dropped them (the first screenshots had no card), so its geometry mode is
  left empty.

### Known gaps

- `TitleCard_InitBossName` (the boss cards) waits for Phase 6's bosses.
- The other languages' textures aren't baked.

## Milestone 6: the opening's nightmare

**Answer:** done (BACKLOG #7). The nightmare now has everything the C draws in it except
Ganondorf's cape:
- the castle's drawbridge with its chains and torches, raised, then lowered on the script's env
  flag;
- Zelda and Impa on Zelda's white horse, and Ganondorf on his black horse, rearing, each
  following the script's cues;
- the rain, and the lightning bolts.

The horses are skin skeletons, new to the engine ([ADR 0030](adr/0030-skin-skeletons.md)).

The tests: 305 pass, 1 ignored. The goldens: one trace re-recorded (`new_file_deku_tree`: three
more actors during the nightmare); every render unchanged.

### What was built

1. **`Bg_Spot00_Hanebasi`** (`oot_actors::bg_spot00_hanebasi`), whole:
   - the drawbridge and its two chains as DynaPoly actors, each spawning the next;
   - raised in the opening's layers, by night for a child, and with the three stones before
     `EVENTCHKINF_80`;
   - `DrawbridgeWait` and `DrawbridgeRiseAndFall` (80, the chains 0.4 of it past -0x27D8), with
     their sounds;
   - the chains placed by the bridge's draw matrix (at draw time);
   - the torches' point lights flickering by `Rand_ZeroOne` and their flames (`gEffFire1DL` turned
     to the camera, scrolled);
   - the child's entry into Zelda's escape (`ENTR_SPOT00_0`, cutscene 0xFFF1) and the storm
     request.
2. **`En_Viewer`** (`oot_actors::en_viewer`): all ten types' logic:
   - waiting for their objects;
   - the cue's path (`Environment_LerpWeight`);
   - the animation changes on the cues' actions (Ganondorf's rear, look and charge; Zelda's and
     Impa's looks back);
   - the sounds (the gallop, the neighs, the cuts' `NA_SE_SY_DEMO_CUT`, Ganondorf's fanfare);
   - the shared `sTimer` and `sHorseSfxPlayed`;
   - the spawns (`En_Ganon_Mant`, `Item_Ocarina`, `Demo_6K`).

   The draws, as bakes:
   - Zelda's eyes and mouth by the frame, and her Hyrule Field dress and hidden limbs;
   - Impa's masked head;
   - young Ganondorf's eyes and his open hand from the nightmare's frame 400;
   - the horses' skins.
3. **Skin skeletons** ([ADR 0030](adr/0030-skin-skeletons.md)):
   - `oot_game::skin`: `z_skin_matrix.c`'s functions, `Skin_ApplyAnimTransformations`, the
     groups' points;
   - `oot_import::skin`: the parser and the bake, with a bone per vertex group;
   - `eng_gbi`'s per-vertex bones;
   - the skin skeletons in the pack (191 of the 194 XML skeletons now);
   - foreign animations (Impa's two in `object_opening_demo1`).
4. **The weather's draw** (`oot_game::weather`):
   - `Environment_DrawRain`'s drops and ground rings, with their `Rand_ZeroOne` calls where
     `Play_Draw` makes them (after the rooms, before the actors);
   - `Environment_DrawLightning`'s bolts on the billboard, their eight textures;
   - `SETUPDL_20` and `SETUPDL_61`;
   - `envCtx.windDirection`.
5. **Tests:**
   - `oot_actors --test nightmare`: `the_drawbridge_lowers_on_the_scripts_flag`,
     `the_riders_follow_their_cues`, `the_horses_draw_their_skins`, `the_rain_draws_its_drops`;
   - `oot_import --test pack`: `the_horses_skins_are_the_roms`;
   - `oot_game`'s unit tests `skin::tests` (the rotations, the product, the limbs' walk).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 305 passed, 1 ignored |
| The drawbridge (`the_drawbridge_lowers_on_the_scripts_flag`) | Spawned once `object_spot00_objects` loads: raised (-0x4000), the chains at 0xF020. The torches' lights at (±260, 168, 690), radius 0. Held until the script's env flag 0; the next update switches action, and the one after starts: 120 a frame (`Math_ScaledStepToS` at `R_UPDATE_RATE` 3), the chains 48 a frame once past -0x27D8, every frame exact. The lights at radius 300 with red and green 128 to 254. The chains at the bridge's ends through its draw matrix |
| The riders (`the_riders_follow_their_cues`) | Zelda and her horse on cue 0's line by `Environment_LerpWeight`, exactly, with the cue and frame from before the frame's cutscene step. On cue 1's action 1 Ganondorf and his horse rear (`gYoungGanondorfHorsebackRearAnim`, `gHorseGanonRearingAnim`) |
| The horses (`the_horses_draw_their_skins`, `the_horses_skins_are_the_roms`) | `gHorseZeldaSkel` 46 limbs, `gHorseGanonSkel` 53, as their headers count them, each with one animated limb whose groups hold every vertex once. Drawn only on their cue, with a bone per limb and group (182 for Zelda's), at the actor at scale 0.01. Posed at the galloping animation's frame 0 the horse spans 27 by 79 by 125 |
| The rain (`the_rain_draws_its_drops`) | A drop per `precipitation[PRECIP_RAIN_CUR]`, each within the C's box 50 ahead of the eye, scaled 1.2 tall; as many rings while Link is below the eye |
| Headless screenshots | The opening's nightmare at 640x360 every 30 frames: the bridge lowering, the riders galloping out, Ganondorf rearing, the rain |
| Golden traces and renders | `new_file_deku_tree`: only `actors` differs, +3 on frames 843 to 1403 (the two chains and `En_Ganon_Mant`'s placeholder). Compared against a build of this code with the two actors unregistered and the rain undrawn, which reproduces milestone 5's hash. The rain's and torches' `Rand` calls end with the scene: each `Play_Init` starts the port's generator at its fixed seed. Every render unchanged |

### Decisions

- **The horses' skin is a bone per vertex group**, exact for the C's groups, with no renderer
  change ([ADR 0030](adr/0030-skin-skeletons.md)).
- **The limbs' walk is iterative**: the recursive `func_800A698C` came out wrong from rustc 1.95
  and 1.98 at `opt-level` 1 and above (ADR 0030).
- **Ganondorf's cape is left out.** In the nightmare the C's cape is degenerate: type 3's
  `EnViewer_UpdateGanondorfCape` sets nothing, so `En_Ganon_Mant` hangs from the world's origin
  with no gravity. Its mesh is also overlay data (`ovl_En_Ganon_Mant`), which the importer
  doesn't read. Phase 6's bosses will need it.

### Known gaps

- `En_Ganon_Mant` (a placeholder), `Item_Ocarina` (`object_gi_ocarina` isn't in the nightmare's
  object list, so its spawn fails there as in the C), `Demo_6K`.
- `En_Viewer`'s horse shadows (`ActorShadow_DrawHorse`), the adult Ganondorf's draw (type 9) and
  type 5's flames.
- The lightning's flash: in the nightmare `lightningState` is off, so it never runs. Elsewhere it
  would be a fill under the rooms. Also the snow, the sandstorm, and the point lights' glows.

## Milestone 7: cutscene polish

**Answer:** done for the tests. They found no port bugs: `Camera_Demo1` follows the scripts'
splines on every frame of the narration and of the Deku Tree's talk, the letterbox steps as
`Letterbox_Update` does, the narration's text box sits where `Message_Update` puts it, and Link
takes each opening cue's pose from Player's cutscene tables. What's left is to look at the
opening and the Deku Tree's talk by hand.

The tests: 310 pass, 1 ignored. The goldens: 85 of 85 unchanged.

### What was built

1. **`oot_actors --test polish`**, five tests with their expectations from the C:
   - `the_narrations_camera_follows_its_splines` and
     `the_deku_trees_talk_camera_follows_its_splines`: the active camera's eye and at on every
     frame against `Camera_Demo1` run independently from the script's lists.

     The rules come from `z_demo.c` and `z_camera.c`:
     - a pair takes over on the first frame after its start, once both lists have been seen;
     - the script's first pair is applied on two frames running (the at command completes it,
       then the eye command's `unk_18` is set a frame later), and each application resets
       `Camera_Demo1`;
     - the eye's and the at's splines share one keyframe and frame counter, so the at's call
       sees the counter the eye's moved;
     - the camera keeps updating while a text holds the script's frame;
     - it stops when a spline ends (`animState` 2).

     About 250 frames of the narration and over 100 of the talk, to 0.01.
   - `the_letterbox_grows_and_shrinks_by_ten_rows`: the talk's start (`cutsceneTrigger`, so
     `func_80064760` sets the target each frame) grows 0, 10, 20, 30, 32; its end, when Link's
     camera takes the view back, shrinks 32, 22, 12, 2, 0.
   - `the_narrations_text_sits_where_the_c_puts_it`: text 0x109D's box (type 4,
     `TEXTBOX_TYPE_NONE_BOTTOM`) at `sTextboxXPositions`' and `sTextboxLowerYPositions`' 34 and
     174, the end icon 34 below, put straight there at full size.
   - `links_poses_follow_the_cutscene_mode_tables`: through the whole opening, each new cue's
     mode (`D_808547C4`) starts the animation its `D_80854B18` entry names. Covered:
     - the type-2 and type-6 entries: 9 `link_demo_furimuki`, 40 `clink_op3_okiagari`,
       41 `clink_op3_tatiagari`;
     - the function starts: 38 `func_80851F84` (`clink_op3_wait1`, the shadow off) and
       39 `func_80851E90` (`clink_op3_negaeri`).
2. `oot_actors::player::d_808547c4` is public, for the tests.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 310 passed, 1 ignored |
| The splines | Every checked frame within 0.01 of the C's rules, the eye and the at |
| The letterbox | 10 rows a frame up and down, the C's sequences |
| The narration's box | Type 4, position 0: (34, 174), end icon at 208, no growing |
| Link's poses | Modes 9, 38, 39, 40 and 41, each on the frame of its cue |
| Golden traces and renders | 85 of 85 unchanged (tests only) |

### Decisions

- **The splines' check runs `Camera_Demo1` from the script, not from the camera's state**: what
  it tests is the wiring, meaning which lists, from which frame, how many updates. The spline
  itself was already checked against the C (`the_b_spline_through_four_points`).

### Known gaps

- The relative-to-Link lists (`CS_CMD_CAM_EYE_REL_TO_PLAYER`) aren't in these two scripts; the
  Deku Tree's intro and the nightmare's lists aren't checked frame by frame.
- **By hand, still to do** (no side-by-side checklists): the opening (the narration, the nightmare
  with its riders, rain and drawbridge, Navi sent, the wake-up) and the Deku Tree's talk and
  intro (its title card), looking for anything that looks wrong.

## Fixes after playing by hand (2026-10-01)

The user played the opening and reported three things. All three are fixed:

1. **The first text after the narration was hidden** under a grey screen. In Kokiri Forest's
   layer 7 (Navi sent), the Deku Tree's first text comes while the entrance's delayed white-out
   (`TRANS_TYPE_FADE_WHITE_CS_DELAYED`, (160, 160, 160)) still covers the screen. The port
   blended all fills over the finished image, the HUD and the message box included.
   - In the C (`Play_Draw`), the transition is drawn at the start of `OVERLAY_DISP`, under the
     HUD and the message box (`Play_DrawOverlayElements`).
   - The script's fill (`envCtx.fillScreen`) is drawn into both the OPA and the XLU lists
     (`FILL_SCREEN_OPA | FILL_SCREEN_XLU`), so the opaque scene takes it twice.
   - Now `DrawLists` carries the three fills (`opa_fill`, `xlu_fill`, `overlay_fill`) and the
     renderer draws each in its place (`PlayState::draw_fills`).
   - The transition's fill also covers the letterbox bars, as the C's full-screen view does. The
     script's fill stays inside them, as the C's scissored view does.
2. **Navi's flight through Kokiri Forest showed the meadow alone, in yellow fog.** Link stands
   in room 1 (the Deku Tree's meadow) the whole time, and the camera flies to the village
   (room 0). In a cutscene, `En_Holl`'s `func_80A59014` tests its plane against the view's eye,
   not Player (`useViewEye`), and the port had left that case out. The village now loads as the
   camera crosses the plane.
3. **Link's pose glitched for a split second**, most of all as he turns to Ganondorf
   (`link_demo_furimuki`) and as he gets up. Some animations write a joint's orientation as two
   Euler triples far apart from one frame to the next (x and z half a turn on, y mirrored). The
   game shows only its own frames; the port's 60 Hz in-between frames blended each angle and
   swung the limb through a wrong pose. Now `eng_anim::JointTable::lerp` blends a joint whose
   angles move more than 0x2000 as a rotation (quaternion slerp, `Matrix_RotateZYX`'s order),
   and a cue that puts Link at its start (`func_808529D0`) isn't blended across.

Tests:
- `oot_actors --test polish`: `the_first_text_shows_over_the_white_out` (the text's characters
  in the overlay with the grey fill under them) and `the_camera_loads_the_village_as_it_flies_in`;
- `eng_anim`'s `a_flipped_joint_stays_on_its_orientation`.

313 tests pass, 1 ignored. The goldens: `new_file_deku_tree`'s trace
re-recorded (rooms and actors during Navi's flight); the three Link viewer sheets re-recorded
(the viewer samples between animation frames with the same blend).

The user's second look at the opening found two more, both fixed:

4. **No Kokiri children during Navi's flight** (their fairies were there). In Kokiri Forest
   `En_Ko` fades a child by its distance to Link (`func_80A98DB4`), but in a cutscene by a
   quarter of its distance to the view's eye; Link stays in the meadow, so they were faded out.
   The port now takes the cutscene case. It does the same for the children's head tracking
   (`func_80A9877C`: the view's eye, 40 up) and Mido's (`EnMd`'s `func_80AAB158`).
5. **No sound as Navi hits the fence.** The opening's Navi sounds belong to `Object_Kankyo`
   (params 0), the placed actor for Kokiri Forest's fairy dust, which was a placeholder. It is
   ported (`oot_actors::object_kankyo`):
   - in layer 7, the wing hum by the camera's speed (`func_800F436C`), her calls (frames 473,
     583), the crash (763) and her cry (771, `NA_SE_VO_RT_THROW`);
   - the dust: up to 64 motes drifting and blinking in front of the camera, the first 32
     circling Link once he stands still for a while, drawn on the billboard (a bake of
     `gKokiriDustMoteMaterialDL` and `gKokiriDustMoteModelDL` with `gSun1Tex`), with every
     `Rand_ZeroOne` call the C makes.

   Its other kinds (the lightning, the snow, the grave's spark, Ganon's castle's beams) aren't
   ported.

Tests:
- `oot_actors --test polish`: `navis_flight_has_its_sounds`, the four sounds on their frames
  and the hum every frame;
- `oot_actors --test scenes`: updated, since `Object_Kankyo` takes room -1 (it outlives room
  changes, so room 1's own kills itself: `sIsSpawned`).

The motes' `Rand` calls changed the random draws in Kokiri Forest. The scripted Deku Tree
route's bushes now give their green rupee in another order: (594, 542), (385, 643), (572, 603),
(678, 596).

314 tests pass, 1 ignored. The goldens: the six Kokiri Forest runs' traces and
`spot04_treemouth_open` (the motes) re-recorded (golden/README.md).
