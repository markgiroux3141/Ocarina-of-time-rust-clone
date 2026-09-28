//! The F3DEX2 display-list interpreter and TMEM texture decoding. Turns N64 display lists
//! into `eng_gfx` draw lists: at import time for everything static, and at runtime for the
//! procedural lists some draw code builds (docs/adr/0003-rendering-model.md).
//!
//! `gbi` and `texture` re-export the `eng_gfx` types they produce, so interpreter users can
//! name everything from one place.

pub mod gbi;
pub mod model;
pub mod texture;

pub use eng_gfx::combiner;
