//! `En_Ma3` (`ovl_En_Ma3/z_en_ma3.c`): adult Malon by the ranch's corral after Epona, with her
//! obstacle course.
//!
//! She is there (`func_80AA2EC8`) for an adult with Epona (`EVENTCHKINF_EPONA_OBTAINED`): killed
//! for a child or before Epona. Placed on the ranch's adult day layer (2) by the corral's gate.
//!
//! Her texts (`EnMa3_GetTextId`): the first time 0x2000 (her thanks; its closing sets
//! `INFTABLE_B8`). After, on foot with no horse within 1,200 (`Actor_FindNearby`, `ACTOR_EN_HORSE`)
//! 0x2001; with a horse near or riding, 0x2002 the first time (it runs on into 0x2003; sets
//! `INFTABLE_B9`) or 0x2003: the course offered. Its choice (`EnMa3_UpdateTalkState`) sets
//! `INFTABLE_B9`; yes: 0x2091 (the cow already won), 0x2092 (no high score yet) or 0x2090 (her
//! record and Link's best). Its event ends the talk: to `ENTR_LON_LON_RANCH_0` with `CS_INDEX_0`
//! (the course: `En_Horse_Game_Check` type 4) in a black starburst circle, `EVENTINF_HORSES_0A`
//! set, the timer `TIMER_STATE_UP_FREEZE`.
//!
//! The course's results, with `EVENTINF_HORSES_0A` set (back at `ENTR_LON_LON_RANCH_7`, beside
//! her): she talks at once (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`, set by the text's choice). Over
//! 210 seconds (a foul or time up: the referee sets 240) 0x208E; else the high score
//! (`HIGH_SCORE(HS_HORSE_RACE)`, the save's `highScores[3]`, seconds) is made 180 if unset or over;
//! under 50 seconds without the cow 0x208F (its closing sets `EVENTCHKINF_HORSE_RACE_COW_UNLOCK`:
//! the cow in Link's house); better than the best 0x2012; else 0x2004. Each result's closing keeps
//! the best time (0x208E's doesn't), clears `EVENTINF_HORSES_0A` and the auto-accept, and stops the
//! timer (`TIMER_STATE_STOP`).
//!
//! Her head and chest turn to Link (`func_80AA2E54`); she blinks (`EnMa3_UpdateEyes`). She has no
//! singing animation of her own (`ENMA3_ANIM_0`, the idle, whatever `func_80AA2EC8` says), but her
//! update turns the Lon Lon song's voice off while she talks and on again after
//! (`Audio_ToggleMalonSinging`: `OOT_VERSION >= PAL_1_0`, on for gc-eu-mq-dbg) and her draw sets its
//! volume by the camera's distance (`Audio_UpdateMalonSinging`).
//!
//! Drawn as `En_Ma2` draws (`EnMa3_Draw`, `EnMa3_OverrideLimbDraw`, `EnMa3_PostLimbDraw` are the
//! same): the model is `en_ma2`'s ([`MalonAdultModel`], its bakes and draw).
//!
//! The PAL N64 versions (`OOT_PAL_N64`, off for this ROM) talk through `EnMa3_UpdateTalking`, which
//! is `Npc_UpdateTalking` that also won't offer a talk while the sub-timer runs with under 6
//! seconds left. Not ported: this ROM uses `Npc_UpdateTalking`.
//!
//! `EnMa3_GetTextId` changes things as it picks (the auto-accept, the high score made 180), which
//! `oot_game::npc::talk_update`'s text closure can't: `npc_update_talking` here is the same
//! `Npc_UpdateTalking` with a mutable one.
//!
//! Params: `params` unused (placed -1, read by nothing).
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (where she is); reads and sets INFTABLE_B8 (text 0x2000
//! closed) and INFTABLE_B9 (text 0x2002 closed, or the course's choice); reads and sets
//! EVENTCHKINF_HORSE_RACE_COW_UNLOCK (text 0x208F closed: the course under 50 seconds); reads, sets
//! and clears EVENTINF_HORSES_0A (the course started; cleared as its result's text closes; also
//! `EVENTINF_CUCCO_GAME_FINISHED`'s bit, Anju's, an alias in `save.h`); reads and writes
//! `HIGH_SCORE(HS_HORSE_RACE)`.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_NPC, ActorImpl, ActorProfile, actor_find_nearby};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::collision_check::*;
use oot_game::horse::{ACTOR_EN_HORSE, ENTR_LON_LON_RANCH_0, TCA_STARBURST, TCC_BLACK, TCS_FAST, trans_type_circle};
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_CLOSING, TEXT_STATE_EVENT};
use oot_game::npc::{NpcTrack, offer_talk, process_talk_request, track_point};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{TIMER_STATE_STOP, TIMER_STATE_UP_FREEZE};
use oot_game::skelanime_std::*;
use oot_game::transition::TRANS_TRIGGER_START;
use oot_game::{event_chk_inf, event_inf, inf_table};

use crate::en_ma2::{
    COL_CHK_INFO_INIT, CYLINDER_INIT, MALON_ADULT_ANIM_IDLE, MALON_ADULT_LIMB_MAX, MalonAdultLook, MalonAdultModel, NPC_TALK_STATE_ACTION, NPC_TALK_STATE_IDLE, NPC_TALK_STATE_TALKING,
    NPC_TRACKING_NONE, NPC_TRACKING_PLAYER_AUTO_TURN, SING_ANIM, decr, draw_malon_adult, draw_update_malon_adult, player_state1, render_state_malon_adult,
};
use crate::player::STATE1_23;

/// `ACTOR_EN_MA3` (`actor_table.h`: 0x01C5).
pub const ACTOR_EN_MA3: i16 = 0x01C5;
pub const OBJECT: &str = crate::en_ma2::OBJECT;

/// `En_Ma3_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_MA3,
    name: "En_Ma3",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed -1; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed -1; read by nothing");

// `EVENTCHKINF_*`, `INFTABLE_*`, `EVENTINF_*` (`save.h`).
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
pub const EVENTCHKINF_HORSE_RACE_COW_UNLOCK: u16 = 0x1E;
pub const INFTABLE_B8: u16 = 0xB8;
pub const INFTABLE_B9: u16 = 0xB9;
pub const EVENTINF_HORSES_0A: u16 = 0x0A;

/// `HS_HORSE_RACE` (`save.h`: 0x03): `HIGH_SCORE(HS_HORSE_RACE)` is `highScores[3]`.
pub const HS_HORSE_RACE: usize = 3;
/// `CS_INDEX_0` (`save.h`: 0xFFF0): the ranch's cutscene layer 4, the course.
const CS_INDEX_0: u16 = 0xFFF0;

/// En_Ma3's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("where she is: killed before Epona"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_HORSE_RACE_COW_UNLOCK)).reads().sets().note("text 0x208F closed: the course under 50 seconds (the cow in Link's house)"),
        FlagUse::new(inf_table!(INFTABLE_B8)).reads().sets().note("text 0x2000 closed"),
        FlagUse::new(inf_table!(INFTABLE_B9)).reads().sets().note("text 0x2002 closed, or the course's choice"),
        FlagUse::new(event_inf!(EVENTINF_HORSES_0A)).reads().sets().clears().note("the course started (0x2003's event); cleared as its result's text closes"),
        FlagUse::new(Flag::Other("HIGH_SCORE(HS_HORSE_RACE): the save's highScores[3], the course's best in seconds")).reads().sets().note("made 180 when unset or over as a result is picked; the best kept as it closes"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x2000).cond("the first time").note("thank you for the other day; the fairy boy from the forest; Mr. Ingo is himself again"),
        TextLine::new(0x2001).cond("after 0x2000, on foot with no horse within 1,200").note("how is Epona? play Epona's Song and she'll come"),
        TextLine::new(0x2002).cond("with a horse near or riding, the first time; runs on into 0x2003").note("Epona looks great; you tamed her"),
        TextLine::new(0x2003).cond("with a horse near or riding").note("an obstacle course, two laps: Let's go / Don't do it"),
        TextLine::new(0x2090).cond("\"Let's go\" with a high score, before the cow").note("the record is 50 seconds; your best time; beat it for a present"),
        TextLine::new(0x2091).cond("\"Let's go\" once the cow is won").note("your best time; how fast can you go?"),
        TextLine::new(0x2092).cond("\"Let's go\" with no high score yet").note("the current record is 50 seconds; beat it for a present"),
        TextLine::new(0x208E).cond("the course's result: over 210 seconds (a foul or time up)").note("approach the fences head on; mount Epona to try again"),
        TextLine::new(0x208F).cond("the course's result: under 50 seconds, the first time").note("the new course record! a present delivered to your house"),
        TextLine::new(0x2012).cond("the course's result: better than the best").note("great! your time; you and Epona are a great team"),
        TextLine::new(0x2004).cond("the course's result: not better than the best").note("your time; train harder"),
    ],
};

/// `TRANS_TYPE_CIRCLE(TCA_STARBURST, TCC_BLACK, TCS_FAST)` (`transition.h`).
const TRANS_TYPE_CIRCLE_STARBURST_BLACK_FAST: u8 = trans_type_circle(TCA_STARBURST, TCC_BLACK, TCS_FAST);
/// `SCREEN_WIDTH`, `SCREEN_HEIGHT`.
const SCREEN_WIDTH: i16 = 320;
const SCREEN_HEIGHT: i16 = 240;

/// `actionFunc`: `func_80AA3200` is her only one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Func80aa3200,
}

pub struct EnMa3 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub model: Option<MalonAdultModel>,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub interact_info: NpcTrack,
    /// `unk_208`: unused.
    pub unk_208: i16,
    /// `isNotSinging`: her voice turned off while she talks.
    pub is_not_singing: bool,
    pub blink_timer: i16,
    pub eye_index: i16,
    pub mouth_index: i16,
    /// `unk_212`: never written (zeros).
    pub unk_212: [[i16; 3]; MALON_ADULT_LIMB_MAX],
}

impl EnMa3 {
    /// `EnMa3_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(model) = MalonAdultModel::load(play, PROFILE.name) else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let mut this = EnMa3 {
            // SkelAnime_InitFlex(.., &gMalonAdultSkel, NULL, ..).
            skel: SkelAnimeStd::init_flex(model.skeleton.limbs.len(), None),
            model: Some(model),
            action: Action::Func80aa3200,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            unk_208: 0,
            is_not_singing: false,
            blink_timer: 0,
            eye_index: 0,
            mouth_index: 0,
            unk_212: [[0; 3]; MALON_ADULT_LIMB_MAX],
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 18.0);
        // DamageTable_Get(22).
        this.actor.col_chk_info.set_info2(Some(&crate::en_ma1::S_DAMAGE_TABLE_PRESET_22), &COL_CHK_INFO_INIT);

        match Self::func_80aa2ec8(play) {
            // ENMA3_ANIM_0 for both.
            0 | 1 => {
                this.change_anim(MALON_ADULT_ANIM_IDLE);
                this.action = Action::Func80aa3200;
            }
            _ => {
                this.actor.kill();
                return Box::new(this);
            }
        }

        this.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        this.actor.scale = Vec3::splat(0.01);
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        Box::new(this)
    }

    /// `EnMa3_GetTextId`. With the course's result due it also makes her talk at once and puts the
    /// high score at 180 when unset or over.
    pub fn get_text_id(play: &mut PlayState, actor: &mut Actor) -> u16 {
        if !play.save.get_inf_table(INFTABLE_B8) {
            return 0x2000;
        }
        if play.save.get_event_inf(EVENTINF_HORSES_0A) {
            // gSaveContext.timerSeconds = gSaveContext.timerSeconds: a no-op.
            actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            let s = &mut play.save;
            let timer_seconds = s.timer_seconds as i32;
            if timer_seconds > 210 {
                return 0x208E;
            }
            if s.high_scores[HS_HORSE_RACE] == 0 || s.high_scores[HS_HORSE_RACE] >= 180 {
                s.high_scores[HS_HORSE_RACE] = 180;
            }
            if !s.get_event_chk_inf(EVENTCHKINF_HORSE_RACE_COW_UNLOCK) && timer_seconds < 50 {
                return 0x208F;
            }
            if s.high_scores[HS_HORSE_RACE] > timer_seconds {
                return 0x2012;
            }
            return 0x2004;
        }
        // Actor_FindNearby(play, thisx, ACTOR_EN_HORSE, 1 (ACTORCAT_BG), 1200).
        let me = play.cur_actor;
        if player_state1(play) & STATE1_23 == 0 && actor_find_nearby(play, actor, me, ACTOR_EN_HORSE, ACTORCAT_BG, 1200.0).is_none() {
            return 0x2001;
        }
        if !play.save.get_inf_table(INFTABLE_B9) {
            return 0x2002;
        }
        0x2003
    }

    /// `EnMa3_UpdateTalkState`.
    pub fn update_talk_state(play: &mut PlayState, actor: &mut Actor) -> i16 {
        let mut talk_state = NPC_TALK_STATE_TALKING;
        match play.message_state() {
            TEXT_STATE_EVENT => {
                if play.message_should_advance() {
                    play.transition.next_entrance_index = ENTR_LON_LON_RANCH_0;
                    play.save.next_cutscene_index = CS_INDEX_0;
                    play.transition.ty = TRANS_TYPE_CIRCLE_STARBURST_BLACK_FAST;
                    play.transition.trigger = TRANS_TRIGGER_START;
                    play.save.set_event_inf(EVENTINF_HORSES_0A);
                    play.save.timer_state = TIMER_STATE_UP_FREEZE;
                }
            }
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() {
                    play.save.set_inf_table(INFTABLE_B9);
                    if play.msg_ctx.choice_index == 0 {
                        if play.save.get_event_chk_inf(EVENTCHKINF_HORSE_RACE_COW_UNLOCK) {
                            play.continue_textbox(0x2091);
                        } else if play.save.high_scores[HS_HORSE_RACE] == 0 {
                            play.continue_textbox(0x2092);
                        } else {
                            play.continue_textbox(0x2090);
                        }
                    }
                }
            }
            TEXT_STATE_CLOSING => {
                let s = &mut play.save;
                match actor.text_id {
                    0x2000 => {
                        s.set_inf_table(INFTABLE_B8);
                        talk_state = NPC_TALK_STATE_IDLE;
                    }
                    0x208F | 0x2004 | 0x2012 | 0x208E => {
                        if actor.text_id == 0x208F {
                            s.set_event_chk_inf(EVENTCHKINF_HORSE_RACE_COW_UNLOCK);
                        }
                        // 0x208F falls through to 0x2004 and 0x2012: the best kept.
                        if actor.text_id != 0x208E && s.high_scores[HS_HORSE_RACE] > s.timer_seconds as i32 {
                            s.high_scores[HS_HORSE_RACE] = s.timer_seconds as i32;
                        }
                        // And on to 0x208E.
                        s.clear_event_inf(EVENTINF_HORSES_0A);
                        actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                        talk_state = NPC_TALK_STATE_IDLE;
                        s.timer_state = TIMER_STATE_STOP;
                    }
                    0x2002 | 0x2003 => {
                        if actor.text_id == 0x2002 {
                            s.set_inf_table(INFTABLE_B9);
                        }
                        // 0x2002 falls through to 0x2003.
                        if !s.get_event_inf(EVENTINF_HORSES_0A) {
                            talk_state = NPC_TALK_STATE_IDLE;
                        }
                    }
                    _ => talk_state = NPC_TALK_STATE_IDLE,
                }
            }
            _ => {}
        }
        talk_state
    }

    /// `func_80AA2E54`: the tracking, Link's position; none while she sings and isn't talking.
    fn func_80aa2e54(&mut self, play: &mut PlayState) {
        let tracking_mode = if self.interact_info.talk_state == NPC_TALK_STATE_IDLE && self.skel.is(SING_ANIM) { NPC_TRACKING_NONE } else { NPC_TRACKING_PLAYER_AUTO_TURN };
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.eye_height = 0.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 0, tracking_mode);
        self.interact_info = t;
    }

    /// `func_80AA2EC8`: 2 (killed) for a child or before Epona; 1 with the course's result due
    /// (`EVENTINF_HORSES_0A`); else 0.
    pub fn func_80aa2ec8(play: &PlayState) -> i32 {
        let s = &play.save;
        // LINK_IS_CHILD.
        if !s.adult {
            return 2;
        }
        if !s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
            return 2;
        }
        if s.get_event_inf(EVENTINF_HORSES_0A) {
            return 1;
        }
        0
    }

    /// `func_80AA2F28`: singing and not talking, her blink held and, once her eyes are closed, her
    /// mouth happy. (She never sings: `ENMA3_ANIM_0` only.)
    fn func_80aa2f28(&mut self) -> bool {
        if !self.skel.is(SING_ANIM) {
            return false;
        }
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            return false;
        }
        self.blink_timer = 0;
        if self.eye_index != 2 {
            return false;
        }
        self.mouth_index = 2;
        true
    }

    /// `EnMa3_UpdateEyes`.
    fn update_eyes(&mut self, play: &mut PlayState) {
        if !self.func_80aa2f28() && decr(&mut self.blink_timer) == 0 {
            self.eye_index += 1;
            if self.eye_index >= 3 {
                self.blink_timer = play.rand.s16_offset(30, 30);
                self.eye_index = 0;
            }
        }
    }

    /// `EnMa3_ChangeAnim`.
    fn change_anim(&mut self, index: usize) {
        if let Some(m) = &self.model {
            m.change_anim(&mut self.skel, index);
        }
    }

    /// `func_80AA3200`: the talk's action ends it.
    fn func_80aa3200(&mut self, _play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        }
    }

    fn look(&self) -> MalonAdultLook<'_> {
        MalonAdultLook { skel: &self.skel, interact_info: &self.interact_info, upper_body_rot: &self.unk_212, eye_index: self.eye_index, mouth_index: self.mouth_index }
    }
}

/// `Npc_UpdateTalking` (`z_actor.c`), as `oot_game::npc::talk_update`, with a text function that
/// may change the save and the actor (`EnMa3_GetTextId` does).
fn npc_update_talking(play: &mut PlayState, actor: &mut Actor, talk_state: &mut i16, interact_range: f32, get_text_id: impl FnOnce(&mut PlayState, &mut Actor) -> u16, update_talk_state: impl FnOnce(&mut PlayState, &mut Actor) -> i16) -> bool {
    if process_talk_request(actor) {
        *talk_state = NPC_TALK_STATE_TALKING;
        return true;
    }
    if *talk_state != NPC_TALK_STATE_IDLE {
        *talk_state = update_talk_state(play, actor);
        return false;
    }
    let (x, y) = oot_game::target::actor_screen_pos(play.view_proj, actor);
    if x < 0 || x > SCREEN_WIDTH || y < 0 || y > SCREEN_HEIGHT {
        // Actor is offscreen.
        return false;
    }
    if !offer_talk(play, actor, interact_range) {
        return false;
    }
    actor.text_id = get_text_id(play, actor);
    false
}

impl ActorImpl for EnMa3 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMa3_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.skel.update();
        self.update_eyes(play);
        match self.action {
            Action::Func80aa3200 => self.func_80aa3200(play),
        }
        self.func_80aa2e54(play);

        // !OOT_PAL_N64: Npc_UpdateTalking(.., collider.dim.radius + 150, EnMa3_GetTextId,
        // EnMa3_UpdateTalkState).
        let mut talk_state = self.interact_info.talk_state;
        let range = self.collider.dim.radius as f32 + 150.0;
        npc_update_talking(play, &mut self.actor, &mut talk_state, range, Self::get_text_id, Self::update_talk_state);
        self.interact_info.talk_state = talk_state;

        // OOT_VERSION >= PAL_1_0.
        if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            if self.is_not_singing {
                // Turn on singing.
                play.audio.toggle_malon_singing(false);
                self.is_not_singing = false;
            }
        } else if !self.is_not_singing {
            // Turn off singing.
            play.audio.toggle_malon_singing(true);
            self.is_not_singing = true;
        }
    }
    /// `EnMa3_Draw`'s `Audio_UpdateMalonSinging`, and `EnMa3_PostLimbDraw`'s focus.
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(model) = self.model.take() else { return };
        let look = MalonAdultLook { skel: &self.skel, interact_info: &self.interact_info, upper_body_rot: &self.unk_212, eye_index: self.eye_index, mouth_index: self.mouth_index };
        draw_update_malon_adult(play, &mut self.actor, &model, &look);
        self.model = Some(model);
    }
    fn render_state(&self) -> RenderState {
        render_state_malon_adult(&self.actor, &self.look())
    }
    /// `EnMa3_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if let Some(model) = &self.model {
            draw_malon_adult(model, rs, out);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_circle_transition_is_starburst_black_fast() {
        // TRANS_TYPE_CIRCLE(3, 0, 0) = 0x20 | (3 << 1).
        assert_eq!(TRANS_TYPE_CIRCLE_STARBURST_BLACK_FAST, 0x26);
    }
}
