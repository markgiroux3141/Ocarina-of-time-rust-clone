# 0027: Sound effects: the C's pointers as named sources, read when the C reads them; actors' positions through `projectedPos`; the tables in the pack

- **Status:** accepted, built in GAME-04 milestone 3; builds on ADR 0026 (the boundary)
- **Date:** 2026-10-01

## Context

`Audio_PlaySfxGeneral(sfxId, pos, token, freqScale, vol, reverbAdd)` doesn't take values: it
takes pointers, and keeps them. A request becomes an entry of one of seven banks
(`sfx.c`), and every `Audio_Update` the bank's entries are read through those
pointers again: the position (`Vec3f*`, nearly always an actor's `projectedPos`, which
`Actor_DrawAll` recomputes every frame, or `gSfxDefaultPos`) for the distance, the priority,
the pan and the reverb; the frequency and volume scales (`f32*`) and the reverb offset (`s8*`)
for the channel's properties (`Audio_SetSfxProperties`). Entries are matched by pointer: a
request at the same `pos` refreshes or replaces the entry there, and the stops
(`Audio_StopSfxByPos`, which `Actor_Delete` calls) find entries by it.

The scales and offsets point at a handful of the audio code's statics (`D_8016B7A8` and its
neighbours, the river's and waterfall's lerps, `gPitchFrequencies[i]`) or at the defaults
(`gSfxDefaultFreqAndVolScale`, `gSfxDefaultReverb`). The game also reads the audio thread for
the sound effects: the sound effects' sequence's channels (`IS_SEQUENCE_CHANNEL_VALID`, their
IO port 1, which says a sound ended), `audioRandom`, and `AudioThread_NextRandom`'s `sAudioRandom`, which
both threads advance.

## Decision

- **Pointers become named sources, compared as the C compares the pointers:**
  - `SfxPos`: `Default` (`gSfxDefaultPos`) or `Actor(handle)` (`&actor->projectedPos`);
  - `SfxF32`: `One` and each static the C points at by name (`D8016B7A8`, ..., `RiverFreq`,
    `Pitch(i)`); `SfxS8` likewise;
  - each is read where the C dereferences it: the scales and offsets from `GameAudio`'s
    statics when `Audio_SetSfxProperties` runs; the positions once per `Audio_Update`, just
    before `func_800F8F88` reads them (`GameAudio::audio_update_with`, with the play state's
    `projectedPos`es; a position that no longer resolves keeps its last value, as a dangling
    pointer would read on).
- **`projectedPos`** is computed where `Actor_DrawAll` computes it, through the frame's
  `viewProjectionMtxF` (`guPerspective` with the camera's look-at, as the port's
  `view_proj`), and `actor->sfx` is played there too (`Actor_UpdateFlaggedAudio`); `Actor_UpdateAll`
  clears it, `Actor_Delete` stops the actor's sounds.
- **The sound effects' engine is ported whole** (`oot_game::audio::sfx`): the requests and
  their swap table, the banks' linked lists in their arrays, the choice by priority, the
  channels' start and refresh, every stop, `Audio_ResetSfx`, the properties' arithmetic (volume
  by distance, reverb, pan, frequency, the surround mode's stereo bits and filter), the helpers
  with scales of their own (`func_800F4010`'s footsteps and the rest), `z_lib.c`'s shorthands
  and `AudioMgr_StopAllSfx`.
- **The audio thread's state the sound effects read comes through the view** (ADR 0026):
  the sound effects' channels' validity and IO ports, `audioRandom`; the game's
  `AudioThread_NextRandom` advances its copy of `sAudioRandom` and sends it back (`GameOp::SetAudRand`).
- **Player's sounds are requests** (`PlayRequest::Sfx`), applied right after its update in the
  order it made them, as its other requests are (ADR 0016); the rest of the game calls
  `GameAudio` directly.
- **The tables are in the pack**, read from the C (`table/audio`): `gSfxParams` from the seven
  bank tables (with the names, so the ids the code names are checked against them), the banks'
  sizes, `gChannelsPerBank`, `gUsedChannelsPerBank`, `gIsLargeSfxBank`, `sBehindScreenZ`,
  `sSfxSwordChargeFreqLevels`, `sSurfaceMaterialToSfxOffset` (the floors' footsteps) and `z_player.c`'s 40 `AnimSfxEntry`
  tables (the animations' sounds). The ids the code uses are constants (`NA_SE_*`), each
  checked against its table row by the pack's test.

## Consequences

- A sound effect's lifetime is the C's: requested, refreshed while asked for each frame, started
  on a channel by priority, let go when the channel says it ended; and sounds follow their
  actor as it moves, through the same pointer semantics.
- Tests can check the C's sound calls frame by frame (`GameAudio::log`'s requests) and that
  they sound (the sound effects' sequence strikes their notes).
- The view is a frame old when `Audio_PlayActiveSfx` reads a channel's IO port, as with the
  music's reads (ADR 0026).
- Calls with other pointers need new names when they're ported: the play state's sound sources
  are `SfxPos::Source(i)` (`z_sfx_source.c`), `sSariaBgmPtr` is `SariaPos` (an actor's
  `projectedPos` or `home.pos`, read through the actor at the call); an actor's own floats
  would be a new `SfxF32` source.
