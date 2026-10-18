//! `En_Zl4` (`ovl_En_Zl4/z_en_zl4.c`): Princess Zelda as a child, in her courtyard. She stands
//! at the window looking in; Link coming up in front of her (on her floor, within 70 and inside
//! her quarter-turn) talks, she snaps him 56 in front of her and the meeting runs as her own
//! cutscene (`EnZl4_Cutscene`), with the main camera on her fixed views (`sCamDirections`,
//! `CAM_SET_FREE0`) and moves (`sCamMove`, `CAM_SET_CS_0`):
//! - the meeting (`NA_BGM_APPEAR`, 0x702E, 0x702F, `gZeldasCourtyardMeetCs` for Navi, 0x70F9 to
//!   0x70FB);
//! - the stone (0x7030: yes 0x7032; no 0x7031 and the question again), the Triforce (0x70FC to
//!   0x70FE);
//! - the names (0x70FF, 0x2073 to 0x2075, 0x7033: yes on, no 0x7034 and again; 0x7035), and 130
//!   frames on the white fade to the cutscene map's legend (`ENTR_CUTSCENE_MAP_0`, `CS_INDEX_7`);
//! - back by `ENTR_CASTLE_COURTYARD_ZELDA_1` (`EnZl4_SetupFromLegendCs`): the legend's questions
//!   (0x7037, 0x2076, 0x2077: yes 0x7005; no 0x7038 and again), the window (0x2078: yes, or 0x700B
//!   first), `gZeldasCourtyardWindowCs` held at frame 90, `gZeldasCourtyardGanonCs` held at 209,
//!   0x7039;
//! - Ganondorf (0x2079, 0x207A, 0x207B, 0x703A: yes 0x703B; no 0x7073 and again);
//! - the plan (0x7123, 0x207C, 0x207D) and Zelda's Letter (`GI_ZELDAS_LETTER`): Link's talk
//!   started (`play->talkWithPlayer`, her text -1), the get-item offered until it's taken, then
//!   `EVENTCHKINF_OBTAINED_ZELDAS_LETTER` and `EnZl4_Idle`.
//!
//! After the letter she stands and talks (0x703C, or 0x703D with two stones). On a cutscene
//! layer (the credits) she follows the script's cues (`EnZl4_TheEnd`). The animations that move
//! her (`EnZl4_InMovingAnim`) carry her root's motion over (`AnimTaskQueue_AddActorMovement`).
//!
//! The whole overlay is ported. `MaskReaction_GetTextId` gives 0 (no masks yet) and
//! `Rumble_Request` does nothing (no rumble). Drawn from one bake of her skeleton without limbs 3
//! to 6 (`EnZl4_OverrideLimbDraw`), her eyes and mouth swapped in per draw by their textures'
//! segments (`DrawParams::texture_images`); the head and torso turns as `OverrideLimbDraw`'s
//! matrices.
//!
//! Params: none (what she does is the scene layer's, the entrance's and the letter's).
//! Flags: reads EVENTCHKINF_OBTAINED_ZELDAS_LETTER at init (then she only talks) and sets it with
//! the letter.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK, ACTOR_FLAG_TALK_WITH_C_UP, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, FlagUse};
use oot_game::event_chk_inf;
use oot_game::camera::{CAM_SET_CS_0, CAM_SET_FREE0, CAM_SET_NORMAL0};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CS_STATE_STOP, CutsceneCameraPoint};
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::message::{MSGMODE_PAUSED, MSGMODE_TEXT_CLOSING, TEXT_STATE_CHOICE, TEXT_STATE_CLOSING, TEXT_STATE_EVENT};
use oot_game::npc::{NpcTrack, offer_talk, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::sys_matrix::MtxF;
use oot_game::transition::{TRANS_TRIGGER_START, TRANS_TYPE_FADE_WHITE};

use crate::player::Player;

/// `ACTOR_EN_ZL4` (`actor_table.h`: 0x01D3).
pub const ACTOR_EN_ZL4: i16 = 0x01D3;
pub const OBJECT: &str = "object_zl4";

/// `En_Zl4_Profile`: `FLAGS` `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_CULLING_DISABLED`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ZL4, name: "En_Zl4", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `EVENTCHKINF_OBTAINED_ZELDAS_LETTER` (`save.h`: 0x40).
pub const EVENTCHKINF_OBTAINED_ZELDAS_LETTER: u16 = 0x40;

/// En_Zl4's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_ZELDAS_LETTER)).reads().sets().note("the letter given: she only talks")],
    todo: false,
    objects: &[],
};
/// `ENTR_CUTSCENE_MAP_0` (0x0A0), `ENTR_CASTLE_COURTYARD_ZELDA_1` (0x5F0) (`entrance_table.h`).
const ENTR_CUTSCENE_MAP_0: u16 = 0x0A0;
pub const ENTR_CASTLE_COURTYARD_ZELDA_1: u16 = 0x5F0;
/// `CS_INDEX_7` (`save.h`).
const CS_INDEX_7: u16 = 0xFFF7;
/// `GI_ZELDAS_LETTER` (`item.h`: 0x0B).
pub const GI_ZELDAS_LETTER: i16 = 0x0B;
/// `NA_BGM_APPEAR` (`sequence_table.h`: the 81st).
const NA_BGM_APPEAR: u16 = 0x51;
/// `HUD_VISIBILITY_NOTHING_ALT`, `HUD_VISIBILITY_ALL` (`save.h`).
const HUD_VISIBILITY_NOTHING_ALT: u16 = 2;
const HUD_VISIBILITY_ALL: u16 = 50;
/// `PLAYER_CSACTION_1` (stand still), `_7`, `_8` (`player.h`).
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;
/// `NPC_TRACKING_NONE`, `NPC_TRACKING_HEAD_AND_TORSO` (`actor.h`).
const NPC_TRACKING_NONE: i16 = 1;
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
/// `NPC_TALK_STATE_IDLE`, `NPC_TALK_STATE_TALKING`.
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;

// Her voice (`voicebank_table.h`).
const NA_SE_VO_Z0_HURRY: u16 = 0x6869;
const NA_SE_VO_Z0_MEET: u16 = 0x686A;
const NA_SE_VO_Z0_QUESTION: u16 = 0x686B;
const NA_SE_VO_Z0_SIGH_0: u16 = 0x686C;
const NA_SE_VO_Z0_SMILE_0: u16 = 0x686D;

// EnZl4CutsceneState.
const ZL4_CS_WAIT: u8 = 0;
const ZL4_CS_START: u8 = 1;
const ZL4_CS_MEET: u8 = 2;
const ZL4_CS_STONE: u8 = 3;
const ZL4_CS_NAMES: u8 = 4;
const ZL4_CS_LEGEND: u8 = 5;
const ZL4_CS_WINDOW: u8 = 6;
const ZL4_CS_GANON: u8 = 7;
const ZL4_CS_PLAN: u8 = 8;

// EnZl4EyeExpression.
const ZL4_EYES_NEUTRAL: u8 = 0;
const ZL4_EYES_SHUT: u8 = 1;
const ZL4_EYES_LOOK_LEFT: u8 = 2;
const ZL4_EYES_LOOK_RIGHT: u8 = 3;
const ZL4_EYES_WIDE: u8 = 4;
const ZL4_EYES_SQUINT: u8 = 5;
const ZL4_EYES_OPEN: u8 = 6;

// EnZl4MouthExpression.
const ZL4_MOUTH_NEUTRAL: u8 = 0;
const ZL4_MOUTH_HAPPY: u8 = 1;
const ZL4_MOUTH_WORRIED: u8 = 2;
const ZL4_MOUTH_SURPRISED: u8 = 3;

// EnZl4EyeState.
const ZL4_EYE_OPEN: i16 = 0;
const ZL4_EYE_SHUT: u8 = 2;
const ZL4_EYE_WIDE: u8 = 3;
const ZL4_EYE_SQUINT: u8 = 4;
const ZL4_EYE_LOOK_OUT: u8 = 5;
const ZL4_EYE_LOOK_IN: u8 = 6;

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
    dim: Cylinder16 { radius: 10, height: 44, y_shift: 0, pos: [0; 3] },
};

/// `sAnimationInfo`: by `EnZl4Animation`, the animation, its mode and its morph (speed 1, from 0,
/// to the last frame).
const S_ANIMATION_INFO: [(&str, u8, f32); 34] = [
    ("gChildZeldaAnim_000654", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_00E5C8", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_00EBC4", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_010DF8", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_011248", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_011698", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_011B34", ANIMMODE_LOOP, -8.0),
    ("gChildZeldaAnim_0125E4", ANIMMODE_ONCE, 0.0),
    ("gChildZeldaAnim_012E58", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_013280", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_013628", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_013A50", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_013EA0", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_015F14", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_0169B4", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_016D08", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_01726C", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_017818", ANIMMODE_LOOP, -8.0),
    ("gChildZeldaAnim_01805C", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_018898", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_01910C", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_019600", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_01991C", ANIMMODE_ONCE, -1.0),
    ("gChildZeldaAnim_01A2FC", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_01AAE0", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_01AE88", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_01B874", ANIMMODE_ONCE, 0.0),
    ("gChildZeldaAnim_01BCF0", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_01C494", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_01C7B0", ANIMMODE_LOOP, -1.0),
    ("gChildZeldaAnim_01CE08", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_00F0A4", ANIMMODE_ONCE, 0.0),
    ("gChildZeldaAnim_00F894", ANIMMODE_LOOP, 0.0),
    ("gChildZeldaAnim_000654", ANIMMODE_LOOP, -8.0),
];

// EnZl4Animation: the ones the code names.
const ZL4_ANIM_0: usize = 0;
const ZL4_ANIM_1: usize = 1;
const ZL4_ANIM_2: usize = 2;
const ZL4_ANIM_3: usize = 3;
const ZL4_ANIM_4: usize = 4;
const ZL4_ANIM_5: usize = 5;
const ZL4_ANIM_6: usize = 6;
const ZL4_ANIM_7: usize = 7;
const ZL4_ANIM_8: usize = 8;
const ZL4_ANIM_9: usize = 9;
const ZL4_ANIM_10: usize = 10;
const ZL4_ANIM_11: usize = 11;
const ZL4_ANIM_12: usize = 12;
const ZL4_ANIM_13: usize = 13;
const ZL4_ANIM_14: usize = 14;
const ZL4_ANIM_15: usize = 15;
const ZL4_ANIM_16: usize = 16;
const ZL4_ANIM_17: usize = 17;
const ZL4_ANIM_18: usize = 18;
const ZL4_ANIM_19: usize = 19;
const ZL4_ANIM_20: usize = 20;
const ZL4_ANIM_21: usize = 21;
const ZL4_ANIM_22: usize = 22;
const ZL4_ANIM_23: usize = 23;
const ZL4_ANIM_24: usize = 24;
const ZL4_ANIM_25: usize = 25;
const ZL4_ANIM_26: usize = 26;
const ZL4_ANIM_27: usize = 27;
const ZL4_ANIM_28: usize = 28;
const ZL4_ANIM_29: usize = 29;
const ZL4_ANIM_30: usize = 30;
const ZL4_ANIM_31: usize = 31;
const ZL4_ANIM_32: usize = 32;
const ZL4_ANIM_33: usize = 33;

/// `EnZl4_InMovingAnim`'s animations.
const MOVING_ANIMS: [&str; 15] = [
    "gChildZeldaAnim_01B874",
    "gChildZeldaAnim_01BCF0",
    "gChildZeldaAnim_0125E4",
    "gChildZeldaAnim_012E58",
    "gChildZeldaAnim_015F14",
    "gChildZeldaAnim_0169B4",
    "gChildZeldaAnim_016D08",
    "gChildZeldaAnim_01805C",
    "gChildZeldaAnim_01A2FC",
    "gChildZeldaAnim_01AAE0",
    "gChildZeldaAnim_01CE08",
    "gChildZeldaAnim_018898",
    "gChildZeldaAnim_01910C",
    "gChildZeldaAnim_00F0A4",
    "gChildZeldaAnim_00F894",
];

/// `EnZl4_Draw`'s `eyeTex` (by `EnZl4EyeState`) and `mouthTex` (by `mouthState`).
const EYE_TEXTURES: [&str; 7] = ["gChildZeldaEyeOpenTex", "gChildZeldaEyeBlinkTex", "gChildZeldaEyeShutTex", "gChildZeldaEyeWideTex", "gChildZeldaEyeSquintTex", "gChildZeldaEyeOutTex", "gChildZeldaEyeInTex"];
const MOUTH_TEXTURES: [&str; 4] = ["gChildZeldaMouthNeutralTex", "gChildZeldaMouthHappyTex", "gChildZeldaMouthWorriedTex", "gChildZeldaMouthSurprisedTex"];

/// The segments `EnZl4_Draw` binds her right eye, left eye and mouth to: the bake loads them from
/// there (`G_SETTIMG` on segment 8, 9, 0xA), and a draw puts each state's texture in their place.
const SEG_RIGHT_EYE: u32 = 0x0800_0000;
const SEG_LEFT_EYE: u32 = 0x0900_0000;
const SEG_MOUTH: u32 = 0x0A00_0000;

/// Her skeleton's bake.
pub const BAKE: &str = "En_Zl4/skel";

/// The mesh `EnZl4_Draw` draws: her skeleton without limbs 3 to 6, each face segment bound to a
/// texture of its own (the open eye, the blinking eye, the neutral mouth) so a draw can tell them
/// apart.
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(0x08, tex(EYE_TEXTURES[0])), (0x09, tex(EYE_TEXTURES[1])), (0x0A, tex(MOUTH_TEXTURES[0]))],
        prelude: Vec::new(),
        body: BakeBody::Skeleton {
            file: OBJECT.into(),
            symbol: "gChildZeldaSkel".into(),
            // EnZl4_OverrideLimbDraw: *dList = NULL for limbs 3 to 6.
            limbs: (3u8..7).map(|l| LimbOverride { limb: l - 1, file: OBJECT.into(), symbol: String::new() }).collect(),
        },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Cutscene,
    Idle,
    TheEnd,
}

/// `CutsceneCameraDirection`.
#[derive(Debug, Clone, Copy)]
struct CamDirection {
    at: Vec3,
    eye: Vec3,
    roll: i16,
    fov: i16,
}

pub struct EnZl4 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    pub interact_info: NpcTrack,
    pub talk_state: u8,
    pub cs_state: u8,
    pub left_eye_state: u8,
    pub right_eye_state: u8,
    pub mouth_state: u8,
    pub eye_expression: u8,
    pub mouth_expression: u8,
    pub unk_20f: u8,
    pub blink_timer: i16,
    pub talk_timer1: i16,
    pub talk_timer2: i16,
    pub cue_id: i16,
    /// Root motion queued this frame (`EnZl4_SetMove`'s `AnimTaskQueue_AddActorMovement`).
    movement_queued: bool,
    anims: Vec<Anim>,
    /// The face textures as images, by `EYE_TEXTURES` then `MOUTH_TEXTURES`.
    face: Vec<DrawImage>,
}

/// `DECR`: down to 0 and the new value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

/// `BINANG_TO_RAD_ALT`.
fn binang_to_rad_alt(b: i16) -> f32 {
    ((b as f32 / 32768.0) as f64 * std::f64::consts::PI) as f32
}

impl EnZl4 {
    /// `EnZl4_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gChildZeldaSkel")?;
            let anims = S_ANIMATION_INFO.iter().map(|(s, _, _)| assets.animation(OBJECT, s)).collect::<anyhow::Result<Vec<_>>>()?;
            let mut face = Vec::new();
            for s in EYE_TEXTURES.iter().chain(MOUTH_TEXTURES.iter()) {
                let t = assets.pack.texture(OBJECT, s)?;
                face.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, face))
        })();
        let (skeleton, anims, face) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Zl4: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnZl4 {
            // SkelAnime_InitFlex(.., gChildZeldaSkel, NULL, .., 18).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Cutscene,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            talk_state: 0,
            cs_state: 0,
            left_eye_state: 0,
            right_eye_state: 0,
            mouth_state: 0,
            eye_expression: 0,
            mouth_expression: 0,
            unk_20f: 0,
            blink_timer: 0,
            talk_timer1: 0,
            talk_timer2: 0,
            cue_id: 0,
            movement_queued: false,
            anims,
            face,
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 18.0);
        this.change_anim(ZL4_ANIM_21);
        this.collider.update(&this.actor);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.text_id = 0xFFFF;
        this.eye_expression = ZL4_MOUTH_NEUTRAL;
        this.mouth_expression = ZL4_MOUTH_NEUTRAL;
        if play.save.scene_layer > 3 {
            // IS_CUTSCENE_LAYER.
            this.change_anim(ZL4_ANIM_0);
            this.action = Action::TheEnd;
        } else if play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
            this.change_anim(ZL4_ANIM_0);
            this.action = Action::Idle;
        } else {
            if play.save.entrance_index != ENTR_CASTLE_COURTYARD_ZELDA_1 {
                this.change_anim(ZL4_ANIM_21);
                this.cs_state = ZL4_CS_WAIT;
            } else {
                this.setup_from_legend_cs(play);
                this.change_anim(ZL4_ANIM_0);
                this.cs_state = ZL4_CS_LEGEND;
            }
            this.talk_state = 0;
            this.action = Action::Cutscene;
        }
        Box::new(this)
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, i)`.
    fn change_anim(&mut self, i: usize) {
        let (_, mode, morph) = S_ANIMATION_INFO[i];
        let info = AnimationInfo { animation: self.anims[i].clone(), play_speed: 1.0, start_frame: 0.0, frame_count: -1.0, mode, morph_frames: morph };
        self.skel.change_by_info(&info);
    }

    /// `EnZl4_SetActiveCamDir`: the active camera to `CAM_SET_FREE0`, on fixed view `index`.
    fn set_active_cam_dir(play: &mut PlayState, index: usize) {
        let id = play.active_cam_id;
        play.camera_change_setting(id, CAM_SET_FREE0);
        let d = S_CAM_DIRECTIONS[index];
        if let Some(c) = play.camera_mut(id) {
            c.at = d.at;
            c.eye = d.eye;
            c.eye_next = d.eye;
            c.roll = d.roll;
            c.fov = d.fov as f32;
        }
    }

    /// `EnZl4_SetActiveCamMove`: the active camera to `CAM_SET_CS_0`, on move `index`.
    fn set_active_cam_move(play: &mut PlayState, index: usize) {
        let id = play.active_cam_id;
        play.camera_change_setting(id, CAM_SET_CS_0);
        let player = play.player_view();
        let (at, eye, relative) = S_CAM_MOVE[index];
        if let Some(c) = play.camera_mut(id) {
            c.reset_anim();
            if let Some(p) = &player {
                c.set_cs_params(at.to_vec(), eye.to_vec(), p, relative);
            }
        }
    }

    /// `EnZl4_GetTextId`: 0x703C, or 0x703D with two stones or more.
    fn get_text_id(play: &PlayState, _actor: &Actor) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_ZELDA): no masks yet, so 0.
        let s = &play.save;
        let mut stone_count = 0;
        if s.check_quest_item(oot_game::item::QUEST_KOKIRI_EMERALD) {
            stone_count = 1;
        }
        if s.check_quest_item(oot_game::item::QUEST_GORON_RUBY) {
            stone_count += 1;
        }
        if s.check_quest_item(oot_game::item::QUEST_ZORA_SAPPHIRE) {
            stone_count += 1;
        }
        if stone_count > 1 { 0x703D } else { 0x703C }
    }

    /// `EnZl4_UpdateTalkState`.
    fn update_talk_state(play: &mut PlayState, _actor: &mut Actor) -> i16 {
        if play.message_state() == TEXT_STATE_CLOSING { NPC_TALK_STATE_IDLE } else { NPC_TALK_STATE_TALKING }
    }

    /// `EnZl4_UpdateFace`.
    fn update_face(&mut self, play: &mut PlayState) {
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
        } else {
            self.blink_timer = 0;
        }
        if self.blink_timer <= 2 {
            self.left_eye_state = self.blink_timer as u8;
            self.right_eye_state = self.blink_timer as u8;
        }
        match self.eye_expression {
            ZL4_EYES_NEUTRAL => {
                if self.blink_timer == 0 {
                    self.blink_timer = play.rand.s16_offset(30, 30);
                }
            }
            ZL4_EYES_SHUT => {
                if self.blink_timer == 0 {
                    self.left_eye_state = ZL4_EYE_SHUT;
                    self.right_eye_state = ZL4_EYE_SHUT;
                }
            }
            ZL4_EYES_LOOK_LEFT => {
                if self.blink_timer == 0 {
                    self.left_eye_state = ZL4_EYE_LOOK_OUT;
                    self.right_eye_state = ZL4_EYE_LOOK_IN;
                }
            }
            ZL4_EYES_LOOK_RIGHT => {
                if self.blink_timer == 0 {
                    self.left_eye_state = ZL4_EYE_LOOK_IN;
                    self.right_eye_state = ZL4_EYE_LOOK_OUT;
                }
            }
            ZL4_EYES_WIDE => {
                if self.blink_timer == 0 {
                    self.left_eye_state = ZL4_EYE_WIDE;
                    self.right_eye_state = ZL4_EYE_WIDE;
                }
            }
            ZL4_EYES_SQUINT => {
                if self.blink_timer == 0 {
                    self.left_eye_state = ZL4_EYE_SQUINT;
                    self.right_eye_state = ZL4_EYE_SQUINT;
                }
            }
            ZL4_EYES_OPEN => {
                if self.blink_timer >= 3 {
                    self.blink_timer = ZL4_EYE_OPEN;
                }
            }
            _ => {}
        }
        self.mouth_state = match self.mouth_expression {
            ZL4_MOUTH_HAPPY => 1,
            ZL4_MOUTH_WORRIED => 2,
            ZL4_MOUTH_SURPRISED => 3,
            _ => 0,
        };
    }

    /// `EnZl4_SetMove`: `ANIM_FLAG_UPDATE_XZ`, and her root's motion queued.
    fn set_move(&mut self) {
        self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ;
        self.movement_queued = true;
    }

    /// `func_80B5BB78`: the head and torso on Link.
    fn func_80b5bb78(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        self.interact_info.target = player_pos;
        track_point(play, &mut self.actor, &mut self.interact_info, 2, NPC_TRACKING_HEAD_AND_TORSO);
    }

    /// Link snapped 56 in front of her, his speeds stopped.
    fn place_player_in_front(&self, play: &mut PlayState, dist: f32, rot_y: i16) {
        let mut pos = self.actor.world_pos;
        pos.x += dist * eng_math::sin_s(rot_y);
        pos.z += dist * eng_math::cos_s(rot_y);
        if let Some(h) = play.player {
            if let Some(a) = play.actors.actor_mut(h) {
                a.world_pos = pos;
                a.speed_xz = 0.0;
            }
            if let Some(p) = play.actors.downcast_mut::<Player>(h) {
                p.linear_velocity = 0.0;
            }
        }
    }

    /// `EnZl4_SetupFromLegendCs`: back from the legend, Link held 56 in front of her, her camera's
    /// move 5, the letterbox and the HUD gone.
    fn setup_from_legend_cs(&mut self, play: &mut PlayState) -> bool {
        play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_8);
        self.place_player_in_front(play, 56.0, self.actor.shape_rot.y);
        Self::set_active_cam_move(play, 5);
        play.letterbox.set_size_target(32);
        oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_NOTHING_ALT);
        self.talk_timer2 = 0;
        true
    }

    /// `EnZl4_SetNextAnim`: `next` once the animation is on its end frame.
    fn set_next_anim(&mut self, next: usize) -> bool {
        if !self.skel.on_frame(self.skel.end_frame) {
            return false;
        }
        self.change_anim(next);
        true
    }

    /// `EnZl4_ReverseAnimation`.
    fn reverse_animation(&mut self) {
        let temp_frame = self.skel.start_frame;
        self.skel.start_frame = self.skel.end_frame;
        self.skel.cur_frame = self.skel.end_frame;
        self.skel.end_frame = temp_frame;
        self.skel.play_speed = -1.0;
    }

    /// `(Message_GetState == state) && Message_ShouldAdvance`.
    fn advanced(play: &mut PlayState, state: u8) -> bool {
        play.message_state() == state && play.message_should_advance()
    }

    fn pause_box(play: &mut PlayState) {
        play.msg_ctx.msg_mode = MSGMODE_PAUSED;
    }

    /// `EnZl4_CsWaitForPlayer`: her talk offered to Link on her floor in her front half; taken, he's
    /// put 56 in front of her.
    fn cs_wait_for_player(&mut self, play: &mut PlayState) -> bool {
        if !oot_game::npc::process_talk_request(&mut self.actor) {
            let player_y = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(f32::MAX);
            // (f32)yawTowardsPlayer - shape.rot.y, into an s16.
            let yaw_diff = (self.actor.yaw_towards_player as f32 - self.actor.shape_rot.y as f32) as i32 as i16;
            let abs_yaw_diff = (yaw_diff as i32).abs() as i16;
            if player_y != self.actor.world_pos.y || abs_yaw_diff >= 0x3FFC {
                return false;
            }
            let a = self.actor.clone();
            offer_talk(play, &a, self.collider.dim.radius as f32 + 60.0);
            return false;
        }
        self.place_player_in_front(play, 56.0, self.actor.shape_rot.y);
        true
    }

    /// `EnZl4_CsMeetPlayer`.
    fn cs_meet_player(&mut self, play: &mut PlayState) -> bool {
        match self.talk_state {
            0 => {
                if self.skel.cur_frame == 50.0 {
                    audio_play_actor_sfx2(play, NA_SE_VO_Z0_MEET);
                }
                if self.set_next_anim(ZL4_ANIM_4) {
                    play.start_textbox(0x702E, None);
                    self.talk_state += 1;
                }
            }
            1 => {
                if Self::advanced(play, TEXT_STATE_EVENT) {
                    Self::set_active_cam_dir(play, 1);
                    play.start_textbox(0x702F, None);
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                }
            }
            2 => {
                if Self::advanced(play, TEXT_STATE_EVENT) {
                    play.cs_ctx.segment = play.cutscene_script("gZeldasCourtyardMeetCs");
                    play.save.cutscene_trigger = 1;
                    Self::set_active_cam_move(play, 0);
                    Self::pause_box(play);
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                }
            }
            3 => {
                self.talk_timer2 += 1;
                if self.talk_timer2 >= 45 {
                    play.start_textbox(0x70F9, None);
                    self.talk_state += 1;
                }
            }
            4 => {
                if Self::advanced(play, TEXT_STATE_EVENT) {
                    Self::set_active_cam_move(play, 1);
                    Self::pause_box(play);
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                }
            }
            5 => {
                self.talk_timer2 += 1;
                if self.talk_timer2 >= 10 {
                    play.start_textbox(0x70FA, None);
                    self.talk_state += 1;
                }
            }
            6 => {
                if Self::advanced(play, TEXT_STATE_EVENT) {
                    Self::set_active_cam_dir(play, 2);
                    self.change_anim(ZL4_ANIM_22);
                    self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                    play.start_textbox(0x70FB, None);
                }
            }
            _ => {}
        }
        self.talk_state == 7
    }

    /// The stone question's answer: yes (0x7032, squinting, laughing) or no (0x7031, worried, and
    /// `no_state` next).
    fn stone_answer(&mut self, play: &mut PlayState, no_state: u8) {
        if play.msg_ctx.choice_index == 0 {
            Self::set_active_cam_dir(play, 4);
            self.change_anim(ZL4_ANIM_28);
            self.blink_timer = 0;
            self.eye_expression = ZL4_EYES_SQUINT;
            self.mouth_expression = ZL4_MOUTH_HAPPY;
            play.start_textbox(0x7032, None);
            self.talk_state = 7;
        } else {
            Self::set_active_cam_dir(play, 2);
            self.change_anim(ZL4_ANIM_9);
            self.mouth_expression = ZL4_MOUTH_WORRIED;
            play.start_textbox(0x7031, None);
            self.talk_state = no_state;
        }
    }

    /// `EnZl4_CsAskStone`. The C's fallthroughs run a state's next case the same frame.
    fn cs_ask_stone(&mut self, play: &mut PlayState) -> bool {
        let mut state = self.talk_state;
        loop {
            match state {
                0 => {
                    if self.set_next_anim(ZL4_ANIM_25) {
                        self.talk_state += 1;
                    }
                    state = 1;
                    continue;
                }
                1 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 3);
                        Self::pause_box(play);
                        self.talk_timer1 = 40;
                        self.talk_state = 2;
                    }
                }
                2 => {
                    if decr(&mut self.talk_timer1) == 0 {
                        play.start_textbox(0x7030, None);
                        self.talk_state += 1;
                    }
                }
                3 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        self.stone_answer(play, 4);
                    }
                }
                4 => {
                    if self.skel.cur_frame == 16.0 {
                        audio_play_actor_sfx2(play, NA_SE_VO_Z0_QUESTION);
                    }
                    if self.set_next_anim(ZL4_ANIM_10) {
                        self.talk_state += 1;
                    }
                    state = 5;
                    continue;
                }
                5 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::pause_box(play);
                        self.change_anim(ZL4_ANIM_9);
                        self.mouth_expression = ZL4_MOUTH_WORRIED;
                        self.reverse_animation();
                        self.talk_state = 6;
                    }
                }
                6 => {
                    self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                    Self::set_active_cam_dir(play, 3);
                    play.start_textbox(0x7030, None);
                    self.talk_state = 12;
                }
                12 => {
                    if self.set_next_anim(ZL4_ANIM_25) {
                        self.talk_state = 13;
                    }
                    state = 13;
                    continue;
                }
                13 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        self.stone_answer(play, 4);
                    }
                }
                7 => {
                    if self.skel.cur_frame == 17.0 {
                        audio_play_actor_sfx2(play, NA_SE_VO_Z0_SMILE_0);
                    }
                    if self.set_next_anim(ZL4_ANIM_29) {
                        self.talk_state += 1;
                    }
                    state = 8;
                    continue;
                }
                8 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_move(play, 2);
                        self.change_anim(ZL4_ANIM_0);
                        self.blink_timer = 0;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                        play.start_textbox(0x70FC, None);
                        self.talk_state = 9;
                    }
                }
                9 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 5);
                        play.start_textbox(0x70FD, None);
                        self.talk_state += 1;
                    }
                }
                10 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_5);
                        self.eye_expression = ZL4_EYES_OPEN;
                        self.mouth_expression = ZL4_MOUTH_SURPRISED;
                        play.start_textbox(0x70FE, None);
                        self.talk_state += 1;
                    }
                }
                _ => {}
            }
            break;
        }
        self.talk_state == 11
    }

    /// `EnZl4_CsAskName`.
    fn cs_ask_name(&mut self, play: &mut PlayState) -> bool {
        let mut state = self.talk_state;
        loop {
            match state {
                0 => {
                    if self.set_next_anim(ZL4_ANIM_4) {
                        self.talk_state += 1;
                    }
                }
                1 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 6);
                        self.change_anim(ZL4_ANIM_1);
                        self.blink_timer = 11;
                        self.eye_expression = ZL4_EYES_SQUINT;
                        self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                        Self::pause_box(play);
                        play.start_textbox(0x70FF, None);
                        self.talk_state += 1;
                    }
                }
                2 => {
                    if self.set_next_anim(ZL4_ANIM_2) {
                        self.talk_state += 1;
                    }
                    state = 3;
                    continue;
                }
                3 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_16);
                        self.blink_timer = 0;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        Self::pause_box(play);
                        self.talk_state = 4;
                    }
                }
                4 => {
                    if self.set_next_anim(ZL4_ANIM_17) {
                        play.start_textbox(0x2073, None);
                        self.talk_state += 1;
                    }
                }
                5 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_move(play, 3);
                        self.change_anim(ZL4_ANIM_0);
                        Self::pause_box(play);
                        self.talk_timer2 = 0;
                        self.talk_state = 6;
                    }
                }
                6 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 15 {
                        play.start_textbox(0x2074, None);
                        self.talk_state += 1;
                    }
                }
                7 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_6);
                        self.mouth_expression = ZL4_MOUTH_HAPPY;
                        play.start_textbox(0x2075, None);
                        self.talk_state += 1;
                    }
                }
                8 => {
                    if self.set_next_anim(ZL4_ANIM_25) {
                        self.talk_state += 1;
                    }
                    state = 9;
                    continue;
                }
                9 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        play.start_textbox(0x7033, None);
                        self.talk_state = 10;
                    }
                }
                10 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        if play.msg_ctx.choice_index == 0 {
                            Self::set_active_cam_move(play, 4);
                            self.change_anim(ZL4_ANIM_33);
                            self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                            Self::pause_box(play);
                            self.talk_timer2 = 0;
                            self.talk_state = 15;
                        } else {
                            Self::set_active_cam_dir(play, 6);
                            Self::pause_box(play);
                            self.talk_timer1 = 20;
                            self.talk_state += 1;
                            self.skel.play_speed = 0.0;
                        }
                    }
                }
                11 => {
                    if decr(&mut self.talk_timer1) == 0 {
                        self.change_anim(ZL4_ANIM_11);
                        self.blink_timer = 11;
                        self.eye_expression = ZL4_EYES_LOOK_RIGHT;
                        self.mouth_expression = ZL4_MOUTH_WORRIED;
                        play.start_textbox(0x7034, None);
                        self.talk_state += 1;
                    }
                }
                12 => {
                    if self.skel.cur_frame == 5.0 {
                        audio_play_actor_sfx2(play, NA_SE_VO_Z0_SIGH_0);
                    }
                    if self.set_next_anim(ZL4_ANIM_12) {
                        self.talk_state += 1;
                    }
                    state = 13;
                    continue;
                }
                13 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_6);
                        self.blink_timer = 3;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_HAPPY;
                        Self::pause_box(play);
                        self.talk_state = 14;
                    }
                }
                14 => {
                    if self.set_next_anim(ZL4_ANIM_25) {
                        play.start_textbox(0x7033, None);
                        self.talk_state = 10;
                    }
                }
                15 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 30 {
                        play.start_textbox(0x7035, None);
                        self.talk_state += 1;
                    }
                }
                16 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::pause_box(play);
                        self.talk_state += 1;
                    }
                    state = 17;
                    continue;
                }
                17 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 == 130 {
                        Self::pause_box(play);
                        play.transition.next_entrance_index = ENTR_CUTSCENE_MAP_0;
                        play.save.next_cutscene_index = CS_INDEX_7;
                        play.transition.trigger = TRANS_TRIGGER_START;
                        play.transition.ty = TRANS_TYPE_FADE_WHITE;
                    }
                }
                _ => {}
            }
            break;
        }
        // @bug (game): `talkTimer2 == 17 && talkTimer2 > 130` is never true; the transition to the
        // legend ends the scene instead.
        self.talk_timer2 == 17 && self.talk_timer2 > 130
    }

    /// `EnZl4_CsTellLegend`.
    fn cs_tell_legend(&mut self, play: &mut PlayState) -> bool {
        let mut state = self.talk_state;
        loop {
            match state {
                0 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 60 {
                        play.start_textbox(0x7037, None);
                        self.talk_state += 1;
                    }
                }
                1 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 7);
                        play.start_textbox(0x2076, None);
                        self.talk_state += 1;
                    }
                }
                2 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_move(play, 6);
                        Self::pause_box(play);
                        self.talk_state += 1;
                    }
                }
                3 => {
                    if play.active_camera().anim_state() == 2 {
                        play.start_textbox(0x2077, None);
                        self.talk_state += 1;
                    }
                }
                4 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        if play.msg_ctx.choice_index == 0 {
                            Self::set_active_cam_dir(play, 8);
                            play.start_textbox(0x7005, None);
                            self.talk_state = 9;
                        } else {
                            self.change_anim(ZL4_ANIM_5);
                            self.mouth_expression = ZL4_MOUTH_SURPRISED;
                            play.start_textbox(0x7038, None);
                            self.talk_state += 1;
                            audio_play_actor_sfx2(play, NA_SE_VO_Z0_HURRY);
                        }
                    }
                }
                5 => {
                    if self.set_next_anim(ZL4_ANIM_4) {
                        self.talk_state += 1;
                    }
                    state = 6;
                    continue;
                }
                6 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_33);
                        self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                        play.start_textbox(0x7037, None);
                        self.talk_state += 1;
                    }
                }
                7 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        play.start_textbox(0x2076, None);
                        self.talk_state += 1;
                    }
                }
                8 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        play.start_textbox(0x2077, None);
                        self.talk_state = 4;
                    }
                }
                9 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_26);
                        play.start_textbox(0x2078, None);
                        self.talk_state += 1;
                    }
                }
                10 => {
                    if self.set_next_anim(ZL4_ANIM_27) {
                        self.talk_state += 1;
                    }
                    state = 11;
                    continue;
                }
                11 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        if play.msg_ctx.choice_index == 0 {
                            Self::pause_box(play);
                            self.talk_state = 13;
                        } else {
                            play.start_textbox(0x700B, None);
                            self.talk_state = 12;
                        }
                    }
                }
                12 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::pause_box(play);
                        self.talk_state = 13;
                    }
                }
                _ => {}
            }
            break;
        }
        self.talk_state == 13
    }

    /// `EnZl4_CsLookWindow`: the walk to the window held at frame 90, then Ganondorf's script held
    /// at frame 209.
    fn cs_look_window(&mut self, play: &mut PlayState) -> bool {
        match self.talk_state {
            0 => {
                Self::set_active_cam_move(play, 7);
                play.cs_ctx.segment = play.cutscene_script("gZeldasCourtyardWindowCs");
                play.save.cutscene_trigger = 1;
                self.talk_state += 1;
            }
            1 => {
                if play.cs_ctx.state != CS_STATE_IDLE {
                    if play.cs_ctx.frames == 90 {
                        play.cs_ctx.state = CS_STATE_STOP;
                    }
                } else {
                    play.cs_ctx.segment = play.cutscene_script("gZeldasCourtyardGanonCs");
                    play.save.cutscene_trigger = 1;
                    self.talk_state += 1;
                    play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_8);
                }
            }
            2 => {
                if play.cs_ctx.state != CS_STATE_IDLE {
                    if play.cs_ctx.frames == 209 {
                        play.cs_ctx.state = CS_STATE_STOP;
                    }
                } else {
                    // Rumble_Request(0, 160, 10, 40): no rumble.
                    play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
                    self.change_anim(ZL4_ANIM_30);
                    Self::set_active_cam_dir(play, 11);
                    play.start_textbox(0x7039, None);
                    self.talk_state += 1;
                }
            }
            3 => {
                if Self::advanced(play, TEXT_STATE_EVENT) {
                    Self::pause_box(play);
                    self.talk_state += 1;
                }
            }
            _ => {}
        }
        self.talk_state == 4
    }

    /// `EnZl4_CsWarnAboutGanon`.
    fn cs_warn_about_ganon(&mut self, play: &mut PlayState) -> bool {
        let mut state = self.talk_state;
        loop {
            match state {
                0 => {
                    // Link 34 to her side (a quarter turn off her facing): no speeds touched.
                    let rot_y = self.actor.shape_rot.y.wrapping_sub(0x3FFC);
                    let mut pos = self.actor.world_pos;
                    pos.x += 34.0 * eng_math::sin_s(rot_y);
                    pos.z += 34.0 * eng_math::cos_s(rot_y);
                    if let Some(a) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                        a.world_pos = pos;
                    }
                    Self::set_active_cam_move(play, 8);
                    self.blink_timer = 0;
                    self.eye_expression = ZL4_EYES_WIDE;
                    self.mouth_expression = ZL4_MOUTH_WORRIED;
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                    play.start_textbox(0x2079, None);
                    state = 1;
                    continue;
                }
                1 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 20 {
                        self.talk_state += 1;
                    }
                }
                2 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_move(play, 9);
                        Self::pause_box(play);
                        self.talk_timer2 = 0;
                        self.talk_state += 1;
                    }
                }
                3 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 20 {
                        play.start_textbox(0x207A, None);
                        self.talk_state += 1;
                    }
                }
                4 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 12);
                        self.change_anim(ZL4_ANIM_23);
                        self.blink_timer = 0;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_SURPRISED;
                        Self::pause_box(play);
                        self.talk_state += 1;
                    }
                }
                5 => {
                    if self.set_next_anim(ZL4_ANIM_24) {
                        play.start_textbox(0x207B, None);
                        self.talk_state += 1;
                    }
                }
                6 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        play.start_textbox(0x703A, None);
                        self.talk_state += 1;
                    }
                }
                7 => {
                    if Self::advanced(play, TEXT_STATE_CHOICE) {
                        if play.msg_ctx.choice_index == 0 {
                            self.change_anim(ZL4_ANIM_31);
                            self.blink_timer = 11;
                            self.eye_expression = ZL4_EYES_SQUINT;
                            self.mouth_expression = ZL4_MOUTH_HAPPY;
                            play.start_textbox(0x703B, None);
                            self.talk_state = 11;
                        } else {
                            self.change_anim(ZL4_ANIM_13);
                            self.blink_timer = 11;
                            self.eye_expression = ZL4_EYES_LOOK_LEFT;
                            self.mouth_expression = ZL4_MOUTH_WORRIED;
                            Self::pause_box(play);
                            self.talk_state += 1;
                        }
                    }
                }
                8 => {
                    if self.set_next_anim(ZL4_ANIM_15) {
                        self.blink_timer = 3;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_SURPRISED;
                        play.start_textbox(0x7073, None);
                        self.talk_state += 1;
                    }
                }
                9 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        self.change_anim(ZL4_ANIM_14);
                        Self::pause_box(play);
                        self.talk_state += 1;
                    }
                }
                10 => {
                    if self.set_next_anim(ZL4_ANIM_24) {
                        play.start_textbox(0x703A, None);
                        self.talk_state = 7;
                    }
                }
                11 => {
                    if self.set_next_anim(ZL4_ANIM_32) {
                        self.talk_state += 1;
                    }
                    state = 12;
                    continue;
                }
                12 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::pause_box(play);
                        self.talk_state = 13;
                    }
                }
                _ => {}
            }
            break;
        }
        self.talk_state == 13
    }

    /// `play->talkWithPlayer(play, &this->actor)` (`Player_StartTalking`) with her text -1: Link
    /// stands and looks at her (`PLAYER_CSACTION_1`) with no text, his item put away.
    fn talk_with_player(&mut self, play: &mut PlayState) {
        let me = play.cur_actor;
        let Some(ph) = play.player else { return };
        let data = play.data.clone();
        let Some(p) = play.actors.downcast_mut::<Player>(ph) else { return };
        let both = ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_TALK_WITH_C_UP;
        if p.target_actor.is_some() || (me.is_some() && p.navi_actor == me) || self.actor.flags & both == both {
            self.actor.flags |= ACTOR_FLAG_TALK;
        }
        p.target_actor = me;
        p.exchange_item_id = 0;
        // actor->textId == 0xFFFF.
        play.player_set_cs_action_with_halted_actors(me, PLAYER_CSACTION_1);
        self.actor.flags |= ACTOR_FLAG_TALK;
        if let Some(p) = play.actors.downcast_mut::<Player>(ph) {
            p.put_away_held_item(&data);
        }
    }

    /// `EnZl4_CsMakePlan`: her plan, and Zelda's Letter.
    fn cs_make_plan(&mut self, play: &mut PlayState) -> bool {
        let mut state = self.talk_state;
        loop {
            match state {
                0 => {
                    self.change_anim(ZL4_ANIM_18);
                    self.blink_timer = 0;
                    self.eye_expression = ZL4_EYES_NEUTRAL;
                    self.mouth_expression = ZL4_MOUTH_WORRIED;
                    Self::set_active_cam_move(play, 10);
                    self.talk_timer2 = 0;
                    self.talk_state += 1;
                    state = 1;
                    continue;
                }
                1 => {
                    self.talk_timer2 += 1;
                    if self.talk_timer2 >= 10 {
                        play.start_textbox(0x7123, None);
                        self.talk_state += 1;
                    }
                }
                2 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        Self::set_active_cam_dir(play, 13);
                        self.change_anim(ZL4_ANIM_19);
                        self.blink_timer = 0;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_SURPRISED;
                        play.start_textbox(0x207C, None);
                        self.talk_state += 1;
                    }
                }
                3 => {
                    if self.set_next_anim(ZL4_ANIM_20) {
                        self.talk_state += 1;
                    }
                    state = 4;
                    continue;
                }
                4 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        play.start_textbox(0x207D, None);
                        self.change_anim(ZL4_ANIM_7);
                        self.blink_timer = 0;
                        self.eye_expression = ZL4_EYES_NEUTRAL;
                        self.mouth_expression = ZL4_MOUTH_NEUTRAL;
                        self.talk_state = 5;
                        self.cue_id = 0;
                        self.unk_20f = 0;
                    }
                }
                5 => {
                    if self.set_next_anim(ZL4_ANIM_8) {
                        self.talk_state += 1;
                    }
                    state = 6;
                    continue;
                }
                6 => {
                    if Self::advanced(play, TEXT_STATE_EVENT) {
                        let id = play.active_cam_id;
                        play.camera_change_setting(id, CAM_SET_NORMAL0);
                        self.talk_state = 7;
                        self.talk_with_player(play);
                        let a = self.actor.clone();
                        offer_get_item_range(play, &a, GI_ZELDAS_LETTER, self.actor.xz_dist_to_player.abs() + 1.0, self.actor.y_dist_to_player.abs() + 1.0);
                        play.msg_ctx.state_timer = 4;
                        play.msg_ctx.msg_mode = MSGMODE_TEXT_CLOSING;
                    }
                }
                7 => {
                    if actor_has_parent(&self.actor) {
                        self.change_anim(ZL4_ANIM_0);
                        self.talk_state += 1;
                    } else {
                        let a = self.actor.clone();
                        offer_get_item_range(play, &a, GI_ZELDAS_LETTER, self.actor.xz_dist_to_player.abs() + 1.0, self.actor.y_dist_to_player.abs() + 1.0);
                    }
                }
                _ => {}
            }
            break;
        }
        self.talk_state == 8
    }

    /// `EnZl4_Cutscene`.
    fn cutscene(&mut self, play: &mut PlayState) {
        match self.cs_state {
            ZL4_CS_WAIT => {
                if self.cs_wait_for_player(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_START => {
                self.change_anim(ZL4_ANIM_3);
                self.blink_timer = 0;
                self.eye_expression = ZL4_EYES_NEUTRAL;
                self.mouth_expression = ZL4_MOUTH_SURPRISED;
                play.audio.play_fanfare(NA_BGM_APPEAR);
                Self::set_active_cam_dir(play, 0);
                oot_game::interface::change_alpha(&mut play.save, HUD_VISIBILITY_NOTHING_ALT);
                play.letterbox.set_size_target(32);
                self.talk_state = 0;
                self.cs_state += 1;
            }
            ZL4_CS_MEET => {
                if self.cs_meet_player(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_STONE => {
                if self.cs_ask_stone(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_NAMES => {
                if self.cs_ask_name(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_LEGEND => {
                if self.cs_tell_legend(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_WINDOW => {
                if self.cs_look_window(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_GANON => {
                if self.cs_warn_about_ganon(play) {
                    self.talk_state = 0;
                    self.cs_state += 1;
                }
            }
            ZL4_CS_PLAN => {
                if self.cs_make_plan(play) {
                    play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
                    play.save.prev_hud_visibility_mode = HUD_VISIBILITY_ALL;
                    play.save.set_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER);
                    self.action = Action::Idle;
                }
            }
            _ => {}
        }
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        self.interact_info.target = player_pos;
        let mode = if self.cs_state == ZL4_CS_WINDOW { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
        track_point(play, &mut self.actor, &mut self.interact_info, 2, mode);
        if self.in_moving_anim() {
            self.set_move();
        }
    }

    /// `EnZl4_InMovingAnim`.
    fn in_moving_anim(&self) -> bool {
        MOVING_ANIMS.iter().any(|a| self.skel.is(a))
    }

    /// `EnZl4_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        let range = self.collider.dim.radius as f32 + 60.0;
        let mut talk_state = self.interact_info.talk_state;
        talk_update(play, &mut self.actor, &mut talk_state, range, Self::get_text_id, Self::update_talk_state);
        self.interact_info.talk_state = talk_state;
        self.func_80b5bb78(play);
    }

    /// `EnZl4_TheEnd`: the credits' cues (`actorCues[0]`).
    fn the_end(&mut self, play: &mut PlayState) {
        const ANIM_INDEX: [usize; 10] = [ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_0, ZL4_ANIM_26, ZL4_ANIM_21, ZL4_ANIM_3];
        if self.skel.update() && self.skel.is("gChildZeldaAnim_010DF8") {
            self.change_anim(ZL4_ANIM_4);
        }
        if self.in_moving_anim() {
            self.set_move();
        }
        if play.cs_ctx.frames == 100 {
            self.eye_expression = ZL4_EYES_LOOK_LEFT;
        }
        if play.cs_ctx.frames == 450 {
            self.blink_timer = 3;
            self.eye_expression = ZL4_EYES_NEUTRAL;
            self.mouth_expression = ZL4_MOUTH_SURPRISED;
        }
        if let Some(cue) = play.cs_ctx.npc_actions[0] {
            let pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
            if self.cue_id == 0 {
                self.actor.world_pos = pos;
                self.actor.home_pos = pos;
            }
            if self.cue_id != cue.action as i16 {
                self.change_anim(ANIM_INDEX[(cue.action as usize).min(ANIM_INDEX.len() - 1)]);
                self.cue_id = cue.action as i16;
            }
            self.actor.velocity = Vec3::ZERO;
        }
    }

    /// Her skeleton posed as `EnZl4_OverrideLimbDraw` turns it: the head (limb 17) about a point 900
    /// along it by the head's yaw and pitch, the torso (limb 10) by the torso's.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, _rot| {
            if limb == 17 {
                let mut m = MtxF::IDENTITY;
                m.translate(900.0, 0.0, 0.0);
                m.rotate_x(binang_to_rad_alt(head[1]));
                m.rotate_z(binang_to_rad_alt(head[0]));
                m.translate(-900.0, 0.0, 0.0);
                return m.to_mat4();
            }
            if limb == 10 {
                let mut m = MtxF::IDENTITY;
                m.rotate_y(binang_to_rad_alt(torso[1]));
                m.rotate_x(binang_to_rad_alt(torso[0]));
                return m.to_mat4();
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnZl4 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnZl4_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.action != Action::TheEnd {
            self.skel.update();
        }
        self.update_face(play);
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Cutscene => self.cutscene(play),
            Action::Idle => self.idle(play),
            Action::TheEnd => self.the_end(play),
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
    /// `AnimTask_ActorMovement` for her queued move: her root's motion, at her scale.
    fn animation_update(&mut self) {
        if !std::mem::take(&mut self.movement_queued) {
            return;
        }
        let diff = self.skel.update_translation(self.actor.shape_rot.y);
        self.actor.world_pos.x += diff.x * self.actor.scale.x;
        self.actor.world_pos.y += diff.y * self.actor.scale.y;
        self.actor.world_pos.z += diff.z * self.actor.scale.z;
    }
    /// `EnZl4_PostLimbDraw`: the focus at the head (limb 17).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.interact_info.head, self.interact_info.torso);
        if let Some(head) = bones.get(16) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::ZERO);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.interact_info.head, self.interact_info.torso);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.switches = vec![self.right_eye_state as u32, self.left_eye_state as u32, self.mouth_state as u32];
        rs
    }
    /// `EnZl4_Draw`: her eyes on segments 8 (right) and 9 (left), her mouth on 0xA.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 || rs.switches.len() < 3 {
            return;
        }
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]]);
        let eye = |i: u32| self.face[(i as usize).min(EYE_TEXTURES.len() - 1)].clone();
        let mouth = self.face[EYE_TEXTURES.len() + (rs.switches[2] as usize).min(MOUTH_TEXTURES.len() - 1)].clone();
        let texture_images = vec![
            SourceImage { source: SEG_RIGHT_EYE, image: eye(rs.switches[0]) },
            SourceImage { source: SEG_LEFT_EYE, image: eye(rs.switches[1]) },
            SourceImage { source: SEG_MOUTH, image: mouth },
        ];
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones, params: DrawParams { texture_images, ..Default::default() } });
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

// z_en_zl4_cutscene_data.inc.c.

/// `sCamDirections`: the camera's fixed views (`EnZl4_SetActiveCamDir`): at, eye, roll, fov.
const S_CAM_DIRECTIONS: [CamDirection; 14] = [
    CamDirection { at: Vec3::new(-490.0, 120.0, 0.0), eye: Vec3::new(-440.0, 117.0, 0.0), roll: 0, fov: 45 },
    CamDirection { at: Vec3::new(-484.0, 122.0, -29.0), eye: Vec3::new(-480.0, 116.0, 18.0), roll: 0, fov: 80 },
    CamDirection { at: Vec3::new(-413.0, 136.0, -72.0), eye: Vec3::new(-403.0, 141.0, -89.0), roll: 0, fov: 25 },
    CamDirection { at: Vec3::new(-454.0, 120.0, 0.0), eye: Vec3::new(-434.0, 121.0, 0.0), roll: 0, fov: 20 },
    CamDirection { at: Vec3::new(-453.0, 107.0, -13.0), eye: Vec3::new(-440.0, 102.0, -25.0), roll: 0, fov: 40 },
    CamDirection { at: Vec3::new(-454.0, 105.0, 50.0), eye: Vec3::new(-453.0, 105.0, 66.0), roll: 0, fov: 60 },
    CamDirection { at: Vec3::new(-496.0, 119.0, 0.0), eye: Vec3::new(-443.0, 115.0, 0.0), roll: 0, fov: 45 },
    CamDirection { at: Vec3::new(-482.0, 119.0, 0.0), eye: Vec3::new(-440.0, 115.0, 0.0), roll: 0, fov: 30 },
    CamDirection { at: Vec3::new(-551.0, 119.0, 7.0), eye: Vec3::new(-587.0, 115.0, 14.0), roll: 0, fov: 25 },
    CamDirection { at: Vec3::new(-489.0, 129.0, 0.0), eye: Vec3::new(-470.0, 128.0, 0.0), roll: 0, fov: 40 },
    CamDirection { at: Vec3::new(-525.0, 126.0, 0.0), eye: Vec3::new(-509.0, 126.0, 0.0), roll: 0, fov: 10 },
    CamDirection { at: Vec3::new(-491.0, 120.0, -7.0), eye: Vec3::new(-509.0, 115.0, -7.0), roll: 0, fov: 75 },
    CamDirection { at: Vec3::new(-485.0, 119.0, -35.0), eye: Vec3::new(-484.0, 116.0, 15.0), roll: 0, fov: 45 },
    CamDirection { at: Vec3::new(-484.0, 123.0, -6.0), eye: Vec3::new(-435.0, 122.0, -11.0), roll: 0, fov: 45 },
];

const D_80B5EC70: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-484, 122, -29] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-484, 122, -29] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-484, 122, -29] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-452, 121, -21] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-452, 121, -21] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-452, 121, -21] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 20, view_angle: 80.79987, pos: [-452, 121, -21] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 80.79987, pos: [-452, 121, -21] },
];
const D_80B5ECF0: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 80.79987, pos: [-480, 116, 18] },
];
const D_80B5ED70: [CutsceneCameraPoint; 10] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 45.200058, pos: [-439, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 45.200058, pos: [-439, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 50.60008, pos: [-433, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 55.600098, pos: [-431, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 60.000114, pos: [-427, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 65.000114, pos: [-424, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 70.800026, pos: [-422, 116, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 20, view_angle: 75.59995, pos: [-419, 115, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 20, view_angle: 75.59995, pos: [-419, 116, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 75.59995, pos: [-419, 115, 0] },
];
const D_80B5EE10: [CutsceneCameraPoint; 10] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.400116, pos: [-480, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.200058, pos: [-480, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.200058, pos: [-474, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 50.60008, pos: [-472, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 55.600098, pos: [-468, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-465, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 65.000114, pos: [-463, 114, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 70.800026, pos: [-460, 114, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 75.59995, pos: [-460, 114, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 75.59995, pos: [-460, 114, 0] },
];
const D_80B5EEB0: [CutsceneCameraPoint; 14] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-116, 50, 469] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-116, 50, 469] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-118, 50, 467] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-120, 50, 465] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-123, 50, 464] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-126, 50, 463] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-129, 50, 462] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-131, 50, 462] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-134, 50, 463] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-137, 50, 464] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-140, 50, 466] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-140, 50, 466] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-140, 50, 466] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 25, view_angle: 60.000114, pos: [-140, 50, 466] },
];
const D_80B5EF90: [CutsceneCameraPoint; 14] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.399963, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-130, 50, 480] },
];
const D_80B5F070: [CutsceneCameraPoint; 9] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-427, 116, 4] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-426, 115, 4] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-429, 115, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-428, 114, 12] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-430, 114, 14] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-430, 114, 14] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-430, 114, 14] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-430, 112, 14] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-430, 112, 14] },
];
const D_80B5F100: [CutsceneCameraPoint; 9] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-389, 109, 36] },
];
const D_80B5F190: [CutsceneCameraPoint; 15] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.0, pos: [-496, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.0, pos: [-496, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.0, pos: [-496, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 45.0, pos: [-496, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 45.80006, pos: [-471, 122, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 45.80006, pos: [-395, 139, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 45.80006, pos: [-193, 183, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 45.80006, pos: [29, 232, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [360, 304, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 45.80006, pos: [429, 328, 0] },
];
const D_80B5F280: [CutsceneCameraPoint; 15] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.0, pos: [-443, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.0, pos: [-443, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.0, pos: [-443, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.0, pos: [-443, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [-420, 133, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [-344, 150, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [-143, 194, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [80, 243, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [412, 315, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [482, 332, 0] },
];
const D_80B5F370: [CutsceneCameraPoint; 14] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.600117, pos: [66, 404, 425] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 60.400116, pos: [66, 404, 425] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 55.600098, pos: [66, 404, 426] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 55.200096, pos: [63, 373, 413] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 50.400078, pos: [26, 353, 408] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 50.400078, pos: [17, 325, 397] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 45.40006, pos: [17, 326, 397] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 45.200058, pos: [-136, 177, 259] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 40.40004, pos: [-258, 111, 169] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 40.20004, pos: [-377, 108, 65] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 35.20002, pos: [-377, 108, 65] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 25, view_angle: 35.20002, pos: [-377, 108, 65] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 30.2, pos: [-376, 108, 65] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 30.2, pos: [-376, 108, 65] },
];
const D_80B5F450: [CutsceneCameraPoint; 14] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [29, 383, 445] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [29, 383, 445] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [29, 383, 445] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [29, 383, 445] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [29, 383, 445] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [49, 347, 424] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.80006, pos: [49, 347, 424] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-103, 192, 286] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-224, 113, 198] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-345, 109, 96] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-345, 109, 96] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-345, 109, 96] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-345, 109, 96] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 30.2, pos: [-345, 109, 96] },
];
const D_80B5F530: [CutsceneCameraPoint; 10] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.0, pos: [-482, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.0, pos: [-482, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.0, pos: [-482, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.800003, pos: [-482, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.800003, pos: [-482, 119, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 10, view_angle: 30.800003, pos: [-462, 126, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 30.800003, pos: [-395, 150, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 30.800003, pos: [-395, 150, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 30.800003, pos: [-395, 150, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 30.800003, pos: [-395, 150, 0] },
];
const D_80B5F5D0: [CutsceneCameraPoint; 10] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.0, pos: [-440, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.0, pos: [-440, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.0, pos: [-440, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-440, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-440, 115, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-423, 142, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-358, 171, 0] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-358, 171, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-358, 171, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 30.800003, pos: [-358, 171, 0] },
];
const D_80B5F670: [CutsceneCameraPoint; 9] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 25.0, pos: [-551, 119, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 25.0, pos: [-551, 119, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 25.0, pos: [-551, 119, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 15, view_angle: 60.0, pos: [-485, 120, -7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.0, pos: [-485, 120, -7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.0, pos: [-485, 120, -7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.0, pos: [-485, 118, -5] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.0, pos: [-485, 119, -6] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.0, pos: [-485, 119, -6] },
];
const D_80B5F700: [CutsceneCameraPoint; 9] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 45.200058, pos: [-587, 115, 14] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 25.400097, pos: [-587, 115, 14] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 25.400097, pos: [-587, 115, 14] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.20023, pos: [-521, 117, -1] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.20023, pos: [-521, 117, -1] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.20023, pos: [-521, 117, -1] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.20023, pos: [-521, 117, -1] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.20023, pos: [-521, 116, 0] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.19925, pos: [-521, 116, 0] },
];
const D_80B5F790: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [75, 52, 50] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [75, 52, 48] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 60, view_angle: 60.000114, pos: [74, 52, 45] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 60, view_angle: 60.000114, pos: [49, 51, -43] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [49, 51, -43] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [49, 51, -43] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [49, 51, -43] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [49, 51, -43] },
];
const D_80B5F810: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [90, 52, 53] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [90, 52, 50] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [90, 52, 45] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [65, 51, -44] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [65, 51, -44] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [65, 51, -44] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [65, 51, -44] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [65, 51, -44] },
];
const D_80B5F890: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 31, view_angle: 60.000114, pos: [-449, 121, -19] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-449, 121, -19] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-449, 121, -19] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-456, 110, -17] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-456, 110, -17] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-456, 110, -17] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-456, 110, -17] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 60.000114, pos: [-456, 110, -17] },
];
const D_80B5F910: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 60.000114, pos: [-441, 107, -22] },
];
const D_80B5F990: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-331, 110, -91] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-331, 110, -91] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-331, 110, -91] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-511, 121, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-511, 121, 7] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-511, 121, 7] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-511, 121, 7] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 30, view_angle: 20.799965, pos: [-511, 121, 7] },
];
const D_80B5FA10: [CutsceneCameraPoint; 8] = [
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-369, 110, -70] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-369, 110, -70] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-369, 110, -70] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-549, 124, 29] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-549, 124, 29] },
    CutsceneCameraPoint { continue_flag: 0, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-549, 124, 29] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-549, 124, 29] },
    CutsceneCameraPoint { continue_flag: -1, camera_roll: 0, next_point_frame: 0, view_angle: 20.799965, pos: [-549, 124, 29] },
];

/// `sCamMove`: the camera's moves (`EnZl4_SetActiveCamMove`): at points, eye points, relative to
/// Player.
const S_CAM_MOVE: [(&[CutsceneCameraPoint], &[CutsceneCameraPoint], i16); 11] = [
    (&D_80B5EC70, &D_80B5ECF0, 0),
    (&D_80B5ED70, &D_80B5EE10, 0),
    (&D_80B5EEB0, &D_80B5EF90, 0),
    (&D_80B5F070, &D_80B5F100, 0),
    (&D_80B5F190, &D_80B5F280, 0),
    (&D_80B5F370, &D_80B5F450, 0),
    (&D_80B5F530, &D_80B5F5D0, 0),
    (&D_80B5F670, &D_80B5F700, 0),
    (&D_80B5F790, &D_80B5F810, 0),
    (&D_80B5F890, &D_80B5F910, 0),
    (&D_80B5F990, &D_80B5FA10, 0),
];
