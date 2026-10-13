# 0063: The guards' courtyard, Zelda and Impa

- **Status:** accepted, built in GAME-07 milestone 4 (2026-10-13)
- **Date:** 2026-10-13
- **Builds on:** [ADR 0060](0060-hyrule-castles-grounds.md) (the sentries' catch and the circle
  wipe), [ADR 0061](0061-the-ocarina.md) (the cutscenes' ocarina texts, the playback modes),
  [ADR 0051](0051-textures-replaced-by-source-and-the-object-ram.md) (a draw's own images by texture source),
  [ADR 0033](0033-effects.md) (the effects' table), [ADR 0062](0062-the-aim-master-quest-and-the-game-editor.md)
  (each actor's Params and Flags).

## Context

Milestone 4 takes child Link from the castle's drain to Zelda and her lullaby:

- **The guards' courtyard** (`hairal_niwa`): a 3D garden seen side-on by `Camera_Special6`
  (`CAM_SET_PREREND_SIDE_SCROLL`, nine bg cameras, one per "screen"). Eight patrolling guards
  (`En_Heishi1`) look along a line: each frame a search ball (`Effect_Ss_Solder_Srch_Ball`) flies
  from their head at 30 a frame, and if it reaches Link within 70 before a sphere of 30 at it
  touches a poly, it sets the guard's `linkDetected` through a pointer the effect keeps in its
  `actor` field. A guard looks only while the active camera shows his screen. The path 3 guard
  spawns eight rupees (`En_Ex_Ruppy` type 3).
- **Zelda** (`En_Zl4`, 1,527 lines): her meeting is her own state machine (`EnZl4_Cutscene`), not
  a script: she moves the active camera herself (`sCamDirections` on `CAM_SET_FREE0`, `sCamMove` on
  `CAM_SET_CS_0`), starts scene scripts (`gZeldasCourtyardMeetCs`, `WindowCs`, `GanonCs`) and holds
  them at frames, sends Link to the cutscene map's legend and back, and gives the letter through
  `play->talkWithPlayer` with her text -1. Her walking animations carry her root's motion
  (`AnimTaskQueue_AddActorMovement`). Her face is two eyes of seven textures and a mouth of four.
- **Impa** (`Demo_Im`): her lullaby in Zelda's courtyard (type 5) and her goodbye on Hyrule Field's
  layer 12 (type 4), cue-driven; types 0 to 2 and 6 belong to the Chamber of Sages and the ending.
- **The draw configs** `SDC_HYRULE_CASTLE`, `SDC_CASTLE_COURTYARD_GUARDS`, `SDC_CALM_WATER`,
  `SDC_TEMPLE_OF_TIME` (BACKLOG #46's castle part).

## Decision

1. **The camera's write to Player.** `Camera_Special6` sets `camera->player->actor.freezeTimer`
   at a screen change. As `Camera_Subj4`'s position write (ADR 0020), the camera records it
   (`GameCamera::player_freeze`) and the play state applies it to Player right after the camera's
   update. While Player is frozen the guards don't update (`EnHeishi1_Update`'s first check).
2. **The search ball's write to its guard.** An effect can't hold a pointer into an actor, so the
   effect keeps the guard's handle in `actor` (where the C keeps the pointer) and calls
   `ActorImpl::solder_srch_ball_detect` on it, which only `En_Heishi1` implements (setting
   `linkDetected`). The effects update after the actors, as `EffectSs_UpdateAll` does, so the guard
   reads it on his next update as in the C. `En_Ex_Ruppy`'s unused bomb types write their parent's
   `linkDetected` the same way.
3. **`En_Heishi1`'s profile id is 0**, as the decomp's `En_Heishi1_Profile` has it (`/**/ 0,`): the
   guards' `actor.id` is Player's, as in the game. Type 5's `sAnimParamsInit` row is read past the
   table's end (`@bug (game)`); the overrun's values are the overlay's following data, and nothing
   but the init's prints reads them.
4. **Root motion for any actor.** `SkelAnimeStd` gains `SkelAnime_UpdateTranslation` (with
   `movementFlags`, `baseTransl`, `prevTransl`, `prevRot`); an actor queues its move in its update
   and applies it in `ActorImpl::animation_update`, the port's `AnimTaskQueue_Update` share
   (`AnimTask_ActorMovement`).
5. **Faces by texture source.** Zelda's and Impa's skeletons are baked once, each face segment bound
   to a texture of its own (Zelda's right eye on 8 to the open eye, her left on 9 to the blinking
   one, her mouth on 0xA), so their loads have distinct sources (`G_SETTIMG` at 0x08000000,
   0x09000000, 0x0A000000). A draw puts the state's texture (from the pack's decoded textures) in
   each place through `DrawParams::texture_images`. One bake instead of 196 (Zelda) or six (Impa:
   three eyes by two hands; the pointing hand stays a second bake).
6. **Zelda's camera tables in the source.** `sCamDirections` and `sCamMove`'s point arrays are the
   overlay's static data (`z_en_zl4_cutscene_data.inc.c`), generated into the Rust source as
   `Demo_Kankyo`'s were (GAME-06 milestone 1b), not imported into the pack.
7. **`Player_StartTalking` with text -1.** The port logged the branch; it now holds Link
   (`PLAYER_CSACTION_1`, the halting flag) and puts his item away, as the C does. Zelda's
   `talkWithPlayer` calls the same steps on Player from her update (`EnZl4::talk_with_player`).
8. **Impa's scope**: types 3, 4 and 5 and the shared code (the user's scope for the phase); the
   others log and stay hidden.
9. **The exit's run** (`Route::Zelda`): the sneak is waits at spots the C's rules keep unseen
   (a screen whose guards don't look, a fence or pillar between, a guard's back), each released when
   the next screen's guards are in phases of their rounds that look away (`Task::WaitGuards`'s
   conditions on their target waypoint, position and wait), and runs between them that stop if a
   step ahead would be seen (`Task::Sneak`: the guard's 70, or a ball marched the C's way from his
   head in his head's reach, or one in flight). Zelda's talk is `Task::Converse` (A at every box
   through scripts and scenes, the first answer unless told otherwise), the lullaby's playback
   `Task::Playback` (a wrong note first, then the song). A second route, `Route::ZeldaTalk`, starts
   at Zelda's courtyard for the tests.

## Consequences

- The guards' phases depend on their `Rand` calls (their waits, their middle points' skips, their
  heads' timers). A change in who calls `Rand` before the courtyard moves them; the run's conditions
  then still hold or wait longer, and a catch fails the run with where it happened.
- The custom lens flare the Temple of Time's config turns on for layer 5 is recorded
  (`DrawConfigState::custom_lens_flare`) but not drawn (`Environment_DrawCustomLensFlare` isn't
  ported).
- `En_Ex_Ruppy`'s diving game rupees wait for `En_Diving_Game`; its bombs for `Effect_Ss_Bomb2`.
