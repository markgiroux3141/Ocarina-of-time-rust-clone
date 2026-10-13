//! `En_Heishi2` (`ovl_En_Heishi2/z_en_heishi2.c`): the guards that open gates. `params & 0xFF` is
//! the type, `params >> 8` a message number (`unk_2F0`, kept, unread).
//!
//! - **Type 2, Hyrule Castle's gate guard** (by Lon Lon's fence, `Bg_Spot15_Saku`): "won't pass"
//!   (0x7029); with the Weird Egg had (`EVENTCHKINF_RECEIVED_WEIRD_EGG`) a bribe (0x7071, then 0x7072:
//!   a choice). Ten rupees (0x7098) and he slams his spear (`NA_SE_EV_SPEAR_HIT`), calls the fence
//!   open and watches it from a sub camera for up to 200 frames, Link held; after, "anything passes"
//!   (0x7099, `unk_30C`, not saved). Too few rupees 0x7097, no 0x7096. By night "kids should be
//!   asleep" (0x7002); with the three stones 0x7006.
//! - **Type 5, Kakariko's gate guard** (at Death Mountain's gate, `Bg_Gate_Shutter`): shown Zelda's
//!   letter (`EXCH_ITEM_ZELDAS_LETTER`, `NA_SE_SY_CORRECT_CHIME`, 0x2010) he opens the gate the same
//!   way and sets `INFTABLE_76` (0x2015); after, he wants the Keaton Mask (0x200A/0x200B: a choice
//!   that sells it, `ITEM_SOLD_OUT`, 15 rupees back as 0x2021, `INFTABLE_77`), then 0x2020, and wears
//!   it (drawn on his head from `object_link_child` once sold). Not shown the letter 0x200E, shown
//!   anything else 0x200F.
//! - **Type 6**: the guards seen through Zelda's courtyard's window, a static list
//!   (`EnHeishi2_DrawKingGuard`, `gHeishiKingGuardDL`).
//! - **Type 9** (not placed): a guard lying on his side who rolls over, says 0x708F and throws a
//!   bomb (`En_Bom`, a placeholder: BACKLOG #33), and is gone.
//!
//! Each turns his head to Link (`Actor_TrackPlayer`) and offers to talk within 120 in front of him.
//! The mask reactions give 0 (no masks yet: BACKLOG #39). The whole overlay is ported.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2,
    UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4,
};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ACTORCAT_NPC, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::camera::{CAM_ID_MAIN, CAM_STAT_ACTIVE, CAM_STAT_WAIT};
use oot_game::collision_check::*;
use oot_game::item::{ITEM_SOLD_OUT, item_give, rupees_change_by};
use oot_game::message::{MSGMODE_PAUSED, TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT};
use oot_game::npc::{EXCH_ITEM_NONE, actor_track_player, get_player_exchange_item_id, offer_talk_default, offer_talk_exchange, process_talk_request};
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

use crate::bg_gate_shutter::{ACTOR_BG_GATE_SHUTTER, BgGateShutter, INFTABLE_76};
use crate::bg_spot15_saku::{ACTOR_BG_SPOT15_SAKU, BgSpot15Saku};
use crate::en_heishi4::{self, EnHeishi4};

/// `ACTOR_EN_HEISHI2` (`actor_table.h`: 0x00B3).
pub const ACTOR_EN_HEISHI2: i16 = 0x00B3;
pub const OBJECT: &str = "object_sd";

/// `En_Heishi2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HEISHI2, name: "En_Heishi2", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY, object: OBJECT };

/// `ACTOR_EN_BOM` (`actor_table.h`: 0x0010).
const ACTOR_EN_BOM: i16 = 0x0010;
/// `OBJECT_LINK_CHILD` (`object_table.h`: 0x0015).
const OBJECT_LINK_CHILD: i16 = 0x0015;

// `EVENTCHKINF_*`, `INFTABLE_*`, `ITEMGETINF_*` (`save.h`).
pub const EVENTCHKINF_RECEIVED_WEIRD_EGG: u16 = 0x12;
const EVENTCHKINF_09: u16 = 0x09;
const EVENTCHKINF_25: u16 = 0x25;
const EVENTCHKINF_37: u16 = 0x37;
pub const INFTABLE_77: u16 = 0x77;
const ITEMGETINF_38: u16 = 0x38;

/// `EXCH_ITEM_ZELDAS_LETTER` (`item.h`: 0x01).
pub const EXCH_ITEM_ZELDAS_LETTER: u8 = 0x01;
/// `PLAYER_MASK_KEATON` (`player.h`: 1).
const PLAYER_MASK_KEATON: u8 = 1;
/// `NA_SE_EV_SPEAR_HIT` (`environmentbank_table.h`: 0x2823).
pub const NA_SE_EV_SPEAR_HIT: u16 = 0x2823;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `PLAYER_CSACTION_7` (back to normal), `PLAYER_CSACTION_8` (held).
const PLAYER_CSACTION_7: u8 = 7;
const PLAYER_CSACTION_8: u8 = 8;

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
    dim: Cylinder16 { radius: 33, height: 40, y_shift: 0, pos: [0; 3] },
};

/// `gLinkChildKeatonMaskDL` from its second command: its first loads the matrix the draw puts on
/// segment 13 (`MATRIX_FINALIZE(..) - 7`, the list's `0x0D0001C0`), which the draw's transform
/// stands for.
pub const KEATON_MASK_BAKE: &str = "En_Heishi2/keaton_mask";

/// The mesh `EnHeishi2_Draw` draws besides the skeleton (`En_Heishi4`'s bake);
/// `EnHeishi2_DrawKingGuard`'s `gHeishiKingGuardDL` is the pack's mesh.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: KEATON_MASK_BAKE.into(),
        object: "object_link_child".into(),
        segments: Vec::new(),
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![("object_link_child".into(), "gLinkChildKeatonMaskDL+0x8".into())]),
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    /// `EnHeishi2_DoNothing1`: type 6.
    DoNothing1,
    /// `EnHeishi_DoNothing2`.
    DoNothing2,
    /// `func_80A531E4`: type 2's idle animation.
    Func80A531E4,
    /// `func_80A53278`: type 2's text.
    Func80A53278,
    /// `func_80A5344C`: the bribe's choice.
    Func80A5344C,
    /// `func_80A53538`: paid; Link held.
    Func80A53538,
    /// `func_80A535BC`: the spear's slam.
    Func80A535BC,
    /// `func_80A53638`: the slam, then the fence called open.
    Func80A53638,
    /// `func_80A5372C`: the sub camera on the fence.
    Func80A5372C,
    /// `func_80A53850`: watching the fence.
    Func80A53850,
    /// `func_80A53908`: type 5's idle animation.
    Func80A53908,
    /// `func_80A5399C`: type 5's text.
    Func80A5399C,
    /// `func_80A53AD4`: type 5 waiting for the letter.
    Func80A53AD4,
    /// `func_80A53C0C`: the letter shown; Link held.
    Func80A53C0C,
    /// `func_80A53C90`: the spear's slam.
    Func80A53C90,
    /// `func_80A53D0C`: the slam, then the gate called open.
    Func80A53D0C,
    /// `func_80A53DF8`: the sub camera on the gate.
    Func80A53DF8,
    /// `func_80A53F30`: watching the gate.
    Func80A53F30,
    /// `func_80A54038`: the gate's text read; `INFTABLE_76`.
    Func80A54038,
    /// `func_80A540C0`: the Keaton Mask's choice.
    Func80A540C0,
    /// `func_80A541FC`: the mask sold, the rupees.
    Func80A541FC,
    /// `func_80A5427C`: the gate shut again (or not).
    Func80A5427C,
    /// `func_80A54320`: the spear's slam.
    Func80A54320,
    /// `func_80A543A0`: the slam, then the gate called.
    Func80A543A0,
    /// `func_80A544AC`: type 9 rolling over.
    Func80A544AC,
    /// `func_80A5455C`: type 9's text, then the bomb.
    Func80A5455C,
    /// `func_80A546DC`: type 9 rolling back, then gone.
    Func80A546DC,
    /// `func_80A5475C`: offering to talk.
    Func80A5475C,
    /// `func_80A54954`: the idle animation after a text.
    Func80A54954,
    /// `func_80A549E8`: the text's end.
    Func80A549E8,
}

pub struct EnHeishi2 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    /// `unk_260`, `unk_26C`: the head's and torso's turns (`Actor_TrackPlayer`).
    pub unk_260: [i16; 3],
    pub unk_26c: [i16; 3],
    /// `unk_274`: where type 9 was placed.
    pub unk_274: Vec3,
    pub sub_cam_eye: Vec3,
    pub sub_cam_at: Vec3,
    pub sub_cam_at_init: Vec3,
    pub sub_cam_eye_init: Vec3,
    /// `unk_2E0`: the focus's height.
    pub unk_2e0: f32,
    /// `unk_2E4`: type 9's roll speed.
    pub unk_2e4: f32,
    /// `unk_2EC`: the slam's last frame.
    pub unk_2ec: f32,
    pub unk_2f0: u16,
    /// `unk_2F2`: timers; `[0]` the sub camera's 200 frames.
    pub unk_2f2: [i16; 5],
    pub unk_2fc: i16,
    pub ty: i16,
    /// `unk_300`: the message state the text ends on.
    pub unk_300: u8,
    /// `audioFlag`: the slam's sound played.
    pub audio_flag: u8,
    /// `unk_309`: the Keaton Mask asked about once.
    pub unk_309: u8,
    /// `unk_30A`: type 5's gate: 0 opening for the letter, 1 shutting, 2 the mask's opening.
    pub unk_30a: u8,
    /// `unk_30B`: the text is a mask reaction.
    pub unk_30b: u8,
    /// `unk_30C`: type 2: the fence opened ("anything passes").
    pub unk_30c: u8,
    /// `unk_30E`: type 2: the bribe offered once; type 5: the gate shuts after.
    pub unk_30e: i16,
    pub sub_cam_id: i16,
    /// `gate`: the `Bg_Spot15_Saku` or `Bg_Gate_Shutter` called.
    pub gate: Option<ActorHandle>,
    pub collider: Option<ColliderCylinder>,
    /// `actor.draw`: `EnHeishi2_DrawKingGuard` for types 6 and 9.
    pub draw_king_guard: bool,
    anims: Option<Anims>,
}

struct Anims {
    idle: Anim,
    slam_spear: Anim,
}

impl EnHeishi2 {
    /// `EnHeishi2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnHeishi2 {
            skel: SkelAnimeStd::init_flex(0, None),
            skeleton: None,
            action: Action::None,
            unk_260: [0; 3],
            unk_26c: [0; 3],
            unk_274: Vec3::ZERO,
            sub_cam_eye: Vec3::ZERO,
            sub_cam_at: Vec3::ZERO,
            sub_cam_at_init: Vec3::ZERO,
            sub_cam_eye_init: Vec3::ZERO,
            unk_2e0: 0.0,
            unk_2e4: 0.0,
            unk_2ec: 0.0,
            unk_2f0: 0,
            unk_2f2: [0; 5],
            unk_2fc: 0,
            ty: actor.params & 0xFF,
            unk_300: 0,
            audio_flag: 0,
            unk_309: 0,
            unk_30a: 0,
            unk_30b: 0,
            unk_30c: 0,
            unk_30e: 0,
            sub_cam_id: 0,
            gate: None,
            collider: None,
            draw_king_guard: false,
            anims: None,
            actor,
        };
        this.actor.scale = Vec3::splat(0.01);
        this.actor.col_chk_info.mass = MASS_IMMOVABLE;
        if this.ty == 6 || this.ty == 9 {
            this.draw_king_guard = true;
            this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            // Actor_ChangeCategory(.., ACTORCAT_PROP).
            this.actor.category = ACTORCAT_PROP;
            if this.ty == 6 {
                this.action = Action::DoNothing1;
            } else {
                log::debug!(" ☆☆☆☆☆ Oh, no, I was just careless. ☆☆☆☆☆ ");
                this.actor.scale = Vec3::splat(0.02);
                this.unk_274 = this.actor.world_pos;
                this.actor.world_rot.y = 0x7918;
                this.actor.world_pos += Vec3::new(90.0, -60.0, 90.0);
                this.actor.shape_rot.y = this.actor.world_rot.y;
                // Collider_DestroyCylinder (never initialised).
                play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
                this.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                this.action = Action::Func80A544AC;
            }
            return Box::new(this);
        }
        this.unk_2e0 = 60.0;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 30).
        this.actor.shape_y_offset = 0.0;
        let loaded = play.assets.clone().map(|assets| -> anyhow::Result<_> {
            Ok((assets.skeleton(OBJECT, "gEnHeishiSkel")?, Anims { idle: assets.animation(OBJECT, "gEnHeishiIdleAnim")?, slam_spear: assets.animation(OBJECT, "gEnHeishiSlamSpearAnim")? }))
        });
        match loaded {
            Some(Ok((skeleton, anims))) => {
                // SkelAnime_Init(.., gEnHeishiSkel, gEnHeishiIdleAnim, .., 17).
                this.skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims.idle.clone()));
                this.skeleton = Some(skeleton);
                this.anims = Some(anims);
            }
            Some(Err(e)) => {
                log::error!("En_Heishi2: {e:#}");
                this.actor.kill();
                return oot_game::spawn::Placeholder::init(this.actor, play);
            }
            None => {
                this.actor.kill();
                return oot_game::spawn::Placeholder::init(this.actor, play);
            }
        }
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.dim.y_shift = 0;
        collider.dim.radius = 15;
        collider.dim.height = 70;
        this.collider = Some(collider);
        this.actor.target_mode = ATTENTION_RANGE_6;
        match this.ty {
            2 => {
                this.action = Action::Func80A531E4;
                this.actor.gravity = -1.0;
            }
            5 => {
                this.action = Action::Func80A53908;
                this.actor.gravity = -1.0;
            }
            // (Type 6 is taken above: the C's case here is never reached.) Any other type keeps
            // actionFunc NULL: the update would jump to address 0.
            t => log::warn!("En_Heishi2: type {t} has no action (actionFunc NULL)"),
        }
        this.unk_2f0 = this.actor.params as u16 >> 8;
        log::debug!(" ☆☆☆☆☆ Soldier Set 2 Completed!  ☆☆☆☆☆ {}", this.actor.params);
        Box::new(this)
    }

    fn anims(&self) -> &Anims {
        self.anims.as_ref().expect("a skeleton guard's animations")
    }

    /// `Animation_Change(&skelAnime, idle, 1, 0, (s16)lastFrame, LOOP, -10)`.
    fn change_idle(&mut self) {
        let anim = self.anims().idle.clone();
        let frames = anim.last_frame() as i16 as f32;
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_LOOP, -10.0);
    }

    /// `Animation_Change(&skelAnime, gEnHeishiSlamSpearAnim, 1, 0, lastFrame, ONCE, -10)`, its last
    /// frame in `unk_2EC`.
    fn change_slam(&mut self) {
        let anim = self.anims().slam_spear.clone();
        let frames = anim.last_frame();
        self.unk_2ec = frames;
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_ONCE, -10.0);
    }

    /// `MaskReaction_GetTextId(play, MASK_REACTION_SET_HYRULIAN_GUARD)`: no masks, so 0
    /// (BACKLOG #39).
    fn mask_reaction_text(&self) -> u16 {
        0
    }

    /// `func_80A531E4`.
    fn func_80a531e4(&mut self) {
        self.change_idle();
        self.action = Action::Func80A53278;
    }

    /// `func_80A53278`: the castle gate guard's text.
    fn func_80a53278(&mut self, play: &PlayState) {
        self.unk_30b = 0;
        self.unk_30e = 0;
        let s = &play.save;
        if self.mask_reaction_text() != 0 {
            self.actor.text_id = self.mask_reaction_text();
            self.unk_30b = 1;
            self.unk_300 = TEXT_STATE_DONE;
            self.action = Action::Func80A5475C;
        } else if s.get_event_chk_inf(EVENTCHKINF_09) && s.get_event_chk_inf(EVENTCHKINF_25) && s.get_event_chk_inf(EVENTCHKINF_37) {
            log::debug!(" ☆☆☆☆☆ All the spirit stones GET! ☆☆☆☆☆ ");
            self.unk_300 = TEXT_STATE_DONE;
            self.actor.text_id = 0x7006;
            self.action = Action::Func80A5475C;
        } else if !s.is_day() {
            log::debug!(" ☆☆☆☆☆ Sleep early for children! ☆☆☆☆☆ ");
            self.unk_300 = TEXT_STATE_DONE;
            self.actor.text_id = 0x7002;
            self.action = Action::Func80A5475C;
        } else if self.unk_30c != 0 {
            log::debug!(" ☆☆☆☆☆ Anything passes ☆☆☆☆☆ ");
            self.unk_300 = TEXT_STATE_DONE;
            self.actor.text_id = 0x7099;
            self.action = Action::Func80A5475C;
        } else if s.get_event_chk_inf(EVENTCHKINF_RECEIVED_WEIRD_EGG) {
            // (unk_30E was zeroed above: always the first bribe's text.)
            if self.unk_30e == 0 {
                log::debug!(" ☆☆☆☆☆ Start under the first sleeve! ☆☆☆☆☆ ");
                self.actor.text_id = 0x7071;
                self.unk_30e = 1;
            } else {
                log::debug!(" ☆☆☆☆☆ Start under the second sleeve! ☆☆☆☆☆ ");
                self.actor.text_id = 0x7072;
            }
            self.unk_300 = TEXT_STATE_CHOICE;
            self.action = Action::Func80A5475C;
        } else {
            log::debug!(" ☆☆☆☆☆ That won't work ☆☆☆☆☆ ");
            self.unk_300 = TEXT_STATE_DONE;
            self.actor.text_id = 0x7029;
            self.action = Action::Func80A5475C;
        }
    }

    /// `func_80A5344C`: the bribe's choice: ten rupees (0x7098) or too few (0x7097), or no (0x7096).
    fn func_80a5344c(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            self.unk_300 = TEXT_STATE_EVENT;
            match play.msg_ctx.choice_index {
                0 => {
                    if play.save.rupees >= 10 {
                        rupees_change_by(&mut play.save, -10);
                        self.actor.text_id = 0x7098;
                        self.action = Action::Func80A53538;
                    } else {
                        self.actor.text_id = 0x7097;
                        self.action = Action::Func80A54954;
                    }
                }
                1 => {
                    self.actor.text_id = 0x7096;
                    self.action = Action::Func80A54954;
                }
                _ => {}
            }
            let id = self.actor.text_id;
            play.continue_textbox(id);
        }
    }

    /// `func_80A53538`.
    fn func_80a53538(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_300 == play.message_state() && play.message_should_advance() {
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            self.action = Action::Func80A535BC;
        }
    }

    /// `func_80A535BC`.
    fn func_80a535bc(&mut self) {
        self.change_slam();
        self.action = Action::Func80A53638;
    }

    /// The slam's sound from its frame 12 (`frameCount` is the frame before this update's).
    fn slam_sound(&mut self, play: &mut PlayState, frame_count: f32) {
        if frame_count >= 12.0 && self.audio_flag == 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_SPEAR_HIT);
            self.audio_flag = 1;
        }
    }

    /// `func_80A53638`: at the slam's end, the first `Bg_Spot15_Saku` of the item actions is
    /// called open (`unk_168`).
    fn func_80a53638(&mut self, play: &mut PlayState) {
        let frame_count = self.skel.cur_frame;
        self.skel.update();
        self.slam_sound(play, frame_count);
        if self.unk_2ec <= frame_count {
            match play.actors.find(ACTOR_BG_SPOT15_SAKU, ACTORCAT_ITEMACTION) {
                Some(h) => {
                    self.gate = Some(h);
                    if let Some(s) = play.actors.downcast_mut::<BgSpot15Saku>(h) {
                        s.unk_168 = 1;
                    }
                    log::debug!("☆☆☆ It's here, it's here, it's here! ☆☆☆");
                }
                // The C reads the NULL list end's next (a debug PRINTF).
                None => log::warn!("En_Heishi2: no Bg_Spot15_Saku among the item actions"),
            }
            self.action = Action::Func80A5372C;
        }
    }

    /// `func_80A5372C`: the sub camera, from (1164, 1145, 3014) at (947, 1195, 2682), for up to 200
    /// frames.
    fn func_80a5372c(&mut self, play: &mut PlayState) {
        self.change_idle();
        self.unk_2f2[0] = 200;
        self.sub_cam_id = play.create_sub_camera();
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        self.sub_cam_eye = Vec3::new(947.0, 1195.0, 2682.0);
        self.sub_cam_at = Vec3::new(1164.0, 1145.0, 3014.0);
        // Play_SetCameraAtEye(play, subCamId, &subCamEye, &subCamAt): the fields go in the other
        // way round (subCamEye is the at).
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_eye, self.sub_cam_at);
        self.action = Action::Func80A53850;
    }

    /// `func_80A53850`: until the fence is done or 200 frames, then the main camera back, the
    /// text closed, Link free and "anything passes" (`unk_30C`).
    fn func_80a53850(&mut self, play: &mut PlayState) {
        self.skel.update();
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_eye, self.sub_cam_at);
        let gate_open_call = self.gate.and_then(|h| play.actors.downcast::<BgSpot15Saku>(h)).map_or(0, |s| s.unk_168);
        if self.unk_2f2[0] == 0 || gate_open_call == 0 {
            play.clear_camera(self.sub_cam_id);
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.unk_30c = 1;
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.action = Action::Func80A531E4;
        }
    }

    /// `func_80A53908`.
    fn func_80a53908(&mut self) {
        self.change_idle();
        self.action = Action::Func80A5399C;
    }

    /// `func_80A5399C`: Kakariko's gate guard's text.
    fn func_80a5399c(&mut self, play: &PlayState) {
        self.unk_30b = 0;
        let mut var = 0;
        if play.save.get_inf_table(INFTABLE_76) {
            if !play.save.get_inf_table(INFTABLE_77) {
                let mask = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map_or(0, |p| p.current_mask());
                if mask == PLAYER_MASK_KEATON {
                    self.actor.text_id = if self.unk_309 == 0 { 0x200A } else { 0x200B };
                    self.unk_300 = TEXT_STATE_CHOICE;
                    self.unk_30b = 1;
                    var = 1;
                } else {
                    self.actor.text_id = 0x2016;
                    self.unk_300 = TEXT_STATE_DONE;
                    var = 1;
                }
            } else {
                self.actor.text_id = 0x2020;
                self.unk_300 = TEXT_STATE_EVENT;
                self.unk_30e = 0;
            }
            if self.mask_reaction_text() != 0 && var == 0 {
                self.actor.text_id = self.mask_reaction_text();
                self.unk_30b = 1;
                self.unk_300 = TEXT_STATE_DONE;
                self.unk_30e = 0;
            }
            self.action = Action::Func80A5475C;
        } else {
            log::debug!(" ☆☆☆☆☆ There is no way out ☆☆☆☆☆ ");
            self.action = Action::Func80A53AD4;
        }
    }

    /// `func_80A53AD4`: offering a talk for Zelda's letter within 120 in front.
    fn func_80a53ad4(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.actor.text_id = if self.mask_reaction_text() != 0 { self.mask_reaction_text() } else { 0x200E };
        self.unk_300 = TEXT_STATE_DONE;
        if process_talk_request(&mut self.actor) {
            let exchange_item_id = get_player_exchange_item_id(play);
            if exchange_item_id == EXCH_ITEM_ZELDAS_LETTER {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
                set_player_text_id(play, 0x2010);
                self.unk_300 = TEXT_STATE_EVENT;
                self.action = Action::Func80A53C0C;
            } else if exchange_item_id != EXCH_ITEM_NONE {
                set_player_text_id(play, 0x200F);
            }
            return;
        }
        let yaw_diff = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs();
        if !(120.0 < self.actor.xz_dist_to_player) && yaw_diff < 0x4300 {
            let a = self.actor.clone();
            offer_talk_exchange(play, &a, 100.0, EXCH_ITEM_ZELDAS_LETTER);
        }
    }

    /// `func_80A53C0C`.
    fn func_80a53c0c(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_300 == play.message_state() && play.message_should_advance() {
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_8);
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            self.action = Action::Func80A53C90;
        }
    }

    /// `func_80A53C90`.
    fn func_80a53c90(&mut self) {
        self.change_slam();
        self.action = Action::Func80A53D0C;
    }

    /// The first `Bg_Gate_Shutter` of the item actions, its `openingState` set.
    fn call_gate(&mut self, play: &mut PlayState, opening_state: Option<i16>) {
        match play.actors.find(ACTOR_BG_GATE_SHUTTER, ACTORCAT_ITEMACTION) {
            Some(h) => {
                self.gate = Some(h);
                if let (Some(g), Some(state)) = (play.actors.downcast_mut::<BgGateShutter>(h), opening_state) {
                    g.opening_state = state;
                }
                log::debug!("☆☆☆ It's here, it's here, it's here! ☆☆☆");
            }
            // The C reads the NULL list end's next.
            None => log::warn!("En_Heishi2: no Bg_Gate_Shutter among the item actions"),
        }
    }

    /// `func_80A53D0C`: at the slam's end, the gate called open (`openingState` 1).
    fn func_80a53d0c(&mut self, play: &mut PlayState) {
        let frame_count = self.skel.cur_frame;
        self.skel.update();
        self.slam_sound(play, frame_count);
        if self.unk_2ec <= frame_count {
            self.call_gate(play, Some(1));
            self.action = Action::Func80A53DF8;
        }
    }

    /// `func_80A53DF8`: the sub camera, from (181, 417, -1079) at (-71, 571, -1487), for up to 200
    /// frames.
    fn func_80a53df8(&mut self, play: &mut PlayState) {
        self.change_idle();
        self.unk_2f2[0] = 200;
        self.sub_cam_id = play.create_sub_camera();
        play.change_camera_status(CAM_ID_MAIN, CAM_STAT_WAIT);
        play.change_camera_status(self.sub_cam_id, CAM_STAT_ACTIVE);
        self.sub_cam_eye_init = Vec3::new(-71.0, 571.0, -1487.0);
        self.sub_cam_eye = self.sub_cam_eye_init;
        self.sub_cam_at_init = Vec3::new(181.0, 417.0, -1079.0);
        self.sub_cam_at = self.sub_cam_at_init;
        // (subCamEye as the at, as for the fence.)
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_eye, self.sub_cam_at);
        self.action = Action::Func80A53F30;
    }

    /// `func_80A53F30`: until the gate is done or 200 frames; then 0x2015 for the letter, or Link
    /// free after the gate's shut, or the mask's 15 rupees (0x2021).
    fn func_80a53f30(&mut self, play: &mut PlayState) {
        self.skel.update();
        play.camera_set_at_eye(self.sub_cam_id, self.sub_cam_eye, self.sub_cam_at);
        let opening_state = self.gate.and_then(|h| play.actors.downcast::<BgGateShutter>(h)).map_or(0, |g| g.opening_state);
        if self.unk_2f2[0] == 0 || opening_state == 0 {
            play.clear_camera(self.sub_cam_id);
            play.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
            if self.unk_30a != 2 {
                if self.unk_30a == 0 {
                    self.actor.text_id = 0x2015;
                    play.continue_textbox(0x2015);
                    self.action = Action::Func80A54038;
                } else {
                    play.with_msg(|m, f| m.close_textbox(f.audio));
                    play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
                    self.action = Action::Func80A53908;
                }
            } else {
                self.unk_30e = 0;
                self.actor.text_id = 0x2021;
                rupees_change_by(&mut play.save, 15);
                play.continue_textbox(0x2021);
                self.action = Action::Func80A5427C;
            }
        }
    }

    /// `func_80A54038`: the gate's text read: `INFTABLE_76`, Link free.
    fn func_80a54038(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.save.set_inf_table(INFTABLE_76);
            play.with_msg(|m, f| m.close_textbox(f.audio));
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            self.action = Action::Func80A53908;
        }
    }

    /// `func_80A540C0`: the Keaton Mask's choice: yes sells it (0x2020, `INFTABLE_77`,
    /// `ITEMGETINF_38`, `ITEM_SOLD_OUT`), no 0x200C.
    fn func_80a540c0(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_CHOICE && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    self.actor.text_id = 0x2020;
                    play.continue_textbox(0x2020);
                    if let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                        p.unset_mask();
                    }
                    play.save.set_inf_table(INFTABLE_77);
                    play.save.set_item_get_inf(ITEMGETINF_38);
                    item_give(&mut play.save, Some(&mut play.audio), ITEM_SOLD_OUT);
                    if self.unk_30a != 0 {
                        self.unk_30a = 2;
                        self.unk_30e = 1;
                        self.action = Action::Func80A5427C;
                    } else {
                        self.unk_30e = 0;
                        self.action = Action::Func80A541FC;
                    }
                }
                1 => {
                    self.unk_30e = 1;
                    self.actor.text_id = 0x200C;
                    play.continue_textbox(0x200C);
                    self.unk_300 = TEXT_STATE_EVENT;
                    self.action = if self.unk_30a == 0 { Action::Func80A5427C } else { Action::Func80A54954 };
                }
                _ => {}
            }
        }
    }

    /// `func_80A541FC`: the mask's 15 rupees (0x2021).
    fn func_80a541fc(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            self.actor.text_id = 0x2021;
            rupees_change_by(&mut play.save, 15);
            play.continue_textbox(0x2021);
            self.action = Action::Func80A5427C;
        }
    }

    /// `func_80A5427C`: done, or the gate shut (or opened, for the mask's) first.
    fn func_80a5427c(&mut self, play: &mut PlayState) {
        self.skel.update();
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            if self.unk_30e == 0 {
                self.unk_30e = 0;
                self.unk_30a = 0;
                play.with_msg(|m, f| m.close_textbox(f.audio));
                self.action = Action::Func80A53908;
            } else {
                play.msg_ctx.msg_mode = MSGMODE_PAUSED;
                self.action = Action::Func80A54320;
            }
        }
    }

    /// `func_80A54320`.
    fn func_80a54320(&mut self) {
        self.change_slam();
        self.audio_flag = 0;
        self.action = Action::Func80A543A0;
    }

    /// `func_80A543A0`: at the slam's end, the gate shut (`openingState` -1) or, for the mask's,
    /// opened (2).
    fn func_80a543a0(&mut self, play: &mut PlayState) {
        let frame_count = self.skel.cur_frame;
        self.skel.update();
        self.slam_sound(play, frame_count);
        if self.unk_2ec <= frame_count {
            let state = if self.unk_30a != 2 { -1 } else { 2 };
            self.call_gate(play, Some(state));
            if self.unk_30a == 0 {
                self.unk_30a = 1;
            }
            self.action = Action::Func80A53DF8;
        }
    }

    /// `func_80A544AC`: type 9 rolls onto his side (`shape.rot.z` to -6100), then says 0x708F.
    fn func_80a544ac(&mut self, play: &mut PlayState) {
        eng_math::smooth_step_to_s(&mut self.actor.shape_rot.z, -6100, 5, self.unk_2e4 as i16, 0);
        eng_math::approach_f(&mut self.unk_2e4, 3000.0, 1.0, 500.0);
        self.actor.world_rot.z = self.actor.shape_rot.z;
        if self.actor.shape_rot.z < -6000 {
            play.start_textbox(0x708F, None);
            self.actor.flags |= ACTOR_FLAG_TALK_OFFER_AUTO_ACCEPTED;
            self.action = Action::Func80A5455C;
            self.unk_2e4 = 0.0;
        }
    }

    /// `func_80A5455C`: on the text's event, Link free, the text closed and a bomb thrown.
    fn func_80a5455c(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.player_set_cs_action_with_halted_actors(None, PLAYER_CSACTION_7);
            play.with_msg(|m, f| m.close_textbox(f.audio));
            let pos = Vec3::new(play.rand.centered_float(20.0) + self.unk_274.x, play.rand.centered_float(20.0) + (self.unk_274.y - 40.0), play.rand.centered_float(20.0) + (self.unk_274.z - 20.0));
            let rot_y = play.rand.centered_float(7000.0) + self.actor.yaw_towards_player as f32;
            match play.actor_spawn(ACTOR_EN_BOM, pos, [0, rot_y as i32 as i16, 0], 0) {
                Ok(h) => {
                    let speed = play.rand.centered_float(5.0) + 10.0;
                    let vy = play.rand.centered_float(5.0) + 10.0;
                    if let Some(b) = play.actors.actor_mut(h) {
                        b.speed_xz = speed;
                        b.velocity.y = vy;
                    }
                    log::info!("En_Heishi2: its bomb (En_Bom) is a placeholder (BACKLOG #33)");
                }
                Err(e) => log::debug!("En_Heishi2: no bomb: {e:?}"),
            }
            log::debug!(" ☆☆☆☆☆ This is down! ☆☆☆☆☆ ");
            self.action = Action::Func80A546DC;
        }
    }

    /// `func_80A546DC`: type 9 rolls back (`shape.rot.z` to 200), then is gone.
    fn func_80a546dc(&mut self) {
        eng_math::smooth_step_to_s(&mut self.actor.shape_rot.z, 200, 5, self.unk_2e4 as i16, 0);
        eng_math::approach_f(&mut self.unk_2e4, 3000.0, 1.0, 500.0);
        self.actor.world_rot.z = self.actor.shape_rot.z;
        if self.actor.shape_rot.z > 0 {
            self.actor.kill();
        }
    }

    /// `func_80A5475C`: back to the text when a mask goes on or off; on a talk, the bribe's choice
    /// or the text again; else offering to talk (types 2 and 5 within 120 in front).
    fn func_80a5475c(&mut self, play: &mut PlayState) {
        self.skel.update();
        let back_to_text = if self.mask_reaction_text() != 0 { self.unk_30b == 0 } else { self.unk_30b != 0 };
        if back_to_text {
            if self.ty == 2 {
                self.action = Action::Func80A53278;
                return;
            }
            if self.ty == 5 {
                self.action = Action::Func80A5399C;
                return;
            }
        }
        if process_talk_request(&mut self.actor) {
            if self.ty == 2 {
                self.action = if self.unk_30e == 1 { Action::Func80A5344C } else { Action::Func80A53278 };
                return;
            } else if self.ty == 5 {
                if self.unk_300 == TEXT_STATE_DONE {
                    self.action = Action::Func80A5399C;
                }
                if self.unk_300 == TEXT_STATE_EVENT {
                    self.action = Action::Func80A54954;
                }
                if self.unk_300 == TEXT_STATE_CHOICE {
                    self.unk_309 = 1;
                    play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    self.action = Action::Func80A540C0;
                }
                return;
            }
        }
        let offers = if self.ty != 2 && self.ty != 5 {
            true
        } else {
            let yaw_diff = (self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) as i32).abs();
            !(self.actor.xz_dist_to_player > 120.0) && yaw_diff < 0x4300
        };
        if offers {
            let a = self.actor.clone();
            offer_talk_default(play, &a);
        }
    }

    /// `func_80A54954`.
    fn func_80a54954(&mut self) {
        self.change_idle();
        self.action = Action::Func80A549E8;
    }

    /// `func_80A549E8`: the text's end.
    fn func_80a549e8(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_300 == play.message_state() && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            if self.ty == 2 {
                self.action = Action::Func80A531E4;
            }
            if self.ty == 5 {
                self.action = Action::Func80A53908;
            }
        }
    }
}

/// `player->actor.textId = id` (the text Player's exchange shows).
fn set_player_text_id(play: &mut PlayState, id: u16) {
    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
        p.text_id = id;
    }
}

impl ActorImpl for EnHeishi2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHeishi2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.set_focus(self.unk_2e0);
        if self.ty == 2 || self.ty == 5 {
            // (focus.pos.y = 70, which Actor_SetFocus overwrites at once.)
            self.actor.set_focus(70.0);
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.unk_260, &mut self.unk_26c, focus);
        }
        self.unk_2fc = self.unk_2fc.wrapping_add(1);
        for t in &mut self.unk_2f2 {
            if *t != 0 {
                *t -= 1;
            }
        }
        match self.action {
            Action::None => {}
            Action::DoNothing1 | Action::DoNothing2 => {}
            Action::Func80A531E4 => self.func_80a531e4(),
            Action::Func80A53278 => self.func_80a53278(play),
            Action::Func80A5344C => self.func_80a5344c(play),
            Action::Func80A53538 => self.func_80a53538(play),
            Action::Func80A535BC => self.func_80a535bc(),
            Action::Func80A53638 => self.func_80a53638(play),
            Action::Func80A5372C => self.func_80a5372c(play),
            Action::Func80A53850 => self.func_80a53850(play),
            Action::Func80A53908 => self.func_80a53908(),
            Action::Func80A5399C => self.func_80a5399c(play),
            Action::Func80A53AD4 => self.func_80a53ad4(play),
            Action::Func80A53C0C => self.func_80a53c0c(play),
            Action::Func80A53C90 => self.func_80a53c90(),
            Action::Func80A53D0C => self.func_80a53d0c(play),
            Action::Func80A53DF8 => self.func_80a53df8(play),
            Action::Func80A53F30 => self.func_80a53f30(play),
            Action::Func80A54038 => self.func_80a54038(play),
            Action::Func80A540C0 => self.func_80a540c0(play),
            Action::Func80A541FC => self.func_80a541fc(play),
            Action::Func80A5427C => self.func_80a5427c(play),
            Action::Func80A54320 => self.func_80a54320(),
            Action::Func80A543A0 => self.func_80a543a0(play),
            Action::Func80A544AC => self.func_80a544ac(play),
            Action::Func80A5455C => self.func_80a5455c(play),
            Action::Func80A546DC => self.func_80a546dc(),
            Action::Func80A5475C => self.func_80a5475c(play),
            Action::Func80A54954 => self.func_80a54954(),
            Action::Func80A549E8 => self.func_80a549e8(play),
        }
        self.actor.move_forward();
        if self.ty != 6 && self.ty != 9 {
            self.actor.update_bg_check_info(&play.col, 10.0, 10.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
            if let Some(c) = self.collider.as_mut() {
                c.update(&self.actor);
                play.collision_check_set_oc(&self.actor, 0, c);
            }
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if !self.draw_king_guard {
            rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
            rs.angles = vec![self.unk_260[0], self.unk_260[1], self.unk_260[2], self.unk_26c[0], self.unk_26c[1], self.unk_26c[2]];
        }
        rs
    }
    /// `EnHeishi2_Draw` (`EnHeishi2_OverrideLimbDraw`: but for types 1 and 7, limb 9's x by the
    /// torso's yaw and limb 16's x and z by the head's yaw and roll; `EnHeishi2_PostLimbDraw` keeps
    /// limb 16's matrix for the Keaton Mask), or `EnHeishi2_DrawKingGuard`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let model = oot_game::play::actor_draw_matrix(rs);
        if self.draw_king_guard {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, "gHeishiKingGuardDL")), model));
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        if rs.angles.len() < 6 {
            return;
        }
        let a = &rs.angles;
        let bones = if self.ty == 1 || self.ty == 7 { EnHeishi4::pose(skeleton, &joints.rot, [0; 3], [0; 3]) } else { EnHeishi4::pose(skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]]) };
        // (Limb 16 is bone 15: the limbs count from 1.)
        let head = bones.get(15).copied();
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(en_heishi4::BAKE)), transform: model, bones, params: Default::default() });
        if self.ty == 5 && play.save.get_inf_table(INFTABLE_77) && play.object_ctx.get_index(OBJECT_LINK_CHILD).is_some() {
            if let Some(head) = head {
                // Matrix_Put(&mtxf_330), Matrix_Translate(-570, 0, 0), Matrix_RotateZ(DEG_TO_RAD(70)).
                let m = model * head * Mat4::from_translation(Vec3::new(-570.0, 0.0, 0.0)) * Mat4::from_rotation_z(70f32.to_radians());
                out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(KEATON_MASK_BAKE)), m));
            }
        }
        // ActorShadow_DrawCircle (shadowScale 30): the circle shadow's stand-in.
        let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
        let shadow = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(30.0, 1.0, 30.0));
        out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), shadow));
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(()).and(self.collider.as_mut()).map(ColliderMut::Cylinder)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
