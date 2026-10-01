# 0025: The mixer's faithfulness: the audio library ported whole, the microcode by its HLE, 32006 Hz at 60 retraces a second, offline as on the console; the device resampled at the end

- **Status:** accepted, built in GAME-04 milestone 1
- **Date:** 2026-09-30

## Context

On the console, sound is made in three places:
- **The audio thread** (`code_800E4FE0.c`'s `func_800E5000`, run by `AudioMgr_HandleRetrace` on
  every VI retrace): it runs the frame's sequence updates (`audio_seqplayer.c`,
  `audio_effects.c`, `audio_playback.c`), then builds an RSP command list for the frame
  (`audio_synthesis.c`), and hands the audio interface (AI) the buffer finished two frames ago.
- **The RSP's audio microcode** (`aspMain`), which runs the list: decodes the samples
  (`aADPCMdec`), resamples them to the output rate (`aResample`), mixes them into dry and wet
  channels with their volumes and ramps (`aEnvMixer`), runs the reverbs' ring buffers, and
  interleaves the frame into the AI buffer. Its code and data are `.incbin`'d from the ROM: the
  decomp doesn't say what the commands do.
- **The AI**, which plays the buffers at its DAC's rate.

The decomp has the C whole (about 18,000 lines); the microcode's behaviour has to come from
elsewhere. mupen64plus-rsp-hle's audio lists (`alist.c`, `alist_nead.c`,
`alist_process_nead_oot`) are what emulators play this game with.

## Decision

- **The C is ported whole and literally** (`eng_audio`): every function of `audio_heap.c`,
  `audio_load.c`, `audio_playback.c`, `audio_effects.c`, `audio_seqplayer.c`,
  `audio_synthesis.c` and the audio thread's part of `code_800E4FE0.c`, with the decomp's
  names, in the C's types (f32 maths in f32, the integer widths and their wraps). The heap's
  pools and caches are the C's, with the C's sizes, on the C's 0x38000-byte heap, so what fits
  is what fits on the console: on spec 0, 72 short-lived sample DMA buffers and only 5 of the 24
  long-lived ones. The data structures that live in RAM in the C live in a simulated RDRAM here
  (ADR 0024); notes, layers, channels and players are arrays linked by index, and the C's lists
  (`AudioListItem`) keep their order, which decides which note a layer gets.
- **The microcode is a high-level emulation of its commands** (`eng_audio::rsp`) on a 4 KB DMEM:
  - the commands are built word for word as `abi.h`'s macros build them and decoded as the
    microcode reads them (`aLoadBuffer`'s size in 16-byte units, `aEnvMixer`'s count in a
    byte, the DMA's 8-byte alignment);
  - what each does follows rsp-hle's `alist_process_nead_oot`, but for two points where the
    decomp says otherwise or the HLE skips a flag: `aClearBuffer` clears its size rounded up to
    16 bytes (`abi.h`'s comment), and `aFilter` with `A_INIT` starts from a zero history;
  - `A_S8DEC`, `A_UNK3` and `A_UNK19` do nothing, as in that table (no sample is S8; the two
    unknowns are what channels with `bookOffset` 2 and 3 add);
  - the resampler's 64 × 4 filter table is read from the ROM's `aspMainData` at 0xE0 (where
    rsp-hle's `RESAMPLE_LUT` is found in this ROM).
- **The rates and the cadence are the console's, on NTSC**, as the rest of the port runs its
  VI at 60 Hz:
  - `osAiSetFrequency(32000)`: 48681812 / 1521 = **32006 Hz**;
  - `refreshRate` 60: `samplesPerFrameTarget` 544, 3 updates a frame of about 176 samples,
    `tempoInternalToExternal` 10770 (`3 * 2880000 / 48 / 16.713`);
  - one audio frame per retrace, its three sequence updates before its synthesis
    (`AudioSynth_Update`), the AI handed the buffer of two frames before, and each frame's
    length chosen from `osAiGetLength`, which a model of the AI's queue answers (`thread::Ai`);
  - the game's commands (`Audio_QueueCmd*`, `Audio_ScheduleProcessCmds`) run at the next
    retrace, as the command queue does.
- **Offline is the console:** `Renderer` runs retraces and the AI model at 32006 Hz, and the
  output is what the AI would play, sample for sample. Tests and tools use it.
- **The device is the end of the line:** for the window app, a thread of its own runs the same
  retraces on the clock, every 1/60 s, and queues what the AI plays; the device's callback
  (cpal, behind `eng_audio`'s `device` feature) resamples it linearly to the device's rate, and
  keeps the queue near 64 ms by playing up to half a percent faster or slower. It starts once
  the queue holds its 64 ms, and waits for that again if the queue ever runs dry (one gap rather
  than a crackle). Measured in the game window with Kokiri Forest's music: the queue held 1969 to
  2012 frames over 15 s and never ran dry; started empty without the wait, 892 of the device's
  frames went unfilled in the first 5 s.

## What's approximated

- **The microcode is an HLE.** It isn't checked against the RSP itself (there's no RSP
  emulator here); it is checked against the C that builds the lists (the decoder against the
  reference decoder on every sample, the resampler's stepping, the mixer's volumes and ramps,
  the reverb's decay).
- **`osGetCount()`** (the CPU's count register, mixed into `audioRandom` and `Audio_NextRandom`)
  advances by 46875000 / 60 per retrace, so runs repeat; on the console it depends on timing,
  and differs run to run. It reaches the sequences' random commands (`0xB7`, `0xB8`, `0xBD`,
  `0xCE`), the velocity and gate variances, and where the noise wave reads.
- **The PI's DMAs complete at once**; an async load's chunk arrives a frame later, as its
  message would. Disk-drive media do nothing (as the game, which has no drive).
- **NTSC, not PAL**, although the ROM is a European one: the port runs at 60 VIs a second
  throughout (PAL would be 50, with `unk_2960` 20.03042).
- **The DAC and the analog stage** aren't modelled: the output is the AI's samples; the device's
  linear resampling and its rate nudging are the port's. The latency is about two frames (the
  console's own pipeline) plus the queue's 64 ms.
- **Undefined behaviour in the C** is kept where it's within the structs: a script's unchecked
  call depth (`0xE4`'s `@bug`) writes past `stack[3]` into `remLoopIters`, as the struct's bytes
  lie. A division by zero (a zero portamento time, a zero variance) is guarded.
- **`D_801755D0`** (a callback the game never sets) and the disk drive's async loads are left
  out.

## Consequences

- Sequences, notes, envelopes, vibrato, portamento, pan, reverb and the mute behaviours all
  work the C's way from the start: the game's side (which sequence plays when, the sound
  effects: `code_800EC960.c`, `code_800F7260.c`, `code_800F9280.c`) only has to send the C's
  commands.
- Tests can check sound against the C: exact volumes per update, pitch by correlation, note
  counts against the extractor's interpreter, the reverb's decay.
- The window app has sound; headless runs make none and don't open a device.
