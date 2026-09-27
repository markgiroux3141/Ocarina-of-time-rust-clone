//! Game logic ported from the decomp, kept apart from rendering: N64 pad input, Link's
//! SkelAnime, Player movement, actor physics and static background collision.
//!
//! Everything runs at the game's 20 Hz logic rate (`math::GAME_HZ`). Units are the game's
//! world units; Link's model is drawn at actor scale 0.01.

pub mod actor;
pub mod bgcheck;
pub mod course;
pub mod data;
pub mod input;
pub mod math;
pub mod player;
pub mod skelanime;
pub mod world;
