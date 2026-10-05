//! `En_Karebaba` (`ovl_En_Karebaba/z_en_karebaba.c`): the withered Deku Baba, a dead-looking
//! Deku Baba head on its leaves that springs up when Link comes near, sways upright, then spins
//! its head round on its stem; any sword or boomerang hit while it's up kills it, leaving a Deku
//! Stick; it grows back.
//!
//! Params: 0 starts by growing (`EnKarebaba_Grow`), anything else already grown
//! (`EnKarebaba_Idle`); `params` is then its timer.
//!
//! - **Idle**: small (scale 0.005) in its leaves until Link is within 200 across and 30 up or
//!   down.
//! - **Awaken** (`NA_SE_EN_DUMMY482`): rising 60 at 5 a frame, spinning, throwing dirt
//!   (`EffectSsHahen_SpawnBurst` at home); then **upright** (`EnKarebaba_SetupUpright`): full
//!   size, its body soft (`COL_MATERIAL_HIT6`, hit by swords and the boomerang), for 40 frames,
//!   then 40 of **spinning** its head round (`EnKarebaba_Spin`), alternately. Past 240 from
//!   Link it **retracts** to idle.
//! - **Hit** while up: it dies (`EnKarebaba_SetupDying`, the finishing blow's freeze): the head
//!   flies back, throwing dirt, and lands; then dust along the stem, and a Deku Stick lies 200
//!   frames to be picked up (`EnKarebaba_DeadItemDrop`, a misc actor meanwhile); then 200 frames
//!   gone (`EnKarebaba_Dead`) and it regrows (`EnKarebaba_Regrow`) over 20.
//!
//! Its colliders stay where `EnKarebaba_Init` put them (at home: the C never moves them): the
//! body's cylinder (AC) and the head's (AT and OC), their sizes changing with its state.
//! `DamageTable_Get(1)` is its table, though only `AC_HIT` is read.
//!
//! The whole overlay is ported. Not ported: the generic circle shadow (`ActorShadow_DrawCircle`,
//! not ported for any actor; its own shadow at home, `EnKarebaba_DrawCenterShadow`, is drawn).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::PolyId;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey};
use eng_math::{cos_s, scaled_step_to_s, sin_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_MISC, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::audio::sfx::*;
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::sys_matrix::MtxF;

pub const ACTOR_EN_KAREBABA: i16 = 0x00C7;
const OBJECT: &str = "object_dekubaba";

/// `En_Karebaba_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_KAREBABA, name: "En_Karebaba", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE, object: OBJECT };

/// `NAVI_ENEMY_WITHERED_DEKU_BABA` (`actor.h`).
const NAVI_ENEMY_WITHERED_DEKU_BABA: u8 = 0x09;

/// The colliders' ids (`collider_mut`): the body (AC), the head (AT and OC).
const COL_BODY: u8 = 0;
const COL_HEAD: u8 = 1;

/// `sBodyColliderInit`: hard, hit by everything but the shield and the mirror's ray; 7 by 25.
const BODY_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 7, height: 25, y_shift: 0, pos: [0, 0, 0] },
};

/// `sHeadColliderInit`: the bite (`0xFFCFFFFF` for 8, `ATELEM_SFX_HARD`) and the push; 4 by 25.
const HEAD_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 4, height: 25, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColCheckInfoInit`: 1 health, 15 by 80, heavy.
const COL_CHECK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 15, cyl_height: 80, mass: MASS_HEAVY };

/// `DamageTable_Get(1)` (`z_collision_btltbls.c`'s `sDamageTablePresets[1]`, "Used by
/// En_Karebaba").
pub static S_DAMAGE_TABLE_PRESET_1: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(1, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(0, 0xE), // Boomerang
        dmg_entry(1, 0x0), // Normal arrow
        dmg_entry(2, 0xF), // Hammer swing
        dmg_entry(0, 0x1), // Hookshot
        dmg_entry(1, 0xF), // Kokiri sword
        dmg_entry(2, 0xF), // Master sword
        dmg_entry(2, 0xF), // Giant's Knife
        dmg_entry(2, 0x2), // Fire arrow
        dmg_entry(1, 0x0), // Ice arrow
        dmg_entry(1, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(2, 0x2), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(2, 0x0), // Giant spin
        dmg_entry(2, 0x0), // Master spin
        dmg_entry(0, 0x0), // Kokiri jump
        dmg_entry(0, 0x0), // Giant jump
        dmg_entry(0, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Grow,
    Idle,
    Awaken,
    Upright,
    Spin,
    Dying,
    DeadItemDrop,
    Retract,
    Dead,
    Regrow,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Grow => "EnKarebaba_Grow",
            Action::Idle => "EnKarebaba_Idle",
            Action::Awaken => "EnKarebaba_Awaken",
            Action::Upright => "EnKarebaba_Upright",
            Action::Spin => "EnKarebaba_Spin",
            Action::Dying => "EnKarebaba_Dying",
            Action::DeadItemDrop => "EnKarebaba_DeadItemDrop",
            Action::Retract => "EnKarebaba_Retract",
            Action::Dead => "EnKarebaba_Dead",
            Action::Regrow => "EnKarebaba_Regrow",
        }
    }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

pub struct EnKarebaba {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    chomp: Option<Anim>,
    pub action: Action,
    /// `boundFloor`: its floor, the first one found (its shadow's).
    pub bound_floor: Option<PolyId>,
    pub body_collider: ColliderCylinder,
    pub head_collider: ColliderCylinder,
}

impl EnKarebaba {
    /// `EnKarebaba_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 2500, ATTENTION_RANGE_1, NAVI_ENEMY_WITHERED_DEKU_BABA.
        actor.target_arrow_offset = 2500.0;
        actor.target_mode = 1;
        actor.navi_enemy_id = NAVI_ENEMY_WITHERED_DEKU_BABA;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 22): the circle shadow isn't ported.
        actor.shape_y_offset = 0.0;
        let (skeleton, chomp) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, "gDekubabaHeadSkel").map_err(|e| log::error!("En_Karebaba: {e:#}")).ok();
                let c = a.animation(OBJECT, "gDekubabaChompAnim").map_err(|e| log::error!("En_Karebaba: {e:#}")).ok();
                (s, c)
            }
            None => (None, None),
        };
        // SkelAnime_Init(&gDekubabaHeadSkel, &gDekubabaChompAnim): DEKUBABA_HEAD_LIMB_MAX entries.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(7);
        let skel = SkelAnimeStd::init_flex(limbs, chomp.clone());
        let mut body_collider = ColliderCylinder::new(&BODY_INIT);
        body_collider.update(&actor);
        let mut head_collider = ColliderCylinder::new(&HEAD_INIT);
        head_collider.update(&actor);
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE_PRESET_1), &COL_CHECK_INFO_INIT);
        let mut b = EnKarebaba { actor, skel, skeleton, chomp, action: Action::Idle, bound_floor: None, body_collider, head_collider };
        if b.actor.params == 0 {
            b.setup_grow();
        } else {
            b.setup_idle();
        }
        Box::new(b)
    }

    /// `EnKarebaba_ResetCollider`: the body hard again (`DMG_DEFAULT`), both 25 high, the body 7
    /// across.
    fn reset_collider(&mut self) {
        self.body_collider.dim.radius = 7;
        self.body_collider.dim.height = 25;
        self.body_collider.base.col_type = COL_MATERIAL_HARD;
        self.body_collider.base.ac_flags |= AC_HARD;
        self.body_collider.info.ac_dmg_info.dmg_flags = DMG_DEFAULT;
        self.head_collider.dim.height = 25;
    }

    /// `EnKarebaba_SetupGrow`.
    fn setup_grow(&mut self) {
        self.actor.scale = Vec3::ZERO;
        self.actor.shape_rot.x = -0x4000;
        self.action = Action::Grow;
        self.actor.world_pos.y = self.actor.home_pos.y + 14.0;
    }

    /// `EnKarebaba_SetupIdle`.
    fn setup_idle(&mut self) {
        self.actor.scale = Vec3::splat(0.005);
        self.actor.shape_rot.x = -0x4000;
        self.action = Action::Idle;
        self.actor.world_pos.y = self.actor.home_pos.y + 14.0;
    }

    /// `EnKarebaba_SetupAwaken`.
    fn setup_awaken(&mut self, play: &mut PlayState) {
        if let Some(a) = self.chomp.clone() {
            let last = a.last_frame();
            self.skel.change(a, 4.0, 0.0, last, ANIMMODE_LOOP, -3.0);
        }
        audio_play_actor_sfx2(play, NA_SE_EN_DUMMY482);
        self.action = Action::Awaken;
    }

    /// `EnKarebaba_SetupUpright`: (not from spinning) full size, the body soft and hit by swords
    /// and the boomerang (child Link's: not the Master Sword's jump attack, which he hasn't), 15
    /// by 80; 40 frames.
    fn setup_upright(&mut self, play: &PlayState) {
        if self.action != Action::Spin {
            self.actor.scale = Vec3::splat(0.01);
            self.body_collider.base.col_type = COL_MATERIAL_HIT6;
            self.body_collider.base.ac_flags &= !AC_HARD;
            self.body_collider.info.ac_dmg_info.dmg_flags = if !play.save.adult { (DMG_SWORD | DMG_BOOMERANG) & !DMG_JUMP_MASTER } else { DMG_SWORD | DMG_BOOMERANG };
            self.body_collider.dim.radius = 15;
            self.body_collider.dim.height = 80;
            self.head_collider.dim.height = 80;
        }
        self.actor.params = 40;
        self.action = Action::Upright;
    }

    /// `EnKarebaba_SetupSpin`.
    fn setup_spin(&mut self) {
        self.actor.params = 40;
        self.action = Action::Spin;
    }

    /// `EnKarebaba_SetupDying`: thrown back off its stem at 3, up at 4, falling by 0.8.
    fn setup_dying(&mut self, play: &mut PlayState) {
        self.actor.params = 0;
        self.actor.gravity = -0.8;
        self.actor.velocity.y = 4.0;
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(i16::MIN);
        self.actor.speed_xz = 3.0;
        audio_play_actor_sfx2(play, NA_SE_EN_DEKU_JR_DEAD);
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED;
        self.action = Action::Dying;
    }

    /// `EnKarebaba_SetupDeadItemDrop`: the head now a Deku Stick lying at its landing, a misc
    /// actor, for 200 frames.
    fn setup_dead_item_drop(&mut self, play: &mut PlayState) {
        self.actor.scale = Vec3::splat(0.03);
        self.actor.shape_rot.x = self.actor.shape_rot.x.wrapping_sub(0x4000);
        self.actor.shape_y_offset = 1000.0;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        // (shape.shadowScale = 3.)
        // Actor_ChangeCategory(play, &play->actorCtx, &this->actor, ACTORCAT_MISC).
        self.actor.category = ACTORCAT_MISC;
        if let Some(me) = play.cur_actor {
            play.actors.change_category(me, ACTORCAT_MISC);
        }
        self.actor.params = 200;
        self.actor.flags &= !ACTOR_FLAG_DRAW_CULLING_DISABLED;
        self.action = Action::DeadItemDrop;
    }

    /// `EnKarebaba_SetupRetract`.
    fn setup_retract(&mut self) {
        if let Some(a) = self.chomp.clone() {
            let last = a.last_frame();
            self.skel.change(a, -3.0, last, 0.0, ANIMMODE_ONCE, -3.0);
        }
        self.reset_collider();
        self.action = Action::Retract;
    }

    /// `EnKarebaba_SetupDead`: gone (not drawn but its leaves) for 200 frames, back at home.
    fn setup_dead(&mut self) {
        if let Some(a) = self.chomp.clone() {
            self.skel.change(a, 0.0, 0.0, 0.0, ANIMMODE_ONCE, 0.0);
        }
        self.reset_collider();
        self.actor.shape_rot.x = -0x4000;
        self.actor.params = 200;
        self.actor.parent = None;
        // (shape.shadowScale = 0.)
        self.actor.world_pos = self.actor.home_pos;
        self.action = Action::Dead;
    }

    /// `EnKarebaba_SetupRegrow`.
    fn setup_regrow(&mut self) {
        self.actor.shape_y_offset = 0.0;
        // (shape.shadowScale = 22.)
        self.head_collider.dim.radius = HEAD_INIT.dim.radius;
        self.actor.scale = Vec3::ZERO;
        self.action = Action::Regrow;
    }

    /// `EnKarebaba_Grow`: 20 frames growing to the idle size.
    fn grow(&mut self) {
        self.actor.params += 1;
        let scale = self.actor.params as f32 * 0.05;
        self.actor.scale = Vec3::splat(0.005 * scale);
        self.actor.world_pos.y = self.actor.home_pos.y + (14.0 * scale);
        if self.actor.params == 20 {
            self.setup_idle();
        }
    }

    /// `EnKarebaba_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        if self.actor.xz_dist_to_player < 200.0 && self.actor.y_dist_to_player.abs() < 30.0 {
            self.setup_awaken(play);
        }
    }

    /// `EnKarebaba_SetupDying`'s and `EnKarebaba_Awaken`'s
    /// `EffectSsHahen_SpawnBurst(play, pos, 3, 0, 12, 5, count, HAHEN_OBJECT_DEFAULT, 10, NULL)`.
    fn hahen_burst(play: &mut PlayState, pos: Vec3, count: i16) {
        play.with_ss(|s| s.hahen_spawn_burst(pos, 3.0, 0, 12, 5, count, -1, 10, None));
    }

    /// `EnKarebaba_Awaken`: growing to full size and rising to 60 at 5 a frame, spinning; dirt
    /// at home every frame.
    fn awaken(&mut self, play: &mut PlayState) {
        self.skel.update();
        step_to_f(&mut self.actor.scale.x, 0.01, 0.0005);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 60.0, 5.0) {
            self.setup_upright(play);
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1999);
        Self::hahen_burst(play, self.actor.home_pos, 1);
    }

    /// `EnKarebaba_Upright`: chomping (`NA_SE_EN_DEKU_JR_MOUTH` at frames 0 and 12); hit, it
    /// dies; Link past 240 of home, it retracts; its 40 frames up, it spins.
    fn upright(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        self.skel.update();
        if self.actor.params != 0 {
            self.actor.params -= 1;
        }
        if self.skel.on_frame(0.0) || self.skel.on_frame(12.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_DEKU_JR_MOUTH);
        }
        if self.body_collider.base.ac_flags & AC_HIT != 0 {
            self.setup_dying(play);
            enemy_start_finishing_blow(play, &self.actor);
        } else if dist_xz(self.actor.home_pos, player_pos) > 240.0 {
            self.setup_retract();
        } else if self.actor.params == 0 {
            self.setup_spin();
        }
    }

    /// `EnKarebaba_Spin`: for 40 frames the head swings out on the stem (up to 10 × 0x100 down
    /// from upright, the bite 4 + 2 × that wide) and round (up to 10 × 0x2C0 a frame); hit, it
    /// dies; then upright again.
    fn spin(&mut self, play: &mut PlayState) {
        if self.actor.params != 0 {
            self.actor.params -= 1;
        }
        self.skel.update();
        if self.skel.on_frame(0.0) || self.skel.on_frame(12.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_DEKU_JR_MOUTH);
        }
        let mut value = 20 - self.actor.params as i32;
        value = 20 - value.abs();
        if value > 10 {
            value = 10;
        }
        self.head_collider.dim.radius = HEAD_INIT.dim.radius + (value * 2) as i16;
        self.actor.shape_rot.x = (0xC000 - (value * 0x100)) as i16;
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add((value * 0x2C0) as i16);
        self.actor.world_pos.y = (sin_s(self.actor.shape_rot.x) * -60.0) + self.actor.home_pos.y;
        let cos60 = cos_s(self.actor.shape_rot.x) * 60.0;
        self.actor.world_pos.x = (sin_s(self.actor.shape_rot.y) * cos60) + self.actor.home_pos.x;
        self.actor.world_pos.z = (cos_s(self.actor.shape_rot.y) * cos60) + self.actor.home_pos.z;
        if self.body_collider.base.ac_flags & AC_HIT != 0 {
            self.setup_dying(play);
            enemy_start_finishing_blow(play, &self.actor);
        } else if self.actor.params == 0 {
            self.setup_upright(play);
        }
    }

    /// `EnKarebaba_Dying`: the head flies, pitching over, throwing dirt; hitting the ground or a
    /// wall it shrinks to nothing in a burst of 15; on the ground (`NA_SE_EN_DODO_M_GND`), the
    /// next frame dust along the fallen stem and at home, and the stick.
    fn dying(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.1);
        if self.actor.params == 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.x, 0x4800, 0x71C);
            Self::hahen_burst(play, self.actor.world_pos, 1);
            if self.actor.scale.x > 0.005 && (self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 || self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0) {
                self.actor.scale = Vec3::ZERO;
                self.actor.speed_xz = 0.0;
                self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE);
                Self::hahen_burst(play, self.actor.world_pos, 15);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
                self.actor.params = 1;
            }
        } else if self.actor.params == 1 {
            let mut position = self.actor.world_pos;
            let r = self.actor.shape_rot;
            // rotation: the stem's step, z its height.
            let rz = sin_s(r.x) * 20.0;
            let rx = -20.0 * cos_s(r.x) * sin_s(r.y);
            let ry = -20.0 * cos_s(r.x) * cos_s(r.y);
            let home = self.actor.home_pos;
            play.with_ss(|ss| {
                for _ in 0..4 {
                    ss.func_800286cc(position, Vec3::ZERO, Vec3::ZERO, 500, 50);
                    position.x += rx;
                    position.y += rz;
                    position.z += ry;
                }
                ss.func_800286cc(home, Vec3::ZERO, Vec3::ZERO, 500, 100);
            });
            self.setup_dead_item_drop(play);
        }
    }

    /// `EnKarebaba_DeadItemDrop`: the stick offered (`GI_DEKU_STICKS_1`) until taken or its 200
    /// frames run out.
    fn dead_item_drop(&mut self, play: &mut PlayState) {
        if self.actor.params != 0 {
            self.actor.params -= 1;
        }
        if oot_game::get_item::actor_has_parent(&self.actor) || self.actor.params == 0 {
            self.setup_dead();
        } else {
            let a = self.actor.clone();
            oot_game::get_item::offer_get_item(play, &a, oot_game::item::GI_DEKU_STICKS_1);
        }
    }

    /// `EnKarebaba_Retract`: shrinking to the idle size and sinking to 14, spinning, throwing
    /// dirt at home.
    fn retract(&mut self, play: &mut PlayState) {
        self.skel.update();
        step_to_f(&mut self.actor.scale.x, 0.005, 0.0005);
        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 14.0, 5.0) {
            self.setup_idle();
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x1999);
        Self::hahen_burst(play, self.actor.home_pos, 1);
    }

    /// `EnKarebaba_Dead`.
    fn dead(&mut self) {
        self.skel.update();
        if self.actor.params != 0 {
            self.actor.params -= 1;
        }
        if self.actor.params == 0 {
            self.setup_regrow();
        }
    }

    /// `EnKarebaba_Regrow`: 20 frames growing; then an enemy again, idle.
    fn regrow(&mut self, play: &mut PlayState) {
        self.actor.params += 1;
        let scale_factor = self.actor.params as f32 * 0.05;
        self.actor.scale = Vec3::splat(0.005 * scale_factor);
        self.actor.world_pos.y = self.actor.home_pos.y + (14.0 * scale_factor);
        if self.actor.params == 20 {
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE;
            self.actor.category = ACTORCAT_ENEMY;
            if let Some(me) = play.cur_actor {
                play.actors.change_category(me, ACTORCAT_ENEMY);
            }
            self.setup_idle();
        }
    }

    /// `EnKarebaba_Draw`'s stem: its parts' matrices, from the head (`Matrix_Translate(world)`,
    /// the scale, `Matrix_RotateZYX(shape.rot.x, shape.rot.y, 0)`, each part 2000 further back).
    fn stem_matrices(&self, n: usize) -> Vec<MtxF> {
        let a = &self.actor;
        let scale = self.stem_scale();
        let mut m = MtxF::set_translate(a.world_pos.x, a.world_pos.y, a.world_pos.z);
        m.scale(scale, scale, scale);
        m.rotate_zyx(a.shape_rot.x, a.shape_rot.y, 0);
        let mut out = Vec::new();
        for _ in 0..n {
            m.translate(0.0, 0.0, -2000.0);
            out.push(m);
        }
        out
    }

    /// The stem's scale: `params × 0.0005` while growing, else 0.01.
    fn stem_scale(&self) -> f32 {
        if self.action == Action::Regrow || self.action == Action::Grow { self.actor.params as f32 * 0.0005 } else { 0.01 }
    }
}

/// `func_80026230(play, &black, 1, 2)`: black fog from 0 to `|cos(0x4000)| × 3000 + 1500`, the
/// withered look of its head, stem and leaves (`func_80026608` puts the scene's back).
fn withered_fog() -> eng_gfx::FogOverride {
    let abs_cos = cos_s(((0x8000 / 2) * 1) as i16).abs();
    oot_game::gbi::sp_fog_position([0, 0, 0, 0], 0, ((abs_cos * 3000.0) as i16 as i32) + 1500)
}

impl ActorImpl for EnKarebaba {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnKarebaba_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Grow => self.grow(),
            Action::Idle => self.idle(play),
            Action::Awaken => self.awaken(play),
            Action::Upright => self.upright(play),
            Action::Spin => self.spin(play),
            Action::Dying => self.dying(play),
            Action::DeadItemDrop => self.dead_item_drop(play),
            Action::Retract => self.retract(play),
            Action::Dead => self.dead(),
            Action::Regrow => self.regrow(play),
        }
        if self.action != Action::Dead {
            if self.action == Action::Dying {
                self.actor.move_forward();
                self.actor.update_bg_check_info(&play.col, 10.0, 15.0, 10.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
            } else {
                self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
                if self.bound_floor.is_none() {
                    self.bound_floor = self.actor.floor_poly;
                }
            }
            if self.action != Action::Dying && self.action != Action::DeadItemDrop {
                if self.action != Action::Regrow && self.action != Action::Grow {
                    play.collision_check_set_at(&self.actor, COL_HEAD, &mut self.head_collider);
                    play.collision_check_set_ac(&self.actor, COL_BODY, &mut self.body_collider);
                }
                play.collision_check_set_oc(&self.actor, COL_HEAD, &mut self.head_collider);
                self.actor.set_focus((self.actor.scale.x * 10.0) / 0.01);
                let height = self.actor.home_pos.y + 40.0;
                self.actor.focus_pos.x = self.actor.home_pos.x;
                self.actor.focus_pos.y = self.actor.focus_pos.y.min(height);
                self.actor.focus_pos.z = self.actor.home_pos.z;
            }
        }
    }

    /// `EnKarebaba_Draw`'s effect on the actor: while it's dying, the focus follows the stem's
    /// first part (`Matrix_MultVec3f(&zeroVec, &this->actor.focus.pos)`).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.action == Action::Dying {
            let m = self.stem_matrices(1)[0];
            self.actor.focus_pos = Vec3::new(m.xw, m.yw, m.zw);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.actor.home_rot.y];
        rs.values = vec![self.actor.home_pos.x, self.actor.home_pos.y, self.actor.home_pos.z, self.stem_scale()];
        let action = match self.action {
            Action::DeadItemDrop => 1,
            Action::Dead => 2,
            Action::Dying => 3,
            Action::Grow => 4,
            _ => 0,
        };
        rs.switches = vec![action, self.actor.params as u16 as u32];
        rs
    }

    /// `EnKarebaba_Draw`: as a stick, the stick (blinking for its last 40 frames); else, unless
    /// gone, the head (`SkelAnime_DrawOpa`) and the stem's three parts (two while dying), black
    /// fogged; then the leaves at home (and the dying stem's lower part), black fogged; and its
    /// shadow at home (`EnKarebaba_DrawCenterShadow`).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), [home_rot_y], [hx, hy, hz, stem_scale], [action, params]) = (&rs.joints, rs.angles.as_slice(), rs.values.as_slice(), rs.switches.as_slice()) else {
            return;
        };
        let home = Vec3::new(*hx, *hy, *hz);
        let fog = DrawParams { fog: Some(withered_fog()), ..Default::default() };
        let mesh = |name: &str| MeshKey::named(keys::mesh(OBJECT, name));
        let model = oot_game::play::actor_draw_matrix(rs);
        let params = *params as u16 as i16;
        match *action {
            1 => {
                // EnKarebaba_DeadItemDrop: params > 40, or odd.
                if params > 40 || params & 1 != 0 {
                    out.opa.push(DrawCmd::new(mesh("gDekubabaDekuStickDL"), model * Mat4::from_translation(Vec3::new(0.0, 0.0, 200.0))));
                }
            }
            2 => {}
            _ => {
                if let Some(skeleton) = &self.skeleton {
                    let bones = skeleton.pose(joints);
                    out.opa.push(DrawCmd { mesh: mesh("gDekubabaHeadSkel"), transform: model, bones, params: fog.clone() });
                }
                let dying = *action == 3;
                let n = if dying { 2 } else { 3 };
                let stems = ["gDekubabaStem0DL", "gDekubabaStem1DL", "gDekubabaStem2DL"];
                let rot = |pitch: i16, yaw: i16| Mat4::from_rotation_y(eng_math::binang_to_rad(yaw)) * Mat4::from_rotation_x(eng_math::binang_to_rad(pitch));
                let mut m = Mat4::from_translation(rs.pos) * Mat4::from_scale(Vec3::splat(*stem_scale)) * rot(rs.rot[0], rs.rot[1]);
                for stem in stems.iter().take(n) {
                    m *= Mat4::from_translation(Vec3::new(0.0, 0.0, -2000.0));
                    out.opa.push(DrawCmd { mesh: mesh(stem), transform: m, bones: Vec::new(), params: fog.clone() });
                }
            }
        }
        // The leaves: at Grow the stem's scale, else 0.01.
        let scale = if *action == 4 { *stem_scale } else { 0.01 };
        let leaves = Mat4::from_translation(home) * Mat4::from_scale(Vec3::splat(scale)) * Mat4::from_rotation_y(eng_math::binang_to_rad(*home_rot_y));
        out.opa.push(DrawCmd { mesh: mesh("gDekubabaBaseLeavesDL"), transform: leaves, bones: Vec::new(), params: fog.clone() });
        if *action == 3 {
            let m = leaves * Mat4::from_rotation_y(eng_math::binang_to_rad(rs.rot[1].wrapping_sub(*home_rot_y))) * Mat4::from_rotation_x(eng_math::binang_to_rad(-0x4000));
            out.opa.push(DrawCmd { mesh: mesh("gDekubabaStem2DL"), transform: m, bones: Vec::new(), params: fog });
        }
        // EnKarebaba_DrawCenterShadow: gCircleShadowDL (radius 74) on the floor's plane at home
        // (func_80038A28), scaled by 0.15; drawn as the builtin disc.
        if let Some(f) = self.bound_floor {
            let n = play.col.poly(f).normal;
            let r = 74.0 * 0.15;
            let m = crate::en_dekubaba::floor_matrix([n[0], n[1], n[2]], home) * Mat4::from_scale(Vec3::new(r, 1.0, r));
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BODY => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            COL_HEAD => Some(ColliderMut::Cylinder(&mut self.head_collider)),
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
