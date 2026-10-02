# 0022: Cutscenes: the scripts as the ROM's bytes, walked as `Cutscene_ProcessScript` walks them; the overlays' scripts found in the ROM from their C; `z_demo.c` ported onto the play state; sub cameras and the camera's shared state; Player's cutscene modes as one action

- **Status:** accepted, built in GAME-03 milestone 4; extends ADR 0008 (the pack), ADR 0015 (camera settings), ADR 0018 (the Deku Tree's mouth) and ADR 0020 (routes of steering tasks)
- **Date:** 2026-09-29

## Context

On a new save, the Deku Tree's mouth opens only through his talk, a chain of cutscene scripts
that `Bg_Treemouth` starts (`gDekuTreeMeetingCs`, then `gDekuTreeMouthOpeningCs` on yes or `gDekuTreeAskAgainCs` on no). ADR
0018 ported the mouth's side against an idle `csCtx`, and presets stood in for the flags.

**Where the scripts are.** A script is an array of `CutsceneData` words
(`cutscene_commands.h`): the number of commands and the last frame, then commands, each a
type word and its entries, up to `CS_END_OF_SCRIPT`. They live in two places:
- **scene files:** the decomp's XMLs name 73 of them (`<Cutscene>`), by offset, with no length.
  Kokiri Forest has two (the adult's Deku Sprout, `gKokiriForestDekuSproutPart3Cs`, and
  `gKokiriForestSariaGreetingCs`); the Deku Tree's scene has its intro (`gDekuTreeIntroCs`);
- **actor overlays' data:** 27 arrays in `*_cutscene_data*.c`, `Bg_Treemouth`'s four among them.
  No XML gives their place in the overlay.

`sEntranceCutsceneTable` (`z_demo.c`) names each entrance's script by symbol.

**How the C reads them.** `Cutscene_ProcessScript` steps through the words with `MemCopy`
and pointer arithmetic. Each command's size depends on its type: 0x30-byte entries for most, 0xC
for the text, time and rumble commands, a header and 0x10-byte points up to one flagged
`CS_CAM_STOP` for the camera lists, a fixed 12 bytes for the terminator and the transition.
Unknown types are skipped as 0x30-byte lists. The context keeps pointers into the script: the
cues in effect (`linkAction`, `npcActions[10]`) and the camera's two point lists, which the
camera then reads each frame.

**The cutscene camera is a sub camera.** With `gUseCutsceneCam` set, `Cutscene_SetupScripted` creates a sub
camera (`Play_CreateSubCamera`, `Camera_Init`), and once both an eye and an at list have been
seen, the script puts it on `CAM_SET_CS_0` (`Camera_Demo1`, the B-spline of `z_cutscene_spline.c`),
makes it active and sets the main camera waiting. At the end the camera that was active comes
back and the sub camera is cleared. The decomp at this commit names `gUseCutsceneCam` only in
`z_demo.c`'s debug D-pad replays and `db_camera.c`, both of which clear it or set it on a
button. A scan of the ROM for every store to it found one more: `Environment_Init` sets it to 1
(`z_kankyo.c:419`) on every `Play_Init`. (Later decomps call it `gUseCutsceneCam`.)

The port had one camera, the main one, and kept `z_camera.c`'s file-scope state
(`sCameraInterfaceField`, `sCameraHudVisibilityMode`, `sOOBTimer`) inside it. `Camera_Init` resets
that shared state, including `sSceneInitLetterboxTimer`, which holds the main camera's interface at 0x3200 for
its next three active updates.

**Player's side** is a mode (`csMode`, set by `Player_SetCsActionWithHaltedActors`) and one action,
`Player_Action_CsAction`, which runs each mode's start (`D_80854B18`) and update (`D_80854E50`), about
100 of them. A running script puts Player in mode 6, following its `linkAction` cues
(`sCueToCsActionMap` maps a cue to a mode), or in mode 0x31, held still. The action takes over through
the interrupts: `Player_ActionHandler_0` (interrupt 0) and `Player_ActionHandler_13` (13) call `Player_StartCsAction`.
`En_Wonder_Talk2`'s forced texts use modes 8 and 7 the same way.

## Decision

- **The pack holds each script as the ROM's bytes** (`cutscene::CutsceneScript`, keyed
  `cutscene/<file>/<symbol>`), big-endian as the N64 reads them, from its entry count through
  `CS_END_OF_SCRIPT`. `table/cutscenes` (`CutsceneTables`) lists every script's key by symbol and holds
  `sEntranceCutsceneTable` (the entrance's index, the age, the flag, the script). A scene layer's
  `SCENE_CMD_ID_CUTSCENE_DATA` is `LayerData.cutscene`, the key of the XML symbol at that offset.
  Pack format 11.
  - Keeping the bytes, not a parsed form, keeps the C's stepping exact: the entry sizes, the
    `totalEntries` count, the unknown commands skipped by their sizes, the struct reads (a cue's
    `normal` is the script's float bits read as the `Vec3i` the struct declares).
  - The runtime walks them as `Cutscene_ProcessScript` does (`cutscene::walk`,
    `PlayState::cutscene_process_commands`). A byte offset stands for each pointer; `CsPtr`
    keeps the script it points into alive, as the C's pointers stay valid after `Bg_Treemouth`
    swaps `csCtx.segment`. The cues are copies of the script's entries.
- **The importer finds the scripts without the runtime ever reading C:**
  - scene scripts: from the XML's offset, walked to `CS_END_OF_SCRIPT` with the runtime's own walk
    (`oot_import::cutscene::scene_script`); a script that doesn't end is an import error;
  - overlay scripts: each `CutsceneData` array's words are built from its C with
    `cutscene_commands.h`'s own macro definitions, the `CutsceneCmd` and
    `CutsceneDestination` enums and `command_macros_base.h`'s packing
    (`oot_import::cutscene::Macros`); those words are then looked for in the overlay's file in
    the ROM, and the ROM's bytes are stored. An array not found is an import error. All 27 are
    found;
  - `sEntranceCutsceneTable` is read from `z_demo.c`, its flags from `save.h`.
- **`z_demo.c` is ported onto the play state** (`impl PlayState` in `oot_game::cutscene`), with
  the C's names: `Cutscene_InitContext`, `Cutscene_UpdateManual`, `Cutscene_UpdateScripted` and both state tables, the
  misc commands, the lighting, time, music, rumble and terminator commands, the transition fill,
  the camera commands 1, 2, 5 to 8, the text command, `CutsceneHandler_RunScript`, `CutsceneHandler_StopManual`,
  `CutsceneHandler_StopScript`, `Cutscene_SetupScripted`, `Cutscene_HandleEntranceTriggers`,
  `Cutscene_HandleConditionalTriggers` and `Cutscene_SetScript`. The play frame calls
  `Cutscene_UpdateManual` and `Cutscene_UpdateScripted` after `Actor_UpdateAll`, as `Play_Update` does.
  `Play_Init` runs `nextCutsceneIndex`, the conditional triggers, `Environment_Init`'s
  `gUseCutsceneCam = 1` and the entrance triggers. The file's statics (`DemoStatics`) carry over from
  one play state to the next, as the code segment's do; `play->cutsceneFlags` is `env_flags`.
  - The terminator's destinations (1 to 119): the ones that only start a transition are a table
    transcribed from the C in its order; the others (items given, flags, the credits, the title
    screen's chain, the barrier) are ported as arms.
  - Logged when their first frame comes, not ported: the weather, the lights and fog, the
    skybox change, quakes, title cards, the sandstorm, the Sun's Song, the screen tint, the
    ocarina texts (`Message_StartOcarina`), `linkAgeOnLoad`, the music and the rumble.
  - A cutscene index of 0xFFF0 and up asks `Play_Init` for a scene's cutscene layer, which the
    pack doesn't hold (layers 0 to 3 only): it's logged as an error and the normal layer loads.
- **Cameras: sub cameras, and the state they share.**
  - `PlayState` has three sub camera slots beside the main camera (`cameraPtrs`),
    `activeCamId` and `nextCamId`, and the `Play_*Camera*` helpers (`Play_CreateSubCamera`,
    `Play_ChangeCameraStatus`, `Play_ClearCamera`, `Play_RequestCameraSetting`,
    `Play_SetCameraAtEye`, `Play_SetCameraFov`, `Play_CopyCamera`). Every camera updates each
    frame, the active one last, and the view is the active camera's.
  - `Camera_Update` follows the C's statuses: a cut camera returns at once, a waiting one after
    its player part (so the main camera still follows the floor's bg cameras), and only an active
    one sets the interface.
  - `z_camera.c`'s shared state is `CameraGlobals`: `sCameraInterfaceField`,
    `sCameraHudVisibilityMode`, `sSceneInitLetterboxTimer`, `sOOBTimer`, `sNextUID`. A sub camera's `Camera_Init`
    resets it, and `Play_Init` does for the main one. Each camera works on its own copy during
    its update, and the copy goes back afterwards.
  - `Camera_Demo1`, `Camera_SetCSParams`, `Camera_ResetAnim`, `Camera_SetViewParam` (at, eye, fov,
    roll), `Camera_Copy` and the spline (`func_800BB0A0`, `func_800BB2B4`) are ported. The eye's
    and the at's splines share one keyframe counter, as in the C.
- **Player's cutscene modes are one action** (`Action::Cutscene`, `Player_Action_CsAction`), with
  `func_80852B4C`'s dispatch of the two tables. The modes Kokiri Forest and the Deku Tree use
  are ported: 1, 3 and 4 (a cue's walk, `func_80845964` with its cue), 6 (`func_80852C50`,
  following `linkAction`), 7 (`func_80852944`, the end), 8 and 0x31 (held). The others are
  logged. `Player_UpdateCommon`'s cutscene block, `Player_ActionHandler_0`'s and `Player_ActionHandler_13`'s
  `unk_6AD` path and `Player_StartCsAction` are ported; `Player_ActionHandler_0`'s C-Up into first person isn't.
  Other actors set the mode through `PlayState::Player_SetCsActionWithHaltedActors` (`PlayerIface::set_cs_mode`).
- **The mouth's scripts are real:** `Bg_Treemouth` sets `csCtx.segment` and `cutsceneTrigger`
  where the C does. The presets stay for the shortcuts and the Deku Tree run; the new route
  (`Route::NewSaveDekuTree`) needs none.

## Consequences

- **A new save gets into the Deku Tree the game's way:** his first talk plays by itself in his
  meadow, yes opens his mouth, and his scene's intro plays the first time in. The scripted run
  from Link's bed does it in 7576 frames.
- **Every scene's start shows the letterbox for a moment:** `sSceneInitLetterboxTimer`'s three frames at
  0x3200 grow the bars towards 32 during the fade-in, and the HUD's alpha type 2 is then held
  through the transition (0xF200), so it fades back in after it. This is the C's behaviour; 21
  golden sheets changed for it.
- **Forced texts hold Link** (mode 8), and the scripted walks no longer stop for them: they press
  A while he's held.
- **Scripts from every scene are in the pack,** but only what Kokiri Forest and the Deku Tree's
  entrance use is exercised. Cutscene layers (Navi's wake-up on a new file, the intro cutscene
  layers the terminators ask for) need the pack to hold layers 4 and up: GAME-03 milestone 5.
- **The camera can be switched from outside it.** One-point cutscenes (`OnePointCutscene_Init`,
  the crawl's 9601 and 9602) can now be built on the sub cameras; they aren't yet.
- **The pack grows by the scripts:** 52.0 MB, format 11.
