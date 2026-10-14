//! `En_Fu` (`ovl_En_Fu/z_en_fu.c`): the windmill man, turning his music box in the windmill
//! (`hakasitarelay`, room 6).
//!
//! For a child he is calm (`FU_FACE_CALM`) and talks within 100 in front of him (0x2301 of his
//! facing): 0x5032 before the well is drained, 0x5033 after (`EVENTCHKINF_DRAINED_WELL`), moving
//! his head with that one (`gWindmillManPlayAndMoveHeadAnim`). The Song of Storms the child plays
//! there is `En_Okarina_Tag`'s (type 2: `gWindmillSpinningFasterCs`, the windmill faster, then the
//! well's draining in Kakariko, which sets `EVENTCHKINF_DRAINED_WELL`).
//!
//! For an adult he is mad (`FU_FACE_MAD`, his animation twice as fast) and teaches the Song of
//! Storms: in front of him within 100 he offers 0x5034 and tells Link's ocarina it listens
//! (`PLAYER_STATE2_23`); the ocarina out (`PLAYER_STATE2_24`) opens 0x5035 (`EnFu_WaitAdult`),
//! then the song's demonstration (`OCARINA_ACTION_TEACH_STORMS`, `EnFu_TeachSong`, his organ: the
//! message box's `OCARINA_INSTRUMENT_GRIND_ORGAN`) and its playback (`OCARINA_ACTION_PLAYBACK_STORMS`,
//! `EnFu_WaitForPlayback`). Played back right (`OCARINA_MODE_03`, `func_80A1DBD4`): the chime,
//! `gSongOfStormsCs`, `Item_Give(ITEM_SONG_STORMS)`, `EVENTCHKINF_65` (the windmill faster); once
//! the cutscene is over (`func_80A1DB60`) `EVENTCHKINF_5B` and `OCARINA_MODE_04`. After it he says
//! 0x508E.
//!
//! His head turns to Link (`Actor_TrackPlayer`) unless Link is out of his talk's reach
//! (`FU_RESET_LOOK_ANGLE`: turned back to straight). While he waits on the lesson (`FU_WAIT`) his
//! animation stops and his chest and music box fidget.
//!
//! Drawn as `EnFu_Draw` does: the eyes on segment 8 and the mouth on segment 9 by his face
//! (`sEyesSegments`, `sMouthSegments`), his head turned by `lookAngleOffset`, the focus 700 along
//! and 700 up his head (`EnFu_PostLimbDraw`). From meshes baked per face
//! (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId(play,
//! MASK_REACTION_SET_WINDMILL_MAN)`: no masks yet, so none).
//!
//! Params: none.
//! Flags: reads EVENTCHKINF_DRAINED_WELL (the child's text); reads and sets EVENTCHKINF_5B (the
//! lesson done), sets EVENTCHKINF_65 (the windmill faster, with the lesson's song).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, UPDBGCHECKINFO_FLAG_2,
};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, FlagUse};
use oot_game::event_chk_inf;
use oot_game::audio::ocarina::OCARINA_INSTRUMENT_HARP;
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::item::ITEM_SONG_STORMS;
use oot_game::message::{OCARINA_ACTION_TEACH_STORMS, OCARINA_MODE_00, OCARINA_MODE_01, OCARINA_MODE_02, OCARINA_MODE_03, OCARINA_MODE_04, TEXT_STATE_CLOSING, TEXT_STATE_SONG_DEMO_DONE};
use oot_game::npc::{actor_track_player, offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

use crate::player::{STATE2_23, STATE2_24};

/// `ACTOR_EN_FU` (`actor_table.h`: 0x0153).
pub const ACTOR_EN_FU: i16 = 0x0153;
/// `OBJECT_FU` (`object_table.h`: 0x0133).
pub const OBJECT: &str = "object_fu";

/// `En_Fu_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_FU,
    name: "En_Fu",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `FU_RESET_LOOK_ANGLE`, `FU_WAIT` (`z_en_fu.c`).
pub const FU_RESET_LOOK_ANGLE: u16 = 1 << 0;
pub const FU_WAIT: u16 = 1 << 1;
/// `EnFuFace`.
pub const FU_FACE_CALM: u16 = 0;
pub const FU_FACE_MAD: u16 = 1;

/// `EVENTCHKINF_5B`, `EVENTCHKINF_65`, `EVENTCHKINF_DRAINED_WELL` (`save.h`: 0x5B, 0x65, 0x67).
pub const EVENTCHKINF_5B: u16 = 0x5B;
pub const EVENTCHKINF_65: u16 = 0x65;
pub const EVENTCHKINF_DRAINED_WELL: u16 = 0x67;

/// En_Fu's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_DRAINED_WELL)).reads().note("the child's text"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_5B)).reads().sets().note("the Song of Storms' lesson done"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_65)).sets().note("the windmill faster, with the lesson's song"),
    ],
    todo: false,
};
/// `OCARINA_ACTION_PLAYBACK_STORMS` (`ocarina.h`: 0x1A).
pub const OCARINA_ACTION_PLAYBACK_STORMS: u16 = 0x1A;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;

/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_FREQ_Y: i32 = 0x814;
const FIDGET_FREQ_Z: i32 = 0x940;
const FIDGET_FREQ_LIMB: i32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

// `WindmillManLimb` (`object_fu.h`), 1-based as `EnFu_OverrideLimbDraw` sees them.
const WINDMILL_MAN_LIMB_CHEST_MUSIC_BOX: usize = 8;
const WINDMILL_MAN_LIMB_UNK: usize = 10;
const WINDMILL_MAN_LIMB_HEAD: usize = 14;

/// `sCylinderInit`.
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

/// `sMtxSrc`: the focus on the head.
const MTX_SRC: Vec3 = Vec3::new(700.0, 700.0, 0.0);

const SKEL: &str = "gWindmillManSkel";
const PLAY_STILL_ANIM: &str = "gWindmillManPlayStillAnim";
const PLAY_AND_MOVE_HEAD_ANIM: &str = "gWindmillManPlayAndMoveHeadAnim";
/// `sEyesSegments`, `sMouthSegments`, by `EnFuFace`.
const EYE_TEXTURES: [&str; 2] = ["gWindmillManEyeClosedTex", "gWindmillManEyeAngryTex"];
const MOUTH_TEXTURES: [&str; 2] = ["gWindmillManMouthOpenTex", "gWindmillManMouthAngryTex"];

/// The bake drawn with face `face`.
pub fn bake_name(face: usize) -> String {
    format!("En_Fu/face{face}")
}

/// The meshes `EnFu_Draw` draws: the skeleton with each face's eyes and mouth.
pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|face| MeshBake {
            name: bake_name(face),
            object: OBJECT.into(),
            segments: vec![(0x08, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[face].into() }), (0x09, BakeSegment::Texture { file: OBJECT.into(), symbol: MOUTH_TEXTURES[face].into() })],
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitChild,
    /// `func_80A1DA04`: the child's talk, until its box closes.
    Func80a1da04,
    WaitAdult,
    TeachSong,
    WaitForPlayback,
    /// `func_80A1DBA0`: the adult's talk, until its box closes.
    Func80a1dba0,
    /// `func_80A1DBD4`: the playback's outcome.
    Func80a1dbd4,
    /// `func_80A1DB60`: the lesson's cutscene, until it's over.
    Func80a1db60,
}

pub struct EnFu {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    /// `lookAngleOffset`, `unk_2A2`: `Actor_TrackPlayer`'s head and torso turns.
    pub look_angle_offset: [i16; 3],
    pub unk_2a2: [i16; 3],
    pub behavior_flags: u16,
    pub facial_expression: u16,
    pub action: Action,
    play_still: Anim,
    play_and_move_head: Anim,
}

/// `player->stateFlags2`.
fn player_state2(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2())
}

/// `player->stateFlags2`: `set` set, `clear` cleared.
fn change_player_state2(play: &mut PlayState, set: u32, clear: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(set, clear);
    }
}

impl EnFu {
    /// `EnFu_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, SKEL)?, assets.animation(OBJECT, PLAY_STILL_ANIM)?, assets.animation(OBJECT, PLAY_AND_MOVE_HEAD_ANIM)?)) })();
        let (skeleton, play_still, play_and_move_head) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Fu: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnFu {
            // SkelAnime_InitFlex(.., gWindmillManPlayStillAnim, ..), Animation_PlayLoop(the same).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(play_still.clone())),
            skeleton,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            look_angle_offset: [0; 3],
            unk_2a2: [0; 3],
            behavior_flags: 0,
            facial_expression: FU_FACE_CALM,
            action: Action::WaitChild,
            play_still,
            play_and_move_head,
            actor,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 36).
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        if !play.save.adult {
            this.action = Action::WaitChild;
            this.facial_expression = FU_FACE_CALM;
        } else {
            this.action = Action::WaitAdult;
            this.facial_expression = FU_FACE_MAD;
            this.skel.play_speed = 2.0;
        }
        this.behavior_flags = 0;
        this.actor.target_mode = ATTENTION_RANGE_6;
        Box::new(this)
    }

    /// `func_80A1D94C`: the talk accepted (to `action`), or `text_id` offered within 100 in front
    /// (0x2301 of his facing); out of that, his head's turned back.
    fn func_80a1d94c(&mut self, play: &mut PlayState, text_id: u16, action: Action) -> bool {
        if process_talk_request(&mut self.actor) {
            self.action = action;
            return true;
        }
        self.actor.text_id = text_id;
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if (yaw_diff as i32).abs() < 0x2301 && self.actor.xz_dist_to_player < 100.0 {
            offer_talk(play, &self.actor, 100.0);
        } else {
            self.behavior_flags |= FU_RESET_LOOK_ANGLE;
        }
        false
    }

    /// `func_80A1DA04`: back to waiting as the box closes, the head's move ended.
    fn func_80a1da04(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.behavior_flags &= !FU_WAIT;
            self.action = Action::WaitChild;
            if self.skel.is(PLAY_AND_MOVE_HEAD_ANIM) {
                let last = self.play_still.last_frame();
                self.skel.change(self.play_still.clone(), 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
            }
        }
    }

    /// `EnFu_WaitChild`'s text: 0x5033 once the well is drained, else 0x5032.
    pub fn child_text_id(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_WINDMILL_MAN): no masks, so 0.
        if play.save.get_event_chk_inf(EVENTCHKINF_DRAINED_WELL) { 0x5033 } else { 0x5032 }
    }

    /// `EnFu_WaitChild`.
    fn wait_child(&mut self, play: &mut PlayState) {
        let text_id = Self::child_text_id(play);
        if self.func_80a1d94c(play, text_id, Action::Func80a1da04) && text_id == 0x5033 {
            let last = self.play_and_move_head.last_frame();
            self.skel.change(self.play_and_move_head.clone(), 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
        }
    }

    /// `func_80A1DB60`: the lesson's cutscene over.
    fn func_80a1db60(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            self.action = Action::WaitAdult;
            play.save.set_event_chk_inf(EVENTCHKINF_5B);
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        }
    }

    /// `func_80A1DBA0`.
    fn func_80a1dba0(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::WaitAdult;
        }
    }

    /// `func_80A1DBD4`: the playback's outcome.
    fn func_80a1dbd4(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode >= OCARINA_MODE_04 {
            self.action = Action::WaitAdult;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        } else if mode == OCARINA_MODE_03 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.action = Action::Func80a1db60;
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            play.cs_ctx.segment = play.cutscene_script("gSongOfStormsCs");
            play.save.cutscene_trigger = 1;
            oot_game::item::item_give(&mut play.save, Some(&mut play.audio), ITEM_SONG_STORMS);
            play.msg_ctx.ocarina_mode = OCARINA_MODE_00;
            play.save.set_event_chk_inf(EVENTCHKINF_65);
        } else if mode == OCARINA_MODE_02 {
            change_player_state2(play, 0, STATE2_24);
            self.action = Action::WaitAdult;
        } else if mode == OCARINA_MODE_01 {
            change_player_state2(play, STATE2_23, 0);
        }
    }

    /// `EnFu_WaitForPlayback`: the demonstration over (`TEXT_STATE_SONG_DEMO_DONE`), the playback.
    fn wait_for_playback(&mut self, play: &mut PlayState) {
        change_player_state2(play, STATE2_23, 0);
        if play.message_state() == TEXT_STATE_SONG_DEMO_DONE {
            play.message_start_ocarina(OCARINA_ACTION_PLAYBACK_STORMS);
            self.action = Action::Func80a1dbd4;
        }
    }

    /// `EnFu_TeachSong`: 0x5035 closing, the demonstration.
    fn teach_song(&mut self, play: &mut PlayState) {
        change_player_state2(play, STATE2_23, 0);
        if play.message_state() == TEXT_STATE_CLOSING {
            self.behavior_flags &= !FU_WAIT;
            // The harp, at once overwritten by the grind organ in the message system
            // (MSGMODE_SONG_DEMONSTRATION_SELECT_INSTRUMENT for OCARINA_ACTION_TEACH_STORMS).
            play.audio.audio_ocarina_set_instrument(OCARINA_INSTRUMENT_HARP);
            play.message_start_ocarina(OCARINA_ACTION_TEACH_STORMS);
            self.action = Action::WaitForPlayback;
        }
    }

    /// `EnFu_WaitAdult`.
    fn wait_adult(&mut self, play: &mut PlayState) {
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if play.save.get_event_chk_inf(EVENTCHKINF_5B) {
            self.func_80a1d94c(play, 0x508E, Action::Func80a1dba0);
        } else if player_state2(play) & STATE2_24 != 0 {
            self.actor.text_id = 0x5035;
            play.start_textbox(self.actor.text_id, None);
            self.action = Action::TeachSong;
            self.behavior_flags |= FU_WAIT;
        } else if process_talk_request(&mut self.actor) {
            self.action = Action::Func80a1dba0;
        } else if (yaw_diff as i32).abs() < 0x2301 && self.actor.xz_dist_to_player < 100.0 {
            self.actor.text_id = 0x5034;
            offer_talk(play, &self.actor, 100.0);
            change_player_state2(play, STATE2_23, 0);
        }
    }

    /// The limb matrices `EnFu_Draw` draws with (`EnFu_OverrideLimbDraw`): the head by
    /// `lookAngleOffset`; with `FU_WAIT`, the chest and music box's fidget by `frames`.
    fn pose(&self, joints: &[[i16; 3]], look: [i16; 2], wait: bool, frames: u32) -> Vec<Mat4> {
        self.skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == WINDMILL_MAN_LIMB_UNK {
                return Mat4::IDENTITY;
            }
            if limb == WINDMILL_MAN_LIMB_HEAD {
                rot[0] = rot[0].wrapping_add(look[1]);
                rot[2] = rot[2].wrapping_add(look[0]);
            }
            if wait && limb == WINDMILL_MAN_LIMB_CHEST_MUSIC_BOX {
                let l = limb as i32;
                let fy = frames.wrapping_mul((l * FIDGET_FREQ_LIMB + FIDGET_FREQ_Y) as u32) as i16;
                let fz = frames.wrapping_mul((l * FIDGET_FREQ_LIMB + FIDGET_FREQ_Z) as u32) as i16;
                rot[1] = (rot[1] as f32 + sin_s(fy) * FIDGET_AMPLITUDE) as i16;
                rot[2] = (rot[2] as f32 + cos_s(fz) * FIDGET_AMPLITUDE) as i16;
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnFu {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnFu_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if self.behavior_flags & FU_WAIT == 0 && self.skel.update() {
            // The animation it has, again from its start, once (a loop's update never ends).
            if let Some(anim) = self.skel.animation.clone() {
                let last = anim.last_frame();
                self.skel.change(anim, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
            }
        }
        match self.action {
            Action::WaitChild => self.wait_child(play),
            Action::Func80a1da04 => self.func_80a1da04(play),
            Action::WaitAdult => self.wait_adult(play),
            Action::TeachSong => self.teach_song(play),
            Action::WaitForPlayback => self.wait_for_playback(play),
            Action::Func80a1dba0 => self.func_80a1dba0(play),
            Action::Func80a1dbd4 => self.func_80a1dbd4(play),
            Action::Func80a1db60 => self.func_80a1db60(play),
        }
        if self.behavior_flags & FU_RESET_LOOK_ANGLE != 0 {
            smooth_step_to_s(&mut self.look_angle_offset[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.look_angle_offset[1], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.unk_2a2[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.unk_2a2[1], 0, 6, 6200, 100);
            self.behavior_flags &= !FU_RESET_LOOK_ANGLE;
        } else {
            let focus = self.actor.focus_pos;
            let (mut head, mut torso) = (self.look_angle_offset, self.unk_2a2);
            actor_track_player(play, &mut self.actor, &mut head, &mut torso, focus);
            self.look_angle_offset = head;
            self.unk_2a2 = torso;
        }
    }
    /// `EnFu_PostLimbDraw`, the head: the focus at `sMtxSrc` on it.
    fn draw_update(&mut self, play: &mut PlayState) {
        let wait = self.behavior_flags & FU_WAIT != 0;
        let bones = self.pose(&self.skel.joint_table, [self.look_angle_offset[0], self.look_angle_offset[1]], wait, play.state_frames);
        if let Some(head) = bones.get(WINDMILL_MAN_LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(MTX_SRC);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.look_angle_offset[0], self.look_angle_offset[1]];
        rs.switches = vec![self.facial_expression as u32, (self.behavior_flags & FU_WAIT != 0) as u32];
        rs
    }
    /// `EnFu_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 2 || rs.switches.len() < 2 {
            return;
        }
        let bones = self.pose(&joints.rot, [rs.angles[0], rs.angles[1]], rs.switches[1] != 0, play.state_frames);
        let face = (rs.switches[0] as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(face))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale 36): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(36.0, 1.0, 36.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
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
