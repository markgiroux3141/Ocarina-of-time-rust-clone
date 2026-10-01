# GAME-04: audio

**Goal:** Phase 5 of [ROADMAP.md](ROADMAP.md): music and sound effects, from the ROM through the
pack.

| # | Milestone | Status |
|---|---|---|
| 1 | The import and the synth: the audio data in the pack, `eng_audio` (the audio library and its microcode, offline and through an output device), the mixer's ADR | done |
| 2 | The sequence player at runtime: `audio_seqplayer.c` with the game's IO ports, the scenes' music (`Environment_PlaySceneSequence`, `code_800EC960.c`), the ambience | started: the sequence player is ported whole (milestone 1) and plays Kokiri Forest's sequence offline |
| 3 | Sound effects: `Audio_PlaySfxGeneral` and the sfx channels, the calls the ported code marks as left out | |

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

## Milestone 2: the sequence player at runtime (started)

Started in milestone 1: `audio_seqplayer.c` is ported whole, with the game's IO ports
(`soundScriptIO`) and the player and channel commands (`func_800E6128`, `func_800E6300`), and
plays Kokiri Forest's sequence offline and in the window.

Left for the milestone:
- `code_800EC960.c`'s sequence commands (`Audio_QueueSeqCmd`, `func_800F9280.c`'s processing:
  fades, `seqCmd`s) and the game's per-frame `Audio_Update` sending them to the audio thread;
- `Environment_PlaySceneSequence` and the scenes' sound settings in the pack (the spec, the
  nature ambience, the sequence), with the time of day's music;
- the nature ambience (`Audio_PlayNatureAmbienceSequence` on its player, its IO ports);
- **exit:** Kokiri Forest's music plays and loops in the game like the game, headless (an audio
  trace) and in the window.

## Recommended next step

Milestone 2's game side: the sequence commands and `Environment_PlaySceneSequence`, so Kokiri
Forest starts its own music; then the ambience. Ask the user to listen first
(`scripts\run\game-music.bat`, `scripts\run\audio-wav.bat`): the music is audible now.
