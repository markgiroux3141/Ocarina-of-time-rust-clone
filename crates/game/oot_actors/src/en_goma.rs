//! `En_Goma` (`ovl_En_Goma/z_en_goma.c`): Gohma's eggs and larvae (the Gohma Larvae, "Gohma
//! Jr."), the eggs' hatching debris, and the pieces Queen Gohma falls apart into.
//!
//! Params: 0 to 2 an egg Queen Gohma lays (`Boss_Goma` is its parent, and the egg's index in its
//! `childrenGohmaState`); 6 and 7 an egg on the floor, 8 and 9 one on the ceiling (the Deku
//! Tree's are 6 to 9); 10 to 24 the hatching debris; 100 and up a piece of Queen Gohma
//! (`bossLimbDL` set by `Boss_Goma`).
//!
//! - **An egg** (`EnGoma_Egg`): squishing slowly, now and then shedding fragments
//!   (`EffectSsHahen`); Link within 100 in x and z for 10 frames, it **falls**
//!   (`EnGoma_EggFallToGround`) and on the ground **hatches** (`EnGoma_SetupHatch`): half size,
//!   15 pieces of shell flying off (`EnGoma_SpawnHatchDebris`), facing Link. Queen Gohma's eggs
//!   bounce twice and wait 80 frames first. A hit before it hatches breaks it.
//! - **The larva:** **standing** (`EnGoma_Stand`) 10 to 39 frames turning to Link, then
//!   **chasing** him (`EnGoma_ChasePlayer`) at 10/3; within 150 it **prepares to jump**
//!   (`EnGoma_PrepareJump`, 30 frames, eyes red), **jumps** at him (`EnGoma_Jump`, up at 8,
//!   ahead at up to 10) and **lands** (`EnGoma_Land`), then stands again. Its eyes follow Link
//!   (`EnGoma_LookAtPlayer`); it tilts with the floor's slope (`EnGoma_SetFloorRot`).
//! - **Hit** (`EnGoma_UpdateHit`, no damage table): the shield knocks it back (out of a jump, it
//!   lands); a Deku Nut **stuns** it (`EnGoma_Stunned`) for 100 frames (eyes grey, shaking for
//!   the last 30); anything else takes its sword damage (`CollisionCheck_GetSwordDamage`, at
//!   least 1; a sword hit sprays bubbles, `EffectSsSibuki`) off its 2 health: it's **hurt**
//!   (`EnGoma_Hurt`, thrown back at 20, flashing red and its body's colours at random), then
//!   **flees** (`EnGoma_Flee`) for 20 frames, or at no health **dies** (`EnGoma_Die`, 30
//!   frames), lies **dead** (`EnGoma_Dead`, a flame: `EffectSsKFire`), shrinks away with
//!   `NA_SE_EN_EXTINCT` and drops from table 3.
//!
//! The whole overlay is ported. Queen Gohma (`Boss_Goma`) isn't (GAME-05 milestone 6): its eggs'
//! and larvae's writes to her `childrenGohmaState` go through `boss_goma_set_child_state`, which
//! logs them; the pieces' display lists (`bossLimbDL`) are hers, baked by `boss_limb_bake` when
//! she's ported. `En_Goma_Profile`'s id is `ACTOR_BOSS_GOMA` (@bug (game)): every En_Goma gets
//! that id, as `Actor_Spawn` copies it (`PROFILE`). Not ported: the generic circle shadow
//! (`ActorShadow_DrawCircle`, for no actor), and the camera's quake offset in the larva's draw
//! (quakes aren't ported: 0).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, approach_s, approach_zero_f, atan2_s, smooth_step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BOSS, ACTORCAT_ENEMY, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::effect::hahen::HAHEN_OBJECT_DEFAULT;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::{MtxF, rad_to_binang};

/// `ACTOR_EN_GOMA`, `ACTOR_BOSS_GOMA` (`actor_table.h`).
pub const ACTOR_EN_GOMA: i16 = 0x002B;
pub const ACTOR_BOSS_GOMA: i16 = 0x0028;
const OBJECT: &str = "object_gol";

/// `En_Goma_Profile`: its id is `ACTOR_BOSS_GOMA` (@bug (game)), so `Actor_Spawn` gives every
/// En_Goma (spawned as `ACTOR_EN_GOMA`) the boss's id.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BOSS_GOMA,
    name: "En_Goma",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
};

/// `GomaType`.
pub const ENGOMA_NORMAL: i16 = 0;
pub const ENGOMA_EGG: i16 = 1;
pub const ENGOMA_HATCH_DEBRIS: i16 = 2;
pub const ENGOMA_BOSSLIMB: i16 = 3;

/// `EnGomaLimb`: the body and the eyes' root.
const GOMA_LIMB_BODY: usize = 3;
const GOMA_LIMB_EYE_IRIS_ROOT1: usize = 7;
/// `GOMA_LIMB_MAX`.
const GOMA_LIMB_MAX: usize = 24;

/// `NAVI_ENEMY_GOHMA_EGG`, `NAVI_ENEMY_GOHMA_LARVA` (`actor.h`).
const NAVI_ENEMY_GOHMA_EGG: u8 = 0x02;
const NAVI_ENEMY_GOHMA_LARVA: u8 = 0x03;
/// `COLLECTIBLE_DROP_TABLE_3` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_3: i16 = 3;

/// The colliders' ids (`collider_mut`): `colliderCylinder1` (AT and OC), `colliderCylinder2`
/// (AC).
pub const COL_CYLINDER1: u8 = 0;
pub const COL_CYLINDER2: u8 = 1;

/// `D_80A4B7A0`: the bite (`0xFFCFFFFF` for 8) and the push, 15 by 30, 10 up.
const CYLINDER1_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFDF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 30, y_shift: 10, pos: [0, 0, 0] },
};

/// `D_80A4B7CC`: hit by everything but the mirror's ray (`0xFFDFFFFF`, the shield's too), 15
/// by 30, 10 up (35 by 35 while Link swings).
const CYLINDER2_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFDF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 15, height: 30, y_shift: 10, pos: [0, 0, 0] },
};

/// `sDeadEffectVel`.
const S_DEAD_EFFECT_VEL: Vec3 = Vec3::ZERO;

/// `sShieldKnockbackVel` (`EnGoma_UpdateHit`).
const S_SHIELD_KNOCKBACK_VEL: Vec3 = Vec3::new(0.0, 0.0, 20.0);

/// `sTargetEyeEnvColors` (`EnGoma_UpdateEyeEnvColor`), read `[0][visualState]`,
/// `[1][visualState]`, `[2][visualState]`: each row is one channel's three states (red when
/// jumping, green normally, grey dead or stunned).
const S_TARGET_EYE_ENV_COLORS: [[f32; 3]; 3] = [[255.0, 0.0, 50.0], [17.0, 255.0, 50.0], [0.0, 170.0, 50.0]];

/// The overlay's statics shared by its actors: `sSpawnNum`.
#[derive(Debug, Default)]
struct Statics {
    spawn_num: u8,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Flee,
    EggFallToGround,
    Egg,
    Hatch,
    Hurt,
    Die,
    Dead,
    PrepareJump,
    Land,
    Jump,
    Stand,
    ChasePlayer,
    Stunned,
    Debris,
    BossLimb,
    /// An egg of another params (3 to 5): the C leaves `actionFunc` NULL (@bug (game): its
    /// update would jump to 0). No such egg is placed.
    None,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Flee => "EnGoma_Flee",
            Action::EggFallToGround => "EnGoma_EggFallToGround",
            Action::Egg => "EnGoma_Egg",
            Action::Hatch => "EnGoma_Hatch",
            Action::Hurt => "EnGoma_Hurt",
            Action::Die => "EnGoma_Die",
            Action::Dead => "EnGoma_Dead",
            Action::PrepareJump => "EnGoma_PrepareJump",
            Action::Land => "EnGoma_Land",
            Action::Jump => "EnGoma_Jump",
            Action::Stand => "EnGoma_Stand",
            Action::ChasePlayer => "EnGoma_ChasePlayer",
            Action::Stunned => "EnGoma_Stunned",
            Action::Debris => "EnGoma_Debris",
            Action::BossLimb => "EnGoma_BossLimb",
            Action::None => "NULL",
        }
    }
}

/// The animations of `object_gol` the larva plays.
#[derive(Default)]
struct Anims {
    stand: Option<Anim>,
    running: Option<Anim>,
    jump_headbutt: Option<Anim>,
    damaged: Option<Anim>,
    death: Option<Anim>,
    dead_twitching: Option<Anim>,
    prepare_jump: Option<Anim>,
    land_from_jump: Option<Anim>,
}

/// `Math_Vec3f_Pitch`.
fn vec3f_pitch(a: Vec3, b: Vec3) -> i16 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    atan2_s((dx * dx + dz * dz).sqrt(), a.y - b.y)
}

/// `BINANG_TO_RAD_ALT`: `((f32)binang / (f32)0x8000) * M_PI`, the product in double precision.
fn binang_to_rad_alt(b: i16) -> f32 {
    ((b as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32
}

pub struct EnGoma {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    pub action: Action,
    pub slope_pitch: i16,
    pub slope_roll: i16,
    /// `gomaType` (`ENGOMA_*`).
    pub goma_type: i16,
    pub eye_pitch: i16,
    pub eye_yaw: i16,
    pub hatch_state: i16,
    pub egg_timer: i16,
    pub hurt_timer: i16,
    /// `visualState`: the eyes' colour (0 red, 1 green, 2 grey).
    pub visual_state: i16,
    pub player_detection_timer: i16,
    /// `spawnNum`: "some debug spawn ID", from `sSpawnNum` (params 6 and 8 only).
    pub spawn_num: i16,
    pub invincibility_timer: i16,
    pub action_timer: i16,
    pub egg_scale: f32,
    pub egg_pitch: f32,
    pub egg_squish_angle: f32,
    pub egg_squish_accel: f32,
    pub eye_env_color: [f32; 3],
    pub egg_squish_amount: f32,
    pub egg_y_offset: f32,
    pub stun_timer: i16,
    pub shield_knockback_vel: Vec3,
    /// `bossLimbDL` (set by `Boss_Goma`): the file and symbol of one of her limbs' lists.
    pub boss_limb_dl: Option<(&'static str, &'static str)>,
    pub collider_cylinder1: ColliderCylinder,
    pub collider_cylinder2: ColliderCylinder,
    /// The body's env colour `EnGoma_OverrideLimbDraw` drew this game frame while hurt (its
    /// `Rand` colours, made in `draw_update`), else `None`.
    pub hurt_body_env: Option<[u8; 3]>,
}

impl EnGoma {
    /// `EnGoma_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // this->eggTimer = Rand_ZeroOne() * 200.0f.
        let egg_timer = (play.rand.zero_one() * 200.0) as i16;
        // sInitChain: ATTENTION_RANGE_3, NAVI_ENEMY_GOHMA_LARVA, gravity 0, lockOnArrowOffset 20.
        actor.target_mode = 3;
        actor.navi_enemy_id = NAVI_ENEMY_GOHMA_LARVA;
        actor.gravity = 0.0;
        actor.target_arrow_offset = 20.0;
        actor.scale = Vec3::splat(0.01);
        let params = actor.params;
        let mut g = EnGoma {
            actor,
            skel: SkelAnimeStd::init_flex(GOMA_LIMB_MAX - 1, None),
            skeleton: None,
            anims: Anims::default(),
            action: Action::None,
            slope_pitch: 0,
            slope_roll: 0,
            goma_type: ENGOMA_NORMAL,
            eye_pitch: 0,
            eye_yaw: 0,
            hatch_state: 0,
            egg_timer,
            hurt_timer: 0,
            visual_state: 0,
            player_detection_timer: 0,
            spawn_num: 0,
            invincibility_timer: 0,
            action_timer: 0,
            egg_scale: 0.0,
            egg_pitch: 0.0,
            egg_squish_angle: 0.0,
            egg_squish_accel: 0.0,
            eye_env_color: [0.0; 3],
            egg_squish_amount: 0.0,
            egg_y_offset: 0.0,
            stun_timer: 0,
            shield_knockback_vel: Vec3::ZERO,
            boss_limb_dl: None,
            collider_cylinder1: ColliderCylinder::new(&CYLINDER1_INIT),
            collider_cylinder2: ColliderCylinder::new(&CYLINDER2_INIT),
            hurt_body_env: None,
        };
        if params >= 100 {
            // A piece of Queen Gohma: Actor_ChangeCategory(ACTORCAT_BOSS) (not in a list yet:
            // it goes into the boss list).
            g.actor.category = ACTORCAT_BOSS;
            g.action = Action::BossLimb;
            g.goma_type = ENGOMA_BOSSLIMB;
            // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 0): the circle shadow isn't ported.
            g.actor.shape_y_offset = 0.0;
            g.action_timer = g.actor.params.wrapping_add(150);
            g.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        } else if params >= 10 {
            // Debris when hatching.
            g.actor.gravity = -1.3;
            g.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            g.action_timer = 50;
            g.goma_type = ENGOMA_HATCH_DEBRIS;
            g.egg_scale = 1.0;
            g.actor.velocity.y = play.rand.zero_one() * 5.0 + 5.0;
            g.action = Action::Debris;
            g.actor.speed_xz = play.rand.zero_one() * 2.3 + 1.5;
            g.action_timer = 30;
            g.actor.scale.x = play.rand.zero_one() * 0.005 + 0.01;
            g.actor.scale.y = play.rand.zero_one() * 0.005 + 0.01;
            g.actor.scale.z = play.rand.zero_one() * 0.005 + 0.01;
            // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 0).
            g.actor.shape_y_offset = 0.0;
        } else {
            // An egg. ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 40).
            g.actor.shape_y_offset = 0.0;
            // SkelAnime_Init(&gObjectGolSkel, &gObjectGolStandAnim), Animation_PlayLoop.
            if let Some(a) = play.assets.clone() {
                let anim = |name: &str| a.animation(OBJECT, name).map_err(|e| log::error!("En_Goma: {e:#}")).ok();
                g.skeleton = a.skeleton(OBJECT, "gObjectGolSkel").map_err(|e| log::error!("En_Goma: {e:#}")).ok();
                g.anims = Anims {
                    stand: anim("gObjectGolStandAnim"),
                    running: anim("gObjectGolRunningAnim"),
                    jump_headbutt: anim("gObjectGolJumpHeadbuttAnim"),
                    damaged: anim("gObjectGolDamagedAnim"),
                    death: anim("gObjectGolDeathAnim"),
                    dead_twitching: anim("gObjectGolDeadTwitchingAnim"),
                    prepare_jump: anim("gObjectGolPrepareJumpAnim"),
                    land_from_jump: anim("gObjectGolLandFromJumpAnim"),
                };
            }
            let limbs = g.skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(GOMA_LIMB_MAX - 1);
            g.skel = SkelAnimeStd::init_flex(limbs, g.anims.stand.clone());
            g.actor.col_chk_info.health = 2;
            if g.actor.params < 3 {
                // Spawned by the boss.
                g.action = Action::EggFallToGround;
                g.invincibility_timer = 10;
                g.actor.speed_xz = 1.5;
            } else if g.actor.params == 8 || g.actor.params == 6 {
                g.action = Action::Egg;
                let s = play.overlay_static::<Statics>(ACTOR_EN_GOMA);
                g.spawn_num = s.spawn_num as i16;
                s.spawn_num = s.spawn_num.wrapping_add(1);
            } else if g.actor.params == 9 || g.actor.params == 7 {
                g.action = Action::Egg;
            }
            // On the ceiling from 8.
            g.egg_y_offset = if g.actor.params >= 8 { -1500.0 } else { 1500.0 };
            g.goma_type = ENGOMA_EGG;
            g.egg_scale = 1.0;
            g.egg_squish_angle = play.rand.zero_one() * 1000.0;
            g.action_timer = 50;
            // Collider_SetCylinder: the colliders' positions follow from Collider_UpdateCylinder.
        }
        Box::new(g)
    }

    /// The `params < 6` choice the C makes for every sound: Queen Gohma's larvae (`BJR`) or the
    /// placed ones (`JR`).
    fn sfx(&self, bjr: u16, jr: u16) -> u16 {
        if self.actor.params < 6 { bjr } else { jr }
    }

    /// `Animation_Change(&skelanime, anim, speed, 0, Animation_GetLastFrame(anim), mode, morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, mode: u8, morph: f32) {
        if let Some(a) = anim {
            let last = a.last_frame();
            self.skel.change(a, speed, 0.0, last, mode, morph);
        }
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default()
    }

    /// `Actor_WorldYawTowardActor(&this->actor, &GET_PLAYER(play)->actor)`.
    fn yaw_to_player(&self, play: &PlayState) -> i16 {
        vec3f_yaw(self.actor.world_pos, Self::player_pos(play))
    }

    /// `EnGoma_SetupFlee`: running (at twice the speed) for 20 frames.
    fn setup_flee(&mut self, play: &mut PlayState) {
        self.change(self.anims.running.clone(), 2.0, ANIMMODE_LOOP, -2.0);
        self.action = Action::Flee;
        self.action_timer = 20;
        audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_DAM2, NA_SE_EN_GOMA_JR_DAM2));
    }

    /// `EnGoma_Flee`: away from Link at up to 20/3, then standing.
    fn flee(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_f(&mut self.actor.speed_xz, 20.0 / 3.0, 0.5, 2.0);
        let away = self.yaw_to_player(play).wrapping_add(i16::MIN);
        approach_s(&mut self.actor.world_rot.y, away, 3, 2000);
        approach_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 2, 3000);
        if self.action_timer == 0 {
            self.setup_stand(play);
        }
    }

    /// `EnGoma_EggFallToGround`: falling (gravity -1.3), spinning ever faster, its squish
    /// settling and the egg's offset to 1500 (a ceiling egg turns over). On the ground
    /// (`NA_SE_EN_GOMA_EGG1`) a placed egg hatches; Queen Gohma's bounces (state 1: swelling for
    /// 3 frames; 2: squashed for 3, thrown up at 5 and on at 2; 3: back to size), and hatches
    /// after 80 frames. On the ground it slows; it rolls with its speed.
    fn egg_fall_to_ground(&mut self, play: &mut PlayState) {
        self.actor.gravity = -1.3;
        self.egg_squish_accel += 0.03;
        self.egg_squish_angle += 1.0 + self.egg_squish_accel;
        approach_zero_f(&mut self.egg_squish_amount, 1.0, 0.005);
        approach_f(&mut self.egg_y_offset, 1500.0, 1.0, 150.0);
        match self.hatch_state {
            0 => {
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_EGG1, NA_SE_EN_GOMA_EGG1));
                    if self.actor.params > 5 {
                        self.setup_hatch(play);
                    } else {
                        self.hatch_state = 1;
                        self.action_timer = 3;
                        approach_f(&mut self.egg_scale, 1.5, 0.5, 1.0);
                    }
                }
            }
            1 => {
                if self.action_timer == 0 {
                    self.hatch_state = 2;
                    self.action_timer = 3;
                    approach_f(&mut self.egg_scale, 0.75, 0.5, 1.0);
                    self.actor.velocity.y = 5.0;
                    self.actor.speed_xz = 2.0;
                } else {
                    approach_f(&mut self.egg_scale, 1.5, 0.5, 1.0);
                }
            }
            2 => {
                if self.action_timer == 0 {
                    self.hatch_state = 3;
                    self.action_timer = 80;
                } else {
                    approach_f(&mut self.egg_scale, 0.75, 0.5, 1.0);
                }
            }
            3 => {
                approach_f(&mut self.egg_scale, 1.0, 0.1, 0.1);
                if self.action_timer == 0 {
                    self.setup_hatch(play);
                }
            }
            _ => {}
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            approach_zero_f(&mut self.actor.speed_xz, 0.2, 0.05);
        }
        self.egg_pitch += self.actor.speed_xz * 0.1;
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnGoma_Egg`: squishing; Link within 100 in x and z for 10 frames, it falls; every 16th
    /// frame of `eggTimer`, a half chance of two fragments (`EffectSsHahen_Spawn`, 10 to 14 big,
    /// 10 frames) within 15 across and 30 up.
    fn egg(&mut self, play: &mut PlayState) {
        let player_pos = Self::player_pos(play);
        self.egg_squish_angle += 1.0;
        approach_f(&mut self.egg_squish_amount, 0.1, 1.0, 0.005);
        if (self.actor.world_pos.x - player_pos.x).abs() < 100.0 && (self.actor.world_pos.z - player_pos.z).abs() < 100.0 {
            self.player_detection_timer += 1;
            if self.player_detection_timer > 9 {
                self.action = Action::EggFallToGround;
            }
        } else {
            self.player_detection_timer = 0;
        }
        if self.egg_timer & 0xF == 0 && play.rand.zero_one() < 0.5 {
            let p = self.actor.world_pos;
            play.with_ss(|ss| {
                for _ in 0..2 {
                    let vel = Vec3::ZERO;
                    let acc = Vec3::new(0.0, -0.5, 0.0);
                    let x = ss.rand.centered_float(30.0) + p.x;
                    let y = ss.rand.zero_float(30.0) + p.y;
                    let z = ss.rand.centered_float(30.0) + p.z;
                    let scale = (ss.rand.zero_one() * 5.0) as i16 + 10;
                    ss.hahen_spawn(Vec3::new(x, y, z), vel, acc, 0, scale, HAHEN_OBJECT_DEFAULT, 10, None);
                }
            });
        }
    }

    /// `EnGoma_SetupHatch`: the larva at half size, facing Link, after 5 frames standing; the
    /// shell's 15 pieces fly off.
    fn setup_hatch(&mut self, play: &mut PlayState) {
        self.change(self.anims.jump_headbutt.clone(), 1.0, ANIMMODE_ONCE, 0.0);
        self.action = Action::Hatch;
        self.actor.scale = Vec3::splat(0.005);
        self.goma_type = ENGOMA_NORMAL;
        self.action_timer = 5;
        self.actor.shape_rot.y = self.yaw_to_player(play);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.spawn_hatch_debris(play);
        self.egg_scale = 1.0;
        self.actor.speed_xz = 0.0;
    }

    /// `EnGoma_Hatch`.
    fn hatch(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.action_timer == 0 {
            self.setup_stand(play);
        }
    }

    /// `EnGoma_SetupHurt`: thrown back from Link at 20; at no health 5 frames (the finishing
    /// blow), else 10.
    fn setup_hurt(&mut self, play: &mut PlayState) {
        self.change(self.anims.damaged.clone(), 1.0, ANIMMODE_ONCE, -2.0);
        self.action = Action::Hurt;
        if self.actor.col_chk_info.health as i8 <= 0 {
            self.action_timer = 5;
            enemy_start_finishing_blow(play, &self.actor);
        } else {
            self.action_timer = 10;
        }
        self.actor.speed_xz = 20.0;
        self.actor.world_rot.y = self.actor.yaw_towards_player.wrapping_add(i16::MIN);
        audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_DAM1, NA_SE_EN_GOMA_JR_DAM1));
    }

    /// `EnGoma_Hurt`: slowing on the ground; then dying or fleeing.
    fn hurt(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            approach_zero_f(&mut self.actor.speed_xz, 1.0, 2.0);
        }
        if self.action_timer == 0 {
            if self.actor.col_chk_info.health as i8 <= 0 {
                self.setup_die(play);
            } else {
                self.setup_flee(play);
            }
        }
    }

    /// `EnGoma_SetupDie`: 30 frames, invincible for 100, no longer targetable.
    fn setup_die(&mut self, play: &mut PlayState) {
        self.change(self.anims.death.clone(), 1.0, ANIMMODE_ONCE, -2.0);
        self.action = Action::Die;
        self.action_timer = 30;
        audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_DEAD, NA_SE_EN_GOMA_JR_DEAD));
        self.invincibility_timer = 100;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
    }

    /// `EnGoma_Die`: slowing on the ground, falling over (`NA_SE_EN_GOMA_JR_LAND` at 17 left).
    fn die(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            approach_zero_f(&mut self.actor.speed_xz, 1.0, 2.0);
        }
        if self.action_timer == 17 {
            audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_LAND, NA_SE_EN_GOMA_JR_LAND));
        }
        if self.action_timer == 0 {
            self.setup_dead();
        }
    }

    /// `EnGoma_SetupDead`.
    fn setup_dead(&mut self) {
        self.change(self.anims.dead_twitching.clone(), 1.0, ANIMMODE_LOOP, -2.0);
        self.action = Action::Dead;
        self.action_timer = 3;
    }

    /// `EnGoma_Dead`: twitching; at 2 left a flame (`EffectSsKFire_Spawn`, 40, rising by 0.03);
    /// from 0 its height shrinks, and at 0.001 or less (Queen Gohma's larva freeing its slot)
    /// `NA_SE_EN_EXTINCT`, gone, and a drop from table 3. Eyes grey.
    fn dead(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 1.0, 2.0);
        if self.action_timer == 2 {
            let p = self.actor.world_pos;
            let pos = Vec3::new(p.x, (p.y + 5.0) - 10.0, p.z);
            let accel = Vec3::new(S_DEAD_EFFECT_VEL.x, 0.03, S_DEAD_EFFECT_VEL.z);
            play.with_ss(|ss| ss.k_fire_spawn(pos, S_DEAD_EFFECT_VEL, accel, 40, 0));
        }
        if self.action_timer == 0 && smooth_step_to_f(&mut self.actor.scale.y, 0.0, 0.5, 0.00225, 0.00001) <= 0.001 {
            if self.actor.params < 6 {
                boss_goma_set_child_state(play, self.actor.parent, self.actor.params, -1);
            }
            // SFX_PLAY_AT_POS(&this->actor.projectedPos, NA_SE_EN_EXTINCT).
            audio_play_actor_sfx2(play, NA_SE_EN_EXTINCT);
            self.actor.kill();
            // COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_3, false), no actor.
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, None, pos, COLLECTIBLE_DROP_TABLE_3 * 16);
        }
        self.visual_state = 2;
    }

    /// `EnGoma_SetupStand`: 10 to 39 frames (`Rand_S16Offset(10, 30)`).
    fn setup_stand(&mut self, play: &mut PlayState) {
        self.action_timer = play.rand.s16_offset(10, 30);
        self.change(self.anims.stand.clone(), 1.0, ANIMMODE_LOOP, -5.0);
        self.action = Action::Stand;
        self.goma_type = ENGOMA_NORMAL;
    }

    /// `EnGoma_SetupChasePlayer`: 70 to 179 frames (`Rand_S16Offset(70, 110)`).
    fn setup_chase_player(&mut self, play: &mut PlayState) {
        self.change(self.anims.running.clone(), 1.0, ANIMMODE_LOOP, -5.0);
        self.action = Action::ChasePlayer;
        self.action_timer = play.rand.s16_offset(70, 110);
    }

    /// `EnGoma_SetupPrepareJump`: 30 frames.
    fn setup_prepare_jump(&mut self) {
        self.change(self.anims.prepare_jump.clone(), 1.0, ANIMMODE_ONCE, -5.0);
        self.action = Action::PrepareJump;
        self.action_timer = 30;
    }

    /// `EnGoma_PrepareJump`: stopping, turning to Link; then jumping. Eyes red.
    fn prepare_jump(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        let target_angle = self.yaw_to_player(play);
        approach_s(&mut self.actor.world_rot.y, target_angle, 2, 4000);
        approach_s(&mut self.actor.shape_rot.y, target_angle, 2, 3000);
        if self.action_timer == 0 {
            self.setup_jump(play);
        }
        self.visual_state = 0;
    }

    /// `EnGoma_SetupLand`: 10 frames.
    fn setup_land(&mut self) {
        self.change(self.anims.land_from_jump.clone(), 1.0, ANIMMODE_ONCE, 0.0);
        self.action = Action::Land;
        self.action_timer = 10;
    }

    /// `EnGoma_Land`: slowing on the ground; then standing.
    fn land(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            approach_zero_f(&mut self.actor.speed_xz, 1.0, 2.0);
        }
        if self.action_timer == 0 {
            self.setup_stand(play);
        }
    }

    /// `EnGoma_SetupJump`: up at 8, `NA_SE_EN_GOMA_JR_CRY`.
    fn setup_jump(&mut self, play: &mut PlayState) {
        self.change(self.anims.jump_headbutt.clone(), 1.0, ANIMMODE_ONCE, 0.0);
        self.action = Action::Jump;
        self.actor.velocity.y = 8.0;
        audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_CRY, NA_SE_EN_GOMA_JR_CRY));
    }

    /// `EnGoma_Jump`: its body's hit sounds as Player's (`ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT`),
    /// on at up to 10; falling onto the ground, it lands (`NA_SE_EN_GOMA_JR_LAND2`). Eyes red.
    fn jump(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.skel.update();
        approach_f(&mut self.actor.speed_xz, 10.0, 0.5, 5.0);
        if self.actor.velocity.y <= 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.setup_land();
            audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_LAND2, NA_SE_EN_GOMA_JR_LAND2));
        }
        self.visual_state = 0;
    }

    /// `EnGoma_Stand`: stopping, turning to Link; then chasing him.
    fn stand(&mut self, play: &mut PlayState) {
        self.skel.update();
        approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        let yaw = self.yaw_to_player(play);
        approach_s(&mut self.actor.shape_rot.y, yaw, 2, 3000);
        if self.action_timer == 0 {
            self.setup_chase_player(play);
        }
    }

    /// `EnGoma_ChasePlayer`: running at Link (steps at frames 1 and 5) at up to 10/3; within
    /// 150 across, preparing to jump. (Its `actionTimer` isn't read.)
    fn chase_player(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(1.0) || self.skel.on_frame(5.0) {
            audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_WALK, NA_SE_EN_GOMA_JR_WALK));
        }
        approach_f(&mut self.actor.speed_xz, 10.0 / 3.0, 0.5, 2.0);
        approach_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 3, 2000);
        approach_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 2, 3000);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.velocity.y = 0.0;
        }
        if self.actor.xz_dist_to_player <= 150.0 {
            self.setup_prepare_jump();
        }
    }

    /// `EnGoma_SetupStunned`: 100 frames, its animation going for 3 to 17 (`(s16)Rand_ZeroFloat(15)
    /// + 3`), `NA_SE_EN_GOMA_JR_FREEZE`.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.action = Action::Stunned;
        self.stun_timer = 100;
        // Animation_MorphToLoop(&skelanime, &gObjectGolStandAnim, -5).
        if let Some(a) = self.anims.stand.clone() {
            self.skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -5.0);
        }
        self.action_timer = play.rand.zero_float(15.0) as i16 + 3;
        audio_play_actor_sfx2(play, self.sfx(NA_SE_EN_GOMA_BJR_FREEZE, NA_SE_EN_GOMA_JR_FREEZE));
    }

    /// `EnGoma_Stunned`: blue (`Actor_SetColorFilter`, 180, 2 frames at a time), eyes grey;
    /// stopping on the ground; for its last 30 frames shaking 1.5 in x and z; then standing.
    fn stunned(&mut self, play: &mut PlayState) {
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 180, COLORFILTER_BUFFLAG_OPA, 2);
        self.visual_state = 2;
        if self.action_timer != 0 {
            self.skel.update();
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.velocity.y = 0.0;
            approach_zero_f(&mut self.actor.speed_xz, 0.5, 2.0);
        }
        if self.stun_timer == 0 {
            self.setup_stand(play);
        } else {
            self.stun_timer -= 1;
            if self.stun_timer < 30 {
                if self.stun_timer & 1 != 0 {
                    self.actor.world_pos.x += 1.5;
                    self.actor.world_pos.z += 1.5;
                } else {
                    self.actor.world_pos.x -= 1.5;
                    self.actor.world_pos.z -= 1.5;
                }
            }
        }
    }

    /// `EnGoma_LookAtPlayer`: the eyes towards Link, their yaw within ±6000 of the body's.
    fn look_at_player(&mut self, play: &PlayState) {
        let player_pos = Self::player_pos(play);
        let mut eye_yaw = vec3f_yaw(self.actor.world_pos, player_pos).wrapping_sub(self.actor.shape_rot.y);
        let eye_pitch = vec3f_pitch(self.actor.world_pos, player_pos).wrapping_sub(self.actor.shape_rot.x);
        eye_yaw = eye_yaw.clamp(-6000, 6000);
        approach_s(&mut self.eye_yaw, eye_yaw, 3, 2000);
        approach_s(&mut self.eye_pitch, eye_pitch, 3, 2000);
    }

    /// `EnGoma_UpdateHit`: after a hit, 8 (stun) or 13 frames of `hurtTimer` before the next.
    /// Its bite landing (`AT_HIT`) mid-jump, it lands where it is. A hit on the larva:
    /// - the shield (`DMG_SHIELD`): out of a jump, landing pushed back at 5; else knocked back
    ///   along Link's facing at 20 (`sShieldKnockbackVel`), invincible 5 frames;
    /// - a Deku Nut (`DMG_DEKU_NUT`): stunned (unless it is);
    /// - anything else: its sword damage (the bubbles for a sword; at least 1) off its health,
    ///   hurt, flashing red 5 frames.
    ///
    /// A hit on an egg breaks it (its debris; Queen Gohma's frees its slot).
    fn update_hit(&mut self, play: &mut PlayState) {
        let player_rot_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.shape_rot.y).unwrap_or(0);
        if self.hurt_timer != 0 {
            self.hurt_timer -= 1;
            return;
        }
        if self.collider_cylinder1.base.at_flags & AT_HIT != 0 && self.action == Action::Jump {
            self.setup_land();
            self.actor.speed_xz = 0.0;
            self.actor.velocity.y = 0.0;
        }
        if self.collider_cylinder2.base.ac_flags & AC_HIT != 0 && (self.actor.col_chk_info.health as i8) > 0 {
            let ac_hit_elem = self.collider_cylinder2.info.ac_hit_elem;
            self.collider_cylinder2.base.ac_flags &= !AC_HIT;
            if self.goma_type == ENGOMA_NORMAL {
                // acHitElem->atDmgInfo.dmgFlags (the C reads through it: a hit always sets it).
                let dmg_flags = ac_hit_elem.map(|e| e.at_dmg_info.dmg_flags).unwrap_or(0);
                if dmg_flags & DMG_SHIELD != 0 {
                    if self.action == Action::Jump {
                        self.setup_land();
                        self.actor.velocity.y = 0.0;
                        self.actor.speed_xz = -5.0;
                    } else {
                        // Matrix_RotateY(BINANG_TO_RAD_ALT(player->actor.shape.rot.y), MTXMODE_NEW),
                        // Matrix_MultVec3f(&sShieldKnockbackVel, &this->shieldKnockbackVel).
                        let mut m = MtxF::IDENTITY;
                        m.rotate_y(binang_to_rad_alt(player_rot_y));
                        self.shield_knockback_vel = m.mult_vec3f(S_SHIELD_KNOCKBACK_VEL);
                        self.invincibility_timer = 5;
                    }
                } else if dmg_flags & DMG_DEKU_NUT != 0 {
                    // Stun.
                    if self.action != Action::Stunned {
                        self.setup_stunned(play);
                        self.hurt_timer = 8;
                    }
                } else {
                    let mut sword_damage = collision_check_get_sword_damage(dmg_flags);
                    if sword_damage != 0 {
                        let focus = self.actor.focus_pos;
                        play.with_ss(|ss| ss.sibuki_spawn_burst(focus));
                    } else {
                        sword_damage = 1;
                    }
                    self.actor.col_chk_info.health = self.actor.col_chk_info.health.wrapping_sub(sword_damage);
                    self.setup_hurt(play);
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 5);
                    self.hurt_timer = 13;
                }
            } else {
                // Die if still an egg.
                if self.actor.params <= 5 {
                    // @bug (game): "BossGoma only has 3 children" (the C's `//!`): params 3 to
                    // 5 would write past `childrenGohmaState` (none is placed).
                    boss_goma_set_child_state(play, self.actor.parent, self.actor.params, -1);
                }
                self.spawn_hatch_debris(play);
                self.actor.kill();
            }
        }
    }

    /// `EnGoma_UpdateEyeEnvColor`: the eyes' (and body's) env colour halfway to `visualState`'s,
    /// at most 20 a frame.
    fn update_eye_env_color(&mut self) {
        let v = self.visual_state as usize;
        approach_f(&mut self.eye_env_color[0], S_TARGET_EYE_ENV_COLORS[0][v], 0.5, 20.0);
        approach_f(&mut self.eye_env_color[1], S_TARGET_EYE_ENV_COLORS[1][v], 0.5, 20.0);
        approach_f(&mut self.eye_env_color[2], S_TARGET_EYE_ENV_COLORS[2][v], 0.5, 20.0);
    }

    /// `EnGoma_SetFloorRot`: tilting towards its floor's slope, 1000 a frame at most.
    fn set_floor_rot(&mut self, play: &PlayState) {
        if let Some(f) = self.actor.floor_poly {
            let n = play.col.poly(f).normal;
            // COLPOLY_GET_NORMAL.
            let (nx, ny, nz) = (n[0] as f32 * (1.0 / 32767.0), n[1] as f32 * (1.0 / 32767.0), n[2] as f32 * (1.0 / 32767.0));
            approach_s(&mut self.slope_pitch, rad_to_binang(-oot_game::camera::f_atan2f(-nz * ny, 1.0)), 1, 1000);
            approach_s(&mut self.slope_roll, rad_to_binang(oot_game::camera::f_atan2f(-nx * ny, 1.0)), 1, 1000);
        }
    }

    /// `EnGoma_Debris`: spinning; gone when its 30 frames are up.
    fn debris(&mut self) {
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(2500);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_add(3500);
        if self.action_timer == 0 {
            self.actor.kill();
        }
    }

    /// `EnGoma_SpawnHatchDebris`: `NA_SE_EN_GOMA_EGG2` where it is (40 frames), and 15 pieces of
    /// shell (`ACTOR_EN_GOMA`, params 10 to 24) as its children, within 5 of it (15 up), at
    /// random yaws.
    ///
    /// Each spawn's arguments make their `Rand_CenteredFloat` calls left to right (x, y, z, the
    /// yaw), as IDO evaluates them.
    fn spawn_hatch_debris(&mut self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        play.sfx_source_play_sfx_at_fixed_world_pos(pos, 40, self.sfx(NA_SE_EN_GOMA_BJR_EGG2, NA_SE_EN_GOMA_EGG2));
        for i in 0..15i16 {
            let p = self.actor.world_pos;
            let x = play.rand.centered_float(10.0) + p.x;
            let y = play.rand.centered_float(10.0) + p.y + 15.0;
            let z = play.rand.centered_float(10.0) + p.z;
            // (s16)Rand_CenteredFloat(0x10000 - 0.01f).
            let yaw = play.rand.centered_float(65536.0 - 0.01) as i32 as i16;
            if let Err(e) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_GOMA, Vec3::new(x, y, z), [0, yaw, 0], i + 10) {
                log::debug!("En_Goma: debris {i} not spawned: {e:?}");
            }
        }
    }

    /// `EnGoma_BossLimb`: (5 lower for the floor check) on the ground it stops, else spins
    /// (once under 250); falls from 250 (gravity -1); under 121 it shrinks away; every 8th frame
    /// a puff of blue dust (`func_8002836C`, 500 big, 10 frames) within 10 across and 5 up.
    fn boss_limb(&mut self, play: &mut PlayState) {
        let vel = Vec3::ZERO;
        let accel = Vec3::new(0.0, 1.0, 0.0);
        let prim_color = [255, 255, 255, 255];
        let env_color = [0, 100, 255, 255];
        self.actor.world_pos.y -= 5.0;
        self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_2);
        self.actor.world_pos.y += 5.0;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.velocity.y = 0.0;
        } else if self.action_timer < 250 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(2000);
        }
        if self.action_timer == 250 {
            self.actor.gravity = -1.0;
        }
        if self.action_timer < 121 {
            if smooth_step_to_f(&mut self.actor.scale.y, 0.0, 1.0, 0.00075, 0.0) <= 0.001 {
                self.actor.kill();
            }
            self.actor.scale.x = self.actor.scale.y;
            self.actor.scale.z = self.actor.scale.y;
        }
        if self.action_timer % 8 == 0 && self.action_timer != 0 {
            let p = self.actor.world_pos;
            play.with_ss(|ss| {
                let x = ss.rand.centered_float(20.0) + p.x;
                let y = ss.rand.centered_float(10.0) + p.y;
                let z = ss.rand.centered_float(20.0) + p.z;
                ss.func_8002836c(Vec3::new(x, y, z), vel, accel, prim_color, env_color, 500, 10, 10);
            });
        }
    }

    /// `EnGoma_Draw` for `ENGOMA_NORMAL`: the larva's model matrix, its slope's tilt under its
    /// own rotation (`BINANG_TO_RAD_ALT`).
    fn larva_matrix(rs: &RenderState, slope_pitch: i16, slope_roll: i16) -> Mat4 {
        // play->mainCamera.quakeOffset.y: quakes aren't ported (0).
        Mat4::from_translation(rs.pos + Vec3::Y * (rs.y_offset * rs.scale.y))
            * Mat4::from_rotation_x(binang_to_rad_alt(slope_pitch))
            * Mat4::from_rotation_z(binang_to_rad_alt(slope_roll))
            * Mat4::from_rotation_y(binang_to_rad_alt(rs.rot[1]))
            * Mat4::from_rotation_x(binang_to_rad_alt(rs.rot[0]))
            * Mat4::from_rotation_z(binang_to_rad_alt(rs.rot[2]))
            * Mat4::from_scale(rs.scale)
    }
}

/// `((BossGoma*)this->actor.parent)->childrenGohmaState[index] = state`.
///
/// HOOK (GAME-05 milestone 6): `Boss_Goma` isn't ported, so there's no state to write; the
/// write is logged. Its port makes this the write into the parent's `childrenGohmaState`. (The
/// C writes through whatever the parent is: with none, a placed egg's NULL; only Queen Gohma's
/// own eggs and larvae, params 0 to 2, reach here in the game.)
pub fn boss_goma_set_child_state(play: &mut PlayState, parent: Option<ActorHandle>, index: i16, state: i8) {
    let parent_name = parent.and_then(|h| play.actors.get(h)).map(|a| a.name());
    match parent_name {
        Some(name) => log::warn!("En_Goma: childrenGohmaState[{index}] = {state} on its parent ({name}): Boss_Goma isn't ported"),
        None => log::warn!("En_Goma: childrenGohmaState[{index}] = {state} with no parent (the C writes through NULL)"),
    }
}

/// The env colour segment the larva's bakes take (`gDPSetEnvColor` before each limb).
const SEG_ENV: u8 = 0x0B;
/// The egg's scrolling texture (`gSPSegment(0x08, func_80094E78(...))`), and the boss limb's
/// render mode (`gSPSegment(0x08, EnGoma_NoBackfaceCullingDlist(...))`).
const SEG_08: u8 = 0x08;
const BAKE_EGG: &str = "En_Goma/egg";
const BAKE_LARVA: &str = "En_Goma/larva";
const BAKE_LARVA_BODY: &str = "En_Goma/larva_body";

/// `func_80094E78(gfxCtx, 0, y)`: `Gfx_TexScroll(gfxCtx, 0, y, 0, 0)`.
fn egg_scroll(y: u32) -> Vec<(u32, u32)> {
    oot_game::scene_table::gfx_tex_scroll(0, y, 0, 0)
}

/// The name of the bake of Queen Gohma's limb list `symbol` (`boss_limb_bake`).
pub fn boss_limb_bake_name(symbol: &str) -> String {
    format!("En_Goma/boss_limb/{symbol}")
}

/// `EnGoma_NoBackfaceCullingDlist` on segment 8, then `bossLimbDL` (`file`'s `symbol`): the bake
/// of one of Queen Gohma's pieces. `Boss_Goma`'s port (GAME-05 milestone 6) lists one per limb
/// list her `BossGoma_PostLimbDraw` hands over.
pub fn boss_limb_bake(file: &str, symbol: &str) -> MeshBake {
    use oot_game::gbi::*;
    /// `G_RM_AA_ZB_TEX_EDGE2` (`gbi.h`): `AA_EN | Z_CMP | Z_UPD | IM_RD | CVG_DST_CLAMP |
    /// CVG_X_ALPHA | ALPHA_CVG_SEL | ZMODE_OPA | GBL_c2(G_BL_CLR_IN, G_BL_A_IN, G_BL_CLR_MEM,
    /// G_BL_A_MEM)`.
    const G_RM_AA_ZB_TEX_EDGE2: u32 = 0x0044_3078;
    let mut d = Dl::default();
    d.pipe_sync();
    d.render_mode(G_RM_PASS, G_RM_AA_ZB_TEX_EDGE2);
    d.clear_geometry_mode(G_CULL_BACK);
    d.end();
    MeshBake {
        name: boss_limb_bake_name(symbol),
        object: file.into(),
        segments: vec![(SEG_08, BakeSegment::Commands(d.0))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![(file.into(), symbol.into())]),
    }
}

/// `EnGoma_Draw`'s meshes (after `Gfx_SetupDL_25Opa`):
/// - the egg: `gObjectGolEggDL`, its texture's scroll on segment 8 dynamic;
/// - the larva: `gObjectGolSkel` with the env colour (`EnGoma_OverrideLimbDraw`'s, before every
///   limb) dynamic, its body limb (`GOMA_LIMB_BODY`) left out; and the body's list,
///   `gObjectGolBodyDL`, alone with its own env colour (the random one while hurt).
///
/// The debris' `gBrownFragmentDL` (`gameplay_dangeon_keep`) is the pack's mesh.
pub fn bakes() -> Vec<MeshBake> {
    let env = || (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false });
    vec![
        MeshBake {
            name: BAKE_EGG.into(),
            object: OBJECT.into(),
            segments: vec![(SEG_08, BakeSegment::Dynamic(egg_scroll(0)))],
            prelude: Vec::new(),
            body: BakeBody::DLists(vec![(OBJECT.into(), "gObjectGolEggDL".into())]),
        },
        MeshBake {
            name: BAKE_LARVA.into(),
            object: OBJECT.into(),
            segments: vec![env()],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton {
                file: OBJECT.into(),
                symbol: "gObjectGolSkel".into(),
                limbs: vec![LimbOverride { limb: (GOMA_LIMB_BODY - 1) as u8, file: OBJECT.into(), symbol: String::new() }],
            },
        },
        MeshBake { name: BAKE_LARVA_BODY.into(), object: OBJECT.into(), segments: vec![env()], prelude: vec![SEG_ENV], body: BakeBody::DLists(vec![(OBJECT.into(), "gObjectGolBodyDL".into())]) },
    ]
}

/// `EnGoma_OverrideLimbDraw`'s change to the pose: the eyes' root turned by `eyePitch` and
/// `eyeYaw`.
fn larva_pose(skeleton: &Skeleton, joints: &[[i16; 3]], eye_pitch: i16, eye_yaw: i16) -> Vec<Mat4> {
    skeleton.pose_override(joints, |limb, _pos, rot| {
        if limb == GOMA_LIMB_EYE_IRIS_ROOT1 {
            rot[0] = rot[0].wrapping_add(eye_pitch);
            rot[1] = rot[1].wrapping_add(eye_yaw);
        }
        Mat4::IDENTITY
    })
}

/// `render_state`'s layout.
/// switches: `gomaType`, `eggTimer`, the hurt body's env colour (`0x01RRGGBB`, or 0); angles:
/// `slopePitch`, `slopeRoll`, `eyePitch`, `eyeYaw`; values:
mod rs {
    pub const EGG_SCALE: usize = 0;
    pub const EGG_PITCH: usize = 1;
    pub const EGG_SQUISH_ANGLE: usize = 2;
    pub const EGG_SQUISH_AMOUNT: usize = 3;
    pub const EGG_Y_OFFSET: usize = 4;
    pub const EYE_ENV: usize = 5;
}

impl ActorImpl for EnGoma {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnGoma_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let melee_weapon_state = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.melee_weapon_state()).unwrap_or(0);
        if self.action_timer != 0 {
            self.action_timer -= 1;
        }
        if self.invincibility_timer != 0 {
            self.invincibility_timer -= 1;
        }
        match self.action {
            Action::Flee => self.flee(play),
            Action::EggFallToGround => self.egg_fall_to_ground(play),
            Action::Egg => self.egg(play),
            Action::Hatch => self.hatch(play),
            Action::Hurt => self.hurt(play),
            Action::Die => self.die(play),
            Action::Dead => self.dead(play),
            Action::PrepareJump => self.prepare_jump(play),
            Action::Land => self.land(play),
            Action::Jump => self.jump(play),
            Action::Stand => self.stand(play),
            Action::ChasePlayer => self.chase_player(play),
            Action::Stunned => self.stunned(play),
            Action::Debris => self.debris(),
            Action::BossLimb => self.boss_limb(play),
            Action::None => log::warn!("En_Goma: params {} has no actionFunc (the C jumps to NULL)", self.actor.params),
        }
        self.actor.move_forward();
        self.actor.world_pos.x += self.shield_knockback_vel.x;
        self.actor.world_pos.z += self.shield_knockback_vel.z;
        approach_zero_f(&mut self.shield_knockback_vel.x, 1.0, 3.0);
        approach_zero_f(&mut self.shield_knockback_vel.z, 1.0, 3.0);
        if self.actor.params < 10 {
            self.egg_timer = self.egg_timer.wrapping_add(1);
            smooth_step_to_f(&mut self.actor.scale.x, 0.01, 0.5, 0.00075, 0.000001);
            smooth_step_to_f(&mut self.actor.scale.y, 0.01, 0.5, 0.00075, 0.000001);
            smooth_step_to_f(&mut self.actor.scale.z, 0.01, 0.5, 0.00075, 0.000001);
            self.update_hit(play);
            self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            self.set_floor_rot(play);
            self.actor.set_focus(20.0);
            self.look_at_player(play);
            self.update_eye_env_color();
            self.visual_state = 1;
            if melee_weapon_state != 0 {
                self.collider_cylinder2.dim.radius = 35;
                self.collider_cylinder2.dim.height = 35;
                self.collider_cylinder2.dim.y_shift = 0;
            } else {
                self.collider_cylinder2.dim.radius = 15;
                self.collider_cylinder2.dim.height = 30;
                self.collider_cylinder2.dim.y_shift = 10;
            }
            if self.invincibility_timer == 0 {
                self.collider_cylinder1.update(&self.actor);
                self.collider_cylinder2.update(&self.actor);
                play.collision_check_set_oc(&self.actor, COL_CYLINDER1, &mut self.collider_cylinder1);
                play.collision_check_set_ac(&self.actor, COL_CYLINDER2, &mut self.collider_cylinder2);
                play.collision_check_set_at(&self.actor, COL_CYLINDER1, &mut self.collider_cylinder1);
            }
        }
    }

    /// `EnGoma_Draw`'s effects on the actor, once per game frame: its Navi enemy id by its type
    /// (the larva's, the egg's), and the hurt larva's body colours: `EnGoma_OverrideLimbDraw`'s
    /// three `Rand_ZeroOne() × 255` (red, green, blue, as IDO evaluates the macro's operands) for
    /// `GOMA_LIMB_BODY` while `hurtTimer` runs.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.hurt_body_env = None;
        if self.actor.killed {
            return;
        }
        match self.goma_type {
            ENGOMA_NORMAL => {
                self.actor.navi_enemy_id = NAVI_ENEMY_GOHMA_LARVA;
                if self.skeleton.is_some() && self.hurt_timer != 0 {
                    let r = (play.rand.zero_one() * 255.0) as i16 as u8;
                    let g = (play.rand.zero_one() * 255.0) as i16 as u8;
                    let b = (play.rand.zero_one() * 255.0) as i16 as u8;
                    self.hurt_body_env = Some([r, g, b]);
                }
            }
            ENGOMA_EGG => self.actor.navi_enemy_id = NAVI_ENEMY_GOHMA_EGG,
            _ => {}
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let body_env = self.hurt_body_env.map(|c| 0x0100_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).unwrap_or(0);
        r.switches = vec![self.goma_type as u16 as u32, self.egg_timer as u16 as u32, body_env];
        r.values = vec![self.egg_scale, self.egg_pitch, self.egg_squish_angle, self.egg_squish_amount, self.egg_y_offset, self.eye_env_color[0], self.eye_env_color[1], self.eye_env_color[2]];
        r.angles = vec![self.slope_pitch, self.slope_roll, self.eye_pitch, self.eye_yaw];
        r
    }

    /// `EnGoma_Draw`:
    /// - the larva: its own matrix (the slope's tilt), the skeleton with the eyes' colours as its
    ///   env colour, the eyes turned to Link, and the body's random colours while hurt;
    /// - the egg: at `Actor_Draw`'s matrix, squashed and stretched (`eggScale`), squished about a
    ///   turning axis (`eggSquishAngle`, `eggSquishAmount`), moved by `eggYOffset` and rolled by
    ///   `eggPitch`; its texture scrolling with `eggTimer` (`sin(eggTimer × 5°) × 31.9 + 31`);
    /// - the debris: `gBrownFragmentDL` at `Actor_Draw`'s matrix;
    /// - a piece of Queen Gohma: `bossLimbDL` with back faces drawn.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [goma_type, egg_timer, body_env], v, [slope_pitch, slope_roll, eye_pitch, eye_yaw]) = (&rs.joints, rs.switches.as_slice(), rs.values.as_slice(), rs.angles.as_slice())
        else {
            return;
        };
        if v.len() < 8 {
            return;
        }
        let model = oot_game::play::actor_draw_matrix(rs);
        match *goma_type as u16 as i16 {
            ENGOMA_NORMAL => {
                let Some(skeleton) = &self.skeleton else { return };
                let m = Self::larva_matrix(rs, *slope_pitch, *slope_roll);
                let bones = larva_pose(skeleton, &joints.rot, *eye_pitch, *eye_yaw);
                // (s16)this->eyeEnvColor[i], 255.
                let e = rs::EYE_ENV;
                let eye_env = [v[e] as i16 as u8, v[e + 1] as i16 as u8, v[e + 2] as i16 as u8, 255];
                let body = if *body_env != 0 { [(*body_env >> 16) as u8, (*body_env >> 8) as u8, *body_env as u8, 255] } else { eye_env };
                let seg = |env: [u8; 4]| {
                    let mut sv = SegmentValues::default();
                    sv.env[SEG_ENV as usize] = Some(env);
                    DrawParams { segments: Some(sv), ..Default::default() }
                };
                let body_m = m * bones[GOMA_LIMB_BODY - 1];
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_LARVA)), transform: m, bones, params: seg(eye_env) });
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_LARVA_BODY)), transform: body_m, bones: Vec::new(), params: seg(body) });
            }
            ENGOMA_EGG => {
                let egg_timer = *egg_timer as u16 as i16;
                let y = (((egg_timer as f32 * 5.0 * 3.1415) / 180.0).sin() * 31.9) as i16 as i32;
                let y = (y + 31) as i16 as i32;
                let (scale, amount, angle) = (v[rs::EGG_SCALE], v[rs::EGG_SQUISH_AMOUNT], v[rs::EGG_SQUISH_ANGLE]);
                let m = model
                    * Mat4::from_scale(Vec3::new(scale, 1.0 / scale, scale))
                    * Mat4::from_rotation_y(angle * 0.15)
                    * Mat4::from_rotation_z(angle * 0.1)
                    * Mat4::from_scale(Vec3::new(0.95 - amount, amount + 1.05, 0.95 - amount))
                    * Mat4::from_rotation_z(-(angle * 0.1))
                    * Mat4::from_rotation_y(-(angle * 0.15))
                    * Mat4::from_translation(Vec3::new(0.0, v[rs::EGG_Y_OFFSET], 0.0))
                    * Mat4::from_rotation_x(v[rs::EGG_PITCH]);
                let mut sv = SegmentValues::default();
                sv.read(SEG_08, &egg_scroll(y as u32));
                out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_EGG)), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
            }
            ENGOMA_HATCH_DEBRIS => {
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh("gameplay_dangeon_keep", "gBrownFragmentDL")), model));
            }
            ENGOMA_BOSSLIMB => {
                if let Some((_, symbol)) = self.boss_limb_dl {
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&boss_limb_bake_name(symbol))), model));
                }
            }
            _ => {}
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER1 => Some(ColliderMut::Cylinder(&mut self.collider_cylinder1)),
            COL_CYLINDER2 => Some(ColliderMut::Cylinder(&mut self.collider_cylinder2)),
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
