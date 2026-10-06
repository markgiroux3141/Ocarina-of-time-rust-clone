# 0039: Quakes

- **Status:** accepted, built in GAME-05 milestone 4a (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0013](0013-camera-modes-and-screen.md) and
  [ADR 0015](0015-camera-settings-and-bg-cameras.md) (the camera),
  [ADR 0016](0016-player-requests.md) (Player's requests) and
  [ADR 0029](0029-one-point-cutscenes.md) (the one-point cutscenes' statics).

## Context

The camera's shakes (`z_quake.c`, 509 lines) weren't ported: ADR 0029 left out the one-point
cases that need them, and every caller logged them. Milestone 4a's `Door_Shutter` shakes the
camera when it slams, and 4b's `Obj_Lift` when it gives way. Player already had two callers
(the roll into a wall, a damaging fall).

`z_quake.c` keeps a table of four requests in `code`'s `.bss`. Each request has its camera (a
`Camera*`), a type (1 to 6), speed, perturbations, a countdown and an orientation. `Camera_Update`
runs `Quake_Update` with the camera, which sums the requests for that camera into a `ShakeInfo`.
The camera adds the shake to the view it gives `View_LookAt` (at, eye, fov, the up vector's
roll) and keeps it as `quakeOffset`, which some actors' draws read.

## Decision

- **`z_quake.c` is ported whole** (`oot_game::quake`): the request table as `QuakeStatics` on the
  play state, carried over a scene change as a code static and cleared by `Play_Init`'s
  `Quake_Init`; every callback, `Quake_Request`, `Quake_SetSpeed`, `_SetPerturbations`,
  `_SetDuration`, `_SetOrientation`, `_GetTimeLeft`, `_RemoveRequest`, `Quake_Update`. Its `Rand`
  calls are made in the C's order on the game's generator. Faithful bugs kept: a request
  replacing another still counts up; `Quake_SetValue` has no return value; a request with a
  negative timer is never removed.
- **A request keeps its camera's id** in place of the `Camera*`: a sub camera's struct keeps its id
  after `Play_ClearCamera`, so the two name the same camera.
- **`Camera_Update` gets every camera's eye and at** (`CamFrame::cameras`), since a request's shake
  is computed from its own camera's view, and adds the shake to its view where the C does, but on
  `CAM_SET_TURN_AROUND`. The shake is kept as offsets on the camera (`view_offset`,
  `view_fov_offset`, reset each update); `GameCamera::shaken_view()` is `play->view` for the
  view-projection and the render; `quake_offset` is kept for the draws.
- **The API actors call** is on the play state, after the C: `quake_request(cam_id, type)`, then
  `quake_set_speed`, `quake_set_perturbations`, `quake_set_duration`; `CAM_ID_NONE` stands for
  `GET_ACTIVE_CAM`. Player's two go through `PlayRequest::Quake`. `Actor_RequestQuake` and its
  variants are ported (rumble only logged).
- **Callers wired:** `Door_Shutter` (milestone 4a), Player's roll bonk and damaging fall,
  `En_Goroiwa`'s drop, the cutscene commands `CS_MISC_QUAKE_START` and `_STOP`. `En_Goma`'s larva
  and `En_Firefly` read `quakeOffset` in their draws (`ACTOR_FLAG_IGNORE_QUAKE`). No ported
  one-point case needed a quake (all 21 in `z_onepointdemo.c` belong to actors not in play).

## Consequences

- The camera shakes on the roll into a wall and on a damaging fall: three course sheets in the
  goldens change, not their traces (the eye and at move together, so the camera's direction
  doesn't).
- The roll part of a shake isn't drawn: the renderer keeps Y up (a camera roll isn't drawn
  anywhere yet). Nor is the shake applied to the prerendered rooms' backgrounds or the skybox.
- An actor with `ACTOR_FLAG_IGNORE_QUAKE` applies `quakeOffset` in its own draw matrix; a future
  one must do the same.
