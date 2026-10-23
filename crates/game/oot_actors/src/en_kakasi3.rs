//! `En_Kakasi3` (`ovl_En_Kakasi3/z_en_kakasi3.c`): Bonooru, the scarecrow by Lake Hylia's lab who
//! learns the scarecrow's (spawn) song from a child and hears it back from the adult (Lake
//! Hylia's room 0 places him at (-154, -1293, 2888), params 0xFFFF). `object_ka`'s skeleton,
//! drawn and danced as `En_Kakasi` (`crate::en_kakasi::Dance`: his `func_80A90E28` and
//! `func_80A90EBC` are its code).
//!
//! - **Standing** (`func_80A911F0`, `func_80A91284`, `func_80A91348`): his text 0x40A1; a child
//!   with the song set (`scarecrowSpawnSongSet`) 0x40A0 (an event: talked to, he plays it back,
//!   `func_80A91760`); an adult with it set 0x40A2 once he's heard it this visit (`unk_195`), else
//!   0x40A3. Talks within 100, Link within 120 and in front of him (0x4300).
//! - **A child's recording**: the ocarina out in front of him, the one-point cutscene 2260 and
//!   0x40A4 (`func_80A915B8`); at its event `OCARINA_ACTION_SCARECROW_SPAWN_RECORDING`
//!   (`func_80A91620`), dancing to the notes. Taken (`OCARINA_MODE_03`, the box closed): 0x40A5,
//!   Player held (`func_80A91A90`), the song's mode back to 4 and Link let go as it closes.
//!   Refused (`OCARINA_MODE_04`, or 5 to 10): the cutscene ended, back to standing.
//! - **A child's playback** (0x40A0's event): 2280 and `OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK`,
//!   dancing by Rand until `OCARINA_MODE_0F` (`func_80A917FC`).
//! - **An adult's check**, the song set and not yet heard: the ocarina out, 2260 and 0x40A8
//!   (`func_80A9187C`); at its event `OCARINA_ACTION_CHECK_SCARECROW_SPAWN` (`func_80A918E4`).
//!   Played (`OCARINA_MODE_03`): heard (`unk_195`), 0x40A7; as it closes `EVENTCHKINF_9C` is set
//!   (Pierre comes up to the song from then on: `En_Kakasi2`). Wrong (4, or 5 to 10): 0x40A6.
//!
//! The whole overlay is ported (its `BREG(2)`, `BREG(3)` prints aside). No shadow.
//!
//! Params: `params` unused (placed 0xFFFF).
//! Flags: reads `scarecrowSpawnSongSet` (his texts; set by the message box's recording); reads and sets
//! EVENTCHKINF_9C (the song played back to him by the adult, as its text closes).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::camera::{CAM_ID_MAIN, CAM_ID_NONE};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::message::{
    MSGMODE_NONE, MSGMODE_PAUSED, OCARINA_ACTION_CHECK_SCARECROW_SPAWN, OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK, OCARINA_ACTION_SCARECROW_SPAWN_RECORDING, OCARINA_MODE_01, OCARINA_MODE_03, OCARINA_MODE_04, OCARINA_MODE_0B, OCARINA_MODE_0F,
    TEXT_STATE_DONE, TEXT_STATE_EVENT,
};
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, Anim, SkelAnimeStd};

use crate::en_kakasi::{ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER, ATTENTION_RANGE_6, Dance, OBJECT, PLAYER_CSACTION_7, PLAYER_CSACTION_8, cam_actor, camera_is_null, camera_set_finished, change_anim, draw_skel, load, message_close_textbox, player_state2, set_player_state2, skel_render_state};
use crate::player::{STATE2_23, STATE2_24};

/// `ACTOR_EN_KAKASI3` (`actor_table.h`: 0x01CA).
pub const ACTOR_EN_KAKASI3: i16 = 0x01CA;

/// `En_Kakasi3_Profile`: `ACTORCAT_NPC`, attention-enabled, friendly, updating during the ocarina.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_KAKASI3,
    name: "En_Kakasi3",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed 0xFFFF; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed 0xFFFF; read by nothing");

/// En_Kakasi3's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(Flag::Other("scarecrowSpawnSongSet")).reads().note("his texts; set by the message box once the song's recorded"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_9C)).reads().sets().note("the adult's played the song back to him: Pierre (En_Kakasi2) comes up to it"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x40A0).cond("a child, the song set").note("I'm Bonooru; this is the song you tooted last time (its event plays it back)"),
        TextLine::new(0x40A1).cond("no song set").note("I am Bonooru, the scarecrow musical genius"),
        TextLine::new(0x40A2).cond("an adult, the song set, played back to him this visit").note("play the usual tune to call Pierre"),
        TextLine::new(0x40A3).cond("an adult, the song set, not played back this visit").note("you grew; if you have an ocarina, play that song"),
        TextLine::new(0x40A4).cond("a child taking out the ocarina before him").note("lay a tune on me (its event starts the recording)"),
        TextLine::new(0x40A5).cond("the child's song taken").note("not bad, I will remember it"),
        TextLine::new(0x40A6).cond("the adult's song wrong").note("play it like that and you'll never move me"),
        TextLine::new(0x40A7).cond("the adult's song right").note("that's the tune; call Pierre with it (EVENTCHKINF_9C set as it closes)"),
        TextLine::new(0x40A8).cond("an adult taking out the ocarina before him, the song set").note("play that song with it (its event starts the check)"),
    ],
};

/// `EVENTCHKINF_9C` (`save.h`: 0x9C): the scarecrow's song played back to Bonooru.
pub const EVENTCHKINF_9C: u16 = 0x9C;
/// `OCARINA_MODE_05` (`ocarina.h`): another song than the one checked (5 and up).
const OCARINA_MODE_05: u16 = 0x05;

/// `sCylinderInit`: AC from Player (and hookable), OC; only OC is set.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCFFFFF, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCFFFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A911F0`: the dance's animation set.
    Func80a911f0,
    /// `func_80A91284`: his text chosen.
    Func80a91284,
    /// `func_80A91348`: standing, offering his talk; the ocarina out starts the recording (a
    /// child) or the check (an adult).
    Func80a91348,
    /// `func_80A915B8`: 0x40A4; its event starts the recording.
    Func80a915b8,
    /// `func_80A91620`: the child's recording.
    Func80a91620,
    /// `func_80A91760`: 0x40A0; its event plays the song back.
    Func80a91760,
    /// `func_80A917FC`: playing the song back.
    Func80a917fc,
    /// `func_80A9187C`: 0x40A8; its event starts the check.
    Func80a9187c,
    /// `func_80A918E4`: the adult's check.
    Func80a918e4,
    /// `func_80A91A90`: the closing text, Player held; Link let go.
    Func80a91a90,
}

pub struct EnKakasi3 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Arc<Skeleton>,
    anim: Anim,
    pub action: Action,
    /// `unk_194`: Link an adult.
    pub unk_194: bool,
    /// `unk_195`: the adult played the song back to him this visit.
    pub unk_195: bool,
    /// `dialogState`: the text state his talk waits for.
    pub dialog_state: u8,
    /// `unk_198`: the update's count.
    pub unk_198: i16,
    /// `unk_1A8`: a child with the song set (his talk plays it back); never cleared.
    pub unk_1a8: i16,
    pub dance: Dance,
    pub collider: ColliderCylinder,
    /// `subCamId`.
    pub sub_cam_id: i16,
}

impl EnKakasi3 {
    /// `EnKakasi3_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some((skeleton, anim)) = load(play, "En_Kakasi3") else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        log::debug!("Bonooru");
        let mut this = EnKakasi3 {
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone())),
            skeleton,
            anim,
            action: Action::Func80a911f0,
            unk_194: false,
            unk_195: false,
            dialog_state: 0,
            unk_198: 0,
            unk_1a8: 0,
            dance: Dance::default(),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            sub_cam_id: 0,
            actor,
        };
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.flags |= ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER;
        this.dance.rot = this.actor.world_rot;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        Box::new(this)
    }

    /// `func_80A911F0`.
    fn func_80a911f0(&mut self, _play: &mut PlayState) {
        change_anim(&mut self.skel, &self.anim, 1.0, ANIMMODE_LOOP);
        self.action = Action::Func80a91284;
    }

    /// `func_80A91284`: his text by Link's age and the song.
    fn func_80a91284(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.actor.text_id = 0x40A1;
        self.dialog_state = TEXT_STATE_DONE;
        self.dance.unk_19a = 0;
        if !play.save.adult {
            self.unk_194 = false;
            if play.save.scarecrow_spawn_song_set != 0 {
                self.actor.text_id = 0x40A0;
                self.dialog_state = TEXT_STATE_EVENT;
                self.unk_1a8 = 1;
            }
        } else {
            self.unk_194 = true;
            if play.save.scarecrow_spawn_song_set != 0 {
                self.actor.text_id = if self.unk_195 { 0x40A2 } else { 0x40A3 };
            }
        }
        self.action = Action::Func80a91348;
    }

    /// The ocarina out before him: the cutscene 2260, the box paused, `text_id`, then `next`.
    fn ask_for_song(&mut self, play: &mut PlayState, text_id: u16, next: Action) {
        let me = cam_actor(play, &self.actor);
        self.sub_cam_id = play.onepoint_cutscene_init(2260, -99, me, CAM_ID_MAIN);
        play.msg_ctx.msg_mode = MSGMODE_PAUSED;
        self.dialog_state = TEXT_STATE_EVENT;
        self.dance.unk_1b8 = 0.0;
        play.start_textbox(text_id, None);
        set_player_state2(play, STATE2_23);
        self.action = next;
    }

    /// `func_80A91348`: standing; talked to, his text (a child's with the song set plays it
    /// back). Link within 120 in front of him (0x4300): a child's ocarina out asks for a song
    /// (0x40A4); an adult's, the song set and not yet heard, asks for it (0x40A8); within 80 the
    /// ocarina turns to him.
    fn func_80a91348(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        self.sub_cam_id = CAM_ID_NONE;
        if process_talk_request(&mut self.actor) {
            self.action = if !self.unk_194 && self.unk_1a8 != 0 { Action::Func80a91760 } else { Action::Func80a91284 };
            return;
        }
        let angle_towards_link = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if !(self.actor.xz_dist_to_player > 120.0) && (angle_towards_link as i32).abs() < 0x4300 {
            if !self.unk_194 {
                if player_state2(play) & STATE2_24 != 0 {
                    self.ask_for_song(play, 0x40A4, Action::Func80a915b8);
                    return;
                }
                if self.actor.xz_dist_to_player < 80.0 {
                    set_player_state2(play, STATE2_23);
                }
            } else if play.save.scarecrow_spawn_song_set != 0 && !self.unk_195 {
                if player_state2(play) & STATE2_24 != 0 {
                    self.ask_for_song(play, 0x40A8, Action::Func80a9187c);
                    return;
                }
                if self.actor.xz_dist_to_player < 80.0 {
                    set_player_state2(play, STATE2_23);
                }
            }
            offer_talk(play, &self.actor, 100.0);
        }
    }

    /// `func_80A915B8`: at 0x40A4's event and A, the recording.
    fn func_80a915b8(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            message_close_textbox(play);
            play.message_start_ocarina(OCARINA_ACTION_SCARECROW_SPAWN_RECORDING);
            self.action = Action::Func80a91620;
        }
    }

    /// `OCARINA_MODE_04`, or 5 to 10 (`< OCARINA_MODE_0B`): the song refused or wrong.
    fn refused(mode: u16) -> bool {
        mode == OCARINA_MODE_04 || (OCARINA_MODE_05..OCARINA_MODE_0B).contains(&mode)
    }

    /// `func_80A91620`: the recording. Refused, the cutscene ended (its camera told it's done if
    /// it's still there), back to standing; taken (`OCARINA_MODE_03`), 0x40A5 with Player held;
    /// while Link plays, the dance to his notes.
    fn func_80a91620(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if Self::refused(mode) && play.msg_ctx.msg_mode == MSGMODE_NONE {
            play.onepoint_end_cutscene(self.sub_cam_id);
            if camera_is_null(play, self.sub_cam_id) {
                self.sub_cam_id = CAM_ID_NONE;
            }
            if self.sub_cam_id != CAM_ID_NONE {
                camera_set_finished(play, self.sub_cam_id);
            }
            self.action = Action::Func80a911f0;
            return;
        }
        if mode == OCARINA_MODE_03 && play.msg_ctx.msg_mode == MSGMODE_NONE {
            self.dialog_state = TEXT_STATE_EVENT;
            play.start_textbox(0x40A5, None);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::Func80a91a90;
            return;
        }
        if mode == OCARINA_MODE_01 {
            self.dance.dance(&mut self.actor, &mut self.skel, play, false);
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_80A91760`: at 0x40A0's state and A, the box paused, the playback and the cutscene
    /// 2280.
    fn func_80a91760(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        if self.dialog_state == play.message_state() && play.message_should_advance() {
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            play.message_start_ocarina(OCARINA_ACTION_SCARECROW_SPAWN_PLAYBACK);
            self.action = Action::Func80a917fc;
            let me = cam_actor(play, &self.actor);
            self.sub_cam_id = play.onepoint_cutscene_init(2280, -99, me, CAM_ID_MAIN);
        }
    }

    /// `func_80A917FC`: dancing by Rand until the playback's over (`OCARINA_MODE_0F`): the mode
    /// to 4, the box closed, the cutscene ended.
    fn func_80a917fc(&mut self, play: &mut PlayState) {
        if play.msg_ctx.ocarina_mode != OCARINA_MODE_0F {
            self.dance.dance(&mut self.actor, &mut self.skel, play, true);
        } else {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            message_close_textbox(play);
            play.onepoint_end_cutscene(self.sub_cam_id);
            self.action = Action::Func80a911f0;
        }
    }

    /// `func_80A9187C`: at 0x40A8's event and A, the check.
    fn func_80a9187c(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            message_close_textbox(play);
            play.message_start_ocarina(OCARINA_ACTION_CHECK_SCARECROW_SPAWN);
            self.action = Action::Func80a918e4;
        }
    }

    /// `func_80A918E4`: the adult's check. Wrong (4, or 5 to 10, the box closed): 0x40A6, the
    /// cutscene ended; right (`OCARINA_MODE_03`): the mode to 4, heard (`unk_195`), 0x40A7; both
    /// with Player held. While Link plays, the dance to his notes.
    fn func_80a918e4(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if Self::refused(mode) && play.msg_ctx.msg_mode == MSGMODE_NONE {
            play.start_textbox(0x40A6, None);
            self.dialog_state = TEXT_STATE_EVENT;
            play.onepoint_end_cutscene(self.sub_cam_id);
            self.sub_cam_id = CAM_ID_NONE;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::Func80a91a90;
            return;
        }
        if mode == OCARINA_MODE_03 && play.msg_ctx.msg_mode == MSGMODE_NONE {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            log::debug!("En_Kakasi3: with this, other guys are OK! That's it!");
            self.unk_195 = true;
            play.start_textbox(0x40A7, None);
            self.dialog_state = TEXT_STATE_EVENT;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::Func80a91a90;
            return;
        }
        if mode == OCARINA_MODE_01 {
            self.dance.dance(&mut self.actor, &mut self.skel, play, false);
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_80A91A90`: Player held; at the text's state and A: heard, `EVENTCHKINF_9C`; the
    /// camera told it's done (if it's there), the box closed, the song's mode to 4, Link let go.
    fn func_80a91a90(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
        if self.dialog_state == play.message_state() && play.message_should_advance() {
            if self.unk_195 && !play.save.get_event_chk_inf(EVENTCHKINF_9C) {
                play.save.set_event_chk_inf(EVENTCHKINF_9C);
            }
            if camera_is_null(play, self.sub_cam_id) {
                self.sub_cam_id = CAM_ID_NONE;
            }
            if self.sub_cam_id != CAM_ID_NONE {
                camera_set_finished(play, self.sub_cam_id);
            }
            message_close_textbox(play);
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.action = Action::Func80a911f0;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80a911f0 => self.func_80a911f0(play),
            Action::Func80a91284 => self.func_80a91284(play),
            Action::Func80a91348 => self.func_80a91348(play),
            Action::Func80a915b8 => self.func_80a915b8(play),
            Action::Func80a91620 => self.func_80a91620(play),
            Action::Func80a91760 => self.func_80a91760(play),
            Action::Func80a917fc => self.func_80a917fc(play),
            Action::Func80a9187c => self.func_80a9187c(play),
            Action::Func80a918e4 => self.func_80a918e4(play),
            Action::Func80a91a90 => self.func_80a91a90(play),
        }
    }
}

impl ActorImpl for EnKakasi3 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKakasi3_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_198 = self.unk_198.wrapping_add(1);
        self.actor.world_rot = self.actor.shape_rot;
        self.dance.tick_timers();
        self.actor.set_focus(60.0);
        self.run_action(play);
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    /// `EnKakasi3_Destroy`: the cylinder. `//! @bug (game)`: `SkelAnime_Free` isn't called.
    fn destroy(&mut self, _play: &mut PlayState) {}
    fn render_state(&self) -> RenderState {
        skel_render_state(&self.actor, &self.skel)
    }
    /// `EnKakasi3_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        draw_skel(&self.skeleton, rs, out);
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
