//! Core decoding for the OoT clone spikes: ROM filesystem, decomp symbols, F3DEX2 display
//! lists, colour combiner, skeletons and animations. Contains no game data; everything is
//! read at runtime from a ROM the user supplies.

pub mod anim;
pub mod combiner;
pub mod gbi;
pub mod rom;
pub mod skeleton;
pub mod symbols;
pub mod texture;
pub mod yaz0;
pub mod model;
pub mod project;
pub mod csrc;
pub mod player;
pub mod synth;
pub mod collision;
pub mod scene;
pub mod drawcfg;
pub mod room;
