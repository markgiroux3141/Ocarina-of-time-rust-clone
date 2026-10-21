//! `En_Weiyer` (`ovl_En_Weiyer/z_en_weiyer.c`): the Stinger in water, which swims about a pool
//! and darts at Link when he's in it, jumping out after him when he's at the surface (GAME-10
//! milestone 4). `object_ei`'s `gStingerSkel` (18 limbs), as `En_Eiyer`'s, smaller (scale 3 /
//! 1000 against 5).
//!
//! Jabu-Jabu's Belly (Master Quest, layers 0 to 3) places one in room 1 at (-119, -1246, -1691)
//! and two in room 13 at (-987, -1203, -2604) turned -7282 and (-871, -1163, -2316) turned
//! -23666; params 0 each (read by nothing).
//!
//! - **Placed** (`EnWeiyer_InitInsideWaterBox`, its first update): half a unit up, the floor
//!   under it; no water over it, or the surface's 5 under (its `home.pos.y`, kept every update as
//!   the water's surface over it less 5) at or under the floor, and it's gone; else halfway
//!   between the floor and that.
//! - **Swimming** (`EnWeiyer_FreeSwim`, `gStingerHitAnim` looped, AT off): bobbing 3 about a
//!   height (`swimHeight`) stepping 0.5 a frame to another (`targetSwimHeight`: from the floor at
//!   first; a random one now and then, or in the upper half off the floor, or in the lower at the
//!   surface), speeding up to 1 or 1.3, turning 182 a frame to a yaw (a wall's, or every 30
//!   frames a random 0x2000 to 0x4000 either way, a third of the time with `NA_SE_EN_EIER_CRY`).
//!   Breaking the surface it's held at it (a new height in the lower half off the floor); with
//!   the surface under its floor (the water gone) it **turns over** (`EnWeiyer_TurnAround`). Link
//!   within 400 across, 250 up or down and less than 20 over the surface, it **attacks**
//!   (`EnWeiyer_Attack`: AT on, the animation 3 times as fast, 200 frames): turning to him (a
//!   half, 0x200 at most) and pitching to his waist, speeding to 4 facing him (within 0x2800) or
//!   1.3; held under the surface (pitching to 0x1000 there). Link 20 over the surface, the 200
//!   frames out or a hit on him, it **backs off** (`EnWeiyer_Inactive`: 200 frames swimming away
//!   from him, in the lower quarter off the floor, AT off; then swimming again). Link under 50
//!   deep, it under 20 and facing him (within 0x2000), it **jumps out** (`EnWeiyer_OutOfWater`:
//!   pitched up 0x2000 at 5; out of the water (`depthInWater` under 0), a splash 400 and
//!   `NA_SE_EN_OCTAROCK_JUMP`, then 10 frames pitching at Link's waist (down only), then down at
//!   0x1800: back in the water pitched down a splash, `NA_SE_EN_OCTAROCK_SINK` and backing off; on
//!   the ground it turns over).
//! - **Turning over** (`EnWeiyer_TurnAround`, AT on, at 5): pitching up to 0x4000 (rolled twice
//!   its pitch), then turned round (yaw + 0x8000) pitching down to 0x1800: in the water (a splash
//!   if pitched down) it backs off; on the ground it's **stuck** (`EnWeiyer_StuckOnFloor`:
//!   `gStingerPopOutAnim` at 2, fluttering, slowing to 0; in the water again it swims, else after
//!   40 frames it turns over again).
//! - **Hit** (`EnWeiyer_UpdateDamage`): health 2. A Deku nut or the boomerang (reaction 1) **stuns**
//!   it (`EnWeiyer_Stunned`, no damage: blue 80 frames, `gStingerPopOutAnim`, falling (gravity -1,
//!   no pitch), its cylinder 15 taller, `NA_SE_EN_GOMA_JR_FREEZE`; the water gone (its surface
//!   under the floor) fluttering and `NA_SE_EN_DODO_M_GND` landing; then swimming); damage left
//!   it **hurt** (`EnWeiyer_Hurt`: red 40 frames, AT and AC off, at 3, rolling; then swimming, AC
//!   on); none left it **dies**
//!   (`EnWeiyer_Die`: the finishing blow, `NA_SE_EN_EIER_DEAD`, no lock-on, red 40 frames,
//!   pitching up at 3 and spinning, 20 frames or a ceiling), then **fades** (`EnWeiyer_Dead`:
//!   translucent at its shadow's alpha, down 5 and 2 lower a frame; gone, a drop from table 14).
//! - Its update moves it straight on (`Actor_MoveXZGravity`) unpitched or stunned, else along its
//!   pitch (`Actor_MoveXYZ`; `world.rot.x` is minus `shape.rot.x`); its yaw is the shape's. A hit
//!   on Link (AT) plays `NA_SE_EN_EIER_ATTACK` and turns AT off.
//!
//! @bug (game): `EnWeiyer_Hurt` turns `world.rot.y` away from Link (or a wall), but the update
//! sets `world.rot.y` from `shape.rot.y` after every action: hurt, it keeps straight on at 3.
//!
//! The whole overlay is ported. Its draw (`EnWeiyer_Draw`) is `gStingerSkel` (`SkelAnime_Draw`,
//! the root 2000 further forward, `EnWeiyer_OverrideLimbDraw`), opaque but dead, translucent then
//! at its shadow's alpha: `En_Eiyer`'s bakes.
//!
//! Params: none (placed 0).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_math::{scaled_step_to_s, smooth_step_to_s, step_to_f};
use glam::Vec3;
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::ActorSchema;
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE};
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, Anim, SkelAnimeStd};

use crate::en_eiyer::{Anims, NA_SE_EN_EIER_ATTACK, NA_SE_EN_EIER_CRY, NA_SE_EN_EIER_DEAD, NA_SE_EN_EIER_FLUTTER, NAVI_ENEMY_STINGER, OBJECT, draw_stinger, is_facing_player, pitch_toward_player, player_pos, skel_init};
use crate::en_okuta::{NA_SE_EN_OCTAROCK_JUMP, NA_SE_EN_OCTAROCK_SINK};

/// `ACTOR_EN_WEIYER` (`actor_table.h`: 0x018C).
pub const ACTOR_EN_WEIYER: i16 = 0x018C;

/// `En_Weiyer_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, `OBJECT_EI` (`object_table.h`:
/// 0x0026).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_WEIYER, name: "En_Weiyer", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT, schema: &SCHEMA };

/// En_Weiyer's configuration (docs/adr/0066-the-actor-schema.md): it reads nothing of its
/// placement.
pub const SCHEMA: ActorSchema = ActorSchema::NONE;

/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_14, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_14: i16 = 14 * 16;

/// `sCylinderInit`: hurting 8 when on (`ATELEM_SFX_HARD`), hit by Player's `0xFFCFFFFF`; 16 by 10,
/// 6 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 16, height: 10, y_shift: -6, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 2, 45 by 15, mass 100.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 2, cyl_radius: 45, cyl_height: 15, mass: 100 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
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
        dmg_entry(2, 0x0), // Fire arrow
        dmg_entry(2, 0x0), // Ice arrow
        dmg_entry(2, 0x0), // Light arrow
        dmg_entry(2, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(0, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(4, 0x0), // Giant spin
        dmg_entry(2, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(8, 0x0), // Giant jump
        dmg_entry(4, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(4, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `EnWeiyer_OverrideLimbDraw`: the root (limb 1) this much further forward.
pub const ROOT_FORWARD: f32 = 2000.0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    InitInsideWaterBox,
    FreeSwim,
    TurnAround,
    StuckOnFloor,
    Attack,
    Inactive,
    Hurt,
    Stunned,
    Die,
    Dead,
    OutOfWater,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::InitInsideWaterBox => "EnWeiyer_InitInsideWaterBox",
            Action::FreeSwim => "EnWeiyer_FreeSwim",
            Action::TurnAround => "EnWeiyer_TurnAround",
            Action::StuckOnFloor => "EnWeiyer_StuckOnFloor",
            Action::Attack => "EnWeiyer_Attack",
            Action::Inactive => "EnWeiyer_Inactive",
            Action::Hurt => "EnWeiyer_Hurt",
            Action::Stunned => "EnWeiyer_Stunned",
            Action::Die => "EnWeiyer_Die",
            Action::Dead => "EnWeiyer_Dead",
            Action::OutOfWater => "EnWeiyer_OutOfWater",
        }
    }
}

pub struct EnWeiyer {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    pub timer: i16,
    pub target_yaw: i16,
    pub swim_height: f32,
    pub target_swim_height: f32,
    pub collider: ColliderCylinder,
}

impl EnWeiyer {
    /// `EnWeiyer_Init`: the circle shadow 65 at 1000 up; placed on its first update.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: NAVI_ENEMY_STINGER, scale 3 / 1000, lockOnArrowOffset 2500.
        actor.navi_enemy_id = NAVI_ENEMY_STINGER;
        actor.scale = Vec3::splat(0.003);
        actor.target_arrow_offset = 2500.0;
        actor.shape_init(1000.0, oot_game::shadow::ShadowDraw::Circle, 65.0);
        let anims = Anims::load(play, PROFILE.name);
        let (skeleton, skel) = skel_init(play, PROFILE.name, anims.as_ref());
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        Box::new(EnWeiyer {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::InitInsideWaterBox,
            timer: 0,
            target_yaw: 0,
            swim_height: 0.0,
            target_swim_height: 0.0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
        })
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change`, looped from 0.
    fn change_loop(&mut self, f: impl Fn(&Anims) -> &Anim, play_speed: f32, morph: f32) {
        if let Some(a) = self.anim(f) {
            self.skel.change(a, play_speed, 0.0, 0.0, ANIMMODE_LOOP, morph);
        }
    }

    /// `EnWeiyer_SetupFreeSwim`: see the module's docs.
    pub fn setup_free_swim(&mut self) {
        self.target_yaw = self.actor.shape_rot.y;
        // cosf(-M_PI / 8): M_PI a float.
        self.swim_height = ((-std::f32::consts::PI / 8.0).cos() * 3.0) + self.actor.world_pos.y;
        // Animation_MorphToLoop.
        self.change_loop(|a| &a.hit, 1.0, -5.0);
        self.timer = 30;
        self.actor.speed_xz = self.actor.speed_xz.min(2.5);
        self.collider.base.at_flags &= !AT_ON;
        self.target_swim_height = self.actor.floor_height;
        self.action = Action::FreeSwim;
    }

    /// `EnWeiyer_SetupTurnAround`: AT on, at 5.
    pub fn setup_turn_around(&mut self) {
        self.change_loop(|a| &a.hit, 1.0, -5.0);
        self.collider.base.at_flags |= AT_ON;
        self.timer = 0;
        self.actor.speed_xz = 5.0;
        self.action = Action::TurnAround;
    }

    /// `EnWeiyer_SetupStuckOnFloor`: `gStingerPopOutAnim` at 2, 40 frames, AT on.
    pub fn setup_stuck_on_floor(&mut self) {
        self.change_loop(|a| &a.pop_out, 2.0, -8.0);
        self.timer = 40;
        self.collider.base.at_flags |= AT_ON;
        self.action = Action::StuckOnFloor;
    }

    /// `EnWeiyer_SetupAttack`: 200 frames, AT on, the animation at 3.
    pub fn setup_attack(&mut self) {
        self.timer = 200;
        self.collider.base.at_flags |= AT_ON;
        self.skel.play_speed = 3.0;
        self.action = Action::Attack;
    }

    /// `EnWeiyer_SetupInactive`: 200 frames away from Link, at 4 at most, AT off.
    pub fn setup_inactive(&mut self) {
        self.timer = 200;
        self.target_yaw = self.actor.yaw_towards_player.wrapping_add(-0x8000i16);
        self.swim_height = self.actor.world_pos.y;
        self.actor.speed_xz = self.actor.speed_xz.min(4.0);
        self.collider.base.at_flags &= !AT_ON;
        self.skel.play_speed = 1.0;
        self.action = Action::Inactive;
    }

    /// `EnWeiyer_SetupHurt`: see the module's docs.
    pub fn setup_hurt(&mut self) {
        self.change_loop(|a| &a.hit, 2.0, -3.0);
        self.timer = 40;
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.speed_xz = 3.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 40);
        self.collider.dim.height = CYLINDER_INIT.dim.height;
        self.action = Action::Hurt;
    }

    /// `EnWeiyer_SetupStunned`: see the module's docs.
    pub fn setup_stunned(&mut self, play: &mut PlayState) {
        self.change_loop(|a| &a.pop_out, 2.0, -8.0);
        self.timer = 80;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.gravity = -1.0;
        self.collider.dim.height = CYLINDER_INIT.dim.height + 15;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, 80);
        self.collider.base.at_flags &= !AT_ON;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
    }

    /// `EnWeiyer_SetupDie`: red 40 frames, AT and AC off, at 3.
    pub fn setup_die(&mut self) {
        self.change_loop(|a| &a.hit, 1.0, -5.0);
        self.timer = 20;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_OPA, 40);
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.speed_xz = 3.0;
        self.action = Action::Die;
    }

    /// `EnWeiyer_SetupDead`: still; drawn translucent (`colorFilterParams |= 0x2000`).
    pub fn setup_dead(&mut self) {
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.color_filter_params |= 0x2000;
        self.action = Action::Dead;
    }

    /// `EnWeiyer_SetupOutOfWater`: pitched up 0x2000 at 5, waiting to break the surface (`timer`
    /// -1).
    pub fn setup_out_of_water(&mut self) {
        self.actor.shape_rot.x = -0x2000;
        self.timer = -1;
        self.actor.speed_xz = 5.0;
        self.action = Action::OutOfWater;
    }

    /// A splash where it is (`EffectSsGSplash_Spawn(.., 1, 400)`).
    fn splash(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 400));
    }

    /// `EnWeiyer_InitInsideWaterBox`: see the module's docs.
    fn init_inside_water_box(&mut self, play: &mut PlayState) {
        self.actor.world_pos.y += 0.5;
        // BgCheck_EntityRaycastDown4.
        let (floor_height, floor_poly) = play.col.entity_raycast_down(self.actor.world_pos);
        self.actor.floor_height = floor_height;
        self.actor.floor_poly = floor_poly;
        let p = self.actor.world_pos;
        // BgCheck_GetWaterSurface (writing home.pos.y when there's water).
        let surface = play.col.water_surface(p.x, p.z, play.col.water_room);
        if let Some(y) = surface {
            self.actor.home_pos.y = y;
        }
        if surface.is_none() || (self.actor.home_pos.y - 5.0) <= self.actor.floor_height {
            self.actor.kill();
        } else {
            self.actor.home_pos.y -= 5.0;
            self.actor.world_pos.y = (self.actor.home_pos.y + self.actor.floor_height) / 2.0;
            self.setup_free_swim();
        }
    }

    /// The bob: 3 about `swimHeight` with the animation's frame
    /// (`cosf((curFrame - 5.0f) * (M_PI / 40)) * 3.0f`); speeding to 1 to frame 45, 1.3 after.
    fn bob(&mut self, cur_frame: f32) {
        self.actor.world_pos.y = self.swim_height - ((cur_frame - 5.0) * (std::f32::consts::PI / 40.0)).cos() * 3.0;
        if cur_frame <= 45.0 {
            step_to_f(&mut self.actor.speed_xz, 1.0, 0.03);
        } else {
            step_to_f(&mut self.actor.speed_xz, 1.3, 0.03);
        }
    }

    /// `EnWeiyer_FreeSwim`: see the module's docs.
    fn free_swim(&mut self, play: &mut PlayState) {
        self.skel.update();
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x800);
        let anim_restarted = self.skel.on_frame(0.0);
        let cur_frame = self.skel.cur_frame;
        step_to_f(&mut self.swim_height, self.target_swim_height, 0.5);
        self.bob(cur_frame);
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.target_yaw = self.actor.wall_yaw;
            self.timer = 30;
        }
        if scaled_step_to_s(&mut self.actor.shape_rot.y, self.target_yaw, 182) {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                let offset = play.rand.s16_offset(0x2000, 0x2000) as i32;
                let sign = if play.rand.zero_one() < 0.5 { -1 } else { 1 };
                self.target_yaw = (offset * sign + self.actor.shape_rot.y as i32) as i16;
                self.timer = 30;
                if play.rand.zero_one() < 0.3333 {
                    audio_play_actor_sfx2(play, NA_SE_EN_EIER_CRY);
                }
            }
        }
        let (home_y, floor) = (self.actor.home_pos.y, self.actor.floor_height);
        if home_y < self.actor.world_pos.y {
            if home_y < floor {
                self.setup_turn_around();
            } else {
                self.actor.world_pos.y = home_y;
                self.target_swim_height = play.rand.zero_one() * ((home_y - floor) / 2.0) + floor;
            }
        } else {
            let player = player_pos(play);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.target_swim_height = home_y - play.rand.zero_one() * ((home_y - floor) / 2.0);
            } else if anim_restarted && play.rand.zero_one() < 0.1 {
                self.target_swim_height = play.rand.zero_one() * (home_y - floor) + floor;
            }
            if self.actor.xz_dist_to_player < 400.0 && self.actor.y_dist_to_player.abs() < 250.0 && player.y < (home_y + 20.0) {
                self.setup_attack();
            }
        }
    }

    /// `EnWeiyer_TurnAround`: see the module's docs.
    fn turn_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer == 0 {
            if scaled_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 0x800) {
                self.actor.shape_rot.z = 0;
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(-0x8000i16);
                self.timer = 1;
            } else {
                self.actor.shape_rot.z = (self.actor.shape_rot.x as i32 * 2) as i16;
            }
        } else {
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0x1800, 0x800);
            if self.actor.world_pos.y < self.actor.home_pos.y {
                if self.actor.shape_rot.x > 0 {
                    self.splash(play);
                    audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_SINK);
                }
                self.setup_inactive();
            } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.setup_stuck_on_floor();
            }
        }
    }

    /// `EnWeiyer_StuckOnFloor`: fluttering, slowing; 40 frames, then turning over (back in the
    /// water first: swimming).
    fn stuck_on_floor(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_FLUTTER);
        }
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x800);
        step_to_f(&mut self.actor.speed_xz, 0.0, 1.0);
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.setup_turn_around();
        } else if self.actor.world_pos.y < self.actor.home_pos.y {
            self.setup_free_swim();
        }
    }

    /// `EnWeiyer_Attack`: see the module's docs.
    fn attack(&mut self, play: &mut PlayState) {
        let player_y = player_pos(play).y;
        let player_depth = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.y_dist_to_water).unwrap_or(0.0);
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 || (self.actor.home_pos.y + 20.0) <= player_y || self.collider.base.at_flags & AT_HIT != 0 {
            self.setup_inactive();
            return;
        }
        if is_facing_player(&self.actor, 0x2800) {
            step_to_f(&mut self.actor.speed_xz, 4.0, 0.2);
        } else {
            step_to_f(&mut self.actor.speed_xz, 1.3, 0.2);
        }
        if self.actor.home_pos.y < self.actor.world_pos.y {
            if self.actor.home_pos.y < self.actor.floor_height {
                self.actor.shape_rot.x = 0;
                self.setup_turn_around();
                return;
            }
            self.actor.world_pos.y = self.actor.home_pos.y;
            smooth_step_to_s(&mut self.actor.shape_rot.x, 0x1000, 2, 0x100, 0x40);
        } else {
            let pitch = pitch_toward_player(&self.actor, play);
            smooth_step_to_s(&mut self.actor.shape_rot.x, pitch, 2, 0x100, 0x40);
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 2, 0x200, 0x80);
        if player_depth < 50.0 && self.actor.y_dist_to_water < 20.0 && is_facing_player(&self.actor, 0x2000) {
            self.setup_out_of_water();
        }
    }

    /// `EnWeiyer_Inactive`: see the module's docs.
    fn inactive(&mut self) {
        self.skel.update();
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x800);
        let cur_frame = self.skel.cur_frame;
        let (home_y, floor) = (self.actor.home_pos.y, self.actor.floor_height);
        step_to_f(&mut self.swim_height, (home_y - floor) / 4.0 + floor, 1.0);
        self.bob(cur_frame);
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.target_yaw = self.actor.wall_yaw;
        }
        if smooth_step_to_s(&mut self.actor.shape_rot.y, self.target_yaw, 2, 0x200, 0x80) == 0 {
            self.target_yaw = self.actor.yaw_towards_player.wrapping_add(-0x8000i16);
        }
        if home_y < self.actor.world_pos.y {
            if home_y < floor {
                self.setup_turn_around();
            } else {
                self.actor.world_pos.y = home_y;
            }
        }
        if self.timer == 0 {
            self.setup_free_swim();
        }
    }

    /// `EnWeiyer_Hurt`: see the module's docs (the turn is lost: the module's `@bug`).
    fn hurt(&mut self) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.target_yaw = self.actor.wall_yaw;
        } else {
            self.target_yaw = self.actor.yaw_towards_player.wrapping_add(-0x8000i16);
        }
        scaled_step_to_s(&mut self.actor.world_rot.y, self.target_yaw, 0x38E);
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x200);
        // sinf(timer * (M_PI / 5)) * 5120.0f.
        self.actor.shape_rot.z = ((self.timer as f32 * (std::f32::consts::PI / 5.0)).sin() * 5120.0) as i32 as i16;
        if self.timer == 0 {
            self.actor.shape_rot.z = 0;
            self.collider.base.ac_flags |= AC_ON;
            self.setup_free_swim();
        }
    }

    /// `EnWeiyer_Die`: pitching up, spinning; 20 frames or a ceiling.
    fn die(&mut self) {
        self.skel.update();
        scaled_step_to_s(&mut self.actor.shape_rot.x, -0x4000, 0x400);
        self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1000);
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 || self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.setup_dead();
        }
    }

    /// `EnWeiyer_Dead`: fading 5 a frame and sinking 2; gone, a drop from table 14.
    fn dead(&mut self, play: &mut PlayState) {
        self.actor.shape_shadow_alpha = (self.actor.shape_shadow_alpha as i16 - 5).max(0) as u8;
        self.actor.world_pos.y -= 2.0;
        if self.actor.shape_shadow_alpha == 0 {
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_14);
            self.actor.kill();
        }
    }

    /// `EnWeiyer_Stunned`: 80 frames, then swimming; the water gone, fluttering.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        scaled_step_to_s(&mut self.actor.shape_rot.x, 0, 0x200);
        scaled_step_to_s(&mut self.actor.shape_rot.z, 0, 0x200);
        self.skel.update();
        if self.actor.home_pos.y < self.actor.floor_height {
            if self.skel.on_frame(0.0) {
                audio_play_actor_sfx2(play, NA_SE_EN_EIER_FLUTTER);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            }
        }
        if self.timer == 0 {
            self.actor.gravity = 0.0;
            self.actor.velocity.y = 0.0;
            self.collider.dim.height = CYLINDER_INIT.dim.height;
            self.setup_free_swim();
        }
    }

    /// `EnWeiyer_OutOfWater`: see the module's docs.
    fn out_of_water(&mut self, play: &mut PlayState) {
        self.skel.update();
        let player_is_out_of_water = (self.actor.home_pos.y + 20.0) <= player_pos(play).y;
        if self.timer == -1 {
            if player_is_out_of_water || self.collider.base.at_flags & AT_HIT != 0 {
                self.setup_inactive();
            } else if self.actor.y_dist_to_water < 0.0 {
                self.timer = 10;
                self.splash(play);
                audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_JUMP);
            }
        } else {
            if player_is_out_of_water || self.collider.base.at_flags & AT_HIT != 0 {
                self.timer = 0;
            } else if self.timer != 0 {
                self.timer -= 1;
            }
            let pitch = if self.timer == 0 { 0x1800 } else { pitch_toward_player(&self.actor, play).max(0) };
            if self.actor.shape_rot.x < pitch {
                scaled_step_to_s(&mut self.actor.shape_rot.x, pitch, 0x400);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.setup_turn_around();
            } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && self.actor.shape_rot.x > 0 {
                self.splash(play);
                audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_SINK);
                self.setup_inactive();
            } else {
                smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 8, 0x100, 0x80);
            }
        }
    }

    /// `EnWeiyer_UpdateDamage`: see the module's docs.
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let elem = self.collider.info;
        self.actor.set_drop_flag(&elem, true);
        let c = &self.actor.col_chk_info;
        if c.damage_reaction == 0 && c.damage == 0 {
            return;
        }
        if self.actor.col_chk_info.damage_reaction == 1 {
            if self.action != Action::Stunned {
                self.setup_stunned(play);
            }
        } else if self.actor.apply_damage() == 0 {
            enemy_start_finishing_blow(play, &self.actor);
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_DEAD);
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_die();
        } else {
            self.setup_hurt();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::InitInsideWaterBox => self.init_inside_water_box(play),
            Action::FreeSwim => self.free_swim(play),
            Action::TurnAround => self.turn_around(play),
            Action::StuckOnFloor => self.stuck_on_floor(play),
            Action::Attack => self.attack(play),
            Action::Inactive => self.inactive(),
            Action::Hurt => self.hurt(),
            Action::Stunned => self.stunned(play),
            Action::Die => self.die(),
            Action::Dead => self.dead(play),
            Action::OutOfWater => self.out_of_water(play),
        }
    }
}

impl ActorImpl for EnWeiyer {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnWeiyer_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.home_pos.y = self.actor.y_dist_to_water + self.actor.world_pos.y - 5.0;
        self.update_damage(play);
        self.run_action(play);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.actor.world_rot.x = self.actor.shape_rot.x.wrapping_neg();
        if self.actor.world_rot.x == 0 || self.action == Action::Stunned {
            self.actor.move_forward();
        } else {
            self.actor.move_xyz();
        }
        self.actor.update_bg_check_info(&play.col, 10.0, 30.0, 45.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        self.actor.set_focus(0.0);
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !(AT_ON | AT_HIT);
            audio_play_actor_sfx2(play, NA_SE_EN_EIER_ATTACK);
        }
        self.collider.update(&self.actor);
        if self.collider.base.at_flags & AT_ON != 0 {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        // `acFlags & AT_ON`: AT_ON and AC_ON are both 1 << 0.
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnWeiyer_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        r.switches = vec![(self.action == Action::Dead) as u32];
        r.values = vec![self.actor.shape_shadow_alpha as f32];
        r
    }

    /// `EnWeiyer_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(skeleton) = &self.skeleton else { return };
        let (Some(&dead), Some(&alpha)) = (rs.switches.first(), rs.values.first()) else { return };
        draw_stinger(skeleton, rs, ROOT_FORWARD, dead != 0, false, alpha.clamp(0.0, 255.0) as u8, out);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
