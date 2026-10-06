//! `En_Sw` (`ovl_En_Sw/z_en_sw.c`): the Skullwalltula, a spider on a wall that turns now and
//! then and dashes at Link while he climbs near it, and the Gold Skulltula, which walks its wall
//! or floor turning on the spot and leaves a Gold Skulltula Token (`En_Si`) when killed.
//!
//! Params (after `EnSw_Init`'s conversion): bits 13..15 the type, 8..12 the Gold Skulltula
//! flags' index (`GET_GS_FLAGS`), 0..7 its flag. A placed Gold Skulltula has 0x8000 set: its
//! type becomes 1 more than bits 13..15 and its index 1 less than bits 8..12 (the scene's
//! number + 1). Types:
//! - **0, the Skullwalltula** (an enemy): it finds its wall 60 behind it (`func_80B0DFFC`) and
//!   **turns** on it by 0x2EE0 to 0x7C00 at random, waiting 10 to 40 frames between turns
//!   (`func_80B0E5E0`). With Link climbing (`PLAYER_STATE1_21`) within 130, in its sight
//!   (`func_80B0DEA8`) and moving (`func_8002DDF4`: `PLAYER_STATE2_12`, holding still on the
//!   wall), it **laughs** and turns to him for 20 frames (`func_80B0E728`), then **dashes** at
//!   him at up to 8 along the wall, purple-fogged, while its wall goes on (`func_80B0DFFC`'s
//!   line tests from its body's corners) until there or Link holds still, **brakes**
//!   (`func_80B0E90C`) and **goes home** (`func_80B0E9BC`). A hit (`DamageTable_Get(0xE)`, 1
//!   health) kills it: it **falls** spinning (`func_80B0DB00`), bouncing twice with a ring of
//!   dust, then **dissolves** in 9 puffs (`EffectSsDeadDb`) and drops from table 3.
//! - **1 to 4, the Gold Skulltula** (2 health, its bite 16): it **walks its surface**
//!   (`func_80B0C0CC`, keeping to the polys it finds 18 above and below and 24 ahead), turning
//!   on the spot by the animation (`func_80B0D590`). 2 is out only at night (it shrinks away by
//!   day); 3 and 4 are spawned by others and **jump out** (`NA_SE_SY_CORRECT_CHIME`,
//!   `func_80B0D3AC`), landing with dust. Killed it **spins** (`func_80B0D878`), 9 puffs, then
//!   `NA_SE_SY_KINSTA_MARK_APPEAR` and its token 10 off its surface. One whose flag is set
//!   (its token taken) isn't spawned.
//!
//! The whole overlay is ported. Not ported: the circle shadow of a falling Skullwalltula
//! (`ActorShadow_DrawCircle`, not ported for any actor); `func_8002EBCC`'s look-at (the shine
//! on the Gold Skulltula's texgen limbs is lit from the camera's view, not from the eye towards
//! the actor); limb 4's env colour (`unk_1F4`, never written: black, which none of the limbs'
//! combiners reads). `sAnimationInfo`'s other three entries are never played.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::{BGCHECK_Y_MIN, PolyId};
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use eng_math::{approach_f, atan2_s, cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, audio_play_actor_sfx2, enemy_start_finishing_blow, func_8002ddf4};
use oot_game::audio::sfx::*;
use oot_game::camera::f_atan2f;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, Anim, AnimationInfo, SkelAnimeStd};
use oot_game::surface::{SurfaceType, WALL_FLAG_CRAWLSPACE};
use oot_game::sys_matrix::{MtxF, rad_to_binang};

pub const ACTOR_EN_SW: i16 = 0x0095;
const OBJECT: &str = "object_st";
/// `object_st_Skel_005298`.
const SKELETON: &str = "object_st_Skel_005298";
/// The Gold Skulltula's bake: the skeleton with `EnSw_OverrideLimbDraw`'s gold lists.
const BAKE_GOLD: &str = "En_Sw/gold";

/// `En_Sw_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_SW,
    name: "En_Sw",
    category: oot_game::actor_ctx::ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
};

/// `ACTOR_EN_SI` (`actor_table.h`).
const ACTOR_EN_SI: i16 = crate::en_si::ACTOR_EN_SI;

/// `NAVI_ENEMY_SKULLWALLTULA`, `NAVI_ENEMY_GOLD_SKULLTULA` (`actor.h`).
const NAVI_ENEMY_SKULLWALLTULA: u8 = 0x1F;
const NAVI_ENEMY_GOLD_SKULLTULA: u8 = 0x20;

/// `COLLECTIBLE_DROP_TABLE_3` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_3: i16 = 3;

/// `PLAYER_STATE1_21` (`player.h`): climbing.
const PLAYER_STATE1_21: u32 = 1 << 21;

/// The sounds (`sfx` tables: `enemybank_table.h`, `systembank_table.h`).
pub const NA_SE_EN_STALTU_DAMAGE: u16 = 0x386B;
pub const NA_SE_EN_STALWALL_DEAD: u16 = 0x3885;
pub const NA_SE_EN_STALWALL_ROLL: u16 = 0x388C;
pub const NA_SE_EN_STALWALL_DASH: u16 = 0x388D;
pub const NA_SE_EN_STALGOLD_ROLL: u16 = 0x39DA;
pub const NA_SE_EN_STALGOLD_UP_CRY: u16 = 0x39EA;
pub const NA_SE_EN_STALWALL_LAUGH: u16 = 0x39F2;
pub const NA_SE_EN_DODO_M_UP: u16 = 0x3824;
pub const NA_SE_SY_KINSTA_MARK_APPEAR: u16 = 0x4843;

/// `DamageTable_Get(0xE)` (`z_collision_btltbls.c`'s `sDamageTablePresets[14]`, "Used by
/// En_Sw").
pub static S_DAMAGE_TABLE_PRESET_14: DamageTable = DamageTable {
    table: [
        dmg_entry(1, 0x0), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(1, 0x0), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x0), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(4, 0x0), // Fire arrow
        dmg_entry(4, 0x0), // Ice arrow
        dmg_entry(4, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(4, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(2, 0x0), // Giant spin
        dmg_entry(4, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(4, 0x0), // Giant jump
        dmg_entry(8, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `sJntSphElementsInit`: one sphere on limb 2 (`{0, -300, 0}`, 21): the bite (`0xFFCFFFFF`
/// for 8), hit by `0xFFC3FFFE` (everything but the Deku nut, the shield and the mirror's
/// ray), hookable.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFC3_FFFE, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 2,
        model_sphere: Sphere16 { center: [0, -300, 0], radius: 21 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `D_80B0F074`: 1 health, 2 by 25 (25 up), immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 1, cyl_radius: 2, cyl_height: 25, cyl_y_shift: 25, mass: MASS_IMMOVABLE };

/// `ENSW_ANIM_0`'s animation (`sAnimationInfo[0]`: `object_st_Anim_000304`, looped, no
/// morph).
const ANIM_0: &str = "object_st_Anim_000304";

/// `EnSw_OverrideLimbDraw`'s gold lists: (`limbIndex`, list).
const GOLD_LIMBS: [(usize, &str); 10] = [
    (23, "object_st_DL_004788"),
    (8, "object_st_DL_0046F0"),
    (14, "object_st_DL_004658"),
    (11, "object_st_DL_0045C0"),
    (26, "object_st_DL_004820"),
    (20, "object_st_DL_0048B8"),
    (17, "object_st_DL_004950"),
    (29, "object_st_DL_0049E8"),
    (5, "object_st_DL_003FB0"),
    (4, "object_st_DL_0043D8"),
];

/// The Gold Skulltula's bake: `SkelAnime_DrawOpa` with `EnSw_OverrideLimbDraw` setting
/// `*dList` on its ten limbs (`LimbOverride`'s limbs count from 0).
pub fn bakes() -> Vec<MeshBake> {
    let limbs = GOLD_LIMBS.iter().map(|&(l, s)| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: s.into() }).collect();
    vec![MeshBake { name: BAKE_GOLD.into(), object: OBJECT.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKELETON.into(), limbs } }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B0D364`: the jump out's start (types 3 and 4).
    SetupEmerge,
    /// `func_80B0D3AC`: jumping out, onto its surface.
    Emerge,
    /// `func_80B0D590`: a Gold Skulltula turning on the spot.
    GoldIdle,
    /// `func_80B0D878`: a Gold Skulltula dying, spinning.
    GoldDie,
    /// `func_80B0DB00`: a Skullwalltula falling dead.
    Fall,
    /// `func_80B0DC7C`: its puffs, then its drop.
    Dissolve,
    /// `func_80B0E5E0`: a Skullwalltula turning on its wall.
    Idle,
    /// `func_80B0E728`: turning to Link, then dashing at him.
    Dash,
    /// `func_80B0E90C`: braking.
    Brake,
    /// `func_80B0E9BC`: going home.
    ReturnHome,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::SetupEmerge => "func_80B0D364",
            Action::Emerge => "func_80B0D3AC",
            Action::GoldIdle => "func_80B0D590",
            Action::GoldDie => "func_80B0D878",
            Action::Fall => "func_80B0DB00",
            Action::Dissolve => "func_80B0DC7C",
            Action::Idle => "func_80B0E5E0",
            Action::Dash => "func_80B0E728",
            Action::Brake => "func_80B0E90C",
            Action::ReturnHome => "func_80B0E9BC",
        }
    }
}

/// `PARAMS_GET_S(params, 13, 3)`: the type.
pub fn sw_type(params: i16) -> i32 {
    (params as i32 >> 13) & 7
}

/// `EnSw_Init`'s params: a placed Gold Skulltula's (0x8000 set) type one more than its bits
/// 13..15; then any Gold Skulltula's flags' index one less (from the scene's number + 1).
///
/// @bug (game): an index of 0 becomes -1, whose `<< 8` sets every bit from 8 up, the type's
/// too (7). No placement has one.
pub fn init_params(params: i16) -> i16 {
    let mut params = params;
    let p = params as i32;
    if p & 0x8000 != 0 {
        // PARAMS_GET_S(thisx->params - 0x8000, 13, 3) + 1.
        let phi_v0 = ((p - 0x8000) >> 13 & 7) + 1;
        params = ((p & 0x1FFF) | (phi_v0 << 0xD)) as i16;
    }
    if sw_type(params) > 0 {
        let p = params as i32;
        let phi_v0 = ((p >> 8) & 0x1F) - 1;
        params = ((p & !(0x1F << 8)) | (phi_v0 << 8)) as i16;
    }
    params
}

/// `Math_Vec3f_Pitch`.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// `Math_FAcosF` (`math64.c`): `M_PI / 2 - Math_FAsinF(x)`, `Math_FAsinF(x)` being
/// `Math_FAtan2F(x, sqrtf(1 - x²))`.
fn f_acosf(x: f32) -> f32 {
    std::f32::consts::FRAC_PI_2 - f_atan2f(x, (1.0 - x * x).sqrt())
}

/// `EnSw_CrossProduct`.
fn cross_product(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new((a.y * b.z) - (a.z * b.y), (a.z * b.x) - (a.x * b.z), (a.x * b.y) - (a.y * b.x))
}

/// `Math3D_Vec3fMagnitude`.
fn magnitude(v: Vec3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

/// `COLPOLY_GET_NORMAL` of a poly's normal.
fn poly_normal(play: &PlayState, p: PolyId) -> Vec3 {
    let n = play.col.poly(p).normal;
    Vec3::new(n[0] as f32 * (1.0 / 32767.0), n[1] as f32 * (1.0 / 32767.0), n[2] as f32 * (1.0 / 32767.0))
}

/// The matrix `func_80B0BE20` and `func_80B0CCF4` build: its x axis `unk_370`, y `unk_364`, z
/// `unk_37C`.
fn axes_mtx(x: Vec3, y: Vec3, z: Vec3) -> MtxF {
    MtxF { xx: x.x, yx: x.y, zx: x.z, wx: 0.0, xy: y.x, yy: y.y, zy: y.z, wy: 0.0, xz: z.x, yz: z.y, zz: z.z, wz: 0.0, xw: 0.0, yw: 0.0, zw: 0.0, ww: 1.0 }
}

/// `func_80B0C020`: the poly the segment `a`→`b` meets (walls, floors and ceilings,
/// `BgCheck_EntityLineTest1`), unless it's a crawlspace's wall (`WALL_FLAG_CRAWLSPACE`) or
/// ignored by projectiles; with where (`arg3`) and its bg (`arg4`).
fn func_80b0c020(play: &PlayState, a: Vec3, b: Vec3) -> Option<(Vec3, PolyId)> {
    let (hit, poly) = play.col.entity_line_test(a, b, true, true, true, false)?;
    if play.col.wall_flags(poly) & WALL_FLAG_CRAWLSPACE != 0 {
        return None;
    }
    if play.col.is_ignored_by_projectiles(poly) {
        return None;
    }
    Some((hit, poly))
}

pub struct EnSw {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anim_0: Option<Anim>,
    pub action: Action,
    pub collider: ColliderJntSph,
    /// `unk_1F4`: limb 4's env colour (never written).
    pub unk_1f4: [u8; 4],
    /// `unk_360`: jumping out (the floor test skipped while it rises).
    pub unk_360: u8,
    /// `unk_364`: its surface's normal (up); `unk_370` its right; `unk_37C` its forward.
    pub unk_364: Vec3,
    pub unk_370: Vec3,
    pub unk_37c: Vec3,
    /// `unk_388`: the wait between turns.
    pub unk_388: i16,
    /// `unk_38A`: the turn's wait (gold), or the bounces left (falling).
    pub unk_38a: i16,
    /// `unk_38C`: the turn's length (gold), the jump out's delay.
    pub unk_38c: i16,
    /// `unk_38E`: the wait before a turn (gold).
    pub unk_38e: i16,
    /// `unk_390`: the bite's rest after a hit (AT off).
    pub unk_390: i16,
    /// `unk_392`: the rest after being hit (AC off), the colour filter's length.
    pub unk_392: i16,
    /// `unk_394`: the puffs' count.
    pub unk_394: i16,
    pub unk_3d8: MtxF,
    /// `unk_420`: the turn's speed (radians a frame).
    pub unk_420: f32,
    pub unk_42c: u8,
    /// `unk_430`: its wall (`func_80B0DFFC`'s last line test).
    pub unk_430: Option<PolyId>,
    pub unk_434: Vec3,
    /// `unk_440`: the roll's and the dash's sound timer.
    pub unk_440: i16,
    /// `unk_442`: the turn to Link before the dash, the wait after.
    pub unk_442: i16,
    /// `unk_444`: the roll (`shape.rot.z`) to turn to.
    pub unk_444: i16,
    pub unk_446: i16,
    /// `unk_448`: where it dashes or goes.
    pub unk_448: Vec3,
    /// `unk_454`, `unk_460`, `unk_46C`, `unk_478`: its body's corners, `unk_484` behind it (the
    /// draw's, limb 1's override).
    pub unk_454: Vec3,
    pub unk_460: Vec3,
    pub unk_46c: Vec3,
    pub unk_478: Vec3,
    pub unk_484: Vec3,
}

impl EnSw {
    /// `PARAMS_GET_S(params, 13, 3)`.
    fn ty(&self) -> i32 {
        sw_type(self.actor.params)
    }

    /// `EnSw_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.params = init_params(actor.params);
        // Check to see if this gold skull token has already been retrieved.
        let p = actor.params as i32;
        let gs_index = (p >> 8) & 0x1F;
        let anim_0 = play.assets.clone().and_then(|a| a.animation(OBJECT, ANIM_0).map_err(|e| log::error!("En_Sw: {e:#}")).ok());
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKELETON).map_err(|e| log::error!("En_Sw: {e:#}")).ok());
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(29);
        let elements = jnt_sph_elements();
        let mut s = EnSw {
            actor,
            // SkelAnime_Init(&object_st_Skel_005298, NULL, jointTable, morphTable, 30).
            skel: SkelAnimeStd::init_flex(limbs, None),
            skeleton,
            anim_0,
            action: Action::Idle,
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &elements),
            unk_1f4: [0; 4],
            unk_360: 0,
            unk_364: Vec3::ZERO,
            unk_370: Vec3::ZERO,
            unk_37c: Vec3::ZERO,
            unk_388: 0,
            unk_38a: 0,
            unk_38c: 0,
            unk_38e: 0,
            unk_390: 0,
            unk_392: 0,
            unk_394: 0,
            unk_3d8: MtxF::IDENTITY,
            unk_420: 0.0,
            unk_42c: 0,
            unk_430: None,
            unk_434: Vec3::ZERO,
            unk_440: 0,
            unk_442: 0,
            unk_444: 0,
            unk_446: 0,
            unk_448: Vec3::ZERO,
            unk_454: Vec3::ZERO,
            unk_460: Vec3::ZERO,
            unk_46c: Vec3::ZERO,
            unk_478: Vec3::ZERO,
            unk_484: Vec3::ZERO,
        };
        if play.save.get_gs_flags(gs_index) & (p & 0xFF) as u32 != 0 {
            s.actor.kill();
            return Box::new(s);
        }
        // Animation_ChangeByInfo(&skelAnime, sAnimationInfo, ENSW_ANIM_0).
        if let Some(a) = s.anim_0.clone() {
            s.skel.change_by_info(&AnimationInfo { animation: a, play_speed: 1.0, start_frame: 0.0, frame_count: -1.0, mode: ANIMMODE_LOOP_INTERP, morph_frames: 0.0 });
        }
        // ActorShape_Init(&shape, 0, NULL, 0).
        s.actor.shape_y_offset = 0.0;
        s.actor.col_chk_info.set_info2(Some(&S_DAMAGE_TABLE_PRESET_14), &COL_CHK_INFO_INIT);
        s.actor.scale.x = 0.02;
        if s.ty() == 0 {
            s.actor.world_rot.x = 0;
            s.actor.world_rot.z = 0;
            s.actor.shape_rot = s.actor.world_rot;
            s.unk_484.y = s.actor.world_pos.y;
            s.unk_484.x = s.actor.world_pos.x + (sin_s(s.actor.world_rot.y) * -60.0);
            s.unk_484.z = s.actor.world_pos.z + (cos_s(s.actor.world_rot.y) * -60.0);
            s.func_80b0dffc(play);
            s.actor.home_pos = s.actor.world_pos;
        } else {
            let y = s.actor.shape_rot.y;
            s.unk_370 = Vec3::new(sin_s(y.wrapping_add(0x4000)), 0.0, cos_s(y.wrapping_add(0x4000)));
            s.unk_364 = Vec3::new(0.0, 1.0, 0.0);
            s.unk_37c = Vec3::new(sin_s(y), 0.0, cos_s(y));
            s.func_80b0c0cc(play, 1);
        }
        if s.ty() >= 3 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
        }
        match s.ty() {
            3 | 4 | 2 | 1 => {
                if s.ty() >= 3 {
                    s.unk_360 = 1;
                    s.actor.velocity.y = 8.0;
                    s.actor.speed_xz = 4.0;
                    s.actor.gravity = -1.0;
                }
                if s.ty() >= 2 {
                    s.actor.scale.x = 0.0;
                }
                s.collider.elements[0].info.at_dmg_info.damage *= 2;
                s.actor.navi_enemy_id = NAVI_ENEMY_GOLD_SKULLTULA;
                s.actor.col_chk_info.health *= 2;
                s.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            }
            _ => {
                // Actor_ChangeCategory(play, &play->actorCtx, &this->actor, ACTORCAT_ENEMY): it's
                // put in its list when the spawn inserts it.
                s.actor.category = ACTORCAT_ENEMY;
                s.actor.navi_enemy_id = NAVI_ENEMY_SKULLWALLTULA;
            }
        }
        s.unk_38e = play.rand.s16_offset(0xF, 0x1E);
        s.actor.scale = Vec3::splat(s.actor.scale.x);
        s.actor.home_pos = s.actor.world_pos;
        s.actor.shape_rot = s.actor.world_rot;
        if s.ty() >= 3 {
            s.unk_38c = 0x28;
            s.unk_394 = 1;
            s.action = Action::SetupEmerge;
        } else if s.ty() == 0 {
            s.action = Action::Idle;
        } else {
            s.action = Action::GoldIdle;
        }
        Box::new(s)
    }

    /// `func_80B0BE20`: onto the poly `poly`: its right axis turned with the surface (from
    /// `unk_364` to the poly's normal, `Matrix_RotateAxis`), its forward axis from that, and
    /// `world.rot` from the three (`Matrix_MtxFToYXZRotS`).
    ///
    /// @bug (game): Does not return, but the return value is not used by any caller so it
    /// doesn't matter.
    fn func_80b0be20(&mut self, play: &PlayState, poly: PolyId) {
        self.actor.floor_poly = Some(poly);
        let poly_normal = poly_normal(play, poly);
        let sp34 = f_acosf(poly_normal.dot(self.unk_364));
        let sp38 = cross_product(self.unk_364, poly_normal);
        let m = MtxF::rotate_axis(sp34, sp38);
        self.unk_370 = m.mult_vec3f(self.unk_370);
        self.unk_37c = cross_product(self.unk_370, poly_normal);
        let temp_f0 = magnitude(self.unk_37c);
        if temp_f0 < 0.001 {
            return;
        }
        self.unk_37c.x *= 1.0 / temp_f0;
        self.unk_37c.y *= 1.0 / temp_f0;
        self.unk_37c.z *= 1.0 / temp_f0;
        self.unk_364 = poly_normal;
        self.unk_3d8 = axes_mtx(self.unk_370, self.unk_364, self.unk_37c);
        let r = self.unk_3d8.to_yxz_rot_s(false);
        self.actor.world_rot = Rot { x: r[0], y: r[1], z: r[2] };
    }

    /// `func_80B0C0CC`: keeping to its surface: the poly 18 above to 18 below it (and, unless
    /// jumping out, whatever is 24 ahead of that), else the first of 24 behind, right or left
    /// of its feet; on it with `arg2` 1. Turns its shape towards `world.rot`. True when it's on
    /// a surface.
    fn func_80b0c0cc(&mut self, play: &mut PlayState, arg2: i32) -> bool {
        let mut sp64 = false;
        self.unk_42c = 1;
        let mut sp84 = self.actor.world_pos;
        let mut sp78 = self.actor.world_pos;
        sp84 += self.unk_364 * 18.0;
        sp78 -= self.unk_364 * 18.0;
        let temp_s1 = func_80b0c020(play, sp84, sp78);
        if temp_s1.is_some() && self.unk_360 == 0 {
            let (sp90, s1) = temp_s1.unwrap();
            sp78 = sp84 + self.unk_37c * 24.0;
            match func_80b0c020(play, sp84, sp78) {
                Some((sp9c, p)) => {
                    if arg2 == 1 {
                        self.func_80b0be20(play, p);
                        self.actor.world_pos = sp9c;
                        self.actor.floor_bg_id = p.bg;
                    }
                }
                None => {
                    if self.actor.floor_poly != Some(s1) {
                        self.func_80b0be20(play, s1);
                    }
                    self.actor.world_pos = sp90;
                    self.actor.floor_bg_id = s1.bg;
                }
            }
            sp64 = true;
        } else {
            sp84 = sp78;
            for phi_s1 in 0..3 {
                sp78 = match phi_s1 {
                    0 => sp84 - self.unk_37c * 24.0,
                    1 => sp84 + self.unk_370 * 24.0,
                    _ => sp84 - self.unk_370 * 24.0,
                };
                let Some((sp9c, p)) = func_80b0c020(play, sp84, sp78) else { continue };
                if arg2 == 1 {
                    self.func_80b0be20(play, p);
                    self.actor.world_pos = sp9c;
                    self.actor.floor_bg_id = p.bg;
                }
                sp64 = true;
                break;
            }
        }
        smooth_step_to_s(&mut self.actor.shape_rot.x, self.actor.world_rot.x, 8, 0xFA0, 1);
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 8, 0xFA0, 1);
        smooth_step_to_s(&mut self.actor.shape_rot.z, self.actor.world_rot.z, 8, 0xFA0, 1);
        sp64
    }

    /// `func_80B0C9F0`: a hit (or a hammer's quake within 400 for a Skullwalltula), unless
    /// resting from the last: red for 16 frames, the damage; alive, `NA_SE_EN_STALTU_DAMAGE`;
    /// killed, the finishing blow and `NA_SE_EN_STALWALL_DEAD`, and it spins (gold) or falls.
    /// Its bite meeting something rests it 30 frames.
    fn func_80b0c9f0(&mut self, play: &mut PlayState) -> bool {
        let mut phi_v1 = false;
        if self.actor.xyz_dist_to_player_sq < 400.0 * 400.0 && self.ty() == 0 && play.actors.unk_02 != 0 {
            self.actor.col_chk_info.damage = self.actor.col_chk_info.health;
            phi_v1 = true;
        }
        if self.unk_392 == 0 && (self.collider.base.ac_flags & AC_HIT != 0 || phi_v1) {
            self.collider.base.ac_flags &= !AC_HIT;
            self.unk_392 = 0x10;
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, self.unk_392 as u16);
            if self.actor.apply_damage() != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_STALTU_DAMAGE);
                return true;
            }
            enemy_start_finishing_blow(play, &self.actor);
            if self.ty() != 0 {
                self.skel.play_speed = 8.0;
                self.unk_420 = if play.state_frames & 1 == 0 { 0.1 } else { -0.1 };
                self.unk_394 = 0xA;
                self.unk_38a = 1;
                self.unk_420 *= 4.0;
                self.action = Action::GoldDie;
            } else {
                // shape.shadowDraw = ActorShadow_DrawCircle, alpha 255, scale 16: not ported.
                self.unk_38a = 2;
                self.actor.gravity = -1.0;
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                self.action = Action::Fall;
            }
            audio_play_actor_sfx2(play, NA_SE_EN_STALWALL_DEAD);
            return true;
        }
        if self.unk_390 == 0 && self.collider.base.at_flags & AT_HIT != 0 {
            self.unk_390 = 30;
        }
        false
    }

    /// `func_80B0CBE8`: a Gold Skulltula out of its turning only counts down its rest; else the
    /// bite (unless resting from a bite), the body (unless resting from a hit), and the push.
    fn func_80b0cbe8(&mut self, play: &mut PlayState) {
        if self.ty() > 0 && self.action != Action::GoldIdle {
            if self.unk_392 != 0 {
                self.unk_392 -= 1;
            }
        } else {
            if decr(&mut self.unk_390) == 0 && self.actor.col_chk_info.health != 0 {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
            if decr(&mut self.unk_392) == 0 && self.actor.col_chk_info.health != 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    /// `func_80B0CCF4`: turned by `arg1` radians about its floor's normal (`Matrix_RotateAxis`),
    /// `world.rot` from its new axes. False without a floor or with no forward axis.
    fn func_80b0ccf4(&mut self, play: &PlayState, arg1: f32) -> bool {
        let Some(floor_poly) = self.actor.floor_poly else { return false };
        let floor_poly_normal = poly_normal(play, floor_poly);
        let m = MtxF::rotate_axis(arg1, floor_poly_normal);
        self.unk_370 = m.mult_vec3f(self.unk_370);
        self.unk_37c = cross_product(self.unk_370, self.unk_364);
        let mut temp_f0 = magnitude(self.unk_37c);
        if temp_f0 < 0.001 {
            return false;
        }
        temp_f0 = 1.0 / temp_f0;
        self.unk_37c *= temp_f0;
        let sp2c = axes_mtx(self.unk_370, self.unk_364, self.unk_37c);
        let r = sp2c.to_yxz_rot_s(false);
        self.actor.world_rot = Rot { x: r[0], y: r[1], z: r[2] };
        true
    }

    /// `func_80B0CEA8`: the roll's sound (the gold one's or the Skullwalltula's), when grown
    /// and within 380 of the camera's eye.
    fn func_80b0cea8(&mut self, play: &mut PlayState) {
        if !(self.actor.scale.x < 0.0139999995) {
            let eye = play.active_camera().eye;
            if !(self.actor.world_pos.distance(eye) >= 380.0) {
                audio_play_actor_sfx2(play, if self.ty() > 0 { NA_SE_EN_STALGOLD_ROLL } else { NA_SE_EN_STALWALL_ROLL });
            }
        }
    }

    /// `func_80B0CF44` and `func_80B0D14C`: `cnt + 1` puffs of dust round it, `radius` out
    /// (`func_8002836C`).
    fn dust_ring(&self, play: &mut PlayState, cnt: i32, radius: f32, scale: i16, scale_step: i16, life: i16) {
        // primColor, envColor, velocity, accel.
        const PRIM: [u8; 4] = [80, 80, 50, 255];
        const ENV: [u8; 4] = [100, 100, 80, 0];
        let velocity = Vec3::ZERO;
        let mut accel = Vec3::new(0.0, 0.3, 0.0);
        let p = self.actor.world_pos;
        let mut angle = ((play.rand.zero_one() - 0.5) * 65536.0) as i32 as i16;
        let mut i = cnt;
        while i >= 0 {
            accel.x = (play.rand.zero_one() - 0.5) * 2.0;
            accel.z = (play.rand.zero_one() - 0.5) * 2.0;
            let pos = Vec3::new(p.x + (sin_s(angle) * radius), p.y, p.z + (cos_s(angle) * radius));
            play.with_ss(|ss| ss.func_8002836c(pos, velocity, accel, PRIM, ENV, scale, scale_step, life));
            i -= 1;
            angle = angle.wrapping_add((0x10000 / cnt) as i16);
        }
    }

    /// `func_80B0CF44`: the jump out's dust, 2 round (20, growing 30, 12 frames).
    fn func_80b0cf44(&self, play: &mut PlayState, cnt: i32) {
        self.dust_ring(play, cnt, 2.0, 20, 30, 12);
    }

    /// `func_80B0D14C`: the landing's dust, 14 round (20, growing 40, 10 frames).
    fn func_80b0d14c(&self, play: &mut PlayState, cnt: i32) {
        self.dust_ring(play, cnt, 14.0, 20, 40, 10);
    }

    /// `func_80B0D364`: the jump out starts at once (type 4) or after 10 frames (type 3).
    fn func_80b0d364(&mut self) {
        if self.ty() == 4 {
            self.unk_38c = 0;
            self.action = Action::Emerge;
        } else {
            self.unk_38c = 10;
            self.action = Action::Emerge;
        }
    }

    /// `func_80B0D3AC`: its delay out (dust every few frames), then its cry; growing to 0.02,
    /// flying out along its surface's normal and forward, falling; landing on a surface
    /// (`NA_SE_EN_DODO_M_GND`, dust), it starts turning.
    fn func_80b0d3ac(&mut self, play: &mut PlayState) {
        if self.unk_38c != 0 {
            if self.unk_38c & 4 != 0 {
                self.func_80b0cf44(play, 5);
            }
            self.unk_38c -= 1;
            if self.unk_38c == 0 {
                play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EN_STALGOLD_UP_CRY);
                play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EN_DODO_M_UP);
            } else {
                return;
            }
        }
        approach_f(&mut self.actor.scale.x, 0.02, 0.2, 0.01);
        self.actor.scale = Vec3::splat(self.actor.scale.x);
        self.actor.world_pos += self.unk_364 * self.actor.velocity.y;
        self.actor.world_pos += self.unk_37c * self.actor.speed_xz;
        self.actor.velocity.y += self.actor.gravity;
        self.actor.velocity.y = self.actor.velocity.y.max(self.actor.min_velocity_y);
        if self.actor.velocity.y < 0.0 {
            self.unk_360 = 0;
        }
        if self.func_80b0c0cc(play, 1) {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            self.func_80b0d14c(play, 8);
            self.actor.scale.x = 0.02;
            self.actor.scale = Vec3::splat(0.02);
            self.action = Action::GoldIdle;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
            self.actor.gravity = 0.0;
        }
    }

    /// `func_80B0D590`: (type 2: out only at night, its body off while small) a wait of
    /// `unk_38E`, then the roll's sound and a turn of `unk_38C` frames by `unk_420` (both twice
    /// as long, and the turn twice as fast, for the gold ones): the animation's steps at 4 (8)
    /// after 1 frame's pause each, turning by the step's sine.
    fn func_80b0d590(&mut self, play: &mut PlayState) {
        if self.ty() == 2 {
            if self.actor.scale.x < 0.0139999995 {
                self.collider.elements[0].info.at_elem_flags = ATELEM_NONE;
                self.collider.elements[0].info.ac_elem_flags = ACELEM_NONE;
                self.collider.elements[0].info.oc_elem_flags = OCELEM_NONE;
            }
            if self.actor.scale.x >= 0.0139999995 {
                self.collider.elements[0].info.at_elem_flags = ATELEM_ON;
                self.collider.elements[0].info.ac_elem_flags = ACELEM_ON;
                self.collider.elements[0].info.oc_elem_flags = OCELEM_ON;
            }
            approach_f(&mut self.actor.scale.x, if !play.save.is_day() { 0.02 } else { 0.0 }, 0.2, 0.01);
            self.actor.scale = Vec3::splat(self.actor.scale.x);
        }
        if self.unk_38e != 0 {
            self.unk_38e -= 1;
            if self.unk_38e == 0 {
                self.func_80b0cea8(play);
                self.unk_420 = if play.state_frames % 2 == 0 { 0.1 } else { -0.1 };
                self.unk_38a = 1;
                self.unk_38c = play.rand.s16_offset(30, 60);
                if self.ty() != 0 {
                    self.unk_38c = self.unk_38c.wrapping_mul(2);
                    self.unk_420 *= 2.0;
                }
            }
        } else {
            self.unk_38c = self.unk_38c.wrapping_sub(1);
            if self.unk_38c == 0 {
                self.unk_38e = play.rand.s16_offset(15, 30);
                self.unk_38a = 0;
                self.skel.play_speed = 0.0;
                if self.ty() != 0 {
                    self.unk_38e /= 2;
                }
            } else if self.unk_38a != 0 {
                self.unk_38a -= 1;
                self.skel.play_speed = if self.unk_38a == 0 { 4.0 } else { 0.0 };
                if self.skel.play_speed > 0.0 {
                    self.func_80b0cea8(play);
                }
                if self.ty() != 0 {
                    self.skel.play_speed *= 2.0;
                }
            } else {
                if self.skel.on_frame(self.skel.end_frame) {
                    self.unk_38a = 2;
                }
                let mut sp2c = 32768.0 / self.skel.end_frame;
                sp2c *= self.skel.cur_frame;
                // Math_SinS((s16)sp2C).
                sp2c = sin_s(sp2c as i32 as i16) * self.unk_420;
                self.func_80b0ccf4(play, sp2c);
                self.actor.shape_rot = self.actor.world_rot;
            }
        }
    }

    /// `func_80B0D878`: spinning by `unk_420` (the roll's sound each loop); its rest out, 9
    /// puffs, then `NA_SE_SY_KINSTA_MARK_APPEAR` and its token (`En_Si`, its params) 10 off its
    /// surface, and it's gone.
    fn func_80b0d878(&mut self, play: &mut PlayState) {
        // velAndAccel.
        let vel_and_accel = Vec3::new(0.0, 0.5, 0.0);
        if self.skel.on_frame(self.skel.end_frame) {
            self.func_80b0cea8(play);
        }
        self.func_80b0ccf4(play, self.unk_420);
        self.actor.shape_rot = self.actor.world_rot;
        if self.unk_394 == 0 && self.unk_392 == 0 {
            play.audio.play_sfx_centered(NA_SE_SY_KINSTA_MARK_APPEAR);
            let x = self.unk_364.x * 10.0;
            let y = self.unk_364.y * 10.0;
            let z = self.unk_364.z * 10.0;
            let p = self.actor.world_pos;
            let params = self.actor.params;
            if let Ok(h) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_SI, Vec3::new(p.x + x, p.y + y, p.z + z), [0, 0, 0], params)
                && let Some(t) = play.actors.actor_mut(h)
            {
                t.parent = None;
            }
            self.actor.kill();
            return;
        }
        if self.unk_392 == 0 && decr(&mut self.unk_394) != 0 {
            let mut pos = self.actor.world_pos;
            pos.y += 10.0 + ((play.rand.zero_one() - 0.5) * 6.0);
            pos.x += (play.rand.zero_one() - 0.5) * 32.0;
            pos.z += (play.rand.zero_one() - 0.5) * 32.0;
            play.with_ss(|ss| ss.dead_db_spawn(pos, vel_and_accel, vel_and_accel, 42, 0, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
        }
    }

    /// `func_80B0DB00`: falling, tumbling; on the ground (over no floor it's gone) it bounces
    /// up at 8, then 4 (`NA_SE_EN_DODO_M_GND` and a ring of 13 dust clouds each landing), then
    /// dissolves over 10 frames.
    fn func_80b0db00(&mut self, play: &mut PlayState) {
        self.actor.move_forward();
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(0x1000);
        self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1000);
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && !(0.0 <= self.actor.velocity.y) {
            if self.actor.floor_height <= BGCHECK_Y_MIN || self.actor.floor_height >= 32000.0 {
                self.actor.kill();
                return;
            }
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
            if self.unk_38a == 0 {
                self.action = Action::Dissolve;
                self.unk_394 = 10;
            } else {
                // (this->unk_38A--) * 8.0f * 0.5f.
                let n = self.unk_38a;
                self.unk_38a -= 1;
                self.actor.velocity.y = ((n as f32) * 8.0) * 0.5;
            }
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            let a = self.actor.clone();
            actor_spawn_floor_dust_ring(play, &a, a.world_pos, 16.0, 12, 2.0, 120, 10, false);
        }
    }

    /// `func_80B0DC7C`: 9 puffs (`EffectSsDeadDb`), tumbling; then its drop (table 3) and it's
    /// gone.
    fn func_80b0dc7c(&mut self, play: &mut PlayState) {
        // velAndAccel, pos.
        let vel_and_accel = Vec3::new(0.0, 0.5, 0.0);
        let mut pos = Vec3::ZERO;
        if decr(&mut self.unk_394) != 0 {
            pos.y = ((play.rand.zero_one() - 0.5) * 6.0) + (self.actor.world_pos.y + 10.0);
            pos.x = ((play.rand.zero_one() - 0.5) * 32.0) + self.actor.world_pos.x;
            pos.z = ((play.rand.zero_one() - 0.5) * 32.0) + self.actor.world_pos.z;
            play.with_ss(|ss| ss.dead_db_spawn(pos, vel_and_accel, vel_and_accel, 42, 0, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
            self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(0x1000);
            self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1000);
        } else {
            // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_3, false).
            let p = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, None, p, COLLECTIBLE_DROP_TABLE_3 * 16);
            self.actor.kill();
        }
    }

    /// `func_80B0DE34`: the roll on its wall that points its head at `arg1`.
    fn func_80b0de34(&self, arg1: Vec3) -> i16 {
        let yaw = vec3f_yaw(self.actor.world_pos, arg1).wrapping_sub(self.actor.wall_yaw);
        let pitch = vec3f_pitch(self.actor.world_pos, arg1).wrapping_sub(0x4000);
        (pitch as i32 * if yaw >= 0 { -1 } else { 1 }) as i16
    }

    /// `func_80B0DEA8`: (with `arg2`) Link climbing, not holding still; facing him within 0x1FC2
    /// of its roll; within 130; nothing between them.
    fn func_80b0dea8(&self, play: &PlayState, arg2: bool) -> bool {
        let Some(player) = play.player.and_then(|h| play.actors.get(h)) else { return false };
        let state1 = player.as_player().map(|p| p.state_flags1()).unwrap_or(0);
        let player_pos = player.base().world_pos;
        if state1 & PLAYER_STATE1_21 == 0 && arg2 {
            false
        } else if func_8002ddf4(play) && arg2 {
            false
        } else if (self.func_80b0de34(player_pos) as i32 - self.actor.shape_rot.z as i32).abs() >= 0x1FC2 {
            // ABS(s16 - s16), in int.
            false
        } else if self.actor.world_pos.distance(player_pos) >= 130.0 {
            false
        } else {
            play.col.entity_line_test(self.actor.world_pos, player_pos, true, false, false, true).is_none()
        }
    }

    /// `func_80B0DFFC`: whether its way on is clear: not pushing against anything, and (one
    /// test a frame, by `play->state.frames % 4`) a wall under its front corners (`unk_454`,
    /// `unk_46C`) and none under its back ones (`unk_460`, `unk_478`). Onto its wall behind
    /// (`unk_484`) if there's one: `wallYaw` from it, 6 out from it.
    fn func_80b0dffc(&mut self, play: &mut PlayState) -> bool {
        let mut sp4c = true;
        let test = |p: Vec3, to: Vec3| play.col.entity_line_test(p, to, true, false, false, true).is_some();
        let pos = self.actor.world_pos;
        let frames = play.state_frames % 4;
        if self.collider.base.oc_flags1 & OC1_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            sp4c = false;
        } else if frames == 0 && !test(pos, self.unk_454) {
            sp4c = false;
        } else if frames == 1 && test(pos, self.unk_460) {
            sp4c = false;
        } else if frames == 2 && !test(pos, self.unk_46c) {
            sp4c = false;
        } else if frames == 3 && test(pos, self.unk_478) {
            sp4c = false;
        }
        if let Some((sp50, wall)) = play.col.entity_line_test(pos, self.unk_484, true, false, false, true) {
            self.unk_430 = Some(wall);
            let n = play.col.poly(wall).normal;
            self.actor.wall_yaw = rad_to_binang(f_atan2f(n[0] as f32, n[2] as f32));
            self.actor.world_pos = sp50;
            self.actor.world_pos.x += 6.0 * sin_s(self.actor.world_rot.y);
            self.actor.world_pos.z += 6.0 * cos_s(self.actor.world_rot.y);
            self.unk_434 = sp50;
            self.unk_434.x += sin_s(self.actor.world_rot.y);
            self.unk_434.z += cos_s(self.actor.world_rot.y);
        }
        sp4c
    }

    /// `func_80B0E314`: towards `arg1` at a speed eased to `arg4`.
    fn func_80b0e314(&mut self, arg1: Vec3, arg4: f32) {
        smooth_step_to_f(&mut self.actor.speed_xz, arg4, 0.3, 100.0, 0.1);
        let x_diff = arg1.x - self.actor.world_pos.x;
        let y_diff = arg1.y - self.actor.world_pos.y;
        let z_diff = arg1.z - self.actor.world_pos.z;
        let dist = (x_diff * x_diff + y_diff * y_diff + z_diff * z_diff).sqrt();
        let (mut x_dist, mut y_dist, mut z_dist) = if dist == 0.0 { (0.0, 0.0, 0.0) } else { (x_diff / dist, y_diff / dist, z_diff / dist) };
        x_dist *= self.actor.speed_xz;
        y_dist *= self.actor.speed_xz;
        z_dist *= self.actor.speed_xz;
        self.actor.world_pos.x += x_dist;
        self.actor.world_pos.y += y_dist;
        self.actor.world_pos.z += z_dist;
    }

    /// `func_80B0E430`: after its wait (`unk_388`, the animation slowing to a stop), the
    /// animation up to `arg1` (with `arg3` 1, not past its loop's end) and the roll turned
    /// towards `unk_444` by `arg2` (the roll's sound every 4 frames near the camera). True once
    /// it's there.
    fn func_80b0e430(&mut self, arg1: f32, arg2: i16, arg3: i32, play: &mut PlayState) -> bool {
        let last_frame = self.anim_0.as_ref().map(|a| a.last_frame()).unwrap_or(0.0);
        if decr(&mut self.unk_388) != 0 {
            smooth_step_to_f(&mut self.skel.play_speed, 0.0, 0.6, 1000.0, 0.01);
            return false;
        }
        smooth_step_to_f(&mut self.skel.play_speed, arg1, 0.6, 1000.0, 0.01);
        if arg3 == 1 && last_frame < (self.skel.cur_frame + self.skel.play_speed) {
            return false;
        }
        let eye = play.active_camera().eye;
        if self.actor.world_pos.distance(eye) < 380.0 {
            if decr(&mut self.unk_440) == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_STALWALL_ROLL);
                self.unk_440 = 4;
            }
        } else {
            self.unk_440 = 0;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.z, self.unk_444, 4, arg2, arg2);
        self.actor.world_rot = self.actor.shape_rot;
        self.actor.shape_rot.z == self.unk_444
    }

    /// `func_80B0E5E0`: turning on its wall: each turn done, the next by 0x2EE0 to 0x7C00 either
    /// way, after 10 to 40 frames; Link climbing in reach, `NA_SE_EN_STALWALL_LAUGH` and the
    /// dash's 20 frames of turning to him.
    fn func_80b0e5e0(&mut self, play: &mut PlayState) {
        if self.func_80b0e430(6.0, 0x3E8, 1, play) {
            let rand = play.rand.zero_one();
            // ((s16)(20000.0f * rand) + 0x2EE0) * (Rand_ZeroOne() >= 0.5f ? 1.0f : -1.0f) + world.rot.z.
            let turn = ((20000.0 * rand) as i32 as i16) as i32 + 0x2EE0;
            let sign = if play.rand.zero_one() >= 0.5 { 1.0 } else { -1.0 };
            self.unk_444 = (turn as f32 * sign + self.actor.world_rot.z as f32) as i32 as i16;
            self.unk_388 = play.rand.s16_offset(10, 30);
        }
        if decr(&mut self.unk_442) == 0 && self.func_80b0dea8(play, true) {
            audio_play_actor_sfx2(play, NA_SE_EN_STALWALL_LAUGH);
            self.unk_442 = 20;
            self.action = Action::Dash;
        }
    }

    /// `func_80B0E728`: for 20 frames turning to Link (30 above him) while he's still in
    /// reach, else back to turning; then, while its way on is clear, dashing there at up to 8
    /// (`NA_SE_EN_STALWALL_DASH` every 4 frames) until within 13 or Link holds still; blocked,
    /// home after 20 to 30 frames.
    fn func_80b0e728(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        if decr(&mut self.unk_442) != 0 {
            if self.func_80b0dea8(play, true) {
                self.unk_448 = player_pos;
                self.unk_448.y += 30.0;
                self.unk_444 = self.func_80b0de34(self.unk_448);
                self.func_80b0e430(6.0, 0xFA0u16 as i16, 0, play);
            } else {
                self.action = Action::Idle;
            }
        } else if !self.func_80b0dffc(play) {
            self.unk_442 = play.rand.s16_offset(20, 10);
            self.unk_444 = self.func_80b0de34(self.actor.home_pos);
            self.unk_448 = self.actor.home_pos;
            self.action = Action::ReturnHome;
        } else {
            self.func_80b0e314(self.unk_448, 8.0);
            if decr(&mut self.unk_440) == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_STALWALL_DASH);
                self.unk_440 = 4;
            }
            if !(self.actor.world_pos.distance(self.unk_448) > 13.0) || func_8002ddf4(play) {
                self.action = Action::Brake;
            }
        }
    }

    /// `func_80B0E90C`: slowing to a stop on its way; then home.
    fn func_80b0e90c(&mut self) {
        self.func_80b0e314(self.unk_448, 0.0);
        if self.actor.speed_xz == 0.0 {
            self.unk_444 = self.func_80b0de34(self.actor.home_pos);
            self.unk_448 = self.actor.home_pos;
            self.action = Action::ReturnHome;
        }
    }

    /// `func_80B0E9BC`: turned towards home, back there at 2; within 4, turning again.
    fn func_80b0e9bc(&mut self, play: &mut PlayState) {
        if self.func_80b0e430(6.0, 0x3E8, 0, play) {
            self.func_80b0e314(self.unk_448, 2.0);
            if !(self.actor.world_pos.distance(self.unk_448) > 4.0) {
                self.action = Action::Idle;
            }
        }
    }

    /// `Actor_Draw`'s model matrix (`Matrix_SetTranslateRotateYXZ`, then the scale), and
    /// `EnSw_Draw`'s for a Gold Skulltula: `Matrix_RotateX(DEG_TO_RAD(-80))`, and 200 out while
    /// it's alive.
    fn model_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        if self.ty() != 0 {
            m.rotate_x(deg_to_rad_m80());
            if a.col_chk_info.health != 0 {
                m.translate(0.0, 0.0, 200.0);
            }
        }
        m
    }
}

/// `DEG_TO_RAD(-80)`: `-80 * (M_PI / 180.0f)`, in double precision.
fn deg_to_rad_m80() -> f32 {
    (-80.0f64 * (std::f64::consts::PI / 180.0f32 as f64)) as f32
}

/// `DECR`: `x` down by 1 unless 0; the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

impl ActorImpl for EnSw {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSw_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.func_80b0c9f0(play);
        match self.action {
            Action::SetupEmerge => self.func_80b0d364(),
            Action::Emerge => self.func_80b0d3ac(play),
            Action::GoldIdle => self.func_80b0d590(play),
            Action::GoldDie => self.func_80b0d878(play),
            Action::Fall => self.func_80b0db00(play),
            Action::Dissolve => self.func_80b0dc7c(play),
            Action::Idle => self.func_80b0e5e0(play),
            Action::Dash => self.func_80b0e728(play),
            Action::Brake => self.func_80b0e90c(),
            Action::ReturnHome => self.func_80b0e9bc(play),
        }
        self.func_80b0cbe8(play);
    }

    /// `EnSw_OverrideLimbDraw`'s effects on the actor (`SkelAnime_DrawOpa`'s matrix at each
    /// limb's override, before the limb's own): limb 1's points (`unk_454`..`unk_484`), the
    /// focus at limb 5's, and the sphere (`Collider_UpdateSpheres` at every limb: limb 2's).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = self.model_mtx();
        // Each limb's matrix after its Matrix_TranslateRotateZYX, in draw order.
        let joints = &self.skel.joint_table;
        let mut limb_mtx = vec![MtxF::IDENTITY; skeleton.limbs.len()];
        for &l in &skeleton.draw_order {
            let l = l as usize;
            let at_override = skeleton.parents[l].map(|p| limb_mtx[p as usize]).unwrap_or(model);
            let limb_index = l + 1;
            if limb_index == 1 {
                // sp7C, sp70, sp64, sp58, sp4C.
                self.unk_454 = at_override.mult_vec3f(Vec3::new(1400.0, -2600.0, -800.0));
                self.unk_460 = at_override.mult_vec3f(Vec3::new(1400.0, -1600.0, 0.0));
                self.unk_46c = at_override.mult_vec3f(Vec3::new(-1400.0, -2600.0, -800.0));
                self.unk_478 = at_override.mult_vec3f(Vec3::new(-1400.0, -1600.0, 0.0));
                self.unk_484 = at_override.mult_vec3f(Vec3::new(0.0, 0.0, -600.0));
            }
            if limb_index == 5 {
                self.actor.focus_pos = at_override.mult_vec3f(Vec3::ZERO);
            }
            self.collider.update_spheres(limb_index as u8, &at_override.to_mat4());
            let pos = if l == 0 {
                let r = joints.first().copied().unwrap_or([0; 3]);
                Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32)
            } else {
                let p = skeleton.limbs[l].joint_pos;
                Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
            };
            let mut m = at_override;
            m.translate_rotate_zyx(pos, joints.get(l + 1).copied().unwrap_or([0; 3]));
            limb_mtx[l] = m;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let gold = self.ty() != 0;
        rs.switches = vec![gold as u32, (self.actor.col_chk_info.health != 0) as u32, (self.action == Action::Dash) as u32];
        rs
    }

    /// `EnSw_Draw`: a Gold Skulltula pitched up 80 degrees (200 out while alive), its gold
    /// limbs (the bake); a dashing Skullwalltula purple-fogged (`func_80B0EDB8`: `Gfx_SetFog2`
    /// from 0 to `11500 / 30 × (30 - 20)`, until `func_80B0EEA4` puts the scene's back).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [gold, alive, dashing]) = (&rs.joints, rs.switches.as_slice()) else { return };
        let Some(skeleton) = &self.skeleton else { return };
        let mut model = oot_game::play::actor_draw_matrix(rs);
        let mut params = DrawParams::default();
        let mesh = if *gold != 0 {
            model *= Mat4::from_rotation_x(deg_to_rad_m80());
            if *alive != 0 {
                model *= Mat4::from_translation(Vec3::new(0.0, 0.0, 200.0));
            }
            // func_8002EBCC(&this->actor, play, 0): the look-at for the texgen limbs' shine (the
            // renderer's texgen reads the view).
            MeshKey::named(keys::bake(BAKE_GOLD))
        } else {
            if *dashing != 0 {
                // sp30 { 184, 0, 228, 255 }; temp_f2 = (11500.0f / 0x1E) * (0x1E - 0x14).
                let far = ((11500.0f32 / 30.0) * (30 - 20) as f32) as i16 as i32;
                params.fog = Some(oot_game::gbi::gfx_set_fog(184, 0, 228, 255, 0, far));
            }
            MeshKey::named(keys::mesh(OBJECT, SKELETON))
        };
        let bones = skeleton.pose(joints);
        out.opa.push(DrawCmd { mesh, transform: model, bones, params });
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
