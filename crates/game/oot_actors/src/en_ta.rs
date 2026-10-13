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
//! **Ported:** the castle's and Kakariko's parts (Kakariko's Talon is killed for a child, so his
//! talk, `EXCH_ITEM_POCKET_CUCCO`, waits for the adult), the ranch's adult Talon's talk (0x2055),
//! the night's asleep Talon in the ranch's house (0x204B), the blinks, the animations and the draw.
//! **Not ported** (the user's scope, GAME-07): the ranch's house by day, the cucco game and the milk
//! (`EnTa_SetTextForTalkInLonLonHouse` to `EnTa_IdleAfterCuccoGameFinished`, about 600 lines): that
//! Talon is a placeholder.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, audio_play_actor_sfx2, func_80033480};
use oot_game::audio::sfx::NA_SE_PL_WALK_GROUND;
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::message::{TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{EXCH_ITEM_NONE, actor_track_player, get_player_exchange_item_id, offer_talk, offer_talk_exchange, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_TA` (`actor_table.h`: 0x0084).
pub const ACTOR_EN_TA: i16 = 0x0084;
pub const OBJECT: &str = "object_ta";

/// `En_Ta_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_TA, name: "En_Ta", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT };

/// `EnTaType`.
pub const ENTA_CHILD_ERA: i16 = -1;
pub const ENTA_IN_KAKARIKO: i16 = 1;
pub const ENTA_RETURNED_FROM_KAKARIKO: i16 = 2;

// `TALON_STATE_FLAG_*`.
const TALON_STATE_FLAG_TRACKING_PLAYER: u16 = 1 << 0;
const TALON_STATE_FLAG_SUPPRESS_BLINK: u16 = 1 << 2;
const TALON_STATE_FLAG_SUPPRESS_ROCKING_ANIM: u16 = 1 << 3;
const TALON_STATE_FLAG_ANIMATION_FINISHED: u16 = 1 << 4;
const TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY: u16 = 1 << 9;

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
    pub timer: i16,
    pub rapid_blinks: i16,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    pub state_flags: u16,
    pub nod_off_timer: i16,
    /// `currentAnimation`: what `EnTa_AnimRepeatCurrent` replays.
    pub current_animation: Anim,
    /// `shape.shadowScale`.
    pub shadow_scale: f32,
    anims: Anims,
}

struct Anims {
    stand: Anim,
    sleep: Anim,
    wake_up: Anim,
    run: Anim,
    run_transition: Anim,
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
            timer: 0,
            rapid_blinks: 0,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            state_flags: 0,
            nod_off_timer: 0,
            current_animation: anims.stand.clone(),
            // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 36).
            shadow_scale: 36.0,
            anims,
            actor,
        };
        this.actor.shape_y_offset = 0.0;
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        this.actor.target_mode = ATTENTION_RANGE_6;
        this.actor.velocity.y = -4.0;
        this.actor.min_velocity_y = -4.0;
        this.actor.gravity = -1.0;
        let adult = play.save.adult;
        let ev = |f: u16| play.save.get_event_chk_inf(f);
        match this.actor.params {
            ENTA_IN_KAKARIKO => {
                log::debug!(" Exile Talon ");
                if ev(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO) || !adult {
                    this.actor.kill();
                } else if ev(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO) {
                    this.setup_awake_standing(Action::IdleAwakeInKakariko);
                } else {
                    this.setup_asleep(Action::IdleAsleepInKakariko);
                }
            }
            ENTA_RETURNED_FROM_KAKARIKO => {
                log::debug!(" Return Talon ");
                if !ev(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO) || !adult {
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
                    if ev(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                        this.actor.kill();
                    } else if ev(EVENTCHKINF_TALON_WOKEN_IN_CASTLE) {
                        this.setup_awake_standing(Action::IdleAwakeInCastle);
                    } else {
                        this.setup_asleep(Action::IdleAsleepInCastle);
                    }
                } else if play.scene_id == SCENE_LON_LON_BUILDINGS {
                    log::debug!(" Talon in the warehouse at Lon Lon Ranch");
                    if !ev(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) || adult {
                        this.actor.kill();
                    } else if play.save.is_day() {
                        // The super cuccos, the cucco game and the milk (EnTa_SetTextForTalkInLonLonHouse,
                        // EnTa_IdleSittingInLonLonHouse, EnTa_IdleAfterCuccoGameFinished): the ranch's
                        // part, not ported.
                        log::info!("En_Ta: the ranch's Talon by day (the cucco game, the milk) isn't ported: a placeholder");
                        return oot_game::spawn::Placeholder::init(this.actor, play);
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
        self.shadow_scale = 54.0;
    }

    /// `EnTa_SetupAction`.
    fn setup_action(&mut self, action: Action, anim_func: AnimFunc) {
        self.action = action;
        self.anim_func = anim_func;
    }

    /// `EnTa_DecreaseShadowSize`: 0.8 a frame back to 36.
    fn decrease_shadow_size(&mut self) {
        if self.shadow_scale > 36.0 {
            self.shadow_scale -= 0.8;
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
    /// `EnTa_Destroy`: the ranch's timer off (the ranch's Talon), the music back
    /// (`TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY`, set only by the cucco game).
    fn destroy(&mut self, play: &mut PlayState) {
        if self.actor.params != ENTA_IN_KAKARIKO && self.actor.params != ENTA_RETURNED_FROM_KAKARIKO && play.scene_id == SCENE_LON_LON_BUILDINGS {
            log::debug!("En_Ta: gSaveContext.timerState = TIMER_STATE_OFF (the cucco game's timer isn't ported)");
        }
        if self.state_flags & TALON_STATE_FLAG_RESTORE_BGM_ON_DESTROY != 0 {
            log::debug!("En_Ta: func_800F5B58 (the music back after the cucco game) isn't ported");
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1], self.head_rot[2], self.torso_rot[0], self.torso_rot[1], self.torso_rot[2]];
        rs.switches = vec![self.eye_index as u32, self.shadow_scale.to_bits()];
        rs
    }
    /// `EnTa_Draw`: `Gfx_SetupDL_37Opa`, the eyes on 8, the head's skin on 9, the skeleton.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        let ([eye, shadow], [h0, h1, h2, t0, t1, t2]) = (rs.switches.as_slice(), rs.angles.as_slice()) else { return };
        let bones = Self::pose(&self.skeleton, &joints.rot, [*h0, *h1, *h2], [*t0, *t1, *t2], play.state_frames);
        let eye = (*eye as usize).min(EYE_TEXTURES.len() - 1);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye))), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
        // ActorShadow_DrawCircle (shadowScale): the circle shadow's stand-in.
        let s = f32::from_bits(*shadow);
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(s, 1.0, s));
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
