# 0013: Camera modes by the imported function, the letterbox and the screen overlay

- **Status:** accepted, built in GAME-02 milestone 1
- **Date:** 2026-09-28

## Context

Until now the camera ran `Camera_Normal1` for the NORMAL mode of `CAM_SET_NORMAL0`, and nothing else. Z-targeting polish needs three things from the game:
- **The mode switch.** Player asks for a mode every frame (`Player_UpdateCamAndSeqModes`), and `Camera_ChangeModeFlags` accepts it, refuses it, or resets the mode function.
- **The mode functions:** `Camera_Parallel1` for Z with nothing targeted, `Camera_KeepOn1` for a friendly target.
- **The screen state they drive:** the letterbox bars and the reticle.

Four things had to be decided:
1. how the camera finds a mode's function and data;
2. what an unported mode does;
3. where the bars and the 2D reticle live between the game and the engine;
4. at what rate the reticle updates, when the renderer blends between game frames.

## Decision

- **Modes are data from the C.**
  - The importer reads `sCamSetNormal0Modes`: for each `CAM_MODE_*`, its `CAM_FUNC_*` name, its data symbol, and its `CAM_FUNCDATA_*` arguments in order. It also reads NORMAL0's valid-mode mask from `sCameraSettings`.
  - `GameCamera::update` dispatches on the function name: `CAM_FUNC_NORM1`, `CAM_FUNC_PARA1` and `CAM_FUNC_KEEP1` are ported.
  - Each function reloads its read-only data from the current mode's values, as `RELOAD_PARAMS` does. So STILL (a Normal1 mode) and PUSHPULL (a Parallel1 mode) get their own data for free.
- **An unported mode runs `Camera_Normal1` on NORMAL's data**, but `camera->mode`, `animState` and the refusal rules still change as in the game.
  - This is what the camera did before for every mode. The difference is that a mode change resets the function, as the game's does.
  - The modes affected are BATTLE (`Camera_Battle1`), TALK (`Camera_KeepOn3`), JUMP / FREEFALL / CLIMB / CLIMBZ (`Jump1`, `Jump2`), HANG (`Uniq1`) and first person (`Subj3`).
- **`PREG` registers are 0.** `Camera_CopyDataToRegs` fills `PREG`s only for the debug register editor. `PREG(75)` and `PREG(76)` are 0 in the retail flow, so the at-calculations take those branches.
- **The letterbox is game state, drawn by the engine.**
  - `oot_game::letterbox::Letterbox` is `shrink_window.c`. `Letterbox_Update` runs in `PlayState::tick_with` before the camera, and `Camera_UpdateInterface` sets its target from `sCameraInterfaceFlags`.
  - The render frame carries the size, blended between game frames like the view.
  - `eng_gfx::DrawLists::letterbox_rows` asks the renderer for black bars over the 3D lists and under the overlay. That looks the same as `View_ApplyLetterbox`'s scissor over a black-filled frame.
- **2D overlay draws are an engine list with an orthographic projection.**
  - `DrawLists::overlay_2d` is `OVERLAY_DISP` after `View_ApplyOrthoToOverlay`: the 320x240 screen centred on 0, y up. The meshes are ordinary baked meshes (ADR 0012).
  - On a target wider than 4:3, x extends by the aspect ratio. So a point computed with the game's 4:3 view projection stays over the same spot of the 3D view, which is drawn at the target's aspect.
- **The reticle updates at the game's rate.**
  - `func_8002C124` both moves the reticle (the trail entries, the fade) and draws it. The moving half (`target::draw_update`) runs once per game frame, after the frame's view is set up, as `Interface_Draw` does. The drawing half (`target::draw`) only reads that state.
  - So the reticle moves at 20 Hz as on the N64, while the world around it is blended at the display rate.

## Consequences

- Porting another mode function is a new arm in the dispatch plus its read-only data layout. The data is already in the pack for all 21 NORMAL0 modes.
- **Locking on to an enemy shows no bars and no battle framing** until `Camera_Battle1` is ported (Kokiri Forest has no enemies). Jumps, falls, climbs and hangs frame like the Normal camera, with a reset each time the mode changes.
- **The camera traces changed** wherever Link jumps, falls, climbs, slashes or Z-targets: 28 golden hashes, recorded in `golden/README.md`.
- The overlay list and the bars are the first screen-space drawing. The HUD (`Interface_Draw`) will draw into the same list.
