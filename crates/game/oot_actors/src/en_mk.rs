//! `En_Mk` (`ovl_En_Mk/z_en_mk.c`): the lakeside professor in his laboratory by Lake Hylia.
//!
//! He stands in his idle loop (`object_mk_Anim_000D88`) and offers to talk, or to be shown the
//! Eyeball Frog (`EXCH_ITEM_EYEBALL_FROG`), when Link is within 100 and within 0x2151 of his front
//! (`EnMk_Wait`, `Actor_OfferTalkExchangeEquiCylinder`); his head follows Link while he does
//! (`Actor_TrackPlayer`, limb 11), else it eases back.
//!
//! **The dive** (`EnMk_Update`): whenever Link swims under water (`PLAYER_STATE2_10`, Kokiri
//! boots: the iron boots stop the count until he surfaces), his deepest `depthInWater` is kept as
//! `swimFlag`: 1 under 80, 2 to 7 from 80 (`depth × 0.025`, 7 from 320), 8 from 355, which the
//! first time chimes (`NA_SE_SY_CORRECT_CHIME`). Talked to: 0x4018 with none, 0x406C plus the
//! count from 1 to 7 (0x406D to 0x4073, how deep he went), and at 8 0x4074 and the piece of heart
//! (`GI_HEART_PIECE`, offered at once from 10000 across, `ITEMGETINF_10`; the count back to 0), or
//! 0x4075 once he has it. The lab's tank is the dive.
//!
//! **The Eyeball Frog** (an adult's trade): shown it he takes it (0x4019, `NA_SE_SY_TRE_BOX_APPEAR`,
//! the sub timer off, `object_mk_Anim_000368`), goes on to 0x4000 and 0x4001 (`object_mk_Anim_000AC0`),
//! closes the box (`MSGMODE_PAUSED`, Link's exchange item cleared) and turns away 16 frames
//! (0x800 a frame), works 120 frames (`object_mk_Anim_000724`), turns back 16, says 0x4030, and
//! gives the Eye Drops (`GI_EYE_DROPS`); taken: the marathon's 240 seconds on the sub timer
//! (`Interface_SetSubTimer`), `EVENTINF_MARATHON_ACTIVE` cleared. With the Eye Drops in the trade
//! slot already he says 0x4032; anything else shown, 0x4018.
//!
//! Drawn as `EnMk_Draw` does: `object_mk_Skel_005DF0` with the head (limb 11) turned
//! (`EnMk_OverrideLimbDraw`), its focus 1000 along and 100 down it (`EnMk_PostLimbDraw`), from a
//! baked mesh (docs/adr/0012-actor-bakes.md) set up with `Gfx_SetupDL_25Opa` where the draw uses
//! `Gfx_SetupDL_37Opa`. The circle shadow is the shape's (`ActorShape_Init`, 36).
//!
//! The whole overlay is ported. Not ported here: the mask reactions
//! (`MaskReaction_GetTextId(play, MASK_REACTION_SET_LAKESIDE_PROFESSOR)`: no masks yet, so none).
//!
//! Params: none read (the lab places him with 0).
//! Flags: reads and sets ITEMGETINF_10 (the dive's piece of heart taken); clears EVENTINF_MARATHON_ACTIVE
//! (the Eye Drops taken: their timer).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::smooth_step_to_s;
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, PLAYER_BOOTS_IRON};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{GI_EYE_DROPS, GI_HEART_PIECE};
use oot_game::message::{MSGMODE_PAUSED, TEXT_STATE_EVENT};
use oot_game::npc::{EXCH_ITEM_NONE, actor_track_player, get_player_exchange_item_id, offer_talk_exchange, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::SUBTIMER_STATE_OFF;
use oot_game::skelanime_std::*;
use oot_game::{event_inf, item_get_inf};

/// `ACTOR_EN_MK` (`actor_table.h`: 0x014A).
pub const ACTOR_EN_MK: i16 = 0x014A;
/// `OBJECT_MK` (`object_table.h`: 0x0132).
pub const OBJECT: &str = "object_mk";

/// `En_Mk_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_MK,
    name: "En_Mk",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0; read by nothing");

/// `ITEMGETINF_10` (`save.h`: 0x10): the dive's piece of heart taken.
pub const ITEMGETINF_10: u16 = 0x10;
/// `EVENTINF_MARATHON_ACTIVE` (`save.h`: 0x10): the Eye Drops' run to Biggoron.
pub const EVENTINF_MARATHON_ACTIVE: u16 = 0x10;

/// En_Mk's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(item_get_inf!(ITEMGETINF_10)).reads().sets().note("the dive's piece of heart taken"),
        FlagUse::new(event_inf!(EVENTINF_MARATHON_ACTIVE)).clears().note("the Eye Drops taken: the sub timer restarted"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x4018).note("the professor: about the lake, nothing dived yet (or something else shown)"),
        TextLine::new(0x406D).cond("Link dived under 80").note("the dive's depth, 1"),
        TextLine::new(0x406E).cond("Link dived 80 to 119").note("the dive's depth, 2"),
        TextLine::new(0x406F).cond("Link dived 120 to 159").note("the dive's depth, 3"),
        TextLine::new(0x4070).cond("Link dived 160 to 199").note("the dive's depth, 4"),
        TextLine::new(0x4071).cond("Link dived 200 to 239").note("the dive's depth, 5"),
        TextLine::new(0x4072).cond("Link dived 240 to 279").note("the dive's depth, 6"),
        TextLine::new(0x4073).cond("Link dived 280 to 354").note("the dive's depth, 7"),
        TextLine::new(0x4074).cond("Link dived 355 or deeper, no piece of heart yet").note("the dive's piece of heart"),
        TextLine::new(0x4075).cond("Link dived 355 or deeper, the piece of heart taken"),
        TextLine::new(0x4032).cond("the Eye Drops in the trade slot"),
        TextLine::new(0x4019).cond("the Eyeball Frog shown (an adult's trade)"),
        TextLine::new(0x4000).cond("after 0x4019"),
        TextLine::new(0x4001).cond("after 0x4000"),
        TextLine::new(0x4030).cond("the Eye Drops made"),
    ],
};

/// `EXCH_ITEM_EYEBALL_FROG` (`item.h`: 0x0D).
pub const EXCH_ITEM_EYEBALL_FROG: u8 = 0x0D;
/// `ITEM_ODD_MUSHROOM` (`item.h`: 0x30): `INV_CONTENT` of it is the adult's trade slot.
pub const ITEM_ODD_MUSHROOM: u8 = 0x30;
/// `ITEM_EYE_DROPS` (`item.h`: 0x36).
pub const ITEM_EYE_DROPS: u8 = 0x36;
/// `ATTENTION_RANGE_6` (`actor.h`).
const ATTENTION_RANGE_6: u8 = 6;
/// `PLAYER_STATE2_10` (`player.h`: 1 << 10): under water.
const PLAYER_STATE2_10: u32 = 1 << 10;
/// The head's limb (1-based).
const HEAD_LIMB: usize = 11;
/// `D_80AAD64C`: the focus on the head.
const FOCUS_OFFSET: Vec3 = Vec3::new(1000.0, -100.0, 0.0);

/// `flags`: 1 Link tracked this frame, 2 the animation done, 4 the dive's chime played (or the
/// piece of heart taken), 8 the iron boots on.
pub const MK_FLAG_TRACKING: u16 = 1;
pub const MK_FLAG_ANIM_DONE: u16 = 2;
pub const MK_FLAG_CHIMED: u16 = 4;
pub const MK_FLAG_IRON_BOOTS: u16 = 8;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 40, y_shift: 0, pos: [0; 3] },
};

/// The professor's mesh.
pub const BAKE: &str = "En_Mk/skel";
const SKEL: &str = "object_mk_Skel_005DF0";
const ANIM_IDLE: &str = "object_mk_Anim_000D88";
const ANIM_WORK: &str = "object_mk_Anim_000724";
const ANIM_TALK: &str = "object_mk_Anim_000AC0";
const ANIM_TAKE: &str = "object_mk_Anim_000368";

/// The mesh `EnMk_Draw` draws: the whole skeleton.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnMk_Wait`.
    Wait,
    /// `func_80AACA40`: talking until the box closes.
    Talking,
    /// `func_80AACA94`: the Eye Drops offered until taken.
    GiveEyeDrops,
    /// `func_80AACB14`: 0x4030 closing, then the Eye Drops.
    EyeDropsText,
    /// `func_80AACB6C`: waiting for 0x4030's talk.
    WaitEyeDropsText,
    /// `func_80AACBAC`: turning back.
    TurnBack,
    /// `func_80AACC04`: working on the Eye Drops.
    Work,
    /// `func_80AACCA0`: turning away.
    TurnAway,
    /// `func_80AACD48`: 0x4001 until its event.
    Frog4001,
    /// `func_80AACE2C`: 0x4000 until its event.
    Frog4000,
    /// `func_80AACEE8`: 0x4019 until its event.
    Frog4019,
    /// `func_80AACFA0`: the piece of heart offered until taken.
    GiveHeartPiece,
    /// `func_80AAD014`: 0x4074 closing, then the piece of heart.
    HeartPieceText,
}

pub struct EnMk {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    /// `headRotation`.
    pub head_rotation: [i16; 3],
    pub flags: u16,
    pub swim_flag: i16,
    pub timer: u16,
    pub action: Action,
    anim_idle: Anim,
    anim_work: Anim,
    anim_talk: Anim,
    anim_take: Anim,
}

/// `player->actor.textId = id`.
fn set_player_text_id(play: &mut PlayState, id: u16) {
    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
        p.text_id = id;
    }
}

/// `Message_CloseTextbox`.
fn close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

impl EnMk {
    /// `EnMk_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            Ok((
                assets.skeleton(OBJECT, SKEL)?,
                assets.animation(OBJECT, ANIM_IDLE)?,
                assets.animation(OBJECT, ANIM_WORK)?,
                assets.animation(OBJECT, ANIM_TALK)?,
                assets.animation(OBJECT, ANIM_TAKE)?,
            ))
        })();
        let (skeleton, anim_idle, anim_work, anim_talk, anim_take) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Mk: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnMk {
            actor,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            // SkelAnime_InitFlex(.., object_mk_Skel_005DF0, object_mk_Anim_000D88, .., 13).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anim_idle.clone())),
            skeleton,
            head_rotation: [0; 3],
            flags: 0,
            swim_flag: 0,
            timer: 0,
            action: Action::Wait,
            anim_idle,
            anim_work,
            anim_talk,
            anim_take,
        };
        this.actor.min_velocity_y = -4.0;
        this.actor.gravity = -1.0;
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 36.0);
        // Animation_PlayLoop(object_mk_Anim_000D88).
        let idle = this.anim_idle.clone();
        let last = idle.last_frame();
        this.skel.change(idle, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        this.actor.scale = Vec3::splat(0.01);
        this.action = Action::Wait;
        this.flags = 0;
        this.swim_flag = 0;
        this.actor.target_mode = ATTENTION_RANGE_6;
        if play.save.get_item_get_inf(ITEMGETINF_10) {
            this.flags |= MK_FLAG_CHIMED;
        }
        Box::new(this)
    }

    /// `Animation_Change(&skelAnime, anim, 1.0f, 0.0f, Animation_GetLastFrame(anim), mode, -4.0f)`,
    /// and the animation's end flag cleared.
    fn change_anim(&mut self, anim: Anim, mode: u8) {
        let last = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, last, mode, -4.0);
        self.flags &= !MK_FLAG_ANIM_DONE;
    }

    /// `func_80AACA40`: talking; the box closing, back to waiting.
    fn talking(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.actor.flags &= !ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::Wait;
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `func_80AACA94`: the Eye Drops taken: the marathon's 240 seconds; else offered.
    fn give_eye_drops(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::Talking;
            play.interface_set_sub_timer(240);
            play.save.clear_event_inf(EVENTINF_MARATHON_ACTIVE);
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_EYE_DROPS, 10000.0, 50.0);
        }
    }

    /// `func_80AACB14`: 0x4030 closing: the Eye Drops offered.
    fn eye_drops_text(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::GiveEyeDrops;
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_EYE_DROPS, 10000.0, 50.0);
        }
    }

    /// `func_80AACB6C`.
    fn wait_eye_drops_text(&mut self) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::EyeDropsText;
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `func_80AACBAC`: turning back 0x800 a frame, then 0x4030.
    fn turn_back(&mut self, play: &mut PlayState) {
        if self.timer > 0 {
            self.timer -= 1;
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_sub(0x800);
        } else {
            self.action = Action::WaitEyeDropsText;
            play.continue_textbox(0x4030);
        }
    }

    /// `func_80AACC04`: working, then turning back.
    fn work(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
        } else {
            self.timer = 16;
            self.action = Action::TurnBack;
            let a = self.anim_idle.clone();
            self.change_anim(a, ANIMMODE_LOOP);
        }
    }

    /// `func_80AACCA0`: turning away 0x800 a frame, then 120 frames' work.
    fn turn_away(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
        } else {
            self.timer = 120;
            self.action = Action::Work;
            let a = self.anim_work.clone();
            self.change_anim(a, ANIMMODE_LOOP);
        }
    }

    /// `func_80AACD48`: 0x4001's event: the box paused, the frog taken from Link's hand, turning away.
    fn frog_4001(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            close_textbox(play);
            self.action = Action::TurnAway;
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                p.set_exchange_item_id(EXCH_ITEM_NONE);
            }
            self.timer = 16;
            let a = self.anim_idle.clone();
            self.change_anim(a, ANIMMODE_LOOP);
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `func_80AACE2C`: 0x4000's event: on to 0x4001.
    fn frog_4000(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.continue_textbox(0x4001);
            let a = self.anim_talk.clone();
            self.change_anim(a, ANIMMODE_ONCE);
            self.action = Action::Frog4001;
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `func_80AACEE8`: 0x4019's event: on to 0x4000.
    fn frog_4019(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.continue_textbox(0x4000);
            let a = self.anim_talk.clone();
            self.change_anim(a, ANIMMODE_LOOP);
            self.action = Action::Frog4000;
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `func_80AACFA0`: the piece of heart taken: `ITEMGETINF_10`; else offered.
    fn give_heart_piece(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::Talking;
            play.save.set_item_get_inf(ITEMGETINF_10);
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_HEART_PIECE, 10000.0, 50.0);
        }
    }

    /// `func_80AAD014`: 0x4074 closing: the piece of heart offered.
    fn heart_piece_text(&mut self, play: &mut PlayState) {
        if textbox_is_closing(play) {
            self.action = Action::GiveHeartPiece;
            let a = self.actor.clone();
            offer_get_item_range(play, &a, GI_HEART_PIECE, 10000.0, 50.0);
        }
        self.flags |= MK_FLAG_TRACKING;
    }

    /// `EnMk_Wait`: a talk taken picks Player's text by what's shown and the dive; else his text
    /// offered within 100 when Link is in front.
    fn wait(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            let player_exchange_item = get_player_exchange_item_id(play);
            if self.actor.text_id != 0x4018 {
                // A mask reaction's text (none: no masks).
                let id = self.actor.text_id;
                set_player_text_id(play, id);
                self.action = Action::Talking;
            } else if play.save.inv_content(ITEM_ODD_MUSHROOM) == ITEM_EYE_DROPS {
                set_player_text_id(play, 0x4032);
                self.action = Action::Talking;
            } else {
                match player_exchange_item {
                    EXCH_ITEM_NONE => {
                        if self.swim_flag >= 8 {
                            if play.save.get_item_get_inf(ITEMGETINF_10) {
                                set_player_text_id(play, 0x4075);
                                self.action = Action::Talking;
                            } else {
                                set_player_text_id(play, 0x4074);
                                self.action = Action::HeartPieceText;
                                self.swim_flag = 0;
                            }
                        } else if self.swim_flag == 0 {
                            set_player_text_id(play, 0x4018);
                            self.action = Action::Talking;
                        } else {
                            // schema: 0x406C plus 1 to 7, the lines 0x406D to 0x4073.
                            set_player_text_id(play, (0x406C + self.swim_flag) as u16);
                            self.action = Action::Talking;
                        }
                    }
                    EXCH_ITEM_EYEBALL_FROG => {
                        set_player_text_id(play, 0x4019);
                        self.action = Action::Frog4019;
                        let a = self.anim_take.clone();
                        self.change_anim(a, ANIMMODE_ONCE);
                        play.save.sub_timer_state = SUBTIMER_STATE_OFF;
                        play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    }
                    _ => {
                        set_player_text_id(play, 0x4018);
                        self.action = Action::Talking;
                    }
                }
            }
        } else {
            // MaskReaction_GetTextId(play, MASK_REACTION_SET_LAKESIDE_PROFESSOR): no masks, so 0.
            self.actor.text_id = 0;
            if self.actor.text_id == 0 {
                self.actor.text_id = 0x4018;
            }
            let angle = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32;
            if angle.abs() < 0x2151 && self.actor.xz_dist_to_player < 100.0 {
                let a = self.actor.clone();
                offer_talk_exchange(play, &a, 100.0, EXCH_ITEM_EYEBALL_FROG);
                self.flags |= MK_FLAG_TRACKING;
            }
        }
    }

    /// The dive's count (`EnMk_Update`'s end): Link's deepest under water, the chime at 8.
    fn update_swim_flag(&mut self, play: &mut PlayState) {
        let Some((state2, boots, depth)) = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player().map(|pi| (pi.state_flags2(), pi.current_boots(), p.base().y_dist_to_water))) else {
            return;
        };
        if self.flags & MK_FLAG_IRON_BOOTS != 0 {
            if state2 & PLAYER_STATE2_10 == 0 {
                self.flags &= !MK_FLAG_IRON_BOOTS;
            }
        } else if boots == PLAYER_BOOTS_IRON {
            self.flags |= MK_FLAG_IRON_BOOTS;
        } else if state2 & PLAYER_STATE2_10 != 0 {
            // swimFlag = player->actor.depthInWater: an s16.
            let mut swim_flag = depth as i16;
            if swim_flag > 0 {
                if swim_flag >= 320 {
                    swim_flag = if swim_flag >= 355 { 8 } else { 7 };
                } else if swim_flag < 80 {
                    swim_flag = 1;
                } else {
                    swim_flag = (swim_flag as f32 * 0.025) as i16;
                }
                if self.swim_flag < swim_flag {
                    self.swim_flag = swim_flag;
                    if self.flags & MK_FLAG_CHIMED == 0 && self.swim_flag >= 8 {
                        self.flags |= MK_FLAG_CHIMED;
                        play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                    }
                }
            }
        }
    }

    /// The pose with `EnMk_OverrideLimbDraw`'s head turn: limb 11's y less the head's y, its z
    /// plus the head's x.
    pub fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == HEAD_LIMB {
                rot[1] = rot[1].wrapping_sub(head[1]);
                rot[2] = rot[2].wrapping_add(head[0]);
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnMk {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnMk_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if self.flags & MK_FLAG_ANIM_DONE == 0 && self.skel.update() {
            self.flags |= MK_FLAG_ANIM_DONE;
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::Talking => self.talking(play),
            Action::GiveEyeDrops => self.give_eye_drops(play),
            Action::EyeDropsText => self.eye_drops_text(play),
            Action::WaitEyeDropsText => self.wait_eye_drops_text(),
            Action::TurnBack => self.turn_back(play),
            Action::Work => self.work(),
            Action::TurnAway => self.turn_away(),
            Action::Frog4001 => self.frog_4001(play),
            Action::Frog4000 => self.frog_4000(play),
            Action::Frog4019 => self.frog_4019(play),
            Action::GiveHeartPiece => self.give_heart_piece(play),
            Action::HeartPieceText => self.heart_piece_text(play),
        }
        if self.flags & MK_FLAG_TRACKING != 0 {
            let focus = self.actor.focus_pos;
            let mut vec = [0i16; 3];
            actor_track_player(play, &mut self.actor, &mut self.head_rotation, &mut vec, focus);
        } else {
            smooth_step_to_s(&mut self.head_rotation[0], 0, 6, 6200, 100);
            smooth_step_to_s(&mut self.head_rotation[1], 0, 6, 6200, 100);
        }
        self.update_swim_flag(play);
    }

    /// `EnMk_PostLimbDraw`, limb 11: the focus.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.head_rotation);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = self.head_rotation.to_vec();
        rs
    }

    /// `EnMk_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < 3 {
            return;
        }
        let bones = Self::pose(&self.skeleton, &joints.rot, [rs.angles[0], rs.angles[1], rs.angles[2]]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: oot_game::play::actor_draw_matrix(rs), bones, params: Default::default() });
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
