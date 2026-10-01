# GAME-04: audio

**Goal:** Phase 5 of [ROADMAP.md](ROADMAP.md): music and sound effects, from the ROM through the
pack.

| # | Milestone | Status |
|---|---|---|
| 1 | The import and the synth: the audio data in the pack, `eng_audio` (the audio library and its microcode, offline and through an output device), the mixer's ADR | done |
| 2 | The game's music: the scenes' sound settings in the pack, the sequence commands (`code_800F9280.c`), the scene's music and the ambience (`code_800EC960.c`, `Environment_PlaySceneSequence`), the boundary between the game's thread and the audio thread | done |
| 3 | Sound effects: `Audio_PlaySfxGeneral` and the sfx channels, the calls the ported code marks as left out | started: the engine whole, positions through `projectedPos`, the message box, most of Player's ported actions |

**Phase exit:** Kokiri Forest's music plays and loops like the game, and a scripted run's sound
effects log matches the calls in the C.

The working rules are the same as for the earlier phases:
- no game data in the repo;
- the engine never depends on game code (`cargo test -p layering`);
- the runtime reads only the pack;
- ports go function by function with the decomp's names, every constant is cited, and faithful
  bugs are marked `@bug (game)`;
- test expectations come from the C.

**Priorities (2026-09-30):** gameplay first. No music or sounds inside cutscenes in this phase:
the cutscenes' audio waits for the cutscene polish pass ([BACKLOG.md](BACKLOG.md) #10).

Decisions are in [docs/adr/](adr/README.md) (0024 on).

## Milestone 1: the import and the synth

**Answer:** done. The pack holds the game's audio as the ROM has it, and `eng_audio` is the
game's audio library ported whole: the heap and the loads, the sequence player, the notes,
envelopes, vibrato and portamento, the synthesis, the audio thread's frame, and the RSP's audio
microcode that the command lists run on. It renders offline exactly as the console's audio
interface would play (32006 Hz, a frame per VI retrace), and in the window it plays through the
default output device. A note of a Kokiri Forest instrument plays its sample at the C's pitch
with the C's volume on every update; Kokiri Forest's sequence plays, loops, and strikes the
notes the extractor's independent interpreter reads.

The game's side isn't ported yet: nothing plays by itself. `--music 60` plays Kokiri Forest's
sequence in the window (`scripts\run\game-music.bat`), and `ootx audio-wav` writes WAVs.

The tests: 270 pass, 1 ignored (257 before). The goldens are unchanged: 84 of 84 identical.

### What was built

1. **The audio data in the pack** ([ADR 0024](adr/0024-audio-data.md)), pack format 13:
   - the ROM's `Audiobank`, `Audioseq` and `Audiotable` as they are, with their ROM addresses
     (`audio/rom/<file>`, `eng_audio::RomFile`);
   - `audio/tables` (`eng_audio::AudioTables`): the four audio tables from the ROM where
     `data/audio_tables.rodata.s` puts them; `audio_data.c`'s tables (the wave samples,
     `gPitchFrequencies`, the bend tables, the pan volumes, the Haas delays, the default envelope
     and short-note tables, the filters, `D_8012FBA8`) and `audio_init_params.c`'s 18 audio specs
     with their reverbs, read from the C (`oot_import::audio`); `gAudioHeap`'s size and the init
     sizes' `#define`s; `gTatumsPerBeat`;
   - the microcode's resampler filters, from the ROM's `aspMainData` at 0xE0 (`data/rsp.rodata.s`);
   - the 0x11000 bytes of `code` from `func_800E4FE0` that `gWaveSamples[8]` reads as noise.
2. **`eng_audio`, the audio library** (engine layer; `cargo test -p layering` knows it, and only
   it may use `cpal`), ported function by function ([ADR 0025](adr/0025-audio-mixer.md)):
   - `context`: `gAudioContext` and every struct of `z64audio.h`; notes, layers, channels and
     players as arrays linked by index; the C's `AudioListItem` lists as an arena of nodes, in
     the C's order; `SeqScriptState` as its 0x1C bytes, so the unchecked call depth lands where
     the C's does;
   - `ram`, `layout`: a simulated RDRAM with KSEG0 addresses and a cartridge of the three files;
     `Sample`, `Instrument`, `Drum`, `AdpcmLoop`, `AdpcmBook` and `EnvelopePoint` read and
     written where they are;
   - `heap` (`audio_heap.c`): the pools, the persistent and temporary caches, the sample caches,
     the ADSR decay table, the filters, the reset steps, `AudioHeap_Init`;
   - `load` (`audio_load.c`): the sample DMAs (`AudioLoad_DmaSampleData` and its two reuse
     queues), the sync, async, slow and script loads, the preloads, font relocation,
     `AudioLoad_Init`;
   - `seqplayer` (`audio_seqplayer.c`): the flow control, the layers' five steps, the channels'
     and players' interpreters, player init and reset;
   - `effects` (`audio_effects.c`): the channels' and players' volume, pan and bend, portamento,
     vibrato, `Audio_AdsrUpdate`;
   - `playback` (`audio_playback.c`): `Audio_ProcessNotes`, `Audio_InitNoteSub`, note allocation
     and its policies, the synthetic waves;
   - `synthesis` (`audio_synthesis.c`): the command list of a frame: each note's decode,
     resample, gain, filter, envelope mixer and Haas effect, the reverbs' ring buffers, the
     interleave;
   - `thread` (`code_800E4FE0.c`): `func_800E5000` (one audio frame per retrace), the command
     queue (`Audio_QueueCmd*`, `Audio_ScheduleProcessCmds`, `Audio_ProcessCmds`), the library's,
     players' and channels' commands; and the AI's queue (`Ai`), which answers `osAiGetLength`.
3. **The microcode** (`rsp`): the commands built word for word as `abi.h`'s macros build them,
   run on a 4 KB DMEM after mupen64plus-rsp-hle's `alist_process_nead_oot`, with `aClearBuffer`
   rounded up to 16 bytes as the decomp documents it and `aFilter`'s `A_INIT` honoured.
4. **Offline** (`Renderer`): retraces at 60 a second with the AI playing at 32006 Hz; the output
   is what the AI plays. `play_script` and `note_script` (not in the C) start a sequence script
   of our own on a player, as `AudioLoad_SyncInitSeqPlayerInternal` starts a sequence: one note
   of any instrument or drum through the whole library, for tools and tests.
5. **The output device** (`output`, `device` feature): a thread that runs the same retraces on
   the clock and queues the AI's output; a cpal stream that resamples it to the device's rate
   and keeps 64 ms queued (starting once it has them).
6. **The window app**: `oot` (and the sandbox's window) opens the default output device;
   `--music <n>` starts sequence n on player 0, `--no-audio` keeps it quiet; the HUD names the
   device.
7. **`ootx audio-wav`**: a WAV of a sequence (`--seq`), of a note of an instrument
   (`--font --inst --note`) or a drum (`--font --drum`), played through the library, or of the
   note's sample as stored (`--raw`).
8. **Shared with the extractor:** the reference VADPCM decoder moved from `oot_extract::audio`
   to `eng_audio::adpcm`, with a typed font reader (`eng_audio::font`); the extractor uses it
   (its output is unchanged).
9. **Run scripts:** `test-audio.bat`, `game-music.bat`, `audio-wav.bat` (menu 25 to 27).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 270 passed, 1 ignored |
| The microcode's ADPCM (`oot_game --test audio`) | All 450 samples of the 38 fonts decode through `aADPCMdec` exactly as the reference decoder decodes them, in 8-frame commands as `AudioSynth_ProcessNote` lays out DMEM. All 115 checkable loops restart alike from their stored predictor states (`aSetLoop`, `A_LOOP`). Font 37's instrument 0 is compared up to its frame 164: from there 1828 of its 2638 frames use predictors 2 and 3 of a 2-predictor book, which on the RSP read whatever an earlier book left in DMEM; that is also the one sample whose stored loop state the extractor found differing (ADR 0024) |
| A Kokiri Forest note | Font 15's instrument 4 (tuning 1.0; envelope (2, 32700), (1, 32700), (32700, 29430), hang; decay index 239), C4 for 96 tatums at 120 bpm (179 updates: a tick every 10770 / 5760 updates). Every held update's `targetVolLeft` equals the C's ADSR (`Audio_AdsrUpdate`, `updatesPerFrameScaled` 0.75) × `gDefaultPanVolume[64]` × (0x1000 − 0.001), exactly. C4: the output correlates 1.00000 with the decoded sample, 1840 samples later (the AI's two-frame pipeline and the note's start). C3 (`gPitchFrequencies[27]` 0.5): 1.00000 against the sample at half speed. `adsrDecayTable[239]` = (1/3) / 12: silent within half a second of the release. Left/right 0.988 (`gDefaultPanVolume[64]` / `[63]`) |
| A drum | Font 3's drum 0 (tuning 0.386): correlation 0.990 with its sample at its tuning |
| The reverb | A note with channel reverb 0x7F rings on after it ends; past its first trip round the 3072-sample ring, each window is the last at 0.3749, 0.3746, 0.3740 (`decayRatio` 0x3000 / 0x8000) |
| The heap (`AudioHeap_Init`, spec 0) | 32006 Hz (`osAiSetFrequency`), `samplesPerFrameTarget` 544 (528 to 560), 3 updates a frame of 176 (168 to 184), `tempoInternalToExternal` 10770; the misc pool 0x21E90 bytes; the sample DMA buffers 72 of 0x300 and 5 of 0x200, where the C asks for 24 of 0x200 (the pool is full, the C stops); the reverbs' windows 3072 and 2048 samples |
| Kokiri Forest's sequence (60) | 5952 ticks in 3071 retraces (51.18 s; the extractor's tempo arithmetic says 51.31 s). 597 notes before the tick that loops, as the extractor's interpreter reads them (it drops that tick's second pass). Plays on past the loop: 75 s rendered, peak 14218, no clipping |
| The microcode on made-up data (`eng_audio --test microcode`) | ADPCM against the reference decoder (4-bit and 2-bit, 1, 2 and 4 predictors); the resampler's stepping; the envelope mixer's volumes, ramp and wet send; `abi.h`'s truncations; the AI's queue; the note script |
| The pack (`oot_import --test pack`) | The audio data is what the importer reads: the three files byte for byte; 38 fonts, 110 sequences, 7 sample banks; `gPitchFrequencies` 1.0 at C4 and 0.055681 at 0x75; `gDefaultEnvelope` with `ADSR_HANG` and `ADSR_DISABLE`; 18 specs; the resampler's 64 phases, each mirroring its opposite |
| Import | 15.7 s, audio 0.0 s; 63.7 MB (59.6 before); format 13, into `out/data10` |
| Golden traces and renders | 84 of 84 identical |
| The window | The device opens and plays in real time (60 retraces a second): with `--music 60` its queue held 1969 to 2012 frames over 15 s and never ran dry. Not yet heard by hand |
| WAVs | `ootx audio-wav`: Kokiri Forest's 75 s in 1.9 s, the pack's loading included |

### Decisions

- **[ADR 0024](adr/0024-audio-data.md):** the ROM's audio files as they are in the pack, with
  the tables; nothing decoded at import. The loops' stored predictor states and the sample that
  can't be decoded ahead of time decide it; so do the C's pointers into fonts and sequences.
- **[ADR 0025](adr/0025-audio-mixer.md):** the library ported whole and literally, the
  microcode by its HLE; 32006 Hz at 60 retraces a second (NTSC, as the port's VI runs);
  offline is the console; the device resampled at the end. What's approximated: `osGetCount`
  (deterministic), the PI's timing, the DAC, the HLE itself.
- **The tests that need the pack live in `oot_game`**, as the engine can't depend on the game
  even for its tests; `eng_audio`'s own tests use made-up data.
- **Two counters not in the C** (`Stats`: notes struck, ticks per player), for tests and tools.
- **Tools and tests play notes with sequence scripts of their own** (`note_script`), through the
  real player, rather than a side door into the notes.

### Known gaps

- **The game's side** (`code_800EC960.c`, `code_800F7260.c`, `code_800F9280.c`,
  `z_sfx_source.c`): which sequence plays where and when, the ambience, the sound effects. The
  scenes' sound settings (`SCENE_CMD_SOUND_SETTINGS`) aren't in the pack yet. Milestones 2 and 3.
- **Not exercised on the game's data:** the Haas effect and headset mode (the sound mode is
  stereo), the filters (`aFilter`), the sample caches (zero-sized in every spec), the slow and
  script loads, the reset steps on a spec change, a sample DMA running out of buffers.
- **The microcode** is checked against the C that builds the lists, not against the RSP itself.
- **NTSC** although the ROM is European (ADR 0025); `osGetCount` is deterministic.
- **The DAC and the analog output** aren't modelled; the device's resampling is linear.
- **Cutscene audio** stays deferred (BACKLOG #10).
- **By hand:** the music hasn't been heard by ear yet (`game-music.bat`, `audio-wav.bat`).

## Milestone 2: the game's music

**Answer:** done. The game starts and changes its own music: each scene's sound settings come
from the pack, `Play_Init` queues the spec change and `Environment_PlaySceneSequence` the music
(or, by night, the nature ambience), and every frame's `Audio_Update` turns the game's sequence
commands into the library's, as `code_800F9280.c` and `code_800EC960.c` do. Kokiri Forest
starts and loops its music headless and in the window with no `--music`. Leaving a scene fades
every player out, the next scene changes the spec and starts its own; back in the forest from a
house or the shop, the music resumes where it left off, as the game's does. Chests and items
play their fanfares, the music fading under them and back.

The tests: 276 pass, 1 ignored (270 before). The goldens are unchanged: 84 of 84 identical.

### What was built

1. **The scenes' sound settings in the pack** (pack format 14, `out/data11`):
   `SCENE_CMD_SOUND_SETTINGS` of every layer (`LayerData.sound`: spec, nature ambience,
   sequence); and the game's audio tables, read from the C (`table/audio`,
   `oot_game::audio::AudioGameTables`): `sSeqFlags` with its `SEQ_FLAG_*` defines,
   `sSpecReverbs`, `sNatureAmbienceDataIO` with `sequence.h`'s `NATURE_IO_*` macros and enums
   expanded, `gSoundModeList`. `ootx scene-info` prints a layer's sound settings.
2. **The boundary** ([ADR 0026](adr/0026-the-games-audio.md), `eng_audio::link`): what the
   game's thread does to the library (`GameOp`: commands, schedules, the ring's rewind, the spec
   change) is applied in order by the audio side (`AudioContext::apply`) before its next
   retrace; what the game reads of it (`AudioView`: the players' `enabled`, `tempo` and IO
   ports, the channels' IO ports and note priorities, `updatesPerFrame`, the reset and load
   queues' messages, the notes sounding) comes back after. `func_800E5F88`'s audio side;
   `AudioTables::fonts_for_sequence` (`AudioLoad_GetFontsForSequence`); `AudioView::boot`.
   Offline: `Renderer::game_frame`; the window: `AudioOutput::send_ops` and `take_view`.
3. **The game's side, ported whole** (`oot_game::audio`, the game layer):
   - `seqcmd` (`code_800F9280.c`): `Audio_QueueSeqCmd`, `Audio_ProcessSeqCmd(s)` (every op:
     the starts and stops, the players' queues, the volume, frequency and tempo fades, the IO
     ports, the channel masks, the setup commands, the sound mode, the spec change),
     `func_800F9280`, `func_800F9474`, `func_800FA0B4`, `func_800FA11C`, `Audio_SetVolScale`,
     `func_800FA3DC`, `func_800FAD34`, the resets;
   - `bgm` (`code_800EC960.c`'s sequences): `Audio_Update` (`func_800F3054`),
     `func_800F5550` (the scene's music, resumed through `D_8013062C` and port 7),
     `func_800F56A8`, `Audio_SetSequenceMode` and the enemy music, `Audio_SplitBgmChannels`,
     `Audio_PlayFanfare` and `func_800F5CF8`, the mini-boss and ambience swaps and their
     restores, the river and Ganon's Tower volumes, `func_800F6964` (the fade on leaving),
     `Audio_PlayNatureAmbienceSequence`, `Audio_StartNatureAmbienceSequence`,
     `Audio_SetNatureAmbienceChannelIO`, `Audio_InitSound`, `func_800F6C34`, `func_800F7170`,
     `func_800F71BC`, the reverbs and filters' setters;
   - `scene` (`z_scene.c`, `z_kankyo.c`): `Scene_CommandSoundSettings`,
     `Environment_PlaySceneSequence` (the Lost Woods' bridge, the forced sequence, no music,
     no ambience, by day, by night), `Environment_PlayTimeBasedSequence` (every state),
     `Environment_ForcePlaySequence`;
   - `offline`: `OfflineAudio`, the headless audio side.
4. **Wired into play:** the boot (`Audio_InitSound`, then the title screen's `func_800F6700` with a fresh SRAM's sound setting, stereo: the title and the file select aren't ported, so their own music isn't either); `Play_Init` (`Audio_SetExtraFilter(0)`, the sound settings,
   `Environment_Init`'s `TIMESEQ_DAY_BGM`, `Environment_PlaySceneSequence` and the save's
   `seqId`/`natureAmbienceId`), `GameAudio` carried over a scene change
   (`PlayState::play_init_with`), the transition's fade-out unless the next entrance continues
   the music, `Environment_Update`'s time of day, `Audio_Update` at each frame's end and as a
   game state ends, the room change's reverb (`func_80097534`), `Audio_SetCutsceneFlag` in
   `z_demo.c`'s six places; Player's `Audio_SetSequenceMode` every frame and its item-get
   fanfare (`func_8084DFF4`, after `Item_Give`) and the secret hole's fade; `En_Box`'s chest
   fanfare. The save holds `seqId`, `natureAmbienceId`, `forcedSeqId` (`SaveContext_Init`'s).
5. **The window:** each game frame's ops to the device's thread and the latest view back
   (`PlayState::advance_with`); `--music <n>` forces sequence n
   (`Environment_ForcePlaySequence`); the audio thread's log names what each player plays.
6. **The sandbox:** `--audio-log <json>` (every sequence command and library command by frame,
   and what each player played) and `--wav <file>` for any scripted run.
7. **Run scripts:** `test-music.bat`, `game-night.bat`, `sandbox-audio-log.bat`;
   `game-music.bat` now forces the title theme (menu 28 to 30).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 276 passed, 1 ignored |
| Kokiri Forest from a new game (`oot_actors --test music`) | `Play_Init` queues, as the C does, `F0000001` (spec 1, `seqId` disabled), `700700FF` and `0000003C` (`func_800F5550`: port 7 to 0xFF, `sSeqFlags[NA_BGM_GENERAL_SFX]` lacking `SEQ_FLAG_5`); `Audio_InitSound`'s `46000000 FF000000` and `82020000 7` come first, and the title's sound mode (`E0000000`: `func_800F6700(0)`, stereo). The first `Audio_Update` gives `F0000000 0` (stereo), the reset to spec 1, `46000000`, `F8`, `46000007`, `82003C00 0`, a schedule, and `func_800FA3DC`'s four `4p01007F`. Then nothing for the 6 game frames of the reset (18 audio frames), then `func_800FAD34`'s `46020000` and `func_800F7170`. The forest's sequence plays on player 0, the sound effects' 0 on player 2; after 1212 game frames, 7014 ticks (past the loop at 5952), 735 notes, still sounding in the last 5 s |
| From Link's house into the forest | The exit: `func_800F6964(0x14)`'s `101E00FF`, `111E00FF`, 15 sfx channel fades (all but the ocarina's), `131E00FF`; the forest: `F0000001`, `70070000`, `0000003C` (from `NA_BGM_LINK_HOUSE`'s `SEQ_FLAG_5` to the forest's `SEQ_FLAG_4`: port 7 from `D_8013062C`) |
| Kokiri Forest at 20:00 | `Audio_PlayNatureAmbienceSequence(NATURE_ID_KOKIRI_REGION)`'s commands from `sNatureAmbienceDataIO[4]`, then the night's critters (`TIMESEQ_NIGHT_CRITTERS`: critter 0 off, 1 to 3 on, while the start is still queued); `NA_BGM_NATURE_AMBIENCE` plays, its channels hold their critters' types; 91 notes in 20 s |
| The chests (`oot_actors --test chest`) | The Kokiri Sword's chest: `0101092B` (`NA_BGM_OPEN_TRE_BOX \| 0x900`), then `01010922` (`NA_BGM_ITEM_GET \| 0x900`); a second heart piece: `01010039` (`NA_BGM_SMALL_ITEM_GET`) |
| The boundary (`oot_game --test audio`) | `AudioView::boot`'s `updatesPerFrame` 3 is `AudioLoad_Init`'s; `func_800E5F88`'s paths: the reset runs over 18 audio frames and posts its spec; the same spec again: -2; another early: -3, switched; another late: the reset finishes first |
| The pack (`oot_import --test pack`) | The game's audio tables are the importer's reading of the C (`sSeqFlags` by the rows' `NA_BGM_*`, `sSpecReverbs` 40 and 15, the general night's ambience, `gSoundModeList`); five scenes' sound settings per layer are the ROM headers'; Kokiri Forest by day: spec 1, `NATURE_ID_KOKIRI_REGION`, `NA_BGM_KOKIRI` |
| The new file's run with sound (`sandbox-audio-log.bat`) | The same route, frame for frame (11748 frames), with 587 s of sound: the opening's layers start what their headers name, Link's house 0x1F, the forest 0x3C, Mido's house 0x1F and back (resumed: `001E003C`), the shop 0x55 and back (resumed), the Deku Tree 0x1C with spec 3; the chest's fanfare and the sword's and shield's item fanfares on player 1, the music fading under them and back. 4945 sequence commands, 18488 library commands; 7.5 s with the audio offline |
| The window | No `--music`: the device plays the forest's sequence on player 0 and the sound effects' on player 2; the queue held 1961 to 2021 frames over 20 s and never ran dry. Not yet heard by hand |
| Import | 12.5 s; 63.7 MB; format 14, into `out/data11` |
| Golden traces and renders | 84 of 84 identical |

### Decisions

- **[ADR 0026](adr/0026-the-games-audio.md):** the game's thread and the audio thread meet at
  the end of each game frame: the frame's `GameOp`s in, applied in order before the next
  retrace; an `AudioView` back, read the whole next frame; the queues' messages kept until
  received. `func_800E5F88` split between the two; its blocking wait finishes the reset at
  once. The game's side's statics as one struct carried over a scene change. The tables and
  the sound settings in the pack.
- **`--music` is the C's forced sequence** (`Environment_ForcePlaySequence`), not a side door.
- **A play state can carry its audio side** (`PlayState::audio_side`) for headless runs and
  tests; the window drives its device through `advance_with`.
- **`oot_actors` depends on `eng_audio`** (for the ops in tests; milestone 3's sound effects
  need it there).

### Known gaps

- **Sound effects** (milestone 3): `Audio_ProcessSfxRequests`, `func_800F8F88` and the sfx
  banks; the calls marked as left out (Player's, the actors', the message box's, the HUD's).
  The reverbs the floor and the room set (`Audio_SetCodeReverb`, `Audio_SetEnvReverb`) are
  stored for them. (Milestone 3 has started on all of this.)
- **Cutscene audio** stays deferred (BACKLOG #10): the opening's layers start what their
  headers name (the sound effects' sequence, the nature ambience), and their scripts'
  `CS_CMD_PLAYBGM`, `_STOPBGM`, `_FADEBGM` do nothing.
- **Time doesn't pass**, so the time of day's music stays in the state a scene starts in; no
  weather, so the rain's checks always pass. The day's count, the cucco's crow and the egg's
  hatching at dawn wait for the clock.
- **Not ported here:** the ocarina (`AudioOcarina_*`), the debug screen (`AudioDebug_*`),
  `Audio_PlaySariaBgm` (no caller yet), the pause menu's mute (`func_800F64E0` is ported, its
  caller isn't), the enemy music (`targetCtx.bgmEnemy` is never set: no enemies), game over's
  music, the bottle catch's fanfare; `Interface_ChangeAlpha(1)` in the transition's setup.
- **The approximations** (ADR 0026): the spec change's wait, the game's reads at frame
  boundaries.
- **By hand:** the window's music hasn't been heard by ear yet (`game.bat`, `game-night.bat`,
  `sandbox-audio-log.bat`'s WAV).

## Milestone 3: sound effects (started)

**Answer:** started. The sound effects' engine is ported whole and sounds: a request goes into
its bank, the banks' entries are chosen by priority every frame and started on the sound
effects' sequence's channels, refreshed while they're asked for, and let go when the channel
says they ended. Link's footsteps (by floor), jumps, landings, voice, the roll, the sword's
swing, ladders, ledges and the crawl sound in the game; so do the message box's sounds and the
item sounds. Many of the game's calls aren't wired yet (below).

The tests: 279 pass, 1 ignored (276 at milestone 2's end). The goldens are unchanged: 84 of 84.

### What was built

1. **The tables in the pack** (`table/audio`, still format 14): `gSfxParams` from the seven
   bank tables (`include/tables/sfx/*.h`, 1259 rows with their names), the banks' sizes
   (`gSfxBanks`' arrays), `gChannelsPerBank`, `gUsedChannelsPerBank`, `gIsLargeSfxBank`,
   `sBehindScreenZ`, `D_801305E4`, `D_80119E10` (the floors' footsteps, `z_bgcheck.c`), and
   `z_player.c`'s 40 `struct_80832924` tables (the animations' sounds); `sGanonsTowerLevelsVol`
   moved here from the code.
2. **The engine** ([ADR 0027](adr/0027-sound-effects.md), `oot_game::audio::sfx`):
   `code_800F7260.c` whole (`Audio_PlaySfxGeneral` and its swap table,
   `Audio_ProcessSfxRequest(s)`, the banks' lists, `Audio_ChooseActiveSfx`,
   `Audio_PlayActiveSfx`, `Audio_RemoveSfxBankEntry`, every `Audio_StopSfx*`,
   `Audio_IsSfxPlaying`, `Audio_ResetSfx`, the bgm mutes, the unused bank lerps);
   `code_800EC960.c`'s sound effect parts (`Audio_ComputeSfxVolume`, `_Reverb`, `_PanSigned`,
   `_FreqScale`, `func_800F37B8`, `func_800F3990`, `Audio_SetSfxProperties`, `func_800F3F84`,
   `func_800F4010` and the other helpers, the river and the waterfall, the transposed ones);
   `z_lib.c`'s `func_80078884`, `func_800788CC`, `func_80078914`; `AudioMgr_StopAllSfx`.
   `Audio_Update` now runs the requests and `func_800F8F88`.
3. **Positions:** `projectedPos` and `projectedW` on every actor, from `Actor_DrawAll`'s spot in
   the frame; `actor->sfx` and its four setters (`func_8002F8F0`...), played there
   (`func_80030ED8`); cleared by `Actor_UpdateAll`; `Actor_Delete` stops the actor's sounds.
4. **The boundary** (ADR 0026): the view carries `audioRandom`, `audRand` and the count
   register's value; `GameOp::SetAudRand` sends the game's `Audio_NextRandom` back.
5. **The message box:** `Message_ShouldAdvance`'s `NA_SE_SY_MESSAGE_PASS` (and the silent
   variant, as the C uses each, in the message code, the cutscene's texts and the actors),
   `Message_HandleChoiceSelection`'s cursor, `Message_DrawText`'s `NA_SE_SY_MESSAGE_END` and the
   text's own sound codes (`MESSAGE_SFX`), `Message_Update`'s `NA_SE_SY_DECIDE` and pass, and
   the C's silent (id 0) calls.
6. **Player** (`PlayRequest::Sfx`): the helpers (`func_80832698` the voice, `func_808327F8` the
   footsteps, `func_80832854` the jump, `func_808328A0` the landing, `func_80832770`,
   `func_808327C4`, `func_808328EC`, `func_80832924` the animations' tables, `func_808326F0`,
   `func_8084BEE4` the ladder), `unk_89E` (the floor's footstep, `func_80847BA0`) and the
   floor's echo (`Audio_SetCodeReverb`, with `SurfaceType_GetEcho`); and the sounds of the
   actions ported: walking and running, the side step, jumps and their voice, the automatic
   jump, landings and a fall's damage, the roll with its dust and its bonk, being hurt (the
   damage, the voices, the body hit), being knocked down and getting up, slipping off a
   ledge, hanging, climbing up ledges and walls, the ladder, the crawl, the sword's swing
   (`func_80833A20`), the child's chest opening, the item sounds (`func_8083E4C4`, the
   rupees' and hearts' `NA_SE_SY_GET_BOXITEM`), the voids and the secret hole.
7. **The sandbox's `--audio-log`** lists the sound effects asked for, by frame, with their names.
8. **Run scripts:** `test-sfx.bat` (menu 31).

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 279 passed, 1 ignored |
| Walking in Kokiri Forest (`oot_actors --test sfx`) | 10 footsteps in 60 frames from the walk's start, on exactly the frames where the walk phase crosses 10 or 24 of its 29 (`func_8084021C`, while running), each `NA_SE_PL_WALK_GROUND` + `D_80119E10`[the floor] + the age's `unk_94` (0x800 on the path, 0x808 on the grass); each started on a channel of the sound effects' sequence (port 0 to 1, port 4 the index); walking strikes 65 notes where standing still strikes the music's 36 |
| The message box | Text 0x1005: an A at each box break plays `NA_SE_SY_MESSAGE_PASS`, its end `NA_SE_SY_MESSAGE_END` once, the A that closes it `NA_SE_SY_DECIDE` |
| The tables (`oot_import --test pack`) | The banks' rows (224, 80, 248, 499, 72, 8, 128); `DEFINE_SFX(NA_SE_PL_WALK_GROUND, 0x20, 0, 2, SFX_FLAG_10)` packed as the C packs it; every id the code names is its table's row; the banks' sizes 9, 12, 22, 20, 8, 3, 5; the channel layouts; `sBehindScreenZ`, `D_801305E4` |
| The new file's run (`sandbox-audio-log.bat`) | The same route, frame for frame; 4052 requests: 1083 sounds (356 grass footsteps, 166 ground, 66 ladder, 34 water, 25 concrete, 299 metal jingles with them as `func_800F4010` adds at a run, jumps, landings, voices, 40 passes, 18 ends, 17 decides, the chest's item sound, Navi's and Zelda's text sounds) and the C's 2969 silent ones; 33166 library commands (18488 without the sound effects) |
| The window | The device plays as before (the queue never ran dry). Not yet heard by hand |
| Golden traces and renders | 84 of 84 identical |

### Decisions

- **[ADR 0027](adr/0027-sound-effects.md):** the C's pointers as named sources (`SfxPos`,
  `SfxF32`, `SfxS8`), compared as the pointers are and read when the C reads them; positions
  through `projectedPos`, resolved once per `Audio_Update`; the engine whole; Player's sounds
  as requests in their order; the tables in the pack, the named ids checked against them.
- **The C's silent calls stay** (`Audio_PlaySfxGeneral(0, ...)`: the typing, the box breaks):
  they fill the request ring as on the console and play nothing.

### Known gaps

- **Not wired yet:** the other actors' sounds (`En_Box`'s lid, `En_Door`, `Bg_Treemouth`,
  `En_Elf`'s, `En_Kusa` and `En_Ishi`, `En_Item00` and the rupees, `En_Wonder_Item`,
  `En_Goroiwa`'s hit), the HUD's (`z_parameter.c`: the hearts, the low-health alarm, the
  rupees counting), `z_play.c`'s (the viewpoint's zoom, the error sound), the collision check's
  (`z_collision_check.c`: the shield, the sword's strikes), `Audio_SetBaseFilter`'s bubble.
- **Player:** 104 sites in its 71 functions not ported (the items, the shield, bottles, the
  ocarina, the boomerang and the hookshot, swimming's strokes, Epona, the cutscene modes'
  voices) and, in ported ones, the water's (`func_8083CFA8`'s splash check isn't ported), the
  masks', the lens', the hover boots'.
- **Not exercised:** the surround mode's stereo bits and filter (the sound mode is stereo, as a
  fresh SRAM's), the headset, the swap table (the debug screen's), the river and waterfall
  helpers (no `En_River_Sound` yet).
- **By hand:** not yet heard (`game.bat`, `sandbox-audio-log.bat`'s WAV).

## Recommended next step

Listen first: `scripts\run\game.bat` (Kokiri Forest's music, Link's sounds; walk into a house
and out, talk to a Kokiri), `scripts\run\game-night.bat` (the night's ambience),
`scripts\run\sandbox-audio-log.bat` (the whole new-file run as a WAV). Then milestone 3 on:
the actors' sounds the ported code marks as left out (`En_Box`, `En_Door`, `Bg_Treemouth`,
`En_Elf`, the bushes and rocks, `En_Item00`), the HUD's (`z_parameter.c`, the low-health
alarm), `z_play.c`'s and the collision check's; Player's water sounds with `func_8083CFA8`;
and for the phase's exit, a scripted run's sound effect log checked against the C's calls.
