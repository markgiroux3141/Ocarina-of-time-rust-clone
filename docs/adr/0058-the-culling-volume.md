# 0058: The culling volume, and `Play_Draw`'s view taken where `View_Apply` takes it

- **Status:** accepted, built in GAME-07 milestone 1 (2026-10-11)
- **Date:** 2026-10-11
- **Builds on:** [ADR 0057](0057-hyrule-fields-actors.md) (which left the culling volume out),
  [ADR 0013](0013-camera-modes-and-screen.md) (the game's 4:3 view).

## Context

`Actor_DrawAll` (`z_actor.c`) projects each actor's position through `play->viewProjectionMtxF`
(`projectedPos`, `projectedW`) and tests it against the actor's culling volume
(`Actor_CullingVolumeTest`: a frustum from the eye to `cullingVolumeDistance`, a box behind the
eye, widened by `cullingVolumeScale` and reaching `cullingVolumeDownward` below). The result is
`ACTOR_FLAG_INSIDE_CULLING_VOLUME`:

- `Actor_UpdateAll` updates an actor only with that flag or `ACTOR_FLAG_UPDATE_CULLING_DISABLED`;
- `Actor_DrawAll` draws one only with it or `ACTOR_FLAG_DRAW_CULLING_DISABLED`, and notes
  `isDrawn`.

The port had none of it (ADR 0057): every actor updated and drew wherever it was. Phase 8's market
is the first crowd, and Hyrule Field's spawned trees never left (BACKLOG #32).

Three things came up while porting it:

- **The init chains.** 27 ported actors set their culling volume in their init chain or init
  (`ICHAIN_F32(cullingVolumeDistance, ...)`) and the port had left those out ("not ported").
- **Where the view is taken.** The port projected the actors (and set `PlayState::view_proj`, which
  the attention system and the one-point cutscenes read) at the end of its frame, after `Play_Draw`'s
  second `Camera_Update` (`view.unk_124`) and `Camera_Finish`. `Play_Draw` takes
  `viewProjectionMtxF` at `View_Apply` (z_play.c 1208), before `Actor_DrawAll` (1366), the second
  update (1436) and `Camera_Finish` (1445); and it takes it every frame, the pause background
  saved or not.
- **`zFar`.** `View_SetPerspective` uses `lightCtx.zFar` (the light settings' `zFar`, at most
  12,800); the port used 12,800 always. The projected z, which the volume's depth test compares, moves
  with it.

## Decision

- **The culling test where `Actor_DrawAll` makes it** (`oot_game::play`): before the actors'
  draw-time state, each actor's projection (`actor_draw_all_cull`), `Actor_CullingVolumeTest`
  (`actor_culling_volume_test`) and the flag; `Actor::is_drawn` for an initialised, unkilled actor in
  its volume or with `ACTOR_FLAG_DRAW_CULLING_DISABLED`. Only drawn actors run their `draw_update` and
  `draw_sfx`, and only they go into the frame's render state (`capture`), so the renderer draws what
  `Actor_DrawAll` draws. `Actor_UpdateAll` reads the flag as the last draw left it: an actor spawned
  this frame waits for its first test (as in the C, where `Actor_Spawn` copies the profile's flags).
- **Every ported actor's culling volume from its C**, constants cited at the init chain, with the
  inits' and updates' own writes (`En_Ishi` and `En_Kusa` a thousand further in a cutscene, the
  Jabu-Jabu shutter's scale, `En_Viewer`'s type 7, the default owl's distance, `Bg_Spot09_Obj`'s two
  chains).
- **`viewProjectionMtxF` at `View_Apply`'s place**: `PlayState::view_proj` set once a frame, before
  the draw's actor pass, with `zFar` from the scene's lights (`camera_view_proj`), whatever the
  pause background's state; the end of the frame no longer re-takes it.
- **Tests follow the C**: where a test's actor stood out of view, Link is placed facing it (as he
  would be playing); tests about something else (the update order, a Keese's dive followed past the
  camera) give their actor `ACTOR_FLAG_UPDATE_CULLING_DISABLED` and say so. The Combat route fights
  the Keese before the Deku Baba: fought under its perch, the Deku Baba set it diving, and it pulled
  up over Link out of view and held still there.

## Consequences

- Actors out of view hold still, as on the console: a Keese that dived over Link's head waits there
  until he turns; a rock circle behind the camera keeps its rocks in; distant townsfolk and signs
  aren't drawn.
- 36 golden hashes changed, each proved in three builds (golden/README.md): the culling taken out
  with the old timing gives back 145, and `combat`'s with its old order; the view's order and `zFar`
  alone change 10; the culling the rest.
- `ACTOR_FLAG_INSIDE_CULLING_VOLUME` is now real for `En_Wood02` and `En_Peehat`, which read it
  (BACKLOG #32 closed).
- **Not modelled:** the Lens of Truth's actors (`ACTOR_FLAG_REACT_TO_LENS`, drawn in their own pass),
  and `Actor_Delete`'s wait for an actor still drawn (`isDrawn`): a killed actor goes at once, as
  before.
