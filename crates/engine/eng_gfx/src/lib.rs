//! Engine graphics types. The material model is the N64's (colour combiner, othermode, tiles,
//! render mode), since every asset uses it; meshes are `DrawList`s of batches that share a
//! `Material`. No GPU code: `eng_render` draws these, `eng_gbi` builds them from display lists,
//! and the asset pack stores them.

pub mod combiner;
pub mod draw;
pub mod submit;
pub mod texture;

pub use draw::*;
pub use submit::{DrawCmd, DrawLists, DrawParams, FogOverride, LinePoint, MAX_POINT_LIGHTS, MeshKey, PointLight};
