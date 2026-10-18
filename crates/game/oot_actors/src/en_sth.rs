//! `En_Sth` (`ovl_En_Sth/z_en_sth.c`): the House of Skulltula's family, cured. Each stands on the
//! floor under its cursed self (`En_Ssh`) and appears once Link holds enough Gold Skulltula tokens
//! (`gsTokens`): the five children at 10, 20, 30, 40 and 50, the father at 100.
//!
//! - **Init** (`EnSth_Init`): too few tokens, gone. Else it finds its object's slot (the father
//!   `object_ahg`, the children `object_boj`) and waits for it (`EnSth_WaitForObject`), not drawn.
//! - **The object in** (`EnSth_SetupAfterObjectLoaded`): its collider, the father's or a child's
//!   skeleton (`gHylianMan1Skel`, `gHylianMan2Skel`) dancing its loop (`sParentDanceAnim`,
//!   `sChildDanceAnim`, from the overlay), its update the full one (`EnSth_Update2`); its reward
//!   taken already (its `EVENTCHKINF_SKULLTULA_REWARD_*` bit), it just talks; else it offers it.
//! - **The reward** (`EnSth_RewardUnobtainedWait`, within 100): its text (the father 0x28, a
//!   child 0x21); on the text's event and A (`EnSth_RewardUnobtainedTalk`) the box closes and
//!   it offers the item (`EnSth_GivePlayerItem`, 10000 across, 50 up or down): 10 the adult's
//!   wallet, 20 the Stone of Agony, 30 the giant's wallet, 40 ten Bombchus, 50 a piece of heart,
//!   100 a gold rupee; the 10's and 30's give the next wallet up from the one held (none: the
//!   adult's; the adult's: the giant's; else their own). Taken (`EnSth_GiveReward`), its bit set;
//!   then it talks (`EnSth_RewardObtainedTalk`) until the box closes.
//! - **Rewarded** (`EnSth_ChildRewardObtainedWait`: 0x20, or 0x1F from 50 tokens;
//!   `EnSth_ParentRewardObtainedWait`: 0x23), within 100.
//! - While waiting it turns its head to Link within 100 and 0x4300 of its facing
//!   (`EnSth_LookAtPlayer`); while talking it turns to face him (`EnSth_FacePlayer`).
//!
//! The father's reward bit is 0 (`sEventFlags[0]`): he is never "rewarded", so he gives a gold
//! rupee each time Link comes back in.
//!
//! Drawn as `EnSth_Draw` does: `Gfx_SetupDL_37Opa`, the shirt's env colour on segment 8 by params
//! (`sShirtColors`) and the trousers' on 9 (the father's brown, the children's grey-blue), the
//! skeleton with the overlay's head (`D_80B0A050` for limb 15, turned by `headRot`) and the
//! children's hair after it (`D_80B0A3C0`, `EnSth_PostLimbDraw`), the arms' and torso's fidget
//! (limbs 8, 10, 13: `FIDGET_*` by the frame counter), the focus 700 along the head and 400 up.
//! From baked meshes (docs/adr/0012-actor-bakes.md; the overlay's head and dances are read from
//! the overlay at its link address).
//!
//! The circle shadow (36) is the shape's (`EnSth_SetupShapeColliderUpdate2AndDraw`'s
//! `ActorShape_Init`, drawn by `Actor_Draw`). Kept though unused, as in the C: `unk_2B2` (bit 2 would skip a limb's fidget; never set) and the
//! blink timer `unk_2B6` / index `unk_2B4` (no eyes are swapped).
//!
//! Params: `params` whole, the family member, 0 to 5 (0 the father at 100 tokens; 1 to 5 the
//! children at `params × 10` tokens): its object, skeleton, dance, shirt, reward and reward bit.
//! Flags: reads `gsTokens`; reads and sets EVENTCHKINF_SKULLTULA_REWARD_10, _20, _30, _40, _50
//! (params 1 to 5, by `sEventFlags[params]` in `eventChkInf[0xD]`; the father none).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::collision_check::*;
use oot_game::event_chk_inf;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{GI_HEART_PIECE, GI_RUPEE_GOLD, UPG_WALLET};
use oot_game::message::TEXT_STATE_EVENT;
use oot_game::npc::{actor_track_player, offer_talk, process_talk_request, textbox_is_closing};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_STH` (`actor_table.h`: 0x0189).
pub const ACTOR_EN_STH: i16 = 0x0189;

/// `En_Sth_Profile`: `OBJECT_GAMEPLAY_KEEP`; it loads its own object.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_STH, name: "En_Sth", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep", schema: &SCHEMA };

/// `this->actor.params`, whole: the family member.
pub const MEMBER: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "the father: 100 tokens, the gold rupee"),
        (1, "10 tokens: the Adult's Wallet"),
        (2, "20 tokens: the Stone of Agony"),
        (3, "30 tokens: the Giant's Wallet"),
        (4, "40 tokens: the bombchus"),
        (5, "50 tokens: the piece of heart"),
    ]),
    "which of the cursed family: its object, skeleton, dance, shirt, reward and reward's flag; there from params × 10 tokens (the father 100)",
);

/// En_Sth's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[MEMBER],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SKULLTULA_REWARD_10)).reads().sets().note("params 1: its reward given").indirect("sEventFlags[params], a mask of eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SKULLTULA_REWARD_20)).reads().sets().note("params 2: its reward given").indirect("sEventFlags[params], a mask of eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SKULLTULA_REWARD_30)).reads().sets().note("params 3: its reward given").indirect("sEventFlags[params], a mask of eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SKULLTULA_REWARD_40)).reads().sets().note("params 4: its reward given").indirect("sEventFlags[params], a mask of eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_SKULLTULA_REWARD_50)).reads().sets().note("params 5: its reward given").indirect("sEventFlags[params], a mask of eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]"),
    ],
    todo: false,
};

/// `OBJECT_AHG` (`object_table.h`: 0x0107), `OBJECT_BOJ` (0x0111).
pub const OBJECT_AHG: i16 = 0x0107;
pub const OBJECT_BOJ: i16 = 0x0111;

/// The overlay, for its head, hair and dances.
const OVERLAY: &str = "ovl_En_Sth";

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;

// `GetItemID` (`item.h`).
/// `GI_BOMBCHUS_10` (`item.h`: 0x03).
pub const GI_BOMBCHUS_10: i16 = 0x03;
/// `GI_STONE_OF_AGONY` (`item.h`: 0x39).
pub const GI_STONE_OF_AGONY: i16 = 0x39;
/// `GI_WALLET_ADULT` (`item.h`: 0x45).
pub const GI_WALLET_ADULT: i16 = 0x45;
/// `GI_WALLET_GIANT` (`item.h`: 0x46).
pub const GI_WALLET_GIANT: i16 = 0x46;

/// `EVENTCHKINF_INDEX_SKULLTULA_REWARD` (`save.h`: 0xD): the word of `eventChkInf` the rewards'
/// bits are in.
pub const EVENTCHKINF_INDEX_SKULLTULA_REWARD: usize = 0xD;
/// `EVENTCHKINF_SKULLTULA_REWARD_10` .. `_50` (`save.h`: 0xDA to 0xDE).
pub const EVENTCHKINF_SKULLTULA_REWARD_10: u16 = 0xDA;
pub const EVENTCHKINF_SKULLTULA_REWARD_20: u16 = 0xDB;
pub const EVENTCHKINF_SKULLTULA_REWARD_30: u16 = 0xDC;
pub const EVENTCHKINF_SKULLTULA_REWARD_40: u16 = 0xDD;
pub const EVENTCHKINF_SKULLTULA_REWARD_50: u16 = 0xDE;

/// `EVENTCHKINF_MASK(flag)`: the flag's bit in its word.
const fn eventchkinf_mask(flag: u16) -> u16 {
    1 << (flag & 0xF)
}

/// `sObjectIds`.
const S_OBJECT_IDS: [i16; 6] = [OBJECT_AHG, OBJECT_BOJ, OBJECT_BOJ, OBJECT_BOJ, OBJECT_BOJ, OBJECT_BOJ];

/// `sSkeletons`: (the object's file, the skeleton).
const S_SKELETONS: [(&str, &str); 6] = [
    ("object_ahg", "gHylianMan1Skel"),
    ("object_boj", "gHylianMan2Skel"),
    ("object_boj", "gHylianMan2Skel"),
    ("object_boj", "gHylianMan2Skel"),
    ("object_boj", "gHylianMan2Skel"),
    ("object_boj", "gHylianMan2Skel"),
];

/// `sAnimations`.
const S_ANIMATIONS: [&str; 6] = ["sParentDanceAnim", "sChildDanceAnim", "sChildDanceAnim", "sChildDanceAnim", "sChildDanceAnim", "sChildDanceAnim"];

/// `sEventFlags`: the father's 0.
const S_EVENT_FLAGS: [u16; 6] = [
    0,
    eventchkinf_mask(EVENTCHKINF_SKULLTULA_REWARD_10),
    eventchkinf_mask(EVENTCHKINF_SKULLTULA_REWARD_20),
    eventchkinf_mask(EVENTCHKINF_SKULLTULA_REWARD_30),
    eventchkinf_mask(EVENTCHKINF_SKULLTULA_REWARD_40),
    eventchkinf_mask(EVENTCHKINF_SKULLTULA_REWARD_50),
];

/// `sGetItemIds`.
pub const S_GET_ITEM_IDS: [i16; 6] = [GI_RUPEE_GOLD, GI_WALLET_ADULT, GI_STONE_OF_AGONY, GI_WALLET_GIANT, GI_BOMBCHUS_10, GI_HEART_PIECE];

/// `EnSth_Draw`'s `sShirtColors`, by params.
const S_SHIRT_COLORS: [[u8; 3]; 6] = [[190, 110, 0], [0, 180, 110], [0, 255, 80], [255, 160, 60], [190, 230, 250], [240, 230, 120]];

/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_FREQ_Y: i32 = 0x814;
const FIDGET_FREQ_Z: i32 = 0x940;
const FIDGET_FREQ_LIMB: i32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

/// The head (1-based, `EnSth_OverrideLimbDraw`'s `limbIndex` 15).
const HEAD_LIMB: usize = 15;

/// `sCylinderInit`: 30 by 40.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ENEMY, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 40, y_shift: 0, pos: [0; 3] },
};

const SEG_SHIRT: u8 = 0x08;
const SEG_TROUSERS: u8 = 0x09;
const SEG_SETUP: u8 = 0x0C;

/// A member's mesh: its skeleton with the overlay's head, its colours.
pub fn bake_name(params: i16) -> String {
    format!("En_Sth/member{params}")
}

/// The children's hair (`D_80B0A3C0`), drawn at the head after it.
pub const HAIR_BAKE: &str = "En_Sth/hair";

/// `gDPSetEnvColor(r, g, b, 255)` (`EnSth_AllocColorDList`'s list).
fn env(c: [u8; 3]) -> BakeSegment {
    BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes([c[0], c[1], c[2], 255]))])
}

/// The meshes `EnSth_Draw` draws: each member's skeleton after `Gfx_SetupDL_37Opa` with its
/// shirt (8) and trousers (9), limb 15 the overlay's head; and the children's hair.
pub fn bakes() -> Vec<MeshBake> {
    let mut v: Vec<MeshBake> = (0..6)
        .map(|params| {
            let (file, skel) = S_SKELETONS[params];
            let trousers = if params == 0 { [190, 110, 0] } else { [90, 110, 130] };
            MeshBake {
                name: bake_name(params as i16),
                object: file.into(),
                segments: vec![(SEG_SHIRT, env(S_SHIRT_COLORS[params])), (SEG_TROUSERS, env(trousers)), (SEG_SETUP, BakeSegment::Commands(crate::en_owl::setup_dl_37()))],
                prelude: vec![SEG_SETUP],
                body: BakeBody::Skeleton {
                    file: file.into(),
                    symbol: skel.into(),
                    limbs: vec![LimbOverride { limb: (HEAD_LIMB - 1) as u8, file: OVERLAY.into(), symbol: "D_80B0A050".into() }],
                },
            }
        })
        .collect();
    v.push(MeshBake { name: HAIR_BAKE.into(), object: "object_boj".into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![(OVERLAY.into(), "D_80B0A3C0".into())]) });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObject,
    SetupAfterObjectLoaded,
    RewardObtainedTalk,
    ParentRewardObtainedWait,
    GiveReward,
    RewardUnobtainedTalk,
    RewardUnobtainedWait,
    ChildRewardObtainedWait,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitForObject => "EnSth_WaitForObject",
            Action::SetupAfterObjectLoaded => "EnSth_SetupAfterObjectLoaded",
            Action::RewardObtainedTalk => "EnSth_RewardObtainedTalk",
            Action::ParentRewardObtainedWait => "EnSth_ParentRewardObtainedWait",
            Action::GiveReward => "EnSth_GiveReward",
            Action::RewardUnobtainedTalk => "EnSth_RewardUnobtainedTalk",
            Action::RewardUnobtainedWait => "EnSth_RewardUnobtainedWait",
            Action::ChildRewardObtainedWait => "EnSth_ChildRewardObtainedWait",
        }
    }
}

/// `sRewardObtainedWaitActions`.
const S_REWARD_OBTAINED_WAIT_ACTIONS: [Action; 6] = [
    Action::ParentRewardObtainedWait,
    Action::ChildRewardObtainedWait,
    Action::ChildRewardObtainedWait,
    Action::ChildRewardObtainedWait,
    Action::ChildRewardObtainedWait,
    Action::ChildRewardObtainedWait,
];

pub struct EnSth {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<Skeleton>>,
    /// `eventFlag`: its reward's bit in `eventChkInf[EVENTCHKINF_INDEX_SKULLTULA_REWARD]`.
    pub event_flag: u16,
    /// `actor.draw` set (`drawFunc`, once the object is in).
    pub drawn: bool,
    /// `actor.update` is `EnSth_Update2` (once the object is in).
    pub update2: bool,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    /// `headRot`, `unk_2AC` (the torso's turn, which nothing draws).
    pub head_rot: [i16; 3],
    pub unk_2ac: [i16; 3],
    pub unk_2b2: u16,
    pub unk_2b4: i16,
    pub unk_2b6: i16,
    pub action: Action,
}

impl EnSth {
    fn params(&self) -> usize {
        MEMBER.get(self.actor.params) as usize
    }

    /// `EnSth_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnSth {
            actor,
            collider: ColliderCylinder::default(),
            skel: SkelAnimeStd::init_flex(15, None),
            skeleton: None,
            event_flag: 0,
            drawn: false,
            update2: false,
            required_object_slot: None,
            head_rot: [0; 3],
            unk_2ac: [0; 3],
            unk_2b2: 0,
            unk_2b4: 0,
            unk_2b6: 0,
            action: Action::WaitForObject,
        };
        let params = MEMBER.get(this.actor.params);
        log::debug!("Gold Skulltula Shop no = {params}");
        let tokens = play.save.inventory.gs_tokens;
        if params == 0 {
            if tokens < 100 {
                this.actor.kill();
                log::debug!("Gold Skulltula Shop I still can't be a human");
                return Box::new(this);
            }
        } else if tokens < params.wrapping_mul(10) {
            this.actor.kill();
            log::debug!("Gold Skulltula Shop I still can't be a human");
            return Box::new(this);
        }
        let Some(&object_id) = S_OBJECT_IDS.get(params as usize) else {
            log::error!("En_Sth: params {params} out of range");
            this.actor.kill();
            return Box::new(this);
        };
        // OBJECT_GAMEPLAY_KEEP never comes up (sObjectIds holds none).
        let object_slot = play.object_ctx.get_index(object_id);
        log::debug!("bank_ID = {object_slot:?}");
        if object_slot.is_none() {
            // ASSERT(0, "0", "../z_en_sth.c", 1564).
            log::error!("En_Sth: object {object_id:#x} not in the scene's objects");
            this.actor.kill();
            return Box::new(this);
        }
        this.required_object_slot = object_slot;
        // drawFunc = EnSth_Draw; actor.draw = NULL.
        this.actor.scale = Vec3::splat(0.01);
        this.action = Action::WaitForObject;
        this.drawn = false;
        this.unk_2b2 = 0;
        this.actor.target_mode = ATTENTION_RANGE_6;
        Box::new(this)
    }

    /// `EnSth_SetupShapeColliderUpdate2AndDraw`: the shadow (a circle, 36), the collider,
    /// immovable, `EnSth_Update2`, and drawn.
    fn setup_shape_collider_update2_and_draw(&mut self) {
        self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 36.0);
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.actor.col_chk_info.mass = MASS_IMMOVABLE;
        self.update2 = true;
        self.drawn = true;
    }

    /// `EnSth_SetupAfterObjectLoaded`: the skeleton dancing, its reward's bit, and its wait.
    fn setup_after_object_loaded(&mut self, play: &mut PlayState) {
        self.setup_shape_collider_update2_and_draw();
        let p = self.params();
        let (file, skel) = S_SKELETONS[p];
        if let Some(assets) = play.assets.clone() {
            match assets.skeleton(file, skel).and_then(|s| Ok((s, assets.animation(OVERLAY, S_ANIMATIONS[p])?))) {
                Ok((skeleton, anim)) => {
                    // SkelAnime_InitFlex(.., 16), Animation_PlayLoop.
                    self.skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), None);
                    self.skeleton = Some(skeleton);
                    let end = anim.last_frame();
                    self.skel.change(anim, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
                }
                Err(e) => log::error!("En_Sth: {e:#}"),
            }
        }
        self.event_flag = S_EVENT_FLAGS[p];
        // schema: EVENTCHKINF_SKULLTULA_REWARD_10 to _50, by sEventFlags[params]
        if play.save.event_chk_inf[EVENTCHKINF_INDEX_SKULLTULA_REWARD] & self.event_flag != 0 {
            self.action = S_REWARD_OBTAINED_WAIT_ACTIONS[p];
        } else {
            self.action = Action::RewardUnobtainedWait;
        }
    }

    /// `EnSth_WaitForObject`.
    fn wait_for_object(&mut self, play: &PlayState) {
        if self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s)) {
            self.actor.obj_bank_index = self.required_object_slot;
            self.action = Action::SetupAfterObjectLoaded;
        }
    }

    /// `EnSth_FacePlayer`: within 0x4000 of its facing, turning to Link (by 6, at most 0xFA0) and
    /// its head tracking him; further round, its head held 0x2000 to his side while it turns
    /// (by 12, at most 0x3E8).
    fn face_player(&mut self, play: &PlayState) {
        let diff_rot = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if (diff_rot as i32).abs() <= 0x4000 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 6, 0xFA0, 0x64);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.unk_2ac, focus);
        } else {
            if diff_rot < 0 {
                smooth_step_to_s(&mut self.head_rot[1], -0x2000, 6, 0x1838, 0x100);
            } else {
                smooth_step_to_s(&mut self.head_rot[1], 0x2000, 6, 0x1838, 0x100);
            }
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0xC, 0x3E8, 0x64);
            self.actor.world_rot.y = self.actor.shape_rot.y;
        }
    }

    /// `EnSth_LookAtPlayer`: within 0x4300 of its facing and 100 across, its head tracks Link;
    /// else the head and torso turn back.
    fn look_at_player(&mut self, play: &PlayState) {
        let diff_rot = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if (diff_rot as i32).abs() <= 0x4300 && self.actor.xz_dist_to_player < 100.0 {
            let focus = self.actor.focus_pos;
            actor_track_player(play, &mut self.actor, &mut self.head_rot, &mut self.unk_2ac, focus);
        } else {
            smooth_step_to_s(&mut self.head_rot[0], 0, 6, 0x1838, 0x64);
            smooth_step_to_s(&mut self.head_rot[1], 0, 6, 0x1838, 0x64);
            smooth_step_to_s(&mut self.unk_2ac[0], 0, 6, 0x1838, 0x64);
            smooth_step_to_s(&mut self.unk_2ac[1], 0, 6, 0x1838, 0x64);
        }
    }

    /// `EnSth_RewardObtainedTalk`: the box closing, back to its rewarded wait.
    fn reward_obtained_talk(&mut self, play: &PlayState) {
        if textbox_is_closing(play) {
            self.action = if MEMBER.get(self.actor.params) == 0 { Action::ParentRewardObtainedWait } else { Action::ChildRewardObtainedWait };
        }
        self.face_player(play);
    }

    /// `EnSth_ParentRewardObtainedWait`: 0x23, within 100.
    fn parent_reward_obtained_wait(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::RewardObtainedTalk;
        } else {
            self.actor.text_id = 0x23;
            if self.actor.xz_dist_to_player < 100.0 {
                let a = self.actor.clone();
                offer_talk(play, &a, 100.0);
            }
        }
        self.look_at_player(play);
    }

    /// `EnSth_GivePlayerItem`: its item (the 10's and 30's the next wallet up), within 10000
    /// across and 50 up or down.
    fn give_player_item(&self, play: &mut PlayState) {
        let mut get_item_id = S_GET_ITEM_IDS[self.params()];
        if matches!(MEMBER.get(self.actor.params), 1 | 3) {
            match play.save.cur_upg_value(UPG_WALLET) {
                0 => get_item_id = GI_WALLET_ADULT,
                1 => get_item_id = GI_WALLET_GIANT,
                _ => {}
            }
        }
        let a = self.actor.clone();
        offer_get_item_range(play, &a, get_item_id, 10000.0, 50.0);
    }

    /// `EnSth_GiveReward`: taken, its reward's bit set and it talks; else offered again.
    fn give_reward(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::RewardObtainedTalk;
            // schema: EVENTCHKINF_SKULLTULA_REWARD_10 to _50, by sEventFlags[params]
            play.save.event_chk_inf[EVENTCHKINF_INDEX_SKULLTULA_REWARD] |= self.event_flag;
        } else {
            self.give_player_item(play);
        }
        self.face_player(play);
    }

    /// `EnSth_RewardUnobtainedTalk`: the text's event and A: the box closed and the item offered.
    fn reward_unobtained_talk(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_EVENT && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.action = Action::GiveReward;
            self.give_player_item(play);
        }
        self.face_player(play);
    }

    /// `EnSth_RewardUnobtainedWait`: 0x28 (the father) or 0x21, within 100.
    fn reward_unobtained_wait(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::RewardUnobtainedTalk;
        } else {
            self.actor.text_id = if MEMBER.get(self.actor.params) == 0 { 0x28 } else { 0x21 };
            if self.actor.xz_dist_to_player < 100.0 {
                let a = self.actor.clone();
                offer_talk(play, &a, 100.0);
            }
        }
        self.look_at_player(play);
    }

    /// `EnSth_ChildRewardObtainedWait`: 0x20, or 0x1F from 50 tokens; within 100.
    fn child_reward_obtained_wait(&mut self, play: &mut PlayState) {
        if process_talk_request(&mut self.actor) {
            self.action = Action::RewardObtainedTalk;
        } else {
            self.actor.text_id = if play.save.inventory.gs_tokens < 50 { 0x20 } else { 0x1F };
            if self.actor.xz_dist_to_player < 100.0 {
                let a = self.actor.clone();
                offer_talk(play, &a, 100.0);
            }
        }
        self.look_at_player(play);
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitForObject => self.wait_for_object(play),
            Action::SetupAfterObjectLoaded => self.setup_after_object_loaded(play),
            Action::RewardObtainedTalk => self.reward_obtained_talk(play),
            Action::ParentRewardObtainedWait => self.parent_reward_obtained_wait(play),
            Action::GiveReward => self.give_reward(play),
            Action::RewardUnobtainedTalk => self.reward_unobtained_talk(play),
            Action::RewardUnobtainedWait => self.reward_unobtained_wait(play),
            Action::ChildRewardObtainedWait => self.child_reward_obtained_wait(play),
        }
    }

    /// `EnSth_Update2`: the collider, the move, the floor, the dance (from its start at its
    /// end), the action and the (unused) blink.
    fn update2(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        self.actor.move_forward();
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        if self.skel.update() {
            self.skel.cur_frame = 0.0;
        }
        self.run_action(play);
        // Likely an unused blink timer and eye index.
        if self.unk_2b6 != 0 {
            self.unk_2b6 -= 1;
        }
        if self.unk_2b6 == 0 {
            self.unk_2b6 = play.rand.s16_offset(0x3C, 0x3C);
        }
        self.unk_2b4 = self.unk_2b6;
        if self.unk_2b4 >= 3 {
            self.unk_2b4 = 0;
        }
    }

    /// The limb matrices `EnSth_OverrideLimbDraw` gives: the head turned by `headRot` (its x by
    /// `headRot.y`, its z by `headRot.x`), and limbs 8, 10 and 13 fidgeting by the frame
    /// counter `frames`.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head_rot: [i16; 2], frames: u32, unk_2b2: u16) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == HEAD_LIMB {
                rot[0] = rot[0].wrapping_add(head_rot[1]);
                rot[2] = rot[2].wrapping_add(head_rot[0]);
            }
            // unk_2B2 & 2 (never set) would skip the fidget for this limb and clear the bit.
            if unk_2b2 & 2 != 0 {
                return Mat4::IDENTITY;
            }
            if matches!(limb, 8 | 10 | 13) {
                let fidget_frequency = limb as i32 * FIDGET_FREQ_LIMB;
                let f = frames as i32;
                rot[1] = (rot[1] as f32 + sin_s(f.wrapping_mul(fidget_frequency + FIDGET_FREQ_Y) as i16) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(f.wrapping_mul(fidget_frequency + FIDGET_FREQ_Z) as i16) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnSth {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnSth_Update` (the action only) until the object is in, then `EnSth_Update2`.
    fn update(&mut self, play: &mut PlayState) {
        if self.update2 {
            self.update2(play);
        } else {
            self.run_action(play);
        }
    }
    /// `EnSth_PostLimbDraw`, the head: the focus at (700, 400, 0) along it.
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.drawn {
            return;
        }
        let Some(skeleton) = &self.skeleton else { return };
        let bones = Self::pose(skeleton, &self.skel.joint_table, [self.head_rot[0], self.head_rot[1]], play.state_frames, self.unk_2b2);
        if let Some(head) = bones.get(HEAD_LIMB - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(700.0, 400.0, 0.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if !self.drawn {
            return rs;
        }
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot[0], self.head_rot[1]];
        rs
    }
    /// `EnSth_Draw`: the member's mesh, and a child's hair at its head.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if !self.drawn {
            return;
        }
        let (Some(skeleton), Some(joints), [hx, hy]) = (&self.skeleton, &rs.joints, rs.angles.as_slice()) else { return };
        let bones = Self::pose(skeleton, &joints.rot, [*hx, *hy], play.state_frames, self.unk_2b2);
        let m = oot_game::play::actor_draw_matrix(rs);
        if MEMBER.get(self.actor.params) != 0
            && let Some(head) = bones.get(HEAD_LIMB - 1)
        {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(HAIR_BAKE)), m * *head));
        }
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(MEMBER.get(self.actor.params)))), transform: m, bones, params: Default::default() });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_bits_are_word_0xd() {
        // EVENTCHKINF_SKULLTULA_REWARD_10 0xDA: word 0xD, bit 10.
        assert_eq!((EVENTCHKINF_SKULLTULA_REWARD_10 >> 4) as usize, EVENTCHKINF_INDEX_SKULLTULA_REWARD);
        assert_eq!(S_EVENT_FLAGS, [0, 1 << 10, 1 << 11, 1 << 12, 1 << 13, 1 << 14]);
    }
}
