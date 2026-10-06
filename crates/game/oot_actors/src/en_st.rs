//! `En_St` (`ovl_En_St/z_en_st.c`): the Skulltula, a big spider hanging from the ceiling on its
//! thread that drops in front of Link, armoured in front and soft on its back.
//!
//! Params: 0 the normal one, 1 the big one (1.4 times the size: its scale, colliders and
//! hover height), 2 the invisible one (`ACTOR_FLAG_REACT_TO_LENS`). The Deku Tree's two (MQ:
//! room 2 at (-1257, 748, 1249), room 5 at (-1347, -806, 1079)) are both 0.
//!
//! - **On the ceiling** (`EnSt_WaitOnCeiling`): bobbing (`EnSt_Bob`: up and down by 0.5 as
//!   `play->state.frames & 8` says), until Link is within 160 across, below it (up to 400) and
//!   not below its floor; then it **drops** (`EnSt_MoveToGround`) at 10 a frame,
//!   `NA_SE_EN_STALTU_DOWN` every 3 frames, and close to the ground a white shockwave
//!   (`EffectSsBlast_SpawnWhiteShockwaveSetScale`) as it **lands** (`EnSt_LandOnGround`,
//!   `NA_SE_EN_STALTU_DOWN_SET` 14 frames on), rising to 32 above its floor.
//! - **On the ground** (`EnSt_WaitOnGround`): bobbing, laughing every 64 frames, its teeth
//!   flashing red, turning to face Link for 30 frames and away for 30 (`EnSt_UpdateYaw`,
//!   shaking for the last 10 of each, `NA_SE_EN_STALTU_ROLL` at each turn). Link out of range,
//!   it **returns to the ceiling** (`EnSt_ReturnToCeiling`) and waits there again.
//! - **Its colliders** (`EnSt_InitColliders`): six cylinders and a sphere.
//!   - The body (0) takes Din's Fire, arrows, the hookshot, the hammer, the boomerang,
//!     explosives and Deku Nuts from any side.
//!   - The legs take everything else: the back (1) when Link is behind it, the front (2, metal,
//!     hookable) when he's in front (`EnSt_SetLegsCylinderAC`). A Deku Stick hits the legs,
//!     or the body when it burns (`EnSt_CheckBodyStickHit`, Player's `unk_860`).
//!   - Three OC cylinders in a row across it (`EnSt_SetCylinderOC`) push Link and hurt him.
//!   - The sphere on its root limb is moved by the draw (`EnSt_PostLimbDraw`) but never
//!     registered.
//! - **Hit in front** (`EnSt_CheckHitFrontside`): no damage; it **sways** on its thread for 60
//!   frames (`EnSt_Sway`, `NA_SE_EN_STALTU_WAVE` at each swing's end).
//! - **Hit on the back or the body** (`EnSt_CheckHitBackside`, `DamageTable_Get(2)`):
//!   - a Deku Nut or the boomerang stuns it for 120 frames (blue, shaking for the last 30);
//!   - anything else spins it (flashing red, `NA_SE_EN_STALTU_DAMAGE`);
//!   - at 0 health (2 to start; the Kokiri Sword's slash does 1) the finishing blow
//!     (`Enemy_StartFinishingBlow`): it falls and **bounces** (`EnSt_BounceAround`: 6 up, then
//!     3, staying down at the third landing; dust each time), turns over on its back (`EnSt_FinishBouncing`) for 20 frames, then
//!     **dies** (`EnSt_Die`) in seven flames (`EffectSsDeadDb`) and drops from table 14; an
//!     arrow's kill skips the bounces.
//! - **Touching Link** (`EnSt_CheckHitPlayer`, its OC cylinders): half a heart
//!   (`play->damagePlayer(play, -8)`, `NA_SE_PL_BODY_HIT`), a knockdown
//!   (`Actor_SetPlayerKnockbackLargeNoDamage`), and it spins for 30 frames.
//!
//! The whole overlay is ported, with `Effect_Ss_Blast` (`oot_game::effect::blast`). Not
//! reached: the Lens of Truth (params 2 sets `ACTOR_FLAG_REACT_TO_LENS`; `Actor_DrawAll`'s lens
//! handling isn't ported, so it's drawn as any actor; no Deku Tree placement uses it), arrows
//! (`ACTOR_FLAG_ATTACHED_TO_ARROW` is never set, and no arrow hits it), a burning Deku Stick
//! (Player's `unk_860` stays 0), the hammer's shock wave (`play->actorCtx.unk_02`). Not ported:
//! the spin's trail (`EnSt_CreateBlureEffect`, `EnSt_AddBlurVertex`, `EnSt_AddBlurSpace`), as
//! ADR 0033 has `EffectBlure`: `Effect_Add` finds no slot, so `blureIdx` is
//! `TOTAL_EFFECT_COUNT`, `Effect_GetByIndex` gives NULL and the trail's calls do nothing, as
//! they do in the C with no slot free. The generic circle shadow (`ActorShadow_DrawCircle`)
//! isn't ported for any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx_at, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE_INTERP, Anim, AnimationInfo, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, binang_to_rad};

pub const ACTOR_EN_ST: i16 = 0x0037;
const OBJECT: &str = "object_st";
const SKEL: &str = "object_st_Skel_005298";

/// `ACTOR_FLAG_CAN_ATTACH_TO_ARROW`, `ACTOR_FLAG_ATTACHED_TO_ARROW` (`actor.h`).
const ACTOR_FLAG_CAN_ATTACH_TO_ARROW: u32 = 1 << 14;
const ACTOR_FLAG_ATTACHED_TO_ARROW: u32 = 1 << 15;

/// `En_St_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_ST,
    name: "En_St",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
};

/// `NAVI_ENEMY_SKULLTULA`, `NAVI_ENEMY_BIG_SKULLTULA` (`actor.h`).
const NAVI_ENEMY_SKULLTULA: u8 = 0x04;
const NAVI_ENEMY_BIG_SKULLTULA: u8 = 0x05;

/// `COLLECTIBLE_DROP_TABLE_14` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_14: i16 = 14;

/// `TOTAL_EFFECT_COUNT` (`effect.h`): `SPARK_COUNT + BLURE_COUNT + SHIELD_PARTICLE_COUNT`.
const TOTAL_EFFECT_COUNT: usize = oot_game::effect::SPARK_COUNT + oot_game::effect::BLURE_COUNT + oot_game::effect::SHIELD_PARTICLE_COUNT;

/// The colliders' ids (`collider_mut`): `colliderCylinders[0..6]`, then `colliderJntSph`.
pub const COL_CYLINDER_BODY: u8 = 0;
pub const COL_CYLINDER_BACK: u8 = 1;
pub const COL_CYLINDER_FRONT: u8 = 2;
pub const COL_JNT_SPH: u8 = 6;

/// The segment the teeth's env colour is on in the bake (`gDPSetEnvColor` in
/// `EnSt_OverrideLimbDraw` before limb 4's list, the only one whose combiner reads it).
const SEG_TEETH: u8 = 0x08;
const BAKE: &str = "En_St/skel";

/// `DamageTable_Get(2)` (`z_collision_btltbls.c`'s `sDamageTablePresets[2]`, "Used by En_St,
/// En_Ssh").
pub static S_DAMAGE_TABLE_PRESET_2: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(0, 0x1), // Boomerang
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
        dmg_entry(3, 0x0), // Ice magic
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

/// `sCylinderInit`: the body and the legs (AC, hit by the player's attacks), 32 by 50, 24 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 32, height: 50, y_shift: -24, pos: [0, 0, 0] },
};

/// `sColChkInit`: 2 health, no cylinder of its own, immovable.
const COL_CHK_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 2, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `sCylinderInit2`: the three OC cylinders, 20 by 60, 30 down.
pub const CYLINDER_INIT2: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 60, y_shift: -30, pos: [0, 0, 0] },
};

/// `sJntSphElementsInit`: one sphere on limb 1 (`{0, -240, 0}`, 28), AT `0xFFCFFFFF` for 4.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 1,
        model_sphere: Sphere16 { center: [0, -240, 0], radius: 28 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// The body's damage flags (`EnSt_InitColliders`): `DMG_MAGIC_FIRE | DMG_ARROW | DMG_HOOKSHOT |
/// DMG_HAMMER_SWING | DMG_BOOMERANG | DMG_EXPLOSIVE | DMG_DEKU_NUT`.
pub const BODY_DMG_FLAGS: u32 = DMG_MAGIC_FIRE | DMG_ARROW | DMG_HOOKSHOT | DMG_HAMMER_SWING | DMG_BOOMERANG | DMG_EXPLOSIVE | DMG_DEKU_NUT;
/// The back's: `DMG_DEFAULT` without the body's, nor light and ice magic.
pub const BACK_DMG_FLAGS: u32 = DMG_DEFAULT & !BODY_DMG_FLAGS & !(DMG_MAGIC_LIGHT | DMG_MAGIC_ICE);
/// The front's: `DMG_DEFAULT` without the body's.
pub const FRONT_DMG_FLAGS: u32 = DMG_DEFAULT & !BODY_DMG_FLAGS;

/// `object_st_Anim_000304`, `object_st_Anim_005B98`, `object_st_Anim_0055A8`.
const ANIMS: [&str; 3] = ["object_st_Anim_000304", "object_st_Anim_005B98", "object_st_Anim_0055A8"];

/// `EnStAnimation`.
pub const ENST_ANIM_0: usize = 0;
pub const ENST_ANIM_1: usize = 1;
pub const ENST_ANIM_2: usize = 2;
pub const ENST_ANIM_3: usize = 3;
pub const ENST_ANIM_4: usize = 4;
pub const ENST_ANIM_5: usize = 5;
pub const ENST_ANIM_6: usize = 6;
pub const ENST_ANIM_7: usize = 7;

/// `sAnimationInfo`: (the animation in `ANIMS`, play speed, start frame, frame count (the last
/// frame when not above 0), mode, morph frames).
const S_ANIMATION_INFO: [(usize, f32, f32, f32, u8, f32); 8] = [
    (0, 1.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, 0.0),
    (1, 1.0, 0.0, -1.0, ANIMMODE_ONCE_INTERP, -8.0),
    (0, 4.0, 0.0, -1.0, ANIMMODE_ONCE_INTERP, -8.0),
    (0, 1.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, -8.0),
    (2, 1.0, 0.0, -1.0, ANIMMODE_ONCE_INTERP, -8.0),
    (0, 8.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, -8.0),
    (0, 6.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, -8.0),
    (1, 2.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, -8.0),
];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    StartOnCeilingOrGround,
    WaitOnCeiling,
    MoveToGround,
    LandOnGround,
    WaitOnGround,
    ReturnToCeiling,
    BounceAround,
    FinishBouncing,
    Die,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::StartOnCeilingOrGround => "EnSt_StartOnCeilingOrGround",
            Action::WaitOnCeiling => "EnSt_WaitOnCeiling",
            Action::MoveToGround => "EnSt_MoveToGround",
            Action::LandOnGround => "EnSt_LandOnGround",
            Action::WaitOnGround => "EnSt_WaitOnGround",
            Action::ReturnToCeiling => "EnSt_ReturnToCeiling",
            Action::BounceAround => "EnSt_BounceAround",
            Action::FinishBouncing => "EnSt_FinishBouncing",
            Action::Die => "EnSt_Die",
        }
    }
}

/// `DECR`: down by one unless 0; the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

/// `BINANG_TO_RAD_ALT`: `((f32)binang / (f32)0x8000) * M_PI`, the product in double precision.
fn binang_to_rad_alt(binang: i16) -> f32 {
    ((binang as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32
}

/// The bake the draw uses: the skeleton after `Gfx_SetupDL_25Opa` (the bake's start), the teeth's
/// env colour dynamic (`EnSt_OverrideLimbDraw`'s `gDPSetEnvColor` for limb 4).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_TEETH, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_TEETH],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
    }]
}

pub struct EnSt {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: [Option<Anim>; 3],
    pub action: Action,
    /// `colliderCylinders`: the body, the back, the front (AC), then the three OC ones.
    pub collider_cylinders: [ColliderCylinder; 6],
    /// `colliderJntSph`.
    pub collider_jnt_sph: ColliderJntSph,
    /// `initialYaw`: its home yaw, which the OC cylinders' row keeps.
    pub initial_yaw: i16,
    pub death_yaw_target: i16,
    pub ground_bounces: i16,
    /// `animFrames`: the landing animation's frames left.
    pub anim_frames: i16,
    pub sway_timer: i16,
    pub set_target_yaw_timer: i16,
    /// `rotAwayTimer`, `rotTowardsTimer`: on the ground, the frames left facing Link and facing
    /// away (the C's names are the other way round from what they do).
    pub rot_away_timer: i16,
    pub rot_towards_timer: i16,
    pub take_damage_spin_timer: i16,
    pub stun_timer: i16,
    pub invulnerable_timer: i16,
    pub sfx_timer: i16,
    pub gave_damage_spin_timer: i16,
    pub finish_death_timer: i16,
    pub death_timer: i16,
    pub abs_prev_sway_angle: i16,
    pub play_sway_flag: u8,
    pub teeth_r: u8,
    pub teeth_g: u8,
    pub teeth_b: u8,
    pub unused_pos: Vec3,
    /// `ceilingPos`: where its thread hangs from (the sway's pivot).
    pub ceiling_pos: Vec3,
    /// `blureIdx`: the spin's trail (`TOTAL_EFFECT_COUNT`: `EffectBlure` isn't ported).
    pub blure_idx: usize,
    /// `colliderScale`: 1, or 1.4 for the big one.
    pub collider_scale: f32,
    /// `floorHeightOffset`: how high above its floor it waits (32, or 44.8 for the big one).
    pub floor_height_offset: f32,
    pub sway_angle: i16,
}

impl EnSt {
    /// `EnSt_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 14): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let (skeleton, anims) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_St: {e:#}")).ok();
                let anims = ANIMS.map(|n| a.animation(OBJECT, n).map_err(|e| log::error!("En_St: {e:#}")).ok());
                (s, anims)
            }
            None => (None, [None, None, None]),
        };
        // SkelAnime_Init(&object_st_Skel_005298, NULL, jointTable, morphTable, 30).
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(29);
        let skel = SkelAnimeStd::init_flex(limbs, None);
        let mut s = EnSt {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::StartOnCeilingOrGround,
            collider_cylinders: Default::default(),
            collider_jnt_sph: ColliderJntSph::default(),
            initial_yaw: 0,
            death_yaw_target: 0,
            ground_bounces: 0,
            anim_frames: 0,
            sway_timer: 0,
            set_target_yaw_timer: 0,
            rot_away_timer: 0,
            rot_towards_timer: 0,
            take_damage_spin_timer: 0,
            stun_timer: 0,
            invulnerable_timer: 0,
            sfx_timer: 0,
            gave_damage_spin_timer: 0,
            finish_death_timer: 0,
            death_timer: 0,
            abs_prev_sway_angle: 0,
            play_sway_flag: 0,
            teeth_r: 0,
            teeth_g: 0,
            teeth_b: 0,
            unused_pos: Vec3::ZERO,
            ceiling_pos: Vec3::ZERO,
            blure_idx: TOTAL_EFFECT_COUNT,
            collider_scale: 0.0,
            floor_height_offset: 0.0,
            sway_angle: 0,
        };
        s.change_by_info(ENST_ANIM_0);
        s.blure_idx = Self::create_blure_effect(play);
        s.init_colliders();
        if s.actor.params == 2 {
            s.actor.flags |= ACTOR_FLAG_REACT_TO_LENS;
        }
        s.actor.navi_enemy_id = if s.actor.params == 1 { NAVI_ENEMY_BIG_SKULLTULA } else { NAVI_ENEMY_SKULLTULA };
        s.check_ceiling_pos(play);
        s.actor.flags |= ACTOR_FLAG_CAN_ATTACH_TO_ARROW;
        s.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        s.set_collider_scale();
        s.actor.gravity = 0.0;
        s.initial_yaw = s.actor.world_rot.y;
        s.action = Action::StartOnCeilingOrGround;
        Box::new(s)
    }

    /// `Animation_ChangeByInfo(&this->skelAnime, sAnimationInfo, index)`.
    fn change_by_info(&mut self, index: usize) {
        let (anim, play_speed, start_frame, frame_count, mode, morph_frames) = S_ANIMATION_INFO[index];
        if let Some(animation) = self.anims[anim].clone() {
            self.skel.change_by_info(&AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames });
        }
    }

    /// `EnSt_SpawnDust`: `dust_cnt + 1` puffs of brown dust 22 out round it on its floor, from a
    /// random angle on by `0x10000 / dust_cnt` (`func_8002836C`, 120 big, growing by 40, 10
    /// frames), each drifting at random.
    fn spawn_dust(&self, play: &mut PlayState, dust_cnt: i32) {
        let prim_color = [170, 130, 90, 255];
        let env_color = [100, 60, 20, 0];
        let dust_vel = Vec3::ZERO;
        let (floor_height, pos) = (self.actor.floor_height, self.actor.world_pos);
        play.with_ss(|ss| {
            let mut y_angle = ((ss.rand.zero_one() - 0.5) * 65536.0) as i32 as i16;
            let mut dust_accel = Vec3::new(0.0, 0.3, 0.0);
            let mut i = dust_cnt;
            while i >= 0 {
                dust_accel.x = (ss.rand.zero_one() - 0.5) * 4.0;
                dust_accel.z = (ss.rand.zero_one() - 0.5) * 4.0;
                let dust_pos = Vec3::new(pos.x + (sin_s(y_angle) * 22.0), floor_height, pos.z + (cos_s(y_angle) * 22.0));
                ss.func_8002836c(dust_pos, dust_vel, dust_accel, prim_color, env_color, 120, 40, 10);
                i -= 1;
                y_angle = y_angle.wrapping_add((0x10000 / dust_cnt) as i16);
            }
        });
    }

    /// `EnSt_SpawnBlastEffect`: a white shockwave on its floor (100 big, spreading by 220, 8
    /// frames).
    fn spawn_blast_effect(&self, play: &mut PlayState) {
        let zero = Vec3::ZERO;
        let blast_pos = Vec3::new(self.actor.world_pos.x, self.actor.floor_height, self.actor.world_pos.z);
        play.with_ss(|ss| ss.blast_spawn_white_shockwave_set_scale(blast_pos, zero, zero, 100, 220, 8));
    }

    /// `EnSt_SpawnDeadEffect`: a flame (`EffectSsDeadDb_Spawn`, 100 big, white fading to red, 9
    /// frames, with its sound) somewhere within 30 across and 22.5 up or down of 10 above it.
    fn spawn_dead_effect(&self, play: &mut PlayState) {
        let zero = Vec3::ZERO;
        let p = self.actor.world_pos;
        let x = p.x + ((play.rand.zero_one() - 0.5) * 60.0);
        let y = (p.y + 10.0) + ((play.rand.zero_one() - 0.5) * 45.0);
        let z = p.z + ((play.rand.zero_one() - 0.5) * 60.0);
        let fire_pos = Vec3::new(x, y, z);
        play.with_ss(|ss| ss.dead_db_spawn(fire_pos, zero, zero, 100, 0, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
    }

    /// `EnSt_CreateBlureEffect`: the spin's trail (`EFFECT_BLURE1`: white from alpha 75 to 0,
    /// `elemDuration` 6, `calcMode` 3). `EffectBlure` isn't ported (ADR 0033): `Effect_Add` finds
    /// no slot and gives `TOTAL_EFFECT_COUNT`.
    fn create_blure_effect(play: &mut PlayState) -> usize {
        play.effect_add(EffectInit::Blure)
    }

    /// `EnSt_CheckCeilingPos`: the ceiling straight above (`BgCheck_EntityLineTest1`, 1000 up,
    /// ceilings only, one face); without one, the line's end, 1000 above.
    fn check_ceiling_pos(&mut self, play: &PlayState) -> bool {
        let p = self.actor.world_pos;
        let check_pos = Vec3::new(p.x, p.y + 1000.0, p.z);
        match play.col.entity_line_test(p, check_pos, false, false, true, true) {
            None => {
                self.ceiling_pos = check_pos;
                false
            }
            Some((hit, _poly)) => {
                self.ceiling_pos = hit;
                self.unused_pos = self.actor.world_pos;
                self.unused_pos.y -= 100.0;
                true
            }
        }
    }

    /// `EnSt_AddBlurVertex`: the trail's two points (`{834, 834, 0}` and `{834, -584, 0}` times
    /// `colliderScale`) through the matrix `mtx`, added to the trail
    /// (`EffectBlure_AddVertex(Effect_GetByIndex(blureIdx))`). With `blureIdx`
    /// `TOTAL_EFFECT_COUNT`, `Effect_GetByIndex` gives NULL and the add does nothing.
    fn add_blur_vertex(&self, mtx: &MtxF) {
        let s = self.collider_scale;
        let v1 = Vec3::new(834.0 * s, 834.0 * s, 0.0 * s);
        let v2 = Vec3::new(834.0 * s, -584.0 * s, 0.0 * s);
        let _v1_pos = mtx.mult_vec3f(v1);
        let _v2_pos = mtx.mult_vec3f(v2);
        debug_assert_eq!(self.blure_idx, TOTAL_EFFECT_COUNT);
    }

    /// `EnSt_AddBlurSpace`: `EffectBlure_AddSpace(Effect_GetByIndex(blureIdx))`, nothing with no
    /// trail (as `add_blur_vertex`).
    fn add_blur_space(&self) {
        debug_assert_eq!(self.blure_idx, TOTAL_EFFECT_COUNT);
    }

    /// `EnSt_SetWaitingAnimation`.
    fn set_waiting_animation(&mut self) {
        self.change_by_info(ENST_ANIM_3);
    }

    /// `EnSt_SetReturnToCeilingAnimation`: `NA_SE_EN_STALTU_UP`.
    fn set_return_to_ceiling_animation(&mut self, play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EN_STALTU_UP);
        self.change_by_info(ENST_ANIM_2);
    }

    /// `EnSt_SetLandAnimation`: at its hover height above the floor.
    fn set_land_animation(&mut self) {
        self.actor.world_pos.y = self.actor.floor_height + self.floor_height_offset;
        self.change_by_info(ENST_ANIM_4);
        self.sfx_timer = 0;
        self.anim_frames = self.skel.anim_length as i16;
    }

    /// `EnSt_SetDropAnimAndVel`: down at 10 (the animation unless spinning).
    fn set_drop_anim_and_vel(&mut self) {
        if self.take_damage_spin_timer == 0 {
            self.change_by_info(ENST_ANIM_4);
            self.anim_frames = self.skel.anim_length as i16;
        }
        self.sfx_timer = 0;
        self.actor.velocity.y = -10.0;
    }

    /// `EnSt_InitColliders`: the six cylinders (three `sCylinderInit`, three `sCylinderInit2`),
    /// the AC ones' damage flags, the front metal and hookable (no AT info back to the attack),
    /// `DamageTable_Get(2)`, and the sphere.
    fn init_colliders(&mut self) {
        let cylinders = [&CYLINDER_INIT, &CYLINDER_INIT, &CYLINDER_INIT, &CYLINDER_INIT2, &CYLINDER_INIT2, &CYLINDER_INIT2];
        for (c, init) in self.collider_cylinders.iter_mut().zip(cylinders) {
            *c = ColliderCylinder::new(init);
        }
        self.collider_cylinders[0].info.ac_dmg_info.dmg_flags = BODY_DMG_FLAGS;
        self.collider_cylinders[1].info.ac_dmg_info.dmg_flags = BACK_DMG_FLAGS;
        self.collider_cylinders[2].base.col_type = COL_MATERIAL_METAL;
        self.collider_cylinders[2].info.ac_elem_flags = ACELEM_ON | ACELEM_HOOKABLE | ACELEM_NO_AT_INFO;
        self.collider_cylinders[2].info.elem_material = ELEM_MATERIAL_UNK2;
        self.collider_cylinders[2].info.ac_dmg_info.dmg_flags = FRONT_DMG_FLAGS;
        self.actor.col_chk_info.set_info2(Some(&S_DAMAGE_TABLE_PRESET_2), &COL_CHK_INIT);
        self.collider_jnt_sph = ColliderJntSph::new(&JNT_SPH_INIT, &jnt_sph_elements());
    }

    /// `EnSt_CheckBodyStickHit`: a burning Deku Stick (Player's `unk_860`) hits the body, else
    /// the legs.
    fn check_body_stick_hit(&mut self, play: &PlayState) {
        let unk_860 = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.unk_860).unwrap_or(0);
        let c = &mut self.collider_cylinders;
        if unk_860 != 0 {
            c[0].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
            c[1].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
            c[2].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
        } else {
            c[0].info.ac_dmg_info.dmg_flags &= !DMG_DEKU_STICK;
            c[1].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
            c[2].info.ac_dmg_info.dmg_flags |= DMG_DEKU_STICK;
        }
    }

    /// `EnSt_SetBodyCylinderAC`.
    fn set_body_cylinder_ac(&mut self, play: &mut PlayState) {
        self.collider_cylinders[0].update(&self.actor);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider_cylinders[0]);
    }

    /// `EnSt_SetLegsCylinderAC`: the front's with Link within 0x3FFC of its facing, else the
    /// back's. (`ABS` of the `s16` difference -0x8000 stays -0x8000 in the `s16`: Link right
    /// behind it by that count is "in front".)
    fn set_legs_cylinder_ac(&mut self, play: &mut PlayState) {
        let angle_towards_link = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() as i16;
        let i = if angle_towards_link < 0x3FFC { 2 } else { 1 };
        self.collider_cylinders[i].update(&self.actor);
        play.collision_check_set_ac(&self.actor, i as u8, &mut self.collider_cylinders[i]);
    }

    /// `EnSt_SetCylinderOC`: the three OC cylinders 40 (times `colliderScale`) apart across its
    /// home facing (`initialYaw`), at its position.
    fn set_cylinder_oc(&mut self, play: &mut PlayState) -> bool {
        let mut cyl_offsets = [Vec3::new(40.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(-40.0, 0.0, 0.0)];
        for (i, off) in cyl_offsets.iter_mut().enumerate() {
            let p = self.actor.world_pos;
            *off *= self.collider_scale;
            let mut m = MtxF::set_translate(p.x, p.y, p.z);
            m.rotate_y(binang_to_rad_alt(self.initial_yaw));
            let cyl_pos = m.mult_vec3f(*off);
            let c = &mut self.collider_cylinders[i + 3];
            c.dim.pos = [cyl_pos.x as i32 as i16, cyl_pos.y as i32 as i16, cyl_pos.z as i32 as i16];
            play.collision_check_set_oc(&self.actor, (i + 3) as u8, c);
        }
        true
    }

    /// `EnSt_UpdateCylinders`: alive (or turning over after its bounces), the OC cylinders when
    /// it isn't spinning from hurting Link, the AC ones when neither invulnerable nor spinning
    /// from a hit.
    fn update_cylinders(&mut self, play: &mut PlayState) {
        if self.actor.col_chk_info.health != 0 || self.action == Action::FinishBouncing {
            if decr(&mut self.gave_damage_spin_timer) == 0 {
                self.set_cylinder_oc(play);
            }
            decr(&mut self.invulnerable_timer);
            decr(&mut self.take_damage_spin_timer);
            if self.invulnerable_timer == 0 && self.take_damage_spin_timer == 0 {
                self.set_body_cylinder_ac(play);
                self.set_legs_cylinder_ac(play);
            }
        }
    }

    /// `EnSt_CheckHitPlayer`: an OC cylinder touched Link: half a heart off him
    /// (`play->damagePlayer(play, -8)`), `NA_SE_PL_BODY_HIT` at him, a knockdown at 4 (6 up)
    /// away from it, and it spins for 30 frames (`NA_SE_EN_STALTU_ROLL` unless swaying).
    fn check_hit_player(&mut self, play: &mut PlayState) -> bool {
        let mut hit = false;
        for i in 0..3 {
            if self.collider_cylinders[i + 3].base.oc_flags2 & OC2_HIT_PLAYER == 0 {
                continue;
            }
            self.collider_cylinders[i + 3].base.oc_flags2 &= !OC2_HIT_PLAYER;
            hit = true;
        }
        if !hit {
            return false;
        }
        if self.sway_timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_ROLL);
        }
        self.gave_damage_spin_timer = 30;
        crate::player::play_damage_player(play, -8);
        if let Some(player) = play.player {
            audio_play_actor_sfx_at(play, player, NA_SE_PL_BODY_HIT);
        }
        actor_set_player_knockback_large_no_damage(play, 4.0, self.actor.yaw_towards_player, 6.0);
        true
    }

    /// `EnSt_CheckHitFrontside`: the front hit: 8 frames invulnerable, and 60 of swaying.
    fn check_hit_frontside(&mut self) -> bool {
        let ac_flags = self.collider_cylinders[2].base.ac_flags;
        if ac_flags & AC_HIT == 0 {
            // Not hit.
            false
        } else {
            self.collider_cylinders[2].base.ac_flags &= !AC_HIT;
            self.invulnerable_timer = 8;
            self.play_sway_flag = 0;
            self.sway_timer = 60;
            true
        }
    }

    /// `EnSt_CheckHitBackside`: the body or the back hit (their attacks' damage flags gathered):
    /// 8 frames invulnerable; a stun (`damageReaction` 1: a Deku Nut, the boomerang) for 120
    /// frames, blue, unless stunned already; else it spins (`ENST_ANIM_3`'s length, flashing red
    /// as long) and takes the damage. True when that killed it: the finishing blow, not
    /// targetable, three bounces after 20 frames' fall by 1 (`NA_SE_EN_STALWALL_DEAD`), or with
    /// an arrow straight to dying.
    fn check_hit_backside(&mut self, play: &mut PlayState) -> bool {
        let mut flags = 0u32;
        let mut hit = false;
        for i in [0, 1] {
            let c = &mut self.collider_cylinders[i];
            if c.base.ac_flags & AC_HIT != 0 {
                c.base.ac_flags &= !AC_HIT;
                hit = true;
                flags |= c.info.ac_hit_elem.map(|h| h.at_dmg_info.dmg_flags).unwrap_or(0);
            }
        }
        if !hit {
            return false;
        }
        self.invulnerable_timer = 8;
        if self.actor.col_chk_info.damage_reaction == 1 {
            if self.stun_timer == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
                self.stun_timer = 120;
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, self.stun_timer as u16);
            }
            return false;
        }
        self.stun_timer = 0;
        self.sway_timer = 0;
        self.gave_damage_spin_timer = 1;
        self.change_by_info(ENST_ANIM_3);
        self.take_damage_spin_timer = self.skel.anim_length as i16;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, self.take_damage_spin_timer as u16);
        if self.actor.apply_damage() != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_DAMAGE);
            return false;
        }
        enemy_start_finishing_blow(play, &self.actor);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.ground_bounces = 3;
        self.death_timer = 20;
        self.actor.gravity = -1.0;
        audio_play_actor_sfx2(play, NA_SE_EN_STALWALL_DEAD);
        if flags & DMG_ARROW != 0 {
            self.action = Action::Die;
            self.finish_death_timer = 8;
        } else {
            self.action = Action::BounceAround;
        }
        true
    }

    /// `EnSt_CheckColliders`: true when a hit dealt damage this frame (or the hammer's shock
    /// wave is going: `actorCtx.unk_02`); a hit in front first; touching Link when neither
    /// stunned nor spinning from a hit.
    fn check_colliders(&mut self, play: &mut PlayState) -> bool {
        if self.check_hit_frontside() {
            // Link has hit the front shield area of the Skulltula.
            return false;
        }
        if play.actors.unk_02 != 0 {
            return true;
        }
        if self.check_hit_backside(play) {
            // Link has hit the backside of the Skulltula.
            return true;
        }
        if self.stun_timer == 0 && self.take_damage_spin_timer == 0 {
            // Check if the Skulltula has hit Link.
            self.check_hit_player(play);
        }
        false
    }

    /// `EnSt_SetColliderScale`: the big one (params 1) 1.4 times: the sphere's radius, each
    /// cylinder's shift, radius and height, the scale (0.04) and the hover height (32).
    fn set_collider_scale(&mut self) {
        let mut scale_amount = 1.0f32;
        if self.actor.params == 1 {
            scale_amount = 1.4;
        }
        let e = &mut self.collider_jnt_sph.elements[0];
        let radius = e.dim.model_sphere.radius as f32 * scale_amount;
        e.dim.model_sphere.radius = radius as i32 as i16;
        for c in self.collider_cylinders.iter_mut() {
            let y_shift = c.dim.y_shift as f32 * scale_amount;
            let radius = c.dim.radius as f32 * scale_amount;
            let height = c.dim.height as f32 * scale_amount;
            c.dim.y_shift = y_shift as i32 as i16;
            c.dim.radius = radius as i32 as i16;
            c.dim.height = height as i32 as i16;
        }
        self.actor.scale = Vec3::splat(0.04 * scale_amount);
        self.collider_scale = scale_amount;
        self.floor_height_offset = 32.0 * scale_amount;
    }

    /// `EnSt_SetTeethColor`: the teeth's colour towards the target by `255 / (s16)(0.6 ×
    /// min_max_step)` a frame (63 for 8).
    fn set_teeth_color(&mut self, red_target: i16, green_target: i16, blue_target: i16, min_max_step: i16) -> i32 {
        let mut red = self.teeth_r as i16;
        let mut green = self.teeth_g as i16;
        let mut blue = self.teeth_b as i16;
        let mut min_max_step = (255 / ((0.6f32 * min_max_step as f32) as i32 as i16) as i32) as i16;
        if min_max_step <= 0 {
            min_max_step = 1;
        }
        smooth_step_to_s(&mut red, red_target, 1, min_max_step, min_max_step);
        smooth_step_to_s(&mut green, green_target, 1, min_max_step, min_max_step);
        smooth_step_to_s(&mut blue, blue_target, 1, min_max_step, min_max_step);
        self.teeth_r = red as u8;
        self.teeth_g = green as u8;
        self.teeth_b = blue as u8;
        1
    }

    /// `EnSt_DecrStunTimer`.
    ///
    /// @bug (game): no return when it counts down: the C returns whatever `v0` holds (the timer
    /// before the decrement). The value isn't used.
    fn decr_stun_timer(&mut self) -> i32 {
        if self.stun_timer == 0 {
            return 0;
        }
        let before = self.stun_timer;
        self.stun_timer -= 1;
        before as i32
    }

    /// `EnSt_UpdateYaw`: stunned, it shakes (±0x800) for the stun's last 30 frames. Otherwise,
    /// unless swaying or dying: spinning (0x2000 a frame) after a hit or hurting Link; off the
    /// ground, back to its home yaw (both timers reset); on the ground, facing Link while
    /// `rotAwayTimer` runs, then away while `rotTowardsTimer` runs, 30 frames each
    /// (`NA_SE_EN_STALTU_ROLL` at each change), turning by at most 0x2000 (smoothly within
    /// 0x4000), shaking for each's last 10 frames.
    fn update_yaw(&mut self, play: &mut PlayState) {
        let mut yaw_dir: u16 = 0;
        // Shake towards the end of the stun.
        if self.stun_timer != 0 {
            if self.stun_timer < 30 {
                if (self.stun_timer % 2) != 0 {
                    self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
                } else {
                    self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x800);
                }
            }
            return;
        }
        if self.sway_timer == 0 && self.death_timer == 0 && self.finish_death_timer == 0 {
            // Not swaying or dying.
            if self.take_damage_spin_timer != 0 || self.gave_damage_spin_timer != 0 {
                // A spinning animation.
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x2000);
                return;
            }
            if self.action != Action::WaitOnGround {
                self.rot_away_timer = 30;
                self.rot_towards_timer = 0;
            }
            if self.rot_away_timer != 0 {
                self.rot_away_timer -= 1;
                if self.rot_away_timer == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_STALTU_ROLL);
                    self.rot_towards_timer = 30;
                }
            } else if self.rot_towards_timer != 0 {
                self.rot_towards_timer -= 1;
                if self.rot_towards_timer == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_STALTU_ROLL);
                    self.rot_away_timer = 30;
                }
                yaw_dir = 0x8000;
            }
            // The new yaw to or away from Link (s16 ^ u16 in int, back to s16).
            let mut rot = self.actor.shape_rot;
            let yaw_target = if self.action == Action::WaitOnGround { self.actor.yaw_towards_player } else { self.initial_yaw };
            let target = (yaw_target as i32 ^ yaw_dir as i32) as i16;
            let yaw_diff = rot.y.wrapping_sub(target);
            if (yaw_diff as i32).abs() <= 0x4000 {
                smooth_step_to_s(&mut rot.y, target, 4, 0x2000, 1);
            } else {
                rot.y = rot.y.wrapping_add(0x2000);
            }
            self.actor.shape_rot = rot;
            self.actor.world_rot = rot;
            // The shaking.
            let timer = if yaw_dir == 0 && self.rot_away_timer < 0xA {
                self.rot_away_timer
            } else if yaw_dir == 0x8000 && self.rot_towards_timer < 0xA {
                self.rot_towards_timer
            } else {
                return;
            };
            if (timer % 2) != 0 {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
            } else {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x800);
            }
        }
    }

    /// `EnSt_IsDoneBouncing`: falling onto the ground with bounces left: `NA_SE_EN_DODO_M_GND`,
    /// eleven puffs of dust, and up again at `6 / (4 - groundBounces)`; true at the last one
    /// (staying down).
    fn is_done_bouncing(&mut self, play: &mut PlayState) -> bool {
        if self.actor.velocity.y > 0.0 || self.ground_bounces == 0 {
            // Moving upwards, or no bounces left.
            return false;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            // Not on the ground.
            return false;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
        self.spawn_dust(play, 10);
        // An elastic bounce, less for each hit on the ground.
        self.actor.velocity.y = 6.0 / (4 - self.ground_bounces) as f32;
        self.ground_bounces -= 1;
        if self.ground_bounces != 0 {
            return false;
        }
        // Stay on the ground.
        self.actor.velocity.y = 0.0;
        true
    }

    /// `EnSt_Bob`: the vertical speed towards 0.5, or -0.5 while `play->state.frames & 8`.
    fn bob(&mut self, play: &PlayState) {
        let mut y_speed_target = 0.5f32;
        if (play.state_frames & 8) != 0 {
            y_speed_target *= -1.0;
        }
        smooth_step_to_f(&mut self.actor.velocity.y, y_speed_target, 0.4, 1000.0, 0.0);
    }

    /// `EnSt_IsCloseToPlayer`: not spinning from a hit, Link within 160 across, below it by at
    /// most 400, and not below its floor.
    fn is_close_to_player(&self, play: &PlayState) -> bool {
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
        if self.take_damage_spin_timer != 0 {
            // Spinning from damage.
            return false;
        } else if self.actor.xz_dist_to_player > 160.0 {
            return false;
        }
        let y_dist = self.actor.world_pos.y - player_y;
        if !(0.0..=400.0).contains(&y_dist) {
            // Link above it, or more than 400 below.
            return false;
        }
        if player_y < self.actor.floor_height {
            // Link below its floor.
            return false;
        }
        true
    }

    /// `EnSt_IsCloseToInitialPos`: two frames' rise from its home height.
    fn is_close_to_initial_pos(&self) -> bool {
        let vel_y = self.actor.velocity.y;
        let check_y = self.actor.world_pos.y + (vel_y * 2.0);
        check_y >= self.actor.home_pos.y
    }

    /// `EnSt_IsCloseToGround`: two frames' fall from its hover height.
    fn is_close_to_ground(&self) -> bool {
        let vel_y = self.actor.velocity.y;
        let check_y = self.actor.world_pos.y + (vel_y * 2.0);
        check_y - self.actor.floor_height <= self.floor_height_offset
    }

    /// `EnSt_Sway`: after a hit in front, swinging on its 200-long thread from `ceilingPos`
    /// across its facing, by `sin(swayAngle)` of a swing shrinking with the timer
    /// (`swayTimer × 7/15` degrees), `NA_SE_EN_STALTU_WAVE` at each swing's end, leaning by twice
    /// the angle.
    fn sway(&mut self, play: &mut PlayState) {
        if self.sway_timer != 0 {
            self.sway_angle = self.sway_angle.wrapping_add(0xA28);
            self.sway_timer -= 1;
            if self.sway_timer == 0 {
                self.sway_angle = 0;
            }
            let sway_amt = self.sway_timer as f32 * (7.0f32 / 15.0f32);
            let rot_angle = (sin_s(self.sway_angle) * (sway_amt * (65536.0f32 / 360.0f32))) as i32 as i16;
            let abs_rot_angle = (rot_angle as i32).abs() as i16;
            if self.abs_prev_sway_angle >= abs_rot_angle && self.play_sway_flag == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_STALTU_WAVE);
                self.play_sway_flag = 1;
            }
            if self.abs_prev_sway_angle < abs_rot_angle {
                self.play_sway_flag = 0;
            }
            self.abs_prev_sway_angle = abs_rot_angle;
            let amt_to_translate = Vec3::new(sin_s(rot_angle) * -200.0, cos_s(rot_angle) * -200.0, 0.0);
            let c = self.ceiling_pos;
            let mut m = MtxF::set_translate(c.x, c.y, c.z);
            m.rotate_y(binang_to_rad(self.actor.world_rot.y));
            let translated_pos = m.mult_vec3f(amt_to_translate);
            self.actor.shape_rot.z = (-(rot_angle as i32 * 2)) as i16;
            self.actor.world_pos.x = translated_pos.x;
            self.actor.world_pos.z = translated_pos.z;
        }
    }

    /// `EnSt_WaitOnCeiling`: Link near, it drops; else it bobs.
    fn wait_on_ceiling(&mut self, play: &mut PlayState) {
        if self.is_close_to_player(play) {
            self.set_drop_anim_and_vel();
            self.action = Action::MoveToGround;
        } else {
            self.bob(play);
        }
    }

    /// `EnSt_WaitOnGround`: the spin's and the landing's animations to the waiting one; Link out
    /// of range, back up; else a laugh every 64 frames (`NA_SE_EN_STALTU_LAUGH`) and the bob.
    fn wait_on_ground(&mut self, play: &mut PlayState) {
        if self.take_damage_spin_timer != 0 {
            self.take_damage_spin_timer -= 1;
            if self.take_damage_spin_timer == 0 {
                self.change_by_info(ENST_ANIM_3);
            }
        }
        if self.anim_frames != 0 {
            self.anim_frames -= 1;
            if self.anim_frames == 0 {
                self.change_by_info(ENST_ANIM_3);
            }
        }
        if !self.is_close_to_player(play) {
            // Link is out of range: back to the ceiling.
            self.set_return_to_ceiling_animation(play);
            self.action = Action::ReturnToCeiling;
            return;
        }
        if decr(&mut self.sfx_timer) == 0 {
            // The laugh, every 64 frames.
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_LAUGH);
            self.sfx_timer = 64;
        }
        // Bob up and down.
        self.bob(play);
    }

    /// `EnSt_LandOnGround`: the animations as `EnSt_WaitOnGround`'s, `NA_SE_EN_STALTU_DOWN_SET`
    /// on its 14th frame; rising (towards 2 a frame) until above its hover height, then waiting
    /// there.
    fn land_on_ground(&mut self, play: &mut PlayState) {
        if self.anim_frames != 0 {
            self.anim_frames -= 1;
            if self.anim_frames == 0 {
                self.change_by_info(ENST_ANIM_3);
            }
        }
        if self.take_damage_spin_timer != 0 {
            self.take_damage_spin_timer -= 1;
            if self.take_damage_spin_timer == 0 {
                self.change_by_info(ENST_ANIM_3);
            }
        }
        self.sfx_timer += 1;
        if self.sfx_timer == 14 {
            // The sound of the Skulltula hitting the ground.
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_DOWN_SET);
        }
        if (self.actor.floor_height + self.floor_height_offset) < self.actor.world_pos.y {
            // Up at its hover height.
            self.sfx_timer = 0;
            self.action = Action::WaitOnGround;
        } else {
            smooth_step_to_f(&mut self.actor.velocity.y, 2.0, 0.3, 1.0, 0.0);
        }
    }

    /// `EnSt_MoveToGround`: Link out of range, back up; close to the ground, the shockwave and
    /// the landing; else `NA_SE_EN_STALTU_DOWN` every 3 frames.
    fn move_to_ground(&mut self, play: &mut PlayState) {
        if self.take_damage_spin_timer != 0 {
            self.take_damage_spin_timer -= 1;
            if self.take_damage_spin_timer == 0 {
                self.change_by_info(ENST_ANIM_5);
            }
        }
        if !self.is_close_to_player(play) {
            // Link moved out of range: back to the ceiling.
            self.set_return_to_ceiling_animation(play);
            self.action = Action::ReturnToCeiling;
        } else if self.is_close_to_ground() {
            self.spawn_blast_effect(play);
            self.set_land_animation();
            self.action = Action::LandOnGround;
        } else if decr(&mut self.sfx_timer) == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALTU_DOWN);
            self.sfx_timer = 3;
        }
    }

    /// `EnSt_ReturnToCeiling`: climbing at `4 ×` the climbing animation's progress (again from
    /// its start when it ends); Link back in range, it drops; near its home height, it waits.
    fn return_to_ceiling(&mut self, play: &mut PlayState) {
        let anim_pct_done = self.skel.cur_frame / (self.skel.anim_length - 1.0);
        if anim_pct_done == 1.0 {
            self.set_return_to_ceiling_animation(play);
        }
        if self.is_close_to_player(play) {
            // Link came back into range.
            self.set_drop_anim_and_vel();
            self.action = Action::MoveToGround;
        } else if self.is_close_to_initial_pos() {
            self.set_waiting_animation();
            self.action = Action::WaitOnCeiling;
        } else {
            // Accelerate by the animation's frame.
            self.actor.velocity.y = 4.0 * anim_pct_done;
        }
    }

    /// `EnSt_BounceAround`: killed: tumbling (0x800 a frame about x and -z), the red flash held
    /// by `deathTimer`, falling by 1, bouncing; its model rising to 400 above its origin; after
    /// the last bounce it turns over (`EnSt_FinishBouncing`: drifting at 1, falling by 2).
    fn bounce_around(&mut self, play: &mut PlayState) {
        self.actor.color_filter_timer = self.death_timer as u8;
        self.actor.update_velocity();
        self.actor.world_rot.x = self.actor.world_rot.x.wrapping_add(0x800);
        self.actor.world_rot.z = self.actor.world_rot.z.wrapping_sub(0x800);
        self.actor.shape_rot = self.actor.world_rot;
        if self.is_done_bouncing(play) {
            self.actor.shape_y_offset = 400.0;
            self.actor.speed_xz = 1.0;
            self.actor.gravity = -2.0;
            self.action = Action::FinishBouncing;
        } else {
            smooth_step_to_f(&mut self.actor.shape_y_offset, 400.0, 0.4, 10000.0, 0.0);
        }
    }

    /// `EnSt_FinishBouncing`: for 20 frames rolling onto its back (pitch 0x3FFC, no roll),
    /// turning towards its home (aimed every 8 frames), bouncing at 3; then it dies.
    fn finish_bouncing(&mut self, play: &mut PlayState) {
        if decr(&mut self.death_timer) == 0 {
            self.actor.velocity = Vec3::ZERO;
            self.finish_death_timer = 8;
            self.action = Action::Die;
            return;
        }
        if decr(&mut self.set_target_yaw_timer) == 0 {
            self.death_yaw_target = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
            self.set_target_yaw_timer = 8;
        }
        smooth_step_to_s(&mut self.actor.world_rot.x, 0x3FFC, 4, 0x2710, 1);
        smooth_step_to_s(&mut self.actor.world_rot.z, 0, 4, 0x2710, 1);
        smooth_step_to_s(&mut self.actor.world_rot.y, self.death_yaw_target, 0xA, 0x2710, 1);
        self.actor.shape_rot = self.actor.world_rot;
        self.actor.update_velocity();
        self.ground_bounces = 2;
        self.is_done_bouncing(play);
    }

    /// `EnSt_Die`: a flame a frame while `finishDeathTimer` counts down from 8 (seven), then its
    /// drop from table 14 and gone.
    fn die(&mut self, play: &mut PlayState) {
        if decr(&mut self.finish_death_timer) != 0 {
            self.spawn_dead_effect(play);
        } else {
            // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_14, false).
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, None, pos, COLLECTIBLE_DROP_TABLE_14 * 16);
            self.actor.kill();
        }
    }

    /// `EnSt_StartOnCeilingOrGround`: high up, it waits on the ceiling (`rotAwayTimer` 60); near
    /// its floor, it lands. Either runs at once.
    fn start_on_ceiling_or_ground(&mut self, play: &mut PlayState) {
        if !self.is_close_to_ground() {
            self.rot_away_timer = 60;
            self.action = Action::WaitOnCeiling;
            self.wait_on_ceiling(play);
        } else {
            self.set_land_animation();
            self.action = Action::LandOnGround;
            self.land_on_ground(play);
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::StartOnCeilingOrGround => self.start_on_ceiling_or_ground(play),
            Action::WaitOnCeiling => self.wait_on_ceiling(play),
            Action::MoveToGround => self.move_to_ground(play),
            Action::LandOnGround => self.land_on_ground(play),
            Action::WaitOnGround => self.wait_on_ground(play),
            Action::ReturnToCeiling => self.return_to_ceiling(play),
            Action::BounceAround => self.bounce_around(play),
            Action::FinishBouncing => self.finish_bouncing(play),
            Action::Die => self.die(play),
        }
    }

    /// `Actor_Draw`'s model matrix (`Matrix_SetTranslateRotateYXZ`, then the scale).
    fn actor_mtx(&self) -> MtxF {
        let a = &self.actor;
        let mut m = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + a.shape_y_offset * a.scale.y, a.world_pos.z, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]);
        m.scale(a.scale.x, a.scale.y, a.scale.z);
        m
    }
}

impl ActorImpl for EnSt {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSt_Update`: on an arrow, only the animation; else, unless a hit dealt damage this
    /// frame: the animation (not while stunned), the move (not while stunned or swaying), the
    /// floor, the action (or the stun's count, or the sway), the yaw, the teeth (red every
    /// other 16 frames on the ground), the colliders and the focus.
    fn update(&mut self, play: &mut PlayState) {
        let mut color = [0i16; 4];
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW != 0 {
            self.skel.update();
        } else if !self.check_colliders(play) {
            // No damage this frame.
            if self.stun_timer == 0 {
                self.skel.update();
            }
            if self.sway_timer == 0 && self.stun_timer == 0 {
                self.actor.update_pos();
            }
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
            if self.stun_timer == 0 && self.sway_timer == 0 {
                self.run_action(play);
            } else if self.stun_timer != 0 {
                self.decr_stun_timer();
            } else {
                self.sway(play);
            }
            self.update_yaw(play);
            if self.action == Action::WaitOnGround && (play.state_frames & 0x10) != 0 {
                color[0] = 255;
            }
            self.set_teeth_color(color[0], color[1], color[2], 8);
            self.update_cylinders(play);
            self.actor.set_focus(0.0);
        }
    }

    /// `ActorProfile.destroy`: `Effect_Delete` of the trail (no slot: nothing).
    fn destroy(&mut self, play: &mut PlayState) {
        play.effect_delete(self.blure_idx);
    }

    /// `EnSt_Draw`'s effects on the actor: the Deku Stick's target (`EnSt_CheckBodyStickHit`);
    /// limb 1's trail point (`EnSt_OverrideLimbDraw`, at the actor's matrix: nothing, no trail);
    /// the sphere follows limb 1 (`EnSt_PostLimbDraw`'s `Collider_UpdateSpheres`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        self.check_body_stick_hit(play);
        let model = self.actor_mtx();
        if self.gave_damage_spin_timer != 0 && self.sway_timer == 0 {
            if self.gave_damage_spin_timer >= 2 {
                self.add_blur_vertex(&model);
            } else {
                self.add_blur_space();
            }
        }
        // SkelAnime_DrawOpa's root limb: Matrix_TranslateRotateZYX(jointTable[0], jointTable[1]).
        let j = &self.skel.joint_table;
        let mut root = model;
        root.translate_rotate_zyx(Vec3::new(j[0][0] as f32, j[0][1] as f32, j[0][2] as f32), j[1]);
        self.collider_jnt_sph.update_spheres(1, &root.to_mat4());
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.teeth_r as u32, self.teeth_g as u32, self.teeth_b as u32];
        rs
    }

    /// `EnSt_Draw`: the skeleton (`SkelAnime_DrawOpa`), the teeth's env colour set before limb 4
    /// (`EnSt_OverrideLimbDraw`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [r, g, b]) = (&rs.joints, rs.switches.as_slice()) else { return };
        let Some(skeleton) = &self.skeleton else { return };
        let bones = skeleton.pose(joints);
        let mut sv = SegmentValues::default();
        sv.env[SEG_TEETH as usize] = Some([*r as u8, *g as u8, *b as u8, 0]);
        let model = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: model, bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0..=5 => Some(ColliderMut::Cylinder(&mut self.collider_cylinders[id as usize])),
            COL_JNT_SPH => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
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
