//! `En_Crow` (`ovl_En_Crow/z_en_crow.c`): the Guay, a crow circling its home in the sky that
//! dives at Link. Lon Lon Ranch's night layers place fifteen; Hyrule Field and Kakariko have
//! theirs.
//!
//! - **Idle** (`EnCrow_FlyIdle`): flying at 3 to 4.5, turning back home from beyond 300 across
//!   (off walls), and on a flap's start a chance (1 in 10, once settled) to turn a little more
//!   towards home with a cry (`NA_SE_EN_KAICHO_CRY`), or to pitch up or down by its height against
//!   home (kept within 50 of it); after its 100 frames, with Link within 300 across, not riding
//!   (`PLAYER_STATE1_23`), the Guay out of water (`depthInWater < -40`) and no Skull Mask, it
//!   **dives** (`EnCrow_DiveAttack`).
//! - **Diving** (`EnCrow_DiveAttack`): for 300 frames at 4, wings at double speed, pitched at
//!   Link (20 up, at most 0x3000) while facing him, turning to him; its sphere's AT (8 damage)
//!   hits him. A hit (`NA_SE_EN_KAICHO_ATTACK`), its time out, the ground or a wall, water or Link
//!   riding (or a Skull Mask): idle again.
//! - **Hit** (`EnCrow_UpdateDamage`, `sDamageTable`): a Deku Nut (reaction 1) **turns it away**
//!   (`EnCrow_TurnAway`: blue for 5 frames, `NA_SE_EN_GOMA_JR_FREEZE`, 100 frames climbing away
//!   from Link at 3.5). Anything else with damage or a reaction kills it at 1 health
//!   (`Actor_ApplyDamage`, `Enemy_StartFinishingBlow`, no more targeting): it's **damaged**
//!   (`EnCrow_SetupDamaged`, `NA_SE_EN_KAICHO_DEAD`): red (ice: blue, eight pieces of ice
//!   `EffectSsEnIce_SpawnFlyingVec3f`; fire: red, four flames `EffectSsEnFire_SpawnVec3f` on its
//!   body parts) for 40 frames, falling under gravity and, red, spinning nose down
//!   (`EnCrow_Damaged`); on the ground (or over no floor) a puff of smoke (`EffectSsDeadDb`) and it
//!   **dies** (`EnCrow_Die`): shrinking (0.002 a frame, a big one 0.006) to nothing, then its drop
//!   (`Item_DropCollectibleRandom` from table 0, a big one a red rupee) and it **respawns**
//!   (`EnCrow_Respawn`): undrawn at home for 300 frames, then growing back (a tenth of its size a
//!   frame), targetable again, idle.
//! - **The tenth** (`sDeathCount`, the overlay's static, shared by every Guay): each normal one's
//!   death counts one; every init resets it. The respawn after the tenth makes that Guay **big**
//!   (`params` 1, scale 0.03, its sphere 59 instead of 20, a red rupee) and starts the count again.
//!   (59: `20 × 0.03f × 100.0f` in floats is 59.999996, truncated.)
//! - **Drawn** (`EnCrow_Draw`): `gGuaySkel` flapping (`gGuayFlyAnim`), its wings (limbs 7 and 8)
//!   beating harder while alive (`EnCrow_OverrideLimbDraw`); `EnCrow_PostLimbDraw` keeps its four
//!   body parts' places (the head 2500 out, limbs 4, 6, 8; 20 down) for the fire's flames. Its
//!   collider sphere is placed by the update (on its position, 20 × scale up while alive), not by
//!   the draw. The circle shadow (20) is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`
//!   while `actor.draw` is set).
//!
//! The whole overlay is ported. Never reached here: arrows (`ACTOR_FLAG_ATTACHED_TO_ARROW` is never
//! set: no arrow attaches) and the Skull Mask (`Player_GetMask` is `currentMask`, which nothing
//! sets to `PLAYER_MASK_SKULL`).
//!
//! Params: `params` whole: 0 a Guay, 1 a big one; every placement is 0, and the code writes it
//! (`EnCrow_SetupRespawn`: 1 after the tenth death, else 0).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_s, cos_s, scaled_step_to_s, smooth_step_to_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};
use oot_game::target::{pitch_to, yaw_to};

use crate::en_item00::{ITEM00_RUPEE_RED, item_drop_collectible, item_drop_collectible_random};

/// `ACTOR_EN_CROW` (`actor_table.h`: 0x01C0).
pub const ACTOR_EN_CROW: i16 = 0x01C0;
/// `OBJECT_CROW` (`object_table.h`: 0x0008).
pub const OBJECT: &str = "object_crow";
const SKEL: &str = "gGuaySkel";
const FLY_ANIM: &str = "gGuayFlyAnim";
/// `gGuaySkel` after `Gfx_SetupDL_25Opa` (`EnCrow_Draw`).
pub const BAKE: &str = "En_Crow/skel";

/// `ACTOR_FLAG_IGNORE_QUAKE`, `ACTOR_FLAG_CAN_ATTACH_TO_ARROW`, `ACTOR_FLAG_ATTACHED_TO_ARROW`
/// (`actor.h`).
const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;
const ACTOR_FLAG_CAN_ATTACH_TO_ARROW: u32 = 1 << 14;
const ACTOR_FLAG_ATTACHED_TO_ARROW: u32 = 1 << 15;

/// `En_Crow_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_CROW,
    name: "En_Crow",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_IGNORE_QUAKE | ACTOR_FLAG_CAN_ATTACH_TO_ARROW,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: a big Guay (`!= 0`) or not, read by `EnCrow_Die` and `EnCrow_Respawn`.
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(0, "a Guay"), (1, "a big Guay")]), "a Guay or a big one (scale 0.03, a red rupee); placed 0, the respawn after the tenth death writes 1");

/// En_Crow's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false };

/// `NAVI_ENEMY_GUAY` (`actor.h`: 0x58).
const NAVI_ENEMY_GUAY: u8 = 0x58;

/// `PLAYER_STATE1_23` (`player.h`): riding.
const PLAYER_STATE1_23: u32 = 1 << 23;
/// `PLAYER_MASK_SKULL` (`player.h`: 0x02).
const PLAYER_MASK_SKULL: u8 = 0x02;

/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_0, false)` (`z_en_item00.h`: table × 16).
const DROP_TABLE_0: i16 = 0;

// Sound effects (`sfx/enemybank_table.h`).
pub const NA_SE_EN_KAICHO_FLUTTER: u16 = 0x3897;
pub const NA_SE_EN_KAICHO_CRY: u16 = 0x38B6;
pub const NA_SE_EN_KAICHO_ATTACK: u16 = 0x38B7;
pub const NA_SE_EN_KAICHO_DEAD: u16 = 0x38EB;
pub const NA_SE_EN_GOMA_JR_FREEZE: u16 = oot_game::audio::sfx::NA_SE_EN_GOMA_JR_FREEZE;

/// `sZeroVecAccel`.
const S_ZERO_VEC_ACCEL: Vec3 = Vec3::ZERO;
/// `sHeadVec`: the head's body part, along limb 2.
const S_HEAD_VEC: Vec3 = Vec3::new(2500.0, 0.0, 0.0);

/// `EnCrow`'s damage reactions (`sDamageTable`'s effects): 1 a Deku Nut, 2 fire, 3 ice.
pub const EN_CROW_DMG_REACT_NUT: u8 = 1;
pub const EN_CROW_DMG_REACT_FIRE: u8 = 2;
pub const EN_CROW_DMG_REACT_ICE: u8 = 3;

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
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
        dmg_entry(4, 0x2), // Fire arrow
        dmg_entry(2, 0x3), // Ice arrow
        dmg_entry(2, 0x0), // Light arrow
        dmg_entry(4, 0x0), // Unk arrow 1
        dmg_entry(2, 0x0), // Unk arrow 2
        dmg_entry(2, 0x0), // Unk arrow 3
        dmg_entry(4, 0x2), // Fire magic
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

/// `sJntSphElementsInit`: one sphere (limb 1, radius 20, scale 100), its bite 8 (`0xFFCFFFFF`,
/// `ATELEM_SFX_HARD`), hit by everything but the shield and the mirror's ray.
pub fn jnt_sph_elements() -> [ColliderJntSphElementInit; 1] {
    [ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 1,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
        scale: 100,
    }]
}

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT3, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sJntSphInit.elements[0].dim.modelSphere.radius`.
const MODEL_RADIUS: i16 = 20;

/// `sColChkInfoInit`: 1 health, 15 by 30, mass 30.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 1, cyl_radius: 15, cyl_height: 30, mass: 30 };

/// `ActorShape_Init(&shape, 2000.0f, ActorShadow_DrawCircle, 20.0f)`.
const SHAPE_Y_OFFSET: f32 = 2000.0;

/// The overlay's statics (docs/adr/0059-castle-towns-actors.md): `sDeathCount`, the normal
/// Guays' deaths since the last init or big one.
#[derive(Debug, Default)]
pub struct Statics {
    pub s_death_count: u32,
}

/// The statics, zeroed with a fresh overlay (`static u32 sDeathCount = 0`).
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_CROW)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    FlyIdle,
    DiveAttack,
    Damaged,
    Die,
    TurnAway,
    Respawn,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::FlyIdle => "EnCrow_FlyIdle",
            Action::DiveAttack => "EnCrow_DiveAttack",
            Action::Damaged => "EnCrow_Damaged",
            Action::Die => "EnCrow_Die",
            Action::TurnAway => "EnCrow_TurnAway",
            Action::Respawn => "EnCrow_Respawn",
        }
    }
}

/// `EnCrow` (`z_en_crow.h`).
pub struct EnCrow {
    pub actor: Actor,
    /// `bodyPartsPos`: the head (2500 along limb 2) and limbs 4, 6 and 8, each 20 down, from the
    /// draw (`EnCrow_PostLimbDraw`); the fire's flames follow them.
    pub body_parts_pos: [Vec3; 4],
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    fly: Option<Anim>,
    pub action: Action,
    pub timer: i16,
    pub aim_rot_x: i16,
    pub aim_rot_y: i16,
    pub collider: ColliderJntSph,
    /// `actor.draw != NULL` (`EnCrow_SetupRespawn` takes the draw away until it grows back).
    pub drawn: bool,
}

/// `(s16)f`: a float to `s16` as the MIPS does it (truncated to `s32`, the low 16 bits).
fn f_to_s16(f: f32) -> i16 {
    f as i32 as i16
}

/// `Actor_WorldDistXZToPoint`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `Actor_IsFacingPlayer`.
fn is_facing_player(a: &Actor, max_angle: i16) -> bool {
    (a.yaw_towards_player.wrapping_sub(a.shape_rot.y) as i32).abs() < max_angle as i32
}

impl EnCrow {
    /// `EnCrow_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: cullingVolumeScale 3000, NAVI_ENEMY_GUAY, gravity -0.2, lockOnArrowOffset 2000.
        actor.culling_volume_scale = 3000.0;
        actor.navi_enemy_id = NAVI_ENEMY_GUAY;
        actor.gravity = -0.2;
        actor.target_arrow_offset = 2000.0;
        let (skeleton, fly) = match play.assets.clone() {
            Some(a) => {
                let s = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Crow: {e:#}")).ok();
                let f = a.animation(OBJECT, FLY_ANIM).map_err(|e| log::error!("En_Crow: {e:#}")).ok();
                (s, f)
            }
            None => (None, None),
        };
        // SkelAnime_InitFlex(&gGuaySkel, &gGuayFlyAnim, jointTable, morphTable, 9): 8 limbs.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(8);
        let skel = SkelAnimeStd::init_flex(limbs, fly.clone());
        let mut collider = ColliderJntSph::new(&JNT_SPH_INIT, &jnt_sph_elements());
        collider.elements[0].dim.world_sphere.radius = MODEL_RADIUS;
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        actor.shape_init(SHAPE_Y_OFFSET, oot_game::shadow::ShadowDraw::Circle, 20.0);
        let mut this = EnCrow {
            actor,
            body_parts_pos: [Vec3::ZERO; 4],
            skel,
            skeleton,
            fly,
            action: Action::FlyIdle,
            timer: 0,
            aim_rot_x: 0,
            aim_rot_y: 0,
            collider,
            drawn: true,
        };
        statics(play).s_death_count = 0;
        this.setup_fly_idle();
        Box::new(this)
    }

    /// `EnCrow_SetupFlyIdle`: 100 frames, hittable, flapping at speed 1.
    fn setup_fly_idle(&mut self) {
        self.timer = 100;
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::FlyIdle;
        self.skel.play_speed = 1.0;
    }

    /// `EnCrow_SetupDiveAttack`: 300 frames at 4, flapping at double speed.
    fn setup_dive_attack(&mut self) {
        self.timer = 300;
        self.actor.speed_xz = 4.0;
        self.skel.play_speed = 2.0;
        self.action = Action::DiveAttack;
    }

    /// `&this->actor` for an effect's spawn.
    fn ss_actor(&self, play: &PlayState) -> Option<oot_game::effect::SsActor> {
        let r = self.actor.shape_rot;
        play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] })
    }

    /// `EnCrow_SetupDamaged`: its forward speed kept, its flap slowed (0.4), raised to its middle
    /// (20 × scale), off the ground; `NA_SE_EN_KAICHO_DEAD`; blue with ice (eight pieces on a cube's
    /// corners, 7 × scale out), red with fire (four flames) or else; no more hits; always updated.
    fn setup_damaged(&mut self, play: &mut PlayState) {
        self.actor.speed_xz *= cos_s(self.actor.world_rot.x);
        self.actor.velocity.y = 0.0;
        if let Some(a) = self.fly.clone() {
            self.skel.change(a, 0.4, 0.0, 0.0, ANIMMODE_LOOP_INTERP, -3.0);
        }
        let scale = self.actor.scale.x * 100.0;
        self.actor.world_pos.y += 20.0 * scale;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        self.actor.shape_y_offset = 0.0;
        self.actor.target_arrow_offset = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_KAICHO_DEAD);

        let me = self.ss_actor(play);
        if self.actor.col_chk_info.damage_reaction == EN_CROW_DMG_REACT_ICE {
            // Ice arrows.
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 40);
            for i in 0..8 {
                let p = self.actor.world_pos;
                let ice_particle_pos = Vec3::new(
                    ((if i & 1 != 0 { 7.0 } else { -7.0 }) * scale) + p.x,
                    ((if i & 2 != 0 { 7.0 } else { -7.0 }) * scale) + p.y,
                    ((if i & 4 != 0 { 7.0 } else { -7.0 }) * scale) + p.z,
                );
                let s = ((play.rand.zero_one() * 0.15) + 0.85) * scale;
                play.effect_ss_en_ice_spawn_flying_vec3f(me, ice_particle_pos, [150, 150, 150, 250], [235, 245, 255], s);
            }
        } else if self.actor.col_chk_info.damage_reaction == EN_CROW_DMG_REACT_FIRE {
            // Fire arrows and Din's Fire.
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 40);
            for i in 0..4 {
                play.effect_ss_en_fire_spawn_vec3f(me, self.actor.world_pos, f_to_s16(50.0 * scale), 0, 0, i);
            }
        } else {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 40);
        }

        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW != 0 {
            self.actor.speed_xz = 0.0;
        }

        self.collider.base.ac_flags &= !AC_ON;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.action = Action::Damaged;
    }

    /// `EnCrow_SetupDie`.
    fn setup_die(&mut self) {
        self.actor.color_filter_timer = 0;
        self.action = Action::Die;
    }

    /// `EnCrow_SetupTurnAway`: 100 frames at 3.5 climbing (-0x1000) away from Link, flapping at
    /// double speed, blue for 5 frames.
    fn setup_turn_away(&mut self, play: &mut PlayState) {
        self.timer = 100;
        self.actor.speed_xz = 3.5;
        self.aim_rot_x = -0x1000;
        self.aim_rot_y = self.actor.yaw_towards_player.wrapping_add(0x8000u16 as i16);
        self.skel.play_speed = 2.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, 5);
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::TurnAway;
    }

    /// `EnCrow_SetupRespawn`: after the tenth death a big one (params 1, its sphere 59) and the
    /// count from 0, else a normal one; back at home level, undrawn, for 300 frames.
    fn setup_respawn(&mut self, play: &mut PlayState) {
        let s = statics(play);
        if s.s_death_count == 10 {
            // schema: the code writes its params (a big Guay).
            self.actor.params = 1;
            s.s_death_count = 0;
            // 20 × 0.03f = 0.59999996, × 100.0f = 59.999996: 59 as an s16.
            self.collider.elements[0].dim.world_sphere.radius = (MODEL_RADIUS as f32 * 0.03 * 100.0) as i32 as i16;
        } else {
            // schema: the code writes its params (a normal Guay).
            self.actor.params = 0;
            self.collider.elements[0].dim.world_sphere.radius = MODEL_RADIUS;
        }

        // Animation_PlayLoop(&gGuayFlyAnim).
        if let Some(a) = self.fly.clone() {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
        self.actor.world_pos = self.actor.home_pos;
        self.actor.shape_rot.x = 0;
        self.actor.shape_rot.z = 0;
        self.timer = 300;
        self.actor.shape_y_offset = SHAPE_Y_OFFSET;
        self.actor.target_arrow_offset = 2000.0;
        self.drawn = false;
        self.action = Action::Respawn;
    }

    /// `EnCrow_FlyIdle`.
    fn fly_idle(&mut self, play: &mut PlayState) {
        let (state1, mask) = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| (p.state_flags1(), p.current_mask())).unwrap_or((0, 0));

        self.skel.update();
        let skelanime_updated = self.skel.on_frame(0.0);
        self.actor.speed_xz = (play.rand.zero_one() * 1.5) + 3.0;

        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.aim_rot_y = self.actor.wall_yaw;
        } else if dist_xz(self.actor.world_pos, self.actor.home_pos) > 300.0 {
            self.aim_rot_y = yaw_to(self.actor.world_pos, self.actor.home_pos);
        }

        if smooth_step_to_s(&mut self.actor.shape_rot.y, self.aim_rot_y, 5, 0x300, 0x10) == 0 && skelanime_updated && play.rand.zero_one() < 0.1 {
            let var = yaw_to(self.actor.world_pos, self.actor.home_pos).wrapping_sub(self.actor.shape_rot.y);
            if var > 0 {
                self.aim_rot_y = f_to_s16(self.aim_rot_y as f32 + (4096.0 + (4096.0 * play.rand.zero_one())));
            } else {
                self.aim_rot_y = f_to_s16(self.aim_rot_y as f32 - (4096.0 + (4096.0 * play.rand.zero_one())));
            }
            audio_play_actor_sfx2(play, NA_SE_EN_KAICHO_CRY);
        }

        if self.actor.y_dist_to_water > -40.0 {
            self.aim_rot_x = -0x1000;
        } else if self.actor.world_pos.y < (self.actor.home_pos.y - 50.0) {
            self.aim_rot_x = f_to_s16(-2048.0 - (play.rand.zero_one() * 2048.0));
        } else if self.actor.world_pos.y > (self.actor.home_pos.y + 50.0) {
            self.aim_rot_x = f_to_s16(2048.0 + (play.rand.zero_one() * 2048.0));
        }

        if smooth_step_to_s(&mut self.actor.shape_rot.x, self.aim_rot_x, 10, 0x100, 8) == 0 && skelanime_updated && play.rand.zero_one() < 0.1 {
            if self.actor.home_pos.y < self.actor.world_pos.y {
                self.aim_rot_x = f_to_s16(self.aim_rot_x as f32 - ((1024.0 * play.rand.zero_one()) + 1024.0));
            } else {
                self.aim_rot_x = f_to_s16(self.aim_rot_x as f32 + ((1024.0 * play.rand.zero_one()) + 1024.0));
            }
            self.aim_rot_x = self.aim_rot_x.clamp(-0x1000, 0x1000);
        }

        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            scaled_step_to_s(&mut self.actor.shape_rot.x, -0x100, 0x400);
        }

        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 && self.actor.xz_dist_to_player < 300.0 && state1 & PLAYER_STATE1_23 == 0 && self.actor.y_dist_to_water < -40.0 && mask != PLAYER_MASK_SKULL {
            self.setup_dive_attack();
        }
    }

    /// `EnCrow_DiveAttack`.
    fn dive_attack(&mut self, play: &mut PlayState) {
        let (player_pos, state1, mask) =
            play.player.and_then(|h| play.actors.get(h)).map(|p| (p.base().world_pos, p.as_player().map(|q| (q.state_flags1(), q.current_mask())).unwrap_or((0, 0)))).map(|(pos, (s, m))| (pos, s, m)).unwrap_or((Vec3::ZERO, 0, 0));

        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }

        let facing_player = is_facing_player(&self.actor, 0x2800);

        if facing_player {
            let pos = Vec3::new(player_pos.x, player_pos.y + 20.0, player_pos.z);
            // Actor_WorldPitchTowardPoint.
            let mut target = pitch_to(self.actor.world_pos, pos);
            if target > 0x3000 {
                target = 0x3000;
            }
            approach_s(&mut self.actor.shape_rot.x, target, 2, 0x400);
        } else {
            approach_s(&mut self.actor.shape_rot.x, -0x1000, 2, 0x100);
        }

        if facing_player || self.actor.xz_dist_to_player > 80.0 {
            approach_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 4, 0xC00);
        }

        if self.timer == 0
            || mask == PLAYER_MASK_SKULL
            || self.collider.base.at_flags & AT_HIT != 0
            || self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_WALL) != 0
            || state1 & PLAYER_STATE1_23 != 0
            || self.actor.y_dist_to_water > -40.0
        {
            if self.collider.base.at_flags & AT_HIT != 0 {
                self.collider.base.at_flags &= !AT_HIT;
                audio_play_actor_sfx2(play, NA_SE_EN_KAICHO_ATTACK);
            }
            self.setup_fly_idle();
        }
    }

    /// `EnCrow_Damaged`: slowing to a stop, kept coloured; red, it noses down and spins; on the
    /// ground (or over no floor), a puff of smoke (`EffectSsDeadDb`, its size × 10000) and it dies.
    fn damaged(&mut self, play: &mut PlayState) {
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.5);
        self.actor.color_filter_timer = 40;

        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW == 0 {
            if self.actor.color_filter_params & 0x4000 != 0 {
                scaled_step_to_s(&mut self.actor.shape_rot.x, 0x4000, 0x200);
                self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x1780);
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.actor.floor_height == BGCHECK_Y_MIN {
                let (pos, scale) = (self.actor.world_pos, f_to_s16(self.actor.scale.x * 10000.0));
                play.with_ss(|s| s.dead_db_spawn(pos, S_ZERO_VEC_ACCEL, S_ZERO_VEC_ACCEL, scale, 0, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
                self.setup_die();
            }
        }
    }

    /// `EnCrow_Die`: shrinking to nothing (0.002 a frame, a big one 0.006); then a normal one is
    /// counted (`sDeathCount`) and drops from table 0, a big one a red rupee; it respawns.
    fn die(&mut self, play: &mut PlayState) {
        let step = if TYPE.get(self.actor.params) != 0 { 0.006 } else { 0.002 };

        if step_to_f(&mut self.actor.scale.x, 0.0, step) {
            if TYPE.get(self.actor.params) == 0 {
                statics(play).s_death_count += 1;
                let pos = self.actor.world_pos;
                item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_0);
            } else {
                item_drop_collectible(play, self.actor.world_pos, ITEM00_RUPEE_RED);
            }
            self.setup_respawn(play);
        }

        self.actor.scale.y = self.actor.scale.x;
        self.actor.scale.z = self.actor.scale.x;
    }

    /// `EnCrow_TurnAway`: away from Link (off walls), climbing, for its 100 frames; then idle.
    fn turn_away(&mut self) {
        self.skel.update();

        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.aim_rot_y = self.actor.wall_yaw;
        } else {
            self.aim_rot_y = self.actor.yaw_towards_player.wrapping_add(0x8000u16 as i16);
        }

        approach_s(&mut self.actor.shape_rot.y, self.aim_rot_y, 3, 0xC00);
        approach_s(&mut self.actor.shape_rot.x, self.aim_rot_x, 5, 0x100);

        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.setup_fly_idle();
        }
    }

    /// `EnCrow_Respawn`: its 300 frames out, drawn again and growing a tenth of its size (0.01,
    /// a big one 0.03) a frame; grown, targetable, culled again, 1 health, idle.
    fn respawn(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            self.skel.update();
            self.drawn = true;
            let target = if TYPE.get(self.actor.params) != 0 { 0.03 } else { 0.01 };
            if step_to_f(&mut self.actor.scale.x, target, target * 0.1) {
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                self.actor.col_chk_info.health = 1;
                self.setup_fly_idle();
            }
            self.actor.scale.y = self.actor.scale.x;
            self.actor.scale.z = self.actor.scale.x;
        }
    }

    /// `EnCrow_UpdateDamage`: a hit (`AC_HIT`), its drop flag set; with damage or a reaction, a
    /// Deku Nut turns it away, anything else kills it.
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.elements[0].info;
            self.actor.set_drop_flag(&elem, true);
            if self.actor.col_chk_info.damage_reaction != 0 || self.actor.col_chk_info.damage != 0 {
                if self.actor.col_chk_info.damage_reaction == EN_CROW_DMG_REACT_NUT {
                    // Deku Nuts.
                    self.setup_turn_away(play);
                } else {
                    self.actor.apply_damage();
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                    enemy_start_finishing_blow(play, &self.actor);
                    self.setup_damaged(play);
                }
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::FlyIdle => self.fly_idle(play),
            Action::DiveAttack => self.dive_attack(play),
            Action::Damaged => self.damaged(play),
            Action::Die => self.die(play),
            Action::TurnAway => self.turn_away(),
            Action::Respawn => self.respawn(),
        }
    }

    /// `Actor_Draw`'s model matrix for `rs`, moved by the main camera's quake offset (the actor
    /// has `ACTOR_FLAG_IGNORE_QUAKE`).
    fn model_matrix(&self, rs: &RenderState, quake: Vec3) -> Mat4 {
        let mut m = actor_draw_matrix(rs);
        m.w_axis = ignore_quake_translation(self.actor.flags, rs.pos, rs.y_offset * rs.scale.y, quake).extend(1.0);
        m
    }
}

// shared: `Actor_Draw`'s translation with `ACTOR_FLAG_IGNORE_QUAKE` (as `en_firefly.rs` has it):
// the position raised by `shape.yOffset * scale.y` (`y_offset`), and with the flag moved by
// `play->mainCamera.quakeOffset` too, so the actor shakes with the view.
fn ignore_quake_translation(flags: u32, pos: Vec3, y_offset: f32, quake: Vec3) -> Vec3 {
    if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 { Vec3::new(pos.x + quake.x, pos.y + (y_offset + quake.y), pos.z + quake.z) } else { Vec3::new(pos.x, pos.y + y_offset, pos.z) }
}

/// `EnCrow_OverrideLimbDraw`: while alive, the wings (limbs 7 and 8) beat harder, by the frame
/// (`sinf(curFrame × π/4)`, the second 2.5 frames on).
fn override_limb(limb: usize, rot: &mut [i16; 3], alive: bool, cur_frame: f32) {
    if alive {
        let quarter_pi = std::f64::consts::PI / 4.0;
        if limb == 7 {
            rot[1] = f_to_s16(rot[1] as f32 + 3072.0 * ((cur_frame as f64 * quarter_pi) as f32).sin());
        } else if limb == 8 {
            rot[1] = f_to_s16(rot[1] as f32 + 5120.0 * (((cur_frame + 2.5) as f64 * quarter_pi) as f32).sin());
        }
    }
}

/// `gGuaySkel`'s bake (`EnCrow_Draw`: `Gfx_SetupDL_25Opa`, `SkelAnime_DrawFlexOpa`).
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
    }]
}

impl ActorImpl for EnCrow {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnCrow_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.update_damage(play);
        self.run_action(play);
        let scale = self.actor.scale.x * 100.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.actor.world_rot.x = self.actor.shape_rot.x.wrapping_neg();

        let height;
        if self.action != Action::Respawn {
            if self.actor.col_chk_info.health != 0 {
                height = 20.0 * scale;
                self.actor.move_xyz();
            } else {
                height = 0.0;
                self.actor.move_forward();
            }
            self.actor.update_bg_check_info(&play.col, 12.0 * scale, 25.0 * scale, 50.0 * scale, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        } else {
            height = 0.0;
        }

        let p = self.actor.world_pos;
        self.collider.elements[0].dim.world_sphere.center = [f_to_s16(p.x), f_to_s16(p.y + height), f_to_s16(p.z)];

        if self.action == Action::DiveAttack {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
        if self.action != Action::Respawn {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }

        self.actor.set_focus(height);

        if self.actor.col_chk_info.health != 0 && self.skel.on_frame(3.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_KAICHO_FLUTTER);
        }
    }

    /// `EnCrow_Draw`'s changes to the actor: `EnCrow_PostLimbDraw`'s body parts (the head 2500
    /// along limb 2, limbs 4, 6 and 8 at their joints; each 20 down).
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.drawn {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = self.model_matrix(&RenderState::of(&self.actor), play.game_camera.quake_offset);
        let (alive, cur_frame) = (self.actor.col_chk_info.health != 0, self.skel.cur_frame);
        let mut posts = Vec::new();
        draw_opa_pose(
            &skeleton,
            &self.skel.joint_table,
            |limb, _, rot| {
                override_limb(limb, rot, alive, cur_frame);
                LimbDraw::Default(Mat4::IDENTITY)
            },
            |limb, m| posts.push((limb, m)),
        );
        for (limb, m) in posts {
            let m = model * m;
            if limb == 2 {
                let p = m.transform_point3(S_HEAD_VEC);
                self.body_parts_pos[0] = Vec3::new(p.x, p.y - 20.0, p.z);
            } else if limb == 4 || limb == 6 || limb == 8 {
                let p = m.transform_point3(S_ZERO_VEC_ACCEL);
                self.body_parts_pos[(limb >> 1) - 1] = Vec3::new(p.x, p.y - 20.0, p.z);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.skel.cur_frame];
        rs.switches = vec![self.drawn as u32, (self.actor.col_chk_info.health != 0) as u32];
        // The shape's shadow: none with draw NULL (Actor_Draw isn't called).
        if !self.drawn {
            rs.shadow = None;
        }
        rs
    }

    /// `EnCrow_Draw` (`Actor_Draw` only while `actor.draw` is set): `gGuaySkel` with its wings
    /// beating.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let ([cur_frame], [drawn, alive]) = (rs.values.as_slice(), rs.switches.as_slice()) else { return };
        if *drawn == 0 {
            return;
        }
        let bones = skeleton.pose_override(&joints.rot, |limb, _, rot| {
            override_limb(limb, rot, *alive != 0, *cur_frame);
            Mat4::IDENTITY
        });
        let model = self.model_matrix(rs, play.game_camera.quake_offset);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: model, bones, params: Default::default() });
    }

    /// `EnCrow_Destroy`: `Collider_DestroyJntSph` (nothing to free here).
    fn destroy(&mut self, _play: &mut PlayState) {}

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
