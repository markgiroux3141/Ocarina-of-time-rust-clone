# 0029: One-point cutscenes: the tables in the pack and their statics on the play state; the cameras' queue on the cameras; what a camera function does to play as requests; an actor passes itself

- **Status:** accepted, built in GAME-04b milestone 1; extends ADR 0015 (camera settings) and ADR 0022 (sub cameras)
- **Date:** 2026-10-01

## Context

Actors and Player start short camera shots of their own, the one-point cutscenes
(`z_onepointdemo.c`): the crawlspace's exits (9601, 9602), a chest falling (4500), a switch or a
torch drawing attention to what it opened (`OnePointCutscene_Attention`, 5010), and in the Deku
Tree `Bg_Ydan_Hasi` (3040), `Bg_Ydan_Maruta` (3010), `Bg_Ydan_Sp` (3020), `Door_Shutter`,
`Obj_Switch` and `Obj_Syokudai` (attention). Without them the crawlspace's exit handed the view
from `Camera_Subj4` straight to the next floor's camera, which pitched up and jittered
(BACKLOG #3).

**How the C does it.** `OnePointCutscene_Init` makes a sub camera (`Play_CreateSubCamera`),
puts it in a queue in front of the camera it interrupts (`parentCamId`, `childCamId`; the
interrupted main camera goes to `CAM_STAT_UNK3`, still updated but not the view), and
`OnePointCutscene_SetInfo` picks the camera's setting and data by `csId`:
- `CAM_SET_CS_C` (`Camera_Unique9`): a list of `OnePointCsFull` keyframes, each with targets for
  the at and the eye (fixed, from the view, from the camera, round the target or an actor's
  focus), a fov and roll, and how to move to them over its timer;
- `CAM_SET_CS_3` (`Camera_Demo9`): two `CutsceneCameraPoint` splines, relative to Player or the
  target, for a number of frames, then a finishing action (copy the camera to the main one, or
  start 1020, the return to Link);
- `CAM_SET_CS_ATTENTION` (`Camera_Demo5`): picks one of eight keyframe lists by where the target
  is, plays the attention chime, holds Player, then becomes `CAM_SET_CS_C`;
- `CAM_SET_FREE2` (`Camera_Unique6`): a fixed shot.

When a camera's timer reaches 0, `Camera_Finish` (the end of `Play_Draw`) makes its parent active
again, takes it out of the queue and clears it; back at the main camera, Player's cutscene mode
ends. The keyframe and spline tables (`z_onepointdemo_data.c`, and `Camera_Demo5`'s in
`z_camera_data.c`) are statics in `code`: `SetInfo` and `Camera_Demo5` write into them (a
case's timer, targets from the view, random offsets), and the cameras read them through
pointers each frame. The camera functions also reach outside their camera: Player's cutscene
mode (`func_8002DF38`), his position, another camera (`Camera_Copy`, `Camera_ChangeModeFlags`),
a new one-point cutscene, a sound.

## Decision

- **The tables are in the pack** (`CameraData::onepoint`, pack format 15): every
  `OnePointCsFull` array (two-dimensional ones flattened), `CutsceneCameraPoint` array and `s16`
  of `z_onepointdemo_data.c`, by declaration, and `Camera_Demo5`'s eight arrays, read from the
  C by the importer and checked byte for byte against `code` in the ROM (each `D_` symbol at its
  address's offset from one found by its bytes).
- **The play state keeps them as statics** (`crate::onepoint::OnePointStatics`, with
  `sDisableAttention`, `sPrevFrameCs1100`, `D_8011D3AC` and `Camera_Demo5`'s frame counters),
  carried over scene changes as the code segment's are. A keyframe pointer is a
  `KeyFramesRef` (table, row), read when the C reads it; a spline pointer, the list's index.
- **The queue lives on the cameras:** `GameCamera` has `parent_cam_id`, `child_cam_id`,
  `cs_id` (0x7FFF from `Camera_Init`), `data1` (the attention sound), `cs_info`
  (`OnePointCsInfo`) and `one_point_cam_data`; `PlayState` ports `OnePointCutscene_Init`,
  `_SetInfo`, `_SetAsChild`, `_RemoveCamera`, `_EndCutscene`, `_Attention`,
  `_AttentionSetSfx`, `_CheckForCategory`, `func_800C0808` (with `Camera_InitPlayerSettings`
  for a sub camera), `func_800C08AC`, `Play_SetCameraRoll` and `Camera_Finish`, called on the
  active camera at the end of the frame as `Play_Draw` does.
- **`play->view` is tracked** (`PlayState::view`: the eye, at and fov an active camera's update
  last set, `View_LookAt`), since one-point cutscenes start from it.
- **A camera function's effects on the rest of play are requests** (`CamRequest`: Player's mode,
  his position, his freeze, a copy to another camera, another camera's mode, a new one-point
  cutscene), applied by the play state right after that camera's update, in order, as ADR 0016
  does for Player. Its sound joins the camera's others (`CamSfx::Sfx`).
- **What the camera reads of an actor is a `CamActor`** (its category, whether it's alive, its
  focus, world and shape, its screen position, a door's front yaw), built for the camera's own
  target each update. An actor starting a one-point cutscene from its own update is out of the
  arena (ADR 0007), so it passes itself (`PlayState::cam_actor_of`).
- **`SetInfo`'s cases are ported as the played scenes and Phase 6's Deku Tree need them**: 1000,
  1010, 1020, 1030, 1100, 3010, 3020, 3040, 3140, 4500, 4510, 5000, 5010, 5110, 5120, 9500,
  9601, 9602, 9806, 9908. The others (other scenes' actors, most with quakes) are logged as the
  C logs an unknown number; their tables are in the pack already.
- **`Camera_Unique9`, `Camera_Demo9` and `Camera_Demo5` are ported whole.**

## Consequences

- The crawlspace's exit is the game's: the spline up and out in front of Link, then the main
  camera starting from the cutscene's last view (`Camera_Copy`), with no jump.
- Faithful details the port reproduces: while Link is still on the crawlspace's floor, the
  waiting main camera takes the crawlspace's bg camera back the frame after 9601 left it, and its
  `Camera_Subj4` asks for `view.unk_124`, so `Play_Draw` updates the active camera, the
  cutscene's, a second time: the spline's 92 updates take 67 frames.
- The scripted runs through the crawlspace changed after its exits (golden/README.md): the
  camera's yaw steers the routes.
- Player's cutscene modes the attention camera asks for when the target is Player himself (12,
  69) and 3050's (5) aren't ported yet (`Door_Shutter` on Player, Phase 6); they're logged.
- Quakes (`Quake_Add`) aren't ported; the cases that need them aren't either.
