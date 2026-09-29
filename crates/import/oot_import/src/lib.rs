//! Reads the user's ROM with the decomp's help: the ROM filesystem (dmadata, Yaz0), the decomp
//! XML symbol index, the z64 asset decoders, scenes and rooms, Player's draw rules, the C
//! table reader (`csrc`) and the scene draw-config interpreter (`drawcfg`). Contains no game
//! data; everything is read from a ROM the user supplies.
//!
//! This is the import side of the engine/game split (docs/adr/0001-crate-layout.md). Until
//! the asset pack exists (milestone 2), the game still calls into it at startup; after that
//! the runtime never does, and `csrc` / `drawcfg` remain as import tools and test oracles.

pub mod background;
pub mod csrc;
pub mod cutscene;
pub mod drawcfg;
pub mod objects;
pub mod pack;
pub mod player;
pub mod project;
pub mod rom;
pub mod room;
pub mod scene;
pub mod symbols;
pub mod synth;
pub mod tables;
pub mod text;
pub mod yaz0;
pub mod z64;
