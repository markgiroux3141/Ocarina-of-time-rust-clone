//! `En_Niw_Lady` (`ovl_En_Niw_Lady/z_en_niw_lady.c`): Anju, the cucco lady. She waits for
//! `object_ane` (her skeleton, `gCuccoLadySkel`) and `object_os_anime` (her animations)
//! (`func_80AB9F24`), then by scene and age:
//! - in Kakariko as a child (`func_80ABA21C`, `func_80ABA244`): every frame she counts the cuccos
//!   (`En_Niw`) in her pen (|x - 330| < 90, |z - 1610| < 190), keeping each one's saved bit
//!   (`INFTABLE_199` to `INFTABLE_19F` by its index `unk_2AA`, set in the pen and cleared out of
//!   it) until she has given her reward; her text by the count (`sMissingCuccoTextIds`), a chime
//!   when Link has brought more since he last talked, the error sound when fewer. With seven in
//!   she gives the Empty Bottle (`GI_BOTTLE_EMPTY`, `ITEMGETINF_0C`) and clears the bits
//!   (`func_80ABA654`, `func_80ABAC00`, `func_80ABAC84`); again, with the bottle got, a purple
//!   rupee;
//! - in Kakariko as an adult (`func_80ABA778`, `func_80ABA878`): the Pocket Egg offered with a
//!   choice (`func_80ABA9B8`, `ITEMGETINF_2C`), then the trade: shown the Pocket Cucco once Talon
//!   has been woken in Kakariko, Cojiro for it (`func_80ABAB08`, `ITEMGETINF_2E`);
//! - in Impa's house at night (`func_80ABAD38`, `func_80ABAD7C`): 0x503D; by day she isn't there.
//!
//! Her animation follows her texts (`unk_26E`, `EnNiwLady_ChoseAnimation`); she turns to Link
//! (`Npc_TrackPoint`, preset 2, the whole body) and blinks.
//!
//! Drawn as `EnNiwLady_Draw` does: env (0, 0, 0, 255), the eyes on segment 8, an empty list on
//! 0x0C, the skeleton with the head and torso turns and, while fidgeting (`unk_275`), the sway of
//! limbs 8, 10 and 13 (`EnNiwLady_OverrideLimbDraw`); from meshes baked per eye
//! (docs/adr/0012-actor-bakes.md). The circle shadow (20) is the shape's (`ActorShape_Init`'s,
//! drawn by `Actor_Draw`).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: taken as 0, so none), the
//! debug count override (`BREG(7)`, 0).
//!
//! Params: none read (placed as 0xFFFF, unused: her scene and Link's age decide).
//! Flags: sets and clears `INFTABLE_199` to `INFTABLE_19F` (the cuccos penned, a bit each in
//! `infTable[25]`, all cleared once seven are in; read only by its debug prints); reads and sets
//! `ITEMGETINF_0C` (the bottle), `ITEMGETINF_2C` (the Pocket Egg) and `ITEMGETINF_2E` (Cojiro);
//! reads `EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO` (an adult's texts, the Pocket Cucco's trade).

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_NPC, ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind, ObjectNeed, TextLine};
use oot_game::audio::sfx::{NA_SE_SY_ERROR, NA_SE_SY_TRE_BOX_APPEAR};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::GI_RUPEE_PURPLE;
use oot_game::message::{TEXT_STATE_CHOICE, TEXT_STATE_DONE, TEXT_STATE_EVENT, TEXT_STATE_NONE};
use oot_game::npc::{EXCH_ITEM_NONE, NpcTrack, get_player_exchange_item_id, offer_talk, offer_talk_exchange, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table, item_get_inf};

use crate::en_niw::{self, EnNiw};

/// `ACTOR_EN_NIW_LADY` (`actor_table.h`: 0x013C).
pub const ACTOR_EN_NIW_LADY: i16 = 0x013C;
/// `OBJECT_ANE` (`object_table.h`: 0x0110): her skeleton.
const OBJECT_ANE: i16 = 0x0110;
pub const OBJECT: &str = "object_ane";
/// `OBJECT_OS_ANIME` (`object_table.h`: 0x00C5): her animations.
const OBJECT_OS_ANIME: i16 = 0x00C5;
const OS_ANIME: &str = "object_os_anime";

/// `En_Niw_Lady_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_NIW_LADY, name: "En_Niw_Lady", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// The placed params, read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// En_Niw_Lady's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_199)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19A)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19B)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19C)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19D)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19E)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(inf_table!(INFTABLE_19F)).sets().clears().note("a cucco in the pen (D_80ABB3B4's bit); all cleared once seven are in").indirect("the infTable word, by D_80ABB3B4's masks"),
        FlagUse::new(item_get_inf!(ITEMGETINF_0C)).reads().sets().note("the Empty Bottle given"),
        FlagUse::new(item_get_inf!(ITEMGETINF_2C)).reads().sets().note("the Pocket Egg given"),
        FlagUse::new(item_get_inf!(ITEMGETINF_2E)).reads().sets().note("Cojiro given"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO)).reads().note("an adult's texts and the Pocket Cucco's trade"),
    ],
    todo: false,
    // `OBJECT_OS_ANIME`: her animations; without it she is gone at once
    objects: &[ObjectNeed::new("object_os_anime").note("her animations")],
    texts: &[
        TextLine::new(0x5036).cond("Link a child, no cuccos in the pen").note("my cuccos have flown away; bring them back"),
        TextLine::new(0x5070).cond("Link a child, one cucco in the pen").note("catch six more"),
        TextLine::new(0x5072).cond("Link a child, two in the pen").note("five more"),
        TextLine::new(0x5037).cond("Link a child, three in the pen").note("the other four"),
        TextLine::new(0x5038).cond("Link a child, four in the pen").note("the other three"),
        TextLine::new(0x5039).cond("Link a child, five in the pen").note("the other two"),
        TextLine::new(0x503A).cond("Link a child, six in the pen").note("the last one"),
        TextLine::new(0x503B).cond("Link a child, all seven in the pen, the first time").note("thank you; the Empty Bottle (a purple rupee once it's got)"),
        TextLine::new(0x503D).cond("Link a child, all seven in the pen again after the reward").note("don't tease my cuccos"),
        TextLine::new(0x503C).cond("Link a child, fewer than seven in the pen after the reward").note("round them up or pay for them"),
        TextLine::new(0x503E).cond("Link an adult, before the Pocket Egg, the first time").note("the Pocket Cucco; will you try?"),
        TextLine::new(0x503F).cond("Link an adult, before the Pocket Egg, once she's been talked to").note("will you try?"),
        TextLine::new(0x5040).cond("answered No to the Pocket Egg").note("were you pecked by a cucco?"),
        TextLine::new(0x5042).cond("Link an adult, after the Pocket Egg, before Cojiro").note("please make my cucco happy"),
        TextLine::new(0x5027).cond("Link an adult, after Talon's woken, once he's turned down Cojiro").note("is your cucco happy?"),
        TextLine::new(0x5047).cond("Link an adult, after Cojiro").note("my brother must have been lonely"),
        TextLine::new(0x5043).cond("Link showing her the Pocket Cucco, after Talon's woken").note("a good Cucco Keeper; keep Cojiro?"),
        TextLine::new(0x5045).cond("answered No to Cojiro").note("you like that other cucco; keep it a while"),
        TextLine::new(0x00CF).cond("Link showing her another item").note("that item doesn't work here"),
        TextLine::new(0x503D).cond("only in Impa's house, at night").note("don't tease my cuccos"),
    ],
};

/// `SCENE_IMPAS_HOUSE` (`scene_table.h`: 0x37, `labo_scene`).
pub const SCENE_IMPAS_HOUSE: u16 = 0x37;

// `INFTABLE_*`, `ITEMGETINF_*`, `EVENTCHKINF_*` (`save.h`).
/// `INFTABLE_199` (to `INFTABLE_19F`: `INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F`, 25).
pub const INFTABLE_199: u16 = 0x199;
pub const INFTABLE_19A: u16 = 0x19A;
pub const INFTABLE_19B: u16 = 0x19B;
pub const INFTABLE_19C: u16 = 0x19C;
pub const INFTABLE_19D: u16 = 0x19D;
pub const INFTABLE_19E: u16 = 0x19E;
pub const INFTABLE_19F: u16 = 0x19F;
/// `INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F`.
const INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F: usize = 25;
/// `ITEMGETINF_0C`: the Empty Bottle got from Anju.
pub const ITEMGETINF_0C: u16 = 0x0C;
/// `ITEMGETINF_2C`: the Pocket Egg got.
pub const ITEMGETINF_2C: u16 = 0x2C;
/// `ITEMGETINF_2E`: Cojiro got.
pub const ITEMGETINF_2E: u16 = 0x2E;
/// `EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO` (0x6A).
pub const EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO: u16 = 0x6A;

// `GetItemID` (`item.h`).
/// `GI_COJIRO` (0x0E).
pub const GI_COJIRO: i16 = 0x0E;
/// `GI_BOTTLE_EMPTY` (0x0F).
pub const GI_BOTTLE_EMPTY: i16 = 0x0F;
/// `GI_POCKET_EGG` (0x1D).
pub const GI_POCKET_EGG: i16 = 0x1D;
/// `EXCH_ITEM_POCKET_CUCCO` (`item.h`: 0x06).
pub const EXCH_ITEM_POCKET_CUCCO: u8 = 0x06;

/// `ATTENTION_RANGE_6`.
const ATTENTION_RANGE_6: u8 = 6;
/// `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_FULL_BODY: i16 = 4;
/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_FREQ_Y: i32 = 0x814;
const FIDGET_FREQ_Z: i32 = 0x940;
const FIDGET_FREQ_LIMB: i32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

/// Her head and torso (`limbIndex`, 1-based).
const LIMB_TORSO: usize = 8;
const LIMB_HEAD: usize = 15;

/// `sMissingCuccoTextIds`: by the count in the pen (0 to 6), 7 the seventh, 8 again with seven,
/// 9 fewer than seven after the reward.
pub const MISSING_CUCCO_TEXT_IDS: [u16; 10] = [0x5036, 0x5070, 0x5072, 0x5037, 0x5038, 0x5039, 0x503A, 0x503B, 0x503D, 0x503C];

/// `sTradeItemTextIds`: by the adult's text index (`unk_27A`): 0 the first time, 1 again, 2 after
/// Cojiro, 4 and 9 after the Pocket Egg, 10 after Cojiro turned down; 3 the Pocket Egg turned down,
/// 5 the Pocket Cucco shown, 7 another item shown, 8 Cojiro turned down.
// schema: 0x5044 (index 6) is never used (0x5043 goes on to it)
pub const TRADE_ITEM_TEXT_IDS: [u16; 11] = [0x503E, 0x503F, 0x5047, 0x5040, 0x5042, 0x5043, 0x5044, 0x00CF, 0x5045, 0x5042, 0x5027];

/// `D_80ABB3B4`: each cucco's bit in `infTable[25]` (`INFTABLE_MASK(INFTABLE_199)` to
/// `INFTABLE_MASK(INFTABLE_19F)`).
const D_80ABB3B4: [u16; 7] = [1 << 9, 1 << 10, 1 << 11, 1 << 12, 1 << 13, 1 << 14, 1 << 15];

/// The pen (`func_80ABA244`): centre and half extents in x and z.
pub const PEN_CENTER: (f32, f32) = (330.0, 1610.0);
pub const PEN_HALF: (f32, f32) = (90.0, 190.0);

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 10, height: 10, y_shift: 0, pos: [0; 3] },
};

/// `sEyeTextures`, on segment 8.
const EYE_TEXTURES: [&str; 3] = ["gCuccoLadyEyeOpenTex", "gCuccoLadyEyeHalfTex", "gCuccoLadyEyeClosedTex"];
/// `gSPSegment(0x08, eye)`, `(0x0C, EnNiwLady_EmptyDList)`, and the direct
/// `gDPSetEnvColor(0, 0, 0, 255)` run first (0x0E).
const SEG_EYES: u8 = 0x08;
const SEG_EMPTY: u8 = 0x0C;
const SEG_ENV: u8 = 0x0E;

/// Her mesh with eye `eye`.
pub fn bake_name(eye: usize) -> String {
    format!("En_Niw_Lady/eye{eye}")
}

/// The meshes `EnNiwLady_Draw` draws: `gCuccoLadySkel` with each eye.
pub fn bakes() -> Vec<MeshBake> {
    EYE_TEXTURES
        .iter()
        .enumerate()
        .map(|(eye, tex)| MeshBake {
            name: bake_name(eye),
            object: OBJECT.into(),
            segments: vec![
                (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }),
                (SEG_EMPTY, BakeSegment::Commands(Vec::new())),
                (SEG_ENV, BakeSegment::Commands(vec![(0xFB00_0000, u32::from_be_bytes([0, 0, 0, 255]))])),
            ],
            prelude: vec![SEG_ENV],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gCuccoLadySkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Waiting for her objects.
    Func80AB9F24,
    /// Child: her first text.
    Func80ABA21C,
    /// Child: counting the pen, talking.
    Func80ABA244,
    /// Child: the reward's text, then the reward.
    Func80ABA654,
    /// Adult: her text by the trade.
    Func80ABA778,
    /// Adult: talking, shown an item.
    Func80ABA878,
    /// Adult: the Pocket Egg's choice.
    Func80ABA9B8,
    /// Adult: a text, then back to `func_80ABA778`.
    Func80ABAA9C,
    /// Adult: Cojiro's choice.
    Func80ABAB08,
    /// Giving the item.
    Func80ABAC00,
    /// The item got: its text, then its flag.
    Func80ABAC84,
    /// Impa's house.
    Func80ABAD38,
    Func80ABAD7C,
}

/// Her animations from `object_os_anime`.
struct Anims {
    a_07d0: Anim,
    a_9f94: Anim,
    a_0718: Anim,
    a_a630: Anim,
}

pub struct EnNiwLady {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    pub head_rot: [i16; 3],
    pub torso_rot: [i16; 3],
    pub unused_timer: i16,
    /// `unk_262` ("message_end_code"): the message state her choice or event waits for.
    pub unk_262: u8,
    pub unused_timer2: i16,
    pub unused_random_timer: i16,
    pub cuccos_in_pen: i16,
    /// `unk_26A`: the count when Link last talked to her (the most so far).
    pub unk_26a: i16,
    /// `unk_26C` ("heart"): 1 once the seventh is in (the reward given), 2 after the count fell.
    pub unk_26c: i16,
    /// `unk_26E`: the animation to choose next, plus 1 (0 none).
    pub unk_26e: i16,
    /// `unk_270`: the animation chosen.
    pub unk_270: i16,
    pub unk_272: u8,
    /// `unk_273`: the Pocket Egg's choice is up.
    pub unk_273: u8,
    /// `unk_274`: talked to once.
    pub unk_274: u8,
    /// `unk_275`: fidgeting.
    pub unk_275: u8,
    /// `unk_276`: her head follows Link's yaw (0: eased back to her front).
    pub unk_276: u8,
    /// `unk_277`: Cojiro declined.
    pub unk_277: u8,
    /// `unk_278`: 1 in Impa's house.
    pub unk_278: i16,
    /// `unk_27A`: the adult's text index (`sTradeItemTextIds`).
    pub unk_27a: i16,
    /// `faceState`: the eye.
    pub face_state: i16,
    /// `unk_27E`: her skeleton is set up.
    pub unk_27e: i16,
    pub ane_object_slot: Option<usize>,
    pub os_anime_object_slot: Option<usize>,
    pub get_item_id: i16,
    pub interact_info: NpcTrack,
    pub collider: ColliderCylinder,
    anims: Option<Anims>,
}

/// `MaskReaction_GetTextId(play, MASK_REACTION_SET_CUCCO_LADY)`: not ported, 0 (BACKLOG #39).
fn mask_reaction_text(_play: &PlayState) -> u16 {
    0
}

impl EnNiwLady {
    /// `EnNiwLady_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnNiwLady {
            actor,
            skel: SkelAnimeStd::init_flex(15, None),
            skeleton: None,
            action: Action::Func80AB9F24,
            head_rot: [0; 3],
            torso_rot: [0; 3],
            unused_timer: 0,
            unk_262: 0,
            unused_timer2: 0,
            unused_random_timer: 0,
            cuccos_in_pen: 0,
            unk_26a: 0,
            unk_26c: 0,
            unk_26e: 0,
            unk_270: 0,
            unk_272: 0,
            unk_273: 0,
            unk_274: 0,
            unk_275: 0,
            unk_276: 0,
            unk_277: 0,
            unk_278: 0,
            unk_27a: 0,
            face_state: 0,
            unk_27e: 0,
            ane_object_slot: None,
            os_anime_object_slot: None,
            get_item_id: 0,
            interact_info: NpcTrack::default(),
            collider: ColliderCylinder::default(),
            anims: None,
        };
        this.ane_object_slot = play.object_ctx.get_index(OBJECT_ANE); // schema: her profile's object
        this.os_anime_object_slot = play.object_ctx.get_index(OBJECT_OS_ANIME);
        if this.os_anime_object_slot.is_none() || this.ane_object_slot.is_none() {
            this.actor.kill();
            return Box::new(this);
        }
        this.unk_278 = 0;
        if play.scene_id == SCENE_IMPAS_HOUSE {
            this.unk_278 = 1;
        }
        if this.unk_278 != 0 && play.save.is_day() {
            this.actor.kill();
            return Box::new(this);
        }
        log::debug!("En_Niw_Lady: Neechan ugh {}", this.unk_278);
        this.action = Action::Func80AB9F24;
        this.actor.culling_volume_distance = 600.0;
        Box::new(this)
    }

    /// `Animation_Change(anim, 1, 0, lastFrame, ANIMMODE_LOOP, morph)`.
    fn change(&mut self, which: fn(&Anims) -> &Anim, morph: f32) {
        let Some(anims) = &self.anims else { return };
        let anim = which(anims).clone();
        let frames = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, frames, ANIMMODE_LOOP, morph);
    }

    /// `EnNiwLady_ChoseAnimation`: the animation for `arg2` if it isn't the one chosen.
    fn chose_animation(&mut self, play: &PlayState, mut arg2: i16) {
        if mask_reaction_text(play) != 0 {
            arg2 = 8;
        }
        if arg2 != self.unk_270 {
            self.unk_275 = 0;
            self.unk_276 = 1;
            self.unk_270 = arg2;
            match arg2 {
                10 | 9 => {
                    if arg2 == 10 {
                        self.unk_275 = 1;
                    }
                    self.change(|a| &a.a_07d0, -10.0);
                }
                0..=6 | 8 | 21 | 22 | 24 | 29 => self.change(|a| &a.a_9f94, -10.0),
                7 | 20 | 23 | 25 | 26 | 27 | 28 => self.change(|a| &a.a_0718, -10.0),
                100 => {
                    self.change(|a| &a.a_a630, -10.0);
                    self.unk_276 = 0;
                }
                _ => {}
            }
        }
    }

    /// `func_80AB9F24`: once her objects are in, her skeleton, shape, collider and first
    /// animation, and her action by scene and age.
    fn func_80ab9f24(&mut self, play: &mut PlayState) {
        let loaded = |s: Option<usize>| s.is_some_and(|b| play.object_ctx.is_loaded(b));
        if !(loaded(self.ane_object_slot) && loaded(self.os_anime_object_slot)) {
            return;
        }
        let Some(assets) = play.assets.clone() else { return };
        let r = (|| -> anyhow::Result<_> {
            Ok((
                assets.skeleton(OBJECT, "gCuccoLadySkel")?,
                Anims {
                    a_07d0: assets.animation(OS_ANIME, "gObjOsAnim_07D0")?,
                    a_9f94: assets.animation(OS_ANIME, "gObjOsAnim_9F94")?,
                    a_0718: assets.animation(OS_ANIME, "gObjOsAnim_0718")?,
                    a_a630: assets.animation(OS_ANIME, "gObjOsAnim_A630")?,
                },
            ))
        })();
        let (skeleton, anims) = match r {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Niw_Lady: {e:#}");
                self.actor.kill();
                return;
            }
        };
        // SkelAnime_InitFlex(.., gCuccoLadySkel, NULL, .., 16).
        self.skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), None);
        self.skeleton = Some(skeleton);
        self.anims = Some(anims);
        self.unk_27e = 1;
        self.actor.gravity = -3.0;
        self.actor.scale = Vec3::splat(0.01);
        self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 20.0);
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.unk_272 = 0;
        self.actor.target_mode = ATTENTION_RANGE_6;
        let s = &play.save;
        match self.unk_278 {
            0 => {
                // (s16)lastFrame: whole already.
                if !s.get_item_get_inf(ITEMGETINF_0C) && !s.adult {
                    self.change(|a| &a.a_a630, 0.0);
                } else {
                    self.change(|a| &a.a_07d0, 0.0);
                }
                self.action = if s.adult { Action::Func80ABA778 } else { Action::Func80ABA21C };
            }
            1 => {
                self.change(|a| &a.a_07d0, 0.0);
                self.action = Action::Func80ABAD38;
            }
            _ => {}
        }
    }

    /// `func_80ABA21C`.
    fn func_80aba21c(&mut self) {
        self.actor.text_id = MISSING_CUCCO_TEXT_IDS[0];
        self.unk_262 = TEXT_STATE_DONE;
        self.action = Action::Func80ABA244;
    }

    /// The pen's test (`func_80ABA244`).
    pub fn in_pen(pos: Vec3) -> bool {
        (pos.x - PEN_CENTER.0).abs() < PEN_HALF.0 && (pos.z - PEN_CENTER.1).abs() < PEN_HALF.1
    }

    /// `func_80ABA244`: the cuccos in the pen counted (their bits kept until the reward), her text
    /// by the count, and the talk.
    fn func_80aba244(&mut self, play: &mut PlayState) {
        self.cuccos_in_pen = 0;
        let list = play.actors.category(ACTORCAT_PROP).to_vec();
        for h in list {
            let Some(cucco) = play.actors.downcast::<EnNiw>(h) else { continue };
            if cucco.actor.id != en_niw::ACTOR_EN_NIW {
                continue;
            }
            let (pos, idx) = (cucco.actor.world_pos, cucco.unk_2aa as usize);
            // D_80ABB3B4[unk_2AA]: an index past the table reads the next statics; unk_2AA is 0
            // to 6 (Kakariko's) or 0.
            let mask = D_80ABB3B4.get(idx).copied().unwrap_or(0);
            // schema: INFTABLE_199 to INFTABLE_19F, by the cucco's mask
            let inf = &mut play.save.inf_table[INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F];
            if Self::in_pen(pos) {
                if self.unk_26c == 0 {
                    *inf |= mask;
                    log::trace!("En_Niw_Lady: GET inside the chicken fence! {mask:x}");
                }
                self.cuccos_in_pen += 1;
            } else if self.unk_26c == 0 {
                *inf &= !mask;
            }
        }
        // DEBUG_FEATURES && BREG(7) != 0: the count from the debug register (not ported: 0).
        let mut phi_s1 = self.cuccos_in_pen;
        let st = play.message_state();
        if st == TEXT_STATE_NONE || st == TEXT_STATE_DONE {
            self.unk_26e = 101;
        }
        if self.cuccos_in_pen >= 7 {
            phi_s1 = 8;
            if self.unk_26c < 2 && self.unk_26c == 0 {
                phi_s1 = 7;
            }
        }
        if self.unk_26c != 0 && phi_s1 < 7 {
            phi_s1 = 9;
        }
        self.actor.text_id = MISSING_CUCCO_TEXT_IDS[phi_s1 as usize];
        if mask_reaction_text(play) != 0 {
            self.actor.text_id = mask_reaction_text(play);
            self.unk_262 = TEXT_STATE_DONE;
        }
        if self.unk_26c != 0 && phi_s1 != 9 {
            phi_s1 = 10;
            self.unk_26e = 11;
        }
        if process_talk_request(&mut self.actor) {
            log::debug!(
                "En_Niw_Lady: select {phi_s1}, heart {}, saved {}, now {}, talk_message {:x}, message_end_code {}",
                self.unk_26c,
                self.unk_26a,
                self.cuccos_in_pen,
                self.actor.text_id,
                self.unk_262
            );
            if mask_reaction_text(play) == 0 {
                // OOT_VERSION >= NTSC_1_1 (this ROM): 0x503C, fewer than seven after the reward.
                if self.actor.text_id == 0x503C {
                    play.audio.play_sfx_centered(NA_SE_SY_ERROR);
                    self.unk_26c = 2;
                    self.unk_262 = TEXT_STATE_EVENT;
                    self.action = Action::Func80ABA654;
                    return;
                }
                self.unk_26e = phi_s1 + 1;
                if phi_s1 == 7 {
                    play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    self.unk_26c = 1;
                    self.unk_262 = TEXT_STATE_EVENT;
                    self.unk_26a = self.cuccos_in_pen;
                    // schema: INFTABLE_199 to INFTABLE_19F, all cleared
                    let inf = &mut play.save.inf_table[INFTABLE_INDEX_199_19A_19B_19C_19D_19E_19F];
                    log::debug!("En_Niw_Lady: before changing the fence BIT {inf:x}");
                    *inf &= !D_80ABB3B4.iter().fold(0, |a, b| a | b);
                    log::debug!("En_Niw_Lady: after changing the fence BIT {inf:x}");
                    self.action = Action::Func80ABA654;
                    return;
                }
                if self.unk_26a != self.cuccos_in_pen {
                    if self.cuccos_in_pen < self.unk_26a {
                        play.audio.play_sfx_centered(NA_SE_SY_ERROR);
                    } else if phi_s1 + 1 < 9 {
                        play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                    }
                }
                if self.unk_26a < self.cuccos_in_pen {
                    self.unk_26a = self.cuccos_in_pen;
                }
            }
        } else {
            offer_talk(play, &self.actor, 100.0);
        }
    }

    /// `func_80ABA654`: her event text read, the reward: the bottle, or (got) a purple rupee.
    fn func_80aba654(&mut self, play: &mut PlayState) {
        if self.unk_262 == play.message_state() && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            log::debug!("En_Niw_Lady: heart {}, bomb {}", self.unk_26c, self.unk_272);
            self.unk_26e = 0xB;
            if !play.save.get_item_get_inf(ITEMGETINF_0C) {
                self.actor.parent = None;
                self.get_item_id = GI_BOTTLE_EMPTY;
                offer_get_item_range(play, &self.actor, GI_BOTTLE_EMPTY, 100.0, 50.0);
                self.action = Action::Func80ABAC00;
                return;
            }
            if self.unk_26c == 1 {
                self.get_item_id = GI_RUPEE_PURPLE;
                offer_get_item_range(play, &self.actor, GI_RUPEE_PURPLE, 100.0, 50.0);
                // @bug (game): no return: the action is set back to func_80ABA244 below, so the
                // purple rupee is offered this one frame only (lost if Player can't take it then).
                self.action = Action::Func80ABAC00;
            }
            self.action = Action::Func80ABA244;
        }
    }

    /// `func_80ABA778`: the adult's text by the trade.
    fn func_80aba778(&mut self, play: &PlayState) {
        log::trace!("En_Niw_Lady: adult message check");
        let s = &play.save;
        self.unk_262 = TEXT_STATE_DONE;
        self.unk_273 = 0;
        if !s.get_item_get_inf(ITEMGETINF_2C) {
            self.unk_27a = if self.unk_274 != 0 { 1 } else { 0 };
            self.unk_273 = 1;
            self.unk_262 = TEXT_STATE_CHOICE;
        } else {
            self.unk_27a = 2;
            if !s.get_item_get_inf(ITEMGETINF_2E) {
                self.unk_27a = 3;
                if s.get_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO) {
                    self.unk_27a = 9;
                    if self.unk_277 != 0 {
                        self.unk_27a = 10;
                    }
                } else {
                    self.unk_27a = 4;
                }
            }
        }
        self.actor.text_id = TRADE_ITEM_TEXT_IDS[self.unk_27a as usize];
        self.action = Action::Func80ABA878;
    }

    /// `func_80ABA878`: talking, or shown an item (the Pocket Cucco).
    fn func_80aba878(&mut self, play: &mut PlayState) {
        let st = play.message_state();
        if st == TEXT_STATE_NONE || st == TEXT_STATE_DONE {
            self.unk_26e = 11;
        }
        if process_talk_request(&mut self.actor) {
            let player_exchange_item_id = get_player_exchange_item_id(play);
            let set_player_text = |play: &mut PlayState, text_id: u16| {
                if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                    p.text_id = text_id;
                }
            };
            if player_exchange_item_id == EXCH_ITEM_POCKET_CUCCO && play.save.get_event_chk_inf(EVENTCHKINF_TALON_WOKEN_IN_KAKARIKO) {
                play.audio.play_sfx_centered(NA_SE_SY_TRE_BOX_APPEAR);
                set_player_text(play, TRADE_ITEM_TEXT_IDS[5]);
                self.unk_26e = self.unk_27a + 21;
                self.unk_262 = TEXT_STATE_CHOICE;
                self.action = Action::Func80ABAB08;
            } else if player_exchange_item_id != EXCH_ITEM_NONE {
                set_player_text(play, TRADE_ITEM_TEXT_IDS[7]);
                self.unk_26e = self.unk_27a + 21;
            } else {
                self.unk_274 = 1;
                self.unk_26e = self.unk_27a + 21;
                self.action = if self.unk_273 == 0 { Action::Func80ABA778 } else { Action::Func80ABA9B8 };
            }
            return;
        }
        offer_talk_exchange(play, &self.actor, 50.0, EXCH_ITEM_POCKET_CUCCO);
    }

    /// `func_80ABA9B8`: the Pocket Egg's choice.
    fn func_80aba9b8(&mut self, play: &mut PlayState) {
        if self.unk_262 == play.message_state() && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    play.with_msg(|m, f| m.close_textbox(f.audio));
                    self.actor.parent = None;
                    offer_get_item_range(play, &self.actor, GI_POCKET_EGG, 200.0, 100.0);
                    self.action = Action::Func80ABAC00;
                }
                1 => {
                    self.actor.text_id = TRADE_ITEM_TEXT_IDS[3];
                    self.unk_26e = self.unk_27a + 21;
                    play.continue_textbox(self.actor.text_id);
                    self.unk_262 = TEXT_STATE_EVENT;
                    self.action = Action::Func80ABAA9C;
                }
                _ => {}
            }
        }
    }

    /// `func_80ABAA9C`.
    fn func_80abaa9c(&mut self, play: &mut PlayState) {
        self.unk_26e = 11;
        if self.unk_262 == play.message_state() && play.message_should_advance() {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            self.action = Action::Func80ABA778;
        }
    }

    /// `func_80ABAB08`: Cojiro's choice.
    fn func_80abab08(&mut self, play: &mut PlayState) {
        if self.unk_262 == play.message_state() && play.message_should_advance() {
            match play.msg_ctx.choice_index {
                0 => {
                    play.with_msg(|m, f| m.close_textbox(f.audio));
                    self.actor.parent = None;
                    offer_get_item_range(play, &self.actor, GI_COJIRO, 200.0, 100.0);
                    self.action = Action::Func80ABAC00;
                }
                1 => {
                    play.with_msg(|m, f| m.close_textbox(f.audio));
                    self.unk_277 = 1;
                    self.actor.text_id = TRADE_ITEM_TEXT_IDS[8];
                    self.unk_26e = self.unk_27a + 21;
                    play.continue_textbox(self.actor.text_id);
                    self.unk_262 = TEXT_STATE_EVENT;
                    self.action = Action::Func80ABAA9C;
                }
                _ => {}
            }
        }
    }

    /// `func_80ABAC00`: offering the item until Player has it.
    fn func_80abac00(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.action = Action::Func80ABAC84;
        } else {
            let mut get_item_id = self.get_item_id;
            if play.save.adult {
                get_item_id = if !play.save.get_item_get_inf(ITEMGETINF_2C) { GI_POCKET_EGG } else { GI_COJIRO };
            }
            offer_get_item_range(play, &self.actor, get_item_id, 200.0, 100.0);
        }
    }

    /// `func_80ABAC84`: the item's text read, its flag.
    fn func_80abac84(&mut self, play: &mut PlayState) {
        if play.message_state() != TEXT_STATE_DONE || !play.message_should_advance() {
            return;
        }
        log::debug!("En_Niw_Lady: normal termination");
        if play.save.adult {
            if !play.save.get_item_get_inf(ITEMGETINF_2C) {
                play.save.set_item_get_inf(ITEMGETINF_2C);
            } else {
                play.save.set_item_get_inf(ITEMGETINF_2E);
            }
            self.action = Action::Func80ABA778;
        } else {
            play.save.set_item_get_inf(ITEMGETINF_0C);
            self.unk_262 = TEXT_STATE_DONE;
            self.action = Action::Func80ABA244;
        }
    }

    /// `func_80ABAD38`.
    fn func_80abad38(&mut self) {
        log::trace!("En_Niw_Lady: normal message check");
        self.unk_262 = TEXT_STATE_DONE;
        self.action = Action::Func80ABAD7C;
    }

    /// `func_80ABAD7C`: in Impa's house.
    fn func_80abad7c(&mut self, play: &mut PlayState) {
        self.actor.text_id = 0x503D;
        if mask_reaction_text(play) != 0 {
            self.actor.text_id = mask_reaction_text(play);
        }
        let st = play.message_state();
        if st == TEXT_STATE_NONE || st == TEXT_STATE_DONE {
            self.unk_26e = 8;
        }
        if process_talk_request(&mut self.actor) {
            self.unk_274 = 1;
            self.unk_26e = self.unk_27a + 9;
            self.action = Action::Func80ABAD38;
        } else {
            offer_talk(play, &self.actor, 100.0);
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Func80AB9F24 => self.func_80ab9f24(play),
            Action::Func80ABA21C => self.func_80aba21c(),
            Action::Func80ABA244 => self.func_80aba244(play),
            Action::Func80ABA654 => self.func_80aba654(play),
            Action::Func80ABA778 => self.func_80aba778(play),
            Action::Func80ABA878 => self.func_80aba878(play),
            Action::Func80ABA9B8 => self.func_80aba9b8(play),
            Action::Func80ABAA9C => self.func_80abaa9c(play),
            Action::Func80ABAB08 => self.func_80abab08(play),
            Action::Func80ABAC00 => self.func_80abac00(play),
            Action::Func80ABAC84 => self.func_80abac84(play),
            Action::Func80ABAD38 => self.func_80abad38(),
            Action::Func80ABAD7C => self.func_80abad7c(play),
        }
    }

    /// The limb matrices `EnNiwLady_Draw` draws with: `EnNiwLady_OverrideLimbDraw`'s head (x by
    /// the head's yaw, z by its pitch) and torso (x by its yaw), and while fidgeting the sway of
    /// limbs 8, 10 and 13 by `frames`.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3], fidget: bool, frames: u32) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == LIMB_HEAD {
                rot[0] = rot[0].wrapping_add(head[1]);
                rot[2] = rot[2].wrapping_add(head[0]);
            }
            if limb == LIMB_TORSO {
                rot[0] = rot[0].wrapping_add(torso[1]);
            }
            if fidget && (limb == 8 || limb == 10 || limb == 13) {
                let l = limb as i32 * FIDGET_FREQ_LIMB;
                let ay = frames.wrapping_mul((l + FIDGET_FREQ_Y) as u32) as i16;
                let az = frames.wrapping_mul((l + FIDGET_FREQ_Z) as u32) as i16;
                rot[1] = (rot[1] as f32 + sin_s(ay) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(az) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            Mat4::IDENTITY
        })
    }
}

impl ActorImpl for EnNiwLady {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnNiwLady_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.set_focus(60.0);
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.target = player_pos;
        if !play.save.adult {
            self.interact_info.target.y = player_pos.y - 10.0;
        }
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 2, NPC_TRACKING_FULL_BODY);
        self.interact_info = t;
        self.head_rot = self.interact_info.head;
        self.torso_rot = self.interact_info.torso;
        if self.unk_276 == 0 {
            smooth_step_to_s(&mut self.head_rot[1], 0, 5, 3000, 0);
        }
        if self.os_anime_object_slot.is_some() {
            if self.unk_27e != 0 {
                if self.unk_26e != 0 {
                    self.unk_26e -= 1;
                    let a = self.unk_26e;
                    self.chose_animation(play, a);
                    self.unk_26e = 0;
                }
                self.skel.update();
            }
            self.ane_object_slot = play.object_ctx.get_index(OBJECT_ANE); // schema: her profile's object
            if self.ane_object_slot.is_none() {
                return;
            }

            self.run_action(play);
            if self.actor.killed {
                return;
            }

            if self.unused_timer2 != 0 {
                self.unused_timer2 -= 1;
            }
            if self.unused_random_timer != 0 {
                self.unused_random_timer -= 1;
            }
            self.unused_timer = self.unused_timer.wrapping_add(1);
            if self.unused_random_timer == 0 {
                self.face_state += 1;
                if self.face_state >= 3 {
                    self.face_state = 0;
                    self.unused_random_timer = (play.rand.zero_float(60.0) as i16).wrapping_add(0x14);
                }
            }
            self.actor.update_bg_check_info(&play.col, 20.0, 20.0, 60.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if self.unk_27e == 0 {
            return rs;
        }
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.head_rot, self.torso_rot);
        rs.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        rs.switches = vec![self.face_state.max(0) as u32, self.unk_275 as u32];
        rs
    }
    /// `EnNiwLady_Draw`.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        let ([eye, fidget], [h0, h1, h2, t0, t1, t2]) = (rs.switches.as_slice(), rs.angles.as_slice()) else { return };
        let bones = Self::pose(skeleton, &joints.rot, [*h0, *h1, *h2], [*t0, *t1, *t2], *fidget != 0, play.state_frames);
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
