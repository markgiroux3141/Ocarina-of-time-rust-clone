# 0087: The frogs and the scarecrows: the ocarina's recording and the box's frog and scarecrow modes

- **Status:** accepted, built in GAME-10 milestone 9 (2026-10-23); the hand test deferred
- **Date:** 2026-10-23
- **Builds on:** [ADR 0061](0061-the-ocarina.md) (the ocarina: its audio, the message box's modes,
  the scarecrows and the frogs left out, BACKLOG #47), [ADR 0066](0066-the-actor-schema.md) (the
  schemas), [ADR 0079](0079-the-level-texts.md) (the texts).

## Context

Phase 11's last milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`, the user's call 3: here,
not Phase 20). Three placed overlays were left: `En_Fr` (the frogs on Zora's River's log and their
invisible song spot, 1,118 lines), `En_Kakasi` (the scarecrow who learns a long song, 365) and
`En_Kakasi3` (Bonooru, who learns the scarecrow's spawn song, 457), all in this phase's scenes. What
they listen with was the part of the ocarina ADR 0061 left out: the recording
(`AudioOcarina_SetRecordingSong`, `_SetRecordingState`, `_RecordSong`, `general.c`), the message box's
`MSGMODE_SCARECROW_*` and `MSGMODE_FROGS_*`, and the long song after the credits
(`AudioOcarina_PlayLongScarecrowSong`). The adult's Pierre (`En_Kakasi2`, 263 lines, placed in Lake
Hylia's adult layers and other adult scenes) comes up to the spawn song.

## Decision

1. **The recording whole** (`oot_game::audio::ocarina`): the statics (`sRecordSongPos`,
   `sOcarinaRecordTaskStart`, `sRecordOcarina*`, `sScarecrowsLongSongSecondNote`) in `Ocarina`;
   `AudioOcarina_RecordSong` in the audio update (a note written at each change of pitch, volume,
   vibrato or bend, counting the audio side's tasks: three a frame); `SetRecordingSong` writing
   into the long song's notes or, for the spawn song, into the memory game's song as a buffer
   (flat, as the C's memory), and on completion trimming the trailing rests, dropping the first
   entry, mapping the buttons and refusing a song that holds any of the twelve songs or one note
   eight times (`OCARINA_RECORD_REJECTED`, the spawn song's second note's volume 0xFF: unset). This
   ROM isn't `PLATFORM_N64`: a vibrato or bend change counts in the spawn recording too (none is
   played there: the spawn recording takes no bend or vibrato input).
2. **The box's modes whole** (`oot_game::message`): `MSGMODE_SCARECROW_LONG_RECORDING_START` and
   `_ONGOING` (the notes into a scrolling buffer of eight, B or the staff's end stops it,
   `scarecrowLongSongSet` and the song `MemCopy`'d into the save), `_LONG_PLAYBACK` and
   `_SPAWN_PLAYBACK` (`OCARINA_MODE_0F` at the end), `_SPAWN_RECORDING_START`, `_ONGOING`
   (`scarecrowSpawnSongSet` and the song into the save; refused or B: `_FAILED`, 0x40AD,
   `OCARINA_MODE_04`), and `MSGMODE_FROGS_START`, `_PLAYING`, `_WAITING` (a note at a time: the box
   takes it into `lastOcarinaButtonIndex` and waits; `En_Fr` answers and puts `_START` back). The
   unported-mode log is gone. The save's songs are the C's bytes (eight a note, big-endian:
   `ocarina::notes_to_bytes`).
3. **`Sram_OpenSave`'s copies** into the ocarina are the audio's statics, so they run where a play
   state starts on a loaded file (`Ocarina::sram_open_save_scarecrow_songs`: the game's start and the
   console reset), not in `sram_open_save`.
4. **The long song after the credits** (`AudioOcarina_PlayLongScarecrowSong`) whole, wired to
   `CS_MISC_LONG_SCARECROW_SONG` (the cutscene's misc 35, its frame held so it runs on). The credits
   themselves aren't ported, so nothing reaches it yet.
5. **`En_Fr` whole** (by a helper): the song spot listens (Link above it within 30), one-point
   4110, the frogs out of the water one by one, a note jumps its frog, the six songs' dance and
   rewards (a purple rupee the first time for each child song, a piece of heart for the Song of
   Storms; blue rupees after), and with all six the song game (0x40AB, `OCARINA_ACTION_FROGS`, 14
   notes of `gFrogsSongPtr`, then random ones; a piece of heart the first time). `sEnFrPointers`
   is a play-state static. `sFrogsSongNotes` is the pack's (`AudioGameTables::frogs_song_notes`,
   read from `general.c`): **pack format 32**. Its draw: the skeleton in the frog's colour (a dynamic
   env colour), its eyes, limbs 7 and 8 facing the camera, the butterfly (En_Butte's bake) and its
   light.
6. **`En_Kakasi`, `En_Kakasi3` and `En_Kakasi2` whole** (by a second helper): the long song's
   recording and playback (one-points 2260, 2270), Bonooru's spawn recording as a child (2260,
   0x40A4, 0x40A5), his playback (2280, 0x40A0), the adult's check (0x40A8: 0x40A7 and
   `EVENTCHKINF_9C`, or 0x40A6), and Pierre, hidden until the spawn song is played near him with
   `EVENTCHKINF_9C` (his switch flag, the attention camera 5010, his rise). Pierre is small, so he's
   whole rather than a placeholder chosen at init. One-points 2260, 2270 and 2280.
7. **Two runs, debug starts on a new preset `ocarina-c`** (`ocarina-of-time` with the Ocarina of
   Time back on C-Down and the Song of Storms, a debug start's: the adult's windmill):
   `Route::Frogs` (the log's forced text 0x22E, `En_Wonder_Talk2` 0x4BBB; each of the six songs and
   its reward; then the song game) and `Route::Bonooru` (the spawn song recorded, then played back
   by him, ending as the preset `scarecrow-song`). The tasks `OcarinaNotes` (the ocarina out to a
   listener that opens its own box, notes played one at a time while the box is ready),
   `OcarinaHeard` (a song played, done when heard: the listener's texts follow with the ocarina
   out) and `TextsFor` (A through texts until a `Cond`). Debug presets for the hand test:
   `frogs-choir` (the six songs played: the song game), `scarecrow-song` (the spawn song recorded
   as a child) and `scarecrow-pierre` (and played back to Bonooru as an adult: Pierre comes up).

## Consequences

- Every actor Phase 11's scenes place is ported (35 of 35); BACKLOG #47 is closed.
- The game's bugs kept (`@bug (game)`): `En_Fr`'s `flags` bumped on every frame a jump holds frame
  6 (the frogs come out overlapping, `flags` reaching 26); `EnFr_CheckOcarinaInputFrogSong`'s
  butterfly index unset for another button; one-points 2270 and 2280 index a keyframe row by
  `subCamId - 1`; the three scarecrows never free their skeletons.
- A child never sets `EVENTCHKINF_9C` (only the adult's playback to Bonooru does): Pierre needs the
  adult. `game-frogs.bat`'s `bonooru` and `pierre` starts are the adult's.
- Not modelled: the credits that would reach the long song's playback; `BREG` debug displays.
