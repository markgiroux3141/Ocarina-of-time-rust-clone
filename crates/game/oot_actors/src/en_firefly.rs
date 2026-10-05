//! `En_Firefly` (`ovl_En_Firefly/z_en_firefly.c`): the Keese, a bat flying about its home
//! that swoops at Link; Fire and Ice Keese burn or freeze him, and a normal one catches fire by
//! a lit torch (`Obj_Syokudai`).
//!
//! Params (`EnFireflyType`, 0x8000: seen only with the Lens of Truth): 0 and 1 fire (1 perches
//! once its fire's out), 2 normal, 3 normal and perched, 4 ice. The Deku Tree's seven are 3.
//!
//! - **Perched** (`EnFirefly_Perched`): hanging still, flapping now and then, until Link is
//!   within 120 across; then it **attacks from its perch** (`EnFirefly_AttackFromPerched`) for 50
//!   frames, diving at him, before flying about.
//! - **Idle** (`EnFirefly_Idle`): flying at 1.5 to 3 about its home (100 above it for the normal
//!   ones), turning at random with each flap, kept off the floor and under its home's height; a
//!   perching one goes back to its perch with Link 300 away (`EnFirefly_ApproachPerchSpot`), and a
//!   normal one flies to a lit torch (`EnFirefly_ApproachLitTorch`) to catch fire. With its timer
//!   out and Link within 200, it **attacks** (`EnFirefly_Attack`) for 70 to 170 frames, homing on
//!   him at 4 while facing him, then **flies home** (`EnFirefly_FlyTowardsHome`).
//! - **Biting** Link (`AT_HIT`, `NA_SE_EN_FFLY_ATTACK`): a fire one's fire goes out; it **stays**
//!   (`EnFirefly_Stay`), slowing to a hover, then flies home.
//! - **Hit** (`EnFirefly_CheckCollide`, `sDamageTable`): a Deku Nut **stuns** it for 80 frames
//!   (blue); fire (Din's Fire) sets a normal one alight (melts an ice one); ice freezes it to
//!   fall and break (`EnFirefly_SetupDieFrozen`, `EffectSsEnIce`); anything else that does damage
//!   (the Kokiri Sword's 1) kills it at 1 health: it **dies** (`EnFirefly_Die`), flashing red and
//!   spinning as it falls, then **disappears** (`EnFirefly_Disappear`) over 15 frames and drops an
//!   item from table 14 (`Item_DropCollectibleRandom`).
//! - **Its fire or ice** (`EnFirefly_PostLimbDraw`): each wing's end trails flames or frost
//!   (`func_8002843C`'s dust, drawn every frame from the draw).
//!
//! The whole overlay is ported. Not ported, so never reached: the Lens of Truth (a params 0x8000
//! Keese is invisible: `play->actorCtx.lensActive` is never set), arrows (`ACTOR_FLAG_ATTACHED_TO_ARROW`
//! is never set), the Skull Mask (`Player_GetMask` is always `PLAYER_MASK_NONE`), and a lit torch
//! (`Obj_Syokudai` isn't ported: its placeholder has no `litTimer`, so no torch is lit). The
//! draw's env colour (alpha 0 for a fire body, 255 else) doesn't reach the skeleton's mesh, whose
//! combiner doesn't read it. The generic circle shadow (`ActorShadow_DrawCircle`) isn't ported for
//! any actor.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{atan2_s, cos_s, scaled_step_to_s, sin_s, smooth_step_to_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP_INTERP, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

pub const ACTOR_EN_FIREFLY: i16 = 0x0013;
const OBJECT: &str = "object_firefly";

/// `ACTOR_FLAG_IGNORE_QUAKE`, `ACTOR_FLAG_CAN_ATTACH_TO_ARROW`, `ACTOR_FLAG_ATTACHED_TO_ARROW`
/// (`actor.h`).
const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;
const ACTOR_FLAG_CAN_ATTACH_TO_ARROW: u32 = 1 << 14;
const ACTOR_FLAG_ATTACHED_TO_ARROW: u32 = 1 << 15;

/// `En_Firefly_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_FIREFLY,
    name: "En_Firefly",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_IGNORE_QUAKE | ACTOR_FLAG_CAN_ATTACH_TO_ARROW,
    object: OBJECT,
};

/// `EnFireflyType`.
pub const EN_FIREFLY_TYPE_FIRE: i16 = 0;
pub const EN_FIREFLY_TYPE_FIRE_CAN_PERCH: i16 = 1;
pub const EN_FIREFLY_TYPE_NORMAL: i16 = 2;
pub const EN_FIREFLY_TYPE_NORMAL_PERCHED: i16 = 3;
pub const EN_FIREFLY_TYPE_ICE: i16 = 4;

/// `EnFireflyEffectsElementalType`, `EnFireflyBodyElementalType`.
pub const EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE: u8 = 0;
pub const EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE: u8 = 1;
pub const EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_ICE: u8 = 2;
pub const EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL: u8 = 0;
pub const EN_FIREFLY_BODY_ELEMENTAL_TYPE_FIRE: u8 = 1;

/// `EnFireflyDamageReaction`.
pub const EN_FIREFLY_DMG_REACT_NONE: u8 = 0;
pub const EN_FIREFLY_DMG_REACT_STUN: u8 = 1;
pub const EN_FIREFLY_DMG_REACT_DINS_FIRE: u8 = 2;
pub const EN_FIREFLY_DMG_REACT_ICE: u8 = 3;
pub const EN_FIREFLY_DMG_REACT_FIRE_ARROW: u8 = 0xF;

/// `EnFireflyBodyPart`.
pub const EN_FIREFLY_BODY_PART_LEFT_WING: usize = 0;
pub const EN_FIREFLY_BODY_PART_RIGHT_WING: usize = 1;
pub const EN_FIREFLY_BODY_PART_BODY: usize = 2;
pub const EN_FIREFLY_BODY_PART_MAX: usize = 3;

/// `KeeseLimb` (`object_firefly.xml`'s limbs, from 1).
const KEESE_LIMB_ROOT_ROOT: usize = 1;
const KEESE_LIMB_BODY: usize = 10;
const KEESE_LIMB_LEFT_WING_END: usize = 15;
const KEESE_LIMB_RIGHT_WING_END_ROOT: usize = 21;
const KEESE_LIMB_HEAD: usize = 27;

/// `NAVI_ENEMY_FIRE_KEESE`, `NAVI_ENEMY_KEESE`, `NAVI_ENEMY_ICE_KEESE` (`actor.h`).
const NAVI_ENEMY_FIRE_KEESE: u8 = 0x11;
const NAVI_ENEMY_KEESE: u8 = 0x12;
const NAVI_ENEMY_ICE_KEESE: u8 = 0x56;

/// `ACTOR_OBJ_SYOKUDAI` (`actor_table.h`).
const ACTOR_OBJ_SYOKUDAI: i16 = 0x005E;
/// `COLLECTIBLE_DROP_TABLE_14` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_14: i16 = 14;

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, EN_FIREFLY_DMG_REACT_STUN),       // Deku nut
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Deku stick
        dmg_entry(1, EN_FIREFLY_DMG_REACT_NONE),       // Slingshot
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Explosive
        dmg_entry(1, EN_FIREFLY_DMG_REACT_NONE),       // Boomerang
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Normal arrow
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Hammer swing
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Hookshot
        dmg_entry(1, EN_FIREFLY_DMG_REACT_NONE),       // Kokiri sword
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Master sword
        dmg_entry(4, EN_FIREFLY_DMG_REACT_NONE),       // Giant's Knife
        dmg_entry(2, EN_FIREFLY_DMG_REACT_FIRE_ARROW), // Fire arrow
        dmg_entry(4, EN_FIREFLY_DMG_REACT_ICE),        // Ice arrow
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Light arrow
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Unk arrow 1
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Unk arrow 2
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Unk arrow 3
        dmg_entry(0, EN_FIREFLY_DMG_REACT_DINS_FIRE),  // Fire magic
        dmg_entry(4, EN_FIREFLY_DMG_REACT_ICE),        // Ice magic
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Light magic
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Shield
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Mirror Ray
        dmg_entry(1, EN_FIREFLY_DMG_REACT_NONE),       // Kokiri spin
        dmg_entry(4, EN_FIREFLY_DMG_REACT_NONE),       // Giant spin
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Master spin
        dmg_entry(2, EN_FIREFLY_DMG_REACT_NONE),       // Kokiri jump
        dmg_entry(8, EN_FIREFLY_DMG_REACT_NONE),       // Giant jump
        dmg_entry(4, EN_FIREFLY_DMG_REACT_NONE),       // Master jump
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Unknown 1
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Unblockable
        dmg_entry(4, EN_FIREFLY_DMG_REACT_NONE),       // Hammer jump
        dmg_entry(0, EN_FIREFLY_DMG_REACT_NONE),       // Unknown 2
    ],
};

/// `sJntSphElementsInit`: one sphere on the root (`{0, 1000, 0}`, 15): the bite
/// (`0xFFCFFFFF` for 8 with fire, `ATELEM_SFX_HARD`), hit by everything but the shield and the
/// mirror's ray.
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_FIRE, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: KEESE_LIMB_ROOT_ROOT as u8,
        model_sphere: Sphere16 { center: [0, 1000, 0], radius: 15 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sColChkInfoInit`: 1 health, 10 by 10, mass 30.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 10, cyl_height: 10, mass: 30 };

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    Die,
    Disappear,
    Attack,
    Stay,
    FlyTowardsHome,
    Stunned,
    DieFrozen,
    Perched,
    AttackFromPerched,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnFirefly_Idle",
            Action::Die => "EnFirefly_Die",
            Action::Disappear => "EnFirefly_Disappear",
            Action::Attack => "EnFirefly_Attack",
            Action::Stay => "EnFirefly_Stay",
            Action::FlyTowardsHome => "EnFirefly_FlyTowardsHome",
            Action::Stunned => "EnFirefly_Stunned",
            Action::DieFrozen => "EnFirefly_DieFrozen",
            Action::Perched => "EnFirefly_Perched",
            Action::AttackFromPerched => "EnFirefly_AttackFromPerched",
        }
    }
}

/// `Math_Vec3f_Pitch`.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

pub struct EnFirefly {
    pub actor: Actor,
    /// `bodyPartsPos`: the wings' ends and the body (5 below them), from the draw; the fire
    /// effect's positions (`Effect_Ss_En_Fire`'s `firePos`).
    pub body_parts_pos: [Vec3; EN_FIREFLY_BODY_PART_MAX],
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    fly: Option<Anim>,
    pub action: Action,
    /// `effectsElementalType`, `bodyElementalType`.
    pub effects_elemental_type: u8,
    pub body_elemental_type: u8,
    pub timer: i16,
    pub target_pitch: i16,
    /// `homeY`: the height it flies about.
    pub home_y: f32,
    pub collider: ColliderJntSph,
    /// `draw == EnFirefly_DrawXlu`: seen only with the Lens of Truth.
    pub draw_xlu: bool,
}

impl EnFirefly {
    /// `EnFirefly_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: scale 0.005, gravity -0.5, minVelocityY -4, ATTENTION_RANGE_2,
        // lockOnArrowOffset 4000.
        actor.scale = Vec3::splat(0.005);
        actor.gravity = -0.5;
        actor.min_velocity_y = -4.0;
        actor.target_mode = 2;
        actor.target_arrow_offset = 4000.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 25): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let (skeleton, fly) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, "gKeeseSkel").map_err(|e| log::error!("En_Firefly: {e:#}")).ok();
                let f = a.animation(OBJECT, "gKeeseFlyAnim").map_err(|e| log::error!("En_Firefly: {e:#}")).ok();
                (s, f)
            }
            None => (None, None),
        };
        // SkelAnime_Init(&gKeeseSkel, &gKeeseFlyAnim): KEESE_LIMB_MAX entries.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(27);
        let skel = SkelAnimeStd::init_flex(limbs, fly.clone());
        let elements = jnt_sph_elements();
        let collider = ColliderJntSph::new(&JNT_SPH_INIT, &elements);
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let mut f = EnFirefly {
            actor,
            body_parts_pos: [Vec3::ZERO; EN_FIREFLY_BODY_PART_MAX],
            skel,
            skeleton,
            fly,
            action: Action::Idle,
            effects_elemental_type: EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE,
            body_elemental_type: EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL,
            timer: 0,
            target_pitch: 0,
            home_y: 0.0,
            collider,
            draw_xlu: false,
        };
        if f.actor.params as u16 & 0x8000 != 0 {
            f.actor.flags |= ACTOR_FLAG_REACT_TO_LENS;
            f.draw_xlu = true;
            f.actor.params &= 0x7FFF;
        }
        f.body_elemental_type = if f.actor.params <= EN_FIREFLY_TYPE_FIRE_CAN_PERCH { EN_FIREFLY_BODY_ELEMENTAL_TYPE_FIRE } else { EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL };
        if f.body_elemental_type != EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL {
            f.action = Action::Idle;
            f.timer = play.rand.s16_offset(20, 60);
            f.actor.shape_rot.x = 0x1554;
            f.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE;
            f.actor.navi_enemy_id = NAVI_ENEMY_FIRE_KEESE;
            f.home_y = f.actor.home_pos.y;
        } else {
            f.action = if f.actor.params == EN_FIREFLY_TYPE_NORMAL_PERCHED { Action::Perched } else { Action::Idle };
            if f.actor.params == EN_FIREFLY_TYPE_ICE {
                f.collider.elements[0].info.at_dmg_info.hit_special_effect = HIT_SPECIAL_EFFECT_ICE;
                f.actor.navi_enemy_id = NAVI_ENEMY_ICE_KEESE;
            } else {
                f.collider.elements[0].info.at_dmg_info.hit_special_effect = HIT_SPECIAL_EFFECT_NONE;
                f.actor.navi_enemy_id = NAVI_ENEMY_KEESE;
            }
            f.home_y = f.actor.home_pos.y + 100.0;
            f.effects_elemental_type = if f.actor.params == EN_FIREFLY_TYPE_ICE { EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_ICE } else { EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE };
        }
        f.collider.elements[0].dim.world_sphere.radius = f.collider.elements[0].dim.model_sphere.radius;
        Box::new(f)
    }

    /// `EnFirefly_SetElementNormal`: a fire one's fire out (types 0 and 1 become 2 and 3).
    fn set_element_normal(&mut self) {
        self.actor.params += 2;
        self.collider.elements[0].info.at_dmg_info.hit_special_effect = HIT_SPECIAL_EFFECT_NONE;
        self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE;
        self.body_elemental_type = EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL;
        self.actor.navi_enemy_id = NAVI_ENEMY_KEESE;
    }

    /// `EnFirefly_SetElementFire`: alight (an ice one becomes type 0, the normal ones 0 and 1).
    fn set_element_fire(&mut self) {
        if self.actor.params == EN_FIREFLY_TYPE_ICE {
            self.actor.params = EN_FIREFLY_TYPE_FIRE;
        } else {
            self.actor.params -= 2;
        }
        self.collider.elements[0].info.at_dmg_info.hit_special_effect = HIT_SPECIAL_EFFECT_FIRE;
        self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE;
        self.body_elemental_type = EN_FIREFLY_BODY_ELEMENTAL_TYPE_FIRE;
        self.actor.navi_enemy_id = NAVI_ENEMY_FIRE_KEESE;
    }

    /// `EnFirefly_SetupIdle`: 70 to 170 frames at 1.5 to 3, turning homewards, climbing or
    /// diving towards `homeY`.
    fn setup_idle(&mut self, play: &mut PlayState) {
        self.timer = play.rand.s16_offset(70, 100);
        self.actor.speed_xz = (play.rand.zero_one() * 1.5) + 1.5;
        scaled_step_to_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0x300);
        let target_pitch: i32 = if self.home_y < self.actor.world_pos.y { 0xC00 } else { -0xC00 };
        self.target_pitch = (target_pitch + 0x1554) as i16;
        self.skel.play_speed = 1.0;
        self.action = Action::Idle;
    }

    /// `EnFirefly_SetupDie`: 40 frames falling, red.
    fn setup_die(&mut self, play: &mut PlayState) {
        self.timer = 40;
        self.actor.velocity.y = 0.0;
        if let Some(a) = self.fly.clone() {
            self.skel.change(a, 0.5, 0.0, 0.0, ANIMMODE_LOOP_INTERP, -3.0);
        }
        audio_play_actor_sfx2(play, NA_SE_EN_FFLY_DEAD);
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 40);
        self.action = Action::Die;
    }

    /// `EnFirefly_SetupDisappear`.
    fn setup_disappear(&mut self) {
        self.timer = 15;
        self.action = Action::Disappear;
        self.actor.speed_xz = 0.0;
    }

    /// `EnFirefly_SetupStay`.
    fn setup_stay(&mut self) {
        self.actor.world_rot.x = 0x7000;
        self.timer = 18;
        self.action = Action::Stay;
        self.skel.play_speed = 1.0;
        self.actor.speed_xz = 2.5;
    }

    /// `EnFirefly_SetupAttack`: 70 to 170 frames, pitching towards Link's height.
    fn setup_attack(&mut self, play: &mut PlayState) {
        self.timer = play.rand.s16_offset(70, 100);
        self.skel.play_speed = 1.0;
        let target_pitch: i32 = if self.actor.y_dist_to_player > 0.0 { -0xC00 } else { 0xC00 };
        self.target_pitch = (target_pitch + 0x1554) as i16;
        self.action = Action::Attack;
    }

    /// `EnFirefly_SetupFlyTowardsHome`: at most 150 frames.
    fn setup_fly_towards_home(&mut self) {
        self.timer = 150;
        self.target_pitch = 0x954;
        self.action = Action::FlyTowardsHome;
        self.skel.play_speed = 1.0;
    }

    /// `EnFirefly_SetupStunned`: 80 frames, blue, flapping fast, its fire or frost out.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.timer = 80;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 80);
        self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE;
        self.actor.velocity.y = 0.0;
        self.skel.play_speed = 3.0;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
    }

    /// `&this->actor` for an effect's spawn.
    fn ss_actor(&self, play: &PlayState) -> Option<oot_game::effect::SsActor> {
        let r = self.actor.shape_rot;
        play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] })
    }

    /// `EnFirefly_SetupDieFrozen`: frozen blue, ice on each of the cube's eight corners 7 out
    /// (`EffectSsEnIce_SpawnFlyingVec3f`), falling.
    fn setup_die_frozen(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE;
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 255);
        audio_play_actor_sfx2(play, NA_SE_EN_FFLY_DEAD);
        let me = self.ss_actor(play);
        for i in 0..8 {
            let p = self.actor.world_pos;
            let eff_pos = Vec3::new(p.x + if i & 1 != 0 { 7.0 } else { -7.0 }, p.y + if i & 2 != 0 { 7.0 } else { -7.0 }, p.z + if i & 4 != 0 { 7.0 } else { -7.0 });
            let scale = (play.rand.zero_one() * 0.15) + 0.85;
            play.effect_ss_en_ice_spawn_flying_vec3f(me, eff_pos, [150, 150, 150, 250], [235, 245, 255], scale);
        }
        self.action = Action::DieFrozen;
    }

    /// `EnFirefly_SetupPerched`.
    fn setup_perched(&mut self) {
        self.timer = 1;
        self.action = Action::Perched;
        self.actor.speed_xz = 0.0;
    }

    /// `EnFirefly_SetupAttackFromPerched`: 50 frames at 3, turned to Link, flapping fast.
    fn setup_attack_from_perched(&mut self) {
        self.skel.play_speed = 3.0;
        self.actor.shape_rot.x = 0x1554;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.timer = 50;
        self.actor.speed_xz = 3.0;
        self.action = Action::AttackFromPerched;
    }

    /// `EnFirefly_ApproachPerchSpot`: a perching one with Link over 300 from its perch flies
    /// back to it (slowing within 20), perching within 5. True while it does.
    fn approach_perch_spot(&mut self, play: &PlayState) -> bool {
        if self.actor.params != EN_FIREFLY_TYPE_NORMAL_PERCHED {
            return false;
        }
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        if dist_xz(player_pos, self.actor.home_pos) > 300.0 {
            let home_dist = self.actor.world_pos.distance(self.actor.home_pos);
            if home_dist < 5.0 {
                self.setup_perched();
            } else {
                let speed_fac = home_dist * 0.05;
                if speed_fac < 1.0 {
                    self.actor.speed_xz *= speed_fac;
                }
                scaled_step_to_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0x300);
                scaled_step_to_s(&mut self.actor.shape_rot.x, vec3f_pitch(self.actor.world_pos, self.actor.home_pos).wrapping_add(0x1554), 0x100);
            }
            true
        } else {
            false
        }
    }

    /// `EnFirefly_ApproachLitTorch`: the nearest lit torch (an `Obj_Syokudai` with `litTimer`)
    /// draws it to its flame, 67 above it, catching fire within 15. True while there's one.
    ///
    /// `Obj_Syokudai` isn't ported: its placeholder has no `litTimer`, so no torch is lit.
    fn approach_lit_torch(&mut self, play: &PlayState) -> bool {
        let mut closest: Option<Vec3> = None;
        let mut closest_dist = 35000.0;
        for &h in play.actors.category(ACTORCAT_PROP) {
            let Some(a) = play.actors.actor(h) else { continue };
            if a.id == ACTOR_OBJ_SYOKUDAI && obj_syokudai_lit_timer(play, h) != 0 {
                let dist = self.actor.world_pos.distance(a.world_pos);
                if dist < closest_dist {
                    closest_dist = dist;
                    closest = Some(a.world_pos);
                }
            }
        }
        let Some(torch) = closest else { return false };
        let flame = Vec3::new(torch.x, torch.y + 52.0 + 15.0, torch.z);
        if self.actor.world_pos.distance(flame) < 15.0 {
            self.set_element_fire();
        } else {
            scaled_step_to_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.world_pos, torch), 0x300);
            scaled_step_to_s(&mut self.actor.shape_rot.x, vec3f_pitch(self.actor.world_pos, flame).wrapping_add(0x1554), 0x100);
        }
        true
    }

    /// `EnFirefly_Idle`: flying about; with each flap's start a chance (½) to turn homewards or
    /// (⅗ of the rest) at random, and its pitch picked (up from near the floor, down above
    /// `homeY`, else at random); off the floor, ceiling and walls; with its timer out and Link
    /// within 200 (no Skull Mask), it attacks.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        let anim_looped = self.skel.on_frame(0.0);
        self.actor.speed_xz = (play.rand.zero_one() * 1.5) + 1.5;
        if self.body_elemental_type != EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL || self.actor.params == EN_FIREFLY_TYPE_ICE || (!self.approach_perch_spot(play) && !self.approach_lit_torch(play)) {
            if anim_looped {
                let f = play.rand.zero_one();
                if f < 0.5 {
                    scaled_step_to_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0x300);
                } else if f < 0.8 {
                    let yaw_speed = play.rand.centered_float(1536.0);
                    // TRUNCF_BINANG(shape.rot.y + yawSpeed).
                    self.actor.shape_rot.y = (self.actor.shape_rot.y as f32 + yaw_speed) as i32 as i16;
                }
                if self.actor.world_pos.y < self.actor.floor_height + 20.0 {
                    self.target_pitch = 0x954;
                } else if self.home_y < self.actor.world_pos.y {
                    self.target_pitch = 0x2154;
                } else if play.rand.zero_one() > 0.35 {
                    self.target_pitch = 0x954;
                } else {
                    self.target_pitch = 0x2154;
                }
            } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.target_pitch = 0x954;
            } else if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 || self.home_y < self.actor.world_pos.y {
                self.target_pitch = 0x2154;
            }
            scaled_step_to_s(&mut self.actor.shape_rot.x, self.target_pitch, 0x100);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.wall_yaw, 2, 0xC00, 0x300);
        }
        // Player_GetMask(play) != PLAYER_MASK_SKULL: masks aren't ported.
        if self.timer == 0 && self.actor.xz_dist_to_player < 200.0 {
            self.setup_attack(play);
        }
    }

    /// `EnFirefly_Die`: stopping its flap at frame 6, still red, slowing, pitching down and
    /// spinning (unless on an arrow); on the ground or after 40 frames, it disappears.
    fn die(&mut self) {
        if self.skel.on_frame(6.0) {
            self.skel.play_speed = 0.0;
        }
        self.actor.color_filter_timer = 40;
        self.skel.update();
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW != 0 {
            self.actor.color_filter_timer = 40;
        } else {
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0x6800, 0x200);
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x300);
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.timer == 0 {
                self.setup_disappear();
            }
        }
    }

    /// `EnFirefly_Disappear`: shrinking for 15 frames, then its drop (table 14) and gone.
    fn disappear(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        step_to_f(&mut self.actor.scale.x, 0.0, 0.00034);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        if self.timer == 0 {
            // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_14, false).
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_14 * 16);
            self.actor.kill();
        }
    }

    /// `EnFirefly_Attack`: up to 4 a frame; facing Link, it holds its flap at frame 4 and homes
    /// on him (20 up); else it flaps faster, turning to him from beyond 80; off walls; its
    /// timer out (or a Skull Mask), home.
    fn attack(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        step_to_f(&mut self.actor.speed_xz, 4.0, 0.5);
        // Actor_IsFacingPlayer(&this->actor, 0x2800).
        let facing = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() < 0x2800;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.wall_yaw, 2, 0xC00, 0x300);
            scaled_step_to_s(&mut self.actor.shape_rot.x, self.target_pitch, 0x100);
        } else if facing {
            if self.skel.on_frame(4.0) {
                self.skel.play_speed = 0.0;
                self.skel.cur_frame = 4.0;
            }
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xC00, 0x300);
            let target = Vec3::new(player_pos.x, player_pos.y + 20.0, player_pos.z);
            smooth_step_to_s(&mut self.actor.shape_rot.x, vec3f_pitch(self.actor.world_pos, target).wrapping_add(0x1554), 2, 0x400, 0x100);
        } else {
            self.skel.play_speed = 1.5;
            if self.actor.xz_dist_to_player > 80.0 {
                smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0xC00, 0x300);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.target_pitch = 0x954;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 || self.home_y < self.actor.world_pos.y {
                self.target_pitch = 0x2154;
            } else {
                self.target_pitch = 0x954;
            }
            scaled_step_to_s(&mut self.actor.shape_rot.x, self.target_pitch, 0x100);
        }
        if self.timer == 0 {
            self.setup_fly_towards_home();
        }
    }

    /// `EnFirefly_Stay`: levelling out and slowing to a stop, then 18 frames' hover; then home.
    fn stay(&mut self) {
        self.skel.update();
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x100);
        step_to_f(&mut self.actor.velocity.y, 0.0, 0.4);
        if step_to_f(&mut self.actor.speed_xz, 0.0, 0.15) {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                self.setup_fly_towards_home();
            }
        }
    }

    /// `EnFirefly_FlyTowardsHome`: at 3 back to home at `homeY`; within 10 up and 20 across of it
    /// (or after 150 frames), idle.
    fn fly_towards_home(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if ((self.actor.world_pos.y - self.home_y).abs() < 10.0 && dist_xz(self.actor.world_pos, self.actor.home_pos) < 20.0) || self.timer == 0 {
            self.setup_idle(play);
        } else {
            step_to_f(&mut self.actor.speed_xz, 3.0, 0.3);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.target_pitch = 0x954;
            } else if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 || self.home_y < self.actor.world_pos.y {
                self.target_pitch = 0x2154;
            } else {
                self.target_pitch = 0x954;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.wall_yaw, 2, 0xC00, 0x300);
            } else {
                scaled_step_to_s(&mut self.actor.shape_rot.y, vec3f_yaw(self.actor.world_pos, self.actor.home_pos), 0x300);
            }
            scaled_step_to_s(&mut self.actor.shape_rot.x, self.target_pitch, 0x100);
        }
    }

    /// `EnFirefly_Stunned`: slowing to a stop (falling), for 80 frames; then its fire or frost
    /// back, idle.
    fn stunned(&mut self, play: &mut PlayState) {
        self.skel.update();
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0x1554, 0x100);
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            if self.body_elemental_type != EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL {
                self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE;
            } else if self.actor.params == EN_FIREFLY_TYPE_ICE {
                self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_ICE;
            }
            self.setup_idle(play);
        }
    }

    /// `EnFirefly_DieFrozen`: frozen until on the ground (`BGCHECKFLAG_GROUND`, NTSC 1.1 and
    /// later) or over no floor; then it disappears.
    fn die_frozen(&mut self) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.actor.floor_height == eng_collision::bgcheck::BGCHECK_Y_MIN {
            self.actor.color_filter_timer = 0;
            self.setup_disappear();
        } else {
            self.actor.color_filter_timer = 255;
        }
    }

    /// `EnFirefly_Perched`: levelling; a flap now and then (2 in 100 a frame; one flap to frame
    /// 6); Link within 120, it attacks.
    fn perched(&mut self, play: &mut PlayState) {
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x100);
        if self.timer != 0 {
            self.skel.update();
            if self.skel.on_frame(6.0) {
                self.timer -= 1;
            }
        } else if play.rand.zero_one() < 0.02 {
            self.timer = 1;
        }
        if self.actor.xz_dist_to_player < 120.0 {
            self.setup_attack_from_perched();
        }
    }

    /// `EnFirefly_AttackFromPerched`: for its first 10 frames at Link (20 up), then climbing away
    /// (pitch -0xAAC); after 50, idle.
    fn attack_from_perched(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer < 40 {
            scaled_step_to_s(&mut self.actor.shape_rot.x, -0xAAC, 0x100);
        } else {
            let target = Vec3::new(player_pos.x, player_pos.y + 20.0, player_pos.z);
            scaled_step_to_s(&mut self.actor.shape_rot.x, vec3f_pitch(self.actor.world_pos, target).wrapping_add(0x1554), 0x100);
            scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0x300);
        }
        if self.timer == 0 {
            self.setup_idle(play);
        }
    }

    /// `EnFirefly_IceMelt`: a flame on each body part (`EffectSsEnFire_SpawnVec3f`, 40).
    fn ice_melt(&mut self, play: &mut PlayState) {
        let me = self.ss_actor(play);
        for body_part in 0..EN_FIREFLY_BODY_PART_MAX {
            play.effect_ss_en_fire_spawn_vec3f(me, self.actor.world_pos, 40, 0, 0, body_part as i16);
        }
        self.effects_elemental_type = EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_NONE;
    }

    /// `EnFirefly_CheckCollide`: a hit (`AC_HIT`), its drop flag set; one with a reaction or
    /// damage: the finishing blow at 0 health (no more targeting); Din's Fire melts an ice one to
    /// death or sets a normal one alight; ice freezes it (kills an ice one); a Deku Nut stuns it;
    /// anything else kills it (a fire arrow melting an ice one first).
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let elem = self.collider.elements[0].info;
        self.actor.set_drop_flag(&elem, true);
        let reaction = self.actor.col_chk_info.damage_reaction;
        if reaction == EN_FIREFLY_DMG_REACT_NONE && self.actor.col_chk_info.damage == 0 {
            return;
        }
        if self.actor.apply_damage() == 0 {
            enemy_start_finishing_blow(play, &self.actor);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        }
        if reaction == EN_FIREFLY_DMG_REACT_DINS_FIRE {
            if self.actor.params == EN_FIREFLY_TYPE_ICE {
                self.actor.col_chk_info.health = 0;
                enemy_start_finishing_blow(play, &self.actor);
                self.ice_melt(play);
                self.setup_die(play);
            } else if self.body_elemental_type == EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL {
                self.set_element_fire();
                if self.action == Action::Perched {
                    self.setup_idle(play);
                }
            }
        } else if reaction == EN_FIREFLY_DMG_REACT_ICE {
            if self.actor.params == EN_FIREFLY_TYPE_ICE {
                self.setup_die(play);
            } else {
                self.setup_die_frozen(play);
            }
        } else if reaction == EN_FIREFLY_DMG_REACT_STUN {
            if self.action != Action::Stunned {
                self.setup_stunned(play);
            }
        } else {
            if reaction == EN_FIREFLY_DMG_REACT_FIRE_ARROW && self.actor.params == EN_FIREFLY_TYPE_ICE {
                self.ice_melt(play);
            }
            self.setup_die(play);
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

/// `EnFirefly_OverrideLimbDraw`'s change to the pose: the root 2300 higher (unless drawn for
/// the Lens of Truth without it, when no limb is drawn at all). Returns each limb's matrix in
/// the model's space.
fn keese_pose(skeleton: &Skeleton, joints: &[[i16; 3]], xlu_hidden: bool) -> Vec<Mat4> {
    skeleton.pose_override(joints, |limb, pos, _rot| {
        if !xlu_hidden && limb == KEESE_LIMB_ROOT_ROOT {
            pos.y += 2300.0;
        }
        Mat4::IDENTITY
    })
}

/// `((ObjSyokudai*)iter)->litTimer` for the torch `h`: `Obj_Syokudai` isn't ported (GAME-05
/// milestone 4), so no torch is lit.
fn obj_syokudai_lit_timer(_play: &PlayState, _h: oot_game::actor_ctx::ActorHandle) -> i16 {
    0
}

impl ActorImpl for EnFirefly {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnFirefly_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            audio_play_actor_sfx2(play, NA_SE_EN_FFLY_ATTACK);
            if self.body_elemental_type != EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL {
                self.set_element_normal();
            }
            if self.action != Action::AttackFromPerched {
                self.setup_stay();
            }
        }
        self.check_collide(play);
        match self.action {
            Action::Idle => self.idle(play),
            Action::Die => self.die(),
            Action::Disappear => self.disappear(play),
            Action::Attack => self.attack(play),
            Action::Stay => self.stay(),
            Action::FlyTowardsHome => self.fly_towards_home(play),
            Action::Stunned => self.stunned(play),
            Action::DieFrozen => self.die_frozen(),
            Action::Perched => self.perched(play),
            Action::AttackFromPerched => self.attack_from_perched(play),
        }
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW == 0 {
            if self.actor.col_chk_info.health == 0 || self.action == Action::Stunned {
                self.actor.move_forward();
            } else {
                if self.action != Action::Stay {
                    self.actor.world_rot.x = (0x1554 - self.actor.shape_rot.x as i32) as i16;
                }
                self.actor.move_xyz();
            }
        }
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 15.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        let p = self.actor.world_pos;
        self.collider.elements[0].dim.world_sphere.center = [p.x as i16, (p.y + 10.0) as i16, p.z as i16];
        if self.action == Action::Attack || self.action == Action::AttackFromPerched {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.actor.col_chk_info.health != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            if self.skel.on_frame(5.0) {
                audio_play_actor_sfx2(play, NA_SE_EN_FFLY_FLY);
            }
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        let r = self.actor.shape_rot;
        self.actor.focus_pos.x = self.actor.world_pos.x + (10.0 * sin_s(r.x) * sin_s(r.y));
        self.actor.focus_pos.y = self.actor.world_pos.y + (10.0 * cos_s(r.x));
        self.actor.focus_pos.z = self.actor.world_pos.z + (10.0 * sin_s(r.x) * cos_s(r.y));
    }

    /// `EnFirefly_PostLimbDraw`'s effects on the actor, in the draw's limb order: each wing's end
    /// trails flames or frost with fire or ice (`func_8002843C`, 250 big, 3 frames; while
    /// disappearing, spiralling in, 10 frames), and the wings' ends and the body record their
    /// positions, 5 down.
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let xlu_hidden = self.draw_xlu; // && !play->actorCtx.lensActive (never set).
        let bones = keese_pose(&skeleton, &self.skel.joint_table, xlu_hidden);
        let model = self.actor_mtx().to_mat4();
        // sFireEffPrimColor, sFireEffEnvColor, sIceEffPrimColor, sIceEffEnvColor, sEffVel, sEffAccel.
        const FIRE_PRIM: [u8; 4] = [255, 255, 100, 255];
        const FIRE_ENV: [u8; 4] = [255, 50, 0, 0];
        const ICE_PRIM: [u8; 4] = [100, 200, 255, 255];
        const ICE_ENV: [u8; 4] = [0, 0, 255, 0];
        let eff_vel = Vec3::new(0.0, 0.5, 0.0);
        let eff_accel = Vec3::new(0.0, 0.5, 0.0);
        for &l in &skeleton.draw_order {
            let limb = l as usize + 1;
            let m = model * bones[l as usize];
            let is_eyes = self.body_elemental_type == EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL && limb == KEESE_LIMB_HEAD;
            if !is_eyes
                && (self.effects_elemental_type == EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE || self.effects_elemental_type == EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_ICE)
                && (limb == KEESE_LIMB_LEFT_WING_END || limb == KEESE_LIMB_RIGHT_WING_END_ROOT)
            {
                let (eff_pos, scale_step, life);
                if self.action != Action::Disappear {
                    let t = m.w_axis;
                    let x = (play.rand.zero_one() * 5.0) + t.x;
                    let y = (play.rand.zero_one() * 5.0) + t.y;
                    let z = (play.rand.zero_one() * 5.0) + t.z;
                    eff_pos = Vec3::new(x, y, z);
                    scale_step = -40;
                    life = 3;
                } else {
                    let t = self.timer;
                    let a = (t as i32 * 0x238C) as i16;
                    let (dx, dz) = (sin_s(a) * t as f32, cos_s(a) * t as f32);
                    let (x, z) = if limb == 15 { (self.actor.world_pos.x + dx, self.actor.world_pos.z + dz) } else { (self.actor.world_pos.x - dx, self.actor.world_pos.z - dz) };
                    eff_pos = Vec3::new(x, self.actor.world_pos.y + ((15 - t) as f32 * 1.5), z);
                    scale_step = -5;
                    life = 10;
                }
                let (prim, env) = if self.effects_elemental_type == EN_FIREFLY_EFFECTS_ELEMENTAL_TYPE_FIRE { (FIRE_PRIM, FIRE_ENV) } else { (ICE_PRIM, ICE_ENV) };
                play.with_ss(|ss| ss.func_8002843c(eff_pos, eff_vel, eff_accel, prim, env, 250, scale_step, life));
            }
            let part = match limb {
                KEESE_LIMB_LEFT_WING_END => Some(EN_FIREFLY_BODY_PART_LEFT_WING),
                KEESE_LIMB_RIGHT_WING_END_ROOT => Some(EN_FIREFLY_BODY_PART_RIGHT_WING),
                KEESE_LIMB_BODY => Some(EN_FIREFLY_BODY_PART_BODY),
                _ => None,
            };
            if let Some(i) = part {
                // Matrix_MultVec3f(&D_80A14FC8, bodyPartPos); bodyPartPos->y -= 5.
                let p = m.transform_point3(Vec3::ZERO);
                self.body_parts_pos[i] = Vec3::new(p.x, p.y - 5.0, p.z);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.body_elemental_type as u32, self.draw_xlu as u32];
        rs
    }

    /// `EnFirefly_DrawOpa` (`EnFirefly_DrawXlu` for a Lens of Truth one, which draws nothing
    /// without the lens): the skeleton, its root raised, and on a normal one's head the eyes
    /// (`gKeeseEyesDL`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [body, xlu]) = (&rs.joints, rs.switches.as_slice()) else { return };
        let Some(skeleton) = &self.skeleton else { return };
        if *xlu != 0 {
            // Every limb's dList is set to NULL without the lens.
            return;
        }
        let model = oot_game::play::actor_draw_matrix(rs);
        let bones = keese_pose(skeleton, &joints.rot, false);
        if *body as u8 == EN_FIREFLY_BODY_ELEMENTAL_TYPE_NORMAL {
            let head = model * bones[KEESE_LIMB_HEAD - 1];
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gKeeseEyesDL")), head));
        }
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, "gKeeseSkel")), transform: model, bones, params: Default::default() });
    }

    /// `firePos` (`EffectSsEnFire`): the body parts' positions at 0x14C.
    fn effect_fire_pos(&self, i: usize, _vec3s: bool) -> Vec3 {
        self.body_parts_pos.get(i).copied().unwrap_or(Vec3::ZERO)
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
