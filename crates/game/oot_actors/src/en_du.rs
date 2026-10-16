//! `En_Du` (`ovl_En_Du/z_en_du.c`): Darunia, the Gorons' boss.
//!
//! **Where he is** (`func_809FDDB4`): in Goron City for a child, and in the Fire Temple for an
//! adult without `INFTABLE_11A`; anywhere else his init kills him. With a cutscene index
//! (`cutsceneIndex >= CS_INDEX_0`: Goron City's layer 4) his init starts
//! `gGoronCityDaruniaDancingCs`; in the Fire Temple he waits for Link (`func_809FE638`); a
//! child's Darunia waits for the ocarina (`func_809FE3C0`); an adult's in Goron City (dead code:
//! killed first) does nothing (`func_809FE3B4`).
//!
//! **Goron City, a child.** He talks (`Npc_UpdateTalking` within his radius and 116, but not
//! while he listens to the ocarina): a mask's reaction first (`MaskReaction_GetTextId`,
//! `MASK_REACTION_SET_DARUNIA`); with the Goron's Bracelet 0x301D (0x301E with the Goron's Ruby);
//! else 0x301A, then 0x301B once it's read (`INFTABLE_113`, set as 0x301A closes). Within the same
//! range he asks for the ocarina (`PLAYER_STATE2_23`). Link's ocarina out (`PLAYER_STATE2_24`):
//! `Message_StartOcarinaSunsSongDisabled(OCARINA_ACTION_CHECK_SARIA)`, `PLAYER_STATE2_25`, Link's
//! ocarina turned to him (`unk_6A8`), and he listens (`func_809FE4A4`): the ocarina put away
//! (`OCARINA_MODE_04`) sends him back; another song (`OCARINA_MODE_06` and up) starts
//! `gGoronCityDaruniaWrongSongCs`; Saria's Song (`OCARINA_MODE_03`) the chime
//! (`NA_SE_SY_CORRECT_CHIME`) and `gGoronCityDaruniaCorrectSongCs`; each set on `csCtx` in place
//! (`cutsceneTrigger` 1), the ocarina's mode 04.
//!
//! **The cutscenes** (`func_809FE890`): his cue on channel 2 (`npcActions[2]`) places him at its
//! start the first time; a new cue changes his animation (1: standing, `ENDU_ANIM_1`; 7 and 8: the
//! dance, `ENDU_ANIM_7`) and his face (7: eyes shut, mouth open, nose happy; 8: eyes open, mouth
//! happy); cue 7 loops the dance's eight steps (`func_809FE040`), cue 8 plays the wrong song's four
//! (`func_809FE104`); his rotation is the cue's, his velocity from its start to its end over its
//! frames. The script over (`csCtx.state` idle): Link held (`PLAYER_CSACTION_1`), then
//! (`func_809FEB08`) his face back; after the wrong song Link let go (`PLAYER_CSACTION_7`) and he
//! waits again; after the right one 0x301C without the bracelet (its close: Link let go and
//! `Actor_OfferGetItem(GI_GORONS_BRACELET)` at Link's distance + 1 until he takes it,
//! `func_809FEC14` .. `func_809FECE4`), 0x301F with it, his dance's end (`ENDU_ANIM_14`, then 1).
//!
//! **The Fire Temple, an adult** (`func_809FE638` .. `func_809FE798`): once Link is free
//! (`PLAYER_STATE1_29` clear) one-point cutscene 3330 (99 frames; not ported: logged), Link turned
//! to face him, the fanfare `NA_BGM_APPEAR`; 50 frames on 0x3039; read, the camera's finished flag
//! and 90 frames of the door: `NA_SE_EV_CHAIN_KEY_UNLOCK_B` (80 left), `NA_SE_EV_SLIDE_DOOR_OPEN`
//! (60), `_CLOSE` (15), `NA_SE_EV_STONE_BOUND` (5), 10 a frame along -x while 61 or more are left;
//! then he's gone and `INFTABLE_11A` set.
//!
//! His head and torso follow Link (`Npc_TrackPoint`, preset 3, from 10 up; not while idle or in
//! a cutscene); he blinks (`func_809FDE9C`) and his face follows `unk_1EC` (eyes), `unk_1ED`
//! (mouth), `unk_1EE` (nose).
//!
//! Drawn as `EnDu_Draw` does: his eyes on segment 8, mouth on 9, nose on 0xA (texture swaps on
//! one bake, `DrawParams::texture_images`), the skeleton (`gDaruniaSkel`) opaque at full alpha
//! (`func_80034BA0`), the head (limb 16) turned about 2400 along it and the torso (limb 8)
//! (`EnDu_OverrideLimbDraw`), the focus 1000 below the head (`EnDu_PostLimbDraw`); with the circle
//! shadow's stand-in (scale 30).
//!
//! The whole overlay is ported; one-point cutscene 3330 logs (`OnePointCutscene_Init`, not
//! ported).
//!
//! Params: `params` whole unused (0xFFFF in the data: where he is and what he does is the scene's,
//! the age's and the story's).
//! Flags: reads and sets INFTABLE_113 (0x301A read: 0x301B next); reads and sets INFTABLE_11A (the
//! Fire Temple's meeting over: gone); sets EVENTCHKINF_22 (as 0x3020 closes in his talk: never, the
//! wrong song's 0x3020 is the cutscene's, not his); reads `gSaveContext.save.cutsceneIndex` (the
//! dance's layer).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SegmentValues, SourceImage};
use eng_math::binang_to_rad;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_DURING_OCARINA, Actor, Rot, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_CHAIN_KEY_UNLOCK_B, NA_SE_EV_SLIDE_DOOR_CLOSE, NA_SE_EV_SLIDE_DOOR_OPEN, NA_SE_EV_STONE_BOUND, NA_SE_SY_CORRECT_CHIME};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::cutscene::CS_STATE_IDLE;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{QUEST_GORON_RUBY, UPG_STRENGTH};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::player::{Player, STATE1_29, STATE2_23, STATE2_24, STATE2_25};

/// `ACTOR_EN_DU` (`actor_table.h`: 0x0098).
pub const ACTOR_EN_DU: i16 = 0x0098;
/// `OBJECT_DU` (`object_table.h`: 0x009D).
pub const OBJECT: &str = "object_du";

/// `En_Du_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY |
/// ACTOR_FLAG_UPDATE_DURING_OCARINA`).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DU,
    name: "En_Du",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_DURING_OCARINA,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The whole params: 0xFFFF in the game's data, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "0xFFFF in the data; read by nothing (the scene, the age and the story decide)");

// `INFTABLE_*`, `EVENTCHKINF_*` (`save.h`).
/// 0x301A read: 0x301B from then on.
pub const INFTABLE_113: u16 = 0x113;
/// The Fire Temple's meeting over: he's gone from it.
pub const INFTABLE_11A: u16 = 0x11A;
/// `EVENTCHKINF_22` (`save.h`: 0x22): set as 0x3020 closes in his talk.
pub const EVENTCHKINF_22: u16 = 0x22;

/// En_Du's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_113)).reads().sets().note("0x301A read: 0x301B next"),
        FlagUse::new(inf_table!(INFTABLE_11A)).reads().sets().note("the Fire Temple's meeting over (set as he leaves): not there"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_22)).sets().note("0x3020 closing in his talk: never (the wrong song's 0x3020 is the cutscene's)"),
        FlagUse::new(Flag::Other("gSaveContext.save.cutsceneIndex")).reads().note("a cutscene layer (CS_INDEX_0 and up): the dance (gGoronCityDaruniaDancingCs)"),
    ],
    todo: false,
};

// `SCENE_*` (`scene_table.h`).
pub const SCENE_FIRE_TEMPLE: u16 = 0x04;
pub const SCENE_GORON_CITY: u16 = 0x62;

/// `CS_INDEX_0` (`save.h`: 0xFFF0).
const CS_INDEX_0: u16 = 0xFFF0;
/// `GI_GORONS_BRACELET` (`item.h`: 0x54).
pub const GI_GORONS_BRACELET: i16 = 0x54;
/// `NA_BGM_APPEAR` (`sequence_table.h`: 0x51).
pub const NA_BGM_APPEAR: u16 = 0x51;
/// `OCARINA_MODE_06` (`ocarina.h`): another song than the one checked (6 and up).
const OCARINA_MODE_06: u16 = 0x06;
/// `PLAYER_CSACTION_1`, `PLAYER_CSACTION_7` (`player.h`): held, let go.
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
/// `ATTENTION_RANGE_1` (`actor.h`).
const ATTENTION_RANGE_1: u8 = 1;

/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;
pub const NPC_TALK_STATE_ITEM_GIVEN: i16 = 3;
/// `NpcTrackingMode` (`actor.h`).
const NPC_TRACKING_PLAYER_AUTO_TURN: i16 = 0;
const NPC_TRACKING_NONE: i16 = 1;

/// `MASK_REACTION_SET_DARUNIA` (`face_reaction.h`: 0x21).
const MASK_REACTION_SET_DARUNIA: usize = 0x21;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_DARUNIA]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_DARUNIA: [u16; 9] = [0x0000, 0x7199, 0x719A, 0x7198, 0x7198, 0x719A, 0x719A, 0x719B, 0x7198];

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
    dim: Cylinder16 { radius: 20, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnDuAnimation`: indices into `sAnimationInfo`.
pub const ENDU_ANIM_0: usize = 0;
pub const ENDU_ANIM_1: usize = 1;
pub const ENDU_ANIM_2: usize = 2;
pub const ENDU_ANIM_3: usize = 3;
pub const ENDU_ANIM_4: usize = 4;
pub const ENDU_ANIM_5: usize = 5;
pub const ENDU_ANIM_6: usize = 6;
pub const ENDU_ANIM_7: usize = 7;
pub const ENDU_ANIM_8: usize = 8;
pub const ENDU_ANIM_9: usize = 9;
pub const ENDU_ANIM_10: usize = 10;
pub const ENDU_ANIM_11: usize = 11;
pub const ENDU_ANIM_12: usize = 12;
pub const ENDU_ANIM_13: usize = 13;
pub const ENDU_ANIM_14: usize = 14;

/// `gDaruniaDancingEndAnim`, which `EnDu_Update` compares `skelAnime.animation` with.
pub const DANCING_END_ANIM: &str = "gDaruniaDancingEndAnim";

/// `sAnimationInfo`: animation, speed, start, end (-1: its last frame), mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 15] = [
    ("gDaruniaIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gDaruniaIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaItemGiveAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaItemGiveIdleAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaHitLinkAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaHitBreastAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaStandUpAfterFallingAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gDaruniaDancingLoop1Anim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -10.0),
    ("gDaruniaDancingLoop1Anim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, 0.0),
    ("gDaruniaDancingLoop2Anim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, 0.0),
    ("gDaruniaDancingLoop3Anim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, 0.0),
    ("gDaruniaWrongSongAnim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, 0.0),
    ("gDaruniaWrongSongEndAnim", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gDaruniaDancingLoop4Anim", 1.0, 0.0, -1.0, ANIMMODE_ONCE, 0.0),
    (DANCING_END_ANIM, 1.0, 0.0, -1.0, ANIMMODE_ONCE, -6.0),
];

/// `func_809FE040`'s `animationIndices`: the dance's eight steps (cue 7).
pub const DANCE_STEPS: [usize; 8] = [ENDU_ANIM_8, ENDU_ANIM_8, ENDU_ANIM_8, ENDU_ANIM_8, ENDU_ANIM_9, ENDU_ANIM_10, ENDU_ANIM_10, ENDU_ANIM_13];
/// `func_809FE104`'s: the wrong song's four (cue 8).
pub const WRONG_SONG_STEPS: [usize; 4] = [ENDU_ANIM_8, ENDU_ANIM_8, ENDU_ANIM_11, ENDU_ANIM_12];

/// The limbs `EnDu_OverrideLimbDraw` and `EnDu_PostLimbDraw` name (1-based, as `limbIndex`):
/// the torso and the head.
const DARUNIA_LIMB_TORSO: usize = 8;
const DARUNIA_LIMB_HEAD: usize = 16;

/// `EnDu_Draw`'s `eyeTextures`, `mouthTextures`, `noseTextures`.
const EYE_TEXTURES: [&str; 4] = ["gDaruniaEyeOpenTex", "gDaruniaEyeOpeningTex", "gDaruniaEyeShutTex", "gDaruniaEyeClosingTex"];
const MOUTH_TEXTURES: [&str; 4] = ["gDaruniaMouthSeriousTex", "gDaruniaMouthGrinningTex", "gDaruniaMouthOpenTex", "gDaruniaMouthHappyTex"];
const NOSE_TEXTURES: [&str; 2] = ["gDaruniaNoseSeriousTex", "gDaruniaNoseHappyTex"];
/// The segments `EnDu_Draw` binds: the eyes (8), the mouth (9), the nose (0xA); a draw puts its
/// textures in the bake's place by these addresses (each bound to a different texture).
const SEG_EYES: u32 = 0x0800_0000;
const SEG_MOUTH: u32 = 0x0900_0000;
const SEG_NOSE: u32 = 0x0A00_0000;
/// `gSPSegment(0x0C, func_80034B28(...))`: `func_80034BA0`'s empty list.
const SEG_RENDER_MODE: u8 = 0x0C;
/// `func_80034BA0`'s `gDPSetEnvColor(0, 0, 0, alpha)`, as a dynamic colour run before the
/// skeleton.
const SEG_ALPHA: u8 = 0x0E;

/// His skeleton's bake.
pub const BAKE: &str = "En_Du/skel";

/// The mesh `EnDu_Draw` draws: the skeleton with the open eyes, the serious mouth and nose bound
/// to their segments (the draw swaps the textures), opaque (`func_80034BA0`).
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![
            (0x08, tex(EYE_TEXTURES[0])),
            (0x09, tex(MOUTH_TEXTURES[0])),
            (0x0A, tex(NOSE_TEXTURES[0])),
            (SEG_RENDER_MODE, BakeSegment::Commands(Vec::new())),
            (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
        ],
        prelude: vec![SEG_ALPHA],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gDaruniaSkel".into(), limbs: Vec::new() },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_809FE3B4`: an adult's in Goron City: nothing.
    Func809FE3B4,
    /// `func_809FE3C0`: waiting for the ocarina (and talking).
    Func809FE3C0,
    /// `func_809FE4A4`: listening to the ocarina.
    Func809FE4A4,
    /// `func_809FE638`: the Fire Temple: waiting for Link to be free.
    Func809FE638,
    /// `func_809FE6CC`: 50 frames, then 0x3039.
    Func809FE6CC,
    /// `func_809FE740`: 0x3039 read.
    Func809FE740,
    /// `func_809FE798`: the door, then gone.
    Func809FE798,
    /// `func_809FE890`: on the cutscene's cues.
    Func809FE890,
    /// `func_809FEB08`: the cutscene over.
    Func809FEB08,
    /// `func_809FEC14`: 0x301C read.
    Func809FEC14,
    /// `func_809FEC70`: the bracelet offered.
    Func809FEC70,
    /// `func_809FECE4`: the bracelet's text read.
    Func809FECE4,
}

pub struct EnDu {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `unk_1E2`: the Fire Temple's timer.
    pub unk_1e2: i16,
    /// `unk_1E6`: the dance's step.
    pub unk_1e6: i16,
    /// `unk_1E8`: 1 the wrong song's cutscene, 0 the right one's.
    pub unk_1e8: i16,
    /// `unk_1EA`: the cue last seen.
    pub unk_1ea: i16,
    /// `unk_1EC`, `unk_1ED`, `unk_1EE`: the eyes', mouth's and nose's state.
    pub unk_1ec: u8,
    pub unk_1ed: u8,
    pub unk_1ee: u8,
    pub eye_tex_index: u8,
    pub mouth_tex_index: u8,
    pub nose_tex_index: u8,
    pub blink_timer: i16,
    /// `interactInfo`: the talk state and the head and torso tracking.
    pub interact_info: NpcTrack,
    animations: Vec<AnimationInfo>,
    /// The face textures as images: `EYE_TEXTURES`, `MOUTH_TEXTURES`, `NOSE_TEXTURES`.
    face: Vec<DrawImage>,
}

/// `DECR`: 0 when it was 0, else the decremented value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    let h = play.player?;
    play.actors.downcast_mut::<Player>(h)
}

fn set_player_state2(play: &mut PlayState, flags: u32) {
    if let Some(p) = player_mut(play) {
        p.state2 |= flags;
    }
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId` (`z_face_reaction.c`): `sMaskReactionSetTextIds[set][Player_GetMask]`,
/// here only Darunia's row (`MASK_REACTION_SET_DARUNIA`).
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = player(play).map_or(0, |p| p.current_mask) as usize;
    match set {
        MASK_REACTION_SET_DARUNIA => MASK_REACTION_DARUNIA.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `EnDu_GetTextId`: a mask's reaction; with the bracelet 0x301D (0x301E with the Ruby); else
/// 0x301A, then 0x301B.
pub fn get_text_id(play: &PlayState) -> u16 {
    let text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_DARUNIA);
    if text_id != 0 {
        return text_id;
    }
    let s = &play.save;
    if s.cur_upg_value(UPG_STRENGTH) != 0 {
        return if s.check_quest_item(QUEST_GORON_RUBY) { 0x301E } else { 0x301D };
    }
    if s.get_inf_table(INFTABLE_113) { 0x301B } else { 0x301A }
}

/// `EnDu_UpdateTalkState`: talking until the box closes; as it closes 0x301A sets `INFTABLE_113`,
/// 0x301C and 0x301F are his action, 0x3020 sets `EVENTCHKINF_22`; an item's text done (and
/// advanced): the item given.
fn update_talk_state(play: &mut PlayState, actor: &Actor) -> i16 {
    match play.message_state() {
        TEXT_STATE_CLOSING => {
            match actor.text_id {
                0x301A => play.save.set_inf_table(INFTABLE_113),
                0x301C | 0x301F => return NPC_TALK_STATE_ACTION,
                0x3020 => play.save.set_event_chk_inf(EVENTCHKINF_22),
                _ => {}
            }
            NPC_TALK_STATE_IDLE
        }
        TEXT_STATE_DONE => {
            if play.message_should_advance() {
                NPC_TALK_STATE_ITEM_GIVEN
            } else {
                NPC_TALK_STATE_TALKING
            }
        }
        // TEXT_STATE_NONE, _DONE_HAS_NEXT, _DONE_FADING, _CHOICE, _EVENT, _SONG_DEMO_DONE, _8, _9
        // (and the rest).
        _ => NPC_TALK_STATE_TALKING,
    }
}

impl EnDu {
    /// `func_809FDDB4`: here: Goron City for a child, the Fire Temple for an adult without
    /// `INFTABLE_11A`.
    pub fn is_here(play: &PlayState) -> bool {
        let adult = play.save.adult;
        if play.scene_id == SCENE_GORON_CITY && !adult {
            return true;
        }
        play.scene_id == SCENE_FIRE_TEMPLE && !play.save.get_inf_table(INFTABLE_11A) && adult
    }

    /// `EnDu_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gDaruniaSkel")?;
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames }))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let mut face = Vec::new();
            for s in EYE_TEXTURES.iter().chain(MOUTH_TEXTURES.iter()).chain(NOSE_TEXTURES.iter()) {
                let t = assets.pack.texture(OBJECT, s)?;
                face.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, animations, face))
        })();
        let (skeleton, animations, face) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Du: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnDu {
            actor,
            // SkelAnime_InitFlex(.., &gDaruniaSkel, NULL, NULL, NULL, 0).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::Func809FE3B4,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            unk_1e2: 0,
            unk_1e6: 0,
            unk_1e8: 0,
            unk_1ea: 0,
            unk_1ec: 0,
            unk_1ed: 0,
            unk_1ee: 0,
            eye_tex_index: 0,
            mouth_tex_index: 0,
            nose_tex_index: 0,
            blink_timer: 0,
            interact_info: NpcTrack::default(),
            animations,
            face,
        };
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 30).
        this.actor.shape_y_offset = 0.0;
        // Collider_SetCylinder (sCylinderInit), CollisionCheck_SetInfo2(DamageTable_Get(0x16)).
        this.actor.col_chk_info.set_info2(Some(&crate::en_ma1::S_DAMAGE_TABLE_PRESET_22), &COL_CHK_INFO_INIT);
        if !Self::is_here(play) {
            this.actor.kill();
            return Box::new(this);
        }
        this.change_anim(ENDU_ANIM_0);
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_1;
        this.interact_info.talk_state = NPC_TALK_STATE_IDLE;

        if play.save.cutscene_index >= CS_INDEX_0 {
            play.cs_ctx.segment = play.cutscene_script("gGoronCityDaruniaDancingCs");
            play.save.cutscene_trigger = 1;
            this.action = Action::Func809FE890;
        } else if play.scene_id == SCENE_FIRE_TEMPLE {
            this.action = Action::Func809FE638;
        } else if !play.save.adult {
            this.action = Action::Func809FE3C0;
        } else {
            this.action = Action::Func809FE3B4;
        }
        Box::new(this)
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, index)`.
    fn change_anim(&mut self, index: usize) {
        if let Some(info) = self.animations.get(index) {
            self.skel.change_by_info(info);
        }
    }

    /// `func_809FDE24`: the head and torso on Link, from 10 up; none while idle or in a cutscene.
    fn func_809fde24(&mut self, play: &mut PlayState) {
        let mut tracking_mode = NPC_TRACKING_PLAYER_AUTO_TURN;
        if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            tracking_mode = NPC_TRACKING_NONE;
        }
        if self.action == Action::Func809FE890 {
            tracking_mode = NPC_TRACKING_NONE;
        }
        self.interact_info.target = player(play).map_or(self.actor.world_pos, |p| p.actor.world_pos);
        self.interact_info.eye_height = 10.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 3, tracking_mode);
        self.interact_info = t;
    }

    /// `func_809FDE9C`: the blink (shut, opening, open, then 30 to 60 frames open), the eyes by
    /// `unk_1EC` (1, 2: held shut; 3: held open), the mouth by `unk_1ED`, the nose by `unk_1EE`.
    fn func_809fde9c(&mut self, play: &mut PlayState) {
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
        } else {
            self.blink_timer = 0;
        }
        if self.blink_timer < 3 {
            self.eye_tex_index = self.blink_timer as u8;
        }
        match self.unk_1ec {
            0 => {
                if self.blink_timer == 0 {
                    self.blink_timer = play.rand.s16_offset(30, 30);
                }
            }
            1 | 2 => {
                if self.blink_timer == 0 {
                    self.eye_tex_index = 2;
                }
            }
            3 => {
                if self.blink_timer == 0 {
                    self.eye_tex_index = 0;
                }
            }
            _ => {}
        }
        self.mouth_tex_index = match self.unk_1ed {
            1 => 1,
            2 => 2,
            3 => 3,
            _ => 0,
        };
        self.nose_tex_index = if self.unk_1ee == 1 { 1 } else { 0 };
    }

    /// `func_809FE040`: cue 7's dance: at each step's end the next of the eight (looping).
    fn func_809fe040(&mut self) {
        if self.skel.on_frame(self.skel.end_frame) {
            self.unk_1e6 += 1;
            if self.unk_1e6 >= 8 {
                self.unk_1e6 = 0;
            }
            self.change_anim(DANCE_STEPS[self.unk_1e6 as usize]);
        }
    }

    /// `func_809FE104`: cue 8's: at each step's end the next of the four, then stays on the last.
    fn func_809fe104(&mut self) {
        if self.unk_1e6 < 4 && self.skel.on_frame(self.skel.end_frame) {
            self.unk_1e6 += 1;
            if self.unk_1e6 < 4 {
                self.change_anim(WRONG_SONG_STEPS[self.unk_1e6 as usize]);
            }
        }
    }

    /// `func_809FE3C0`: the ocarina out (`PLAYER_STATE2_24`): the song's check and listening; his
    /// action from a talk: Link let go; within his radius and 116 the ocarina asked for.
    fn func_809fe3c0(&mut self, play: &mut PlayState) {
        let state2 = player(play).map_or(0, |p| p.state2);
        if state2 & STATE2_24 != 0 {
            play.message_start_ocarina_suns_song_disabled(OCARINA_ACTION_CHECK_SARIA);
            let me = play.cur_actor;
            if let Some(p) = player_mut(play) {
                p.state2 |= STATE2_25;
                p.unk_6a8 = me;
            }
            self.action = Action::Func809FE4A4;
            return;
        }
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        }
        if self.actor.xz_dist_to_player < 116.0 + self.collider.dim.radius as f32 {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_809FE4A4`: the ocarina put away: back to waiting; another song: the wrong song's
    /// cutscene; Saria's Song: the chime and the right one's; else the ocarina still asked for.
    fn func_809fe4a4(&mut self, play: &mut PlayState) {
        let mode = play.msg_ctx.ocarina_mode;
        if mode == OCARINA_MODE_04 {
            play.msg_ctx.ocarina_mode = OCARINA_MODE_00;
            self.action = Action::Func809FE3C0;
        } else if mode >= OCARINA_MODE_06 {
            play.cs_ctx.segment = play.cutscene_script("gGoronCityDaruniaWrongSongCs");
            play.save.cutscene_trigger = 1;
            self.unk_1e8 = 1;
            self.action = Action::Func809FE890;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else if mode == OCARINA_MODE_03 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            play.cs_ctx.segment = play.cutscene_script("gGoronCityDaruniaCorrectSongCs");
            play.save.cutscene_trigger = 1;
            self.unk_1e8 = 0;
            self.action = Action::Func809FE890;
            play.msg_ctx.ocarina_mode = OCARINA_MODE_04;
        } else {
            set_player_state2(play, STATE2_23);
        }
    }

    /// `func_809FE638`: Link free (`PLAYER_STATE1_29` clear): one-point cutscene 3330, Link turned
    /// to face him, the fanfare, 50 frames.
    fn func_809fe638(&mut self, play: &mut PlayState) {
        if player(play).is_some_and(|p| p.state1 & STATE1_29 != 0) {
            return;
        }
        let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
        // Not ported: onepoint_cutscene_set_info logs the number as the C does an unknown one.
        play.onepoint_cutscene_init(3330, -99, me, CAM_ID_MAIN);
        let yaw = self.actor.world_rot.y.wrapping_add(0x7FFF);
        if let Some(p) = player_mut(play) {
            p.actor.world_rot.y = yaw;
            p.actor.shape_rot.y = yaw;
        }
        play.audio.play_fanfare(NA_BGM_APPEAR);
        self.action = Action::Func809FE6CC;
        self.unk_1e2 = 0x32;
    }

    /// `func_809FE6CC`: the timer out: 0x3039.
    fn func_809fe6cc(&mut self, play: &mut PlayState) {
        if decr(&mut self.unk_1e2) == 0 {
            self.actor.text_id = 0x3039;
            play.start_textbox(self.actor.text_id, None);
            self.interact_info.talk_state = NPC_TALK_STATE_TALKING;
            self.action = Action::Func809FE740;
        }
    }

    /// `func_809FE740`: 0x3039 read: the camera's finished flag, 90 frames.
    fn func_809fe740(&mut self, play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            let active = play.active_cam_id;
            play.camera_set_finished_flag(active);
            self.unk_1e2 = 0x5A;
            self.action = Action::Func809FE798;
        }
    }

    /// `func_809FE798`: the door's sounds on the way, 10 along -x a frame while 61 or more are
    /// left; the timer out: gone, `INFTABLE_11A`.
    fn func_809fe798(&mut self, play: &mut PlayState) {
        if decr(&mut self.unk_1e2) != 0 {
            match self.unk_1e2 {
                0x50 => audio_play_actor_sfx2(play, NA_SE_EV_CHAIN_KEY_UNLOCK_B),
                0x3C => audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_OPEN),
                0xF => audio_play_actor_sfx2(play, NA_SE_EV_SLIDE_DOOR_CLOSE),
                5 => audio_play_actor_sfx2(play, NA_SE_EV_STONE_BOUND),
                _ => {}
            }
            if self.unk_1e2 >= 0x3D {
                self.actor.world_pos.x -= 10.0;
            }
        } else {
            self.actor.kill();
            play.save.set_inf_table(INFTABLE_11A);
        }
    }

    /// `func_809FE890`: the cutscene over: Link held, `func_809FEB08`. Else his cue on channel 2:
    /// placed at its start the first time; a new cue's animation and face; the dance's steps; the
    /// cue's rotation; a velocity from its start to its end over its frames.
    fn func_809fe890(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state == CS_STATE_IDLE {
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
            self.action = Action::Func809FEB08;
            return;
        }
        let Some(cue) = play.cs_ctx.npc_actions[2] else { return };
        // func_809FDFC0, func_809FE000.
        let start_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        let end_pos = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        if self.unk_1ea == 0 {
            self.actor.world_pos = start_pos;
        }
        let id = cue.action as i16;
        if self.unk_1ea != id {
            if id == 1 {
                self.change_anim(ENDU_ANIM_1);
            }
            if id == 7 || id == 8 {
                self.unk_1e6 = 0;
                self.change_anim(ENDU_ANIM_7);
            }
            self.unk_1ea = id;
            if self.unk_1ea == 7 {
                self.blink_timer = 11;
                self.unk_1ec = 2;
                self.unk_1ed = 2;
                self.unk_1ee = 1;
            }
            if self.unk_1ea == 8 {
                self.blink_timer = 11;
                self.unk_1ec = 3;
                self.unk_1ed = 3;
                self.unk_1ee = 0;
            }
        }
        if self.unk_1ea == 7 {
            self.func_809fe040();
        }
        if self.unk_1ea == 8 {
            self.func_809fe104();
        }
        self.actor.shape_rot = Rot { x: cue.rot[0], y: cue.rot[1], z: cue.rot[2] };
        self.actor.velocity = Vec3::ZERO;
        if play.cs_ctx.frames < cue.end_frame {
            let frame = (cue.end_frame as i32 - cue.start_frame as i32) as f32;
            self.actor.velocity.x = (end_pos.x - start_pos.x) / frame;
            self.actor.velocity.y = (end_pos.y - start_pos.y) / frame;
            self.actor.velocity.y += self.actor.gravity;
            if self.actor.velocity.y < self.actor.min_velocity_y {
                self.actor.velocity.y = self.actor.min_velocity_y;
            }
            self.actor.velocity.z = (end_pos.z - start_pos.z) / frame;
        }
    }

    /// `func_809FEB08`: his face back; after the wrong song Link let go and waiting again; after
    /// the right one 0x301C (no bracelet: then the bracelet) or 0x301F, the dance's end.
    fn func_809feb08(&mut self, play: &mut PlayState) {
        self.blink_timer = 11;
        self.unk_1ec = 0;
        self.unk_1ed = 0;
        self.unk_1ee = 0;
        if self.unk_1e8 == 1 {
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
            self.change_anim(ENDU_ANIM_1);
            self.action = Action::Func809FE3C0;
            return;
        }
        if play.save.cur_upg_value(UPG_STRENGTH) == 0 {
            self.actor.text_id = 0x301C;
            self.action = Action::Func809FEC14;
        } else {
            self.actor.text_id = 0x301F;
            self.action = Action::Func809FE3C0;
        }
        play.start_textbox(self.actor.text_id, None);
        self.change_anim(ENDU_ANIM_14);
        self.interact_info.talk_state = NPC_TALK_STATE_TALKING;
    }

    /// `func_809FEC14`: 0x301C read (his action): Link let go, the bracelet offered.
    fn func_809fec14(&mut self, play: &mut PlayState) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ACTION {
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
            self.action = Action::Func809FEC70;
            self.func_809fec70(play);
        }
    }

    /// `func_809FEC70`: taken (`Actor_HasParent`): wait for its text; else
    /// `Actor_OfferGetItem(GI_GORONS_BRACELET)` within Link's distance + 1 (both ways).
    fn func_809fec70(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::Func809FECE4;
        } else {
            let xz_range = self.actor.xz_dist_to_player + 1.0;
            offer_get_item_range(play, &self.actor, GI_GORONS_BRACELET, xz_range, self.actor.y_dist_to_player.abs() + 1.0);
        }
    }

    /// `func_809FECE4`: the item's text read: waiting again.
    fn func_809fece4(&mut self) {
        if self.interact_info.talk_state == NPC_TALK_STATE_ITEM_GIVEN {
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
            self.action = Action::Func809FE3C0;
        }
    }

    /// His skeleton posed as `EnDu_OverrideLimbDraw` turns it: the head (limb 16) about 2400 along
    /// it (x by the head's yaw, z by its pitch), the torso (limb 8) by y then x.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, _rot| {
            if limb == DARUNIA_LIMB_HEAD {
                return Mat4::from_translation(Vec3::new(2400.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-2400.0, 0.0, 0.0));
            }
            if limb == DARUNIA_LIMB_TORSO {
                return Mat4::from_rotation_y(r(torso[1])) * Mat4::from_rotation_x(r(torso[0]));
            }
            Mat4::IDENTITY
        })
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: the head's x, y, z, then the torso's.
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 3;
    /// `switches`: the eyes, the mouth, the nose.
    pub const EYES: usize = 0;
    pub const MOUTH: usize = 1;
    pub const NOSE: usize = 2;
}

impl ActorImpl for EnDu {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDu_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.skel.is(DANCING_END_ANIM) && self.skel.on_frame(self.skel.end_frame) {
            self.change_anim(ENDU_ANIM_1);
        }
        self.skel.update();
        self.func_809fde9c(play);
        self.func_809fde24(play);
        if self.action == Action::Func809FE890 {
            self.actor.world_pos += self.actor.velocity;
        } else {
            self.actor.update_pos();
        }
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if self.action != Action::Func809FE4A4 {
            // Npc_UpdateTalking(.., collider.dim.radius + 116, EnDu_GetTextId, EnDu_UpdateTalkState).
            let range = self.collider.dim.radius as f32 + 116.0;
            let mut talk_state = self.interact_info.talk_state;
            talk_update(play, &mut self.actor, &mut talk_state, range, |play, _| get_text_id(play), |play, actor| update_talk_state(play, actor));
            self.interact_info.talk_state = talk_state;
        }
        match self.action {
            Action::Func809FE3B4 => {}
            Action::Func809FE3C0 => self.func_809fe3c0(play),
            Action::Func809FE4A4 => self.func_809fe4a4(play),
            Action::Func809FE638 => self.func_809fe638(play),
            Action::Func809FE6CC => self.func_809fe6cc(play),
            Action::Func809FE740 => self.func_809fe740(play),
            Action::Func809FE798 => self.func_809fe798(play),
            Action::Func809FE890 => self.func_809fe890(play),
            Action::Func809FEB08 => self.func_809feb08(play),
            Action::Func809FEC14 => self.func_809fec14(play),
            Action::Func809FEC70 => self.func_809fec70(play),
            Action::Func809FECE4 => self.func_809fece4(),
        }
    }

    /// `EnDu_PostLimbDraw`: the focus 1000 below the head (limb 16) along it.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, t.head, t.torso);
        if let Some(head) = bones.get(DARUNIA_LIMB_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(0.0, -1000.0, 0.0));
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.interact_info.head, self.interact_info.torso);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.switches = vec![self.eye_tex_index as u32, self.mouth_tex_index as u32, self.nose_tex_index as u32];
        rs
    }

    /// `EnDu_Draw`: the eyes on segment 8, the mouth on 9, the nose on 0xA; `func_80034BA0` at
    /// alpha 255.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 6 || rs.switches.len() < 3 {
            return;
        }
        // ActorShadow_DrawCircle (shadowScale 30): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(30.0, 1.0, 30.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));

        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1], a[rs::HEAD + 2]], [a[rs::TORSO], a[rs::TORSO + 1], a[rs::TORSO + 2]]);
        let (eyes, mouths) = (EYE_TEXTURES.len(), MOUTH_TEXTURES.len());
        let eye = self.face[(rs.switches[rs::EYES] as usize).min(eyes - 1)].clone();
        let mouth = self.face[eyes + (rs.switches[rs::MOUTH] as usize).min(mouths - 1)].clone();
        let nose = self.face[eyes + mouths + (rs.switches[rs::NOSE] as usize).min(NOSE_TEXTURES.len() - 1)].clone();
        let texture_images = vec![SourceImage { source: SEG_EYES, image: eye }, SourceImage { source: SEG_MOUTH, image: mouth }, SourceImage { source: SEG_NOSE, image: nose }];
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, 255]);
        out.opa.push(DrawCmd {
            mesh: MeshKey::named(keys::bake(BAKE)),
            transform: actor_draw_matrix(rs),
            bones,
            params: DrawParams { segments: Some(sv), texture_images, ..Default::default() },
        });
    }

    /// `EnDu_Destroy`: `SkelAnime_Free`, `Collider_DestroyCylinder`.
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
