//! Triangle-mesh collision ported from `z_bgcheck.c`: the static scene mesh and actor-owned
//! `DynaPoly` meshes behind one `CollisionContext`, water boxes, and raw surface data words
//! (their meaning belongs to the game).

pub mod bgcheck;
pub mod collision;
pub mod dyna;
pub mod math3d;
