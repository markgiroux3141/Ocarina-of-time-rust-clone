//! `En_Bx` (`ovl_En_Bx/z_en_bx.c`): the electrified tentacles of Lord Jabu-Jabu's Belly (GAME-10
//! milestone 5), standing from the floor to the ceiling as barriers, gone once the tentacle
//! (`En_Ba`) of their colour is dead (its switch flag). `object_bxa`'s `object_bxa_DL_0022F0`, no
//! skeleton: a base and four segments 140 apart up from 100 under its place, pulsing.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places five in layers 0 to 3, all with their quad (bit 7):
//! red (0x0B80, the red tentacle's flag 0x0B) in room 1 at (484, -320, -1703) turned -16384 and
//! room 3 at (-531, -1015, -2785) turned 24576; green (0x0C81, flag 0x0C) in room 3 at (-560,
//! -1015, -3340) turned 16384; gray (0x0A82, flag 0x0A) in room 7 at (-655, -340, -4955) and
//! (-840, -340, -4685) turned 16384. The unused cutscene layer (4) has five 0x3FFF.
//!
//! - **Touching it** (`EnBx_Update`): Link within 70 across (at any height), or its cylinder (60
//!   across, 100 high from 100 up: hurting 4 with an electric shock, `0xFFCFFFFF`, and hit by
//!   Player's `0xFFCFFFFF` with an electric backlash) or its quad touching or touched by Link:
//!   knocked back (6 out, 6 up, no damage of its own) away from it (with the quad its facing
//!   `world.rot.y`, else straight away from it); near it with neither collider on him and not
//!   invincible, it costs him 4 (`play->damagePlayer`, his invincibility zeroed for it and put
//!   back after; at -40 or under, only zeroed). Any of that sparks it for 20 frames: each frame
//!   of the four corners a bolt (`EffectSsLightning`, white in pale cyan, 230 big, 6 frames) 170
//!   up, and `NA_SE_EN_BIRI_SPARK`.
//! - **Its quad** (bit 7; `func_809D1D0C`, in the draw): the barrier across it, from ±8000 by
//!   10000 to 15000 up, 2500 out (model units, `Actor_Draw`'s matrix): hurting 4 with an electric
//!   shock (`0xFFCFFFFF`).
//! - Each draw its width pulses (`unk_14E` down 0xBB8 a frame: `scale.x` and `.z` 0.015 ± 0.0075,
//!   each segment's 0x2000 later), `Actor_Draw`'s matrix taking the new width the next frame.
//!
//! The whole overlay is ported. Its draw (`EnBx_Draw`, `Gfx_SetupDL_25Opa`):
//! `object_bxa_DL_0022F0` with its colour's texture on segment 8 (`D_809D2560`: red, green, gray),
//! the scroll on 9 (`Gfx_TwoTexScroll`, the second tile down 10 a frame) and its four segments'
//! matrices on 0x0C, the base under `Actor_Draw`'s. One bake for each colour. `@bug (game)`: a
//! colour over 2 (the cutscene layer's 0x7F) reads past `D_809D2560`: not drawn here.
//!
//! Params: `color` bits 0 to 6 (0 red, 1 green, 2 gray); `hasQuad` bit 7 (its quad on, and the
//! knock-back along its facing); `switchFlag` bits 8 to 15 (set: gone at init; `params &= 0xFF`
//! there).
//! Flags: reads switch flag `switchFlag`.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::cos_s;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::shadow::ShadowDraw;
use oot_game::sys_matrix::MtxF;

/// `ACTOR_EN_BX` (`actor_table.h`: 0x00DF).
pub const ACTOR_EN_BX: i16 = 0x00DF;
/// `OBJECT_BXA` (`object_table.h`: 0x00D5).
pub const OBJECT: &str = "object_bxa";
/// `object_bxa_DL_0022F0` (`object_bxa.xml`).
pub const DL: &str = "object_bxa_DL_0022F0";
/// `D_809D2560`: the colours' textures.
pub const TEXTURES: [&str; 3] = ["object_bxa_Tex_0024F0", "object_bxa_Tex_0027F0", "object_bxa_Tex_0029F0"];
/// `NA_SE_EN_BIRI_SPARK` (`enemybank_table.h`: 0x389A).
pub const NA_SE_EN_BIRI_SPARK: u16 = 0x389A;

/// `En_Bx_Profile`: `ACTORCAT_ENEMY`, always updating, `OBJECT_BXA`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_BX, name: "En_Bx", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 7)`: the colour, an index into `D_809D2560`.
pub const COLOR: Field = Field::params("color", 0, 7, Kind::Range(0, 0x7F), "0 red, 1 green, 2 gray (higher reads past the textures: the cutscene layer's 0x7F)");
/// `PARAMS_GET_NOSHIFT(params, 7, 1)`: its quad.
pub const HAS_QUAD: Field = Field::params("hasQuad", 7, 1, Kind::Named(&[(0, "no quad, knock-back away from it"), (1, "its quad, knock-back along its facing")]), "the barrier's quad");
/// `PARAMS_GET_U(params, 8, 8)`: the flag that takes it down.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 8, Kind::SwitchFlag, "set at init, it's gone (its tentacle's)");

/// En_Bx's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[COLOR, HAS_QUAD, SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().note("set (its tentacle dead), it isn't there")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sCylinderInit`: `COL_MATERIAL_HIT6`; hurting 4 with an electric shock, hit by Player's
/// `0xFFCFFFFF` with an electric backlash; 60 by 100, 100 up.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_ELECTRIC, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 60, height: 100, y_shift: 100, pos: [0; 3] },
};

/// `sQuadInit`: hurting 4 with an electric shock.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// The colliders' ids.
pub const COL_CYLINDER: u8 = 0;
pub const COL_QUAD: u8 = 1;

/// `func_809D1D0C`'s corners (model units): `sp5C`, `sp50`, `D_809D2540`, `D_809D254C`.
const SP5C: Vec3 = Vec3::new(8000.0, 15000.0, 2500.0);
const SP50: Vec3 = Vec3::new(8000.0, 10000.0, 2500.0);
const D_809D2540: Vec3 = Vec3::new(-8000.0, 15000.0, 2500.0);
const D_809D254C: Vec3 = Vec3::new(-8000.0, 10000.0, 2500.0);

pub struct EnBx {
    pub actor: Actor,
    /// `unk_14C`: the sparks' frames left.
    pub unk_14c: i16,
    /// `unk_14E`: the pulse's phase.
    pub unk_14e: i16,
    /// `unk_154`: the segments' places.
    pub unk_154: [Vec3; 4],
    /// `unk_184`: their scales.
    pub unk_184: [Vec3; 4],
    /// `unk_1B4`: their turns.
    pub unk_1b4: [[i16; 3]; 4],
    pub collider: ColliderCylinder,
    pub collider_quad: ColliderQuad,
    /// The scale `Actor_Draw`'s matrix took this frame (the draw changes it after).
    pub drawn_scale: Vec3,
}

impl EnBx {
    /// `EnBx_Init`: see the module's docs.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 5300.
        actor.target_arrow_offset = 5300.0;
        actor.scale.x = 0.01;
        actor.scale.z = 0.01;
        actor.scale.y = 0.03;
        actor.world_pos.y -= 100.0;
        let mut unk_154 = [Vec3::ZERO; 4];
        let mut unk_1b4 = [[0i16; 3]; 4];
        for (i, p) in unk_154.iter_mut().enumerate() {
            if i == 0 {
                unk_1b4[i][0] = actor.shape_rot.x.wrapping_sub(0x4000);
            }
            *p = actor.world_pos;
            p.y = actor.world_pos.y + ((i as i32 + 1) as f32 * 140.0);
        }
        actor.shape_init(0.0, ShadowDraw::Circle, 48.0);
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        actor.culling_volume_downward = 2000.0;
        if play.flags.get_switch(SWITCH_FLAG.get(actor.params) as i32) {
            actor.kill();
        }
        // schema: params &= 0xFF, the colour and the quad kept
        actor.params &= 0xFF;
        let drawn_scale = actor.scale;
        Box::new(EnBx {
            actor,
            unk_14c: 0,
            unk_14e: 0,
            unk_154,
            unk_184: [Vec3::splat(0.015); 4],
            unk_1b4,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            collider_quad: ColliderQuad::new(&QUAD_INIT),
            drawn_scale,
        })
    }

    /// `PARAMS_GET_NOSHIFT(params, 7, 1)`.
    pub fn has_quad(&self) -> bool {
        HAS_QUAD.get(self.actor.params) != 0
    }

    /// `Actor_Draw`'s matrix (its place, `shape.yOffset` up, turned, scaled).
    fn draw_mtx(&self) -> MtxF {
        let a = &self.actor;
        let r = a.shape_rot;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [r.x, r.y, r.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }

    /// `func_809D1D0C`: the quad's corners under `Actor_Draw`'s matrix.
    pub fn set_quad(&mut self) {
        let m = self.draw_mtx();
        let sp44 = m.mult_vec3f(D_809D2540);
        let sp38 = m.mult_vec3f(D_809D254C);
        let q1 = m.mult_vec3f(SP5C);
        let q0 = m.mult_vec3f(SP50);
        self.collider_quad.set_vertices(sp38, sp44, q0, q1);
    }

    /// Segment `i`'s matrix (`EnBx_Draw`'s, absolute).
    pub fn segment_matrix(pos: Vec3, rot: [i16; 3], scale: Vec3) -> MtxF {
        let mut m = MtxF::set_translate(pos.x, pos.y, pos.z);
        m.rotate_zyx(rot[0], rot[1], rot[2]);
        m.scale(scale.x, scale.y, scale.z);
        m
    }
}

/// Player's `invincibilityTimer`.
fn invincibility_timer(play: &PlayState) -> i8 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.invincibility_timer())
}

/// `player->invincibilityTimer = t`.
fn set_invincibility_timer(play: &mut PlayState, t: i8) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.set_invincibility_timer(t);
    }
}

/// The bakes' segments: the colour's texture (8), the scroll (9), the segments' matrices (0x0C).
const SEG_TEX: u8 = 0x08;
const SEG_SCROLL: u8 = 0x09;
const SEG_MATRICES: u8 = 0x0C;

/// The barrier's bake for colour `i`.
pub fn bake_name(i: usize) -> String {
    format!("En_Bx/barrier{i}")
}

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, 0, 0, 16, 16, 1, 0, (gameplayFrames × -10) % 128, 32, 32)`.
pub fn scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 16, 16, 1, 0, frames.wrapping_mul((-10i32) as u32) % 128, 32, 32)
}

/// `object_bxa_DL_0022F0` in each colour.
pub fn bakes() -> Vec<MeshBake> {
    TEXTURES
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: bake_name(i),
            object: OBJECT.into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_SCROLL, BakeSegment::Dynamic(scroll(0))),
                (SEG_MATRICES, BakeSegment::Matrices(vec![0, 1, 2, 3])),
            ],
            prelude: vec![],
            body: BakeBody::DLists(vec![(OBJECT.into(), DL.into())]),
        })
        .collect()
}

impl ActorImpl for EnBx {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBx_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        let player = play.player;
        let is_player = |a: Option<oot_game::actor_ctx::ActorHandle>| a.is_some() && a == player;
        let near = self.actor.xz_dist_to_player <= 70.0;
        if near || self.collider.base.at_flags & AT_HIT != 0 || self.collider.base.ac_flags & AC_HIT != 0 || self.collider_quad.base.at_flags & AT_HIT != 0 {
            let touched = is_player(self.collider.base.at) || is_player(self.collider.base.ac) || is_player(self.collider_quad.base.at);
            if near || touched {
                // tmp33 = invincibilityTimer & 0xFF (put back as the s8 it was).
                let tmp33 = invincibility_timer(play);
                let tmp32 = if !self.has_quad() { self.actor.yaw_towards_player } else { self.actor.world_rot.y };
                if !touched && tmp33 <= 0 {
                    if tmp33 <= -40 {
                        set_invincibility_timer(play, 0);
                    } else {
                        set_invincibility_timer(play, 0);
                        // play->damagePlayer(play, -4).
                        crate::player::play_damage_player(play, -4);
                    }
                }
                actor_set_player_knockback_large_no_damage(play, 6.0, tmp32, 6.0);
                set_invincibility_timer(play, tmp33);
            }
            self.collider.base.at_flags &= !AT_HIT;
            self.collider.base.ac_flags &= !AC_HIT;
            self.collider_quad.base.at_flags &= !AT_HIT;
            self.collider_quad.base.at = None;
            self.collider.base.ac = None;
            self.collider.base.at = None;
            self.unk_14c = 0x14;
        }
        if self.unk_14c != 0 {
            // primColor, envColor.
            const PRIM_COLOR: [u8; 4] = [255, 255, 255, 255];
            const ENV_COLOR: [u8; 4] = [200, 255, 255, 255];
            self.unk_14c -= 1;
            let w = self.actor.world_pos;
            for i in 0..4i32 {
                if (self.unk_14c as i32 + (i << 1)) % 4 == 0 {
                    let yaw = play.rand.centered_float(12288.0) as i32 as i16;
                    let yaw = (yaw as i32 + (i * 0x4000) + 0x2000) as i16;
                    let x = play.rand.centered_float(5.0) + w.x;
                    let y = play.rand.centered_float(30.0) + w.y + 170.0;
                    let z = play.rand.centered_float(5.0) + w.z;
                    let pos = Vec3::new(x, y, z);
                    play.with_ss(|ss| ss.lightning_spawn(pos, PRIM_COLOR, ENV_COLOR, 230, yaw, 6, 0));
                }
            }
            audio_play_actor_sfx2(play, NA_SE_EN_BIRI_SPARK - SFX_FLAG);
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.collider.update(&self.actor);
        play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider);
        play.collision_check_set_at(&self.actor, COL_CYLINDER, &mut self.collider);
        if self.has_quad() {
            play.collision_check_set_at(&self.actor, COL_QUAD, &mut self.collider_quad);
        }
    }

    /// `EnBx_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnBx_Draw`'s changes: the quad (under the frame's `Actor_Draw` matrix), then the pulse:
    /// `unk_14E` down 0xBB8, the width, the segments' scales and turns.
    fn draw_update(&mut self, _play: &mut PlayState) {
        self.drawn_scale = self.actor.scale;
        if self.has_quad() {
            self.set_quad();
        }
        self.unk_14e = self.unk_14e.wrapping_sub(0xBB8);
        let w = (cos_s(self.unk_14e) * 0.0075) + 0.015;
        self.actor.scale.z = w;
        self.actor.scale.x = w;
        for i in (0..4).rev() {
            let off = (0x2000 * i as i32) as i16;
            let s = (cos_s((self.unk_14e as i32 + off as i32) as i16) * 0.0075) + 0.015;
            self.unk_184[i].z = s;
            self.unk_184[i].x = s;
            let r = self.actor.shape_rot;
            self.unk_1b4[i] = [r.x, r.y, r.z];
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.scale = self.drawn_scale;
        r.values = (0..4).flat_map(|i| [self.unk_154[i].x, self.unk_154[i].y, self.unk_154[i].z, self.unk_184[i].x, self.unk_184[i].y, self.unk_184[i].z]).collect();
        r.angles = self.unk_1b4.iter().flatten().copied().collect();
        r
    }

    /// `EnBx_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let color = COLOR.get(self.actor.params) as usize;
        if color >= TEXTURES.len() {
            // @bug (game): D_809D2560[color] past the table.
            return;
        }
        let (v, a) = (&rs.values, &rs.angles);
        let model = actor_draw_matrix(rs);
        if v.len() < 24 || a.len() < 12 || model.determinant() == 0.0 {
            return;
        }
        let inv = model.inverse();
        let bones: Vec<Mat4> = (0..4)
            .map(|i| {
                let p = Vec3::new(v[i * 6], v[i * 6 + 1], v[i * 6 + 2]);
                let s = Vec3::new(v[i * 6 + 3], v[i * 6 + 4], v[i * 6 + 5]);
                inv * Self::segment_matrix(p, [a[i * 3], a[i * 3 + 1], a[i * 3 + 2]], s).to_mat4()
            })
            .collect();
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &scroll(play.gameplay_frames));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(color))), transform: model, bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider)),
            COL_QUAD => Some(ColliderMut::Quad(&mut self.collider_quad)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
