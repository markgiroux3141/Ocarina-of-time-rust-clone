# GAME-04: audio

**Goal:** Phase 5 of [ROADMAP.md](ROADMAP.md): music and sound effects, from the ROM through the
pack.

| # | Milestone | Status |
|---|---|---|
| 1 | The import and the synth: the audio data in the pack, `eng_audio` (the audio library and its microcode, offline and through an output device), the mixer's ADR | done |
| 2 | The game's music: the scenes' sound settings in the pack, the sequence commands (`sequence.c`), the scene's music and the ambience (`general.c`, `Environment_PlaySceneSequence`), the boundary between the game's thread and the audio thread | done |
| 3 | Sound effects: `Audio_PlaySfxGeneral` and the sfx channels, the calls the ported code marks as left out | done: the engine whole, Player, the actors, the HUD, the camera, the collision check, the phase's exit test |

**Phase exit:** Kokiri Forest's music plays and loops like the game, and a scripted run's sound
effects log matches the calls in the C. **Met: Phase 5 is done** (milestone 3).

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
     `data/audio_tables.rodata.s` puts them; `audio/internal/data.c`'s tables (the wave samples,
     `gPitchFrequencies`, the bend tables, the pan volumes, the Haas delays, the default envelope
     and short-note tables, the filters, `D_8012FBA8`) and `session_config.c`'s 18 audio specs
     with their reverbs, read from the C (`oot_import::audio`); `gAudioHeap`'s size and the init
     sizes' `#define`s; `gTempoData.seqTicksPerBeat`;
   - the microcode's resampler filters, from the ROM's `aspMainData` at 0xE0 (`data/rsp.rodata.s`);
   - the 0x11000 bytes of `code` from `AudioThread_Update` that `gWaveSamples[8]` reads as noise.
2. **`eng_audio`, the audio library** (engine layer; `cargo test -p layering` knows it, and only
   it may use `cpal`), ported function by function ([ADR 0025](adr/0025-audio-mixer.md)):
   - `context`: `gAudioCtx` and every struct of `audio.h`; notes, layers, channels and
     players as arrays linked by index; the C's `AudioListItem` lists as an arena of nodes, in
     the C's order; `SeqScriptState` as its 0x1C bytes, so the unchecked call depth lands where
     the C's does;
   - `ram`, `layout`: a simulated RDRAM with KSEG0 addresses and a cartridge of the three files;
     `Sample`, `Instrument`, `Drum`, `AdpcmLoop`, `AdpcmBook` and `EnvelopePoint` read and
     written where they are;
   - `heap` (`heap.c`): the pools, the persistent and temporary caches, the sample caches,
     the ADSR decay table, the filters, the reset steps, `AudioHeap_Init`;
   - `load` (`audio/internal/load.c`): the sample DMAs (`AudioLoad_DmaSampleData` and its two reuse
     queues), the sync, async, slow and script loads, the preloads, font relocation,
     `AudioLoad_Init`;
   - `seqplayer` (`seqplayer.c`): the flow control, the layers' five steps, the channels'
     and players' interpreters, player init and reset;
   - `effects` (`effects.c`): the channels' and players' volume, pan and bend, portamento,
     vibrato, `Audio_AdsrUpdate`;
   - `playback` (`playback.c`): `Audio_ProcessNotes`, `Audio_InitSampleState`, note allocation
     and its policies, the synthetic waves;
   - `synthesis` (`synthesis.c`): the command list of a frame: each note's decode,
     resample, gain, filter, envelope mixer and Haas effect, the reverbs' ring buffers, the
     interleave;
   - `thread` (`audio/internal/thread.c`): `AudioThread_UpdateImpl` (one audio frame per retrace), the command
     queue (`AudioThread_QueueCmd*`, `AudioThread_ScheduleProcessCmds`, `AudioThread_ProcessCmds`), the library's,
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

- **The game's side** (`general.c`, `sfx.c`, `sequence.c`,
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
commands into the library's, as `sequence.c` and `general.c` do. Kokiri Forest
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
   expanded, `gSoundOutputModes`. `ootx scene-info` prints a layer's sound settings.
2. **The boundary** ([ADR 0026](adr/0026-the-games-audio.md), `eng_audio::link`): what the
   game's thread does to the library (`GameOp`: commands, schedules, the ring's rewind, the spec
   change) is applied in order by the audio side (`AudioContext::apply`) before its next
   retrace; what the game reads of it (`AudioView`: the players' `enabled`, `tempo` and IO
   ports, the channels' IO ports and note priorities, `updatesPerFrame`, the reset and load
   queues' messages, the notes sounding) comes back after. `AudioThread_ResetAudioHeap`'s audio side;
   `AudioTables::fonts_for_sequence` (`AudioLoad_GetFontsForSequence`); `AudioView::boot`.
   Offline: `Renderer::game_frame`; the window: `AudioOutput::send_ops` and `take_view`.
3. **The game's side, ported whole** (`oot_game::audio`, the game layer):
   - `seqcmd` (`sequence.c`): `Audio_QueueSeqCmd`, `Audio_ProcessSeqCmd(s)` (every op:
     the starts and stops, the players' queues, the volume, frequency and tempo fades, the IO
     ports, the channel masks, the setup commands, the sound mode, the spec change),
     `Audio_StartSequence`, `Audio_StopSequence`, `Audio_GetActiveSeqId`, `Audio_IsSeqCmdNotQueued`, `Audio_SetVolumeScale`,
     `Audio_UpdateActiveSequences`, `func_800FAD34`, the resets;
   - `bgm` (`general.c`'s sequences): `Audio_Update`,
     `Audio_PlaySceneSequence` (the scene's music, resumed through `sSeqResumePoint` and port 7),
     `Audio_UpdateSceneSequenceResumePoint`, `Audio_SetSequenceMode` and the enemy music, `Audio_SplitBgmChannels`,
     `Audio_PlayFanfare` and `Audio_UpdateFanfare`, the mini-boss and ambience swaps and their
     restores, the river and Ganon's Tower volumes, `func_800F6964` (the fade on leaving),
     `Audio_PlayNatureAmbienceSequence`, `Audio_StartNatureAmbienceSequence`,
     `Audio_SetNatureAmbienceChannelIO`, `Audio_InitSound`, `func_800F6C34`, `func_800F7170`,
     `func_800F71BC`, the reverbs and filters' setters;
   - `scene` (`z_scene.c`, `z_kankyo.c`): `Scene_CommandSoundSettings`,
     `Environment_PlaySceneSequence` (the Lost Woods' bridge, the forced sequence, no music,
     no ambience, by day, by night), `Environment_PlayTimeBasedSequence` (every state),
     `Environment_ForcePlaySequence`;
   - `offline`: `OfflineAudio`, the headless audio side.
4. **Wired into play:** the boot (`Audio_InitSound`, then the title screen's `Audio_SetSoundOutputMode` with a fresh SRAM's sound setting, stereo: the title and the file select aren't ported, so their own music isn't either); `Play_Init` (`Audio_SetExtraFilter(0)`, the sound settings,
   `Environment_Init`'s `TIMESEQ_DAY_BGM`, `Environment_PlaySceneSequence` and the save's
   `seqId`/`natureAmbienceId`), `GameAudio` carried over a scene change
   (`PlayState::play_init_with`), the transition's fade-out unless the next entrance continues
   the music, `Environment_Update`'s time of day, `Audio_Update` at each frame's end and as a
   game state ends, the room change's reverb (`Room_FinishRoomChange`), `Audio_SetCutsceneFlag` in
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
| Kokiri Forest from a new game (`oot_actors --test music`) | `Play_Init` queues, as the C does, `F0000001` (spec 1, `seqId` disabled), `700700FF` and `0000003C` (`Audio_PlaySceneSequence`: port 7 to 0xFF, `sSeqFlags[NA_BGM_GENERAL_SFX]` lacking `SEQ_FLAG_RESUME_PREV`); `Audio_InitSound`'s `46000000 FF000000` and `82020000 7` come first, and the title's sound mode (`E0000000`: `Audio_SetSoundOutputMode(0)`, stereo). The first `Audio_Update` gives `F0000000 0` (stereo), the reset to spec 1, `46000000`, `F8`, `46000007`, `82003C00 0`, a schedule, and `Audio_UpdateActiveSequences`'s four `4p01007F`. Then nothing for the 6 game frames of the reset (18 audio frames), then `func_800FAD34`'s `46020000` and `func_800F7170`. The forest's sequence plays on player 0, the sound effects' 0 on player 2; after 1212 game frames, 7014 ticks (past the loop at 5952), 735 notes, still sounding in the last 5 s |
| From Link's house into the forest | The exit: `func_800F6964(0x14)`'s `101E00FF`, `111E00FF`, 15 sfx channel fades (all but the ocarina's), `131E00FF`; the forest: `F0000001`, `70070000`, `0000003C` (from `NA_BGM_LINK_HOUSE`'s `SEQ_FLAG_RESUME_PREV` to the forest's `SEQ_FLAG_RESUME`: port 7 from `sSeqResumePoint`) |
| Kokiri Forest at 20:00 | `Audio_PlayNatureAmbienceSequence(NATURE_ID_KOKIRI_REGION)`'s commands from `sNatureAmbienceDataIO[4]`, then the night's critters (`TIMESEQ_NIGHT_CRITTERS`: critter 0 off, 1 to 3 on, while the start is still queued); `NA_BGM_NATURE_AMBIENCE` plays, its channels hold their critters' types; 91 notes in 20 s |
| The chests (`oot_actors --test chest`) | The Kokiri Sword's chest: `0101092B` (`NA_BGM_OPEN_TRE_BOX \| 0x900`), then `01010922` (`NA_BGM_ITEM_GET \| 0x900`); a second heart piece: `01010039` (`NA_BGM_SMALL_ITEM_GET`) |
| The boundary (`oot_game --test audio`) | `AudioView::boot`'s `updatesPerFrame` 3 is `AudioLoad_Init`'s; `AudioThread_ResetAudioHeap`'s paths: the reset runs over 18 audio frames and posts its spec; the same spec again: -2; another early: -3, switched; another late: the reset finishes first |
| The pack (`oot_import --test pack`) | The game's audio tables are the importer's reading of the C (`sSeqFlags` by the rows' `NA_BGM_*`, `sSpecReverbs` 40 and 15, the general night's ambience, `gSoundOutputModes`); five scenes' sound settings per layer are the ROM headers'; Kokiri Forest by day: spec 1, `NATURE_ID_KOKIRI_REGION`, `NA_BGM_KOKIRI` |
| The new file's run with sound (`sandbox-audio-log.bat`) | The same route, frame for frame (11748 frames), with 587 s of sound: the opening's layers start what their headers name, Link's house 0x1F, the forest 0x3C, Mido's house 0x1F and back (resumed: `001E003C`), the shop 0x55 and back (resumed), the Deku Tree 0x1C with spec 3; the chest's fanfare and the sword's and shield's item fanfares on player 1, the music fading under them and back. 4945 sequence commands, 18488 library commands; 7.5 s with the audio offline |
| The window | No `--music`: the device plays the forest's sequence on player 0 and the sound effects' on player 2; the queue held 1961 to 2021 frames over 20 s and never ran dry. Not yet heard by hand |
| Import | 12.5 s; 63.7 MB; format 14, into `out/data11` |
| Golden traces and renders | 84 of 84 identical |

### Decisions

- **[ADR 0026](adr/0026-the-games-audio.md):** the game's thread and the audio thread meet at
  the end of each game frame: the frame's `GameOp`s in, applied in order before the next
  retrace; an `AudioView` back, read the whole next frame; the queues' messages kept until
  received. `AudioThread_ResetAudioHeap` split between the two; its blocking wait finishes the reset at
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
  `CS_CMD_START_SEQ`, `_STOPBGM`, `_FADEBGM` do nothing.
- **Time doesn't pass**, so the time of day's music stays in the state a scene starts in; no
  weather, so the rain's checks always pass. The day's count, the cucco's crow and the egg's
  hatching at dawn wait for the clock.
- **Not ported here:** the ocarina (`AudioOcarina_*`), the debug screen (`AudioDebug_*`),
  `Audio_PlaySariaBgm` (no caller yet), the pause menu's mute (`func_800F64E0` is ported, its
  caller isn't), the enemy music (`targetCtx.bgmEnemy` is never set: no enemies), game over's
  music, the bottle catch's fanfare; `Interface_ChangeHudVisibilityMode(1)` in the transition's setup.
- **The approximations** (ADR 0026): the spec change's wait, the game's reads at frame
  boundaries.
- **By hand:** the window's music hasn't been heard by ear yet (`game.bat`, `game-night.bat`,
  `sandbox-audio-log.bat`'s WAV).

## Milestone 3: sound effects

**Answer:** done, and with it Phase 5. The sound effects' engine is ported whole and the game's
calls are wired: Link's actions, his water, the message box, the HUD, the targeting, the
camera, the collision check's strikes, and every ported actor (the chests and the chest's light,
doors, Navi, bushes, rocks, signs, rupees and the other drops, the wonder items, the boulder,
the shopkeeper, Mido, the Kokiri, the Deku Tree's mouth, the forest's waterfall). A request goes
into its bank, the banks' entries are chosen by priority every frame and started on the sound
effects' sequence's channels, refreshed while they're asked for, and let go when the channel
says they ended.

**Phase exit, met:** the scripted runs' sound effect requests match the C's calls, frame by
frame, with where each sound is (`oot_actors --test sfx_route`), and the Mido and shop run's
audio log is a golden (`mido_shop_audio`). Kokiri Forest's music plays and loops like the game
(milestone 2).

The tests: 282 pass, 1 ignored (279 at the milestone's start). The goldens: 85 hashes, 61
cases; 4 traces re-recorded (an actor count only) and 1 new (golden/README.md).

### What was built

The first part (the milestone's start, committed in f82e932):

1. **The tables in the pack** (`table/audio`, still format 14): `gSfxParams` from the seven
   bank tables (`include/tables/sfx/*.h`, 1259 rows with their names), the banks' sizes
   (`gSfxBanks`' arrays), `gChannelsPerBank`, `gUsedChannelsPerBank`, `gIsLargeSfxBank`,
   `sBehindScreenZ`, `sSfxSwordChargeFreqLevels`, `sSurfaceMaterialToSfxOffset` (the floors' footsteps, `z_bgcheck.c`), and
   `z_player.c`'s 40 `AnimSfxEntry` tables (the animations' sounds); `sGanonsTowerLevelsVol`
   moved here from the code.
2. **The engine** ([ADR 0027](adr/0027-sound-effects.md), `oot_game::audio::sfx`):
   `sfx.c` whole (`Audio_PlaySfxGeneral` and its swap table,
   `Audio_ProcessSfxRequest(s)`, the banks' lists, `Audio_ChooseActiveSfx`,
   `Audio_PlayActiveSfx`, `Audio_RemoveSfxBankEntry`, every `Audio_StopSfx*`,
   `Audio_IsSfxPlaying`, `Audio_ResetSfx`, the bgm mutes, the unused bank lerps);
   `general.c`'s sound effect parts (`Audio_ComputeSfxVolume`, `_Reverb`, `_PanSigned`,
   `_FreqScale`, `func_800F37B8`, `func_800F3990`, `Audio_SetSfxProperties`, `func_800F3F84`,
   `func_800F4010` and the other helpers, the river and the waterfall, the transposed ones);
   `z_lib.c`'s `Sfx_PlaySfxCentered`, `Sfx_PlaySfxCentered2`, `Sfx_PlaySfxAtPos`; `AudioMgr_StopAllSfx`.
3. **Positions:** `projectedPos` and `projectedW` on every actor, from `Actor_DrawAll`'s spot in
   the frame; `actor->sfx` and its setters (`Actor_PlaySfx_Flagged2`...), played there
   (`Actor_UpdateFlaggedAudio`); cleared by `Actor_UpdateAll`; `Actor_Delete` stops the actor's sounds.
4. **The boundary** (ADR 0026): the view carries `audioRandom`, `sAudioRandom` and the count
   register's value; `GameOp::SetAudRand` sends the game's `AudioThread_NextRandom` back.
5. **The message box:** `Message_ShouldAdvance`'s `NA_SE_SY_MESSAGE_PASS`, the choice cursor,
   `NA_SE_SY_MESSAGE_END`, the text's own sound codes, `Message_Update`'s `NA_SE_SY_DECIDE`, and
   the C's silent (id 0) calls.
6. **Player's helpers and most of its actions** (`PlayRequest::Sfx`): the voice, footsteps,
   jumps, landings, the animations' tables, the ladder, the floor's echo; walking, jumping,
   landing, the roll, being hurt and knocked down, ledges, climbing, the crawl, the sword's
   swing, the chest, the item sounds, the voids.

The rest (this session):

7. **Sound sources** (`z_sfx_source.c`, `oot_game::sfx_source`): the play state's sixteen
   fixed-position sources (`SfxSource_PlaySfxAtFixedWorldPos`, `_UpdateAll` in `Play_Update`'s
   order, `_InitAll`), positioned through `SfxPos::Source` (ADR 0027's pointer naming).
8. **`z_actor.c`'s helpers:** `Actor_PlaySfx`, `Player_PlaySfx`, `Actor_PlaySfx_SurfaceBomb` (a
   bounce: the bomb's and the floor's), `Actor_PlaySfx_FlaggedTimer` (the timer's tick); the lock-on and
   lock-off (`Attention_Update`, `Actor_UpdateAll`: `NA_SE_SY_LOCK_ON` or `_HUMAN`, `_LOCK_OFF`).
9. **The actors:**
   - `En_Box`: the lid's bounce when a chest falls, the appearing chest, the unlock and the lid
     on the opening's frames 30 and 90, the mimic's breath;
   - `Demo_Tre_Lgt` (`oot_actors::demo_tre_lgt`, new): the big chest's light, its logic whole
     (the curve animation's frames, its alphas, the flash `NA_SE_EV_TRE_BOX_FLASH` past frame
     30, its end); its draw (a curve skeleton) isn't;
   - `En_Door`: the open and close on `sDoorAnimOpenFrames` and `sDoorAnimCloseFrames` (the
     iron ones in the Fire and Shadow Temples), the chain lock's unlock; Player's scene-exit
     door leaves `gSaveContext.entranceSound`, which the next scene's `Player_Init` plays;
   - `En_Elf`: Navi's dash, vanish, hello, enemy and hear calls (silenced by `unk_2C7` as the C
     does), the opening's dashes, the healing fairies' sound, the talk's laugh
     (`func_800F4524`);
   - `En_Kusa`, `En_Ishi`: the cut bush and the broken rock through sound sources at them;
   - `En_Item00`: `NA_SE_SY_GET_RUPY` or `_GET_ITEM` as Link takes one; `En_Wonder_Item`'s
     drops' `NA_SE_SY_GET_ITEM`;
   - `En_Goroiwa`: the rolling loop and Link's body hit (`Player_PlaySfx` at Player);
   - `En_Kanban`: the sword's strike, the pieces' bounces, the splash;
   - `En_Ossan`: every one of its 23 calls (the cursor, the passes, decide, the errors);
     `En_Md`'s correct chime, `En_Ko`'s chest-appear chime, `Bg_Treemouth`'s wooden door;
   - `En_River_Sound` (`oot_actors::en_river_sound`, new, whole): the rivers along their paths
     (`EnRiverSound_GetSfxPos`, the current's frequency from the floor's conveyor speed,
     `Audio_PlaySfxRiver`), the waterfalls, lava, torches and the others where they stand, the
     sandstorm and the rumbling at no position, the bgm lowered at the market, Saria's Song in
     the Lost Woods and Goron City and the Great Fairy's music (`func_800F4E30`,
     `Audio_PlaySariaBgm`, `Audio_ClearSariaBgm*`, with `sSariaBgmPtr` as a named pointer),
     the nature ambience and Ganon's Tower's levels at init. Its sounds are its draw's, made
     where `Actor_DrawAll` makes them (`ActorImpl::draw_sfx`, after the actor's
     `projectedPos`). Kokiri Forest has its small waterfall and the Kokiri houses a torch.
10. **The game's other calls:**
    - the HUD (`z_parameter.c`, `z_lifemeter.c`): the rupees counting in and out
      (`NA_SE_SY_RUPY_COUNT`), the hearts filling (`NA_SE_SY_HP_RECOVER`), the low-health alarm
      (`Health_UpdateBeatingHeart`, `Health_IsCritical`), Navi's call and hello
      (`Interface_SetNaviCall`); `Health_ChangeBy`'s recovery sound, which `Item_Give` and the
      actors now reach through the play state's audio (`Option<&mut GameAudio>`: none for the
      debug presets and `Play_Init`'s triggers);
    - `z_play.c`: the viewpoint's zoom (`Play_SetViewpoint`) and the shop's error on C-Up;
    - `z_camera.c`: the mode changes' sounds (`Camera_RequestModeImpl`: the attention sounds,
      first person's error) and the crawl's steps (`Camera_Subj4`), queued by the camera
      (`CamSfx`) and played by the play state where the C plays them;
    - `z_collision_check.c`: `CollisionCheck_HitEffects`' sounds (the sword's strikes by the
      element's type, the shield's bounce, metal's and wood's), returned by the check in order;
    - `general.c`: `Audio_SetBaseFilter` (the underwater filter and its bubbling),
      `func_800F64E0`'s window sounds;
    - `z_kankyo.c`: the evening's dog and the morning's cucco (time doesn't pass yet).
11. **Player's water** (`func_8083CFA8`'s splash check on the scene's water boxes,
    `func_8083D0A8`, `func_8083D12C`, `func_8083D36C`, `func_8083D53C`): diving in and jumping
    out, the dive's bubbles, surfacing, the underwater filter (`underwaterTimer`); the swim strokes
    (`func_8084D530`, `D_808549D0`); and in the ported functions left: the falls' voices
    (`Player_Action_8084411C`, `func_80843E14`), the surfacing breath (`Player_Action_8084E1EC`), the 15-step
    climb out of water (`Player_Action_80845668`), the recovery sound for a gain.
12. **A fix to the first part:** a new bank entry took its position at the next `Audio_Update`
    (the entry's slot kept its last occupant's); the positions are now read just before
    `func_800F8F88`, after the requests are taken in, as the C reads the new pointer.
13. **The log** (`GameAudio::log`, not in the C) keeps each request's position; the sandbox's
    `--audio-log` writes it (`default`, `source <i>`, `actor <n>` numbered as they appear).
14. **`ootx sfx <id|name|part>`**: a sound effect's row in the pack's tables, and the constant
    to paste. The `NA_SE_*` constants (127) are generated from the names the code uses, each
    checked against its table row by the pack's test (`NAMED_SFX`).
15. **The exit test** (`oot_actors --test sfx_route`), every expectation from the C's arithmetic:
    the Mido and shop run (the sword's chest, Mido's chests, the rupees taken and counted, the
    wonder items' drops, Navi), two doors (a Kakariko house's exit, a room door), and the Deku
    Tree run's bushes; and the golden `mido_shop_audio`.
16. **Run scripts:** `test-sfx-route.bat`, `game-door.bat`, `sandbox-mido-shop-audio.bat`,
    `ootx-sfx.bat` (menu 32 to 35); `_env.bat` on `game12`.

### Results

| Check | Result |
|---|---|
| `cargo test --workspace` | 282 passed, 1 ignored |
| The sword's chest (`sfx_route`, the Mido and shop run) | It starts opening on frame 1676 (Player's A sets `unk_1F4` 1): the chest fanfare's start (`start_seq(SEQ_PLAYER_FANFARE, 1, NA_BGM_OPEN_TRE_BOX \| 0x900)`) on that frame; `NA_SE_EV_TBOX_UNLOCK` on 1696 and `NA_SE_EV_TBOX_OPEN` on 1736 (frames 30 and 90 at 1.5 a frame: 20 and 60 on), at the chest; the light's `NA_SE_EV_TRE_BOX_FLASH` on 1698 (it reads the chest's frame 10 eight frames on, starts at 12 and passes 30 fourteen frames later: 22 on), at the light; the item's fanfare once, with its text |
| Mido's four chests | Kicked open: the lid's sounds as far as each animation goes, at each chest; their rupees' `NA_SE_SY_GET_BOXITEM`, at no position, in place of a fanfare |
| Rupees and drops | Every frame of 5732: `NA_SE_SY_GET_RUPY` exactly when Link takes a rupee (7 by hand; the switch's drop is taken by its first update, in the frame it's spawned), `NA_SE_SY_GET_ITEM` exactly when a wonder item drops (the ordered multitag touched out of order goes without), `NA_SE_SY_RUPY_COUNT` on exactly the frames the wallet counts (in, and the shield's 40 out) |
| Navi | `NA_SE_EV_NAVY_VANISH` at her on exactly the frames she goes from following into Link's hat: 9 on the run |
| Doors (`doors_sound_as_the_c`) | The Kakariko house's exit: `NA_SE_OC_DOOR_OPEN` at the door 17 frames after it starts opening (frame 25 at 1.5), then in Kakariko Village once, at the new Player (`entranceSound`), and `entranceSound` cleared. The souko's room door: open at 17 frames, `NA_SE_EV_DOOR_CLOSE` at 47 (frame 70 for `DOOR_OPEN_ANIM_CHILD_R`), both at the door, nothing else |
| Bushes (the Deku Tree run) | 4 bushes cut, each `NA_SE_EV_PLANT_BROKEN` on its frame through a sound source at the bush's position, its countdown 19 at the frame's end (20, less `SfxSource_UpdateAll`'s) |
| Kokiri Forest's music test | The first update after the reset now also starts the small waterfall's sound (`En_River_Sound`, the only request so far): the music's commands are unchanged |
| The Mido and shop run's audio log (`sandbox-mido-shop-audio.bat`) | 6420 requests (the C's silent ones included), 1339 sequence commands, 32200 library commands; the same bytes over two runs: the golden `mido_shop_audio` |
| Golden traces and renders | `sword_chest`, `mido_shop`, `new_save_deku_tree`, `new_file_deku_tree`: their `actors` count only, one fewer from 238 frames after the sword's chest starts opening until room 2 unloads (the light goes at its animation's end; its placeholder stayed). Every other field and every render the same bytes |
| The window | Heard by hand at the milestone's start (Link, the message box). The rest: to try (below) |

### Decisions

- **[ADR 0027](adr/0027-sound-effects.md)** carries this half: new pointers get new names. The
  sound sources are `SfxPos::Source(i)`; `sSariaBgmPtr` (a `Vec3f*` into an actor read
  immediately) is `SariaPos` (`Projected`, `Home`) with a reader for what it points at.
- **An actor's draw-time sounds run where `Actor_DrawAll` makes them**
  (`ActorImpl::draw_sfx`, right after the actor's `projectedPos` and `sfx`): `En_River_Sound`'s.
- **Code with no audio of its own queues the C's sounds in order** and the play state plays
  them right after: the camera (`CamSfx`), the collision check (its return value), Player
  (`PlayRequest::Sfx`, now with `BaseFilter`). Everything else calls `play.audio`.
- **`Item_Give` and `Health_ChangeBy` take the audio** as `Option<&mut GameAudio>`: the C's
  `play`; `None` where there's no play state (the presets, `Play_Init`'s triggers, tests).
- **`Player_Init`'s entrance sound** plays where the port spawns Player in `Play_Init`, at
  Player, since the port's init doesn't know Player's handle.
- **`Demo_Tre_Lgt` is ported for its logic**, since its flash is the chest's sound and its end
  an actor count; its curve skeleton's draw waits.

### Known gaps

- **Cutscenes** (deferred, BACKLOG #10): their scripts' music commands, the Deku Tree's death
  sound, the piece of heart's, the white-outs', and Player's cutscene voices
  (`func_80851E90`'s groan, `func_80851FB0`'s table).
- **Player:** 95 sound sites in 65 functions not ported (items, the shield, bottles, the
  ocarina, the boomerang and hookshot, Epona, swimming under water, the dives' and the deep
  water's...); in ported functions, the branches for what isn't ported: the items' use
  (`Player_UseItem`: the errors, the lens, the masks' `NA_SE_PL_CHANGE_ARMS`), first person
  (`Player_ActionHandler_13`, `Player_ActionHandler_0`'s C-Up error), the hookshot's lash (`Player_UpdateUpperBody`),
  being frozen or shocked (`func_80837C0C` kinds 3 and 4), the hover and iron boots
  (`func_8084029C`, `Player_Action_8084D610`), Ruto's cry when held (`func_80843E14`).
- **Actors:** lifting bushes and rocks (their pull-up sounds and the throws' landings); the
  healing fairy's sound (`EffectSsDeadSound`, an effect); the sign's ocarina repair; the
  chests that need the ocarina; a locked door's unlock is wired but no door locks here.
- **The pause menu:** `func_800F64E0` plays its window sounds now, but its caller is the pause
  menu's opening (`KaleidoSetup_Update`'s state 1), which the equipping stand-in doesn't open.
- **Effects** (`EffectSs`): the hit marks', sparks' and splashes' own sounds where they have any.
- **Not exercised:** the surround mode's stereo bits and filter, the headset, the swap table;
  rivers (none in the scenes the routes visit; the waterfall and the torches are).
- **By hand:** the actors' sounds in the window (below).

## Phase 5: done

Phase 5 (GAME-04) is complete: the audio library ported whole (milestone 1), the game's music
(milestone 2), and the sound effects (milestone 3), with the phase's exit met by the scripted
runs' requests checked against the C and an audio golden. The next phase is the Deku Tree
(Phase 6, [ROADMAP.md](ROADMAP.md)), on Master Quest, starting with the decomp upgrade
([ADR 0028](adr/0028-phase-6-master-quest-and-the-decomp-upgrade.md)).

## Recommended next step

Listen first:
- `scripts\run\game-sword-chest.bat`: the chest (the unlock, the light's flash, the lid, the
  fanfare), then Enter and E for the sword, and a sign or a bush nearby;
- `scripts\run\game.bat`: Kokiri Forest's small waterfall (`En_River_Sound` at (398, -29, -483)), the
  bushes (E cuts them), the rocks, Navi into Link's cap and out, Z on a Kokiri (the lock-on, her
  hello), rupees and their counting, the shop's cursor (`game-shop.bat`), the boulder in the
  training area, the ford's water (in and out);
- `scripts\run\game-door.bat`: a door, and its sound again on the far side;
- `scripts\run\sandbox-mido-shop-audio.bat`: the exit run as a WAV.

Then Phase 6's first milestone: the decomp upgrade (ADR 0028).
