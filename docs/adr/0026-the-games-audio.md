# 0026: The game's audio: the game's thread and the audio thread meet once per game frame; the game's side ported whole in `oot_game::audio`, its tables and the scenes' sound settings in the pack

- **Status:** accepted, built in GAME-04 milestone 2; builds on ADR 0024 (the audio data) and ADR 0025 (the mixer)
- **Date:** 2026-10-01

## Context

The audio library (`eng_audio`, ADR 0025) runs on the audio thread, a frame per VI retrace.
What plays when is the game's: `code_800F9280.c` (the sequence commands), `code_800EC960.c`
(the scene's music, the sequence modes, fanfares, the nature ambience, `Audio_Update`),
`z_kankyo.c` and `z_scene.c` (the scene's sound settings, the time of day's music). That code
runs on the game's thread, and on the console both threads share `gAudioContext`:
- the game **writes** commands into the library's command ring (`Audio_QueueCmd*`) and hands
  them over (`Audio_ScheduleProcessCmds`); for a spec change, `func_800E5F88` reads and writes
  `resetStatus` and `audioResetSpecIdToLoad` and may rewind the ring (`Audio_ResetCmdQueue`) or
  block on `audioResetQueue` until a reset under way is done;
- the game **reads** a few fields directly: a player's `enabled` and `tempo`, the players' and
  channels' IO ports (`soundScriptIO`), the channels' `notePriority`, `updatesPerFrame`, the
  messages on `audioResetQueue` and `externalLoadQueue`, the notes sounding.

The audio thread runs three retraces in each 20 Hz game frame, at times the game doesn't
control. Here the audio side is either offline (`eng_audio::Renderer`: headless runs, tests,
deterministic) or a thread of its own on the clock (`eng_audio::output::AudioOutput`: the
window). Sharing the context between threads would make the window's sound depend on locks and
timing, and the offline runs need an order.

## Decision

- **The two sides meet at the end of each game frame.** The game's frame produces a list of
  what its thread did to the library, in order (`eng_audio::GameOp`: a command, a schedule, a
  ring rewind, the spec change); the audio side applies them in that order
  (`AudioContext::apply`) before its next retrace, runs its retraces, and gives back what the
  game reads (`eng_audio::AudioView`), which the game uses throughout its next frame. Offline
  that's exactly three retraces per game frame (`R_UPDATE_RATE`); in the window the audio
  thread runs on the clock and publishes a view after every retrace, and the game takes the
  latest at each frame.
  - The queues' messages travel in the view and stay until the game receives them
    (`func_800E5EDC`, `func_800E5E20`), so a reset's message is never lost between views.
  - `func_800E5F88`'s game-thread part (emptying its copy of `audioResetQueue`) runs in the
    game; the rest runs as the audio side applies `GameOp::ResetSpec`. Where the C blocks
    until a reset under way finishes (its status 1 or 2, at most two more audio frames), the
    reset's remaining steps run at once, without those frames' output.
  - Before the audio side's first view, the game reads `AudioView::boot` (`AudioLoad_Init`'s
    spec 0: `updatesPerFrame` from `AudioHeap_Init`'s arithmetic, no player on), which is all
    the game reads at boot.
- **The game's side is ported whole, in `oot_game::audio`** (game layer; `eng_audio` stays
  game-agnostic): `seqcmd` (`code_800F9280.c`), `bgm` (`code_800EC960.c`'s sequence parts and
  `Audio_Update`), `scene` (`Environment_PlaySceneSequence`, `Environment_PlayTimeBasedSequence`,
  `Scene_CommandSoundSettings`). Their statics are one struct, `GameAudio`, on the play state;
  they're the code segment's, so `Play_Init` on a scene change carries them over
  (`PlayState::play_init_with`), as it does `z_demo.c`'s. The order inside a frame is the C's:
  `Play_Init`'s calls, the actors' (Player's `Audio_SetSequenceMode`), `Environment_Update`'s,
  then `Audio_Update` at the end of the graph frame, and again when a game state ends
  (`GameState_Destroy`) before the next `Play_Init`.
- **The tables and the sound settings are in the pack**, read from the C by the importer
  (`table/audio`: `sSeqFlags`, `sSpecReverbs`, `sNatureAmbienceDataIO` with `sequence.h`'s
  `NATURE_IO_*` macros expanded, `gSoundModeList`), and each scene layer's
  `SCENE_CMD_SOUND_SETTINGS` (`LayerData.sound`). Pack format 14.
- **The drivers:**
  - headless: `oot_game::audio::offline::OfflineAudio`, either called after each frame or
    attached to the play state (`PlayState::audio_side`, carried over a scene change), which
    then hands its frames over by itself;
  - the window: `PlayState::advance_with` hands each frame's ops to the device's thread and the
    latest view back;
  - with no audio side (the goldens, `--no-audio`), the game's side runs anyway and its ops are
    dropped (the latest 65536 are kept). `Audio_Update` then waits for the first spec change's
    reset forever, as the C would without an audio thread, so nothing past `Play_Init`'s commands
    is queued.
- **`--music <n>` is `Environment_ForcePlaySequence`**: the first scene plays sequence n in
  place of its own, the C's own mechanism.
- **Not in the C, for tests and traces:** `GameAudio::log` (every sequence command and op, by
  game frame), `AudioView::players[..].seq_id`, `oot_sandbox --audio-log` and `--wav`.

## Consequences

- Headless runs repeat sample for sample, with the music as the game would play it; a test can
  check the commands the game's side queues against the C, frame by frame, and what plays.
- In the window, the game reads the audio side up to a game frame late, as on the console
  (where the three retraces of a frame fall anywhere in it); the commands of a frame arrive
  together, before a retrace.
- The approximations: the spec change's blocking wait (above); the game's reads come at frame
  boundaries rather than mid-frame; `osGetCount` stays deterministic (ADR 0025).
- The sound effects (milestone 3) go through the same boundary: `Audio_PlaySfxGeneral`'s
  requests, then the sfx player's channel IO.
