//! `En_Heishi3` (`ovl_En_Heishi3/z_en_heishi3.c`): Hyrule Castle's sentries. `params <= 0` (the
//! castle grounds' seven, `unk_278` 0) see Link within 250 in front of them by day (200 by night)
//! and 150 behind (100), and within 100 up or down; `params > 0` (the two at the castle's door,
//! `unk_278` 1 east of x -290, 2 west of it) watch a box of the courtyard's approach, each its own
//! half. Seen: "Hey you!" (0x702D, `NA_SE_SY_FOUND`), Link held (`PLAYER_CSACTION_1`), the guard
//! walks to him for 20 frames, and when the text's event is read Link is thrown out to
//! `ENTR_HYRULE_CASTLE_4` (by the gate) with a white starburst circle wipe. One catch at a time:
//! `sPlayerCaught`, the overlay's static, cleared by every guard's init.
//!
//! The whole overlay is ported. The guard's skeleton is `En_Heishi4`'s bake.
//!
//! Params: `params` whole, signed (0 and below, -1 in the data: a grounds sentry, `unk_278` 0;
//! above 0, 1 in the data: a door guard, `unk_278` 1 or 2 by his x).
//! Flags: sets EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS (Link caught; read by nothing).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{Actor, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::collision_check::*;
use oot_game::message::TEXT_STATE_EVENT;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::transition::{TCA_STARBURST, TCC_WHITE, TCS_FAST, TRANS_TRIGGER_START, trans_type_circle};

use crate::en_heishi4::{self, EnHeishi4};

/// `ACTOR_EN_HEISHI3` (`actor_table.h`: 0x0142).
pub const ACTOR_EN_HEISHI3: i16 = 0x0142;
pub const OBJECT: &str = "object_sd";

/// `En_Heishi3_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HEISHI3, name: "En_Heishi3", category: ACTORCAT_NPC, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `ENTR_HYRULE_CASTLE_4` (`entrance_table.h`: 0x47E): by the gate guard.
pub const ENTR_HYRULE_CASTLE_4: u16 = 0x47E;
/// `EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS` (`save.h`: 0x4E, set but unused).
pub const EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS: u16 = 0x4E;

/// `this->actor.params <= 0`: which guard.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "a grounds sentry"), (0, "a grounds sentry"), (1, "a door guard (unk_278 1 east of x -290, 2 west)")]),
    "a grounds sentry (0 and below) or a door guard (above 0)",
);

/// En_Heishi3's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS)).sets().note("Link caught (read by nothing)")],
    todo: false,
};
/// `TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_WHITE, TCS_FAST)`.
pub const TRANS_TYPE_CAUGHT: u8 = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);

/// `NA_SE_SY_FOUND` (`systembank_table.h`: 0x482C), `NA_SE_EV_KNIGHT_WALK` (`environmentbank_table.h`:
/// 0x28F0).
pub const NA_SE_SY_FOUND: u16 = 0x482C;
pub const NA_SE_EV_KNIGHT_WALK: u16 = 0x28F0;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `PLAYER_CSACTION_1`: Link stands still.
const PLAYER_CSACTION_1: u8 = 1;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 70, y_shift: 0, pos: [0; 3] },
};

/// The overlay's statics: `sPlayerCaught`.
#[derive(Debug, Default)]
pub struct Statics {
    pub player_caught: i16,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnHeishi3_SetupGuardType`.
    SetupGuardType,
    /// `EnHeishi3_StandSentinelInGrounds`.
    StandSentinelInGrounds,
    /// `EnHeishi3_StandSentinelInCastle`.
    StandSentinelInCastle,
    /// `EnHeishi3_CatchStart`.
    CatchStart,
    /// `func_80A55BD4`: walking to Link.
    WalkToPlayer,
    /// `EnHeishi3_ResetAnimationToIdle`.
    ResetAnimationToIdle,
    /// `func_80A55D00`: the throw-out on the text's event.
    Respawn,
}

pub struct EnHeishi3 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    /// `unk_262`, `unk_264`, `unk_26E`: the head's and torso's turns the limb draw adds (never
    /// written: 0).
    pub unk_262: i16,
    pub unk_264: i16,
    pub unk_26e: i16,
    pub caught_timer: i16,
    pub unk_274: i16,
    /// `respawnFlag`: the throw-out started.
    pub respawn_flag: u8,
    /// `unk_278`: 0 the grounds' sentries, 1 and 2 the castle door's, east and west.
    pub unk_278: i16,
    pub collider: ColliderCylinder,
    idle: Anim,
    walk: Anim,
}

impl EnHeishi3 {
    /// `EnHeishi3_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gEnHeishiSkel")?, assets.animation(OBJECT, "gEnHeishiIdleAnim")?, assets.animation(OBJECT, "gEnHeishiWalkAnim")?)) })();
        let (skeleton, idle, walk) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Heishi3: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        play.overlay_static::<Statics>(ACTOR_EN_HEISHI3).player_caught = 0;
        let unk_278 = if TYPE.get(actor.params) <= 0 {
            0
        } else if actor.world_pos.x < -290.0 {
            2
        } else {
            1
        };
        let mut this = EnHeishi3 {
            // SkelAnime_Init(.., gEnHeishiSkel, gEnHeishiIdleAnim, .., 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(idle.clone())),
            skeleton,
            action: Action::SetupGuardType,
            unk_262: 0,
            unk_264: 0,
            unk_26e: 0,
            caught_timer: 0,
            unk_274: 0,
            respawn_flag: 0,
            unk_278,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            idle,
            walk,
            actor,
        };
        this.actor.scale = Vec3::splat(0.01);
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.target_mode = ATTENTION_RANGE_6;
        log::debug!("☆☆☆☆☆ Castle gate soldier power-up ☆☆☆☆☆");
        this.actor.gravity = -3.0;
        this.actor.focus_pos = this.actor.world_pos;
        Box::new(this)
    }

    /// `Animation_Change(&skelAnime, anim, 1, 0, (s16)lastFrame, LOOP, -10)`.
    fn change_loop(&mut self, anim: Anim) {
        let frames = anim.last_frame() as i16 as f32;
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
    }

    /// `EnHeishi3_SetupGuardType`.
    fn setup_guard_type(&mut self) {
        self.change_loop(self.idle.clone());
        self.action = if self.unk_278 == 0 { Action::StandSentinelInGrounds } else { Action::StandSentinelInCastle };
    }

    /// The catch both kinds make: "Hey you!", the found sound, Link held.
    fn catch(&mut self, play: &mut PlayState) {
        play.overlay_static::<Statics>(ACTOR_EN_HEISHI3).player_caught = 1;
        play.start_textbox(0x702D, play.cur_actor);
        play.audio.play_sfx_centered(NA_SE_SY_FOUND);
        log::debug!("☆☆☆☆☆ Discovered! ☆☆☆☆☆");
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
        // (OOT_PAL_N64 only: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED.)
        self.action = Action::CatchStart;
    }

    /// `EnHeishi3_StandSentinelInGrounds`: Link within its sight's range (250 in front by day, 200
    /// by night; 150 and 100 behind) and 100 up or down.
    fn stand_sentinel_in_grounds(&mut self, play: &mut PlayState) {
        let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(self.actor.world_pos.y);
        self.skel.update();
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        let yaw_diff_new = (yaw_diff as i32).abs() as i16;
        let day = play.save.is_day();
        let sight_range = if yaw_diff_new < 0x4300 {
            if day { 250.0 } else { 200.0 }
        } else if day {
            150.0
        } else {
            100.0
        };
        if self.actor.xz_dist_to_player < sight_range && (player_y - self.actor.world_pos.y).abs() < 100.0 && play.overlay_static::<Statics>(ACTOR_EN_HEISHI3).player_caught == 0 {
            self.catch(play);
        }
    }

    /// `EnHeishi3_StandSentinelInCastle`: Link in the box x -380 to -190, z 700 to 1020, within
    /// 100 up or down, on this guard's half (x -290).
    fn stand_sentinel_in_castle(&mut self, play: &mut PlayState) {
        let p = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        self.skel.update();
        if p.x < -190.0 && p.x > -380.0 && (p.y - self.actor.world_pos.y).abs() < 100.0 && p.z < 1020.0 && p.z > 700.0 && play.overlay_static::<Statics>(ACTOR_EN_HEISHI3).player_caught == 0 {
            if self.unk_278 == 1 {
                if p.x < -290.0 {
                    return;
                }
            } else if p.x > -290.0 {
                return;
            }
            self.catch(play);
        }
    }

    /// `EnHeishi3_CatchStart`.
    fn catch_start(&mut self) {
        let frames = self.walk.last_frame() as i16 as f32;
        self.skel.change(self.walk.clone(), 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
        self.caught_timer = 20;
        self.action = Action::WalkToPlayer;
        self.actor.speed_xz = 2.5;
    }

    /// `func_80A55BD4`: 20 frames towards Link, the steps on frames 1 and 17.
    fn walk_to_player(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(1.0) || self.skel.on_frame(17.0) {
            audio_play_actor_sfx2(play, NA_SE_EV_KNIGHT_WALK);
        }
        if self.caught_timer == 0 {
            self.action = Action::ResetAnimationToIdle;
            self.actor.speed_xz = 0.0;
        } else {
            eng_math::smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 5, 3000, 0);
        }
    }

    /// `EnHeishi3_ResetAnimationToIdle`.
    fn reset_animation_to_idle(&mut self) {
        self.change_loop(self.idle.clone());
        self.action = Action::Respawn;
    }

    /// `func_80A55D00`: on the text's event, Link out to `ENTR_HYRULE_CASTLE_4`.
    fn respawn(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() && self.respawn_flag == 0 {
            play.save.set_event_chk_inf(EVENTCHKINF_CAUGHT_BY_CASTLE_GUARDS);
            play.transition.next_entrance_index = ENTR_HYRULE_CASTLE_4;
            play.transition.trigger = TRANS_TRIGGER_START;
            self.respawn_flag = 1;
            play.transition.ty = TRANS_TYPE_CAUGHT;
            play.save.next_transition_type = TRANS_TYPE_CAUGHT;
        }
    }
}

impl ActorImpl for EnHeishi3 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHeishi3_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.set_focus(60.0);
        self.unk_274 = self.unk_274.wrapping_add(1);
        if self.caught_timer != 0 {
            self.caught_timer -= 1;
        }
        match self.action {
            Action::SetupGuardType => self.setup_guard_type(),
            Action::StandSentinelInGrounds => self.stand_sentinel_in_grounds(play),
            Action::StandSentinelInCastle => self.stand_sentinel_in_castle(play),
            Action::CatchStart => self.catch_start(),
            Action::WalkToPlayer => self.walk_to_player(play),
            Action::ResetAnimationToIdle => self.reset_animation_to_idle(),
            Action::Respawn => self.respawn(play),
        }
        self.actor.shape_rot = self.actor.world_rot;
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 50.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs
    }
    /// `EnHeishi3_Draw`: `EnHeishi3_OverrideLimbDraw` adds `unk_26E` to limb 9's x and `unk_262`,
    /// `unk_264` to limb 16's x and z: `En_Heishi4`'s turns with the head's yaw and roll and the
    /// torso's yaw in those places.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let bones = EnHeishi4::pose(&self.skeleton, &joints.rot, [0, self.unk_262, self.unk_264], [0, self.unk_26e, 0]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(oot_game::pack::keys::bake(en_heishi4::BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
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
