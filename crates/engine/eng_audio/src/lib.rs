//! The N64 audio library as the game ships it, ported function by function, and its RSP
//! microcode: what `audio_*.c` and the audio thread (`code_800E4FE0.c`) do on the console,
//! rendering into buffers offline (`Renderer`) and, with the `device` feature, through an
//! output device (`output`).
//!
//! - `context`: `gAudioContext` and the structs of `z64audio.h`;
//! - `heap`, `load`: `audio_heap.c` and `audio_load.c` (pools, caches, DMAs, font relocation);
//! - `seqplayer`, `effects`, `playback`: `audio_seqplayer.c`, `audio_effects.c`,
//!   `audio_playback.c` (the sequences, envelopes, vibrato, the notes);
//! - `synthesis`: `audio_synthesis.c`, the RSP command lists;
//! - `rsp`: the microcode, run on DMEM and RDRAM;
//! - `thread`: `func_800E5000` (one audio frame per VI retrace), the command queue, the AI;
//! - `ram`, `layout`: the memory it all runs in, and the structs that live there;
//! - `font`, `adpcm`, `wav`: reading a font's instruments and samples directly, the reference
//!   VADPCM decoder, and WAV files (for tools and tests).
//!
//! It knows nothing about Zelda's game: what plays when is the game's (`oot_game`). The data it
//! runs on comes from the pack (`data::AudioData`). The decisions are in
//! docs/adr/0024-audio-data.md and docs/adr/0025-audio-mixer.md.

pub mod adpcm;
pub mod context;
pub mod data;
pub mod effects;
pub mod font;
pub mod heap;
pub mod layout;
pub mod load;
#[cfg(feature = "device")]
pub mod output;
pub mod playback;
pub mod ram;
pub mod renderer;
pub mod rsp;
pub mod seqplayer;
pub mod synthesis;
pub mod thread;
pub mod wav;

pub use context::AudioContext;
pub use data::{AudioData, AudioTables, RomFile};
pub use renderer::Renderer;
