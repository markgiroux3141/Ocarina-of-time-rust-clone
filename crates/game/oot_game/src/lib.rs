//! Game framework ported from the decomp's `src/code`, kept apart from rendering: `PlayState`
//! and the frame (`z_play.c`), the actor system and the actor base (`z_actor.c`), SkelAnime for
//! Link (`z_skelanime.c`), the camera (`z_camera.c`), the target context, environment lights
//! (`z_kankyo.c`), foot IK (`z_player_lib.c`), what
//! collision surface types mean, per-frame room logic, the scene draw configs, Player's draw
//! rules, the game data tables, and the synthetic test course. Actors (the overlays) are in
//! `oot_actors`.
//!
//! Everything the game needs from the ROM comes from the asset pack (`pack`): scenes and rooms
//! (`scene`), game tables, meshes, skeletons, animations and collision.
//!
//! Everything runs at the game's 20 Hz logic rate (`eng_math::GAME_HZ`). Units are the game's
//! world units; Link's model is drawn at actor scale 0.01.

pub mod actor;
pub mod actor_ctx;
pub mod actor_table;
pub mod camera;
pub mod collision_check;
pub mod course;
pub mod data;
pub mod env;
pub mod footik;
pub mod letterbox;
pub mod npc;
pub mod object_ctx;
pub mod pack;
pub mod play;
pub mod play_scene;
pub mod player_lib;
pub mod room;
pub mod save;
pub mod scene;
pub mod scene_table;
pub mod skelanime;
pub mod skelanime_std;
pub mod skybox;
pub mod spawn;
pub mod surface;
pub mod target;
pub mod transition;
