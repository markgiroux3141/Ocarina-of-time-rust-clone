# 0024: Audio in the pack: the ROM's three audio files as they are, with the tables the game reads; fonts relocated, sequences interpreted and samples decoded at runtime, as the console does

- **Status:** accepted, built in GAME-04 milestone 1; extends ADR 0008 (the pack)
- **Date:** 2026-09-30

## Context

The game's sound is three ROM files and a few tables:
- `Audiobank` (175 KB): 38 soundfonts. A font is a block of offsets: lists of instruments,
  drums and sound effects, each pointing at envelopes and at `Sample` headers, which point at
  loops (with their ADPCM predictor states) and codebooks in the font, and at sample data in a
  sample bank.
- `Audiotable` (4.3 MB): 7 sample banks of VADPCM data (364 samples at 4 bits a sample, 86 at
  2; every book has order 2).
- `Audioseq` (319 KB): 110 sequences, the scripts the sequence player runs.
- In `code`: `gSoundFontTable`, `gSequenceTable`, `gSampleBankTable` (`AudioTable`s) and
  `gSequenceFontTable`; and the C's own tables (`audio/internal/data.c`: the pitch and bend tables, the
  pan volumes, the wave samples, the filters; `session_config.c`: the 18 audio specs and their
  reverbs).

`oot_extract::audio` already parses all of it, decodes the samples (a reference decoder, any
order) and runs the sequences to MIDI. The runtime could take either:
1. **Decoded at import:** fonts as typed records, samples as PCM, sequences as bytes.
2. **As the ROM has them:** the files' bytes, relocated, interpreted and decoded when played,
   by a port of the C (`audio/internal/load.c`, `seqplayer.c`, `synthesis.c`) and of the
   microcode's `aADPCMdec`.

What decides it:
- **The loops' predictor states.** When a looped sample restarts, the synthesis doesn't decode
  the frame that holds the loop's start: `aSetLoop` gives the microcode the loop's stored
  `predictorState` as that frame's output, and decoding goes on from the next frame
  (`AudioSynth_ProcessNote`'s restart). Decoding at import would replace the stored state with a
  decoded one. Of the 116 looped samples, 115 store exactly what decoding gives (checked by the
  extractor, and by this milestone's test through the microcode). The 116th, font 37's
  instrument 0 (bank 0 at 0), can't be decoded ahead of time at all: 1828 of its 2638 frames name
  predictors 2 and 3 of a book that has 2. On the RSP those frames read whatever coefficients
  the last `aLoadADPCM` left in DMEM past the book, so what they decode to depends on what
  played before; its stored state matches neither decode.
- **The C is all pointers.** `AudioLoad_RelocateFont` rewrites a font's offsets into addresses
  where it was loaded, and everything after reads through them: a layer's envelope can point
  into the font or into the sequence (`0xCB`, `0xDA`), sequences write into themselves (`0xC7`,
  `0xCF`), channel filters live in the sequence and the microcode writes back into them. A
  sample's medium decides whether its data is read from RAM or DMA'd through 77 small buffers
  (`AudioLoad_DmaSampleData`), and running out of buffers silences notes. Typed records would
  have to stand in for all of it.
- **Size:** the decoded samples would be about 17 MB (the extractor's WAVs) against 4.3 MB.
- **Speed:** decoding as it plays is cheap: the first 51 s of Kokiri Forest's music render
  offline in under half a second (a release build), with the reverbs.

## Decision

- **The pack holds the ROM's three audio files as they are** (`eng_audio::RomFile`,
  `audio/rom/Audiobank`, `audio/rom/Audioseq`, `audio/rom/Audiotable`), each with its ROM address,
  and **`audio/tables`** (`eng_audio::AudioTables`):
  - the four tables from the ROM, where the decomp's `data/audio_tables.rodata.s` says they are;
  - `audio/internal/data.c`'s tables and `session_config.c`'s specs, read from the C (`csrc`, with
    `DEFAULT_REVERB_SETTINGS` and `SAMPLE_SIZE` expanded), `sizeof(gAudioHeap)` from
    `src/buffers/heaps.c` and the init sizes' `#define`s;
  - the microcode's resampler filters, from the ROM's `aspMainData` (`data/rsp.rodata.s`;
    ADR 0025);
  - the 0x11000 bytes of `code` from `AudioThread_Update`, which `gWaveSamples[8]` reads as noise.
- **Nothing is decoded or parsed at import.** The runtime loads fonts, sequences and banks as
  the C does, from a cartridge of those files into a simulated RDRAM (`eng_audio::ram`: KSEG0
  addresses, so `AudioLoad_RelocateSample`'s `<= K0BASE` test still tells offsets from
  pointers), and the microcode decodes the samples as notes play.
- **The reference decoder moves to the engine** (`eng_audio::adpcm`, from `oot_extract`), with
  a typed font reader (`eng_audio::font`) for tools and tests. `oot_extract` uses it now.
- Pack format 13.

## Consequences

- The pack grows by 4.1 MB (59.6 to 63.7 MB); the audio phase of the import takes no time.
- The microcode's decoder is checked against the reference decoder on every sample of every font
  (450), bit for bit, and on every loop's restart from its stored state (115); font 37's sample
  is compared up to its first frame past the book.
- The game's sound can't be read from the pack without the library (the font layout is the
  ROM's): tools go through `eng_audio::font`.
- Custom sounds (the mod track) would need VADPCM encoding, or the `CODEC_S16` path the C
  already has.
