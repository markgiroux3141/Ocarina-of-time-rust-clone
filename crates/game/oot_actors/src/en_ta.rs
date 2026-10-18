//! `En_Ta` (`ovl_En_Ta/z_en_ta.c`): Talon. `params` -1 (`ENTA_CHILD_ERA`) is the child's Talon:
//! asleep by Lon Lon's crates at Hyrule Castle until he's shown the Cucco, at the ranch's house
//! after; 1 (`ENTA_IN_KAKARIKO`) the adult's, asleep in Kakariko's guest house until he's shown the
//! Pocket Cucco; 2 (`ENTA_RETURNED_FROM_KAKARIKO`) the adult's back at the ranch.
//!
//! **At the castle:** asleep (`EnTa_IdleAsleepInCastle`, `gTalonSleepAnim` with `NA_SE_VO_TA_SLEEP`
//! each loop), he offers a talk within 100 that takes the Cucco (`EXCH_ITEM_CHICKEN`); talked to he
//! says 0x702A in his sleep. Shown the Cucco (Player's text 0x702B), 40 frames on he wakes
//! (`gTalonWakeUpAnim`, `NA_SE_VO_TA_SURPRISE`, three quick blinks), his eyes half open as the text
//! ends, then he's awake (`EVENTCHKINF_TALON_WOKEN_IN_CASTLE`). Talked to awake (0x702C), on its
//! event he runs home (one-point camera 4175, `EVENTCHKINF_TALON_RETURNED_FROM_CASTLE`, the text
//! closed 45 frames on): turning, south 65 frames, west 65, out of the gate 60, raising dust, and
//! gone. He turns his head to Link while he talks (`Actor_TrackPlayer`); his chest and arms sway
//! (`EnTa_OverrideLimbDraw`'s fidget, always on: `@bug (game)`, the flag meant to stop it is cleared
//! at the root limb).
//!
//! **In the ranch's house** (`SCENE_LON_LON_BUILDINGS`, a child's, once he's back from the castle):
//! by night asleep (0x204B); by day he sits nodding off at his table (`gTalonSitSleepingAnim`,
//! `EnTa_IdleSittingInLonLonHouse`, updated wherever he is) with his three super cuccos about him
//! (`En_Niw` 0xD, spawned by his init at (+5, +3, +26), (-20, +40, -30) and (+20, +40, -30)).
//! Talked to (within 100, in front; `NA_SE_VO_TA_SURPRISE`, he wakes, `gTalonSitWakeUpAnim`): 0x207E
//! the first time (then 0x207F), 0x207F "play the Super Cucco game for 10 rupees?", or once he's given
//! the bottle (`ITEMGETINF_TALON_BOTTLE`) 0x208B: buy milk (30 rupees and an empty bottle: `GI_MILK`;
//! without the bottle 0x208A), play, or not. Short of rupees, 0x85; "no", back to his nap.
//!
//! **The cucco game** (`EnTa_StartCuccoGame` to `EnTa_RunCuccoGame`): 0x2080, his hands up
//! (`gTalonSitHandsUpAnim`), and on the text's event he throws the cuccos (`EnTa_ThrowSuperCuccos`,
//! 50 frames: the two on the table turn, then at 35 each flies off, `En_Niw`'s `unk_308`, its
//! gravity coming back 0.03 a frame after 7 to 12 frames); Link held meanwhile
//! (`PLAYER_CSACTION_1`, let go with 7), the timer at 30 (`Interface_SetTimer`), the timed
//! mini-game's music (`func_800F5ACC(NA_BGM_TIMED_MINI_GAME)`, the old music back when he goes,
//! `func_800F5B58`). A super cucco Link lifts and holds 7 frames (`superCuccoTimers`) is found: Talon
//! talks at once (`ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED`): 0x2082 (two left, `NA_SE_VO_TA_SURPRISE`),
//! 0x2083 (one left, `NA_SE_VO_TA_CRY_1`), and the found cucco flies up out of Link's hands and
//! away; the last one is 0x2084, the game won (`EVENTINF_CUCCO_GAME_WON`, the music stopped,
//! `NA_BGM_SMALL_ITEM_GET`). At 10 seconds left the music speeds up; at 0 (not in a cutscene)
//! `NA_SE_SY_FOUND`, 0x2081 "time's up" and the floor camera (`EnTa_CreateFloorCamera`: from (1053,
//! 11, 22) at (1053, 45, -40), gone 22 frames after the text). Either way the text's event sends
//! Link to `ENTR_LON_LON_BUILDINGS_2` (by Talon) through a starburst circle, white if he won
//! (`EVENTINF_CUCCO_GAME_FINISHED`).
//!
//! **Back in the house** (`EnTa_IdleAfterCuccoGameFinished`, he talks at once): the first win 0x2086
//! (the "marry Malon" choice; either answer 0x2087, then a full bottle of Lon Lon Milk,
//! `GI_BOTTLE_MILK_FULL`, `ITEMGETINF_TALON_BOTTLE`); a later win 0x2088 (milk into an empty
//! bottle, `GI_MILK`, else 0x208A); a loss 0x2085 (again for 5 rupees?).
//!
//! **Ported:** the whole overlay. Kakariko's Talon is killed for a child, so his talk
//! (`EXCH_ITEM_POCKET_CUCCO`) waits for the adult. `MaskReaction_GetTextId` reads Player's mask,
//! which is always none until the masks are ported (BACKLOG #39): the mask texts (0x7178 to 0x717B)
//! never show.
//!
//! Params: `params` whole (`EnTaType`: -1 `ENTA_CHILD_ERA` the child's Talon, at Hyrule Castle or
//! in the ranch's house by the scene, and anything else not below; 1 `ENTA_IN_KAKARIKO` the adult's
//! in Kakariko; 2 `ENTA_RETURNED_FROM_KAKARIKO` the adult's back at the ranch, by day in the stable).
//! Flags: reads and sets EVENTCHKINF_TALON_WOKEN_IN_CASTLE, EVENTCHKINF_TALON_RETURNED_FROM_CASTLE
//! (type -1), EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO and EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO (types
//! 1 and 2); reads EVENTCHKINF_EPONA_OBTAINED (type 1 awake: 0x5017, then back to the ranch). The
//! ranch house's cucco game (type -1) reads, sets and clears EVENTINF_CUCCO_GAME_FINISHED and
//! EVENTINF_CUCCO_GAME_WON, reads and sets ITEMGETINF_TALON_BOTTLE (the bottle given) and
//! INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE (talked to there).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::{event_chk_inf, event_inf, inf_table, item_get_inf};
use oot_game::audio::bgm::seq_cmd1;
use oot_game::audio::sfx::{NA_SE_PL_WALK_GROUND, NA_SE_SY_FOUND};
use oot_game::audio::{NA_BGM_SMALL_ITEM_GET, NA_BGM_TIMED_MINI_GAME, SEQ_PLAYER_BGM_MAIN};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{inventory_has_empty_bottle, rupees_change_by};
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{EXCH_ITEM_NONE, actor_track_player, get_player_exchange_item_id, offer_talk, offer_talk_exchange, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::TIMER_STATE_OFF;
use oot_game::skelanime_std::*;
use oot_game::transition::{TCA_STARBURST, TCC_BLACK, TCC_WHITE, TCS_FAST, TRANS_TRIGGER_START, TRANS_TYPE_FADE_BLACK, TRANS_TYPE_FADE_WHITE, trans_type_circle};

use crate::en_niw::{ACTOR_EN_NIW, EnNiw};
use crate::player::{Player, STATE1_11};

/// `ACTOR_EN_TA` (`actor_table.h`: 0x0084).
pub const ACTOR_EN_TA: i16 = 0x0084;
pub const OBJECT: &str = "object_ta";

/// `En_Ta_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TA, name: "En_Ta", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT, schema: &SCHEMA };

/// `EnTaType`.
pub const ENTA_CHILD_ERA: i16 = -1;
pub const ENTA_IN_KAKARIKO: i16 = 1;
pub const ENTA_RETURNED_FROM_KAKARIKO: i16 = 2;

// `TALON_STATE_FLAG_*`.
pub const TALON_STATE_FLAG_TRACKING_PLAYER: u16 = 1 << 0;
pub const TALON_STATE_FLAG_GIVING_MILK_REFILL: u16 = 1 << 1;
pub const TALON_STATE_FLAG_SUPPRESS_BLINK: u16 = 1 << 2;
/// Meant to stop the sway; no effect (`EnTa_OverrideLimbDraw`).
pub const TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM: u16 = 1 << 3;
pub const TALON_STATE_FLAG_ANIMATION_FINISHED: u16 = 1 << 4;
pub const TALON_STATE_FLAG_CUCCO_GAME_START_EVENT_TRIGGERED: u16 = 1 << 5;
pub const TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE: u16 = 1 << 7;
pub const TALON_STATE_FLAG_RAISING_HANDS: u16 = 1 << 8;
pub const TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY: u16 = 1 << 9;

// `TalonCanBuyMilkResult`.
const TALON_CANBUYMILK_NOT_ENOUGH_RUPEES: i32 = 0;
const TALON_CANBUYMILK_NO_EMPTY_BOTTLE: i32 = 1;
const TALON_CANBUYMILK_SUCCESS: i32 = 2;

/// `GI_BOTTLE_MILK_FULL`, `GI_MILK` (`item.h`: 0x14, 0x50).
pub const GI_BOTTLE_MILK_FULL: i16 = 0x14;
pub const GI_MILK: i16 = 0x50;
/// `ACTOR_EN_NIW`'s params for a super cucco (`En_Niw` 0xD).
pub const SUPER_CUCCO_PARAMS: i16 = 0xD;
/// `ENTR_LON_LON_BUILDINGS_2` (`entrance_table.h`: 0x5E4): spawn 2, by Talon in room 1.
pub const ENTR_LON_LON_BUILDINGS_2: u16 = 0x5E4;
/// `PLAYER_CSACTION_1`, `PLAYER_CSACTION_7` (`player.h`): Link held, let go.
const PLAYER_CSACTION_1: u8 = 1;
const PLAYER_CSACTION_7: u8 = 7;
/// `EnTa_CreateFloorCamera`'s eye and at.
pub const FLOOR_CAMERA_EYE: Vec3 = Vec3::new(1053.0, 11.0, 22.0);
pub const FLOOR_CAMERA_AT: Vec3 = Vec3::new(1053.0, 45.0, -40.0);

/// `MASK_REACTION_SET_TALON` (`face_reaction.h`: 0x18).
const MASK_REACTION_SET_TALON: usize = 0x18;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_TALON]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_TALON: [u16; 9] = [0x0000, 0x7178, 0x7179, 0x7179, 0x717B, 0x717A, 0x717B, 0x717A, 0x717B];

// `TalonEyeIndex`.
pub const TALON_EYE_INDEX_OPEN: i16 = 0;
pub const TALON_EYE_INDEX_HALF: i16 = 1;
pub const TALON_EYE_INDEX_CLOSED: i16 = 2;
const TALON_EYE_INDEX_MAX: i16 = 3;

// `TalonLimb` (`object_ta.xml`, counted from 1).
const TALON_LIMB_CHEST: usize = 8;
const TALON_LIMB_LEFT_ARM: usize = 10;
const TALON_LIMB_RIGHT_ARM: usize = 13;
const TALON_LIMB_HEAD: usize = 15;

// `SCENE_*` (`scene_table.h`).
const SCENE_STABLE: u16 = 0x36;
const SCENE_LON_LON_BUILDINGS: u16 = 0x4C;
pub const SCENE_HYRULE_CASTLE: u16 = 0x5F;

// `EVENTCHKINF_*` (`save.h`).
pub const EVENTCHKINF_TALON_WOKEN_IN_CASTLE: u16 = 0x13;
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
const EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO: u16 = 0x6A;
const EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO: u16 = 0x6B;
/// The cucco game's (`save.h`): `EVENTINF_CUCCO_GAME_WON` (`EVENTINF_HORSES_08`),
/// `EVENTINF_CUCCO_GAME_FINISHED` (`EVENTINF_HORSES_0A`), `ITEMGETINF_TALON_BOTTLE`,
/// `INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE`.
pub const EVENTINF_CUCCO_GAME_WON: u16 = 0x08;
pub const EVENTINF_CUCCO_GAME_FINISHED: u16 = 0x0A;
pub const ITEMGETINF_TALON_BOTTLE: u16 = 0x02;
pub const INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE: u16 = 0x7E;

/// `switch (this->actor.params)`: which Talon.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "ENTA_CHILD_ERA"), (1, "ENTA_IN_KAKARIKO"), (2, "ENTA_RETURNED_FROM_KAKARIKO")]),
    "the child's Talon (castle or ranch house, by the scene), or the adult's in Kakariko or back at the ranch",
);

/// En_Ta's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_WOKEN_IN_CASTLE)).reads().sets().note("type -1: woken by the Cucco"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().sets().note("type -1: gone home to the ranch"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO)).reads().sets().note("type 1: woken by the Pocket Cucco"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO)).reads().sets().note("types 1 and 2: back at the ranch"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("type 1 awake: 0x5017, then back to the ranch"),
        FlagUse::new(event_inf!(EVENTINF_CUCCO_GAME_FINISHED)).reads().sets().clears().note("type -1: the ranch house's cucco game just played (he talks at once)"),
        FlagUse::new(event_inf!(EVENTINF_CUCCO_GAME_WON)).reads().sets().clears().note("type -1: the ranch house's cucco game won"),
        FlagUse::new(item_get_inf!(ITEMGETINF_TALON_BOTTLE)).reads().sets().note("type -1: the cucco game's bottle of milk given"),
        FlagUse::new(inf_table!(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE)).reads().sets().note("type -1: talked to in the ranch house (0x207E once)"),
    ],
    todo: false,
};

/// `EXCH_ITEM_CHICKEN`, `EXCH_ITEM_POCKET_CUCCO` (`item.h`: 0x03, 0x06).
pub const EXCH_ITEM_CHICKEN: u8 = 0x03;
pub const EXCH_ITEM_POCKET_CUCCO: u8 = 0x06;

/// `NA_SE_VO_TA_SLEEP`, `NA_SE_VO_TA_SURPRISE`, `NA_SE_VO_TA_CRY_1` (`voicebank_table.h`: 0x6850,
/// 0x6851, 0x6853).
pub const NA_SE_VO_TA_SLEEP: u16 = 0x6850;
pub const NA_SE_VO_TA_SURPRISE: u16 = 0x6851;
pub const NA_SE_VO_TA_CRY_1: u16 = 0x6853;
/// `SURFACE_SFX_OFFSET_DIRT` (`bgcheck.h`: 0).
const SURFACE_SFX_OFFSET_DIRT: u16 = 0;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_FREQ_Y: i32 = 0x814;
const FIDGET_FREQ_Z: i32 = 0x940;
const FIDGET_FREQ_LIMB: i32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0004, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 40, y_shift: 0, pos: [0; 3] },
};

/// `EnTa_Draw`'s `eyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gTalonEyeOpenTex", "gTalonEyeHalfTex", "gTalonEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
const SEG_HEAD_SKIN: u8 = 0x09;
const SEG_SETUP: u8 = 0x0C;

/// The skeleton with each eye (`gSPSegment(8, eyeTextures[eyeIndex])`, `gSPSegment(9,
/// gTalonHeadSkinTex)`), after `Gfx_SetupDL_37Opa`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Ta/eye{eye}")
}

pub fn bakes() -> Vec<MeshBake> {
    (0..EYE_TEXTURES.len())
        .map(|eye| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![
                (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }),
                (SEG_HEAD_SKIN, BakeSegment::Texture { file: OBJECT.into(), symbol: "gTalonHeadSkinTex".into() }),
                (SEG_SETUP, BakeSegment::Commands(crate::en_owl::setup_dl_37())),
            ],
            prelude: vec![SEG_SETUP],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gTalonSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    SleepTalkInKakariko,
    SleepTalkInLonLonHouse,
    TalkWakingUp2,
    TalkWakingUp1,
    WakeUp,
    SleepTalkInCastle,
    IdleAsleepInCastle,
    IdleAsleepInLonLonHouse,
    IdleAsleepInKakariko,
    RunAwayRunOutOfGate,
    RunAwayTurnTowardsGate,
    RunAwayRunWest,
    RunAwayTurnWest,
    RunAwayRunSouth,
    RunAwayStart,
    TalkAwakeInCastle,
    IdleAwakeInCastle,
    TalkAwakeInKakariko,
    IdleAwakeInKakariko,
    TalkAtRanch,
    IdleAtRanch,
    TalkNotEnoughRupees,
    TalkFoundSuperCucco,
    IdleFoundSuperCucco,
    TransitionToPostCuccoGame,
    TalkCuccoGameEnd,
    RunCuccoGame,
    ThrowSuperCuccos,
    StartingCuccoGame3,
    StartingCuccoGame2,
    StartingCuccoGame1,
    TalkGeneralInLonLonHouse,
    GiveItemInLonLonHouse,
    TalkAfterCuccoGameFirstWon,
    WaitBuyMilkOrPlayCuccoGameResponse,
    WaitForPlayCuccoGameResponse,
    WaitForMarryMalonResponse,
    ContinueTalkInLonLonHouse,
    TalkAfterCuccoGameWon,
    IdleSittingInLonLonHouse,
    IdleAfterCuccoGameFinished,
}

/// `animFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimFunc {
    RepeatCurrent,
    Sleeping,
    SitSleeping,
    RunToEnd,
}

/// `blinkFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkFunc {
    WaitUntilNext,
    AdvanceState,
}

pub struct EnTa {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub anim_func: AnimFunc,
    pub collider: ColliderCylinder,
    pub blink_func: BlinkFunc,
    pub eye_index: i16,
    pub blink_timer: i16,
    /// `superCuccos`: the three `En_Niw` 0xD his init spawned, each `NULL`ed once found (a handle
    /// kept after its cucco is gone, as the C's pointer).
    pub super_cuccos: [Option<ActorHandle>; 3],
    /// `superCuccoTimers`: the throw's zero-gravity frames, then the frames a cucco is held.
    pub super_cucco_timers: [i16; 3],
    pub last_found_super_cucco_idx: u8,
    pub timer: i16,
    pub rapid_blinks: i16,
    pub sub_cam_id: i16,
    pub return_to_cam_id: i16,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    pub state_flags: u16,
    pub nod_off_timer: i16,
    /// `currentAnimation`: what `EnTa_AnimRepeatCurrent` replays.
    pub current_animation: Anim,
    anims: Anims,
}

struct Anims {
    stand: Anim,
    sleep: Anim,
    wake_up: Anim,
    run: Anim,
    run_transition: Anim,
    sit_sleeping: Anim,
    sit_hands_up: Anim,
    sit_wake_up: Anim,
}

/// Player (`GET_PLAYER`).
fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    play.player.and_then(|h| play.actors.downcast_mut::<Player>(h))
}

/// A super cucco (`this->superCuccos[i]`), if it's still there.
fn niw_mut(play: &mut PlayState, h: Option<ActorHandle>) -> Option<&mut EnNiw> {
    h.and_then(|h| play.actors.downcast_mut::<EnNiw>(h))
}

// shared: Message_CloseTextbox
/// `Message_CloseTextbox` (`z_message.c`), as the other modules write it inline.
fn message_close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId` (`z_face_reaction.c`): `sMaskReactionSetTextIds[set][Player_GetMask]`,
/// here only Talon's row (`MASK_REACTION_SET_TALON`). Player wears no mask until the masks are
/// ported (BACKLOG #39): 0.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.current_mask()) as usize;
    match set {
        MASK_REACTION_SET_TALON => MASK_REACTION_TALON.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `EnTa_CheckCanBuyMilk`.
fn check_can_buy_milk(play: &PlayState) -> i32 {
    if play.save.rupees < 30 {
        TALON_CANBUYMILK_NOT_ENOUGH_RUPEES
    } else if !inventory_has_empty_bottle(&play.save) {
        TALON_CANBUYMILK_NO_EMPTY_BOTTLE
    } else {
        TALON_CANBUYMILK_SUCCESS
    }
}

impl EnTa {
    /// `EnTa_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            Ok((
                assets.skeleton(OBJECT, "gTalonSkel")?,
                Anims {
                    stand: assets.animation(OBJECT, "gTalonStandAnim")?,
                    sleep: assets.animation(OBJECT, "gTalonSleepAnim")?,
                    wake_up: assets.animation(OBJECT, "gTalonWakeUpAnim")?,
                    run: assets.animation(OBJECT, "gTalonRunAnim")?,
                    run_transition: assets.animation(OBJECT, "gTalonRunTransitionAnim")?,
                    sit_sleeping: assets.animation(OBJECT, "gTalonSitSleepingAnim")?,
                    sit_hands_up: assets.animation(OBJECT, "gTalonSitHandsUpAnim")?,
                    sit_wake_up: assets.animation(OBJECT, "gTalonSitWakeUpAnim")?,
                },
            ))
        })();
        let (skeleton, anims) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ta: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnTa {
            // SkelAnime_InitFlex(.., gTalonSkel, gTalonStandAnim, .., TALON_LIMB_MAX).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims.stand.clone())),
            skeleton,
            action: Action::IdleAsleepInCastle,
            anim_func: AnimFunc::Sleeping,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            blink_func: BlinkFunc::WaitUntilNext,
            eye_index: 0,
            blink_timer: 20,
            super_cuccos: [None; 3],
            super_cucco_timers: [0; 3],
            last_found_super_cucco_idx: 0,
            timer: 0,
            rapid_blinks: 0,
            sub_cam_id: 0,
            return_to_cam_id: 0,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            state_flags: 0,
            nod_off_timer: 0,
            current_animation: anims.stand.clone(),
            anims,
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 36.0);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.velocity.y = -4.0;
        this.actor.min_velocity_y = -4.0;
        this.actor.gravity = -1.0;
        let adult = play.save.adult;
        match TYPE.get(this.actor.params) {
            ENTA_IN_KAKARIKO => {
                log::debug!(" Exile Talon ");
                if play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO) || !adult {
                    this.actor.kill();
                } else if play.save.get_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO) {
                    this.setup_awake_standing(Action::IdleAwakeInKakariko);
                } else {
                    this.setup_asleep(Action::IdleAsleepInKakariko);
                }
            }
            ENTA_RETURNED_FROM_KAKARIKO => {
                log::debug!(" Return Talon ");
                if !play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO) || !adult {
                    this.actor.kill();
                } else if play.scene_id == SCENE_STABLE && !play.save.is_day() {
                    this.actor.kill();
                    log::debug!(" He's not here at night ");
                } else {
                    this.setup_awake_standing(Action::IdleAtRanch);
                }
            }
            _ => {
                log::debug!(" Other Talon ");
                if play.scene_id == SCENE_HYRULE_CASTLE {
                    if play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                        this.actor.kill();
                    } else if play.save.get_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_CASTLE) {
                        this.setup_awake_standing(Action::IdleAwakeInCastle);
                    } else {
                        this.setup_asleep(Action::IdleAsleepInCastle);
                    }
                } else if play.scene_id == SCENE_LON_LON_BUILDINGS {
                    log::debug!(" Talon in the warehouse at Lon Lon Ranch");
                    if !play.save.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) || adult {
                        this.actor.kill();
                    } else if play.save.is_day() {
                        this.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                        this.super_cucco_timers = [7; 3];
                        let p = this.actor.world_pos;
                        for (i, off) in [Vec3::new(5.0, 3.0, 26.0), Vec3::new(-20.0, 40.0, -30.0), Vec3::new(20.0, 40.0, -30.0)].into_iter().enumerate() {
                            this.super_cuccos[i] = play.actor_spawn(ACTOR_EN_NIW, p + off, [0, 0, 0], SUPER_CUCCO_PARAMS).map_err(|e| log::debug!("En_Ta: En_Niw: {e:?}")).ok();
                        }
                        this.set_text_for_talk_in_lon_lon_house(play);
                        if play.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED) {
                            this.setup_action(Action::IdleAfterCuccoGameFinished, AnimFunc::RunToEnd);
                            let last = this.anims.sit_wake_up.last_frame();
                            this.skel.change(this.anims.sit_wake_up.clone(), 1.0, last - 1.0, last, ANIMMODE_ONCE, 0.0);
                            play.save.clear_event_inf(EVENTINF_CUCCO_GAME_FINISHED);
                        } else {
                            this.setup_action(Action::IdleSittingInLonLonHouse, AnimFunc::SitSleeping);
                            this.eye_index = TALON_EYE_INDEX_OPEN;
                            this.skel.play_once(this.anims.sit_sleeping.clone());
                            this.current_animation = this.anims.sit_sleeping.clone();
                        }
                    } else {
                        this.setup_asleep(Action::IdleAsleepInLonLonHouse);
                    }
                } else {
                    this.setup_asleep(Action::IdleAsleepInCastle);
                }
            }
        }
        Box::new(this)
    }

    /// The inits' awake Talon: `gTalonStandAnim`, eyes open, `EnTa_AnimRepeatCurrent`.
    fn setup_awake_standing(&mut self, action: Action) {
        self.setup_action(action, AnimFunc::RepeatCurrent);
        self.eye_index = TALON_EYE_INDEX_OPEN;
        self.skel.play_once(self.anims.stand.clone());
        self.current_animation = self.anims.stand.clone();
    }

    /// The inits' sleeping Talon: `gTalonSleepAnim`, eyes closed, a bigger shadow (54).
    fn setup_asleep(&mut self, action: Action) {
        self.setup_action(action, AnimFunc::Sleeping);
        self.eye_index = TALON_EYE_INDEX_CLOSED;
        self.skel.play_once(self.anims.sleep.clone());
        self.current_animation = self.anims.sleep.clone();
        self.actor.shape_shadow_scale = 54.0;
    }

    /// `EnTa_SetupAction`.
    fn setup_action(&mut self, action: Action, anim_func: AnimFunc) {
        self.action = action;
        self.anim_func = anim_func;
    }

    /// `EnTa_DecreaseShadowSize`: 0.8 a frame back to 36.
    fn decrease_shadow_size(&mut self) {
        if self.actor.shape_shadow_scale > 36.0 {
            self.actor.shape_shadow_scale -= 0.8;
        }
    }

    /// `EnTa_RequestTalk`: accepted, or `textId` and an offer within 100 in front.
    fn request_talk(&mut self, play: &mut PlayState, text_id: u16) -> bool {
        if process_talk_request(&mut self.actor) {
            return true;
        }
        self.actor.text_id = text_id;
        if ((self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)) as i32).abs() <= 0x4300 && self.actor.xz_dist_to_player < 100.0 {
            self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
            let a = self.actor.clone();
            offer_talk(play, &a, 100.0);
        }
        false
    }

    /// `EnTa_SleepTalkInKakariko`.
    fn sleep_talk_in_kakariko(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_action(Action::IdleAsleepInKakariko, AnimFunc::Sleeping);
        }
    }

    /// `EnTa_SleepTalkInLonLonHouse`.
    fn sleep_talk_in_lon_lon_house(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_action(Action::IdleAsleepInLonLonHouse, AnimFunc::Sleeping);
        }
    }

    /// `EnTa_SetupAwake`: awake for good (the castle's flag for a child, Kakariko's for an adult).
    fn setup_awake(&mut self, play: &mut PlayState) {
        if !play.save.adult {
            self.setup_action(Action::IdleAwakeInCastle, AnimFunc::RepeatCurrent);
            play.save.set_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_CASTLE);
        } else {
            self.setup_action(Action::IdleAwakeInKakariko, AnimFunc::RepeatCurrent);
            play.save.set_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO);
        }
    }

    /// `EnTa_TalkWakingUp2`.
    fn talk_waking_up2(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_awake(play);
        }
        self.decrease_shadow_size();
        self.state_flags |= TALON_STATE_FLAG_SUPPRESS_BLINK;
    }

    /// `EnTa_TalkWakingUp1`: half-open eyes once the text reaches its end.
    fn talk_waking_up1(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_awake(play);
            self.blink_timer = 1;
            self.blink_func = BlinkFunc::AdvanceState;
        }
        if play.message_state() == TEXT_STATE_DONE {
            self.eye_index = TALON_EYE_INDEX_HALF;
            self.setup_action(Action::TalkWakingUp2, AnimFunc::RepeatCurrent);
        }
        self.decrease_shadow_size();
        self.state_flags |= TALON_STATE_FLAG_SUPPRESS_BLINK;
    }

    /// `EnTa_WakeUp`: when the timer runs out, `gTalonWakeUpAnim` and the surprise.
    fn wake_up(&mut self, play: &mut PlayState) {
        self.state_flags |= TALON_STATE_FLAG_SUPPRESS_BLINK;
        if self.timer == 0 {
            self.setup_action(Action::TalkWakingUp1, AnimFunc::RepeatCurrent);
            self.rapid_blinks = 3;
            self.timer = 60;
            self.skel.play_once(self.anims.wake_up.clone());
            self.current_animation = self.anims.stand.clone();
            audio_play_actor_sfx2(play, NA_SE_VO_TA_SURPRISE);
        }
    }

    /// `EnTa_SleepTalkInCastle`.
    fn sleep_talk_in_castle(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_action(Action::IdleAsleepInCastle, AnimFunc::Sleeping);
        }
    }

    /// Player's text for the shown item (`player->actor.textId`).
    fn set_player_text_id(play: &mut PlayState, id: u16) {
        if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
            p.text_id = id;
        }
    }

    /// `EnTa_IdleAsleepInCastle`: the Cucco shown wakes him (Player's text 0x702B, 40 frames);
    /// anything else shown, 0x702A as Player's text.
    fn idle_asleep_in_castle(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            let exchange_item_id = get_player_exchange_item_id(play);
            match exchange_item_id {
                EXCH_ITEM_CHICKEN => {
                    Self::set_player_text_id(play, 0x702B);
                    self.setup_action(Action::WakeUp, AnimFunc::RepeatCurrent);
                    self.timer = 40;
                }
                _ => {
                    if exchange_item_id != EXCH_ITEM_NONE {
                        Self::set_player_text_id(play, 0x702A);
                    }
                    self.setup_action(Action::SleepTalkInCastle, AnimFunc::Sleeping);
                }
            }
        } else {
            self.actor.text_id = 0x702A;
            let a = self.actor.clone();
            offer_talk_exchange(play, &a, 100.0, EXCH_ITEM_CHICKEN);
        }
    }

    /// `EnTa_IdleAsleepInLonLonHouse`.
    fn idle_asleep_in_lon_lon_house(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.setup_action(Action::SleepTalkInLonLonHouse, AnimFunc::Sleeping);
        }
        self.actor.text_id = 0x204B;
        let a = self.actor.clone();
        offer_talk(play, &a, 100.0);
    }

    /// `EnTa_IdleAsleepInKakariko`: the Pocket Cucco wakes him (0x702B); anything else 0x5015.
    fn idle_asleep_in_kakariko(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            let exchange_item_id = get_player_exchange_item_id(play);
            match exchange_item_id {
                EXCH_ITEM_POCKET_CUCCO => {
                    Self::set_player_text_id(play, 0x702B);
                    self.setup_action(Action::WakeUp, AnimFunc::RepeatCurrent);
                    self.timer = 40;
                }
                _ => {
                    if exchange_item_id != EXCH_ITEM_NONE {
                        Self::set_player_text_id(play, 0x5015);
                    }
                    self.setup_action(Action::SleepTalkInKakariko, AnimFunc::Sleeping);
                }
            }
        } else {
            self.actor.text_id = 0x5015;
            let a = self.actor.clone();
            offer_talk_exchange(play, &a, 100.0, EXCH_ITEM_POCKET_CUCCO);
        }
    }

    /// `EnTa_RunWithAccelerationAndSfx`: a dirt footstep every 6 frames, up to 6 fast by 0.4.
    fn run_with_acceleration_and_sfx(&mut self, play: &mut PlayState) {
        let frames_mod12 = (play.state_frames as i32) % 12;
        if frames_mod12 == 0 || frames_mod12 == 6 {
            audio_play_actor_sfx2(play, NA_SE_PL_WALK_GROUND + SURFACE_SFX_OFFSET_DIRT);
        }
        if self.actor.speed_xz < 6.0 {
            self.actor.speed_xz += 0.4;
        }
        self.actor.move_forward();
    }

    /// The run's dust (`func_80033480(.., 50, 2, 250, 20, 1)`).
    fn dust(&self, play: &mut PlayState) {
        func_80033480(play, self.actor.world_pos, 50.0, 2, 250, 20, 1);
    }

    /// `EnTa_RunAwayRunOutOfGate`.
    fn run_away_run_out_of_gate(&mut self, play: &mut PlayState) {
        self.dust(play);
        self.run_with_acceleration_and_sfx(play);
        if self.timer == 0 {
            self.actor.kill();
        }
    }

    /// `EnTa_RunAwayTurnTowardsGate`.
    fn run_away_turn_towards_gate(&mut self) {
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0xC00);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0xC00);
        if self.timer == 0 {
            self.setup_action(Action::RunAwayRunOutOfGate, AnimFunc::RepeatCurrent);
            self.timer = 60;
        }
    }

    /// `EnTa_RunAwayRunWest`.
    fn run_away_run_west(&mut self, play: &mut PlayState) {
        self.dust(play);
        self.run_with_acceleration_and_sfx(play);
        if self.timer == 0 {
            self.setup_action(Action::RunAwayTurnTowardsGate, AnimFunc::RepeatCurrent);
            self.timer = 5;
        }
    }

    /// `EnTa_RunAwayTurnWest`.
    fn run_away_turn_west(&mut self) {
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0xD00);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0xD00);
        if self.timer == 0 {
            self.setup_action(Action::RunAwayRunWest, AnimFunc::RepeatCurrent);
            self.timer = 65;
        }
    }

    /// `EnTa_RunAwayRunSouth`: the text closed 20 frames before the turn.
    fn run_away_run_south(&mut self, play: &mut PlayState) {
        self.dust(play);
        self.run_with_acceleration_and_sfx(play);
        if self.timer == 20 {
            play.with_msg(|m, f| m.close_textbox(f.audio));
        }
        if self.timer == 0 {
            self.timer = 5;
            self.setup_action(Action::RunAwayTurnWest, AnimFunc::RepeatCurrent);
        }
    }

    /// `EnTa_RunAwayStart`: turning, then `NA_SE_VO_TA_CRY_1` and the run, updated wherever he is.
    fn run_away_start(&mut self, play: &mut PlayState) {
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(0xC00);
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0xC00);
        if self.timer == 0 {
            audio_play_actor_sfx2(play, NA_SE_VO_TA_CRY_1);
            self.setup_action(Action::RunAwayRunSouth, AnimFunc::RepeatCurrent);
            self.timer = 65;
            self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        }
    }

    /// `EnTa_TalkAwakeInCastle`: on the text's event the run home, its camera (4175) and flag.
    fn talk_awake_in_castle(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT {
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(4175, -99, me, CAM_ID_MAIN);
            self.setup_action(Action::RunAwayStart, AnimFunc::RepeatCurrent);
            self.timer = 5;
            play.save.set_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
            self.skel.play_once(self.anims.run_transition.clone());
            self.current_animation = self.anims.run.clone();
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IdleAwakeInCastle`.
    fn idle_awake_in_castle(&mut self, play: &mut PlayState) {
        if self.request_talk(play, 0x702C) {
            self.setup_action(Action::TalkAwakeInCastle, AnimFunc::RepeatCurrent);
        }
        self.decrease_shadow_size();
    }

    /// `EnTa_TalkAwakeInKakariko`.
    fn talk_awake_in_kakariko(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_action(Action::IdleAwakeInKakariko, AnimFunc::RepeatCurrent);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IdleAwakeInKakariko`: with Epona, 0x5017 and back to the ranch; else 0x5016.
    fn idle_awake_in_kakariko(&mut self, play: &mut PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
            if self.request_talk(play, 0x5017) {
                self.setup_action(Action::TalkAwakeInKakariko, AnimFunc::RepeatCurrent);
                play.save.set_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO);
            }
        } else if self.request_talk(play, 0x5016) {
            self.setup_action(Action::TalkAwakeInKakariko, AnimFunc::RepeatCurrent);
        }
        self.decrease_shadow_size();
    }

    /// `EnTa_TalkAtRanch`.
    fn talk_at_ranch(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !oot_game::actor::ACTOR_FLAG_TALK;
            self.setup_action(Action::IdleAtRanch, AnimFunc::RepeatCurrent);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IdleAtRanch`.
    fn idle_at_ranch(&mut self, play: &mut PlayState) {
        if self.request_talk(play, 0x2055) {
            self.setup_action(Action::TalkAtRanch, AnimFunc::RepeatCurrent);
        }
    }

    /// `EnTa_SetTextForTalkInLonLonHouse`: just after the game 0x2088 (won again), 0x2086 (won the
    /// first time) or 0x2085 (lost), the win forgotten; else the mask's text, or 0x207E the first
    /// time, then 0x207F (0x208B once he's given the bottle).
    fn set_text_for_talk_in_lon_lon_house(&mut self, play: &mut PlayState) {
        let mask_reaction_text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_TALON);
        if play.save.get_event_inf(EVENTINF_CUCCO_GAME_FINISHED) {
            self.actor.text_id = if play.save.get_event_inf(EVENTINF_CUCCO_GAME_WON) {
                if play.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE) { 0x2088 } else { 0x2086 }
            } else {
                0x2085
            };
            play.save.clear_event_inf(EVENTINF_CUCCO_GAME_WON);
        } else if mask_reaction_text_id == 0 {
            self.actor.text_id = if play.save.get_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE) {
                if play.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE) { 0x208B } else { 0x207F }
            } else {
                0x207E
            };
        } else {
            self.actor.text_id = mask_reaction_text_id;
        }
    }

    /// `EnTa_CreateFloorCamera`: a sub camera from (1053, 11, 22) at (1053, 45, -40), the active one
    /// waiting.
    fn create_floor_camera(&mut self, play: &mut PlayState) {
        self.sub_cam_id = play.create_sub_camera();
        self.return_to_cam_id = play.active_cam_id;
        play.change_camera_status(self.return_to_cam_id, CAM_STAT_WAIT);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        play.camera_set_at_eye(self.sub_cam_id, FLOOR_CAMERA_AT, FLOOR_CAMERA_EYE);
    }

    /// `EnTa_RemoveFloorCamera`.
    fn remove_floor_camera(&mut self, play: &mut PlayState) {
        play.change_camera_status(self.return_to_cam_id, CAM_STAT_ACTIVE);
        play.clear_camera(self.sub_cam_id);
    }

    /// `EnTa_SetupActionWithSleepAnimation`: back to nodding off (`gTalonSitSleepingAnim`).
    fn setup_action_with_sleep_animation(&mut self, action: Action) {
        self.setup_action(action, AnimFunc::SitSleeping);
        self.eye_index = TALON_EYE_INDEX_CLOSED;
        let last = self.anims.sit_sleeping.last_frame();
        self.skel.change(self.anims.sit_sleeping.clone(), 1.0, 0.0, last, ANIMMODE_ONCE, -5.0);
        self.nod_off_timer = 0;
        self.current_animation = self.anims.sit_sleeping.clone();
    }

    /// `EnTa_SetupActionWithWakeUpAnimation`: `gTalonSitWakeUpAnim` once, the eyes half open.
    fn setup_action_with_wake_up_animation(&mut self, action: Action) {
        self.eye_index = TALON_EYE_INDEX_HALF;
        self.setup_action(action, AnimFunc::RunToEnd);
        self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
        let last = self.anims.sit_wake_up.last_frame();
        self.skel.change(self.anims.sit_wake_up.clone(), 1.0, 0.0, last, ANIMMODE_ONCE, -5.0);
    }

    /// `gTalonSitWakeUpAnim`'s last frame held (`Animation_Change(.., lastFrame - 1, lastFrame, ..,
    /// morph)`).
    fn change_to_awake(&mut self, morph: f32) {
        let last = self.anims.sit_wake_up.last_frame();
        self.skel.change(self.anims.sit_wake_up.clone(), 1.0, last - 1.0, last, ANIMMODE_ONCE, morph);
    }

    /// `Animation_Change(gTalonSitHandsUpAnim, 1, 8, 29, ANIMMODE_ONCE, -10)`: hands up.
    fn change_to_hands_up(&mut self) {
        self.skel.change(self.anims.sit_hands_up.clone(), 1.0, 8.0, 29.0, ANIMMODE_ONCE, -10.0);
    }

    /// `EnTa_TalkNotEnoughRupees`: 0x85's event, back to nodding off.
    fn talk_not_enough_rupees(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            message_close_textbox(play);
            self.setup_action_with_sleep_animation(Action::IdleSittingInLonLonHouse);
            self.set_text_for_talk_in_lon_lon_house(play);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IsPlayerHoldingSuperCucco`: Player carries (`PLAYER_STATE1_CARRYING_ACTOR`) super
    /// cucco `cucco_idx` (his `interactRangeActor`, an `En_Niw`).
    fn is_player_holding_super_cucco(&self, play: &PlayState, cucco_idx: usize) -> bool {
        let Some(p) = player(play) else { return false };
        if p.state1 & STATE1_11 != 0 {
            if let Some(interact_range_actor) = p.interact_range_actor {
                if play.actors.actor(interact_range_actor).is_some_and(|a| a.id == ACTOR_EN_NIW) && Some(interact_range_actor) == self.super_cuccos[cucco_idx] {
                    return true;
                }
            }
        }
        false
    }

    /// `EnTa_TalkFoundSuperCucco`: on the text's event the game goes on; the found cucco flies
    /// straight up out of Link's hands (gravity 0.1, its parent gone, Player's hold undone) and is
    /// forgotten.
    fn talk_found_super_cucco(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.change_to_awake(10.0);
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            message_close_textbox(play);
            let last_found_super_cucco_idx = self.last_found_super_cucco_idx as usize;
            self.action = Action::RunCuccoGame;
            let h = self.super_cuccos[last_found_super_cucco_idx];
            if let Some(niw) = niw_mut(play, h) {
                niw.actor.gravity = 0.1;
                niw.actor.velocity.y = 0.0;
                niw.actor.speed_xz = 0.0;
                niw.actor.parent = None;
            }
            if let Some(p) = player_mut(play) {
                if h.is_some() && p.interact_range_actor == h {
                    p.interact_range_actor = None;
                }
                if h.is_some() && p.held_actor == h {
                    p.held_actor = None;
                }
                p.state1 &= !STATE1_11;
            }
            self.super_cuccos[last_found_super_cucco_idx] = None;
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IdleFoundSuperCucco`: talking at once (the auto-accepted offer, within 1000).
    fn idle_found_super_cucco(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::TalkFoundSuperCucco;
            // Unset auto-talking.
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        } else {
            offer_talk(play, &self.actor, 1000.0);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_GetSuperCuccosCount`: the cuccos not yet found.
    fn get_super_cuccos_count(&self) -> usize {
        self.super_cuccos.iter().filter(|c| c.is_some()).count()
    }

    /// `EnTa_AnimateHandsUpDown`: `gTalonSitHandsUpAnim`'s frames 17 to 22 and back (21 to 16),
    /// over and over.
    fn animate_hands_up_down(&mut self) {
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            if self.state_flags & TALON_STATE_FLAG_RAISING_HANDS != 0 {
                self.skel.change(self.anims.sit_hands_up.clone(), 1.0, 17.0, 22.0, ANIMMODE_ONCE, 0.0);
                self.state_flags &= !TALON_STATE_FLAG_RAISING_HANDS;
            } else {
                self.skel.change(self.anims.sit_hands_up.clone(), -1.0, 21.0, 16.0, ANIMMODE_ONCE, 3.0);
                self.state_flags |= TALON_STATE_FLAG_RAISING_HANDS;
            }
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
        }
    }

    /// `EnTa_TransitionToPostCuccoGame`: waving, the floor camera gone when the timer runs out.
    fn transition_to_post_cucco_game(&mut self, play: &mut PlayState) {
        self.animate_hands_up_down();
        if self.timer == 0 && self.state_flags & TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE != 0 {
            self.state_flags &= !TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE;
            self.remove_floor_camera(play);
        }
    }

    /// `EnTa_TalkCuccoGameEnd`: on the text's event, out to `ENTR_LON_LON_BUILDINGS_2` through a
    /// starburst circle (white won, black lost), `EVENTINF_CUCCO_GAME_FINISHED`.
    fn talk_cucco_game_end(&mut self, play: &mut PlayState) {
        self.animate_hands_up_down();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.transition.next_entrance_index = ENTR_LON_LON_BUILDINGS_2;
            if play.save.get_event_inf(EVENTINF_CUCCO_GAME_WON) {
                play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_WHITE, TCS_FAST);
                play.save.next_transition_type = TRANS_TYPE_FADE_WHITE;
            } else {
                play.transition.ty = trans_type_circle(TCA_STARBURST, TCC_BLACK, TCS_FAST);
                play.save.next_transition_type = TRANS_TYPE_FADE_BLACK;
            }
            play.transition.trigger = TRANS_TRIGGER_START;
            play.save.set_event_inf(EVENTINF_CUCCO_GAME_FINISHED);
            self.action = Action::TransitionToPostCuccoGame;
            self.timer = 22;
        }
    }

    /// `EnTa_RunCuccoGame`: the cuccos' gravity back; a cucco held 7 frames found (the last wins);
    /// at 10 seconds left the fast tempo, at 0 time's up.
    fn run_cucco_game(&mut self, play: &mut PlayState) {
        for i in 0..self.super_cuccos.len() {
            if self.super_cuccos[i].is_none() {
                continue;
            }
            if let Some(niw) = niw_mut(play, self.super_cuccos[i]) {
                if niw.actor.gravity > -2.0 {
                    niw.actor.gravity -= 0.03;
                }
            }
            if self.is_player_holding_super_cucco(play, i) {
                if self.super_cucco_timers[i] > 0 {
                    // Wait until the cucco's timer runs out after acknowledging that the player
                    // picked it up.
                    self.super_cucco_timers[i] -= 1;
                } else {
                    self.last_found_super_cucco_idx = i as u8;
                    self.change_to_hands_up();
                    self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
                    match self.get_super_cuccos_count() {
                        1 => {
                            // Last cucco found, end the game.
                            play.save.timer_state = TIMER_STATE_OFF;
                            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
                            play.start_textbox(0x2084, play.cur_actor);
                            self.action = Action::TalkCuccoGameEnd;
                            self.change_to_hands_up();
                            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
                            self.state_flags &= !TALON_STATE_FLAG_RAISING_HANDS;
                            play.save.set_event_inf(EVENTINF_CUCCO_GAME_WON);
                            // SEQCMD_STOP_SEQUENCE(SEQ_PLAYER_BGM_MAIN, 0).
                            play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0));
                            self.state_flags &= !TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY;
                            play.audio.play_fanfare(NA_BGM_SMALL_ITEM_GET);
                            return;
                        }
                        2 => {
                            // One cucco remaining.
                            self.actor.text_id = 0x2083;
                            audio_play_actor_sfx2(play, NA_SE_VO_TA_CRY_1);
                        }
                        3 => {
                            // Two cuccos remaining.
                            self.actor.text_id = 0x2082;
                            audio_play_actor_sfx2(play, NA_SE_VO_TA_SURPRISE);
                        }
                        _ => {}
                    }
                    self.action = Action::IdleFoundSuperCucco;
                    // Automatically talk to player.
                    self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
                    offer_talk(play, &self.actor, 1000.0);
                    return;
                }
            } else {
                self.super_cucco_timers[i] = 7;
            }
        }

        if play.save.timer_seconds == 10 {
            play.audio.audio_set_fast_tempo_for_timed_minigame();
        }

        if play.save.timer_seconds == 0 && !play.play_in_cs_mode() {
            play.audio.queue_seq_cmd(seq_cmd1(SEQ_PLAYER_BGM_MAIN, 0));
            self.state_flags &= !TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY;
            play.audio.play_sfx_centered(NA_SE_SY_FOUND);
            play.save.timer_state = TIMER_STATE_OFF;
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
            // Time's up text.
            play.start_textbox(0x2081, play.cur_actor);
            self.action = Action::TalkCuccoGameEnd;
            self.create_floor_camera(play);
            play.save.clear_event_inf(EVENTINF_CUCCO_GAME_WON);
            self.state_flags |= TALON_STATE_FLAG_FLOOR_CAMERA_ACTIVE;
            self.change_to_hands_up();
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            self.state_flags &= !TALON_STATE_FLAG_RAISING_HANDS;
        }

        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_ThrowSuperCuccos`: 50 frames. Down to 36 the cuccos on the table (1 and 2) turn; at 35
    /// all three fly off (`unk_308`, gravity 0) with 7 to 12 frames of no gravity; then their
    /// gravity comes back 0.03 a frame to -2. At 0 the game: Link let go (`PLAYER_CSACTION_7`).
    fn throw_super_cuccos(&mut self, play: &mut PlayState) {
        if self.timer > 35 {
            // During the first part of the throw animation, just turn them (on the table or the
            // floor).
            for i in 1..self.super_cuccos.len() {
                if let Some(niw) = niw_mut(play, self.super_cuccos[i]) {
                    smooth_step_to_s(&mut niw.actor.world_rot.y, (i as i32 * -10000 - 3000) as i16, 2, 0x800, 0x100);
                    niw.actor.shape_rot.y = niw.actor.world_rot.y;
                }
            }
        } else if self.timer == 35 {
            // At this point, prepare each super cucco for flight.
            for i in 0..self.super_cuccos.len() {
                // Set a 7-12 frame zero-gravity time for each super cucco.
                self.super_cucco_timers[i] = (play.rand.centered_float(6.0) + 10.0) as i32 as i16;
                if let Some(niw) = niw_mut(play, self.super_cuccos[i]) {
                    // Mark the cucco as a super cucco: it sets random xz and y velocities.
                    niw.unk_308 = 1;
                    niw.actor.gravity = 0.0;
                }
            }
        } else {
            for i in 0..self.super_cuccos.len() {
                // If the zero-gravity time of the cucco is over, add gravity to it.
                if self.timer < 35 - self.super_cucco_timers[i] {
                    if let Some(niw) = niw_mut(play, self.super_cuccos[i]) {
                        if niw.actor.gravity > -2.0 {
                            niw.actor.gravity -= 0.03;
                        }
                    }
                }
            }
        }

        if self.timer == 0 {
            self.setup_action(Action::RunCuccoGame, AnimFunc::RunToEnd);
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            self.change_to_awake(10.0);
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_7);
        }
    }

    /// `EnTa_StartingCuccoGame3`: once 0x2080's event came and 5 frames are up, the throw (the
    /// hands-up animation again, 50 frames), the timer at 30, the timed mini-game's music, the text
    /// closed, Link held.
    fn starting_cucco_game3(&mut self, play: &mut PlayState) {
        if self.timer == 0 && self.state_flags & TALON_STATE_FLAG_CUCCO_GAME_START_EVENT_TRIGGERED != 0 {
            self.setup_action(Action::ThrowSuperCuccos, AnimFunc::RunToEnd);
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            // Play hand raise animation again so that it looks like Talon throws the cuccos.
            let last = self.anims.sit_hands_up.last_frame();
            self.skel.change(self.anims.sit_hands_up.clone(), 1.0, 1.0, last, ANIMMODE_ONCE, 0.0);
            self.timer = 50;
            play.interface_set_timer(30);
            play.audio.func_800f5acc(NA_BGM_TIMED_MINI_GAME);
            self.state_flags |= TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY;
            message_close_textbox(play);
            play.player_set_cs_action_with_halted_actors(play.cur_actor, PLAYER_CSACTION_1);
        }
        self.check_start_event(play);
    }

    /// The `EnTa_StartingCuccoGame*`s' 0x2080 event: `TALON_STATE_FLAG_CUCCO_GAME_START_EVENT_TRIGGERED`,
    /// and his head to Link.
    fn check_start_event(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.state_flags |= TALON_STATE_FLAG_CUCCO_GAME_START_EVENT_TRIGGERED;
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_StartingCuccoGame2`: the hands down done, frame 0 to 1 held 5 frames.
    fn starting_cucco_game2(&mut self, play: &mut PlayState) {
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.setup_action(Action::StartingCuccoGame3, AnimFunc::RunToEnd);
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            self.skel.change(self.anims.sit_hands_up.clone(), 1.0, 0.0, 1.0, ANIMMODE_ONCE, 0.0);
            self.timer = 5;
        }
        self.check_start_event(play);
    }

    /// `EnTa_StartingCuccoGame1`: the hands up done, down again (29 to 0 backwards).
    fn starting_cucco_game1(&mut self, play: &mut PlayState) {
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.setup_action(Action::StartingCuccoGame2, AnimFunc::RunToEnd);
            self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
            self.skel.change(self.anims.sit_hands_up.clone(), -1.0, 29.0, 0.0, ANIMMODE_ONCE, 10.0);
        }
        self.check_start_event(play);
    }

    /// `EnTa_StartCuccoGame`: hands up and 0x2080.
    fn start_cucco_game(&mut self, play: &mut PlayState) {
        self.setup_action(Action::StartingCuccoGame1, AnimFunc::RunToEnd);
        self.state_flags &= !TALON_STATE_FLAG_ANIMATION_FINISHED;
        self.change_to_hands_up();
        play.continue_textbox(0x2080);
        self.state_flags &= !TALON_STATE_FLAG_CUCCO_GAME_START_EVENT_TRIGGERED;
    }

    /// `EnTa_TalkGeneralInLonLonHouse`: the text closing, back to nodding off.
    fn talk_general_in_lon_lon_house(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !ACTOR_FLAG_TALK;
            self.setup_action_with_sleep_animation(Action::IdleSittingInLonLonHouse);
            self.set_text_for_talk_in_lon_lon_house(play);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_GiveItemInLonLonHouse`: the milk (`GI_MILK`) or the bottle of it (`GI_BOTTLE_MILK_FULL`,
    /// `ITEMGETINF_TALON_BOTTLE`) offered until Link has it.
    fn give_item_in_lon_lon_house(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::TalkGeneralInLonLonHouse;
            if self.state_flags & TALON_STATE_FLAG_GIVING_MILK_REFILL == 0 {
                play.save.set_item_get_inf(ITEMGETINF_TALON_BOTTLE);
            }
            self.state_flags &= !TALON_STATE_FLAG_GIVING_MILK_REFILL;
        } else if self.state_flags & TALON_STATE_FLAG_GIVING_MILK_REFILL != 0 {
            offer_get_item_range(play, &self.actor, GI_MILK, 10000.0, 50.0);
        } else {
            offer_get_item_range(play, &self.actor, GI_BOTTLE_MILK_FULL, 10000.0, 50.0);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_TalkAfterCuccoGameFirstWon`: 0x2087's event, the bottle of milk.
    fn talk_after_cucco_game_first_won(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            message_close_textbox(play);
            self.state_flags &= !TALON_STATE_FLAG_GIVING_MILK_REFILL;
            self.setup_action(Action::GiveItemInLonLonHouse, AnimFunc::RunToEnd);
            offer_get_item_range(play, &self.actor, GI_BOTTLE_MILK_FULL, 10000.0, 50.0);
        }
    }

    /// `EnTa_WaitBuyMilkOrPlayCuccoGameResponse`: 0x208B's choice: milk (30 rupees and an empty
    /// bottle; 0x85 short, 0x208A without the bottle), the game (10 rupees, 0x85 short), or not.
    fn wait_buy_milk_or_play_cucco_game_response(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                // Buy milk.
                0 => match check_can_buy_milk(play) {
                    TALON_CANBUYMILK_NOT_ENOUGH_RUPEES => {
                        play.continue_textbox(0x85);
                        self.setup_action(Action::TalkNotEnoughRupees, AnimFunc::RunToEnd);
                    }
                    TALON_CANBUYMILK_NO_EMPTY_BOTTLE => {
                        play.continue_textbox(0x208A);
                        self.setup_action(Action::TalkGeneralInLonLonHouse, AnimFunc::RunToEnd);
                    }
                    _ => {
                        // TALON_CANBUYMILK_SUCCESS.
                        self.state_flags |= TALON_STATE_FLAG_GIVING_MILK_REFILL;
                        self.setup_action(Action::GiveItemInLonLonHouse, AnimFunc::RunToEnd);
                        rupees_change_by(&mut play.save, -30);
                        offer_get_item_range(play, &self.actor, GI_MILK, 10000.0, 50.0);
                    }
                },
                // Play cucco game.
                1 => {
                    if play.save.rupees < 10 {
                        play.continue_textbox(0x85);
                        self.setup_action(Action::TalkNotEnoughRupees, AnimFunc::RunToEnd);
                    } else {
                        rupees_change_by(&mut play.save, -10);
                        self.start_cucco_game(play);
                    }
                }
                // Cancel.
                2 => {
                    self.setup_action_with_sleep_animation(Action::IdleSittingInLonLonHouse);
                    self.set_text_for_talk_in_lon_lon_house(play);
                }
                _ => {}
            }
        }
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
        }
    }

    /// `EnTa_WaitForPlayCuccoGameResponse`: play (10 rupees, 5 for 0x2085's retry; 0x85 short), or
    /// not.
    fn wait_for_play_cucco_game_response(&mut self, play: &mut PlayState) {
        // Check if this is a retry.
        let price: i16 = if self.actor.text_id == 0x2085 { 5 } else { 10 };
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    if play.save.rupees < price {
                        play.continue_textbox(0x85);
                        self.setup_action(Action::TalkNotEnoughRupees, AnimFunc::RunToEnd);
                    } else {
                        rupees_change_by(&mut play.save, -price);
                        self.start_cucco_game(play);
                    }
                }
                1 => {
                    self.setup_action_with_sleep_animation(Action::IdleSittingInLonLonHouse);
                    self.set_text_for_talk_in_lon_lon_house(play);
                }
                _ => {}
            }
        }
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
        }
    }

    /// `EnTa_WaitForMarryMalonResponse`: either answer, 0x2087.
    fn wait_for_marry_malon_response(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            play.continue_textbox(0x2087);
            self.setup_action(Action::TalkAfterCuccoGameFirstWon, AnimFunc::RunToEnd);
        }
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
        }
    }

    /// `EnTa_ContinueTalkInLonLonHouse`: another text's event (a mask's: never here), then 0x208B or
    /// 0x207F.
    fn continue_talk_in_lon_lon_house(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.save.set_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE);
            if play.save.get_item_get_inf(ITEMGETINF_TALON_BOTTLE) {
                // Play cucco game or buy milk.
                play.continue_textbox(0x208B);
                self.setup_action(Action::WaitBuyMilkOrPlayCuccoGameResponse, AnimFunc::RunToEnd);
            } else {
                // Play cucco game?
                play.continue_textbox(0x207F);
                self.setup_action(Action::WaitForPlayCuccoGameResponse, AnimFunc::RunToEnd);
            }
        }
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED != 0 {
            self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
        }
    }

    /// `EnTa_TalkAfterCuccoGameWon`: 0x2088's event: milk into an empty bottle, or 0x208A.
    fn talk_after_cucco_game_won(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            if inventory_has_empty_bottle(&play.save) {
                message_close_textbox(play);
                self.state_flags |= TALON_STATE_FLAG_GIVING_MILK_REFILL;
                self.setup_action(Action::GiveItemInLonLonHouse, AnimFunc::RunToEnd);
                offer_get_item_range(play, &self.actor, GI_MILK, 10000.0, 50.0);
            } else {
                play.continue_textbox(0x208A);
                self.setup_action(Action::TalkGeneralInLonLonHouse, AnimFunc::RunToEnd);
            }
        }
    }

    /// `EnTa_IdleSittingInLonLonHouse`: nodding off; talked to, he wakes (`NA_SE_VO_TA_SURPRISE`)
    /// to his text's question.
    fn idle_sitting_in_lon_lon_house(&mut self, play: &mut PlayState) {
        let mask_reaction_text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_TALON);
        self.set_text_for_talk_in_lon_lon_house(play);
        let text_id = self.actor.text_id;
        if self.request_talk(play, text_id) {
            audio_play_actor_sfx2(play, NA_SE_VO_TA_SURPRISE);
            if mask_reaction_text_id != 0 {
                self.setup_action_with_wake_up_animation(Action::TalkGeneralInLonLonHouse);
            } else {
                play.save.set_inf_table(INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE);
                match self.actor.text_id {
                    // First time talking in Lon Lon House; play cucco game.
                    0x207E | 0x207F => self.setup_action_with_wake_up_animation(Action::WaitForPlayCuccoGameResponse),
                    // Play cucco game or buy milk.
                    0x208B => self.setup_action_with_wake_up_animation(Action::WaitBuyMilkOrPlayCuccoGameResponse),
                    _ => self.setup_action_with_wake_up_animation(Action::ContinueTalkInLonLonHouse),
                }
            }
        }
        self.state_flags &= !TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_IdleAfterCuccoGameFinished`: talking at once (within 1000), to 0x2085's retry, 0x2086's
    /// question or 0x2088's milk.
    fn idle_after_cucco_game_finished(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            match self.actor.text_id {
                // Retry?
                0x2085 => self.action = Action::WaitForPlayCuccoGameResponse,
                // Initial win.
                0x2086 => self.action = Action::WaitForMarryMalonResponse,
                // Later win.
                0x2088 => self.action = Action::TalkAfterCuccoGameWon,
                _ => {}
            }
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
        } else {
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            offer_talk(play, &self.actor, 1000.0);
        }
        self.state_flags |= TALON_STATE_FLAG_TRACKING_PLAYER;
    }

    /// `EnTa_BlinkWaitUntilNext`.
    fn blink_wait_until_next(&mut self) {
        let blink_timer = self.blink_timer - 1;
        if blink_timer != 0 {
            self.blink_timer = blink_timer;
        } else {
            self.blink_func = BlinkFunc::AdvanceState;
        }
    }

    /// `EnTa_BlinkAdvanceState`: open, half, closed, then open again with the next blink 20 to 80
    /// frames on (after waking, three quick ones first).
    fn blink_advance_state(&mut self, play: &mut PlayState) {
        let mut blink_timer = self.blink_timer - 1;
        if blink_timer != 0 {
            self.blink_timer = blink_timer;
        } else {
            let next_eye_index = self.eye_index + 1;
            if next_eye_index >= TALON_EYE_INDEX_MAX {
                self.eye_index = TALON_EYE_INDEX_OPEN;
                if self.rapid_blinks > 0 {
                    self.rapid_blinks -= 1;
                    blink_timer = 1;
                } else {
                    blink_timer = ((play.rand.zero_one() * 60.0) as i32 + 20) as i16;
                }
                self.blink_timer = blink_timer;
                self.blink_func = BlinkFunc::WaitUntilNext;
            } else {
                self.eye_index = next_eye_index;
                self.blink_timer = 1;
            }
        }
    }

    /// `EnTa_AnimRepeatCurrent`.
    fn anim_repeat_current(&mut self) {
        if self.skel.update() {
            self.skel.play_once(self.current_animation.clone());
        }
    }

    /// `EnTa_AnimSleeping`: a snore each loop.
    fn anim_sleeping(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.skel.play_once(self.current_animation.clone());
            audio_play_actor_sfx2(play, NA_SE_VO_TA_SLEEP);
        }
        self.state_flags |= TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM | TALON_STATE_FLAG_SUPPRESS_BLINK;
    }

    /// `EnTa_AnimSitSleeping`: nodding off, paused 100 to 200 frames between nods, the eyes half
    /// open while he raises his head (frames 53 to 96).
    fn anim_sit_sleeping(&mut self, play: &mut PlayState) {
        if self.nod_off_timer > 0 {
            self.nod_off_timer -= 1;
        } else {
            if self.skel.update() {
                self.skel.play_once(self.current_animation.clone());
                self.nod_off_timer = (play.rand.zero_float(100.0) + 100.0) as i16;
            }
            self.eye_index = if self.skel.cur_frame < 96.0 && self.skel.cur_frame >= 53.0 { TALON_EYE_INDEX_HALF } else { TALON_EYE_INDEX_CLOSED };
            self.state_flags |= TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM;
        }
        self.state_flags |= TALON_STATE_FLAG_SUPPRESS_BLINK;
    }

    /// `EnTa_AnimRunToEnd`.
    fn anim_run_to_end(&mut self) {
        if self.state_flags & TALON_STATE_FLAG_ANIMATION_FINISHED == 0 {
            if self.skel.update() {
                self.state_flags |= TALON_STATE_FLAG_ANIMATION_FINISHED;
            }
            self.state_flags |= TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM;
        }
    }

    /// The limb matrices with `EnTa_OverrideLimbDraw`'s turns: the chest's x by the torso's yaw and
    /// its y less the torso's pitch, the head's x by the head's yaw and z by its pitch; the chest's
    /// and arms' sway by `frames`.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3], frames: u32) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            match limb {
                TALON_LIMB_CHEST => {
                    rot[0] = rot[0].wrapping_add(torso[1]);
                    rot[1] = rot[1].wrapping_sub(torso[0]);
                }
                TALON_LIMB_HEAD => {
                    rot[0] = rot[0].wrapping_add(head[1]);
                    rot[2] = rot[2].wrapping_add(head[0]);
                }
                _ => {}
            }
            // TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM is cleared at the root limb (1), before any
            // of these: they always sway.
            if limb == TALON_LIMB_CHEST || limb == TALON_LIMB_LEFT_ARM || limb == TALON_LIMB_RIGHT_ARM {
                let fidget_frequency = limb as i32 * FIDGET_FREQ_LIMB;
                let ay = frames.wrapping_mul((fidget_frequency + FIDGET_FREQ_Y) as u32) as i16;
                let az = frames.wrapping_mul((fidget_frequency + FIDGET_FREQ_Z) as u32) as i16;
                rot[1] = rot[1].wrapping_add((sin_s(ay) * FIDGET_AMPLITUDE) as i16);
                rot[2] = rot[2].wrapping_add((cos_s(az) * FIDGET_AMPLITUDE) as i16);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnTa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnTa_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        match self.anim_func {
            AnimFunc::RepeatCurrent => self.anim_repeat_current(),
            AnimFunc::Sleeping => self.anim_sleeping(play),
            AnimFunc::SitSleeping => self.anim_sit_sleeping(play),
            AnimFunc::RunToEnd => self.anim_run_to_end(),
        }
        match self.action {
            Action::SleepTalkInKakariko => self.sleep_talk_in_kakariko(play),
            Action::SleepTalkInLonLonHouse => self.sleep_talk_in_lon_lon_house(play),
            Action::TalkWakingUp2 => self.talk_waking_up2(play),
            Action::TalkWakingUp1 => self.talk_waking_up1(play),
            Action::WakeUp => self.wake_up(play),
            Action::SleepTalkInCastle => self.sleep_talk_in_castle(play),
            Action::IdleAsleepInCastle => self.idle_asleep_in_castle(play),
            Action::IdleAsleepInLonLonHouse => self.idle_asleep_in_lon_lon_house(play),
            Action::IdleAsleepInKakariko => self.idle_asleep_in_kakariko(play),
            Action::RunAwayRunOutOfGate => self.run_away_run_out_of_gate(play),
            Action::RunAwayTurnTowardsGate => self.run_away_turn_towards_gate(),
            Action::RunAwayRunWest => self.run_away_run_west(play),
            Action::RunAwayTurnWest => self.run_away_turn_west(),
            Action::RunAwayRunSouth => self.run_away_run_south(play),
            Action::RunAwayStart => self.run_away_start(play),
            Action::TalkAwakeInCastle => self.talk_awake_in_castle(play),
            Action::IdleAwakeInCastle => self.idle_awake_in_castle(play),
            Action::TalkAwakeInKakariko => self.talk_awake_in_kakariko(play),
            Action::IdleAwakeInKakariko => self.idle_awake_in_kakariko(play),
            Action::TalkAtRanch => self.talk_at_ranch(play),
            Action::IdleAtRanch => self.idle_at_ranch(play),
            Action::TalkNotEnoughRupees => self.talk_not_enough_rupees(play),
            Action::TalkFoundSuperCucco => self.talk_found_super_cucco(play),
            Action::IdleFoundSuperCucco => self.idle_found_super_cucco(play),
            Action::TransitionToPostCuccoGame => self.transition_to_post_cucco_game(play),
            Action::TalkCuccoGameEnd => self.talk_cucco_game_end(play),
            Action::RunCuccoGame => self.run_cucco_game(play),
            Action::ThrowSuperCuccos => self.throw_super_cuccos(play),
            Action::StartingCuccoGame3 => self.starting_cucco_game3(play),
            Action::StartingCuccoGame2 => self.starting_cucco_game2(play),
            Action::StartingCuccoGame1 => self.starting_cucco_game1(play),
            Action::TalkGeneralInLonLonHouse => self.talk_general_in_lon_lon_house(play),
            Action::GiveItemInLonLonHouse => self.give_item_in_lon_lon_house(play),
            Action::TalkAfterCuccoGameFirstWon => self.talk_after_cucco_game_first_won(play),
            Action::WaitBuyMilkOrPlayCuccoGameResponse => self.wait_buy_milk_or_play_cucco_game_response(play),
            Action::WaitForPlayCuccoGameResponse => self.wait_for_play_cucco_game_response(play),
            Action::WaitForMarryMalonResponse => self.wait_for_marry_malon_response(play),
            Action::ContinueTalkInLonLonHouse => self.continue_talk_in_lon_lon_house(play),
            Action::TalkAfterCuccoGameWon => self.talk_after_cucco_game_won(play),
            Action::IdleSittingInLonLonHouse => self.idle_sitting_in_lon_lon_house(play),
            Action::IdleAfterCuccoGameFinished => self.idle_after_cucco_game_finished(play),
        }
        if self.state_flags & TALON_STATE_FLAG_SUPPRESS_BLINK == 0 {
            match self.blink_func {
                BlinkFunc::WaitUntilNext => self.blink_wait_until_next(),
                BlinkFunc::AdvanceState => self.blink_advance_state(play),
            }
        }
        if self.state_flags & TALON_STATE_FLAG_TRACKING_PLAYER != 0 {
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.torso_rot, focus);
        } else {
            smooth_step_to_s(&mut self.head_rot[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.head_rot[1], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.torso_rot[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.torso_rot[1], 0, 6, 6200, 100);
        }
        self.state_flags &= !(TALON_STATE_FLAG_TRACKING_PLAYER | TALON_STATE_FLAG_SUPPRESS_BLINK);
        if self.timer > 0 {
            self.timer -= 1;
        }
    }
    /// `EnTa_Draw`'s state: `EnTa_OverrideLimbDraw` clearing the sway's flag at the root limb, and
    /// `EnTa_PostLimbDraw`'s focus (1100, 1000, 0) along the head.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.state_flags &= !TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.head_rot, self.torso_rot, play.state_frames);
        if let Some(head) = bones.get(TALON_LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(1100.0, 1000.0, 0.0));
        }
    }
    /// `EnTa_Destroy`: the timer off (the ranch house's Talon), the music back
    /// (`TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY`, set only while the cucco game runs).
    fn destroy(&mut self, play: &mut PlayState) {
        if TYPE.get(self.actor.params) != ENTA_IN_KAKARIKO && TYPE.get(self.actor.params) != ENTA_RETURNED_FROM_KAKARIKO && play.scene_id == SCENE_LON_LON_BUILDINGS {
            play.save.timer_state = TIMER_STATE_OFF;
        }
        if self.state_flags & TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY != 0 {
            play.audio.func_800f5b58();
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1], self.head_rot[2], self.torso_rot[0], self.torso_rot[1], self.torso_rot[2]];
        rs.switches = vec![self.eye_index as u32];
        rs
    }
    /// `EnTa_Draw`: `Gfx_SetupDL_37Opa`, the eyes on 8, the head's skin on 9, the skeleton.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let ([eye], [h0, h1, h2, t0, t1, t2]) = (rs.switches.as_slice(), rs.angles.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, [*h0, *h1, *h2], [*t0, *t1, *t2], play.state_frames);
        let eye = (*eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
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
