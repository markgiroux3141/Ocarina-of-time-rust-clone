# 0061: The ocarina: its audio, the message box's modes, Player's, the songs' effects

- **Status:** accepted, built in GAME-07 milestone 3 (2026-10-12)
- **Date:** 2026-10-12
- **Builds on:** [ADR 0060](0060-hyrule-castles-grounds.md) (Player's exchange items, the ocarina's
  use noted), [ADR 0056](0056-the-clock.md) (the clock, the Sun's Song's time), [ADR 0053] (the
  vertex sources `Demo_Effect` rebuilds), [ADR 0012](0012-actor-bakes.md) (bakes).

[ADR 0053]: 0053-demo-effect-curve-skeletons-and-vertices-by-source.md

## Context

The ocarina is the Fairy Ocarina on a C button: Link takes it out, the message box shows a staff,
and the notes he plays are heard and checked against the songs he knows. A song played has its
effect (a light, a whirl of leaves, a storm), and what listens acts on it: Navi when it's Saria's
Song, the clock for the Sun's Song, a fairy spot (`Shot_Sun`) for the Song of Storms, a song spot
(`En_Okarina_Tag`) for its own song. Teachers play a song and hear it back (Malon's Epona's Song,
the cutscenes' `CS_TEXT_OCARINA_ACTION` texts). It spans four parts of the C:

- **The audio**, `src/audio/game/general.c`'s `AudioOcarina_*` (about 1,000 lines): the pad read at
  `Audio_Update`, the note as a sound effect whose channel reads its pitch and instrument from IO
  ports, the songs' recognition (with and without the staff), the playback of a song for a lesson.
- **The message box**, `z_message.c`'s `Message_StartOcarinaImpl` and `Message_DrawMain`'s ocarina
  modes (about 40 `MSGMODE_*`s): the staff, the notes and their flash, free play, the checks, the
  lessons and the playbacks, "You played ...", the effect's spawn, `ocarinaMode`.
- **Player**: `Player_ActionHandler_13`'s ocarina branch, `Player_Action_8084E3C4` (the ocarina
  out), `func_8084B4D4` (a talk's ocarina), the listener protocol (`PLAYER_STATE2_23` to `_25`,
  `unk_6A8`), the warp songs' leave and arrival; `Actor_UpdateAll`'s freeze while it's played.
- **The actors**: `Oceff_Wipe`, `_Wipe2`, `_Wipe3`, `_Wipe4`, `_Spot`, `_Storm`,
  `En_Okarina_Effect`, `Shot_Sun`, `En_Okarina_Tag`, and the ported actors that listen.

The user's scope: everything whole except the scarecrows' recording, the Skull Kids' memory game
and the frogs (the plan, 2026-10-11). Their calls this milestone (2026-10-12): one milestone in
steps; `Environment_AdjustLights` wired for both `Oceff_Spot` and `En_Elf`; `Oceff_Wipe4` ported.

## Decision

- **The audio's statics in `GameAudio`** (`oot_game::audio::ocarina`): `AudioOcarina_*` as methods
  on it, the statics in `Ocarina`. The pad is read where the C reads it (`Audio_Update`, the newest
  state rather than the frame's presses), from `Ocarina::pad`, which the play state sets before
  its audio update. A note is `NA_SE_OC_OCARINA` on `SFX_CHANNEL_OCARINA` with the channel's IO
  ports set by `AUDIOCMD_CHANNEL_SET_IO` (the sequence player already ported reads them). The
  playback's timing counts the audio side's tasks (`gAudioCtx.totalTaskCount`), as the C does:
  so a run that plays a song needs the audio side, and a route says so (`Route::needs_audio`;
  the sandbox and the tests run `OfflineAudio` for it).
- **The song tables are the pack's**: `sOcarinaSongNotes`, `gOcarinaSongButtons`,
  `sButtonToPitchMap`, `sPitchToButtonMap` read from `general.c` by the importer into
  `AudioGameTables` (with `include/ocarina.h`'s macros), no game data in the repo. The route and
  the tests read the songs' buttons from there too.
- **The message box's ocarina modes whole** but the scarecrows', the memory game's and the frogs'
  (logged once, BACKLOG #47): `Message_StartOcarinaImpl`, `Message_HandleOcarina`,
  `Message_DrawMain`'s modes with the staff (the treble clef and the five note sprites as bakes
  of the message textures, `R_OCARINA_BUTTONS_*`' positions), the notes' flash and fall
  (`@bug (game)`: the A note's env colour stepped against the C notes'), the effect's spawn, and
  `Message_Update`'s closing (Navi's -0xE0 call after Saria's Song, `@bug (game)`: the song
  "stored"; free play's mode). What the box makes happen elsewhere goes back as `MsgRequest`s
  (`Spawn`, `NaviText`), applied by the play state after the box's frame.
- **Player's ocarina whole**: `Player_ActionHandler_13`'s branch (the turn-around camera's ocarina
  kind; 91 when played to `unk_6A8`), `Player_Action_8084E3C4`, `func_8084B498` and
  `func_8084B4D4` (a talk's or a get-item's ocarina), the warp songs (`OCARINA_MODE_02`: the
  respawn at `sWarpSongEntrances`, the cutscene action, `DEMOKANKYO_WARP_OUT`) and start mode 5
  (`Player_Action_StartWarpSongArrive`, `_WaitForCutscene`; Player drawn while it should be
  invisible, BACKLOG #26). Player's requests (`PlayRequest::StartOcarina`, `OcarinaSetInstrument`,
  `CamViewTarget`, `CamClearFinished`, `WarpSparkles`) carry what it does to the play state.
  An actor sets itself as the listener through `PlayerIface::set_unk_6a8`.
- **The freeze while it's played**: `Actor_UpdateAll`'s `freezeExceptionFlag`: with
  `PLAYER_STATE2_USING_OCARINA`, only actors with `ACTOR_FLAG_UPDATE_DURING_OCARINA` update.
- **The effects whole, drawn as bakes.** The wipes' frustums write their vertices' alphas each
  frame (`sAlphaIndices`): the bake keeps the vertices' sources, as `Demo_Effect`'s
  (`FrustumVertices`). A list's draw that calls a segment between two of its lists (`Oceff_Spot`'s
  material, its `Gfx_TwoTexScroll`, its model) bakes with a call of that segment
  (`pack::SEGMENT_CALL`). `Oceff_Storm`'s `gSPTextureRectangle` of rain is a screen-space quad
  (`DrawParams::screen`, ADR 0056's fills). `Environment_AdjustLights` darkens the scene for
  `Oceff_Spot` and for `En_Elf` (the user's call). `Magic_Reset` and Nayru's Love's restore in
  their destroys are logged (no magic yet, BACKLOG #49).
- **The storm**: `En_Okarina_Effect` whole with `Environment_PlayStormNatureAmbience` and
  `_StopStormNatureAmbience`; `gInterruptSongOfStorms` is kept but nothing sets it (its
  `En_Weather_Tag` isn't ported, BACKLOG #51).
- **`Shot_Sun` and `En_Okarina_Tag` whole.** The fairy spot's fairy, Lake Hylia's sun (its fire
  arrows an `Item_Etcetera` placeholder, BACKLOG #48); every tag type, their cutscenes by name.
- **The listeners wired**: `En_Ma1`'s Epona's Song lesson (`OCARINA_INSTRUMENT_MALON`, teach and
  playback), `En_Md`'s Saria's Song check in the Lost Woods, `En_Gs` (free play, `unk_E3F2`),
  `En_Box`'s song chests (`EnBox_AppearOnCorrectSong`), `Obj_Timeblock`, `En_Kanban`'s repair by
  Zelda's Lullaby; the pause menu's song playback and prompt states read the staffs (the quest
  page that starts them isn't ported, BACKLOG #50); `CutsceneCmd_Text`'s ocarina actions.
- **The exit run**: `Route::Ocarina` (`--script ocarina`, the preset `ocarina-songs`, Hyrule Field
  at 18:30) with the tasks `Ocarina` (the ocarina out, the song's buttons from the pack, each note
  held 3 frames and let go 2), `TalkThrough`, `WaitDawn` and `WaitFairy`.

## Consequences

- Link plays the ocarina: free play, the songs' effects, Navi's call, the Sun's Song's dawn, the
  Song of Storms' rain and the fairy spot's fairy; the teachers' lessons and playbacks run from
  their texts. A song's notes are heard through the audio library already ported.
- `Obj_Timeblock`'s tests play the Song of Time on the pad now (they had set `lastPlayedSong`).
- No golden changed (161 the same bytes): no earlier route takes the ocarina out. New: `ocarina`
  and four of its frames.
- **Not modelled:** the scarecrows, the memory game, the frogs (#47); the fire arrows' item
  (#48); magic (#49); the quest page's songs (#50); `En_Weather_Tag`'s interrupt and
  `En_Horse`'s answer to Epona's Song (#51); Player invisible on a warp song's arrival (#26).
