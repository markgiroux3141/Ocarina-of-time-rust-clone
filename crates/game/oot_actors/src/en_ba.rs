//! `En_Ba` (`ovl_En_Ba/z_en_ba.c`): the parasitic tentacles of Lord Jabu-Jabu's Belly (GAME-10
//! milestone 5), hanging from the ceiling of the three rooms off the big room's north side, and
//! the pink blobs of flesh a dead one drops. `object_bxa`'s `object_bxa_DL_000890`, no skeleton: a
//! chain of 14 segments (`unk_158`, 32 apart, each turned by `unk_2A8`), the base at its place.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places three, the same in layers 0 to 3, each with an
//! `Obj_Roomtimer` sharing its flag: the red one (0x0B00) in room 8 at (1, 20, -5801), the gray
//! (0x0A02) in room 9 at (1561, 20, -4679) turned -16384, the green (0x0C01) in room 10 at
//! (-1562, 20, -4683) turned 16384. Their flags (0x0B, 0x0A, 0x0C) take down the electric
//! barriers of their colour (`En_Bx`).
//!
//! - **Hanging** (`EnBa_Idle`): its base held 100 over its home (330 once it has swung, while Link
//!   is over 175 across); its tip drawn toward a point 448 under the base that wanders 80 about
//!   (each 16th frame, `Rand_CenteredFloat(180)` on two phases), the chain following tip to base,
//!   the segments' turns easing after (a third, `unk_31C` 1500 at most, 182 at least). Link
//!   within 175 across (not knocked back, `PLAYER_STATE1_26`) with the base at its 100, it
//!   **swings at him** (`EnBa_SwingAtPlayer`, the base easing down to 60): 20 frames winding up
//!   (`NA_SE_EN_BALINADE_HAND_UP`), the chain bending back and turning toward Link (his place 30
//!   up kept as the target until the last 10 of them), then 10 frames of the swing
//!   (`NA_SE_EN_BALINADE_HAND_DOWN`, 8000 a frame, the chain's bend following `-cos(unk_31A ×
//!   0xCCC)`); then again (27 frames' wind-up, slower) while he's within 175, else hanging.
//!   Swinging, its tip's sphere hurts (4, `DMG_UNBLOCKABLE`) and touching Link knocks him back
//!   (8 out, 8 up, no damage of its own).
//! - **Hit**: only a boomerang (`DMG_BOOMERANG`, the 7th segment's sphere, half as thick, 20
//!   across), while hanging or swinging; four hits (health 4). A hit with health left **recoils**
//!   (`EnBa_RecoilFromDamage`: red 12 frames, `NA_SE_EN_BALINADE_HAND_DAMAGE`, the base up to 330
//!   at 30 a frame, the chain straightening, 20 frames, not hittable); the last **kills** it
//!   (`EnBa_Die`: `NA_SE_EN_BALINADE_HAND_DEAD` (`EffectSsDeadSound`, 40 frames), seven blobs
//!   from the lower segments, lock-on off, the upper half folding to the 6th segment, 15 frames
//!   rising 8 a frame with a twitch; then its switch flag and gone).
//! - **A blob** (params 3, `EnBa_FallAsBlob`): scale 0.021, flung 0 to 4 out a random way,
//!   falling (gravity -2); on the floor it squashes (0.001 down, 0.0005 out a frame) 20 frames and
//!   is gone. It keeps the profile's flags (an enemy: the room isn't clear until they're gone).
//!
//! The whole overlay is ported. Its draw (`EnBa_Draw`, `Gfx_SetupDL_25Opa`): the tentacle is
//! `object_bxa_DL_000890` with its colour's texture on segment 8 (`object_bxa_Tex_0024F0` red,
//! `_0027F0` green, `_0029F0` gray), a scroll on segment 9 (`Gfx_TwoTexScroll`, the second tile
//! down 10 a frame) and its 14 segments' matrices on segment 0x0C (the 7th half as thick), the
//! base under `Actor_Draw`'s; a blob is `object_bxa_DL_001D80` in pink (prim 255, 125, 100), its
//! two tiles scrolling. The spheres' places (`Collider_UpdateSpheres`, in the draw) are set in
//! `draw_update`. One bake for each colour and one for the blob.
//!
//! Params: `type` bits 0 to 7 (`EnBaType`: 0 the red tentacle, 1 the green, 2 the gray, 3 a
//! blob, spawned by a dying one; any higher a blob too: the unused cutscene layer (4) places three
//! 0x3FFF in rooms 8, 10 and 11, which fall and are gone; `params &= 0xFF` at init);
//! `upperParams` bits 8 to 15 (a tentacle's switch flag).
//! Flags: reads switch flag `upperParams` (set: gone at init) and sets it when it dies.

use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{atan2_s, cos_s, smooth_step_to_f, smooth_step_to_s, vec3f_pitch, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2, math_cos_f, math_sin_f};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::shadow::ShadowDraw;
use oot_game::sys_matrix::MtxF;

/// `ACTOR_EN_BA` (`actor_table.h`: 0x00DE).
pub const ACTOR_EN_BA: i16 = 0x00DE;
/// `OBJECT_BXA` (`object_table.h`: 0x00D5).
pub const OBJECT: &str = "object_bxa";
pub const OBJECT_BXA: i16 = 0x00D5;
/// `object_bxa_DL_000890` (the tentacle), `object_bxa_DL_001D80` (a blob) (`object_bxa.xml`).
pub const TENTACLE_DL: &str = "object_bxa_DL_000890";
pub const BLOB_DL: &str = "object_bxa_DL_001D80";
/// `D_809B8118`: the tentacles' textures, by type.
pub const TEXTURES: [&str; 3] = ["object_bxa_Tex_0024F0", "object_bxa_Tex_0027F0", "object_bxa_Tex_0029F0"];

/// `EnBaType` (`z_en_ba.h`).
pub const EN_BA_TENTACLE_RED: i16 = 0;
pub const EN_BA_TENTACLE_GREEN: i16 = 1;
pub const EN_BA_TENTACLE_GRAY: i16 = 2;
pub const EN_BA_DEAD_BLOB: i16 = 3;

/// `NA_SE_EN_BALINADE_HAND_DOWN`, `_UP`, `_DAMAGE`, `_DEAD` (`enemybank_table.h`: 0x3958 to 0x395B).
pub const NA_SE_EN_BALINADE_HAND_DOWN: u16 = 0x3958;
pub const NA_SE_EN_BALINADE_HAND_UP: u16 = 0x3959;
pub const NA_SE_EN_BALINADE_HAND_DAMAGE: u16 = 0x395A;
pub const NA_SE_EN_BALINADE_HAND_DEAD: u16 = 0x395B;
/// `NAVI_ENEMY_PARASITIC_TENTACLE` (`actor.h`: 0x15).
pub const NAVI_ENEMY_PARASITIC_TENTACLE: u8 = 0x15;
/// `ATTENTION_RANGE_4` (`actor.h`).
const ATTENTION_RANGE_4: u8 = 4;
/// `PLAYER_STATE1_26` (`player.h`: 1 << 26): Link knocked back.
pub const PLAYER_STATE1_26: u32 = 1 << 26;

/// `En_Ba_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating, `OBJECT_BXA`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_BA,
    name: "En_Ba",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `PARAMS_GET_U(params, 0, 8)` (after `params &= 0xFF`): `EnBaType`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Range(0, 255),
    "0 red tentacle, 1 green, 2 gray; 3 (spawned by a dying one) and up a blob (the cutscene layer places 0xFF)",
);
/// `PARAMS_GET_U(params, 8, 8)`: `upperParams`, its switch flag.
pub const SWITCH_FLAG: Field = Field::params("upperParams", 8, 8, Kind::SwitchFlag, "set when it dies; set at init, it's gone");

/// En_Ba's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("upperParams")).reads().sets().note("set when it dies; set, it isn't there")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `D_809B8080`: a segment's length along its z.
const D_809B8080: Vec3 = Vec3::new(0.0, 0.0, 32.0);
/// The segments.
pub const SEGMENTS: usize = 14;
/// The limbs `EnBa_Draw` updates the spheres on (`Collider_UpdateSpheres`): the hit sphere (8,
/// at the 7th segment) and the tip's (13).
pub const LIMB_HIT: u8 = 8;
pub const LIMB_TIP: u8 = 13;

/// `sJntSphElementsInit`: the hit sphere (limb 8, 20 across, a boomerang's: `0x00000010`) and the
/// tip's (limb 13, 25 across, hurting 4 with `0x20000000`, `DMG_UNBLOCKABLE`).
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 2] = [
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0010, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: LIMB_HIT,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
        scale: 100,
    },
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: LIMB_TIP,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 25 },
        scale: 100,
    },
];

/// `sJntSphInit`.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit {
    col_type: COL_MATERIAL_HIT0,
    at_flags: AT_ON | AT_TYPE_ENEMY,
    ac_flags: AC_ON | AC_TYPE_PLAYER,
    oc_flags1: OC1_NONE,
    oc_flags2: OC2_NONE,
    shape: COLSHAPE_JNTSPH,
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    FallAsBlob,
    SwingAtPlayer,
    RecoilFromDamage,
    Die,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnBa_Idle",
            Action::FallAsBlob => "EnBa_FallAsBlob",
            Action::SwingAtPlayer => "EnBa_SwingAtPlayer",
            Action::RecoilFromDamage => "EnBa_RecoilFromDamage",
            Action::Die => "EnBa_Die",
        }
    }
}

/// `func_80033AEC`: `arg1` stepped toward `arg0` (`Math_SmoothStepToF` on each axis, `arg2` of the
/// gap, `arg3` at most) when it's at least `arg4` from it, or more than `arg5`. The steps' gaps
/// left, summed.
// shared: z_actor.c's func_80033AEC (a point eased toward another)
pub fn func_80033aec(arg0: Vec3, arg1: &mut Vec3, arg2: f32, arg3: f32, arg4: f32, arg5: f32) -> f32 {
    let mut ret = 0.0;
    if arg4 <= arg0.distance(*arg1) || arg5 < arg0.distance(*arg1) {
        ret = smooth_step_to_f(&mut arg1.x, arg0.x, arg2, arg3, 0.0);
        ret += smooth_step_to_f(&mut arg1.y, arg0.y, arg2, arg3, 0.0);
        ret += smooth_step_to_f(&mut arg1.z, arg0.z, arg2, arg3, 0.0);
    }
    ret
}

/// `func_80035844`: the turn (x the pitch, y the yaw) from `arg0` to `arg1`, the height
/// difference down (`arg3` false) or up.
// shared: z_actor.c's func_80035844 (the pitch and yaw between two points)
pub fn func_80035844(arg0: Vec3, arg1: Vec3, arg3: bool) -> [i16; 2] {
    let dx = arg1.x - arg0.x;
    let dz = arg1.z - arg0.z;
    let dy = if arg3 { arg1.y - arg0.y } else { arg0.y - arg1.y };
    [atan2_s((dx * dx + dz * dz).sqrt(), dy), atan2_s(dz, dx)]
}

/// `x - 0x8000` / `x + 0x8000` as the C's `s16` parameter takes it.
fn flip(x: i16) -> i16 {
    x.wrapping_add(i16::MIN)
}

pub struct EnBa {
    pub actor: Actor,
    /// `unk_14C`: 2 or 3 hittable (hanging, swinging), 1 recoiling, 0 dying or a blob.
    pub unk_14c: i32,
    pub action: Action,
    /// `upperParams`: the switch flag.
    pub upper_params: i16,
    /// `unk_158`: the segments' places, base first.
    pub unk_158: [Vec3; SEGMENTS],
    /// `unk_200`: their draw scales (0.01).
    pub unk_200: [Vec3; SEGMENTS],
    /// `unk_2A8`: their turns (x, y, z).
    pub unk_2a8: [[i16; 3]; SEGMENTS],
    /// `unk_2FC`: the tip's target.
    pub unk_2fc: Vec3,
    /// `unk_308`: the wander's offsets (x, y) and its x phase (z).
    pub unk_308: Vec3,
    /// `unk_314`: the wander's z phase.
    pub unk_314: f32,
    pub unk_318: i16,
    pub unk_31a: i16,
    /// `unk_31C`: the segments' turn step.
    pub unk_31c: i16,
    pub collider: ColliderJntSph,
}

impl EnBa {
    /// `PARAMS_GET_U(params, 0, 8)`.
    pub fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `EnBa_Init`: see the module's docs.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain.
        actor.navi_enemy_id = NAVI_ENEMY_PARASITIC_TENTACLE;
        actor.culling_volume_scale = 1500.0;
        actor.culling_volume_downward = 2500.0;
        actor.target_arrow_offset = 0.0;
        actor.world_pos.y = actor.home_pos.y + 100.0;
        let mut unk_158 = [Vec3::ZERO; SEGMENTS];
        let mut unk_2a8 = [[0i16; 3]; SEGMENTS];
        for i in (0..SEGMENTS).rev() {
            unk_2a8[i][0] = -0x4000;
            unk_158[i] = actor.world_pos;
            unk_158[i].y = actor.world_pos.y - (i as i32 + 1) as f32 * 32.0;
        }
        actor.target_mode = ATTENTION_RANGE_4;
        let upper_params = SWITCH_FLAG.get(actor.params);
        // schema: params &= 0xFF, the type alone kept
        actor.params &= 0xFF;
        let mut this = EnBa {
            actor,
            unk_14c: 0,
            action: Action::Idle,
            upper_params,
            unk_158,
            unk_200: [Vec3::splat(0.01); SEGMENTS],
            unk_2a8,
            unk_2fc: Vec3::ZERO,
            unk_308: Vec3::ZERO,
            unk_314: 0.0,
            unk_318: 0,
            unk_31a: 0,
            unk_31c: 0,
            collider: ColliderJntSph::default(),
        };
        if this.ty() < EN_BA_DEAD_BLOB {
            if play.flags.get_switch(this.upper_params as i32) {
                this.actor.kill();
                return Box::new(this);
            }
            this.actor.shape_init(0.0, ShadowDraw::Circle, 48.0);
            this.actor.scale = Vec3::splat(0.01);
            this.setup_idle();
            this.actor.col_chk_info.health = 4;
            this.actor.col_chk_info.mass = MASS_HEAVY;
            this.collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        } else {
            this.actor.scale = Vec3::splat(0.021);
            this.setup_fall_as_blob(play);
        }
        Box::new(this)
    }

    /// `EnBa_SetupIdle`.
    pub fn setup_idle(&mut self) {
        self.unk_14c = 2;
        self.unk_31c = 1500;
        self.actor.speed_xz = 10.0;
        self.action = Action::Idle;
    }

    /// The wander (`EnBa_Idle`'s and `EnBa_RecoilFromDamage`'s): each 16th frame two random steps
    /// of the phases, the offsets 80 by their sine and cosine.
    fn wander(&mut self, play: &mut PlayState) {
        if play.gameplay_frames % 16 == 0 {
            self.unk_308.z += play.rand.centered_float(180.0);
            self.unk_314 += play.rand.centered_float(180.0);
            self.unk_308.x = math_sin_f(self.unk_308.z) * 80.0;
            self.unk_308.y = math_cos_f(self.unk_314) * 80.0;
        }
    }

    /// The tip drawn toward `unk_2FC` (448 under the base, the wander's offsets), the chain
    /// following it from the tip to the base (`EnBa_Idle`'s and `EnBa_RecoilFromDamage`'s).
    fn pull_chain_to_target(&mut self, play: &mut PlayState) {
        self.unk_2fc = self.actor.world_pos;
        self.wander(play);
        self.unk_2fc.y -= 448.0;
        self.unk_2fc.x += self.unk_308.x;
        self.unk_2fc.z += self.unk_308.y;
        func_80033aec(self.unk_2fc, &mut self.unk_158[13], 1.0, self.actor.speed_xz, 0.0, 0.0);
        for i in (0..=12).rev() {
            let sp5c = func_80035844(self.unk_158[i + 1], self.unk_158[i], false);
            let n = self.unk_158[i + 1];
            let mut m = MtxF::set_translate(n.x, n.y, n.z);
            m.rotate_zyx(sp5c[0], sp5c[1], 0);
            self.unk_158[i] = m.mult_vec3f(D_809B8080);
        }
    }

    /// `Matrix_Translate(world.pos, MTXMODE_NEW)`, `Matrix_RotateZYX(shape.rot.x - 0x8000,
    /// shape.rot.y, 0)`, `Matrix_MultVec3f(&D_809B8080, &unk_158[0])`: the first segment from the
    /// base.
    fn place_first_segment(&mut self) {
        let w = self.actor.world_pos;
        let mut m = MtxF::set_translate(w.x, w.y, w.z);
        m.rotate_zyx(flip(self.actor.shape_rot.x), self.actor.shape_rot.y, 0);
        self.unk_158[0] = m.mult_vec3f(D_809B8080);
    }

    /// Segment `i + 1` from segment `i` and its turn, `v` along it.
    fn place_next_segment(&mut self, i: usize, v: Vec3) {
        let p = self.unk_158[i];
        let mut m = MtxF::set_translate(p.x, p.y, p.z);
        m.rotate_zyx(flip(self.unk_2a8[i][0]), self.unk_2a8[i][1], 0);
        self.unk_158[i + 1] = m.mult_vec3f(v);
    }

    /// `EnBa_Idle`: see the module's docs.
    fn idle(&mut self, play: &mut PlayState) {
        let state1 = player_state1(play);
        if self.actor.col_chk_info.mass == MASS_IMMOVABLE && self.actor.xz_dist_to_player > 175.0 {
            smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 330.0, 1.0, 7.0, 0.0);
        } else {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 100.0, 1.0, 10.0, 0.0);
        }
        self.pull_chain_to_target(play);
        let sp5c = func_80035844(self.unk_158[0], self.unk_2fc, false);
        let (ty, tx) = (self.unk_2a8[0][1], self.unk_2a8[0][0]);
        smooth_step_to_s(&mut self.actor.shape_rot.y, ty, 3, self.unk_31c, 182);
        smooth_step_to_s(&mut self.actor.shape_rot.x, tx, 3, self.unk_31c, 182);
        self.place_first_segment();
        self.unk_2a8[13][1] = sp5c[1];
        self.unk_2a8[13][0] = flip(sp5c[0]);
        for i in 0..13 {
            let next = self.unk_2a8[i + 1];
            smooth_step_to_s(&mut self.unk_2a8[i][1], next[1], 3, self.unk_31c, 182);
            smooth_step_to_s(&mut self.unk_2a8[i][0], next[0], 3, self.unk_31c, 182);
            self.place_next_segment(i, D_809B8080);
        }
        self.unk_2a8[13][0] = self.unk_2a8[12][0];
        self.unk_2a8[13][1] = self.unk_2a8[12][1];
        if state1 & PLAYER_STATE1_26 == 0 && self.actor.xz_dist_to_player <= 175.0 && self.actor.world_pos.y == self.actor.home_pos.y + 100.0 {
            self.setup_swing_at_player();
        }
    }

    /// `EnBa_SetupFallAsBlob`.
    pub fn setup_fall_as_blob(&mut self, play: &mut PlayState) {
        self.unk_14c = 0;
        self.actor.speed_xz = play.rand.centered_float(8.0);
        self.actor.world_rot.y = play.rand.centered_float(65535.0) as i32 as i16;
        self.unk_318 = 20;
        self.actor.gravity = -2.0;
        self.action = Action::FallAsBlob;
    }

    /// `EnBa_FallAsBlob`: falling; on the floor squashing 20 frames, then gone.
    fn fall_as_blob(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.scale.y -= 0.001;
            self.actor.scale.x += 0.0005;
            self.actor.scale.z += 0.0005;
            self.unk_318 -= 1;
            if self.unk_318 == 0 {
                self.actor.kill();
            }
        } else {
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 30.0, 28.0, 80.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        }
    }

    /// `EnBa_SetupSwingAtPlayer`.
    pub fn setup_swing_at_player(&mut self) {
        self.unk_14c = 3;
        self.unk_318 = 20;
        self.unk_31a = 0;
        self.unk_31c = 1500;
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.actor.speed_xz = 20.0;
        self.action = Action::SwingAtPlayer;
    }

    /// The swing again (27 frames' wind-up, 750 a frame) or hanging (Link over 175 across or
    /// knocked back).
    fn swing_again_or_idle(&mut self, state1: u32) {
        if self.actor.xz_dist_to_player > 175.0 || state1 & PLAYER_STATE1_26 != 0 {
            self.setup_idle();
        } else {
            self.setup_swing_at_player();
            self.unk_318 = 27;
            self.unk_31c = 750;
        }
    }

    /// `EnBa_SwingAtPlayer`: see the module's docs. `@bug (game)` (the C's note): its `AT_HIT` is
    /// read only here, so a hit landing as another action takes over stays set and knocks Link
    /// back on its next swing.
    fn swing_at_player(&mut self, play: &mut PlayState) {
        let state1 = player_state1(play);
        let player = play.player;
        let player_pos = player.and_then(|h| play.actors.actor(h)).map_or(Vec3::ZERO, |p| p.world_pos);
        smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 60.0, 1.0, 10.0, 0.0);
        if self.actor.xz_dist_to_player <= 175.0 || self.unk_31a != 0 {
            if self.unk_318 == 20 {
                audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_HAND_UP);
                self.unk_31c = 1500;
            }
            if self.unk_318 != 0 {
                self.unk_31a = 10;
                self.unk_318 -= 1;
                let phi_fp = if self.unk_318 >= 11 {
                    self.unk_2fc = player_pos;
                    self.unk_2fc.y += 30.0;
                    self.actor.yaw_towards_player
                } else {
                    vec3f_yaw(self.actor.world_pos, self.unk_2fc)
                };
                smooth_step_to_s(&mut self.unk_31c, 1500, 1, 30, 0);
                let sp58 = func_80035844(self.actor.world_pos, self.unk_158[0], false);
                smooth_step_to_s(&mut self.actor.shape_rot.y, sp58[1], 1, self.unk_31c, 0);
                smooth_step_to_s(&mut self.actor.shape_rot.x, flip(sp58[0]), 1, self.unk_31c, 0);
                self.place_first_segment();
                for i in 0..13 {
                    smooth_step_to_s(&mut self.unk_2a8[i][0], ((i as i32 * 1200) - 0x4000) as i16, 1, self.unk_31c, 0);
                    smooth_step_to_s(&mut self.unk_2a8[i][1], phi_fp, 1, self.unk_31c, 0);
                    self.place_next_segment(i, D_809B8080);
                }
            } else {
                if self.unk_31a == 10 {
                    audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_HAND_DOWN);
                }
                if self.unk_31a != 0 {
                    self.unk_31c = 8000;
                    self.actor.speed_xz = 30.0;
                    let phi_fp = vec3f_yaw(self.actor.world_pos, self.unk_2fc);
                    let temp = flip(vec3f_pitch(self.actor.world_pos, self.unk_158[0]));
                    smooth_step_to_s(&mut self.actor.shape_rot.y, phi_fp, 1, self.unk_31c, 0);
                    smooth_step_to_s(&mut self.actor.shape_rot.x, temp, 1, self.unk_31c, 0);
                    self.place_first_segment();
                    for i in 0..13 {
                        // -Math_CosS(unk_31A * 0xCCC) * (i * 1200), into an s16.
                        let temp = (-cos_s((self.unk_31a as i32 * 0xCCC) as i16) * (i as i32 * 1200) as f32) as i32 as i16;
                        smooth_step_to_s(&mut self.unk_2a8[i][0], (temp as i32 - 0x4000) as i16, 1, self.unk_31c, 0);
                        smooth_step_to_s(&mut self.unk_2a8[i][1], phi_fp, 1, self.unk_31c, 0);
                        self.place_next_segment(i, D_809B8080);
                    }
                    self.unk_31a -= 1;
                } else {
                    self.swing_again_or_idle(state1);
                }
            }
            self.unk_2a8[13][0] = self.unk_2a8[12][0];
            self.unk_2a8[13][1] = self.unk_2a8[12][1];
            if self.collider.base.at_flags & AT_HIT != 0 {
                self.collider.base.at_flags &= !AT_HIT;
                if self.collider.base.at.is_some() && self.collider.base.at == player {
                    actor_set_player_knockback_large_no_damage(play, 8.0, self.actor.yaw_towards_player, 8.0);
                }
            }
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        } else {
            self.swing_again_or_idle(state1);
        }
    }

    /// `func_809B7174`: recoiling from a hit.
    pub fn setup_recoil_from_damage(&mut self, play: &mut PlayState) {
        self.unk_14c = 1;
        self.unk_31c = 1500;
        self.unk_318 = 20;
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.actor.speed_xz = 10.0;
        audio_play_actor_sfx2(play, NA_SE_EN_BALINADE_HAND_DAMAGE);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 12);
        self.action = Action::RecoilFromDamage;
    }

    /// `EnBa_RecoilFromDamage`: see the module's docs.
    fn recoil_from_damage(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 330.0, 1.0, 30.0, 0.0);
        self.pull_chain_to_target(play);
        let sp6c = func_80035844(self.actor.world_pos, self.unk_158[0], false);
        smooth_step_to_s(&mut self.actor.shape_rot.y, sp6c[1], 3, self.unk_31c, 182);
        smooth_step_to_s(&mut self.actor.shape_rot.x, flip(sp6c[0]), 3, self.unk_31c, 182);
        self.place_first_segment();
        for i in 0..13 {
            let sp6c = func_80035844(self.unk_158[i], self.unk_158[i + 1], false);
            smooth_step_to_s(&mut self.unk_2a8[i][1], sp6c[1], 3, self.unk_31c, 182);
            smooth_step_to_s(&mut self.unk_2a8[i][0], flip(sp6c[0]), 3, self.unk_31c, 182);
            self.place_next_segment(i, D_809B8080);
        }
        self.unk_2a8[13][0] = self.unk_2a8[12][0];
        self.unk_2a8[13][1] = self.unk_2a8[12][1];
        self.unk_318 -= 1;
        if self.unk_318 == 0 {
            self.setup_idle();
        }
    }

    /// `func_809B75A0`: the last hit: its death's sound, seven blobs from segments 7 to 13, lock-on
    /// off, the upper half folded to the 6th segment.
    pub fn setup_die(&mut self, play: &mut PlayState) {
        self.unk_31c = 2500;
        let pos = self.actor.projected_pos;
        play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_EN_BALINADE_HAND_DEAD, 1, 1, 40));
        self.unk_14c = 0;
        for i in 7..SEGMENTS {
            let _ = play.actor_spawn(ACTOR_EN_BA, self.unk_158[i], [0, 0, 0], EN_BA_DEAD_BLOB);
        }
        let unk_temp = flip(vec3f_pitch(self.actor.world_pos, self.unk_158[0]));
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, self.unk_31c, 0);
        smooth_step_to_s(&mut self.actor.shape_rot.x, unk_temp, 1, self.unk_31c, 0);
        self.place_first_segment();
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.fold_upper_half();
        self.unk_31a = 15;
        self.action = Action::Die;
    }

    /// Segments 5 to 12 turned as the 5th, each 0 along the last (`sp74`): all at the 6th's place.
    fn fold_upper_half(&mut self) {
        for i in 5..13 {
            let five = self.unk_2a8[5];
            smooth_step_to_s(&mut self.unk_2a8[i][0], five[0], 1, self.unk_31c, 0);
            smooth_step_to_s(&mut self.unk_2a8[i][1], five[1], 1, self.unk_31c, 0);
            self.place_next_segment(i, Vec3::ZERO);
        }
    }

    /// `EnBa_Die`: 15 frames rising 8 a frame and twitching; then the switch flag, gone.
    fn die(&mut self, play: &mut PlayState) {
        if self.unk_31a != 0 {
            self.actor.speed_xz = 30.0;
            self.unk_31c = 8000;
            self.actor.world_pos.y += 8.0;
            let temp = flip(vec3f_pitch(self.actor.world_pos, self.unk_158[0]));
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, self.unk_31c, 0);
            smooth_step_to_s(&mut self.actor.shape_rot.x, temp, 1, self.unk_31c, 0);
            self.place_first_segment();
            for i in 0..5 {
                let temp = (-cos_s((self.unk_31a as i32 * 0x444) as i16) * (i as i32 * 400) as f32) as i32 as i16;
                smooth_step_to_s(&mut self.unk_2a8[i][0], (temp as i32 - 0x4000) as i16, 1, self.unk_31c, 0);
                smooth_step_to_s(&mut self.unk_2a8[i][1], self.actor.yaw_towards_player, 1, self.unk_31c, 0);
                self.place_next_segment(i, D_809B8080);
            }
            self.fold_upper_half();
            self.unk_31a -= 1;
        } else {
            play.flags.set_switch(self.upper_params as i32);
            self.actor.kill();
        }
    }

    /// `EnBa_Draw`'s matrix of segment `i`: at its place, turned, its scale (the 7th half as
    /// thick, as its sphere's).
    pub fn segment_matrix(pos: Vec3, rot: [i16; 3], scale: Vec3, i: usize) -> MtxF {
        let mut m = MtxF::set_translate(pos.x, pos.y, pos.z);
        m.rotate_zyx(rot[0], rot[1], rot[2]);
        m.scale(scale.x, scale.y, scale.z);
        if i == 6 {
            m.scale(0.5, 0.5, 1.0);
        }
        m
    }
}

/// Player's `stateFlags1` (0 without him).
fn player_state1(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags1())
}

/// The bakes' segments: the colour's texture (8), the scroll (9), the segments' matrices (0x0C);
/// the blob's prim colour (0x0E, run first).
const SEG_TEX: u8 = 0x08;
const SEG_SCROLL: u8 = 0x09;
const SEG_MATRICES: u8 = 0x0C;
const SEG_BLOB_SCROLL: u8 = 0x08;
const SEG_BLOB_COLOR: u8 = 0x0E;
pub const BLOB_BAKE: &str = "En_Ba/blob";

/// The tentacle's bake for type `ty`.
pub fn bake_name(ty: usize) -> String {
    format!("En_Ba/tentacle{ty}")
}

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, 0, 0, 16, 16, 1, 0, (gameplayFrames × -10) % 128, 32, 32)`
/// (`gameplayFrames` a `u32`).
pub fn tentacle_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_two_tex_scroll(0, 0, 0, 16, 16, 1, 0, frames.wrapping_mul((-10i32) as u32) % 128, 32, 32)
}

/// `Gfx_TwoTexScroll(G_TX_RENDERTILE, (frames × 2) % 128, (frames × 2) % 128, 32, 32, 1, (frames ×
/// -5) % 128, (frames × -5) % 128, 32, 32)`.
pub fn blob_scroll(frames: u32) -> Vec<(u32, u32)> {
    let a = frames.wrapping_mul(2) % 128;
    let b = frames.wrapping_mul((-5i32) as u32) % 128;
    gfx_two_tex_scroll(0, a, a, 32, 32, 1, b, b, 32, 32)
}

/// The tentacles (`object_bxa_DL_000890` in each colour) and the blob (`object_bxa_DL_001D80`,
/// `gDPSetPrimColor(0, 0, 255, 125, 100, 255)` first).
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = TEXTURES
        .iter()
        .enumerate()
        .map(|(i, tex)| MeshBake {
            name: bake_name(i),
            object: OBJECT.into(),
            segments: vec![
                (SEG_TEX, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_SCROLL, BakeSegment::Dynamic(tentacle_scroll(0))),
                (SEG_MATRICES, BakeSegment::Matrices((0..SEGMENTS as u16).collect())),
            ],
            prelude: vec![],
            body: BakeBody::DLists(vec![(OBJECT.into(), TENTACLE_DL.into())]),
        })
        .collect();
    v.push(MeshBake {
        name: BLOB_BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_BLOB_COLOR, BakeSegment::Commands(vec![(0xFA00_0000, 0xFF7D_64FF), (0xDF00_0000, 0)])), (SEG_BLOB_SCROLL, BakeSegment::Dynamic(blob_scroll(0)))],
        prelude: vec![SEG_BLOB_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), BLOB_DL.into())]),
    });
    v
}

/// `render_state`'s layout: values each segment's place and scale (six a segment); angles each
/// segment's turn; switches `gameplayFrames`.
const PER_SEG: usize = 6;

impl ActorImpl for EnBa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBa_Update`: a hit (a tentacle's: health down, recoiling or dying); the action; its focus
    /// the 7th segment; hittable while hanging or swinging.
    fn update(&mut self, play: &mut PlayState) {
        let tentacle = self.ty() < EN_BA_DEAD_BLOB;
        if tentacle && self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.actor.col_chk_info.health = self.actor.col_chk_info.health.wrapping_sub(1);
            if self.actor.col_chk_info.health == 0 {
                self.setup_die(play);
            } else {
                self.setup_recoil_from_damage(play);
            }
        }
        match self.action {
            Action::Idle => self.idle(play),
            Action::FallAsBlob => self.fall_as_blob(play),
            Action::SwingAtPlayer => self.swing_at_player(play),
            Action::RecoilFromDamage => self.recoil_from_damage(play),
            Action::Die => self.die(play),
        }
        if tentacle {
            self.actor.focus_pos = self.unk_158[6];
        }
        if self.unk_14c >= 2 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnBa_Destroy`: the spheres (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnBa_Draw`'s `Collider_UpdateSpheres`: the hit sphere under the 7th segment's matrix (half
    /// as thick), the tip's under the 14th's.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.ty() >= EN_BA_DEAD_BLOB {
            return;
        }
        for (i, limb) in [(6, LIMB_HIT), (13, LIMB_TIP)] {
            let m = Self::segment_matrix(self.unk_158[i], self.unk_2a8[i], self.unk_200[i], i);
            self.collider.update_spheres(limb, &m.to_mat4());
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        if self.ty() < EN_BA_DEAD_BLOB {
            r.values = (0..SEGMENTS).flat_map(|i| [self.unk_158[i].x, self.unk_158[i].y, self.unk_158[i].z, self.unk_200[i].x, self.unk_200[i].y, self.unk_200[i].z]).collect();
            r.angles = self.unk_2a8.iter().flatten().copied().collect();
        }
        r
    }

    /// `EnBa_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let frames = play.gameplay_frames;
        let model = actor_draw_matrix(rs);
        let ty = self.ty();
        let mut sv = SegmentValues::default();
        if ty < EN_BA_DEAD_BLOB {
            let (v, a) = (&rs.values, &rs.angles);
            if v.len() < SEGMENTS * PER_SEG || a.len() < SEGMENTS * 3 || model.determinant() == 0.0 {
                return;
            }
            let inv = model.inverse();
            let bones: Vec<Mat4> = (0..SEGMENTS)
                .map(|i| {
                    let p = Vec3::new(v[i * PER_SEG], v[i * PER_SEG + 1], v[i * PER_SEG + 2]);
                    let s = Vec3::new(v[i * PER_SEG + 3], v[i * PER_SEG + 4], v[i * PER_SEG + 5]);
                    inv * Self::segment_matrix(p, [a[i * 3], a[i * 3 + 1], a[i * 3 + 2]], s, i).to_mat4()
                })
                .collect();
            sv.read(SEG_SCROLL, &tentacle_scroll(frames));
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(ty as usize))), transform: model, bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
        } else {
            sv.read(SEG_BLOB_SCROLL, &blob_scroll(frames));
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BLOB_BAKE)), transform: model, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
