//! `En_Diving_Game` (`ovl_En_Diving_Game/z_en_diving_game.c`): Zora's Domain's diving game, the
//! Zora at the top of the falls (`object_zo`'s `gZoraSkel`, scaled 0.01 across, 0.013 up and 0.014
//! deep) standing in the upper pool. Both of the Domain's rooms place him at (-149, 857, -1020);
//! his init makes him room -1 (kept through the rooms), and the second one (`sIsInitialized`, the
//! overlay's static) is a duplicate, killed at once (`isDuplicate`, whose destroy leaves the timer
//! alone). The first never goes before the scene does, so the overlay never unloads mid-scene and
//! the static is never cleared.
//!
//! **The talk** (`EnDivingGame_Idle`, within 80 across and up or down): a mask's reaction first
//! (`MaskReaction_GetTextId`, `MASK_REACTION_SET_ZORA`, an ordinary text); else 0x4053 (before the
//! silver scale: `phase` 1) or 0x405C (after it: `phase` 2), a choice; Link held
//! (`PLAYER_CSACTION_8`). Yes with 20 rupees or more: `Rupees_ChangeBy(-20)`, 0x4054 before the scale,
//! straight to the throw after it (the message box paused, `MSGMODE_PAUSED`); with fewer, 0x85; no,
//! 0x2D. Either of those ends the game (`phase` 0): the text's event closes it, Link is let go
//! (`PLAYER_CSACTION_7`).
//!
//! **The throw** (`EnDivingGame_StartThrowAnimation` to `_UnderwaterViewCs`): `gZoraThrowRupeesAnim`
//! once (the fidget stops, `notPlayingMinigame`); from its frame 15 a sub camera (the main one
//! waiting, the main camera's fov) from the view's eye raised 80 and moved 250 along z, at the view's
//! at, easing over up to 60 frames (`subCamTimer`) to the eye (-280, -20, -240) and the at (-210,
//! -80, -1020), each axis at a tenth of the way a frame, at most 0.04 of its distance times a factor
//! growing 0.02 a frame. Ten frames in, a rupee every five (`EnDivingGame_SpawnRuppy`: `En_Ex_Ruppy`
//! type 0 as his child, ±15 across, 30 ± 10 up, ±10 deep, turned `Rand_CenteredFloat(3500) - 1000`,
//! its `world.rot.z` the rupees left to throw, at 12 forward and 6 up): five before the scale, ten
//! after, each setting `throwTimer` (30 before, 5 after), which ends the rupees' fall. The pan done,
//! once `throwTimer` is out the camera cuts to the pool (`rupeePhase` 1), then the rupees sink
//! (`rupeePhase` 2) for 70 frames; the main camera back, 0x405A ("START!") continues the paused box.
//! @bug (game): the eye's height never eases (no `Math_ApproachF` for its y, though its speed is
//! worked out), so the pan always runs its 60 frames. While the sub camera pans, his projected
//! position under water (`func_800C0DB4`; @bug (game): `projectedPos` is in view space, read as a
//! world position) turns the water's muffling off (`Audio_SetExtraFilter(0)`).
//!
//! **The game** (`EnDivingGame_StartMinigame`): 0x405A read, the box closes, the HUD timer at 50
//! seconds (`Interface_SetTimer(50 + BREG(2))`, the same either way), the timed mini-game's music
//! (`func_800F5ACC(NA_BGM_TIMED_MINI_GAME)`), Link let go; talked to now, 0x405B. The rupees count
//! themselves into `grabbedRupees` as Link takes them (`En_Ex_Ruppy`). Each frame
//! (`EnDivingGame_HasMinigameFinished`): the timer out (`TIMER_STATE_STOP`, the one frame
//! `Interface_Draw` leaves it so, not in a cutscene's hold) is the loss: `NA_SE_SY_FOUND`, 0x71AD,
//! Link held, the game over (`phase` 0, the rupees blinking out); five rupees (ten after the scale)
//! the win: the timer off, the music back (`func_800F5B58`), `NA_BGM_SMALL_ITEM_GET`, Link held,
//! 0x4055 before the scale (Link let go as it ends; talked to again, 0x4056, then
//! `Actor_OfferGetItem(GI_SCALE_SILVER)` within 90 across and 10 up or down until he takes it, and
//! once its text is read `EVENTCHKINF_OBTAINED_SILVER_SCALE`), 0x405D after it (`extraWinCount` up to
//! 100, which nothing reads: see `En_Ex_Ruppy`). At 10 seconds left the music speeds up
//! (`Audio_SetFastTempoForTimedMinigame`, each frame it shows 10).
//!
//! He blinks (two frames a step, then 20 to 79 open), turns his head and body to Link
//! (`Npc_TrackPoint`, preset 2, `NPC_TRACKING_FULL_BODY`), focuses 80 up, and every 16 gameplay
//! frames rings a ripple 20 above his feet (`EffectSsGRipple_Spawn`, 100 to 500 over 30): the
//! surface he stands in. His cylinder (10 by 10) pushes, immovable.
//!
//! Drawn as `EnDivingGame_Draw` does: the eyes on segment 8 (`En_Diving_Game/eye<n>`, one bake each),
//! segment 0xC an empty list, the env colour black, the skeleton through
//! `EnDivingGame_OverrideLimbDraw`: the head (limb 15) turned by the head's yaw on x and its z on z,
//! the arms and torso (8, 9, 12) fidgeting while he isn't playing (`FIDGET_*` by `state.frames`).
//! @bug (game): the torso's yaw turns limb 6, the right shin, not the torso (8); and the head's z
//! (`headRot.z`) is never set by the tracking, so the head never tilts. The circle shadow (30) is
//! the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! The whole overlay is ported.
//!
//! Params: `params` whole unused (0xFFFF in both rooms).
//! Flags: reads and sets EVENTCHKINF_OBTAINED_SILVER_SCALE (the first win's prize given: 0x405C,
//! ten rupees and 0x405D from then on); writes `gSaveContext.timerState` (off as the game ends, as
//! he goes) and reads `timerSeconds`.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::NA_SE_SY_FOUND;
use oot_game::audio::{NA_BGM_SMALL_ITEM_GET, NA_BGM_TIMED_MINI_GAME};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::rupees_change_by;
use oot_game::message::{MSGMODE_PAUSED, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{NpcTrack, offer_talk, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::save::{TIMER_STATE_OFF, TIMER_STATE_STOP};
use oot_game::skelanime_std::*;

use crate::en_ex_ruppy::ACTOR_EN_EX_RUPPY;

/// `ACTOR_EN_DIVING_GAME` (`actor_table.h`: 0x0124).
pub const ACTOR_EN_DIVING_GAME: i16 = 0x0124;
/// `OBJECT_ZO` (`object_table.h`: 0x00FE).
pub const OBJECT: &str = "object_zo";

/// `En_Diving_Game_Profile`: `ACTORCAT_NPC`, `FLAGS` (`ACTOR_FLAG_ATTENTION_ENABLED |
/// ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED`).
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DIVING_GAME,
    name: "En_Diving_Game",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The whole params: 0xFFFF in the game's data, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "0xFFFF in the data; read by nothing");

/// `EVENTCHKINF_OBTAINED_SILVER_SCALE` (`save.h`: 0x38).
pub const EVENTCHKINF_OBTAINED_SILVER_SCALE: u16 = 0x38;

/// En_Diving_Game's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_SILVER_SCALE)).reads().sets().note("the first win's prize given: then 0x405C, ten rupees and no prize"),
        FlagUse::new(Flag::Other("gSaveContext.timerState")).reads().sets().note("the HUD timer: set at the start, off as the game ends or he goes; its stop is the loss"),
    ],
    todo: false,
    objects: &[],
    // EnDivingGame_Idle (a mask's reaction first), _HandlePlayChoice, _UnderwaterViewCs,
    // _StartMinigame, _HasMinigameFinished, _EarnedScale.
    texts: &[
        TextLine::new(0x7188).cond("Link wearing the Keaton Mask, Skull Mask or Bunny Hood").note("what do you want?"),
        TextLine::new(0x7189).cond("Link wearing the Spooky Mask, Goron Mask or Mask of Truth").note("you don't look like you can swim very well"),
        TextLine::new(0x718B).cond("Link wearing the Zora Mask").note("you look a little small for a Zora"),
        TextLine::new(0x718A).cond("Link wearing the Gerudo Mask").note("red hair, tan skin: I've seen people like that"),
        TextLine::new(0x4053).cond("before the silver scale").note("Diving Game, 20 Rupees: want to play?"),
        TextLine::new(0x405C).cond("after the silver scale").note("take as much as you want for 20 Rupees"),
        TextLine::new(0x4054).cond("yes with 20 Rupees, before the silver scale").note("OKAY! pick up all the Rupees I throw"),
        TextLine::new(0x0085).cond("yes with fewer than 20 Rupees").note("not enough Rupees"),
        TextLine::new(0x002D).cond("no").note("you don't have to play"),
        TextLine::new(0x405A).cond("the rupees thrown").note("START!"),
        TextLine::new(0x405B).cond("talked to while the game runs").note("go over the falls for a shortcut"),
        TextLine::new(0x71AD).cond("the time run out").note("too bad! practice hard"),
        TextLine::new(0x4055).cond("five rupees taken, before the silver scale").note("congratulations! come and get it"),
        TextLine::new(0x405D).cond("ten rupees taken, after the silver scale").note("congratulations!!"),
        TextLine::new(0x4056).cond("talked to after the first win").note("what a graceful dive: the silver scale"),
    ],
};

// `EnDivingGamePhase`.
pub const ENDIVINGGAME_PHASE_ENDED: i16 = 0;
/// Before the silver scale: five rupees.
pub const ENDIVINGGAME_PHASE_1: i16 = 1;
/// After it: ten.
pub const ENDIVINGGAME_PHASE_2: i16 = 2;
// `EnDivingGameState`.
pub const ENDIVINGGAME_STATE_NOTPLAYING: i16 = 0;
/// Waiting to give the scale.
pub const ENDIVINGGAME_STATE_AWARDPRIZE: i16 = 1;
pub const ENDIVINGGAME_STATE_PLAYING: i16 = 2;
// `EnDivingGameRupeePhase`.
/// No rupees under water yet (they may be in the air).
pub const ENDIVINGGAME_RUPEE_PHASE_NONE: i16 = 0;
/// The camera cut to the pool.
pub const ENDIVINGGAME_RUPEE_PHASE_SWITCHING_CAM: i16 = 1;
/// The rupees placed in the pool, sinking: they can be taken.
pub const ENDIVINGGAME_RUPEE_PHASE_SINKING: i16 = 2;

/// `GI_SCALE_SILVER` (`item.h`: 0x37).
pub const GI_SCALE_SILVER: i16 = 0x37;
/// `PLAYER_CSACTION_7`, `PLAYER_CSACTION_8` (`player.h`): let go; held (also in water).
pub const PLAYER_CSACTION_7: u8 = 7;
pub const PLAYER_CSACTION_8: u8 = 8;
/// `ATTENTION_RANGE_0` (`actor.h`).
const ATTENTION_RANGE_0: u8 = 0;
/// `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `BREG(2)` (`regs.h`): a debug register, 0.
const BREG_2: i16 = 0;
/// `SUB_CAM_ID_DONE` (`camera.h`: 0).
const SUB_CAM_ID_DONE: i16 = 0;
/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`: 0x814,
/// 0x940, 0x32, 200).
const FIDGET_FREQ_Y: i32 = 0x814;
const FIDGET_FREQ_Z: i32 = 0x940;
const FIDGET_FREQ_LIMB: i32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

/// `MASK_REACTION_SET_ZORA` (`face_reaction.h`: 0x1D).
const MASK_REACTION_SET_ZORA: usize = 0x1D;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_ZORA]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_ZORA: [u16; 9] = [0x0000, 0x7188, 0x7188, 0x7189, 0x7188, 0x7189, 0x718B, 0x718A, 0x7189];

/// `EnDivingGame_SetupRupeeThrowCam`'s targets: the sub camera's at and eye over the pool.
pub const SUB_CAM_AT_NEXT: Vec3 = Vec3::new(-210.0, -80.0, -1020.0);
pub const SUB_CAM_EYE_NEXT: Vec3 = Vec3::new(-280.0, -20.0, -240.0);

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
    dim: Cylinder16 { radius: 10, height: 10, y_shift: 0, pos: [0; 3] },
};

// `ZoraLimb` (`object_zo.xml`, counted from 1), as `EnDivingGame_OverrideLimbDraw` numbers them.
const ZORA_LIMB_RIGHT_SHIN: usize = 6;
const ZORA_LIMB_TORSO: usize = 8;
const ZORA_LIMB_LEFT_ARM: usize = 9;
const ZORA_LIMB_RIGHT_ARM: usize = 12;
const ZORA_LIMB_HEAD: usize = 15;

/// `sEyeTextures`.
const EYE_TEXTURES: [&str; 3] = ["gZoraEyeOpenTex", "gZoraEyeHalfTex", "gZoraEyeClosedTex"];
const SEG_EYES: u8 = 0x08;
/// `gSPSegment(0x0C, EnDivingGame_EmptyDList(..))`.
const SEG_EMPTY: u8 = 0x0C;
/// `gDPSetEnvColor(0, 0, 0, 255)`, run before the skeleton.
const SEG_ENV: u8 = 0x0E;

/// The skeleton with eye `eye` (`En_Diving_Game/eye<n>`).
pub fn bake_name(eye: usize) -> String {
    format!("En_Diving_Game/eye{eye}")
}

/// The meshes `EnDivingGame_Draw` draws: the skeleton with each eye on segment 8, segment 0xC an
/// empty list, the env colour black (opaque).
pub fn bakes() -> Vec<MeshBake> {
    let end = (0xDF00_0000u32, 0u32);
    EYE_TEXTURES
        .iter()
        .enumerate()
        .map(|(eye, tex)| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![
                (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_EMPTY, BakeSegment::Commands(vec![end])),
                // G_SETENVCOLOR (0xFB) 0, 0, 0, 255.
                (SEG_ENV, BakeSegment::Commands(vec![(0xFB00_0000, 0x0000_00FF), end])),
            ],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gZoraSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// The overlay's static (docs/adr/0059-castle-towns-actors.md): `sIsInitialized`, the one Zora
/// placed (zeroed with a fresh overlay).
#[derive(Debug, Default)]
pub struct Statics {
    pub is_initialized: bool,
}

/// The statics.
pub fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_DIVING_GAME)
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Reset,
    Idle,
    HandlePlayChoice,
    TalkOutsideMinigame,
    StartThrowAnimation,
    BeforeRupeeThrow,
    SetupRupeeThrowCam,
    ThrowRupees,
    SetupUnderwaterViewCs,
    UnderwaterViewCs,
    StartMinigame,
    TalkDuringMinigame,
    EarnedScale,
    WaitGiveScale,
    GiveScale,
    EndMinigameAfterScaleGiven,
    /// The duplicate, killed in its init (no `actionFunc` set).
    None,
}

pub struct EnDivingGame {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    /// `unk_290`: a counter that only goes up.
    pub unk_290: i16,
    /// `nextTextState`: the message state a text's advance is waited on in.
    pub next_text_state: u8,
    pub sub_cam_timer: i16,
    /// `throwTimer`: read by the rupees (their fall ends at 0).
    pub throw_timer: i16,
    pub eye_timer: i16,
    pub spawn_ruppy_timer: i16,
    /// `phase` (`EnDivingGamePhase`).
    pub phase: i16,
    pub eye_tex_index: i16,
    pub sub_cam_id: i16,
    /// `rupeePhase` (`EnDivingGameRupeePhase`): read by the rupees.
    pub rupee_phase: i16,
    /// `grabbedRupees`: counted up by the rupees.
    pub grabbed_rupees: i16,
    pub rupees_left_to_throw: i16,
    /// `state` (`EnDivingGameState`).
    pub state: i16,
    /// `extraWinCount`: the wins after the scale (up to 100).
    pub extra_win_count: i16,
    pub sub_cam_at: Vec3,
    pub sub_cam_eye: Vec3,
    pub sub_cam_eye_next: Vec3,
    pub sub_cam_eye_max_vel_frac: Vec3,
    pub sub_cam_eye_vel: Vec3,
    pub sub_cam_at_next: Vec3,
    pub sub_cam_at_max_vel_frac: Vec3,
    pub sub_cam_at_vel: Vec3,
    pub sub_cam_vel_factor: f32,
    pub not_playing_minigame: bool,
    pub all_rupees_thrown: bool,
    pub is_duplicate: bool,
    /// `interactInfo`: the head and torso tracking.
    pub interact_info: NpcTrack,
    pub collider: ColliderCylinder,
    idle_anim: Anim,
    throw_rupees_anim: Anim,
}

/// `Message_CloseTextbox`.
fn message_close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

// shared: MaskReaction_GetTextId
/// `MaskReaction_GetTextId` (`z_face_reaction.c`): `sMaskReactionSetTextIds[set][Player_GetMask]`,
/// here only the Zoras' row (`MASK_REACTION_SET_ZORA`); 0 without a mask.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.current_mask()) as usize;
    match set {
        MASK_REACTION_SET_ZORA => MASK_REACTION_ZORA.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

// shared: func_800C0DB4
/// `func_800C0DB4` (`z_play.c`): `pos` under a water surface (`BgCheck_GetWaterSurfaceAllHack`)
/// with a floor below that surface (`BgCheck_EntityRaycastDown3`).
fn func_800c0db4(play: &PlayState, pos: Vec3) -> bool {
    match play.bg_check_get_water_surface_all_hack(pos.x, pos.z, pos.y) {
        Some(surface) if pos.y < surface => play.col.entity_raycast_down(Vec3::new(pos.x, surface, pos.z)).0 != BGCHECK_Y_MIN,
        _ => false,
    }
}

impl EnDivingGame {
    /// `EnDivingGame_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> { Ok((assets.skeleton(OBJECT, "gZoraSkel")?, assets.animation(OBJECT, "gZoraIdleAnim")?, assets.animation(OBJECT, "gZoraThrowRupeesAnim")?)) })();
        let (skeleton, idle_anim, throw_rupees_anim) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Diving_Game: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnDivingGame {
            actor,
            // SkelAnime_InitFlex(.., &gZoraSkel, &gZoraIdleAnim, .., 20).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(idle_anim.clone())),
            skeleton,
            action: Action::None,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            unk_290: 0,
            next_text_state: 0,
            sub_cam_timer: 0,
            throw_timer: 0,
            eye_timer: 0,
            spawn_ruppy_timer: 0,
            phase: ENDIVINGGAME_PHASE_ENDED,
            eye_tex_index: 0,
            sub_cam_id: SUB_CAM_ID_DONE,
            rupee_phase: ENDIVINGGAME_RUPEE_PHASE_NONE,
            grabbed_rupees: 0,
            rupees_left_to_throw: 0,
            state: ENDIVINGGAME_STATE_NOTPLAYING,
            extra_win_count: 0,
            sub_cam_at: Vec3::ZERO,
            sub_cam_eye: Vec3::ZERO,
            sub_cam_eye_next: Vec3::ZERO,
            sub_cam_eye_max_vel_frac: Vec3::ZERO,
            sub_cam_eye_vel: Vec3::ZERO,
            sub_cam_at_next: Vec3::ZERO,
            sub_cam_at_max_vel_frac: Vec3::ZERO,
            sub_cam_at_vel: Vec3::ZERO,
            sub_cam_vel_factor: 0.0,
            not_playing_minigame: false,
            all_rupees_thrown: false,
            is_duplicate: false,
            interact_info: NpcTrack::default(),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            idle_anim,
            throw_rupees_anim,
        };
        this.actor.gravity = -3.0;
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        log::debug!("☆☆☆☆☆ Diving GO ☆☆☆☆☆");
        this.actor.room = -1;
        this.actor.scale = Vec3::new(0.01, 0.012999999, 0.0139999995);
        if statics(play).is_initialized {
            log::debug!("☆☆☆☆☆ I'm already here -Hara ☆☆☆☆☆");
            this.is_duplicate = true;
            this.actor.kill();
        } else {
            statics(play).is_initialized = true;
            this.actor.target_mode = ATTENTION_RANGE_0;
            this.actor.col_chk_info.mass = MASS_IMMOVABLE;
            this.action = Action::Reset;
        }
        Box::new(this)
    }

    /// `EnDivingGame_SpawnRuppy`: a rupee as his child, about 30 above him, thrown at 12 forward
    /// and 6 up, turned about -1000; its `world.rot.z` the rupees left to throw.
    fn spawn_ruppy(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        let x = (play.rand.zero_one() - 0.5) * 30.0 + p.x;
        let y = (play.rand.zero_one() - 0.5) * 20.0 + (p.y + 30.0);
        let z = (play.rand.zero_one() - 0.5) * 20.0 + p.z;
        let yaw = (play.rand.centered_float(3500.0) as i16).wrapping_sub(1000);
        let left = self.rupees_left_to_throw;
        if let Ok(h) = play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_EX_RUPPY, Vec3::new(x, y, z), [0, yaw, left], 0)
            && let Some(rupee) = play.actors.actor_mut(h)
        {
            rupee.speed_xz = 12.0;
            rupee.velocity.y = 6.0;
        }
    }

    /// The wipe the game's ends share: `allRupeesThrown = state = phase = rupeePhase =
    /// grabbedRupees = 0`.
    fn clear_game(&mut self) {
        self.grabbed_rupees = 0;
        self.rupee_phase = ENDIVINGGAME_RUPEE_PHASE_NONE;
        self.phase = ENDIVINGGAME_PHASE_ENDED;
        self.state = ENDIVINGGAME_STATE_NOTPLAYING;
        self.all_rupees_thrown = false;
    }

    /// `EnDivingGame_HasMinigameFinished`: the timer stopped (out of a cutscene's hold) is the loss;
    /// five rupees (ten after the scale) the win.
    fn has_minigame_finished(&mut self, play: &mut PlayState) -> bool {
        if play.save.timer_state == TIMER_STATE_STOP && !play.play_in_cs_mode() {
            // Failed.
            play.save.timer_state = TIMER_STATE_OFF;
            play.audio.func_800f5b58();
            play.audio.play_sfx_centered(NA_SE_SY_FOUND);
            self.actor.text_id = 0x71AD;
            play.start_textbox(self.actor.text_id, None);
            self.next_text_state = TEXT_STATE_EVENT;
            self.clear_game();
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::TalkOutsideMinigame;
            return true;
        }
        let rupees_needed = if play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) { 10 } else { 5 };
        if self.grabbed_rupees >= rupees_needed {
            // Won.
            play.save.timer_state = TIMER_STATE_OFF;
            self.clear_game();
            if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) {
                self.actor.text_id = 0x4055;
            } else {
                self.actor.text_id = 0x405D;
                if self.extra_win_count < 100 {
                    self.extra_win_count += 1;
                }
            }
            play.start_textbox(self.actor.text_id, None);
            self.next_text_state = TEXT_STATE_EVENT;
            play.audio.func_800f5b58();
            play.audio.play_fanfare(NA_BGM_SMALL_ITEM_GET);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) { Action::EarnedScale } else { Action::TalkOutsideMinigame };
            return true;
        }
        false
    }

    /// `this->nextTextState == Message_GetState(&play->msgCtx) && Message_ShouldAdvance(play)`.
    fn text_advanced(&self, play: &mut PlayState) -> bool {
        self.next_text_state == play.message_state() && play.message_should_advance()
    }

    /// `EnDivingGame_Reset`: `gZoraIdleAnim` looping, the fidget on.
    fn reset(&mut self) {
        let frame_count = self.idle_anim.last_frame();
        self.skel.change(self.idle_anim.clone(), 1.0, 0.0, frame_count as i16 as f32, ANIMMODE_LOOP, -10.0);
        self.not_playing_minigame = true;
        self.action = Action::Idle;
    }

    /// `EnDivingGame_Idle`: while playing, the game's end first; a talk taken by the state; else
    /// the text by the state (a mask's first) and an offer within 80.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.state == ENDIVINGGAME_STATE_PLAYING && self.has_minigame_finished(play) {
            return;
        }
        if process_talk_request(&mut self.actor) {
            if self.next_text_state != TEXT_STATE_DONE {
                match self.state {
                    ENDIVINGGAME_STATE_NOTPLAYING => {
                        play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
                        self.action = Action::HandlePlayChoice;
                    }
                    ENDIVINGGAME_STATE_AWARDPRIZE => self.action = Action::WaitGiveScale,
                    ENDIVINGGAME_STATE_PLAYING => self.action = Action::TalkDuringMinigame,
                    _ => {}
                }
            }
        } else {
            let mask_text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_ZORA);
            if mask_text_id != 0 {
                self.actor.text_id = mask_text_id;
                self.next_text_state = TEXT_STATE_DONE;
            } else {
                match self.state {
                    ENDIVINGGAME_STATE_NOTPLAYING => {
                        self.next_text_state = TEXT_STATE_CHOICE;
                        if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) {
                            self.actor.text_id = 0x4053;
                            self.phase = ENDIVINGGAME_PHASE_1;
                        } else {
                            self.actor.text_id = 0x405C;
                            self.phase = ENDIVINGGAME_PHASE_2;
                        }
                    }
                    ENDIVINGGAME_STATE_AWARDPRIZE => {
                        self.actor.text_id = 0x4056;
                        self.next_text_state = TEXT_STATE_EVENT;
                    }
                    ENDIVINGGAME_STATE_PLAYING => {
                        self.actor.text_id = 0x405B;
                        self.next_text_state = TEXT_STATE_EVENT;
                    }
                    _ => {}
                }
            }
            offer_talk(play, &self.actor, 80.0);
        }
    }

    /// `EnDivingGame_HandlePlayChoice`: yes with 20 rupees paid (0x4054 before the scale, straight to
    /// the throw after it), short (0x85) or no (0x2D) the game over.
    fn handle_play_choice(&mut self, play: &mut PlayState) {
        self.skel.update();
        // Did the player select an answer?
        if !self.text_advanced(play) {
            return;
        }
        match play.msg_ctx.choice_index {
            // Yes.
            0 => {
                if play.save.rupees >= 20 {
                    rupees_change_by(&mut play.save, -20);
                    self.actor.text_id = 0x4054;
                } else {
                    self.actor.text_id = 0x85;
                    self.clear_game();
                }
            }
            // No.
            1 => {
                self.actor.text_id = 0x2D;
                self.clear_game();
            }
            _ => {}
        }
        if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) || self.actor.text_id == 0x85 || self.actor.text_id == 0x2D {
            play.continue_textbox(self.actor.text_id);
            self.next_text_state = TEXT_STATE_EVENT;
            self.action = Action::TalkOutsideMinigame;
        } else {
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            self.action = Action::StartThrowAnimation;
        }
    }

    /// `EnDivingGame_TalkOutsideMinigame`: the text's event read: the game over (Link let go, idle
    /// again), or the throw (the box paused).
    fn talk_outside_minigame(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.text_advanced(play) {
            if self.phase == ENDIVINGGAME_PHASE_ENDED {
                message_close_textbox(play);
                play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
                self.action = Action::Reset;
            } else {
                play.msg_ctx.msg_mode = MSGMODE_PAUSED;
                play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
                self.action = Action::StartThrowAnimation;
            }
        }
    }

    /// `EnDivingGame_StartThrowAnimation`: `gZoraThrowRupeesAnim` once, the fidget off.
    fn start_throw_animation(&mut self) {
        let frame_count = self.throw_rupees_anim.last_frame();
        self.skel.change(self.throw_rupees_anim.clone(), 1.0, 0.0, frame_count as i16 as f32, ANIMMODE_ONCE, -10.0);
        self.not_playing_minigame = false;
        self.action = Action::BeforeRupeeThrow;
    }

    /// `EnDivingGame_BeforeRupeeThrow`: from frame 15 (the frame before this update's).
    fn before_rupee_throw(&mut self) {
        let current_frame = self.skel.cur_frame;
        self.skel.update();
        if current_frame >= 15.0 {
            self.action = Action::SetupRupeeThrowCam;
        }
    }

    /// `EnDivingGame_SetupRupeeThrowCam`: the sub camera from the view's eye (80 up, 250 along z)
    /// and at, to pan for up to 60 frames; the first rupee in 10; five to throw (ten after the
    /// scale).
    fn setup_rupee_throw_cam(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.sub_cam_id = play.create_sub_camera();
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        self.spawn_ruppy_timer = 10;
        self.sub_cam_at_next = SUB_CAM_AT_NEXT;
        self.sub_cam_eye_next = SUB_CAM_EYE_NEXT;
        self.rupees_left_to_throw = if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) { 5 } else { 10 };
        self.sub_cam_eye_max_vel_frac = Vec3::splat(0.1);
        self.sub_cam_at_max_vel_frac = Vec3::splat(0.1);
        self.sub_cam_at = play.view.at;
        self.sub_cam_eye = Vec3::new(play.view.eye.x, play.view.eye.y + 80.0, play.view.eye.z + 250.0);
        self.sub_cam_eye_vel = (self.sub_cam_eye - self.sub_cam_eye_next).abs() * 0.04;
        self.sub_cam_at_vel = (self.sub_cam_at - self.sub_cam_at_next).abs() * 0.04;
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_at, self.sub_cam_eye);
        let fov = play.game_camera.fov;
        play.camera_set_fov(self.sub_cam_id, fov);
        self.sub_cam_timer = 60;
        self.action = Action::ThrowRupees;
        self.sub_cam_vel_factor = 0.0;
    }

    /// `EnDivingGame_ThrowRupees`: the pan (not the eye's height), a rupee every five frames; the pan
    /// done (or timed out), to the pool's view or the sinking.
    fn throw_rupees(&mut self, play: &mut PlayState) {
        self.skel.update();
        if func_800c0db4(play, self.actor.projected_pos) {
            play.audio.set_extra_filter(0);
        }
        if self.sub_cam_id != SUB_CAM_ID_DONE {
            let f = self.sub_cam_vel_factor;
            approach_f(&mut self.sub_cam_eye.x, self.sub_cam_eye_next.x, self.sub_cam_eye_max_vel_frac.x, self.sub_cam_eye_vel.x * f);
            approach_f(&mut self.sub_cam_eye.z, self.sub_cam_eye_next.z, self.sub_cam_eye_max_vel_frac.z, self.sub_cam_eye_vel.z * f);
            approach_f(&mut self.sub_cam_at.x, self.sub_cam_at_next.x, self.sub_cam_at_max_vel_frac.x, self.sub_cam_at_vel.x * f);
            approach_f(&mut self.sub_cam_at.y, self.sub_cam_at_next.y, self.sub_cam_at_max_vel_frac.y, self.sub_cam_at_vel.y * f);
            approach_f(&mut self.sub_cam_at.z, self.sub_cam_at_next.z, self.sub_cam_at_max_vel_frac.z, self.sub_cam_at_vel.z * f);
            approach_f(&mut self.sub_cam_vel_factor, 1.0, 1.0, 0.02);
        }
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_at, self.sub_cam_eye);
        if !self.all_rupees_thrown && self.spawn_ruppy_timer == 0 {
            self.spawn_ruppy_timer = 5;
            self.spawn_ruppy(play);
            self.rupees_left_to_throw -= 1;
            self.throw_timer = if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) { 30 } else { 5 };
            if self.rupees_left_to_throw <= 0 {
                self.rupees_left_to_throw = 0;
                self.all_rupees_thrown = true;
            }
        }
        let near = |a: Vec3, b: Vec3| (a.x - b.x).abs() < 2.0 && (a.y - b.y).abs() < 2.0 && (a.z - b.z).abs() < 2.0;
        if self.sub_cam_timer == 0 || (near(self.sub_cam_eye, self.sub_cam_eye_next) && near(self.sub_cam_at, self.sub_cam_at_next)) {
            if self.rupee_phase != ENDIVINGGAME_RUPEE_PHASE_NONE {
                self.sub_cam_timer = 70;
                self.rupee_phase = ENDIVINGGAME_RUPEE_PHASE_SINKING;
                self.action = Action::UnderwaterViewCs;
            } else {
                self.action = Action::SetupUnderwaterViewCs;
            }
        }
    }

    /// `EnDivingGame_SetupUnderwaterViewCs`: the last throw's fall over: the camera cut to the
    /// pool's view, the pan again for up to 100 frames (done at once).
    fn setup_underwater_view_cs(&mut self) {
        self.skel.update();
        if self.throw_timer == 0 {
            self.rupee_phase = ENDIVINGGAME_RUPEE_PHASE_SWITCHING_CAM;
            self.sub_cam_timer = 100;
            self.action = Action::ThrowRupees;
            self.sub_cam_at = SUB_CAM_AT_NEXT;
            self.sub_cam_at_next = SUB_CAM_AT_NEXT;
            self.sub_cam_eye = SUB_CAM_EYE_NEXT;
            self.sub_cam_eye_next = SUB_CAM_EYE_NEXT;
        }
    }

    /// `EnDivingGame_UnderwaterViewCs`: the sinking watched; the timer out, the main camera back and
    /// 0x405A.
    fn underwater_view_cs(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.sub_cam_timer == 0 {
            play.clear_camera(self.sub_cam_id);
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
            self.actor.text_id = 0x405A;
            play.continue_textbox(self.actor.text_id);
            self.next_text_state = TEXT_STATE_EVENT;
            self.action = Action::StartMinigame;
        }
    }

    /// `EnDivingGame_StartMinigame`: 0x405A read: the timer at 50, the music, Link let go, playing.
    fn start_minigame(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.text_advanced(play) {
            message_close_textbox(play);
            // The same 50 seconds either way.
            if !play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE) {
                play.interface_set_timer(50 + BREG_2);
            } else {
                play.interface_set_timer(50 + BREG_2);
            }
            play.audio.func_800f5acc(NA_BGM_TIMED_MINI_GAME);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.actor.text_id = 0x405B;
            self.next_text_state = TEXT_STATE_EVENT;
            self.state = ENDIVINGGAME_STATE_PLAYING;
            self.action = Action::Idle;
        }
    }

    /// `EnDivingGame_TalkDuringMinigame`: 0x405B read; meanwhile the game's end.
    fn talk_during_minigame(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.text_advanced(play) {
            message_close_textbox(play);
            self.action = Action::Idle;
        } else {
            self.has_minigame_finished(play);
        }
    }

    /// `EnDivingGame_EarnedScale`: 0x4055 read: Link let go, the prize waiting (0x4056).
    fn earned_scale(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.text_advanced(play) {
            message_close_textbox(play);
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.actor.text_id = 0x4056;
            self.next_text_state = TEXT_STATE_EVENT;
            self.state = ENDIVINGGAME_STATE_AWARDPRIZE;
            self.action = Action::Idle;
        }
    }

    /// `EnDivingGame_WaitGiveScale`: 0x4056 read: the silver scale offered.
    fn wait_give_scale(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.text_advanced(play) {
            message_close_textbox(play);
            self.actor.parent = None;
            offer_get_item_range(play, &self.actor, GI_SCALE_SILVER, 90.0, 10.0);
            self.action = Action::GiveScale;
        }
    }

    /// `EnDivingGame_GiveScale`: offered until taken.
    fn give_scale(&mut self, play: &mut PlayState) {
        self.skel.update();
        if actor_has_parent(&self.actor) {
            self.action = Action::EndMinigameAfterScaleGiven;
        } else {
            offer_get_item_range(play, &self.actor, GI_SCALE_SILVER, 90.0, 10.0);
        }
    }

    /// `EnDivingGame_EndMinigameAfterScaleGiven`: the item's text read: the scale's flag, idle again.
    fn end_minigame_after_scale_given(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            log::debug!("☆☆☆☆☆ Normal termination ☆☆☆☆☆");
            self.clear_game();
            play.save.set_event_chk_inf(EVENTCHKINF_OBTAINED_SILVER_SCALE);
            self.action = Action::Reset;
        }
    }

    /// The limb matrices with `EnDivingGame_OverrideLimbDraw`'s turns: limb 6 by the torso's yaw on
    /// x, the head (15) by the head's yaw on x and its z on z, the fidget on 8, 9 and 12 while he
    /// isn't playing.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3], not_playing: bool, frames: u32) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            // @bug (game): the right shin, where En_Zo turns the torso (8).
            if limb == ZORA_LIMB_RIGHT_SHIN {
                rot[0] = rot[0].wrapping_add(torso[1]);
            }
            if limb == ZORA_LIMB_HEAD {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[2]);
            }
            if not_playing && (limb == ZORA_LIMB_TORSO || limb == ZORA_LIMB_LEFT_ARM || limb == ZORA_LIMB_RIGHT_ARM) {
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

impl ActorImpl for EnDivingGame {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDivingGame_Update`.
    fn update(&mut self, play: &mut PlayState) {
        for t in [&mut self.sub_cam_timer, &mut self.throw_timer, &mut self.eye_timer, &mut self.spawn_ruppy_timer] {
            if *t != 0 {
                *t -= 1;
            }
        }
        if play.save.timer_seconds == 10 {
            play.audio.audio_set_fast_tempo_for_timed_minigame();
        }
        if self.eye_timer == 0 {
            self.eye_timer = 2;
            self.eye_tex_index += 1;
            if self.eye_tex_index >= 3 {
                self.eye_tex_index = 0;
                self.eye_timer = play.rand.zero_float(60.0) as i16 + 20;
            }
        }
        match self.action {
            Action::Reset => self.reset(),
            Action::Idle => self.idle(play),
            Action::HandlePlayChoice => self.handle_play_choice(play),
            Action::TalkOutsideMinigame => self.talk_outside_minigame(play),
            Action::StartThrowAnimation => self.start_throw_animation(),
            Action::BeforeRupeeThrow => self.before_rupee_throw(),
            Action::SetupRupeeThrowCam => self.setup_rupee_throw_cam(play),
            Action::ThrowRupees => self.throw_rupees(play),
            Action::SetupUnderwaterViewCs => self.setup_underwater_view_cs(),
            Action::UnderwaterViewCs => self.underwater_view_cs(play),
            Action::StartMinigame => self.start_minigame(play),
            Action::TalkDuringMinigame => self.talk_during_minigame(play),
            Action::EarnedScale => self.earned_scale(play),
            Action::WaitGiveScale => self.wait_give_scale(play),
            Action::GiveScale => self.give_scale(play),
            Action::EndMinigameAfterScaleGiven => self.end_minigame_after_scale_given(play),
            // A killed duplicate's update never runs (a NULL actionFunc in the C).
            Action::None => return,
        }
        self.actor.set_focus(80.0);
        let pp = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.target = pp;
        self.interact_info.target.y = pp.y;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 2, NPC_TRACKING_FULL_BODY);
        self.interact_info = t;
        self.head_rot = self.interact_info.head;
        self.torso_rot = self.interact_info.torso;
        if play.gameplay_frames % 16 == 0 {
            let mut pos = self.actor.world_pos;
            pos.y += 20.0;
            play.with_ss(|s| s.g_ripple_spawn(pos, 100, 500, 30));
        }
        self.unk_290 = self.unk_290.wrapping_add(1);
        self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnDivingGame_Destroy`: the timer off (not the duplicate's), `Collider_DestroyCylinder`.
    fn destroy(&mut self, play: &mut PlayState) {
        if !self.is_duplicate {
            play.save.timer_state = TIMER_STATE_OFF;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.head_rot, self.torso_rot);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.switches = vec![self.eye_tex_index as u32, self.not_playing_minigame as u32];
        rs
    }

    /// `EnDivingGame_Draw`: the eye on segment 8, segment 0xC empty, the env colour black, the
    /// skeleton through `EnDivingGame_OverrideLimbDraw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let ([eye, not_playing], [h0, h1, h2, t0, t1, t2]) = (rs.switches.as_slice(), rs.angles.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, [*h0, *h1, *h2], [*t0, *t1, *t2], *not_playing != 0, play.state_frames);
        let eye = (*eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: actor_draw_matrix(rs), bones, params: Default::default() });
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
