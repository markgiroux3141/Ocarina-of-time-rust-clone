//! `En_Skb` (`ovl_En_Skb/z_en_skb.c`): the Stalchild, the skeleton that rises out of Hyrule
//! Field by night (GAME-06 milestone 4).
//!
//! Params: its size (`(params × 0.1 + 1) × 0.01`; its spheres 10 + params and 20 + 2 params).
//! `En_Encount1` spawns them around Link; each one's death counts down its spawner.
//!
//! - **Rising** (`EnSkb_RiseFromGround`): from 8,000 under (its `shape.yOffset`), turning to Link
//!   for its first 4 frames, debris and dust every other frame; then it decides
//!   (`EnSkb_DecideNextAction`): by day it sinks back (`EnSkb_Despawn`); facing Link within 60
//!   (plus 6 a size) it attacks, else it walks at him (`EnSkb_WalkForward`, 160 times its scale,
//!   its steps' sound at frames 8 and 15; 800 from home or by day it sinks).
//! - **Attacking** (`EnSkb_Attack`): its swipe's sphere on from frame 3 to 6; bounced off the
//!   shield, it recoils (`EnSkb_Recoil`).
//! - **Hit** (`EnSkb_CheckDamage`, from attacking on): a Deku Nut stuns it (blue, 120);
//!   other damage flashes it red (fire: five flames) and knocks it back (`EnSkb_TakeDamage`);
//!   a spin or a horizontal slash of the Kokiri Sword's that doesn't kill it knocks its head off
//!   (`BodyBreak` of the head and jaw: then it wanders headless). At no health it dies
//!   (`EnSkb_Death`): it breaks into its 17 limbs (`En_Part`s), drops from table 1 (or rupees as a
//!   bigger one), and goes. Deep water kills it too.
//!
//! The whole overlay is ported. Its limb draws are one skeleton (its head's pulsing env colour
//! for all its limbs: the C's sets it before the head, and no other limb reads it), with and
//! without the head and jaw. The circle shadow isn't drawn (`shadowScale` kept).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{smooth_step_to_f, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, math_cos_f, math_sin_f};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, ANIMMODE_ONCE_INTERP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_part::{BODYBREAK_OBJECT_SLOT_DEFAULT, BodyBreak};

/// `ACTOR_EN_SKB` (`actor_table.h`: 0x01B0).
pub const ACTOR_EN_SKB: i16 = 0x01B0;
pub const OBJECT: &str = "object_skb";
const SKEL: &str = "gStalchildSkel";

/// `En_Skb_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_SKB, name: "En_Skb", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `NAVI_ENEMY_STALCHILD` (`actor.h`: 0x55).
const NAVI_ENEMY_STALCHILD: u8 = 0x55;
/// `COLLECTIBLE_DROP_TABLE_1`; `ITEM00_RUPEE_BLUE`, `ITEM00_RUPEE_RED` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_1: i16 = 1;
const ITEM00_RUPEE_BLUE: i16 = 0x01;
const ITEM00_RUPEE_RED: i16 = 0x02;
/// `PLAYER_MWA_RIGHT_SLASH_1H` .. `PLAYER_MWA_LEFT_COMBO_2H`, `PLAYER_MWA_BACKSLASH_RIGHT`,
/// `_LEFT` (`player.h`).
const PLAYER_MWA_RIGHT_SLASH_1H: usize = 4;
const PLAYER_MWA_LEFT_COMBO_2H: usize = 11;
const PLAYER_MWA_BACKSLASH_RIGHT: usize = 20;
const PLAYER_MWA_BACKSLASH_LEFT: usize = 21;

/// `StalchildBehavior`.
pub const SKB_BEHAVIOR_BURIED: u8 = 0;
pub const SKB_BEHAVIOR_DYING: u8 = 1;
pub const SKB_BEHAVIOR_DAMAGED: u8 = 2;
pub const SKB_BEHAVIOR_ATTACKING: u8 = 3;
pub const SKB_BEHAVIOR_WALKING: u8 = 4;
pub const SKB_BEHAVIOR_RECOILING: u8 = 5;
pub const SKB_BEHAVIOR_STUNNED: u8 = 6;

/// The limbs' lists, by limb (1-based; `gStalchildSkel`'s table).
pub const LIMB_DLISTS: [Option<&str>; 20] = [
    None,
    None,
    Some("gStalchildWaistDL"),
    None,
    Some("gStalchildRightFemurDL"),
    Some("gStalchildRightShinDL"),
    Some("gStalchildRightFootDL"),
    Some("gStalchildLeftFemurDL"),
    Some("gStalchildLeftShinDL"),
    Some("gStalchildLeftFootDL"),
    Some("gStalchildRibCageDL"),
    Some("gStalchildHeadDL"),
    Some("gStalchildJawDL"),
    Some("gStalchildRightHumerusDL"),
    Some("gStalchildRightForearmDL"),
    Some("gStalchildRightHandDL"),
    Some("gStalchildLeftHumerusDL"),
    Some("gStalchildLeftForearmDL"),
    Some("gStalchildLeftHandDL"),
    Some("gStalchildSpineDL"),
];

/// `sJntSphElementsInit`: the swipe on the right hand (limb 15, 4 damage), the body on the root
/// (hit by all but the shield and the mirror's ray, the hookshot's).
fn jnt_sph_elements() -> [ColliderJntSphElementInit; 2] {
    [
        ColliderJntSphElementInit {
            info: ColliderElementInit {
                elem_material: ELEM_MATERIAL_UNK0,
                at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
                ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
                at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
                ac_elem_flags: ACELEM_NONE,
                oc_elem_flags: OCELEM_NONE,
            },
            limb: 15,
            model_sphere: Sphere16 { center: [0, 0, 0], radius: 10 },
            scale: 100,
        },
        ColliderJntSphElementInit {
            info: ColliderElementInit {
                elem_material: ELEM_MATERIAL_UNK0,
                at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
                ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
                at_elem_flags: ATELEM_NONE,
                ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
                oc_elem_flags: OCELEM_ON,
            },
            limb: 1,
            model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
            scale: 100,
        },
    ]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0xF), // Deku stick
        dmg_entry(1, 0xF), // Slingshot
        dmg_entry(2, 0xF), // Explosive
        dmg_entry(0, 0x1), // Boomerang
        dmg_entry(2, 0xF), // Normal arrow
        dmg_entry(2, 0xF), // Hammer swing
        dmg_entry(0, 0x1), // Hookshot
        dmg_entry(1, 0xE), // Kokiri sword
        dmg_entry(2, 0xF), // Master sword
        dmg_entry(4, 0xF), // Giant's Knife
        dmg_entry(4, 0x7), // Fire arrow
        dmg_entry(2, 0xF), // Ice arrow
        dmg_entry(2, 0xF), // Light arrow
        dmg_entry(2, 0xF), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(4, 0x7), // Fire magic
        dmg_entry(0, 0x6), // Ice magic
        dmg_entry(3, 0xD), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0xD), // Kokiri spin
        dmg_entry(4, 0xF), // Giant spin
        dmg_entry(2, 0xF), // Master spin
        dmg_entry(2, 0xF), // Kokiri jump
        dmg_entry(8, 0xF), // Giant jump
        dmg_entry(4, 0xF), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(4, 0xF), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// The skeleton's bakes: whole, and headless (limbs 11 and 12 drawn as nothing).
const BAKE: &str = "En_Skb/skeleton";
const BAKE_HEADLESS: &str = "En_Skb/headless";
const SEG_COLOR: u8 = 0x0B;

pub fn bakes() -> Vec<MeshBake> {
    let bake = |name: &str, limbs: Vec<LimbOverride>| MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        // gDPPipeSync, gDPSetEnvColor(color, color, color, 255) before the head.
        segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_COLOR],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs },
    };
    let none = |limb: u8| LimbOverride { limb, file: OBJECT.into(), symbol: String::new() };
    vec![bake(BAKE, Vec::new()), bake(BAKE_HEADLESS, vec![none(10), none(11)])]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    RiseFromGround,
    Despawn,
    WalkForward,
    Attack,
    Recoil,
    Stunned,
    TakeDamage,
    Death,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::RiseFromGround => "EnSkb_RiseFromGround",
            Action::Despawn => "EnSkb_Despawn",
            Action::WalkForward => "EnSkb_WalkForward",
            Action::Attack => "EnSkb_Attack",
            Action::Recoil => "EnSkb_Recoil",
            Action::Stunned => "EnSkb_Stunned",
            Action::TakeDamage => "EnSkb_TakeDamage",
            Action::Death => "EnSkb_Death",
        }
    }
}

struct Anims {
    uncurling: Anim,
    walking: Anim,
    attacking: Anim,
    damaged: Anim,
    dying: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Skb: {e:#}")).ok();
        Some(Anims {
            uncurling: get("gStalchildUncurlingAnim")?,
            walking: get("gStalchildWalkingAnim")?,
            attacking: get("gStalchildAttackingAnim")?,
            damaged: get("gStalchildDamagedAnim")?,
            dying: get("gStalchildDyingAnim")?,
        })
    }
}

pub struct EnSkb {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action_state: u8,
    pub set_collider_at: bool,
    pub last_damage_reaction: u8,
    /// `breakFlags`: 1 the head breaking, 2 headless, 4 dying (breaking up), 8 broken up.
    pub break_flags: u8,
    pub action: Action,
    pub headless_yaw_offset: i16,
    pub body_break: BodyBreak,
    pub collider: ColliderJntSph,
    /// `shape.shadowScale` (the circle shadow isn't drawn).
    pub shadow_scale: f32,
}

impl EnSkb {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `EnSkb_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 2000, gravity -2.
        actor.target_arrow_offset = 2000.0;
        actor.gravity = -2.0;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.focus_pos = actor.world_pos;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.health = 2;
        actor.shape_y_offset = -8000.0;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Skb: {e:#}")).ok());
        let anims = Anims::load(play);
        let skel = SkelAnimeStd::init_flex(20, anims.as_ref().map(|a| a.uncurling.clone()));
        actor.navi_enemy_id = NAVI_ENEMY_STALCHILD;
        let elements = jnt_sph_elements();
        let mut collider = ColliderJntSph::new(&JNT_SPH_INIT, &elements);
        let p = actor.params;
        actor.scale = Vec3::splat(((p as f32 * 0.1) + 1.0) * 0.01);
        let r0 = (10 + p) as i16;
        let r1 = (20 + (p * 2)) as i16;
        collider.elements[0].dim.model_sphere.radius = r0;
        collider.elements[0].dim.world_sphere.radius = r0;
        collider.elements[1].dim.model_sphere.radius = r1;
        collider.elements[1].dim.world_sphere.radius = r1;
        actor.home_pos = actor.world_pos;
        actor.floor_height = actor.world_pos.y;
        let mut this = EnSkb {
            actor,
            skel,
            skeleton,
            anims,
            action_state: 0,
            set_collider_at: false,
            last_damage_reaction: 0,
            break_flags: 0,
            action: Action::RiseFromGround,
            headless_yaw_offset: 0,
            body_break: BodyBreak::default(),
            collider,
            shadow_scale: 0.0,
        };
        this.setup_rise_from_ground(play);
        Box::new(this)
    }

    /// `EnSkb_SpawnDebris`: a piece of earth (`EffectSsHahen`) 15 from its place at a random
    /// angle, and dust (`func_80033480`).
    fn spawn_debris(&self, play: &mut PlayState, spawn_pos: Vec3) {
        let mut vel = Vec3::new(0.0, 8.0, 0.0);
        let mut accel = Vec3::new(0.0, -1.5, 0.0);
        let spread = (play.rand.zero_one() - 0.5) * 6.28;
        let pos = Vec3::new(math_sin_f(spread) * 15.0 + spawn_pos.x, self.actor.floor_height, math_cos_f(spread) * 15.0 + spawn_pos.z);
        accel.x = play.rand.centered_float(1.0);
        accel.z = play.rand.centered_float(1.0);
        vel.y += (play.rand.zero_one() - 0.5) * 4.0;
        let scale = (play.rand.zero_one() * 5.0) + 12.0;
        play.with_ss(|s| s.hahen_spawn(pos, vel, accel, 2, (scale * 0.8) as i16, HAHEN_OBJECT_DEFAULT, 10, None));
        oot_game::actor_ctx::func_80033480(play, pos, 10.0, 1, 150, 0, 1);
    }

    /// `EnSkb_DecideNextAction`.
    fn decide_next_action(&mut self, play: &mut PlayState) {
        if play.save.is_day() {
            self.setup_despawn(play);
        } else if self.facing_player(0x11C7) && self.actor.xz_dist_to_player < 60.0 + (self.actor.params as f32 * 6.0) {
            self.setup_attack();
        } else {
            self.setup_walk_forward();
        }
    }

    /// `Actor_IsFacingPlayer`.
    fn facing_player(&self, max: i16) -> bool {
        (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs() < max as i32
    }

    /// `EnSkb_SetupRiseFromGround`.
    fn setup_rise_from_ground(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(|a| &a.uncurling) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
        self.action_state = SKB_BEHAVIOR_BURIED;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        audio_play_actor_sfx2(play, NA_SE_EN_RIVA_APPEAR);
        self.action = Action::RiseFromGround;
    }

    /// `EnSkb_RiseFromGround`.
    fn rise_from_ground(&mut self, play: &mut PlayState) {
        if self.skel.cur_frame < 4.0 {
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
        } else {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        }
        smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, 800.0, 0.0);
        smooth_step_to_f(&mut self.shadow_scale, 25.0, 1.0, 2.5, 0.0);
        if play.gameplay_frames & 1 != 0 {
            let pos = self.actor.world_pos;
            self.spawn_debris(play, pos);
        }
        if self.skel.update() && self.actor.shape_y_offset == 0.0 {
            self.decide_next_action(play);
        }
    }

    /// `EnSkb_SetupDespawn`: the uncurling backwards.
    fn setup_despawn(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(|a| &a.uncurling) {
            let last = a.last_frame();
            self.skel.change(a, -1.0, last, 0.0, ANIMMODE_ONCE, -4.0);
        }
        self.action_state = SKB_BEHAVIOR_BURIED;
        self.set_collider_at = false;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_AKINDONUTS_HIDE);
        self.action = Action::Despawn;
    }

    /// `EnSkb_Despawn`.
    fn despawn(&mut self, play: &mut PlayState) {
        if smooth_step_to_f(&mut self.actor.shape_y_offset, -8000.0, 1.0, 500.0, 0.0) != 0.0 && play.gameplay_frames & 1 != 0 {
            let pos = self.actor.world_pos;
            self.spawn_debris(play, pos);
        }
        smooth_step_to_f(&mut self.shadow_scale, 0.0, 1.0, 2.5, 0.0);
        if self.skel.update() {
            self.actor.kill();
        }
    }

    /// `EnSkb_SetupWalkForward`.
    fn setup_walk_forward(&mut self) {
        if let Some(a) = self.anim(|a| &a.walking) {
            let last = a.last_frame();
            self.skel.change(a, 0.96000004, 0.0, last, ANIMMODE_LOOP, -4.0);
        }
        self.action_state = SKB_BEHAVIOR_WALKING;
        self.headless_yaw_offset = 0;
        self.actor.speed_xz = self.actor.scale.y * 160.0;
        self.action = Action::WalkForward;
    }

    /// `EnSkb_WalkForward`.
    fn walk_forward(&mut self, play: &mut PlayState) {
        if self.break_flags != 0 && play.gameplay_frames & 0xF == 0 {
            self.headless_yaw_offset = play.rand.centered_float(50000.0) as i32 as i16;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player.wrapping_add(self.headless_yaw_offset), 1, 0x2EE, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let this_key_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let play_speed = self.skel.play_speed.abs();
        let prev_key_frame = (self.skel.cur_frame - play_speed) as i32;
        if this_key_frame != self.skel.cur_frame as i32 && ((prev_key_frame < 9 && (play_speed as i32 + this_key_frame) >= 8) || !(prev_key_frame >= 16 || (play_speed as i32 + this_key_frame) < 15)) {
            audio_play_actor_sfx2(play, NA_SE_EN_STALKID_WALK);
        }
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        let (dx, dz) = (player_pos.x - self.actor.home_pos.x, player_pos.z - self.actor.home_pos.z);
        if (dx * dx + dz * dz).sqrt() > 800.0 || play.save.is_day() {
            self.setup_despawn(play);
        } else if self.facing_player(0x11C7) && self.actor.xz_dist_to_player < 60.0 + (self.actor.params as f32 * 6.0) {
            self.setup_attack();
        }
    }

    /// `EnSkb_SetupAttack`.
    fn setup_attack(&mut self) {
        if let Some(a) = self.anim(|a| &a.attacking) {
            let last = a.last_frame();
            self.skel.change(a, 0.6, 0.0, last, ANIMMODE_ONCE_INTERP, 4.0);
        }
        self.collider.base.at_flags &= !AT_BOUNCED;
        self.action_state = SKB_BEHAVIOR_ATTACKING;
        self.actor.speed_xz = 0.0;
        self.action = Action::Attack;
    }

    /// `EnSkb_Attack`: its swipe from frame 3 to 6; bounced, it recoils.
    fn attack(&mut self, play: &mut PlayState) {
        let frame = self.skel.cur_frame as i32;
        if frame == 3 {
            audio_play_actor_sfx2(play, NA_SE_EN_STALKID_ATTACK);
            self.set_collider_at = true;
        } else if frame == 6 {
            self.set_collider_at = false;
        }
        if self.collider.base.at_flags & AT_BOUNCED != 0 {
            self.collider.base.at_flags &= !(AT_HIT | AT_BOUNCED);
            self.setup_recoil();
        } else if self.skel.update() {
            self.decide_next_action(play);
        }
    }

    /// `EnSkb_SetupRecoil`: the attack played back from the frame before.
    fn setup_recoil(&mut self) {
        if let Some(a) = self.anim(|a| &a.attacking) {
            let start = self.skel.cur_frame - 1.0;
            self.skel.change(a, -0.4, start, 0.0, ANIMMODE_ONCE_INTERP, 0.0);
        }
        self.collider.base.at_flags &= !AT_BOUNCED;
        self.action_state = SKB_BEHAVIOR_RECOILING;
        self.set_collider_at = false;
        self.action = Action::Recoil;
    }

    /// `EnSkb_Recoil`.
    fn recoil(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.decide_next_action(play);
        }
    }

    /// `EnSkb_SetupStunned`.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = 0.0;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.set_collider_at = false;
        self.action_state = SKB_BEHAVIOR_STUNNED;
        self.action = Action::Stunned;
    }

    /// `EnSkb_Stunned`: until its colour filter's over on the ground: dead, or deciding.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.speed_xz < 0.0 {
            self.actor.speed_xz += 0.05;
        }
        if self.actor.color_filter_timer == 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.col_chk_info.health == 0 {
                self.setup_death(play);
            } else {
                self.decide_next_action(play);
            }
        }
    }

    /// `EnSkb_SetupTakeDamage`: knocked back 4 (on the ground), facing Link.
    fn setup_take_damage(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(|a| &a.damaged) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = -4.0;
        }
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        audio_play_actor_sfx2(play, NA_SE_EN_STALKID_DAMAGE);
        self.action_state = SKB_BEHAVIOR_DAMAGED;
        self.action = Action::TakeDamage;
    }

    /// `EnSkb_TakeDamage`: once its head's parts are out (if it's breaking), headless from then
    /// on; sliding to a stop, turning to Link, deciding at the animation's end on the ground.
    fn take_damage(&mut self, play: &mut PlayState) {
        if self.break_flags != 1 || self.body_break.spawn_parts(&mut self.actor, play, 1) {
            if self.break_flags != 0 {
                self.break_flags |= 2;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                self.actor.speed_xz = 0.0;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.05;
            }
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0x1194, 0);
            if self.skel.update() && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.decide_next_action(play);
            }
        }
    }

    /// `EnSkb_SetupDeath`: knocked back 6, its 18 limbs to break into (`BodyBreak_Alloc`), its
    /// death cry where it is in the view (`EffectSsDeadSound`, 40 frames).
    fn setup_death(&mut self, play: &mut PlayState) {
        if let Some(a) = self.anim(|a| &a.dying) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
        }
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = -6.0;
        }
        self.action_state = SKB_BEHAVIOR_DYING;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.body_break.alloc(18);
        self.break_flags |= 4;
        let pos = self.actor.projected_pos;
        play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_EN_STALKID_DEAD, 1, 1, 0x28));
        self.action = Action::Death;
    }

    /// `EnSkb_Death`: once its parts are out: its drop (table 1 at the smallest size, a blue
    /// rupee to 0.015, three reds above), and gone.
    fn death(&mut self, play: &mut PlayState) {
        if self.body_break.spawn_parts(&mut self.actor, play, 1) {
            let pos = self.actor.world_pos;
            if self.actor.scale.x == 0.01 {
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_1 * 16);
            } else if self.actor.scale.x <= 0.015 {
                crate::en_item00::item_drop_collectible(play, pos, ITEM00_RUPEE_BLUE);
            } else {
                for _ in 0..3 {
                    crate::en_item00::item_drop_collectible(play, pos, ITEM00_RUPEE_RED);
                }
            }
            self.break_flags |= 8;
            self.actor.kill();
        }
    }

    /// `EnSkb_CheckDamage`.
    fn check_damage(&mut self, play: &mut PlayState) {
        if self.action_state != SKB_BEHAVIOR_DYING && self.actor.bg_check_flags & (BGCHECKFLAG_WATER | BGCHECKFLAG_WATER_TOUCH) != 0 && self.actor.y_dist_to_water >= 40.0 {
            self.actor.col_chk_info.health = 0;
            self.set_collider_at = false;
            self.setup_death(play);
            return;
        }
        if self.action_state < SKB_BEHAVIOR_ATTACKING || self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let reaction = self.actor.col_chk_info.damage_reaction;
        if reaction == 6 {
            return;
        }
        self.last_damage_reaction = reaction;
        self.actor.set_drop_flag(&self.collider.elements[1].info, true);
        self.set_collider_at = false;
        if reaction == 1 {
            if self.action_state != SKB_BEHAVIOR_STUNNED {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
                self.actor.apply_damage();
                self.setup_stunned(play);
            }
            return;
        }
        let mut duration = 8;
        if reaction == 7 {
            let scale = (self.actor.scale.y * 7500.0) as i16;
            for _ in 0..5 {
                let mut p = self.actor.world_pos;
                p.x += play.rand.centered_float(20.0);
                p.z += play.rand.centered_float(20.0);
                p.y += play.rand.zero_one() * 25.0;
                if let Some(me) = play.cur_actor {
                    let r = self.actor.shape_rot;
                    let actor = oot_game::effect::SsActor { handle: me, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] };
                    play.with_ss(|s| s.en_fire_spawn_vec3f(Some(actor), p, scale, 0, 0, -1));
                }
            }
            duration = 25;
        }
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, duration);
        if self.actor.apply_damage() == 0 {
            self.setup_death(play);
            return;
        }
        let mwa = play.player.and_then(|h| play.actors.downcast::<crate::player::Player>(h)).map(|p| p.melee_weapon_animation).unwrap_or(0);
        if self.break_flags == 0
            && (reaction == 0xD
                || (reaction == 0xE && ((PLAYER_MWA_RIGHT_SLASH_1H..=PLAYER_MWA_LEFT_COMBO_2H).contains(&mwa) || mwa == PLAYER_MWA_BACKSLASH_RIGHT || mwa == PLAYER_MWA_BACKSLASH_LEFT)))
        {
            self.body_break.alloc(2);
            // The head's body break set up.
            self.break_flags = 1;
        }
        self.setup_take_damage(play);
    }

    /// `EnSkb_OverrideLimbDraw`'s `*dList`: limbs 11 and 12 (the head and the jaw) gone once
    /// headless.
    fn limb_dlist(&self, limb: usize) -> Option<&'static str> {
        if (limb == 11 || limb == 12) && self.break_flags & 2 != 0 {
            return None;
        }
        LIMB_DLISTS.get(limb).copied().flatten()
    }
}

impl ActorImpl for EnSkb {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnSkb_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.check_damage(play);
        self.actor.move_forward();
        let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
        self.actor.update_bg_check_info(&play.col, 15.0, 30.0, 60.0, flags);
        match self.action {
            Action::RiseFromGround => self.rise_from_ground(play),
            Action::Despawn => self.despawn(play),
            Action::WalkForward => self.walk_forward(play),
            Action::Attack => self.attack(play),
            Action::Recoil => self.recoil(play),
            Action::Stunned => self.stunned(play),
            Action::TakeDamage => self.take_damage(play),
            Action::Death => self.death(play),
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 3000.0 * self.actor.scale.y;
        if self.set_collider_at {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.action_state >= SKB_BEHAVIOR_ATTACKING && (self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0) {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnSkb_Draw`'s limbs at `Play_Draw`'s time: the spheres on their limbs
    /// (`Collider_UpdateSpheres`), and the pieces a body break takes (`EnSkb_PostLimbDraw`: the
    /// head and jaw, or every limb dying).
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        let freeze = play.actors.freeze_flash_timer;
        let joints = self.skel.joint_table.clone();
        let mut posts = Vec::new();
        draw_opa_pose(&skeleton, &joints, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |limb, m| posts.push((limb, m)));
        for (limb, m) in posts {
            self.collider.update_spheres(limb as u8, &(model * m));
            let dlist = self.limb_dlist(limb).map(|s| (OBJECT, s));
            if self.break_flags ^ 1 == 0 {
                self.body_break.set_info(freeze, limb as i32, 11, 12, 18, dlist, BODYBREAK_OBJECT_SLOT_DEFAULT, model * m);
            } else if self.break_flags ^ (self.break_flags | 4) == 0 {
                self.body_break.set_info(freeze, limb as i32, 0, 18, 18, dlist, BODYBREAK_OBJECT_SLOT_DEFAULT, model * m);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![(self.break_flags & 2 != 0) as u32];
        rs
    }

    /// `EnSkb_Draw`: the skeleton after `Gfx_SetupDL_25Opa`; its head's env colour pulsing
    /// (`|sin(gameplayFrames × 0x1770) × 95| + 160`), or no head and jaw.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let headless = rs.switches.first().copied().unwrap_or(0) != 0;
        let color = ((eng_math::sin_s((play.gameplay_frames.wrapping_mul(0x1770)) as i16) * 95.0) as i16 as i32).abs() as i16 + 160;
        let c = color as u8;
        let bones = skeleton.pose(joints);
        let mut sv = SegmentValues::default();
        sv.env[SEG_COLOR as usize] = Some([c, c, c, 255]);
        let bake = if headless { BAKE_HEADLESS } else { BAKE };
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: actor_draw_matrix(rs), bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    /// `EnSkb_Destroy`: its spawner counted down.
    fn destroy(&mut self, play: &mut PlayState) {
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<crate::en_encount1::EnEncount1>(h))
            && !p.actor.killed
            && p.cur_num_spawn > 0
        {
            p.cur_num_spawn -= 1;
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
