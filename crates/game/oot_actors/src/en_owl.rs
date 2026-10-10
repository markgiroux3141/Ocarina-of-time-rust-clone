//! `En_Owl` (`ovl_En_Owl/z_en_owl.c`): Kaepora Gaebora, the owl. Its params: a switch flag
//! (bits 0 to 5: once set, he's gone) and a type (bits 6 to 11, `EnOwlType`); 0xFFF is the one
//! outside Kokiri Forest with flag 0x20 (never set).
//!
//! Each type waits on its perch, turned to Link (`EnOwl_LookAtLink`), and when Link comes within
//! its range takes him into a talk he can't refuse (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`,
//! `EnOwl_CheckInitTalk`) with the owl's fanfare and one-point cutscene 8700 on him; its texts
//! ask whether to hear them again. When the talk ends he sets his flag, unfolds his wings, takes
//! off and flies away (`func_80ACA5C8` on), gone 6000 away. The Lake Hylia and Death Mountain owls
//! instead carry Link (`func_80ACC00C`: their scenes' scripts); the cutscene owls follow cue
//! channel 7 (`EnOwl_WaitDefault`, `func_80ACB904`, `func_80ACB994`).
//!
//! Between flights his head turns (`unk_3EE`: a full turn for a choice), bobs and tilts, and his
//! eyes blink (`EnOwl_Update`); the draw turns his limbs by them (`EnOwl_OverrideLimbDraw`) and
//! puts his focus at his head (`EnOwl_PostLimbUpdate`, made in `draw_update`).
//!
//! Ported whole (GAME-06 milestone 2; the carrying owls' `gTimeSpeed = 0` with the clock in
//! milestone 3). Not modelled: the perched animation on the flying skeleton
//! (`func_80ACBAB8`'s cue 2: @bug (game), its missing joints read as zero).

use std::sync::Arc;

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::SEQ_PLAYER_FANFARE;
use oot_game::audio::bgm::seq_cmd1;
use oot_game::audio::sfx::{NA_SE_EN_OWL_FLUTTER, NA_SE_EV_FLYING_AIR, NA_SE_EV_PASS_AIR, NA_SE_SY_TRE_BOX_APPEAR, SFX_FLAG, SfxPos};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_EVENT};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_ONCE, Anim, SkelAnimeStd};

/// `ACTOR_EN_OWL` (`actor_table.h`: 0x014D).
pub const ACTOR_EN_OWL: i16 = 0x014D;
pub const OBJECT: &str = "object_owl";

/// `En_Owl_Profile`: `ACTORCAT_NPC`, `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_OWL, name: "En_Owl", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT };

/// `EnOwlType`.
pub const OWL_DEFAULT: i16 = 0x00;
pub const OWL_OUTSIDE_KOKIRI: i16 = 0x01;
pub const OWL_HYRULE_CASTLE: i16 = 0x02;
pub const OWL_KAKARIKO: i16 = 0x03;
pub const OWL_HYLIA_GERUDO: i16 = 0x04;
pub const OWL_LAKE_HYLIA: i16 = 0x05;
pub const OWL_ZORA_RIVER: i16 = 0x06;
pub const OWL_HYLIA_SHORTCUT: i16 = 0x07;
pub const OWL_DEATH_MOUNTAIN: i16 = 0x08;
pub const OWL_DEATH_MOUNTAIN2: i16 = 0x09;
pub const OWL_DESERT_COLOSSUS: i16 = 0x0A;
pub const OWL_LOST_WOODS_PRESARIA: i16 = 0x0B;
pub const OWL_LOST_WOODS_POSTSARIA: i16 = 0x0C;

/// `EnOwlMessageChoice`.
const OWL_REPEAT: u8 = 0;
const OWL_OK: u8 = 1;

/// `NA_BGM_OWL` (`sequence_table.h`: 0x5A).
pub const NA_BGM_OWL: u16 = 0x5A;
/// `PLAYER_CSACTION_8`.
const PLAYER_CSACTION_8: u8 = 8;

/// `scene_table.h`.
const SCENE_DESERT_COLOSSUS: u16 = 0x5C;

/// `save.h`.
const EVENTCHKINF_OBTAINED_ZELDAS_LETTER: u16 = 0x40;
const EVENTCHKINF_43: u16 = 0x43;
const EVENTCHKINF_39: u16 = 0x39;
const EVENTCHKINF_6F: u16 = 0x6F;
const INFTABLE_195: u16 = 0x195;
/// `QUEST_SONG_LULLABY`, `QUEST_SONG_SARIA` (`item.h`).
const QUEST_SONG_LULLABY: u32 = 0x0C;
const QUEST_SONG_SARIA: u32 = 0x0E;

/// The animations.
const ANIM_FLY: &str = "gOwlFlyAnim";
const ANIM_PERCH: &str = "gOwlPerchAnim";
const ANIM_UNFOLD_WINGS: &str = "gOwlUnfoldWingsAnim";
const ANIM_TAKEOFF: &str = "gOwlTakeoffAnim";
const ANIM_GLIDE: &str = "gOwlGlideAnim";

/// `sOwlCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 40, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitDefault,
    WaitOutsideKokiri,
    WaitHyruleCastle,
    WaitKakariko,
    WaitGerudo,
    WaitLakeHylia,
    WaitZoraRiver,
    WaitHyliaShortcut,
    WaitDeathMountainShortcut,
    /// The Desert Colossus' (`func_80ACB3E0`).
    WaitColossus,
    WaitLWPreSaria,
    WaitLWPostSaria,
    ConfirmKokiriMessage,
    /// `func_80ACA76C`, `func_80ACA7E0`: the talk's end.
    TalkEnd,
    TalkEndOrTurn,
    /// `func_80ACA690`: the head back round, then away.
    TurnBack,
    /// The choices and the second texts of each type (`func_80ACA998` and `func_80ACAA54`, ...).
    Choice(u8),
    Repeat(u8),
    /// `func_80ACB03C`, `func_80ACB148`, `func_80ACB22C`, `func_80ACB274`, `func_80ACB344`.
    ZoraRiverEnd,
    HyliaShortcutEnd,
    DeathMountainEnd,
    DeathMountainAgain,
    ColossusChoice,
    /// The cue-driven flights (`func_80ACB904`, `func_80ACB994`).
    CueOrbit,
    CueLine,
    /// The flight away (`func_80ACBEA0` unfolding, `func_80ACBD4C` taking off, `func_80ACBC0C`
    /// flying).
    Unfold,
    Takeoff,
    FlyAway,
    /// The carrying owls (`func_80ACC30C`, `func_80ACC23C`, `func_80ACC00C`, `func_80ACBF50`).
    CarryTakeoff,
    CarryHop,
    CarryWait,
    CarryTurn,
}

/// `unk_410`: the animation's step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwlFunc {
    /// `func_80ACC540`: once, `actionFlags` 1 at its end.
    Once,
    /// `func_80ACC460`: the wing beats, then the glide.
    Beats,
    /// `func_80ACC390`: the glide's sway.
    Glide,
}

/// Which skeleton `curSkelAnime` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skel {
    Flying,
    Perching,
}

pub struct EnOwl {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    /// `skelAnime` (the flying skeleton), `skelAnime2` (the perching one), `curSkelAnime`.
    pub skel: SkelAnimeStd,
    pub skel2: SkelAnimeStd,
    pub cur: Skel,
    pub skeletons: Option<[Arc<eng_anim::skeleton::Skeleton>; 2]>,
    pub eye: Vec3,
    pub unk_3ec: i16,
    pub unk_3ee: i16,
    pub unk_3f0: i16,
    pub unk_3f2: i16,
    pub eye_tex_index: i16,
    pub blink_timer: i16,
    pub unk_3f8: f32,
    pub action_flags: u16,
    pub unk_3fe: u16,
    pub unk_400: i16,
    pub sub_cam_id: i16,
    pub unk_404: u8,
    pub unk_405: u8,
    pub unk_406: u8,
    pub unk_407: u8,
    pub unk_408: u8,
    pub unk_409: u8,
    pub unk_40a: u8,
    pub action: Action,
    pub unk_410: OwlFunc,
    /// `actor.draw != NULL`.
    pub drawn: bool,
}

impl EnOwl {
    fn owl_type(&self) -> i16 {
        (self.actor.params >> 6) & 0x3F
    }

    fn anim(play: &PlayState, name: &str) -> Option<Anim> {
        play.assets.as_ref()?.animation(OBJECT, name).map_err(|e| log::error!("En_Owl: {e:#}")).ok()
    }

    fn cur_skel(&mut self) -> &mut SkelAnimeStd {
        match self.cur {
            Skel::Flying => &mut self.skel,
            Skel::Perching => &mut self.skel2,
        }
    }

    /// `EnOwl_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnOwl {
            actor,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            skel: SkelAnimeStd::init_flex(0, None),
            skel2: SkelAnimeStd::init_flex(0, None),
            cur: Skel::Perching,
            skeletons: None,
            eye: Vec3::ZERO,
            unk_3ec: 0,
            unk_3ee: 0,
            unk_3f0: 0,
            unk_3f2: 0,
            eye_tex_index: 0,
            blink_timer: 0,
            unk_3f8: 0.0,
            action_flags: 0,
            unk_3fe: 0,
            unk_400: 0,
            sub_cam_id: 0,
            unk_404: 0,
            unk_405: 0,
            unk_406: 0,
            unk_407: 0,
            unk_408: 0,
            unk_409: 0,
            unk_40a: 0,
            action: Action::WaitDefault,
            unk_410: OwlFunc::Once,
            drawn: true,
        };
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 25) and the culling volume.
        this.actor.scale = Vec3::splat(0.025);
        // ActorShape_Init(0, ActorShadow_DrawCircle, 36).
        this.actor.shape_y_offset = 0.0;
        if let Some(a) = play.assets.clone() {
            match (a.skeleton(OBJECT, "gOwlFlyingSkel"), a.skeleton(OBJECT, "gOwlPerchingSkel")) {
                (Ok(fly), Ok(perch)) => {
                    this.skel = SkelAnimeStd::init_flex(fly.limbs.len(), Self::anim(play, ANIM_FLY));
                    this.skel2 = SkelAnimeStd::init_flex(perch.limbs.len(), Self::anim(play, ANIM_PERCH));
                    this.skeletons = Some([fly, perch]);
                }
                (Err(e), _) | (_, Err(e)) => log::error!("En_Owl: {e:#}"),
            }
        }
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.min_velocity_y = -10.0;
        this.actor.target_arrow_offset = 500.0;
        this.change_mode(play, Action::WaitDefault, OwlFunc::Once, Skel::Perching, ANIM_PERCH, 0.0);
        this.action_flags = 0;
        this.unk_406 = 0;
        this.unk_409 = 0;
        this.unk_405 = 4;
        this.unk_404 = 0;
        this.unk_407 = 0;
        this.unk_408 = 4;
        let mut owl_type = this.owl_type();
        let mut switch_flag = (this.actor.params & 0x3F) as i32;
        if this.actor.params == 0xFFF {
            owl_type = OWL_OUTSIDE_KOKIRI;
            switch_flag = 0x20;
        }
        log::debug!(" conversation owl {:4x} no = {owl_type}, sv = {switch_flag}", this.actor.params);
        if owl_type != OWL_DEFAULT && switch_flag < 0x20 && play.flags.get_switch(switch_flag) {
            log::debug!("Save owl with savebit");
            this.actor.kill();
            return Box::new(this);
        }
        this.unk_3ee = 0;
        this.unk_400 = this.actor.world_rot.y;
        let s = &play.save;
        match owl_type {
            OWL_DEFAULT => {
                this.action = Action::WaitDefault;
                this.unk_40a = 0;
            }
            OWL_OUTSIDE_KOKIRI => this.action = Action::WaitOutsideKokiri,
            OWL_HYRULE_CASTLE => {
                this.action_flags |= 2;
                this.unk_3ee = 0x20;
                this.action = Action::WaitHyruleCastle;
            }
            OWL_KAKARIKO => {
                if s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
                    this.actor.kill();
                    return Box::new(this);
                }
                this.action = Action::WaitKakariko;
            }
            OWL_HYLIA_GERUDO => {
                if s.get_event_chk_inf(EVENTCHKINF_43) {
                    this.actor.kill();
                    return Box::new(this);
                }
                this.action = Action::WaitGerudo;
            }
            OWL_LAKE_HYLIA => this.action = Action::WaitLakeHylia,
            OWL_ZORA_RIVER => {
                if s.get_event_chk_inf(EVENTCHKINF_39) || !s.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
                    this.actor.kill();
                    return Box::new(this);
                }
                this.action = Action::WaitZoraRiver;
            }
            OWL_HYLIA_SHORTCUT => {
                this.action = Action::WaitHyliaShortcut;
                play.flags.unset_switch(0x23);
                return Box::new(this);
            }
            OWL_DEATH_MOUNTAIN | OWL_DEATH_MOUNTAIN2 => this.action = Action::WaitDeathMountainShortcut,
            OWL_DESERT_COLOSSUS => this.action = Action::WaitColossus,
            OWL_LOST_WOODS_PRESARIA => {
                if !s.check_quest_item(QUEST_SONG_LULLABY) {
                    this.actor.kill();
                    return Box::new(this);
                }
                this.action = Action::WaitLWPreSaria;
            }
            OWL_LOST_WOODS_POSTSARIA => {
                if !s.check_quest_item(QUEST_SONG_SARIA) {
                    this.actor.kill();
                    return Box::new(this);
                }
                this.action = Action::WaitLWPostSaria;
            }
            _ => {
                log::debug!("no = {owl_type}: Unfinished owl unfinished owl unfinished owl");
                this.action_flags |= 2;
                this.unk_3ee = 0x20;
                this.action = Action::WaitOutsideKokiri;
            }
        }
        Box::new(this)
    }

    /// `EnOwl_ChangeMode`: the skeleton, its animation once from the start, the action and the
    /// animation's step.
    fn change_mode(&mut self, play: &PlayState, action: Action, func: OwlFunc, skel: Skel, anim: &str, morph_frames: f32) {
        self.cur = skel;
        if let Some(a) = Self::anim(play, anim) {
            let last = a.last_frame();
            self.cur_skel().change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph_frames);
        }
        self.action = action;
        self.unk_410 = func;
    }

    /// `EnOwl_LookAtLink`.
    fn look_at_link(&mut self, play: &PlayState) {
        let Some(p) = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos) else { return };
        let yaw = oot_game::target::yaw_to(self.actor.world_pos, p);
        self.actor.shape_rot.y = yaw;
        self.actor.world_rot.y = yaw;
    }

    /// `EnOwl_CheckInitTalk`: a talk accepted (one-point 8700, turning one way or the other); or
    /// the text set and, within `target_dist`, the talk offered to be taken at once.
    fn check_init_talk(&mut self, play: &mut PlayState, text_id: u16, target_dist: f32, flags: u16) -> bool {
        if oot_game::npc::process_talk_request(&mut self.actor) {
            let timer = if self.actor.params == 0xFFF {
                self.action_flags |= 0x40;
                -100
            } else if play.rand.zero_one() < 0.5 {
                self.action_flags |= 0x40;
                if flags & 1 != 0 { -97 } else { -99 }
            } else {
                self.action_flags &= !0x40;
                if flags & 1 != 0 { -96 } else { -98 }
            };
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            self.sub_cam_id = play.onepoint_cutscene_init(8700, timer, me, CAM_ID_MAIN);
            true
        } else {
            self.actor.text_id = text_id;
            let dist_check = if flags & 2 != 0 { 200.0 } else { 1000.0 };
            if self.actor.xz_dist_to_player < target_dist {
                self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                let a = self.actor.clone();
                oot_game::npc::offer_talk_range(play, &a, target_dist, dist_check, 0);
            }
            false
        }
    }

    /// `func_80ACA558`: the shortcut owls' talk, offered within 120 (not taken at once).
    fn func_80aca558(&mut self, play: &mut PlayState, text_id: u16) -> bool {
        if oot_game::npc::process_talk_request(&mut self.actor) {
            true
        } else {
            self.actor.text_id = text_id;
            if self.actor.xz_dist_to_player < 120.0 {
                let a = self.actor.clone();
                oot_game::npc::offer_talk_range(play, &a, 350.0, 1000.0, 0);
            }
            false
        }
    }

    /// `func_80ACA5C8`: the wings unfolding, the eyes open.
    fn func_80aca5c8(&mut self, play: &mut PlayState) {
        self.change_mode(play, Action::Unfold, OwlFunc::Once, Skel::Flying, ANIM_UNFOLD_WINGS, 0.0);
        self.eye_tex_index = 0;
        self.blink_timer = play.rand.s16_offset(60, 60);
    }

    /// `func_80ACA62C`: his flag set, then away.
    fn func_80aca62c(&mut self, play: &mut PlayState) {
        let switch_flag = (self.actor.params & 0x3F) as i32;
        if switch_flag < 0x20 {
            play.flags.set_switch(switch_flag);
            log::debug!(" Actor_Environment_sw = {}", play.flags.get_switch(switch_flag));
        }
        self.func_80aca5c8(play);
    }

    /// `func_80ACA690`.
    fn func_80aca690(&mut self, play: &mut PlayState) {
        if (self.unk_3ee & 0x3F) == 0 {
            self.func_80aca62c(play);
        }
    }

    /// `func_80ACA6C0`: which way the head turns next.
    fn func_80aca6c0(&mut self, play: &mut PlayState) {
        if play.rand.centered_float(1.0) < 0.0 {
            self.action_flags |= 0x20;
        } else {
            self.action_flags &= !0x20;
        }
    }

    /// `func_80ACA71C`: a head turn started.
    fn func_80aca71c(&mut self, play: &mut PlayState) {
        self.func_80aca6c0(play);
        self.unk_3f2 = 0;
        self.action_flags |= 0x10;
        self.unk_408 = 4;
        self.unk_404 = 0;
        self.unk_406 = 0;
        self.unk_405 = 4;
        self.unk_407 = self.unk_3f2 as u8;
    }

    /// `Actor_TextboxIsClosing` (with its `actor->flags &= ~ACTOR_FLAG_TALK`).
    fn textbox_is_closing(&mut self, play: &PlayState) -> bool {
        if oot_game::npc::textbox_is_closing(play) {
            self.actor.flags &= !ACTOR_FLAG_TALK;
            return true;
        }
        false
    }

    /// `SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_FANFARE, 0)`.
    fn stop_fanfare(play: &mut PlayState) {
        play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_FANFARE, 0));
    }

    fn halt_player(&self, play: &mut PlayState) {
        // Player_SetCsActionWithHaltedActors(play, &this->actor, PLAYER_CSACTION_8).
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_8);
    }

    /// `func_80ACA76C`: the Kokiri owl's end: when the box closes, his flag and away.
    fn talk_end(&mut self, play: &mut PlayState) {
        self.halt_player(play);
        if self.textbox_is_closing(play) {
            Self::stop_fanfare(play);
            self.func_80aca62c(play);
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        }
    }

    /// `func_80ACA7E0`: the others' end: away once the head is back round, else it turns first.
    fn talk_end_or_turn(&mut self, play: &mut PlayState) {
        self.halt_player(play);
        if self.textbox_is_closing(play) {
            Self::stop_fanfare(play);
            if (self.unk_3ee & 0x3F) == 0 {
                self.func_80aca62c(play);
            } else {
                self.action_flags &= !2;
                self.func_80aca71c(play);
                self.action = Action::TurnBack;
            }
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        }
    }

    /// A choice answered: `Message_GetState == TEXT_STATE_CHOICE && Message_ShouldAdvance`.
    fn choice(play: &mut PlayState) -> Option<u8> {
        (play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance()).then_some(play.msg_ctx.choice_index)
    }

    /// An event text advanced: `TEXT_STATE_EVENT && Message_ShouldAdvance`.
    fn event(play: &mut PlayState) -> bool {
        play.message_state() == TEXT_STATE_EVENT && play.message_should_advance()
    }

    /// `EnOwl_ConfirmKokiriMessage`: again (0x2065), or done (0x2067, then away).
    fn confirm_kokiri_message(&mut self, play: &mut PlayState) {
        match Self::choice(play) {
            Some(OWL_REPEAT) => play.continue_textbox(0x2065),
            Some(OWL_OK) => {
                play.continue_textbox(0x2067);
                self.action = Action::TalkEnd;
            }
            _ => {}
        }
    }

    /// The other talking owls' choices (`func_80ACA998`, `func_80ACAB88`, `func_80ACAD34`,
    /// `func_80ACAEB8`, `func_80ACB440`, `func_80ACB5C4`): again (the type's second text), or done
    /// (its last); the head turns either way.
    fn owl_choice(&mut self, play: &mut PlayState, which: u8) {
        let Some(c) = Self::choice(play) else { return };
        let (again, done) = match which {
            // func_80ACA998 (Hyrule Castle).
            0 => (0x2069, 0x206B),
            // func_80ACAB88 (Kakariko): the second text by Zelda's letter.
            1 => (if play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) { 0x206D } else { 0x206C }, 0x206E),
            // func_80ACAD34 (Gerudo).
            2 => (0x206F, 0x2070),
            // func_80ACAEB8 (Lake Hylia).
            3 => (0x2071, 0x2072),
            // func_80ACB440 (the Lost Woods before Saria).
            4 => (0x10C1, 0x10C3),
            // func_80ACB5C4 (after).
            _ => (0x10C5, 0x10C7),
        };
        match c {
            OWL_REPEAT => {
                play.continue_textbox(again);
                self.action = Action::Repeat(which);
            }
            OWL_OK => {
                play.continue_textbox(done);
                self.action = Action::TalkEndOrTurn;
            }
            _ => {}
        }
        // (func_80ACB440 has no break before its end: the same.)
        self.action_flags &= !2;
        self.func_80aca71c(play);
    }

    /// The second texts' end (`func_80ACAA54`, `func_80ACAC6C`, `func_80ACADF0`, `func_80ACAF74`,
    /// `func_80ACB4FC`, `func_80ACB680`): the question again (0x206A; the Lost Woods' own), the
    /// head turned round.
    fn owl_repeat(&mut self, play: &mut PlayState, which: u8) {
        if Self::event(play) {
            let q = match which {
                4 => 0x10C2,
                5 => 0x10C6,
                _ => 0x206A,
            };
            play.continue_textbox(q);
            self.action = Action::Choice(which);
            self.action_flags |= 2;
            self.func_80aca71c(play);
        }
    }

    /// `EnOwl_Wait*`: turned to Link, the type's talk; the fanfare, and its next step.
    fn wait(&mut self, play: &mut PlayState) {
        self.look_at_link(play);
        let fanfare = |play: &mut PlayState| play.audio.play_fanfare(NA_BGM_OWL);
        match self.action {
            Action::WaitOutsideKokiri => {
                if self.check_init_talk(play, 0x2064, 360.0, 0) {
                    fanfare(play);
                    self.action = Action::ConfirmKokiriMessage;
                    // Spoke to the owl by the Lost Woods.
                    play.save.set_event_chk_inf(EVENTCHKINF_6F);
                }
            }
            Action::WaitHyruleCastle => {
                if self.check_init_talk(play, 0x2068, 540.0, 0) {
                    fanfare(play);
                    // func_80ACAAC0: its first text's event leads to the question.
                    self.action = Action::Repeat(6);
                }
            }
            Action::WaitKakariko => {
                if self.check_init_talk(play, 0x206C, 480.0, 0) {
                    fanfare(play);
                    self.action = Action::Repeat(1);
                }
            }
            Action::WaitGerudo => {
                if self.check_init_talk(play, 0x206F, 360.0, 0) {
                    fanfare(play);
                    self.action = Action::Repeat(2);
                }
            }
            Action::WaitLakeHylia => {
                if self.check_init_talk(play, 0x2071, 360.0, 0) {
                    fanfare(play);
                    self.action = Action::Repeat(3);
                }
            }
            Action::WaitZoraRiver => {
                let s = &play.save;
                let text = if s.check_quest_item(QUEST_SONG_SARIA) { if s.check_quest_item(QUEST_SONG_LULLABY) { 0x4031 } else { 0x4017 } } else { 0x4002 };
                if self.check_init_talk(play, text, 360.0, 0) {
                    fanfare(play);
                    self.action = Action::ZoraRiverEnd;
                }
            }
            Action::WaitColossus => {
                if self.check_init_talk(play, 0x6079, 360.0, 2) {
                    fanfare(play);
                    self.action = Action::ColossusChoice;
                }
            }
            Action::WaitLWPreSaria => {
                if self.check_init_talk(play, 0x10C0, 190.0, 0) {
                    fanfare(play);
                    self.action = Action::Repeat(4);
                }
            }
            Action::WaitLWPostSaria => {
                if self.check_init_talk(play, 0x10C4, 360.0, 0) {
                    fanfare(play);
                    self.action = Action::Repeat(5);
                }
            }
            _ => {}
        }
    }

    /// `func_80ACAAC0`: Hyrule Castle's first text over: its second (0x2069), the head round.
    fn func_80acaac0(&mut self, play: &mut PlayState) {
        if Self::event(play) {
            play.continue_textbox(0x2069);
            self.action = Action::Repeat(0);
            self.action_flags &= !2;
            self.func_80aca71c(play);
        }
    }

    /// `EnOwl_WaitHyliaShortcut`.
    fn wait_hylia_shortcut(&mut self, play: &mut PlayState) {
        let text = if play.save.get_inf_table(INFTABLE_195) { 0x4004 } else { 0x4003 };
        self.look_at_link(play);
        if self.func_80aca558(play, text) {
            play.save.set_inf_table(INFTABLE_195);
            play.audio.play_fanfare(NA_BGM_OWL);
            self.action = Action::HyliaShortcutEnd;
        }
    }

    /// `EnOwl_WaitDeathMountainShortcut`: by whether Link has magic.
    fn wait_death_mountain_shortcut(&mut self, play: &mut PlayState) {
        self.look_at_link(play);
        if !play.save.is_magic_acquired {
            if self.func_80aca558(play, 0x3062) {
                play.audio.play_fanfare(NA_BGM_OWL);
                self.action = Action::DeathMountainAgain;
            }
        } else if self.func_80aca558(play, 0x3063) {
            play.audio.play_fanfare(NA_BGM_OWL);
            self.action = Action::DeathMountainEnd;
        }
    }

    /// `func_80ACB344`: the Colossus' choice: again (0x607A) or done (0x607C).
    fn colossus_choice(&mut self, play: &mut PlayState) {
        match Self::choice(play) {
            Some(OWL_REPEAT) => play.continue_textbox(0x607A),
            Some(OWL_OK) => {
                play.continue_textbox(0x607C);
                self.action = Action::TalkEndOrTurn;
            }
            _ => {}
        }
    }

    /// `func_80ACB748`: the carrying owls' flight sounds by the cutscene's frame, louder as the eye
    /// moves faster.
    fn func_80acb748(&mut self, play: &mut PlayState) {
        let dist = self.eye.distance(play.view.eye) / 45.0;
        self.eye = play.view.eye;
        let weight = dist.min(1.0);
        let f = play.cs_ctx.frames as i32;
        // D_80ACD62C, a zero vector: the sounds' position (gSfxDefaultPos's).
        let pos = SfxPos::Default;
        match self.owl_type() {
            7 => {
                play.audio.func_800f436c(pos, NA_SE_EV_FLYING_AIR - SFX_FLAG, weight * 2.0);
                if f > 324 || (142..=266).contains(&f) {
                    play.audio.func_800f4414(pos, NA_SE_EN_OWL_FLUTTER, weight * 2.0);
                }
                if f == 85 {
                    play.audio.func_800f436c(pos, NA_SE_EV_PASS_AIR, weight * 2.0);
                }
            }
            8 | 9 => {
                play.audio.func_800f436c(pos, NA_SE_EV_FLYING_AIR - SFX_FLAG, weight * 2.0);
                if f >= 420 || (194..=280).contains(&f) {
                    play.audio.func_800f4414(pos, NA_SE_EN_OWL_FLUTTER, weight * 2.0);
                }
                if f == 217 {
                    play.audio.func_800f436c(pos, NA_SE_EV_PASS_AIR, weight * 2.0);
                }
            }
            _ => {}
        }
    }

    fn cue7(play: &PlayState) -> Option<CsCmdActorCue> {
        if play.cs_ctx.state != CS_STATE_IDLE { play.cs_ctx.npc_actions.get(7).copied().flatten() } else { None }
    }

    /// `func_80ACB904`, `func_80ACB994`: a new cue on channel 7 places him and changes his mode;
    /// then the orbit (`func_80ACD2CC`) or the line (`func_80ACD4D4`).
    fn cue_flight(&mut self, play: &mut PlayState, orbit: bool) {
        if let Some(cue) = Self::cue7(play) {
            if self.unk_40a as u16 != cue.action {
                self.func_80acd130(&cue);
                self.func_80acbab8(play, &cue);
            }
            if !self.actor.killed {
                if orbit {
                    self.func_80acd2cc(play, &cue);
                } else {
                    self.func_80acd4d4(play, &cue);
                }
            }
        }
        if self.action_flags & 0x80 != 0 {
            self.func_80acb748(play);
        }
    }

    /// `EnOwl_WaitDefault`: the cutscene owls, by cue 7.
    fn wait_default(&mut self, play: &mut PlayState) {
        if let Some(cue) = Self::cue7(play) {
            if self.unk_40a as u16 != cue.action {
                self.action_flags |= 4;
                self.func_80acd130(&cue);
                self.func_80acbab8(play, &cue);
            } else {
                self.actor.world_rot.z = cue.rot[1];
            }
        }
        if self.action_flags & 0x80 != 0 {
            self.func_80acb748(play);
        }
    }

    /// `func_80ACBAB8`: cue 7's id: flying round (1), perched (2), flying on a line (3), hidden
    /// (4), gone (5).
    fn func_80acbab8(&mut self, play: &mut PlayState, cue: &CsCmdActorCue) {
        match cue.action {
            1 => self.change_mode(play, Action::CueOrbit, OwlFunc::Once, Skel::Flying, ANIM_FLY, 0.0),
            2 => {
                self.drawn = true;
                // @bug (game): the perched animation on the flying skeleton.
                self.change_mode(play, Action::WaitDefault, OwlFunc::Once, Skel::Flying, ANIM_PERCH, 0.0);
            }
            3 => {
                self.drawn = true;
                self.change_mode(play, Action::CueLine, OwlFunc::Once, Skel::Flying, ANIM_FLY, 0.0);
            }
            4 => {
                self.drawn = false;
                self.action = Action::WaitDefault;
            }
            5 => self.actor.kill(),
            _ => {}
        }
        self.unk_40a = cue.action as u8;
    }

    /// `func_80ACBC0C`: flying away, turning to `unk_400`, faster to 16, rising to 1000 over his
    /// start; gone 6000 from Link.
    fn fly_away(&mut self) {
        self.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
        if self.actor.xz_dist_to_player > 6000.0 && self.action_flags & 0x80 == 0 {
            self.actor.kill();
        }
        smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_400, 2, 0x80, 0x40);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.actor.speed_xz < 16.0 {
            self.actor.speed_xz += 0.5;
        }
        if (self.unk_3f8 + 1000.0) < self.actor.world_pos.y {
            if self.actor.velocity.y > 0.0 {
                self.actor.velocity.y -= 0.4;
            }
        } else if self.actor.velocity.y < 4.0 {
            self.actor.velocity.y += 0.2;
        }
        self.action_flags |= 8;
    }

    /// `func_80ACBD4C`: the takeoff: turning after frame 10, up and forward after 17 and 45; at
    /// its end flying away, a quarter turn more.
    fn takeoff(&mut self, play: &PlayState) {
        let f = self.skel.cur_frame;
        if f > 10.0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_400, 2, 0x400, 0x40);
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }
        if f > 45.0 {
            self.actor.velocity.y = 2.0;
            self.actor.gravity = 0.0;
            self.actor.speed_xz = 8.0;
        } else if f > 17.0 {
            self.actor.velocity.y = 6.0;
            self.actor.gravity = 0.0;
            self.actor.speed_xz = 4.0;
        }
        if self.action_flags & 1 != 0 {
            self.change_mode(play, Action::FlyAway, OwlFunc::Beats, Skel::Flying, ANIM_FLY, 0.0);
            self.unk_3fe = 6;
            if self.action_flags & 0x40 != 0 {
                self.unk_400 = self.unk_400.wrapping_add(0x2000);
            } else {
                self.unk_400 = self.unk_400.wrapping_sub(0x2000);
            }
        }
        self.action_flags |= 8;
    }

    /// `func_80ACBEA0`: the wings unfolded, the takeoff a quarter turn round.
    fn unfold(&mut self, play: &PlayState) {
        if self.action_flags & 1 != 0 {
            self.unk_3fe = 3;
            self.change_mode(play, Action::Takeoff, OwlFunc::Once, Skel::Flying, ANIM_TAKEOFF, 0.0);
            self.unk_3f8 = self.actor.world_pos.y;
            self.actor.velocity.y = 2.0;
            self.unk_400 = if self.action_flags & 0x40 != 0 { self.actor.world_rot.y.wrapping_add(0x4000) } else { self.actor.world_rot.y.wrapping_sub(0x4000) };
        }
        self.action_flags |= 8;
    }

    /// `func_80ACBF50`: the carrying owl turning, then flying away.
    fn carry_turn(&mut self, play: &PlayState) {
        smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_400, 2, 0x384, 0x258);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.action_flags & 1 != 0 {
            self.change_mode(play, Action::FlyAway, OwlFunc::Beats, Skel::Flying, ANIM_FLY, 0.0);
            self.unk_3fe = 6;
            self.actor.velocity.y = 2.0;
            self.actor.gravity = 0.0;
            self.actor.speed_xz = 4.0;
        }
        self.action_flags |= 8;
    }

    /// `func_80ACC00C`: the carrying owl hovering; Link within 50 (outside a cutscene) starts his
    /// scene's flight (`gLakeHyliaOwlCs`, `gDMTOwlCs`).
    fn carry_wait(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_400, 2, 0x384, 0x258);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.actor.xz_dist_to_player < 50.0 && !play.play_in_cs_mode() {
            let owl_type = self.owl_type();
            log::debug!("{owl_type} owl");
            match owl_type {
                7 => {
                    log::debug!("Demo of SPOT 06 has been completed");
                    play.cs_ctx.segment = play.cutscene_script("gLakeHyliaOwlCs");
                    self.drawn = false;
                }
                8 | 9 => {
                    play.cs_ctx.segment = play.cutscene_script("gDMTOwlCs");
                    self.drawn = false;
                }
                _ => log::error!("En_Owl: owl type {owl_type} can't carry Link (ASSERT in z_en_owl.c)"),
            }
            play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
            play.save.cutscene_trigger = 1;
            play.audio.func_800f44ec(0x14, 0xA);
            self.action = Action::WaitDefault;
            self.unk_40a = 0;
            self.action_flags |= 0x80;
            play.env_statics.time_speed = 0;
        }
        if self.skel.cur_frame >= 37.0 {
            if self.unk_3fe > 0 {
                self.skel.cur_frame = 21.0;
                self.unk_3fe -= 1;
            } else {
                self.action = Action::CarryTurn;
            }
        }
        self.action_flags |= 8;
    }

    /// `func_80ACC23C`: the carrying owl's hop up, then hovering.
    fn carry_hop(&mut self) {
        if self.skel.cur_frame < 20.0 {
            self.actor.speed_xz = 1.5;
        } else {
            self.actor.speed_xz = 0.0;
            smooth_step_to_s(&mut self.actor.world_rot.y, self.unk_400, 2, 0x384, 0x258);
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }
        if self.skel.cur_frame >= 37.0 {
            self.skel.cur_frame = 21.0;
            self.action = Action::CarryWait;
            self.unk_3fe = 5;
            self.actor.velocity.y = 0.0;
            self.actor.gravity = 0.0;
            self.actor.speed_xz = 0.0;
        }
        self.action_flags |= 8;
    }

    /// `func_80ACC30C`: the carrying owl's takeoff.
    fn carry_takeoff(&mut self, play: &PlayState) {
        if self.action_flags & 1 != 0 {
            self.unk_3fe = 3;
            self.change_mode(play, Action::CarryHop, OwlFunc::Once, Skel::Flying, ANIM_TAKEOFF, 0.0);
            self.unk_3f8 = self.actor.world_pos.y;
            self.actor.velocity.y = 0.2;
        }
        self.action_flags |= 8;
    }

    /// `unk_410`: the animation's step.
    fn owl_func(&mut self, play: &PlayState) {
        match self.unk_410 {
            // func_80ACC540.
            OwlFunc::Once => {
                if self.cur_skel().update() {
                    let s = self.cur_skel();
                    if let Some(a) = s.animation.clone() {
                        let last = a.last_frame();
                        s.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
                    }
                    self.action_flags |= 1;
                } else {
                    self.action_flags &= !1;
                }
            }
            // func_80ACC460.
            OwlFunc::Beats => {
                if self.cur_skel().update() {
                    if self.unk_3fe > 0 {
                        self.unk_3fe -= 1;
                        let s = self.cur_skel();
                        if let Some(a) = s.animation.clone() {
                            let last = a.last_frame();
                            s.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
                        }
                    } else {
                        self.unk_3fe = 0xA0;
                        self.unk_410 = OwlFunc::Glide;
                        if let Some(a) = Self::anim(play, ANIM_GLIDE) {
                            let last = a.last_frame();
                            self.cur_skel().change(a, 1.0, 0.0, last, 0, 5.0);
                        }
                    }
                }
            }
            // func_80ACC390.
            OwlFunc::Glide => {
                self.cur_skel().update();
                if self.unk_3fe > 0 {
                    self.unk_3fe -= 1;
                    self.actor.shape_rot.z = (sin_s((self.unk_3fe as i32 * 0x333) as i16) * 1000.0) as i16;
                } else {
                    self.unk_410 = OwlFunc::Beats;
                    self.unk_3fe = 6;
                    if let Some(a) = Self::anim(play, ANIM_FLY) {
                        let last = a.last_frame();
                        self.cur_skel().change(a, 1.0, 0.0, last, 2, 5.0);
                    }
                }
            }
        }
    }

    /// `func_80ACC5CC`: the head turned to its target (round, or forward).
    fn func_80acc5cc(&mut self) -> bool {
        let target = if self.action_flags & 2 != 0 { 0x20 } else { 0 };
        if target == (self.unk_3ee & 0x3F) {
            true
        } else {
            if self.action_flags & 0x20 != 0 {
                self.unk_3ee = self.unk_3ee.wrapping_add(4);
            } else {
                self.unk_3ee = self.unk_3ee.wrapping_sub(4);
            }
            false
        }
    }

    /// `func_80ACC624`: whether his flutter sounds play (in the Desert Colossus, only the
    /// Colossus' owl and two stretches of the cutscene).
    fn func_80acc624(&self, play: &PlayState) -> bool {
        let f = play.cs_ctx.frames;
        if play.scene_id != SCENE_DESERT_COLOSSUS {
            true
        } else if self.owl_type() == 0xA {
            true
        } else {
            (300..=430).contains(&f) || (1080..=1170).contains(&f)
        }
    }

    /// `func_80ACD130`: placed at the cue's start, turned by it.
    fn func_80acd130(&mut self, cue: &CsCmdActorCue) {
        self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        self.actor.world_rot.y = cue.rot[1];
        self.actor.shape_rot.y = cue.rot[1];
        self.actor.shape_rot.z = cue.rot[2];
    }

    /// `func_80ACD1C4`: how far through the cue, at most 1.
    fn func_80acd1c4(play: &PlayState, cue: &CsCmdActorCue) -> f32 {
        oot_game::env::lerp_weight(cue.end_frame, cue.start_frame, play.cs_ctx.frames).min(1.0)
    }

    /// `func_80ACD220`: velocity towards `to` (its y stepped), facing it.
    fn func_80acd220(&mut self, to: Vec3, f: f32) {
        let r = (to - self.actor.world_pos) * f;
        step_to_f(&mut self.actor.velocity.y, r.y, 1.0);
        self.actor.speed_xz = (r.x * r.x + r.z * r.z).sqrt();
        self.actor.world_rot.y = oot_game::target::yaw_to(self.actor.world_pos, to);
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `func_80ACD2CC`: round the cue's start, the angle from `world.rot.z` to the cue's yaw, the
    /// radius the cue's `rot.x` (in tenths of a degree's arc).
    fn func_80acd2cc(&mut self, play: &PlayState, cue: &CsCmdActorCue) {
        let t = Self::func_80acd1c4(play, cue);
        let mut pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let mut angle = (cue.rot[1] as i32) - self.actor.world_rot.z as i32;
        if angle < 0 {
            angle += 0x10000;
        }
        let angle = ((t * angle as f32) + self.actor.world_rot.z as f32) as i32 as i16;
        if self.action_flags & 4 != 0 {
            let mut r = cue.rot[0] as f32;
            r *= 10.0 * (360.0 / 65536.0);
            if r < 0.0 {
                r += 360.0;
            }
            pos.x -= sin_s(angle) * r;
            pos.z += cos_s(angle) * r;
            self.unk_3f8 = r;
            self.actor.world_pos = pos;
            self.drawn = true;
            self.action_flags &= !4;
            self.actor.speed_xz = 0.0;
        } else {
            pos.x -= sin_s(angle) * self.unk_3f8;
            pos.z += cos_s(angle) * self.unk_3f8;
            self.func_80acd220(pos, 1.0);
        }
    }

    /// `func_80ACD4D4`: along the cue from its start to its end.
    fn func_80acd4d4(&mut self, play: &PlayState, cue: &CsCmdActorCue) {
        let t = Self::func_80acd1c4(play, cue);
        let s = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let e = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        let pos = Vec3::new((e.x - s.x) * t + s.x, (e.y - s.y) * t + s.y, (e.z - s.z) * t + s.z);
        self.func_80acd220(pos, 1.0);
    }

    /// `EnOwl_Update`'s head and wings: blinking (eyes shut for a choice), then (when the action
    /// didn't fly) the head's turn, bob and tilt (`unk_3F0`, `unk_3EC`, `unk_3F2`).
    fn update_head(&mut self, play: &mut PlayState) {
        if self.action_flags & 2 != 0 {
            self.eye_tex_index = 2;
        } else {
            // DECR(blinkTimer) == 0.
            let decr = if self.blink_timer == 0 {
                0
            } else {
                self.blink_timer -= 1;
                self.blink_timer
            };
            if decr == 0 {
                self.blink_timer = play.rand.s16_offset(60, 60);
            }
            self.eye_tex_index = self.blink_timer;
            if self.eye_tex_index >= 3 {
                self.eye_tex_index = 0;
            }
        }
        if self.action_flags & 8 == 0 {
            let mut phi_a1: i16 = 0;
            if self.action_flags & 0x10 != 0 {
                match self.unk_404 {
                    0 => {
                        self.unk_404 = 1;
                        self.unk_405 = 6;
                    }
                    1 => {
                        self.unk_405 = self.unk_405.wrapping_sub(1);
                        if self.unk_405 != 0 {
                            phi_a1 = (cos_s((self.unk_405 as i32 * 8192) as i16) * 4096.0) as i16;
                        } else {
                            self.unk_3ee = if self.action_flags & 2 != 0 { 0 } else { 0x20 };
                            if self.action_flags & 0x20 != 0 {
                                self.unk_3ee -= 4;
                            } else {
                                self.unk_3ee += 4;
                            }
                            self.unk_404 += 1;
                        }
                        if self.action_flags & 0x20 != 0 {
                            phi_a1 = phi_a1.wrapping_neg();
                        }
                    }
                    2 => {
                        if self.func_80acc5cc() {
                            self.action_flags &= !0x10;
                            self.unk_406 = (play.rand.zero_float(20.0) as i32 + 0x3C) as u8;
                            self.unk_404 = 0;
                            self.func_80aca6c0(play);
                        }
                    }
                    _ => {}
                }
            } else {
                if self.unk_406 > 0 {
                    self.unk_406 -= 1;
                } else {
                    if self.unk_404 == 0 {
                        if play.rand.zero_one() < 0.3 {
                            self.unk_404 = 4;
                            self.unk_405 = 0xC;
                        } else {
                            self.unk_404 = 1;
                            self.unk_405 = 4;
                        }
                    }
                    self.unk_405 = self.unk_405.wrapping_sub(1);
                    let n = self.unk_405 as i32;
                    match self.unk_404 {
                        1 => {
                            phi_a1 = (sin_s(((-n * 4096) + 0x4000) as i16) * 5000.0) as i16;
                            if n <= 0 {
                                self.unk_405 = (play.rand.zero_float(15.0) + 5.0) as i32 as u8;
                                self.unk_404 = 2;
                            }
                        }
                        2 => {
                            phi_a1 = 0x1388;
                            if n <= 0 {
                                self.unk_404 = 3;
                                self.unk_405 = 4;
                            }
                        }
                        3 => {
                            phi_a1 = (sin_s((n * 4096) as i16) * 5000.0) as i16;
                            if n <= 0 {
                                self.unk_406 = (play.rand.zero_float(20.0) as i32 + 0x3C) as u8;
                                self.unk_404 = 0;
                                self.func_80aca6c0(play);
                            }
                        }
                        4 => {
                            phi_a1 = (sin_s((n * 8192) as i16) * 5000.0) as i16;
                            if n <= 0 {
                                self.unk_406 = (play.rand.zero_float(20.0) as i32 + 0x3C) as u8;
                                self.unk_404 = 0;
                                self.func_80aca6c0(play);
                            }
                        }
                        _ => {}
                    }
                    if self.action_flags & 0x20 != 0 {
                        phi_a1 = phi_a1.wrapping_neg();
                    }
                }
                if self.unk_409 > 0 {
                    self.unk_409 -= 1;
                } else {
                    self.unk_408 = self.unk_408.wrapping_sub(1);
                    let n = self.unk_408 as i32;
                    match self.unk_407 {
                        0 => {
                            self.unk_3f2 = ((-n * 0x5DC) + 0x1770) as i16;
                            if n <= 0 {
                                self.unk_407 = 1;
                                self.unk_408 = (play.rand.zero_float(15.0) + 5.0) as i32 as u8;
                            }
                        }
                        1 => {
                            self.unk_3f2 = 0x1770;
                            if n <= 0 {
                                self.unk_407 = 2;
                                self.unk_408 = 4;
                            }
                        }
                        2 => {
                            self.unk_3f2 = (n * 0x5DC) as i16;
                            if n <= 0 {
                                self.unk_407 = 0;
                                self.unk_408 = 4;
                                self.unk_409 = (play.rand.zero_float(40.0) as i32 + 0xA0) as u8;
                            }
                        }
                        _ => {}
                    }
                }
            }
            // (u16)((unk_3EE << 2) << 8) + phi_a1.
            self.unk_3f0 = (((self.unk_3ee as i32) << 2 << 8) as u16 as i32 + phi_a1 as i32) as i16;
            self.unk_3ec = ((self.unk_3f0 as i32).abs() >> 3) as i16;
        } else {
            self.unk_3f2 = 0;
            self.unk_3f0 = if self.action_flags & 2 != 0 { -0x8000 } else { 0 };
            self.unk_3ec = ((self.unk_3f0 as i32).abs() >> 3) as i16;
        }
    }

    /// The posed limbs, `EnOwl_OverrideLimbDraw`'s turns applied (limbs 2 to 5).
    fn pose(&self, skeleton: &eng_anim::skeleton::Skeleton, joints: &[[i16; 3]], a: &[i16]) -> Vec<Mat4> {
        let (unk_3f0, unk_3ec, unk_3f2, flying) = (a[0], a[1], a[2], a[3] != 0);
        skeleton.pose_override(joints, |limb, _pos, rot| {
            match limb {
                3 => {
                    rot[0] = rot[0].wrapping_add(unk_3f0);
                    rot[2] = rot[2].wrapping_add(unk_3ec);
                    rot[2] = rot[2].wrapping_sub(unk_3f2);
                }
                2 => rot[2] = rot[2].wrapping_add(unk_3f2),
                4 if !flying => rot[1] = rot[1].wrapping_sub((unk_3ec as f32 * 1.5) as i16),
                5 if !flying => rot[1] = rot[1].wrapping_add((unk_3ec as f32 * 1.5) as i16),
                _ => {}
            }
            Mat4::IDENTITY
        })
    }
}

/// The bakes: each skeleton with each eye (`gSPSegment(8, eyeTextures[eyeTexIndex])`), after
/// `Gfx_SetupDL_37Opa`.
const EYE_TEXTURES: [&str; 3] = ["gObjOwlEyeOpenTex", "gObjOwlEyeHalfTex", "gObjOwlEyeClosedTex"];
const SKELETONS: [&str; 2] = ["gOwlFlyingSkel", "gOwlPerchingSkel"];
const SEG_EYES: u8 = 0x08;
const SEG_SETUP: u8 = 0x09;

pub fn bake_name(skel: usize, eye: usize) -> String {
    format!("En_Owl/{}_eye{eye}", if skel == 0 { "flying" } else { "perching" })
}

/// `SETUPDL_37` (`z_rcp.c`): texture off, `PRIMITIVE * SHADE` with the prim alpha, then `COMBINED`;
/// `SETUPDL_25`'s other modes and geometry.
fn setup_dl_37() -> Vec<(u32, u32)> {
    use oot_game::gbi::*;
    let mut d = Dl::default();
    d.pipe_sync();
    d.0.push((0xD700_0000, 0xFFFF_FFFF));
    d.combine_lerp([cc_ab::PRIMITIVE, cc_ab::ZERO, cc_c::SHADE, cc_d::ZERO, ac::ZERO, ac::ZERO, ac::ZERO, ac::PRIMITIVE], setup_dl::PASS2);
    // G_AD_NOTPATTERN | G_TC_FILT | G_TF_BILERP | G_TP_PERSP | G_CYC_2CYCLE; G_RM_FOG_SHADE_A |
    // G_RM_AA_ZB_OPA_SURF2.
    d.0.push((0xEF00_0000 | 0x10 | (6 << 9) | (2 << 12) | (1 << 19) | (1 << 20), 0xC811_2078));
    // G_ZBUFFER | G_SHADE | G_CULL_BACK | G_FOG | G_LIGHTING | G_SHADING_SMOOTH.
    d.0.push((0xD900_0000, 0x0023_0405));
    d.end();
    d.0
}

/// Its bakes.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (s, skel) in SKELETONS.iter().enumerate() {
        for (e, tex) in EYE_TEXTURES.iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(s, e),
                object: OBJECT.into(),
                segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }), (SEG_SETUP, BakeSegment::Commands(setup_dl_37()))],
                prelude: vec![SEG_SETUP],
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: (*skel).into(), limbs: Vec::new() },
            });
        }
    }
    v
}

impl ActorImpl for EnOwl {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnOwl_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 10.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        self.owl_func(play);
        self.action_flags &= !8;
        match self.action {
            Action::WaitDefault => self.wait_default(play),
            Action::WaitOutsideKokiri
            | Action::WaitHyruleCastle
            | Action::WaitKakariko
            | Action::WaitGerudo
            | Action::WaitLakeHylia
            | Action::WaitZoraRiver
            | Action::WaitColossus
            | Action::WaitLWPreSaria
            | Action::WaitLWPostSaria => self.wait(play),
            Action::WaitHyliaShortcut => self.wait_hylia_shortcut(play),
            Action::WaitDeathMountainShortcut => self.wait_death_mountain_shortcut(play),
            Action::ConfirmKokiriMessage => self.confirm_kokiri_message(play),
            Action::TalkEnd => self.talk_end(play),
            Action::TalkEndOrTurn => self.talk_end_or_turn(play),
            Action::TurnBack => self.func_80aca690(play),
            Action::Choice(w) => self.owl_choice(play, w),
            Action::Repeat(6) => self.func_80acaac0(play),
            Action::Repeat(w) => self.owl_repeat(play, w),
            // func_80ACB03C.
            Action::ZoraRiverEnd => {
                self.halt_player(play);
                if self.textbox_is_closing(play) {
                    Self::stop_fanfare(play);
                    self.func_80aca62c(play);
                    self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                }
            }
            // func_80ACB148.
            Action::HyliaShortcutEnd => {
                if self.textbox_is_closing(play) {
                    Self::stop_fanfare(play);
                    self.func_80aca5c8(play);
                    self.action = Action::CarryTakeoff;
                    play.flags.set_switch(0x23);
                }
            }
            // func_80ACB22C.
            Action::DeathMountainEnd => {
                if self.textbox_is_closing(play) {
                    Self::stop_fanfare(play);
                    self.func_80aca5c8(play);
                    self.action = Action::CarryTakeoff;
                }
            }
            // func_80ACB274.
            Action::DeathMountainAgain => {
                if self.textbox_is_closing(play) {
                    Self::stop_fanfare(play);
                    self.action = Action::WaitDeathMountainShortcut;
                }
            }
            Action::ColossusChoice => self.colossus_choice(play),
            Action::CueOrbit => self.cue_flight(play, true),
            Action::CueLine => self.cue_flight(play, false),
            Action::Unfold => self.unfold(play),
            Action::Takeoff => self.takeoff(play),
            Action::FlyAway => self.fly_away(),
            Action::CarryTakeoff => self.carry_takeoff(play),
            Action::CarryHop => self.carry_hop(),
            Action::CarryWait => self.carry_wait(play),
            Action::CarryTurn => self.carry_turn(play),
        }
        if self.actor.killed {
            log::debug!("Owl disappears!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
            return;
        }
        if self.action_flags & 0x80 == 0 && self.func_80acc624(play) {
            let f = self.skel.cur_frame;
            let flutter = (self.skel.is(ANIM_TAKEOFF) && [2.0, 9.0, 23.0, 40.0, 58.0].contains(&f)) || (self.skel.is(ANIM_FLY) && f == 4.0);
            if flutter {
                audio_play_actor_sfx2(play, NA_SE_EN_OWL_FLUTTER);
            }
        }
        if self.drawn {
            self.actor.move_forward();
        }
        self.update_head(play);
    }

    /// `EnOwl_PostLimbUpdate`'s focus: 1400 along limb 3 (700 along, 400 up for a choice).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let Some(skeletons) = &self.skeletons else { return };
        if !self.drawn || self.actor.killed {
            return;
        }
        let (skeleton, joints) = match self.cur {
            Skel::Flying => (&skeletons[0], &self.skel.joint_table),
            Skel::Perching => (&skeletons[1], &self.skel2.joint_table),
        };
        let a = [self.unk_3f0, self.unk_3ec, self.unk_3f2, (self.action_flags & 8 != 0) as i16];
        let bones = self.pose(skeleton, joints, &a);
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        let v = if self.action_flags & 2 != 0 { Vec3::new(700.0, 400.0, 0.0) } else { Vec3::new(1400.0, 0.0, 0.0) };
        if let Some(head) = bones.get(2) {
            self.actor.focus_pos = (m * *head).transform_point3(v);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        let joints = match self.cur {
            Skel::Flying => &self.skel.joint_table,
            Skel::Perching => &self.skel2.joint_table,
        };
        rs.joints = Some(eng_anim::anim::JointTable { rot: joints.clone(), face: 0 });
        rs.angles = vec![self.unk_3f0, self.unk_3ec, self.unk_3f2, (self.action_flags & 8 != 0) as i16];
        rs.switches = vec![(self.cur == Skel::Perching) as u32, self.eye_tex_index.clamp(0, 2) as u32, self.drawn as u32];
        rs
    }

    /// `EnOwl_Draw`: `Gfx_SetupDL_37Opa`, the eyes on 8, the current skeleton with
    /// `EnOwl_OverrideLimbDraw`'s turns; the circle shadow.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeletons), Some(joints)) = (&self.skeletons, &rs.joints) else { return };
        if rs.switches.len() < 3 || rs.angles.len() < 4 || rs.switches[2] == 0 {
            return;
        }
        let s = rs.switches[0] as usize;
        let bones = self.pose(&skeletons[s], &joints.rot, &rs.angles);
        let cmd = DrawCmd {
            mesh: MeshKey::named(keys::bake(&bake_name(s, rs.switches[1] as usize))),
            transform: actor_draw_matrix(rs),
            bones,
            params: eng_gfx::DrawParams { segments: Some(SegmentValues::default()), ..Default::default() },
        };
        out.opa.push(cmd);
        // ActorShadow_DrawCircle (shadowScale 36): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(36.0, 1.0, 36.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
    }

    /// `EnOwl_Destroy`.
    fn destroy(&mut self, _play: &mut PlayState) {}

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
