//! `En_Kakasi` (`ovl_En_Kakasi/z_en_kakasi.c`): the scarecrow by Lake Hylia's lab who learns the
//! long song (Pierre, the wandering scarecrow, as 0x4076 has him; Lake Hylia's room 0 places one at (12, -1253,
//! 2664), params 0xFFFF). `object_ka`'s `object_ka_Skel_0065B0` (flex, 27 limbs).
//!
//! - **Standing** (`func_80A8F660`, `func_80A8F75C`): `object_ka_Anim_000214` held at its first
//!   frame (play speed 0, the frame eased to 0), turning back to where it was placed. Its text:
//!   0x4076, or with the long song set (`scarecrowLongSongSet`) 0x407A for a child and 0x4079 for
//!   an adult (those two `TEXT_STATE_EVENT`: talked to, he plays the song back). Talks within 100
//!   (`Actor_OfferTalk`), Link within 120 and in front of it (0x4300).
//! - **Recording** (a child's only): Link's ocarina out (`PLAYER_STATE2_24`) in front of it, the
//!   one-point cutscene 2260 and `Message_StartOcarina(OCARINA_ACTION_SCARECROW_LONG_RECORDING)`
//!   (`func_80A8F8D0`); it **dances** to each note (`func_80A8F320`: A spins it round, C-Down and
//!   C-Up set its animation's speed 1 and 2, C-Right and C-Left rock it on z and x, 0x1388 each
//!   way; eight notes in and it hops, 3 up). The recording over (`OCARINA_MODE_04`, the box
//!   closed): no note played and it's back to standing; else 0x4077 (`func_80A8F9C8`).
//! - **Playing back** (`func_80A8F9C8`): Player held (`PLAYER_CSACTION_8`); at the text's event,
//!   the one-point cutscene 2270 and `OCARINA_ACTION_SCARECROW_LONG_PLAYBACK`; it dances by Rand
//!   until `OCARINA_MODE_0F` (`func_80A8FAA4`), then 0x4077 (an adult) or 0x4078 (a child) with
//!   2260 again, and Link let go (`PLAYER_CSACTION_7`) as it closes (`func_80A8FBB8`).
//!
//! Its draw is the skeleton alone (`SkelAnime_DrawFlexOpa`, no limb overrides: no head or arm
//! turns), at its shape's rotation (the dance's turns). No shadow (no `ActorShape_Init`). The
//! whole overlay is ported; the scarecrow's song itself (recorded, played, `scarecrowLongSongSet`
//! set) is the message box's and the ocarina's.
//!
//! Params: `params` unused (placed 0xFFFF).
//! Flags: reads `scarecrowLongSongSet` (its text; set by the message box's recording).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_zero_f, smooth_step_to_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, BGCHECKFLAG_GROUND, Rot, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::ocarina::{OCARINA_BTN_A, OCARINA_BTN_C_DOWN, OCARINA_BTN_C_LEFT, OCARINA_BTN_C_RIGHT, OCARINA_BTN_C_UP};
use oot_game::camera::{CAM_ID_MAIN, CAM_ID_NONE, CamActor};
use oot_game::collision_check::*;
use oot_game::message::{MSGMODE_NONE, MSGMODE_PAUSED, OCARINA_ACTION_SCARECROW_LONG_PLAYBACK, OCARINA_ACTION_SCARECROW_LONG_RECORDING, OCARINA_MODE_01, OCARINA_MODE_04, OCARINA_MODE_0F, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{offer_talk, process_talk_request};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::player::{STATE2_23, STATE2_24};

/// `ACTOR_EN_KAKASI` (`actor_table.h`: 0x017B).
pub const ACTOR_EN_KAKASI: i16 = 0x017B;
/// `OBJECT_KA` (`object_table.h`: 0x0154).
pub const OBJECT: &str = "object_ka";
pub const OBJECT_KA: i16 = 0x0154;
/// `object_ka_Skel_0065B0` (`object_ka.xml`): the scarecrow (flex).
pub const SKEL: &str = "object_ka_Skel_0065B0";
/// `object_ka_Anim_000214`: its dance (the only animation the scarecrows play).
pub const ANIM: &str = "object_ka_Anim_000214";

/// `En_Kakasi_Profile`: `ACTORCAT_NPC`, attention-enabled, friendly, updating during the ocarina.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_KAKASI,
    name: "En_Kakasi",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: placed 0xFFFF; nothing reads it.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed 0xFFFF; read by nothing");

/// En_Kakasi's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[FlagUse::new(Flag::Other("scarecrowLongSongSet")).reads().note("its text; set by the message box once the long song's recorded")],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x4076).cond("no long song set").note("my name is Pierre, the wandering scarecrow"),
        TextLine::new(0x4077).cond("a child's song recorded; an adult's played back").note("fantastic tune, it touches my heart"),
        TextLine::new(0x4078).cond("a child's song played back").note("come up with a nice song, come back"),
        TextLine::new(0x4079).cond("an adult, the long song set").note("that song still stays in my heart: let's hear it (its event plays it back)"),
        TextLine::new(0x407A).cond("a child, the long song set").note("that song stays in my heart (its event plays it back)"),
    ],
};

// shared: `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER`, missing from `oot_game::actor`.
/// `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER` (`actor.h`: 1 << 10; the hookshot isn't ported).
pub const ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER: u32 = 1 << 10;
/// `ATTENTION_RANGE_6` (`actor.h`).
pub const ATTENTION_RANGE_6: u8 = 6;
/// `PLAYER_CSACTION_7`, `PLAYER_CSACTION_8` (`player.h`): let go, held.
pub const PLAYER_CSACTION_7: u8 = 7;
pub const PLAYER_CSACTION_8: u8 = 8;

// The sounds (`include/tables/sfx/environmentbank_table.h`, `itembank_table.h`).
/// `NA_SE_EV_KAKASHI_SWING` (0x286A).
pub const NA_SE_EV_KAKASHI_SWING: u16 = 0x286A;
/// `NA_SE_EV_KAKASHI_ROLL` (0x286B).
pub const NA_SE_EV_KAKASHI_ROLL: u16 = 0x286B;
/// `NA_SE_IT_KAKASHI_JUMP` (0x1847).
pub const NA_SE_IT_KAKASHI_JUMP: u16 = 0x1847;

/// `sCylinderInit`: OC only (and hookable).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `player->stateFlags2`.
pub(crate) fn player_state2(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.state_flags2())
}

/// `player->stateFlags2 |= flag`.
pub(crate) fn set_player_state2(play: &mut PlayState, flag: u32) {
    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        p.change_state_flags2(flag, 0);
    }
}

/// `Message_CloseTextbox`.
pub(crate) fn message_close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

// shared: `play->cameraPtrs[id] == NULL` and `Camera_SetFinishedFlag(play->cameraPtrs[id])` by
// id (`PlayState::camera` takes `CAM_ID_NONE` as the active camera).
/// `play->cameraPtrs[id] == NULL` (`CAM_ID_NONE` reads before the array: taken as none).
pub(crate) fn camera_is_null(play: &PlayState, id: i16) -> bool {
    id < CAM_ID_MAIN || play.camera(id).is_none()
}

/// `Camera_SetFinishedFlag(play->cameraPtrs[id])` (nothing for a camera that isn't there).
pub(crate) fn camera_set_finished(play: &mut PlayState, id: i16) {
    if !camera_is_null(play, id) {
        play.camera_set_finished_flag(id);
    }
}

/// The updating actor as the one-point cutscenes read it.
pub(crate) fn cam_actor(play: &PlayState, actor: &Actor) -> Option<CamActor> {
    play.cur_actor.map(|h| play.cam_actor_of(h, actor))
}

/// The skeleton and its animation from `object_ka`.
pub(crate) fn load(play: &PlayState, who: &str) -> Option<(Arc<Skeleton>, Anim)> {
    let a = play.assets.clone()?;
    let r = (|| -> anyhow::Result<_> { Ok((a.skeleton(OBJECT, SKEL)?, a.animation(OBJECT, ANIM)?)) })();
    r.map_err(|e| log::error!("{who}: {e:#}")).ok()
}

/// `Animation_Change(.., &object_ka_Anim_000214, speed, 0, (s16)Animation_GetLastFrame(..), mode, -10)`.
pub(crate) fn change_anim(skel: &mut SkelAnimeStd, anim: &Anim, speed: f32, mode: u8) {
    let frame_count = anim.last_frame();
    skel.change(anim.clone(), speed, 0.0, frame_count as i16 as f32, mode, -10.0);
}

/// The scarecrows' dance (En_Kakasi's and En_Kakasi3's: the same code in both overlays) and the
/// fields it keeps.
#[derive(Debug, Clone, Copy, Default)]
pub struct Dance {
    /// `unk_19A`: the notes' count (at 8 it hops).
    pub unk_19a: i16,
    /// `unk_19C`: four timers, down one each update; `[3]` the playback's next made-up note.
    pub unk_19c: [i16; 4],
    /// `unk_1A4`: spinning round (A).
    pub unk_1a4: i16,
    /// `unk_1A6`: the playback's made-up note.
    pub unk_1a6: i16,
    /// `unk_1A8` (En_Kakasi3's `unk_1AA`): the rock on x (C-Left), 0x1388 each way.
    pub rock_x: i16,
    /// `unk_1AC` (En_Kakasi3's `unk_1AE`): the rock on z (C-Right).
    pub rock_z: i16,
    /// `unk_1B8`: the animation's speed (C-Down 1, C-Up 2).
    pub unk_1b8: f32,
    /// `rot`: the rotation it was placed with.
    pub rot: Rot,
}

impl Dance {
    /// `func_80A8F28C` (En_Kakasi3's `func_80A90E28`): still, the animation easing to its first
    /// frame, turning back to where it was placed.
    pub fn stand(&mut self, actor: &mut Actor, skel: &mut SkelAnimeStd) {
        self.unk_1a4 = 0;
        skel.play_speed = 0.0;
        self.rock_x = 0;
        self.rock_z = 0;
        approach_zero_f(&mut skel.cur_frame, 0.5, 1.0);
        smooth_step_to_s(&mut actor.shape_rot.x, self.rot.x, 5, 0x2710, 0);
        smooth_step_to_s(&mut actor.shape_rot.y, self.rot.y, 5, 0x2710, 0);
        smooth_step_to_s(&mut actor.shape_rot.z, self.rot.z, 5, 0x2710, 0);
    }

    /// `func_80A8F320` (En_Kakasi3's `func_80A90EBC`): a step of the dance, to the note Link
    /// played last (`lastOcarinaButtonIndex`), or with `playback` to one made up (`Rand`: a new
    /// one every 30 to 40 frames, the count 5 to 7 again each frame).
    pub fn dance(&mut self, actor: &mut Actor, skel: &mut SkelAnimeStd, play: &mut PlayState, playback: bool) {
        let mut ocarina_note = play.msg_ctx.last_ocarina_button_index as i16;
        if playback {
            if self.unk_19c[3] == 0 {
                self.unk_19c[3] = play.rand.zero_float(10.99) as i16 + 30;
                self.unk_1a6 = play.rand.zero_float(4.99) as i16;
            }
            self.unk_19a = play.rand.zero_float(2.99) as i16 + 5;
            ocarina_note = self.unk_1a6;
        }
        match ocarina_note {
            n if n == OCARINA_BTN_A as i16 => {
                self.unk_19a += 1;
                if self.unk_1a4 == 0 {
                    self.unk_1a4 = 1;
                    audio_play_actor_sfx2(play, NA_SE_EV_KAKASHI_ROLL);
                }
            }
            n if n == OCARINA_BTN_C_DOWN as i16 => {
                self.unk_19a += 1;
                self.unk_1b8 = 1.0;
            }
            n if n == OCARINA_BTN_C_RIGHT as i16 => {
                self.unk_19a += 1;
                if self.rock_z == 0 {
                    self.rock_z = 0x1388;
                }
            }
            n if n == OCARINA_BTN_C_LEFT as i16 => {
                self.unk_19a += 1;
                if self.rock_x == 0 {
                    self.rock_x = 0x1388;
                }
            }
            n if n == OCARINA_BTN_C_UP as i16 => {
                self.unk_19a += 1;
                self.unk_1b8 = 2.0;
            }
            _ => {}
        }
        if self.unk_19a > 8 {
            self.unk_19a = 8;
        }
        if self.unk_19a != 0 {
            actor.gravity = -1.0;
            if self.unk_19a == 8 && actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                actor.velocity.y = 3.0;
                audio_play_actor_sfx2(play, NA_SE_IT_KAKASHI_JUMP);
            }
            approach_f(&mut skel.play_speed, self.unk_1b8, 0.1, 0.2);
            smooth_step_to_s(&mut actor.shape_rot.x, self.rock_x, 5, 0x3E8, 0);
            smooth_step_to_s(&mut actor.shape_rot.z, self.rock_z, 5, 0x3E8, 0);
            // `unk_1A8 *= -1.0f`: back the other way once within 50.
            if self.rock_x != 0 && ((actor.shape_rot.x as i32 - self.rock_x as i32) as f32).abs() < 50.0 {
                self.rock_x = (self.rock_x as f32 * -1.0) as i16;
            }
            if self.rock_z != 0 && ((actor.shape_rot.z as i32 - self.rock_z as i32) as f32).abs() < 50.0 {
                self.rock_z = (self.rock_z as f32 * -1.0) as i16;
            }
            if self.unk_1a4 != 0 {
                actor.shape_rot.y = actor.shape_rot.y.wrapping_add(0x1000);
                if actor.shape_rot.y == 0 {
                    self.unk_1a4 = 0;
                }
            }
            let current_frame = skel.cur_frame as i16;
            if current_frame == 11 || current_frame == 17 {
                audio_play_actor_sfx2(play, NA_SE_EV_KAKASHI_SWING);
            }
            skel.update();
        }
    }

    /// The update's countdown of `unk_19C`.
    pub fn tick_timers(&mut self) {
        for t in &mut self.unk_19c {
            if *t != 0 {
                *t -= 1;
            }
        }
    }
}

/// The scarecrows' `RenderState`: the joints.
pub(crate) fn skel_render_state(actor: &Actor, skel: &SkelAnimeStd) -> RenderState {
    let mut rs = RenderState::of(actor);
    rs.joints = Some(eng_anim::anim::JointTable { rot: skel.joint_table.clone(), face: 0 });
    rs
}

/// `Gfx_SetupDL_25Opa` and `SkelAnime_DrawFlexOpa(.., NULL, NULL, ..)`: the scarecrow, no limb
/// overrides.
pub(crate) fn draw_skel(skeleton: &Skeleton, rs: &RenderState, out: &mut DrawOut) {
    let Some(joints) = &rs.joints else { return };
    let bones = draw_opa_pose(skeleton, &joints.rot, |_, _, _| LimbDraw::Default(glam::Mat4::IDENTITY), |_, _| {});
    out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80A8F660`: the animation set, its text chosen.
    Func80a8f660,
    /// `func_80A8F75C`: standing, offering its talk; the ocarina out starts the recording.
    Func80a8f75c,
    /// `func_80A8F8D0`: recording the long song, dancing to it.
    Func80a8f8d0,
    /// `func_80A8F9C8`: Player held; its text's event plays the song back.
    Func80a8f9c8,
    /// `func_80A8FAA4`: playing the song back, dancing.
    Func80a8faa4,
    /// `func_80A8FBB8`: the closing text; Link let go.
    Func80a8fbb8,
}

pub struct EnKakasi {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Arc<Skeleton>,
    anim: Anim,
    pub action: Action,
    /// `unk_194`: Link an adult.
    pub unk_194: bool,
    /// `unk_196`: the text state its talk waits for (`TEXT_STATE_DONE` or `TEXT_STATE_EVENT`).
    pub unk_196: u8,
    /// `unk_198`: the update's count.
    pub unk_198: i16,
    pub dance: Dance,
    /// `height`.
    pub height: f32,
    pub collider: ColliderCylinder,
    /// `subCamId`.
    pub sub_cam_id: i16,
}

impl EnKakasi {
    /// `EnKakasi_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some((skeleton, anim)) = load(play, "En_Kakasi") else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        log::debug!("Let's DANCE! {}", actor.world_pos.y);
        let mut this = EnKakasi {
            // SkelAnime_InitFlex(.., &object_ka_Skel_0065B0, &object_ka_Anim_000214, ..).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim.clone())),
            skeleton,
            anim,
            action: Action::Func80a8f660,
            unk_194: false,
            unk_196: 0,
            unk_198: 0,
            dance: Dance::default(),
            height: 0.0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            sub_cam_id: 0,
            actor,
        };
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.dance.rot = this.actor.world_rot;
        this.actor.flags |= ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        Box::new(this)
    }

    /// `func_80A8F660`: the dance's animation, and its text: 0x4076; with the long song set
    /// 0x407A (a child) or 0x4079 (an adult), an event.
    fn func_80a8f660(&mut self, play: &mut PlayState) {
        change_anim(&mut self.skel, &self.anim, 1.0, ANIMMODE_LOOP);
        self.actor.text_id = 0x4076;
        self.unk_196 = TEXT_STATE_DONE;
        if !play.save.adult {
            self.unk_194 = false;
            if play.save.scarecrow_long_song_set != 0 {
                self.actor.text_id = 0x407A;
                self.unk_196 = TEXT_STATE_EVENT;
            }
        } else {
            self.unk_194 = true;
            if play.save.scarecrow_long_song_set != 0 {
                self.actor.text_id = 0x4079;
                self.unk_196 = TEXT_STATE_EVENT;
            }
        }
        self.action = Action::Func80a8f75c;
    }

    /// `func_80A8F75C`: standing; talked to, its text (an event's plays the song back); Link
    /// within 120 in front of it (0x4300), a child with his ocarina out starts the recording
    /// (2260), within 80 the ocarina turns to it (`PLAYER_STATE2_23`).
    fn func_80a8f75c(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        self.sub_cam_id = CAM_ID_NONE;
        if process_talk_request(&mut self.actor) {
            self.action = if self.unk_196 == TEXT_STATE_EVENT { Action::Func80a8f9c8 } else { Action::Func80a8f660 };
            return;
        }
        let yaw_towards_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if !(self.actor.xz_dist_to_player > 120.0) && (yaw_towards_player as i32).abs() < 0x4300 {
            if !self.unk_194 {
                if player_state2(play) & STATE2_24 != 0 {
                    let me = cam_actor(play, &self.actor);
                    self.sub_cam_id = play.onepoint_cutscene_init(2260, -99, me, CAM_ID_MAIN);
                    play.message_start_ocarina(OCARINA_ACTION_SCARECROW_LONG_RECORDING);
                    self.dance.unk_19a = 0;
                    self.dance.unk_1b8 = 0.0;
                    set_player_state2(play, STATE2_23);
                    self.action = Action::Func80a8f8d0;
                    return;
                }
                if self.actor.xz_dist_to_player < 80.0 {
                    set_player_state2(play, STATE2_23);
                }
            }
            offer_talk(play, &self.actor, 100.0);
        }
    }

    /// `func_80A8F8D0`: the recording. Over (`OCARINA_MODE_04`, the box closed): a note played,
    /// 0x4077 (an event); none, the cutscene ended and back to standing. While Link plays
    /// (`OCARINA_MODE_01`), it dances to his notes.
    fn func_80a8f8d0(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode == OCARINA_MODE_04 && play.msg_ctx.msg_mode == MSGMODE_NONE {
            log::debug!("En_Kakasi: end?");
            if self.dance.unk_19a != 0 {
                message_close_textbox(play);
                self.actor.text_id = 0x4077;
                self.unk_196 = TEXT_STATE_EVENT;
                play.start_textbox(0x4077, None);
                self.action = Action::Func80a8f9c8;
            } else {
                play.onepoint_end_cutscene(self.sub_cam_id);
                self.sub_cam_id = CAM_ID_NONE;
                self.action = Action::Func80a8f660;
            }
        } else if mode == OCARINA_MODE_01 {
            self.dance.dance(&mut self.actor, &mut self.skel, play, false);
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_80A8F9C8`: Player held; at its text's state (`unk_196`) and A, the cutscene 2270
    /// and the long song's playback (the box paused).
    fn func_80a8f9c8(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
        if self.unk_196 == play.message_state() && play.message_should_advance() {
            if self.sub_cam_id != CAM_ID_NONE {
                camera_set_finished(play, self.sub_cam_id);
            }
            let me = cam_actor(play, &self.actor);
            self.sub_cam_id = play.onepoint_cutscene_init(2270, -99, me, CAM_ID_MAIN);
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            play.message_start_ocarina(OCARINA_ACTION_SCARECROW_LONG_PLAYBACK);
            self.action = Action::Func80a8faa4;
        }
    }

    /// `func_80A8FAA4`: dancing by Rand until the playback's over (`OCARINA_MODE_0F`); the box
    /// closed, 0x4077 (an adult) or 0x4078 (a child), and 2260 again, told it's done.
    fn func_80a8faa4(&mut self, play: &mut PlayState) {
        if play.msg_ctx.ocarina_mode != OCARINA_MODE_0F {
            self.dance.dance(&mut self.actor, &mut self.skel, play, true);
            return;
        }
        log::debug!("game_play->message.msg_mode={}", play.msg_ctx.msg_mode);
        if play.msg_ctx.msg_mode == MSGMODE_NONE {
            if self.unk_194 {
                self.actor.text_id = 0x4077;
                self.unk_196 = TEXT_STATE_EVENT;
                play.start_textbox(0x4077, None);
            } else {
                self.actor.text_id = 0x4078;
                self.unk_196 = TEXT_STATE_EVENT;
                play.start_textbox(0x4078, None);
            }
            self.action = Action::Func80a8fbb8;
            play.onepoint_end_cutscene(self.sub_cam_id);
            let me = cam_actor(play, &self.actor);
            self.sub_cam_id = play.onepoint_cutscene_init(2260, -99, me, CAM_ID_MAIN);
            camera_set_finished(play, self.sub_cam_id);
        }
    }

    /// `func_80A8FBB8`: at its text's state and A, the camera told it's done, the box closed,
    /// Link let go (`PLAYER_CSACTION_7`).
    fn func_80a8fbb8(&mut self, play: &mut PlayState) {
        self.dance.stand(&mut self.actor, &mut self.skel);
        self.skel.update();
        if self.unk_196 == play.message_state() && play.message_should_advance() {
            camera_set_finished(play, self.sub_cam_id);
            message_close_textbox(play);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.action = Action::Func80a8f660;
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80a8f660 => self.func_80a8f660(play),
            Action::Func80a8f75c => self.func_80a8f75c(play),
            Action::Func80a8f8d0 => self.func_80a8f8d0(play),
            Action::Func80a8f9c8 => self.func_80a8f9c8(play),
            Action::Func80a8faa4 => self.func_80a8faa4(play),
            Action::Func80a8fbb8 => self.func_80a8fbb8(play),
        }
    }
}

impl ActorImpl for EnKakasi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnKakasi_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.unk_198 = self.unk_198.wrapping_add(1);
        self.actor.world_rot = self.actor.shape_rot;
        self.dance.tick_timers();
        self.height = 60.0;
        self.actor.set_focus(self.height);
        self.run_action(play);
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 50.0, 50.0, 100.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    /// `EnKakasi_Destroy`: the cylinder. `//! @bug (game)`: `SkelAnime_Free` isn't called (nothing
    /// to free here).
    fn destroy(&mut self, _play: &mut PlayState) {}
    fn render_state(&self) -> RenderState {
        skel_render_state(&self.actor, &self.skel)
    }
    /// `EnKakasi_Draw` (its `BREG(3)` print aside).
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
