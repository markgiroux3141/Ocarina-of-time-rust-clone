//! `Bg_Ydan_Sp` (`ovl_Bg_Ydan_Sp/z_bg_ydan_sp.c`): the Deku Tree's webs, DynaPoly actors of
//! `object_ydan_objects`.
//!
//! Params: bits 0..5 the switch flag set once it's destroyed (`isDestroyedSwitchFlag`: a
//! destroyed web isn't spawned again), bits 6..11 the switch flag that burns a wall web
//! (`burnSwitchFlag`), bits 12..15 the type (`BgYdanSpType`), which the init leaves in `params`.
//!
//! - **Floor webs** (`WEB_FLOOR`, `gDTWebFloorCol`) bounce under Link (`BgYdanSp_FloorWebIdle`):
//!   a landing's `2 √fallDistance`, or 2 while he walks on it, swings it on a 14-frame sine
//!   (`unk_16C` its height, decaying by up to 0.8 a frame), `NA_SE_EV_WEB_VIBRATION` at a swing's
//!   start while it's over 3. Landed on from a fall of more than 750 within 80 of its centre, it
//!   breaks (`BgYdanSp_FloorWebBreaking`): it sinks on a 40-frame sine of height 200, and past
//!   190 down its collision goes, the chime plays, its flag is set, six puffs of dust rise and
//!   Link falls through; it's drawn broken for 40 frames (`BgYdanSp_FloorWebBroken`, eight
//!   strands of `gDTUnknownWebDL`). It's out of the rooms then (room -1): the room change Link's
//!   fall starts doesn't take it.
//! - **Its collision sags with it** (`BgYdanSp_UpdateFloorWebCollision`): each frame it writes
//!   eight vertices' y of `gDTWebFloorCol`, the rim, so that the rim stays at home while the rest
//!   moves with the web. The C writes the object's header itself, which every floor web points
//!   at (rooms 0 and 3 each have one): here that header is the overlay's static on the play state
//!   ([`Statics`]), and a write reaches every bg actor made from it
//!   (`Dyna::replace_shared_header`). `DynaPoly_UpdateContext` reads it again for a bg actor only
//!   when its transform changed, as the C does. `object_ydan_objects` is every room's first
//!   object, so it isn't reloaded on a room change, and the writes stay till the scene changes
//!   (a new play state). The last write of a web that broke stays in the header: the rim 190 up.
//!   No two floor webs are ever loaded at once in the Deku Tree (rooms 0 and 3 are only both
//!   loaded while Link falls between them, after the one in room 0 has stopped writing), and a
//!   floor web writes its own sag before each of its expansions, so the sharing doesn't show.
//! - **Wall webs** (`WEB_WALL`, `gDTWebWallCol`) burn (`BgYdanSp_WallWebIdle`) on their burn
//!   flag (room 0's top floor switch, 0x27, for the web over room 10's door) or a hit by fire
//!   (their tris take `DMG_ARROW_FIRE | DMG_MAGIC_FIRE`), or at a burning Deku Stick's tip within
//!   100 across their face, behind it (local z under 1) and under 200 up: that starts one-point
//!   cutscene 3020.
//! - **Burning** (`BgYdanSp_BurnWeb`): the chime, the destroyed flag, then 30 frames with six
//!   blue-white flames (`EffectSsDeadDb`) every third frame from where it caught fire
//!   (`BgYdanSp_BurnFloorWeb`, `BgYdanSp_BurnWallWeb`), and it's gone. A floor web burns from a
//!   fire hit or a burning stick under it (`Player_IsBurningStickInRange`).
//!
//! The whole overlay is ported. Link can't hold a Deku Stick yet (GAME-05 milestone 4b), so the
//! stick's checks are reached only by the tests, which set what Player holds.

use std::f64::consts::PI;
use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_INTERACT_PLAYER_ON_TOP, DYNA_TRANSFORM_POS};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_zero_f, cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::sys_matrix::MtxF;

use crate::player::Player;

/// `ACTOR_BG_YDAN_SP` (`actor_table.h`: 0x000F).
pub const ACTOR_BG_YDAN_SP: i16 = 0x000F;
pub const OBJECT: &str = "object_ydan_objects";
const FLOOR_COLLISION: &str = "gDTWebFloorCol";
const WALL_COLLISION: &str = "gDTWebWallCol";

/// `Bg_Ydan_Sp_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_YDAN_SP, name: "Bg_Ydan_Sp", category: ACTORCAT_BG, flags: 0, object: OBJECT };

/// `BgYdanSpType`.
pub const WEB_FLOOR: i16 = 0;
pub const WEB_WALL: i16 = 1;

/// `NA_SE_EV_WEB_VIBRATION`, `NA_SE_EV_WEB_BROKEN` (`environmentbank_table.h`: 0x2861, 0x2862).
pub const NA_SE_EV_WEB_VIBRATION: u16 = 0x2861;
pub const NA_SE_EV_WEB_BROKEN: u16 = 0x2862;

/// The vertices of `gDTWebFloorCol` `BgYdanSp_UpdateFloorWebCollision` writes the y of, in its
/// order.
pub const FLOOR_WEB_COL_VERTICES: [usize; 8] = [14, 12, 10, 9, 6, 5, 1, 0];

/// The webs' bakes (docs/adr/0012-actor-bakes.md): each list alone after `Gfx_SetupDL_25Xlu`
/// (the bake's start), drawn translucent.
pub const BAKE_FLOOR: &str = "Bg_Ydan_Sp/floor";
pub const BAKE_WALL: &str = "Bg_Ydan_Sp/wall";
pub const BAKE_STRAND: &str = "Bg_Ydan_Sp/strand";

/// `gDTWebFloorDL`, `gDTWebWallDL` and `gDTUnknownWebDL` (the broken floor web's strands).
pub fn bakes() -> Vec<MeshBake> {
    let bake = |name: &str, dl: &str| MeshBake { name: name.into(), object: OBJECT.into(), segments: vec![], prelude: vec![], body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]) };
    vec![bake(BAKE_FLOOR, "gDTWebFloorDL"), bake(BAKE_WALL, "gDTWebWallDL"), bake(BAKE_STRAND, "gDTUnknownWebDL")]
}

/// `sTrisElementsInit`'s element: hit only by fire (`DMG_ARROW_FIRE | DMG_MAGIC_FIRE`,
/// 0x00020800).
const TRIS_ELEMENT_INFO: ColliderElementInit = ColliderElementInit {
    elem_material: ELEM_MATERIAL_UNK0,
    at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
    ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0002_0800, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
    at_elem_flags: ATELEM_NONE,
    ac_elem_flags: ACELEM_ON,
    oc_elem_flags: OCELEM_NONE,
};

/// `sTrisElementsInit`: the floor web's triangle 8 under it, and the wall web's 280 wide and
/// 288 high (its top's two y differ in the C: 288.8 and 288.0).
pub fn tris_elements_init() -> [ColliderTrisElementInit; 2] {
    [
        ColliderTrisElementInit { info: TRIS_ELEMENT_INFO, vtx: [Vec3::new(75.0, -8.0, 75.0), Vec3::new(-75.0, -8.0, 75.0), Vec3::new(-75.0, -8.0, -75.0)] },
        ColliderTrisElementInit { info: TRIS_ELEMENT_INFO, vtx: [Vec3::new(140.0, 288.8, 0.0), Vec3::new(-140.0, 288.0, 0.0), Vec3::new(-140.0, 0.0, 0.0)] },
    ]
}

/// `sTrisInit`.
pub const TRIS_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_TRIS };

/// The overlay's shared state: `gDTWebFloorCol` as the floor webs have written it (the C
/// writes the object's header in place; see the module's doc). Loaded by the first floor web.
#[derive(Debug, Default)]
pub struct Statics {
    pub floor_web_col: Option<Arc<CollisionHeader>>,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FloorWebIdle,
    FloorWebBreaking,
    FloorWebBroken,
    BurnFloorWeb,
    WallWebIdle,
    BurnWallWeb,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::FloorWebIdle => "BgYdanSp_FloorWebIdle",
            Action::FloorWebBreaking => "BgYdanSp_FloorWebBreaking",
            Action::FloorWebBroken => "BgYdanSp_FloorWebBroken",
            Action::BurnFloorWeb => "BgYdanSp_BurnFloorWeb",
            Action::WallWebIdle => "BgYdanSp_WallWebIdle",
            Action::BurnWallWeb => "BgYdanSp_BurnWallWeb",
        }
    }
}

pub struct BgYdanSp {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub action: Action,
    pub is_destroyed_switch_flag: u8,
    pub burn_switch_flag: u8,
    pub timer: i16,
    /// `unk_16C`: how far the floor web swings (its sine's height).
    pub unk_16c: f32,
    pub collider_tris: ColliderTris,
}

/// `Math_Vec3f_DistXZ`.
fn math_vec3f_dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `sinf((f32)timer * (M_PI / n))`: the product in double precision, as the C's `M_PI` is.
fn timer_sin(timer: i16, n: f64) -> f32 {
    ((timer as f32 as f64 * (PI / n)) as f32).sin()
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `GET_PLAYER(play)`.
fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

impl BgYdanSp {
    /// `BgYdanSp_Init`: scale 0.1 (`sInitChain`); the flags and the type from the params; the
    /// tris, the floor web's flat 8 under it and the wall web's upright on its face; the floor
    /// or wall collision (`DYNA_TRANSFORM_POS`); gone at once if its destroyed flag is set.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // ICHAIN_VEC3F_DIV1000(scale, 100).
        actor.scale = Vec3::splat(0.1);
        let params = actor.params as u16;
        let is_destroyed_switch_flag = (params & 0x3F) as u8;
        let burn_switch_flag = ((params >> 6) & 0x3F) as u8;
        actor.params = ((params >> 12) & 0xF) as i16;
        let tri_inits = tris_elements_init();
        let mut collider_tris = ColliderTris::new(&TRIS_INIT, &tri_inits);
        let pos = actor.world_pos;
        let col_header;
        let action;
        if actor.params == WEB_FLOOR {
            col_header = Self::floor_web_col(play);
            action = Action::FloorWebIdle;
            let v = tri_inits[0].vtx;
            let mut tri = [Vec3::ZERO; 3];
            for i in 0..3 {
                tri[i] = Vec3::new(v[i].x + pos.x, v[i].y + pos.y, v[i].z + pos.z);
            }
            collider_tris.set_vertices(0, tri[0], tri[1], tri[2]);
            tri[1].x = tri[0].x;
            tri[1].z = tri[2].z;
            collider_tris.set_vertices(1, tri[0], tri[2], tri[1]);
        } else {
            col_header = Self::load_collision(play, WALL_COLLISION);
            action = Action::WallWebIdle;
            actor.set_focus(30.0);
            let sins_y = sin_s(actor.shape_rot.y);
            let coss_y = cos_s(actor.shape_rot.y);
            let n_sins_x = -sin_s(actor.shape_rot.x);
            let coss_x = cos_s(actor.shape_rot.x);
            let v = tri_inits[1].vtx;
            let mut tri = [Vec3::ZERO; 3];
            for i in 0..3 {
                tri[i].x = pos.x + (coss_y * v[i].x) - (sins_y * v[i].y * n_sins_x);
                tri[i].y = pos.y + (v[i].y * coss_x);
                tri[i].z = pos.z - (sins_y * v[i].x) + (v[i].y * coss_y * n_sins_x);
            }
            collider_tris.set_vertices(0, tri[0], tri[1], tri[2]);
            tri[1].x = pos.x + (coss_y * v[0].x) - (v[2].y * sins_y * n_sins_x);
            tri[1].y = pos.y + (v[2].y * coss_x);
            tri[1].z = pos.z - (sins_y * v[0].x) + (v[2].y * coss_y * n_sins_x);
            collider_tris.set_vertices(1, tri[0], tri[2], tri[1]);
        }
        // DynaPolyActor_Init(DYNA_TRANSFORM_POS), DynaPoly_SetBgActor.
        let bg = match col_header {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), DYNA_TRANSFORM_POS),
            None => BG_ACTOR_MAX,
        };
        let mut w = BgYdanSp { actor, bg, action, is_destroyed_switch_flag, burn_switch_flag, timer: 0, unk_16c: 0.0, collider_tris };
        if play.flags.get_switch(w.is_destroyed_switch_flag as i32) {
            w.actor.kill();
        }
        Box::new(w)
    }

    /// `CollisionHeader_GetVirtual(&col, ..)`: one of the object's headers from the pack.
    fn load_collision(play: &PlayState, symbol: &str) -> Option<Arc<CollisionHeader>> {
        let r = play.assets.as_ref()?.pack.collision(&keys::collision(OBJECT, symbol));
        r.map(Arc::new).map_err(|e| log::error!("Bg_Ydan_Sp: {e:#}")).ok()
    }

    /// `gDTWebFloorCol` as the floor webs share it: the overlay static, loaded on first use.
    fn floor_web_col(play: &mut PlayState) -> Option<Arc<CollisionHeader>> {
        if let Some(h) = &play.overlay_static::<Statics>(ACTOR_BG_YDAN_SP).floor_web_col {
            return Some(h.clone());
        }
        let h = Self::load_collision(play, FLOOR_COLLISION)?;
        play.overlay_static::<Statics>(ACTOR_BG_YDAN_SP).floor_web_col = Some(h.clone());
        Some(h)
    }

    /// `BgYdanSp_UpdateFloorWebCollision`: the rim's eight vertices of the shared
    /// `gDTWebFloorCol` at `(home.y - world.y) * 10` (model units: scale 0.1), so they stay at
    /// home as the web sinks.
    fn update_floor_web_collision(&mut self, play: &mut PlayState) {
        let new_y = ((self.actor.home_pos.y - self.actor.world_pos.y) * 10.0) as i32 as i16;
        let Some(old) = play.overlay_static::<Statics>(ACTOR_BG_YDAN_SP).floor_web_col.clone() else { return };
        if FLOOR_WEB_COL_VERTICES.iter().all(|&i| old.vertices.get(i).is_none_or(|v| v[1] == new_y)) {
            // Writing the values already there.
            return;
        }
        let mut h = (*old).clone();
        for &i in &FLOOR_WEB_COL_VERTICES {
            if let Some(v) = h.vertices.get_mut(i) {
                v[1] = new_y;
            }
        }
        let new = Arc::new(h);
        play.overlay_static::<Statics>(ACTOR_BG_YDAN_SP).floor_web_col = Some(new.clone());
        play.col.dyna.replace_shared_header(&old, new);
    }

    /// `BgYdanSp_BurnWeb`: 30 frames of burning; the chime and the destroyed flag at once.
    fn burn_web(&mut self, play: &mut PlayState) {
        self.timer = 30;
        play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
        play.flags.set_switch(self.is_destroyed_switch_flag as i32);
        self.action = if self.actor.params == WEB_FLOOR { Action::BurnFloorWeb } else { Action::BurnWallWeb };
    }

    /// `BgYdanSp_BurnFloorWeb`: every third frame, six flames from where it caught fire (`home`)
    /// out towards points 120 round it a sixth of a turn apart (a random start, each ±0x1400),
    /// the near ones turned round; gone when the timer runs out.
    fn burn_floor_web(&mut self, play: &mut PlayState) {
        // static Vec3f accel = { 0 }.
        let accel = Vec3::ZERO;
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.actor.kill();
            return;
        }
        if self.timer % 3 == 0 {
            let mut rot2 = (play.rand.zero_one() * 10922.0) as i32 as i16;
            let mut velocity = Vec3::ZERO;
            let mut pos2 = Vec3::new(0.0, self.actor.world_pos.y, 0.0);
            for _ in 0..6 {
                let rot = (play.rand.centered_float(10240.0) + rot2 as f32) as i32 as i16;
                let mut sins = sin_s(rot);
                let mut coss = cos_s(rot);
                pos2.x = self.actor.world_pos.x + (120.0 * sins);
                pos2.z = self.actor.world_pos.z + (120.0 * coss);
                let mut dist_xz = math_vec3f_dist_xz(self.actor.home_pos, pos2) * (1.0 / 120.0);
                if dist_xz < 0.7 {
                    sins = sin_s(rot.wrapping_add(i16::MIN));
                    coss = cos_s(rot.wrapping_add(i16::MIN));
                    pos2.x = self.actor.world_pos.x + (120.0 * sins);
                    pos2.z = self.actor.world_pos.z + (120.0 * coss);
                    dist_xz = math_vec3f_dist_xz(self.actor.home_pos, pos2) * (1.0 / 120.0);
                }
                velocity.x = (7.0 * sins) * dist_xz;
                velocity.y = 0.0;
                velocity.z = (7.0 * coss) * dist_xz;
                let home = self.actor.home_pos;
                play.with_ss(|s| s.dead_db_spawn(home, velocity, accel, 60, 6, [255, 255, 150, 170], [255, 0, 0], 1, 0xE, 1));
                rot2 = rot2.wrapping_add(0x2AAA);
            }
        }
    }

    /// `BgYdanSp_FloorWebBroken`: drawn broken until the timer runs out.
    fn floor_web_broken(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.actor.kill();
        }
    }

    /// `BgYdanSp_FloorWebBreaking`: sinking on `unk_16C`'s (200) sine of 40 frames; past 190 down
    /// its collision goes, 40 frames broken, the chime and its flag, and six puffs of dust in a
    /// ring of 60, 60 under it (`func_8002829C`).
    fn floor_web_breaking(&mut self, play: &mut PlayState) {
        // static Color_RGBA8 primColor, envColor; static Vec3f zeroVec.
        let prim_color = [250, 250, 250, 255];
        let env_color = [180, 180, 180, 255];
        let zero_vec = Vec3::ZERO;
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.world_pos.y = (timer_sin(self.timer, 20.0) * self.unk_16c) + self.actor.home_pos.y;
        if self.actor.home_pos.y - self.actor.world_pos.y > 190.0 {
            play.col.dyna.set_collision_disabled(self.bg, true);
            self.timer = 40;
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            play.flags.set_switch(self.is_destroyed_switch_flag as i32);
            self.action = Action::FloorWebBroken;
            let mut pos = Vec3::new(0.0, self.actor.world_pos.y - 60.0, 0.0);
            let mut rot: i16 = 0;
            for _ in 0..6 {
                pos.x = sin_s(rot) * 60.0 + self.actor.world_pos.x;
                pos.z = cos_s(rot) * 60.0 + self.actor.world_pos.z;
                play.with_ss(|s| s.func_8002829c(pos, zero_vec, zero_vec, prim_color, env_color, 1000, 10));
                rot = rot.wrapping_add(0x2AAA);
            }
        }
        self.update_floor_web_collision(play);
    }

    /// `BgYdanSp_FloorWebIdle`: a burning stick just under it, or a fire hit, burns it; Link on
    /// it (`DynaPolyActor_IsPlayerOnTop`) from a fall over 750 within 80 of its centre breaks it;
    /// else his landing (`2 √fallDistance`, over 2) or his walking (2) swings it, on a 14-frame
    /// sine that loses up to 0.8 a frame, the vibration at each swing's start while it's over 3.
    fn floor_web_idle(&mut self, play: &mut PlayState) {
        let web_pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y - 50.0, self.actor.world_pos.z);
        let Some(p) = player(play) else { return };
        let (tip, fall_distance, speed) = (p.melee_weapon_info[0].tip, p.fall_distance, p.actor.speed_xz);
        if p.is_burning_stick_in_range(web_pos, 70.0, 50.0) {
            self.actor.home_pos.x = tip.x;
            self.actor.home_pos.z = tip.z;
            self.burn_web(play);
            return;
        }
        if self.collider_tris.base.ac_flags & AC_HIT != 0 {
            self.burn_web(play);
            return;
        }
        if play.col.dyna.interact_flag(self.bg, DYNA_INTERACT_PLAYER_ON_TOP) {
            let sqrt_fall_distance = (fall_distance as f32).max(0.0).sqrt();
            if fall_distance as f32 > 750.0 && self.actor.xz_dist_to_player < 80.0 {
                self.unk_16c = 200.0;
                self.actor.room = -1;
                self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                self.timer = 40;
                audio_play_actor_sfx2(play, NA_SE_EV_WEB_BROKEN);
                self.action = Action::FloorWebBreaking;
                return;
            }
            let unk = sqrt_fall_distance + sqrt_fall_distance;
            if self.unk_16c < unk && unk > 2.0 {
                self.unk_16c = unk;
                self.timer = 14;
            }
            if speed != 0.0 {
                if self.unk_16c < 0.1 {
                    self.timer = 14;
                }
                self.unk_16c = self.unk_16c.max(2.0);
            }
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.timer = 14;
        }
        self.actor.world_pos.y = timer_sin(self.timer, 7.0) * self.unk_16c + self.actor.home_pos.y;
        approach_zero_f(&mut self.unk_16c, 1.0, 0.8);
        if self.timer == 13 {
            if self.unk_16c > 3.0 {
                audio_play_actor_sfx2(play, NA_SE_EV_WEB_VIBRATION);
            } else {
                play.audio.stop_sfx_by_id(NA_SE_EV_WEB_VIBRATION);
            }
        }
        self.update_floor_web_collision(play);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider_tris);
    }

    /// `BgYdanSp_BurnWallWeb`: as the floor web's, the flames from where it caught fire out
    /// towards points on a circle of 140 in its plane, round its middle 140 up.
    fn burn_wall_web(&mut self, play: &mut PlayState) {
        // static Vec3f accel = { 0 }.
        let accel = Vec3::ZERO;
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.actor.kill();
            return;
        }
        if self.timer % 3 == 0 {
            let mut rot2 = (play.rand.zero_one() * 10922.0) as i32 as i16;
            let yaw = self.actor.shape_rot.y;
            let mut sp_c8 = Vec3::ZERO;
            for _ in 0..6 {
                let rot = (play.rand.centered_float(10240.0) + rot2 as f32) as i32 as i16;
                let mut sins = sin_s(rot);
                let mut coss = cos_s(rot);
                let mut coss2 = cos_s(yaw) * sins;
                sins *= sin_s(yaw);
                sp_c8.x = self.actor.world_pos.x + (140.0 * coss2);
                sp_c8.y = self.actor.world_pos.y + (140.0 * (1.0 + coss));
                sp_c8.z = self.actor.world_pos.z - (140.0 * sins);
                let mut dist_xyz = self.actor.home_pos.distance(sp_c8) * (1.0 / 140.0);
                if dist_xyz < 0.65 {
                    sins = sin_s(rot.wrapping_add(i16::MIN));
                    coss = cos_s(rot.wrapping_add(i16::MIN));
                    coss2 = cos_s(yaw) * sins;
                    sins *= sin_s(yaw);
                    sp_c8.x = self.actor.world_pos.x + (140.0 * coss2);
                    sp_c8.y = self.actor.world_pos.y + (140.0 * (1.0 + coss));
                    sp_c8.z = self.actor.world_pos.z - (140.0 * sins);
                    dist_xyz = self.actor.home_pos.distance(sp_c8) * (1.0 / 140.0);
                }
                let velocity = Vec3::new(6.5 * coss2 * dist_xyz, 6.5 * coss * dist_xyz, -6.5 * sins * dist_xyz);
                let home = self.actor.home_pos;
                play.with_ss(|s| s.dead_db_spawn(home, velocity, accel, 80, 6, [255, 255, 150, 170], [255, 0, 0], 1, 0xE, 1));
                rot2 = rot2.wrapping_add(0x2AAA);
            }
        }
    }

    /// `BgYdanSp_WallWebIdle`: its burn flag or a fire hit burns it from 80 up its middle; a
    /// burning stick's tip within 100 across its face, behind it and under 200 up burns it from
    /// the tip, with one-point cutscene 3020 on it for 40 frames.
    fn wall_web_idle(&mut self, play: &mut PlayState) {
        let Some(p) = player(play) else { return };
        let (held_item_ap, unk_860, tip) = (p.held_item_ap, p.unk_860, p.melee_weapon_info[0].tip);
        if play.flags.get_switch(self.burn_switch_flag as i32) || self.collider_tris.base.ac_flags & AC_HIT != 0 {
            self.actor.home_pos.y = self.actor.world_pos.y + 80.0;
            self.burn_web(play);
        } else if held_item_ap == crate::player::PLAYER_IA_DEKU_STICK && unk_860 != 0 {
            let sp30 = self.actor.world_to_actor_coords(tip);
            if sp30.x.abs() < 100.0 && sp30.z < 1.0 && sp30.y < 200.0 {
                let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
                play.onepoint_cutscene_init(3020, 40, me, CAM_ID_MAIN);
                self.actor.home_pos = tip;
                self.burn_web(play);
            }
        }
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider_tris);
    }
}

impl ActorImpl for BgYdanSp {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgYdanSp_Update`, and the new transform for `DynaPoly_UpdateContext` (after the BG
    /// category).
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::FloorWebIdle => self.floor_web_idle(play),
            Action::FloorWebBreaking => self.floor_web_breaking(play),
            Action::FloorWebBroken => self.floor_web_broken(),
            Action::BurnFloorWeb => self.burn_floor_web(play),
            Action::WallWebIdle => self.wall_web_idle(play),
            Action::BurnWallWeb => self.burn_wall_web(play),
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgYdanSp_Destroy`.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    /// `DynaPolyActor_IsPlayerOnTop` reads the interact flags `Actor_UpdateAll` clears after it.
    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.values = vec![self.actor.home_pos.y - self.actor.world_pos.y];
        rs.switches = vec![(self.action == Action::FloorWebBroken) as u32, (self.timer == 40) as u32];
        rs
    }

    /// `BgYdanSp_Draw`, translucent: the wall web as placed; the floor web moved up by its sag
    /// (model units) and stretched by a tenth of it plus 1, so that its rim stays at home; broken,
    /// that floor web on its first frame only, and eight strands round it, an eighth of a turn
    /// apart, tilted out 0x5A0.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([sag], [broken, first_frame]) = (rs.values.as_slice(), rs.switches.as_slice()) else { return };
        // Actor_Draw's matrix.
        let mut mtx_f = MtxF::set_translate_rotate_yxz(rs.pos.x, rs.pos.y + rs.y_offset * rs.scale.y, rs.pos.z, rs.rot);
        mtx_f.scale(rs.scale.x, rs.scale.y, rs.scale.z);
        let push = |out: &mut DrawOut, name: &str, m: &MtxF| out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(name)), m.to_mat4()));
        if self.actor.params == WEB_WALL {
            push(out, BAKE_WALL, &mtx_f);
        } else if *broken != 0 {
            if *first_frame != 0 {
                let mut m = mtx_f;
                m.translate(0.0, sag * 10.0, 0.0);
                m.scale(1.0, (sag + 10.0) * 0.1, 1.0);
                push(out, BAKE_FLOOR, &m);
            }
            for i in 0..8 {
                // Matrix_Put(&mtxF).
                let mut m = mtx_f;
                m.rotate_zyx(-0x5A0, (i * 0x2000) as i16, 0);
                m.translate(0.0, 700.0, -900.0);
                m.scale(3.5, 5.0, 1.0);
                push(out, BAKE_STRAND, &m);
            }
        } else {
            let mut m = mtx_f;
            m.translate(0.0, sag * 10.0, 0.0);
            m.scale(1.0, (sag + 10.0) * 0.1, 1.0);
            push(out, BAKE_FLOOR, &m);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Tris(&mut self.collider_tris))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
